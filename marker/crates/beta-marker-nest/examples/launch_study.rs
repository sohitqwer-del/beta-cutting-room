//! Isolated paired rule experiments. Geometry and raw evidence stay outside git.
use anyhow::{Context, Result, bail, ensure};
use beta_marker_core::{Job, Layout, MarkerResult};
use beta_marker_nest::{
    NestOptions, PortfolioStrategy, SearchTuning, nest_with_progress, nest_with_tuning,
};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
#[path = "launch_study/fold.rs"]
mod fold_study;

fn save(path: impl AsRef<Path>, value: &impl Serialize) -> Result<()> {
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Trial {
    valid: bool,
    wall_s: f64,
    length_m: Option<f64>,
    occupied_pct: Option<f64>,
    useful_pct: Option<f64>,
    surplus_area: Option<f64>,
    error: Option<String>,
}

fn trial(job_path: &Path, options_path: &Path, out: &Path) -> Result<()> {
    fs::create_dir_all(out)?;
    let job: Job = serde_json::from_reader(File::open(job_path)?)?;
    let raw_options: serde_json::Value = serde_json::from_reader(File::open(options_path)?)?;
    let options: NestOptions = serde_json::from_value(raw_options.clone())?;
    save(out.join("job.json"), &job)?;
    save(out.join("options.json"), &raw_options)?;
    let started = Instant::now();
    let mut events = Vec::new();
    let outcome = if raw_options["study_sequential_allocations"].as_bool() == Some(true) {
        nest_with_tuning(
            &job,
            &options,
            &SearchTuning {
                portfolio: PortfolioStrategy::Sequential,
                ..Default::default()
            },
        )
    } else {
        nest_with_progress(&job, &options, |event| {
            eprintln!("progress {:.6}s {:.6}mm", event.seconds, event.length);
            events.push(event);
        })
    };
    let wall_s = started.elapsed().as_secs_f64();
    let summary = match outcome {
        Ok(result) => {
            let report = beta_marker_core::validate(&job, &result);
            save(out.join("result.json"), &result)?;
            save(out.join("validation.json"), &report)?;
            let effective = beta_marker_core::rules::effective_job(&job);
            let layers = if job.fabric.layout == Layout::Open {
                1.0
            } else {
                2.0
            };
            let useful: f64 = beta_marker_core::demands(&effective)
                .iter()
                .map(|d| {
                    beta_marker_core::geometry::area(&effective.pieces[d.piece_index].outline)
                        * f64::from(d.count)
                        / layers
                })
                .sum();
            Trial {
                valid: report.ok,
                wall_s,
                length_m: report.ok.then_some(result.length / 1000.0),
                occupied_pct: report.ok.then_some(result.efficiency * 100.0),
                useful_pct: report
                    .ok
                    .then_some(100.0 * useful / (job.fabric.width * result.length)),
                surplus_area: report.ok.then_some((result.piece_area - useful).max(0.0)),
                error: (!report.ok).then(|| format!("{:?}", report.errors)),
            }
        }
        Err(error) => Trial {
            valid: false,
            wall_s,
            length_m: None,
            occupied_pct: None,
            useful_pct: None,
            surplus_area: None,
            error: Some(format!("{error:#}")),
        },
    };
    save(out.join("progress.json"), &events)?;
    save(out.join("summary.json"), &summary)?;
    Ok(())
}

fn isolated(exe: &Path, job: &Path, options: &Path, out: &Path, cap: f64) -> Result<Trial> {
    fs::create_dir_all(out)?;
    let started = Instant::now();
    let mut command = Command::new(exe);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let mut child = command
        .arg("trial")
        .arg(job)
        .arg(options)
        .arg(out)
        .stdout(Stdio::from(File::create(out.join("stdout.log"))?))
        .stderr(Stdio::from(File::create(out.join("stderr.log"))?))
        .spawn()?;
    let error = loop {
        if let Some(status) = child.try_wait()? {
            break (!status.success()).then(|| format!("child exited {status}"));
        }
        if started.elapsed().as_secs_f64() > cap {
            child.kill()?;
            child.wait()?;
            break Some(format!("wall timeout after {cap}s"));
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let mut result: Trial = if error.is_none() && out.join("summary.json").exists() {
        serde_json::from_reader(File::open(out.join("summary.json"))?)?
    } else {
        Trial {
            valid: false,
            wall_s: 0.0,
            length_m: None,
            occupied_pct: None,
            useful_pct: None,
            surplus_area: None,
            error,
        }
    };
    result.wall_s = started.elapsed().as_secs_f64();
    if result.valid {
        let input: Job = serde_json::from_reader(File::open(job)?)?;
        let marker: MarkerResult = serde_json::from_reader(File::open(out.join("result.json"))?)?;
        let check = beta_marker_core::validate(&input, &marker);
        save(out.join("parent-validation.json"), &check)?;
        ensure!(check.ok, "parent validation failed for {}", out.display());
    }
    save(out.join("supervised.json"), &result)?;
    Ok(result)
}

#[derive(Clone)]
struct Pair {
    id: String,
    group: String,
    scenario: String,
    control: Job,
    variant: Job,
    reverse: bool,
}

#[derive(Serialize, Deserialize)]
struct PairResult {
    id: String,
    group: String,
    scenario: String,
    variant_first: bool,
    control: Trial,
    variant: Trial,
    gain_pp: Option<f64>,
    saved_m: Option<f64>,
}

fn paired(exe: &Path, pair: &Pair, options: &Path, out: &Path, cap: f64) -> Result<()> {
    let dir = out.join(format!("{}-{}", pair.id, pair.scenario));
    fs::create_dir_all(&dir)?;
    save(dir.join("control-job.json"), &pair.control)?;
    save(dir.join("variant-job.json"), &pair.variant)?;
    let mut options_value: serde_json::Value = serde_json::from_reader(File::open(options)?)?;
    if pair.scenario == "flip" {
        options_value["runs"] = 5.into();
        options_value["study_sequential_allocations"] = true.into();
    }
    let pair_options = dir.join("options.json");
    save(&pair_options, &options_value)?;
    let run = |arm: &str| {
        isolated(
            exe,
            &dir.join(format!("{arm}-job.json")),
            &pair_options,
            &dir.join(arm),
            cap,
        )
    };
    let (control, variant) = if pair.reverse {
        let variant = run("variant")?;
        (run("control")?, variant)
    } else {
        let control = run("control")?;
        (control, run("variant")?)
    };
    let gain_pp = control
        .useful_pct
        .zip(variant.useful_pct)
        .map(|(a, b)| b - a);
    let saved_m = control.length_m.zip(variant.length_m).map(|(a, b)| a - b);
    save(
        dir.join("pair.json"),
        &PairResult {
            id: pair.id.clone(),
            group: pair.group.clone(),
            scenario: pair.scenario.clone(),
            variant_first: pair.reverse,
            control,
            variant,
            gain_pp,
            saved_m,
        },
    )?;
    eprintln!(
        "finished {} {} gain_pp={gain_pp:?} saved_m={saved_m:?}",
        pair.id, pair.scenario
    );
    Ok(())
}

fn folded(job: &Job, layout: Layout) -> Result<Job> {
    let mut job = job.clone();
    job.fabric.layout = layout;
    job.fabric.width /= 2.0;
    if job.name.to_lowercase().contains("teagan") {
        for p in &mut job.pieces {
            if matches!(p.name.as_str(), "front" | "back") {
                let b = beta_marker_core::geometry::bbox(&p.outline).context("empty torso")?;
                ensure!(
                    (b[0] + b[2]).abs() < 0.001,
                    "torso is not symmetric about x=0"
                );
                p.fold_line = Some([[0.0, b[1]], [0.0, b[3]]]);
            }
        }
    }
    Ok(job)
}

fn study(dev: &Path, fixtures: &Path, out: &Path, seconds: f64, concurrency: usize) -> Result<()> {
    ensure!(!out.exists(), "use a fresh output directory");
    ensure!(
        seconds.is_finite() && seconds > 0.0,
        "seconds must be finite and positive"
    );
    ensure!((1..=3).contains(&concurrency), "concurrency must be 1..3");
    fs::create_dir_all(out)?;
    let options = NestOptions {
        time_limit_s: seconds,
        seed: Some(1),
        workers: Some(2),
        ..Default::default()
    };
    let options_path = out.join("options.json");
    save(&options_path, &options)?;
    let mut inputs = Vec::new();
    for name in ["simon", "teagan", "titan"] {
        let job: Job =
            serde_json::from_reader(File::open(fixtures.join(format!("{name}-ratio.job.json")))?)?;
        inputs.push((name.to_owned(), "FreeSewing".to_owned(), job));
    }
    for line in BufReader::new(File::open(dev)?).lines().take(20) {
        let v: serde_json::Value = serde_json::from_str(&line?)?;
        inputs.push((
            v["id"].as_str().context("missing task id")?.to_owned(),
            "Lectra-dev".to_owned(),
            serde_json::from_value(v["job"].clone())?,
        ));
    }
    ensure!(
        inputs.len() == 23,
        "expected three garments and 20 development tasks"
    );
    ensure!(
        inputs.iter().all(|(id, _, _)| !id.is_empty()
            && id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))),
        "task ids must be safe directory names"
    );
    save(
        out.join("protocol.json"),
        &serde_json::json!({
            "seconds": seconds, "seed": 1, "workers_per_trial": 2, "concurrent_pairs": concurrency,
            "task_ids": inputs.iter().map(|(id, _, _)| id).collect::<Vec<_>>(),
            "selection": "first 20 rows of frozen development-40.jsonl; failures retained",
            "controls": "fresh adjacent control per contrast; alternating arm order; per-piece overrides explicitly set",
            "flip_schedule": "five sequential allocations on both arms, sharing 60s; other contrasts use one run; production defaults unchanged",
            "interpretation": "Hypothetical permissions, not comparisons with Lectra achieved efficiencies. Tubular uses half open width and two layers; material eligibility is not known for Lectra or Simon. Useful demand yield excludes surplus cuts.",
            "cap_seconds": seconds + 60.0,
        }),
    )?;
    let mut pairs = Vec::new();
    for (index, (id, group, source)) in inputs.into_iter().enumerate() {
        for (s, scenario) in ["tilt1", "tilt2", "flip", "tubular"]
            .into_iter()
            .enumerate()
        {
            let mut control = source.clone();
            let mut variant = source.clone();
            match scenario {
                "tilt1" | "tilt2" => {
                    let tilt = if scenario == "tilt1" { 1.0 } else { 2.0 };
                    control.fabric.tilt_deg = 0.0;
                    variant.fabric.tilt_deg = tilt;
                    for p in &mut control.pieces {
                        p.tilt_deg = Some(0.0);
                    }
                    for p in &mut variant.pieces {
                        p.tilt_deg = Some(tilt);
                    }
                }
                "flip" => {
                    control.fabric.allow_flip = false;
                    variant.fabric.allow_flip = true;
                    for p in &mut control.pieces {
                        p.flip = Some(false);
                    }
                    for p in &mut variant.pieces {
                        p.flip = Some(true);
                    }
                }
                "tubular" => {
                    variant = folded(&source, Layout::Tubular)?;
                }
                _ => unreachable!(),
            }
            pairs.push(Pair {
                id: id.clone(),
                group: group.clone(),
                scenario: scenario.into(),
                control,
                variant,
                reverse: (index + s) % 2 != 0,
            });
        }
    }
    let next = AtomicUsize::new(0);
    let exe = std::env::current_exe()?;
    std::thread::scope(|scope| -> Result<()> {
        let mut handles = Vec::new();
        for _ in 0..concurrency {
            handles.push(scope.spawn(|| -> Result<()> {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(pair) = pairs.get(i) else { break };
                    paired(&exe, pair, &options_path, out, seconds + 60.0)?;
                }
                Ok(())
            }));
        }
        for handle in handles {
            handle
                .join()
                .map_err(|_| anyhow::anyhow!("study worker panicked"))??;
        }
        Ok(())
    })?;
    summarize(out)
}

