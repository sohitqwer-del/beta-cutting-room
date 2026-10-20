use anyhow::{Result, ensure};
use beta_marker_core::{
    Job, MarkerResult, ValidationReport, check_job, metrics::length_saving, validate,
};
use beta_marker_nest::{NestOptions, SearchTuning, nest_with_tuning};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub job: Job,
    pub seed: u64,
    pub budget_s: f64,
    /// Percent, not fraction.
    pub reference: Option<f64>,
    pub stratum: String,
    pub area_ratio: f64,
}
#[derive(Debug, Serialize)]
struct Row {
    id: String,
    seed: u64,
    pieces: u64,
    width: f64,
    length: Option<f64>,
    efficiency: Option<f64>,
    outline_efficiency: Option<f64>,
    demand_efficiency: Option<f64>,
    reference: Option<f64>,
    difference: Option<f64>,
    saving: Option<f64>,
    runtime_s: f64,
    validation_s: f64,
    budget_s: f64,
    valid: bool,
    errors: String,
    warnings: String,
    engine: String,
    stratum: String,
    tier: String,
    started_unix_ms: u64,
    finished_unix_ms: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Search {
    pub tuning: SearchTuning,
    pub runs: u32,
    /// Whole-bundle shade assignment probes; matching jobs use the current default nester.
    #[serde(default = "default_shade_attempts")]
    pub shade_attempts: usize,
    /// Explicit counterfactual baseline: omit matching and shade constraints.
    #[serde(default)]
    pub ignore_rules: bool,
}
fn default_shade_attempts() -> usize {
    1
}

fn run_one(task: &Task, threads: usize, search: &Search, evidence: &Path) -> Row {
    let mut free_task;
    let task = if search.ignore_rules {
        free_task = task.clone();
        free_task.job.fabric.shade = None;
        free_task.job.match_rules.clear();
        &free_task
    } else {
        task
    };
    measure(
        task,
        threads,
        |job, options| {
            let mut options = options.clone();
            options.runs = search.runs;
            fs::create_dir_all(evidence)?;
            fs::write(evidence.join("job.json"), serde_json::to_vec(job)?)?;
            let result = if beta_marker_match::needs_rules(job) {
                ensure!(
                    serde_json::to_value(&search.tuning)?
                        == serde_json::to_value(SearchTuning::default())?,
                    "matching/shade replay requires the recorded default search tuning"
                );
                beta_marker_match::nest_with_shade_search(job, &options, search.shade_attempts)?
            } else {
                nest_with_tuning(job, &options, &search.tuning)?
            };
            fs::write(evidence.join("result.json"), serde_json::to_vec(&result)?)?;
            Ok(result)
        },
        |job, result| {
            let mut report = validate(job, result);
            let saved = serde_json::to_vec(&report)
                .map_err(anyhow::Error::from)
                .and_then(|bytes| {
                    fs::write(evidence.join("validation.json"), bytes).map_err(anyhow::Error::from)
                });
            if let Err(error) = saved {
                report.ok = false;
                report.errors.push(beta_marker_core::Issue {
                    code: "evidence_write".into(),
                    message: format!("could not save validation evidence: {error}"),
                    piece_ids: vec![],
                });
            }
            report
        },
    )
}
fn measure(
    task: &Task,
    threads: usize,
    engine: impl FnOnce(&Job, &NestOptions) -> Result<MarkerResult>,
    validator: impl FnOnce(&Job, &MarkerResult) -> ValidationReport,
) -> Row {
    let entered = unix_ms();
    let mut row = Row {
        id: task.id.clone(),
        seed: task.seed,
        pieces: task.job.pieces.iter().fold(0_u64, |sum, p| {
            sum.saturating_add(
                (u64::from(p.quantity) + u64::from(p.mirror_quantity))
                    .saturating_mul(u64::from(beta_marker_core::rules::multiplier(&task.job, p))),
            )
        }),
        width: task.job.fabric.width,
        length: None,
        efficiency: None,
        outline_efficiency: None,
        demand_efficiency: None,
        reference: task.reference,
        difference: None,
        saving: None,
        runtime_s: 0.0,
        validation_s: 0.0,
        budget_s: task.budget_s,
        valid: false,
        errors: String::new(),
        warnings: String::new(),
        engine: String::new(),
        stratum: task.stratum.clone(),
        started_unix_ms: entered,
        finished_unix_ms: entered,
        tier: task
            .job
            .meta
            .get("benchmark_tier")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned(),
    };
    if task.job.pieces.iter().any(|p| {
        p.quantity
            .checked_add(p.mirror_quantity)
            .and_then(|n| n.checked_mul(beta_marker_core::rules::multiplier(&task.job, p)))
            .is_none()
    }) {
        row.errors = "input: quantity overflows".into();
        return row;
    }
    let issues = check_job(&task.job);
    if !issues.is_empty() {
        row.errors = format!("input: {issues:?}");
        return row;
    }
    let start = Instant::now();
    let options = NestOptions {
        time_limit_s: task.budget_s,
        seed: Some(task.seed),
        workers: Some(threads),
        early_termination: false,
        ..Default::default()
    };
    // A diagnostic can expose a solver panic, for example when no source
    // orientation fits the assumed strip. Preserve that failure as a trial.
    let result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| engine(&task.job, &options)))
            .unwrap_or_else(|payload| {
                let message = payload
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .or_else(|| payload.downcast_ref::<&str>().copied())
                    .unwrap_or("non-string panic payload");
                Err(anyhow::anyhow!("solver panicked: {message}"))
            });
    row.runtime_s = start.elapsed().as_secs_f64();
    match result {
        Err(error) => row.errors = format!("nest: {error:#}"),
        Ok(result) => {
            let start = Instant::now();
            let report = validator(&task.job, &result);
            row.validation_s = start.elapsed().as_secs_f64();
            row.engine = result.engine;
            row.warnings =
                serde_json::to_string(&(result.warnings, &report.warnings)).unwrap_or_default();
            row.valid = report.ok
                && report.errors.is_empty()
                && report.length.is_finite()
                && report.length > 0.0
                && report.efficiency.is_finite()
                && report.efficiency > 0.0
                && report.efficiency <= 1.0 + 1e-8;
            row.errors = serde_json::to_string(&report.errors).unwrap_or_default();
            if row.valid {
                let efficiency = report.efficiency * task.area_ratio * 100.0;
                row.length = Some(report.length);
                row.efficiency = Some(efficiency);
                row.outline_efficiency = Some(report.efficiency * 100.0);
                row.demand_efficiency = Some(
                    if task.job.fabric.layout == beta_marker_core::Layout::Open {
                        report.efficiency * 100.0
                    } else {
                        let effective = beta_marker_core::rules::effective_job(&task.job);
                        let area: f64 = beta_marker_core::demands(&effective)
                            .iter()
                            .map(|d| {
                                beta_marker_core::geometry::area(
                                    &effective.pieces[d.piece_index].outline,
                                ) * f64::from(d.count)
                                    / 2.0
                            })
                            .sum();
                        area / (task.job.fabric.width * report.length) * 100.0
                    },
                );
                if let Some(reference) = task.reference {
                    row.difference = Some(efficiency - reference);
                    row.saving = Some(length_saving(reference, efficiency));
                }
            } else if report.errors.is_empty() {
                row.errors = "validator returned invalid or non-finite metrics".into();
            }
        }
    }
    row.finished_unix_ms = unix_ms();
    row
}
fn unix_ms() -> u64 {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
    )
    .unwrap_or(u64::MAX)
}
fn cell(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}
fn number(n: Option<f64>) -> String {
    n.map_or_else(String::new, |v| format!("{v:.9}"))
}
fn csv(row: &Row) -> String {
    [
        cell(&row.id),
        row.seed.to_string(),
        row.pieces.to_string(),
        number(Some(row.width)),
        number(row.length),
        number(row.efficiency),
        number(row.reference),
        number(row.difference),
        number(row.saving),
        number(Some(row.runtime_s)),
        row.valid.to_string(),
        cell(&row.errors),
        cell(&row.stratum),
        number(Some(row.budget_s)),
        number(Some(row.validation_s)),
        cell(&row.engine),
        cell(&row.warnings),
        number(row.outline_efficiency),
        cell(&row.tier),
        number(row.demand_efficiency),
        row.started_unix_ms.to_string(),
        row.finished_unix_ms.to_string(),
    ]
    .join(",")
}
fn stats(rows: &[&Row]) -> String {
    let too_long = rows
        .iter()
        .filter(|r| !r.valid && r.errors.contains("too_long"))
        .count();
    let mut diffs: Vec<_> = rows
        .iter()
        .filter(|r| r.valid)
        .filter_map(|r| r.difference)
        .collect();
    diffs.sort_by(f64::total_cmp);
    if diffs.is_empty() {
        return format!("{} | 0 | — | — | 0 / 0 / {too_long}", rows.len());
    }
    let n = diffs.len();
    let median = (diffs[(n - 1) / 2] + diffs[n / 2]) / 2.0;
    let wins = diffs.iter().filter(|&&d| d > 0.1 + 1e-9).count();
    let geometric_losses = diffs.iter().filter(|&&d| d < -0.1 - 1e-9).count();
    let losses = geometric_losses + too_long;
    format!(
        "{} | {n} | {:.4} | {median:.4} | {wins} / {} / {losses}",
        rows.len(),
        diffs.iter().sum::<f64>() / n as f64,
        n - wins - geometric_losses
    )
}
/// Saved, self-contained tasks for load-only reruns. Never selects on score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Replay {
    pub dataset: String,
    pub tasks: Vec<Task>,
    pub search: Search,
    pub paired_baseline: Option<Search>,
}

