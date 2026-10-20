//! Offline public-suite orchestration and geometry-free evidence export.
use anyhow::{Context, Result, bail, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[path = "../../crates/beta-marker-cli/src/cmd/bench/csv.rs"]
#[allow(dead_code)]
mod csv;

#[derive(Parser)]
#[command(about = "Run or summarize the fixed public BETA benchmark. See README.md.")]
struct Args {
    #[arg(long)]
    out: PathBuf,
    #[arg(long)]
    executable: Option<PathBuf>,
    #[arg(long, env = "BETA_DATA_DIR")]
    data_dir: PathBuf,
    #[arg(long)]
    input_dir: Option<PathBuf>,
    #[arg(long)]
    jobs: Option<usize>,
    #[arg(long)]
    workers: Option<usize>,
    #[arg(long, default_value_t = 1.0)]
    time_scale: f64,
    #[arg(long)]
    quick: bool,
    #[arg(long, default_value = "all", value_parser = ["all", "extras"])]
    suite: String,
    #[arg(long)]
    summarize: bool,
    #[arg(long)]
    allow_partial: bool,
    #[arg(long)]
    machine: Option<String>,
}

#[derive(Clone, Debug)]
struct Row {
    id: String,
    seed: u64,
    valid: bool,
    errors: String,
    stratum: String,
    tier: String,
    beta: Option<f64>,
    reference: Option<f64>,
    difference: Option<f64>,
    saving: Option<f64>,
    runtime: f64,
    validation: f64,
    budget: f64,
    start: Option<f64>,
    end: Option<f64>,
}

fn sha(path: &Path) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
}
fn read_json(path: &Path) -> Result<Value> {
    serde_json::from_slice(&fs::read(path)?).with_context(|| format!("read {}", path.display()))
}
fn write_json(path: &Path, value: &Value) -> Result<()> {
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(value)?))?;
    Ok(())
}
fn workspace_version(text: &str) -> Option<String> {
    text.split("[workspace.package]")
        .nth(1)?
        .lines()
        .take_while(|line| !line.starts_with('['))
        .find_map(|line| {
            let (key, value) = line.split_once('=')?;
            (key.trim() == "version").then(|| value.trim().trim_matches('"').to_owned())
        })
}
fn run(command: &mut Command) -> Result<()> {
    let status = command
        .status()
        .with_context(|| format!("start {command:?}"))?;
    ensure!(status.success(), "command failed ({status}): {command:?}");
    Ok(())
}
fn ids(path: &Path) -> Result<Vec<String>> {
    fs::read_to_string(path)?
        .lines()
        .filter(|s| !s.trim().is_empty())
        .map(|s| {
            Ok(serde_json::from_str::<Value>(s)?["id"]
                .as_str()
                .context("string task ID")?
                .to_owned())
        })
        .collect()
}
fn check_inputs(marker: &Path, data: &Path, input: &Path, extras: bool) -> Result<Value> {
    let manifest = read_json(&marker.join("bench/public/inputs.json"))?;
    let mut hashes = BTreeMap::new();
    for item in manifest["academic"]
        .as_array()
        .context("academic input manifest")?
    {
        let id = item["id"].as_str().unwrap();
        let path = data.join(format!("datasets/esicup-sparrow/{id}.json"));
        let hash = sha(&path).with_context(|| {
            format!("prepare public academic input {id}; see bench/public/README.md")
        })?;
        ensure!(
            Some(hash.as_str()) == item["sha256"].as_str(),
            "academic input hash mismatch: {id}"
        );
        hashes.insert(format!("{id}.json"), hash);
    }
    if !extras {
        for name in ["sample-300", "sample-match-60"] {
            let path = input.join(format!("{name}.jsonl"));
            ensure!(
                ids(&path)? == ids(&marker.join(format!("bench/public/{name}.ids.jsonl")))?,
                "{name} task IDs/order differ from the frozen suite"
            );
            let hash = sha(&path)?;
            let key = if name == "sample-300" {
                "sample_300_sha256"
            } else {
                "sample_match_60_sha256"
            };
            ensure!(
                Some(hash.as_str()) == manifest["lectra"][key].as_str(),
                "converted input hash mismatch for {name}; use the pinned converter and frozen IDs"
            );
            hashes.insert(format!("{name}.jsonl"), hash);
        }
    }
    for name in ["simon", "teagan", "titan"] {
        let name = format!("{name}-ratio.job.json");
        hashes.insert(
            name.clone(),
            sha(&marker.join("testdata/open/freesewing").join(name))?,
        );
    }
    Ok(json!(hashes))
}
fn launch(args: &Args, marker: &Path, data: &Path, out: &Path) -> Result<()> {
    ensure!(
        !(args.quick && args.suite == "extras"),
        "--quick is a Lectra smoke; it cannot be combined with --suite extras"
    );
    ensure!(
        args.time_scale.is_finite() && args.time_scale > 0.0,
        "time scale must be finite and positive"
    );
    ensure!(
        !out.exists(),
        "use a fresh output directory, or --summarize an existing run"
    );
    let cpus = std::thread::available_parallelism()?.get();
    let workers = args.workers.unwrap_or(cpus.min(3));
    ensure!(workers > 0 && workers <= cpus, "invalid worker count");
    let jobs = args.jobs.unwrap_or((cpus / workers).min(4));
    ensure!(
        jobs > 0 && jobs <= cpus / workers,
        "jobs x workers exceeds logical CPUs"
    );
    let input = args
        .input_dir
        .clone()
        .unwrap_or_else(|| data.join("marker-bench/public-inputs"));
    let input = if input.is_absolute() {
        input
    } else {
        std::env::current_dir()?.join(input)
    };
    let hashes = check_inputs(marker, data, &input, args.suite == "extras")?;
    let executable = if let Some(path) = &args.executable {
        fs::canonicalize(path).context("benchmark executable")?
    } else {
        run(Command::new("cargo").current_dir(marker).args([
            "build",
            "--release",
            "--locked",
            "-p",
            "beta-marker-cli",
        ]))?;
        let target = std::env::var_os("CARGO_TARGET_DIR")
            .context("set CARGO_TARGET_DIR outside the checkout")?;
        let target = PathBuf::from(target);
        let target = if target.is_absolute() {
            target
        } else {
            marker.join(target)
        };
        fs::canonicalize(target.join("release").join(if cfg!(windows) {
            "beta-marker.exe"
        } else {
            "beta-marker"
        }))?
    };
    fs::create_dir_all(out)?;
    write_json(
        &out.join("public-run.json"),
        &json!({"schema":"beta-public-run/1", "quick":args.quick,
        "time_scale":args.time_scale, "suite":args.suite,"jobs":jobs,"workers":workers,
        "input_hashes":hashes,"executable_sha256":sha(&executable)?,"machine":args.machine,
        "beta_version":workspace_version(&fs::read_to_string(marker.join("Cargo.toml"))?)}),
    )?;
    let base = if args.quick { 30.0 } else { 60.0 } * args.time_scale;
    let mut stages: Vec<(String, Vec<String>)> = vec![];
    let search = marker.join("bench/experiments/v03/selected.json");
    if args.suite != "extras" {
        stages.push((
            "lectra-60s".into(),
            vec![
                "lectra".into(),
                "--input".into(),
                input.join("sample-300.jsonl").display().to_string(),
                "--time".into(),
                base.to_string(),
                "--limit".into(),
                if args.quick { "20" } else { "300" }.into(),
                "--seed".into(),
                "1".into(),
                "--search-config".into(),
                search.display().to_string(),
                "--runs".into(),
                "1".into(),
            ],
        ));
        if !args.quick {
            let mut matched = input.join("sample-match-60.jsonl");
            if args.time_scale != 1.0 {
                let text = fs::read_to_string(&matched)?;
                let scaled: Result<Vec<String>> = text
                    .lines()
                    .filter(|s| !s.trim().is_empty())
                    .map(|line| {
                        let mut row: Value = serde_json::from_str(line)?;
                        row["lectra_duration_s"] = json!(
                            row["lectra_duration_s"].as_f64().context("duration")?
                                * args.time_scale
                        );
                        Ok(serde_json::to_string(&row)?)
                    })
                    .collect();
                matched = out.join("scaled-match.jsonl");
                fs::write(&matched, scaled?.join("\n") + "\n")?;
            }
            stages.push((
                "lectra-match".into(),
                vec![
                    "lectra".into(),
                    "--input".into(),
                    matched.display().to_string(),
                    "--time".into(),
                    "match".into(),
                    "--limit".into(),
                    "60".into(),
                    "--seed".into(),
                    "1".into(),
                    "--search-config".into(),
                    search.display().to_string(),
                    "--runs".into(),
                    "1".into(),
                ],
            ));
        }
    }
    if !args.quick {
        stages.push((
            "academic-60s".into(),
            vec![
                "academic".into(),
                "--time".into(),
                base.to_string(),
                "--seeds".into(),
                "1,2,3".into(),
                "--search-config".into(),
                search.display().to_string(),
                "--runs".into(),
                "1".into(),
            ],
        ));
        let garments = ["simon", "teagan", "titan"]
            .map(|id| {
                marker
                    .join(format!("testdata/open/freesewing/{id}-ratio.job.json"))
                    .display()
                    .to_string()
            })
            .join(",");
        for seconds in [60, 300] {
            stages.push((
                format!("freesewing-{seconds}s"),
                vec![
                    "jobs".into(),
                    "--inputs".into(),
                    garments.clone(),
                    "--time".into(),
                    (f64::from(seconds) * args.time_scale).to_string(),
                    "--seed".into(),
                    "1".into(),
                ],
            ));
        }
    }
    for (name, arguments) in stages {
        println!("Public benchmark stage: {name}");
        run(Command::new(&executable)
            .current_dir(marker)
            .env("BETA_DATA_DIR", data)
            .arg("bench")
            .args(arguments)
            .args([
                "--jobs",
                &jobs.to_string(),
                "--threads-per-task",
                &workers.to_string(),
                "--out",
            ])
            .arg(out.join(name)))?;
    }
    Ok(())
}

