//! Paired, resumable development/confirmation measurements. All geometry and
//! individual results go to an explicitly supplied external output directory.
use anyhow::{Context, Result, ensure};
use beta_marker_cluster::{Strategy, nest_clustered};
use beta_marker_core::{Job, MarkerResult};
use beta_marker_nest::NestOptions;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const WALL_GRACE_S: f64 = 60.0;

#[derive(Clone, Serialize, Deserialize)]
struct Task {
    id: serde_json::Value,
    job: Job,
    #[serde(default)]
    lectra_efficiency: Option<f64>,
    #[serde(default)]
    lectra_duration_s: Option<f64>,
    #[serde(default)]
    stratum: String,
    #[serde(default)]
    comparable: bool,
    #[serde(default)]
    tier: String,
    #[serde(default)]
    sheet_type: u32,
    #[serde(default = "one")]
    reference_area_ratio: f64,
}
fn one() -> f64 {
    1.0
}
fn id(task: &Task) -> String {
    task.id
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| task.id.to_string())
}

#[derive(Clone, Serialize, Deserialize)]
struct Trial {
    #[serde(default)]
    timeout: bool,
    valid: bool,
    efficiency_pct: Option<f64>,
    length_mm: Option<f64>,
    wall_s: f64,
    error: Option<String>,
    warnings: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Pair {
    task_id: String,
    stratum: String,
    strategy: Strategy,
    seed: u64,
    budget_s: f64,
    workers: usize,
    variant_first: bool,
    lectra_pct: Option<f64>,
    reference_area_ratio: f64,
    plain: Trial,
    variant: Trial,
}
#[derive(Serialize, Deserialize, PartialEq)]
struct Manifest {
    #[serde(default)]
    revision: String,
    source: String,
    split: String,
    task_ids: Vec<String>,
    strategies: Vec<Strategy>,
    time_s: f64,
    seed: u64,
    workers: usize,
    jobs: usize,
    sampling_seed: u64,
    #[serde(default)]
    wall_grace_s: f64,
}

#[derive(Serialize, Deserialize)]
struct TrialRequest {
    task: Task,
    options: NestOptions,
}

enum Completion {
    Exited(ExitStatus),
    TimedOut,
}

fn child_command(executable: &Path) -> Command {
    #[cfg(windows)]
    {
        let mut command = Command::new(executable);
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        command
    }
    #[cfg(not(windows))]
    {
        Command::new(executable)
    }
}

fn wait_child(child: &mut Child, started: Instant, cap: Duration) -> std::io::Result<Completion> {
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Completion::Exited(status));
        }
        if started.elapsed() >= cap {
            if let Err(error) = child.kill() {
                if let Some(status) = child.try_wait()? {
                    return Ok(Completion::Exited(status));
                }
                return Err(error);
            }
            child.wait()?;
            return Ok(Completion::TimedOut);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn parse_strategy(text: &str) -> Result<Strategy> {
    Ok(match text {
        "plain" => Strategy::Plain,
        "auto" => Strategy::Auto,
        "race" => Strategy::Race,
        "cluster" => Strategy::Cluster,
        "cluster-contour" => Strategy::ClusterContour,
        "aggregate" => Strategy::Aggregate,
        "restart" => Strategy::Restart,
        "gap-fill" => Strategy::GapFill,
        "gap-fill-fixed" => Strategy::GapFillFixed,
        "two-stage" => Strategy::TwoStage,
        "two-stage-fixed" => Strategy::TwoStageFixed,
        _ => anyhow::bail!("unknown strategy {text}"),
    })
}
fn name(strategy: Strategy) -> &'static str {
    match strategy {
        Strategy::Plain => "plain",
        Strategy::Auto => "auto",
        Strategy::Race => "race",
        Strategy::Cluster => "cluster",
        Strategy::ClusterContour => "cluster-contour",
        Strategy::Aggregate => "aggregate",
        Strategy::Restart => "restart",
        Strategy::GapFill => "gap-fill",
        Strategy::GapFillFixed => "gap-fill-fixed",
        Strategy::TwoStage => "two-stage",
        Strategy::TwoStageFixed => "two-stage-fixed",
    }
}

fn hash_id(id: &str) -> u64 {
    id.bytes().fold(7030786u64, |a, b| {
        (a ^ u64::from(b)).wrapping_mul(0x100000001b3)
    })
}