const CSV_HEADER: &str = "id,seed,pieces,width,our_length,our_efficiency,reference_efficiency,difference_pp,relative_length_saving,runtime_s,valid,errors,stratum,budget_s,validation_s,engine,warnings,outline_efficiency,tier,demand_efficiency,started_unix_ms,finished_unix_ms";

pub fn run(
    tasks: Vec<Task>,
    jobs: usize,
    threads: usize,
    out: &Path,
    dataset: &str,
    search: Search,
) -> Result<()> {
    run_set(
        tasks,
        jobs,
        threads,
        out,
        dataset,
        vec![(String::new(), search)],
    )
}

/// Each worker completes an adjacent baseline/variant pair of the same task.
/// Even source indices run baseline first; odd indices run variant first.
pub fn run_paired(
    tasks: Vec<Task>,
    jobs: usize,
    threads: usize,
    out: &Path,
    dataset: &str,
    search: Search,
    baseline: Search,
) -> Result<()> {
    run_set(
        tasks,
        jobs,
        threads,
        out,
        dataset,
        vec![("baseline".into(), baseline), ("variant".into(), search)],
    )
}

struct Sink {
    path: PathBuf,
    label: String,
    search: Search,
    writer: Mutex<BufWriter<File>>,
    rows: Mutex<Vec<(usize, Row)>>,
}