fn summarize(out: &Path) -> Result<()> {
    summarize_with_replays(out, &[])
}

fn summarize_with_replays(out: &Path, replays: &[PathBuf]) -> Result<()> {
    let mut rows: Vec<PairResult> = Vec::new();
    for dir in fs::read_dir(out)? {
        let path = dir?.path().join("pair.json");
        if path.exists() {
            rows.push(serde_json::from_reader(File::open(path)?)?);
        }
    }
    let mut replacements = Vec::new();
    for root in replays {
        for dir in fs::read_dir(root)? {
            let path = dir?.path().join("pair.json");
            if !path.exists() {
                continue;
            }
            let replacement: PairResult = serde_json::from_reader(File::open(&path)?)?;
            let old = rows
                .iter_mut()
                .find(|r| {
                    r.id == replacement.id
                        && r.group == replacement.group
                        && r.scenario == replacement.scenario
                })
                .context("replay has no matching original pair")?;
            ensure!(
                old.variant_first == replacement.variant_first,
                "replay changed arm order"
            );
            replacements.push(serde_json::json!({"id": replacement.id, "scenario": replacement.scenario,
                "replacement": path, "reason": "whole-pair replay after recorded host timing interruption; original raw records retained"}));
            *old = replacement;
        }
    }
    save(out.join("replacements.json"), &replacements)?;
    rows.sort_by(|a, b| (&a.group, &a.id, &a.scenario).cmp(&(&b.group, &b.id, &b.scenario)));
    let mut csv = String::from(
        "group,id,scenario,variant_first,control_valid,variant_valid,control_useful_pct,variant_useful_pct,gain_pp,saved_m,control_wall_s,variant_wall_s\n",
    );
    let number = |n: Option<f64>| n.map(|n| format!("{n:.6}")).unwrap_or_default();
    for p in &rows {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{:.6},{:.6}\n",
            p.group,
            p.id,
            p.scenario,
            p.variant_first,
            p.control.valid,
            p.variant.valid,
            number(p.control.useful_pct),
            number(p.variant.useful_pct),
            number(p.gain_pp),
            number(p.saved_m),
            p.control.wall_s,
            p.variant.wall_s
        ));
    }
    fs::write(out.join("pairs.csv"), csv)?;
    let mut report = String::from(
        "| Group | Freedom | Pairs valid / attempted | Mean gain pp | Mean saved m | W / T / L | Failed control / variant | Mean wall control / variant s |\n|---|---|---:|---:|---:|---|---|---|\n",
    );
    let mut aggregates = Vec::new();
    for group in ["FreeSewing", "Lectra-dev"] {
        for scenario in ["tilt1", "tilt2", "flip", "tubular"] {
            let subset: Vec<_> = rows
                .iter()
                .filter(|p| p.group == group && p.scenario == scenario)
                .collect();
            let good: Vec<_> = subset.iter().filter(|p| p.gain_pp.is_some()).collect();
            let mean = |v: Vec<f64>| {
                if v.is_empty() {
                    f64::NAN
                } else {
                    v.iter().sum::<f64>() / v.len() as f64
                }
            };
            let (mut w, mut t, mut l) = (0, 0, 0);
            for p in &subset {
                match p.gain_pp {
                    Some(g) if g > 0.1 => w += 1,
                    Some(g) if g < -0.1 => l += 1,
                    Some(_) => t += 1,
                    None if p.control.valid => l += 1,
                    None if p.variant.valid => w += 1,
                    _ => {}
                }
            }
            let gain = mean(good.iter().filter_map(|p| p.gain_pp).collect());
            let saving = mean(good.iter().filter_map(|p| p.saved_m).collect());
            let failed_control = subset.iter().filter(|p| !p.control.valid).count();
            let failed_variant = subset.iter().filter(|p| !p.variant.valid).count();
            let wall_control = mean(subset.iter().map(|p| p.control.wall_s).collect());
            let wall_variant = mean(subset.iter().map(|p| p.variant.wall_s).collect());
            report.push_str(&format!("| {group} | {scenario} | {} / {} | {gain:.4} | {saving:.4} | {w} / {t} / {l} | {failed_control} / {failed_variant} | {wall_control:.2} / {wall_variant:.2} |\n", good.len(), subset.len()));
            aggregates.push(serde_json::json!({"group":group,"scenario":scenario,"valid_pairs":good.len(),
                "attempted_pairs":subset.len(),"gain_pp":gain,"saved_m":saving,"wins":w,"ties":t,"losses":l,
                "failed_control":failed_control,"failed_variant":failed_variant,"wall_control_s":wall_control,"wall_variant_s":wall_variant}));
        }
    }
    fs::write(out.join("summary.md"), report)?;
    save(out.join("summary.json"), &aggregates)?;
    save(out.join("selected-pairs.json"), &rows)?;
    save(
        out.join("failures.json"),
        &rows
            .iter()
            .filter(|p| !p.control.valid || !p.variant.valid)
            .collect::<Vec<_>>(),
    )?;
    Ok(())
}

