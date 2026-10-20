//! `beta-marker bench`: reproducible comparisons with independent validation.
mod academic;
mod csv;
mod runner;
mod study;

use anyhow::{Context, Result, ensure};
use beta_marker_core::Job;
use clap::{Args as ClapArgs, Subcommand};
use serde::Deserialize;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
};

#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Audit background CPU from a run with recorded conditions.json CPU samples.
    Audit {
        #[arg(long)]
        run: PathBuf,
        #[arg(long)]
        reference: Option<PathBuf>,
    },
    /// Materialize effective CSVs after bench audit and score-independent load reruns.
    Finalize {
        #[arg(long)]
        run: PathBuf,
    },
    /// Benchmark saved garment jobs, including explicit folded Teagan variants.
    Jobs {
        #[arg(long, value_delimiter = ',', required = true)]
        inputs: Vec<PathBuf>,
        #[arg(long, default_value_t = 120.0)]
        time: f64,
        #[arg(long, value_delimiter = ',', default_value = "as-is")]
        layouts: Vec<String>,
        #[arg(long, default_value_t = 2)]
        jobs: usize,
        #[arg(long, default_value_t = 3)]
        threads_per_task: usize,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long, default_value_t = 1)]
        shade_attempts: usize,
        #[arg(long, conflicts_with = "baseline_shade_attempts")]
        paired_without_rules: bool,
        #[arg(long)]
        baseline_shade_attempts: Option<usize>,
        #[arg(long)]
        out: PathBuf,
    },
    /// Replay saved tasks chosen for a background-load check, with the original settings.
    Replay {
        #[arg(long)]
        input: PathBuf,
        #[arg(long, default_value_t = 2)]
        jobs: usize,
        #[arg(long, default_value_t = 3)]
        threads_per_task: usize,
        #[arg(long)]
        out: PathBuf,
    },
    /// Benchmark cached public instances. Missing local data fails without downloading.
    Academic {
        #[arg(
            long,
            value_delimiter = ',',
            default_value = "shirts,trousers,swim,albano,dagli,mao,marques"
        )]
        instances: Vec<String>,
        #[arg(long, default_value = "60")]
        time: f64,
        #[arg(long, value_delimiter = ',', default_value = "1,2,3")]
        seeds: Vec<u64>,
        #[arg(long, default_value_t = 1)]
        jobs: usize,
        #[arg(long, default_value_t = 3)]
        threads_per_task: usize,
        /// JSON SearchTuning file for a named development experiment.
        #[arg(long)]
        search_config: Option<PathBuf>,
        /// Independent seeds per task (search budget is shared).
        #[arg(long, default_value_t = 1)]
        runs: u32,
        #[arg(long)]
        out: PathBuf,
    },
    /// Benchmark exact or conservative Lectra tasks under the recorded policy.
    Lectra {
        #[arg(long)]
        input: PathBuf,
        #[arg(long, default_value = "60")]
        time: String,
        #[arg(long, default_value_t = 300)]
        limit: usize,
        #[arg(long)]
        jobs: Option<usize>,
        #[arg(long, default_value_t = 1)]
        threads_per_task: usize,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Optional cap for matched time; by default keep the full recorded duration.
        #[arg(long)]
        match_cap: Option<f64>,
        /// Permit only code-2/3 sheet diagnostics; never label them headline results.
        #[arg(long)]
        diagnostic: bool,
        /// Interleave this baseline with the variant for each task, alternating order.
        #[arg(long)]
        paired_baseline: Option<PathBuf>,
        /// JSON SearchTuning file for a named development experiment.
        #[arg(long)]
        search_config: Option<PathBuf>,
        /// Independent seeds per task (search budget is shared).
        #[arg(long, default_value_t = 1)]
        runs: u32,
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(Deserialize)]
struct LectraTask {
    id: String,
    job: Job,
    lectra_efficiency: f64,
    lectra_duration_s: f64,
    comparable: bool,
    #[serde(default)]
    reason: String,
    stratum: String,
    #[serde(default)]
    tier: String,
    /// Outer minus inner area / outer area. Makes length comparisons use the same area.
    #[serde(default = "one")]
    reference_area_ratio: f64,
    #[serde(default)]
    sheet_type: u32,
    #[serde(default)]
    diagnostic_eligible: bool,
}
fn one() -> f64 {
    1.0
}
fn positive(value: f64) -> Result<f64> {
    ensure!(
        value.is_finite() && value > 0.0,
        "time must be finite and positive"
    );
    Ok(value)
}

