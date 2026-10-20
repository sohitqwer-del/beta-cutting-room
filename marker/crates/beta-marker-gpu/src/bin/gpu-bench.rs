//! Paired, sequential, alternating-order benchmark. Evidence stays outside git.
use anyhow::{Context, Result, ensure};
use beta_marker_core::{Job, MarkerResult};
use beta_marker_gpu::{BackendChoice, GpuOptions, Proposer};
use beta_marker_nest::NestOptions;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

fn command(name: &str) -> Command {
    let mut command = Command::new(name);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    command
}

fn save(path: impl AsRef<Path>, value: &impl serde::Serialize) -> Result<()> {
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

fn run(job: &Job, options: &NestOptions) -> Result<MarkerResult> {
    let result = beta_marker_nest::nest(job, options)?;
    let report = beta_marker_core::validate(job, &result);
    ensure!(report.ok, "independent validation: {:?}", report.errors);
    Ok(result)
}

type PairKey = (String, u64);

#[derive(Default)]
struct Prior {
    pairs: BTreeMap<PairKey, Vec<Value>>,
    cpu: BTreeMap<String, Value>,
    fills: BTreeMap<String, Value>,
    root: Option<PathBuf>,
}

fn records(path: impl AsRef<Path>) -> Result<Vec<Value>> {
    fs::read_to_string(path)?
        .lines()
        .map(|line| Ok(serde_json::from_str(line)?))
        .collect()
}

fn overlaps(row: &Value, gaps: &[(f64, f64)]) -> bool {
    let Some((start, wall)) = row["start_unix_ms"].as_f64().zip(row["wall_s"].as_f64()) else {
        return true;
    };
    gaps.iter()
        .any(|&(low, high)| start <= high && start + wall * 1000.0 >= low)
}

fn load_prior(root: Option<&String>, out: &Path) -> Result<Prior> {
    let Some(root) = root else {
        return Ok(Prior::default());
    };
    let root = PathBuf::from(root);
    let mut gaps = Vec::new();
    let mut samples = Vec::new();
    let mut sessions = Vec::new();
    let mut cursor = Some(root.clone());
    while let Some(session) = cursor.take() {
        ensure!(
            sessions.len() < 16 && !sessions.contains(&session),
            "cyclic or excessive prior-session chain"
        );
        let conditions: Value =
            serde_json::from_slice(&fs::read(session.join("conditions.json"))?)?;
        cursor = conditions["args"]
            .get(8)
            .and_then(Value::as_str)
            .map(PathBuf::from);
        let mut times = Vec::new();
        // A stopped sampler is not a suspend: detect gaps only inside each
        // actual session, never across program shutdown/restart boundaries.
        for line in fs::read_to_string(session.join("nvidia-smi.csv"))?
            .lines()
            .skip(1)
        {
            if let Some(time) = line.split(',').next().and_then(|v| v.parse::<u64>().ok()) {
                times.push(time);
                samples.push((time, line.to_owned()));
            }
        }
        gaps.extend(session_gaps(&times));
        sessions.push(session);
    }
    samples.sort();
    samples.dedup();
    let mut csv = String::from(
        "unix_ms,timestamp,utilization_gpu_pct,memory_used_MiB,temperature_C,power_W\n",
    );
    for (time, line) in samples {
        let _ = time;
        csv.push_str(&line);
        csv.push('\n');
    }
    fs::write(out.join("prior-nvidia-smi.csv"), csv)?;
    fs::copy(
        root.join("conditions.json"),
        out.join("prior-conditions.json"),
    )?;
    if root.join("resume-audit.json").exists() {
        fs::copy(
            root.join("resume-audit.json"),
            out.join("prior-resume-audit.json"),
        )?;
    }
    let mut groups = BTreeMap::<PairKey, Vec<Value>>::new();
    for row in records(root.join("runs.jsonl"))? {
        let key = (
            row["id"].as_str().context("prior id")?.to_owned(),
            row["budget_s"].as_f64().context("prior budget")?.to_bits(),
        );
        groups.entry(key).or_default().push(row);
    }
    let mut prior = Prior {
        root: Some(root.clone()),
        ..Default::default()
    };
    let mut discarded = Vec::new();
    for (key, rows) in groups {
        let complete = rows.len() == 2
            && rows.iter().any(|r| r["variant"] == "plain")
            && rows.iter().any(|r| r["variant"] == "gpu_warm");
        let suspended = rows.iter().any(|r| overlaps(r, &gaps));
        if !complete || suspended {
            discarded.push(json!({ "id": key.0, "budget_s": f64::from_bits(key.1), "reason": if suspended { "run overlapped a telemetry gap >5 seconds; discard both variants" } else { "incomplete pair" }, "runs": rows }));
        } else {
            prior.pairs.insert(key, rows);
        }
    }
    for (name, map) in [
        ("cpu-fallback.jsonl", &mut prior.cpu),
        ("fill-gaps.jsonl", &mut prior.fills),
    ] {
        for row in records(root.join(name))? {
            map.insert(row["id"].as_str().context("prior id")?.to_owned(), row);
        }
    }
    save(
        out.join("resume-audit.json"),
        &json!({ "reused_pairs": prior.pairs.len(), "telemetry_sessions": sessions, "telemetry_gaps_unix_ms": gaps, "discarded_pairs": discarded }),
    )?;
    Ok(prior)
}

fn session_gaps(times: &[u64]) -> Vec<(f64, f64)> {
    times
        .windows(2)
        .filter(|pair| pair[1].saturating_sub(pair[0]) > 5000)
        .map(|pair| (pair[0] as f64, pair[1] as f64))
        .collect()
}

fn copy_run(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        ensure!(
            entry.file_type()?.is_file(),
            "unexpected directory in prior run"
        );
        fs::copy(entry.path(), to.join(entry.file_name()))?;
    }
    Ok(())
}