fn load(input: &Path, split: &str) -> Result<Vec<Task>> {
    if split == "freesewing" {
        return ["simon", "teagan", "titan"]
            .iter()
            .map(|style| {
                let job: Job = serde_json::from_reader(File::open(
                    input.join(format!("{style}-ratio.job.json")),
                )?)?;
                Ok(Task {
                    id: (*style).into(),
                    job,
                    lectra_efficiency: None,
                    lectra_duration_s: None,
                    stratum: "FreeSewing ratio".into(),
                    comparable: true,
                    tier: "open".into(),
                    sheet_type: 0,
                    reference_area_ratio: 1.0,
                })
            })
            .collect();
    }
    let mut groups: BTreeMap<String, Vec<Task>> = BTreeMap::new();
    for line in BufReader::new(File::open(input)?).lines() {
        let task: Task = serde_json::from_str(&line?)?;
        ensure!(
            task.comparable && ["A", "B"].contains(&task.tier.as_str()) && task.sheet_type == 0,
            "input must contain only eligible Tier B (including A), sheet-type-0 tasks"
        );
        groups.entry(task.stratum.clone()).or_default().push(task);
    }
    for tasks in groups.values_mut() {
        tasks.sort_by_key(|t| (hash_id(&id(t)), id(t)));
    }
    let mut round_robin = Vec::new();
    let max = groups.values().map(Vec::len).max().unwrap_or(0);
    for i in 0..max {
        for tasks in groups.values() {
            if let Some(task) = tasks.get(i) {
                round_robin.push(task.clone());
            }
        }
    }
    let (skip, count) = match split {
        "dev" => (0, 40),
        "confirm" => (40, 100),
        "matched" => (40, 20),
        "smoke" => (0, 3),
        _ => anyhow::bail!("split must be dev, confirm, matched, smoke or freesewing"),
    };
    ensure!(
        round_robin.len() >= skip + count,
        "not enough disjoint input tasks"
    );
    Ok(round_robin.into_iter().skip(skip).take(count).collect())
}

fn trial_inline(
    task: &Task,
    options: &NestOptions,
    strategy: Strategy,
    file: &Path,
) -> Result<Trial> {
    let started = Instant::now();
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if strategy == Strategy::Plain {
            beta_marker_nest::nest(&task.job, options)
        } else {
            nest_clustered(&task.job, options, strategy)
        }
    }));
    let result = match outcome {
        Ok(result) => result,
        Err(_) => Err(anyhow::anyhow!("engine panicked; no result scored")),
    };
    match result {
        Ok(result) => {
            let report = beta_marker_core::validate(&task.job, &result);
            serde_json::to_writer(
                File::create(file)?,
                &serde_json::json!({ "result": result, "validation": report }),
            )?;
            if !report.ok {
                return Ok(failure(
                    started,
                    format!("independent validator: {:?}", report.errors),
                ));
            }
            Ok(success(started, &result, task.reference_area_ratio))
        }
        Err(error) => Ok(failure(started, format!("{error:#}"))),
    }
}
fn success(started: Instant, result: &MarkerResult, ratio: f64) -> Trial {
    Trial {
        timeout: false,
        valid: true,
        efficiency_pct: Some(result.efficiency * 100.0 * ratio),
        length_mm: Some(result.length),
        wall_s: started.elapsed().as_secs_f64(),
        error: None,
        warnings: result.warnings.clone(),
    }
}
fn failure(started: Instant, error: String) -> Trial {
    Trial {
        timeout: false,
        valid: false,
        efficiency_pct: None,
        length_mm: None,
        wall_s: started.elapsed().as_secs_f64(),
        error: Some(error),
        warnings: vec![],
    }
}