fn rows(path: &Path) -> Result<Vec<Row>> {
    let records = csv::records(&fs::read_to_string(path.join("results.csv"))?)?;
    let header = records.first().context("empty CSV")?;
    let mut rows = Vec::new();
    for record in records.iter().skip(1) {
        ensure!(
            record.len() == header.len(),
            "incomplete CSV row; collect again after the writer flushes"
        );
        let get = |key: &str| -> Result<&str> {
            Ok(record[header
                .iter()
                .position(|s| s == key)
                .with_context(|| format!("missing column {key}"))?]
            .as_str())
        };
        let num = |key| -> Result<Option<f64>> {
            let value = get(key)?;
            if value.is_empty() {
                return Ok(None);
            }
            let number: f64 = value.parse()?;
            ensure!(number.is_finite(), "nonfinite {key}");
            Ok(Some(number))
        };
        let valid = match get("valid")? {
            "true" => true,
            "false" => false,
            _ => bail!("invalid CSV boolean"),
        };
        let row = Row {
            id: get("id")?.into(),
            seed: get("seed")?.parse()?,
            valid,
            errors: get("errors")?.into(),
            stratum: get("stratum")?.into(),
            // Archived round-2 CSVs predate the tier column. Comparisons need
            // their scores, while a new Lectra export requires explicit tiers.
            tier: header
                .iter()
                .position(|s| s == "tier")
                .map(|i| record[i].clone())
                .unwrap_or_default(),
            beta: num("our_efficiency")?,
            reference: num("reference_efficiency")?,
            difference: num("difference_pp")?,
            saving: num("relative_length_saving")?,
            runtime: num("runtime_s")?.context("x_runtime")?,
            validation: num("validation_s")?.context("validation runtime")?,
            budget: num("budget_s")?.context("budget")?,
            start: header
                .iter()
                .position(|s| s == "started_unix_ms")
                .and_then(|i| record[i].parse().ok()),
            end: header
                .iter()
                .position(|s| s == "finished_unix_ms")
                .and_then(|i| record[i].parse().ok()),
        };
        if row.valid {
            ensure!(
                row.beta.is_some_and(|v| v > 0.0 && v <= 100.0 + 1e-6),
                "invalid scored efficiency"
            );
            if let Some(reference) = row.reference {
                ensure!(
                    row.difference
                        .is_some_and(|d| (d - (row.beta.unwrap() - reference)).abs() < 1e-6),
                    "difference disagrees with efficiencies"
                );
                ensure!(
                    row.saving
                        .is_some_and(|s| (s - (1.0 - reference / row.beta.unwrap())).abs() < 1e-6),
                    "saving disagrees with efficiencies"
                );
            }
        } else {
            ensure!(
                row.beta.is_none() && row.difference.is_none() && row.saving.is_none(),
                "failed trial has numeric score"
            );
        }
        rows.push(row);
    }
    let unique: BTreeSet<_> = rows.iter().map(|r| (&r.id, r.seed)).collect();
    ensure!(unique.len() == rows.len(), "duplicate trial ID/seed");
    Ok(rows)
}
fn quantile(values: &[f64], fraction: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let index = (sorted.len() - 1) as f64 * fraction;
    let low = index.floor() as usize;
    let high = index.ceil() as usize;
    Some(sorted[low] + (sorted[high] - sorted[low]) * (index - low as f64))
}
fn mean(values: &[f64]) -> Option<f64> {
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}
fn group(label: &str, rows: &[&Row], headline: bool) -> Value {
    let valid: Vec<_> = rows.iter().filter(|r| r.valid).collect();
    let differences: Vec<_> = valid.iter().filter_map(|r| r.difference).collect();
    let savings: Vec<_> = valid
        .iter()
        .filter_map(|r| r.saving.map(|v| 100.0 * v))
        .collect();
    let too_long = rows
        .iter()
        .filter(|r| !r.valid && r.errors.contains("too_long"))
        .count();
    let comparable = !differences.is_empty();
    json!({"label":label,"headline":headline,"attempts":rows.len(),"valid":valid.len(),
        "mean_diff_pp":mean(&differences),"median_diff_pp":quantile(&differences,0.5),
        "win":comparable.then(||differences.iter().filter(|&&v|v>0.1+1e-9).count()),
        "tie":comparable.then(||differences.iter().filter(|&&v|v.abs()<=0.1+1e-9).count()),
        "loss":comparable.then(||differences.iter().filter(|&&v|v< -0.1-1e-9).count()),
        "too_long_losses":too_long,"other_failures":rows.len()-valid.len()-too_long,
        "mean_length_saving_pct":mean(&savings),"median_length_saving_pct":quantile(&savings,0.5),
        "mean_beta_eff_pct":mean(&valid.iter().filter_map(|r|r.beta).collect::<Vec<_>>()),
        "mean_reference_eff_pct":mean(&valid.iter().filter_map(|r|r.reference).collect::<Vec<_>>()),
        "task_ids":rows.iter().map(|r|&r.id).collect::<BTreeSet<_>>(),
        "x_runtime":{"mean_s":mean(&rows.iter().map(|r|r.runtime).collect::<Vec<_>>())}})
}
fn pearson(x: &[f64], y: &[f64]) -> Option<f64> {
    if x.len() < 2 || x.len() != y.len() {
        return None;
    }
    let (mx, my) = (mean(x)?, mean(y)?);
    let cross = x
        .iter()
        .zip(y)
        .map(|(a, b)| (a - mx) * (b - my))
        .sum::<f64>();
    let divisor = (x.iter().map(|a| (a - mx).powi(2)).sum::<f64>()
        * y.iter().map(|b| (b - my).powi(2)).sum::<f64>())
    .sqrt();
    (divisor > 0.0).then(|| (cross / divisor).clamp(-1.0, 1.0))
}
fn ranks(values: &[f64]) -> Vec<f64> {
    let mut indices: Vec<_> = (0..values.len()).collect();
    indices.sort_by(|&a, &b| values[a].total_cmp(&values[b]));
    let mut out = vec![0.0; values.len()];
    let mut i = 0;
    while i < indices.len() {
        let mut j = i + 1;
        while j < indices.len() && values[indices[j]] == values[indices[i]] {
            j += 1
        }
        for &index in &indices[i..j] {
            out[index] = (i + j - 1) as f64 / 2.0 + 1.0
        }
        i = j;
    }
    out
}
fn validate_evidence(path: &Path, config: &Value, rows: &[Row]) -> Result<Value> {
    let tasks = config["tasks"].as_array().context("run tasks")?;
    let mut reports = 0;
    let mut missing_failed = 0;
    for row in rows {
        let index = tasks
            .iter()
            .position(|t| t["id"].as_str() == Some(&row.id) && t["seed"].as_u64() == Some(row.seed))
            .context("CSV trial absent from run.json")?;
        let report = path.join(format!("trials/{index:05}/validation.json"));
        if report.exists() {
            let report = read_json(&report)?;
            reports += 1;
            ensure!(
                report["ok"].as_bool() == Some(row.valid),
                "CSV and validator disagree for {} seed {}",
                row.id,
                row.seed
            );
            if row.valid {
                ensure!(
                    report["errors"].as_array().is_some_and(|v| v.is_empty()),
                    "valid report contains errors"
                );
            }
        } else {
            ensure!(
                !row.valid,
                "missing independent validator report for scored task {}",
                row.id
            );
            missing_failed += 1;
        }
    }
    Ok(
        json!({"reports_checked":reports,"valid":rows.iter().filter(|r|r.valid).count(),"failed":rows.iter().filter(|r|!r.valid).count(),"failed_before_validator":missing_failed}),
    )
}
fn date(unix_ms: f64) -> String {
    // Gregorian civil date from days since Unix epoch (400-year eras).
    let z = (unix_ms / 86_400_000.0).floor() as i64 + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    y += if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}")
}
fn summarize_stage(
    path: &Path,
    id: &str,
    kind: &str,
    expected: usize,
    args: &Args,
    public: &Value,
) -> Result<Value> {
    let config = read_json(&path.join("run.json"))?;
    let rows = rows(path)?;
    ensure!(!rows.is_empty(), "no completed trials");
    let tasks = config["tasks"].as_array().context("tasks")?;
    ensure!(
        tasks.len() == expected,
        "unexpected planned trial count for {id}: {} instead of {expected}",
        tasks.len()
    );
    if kind == "lectra" {
        ensure!(
            rows.iter().all(|r| r.tier == "A" || r.tier == "B"),
            "new Lectra exports require explicit A/B tier evidence"
        );
        let source = if id == "lectra-matched" {
            "sample-match-60"
        } else {
            "sample-300"
        };
        let frozen =
            ids(&Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("{source}.ids.jsonl")))?;
        let planned: Vec<_> = tasks
            .iter()
            .map(|t| t["id"].as_str().unwrap_or("").to_owned())
            .collect();
        ensure!(
            planned == frozen[..expected],
            "Lectra task IDs/order differ from the frozen suite"
        );
        ensure!(
            tasks.iter().all(|t| t["seed"] == 1),
            "Lectra trials must use seed 1"
        );
    }
    ensure!(rows.len() <= expected, "too many trials");
    let complete = rows.len() == expected && path.join("summary.md").exists();
    ensure!(
        complete || args.allow_partial,
        "{id} is incomplete; use --allow-partial for explicitly provisional output"
    );
    let validation = validate_evidence(path, &config, &rows)?;
    let all: Vec<_> = rows.iter().collect();
    let mut groups = vec![group(
        if kind == "lectra" {
            "Tier B (includes Tier A)"
        } else {
            "All"
        },
        &all,
        true,
    )];
    if kind == "lectra" {
        groups.push(group(
            "Tier A",
            &all.iter()
                .copied()
                .filter(|r| r.tier == "A")
                .collect::<Vec<_>>(),
            false,
        ));
        groups.push(group(
            "Tier B only",
            &all.iter()
                .copied()
                .filter(|r| r.tier == "B")
                .collect::<Vec<_>>(),
            false,
        ));
    }
    if id == "lectra-matched" {
        let fixed = path.parent().context("run root")?.join("lectra-60s");
        if fixed.join("results.csv").exists() {
            let before = crate::rows(&fixed)?;
            let selected: BTreeSet<_> = all.iter().map(|r| r.id.as_str()).collect();
            let subset: Vec<_> = before
                .iter()
                .filter(|r| selected.contains(r.id.as_str()))
                .collect();
            if let Some(first) = subset.first() {
                groups.push(group(
                    &format!("Same tasks at {} s", first.budget),
                    &subset,
                    false,
                ));
            }
        }
    }
    let strata: BTreeSet<_> = rows
        .iter()
        .map(|r| {
            if kind == "freesewing" {
                r.id.as_str()
            } else {
                r.stratum.as_str()
            }
        })
        .collect();
    let strata: Vec<_> = strata
        .into_iter()
        .map(|label| {
            group(
                label,
                &all.iter()
                    .copied()
                    .filter(|r| {
                        if kind == "freesewing" {
                            r.id == label
                        } else {
                            r.stratum == label
                        }
                    })
                    .collect::<Vec<_>>(),
                false,
            )
        })
        .collect();
    let budgets: Vec<_> = rows.iter().map(|r| r.budget).collect();
    let fixed = budgets.iter().all(|v| (v - budgets[0]).abs() < 1e-8);
    let runtime: Vec<_> = rows.iter().map(|r| r.runtime).collect();
    let start = rows.iter().filter_map(|r| r.start).min_by(f64::total_cmp);
    let end = rows.iter().filter_map(|r| r.end).max_by(f64::total_cmp);
    let summary = fs::read_to_string(path.join("summary.md")).unwrap_or_default();
    let wall = summary
        .split("wall time ")
        .nth(1)
        .or_else(|| summary.split("Wall time ").nth(1))
        .and_then(|s| s.split_whitespace().next())
        .and_then(|s| s.parse::<f64>().ok());
    let revision = config["revision"].as_str().unwrap_or("unknown");
    let beta_version = public["beta_version"]
        .as_str()
        .map(str::to_owned)
        .or_else(|| {
            let marker = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
            let output = Command::new("git")
                .arg("-C")
                .arg(marker)
                .args(["show", &format!("{revision}:marker/Cargo.toml")])
                .output()
                .ok()?;
            if !output.status.success() {
                return None;
            }
            workspace_version(&String::from_utf8(output.stdout).ok()?)
        })
        .unwrap_or_else(|| "unrecorded".into());
    let machine = args
        .machine
        .clone()
        .or_else(|| public["machine"].as_str().map(str::to_owned))
        .unwrap_or_else(|| {
            format!(
                "Shared {} / {} machine, {} logical CPUs; {}",
                config["os"].as_str().unwrap_or("unknown"),
                config["arch"].as_str().unwrap_or("unknown"),
                config["logical_cpus"],
                config["processor"]
                    .as_str()
                    .unwrap_or("CPU model unrecorded")
            )
        });
    let reference = match kind {
        "lectra" => {
            json!({"name":"Lectra recorded efficiency","type":"commercial-engine","citation":"Lallier, Vezard, Pinaud and Blin (2022), Nesting Tasks Dataset for 2D-Nesting Efficiency Estimation, version 1.1","url":"https://doi.org/10.5281/zenodo.7030786","licence":"CC BY 4.0"})
        }
        "academic" => {
            json!({"name":"Sparrow paper means, 100 runs at 1200 s","type":"published-paper","url":"https://arxiv.org/html/2509.13329v4"})
        }
        _ => {
            json!({"name":"BETA results only","type":"none","licence":"MIT (FreeSewing fixtures)"})
        }
    };
    let mut assumptions = vec![
        "Search budgets exclude construction, repair and final validation overhead.".to_owned(),
        "Shared hardware and wall-clock cutoffs can change results despite fixed seeds.".into(),
    ];
    if kind == "lectra" {
        assumptions.extend(["Sheet type 0 is assumed to be an ordinary open rectangular strip; no verified codebook is available.",
        "flip_x is assumed to reflect the x coordinate before rotation.","Tier B includes Tier A and retains a subset of permitted orientations; unsupported constraints stay excluded.",
        "The balanced stratified sample is unweighted and does not estimate population performance.","Lectra hardware, engine version, worker count and stopping semantics are unknown."].map(str::to_owned));
    }
    if kind == "academic" {
        assumptions.push("BETA uses Sparrow itself. Published references use 1200 seconds, 100 runs and Ryzen 9 7950X hardware; this is an adapter and budget comparison.".into());
    }
    if kind == "freesewing" {
        assumptions.push("Generated tailoring drafts on open 1500 mm fabric, S1 M2 L2 XL1, no nap and zero gap. No commercial reference is available.".into());
    }
    let altered =
        public["quick"] == true || public["time_scale"].as_f64().is_some_and(|v| v != 1.0);
    let summary_text = if altered {
        "Reduced-budget pipeline run. Do not use as a launch comparison.".to_owned()
    } else if kind == "lectra" && groups[0]["mean_diff_pp"].is_number() {
        format!(
            "BETA minus Lectra: mean {:+.4} pp; median {:+.4} pp. {} wins, {} ties and {} losses. Equal seconds do not imply equal compute.",
            groups[0]["mean_diff_pp"].as_f64().unwrap_or(0.0),
            groups[0]["median_diff_pp"].as_f64().unwrap_or(0.0),
            groups[0]["win"],
            groups[0]["tie"],
            groups[0]["loss"]
        )
    } else {
        "One fixed suite, with every scored marker independently validated. See the assumptions before comparing results.".into()
    };
    let order = match id {
        "lectra-60s" => 10,
        "lectra-matched" => 20,
        "academic-60s" => 50,
        "freesewing-60s" => 60,
        _ => 70,
    };
    let export_id = if altered {
        format!("smoke-{id}")
    } else {
        id.into()
    };
    let mut value = json!({"schema":"beta-site-benchmark/1","id":export_id,"order":order,"title":format!("{}: {}",match kind {"lectra"=>"Lectra dataset","academic"=>"Academic instances",_=>"FreeSewing"},if fixed {format!("{} seconds",budgets[0])}else{"recorded Lectra time".into()}),
        "kind":kind,"status":if complete {"final"}else{"provisional"},"date":date(end.unwrap_or_else(||std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as f64)),
        "summary_text":summary_text,
        "software":{"beta_version":beta_version,"engine_revision":revision,"engine":"sparrow 0.3.0 on jagua-rs 1.0.0 (unmodified)"},
        "reference":reference,"conditions":{"budget_label":if fixed {format!("{} s",budgets[0])}else{format!("Recorded Lectra duration, {}-{} s",quantile(&budgets,0.0).unwrap(),quantile(&budgets,1.0).unwrap())},
            "budget_s":if fixed {Some(budgets[0])}else{None},"machine":machine,"jobs":config["jobs"],"workers_per_task":config["threads_per_task"],
            "seeds":rows.iter().map(|r|r.seed).collect::<BTreeSet<_>>(),"background_load":"Shared machine; see archived load.log and conditions.json. No claim of exclusive CPU use.",
            "notes":"Equal seconds do not imply equal compute. Runtime measures the whole nesting call and evidence I/O; validation is also timed separately."},
        "assumptions":assumptions,"groups":groups,"strata":strata,"task_ids":rows.iter().map(|r|&r.id).collect::<BTreeSet<_>>(),
        "x_runtime":{"sum_s":runtime.iter().sum::<f64>(),"mean_s":mean(&runtime),"median_s":quantile(&runtime,0.5),"max_s":quantile(&runtime,1.0),"validation_sum_s":rows.iter().map(|r|r.validation).sum::<f64>(),
            "budget_min_s":quantile(&budgets,0.0),"budget_median_s":quantile(&budgets,0.5),"budget_max_s":quantile(&budgets,1.0),"started_unix_ms":start,"finished_unix_ms":end,"runner_wall_s":wall},
        "x_validation":validation,"x_provenance":{"run_json_sha256":sha(&path.join("run.json"))?,"results_csv_sha256":sha(&path.join("results.csv"))?,"source_dirty":config["dirty"],"search":config["search"],"inputs":public["input_hashes"]},
        "reproduce":{"commands":["cargo run --release --manifest-path bench/public/Cargo.toml -- --out <fresh-output-directory>"],"doc":"marker/bench/public/README.md"},
        "sources":[{"label":"Method and results","url":"marker/bench/results/2026-10-20-public.md"}]});
    if !complete {
        value["progress"] = json!({"completed":rows.len(),"planned":expected});
        value["status_note"] =
            json!("Run is incomplete. Completion order can bias these provisional numbers.");
    }
    let pairs: Vec<_> = rows
        .iter()
        .filter(|r| r.valid && r.reference.is_some())
        .collect();
    if !pairs.is_empty() {
        let x: Vec<_> = pairs.iter().map(|r| r.beta.unwrap()).collect();
        let y: Vec<_> = pairs.iter().map(|r| r.reference.unwrap()).collect();
        if let (Some(pearson), Some(spearman)) = (pearson(&x, &y), pearson(&ranks(&x), &ranks(&y)))
        {
            value["correlation"] =
                json!({"pairs":pairs.len(),"pearson":pearson,"spearman":spearman});
        }
        let d: Vec<_> = pairs.iter().map(|r| r.difference.unwrap()).collect();
        value["quantiles_pp"] = json!({"min":quantile(&d,0.0),"p05":quantile(&d,0.05),"p50":quantile(&d,0.5),"p95":quantile(&d,0.95),"max":quantile(&d,1.0)});
    }
    Ok(value)
}
fn fmt(value: &Value) -> String {
    value
        .as_f64()
        .map(|v| format!("{v:.4}"))
        .unwrap_or_else(|| "not measured".into())
}
fn paired_changes(label: &str, before: &[Row], after: &[Row]) -> Result<Value> {
    let lookup: BTreeMap<_, _> = before.iter().map(|r| ((&r.id, r.seed), r)).collect();
    let mut changes = vec![];
    let mut savings = vec![];
    let mut ids = vec![];
    let mut eligible = 0;
    for row in after {
        if let Some(old) = lookup.get(&(&row.id, row.seed)) {
            eligible += 1;
            ensure!(
                row.reference == old.reference,
                "reference differs in paired comparison for {}",
                row.id
            );
            if row.valid && old.valid {
                changes.push(row.beta.unwrap() - old.beta.unwrap());
                savings.push(100.0 * (1.0 - old.beta.unwrap() / row.beta.unwrap()));
                ids.push(row.id.clone());
            }
        }
    }
    Ok(
        json!({"label":label,"shared_trials":eligible,"valid_pairs":changes.len(),"mean_efficiency_change_pp":mean(&changes),
        "median_efficiency_change_pp":quantile(&changes,0.5),"improved":changes.iter().filter(|&&v|v>0.1+1e-9).count(),
        "tied":changes.iter().filter(|&&v|v.abs()<=0.1+1e-9).count(),"regressed":changes.iter().filter(|&&v|v< -0.1-1e-9).count(),
        "mean_length_saving_pct":mean(&savings),"median_length_saving_pct":quantile(&savings,0.5),"task_ids":ids,
        "caveat":"Shared-machine wall-time results. Hardware, conversion, engine changes and scheduling are confounded; this is not an isolated speedup measurement."}),
    )
}
fn comparisons(out: &Path, data: &Path) -> Result<Value> {
    let mut result = Vec::new();
    let fixed = out.join("lectra-60s");
    let matched = out.join("lectra-match");
    if fixed.join("results.csv").exists() && matched.join("results.csv").exists() {
        result.push(paired_changes(
            "Same tasks: matched duration minus fixed budget",
            &rows(&fixed)?,
            &rows(&matched)?,
        )?);
    }
    for (new, old) in [
        ("lectra-60s", "2026-10-18-lectra-round2-60s"),
        ("lectra-match", "2026-10-18-lectra-round2-match"),
        ("academic-60s", "2026-10-18-academic-round2-60s"),
    ] {
        let before = data.join("marker-bench/results").join(old);
        let after = out.join(new);
        if before.join("results.csv").exists() && after.join("results.csv").exists() {
            result.push(paired_changes(
                &format!("{new}: current run minus round-2 laptop"),
                &rows(&before)?,
                &rows(&after)?,
            )?);
        }
    }
    let short = out.join("freesewing-60s");
    let long = out.join("freesewing-300s");
    if short.join("results.csv").exists() && long.join("results.csv").exists() {
        result.push(paired_changes(
            "FreeSewing: 300 seconds minus 60 seconds",
            &rows(&short)?,
            &rows(&long)?,
        )?);
    }
    Ok(json!({"schema":"beta-public-paired-analysis/1","comparisons":result}))
}
fn load_summary(path: &Path) -> Result<Value> {
    if !path.exists() {
        return Ok(Value::Null);
    }
    let text = fs::read_to_string(path)?;
    let samples: Vec<Vec<f64>> = text
        .lines()
        .filter_map(|line| line.split("load average:").nth(1))
        .map(|tail| {
            tail.split(',')
                .filter_map(|v| v.trim().parse().ok())
                .collect()
        })
        .filter(|s: &Vec<f64>| s.len() == 3)
        .collect();
    let groups:Vec<_>=(0..3).map(|index|{let values:Vec<_>=samples.iter().map(|s|s[index]).collect();json!({"window_minutes":([1,5,15][index]),"mean":mean(&values),"median":quantile(&values,0.5),"min":quantile(&values,0.0),"max":quantile(&values,1.0)})}).collect();
    Ok(
        json!({"samples":samples.len(),"load_averages":groups,"sha256":sha(path)?,"note":"Linux load averages count runnable and uninterruptible tasks. They are not CPU utilization percentages and do not measure per-process CPU time."}),
    )
}
fn summarize(args: &Args, out: &Path) -> Result<()> {
    let public = if out.join("public-run.json").exists() {
        read_json(&out.join("public-run.json"))?
    } else {
        Value::Null
    };
    let stages = [
        (
            "lectra-60s",
            "lectra-60s",
            "lectra",
            if public["quick"] == true { 20 } else { 300 },
        ),
        ("lectra-match", "lectra-matched", "lectra", 60),
        ("academic-60s", "academic-60s", "academic", 21),
        ("freesewing-60s", "freesewing-60s", "freesewing", 3),
        ("freesewing-300s", "freesewing-300s", "freesewing", 3),
    ];
    let mut exports = vec![];
    for (folder, id, kind, expected) in stages {
        if !out.join(folder).exists() {
            let optional = args.allow_partial
                || (public["quick"] == true && folder != "lectra-60s")
                || (public["suite"] == "extras" && kind == "lectra");
            ensure!(optional, "missing stage {folder}");
            continue;
        }
        if args.allow_partial {
            let csv = out.join(folder).join("results.csv");
            if !csv.exists() || csv::records(&fs::read_to_string(csv)?)?.len() <= 1 {
                continue;
            }
        }
        exports.push(summarize_stage(
            &out.join(folder),
            id,
            kind,
            expected,
            args,
            &public,
        )?);
    }
    ensure!(!exports.is_empty(), "no evidence stages found");
    let paired = comparisons(out, &args.data_dir)?;
    let load = load_summary(&out.join("load.log"))?;
    fs::create_dir_all(out.join("site"))?;
    let mut lines=vec!["# Public benchmark analysis".into(),"".into(),"Differences are BETA minus reference, in percentage points. Numeric summaries use valid markers only. Tier B includes Tier A. too_long failures count as additional losses; other failures stay unscored.".into(),"".into()];
    for value in &exports {
        write_json(
            &out.join("site")
                .join(format!("{}.json", value["id"].as_str().unwrap())),
            value,
        )?;
        lines.extend([format!("## {} ({})",value["title"].as_str().unwrap(),value["status"].as_str().unwrap()),"".into(),
            "| Group | Attempts / valid | Mean pp | Median pp | Win / tie / loss (including too_long) | Mean length saving % | Median length saving % |".into(),"|---|---:|---:|---:|---|---:|---:|".into()]);
        for g in value["groups"]
            .as_array()
            .unwrap()
            .iter()
            .chain(value["strata"].as_array().unwrap())
        {
            let outcomes = if g["win"].is_null() {
                "not compared".into()
            } else {
                format!(
                    "{} / {} / {}",
                    g["win"],
                    g["tie"],
                    g["loss"].as_u64().unwrap() + g["too_long_losses"].as_u64().unwrap()
                )
            };
            lines.push(format!(
                "| {} | {} / {} | {} | {} | {} | {} | {} |",
                g["label"].as_str().unwrap(),
                g["attempts"],
                g["valid"],
                fmt(&g["mean_diff_pp"]),
                fmt(&g["median_diff_pp"]),
                outcomes,
                fmt(&g["mean_length_saving_pct"]),
                fmt(&g["median_length_saving_pct"])
            ));
        }
        lines.extend(["".into(),format!("Machine: {}. {} jobs, {} workers each. Whole-call sum {} s, median {} s, maximum {} s. Validation sum {} s. Runner wall {} s.",
            value["conditions"]["machine"].as_str().unwrap(),value["conditions"]["jobs"],value["conditions"]["workers_per_task"],fmt(&value["x_runtime"]["sum_s"]),fmt(&value["x_runtime"]["median_s"]),fmt(&value["x_runtime"]["max_s"]),fmt(&value["x_runtime"]["validation_sum_s"]),fmt(&value["x_runtime"]["runner_wall_s"])),
            format!("Independent validation: {}. Correlation: {}. Difference quantiles: {}.",value["x_validation"],value["correlation"],value["quantiles_pp"]),"".into()]);
    }
    write_json(&out.join("paired-analysis.json"), &paired)?;
    write_json(&out.join("load-summary.json"), &load)?;
    lines.extend([
        "## Paired changes".into(),
        "".into(),
        format!("```json\n{}\n```", serde_json::to_string_pretty(&paired)?),
        "".into(),
        "## Recorded host load".into(),
        "".into(),
        format!("```json\n{}\n```", serde_json::to_string_pretty(&load)?),
    ]);
    fs::write(out.join("analysis.md"), lines.join("\n"))?;
    println!("Wrote {} site exports and analysis.md", exports.len());
    Ok(())
}
fn main() -> Result<()> {
    let args = Args::parse();
    let marker = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    let cwd = std::env::current_dir()?;
    let out = if args.out.is_absolute() {
        args.out.clone()
    } else {
        cwd.join(&args.out)
    };
    let data = if args.data_dir.is_absolute() {
        args.data_dir.clone()
    } else {
        cwd.join(&args.data_dir)
    };
    if !args.summarize {
        launch(&args, &marker, &data, &out)?;
    }
    summarize(&args, &out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn row(id: &str, d: f64) -> Row {
        Row {
            id: id.into(),
            seed: 1,
            valid: true,
            errors: "[]".into(),
            stratum: "s".into(),
            tier: "A".into(),
            beta: Some(80.0 + d),
            reference: Some(80.0),
            difference: Some(d),
            saving: Some(1.0 - 80.0 / (80.0 + d)),
            runtime: 61.0,
            validation: 0.1,
            budget: 60.0,
            start: None,
            end: None,
        }
    }
    #[test]
    fn ties_failures_and_length_savings_have_the_declared_denominators() {
        let a = row("a", 0.1);
        let b = row("b", -0.1);
        let c = row("c", 1.0);
        let mut failed = row("d", 0.0);
        failed.valid = false;
        failed.errors = "too_long".into();
        failed.beta = None;
        failed.difference = None;
        failed.saving = None;
        let g = group("all", &[&a, &b, &c, &failed], true);
        assert_eq!(g["attempts"], 4);
        assert_eq!(g["valid"], 3);
        assert_eq!(g["tie"], 2);
        assert_eq!(g["win"], 1);
        assert_eq!(g["loss"], 0);
        assert_eq!(g["too_long_losses"], 1);
        assert!((g["mean_diff_pp"].as_f64().unwrap() - 1.0 / 3.0).abs() < 1e-10);
        assert!(
            (g["mean_length_saving_pct"].as_f64().unwrap()
                - 100.0 * (a.saving.unwrap() + b.saving.unwrap() + c.saving.unwrap()) / 3.0)
                .abs()
                < 1e-10
        );
    }
    #[test]
    fn correlations_use_average_ranks_and_null_for_constants() {
        assert_eq!(ranks(&[4.0, 1.0, 1.0, 2.0]), vec![4.0, 1.5, 1.5, 3.0]);
        assert!(pearson(&[1.0, 1.0], &[2.0, 3.0]).is_none());
        assert!((pearson(&[1.0, 2.0, 3.0], &[6.0, 4.0, 2.0]).unwrap() + 1.0).abs() < 1e-12);
        assert_eq!(quantile(&[1.0, 3.0], 0.5), Some(2.0));
    }
    #[test]
    fn dates_are_utc_and_handle_leap_days() {
        assert_eq!(date(0.0), "1970-01-01");
        assert_eq!(date(951782400000.0), "2000-02-29");
    }
    #[test]
    fn a_csv_score_cannot_bypass_its_saved_independent_validator() -> Result<()> {
        let data = PathBuf::from(
            std::env::var_os("BETA_DATA_DIR").context("set BETA_DATA_DIR for evidence tests")?,
        );
        let path = data.join("marker-bench/public-tests").join(format!(
            "evidence-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        fs::create_dir_all(path.join("trials/00000"))?;
        let row = row("x", 1.0);
        let config = json!({"tasks":[{"id":"x","seed":1}]});
        assert!(validate_evidence(&path, &config, std::slice::from_ref(&row)).is_err());
        write_json(
            &path.join("trials/00000/validation.json"),
            &json!({"ok":false,"errors":[]}),
        )?;
        assert!(validate_evidence(&path, &config, std::slice::from_ref(&row)).is_err());
        write_json(
            &path.join("trials/00000/validation.json"),
            &json!({"ok":true,"errors":["overlap"]}),
        )?;
        assert!(validate_evidence(&path, &config, std::slice::from_ref(&row)).is_err());
        write_json(
            &path.join("trials/00000/validation.json"),
            &json!({"ok":true,"errors":[]}),
        )?;
        assert_eq!(
            validate_evidence(&path, &config, &[row])?["reports_checked"],
            1
        );
        Ok(())
    }
}