fn included(job_index: usize, id: &str, budget: f64, sixty_limit: Option<usize>) -> bool {
    budget != 60.0 || sixty_limit.is_none_or(|n| id.starts_with("lectra-") && job_index < n)
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    ensure!(
        args.len() >= 4,
        "usage: gpu-bench probe <job.json> <out> | paired <development.jsonl> <out> [limit=20] [budgets=10,30,60] [cell=2] [60-job-limit] [prior-evidence]"
    );
    let mode = &args[1];
    let out = PathBuf::from(&args[3]);
    fs::create_dir_all(&out)?;
    ensure!(
        !out.join("runs.jsonl").exists(),
        "output already contains runs; choose a fresh directory"
    );
    let cell_mm = args
        .get(if mode == "probe" { 4 } else { 6 })
        .map(|v| v.parse())
        .transpose()?
        .unwrap_or(2.0);
    let mut gpu = Proposer::new(GpuOptions {
        cell_mm,
        backend: BackendChoice::GpuRequired,
        ..Default::default()
    })?;
    save(out.join("device.json"), gpu.device_stats())?;
    let (stop, receive) = std::sync::mpsc::channel();
    let samples = out.join("nvidia-smi.csv");
    let sampler = std::thread::spawn(move || -> Result<()> {
        let mut file = File::create(samples)?;
        writeln!(
            file,
            "unix_ms,timestamp,utilization_gpu_pct,memory_used_MiB,temperature_C,power_W"
        )?;
        loop {
            let unix_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_millis();
            match command("nvidia-smi")
                .args([
                    "--id=0",
                    "--query-gpu=timestamp,utilization.gpu,memory.used,temperature.gpu,power.draw",
                    "--format=csv,noheader,nounits",
                ])
                .output()
            {
                Ok(output) => {
                    write!(file, "{unix_ms},")?;
                    file.write_all(&output.stdout)?;
                    file.flush()?;
                }
                Err(error) => writeln!(file, "sample_error,{error}")?,
            }
            if receive.recv_timeout(Duration::from_millis(500)).is_ok() {
                break;
            }
        }
        Ok(())
    });
    let revision = command("git").args(["rev-parse", "HEAD"]).output()?;
    save(
        out.join("conditions.json"),
        &json!({
            "revision": String::from_utf8_lossy(&revision.stdout).trim(), "args": args,
            "workers": 2, "seed": 1, "cell_mm": cell_mm,
            "order": "sequential; alternate baseline/warm first by (job_index + budget_index) parity",
            "budget": "warm proposal wall time is subtracted from the search budget; adapter import/validation overhead remains",
            "init": "one reusable device; initialization recorded separately",
            "sampling": "nvidia-smi subprocess, 500 ms pause plus process latency; shared-machine utilisation",
        }),
    )?;
    let work = (|| -> Result<()> {
        let mut cpu = Proposer::new(GpuOptions {
            cell_mm,
            backend: BackendChoice::Cpu,
            ..Default::default()
        })?;
        if mode == "probe" {
            let job: Job = serde_json::from_slice(&fs::read(&args[2])?)?;
            let started = Instant::now();
            let proposal = gpu.construct(&job)?;
            save(out.join("proposal.json"), &proposal)?;
            save(out.join("result.json"), &proposal.result)?;
            save(
                out.join("validation.json"),
                &beta_marker_core::validate(&job, &proposal.result),
            )?;
            let cpu = cpu.construct(&job)?;
            save(out.join("cpu-proposal.json"), &cpu)?;
            println!(
                "{} GPU {:.3}s CPU {:.3}s efficiency {:.4} parity {} total {:.3}s",
                job.name,
                proposal.stats.proposal_s,
                cpu.stats.proposal_s,
                proposal.result.efficiency,
                proposal.result.placements == cpu.result.placements,
                started.elapsed().as_secs_f64()
            );
            return Ok(());
        }
        ensure!(mode == "paired", "unknown mode");
        let limit: usize = args.get(4).map(|v| v.parse()).transpose()?.unwrap_or(20);
        let sixty_limit: Option<usize> = args.get(7).map(|v| v.parse()).transpose()?;
        let budgets: Vec<f64> = args
            .get(5)
            .map(String::as_str)
            .unwrap_or("10,30,60")
            .split(',')
            .map(str::parse)
            .collect::<std::result::Result<_, _>>()?;
        ensure!(
            budgets.iter().all(|b| b.is_finite() && *b > 0.0),
            "invalid budgets"
        );
        let file = BufReader::new(File::open(&args[2])?);
        let mut jobs = Vec::new();
        for line in file.lines().take(limit) {
            let record: Value = serde_json::from_str(&line?)?;
            let job: Job = serde_json::from_value(record["job"].clone())?;
            jobs.push((
                format!(
                    "lectra-{}",
                    record["id"].as_str().context("missing task id")?
                ),
                job,
            ));
        }
        // CWD is marker/, as required by the shared brief.
        for style in ["simon", "teagan", "titan"] {
            let job: Job = serde_json::from_slice(&fs::read(format!(
                "testdata/open/freesewing/{style}-ratio.job.json"
            ))?)?;
            jobs.push((style.into(), job));
        }
        save(out.join("manifest.json"), &jobs.iter().enumerate().map(|(i, (id, job))| json!({ "id": id, "job_name": job.name, "copies": beta_marker_core::demands(job).iter().map(|d| u64::from(d.count)).sum::<u64>(), "budgets_s": budgets.iter().copied().filter(|&b| included(i, id, b, sixty_limit)).collect::<Vec<_>>() })).collect::<Vec<_>>())?;
        let prior = load_prior(args.get(8), &out)?;
        let mut log = File::create(out.join("runs.jsonl"))?;
        let mut fallback = File::create(out.join("cpu-fallback.jsonl"))?;
        let mut fills = File::create(out.join("fill-gaps.jsonl"))?;
        fs::create_dir_all(out.join("jobs"))?;
        for (job_index, (id, job)) in jobs.iter().enumerate() {
            save(out.join("jobs").join(format!("{id}.json")), job)?;
            if let Some(root) = &prior.root
                && prior.cpu.contains_key(id)
            {
                let old = fs::read(root.join("jobs").join(format!("{id}.json")))?;
                // Re-parsing serialized f64 values can differ by one ULP.
                // Bind reuse to deterministic bytes from the original input.
                ensure!(
                    old == serde_json::to_vec_pretty(job)?,
                    "prior geometry/rules changed for {id}"
                );
            }
            let start = Instant::now();
            let record = if let Some(record) = prior.cpu.get(id) {
                record.clone()
            } else {
                match cpu.construct(job) {
                    Ok(p) => {
                        json!({ "id": id, "ok": true, "stats": p.stats, "efficiency": p.result.efficiency, "valid": beta_marker_core::validate(job, &p.result).ok })
                    }
                    Err(e) => {
                        json!({ "id": id, "ok": false, "wall_s": start.elapsed().as_secs_f64(), "error": format!("{e:#}") })
                    }
                }
            };
            writeln!(fallback, "{record}")?;
            fallback.flush()?;
            for (budget_index, &budget) in budgets.iter().enumerate() {
                if !included(job_index, id, budget, sixty_limit) {
                    continue;
                }
                let last_budget = budgets
                    .iter()
                    .copied()
                    .rfind(|&b| included(job_index, id, b, sixty_limit))
                    .context("empty job budgets")?;
                if let Some(rows) = prior.pairs.get(&(id.clone(), budget.to_bits())) {
                    let root = prior.root.as_ref().context("prior root")?;
                    for row in rows {
                        let variant = row["variant"].as_str().context("prior variant")?;
                        let dirname = format!("{id}-{budget}-{variant}");
                        copy_run(&root.join(&dirname), &out.join(&dirname))?;
                        writeln!(log, "{row}")?;
                    }
                    log.flush()?;
                    if budget == last_budget
                        && let Some(record) = prior.fills.get(id)
                    {
                        writeln!(fills, "{record}")?;
                        fills.flush()?;
                    }
                    println!("{id} {budget}s pair reused; timestamps clear of standby");
                    continue;
                }
                let warm_first = (job_index + budget_index) % 2 == 1;
                for warm in if warm_first {
                    [true, false]
                } else {
                    [false, true]
                } {
                    let variant = if warm { "gpu_warm" } else { "plain" };
                    let dir = out.join(format!("{id}-{budget}-{variant}"));
                    fs::create_dir_all(&dir)?;
                    let unix_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)?
                        .as_millis();
                    let started = Instant::now();
                    let mut options = NestOptions {
                        time_limit_s: budget,
                        seed: Some(1),
                        workers: Some(2),
                        ..Default::default()
                    };
                    let mut proposal_stats = Value::Null;
                    let mut proposal_error = Value::Null;
                    if warm {
                        match gpu.construct(job) {
                            Ok(proposal) => {
                                save(dir.join("proposal.json"), &proposal)?;
                                proposal_stats = serde_json::to_value(&proposal.stats)?;
                                options.time_limit_s =
                                    (budget - started.elapsed().as_secs_f64()).max(0.001);
                                options.warm_start = Some(proposal.result);
                            }
                            Err(error) => {
                                proposal_error = json!(format!("{error:#}"));
                                options.time_limit_s =
                                    (budget - started.elapsed().as_secs_f64()).max(0.001);
                            }
                        }
                    }
                    let proposal_wall_s = if warm {
                        started.elapsed().as_secs_f64()
                    } else {
                        0.0
                    };
                    let result = run(job, &options);
                    let mut record = json!({ "id": id, "budget_s": budget, "variant": variant, "warm_first": warm_first, "seed": 1, "workers": 2, "search_s": options.time_limit_s, "wall_s": started.elapsed().as_secs_f64(), "start_unix_ms": unix_ms, "proposal_wall_s": proposal_wall_s, "proposal_stats": proposal_stats, "proposal_error": proposal_error });
                    match result {
                        Ok(result) => {
                            let validation = beta_marker_core::validate(job, &result);
                            save(dir.join("result.json"), &result)?;
                            save(dir.join("validation.json"), &validation)?;
                            record["ok"] = json!(true);
                            record["valid"] = json!(validation.ok);
                            record["efficiency"] = json!(result.efficiency);
                            record["length"] = json!(result.length);
                            if !warm && budget == last_budget {
                                let fill_started = Instant::now();
                                let fill = gpu.fill_gaps(job, &result, 12);
                                let fill_record = match fill {
                                    Ok(p) => {
                                        save(dir.join("fill-result.json"), &p)?;
                                        json!({ "id": id, "budget_s": budget, "ok": true, "before_efficiency": result.efficiency, "after_efficiency": p.result.efficiency, "stats": p.stats, "valid": beta_marker_core::validate(job, &p.result).ok })
                                    }
                                    Err(e) => {
                                        json!({ "id": id, "budget_s": budget, "ok": false, "wall_s": fill_started.elapsed().as_secs_f64(), "error": format!("{e:#}") })
                                    }
                                };
                                writeln!(fills, "{fill_record}")?;
                                fills.flush()?;
                            }
                        }
                        Err(error) => {
                            record["ok"] = json!(false);
                            record["error"] = json!(format!("{error:#}"));
                        }
                    }
                    save(dir.join("run.json"), &record)?;
                    writeln!(log, "{record}")?;
                    log.flush()?;
                    println!(
                        "{id} {budget}s {variant} ok={} eff={} wall={:.3}",
                        record["ok"],
                        record["efficiency"],
                        record["wall_s"].as_f64().unwrap_or_default()
                    );
                }
            }
        }
        Ok(())
    })();
    let _ = stop.send(());
    sampler
        .join()
        .map_err(|_| anyhow::anyhow!("sampler thread failed"))??;
    work
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standby_overlap_discards_entire_run_window() {
        let row = json!({ "start_unix_ms": 1000, "wall_s": 60.0 });
        assert!(overlaps(&row, &[(2000.0, 5000.0)]));
        assert!(!overlaps(&row, &[(62_000.0, 90_000.0)]));
        assert!(overlaps(&json!({}), &[]));
    }

    #[test]
    fn sampler_restart_boundaries_are_not_standby() {
        let mut gaps = session_gaps(&[1000, 2000]);
        gaps.extend(session_gaps(&[100_000, 101_000, 200_000]));
        assert_eq!(gaps, vec![(101_000.0, 200_000.0)]);
    }

    #[test]
    fn revised_plan_keeps_short_budgets_and_limits_sixty_to_lectra() {
        assert!(included(19, "lectra-last", 30.0, Some(10)));
        assert!(included(9, "lectra-tenth", 60.0, Some(10)));
        assert!(!included(10, "lectra-eleventh", 60.0, Some(10)));
        assert!(!included(2, "simon", 60.0, Some(10)));
        assert!(included(20, "simon", 60.0, None));
    }
}