fn replay(exe: &Path, original: &Path, out: &Path) -> Result<()> {
    let old: PairResult = serde_json::from_reader(File::open(original.join("pair.json"))?)?;
    ensure!(
        [&old.id, &old.scenario]
            .iter()
            .all(|value| !value.is_empty()
                && value
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))),
        "replay identifiers must be safe directory names"
    );
    let pair = Pair {
        id: old.id,
        group: old.group,
        scenario: old.scenario,
        reverse: old.variant_first,
        control: serde_json::from_reader(File::open(original.join("control-job.json"))?)?,
        variant: serde_json::from_reader(File::open(original.join("variant-job.json"))?)?,
    };
    ensure!(
        !out.join(format!("{}-{}", pair.id, pair.scenario)).exists(),
        "use a fresh replay directory"
    );
    let options = original.join("options.json");
    let opts: NestOptions = serde_json::from_reader(File::open(&options)?)?;
    paired(exe, &pair, &options, out, opts.time_limit_s + 60.0)
}

fn compare_trials(paths: &[String]) -> Result<()> {
    let mut rows = Vec::new();
    let (mut total_gain, mut total_saved, mut valid) = (0.0, 0.0, 0_usize);
    for pair in paths.as_chunks::<2>().0 {
        let before: Trial =
            serde_json::from_reader(File::open(Path::new(&pair[0]).join("summary.json"))?)?;
        let after: Trial =
            serde_json::from_reader(File::open(Path::new(&pair[1]).join("summary.json"))?)?;
        let gain = before
            .useful_pct
            .zip(after.useful_pct)
            .filter(|_| before.valid && after.valid)
            .map(|(a, b)| b - a);
        let saved = before
            .length_m
            .zip(after.length_m)
            .filter(|_| before.valid && after.valid)
            .map(|(a, b)| a - b);
        if let Some((gain, saved)) = gain.zip(saved) {
            total_gain += gain;
            total_saved += saved;
            valid += 1;
        }
        rows.push(
            serde_json::json!({"before":before,"after":after,"gain_pp":gain,"saved_m":saved}),
        );
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({"rows":rows,"valid_pairs":valid,
        "mean_gain_pp":(valid>0).then(||total_gain/valid as f64),
        "mean_saved_m":(valid>0).then(||total_saved/valid as f64)}))?
    );
    Ok(())
}