fn trial(task: &Task, options: &NestOptions, strategy: Strategy, file: &Path) -> Result<Trial> {
    let started = Instant::now();
    let request = file.with_extension("request.json");
    let summary = file.with_extension("trial.json");
    let stderr_path = file.with_extension("stderr.log");
    serde_json::to_writer(
        File::create(&request)?,
        &TrialRequest {
            task: task.clone(),
            options: options.clone(),
        },
    )?;
    let mut command = child_command(&std::env::current_exe()?);
    command
        .arg("--single")
        .arg(&request)
        .arg(name(strategy))
        .arg(file)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::from(File::create(&stderr_path)?));
    let cap = Duration::try_from_secs_f64(options.time_limit_s + WALL_GRACE_S)?;
    let mut child = command
        .spawn()
        .context("could not start isolated nesting trial")?;
    let completion = match wait_child(&mut child, started, cap) {
        Ok(completion) => completion,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error).context("could not supervise isolated trial");
        }
    };
    match completion {
        Completion::TimedOut => {
            let mut failed = failure(
                started,
                format!(
                    "wall-time cap {:.3} s exceeded (search budget {:.3} s plus {:.0} s construction/check grace)",
                    cap.as_secs_f64(),
                    options.time_limit_s,
                    WALL_GRACE_S
                ),
            );
            failed.timeout = true;
            Ok(failed)
        }
        Completion::Exited(status) if !status.success() => Ok(failure(
            started,
            format!(
                "isolated trial exited {status}; inspect {}",
                stderr_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
            ),
        )),
        Completion::Exited(_) => {
            let mut scored: Trial = serde_json::from_reader(
                File::open(&summary).context("isolated trial wrote no summary")?,
            )?;
            scored.wall_s = started.elapsed().as_secs_f64();
            if scored.valid {
                // Validate the original parent input too, including any floating
                // point round-trip difference introduced by process IPC.
                let evidence: serde_json::Value = serde_json::from_reader(File::open(file)?)?;
                let result: MarkerResult = serde_json::from_value(
                    evidence
                        .get("result")
                        .context("missing isolated result")?
                        .clone(),
                )?;
                let report = beta_marker_core::validate(&task.job, &result);
                serde_json::to_writer(
                    File::create(file.with_extension("parent-validation.json"))?,
                    &report,
                )?;
                if !report.ok {
                    return Ok(failure(
                        started,
                        format!("parent independent validation failed: {:?}", report.errors),
                    ));
                }
                scored.efficiency_pct = Some(report.efficiency * 100.0 * task.reference_area_ratio);
                scored.length_mm = Some(report.length);
                scored.wall_s = started.elapsed().as_secs_f64();
            }
            Ok(scored)
        }
    }
}

#[derive(Default)]
struct Stats {
    diffs: Vec<f64>,
    win: usize,
    tie: usize,
    loss: usize,
    failed: usize,
}
impl Stats {
    fn add(&mut self, left: Option<f64>, right: Option<f64>) {
        match (left, right) {
            (Some(a), Some(b)) => {
                let d = a - b;
                self.diffs.push(d);
                if d > 0.1 {
                    self.win += 1;
                } else if d < -0.1 {
                    self.loss += 1;
                } else {
                    self.tie += 1;
                }
            }
            (None, Some(_)) => {
                self.loss += 1;
                self.failed += 1;
            }
            (Some(_), None) => {
                self.win += 1;
                self.failed += 1;
            }
            _ => self.failed += 1,
        }
    }
    fn mean(&self) -> f64 {
        self.diffs.iter().sum::<f64>() / self.diffs.len().max(1) as f64
    }
    fn ci(&self) -> f64 {
        let n = self.diffs.len();
        if n < 2 {
            return 0.0;
        }
        let mean = self.mean();
        1.96 * (self.diffs.iter().map(|d| (d - mean).powi(2)).sum::<f64>()
            / (n - 1) as f64
            / n as f64)
            .sqrt()
    }
}