fn run_set(
    tasks: Vec<Task>,
    jobs: usize,
    threads: usize,
    out: &Path,
    dataset: &str,
    searches: Vec<(String, Search)>,
) -> Result<()> {
    let cores = std::thread::available_parallelism().map_or(1, usize::from);
    ensure!(
        !tasks.is_empty() && jobs > 0 && threads > 0 && jobs.saturating_mul(threads) <= cores,
        "tasks and worker counts must be positive and fit {cores} logical CPUs"
    );
    ensure!(
        tasks
            .iter()
            .all(|t| t.budget_s.is_finite() && t.budget_s > 0.0),
        "every task needs a positive finite budget"
    );
    ensure!(
        searches
            .iter()
            .all(|(_, s)| (1..=1024).contains(&s.runs) && (1..=16).contains(&s.shade_attempts)),
        "invalid run or shade-probe count"
    );
    let paired = searches.len() == 2;
    fs::create_dir_all(out)?;
    let revision = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned());
    let dirty = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .map(|o| !o.stdout.is_empty());
    let replay = Replay {
        dataset: dataset.into(),
        tasks: tasks.clone(),
        search: searches[usize::from(paired)].1.clone(),
        paired_baseline: paired.then(|| searches[0].1.clone()),
    };
    fs::write(out.join("replay.json"), serde_json::to_vec(&replay)?)?;
    drop(replay);
    let mut sinks = Vec::new();
    for (label, search) in searches {
        let path = if label.is_empty() {
            out.to_owned()
        } else {
            out.join(&label)
        };
        fs::create_dir_all(&path)?;
        ensure!(
            !path.join("results.csv").exists(),
            "output already has results.csv; choose a new directory"
        );
        let config = serde_json::json!({"dataset":dataset,"revision":revision,"dirty":dirty,"search":search,
            "paired":paired,"arm":label,"pair_order":"even source index: baseline then variant; odd: variant then baseline",
            "jobs":jobs,"threads_per_task":threads,"os":std::env::consts::OS,"arch":std::env::consts::ARCH,
            "logical_cpus":cores,"processor":std::env::var("PROCESSOR_IDENTIFIER").ok(),
            "trial_evidence":"trials/<input index>/{result,validation}.json","tasks":tasks});
        fs::write(path.join("run.json"), serde_json::to_vec_pretty(&config)?)?;
        let mut writer = BufWriter::new(File::create_new(path.join("results.csv"))?);
        writeln!(writer, "{CSV_HEADER}")?;
        writer.flush()?;
        sinks.push(Sink {
            path,
            label,
            search,
            writer: Mutex::new(writer),
            rows: Mutex::new(Vec::new()),
        });
    }
    let next = AtomicUsize::new(0);
    let started = Instant::now();
    std::thread::scope(|scope| -> Result<()> {
        let mut handles = Vec::new();
        for _ in 0..jobs.min(tasks.len()) {
            handles.push(scope.spawn(|| -> Result<()> {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(task) = tasks.get(index) else {
                        break;
                    };
                    let source_index = task
                        .job
                        .meta
                        .get("benchmark_pair_index")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(index as u64);
                    let order = if paired && !source_index.is_multiple_of(2) {
                        vec![1, 0]
                    } else {
                        (0..sinks.len()).collect()
                    };
                    for arm in order {
                        let sink = &sinks[arm];
                        let row = run_one(
                            task,
                            threads,
                            &sink.search,
                            &sink.path.join("trials").join(format!("{index:05}")),
                        );
                        {
                            let mut writer = sink
                                .writer
                                .lock()
                                .map_err(|_| anyhow::anyhow!("CSV collector poisoned"))?;
                            writeln!(writer, "{}", csv(&row))?;
                            writer.flush()?;
                        }
                        eprintln!(
                            "{} {} seed {}: valid={} runtime={:.3}s {}",
                            sink.label, row.id, row.seed, row.valid, row.runtime_s, row.errors
                        );
                        sink.rows
                            .lock()
                            .map_err(|_| anyhow::anyhow!("row collector poisoned"))?
                            .push((index, row));
                    }
                }
                Ok(())
            }));
        }
        for handle in handles {
            handle
                .join()
                .map_err(|_| anyhow::anyhow!("benchmark worker panicked"))??;
        }
        Ok(())
    })?;
    let elapsed = started.elapsed().as_secs_f64();
    let mut arms = Vec::new();
    for sink in sinks {
        let mut rows = sink
            .rows
            .into_inner()
            .map_err(|_| anyhow::anyhow!("row collector poisoned"))?;
        rows.sort_by_key(|(index, _)| *index);
        let rows: Vec<Row> = rows.into_iter().map(|(_, row)| row).collect();
        write_summary(
            &sink.path,
            dataset,
            revision.as_deref(),
            dirty,
            jobs,
            threads,
            elapsed,
            paired,
            &rows,
        )?;
        arms.push(rows);
    }
    if paired {
        write_pairs(out, &tasks, &arms[0], &arms[1])?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_summary(
    out: &Path,
    dataset: &str,
    revision: Option<&str>,
    dirty: Option<bool>,
    jobs: usize,
    threads: usize,
    elapsed: f64,
    paired: bool,
    rows: &[Row],
) -> Result<()> {
    let all: Vec<_> = rows.iter().collect();
    let mut groups = BTreeMap::<&str, Vec<&Row>>::new();
    for row in &all {
        groups.entry(&row.stratum).or_default().push(row);
        if !row.tier.is_empty() {
            groups
                .entry(if row.tier == "A" {
                    "Tier A"
                } else {
                    "Tier B only"
                })
                .or_default()
                .push(row);
        }
    }
    let mut summary = format!(
        "# BETA Marker {dataset} benchmark\n\nRevision: {}. Dirty worktree: {:?}. Platform: {} / {}.\n\n{} runs; {} valid. Experiment wall time {:.3} s; this arm's summed nesting time {:.3} s. Parallel tasks: {jobs}; workers per task: {threads}. Paired study: {paired}. A paired study's wall time includes both arms. Full tasks, seeds, budgets and settings are saved in run.json and replay.json.\n\nDifferences are percentage points, BETA minus reference. Tie: absolute difference <= 0.1 point. Failed runs have no numeric score. too_long counts as a loss; other failures and pending-validator cases are unscored. Search budgets exclude some import/construction/final-check overhead; runtime_s records the complete engine call plus evidence I/O. The sample is unweighted.\n\n| Group | Attempts | Valid comparisons | Mean difference | Median difference | Win / tie / loss |\n|---|---:|---:|---:|---:|---|\n| All | {} |\n",
        revision.unwrap_or("unknown"),
        dirty,
        std::env::consts::OS,
        std::env::consts::ARCH,
        rows.len(),
        rows.iter().filter(|r| r.valid).count(),
        elapsed,
        rows.iter().map(|r| r.runtime_s).sum::<f64>(),
        stats(&all)
    );
    for (name, group) in groups {
        summary.push_str(&format!(
            "| {} | {} |\n",
            name.replace('|', "/"),
            stats(&group)
        ));
    }
    let pending = rows
        .iter()
        .filter(|r| !r.valid && r.errors.contains("pending_validator"))
        .count();
    summary.push_str(&format!("\nPending validator: {pending}. These trials do not count as wins, ties or losses.\n\nMeasured on a shared machine with unrelated background programs. CPU samples are in conditions.json; per-trial UTC epoch times link each task to those samples. Lectra hardware/software and timing semantics are unknown. The comparison assumes code-0 rectangular strips and x-coordinate reflection before rotation. Tier B retains a subset of orientations. Academic references are published means from longer, different-hardware runs.\n\nSources: [Sparrow paper](https://arxiv.org/html/2509.13329v4), [Lectra dataset, CC BY 4.0](https://doi.org/10.5281/zenodo.7030786).\n"));
    fs::write(out.join("summary.md"), summary)?;
    Ok(())
}

fn write_pairs(out: &Path, tasks: &[Task], baseline: &[Row], variant: &[Row]) -> Result<()> {
    ensure!(
        baseline.len() == tasks.len() && variant.len() == tasks.len(),
        "incomplete paired rows"
    );
    let mut values = Vec::new();
    let mut pairs = Vec::new();
    for (index, task) in tasks.iter().enumerate() {
        let (b, v) = (&baseline[index], &variant[index]);
        let delta = if b.valid && v.valid {
            b.efficiency.zip(v.efficiency).map(|(b, v)| v - b)
        } else {
            None
        };
        if let Some(delta) = delta {
            values.push(delta);
        }
        pairs.push(serde_json::json!({"id":task.id,"seed":task.seed,"baseline_valid":b.valid,"variant_valid":v.valid,
            "baseline_efficiency":b.efficiency,"variant_efficiency":v.efficiency,"change_pp":delta,
            "baseline_started_unix_ms":b.started_unix_ms,"baseline_finished_unix_ms":b.finished_unix_ms,
            "variant_started_unix_ms":v.started_unix_ms,"variant_finished_unix_ms":v.finished_unix_ms,
            "baseline_first":b.started_unix_ms<=v.started_unix_ms}));
    }
    values.sort_by(f64::total_cmp);
    let mean = (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64);
    let median = (!values.is_empty())
        .then(|| (values[(values.len() - 1) / 2] + values[values.len() / 2]) / 2.0);
    fs::write(
        out.join("paired-summary.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"pairs":tasks.len(),"valid_pairs":values.len(),"mean_change_pp":mean,"median_change_pp":median,"rows":pairs}),
        )?,
    )?;
    fs::write(
        out.join("paired-summary.md"),
        format!(
            "# Paired development comparison\n\n{} tasks; {} valid pairs. Mean variant-minus-baseline change: {:?} percentage points; median {:?}. Same task, seed and budget in each pair. Pair order alternates by original source index. Only these paired differences decide promotion; failures remain visible and unscored.\n",
            tasks.len(),
            values.len(),
            mean,
            median
        ),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use beta_marker_core::{
        Fabric, JOB_FORMAT, Nap, Piece, Placement, RESULT_FORMAT, RotationRule, Spacing,
        compute_metrics,
    };
    fn task(id: usize) -> Task {
        Task {
            id: id.to_string(),
            job: Job {
                match_rules: Vec::new(),
                format: JOB_FORMAT.into(),
                name: format!("rect-{id}"),
                fabric: Fabric {
                    width: 10.0,
                    max_length: None,
                    nap: Nap::None,
                    allow_flip: false,
                    tilt_deg: 0.0,
                    ..Default::default()
                },
                spacing: Spacing::default(),
                pieces: vec![Piece {
                    id: "rect".into(),
                    name: "rect".into(),
                    style: None,
                    size: None,
                    material: None,
                    outline: vec![[0.0, 0.0], [2.0, 0.0], [2.0, 5.0], [0.0, 5.0]],
                    grain: None,
                    quantity: id as u32,
                    mirror_quantity: 0,
                    rotation: RotationRule::Fixed,
                    buffer: 0.0,
                    marks: vec![],
                    ..Default::default()
                }],
                order: None,
                meta: BTreeMap::new(),
            },
            seed: 1,
            budget_s: 0.001,
            reference: Some(40.0),
            stratum: "tiny".into(),
            area_ratio: 1.0,
        }
    }
    // Test-only shelf placement, never used by the command or benchmarks.
    fn shelf(job: &Job, _: &NestOptions) -> Result<MarkerResult> {
        let placements = (0..job.pieces[0].quantity)
            .map(|copy| Placement {
                piece_id: "rect".into(),
                copy,
                mirrored: false,
                rotation_deg: 0.0,
                translate: [f64::from(copy) * 2.0, 0.0],
                locked: false,
                ..Default::default()
            })
            .collect::<Vec<_>>();
        let m = compute_metrics(job, &placements);
        Ok(MarkerResult {
            format: RESULT_FORMAT.into(),
            job_name: job.name.clone(),
            width: job.fabric.width,
            length: m.length,
            piece_area: m.piece_area,
            efficiency: m.efficiency,
            engine: "test-only shelf".into(),
            seed: Some(1),
            runtime_s: 0.0,
            placements,
            warnings: vec![],
        })
    }
    #[test]
    fn four_tiny_tasks_recompute_metrics_and_savings() {
        for n in 1..=4 {
            let row = measure(&task(n), 1, shelf, validate);
            assert!(row.valid);
            assert_eq!(row.length, Some(2.0 * n as f64));
            assert_eq!(row.efficiency, Some(50.0));
            assert_eq!(row.difference, Some(10.0));
            assert!((row.saving.unwrap() - 0.2).abs() < 1e-12);
            assert!(csv(&row).contains("test-only shelf"));
        }
    }
    #[test]
    fn failures_and_invalid_placements_never_count_as_wins() {
        let failed = measure(
            &task(1),
            1,
            |_, _| anyhow::bail!("engine failure"),
            validate,
        );
        assert!(!failed.valid && failed.efficiency.is_none());
        let invalid = measure(
            &task(2),
            1,
            |job, opts| {
                let mut result = shelf(job, opts)?;
                result.placements[1].translate = [0.0, 0.0];
                Ok(result)
            },
            validate,
        );
        assert!(!invalid.valid && invalid.difference.is_none());
        assert!(stats(&[&failed, &invalid]).contains("2 | 0 | —"));
    }
    #[test]
    fn summary_counts_boundary_ties_and_excludes_failures() {
        let mut rows = [49.9, 50.1, 49.0, 51.0]
            .into_iter()
            .map(|reference| {
                let mut task = task(1);
                task.reference = Some(reference);
                measure(&task, 1, shelf, validate)
            })
            .collect::<Vec<_>>();
        rows.push(measure(
            &task(1),
            1,
            |_, _| anyhow::bail!("failed"),
            validate,
        ));
        assert_eq!(
            stats(&rows.iter().collect::<Vec<_>>()),
            "5 | 4 | 0.0000 | 0.0000 | 1 / 2 / 1"
        );
    }
    #[test]
    fn csv_escapes_quotes_commas_and_newlines() {
        assert_eq!(cell("a,\"b\"\nc"), "\"a,\"\"b\"\"\nc\"");
    }
    #[test]
    fn finite_sheet_failures_count_as_losses_without_inventing_efficiency() {
        let failed = measure(&task(1), 1, |_, _| anyhow::bail!("too_long"), validate);
        assert!(failed.efficiency.is_none() && failed.difference.is_none());
        assert_eq!(stats(&[&failed]), "1 | 0 | — | — | 0 / 0 / 1");
        let valid = measure(&task(1), 1, shelf, validate);
        assert_eq!(
            stats(&[&valid, &failed]),
            "2 | 1 | 10.0000 | 10.0000 | 1 / 0 / 1"
        );
    }
    #[test]
    fn solver_panics_remain_unscored_trials() {
        let row = measure(&task(1), 1, |_, _| panic!("cannot fit"), validate);
        assert!(!row.valid && row.efficiency.is_none() && row.saving.is_none());
        assert!(row.errors.contains("solver panicked: cannot fit"));
    }
    #[test]
    fn paired_native_trials_keep_tasks_seeds_and_alternate_actual_order() -> Result<()> {
        let root = std::env::var_os("BETA_DATA_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("tmp")
            .join(format!(
                "quality-pair-test-{}-{}",
                std::process::id(),
                unix_ms()
            ));
        let search = Search {
            tuning: SearchTuning::default(),
            runs: 1,
            shade_attempts: 1,
            ignore_rules: false,
        };
        run_paired(
            vec![task(1), task(2)],
            2,
            2,
            &root,
            "paired-tooling",
            search.clone(),
            search,
        )?;
        let pairs: serde_json::Value =
            serde_json::from_slice(&fs::read(root.join("paired-summary.json"))?)?;
        assert_eq!(pairs["valid_pairs"], 2);
        assert_eq!(pairs["rows"][0]["baseline_first"], true);
        assert_eq!(pairs["rows"][1]["baseline_first"], false);
        let replay: Replay = serde_json::from_slice(&fs::read(root.join("replay.json"))?)?;
        assert_eq!(replay.tasks.len(), 2);
        assert!(replay.paired_baseline.is_some());
        assert_eq!(replay.tasks[0].seed, replay.tasks[1].seed);
        Ok(())
    }
}