fn audit_tilt(path: &Path) -> Result<()> {
    let mut rows = Vec::new();
    for line in BufReader::new(File::open(path)?).lines().take(20) {
        let record: serde_json::Value = serde_json::from_str(&line?)?;
        let job: Job = serde_json::from_value(record["job"].clone())?;
        let (mut restricted, mut continuous) = (0_usize, 0_usize);
        for piece in &job.pieces {
            if beta_marker_core::rules::multiplier(&job, piece) == 0
                || (piece.quantity == 0 && piece.mirror_quantity == 0)
            {
                continue;
            }
            if beta_marker_core::allowed_rotations(&job, piece, false).free {
                continuous += 1;
            } else {
                restricted += 1;
            }
        }
        rows.push(
            serde_json::json!({"id":record["id"],"restricted_piece_types":restricted,
            "continuous_piece_types":continuous,"tilt_changes_permission":restricted>0}),
        );
    }
    println!("{}", serde_json::to_string_pretty(&rows)?);
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("fold-matrix") if args.len() == 8 => fold_study::matrix(
            Path::new(&args[1]), Path::new(&args[2]), Path::new(&args[3]),
            Path::new(&args[4]), Path::new(&args[5]), args[6].parse()?, args[7].parse()?,
        ),
        Some("fold-summary") if args.len() == 3 => {
            fold_study::summarize(Path::new(&args[1]), Path::new(&args[2]))
        }
        Some("trial") if args.len() == 4 => trial(
            Path::new(&args[1]),
            Path::new(&args[2]),
            Path::new(&args[3]),
        ),
        Some("freedoms") if args.len() == 6 => study(
            Path::new(&args[1]),
            Path::new(&args[2]),
            Path::new(&args[3]),
            args[4].parse()?,
            args[5].parse()?,
        ),
        Some("summarize") if args.len() >= 2 => summarize_with_replays(
            Path::new(&args[1]),
            &args[2..].iter().map(PathBuf::from).collect::<Vec<_>>(),
        ),
        Some("replay") if args.len() == 4 => replay(
            Path::new(&args[1]),
            Path::new(&args[2]),
            Path::new(&args[3]),
        ),
        Some("compare") if args.len() >= 3 && args.len() % 2 == 1 => compare_trials(&args[1..]),
        Some("audit-tilt") if args.len() == 2 => audit_tilt(Path::new(&args[1])),
        Some("fold-job") if args.len() == 4 => {
            let input: Job = serde_json::from_reader(File::open(&args[1])?)?;
            let layout = match args[2].as_str() {
                "folded" => Layout::Folded,
                "tubular" => Layout::Tubular,
                _ => bail!("expected folded or tubular"),
            };
            save(PathBuf::from(&args[3]), &folded(&input, layout)?)
        }
        _ => bail!(
            "trial JOB OPTIONS OUT | freedoms DEV_JSONL FIXTURE_DIR OUT SECONDS CONCURRENCY | summarize OUT [REPLAY_ROOT...] | replay ORIGINAL_EXE PAIR_DIR OUT | compare BEFORE AFTER [BEFORE AFTER...] | audit-tilt DEV_JSONL | fold-job JOB folded|tubular OUTPUT | fold-matrix BEFORE AFTER FIXTURES DEV_JSONL OUT SECONDS CONCURRENCY | fold-summary STUDY_ROOT OUT"
        ),
    }
}