fn summary(out: &Path, pairs: &[Pair], wall: f64) -> Result<()> {
    let mut grouped: BTreeMap<&str, Vec<&Pair>> = BTreeMap::new();
    for pair in pairs {
        grouped.entry(name(pair.strategy)).or_default().push(pair);
    }
    let mut file = File::create(out.join("summary.md"))?;
    writeln!(
        file,
        "# Paired strategy measurements\n\nCompleted pairs: {}. Runner wall time: {:.2} s. Tie: within 0.1 percentage point.\n\n| Strategy | Valid pairs / attempts | vs plain mean pp | Approx. 95% half-width pp | W / T / L | vs Lectra mean pp | W / T / L | Failures |\n|---|---:|---:|---:|---|---:|---|---:|",
        pairs.len(),
        wall
    )?;
    for (strategy, rows) in grouped {
        let (mut plain, mut lectra) = (Stats::default(), Stats::default());
        for row in &rows {
            plain.add(row.variant.efficiency_pct, row.plain.efficiency_pct);
            if row.lectra_pct.is_some() {
                lectra.add(row.variant.efficiency_pct, row.lectra_pct);
            }
        }
        writeln!(
            file,
            "| {strategy} | {} / {} | {:+.4} | {:.4} | {} / {} / {} | {} | {} | {} |",
            plain.diffs.len(),
            rows.len(),
            plain.mean(),
            plain.ci(),
            plain.win,
            plain.tie,
            plain.loss,
            if lectra.diffs.is_empty() {
                "—".into()
            } else {
                format!("{:+.4}", lectra.mean())
            },
            if lectra.diffs.is_empty() {
                "—".into()
            } else {
                format!("{} / {} / {}", lectra.win, lectra.tie, lectra.loss)
            },
            plain.failed
        )?;
    }
    writeln!(
        file,
        "\n| Strategy | Plain vs Lectra mean pp | Plain W / T / L | Mean plain wall s | Mean variant wall s | Variant plain fallbacks | Active cluster trials | Timeouts plain / variant |\n|---|---:|---|---:|---:|---:|---:|---|"
    )?;
    let mut audit: BTreeMap<&str, Vec<&Pair>> = BTreeMap::new();
    for pair in pairs {
        audit.entry(name(pair.strategy)).or_default().push(pair);
    }
    for (strategy, rows) in audit {
        let mut lectra = Stats::default();
        for row in &rows {
            if row.lectra_pct.is_some() {
                lectra.add(row.plain.efficiency_pct, row.lectra_pct);
            }
        }
        let n = rows.len().max(1) as f64;
        let fallbacks = rows
            .iter()
            .filter(|r| {
                r.variant
                    .warnings
                    .iter()
                    .any(|w| w.contains("candidate failed; plain fallback"))
            })
            .count();
        let active = rows
            .iter()
            .filter(|r| {
                r.variant.warnings.iter().any(|w| {
                    w.strip_prefix("Cluster construction used ")
                        .and_then(|s| s.split_whitespace().next())
                        .and_then(|s| s.parse::<usize>().ok())
                        .is_some_and(|n| n > 0)
                })
            })
            .count();
        writeln!(
            file,
            "| {strategy} | {} | {} | {:.3} | {:.3} | {fallbacks} | {active} | {} / {} |",
            if lectra.diffs.is_empty() {
                "—".into()
            } else {
                format!("{:+.4}", lectra.mean())
            },
            if lectra.diffs.is_empty() {
                "—".into()
            } else {
                format!("{} / {} / {}", lectra.win, lectra.tie, lectra.loss)
            },
            rows.iter().map(|r| r.plain.wall_s).sum::<f64>() / n,
            rows.iter().map(|r| r.variant.wall_s).sum::<f64>() / n,
            rows.iter().filter(|r| r.plain.timeout).count(),
            rows.iter().filter(|r| r.variant.timeout).count()
        )?;
    }
    let mut csv = File::create(out.join("results.csv"))?;
    writeln!(
        csv,
        "task_id,strategy,budget_s,variant_first,plain_valid,variant_valid,plain_pct,variant_pct,difference_pp,lectra_pct,plain_wall_s,variant_wall_s,plain_timeout,variant_timeout"
    )?;
    let number = |v: Option<f64>| v.map(|n| format!("{n:.8}")).unwrap_or_default();
    for p in pairs {
        writeln!(
            csv,
            "{},{},{},{},{},{},{},{},{},{},{:.4},{:.4},{},{}",
            p.task_id,
            name(p.strategy),
            p.budget_s,
            p.variant_first,
            p.plain.valid,
            p.variant.valid,
            number(p.plain.efficiency_pct),
            number(p.variant.efficiency_pct),
            number(
                p.variant
                    .efficiency_pct
                    .zip(p.plain.efficiency_pct)
                    .map(|(a, b)| a - b)
            ),
            number(p.lectra_pct),
            p.plain.wall_s,
            p.variant.wall_s,
            p.plain.timeout,
            p.variant.timeout
        )?;
    }
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|s| s == "--single") {
        ensure!(
            args.len() == 4,
            "internal trial requires request, strategy and output"
        );
        let request: TrialRequest = serde_json::from_reader(File::open(&args[1])?)?;
        let output = Path::new(&args[3]);
        let checkout = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()?;
        let parent = output
            .parent()
            .context("trial output has no parent")?
            .canonicalize()?;
        ensure!(
            !parent.starts_with(&checkout),
            "trial output must be outside the checkout"
        );
        let scored = trial_inline(
            &request.task,
            &request.options,
            parse_strategy(&args[2])?,
            output,
        )?;
        serde_json::to_writer(File::create(output.with_extension("trial.json"))?, &scored)?;
        return Ok(());
    }
    ensure!(
        args.len() >= 5,
        "usage: paired_bench INPUT OUT SPLIT SECONDS STRATEGIES [JOBS=3] [SEED=1]; strategies comma-separated; matched uses recorded durations"
    );
    let (input, out) = (PathBuf::from(&args[0]), PathBuf::from(&args[1]));
    let seconds: f64 = args[3].parse()?;
    ensure!(
        seconds.is_finite() && seconds > 0.0,
        "seconds must be positive"
    );
    let strategies: Vec<_> = args[4]
        .split(',')
        .map(parse_strategy)
        .collect::<Result<_>>()?;
    let jobs: usize = args.get(5).map(|s| s.parse()).transpose()?.unwrap_or(3);
    let seed: u64 = args.get(6).map(|s| s.parse()).transpose()?.unwrap_or(1);
    ensure!(
        (1..=5).contains(&jobs),
        "jobs must be 1..5 on the shared machine"
    );
    let tasks = load(&input, &args[2])?;
    ensure!(
        tasks.iter().all(|t| !id(t).is_empty()
            && id(t)
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')),
        "task IDs must contain only ASCII letters, digits, underscores or hyphens"
    );
    // Refuse putting geometry inside a source checkout.
    let checkout = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()?;
    let absolute = if out.is_absolute() {
        out.clone()
    } else {
        std::env::current_dir()?.join(&out)
    };
    ensure!(
        !absolute.starts_with(&checkout),
        "benchmark output must be outside the checkout"
    );
    fs::create_dir_all(&out)?;
    let output = out.canonicalize()?;
    ensure!(
        !output.starts_with(&checkout),
        "benchmark output must be outside the checkout"
    );
    let manifest = Manifest {
        revision: std::env::var("BETA_CLUSTER_REVISION").unwrap_or_else(|_| "unrecorded".into()),
        source: input.to_string_lossy().into(),
        split: args[2].clone(),
        task_ids: tasks.iter().map(id).collect(),
        strategies: strategies.clone(),
        time_s: seconds,
        seed,
        workers: 2,
        jobs,
        sampling_seed: 7030786,
        wall_grace_s: WALL_GRACE_S,
    };
    let manifest_path = out.join("run.json");
    if manifest_path.exists() {
        let previous: Manifest = serde_json::from_reader(File::open(&manifest_path)?)?;
        ensure!(
            previous == manifest,
            "existing output directory has a different manifest"
        );
    } else {
        serde_json::to_writer_pretty(File::create(&manifest_path)?, &manifest)?;
    }
    let mut work = Vec::new();
    for (i, task) in tasks.into_iter().enumerate() {
        for (j, &strategy) in strategies.iter().enumerate() {
            work.push((task.clone(), strategy, (i + j) % 2 == 1));
        }
    }
    let started = Instant::now();
    let next = AtomicUsize::new(0);
    let rows = Arc::new(Mutex::new(Vec::new()));
    let errors = Arc::new(Mutex::new(Vec::new()));
    let matched = args[2] == "matched";
    std::thread::scope(|scope| {
        for _ in 0..jobs {
            let (work, rows, errors, out, next) =
                (&work, rows.clone(), errors.clone(), &out, &next);
            scope.spawn(move || {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some((task, strategy, variant_first)) = work.get(index) else {
                        break;
                    };
                    let run = || -> Result<Pair> {
                        let stem = format!("{}-{}", id(task), name(*strategy));
                        let pair_file = out.join(format!("{stem}.pair.json"));
                        if pair_file.exists() {
                            return Ok(serde_json::from_reader(File::open(pair_file)?)?);
                        }
                        let budget = if matched {
                            task.lectra_duration_s
                                .context("matched task has no duration")?
                        } else {
                            seconds
                        };
                        let options = NestOptions {
                            time_limit_s: budget,
                            seed: Some(seed),
                            workers: Some(2),
                            ..Default::default()
                        };
                        let (plain, variant) = if *variant_first {
                            let variant = trial(
                                task,
                                &options,
                                *strategy,
                                &out.join(format!("{stem}.variant.json")),
                            )?;
                            let plain = trial(
                                task,
                                &options,
                                Strategy::Plain,
                                &out.join(format!("{stem}.plain.json")),
                            )?;
                            (plain, variant)
                        } else {
                            let plain = trial(
                                task,
                                &options,
                                Strategy::Plain,
                                &out.join(format!("{stem}.plain.json")),
                            )?;
                            let variant = trial(
                                task,
                                &options,
                                *strategy,
                                &out.join(format!("{stem}.variant.json")),
                            )?;
                            (plain, variant)
                        };
                        let pair = Pair {
                            task_id: id(task),
                            stratum: task.stratum.clone(),
                            strategy: *strategy,
                            seed,
                            budget_s: budget,
                            workers: 2,
                            variant_first: *variant_first,
                            lectra_pct: task.lectra_efficiency,
                            reference_area_ratio: task.reference_area_ratio,
                            plain,
                            variant,
                        };
                        let temp = out.join(format!("{stem}.pair.tmp"));
                        serde_json::to_writer_pretty(File::create(&temp)?, &pair)?;
                        fs::rename(temp, pair_file)?;
                        Ok(pair)
                    };
                    match run() {
                        Ok(pair) => {
                            eprintln!(
                                "{}/{} {} {}: plain {:?}, variant {:?}",
                                index + 1,
                                work.len(),
                                pair.task_id,
                                name(*strategy),
                                pair.plain.efficiency_pct,
                                pair.variant.efficiency_pct
                            );
                            if let Ok(mut rows) = rows.lock() {
                                rows.push(pair);
                                if let Err(error) =
                                    summary(out, &rows, started.elapsed().as_secs_f64())
                                    && let Ok(mut errors) = errors.lock()
                                {
                                    errors.push(error.to_string());
                                }
                            }
                        }
                        Err(error) => {
                            if let Ok(mut errors) = errors.lock() {
                                errors.push(format!("{} {}: {error:#}", id(task), name(*strategy)));
                            }
                        }
                    }
                }
            });
        }
    });
    let rows = rows
        .lock()
        .map_err(|_| anyhow::anyhow!("result lock poisoned"))?;
    summary(&out, &rows, started.elapsed().as_secs_f64())?;
    let errors = errors
        .lock()
        .map_err(|_| anyhow::anyhow!("error lock poisoned"))?;
    if !errors.is_empty() {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(out.join("runner-errors.txt"))?;
        for error in errors.iter() {
            writeln!(file, "{error}")?;
        }
        anyhow::bail!("{} runner errors; inspect runner-errors.txt", errors.len());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failures_are_not_scored_as_zero_efficiency() {
        let mut stats = Stats::default();
        stats.add(Some(80.0), Some(79.0));
        stats.add(None, Some(90.0));
        stats.add(Some(80.04), Some(80.0));
        assert_eq!(
            (stats.win, stats.tie, stats.loss, stats.failed),
            (1, 1, 1, 1)
        );
        assert_eq!(stats.diffs.len(), 2);
        assert!((stats.mean() - 0.52).abs() < 1e-10);
    }

    #[test]
    fn sampling_hash_and_strategy_names_are_stable() -> Result<()> {
        assert_eq!(hash_id("12345"), hash_id("12345"));
        assert_ne!(hash_id("12345"), hash_id("12346"));
        for strategy in [
            Strategy::Plain,
            Strategy::Cluster,
            Strategy::ClusterContour,
            Strategy::Aggregate,
            Strategy::Restart,
            Strategy::GapFill,
            Strategy::GapFillFixed,
            Strategy::TwoStage,
            Strategy::TwoStageFixed,
            Strategy::Auto,
            Strategy::Race,
        ] {
            assert_eq!(parse_strategy(name(strategy))?, strategy);
        }
        Ok(())
    }

    #[test]
    #[ignore = "only launched by the child-timeout test"]
    fn timeout_child() {
        std::thread::sleep(Duration::from_secs(5));
    }

    #[test]
    fn slow_children_are_terminated_and_reaped() -> Result<()> {
        let mut command = child_command(&std::env::current_exe()?);
        command
            .args(["--exact", "tests::timeout_child", "--ignored"])
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let started = Instant::now();
        let mut child = command.spawn()?;
        assert!(matches!(
            wait_child(&mut child, started, Duration::from_millis(100))?,
            Completion::TimedOut
        ));
        assert!(child.try_wait()?.is_some());
        Ok(())
    }
}