pub fn run(args: Args) -> Result<()> {
    match args.command {
        Command::Audit { run, reference } => {
            let report = study::audit(&run, reference.as_deref())?;
            println!(
                "CPU audit: {} outlier tasks; cutoff {:?}%",
                report.outlier_tasks.len(),
                report.threshold_cpu
            );
            Ok(())
        }
        Command::Finalize { run } => study::finalize(&run),
        Command::Jobs {
            inputs,
            time,
            layouts,
            jobs,
            threads_per_task,
            seed,
            shade_attempts,
            paired_without_rules,
            baseline_shade_attempts,
            out,
        } => {
            positive(time)?;
            let mut tasks = Vec::new();
            for input in inputs {
                let original: Job = serde_json::from_reader(File::open(&input)?)?;
                let stem = input
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .context("job path needs a name")?
                    .trim_end_matches(".job");
                for layout in &layouts {
                    let mut job = original.clone();
                    match layout.as_str() {
                        "as-is" => {}
                        "folded" | "tubular" => {
                            ensure!(
                                job.fabric.layout == beta_marker_core::Layout::Open
                                    && job.name.to_lowercase().contains("teagan"),
                                "fold conversion is defined for the open Teagan fixture"
                            );
                            job.fabric.layout = if layout == "folded" {
                                beta_marker_core::Layout::Folded
                            } else {
                                beta_marker_core::Layout::Tubular
                            };
                            job.fabric.width /= 2.0;
                            for piece in &mut job.pieces {
                                if matches!(piece.name.as_str(), "front" | "back") {
                                    let b = beta_marker_core::geometry::bbox(&piece.outline)
                                        .context("empty torso outline")?;
                                    ensure!(
                                        (b[0] + b[2]).abs() < 0.001,
                                        "Teagan torso must be centered on x=0"
                                    );
                                    piece.fold_line = Some([[0.0, b[1]], [0.0, b[3]]]);
                                }
                            }
                        }
                        _ => anyhow::bail!("layout must be as-is, folded or tubular"),
                    }
                    job.meta.insert(
                        "benchmark_pair_index".into(),
                        serde_json::json!(tasks.len()),
                    );
                    tasks.push(runner::Task {
                        id: format!("{stem}-{layout}"),
                        job,
                        seed,
                        budget_s: time,
                        reference: None,
                        stratum: layout.clone(),
                        area_ratio: 1.0,
                    });
                }
            }
            let mut search = load_search(None, 1)?;
            search.shade_attempts = shade_attempts;
            if paired_without_rules || baseline_shade_attempts.is_some() {
                let mut baseline = load_search(None, 1)?;
                baseline.ignore_rules = paired_without_rules;
                baseline.shade_attempts = baseline_shade_attempts.unwrap_or(1);
                runner::run_paired(
                    tasks,
                    jobs,
                    threads_per_task,
                    &out,
                    "garments",
                    search,
                    baseline,
                )
            } else {
                runner::run(tasks, jobs, threads_per_task, &out, "garments", search)
            }
        }
        Command::Replay {
            input,
            jobs,
            threads_per_task,
            out,
        } => {
            let replay: runner::Replay = serde_json::from_reader(File::open(input)?)?;
            if let Some(baseline) = replay.paired_baseline {
                runner::run_paired(
                    replay.tasks,
                    jobs,
                    threads_per_task,
                    &out,
                    &replay.dataset,
                    replay.search,
                    baseline,
                )
            } else {
                runner::run(
                    replay.tasks,
                    jobs,
                    threads_per_task,
                    &out,
                    &replay.dataset,
                    replay.search,
                )
            }
        }
        Command::Academic {
            instances,
            time,
            seeds,
            jobs,
            threads_per_task,
            search_config,
            runs,
            out,
        } => {
            positive(time)?;
            ensure!(threads_per_task > 0, "threads-per-task must be positive");
            ensure!(
                !instances.is_empty() && !seeds.is_empty(),
                "instances and seeds cannot be empty"
            );
            let cores = std::thread::available_parallelism().map_or(1, usize::from);
            ensure!(
                jobs > 0 && jobs.saturating_mul(threads_per_task) <= cores,
                "jobs times threads-per-task must be between 1 and {cores}"
            );
            let mut tasks = vec![];
            for id in instances {
                let job = academic::load(&id)?;
                for &seed in &seeds {
                    tasks.push(runner::Task {
                        id: id.clone(),
                        job: job.clone(),
                        seed,
                        budget_s: time,
                        reference: academic::reference(&id),
                        stratum: id.clone(),
                        area_ratio: 1.0,
                    });
                }
            }
            runner::run(
                tasks,
                jobs,
                threads_per_task,
                &out,
                "academic",
                load_search(search_config, runs)?,
            )
        }
        Command::Lectra {
            input,
            time,
            limit,
            jobs,
            threads_per_task,
            seed,
            match_cap,
            diagnostic,
            paired_baseline,
            search_config,
            runs,
            out,
        } => {
            ensure!(
                limit > 0 && threads_per_task > 0,
                "limit and threads-per-task must be positive"
            );
            if let Some(cap) = match_cap {
                positive(cap)?;
            }
            let fixed_time = if time == "match" {
                None
            } else {
                Some(positive(
                    time.parse().context("--time must be seconds or match")?,
                )?)
            };
            let cores = std::thread::available_parallelism().map_or(1, usize::from);
            let jobs = jobs.unwrap_or_else(|| (cores / threads_per_task).max(1));
            ensure!(jobs > 0, "jobs must be positive");
            ensure!(
                jobs.saturating_mul(threads_per_task) <= cores,
                "requested workers exceed {cores} logical CPUs; reduce jobs or threads-per-task"
            );
            let mut tasks = vec![];
            for (line_index, line) in BufReader::new(File::open(&input)?).lines().enumerate() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                let mut task: LectraTask = serde_json::from_str(&line)
                    .with_context(|| format!("input line {}", line_index + 1))?;
                ensure!(
                    task.comparable
                        || (diagnostic
                            && task.diagnostic_eligible
                            && matches!(task.sheet_type, 2 | 3)
                            && task.reason == "sheet_type_not_headline"),
                    "task {} is not comparable: {}",
                    task.id,
                    task.reason
                );
                ensure!(
                    task.lectra_efficiency.is_finite()
                        && task.lectra_efficiency > 0.0
                        && task.lectra_efficiency <= 100.0,
                    "bad efficiency for {}",
                    task.id
                );
                ensure!(
                    task.reference_area_ratio.is_finite()
                        && task.reference_area_ratio > 0.0
                        && task.reference_area_ratio <= 1.0,
                    "bad area ratio for {}",
                    task.id
                );
                let budget_s = match fixed_time {
                    Some(t) => t,
                    None => positive(task.lectra_duration_s)?.min(match_cap.unwrap_or(f64::MAX)),
                };
                task.job.meta.insert(
                    "benchmark_tier".into(),
                    serde_json::Value::String(task.tier),
                );
                task.job
                    .meta
                    .entry("benchmark_pair_index".into())
                    .or_insert(serde_json::json!(line_index));
                tasks.push(runner::Task {
                    id: task.id,
                    job: task.job,
                    seed,
                    budget_s,
                    reference: Some(task.lectra_efficiency),
                    stratum: task.stratum,
                    area_ratio: task.reference_area_ratio,
                });
                if tasks.len() == limit {
                    break;
                }
            }
            ensure!(!tasks.is_empty(), "input contains no tasks");
            let dataset = if diagnostic {
                "lectra-sheet-diagnostic"
            } else {
                "lectra"
            };
            let search = load_search(search_config, runs)?;
            if let Some(path) = paired_baseline {
                runner::run_paired(
                    tasks,
                    jobs,
                    threads_per_task,
                    &out,
                    dataset,
                    search,
                    load_search(Some(path), 1)?,
                )
            } else {
                runner::run(tasks, jobs, threads_per_task, &out, dataset, search)
            }
        }
    }
}

fn load_search(path: Option<PathBuf>, runs: u32) -> Result<runner::Search> {
    ensure!(
        (1..=1024).contains(&runs),
        "runs must be between 1 and 1024"
    );
    let tuning = path
        .map(|path| -> Result<_> { Ok(serde_json::from_reader(File::open(path)?)?) })
        .transpose()?
        .unwrap_or_default();
    Ok(runner::Search {
        tuning,
        runs,
        shade_attempts: 1,
        ignore_rules: false,
    })
}
