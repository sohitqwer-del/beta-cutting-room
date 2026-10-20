//! Summarize saved metadata without loading or publishing job geometry.
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
struct Manifest {
    #[serde(default)]
    revision: String,
    split: String,
    task_ids: Vec<String>,
    strategies: Vec<String>,
    seed: u64,
    workers: usize,
    jobs: usize,
    #[serde(default)]
    wall_grace_s: f64,
}

#[derive(Deserialize)]
struct Trial {
    valid: bool,
    #[serde(default)]
    timeout: bool,
    efficiency_pct: Option<f64>,
    wall_s: f64,
    #[serde(default)]
    warnings: Vec<String>,
}
impl Trial {
    fn score(&self) -> Option<f64> {
        self.efficiency_pct.filter(|_| self.valid)
    }
}

#[derive(Deserialize)]
struct Pair {
    task_id: String,
    stratum: String,
    strategy: String,
    seed: u64,
    workers: usize,
    budget_s: f64,
    variant_first: bool,
    lectra_pct: Option<f64>,
    plain: Trial,
    variant: Trial,
}
struct Run {
    name: String,
    manifest: Manifest,
    pairs: Vec<Pair>,
    excluded_tasks: Vec<String>,
}

#[derive(Default)]
struct Stats {
    differences: Vec<f64>,
    win: usize,
    tie: usize,
    loss: usize,
    both_failed: usize,
}
impl Stats {
    fn add(&mut self, a: Option<f64>, b: Option<f64>) {
        match (a, b) {
            (Some(a), Some(b)) => {
                let difference = a - b;
                self.differences.push(difference);
                if difference > 0.1 {
                    self.win += 1;
                } else if difference < -0.1 {
                    self.loss += 1;
                } else {
                    self.tie += 1;
                }
            }
            (None, Some(_)) => self.loss += 1,
            (Some(_), None) => self.win += 1,
            (None, None) => self.both_failed += 1,
        }
    }
    fn mean(&self) -> Option<f64> {
        (!self.differences.is_empty())
            .then(|| self.differences.iter().sum::<f64>() / self.differences.len() as f64)
    }
    fn half_width(&self) -> Option<f64> {
        let n = self.differences.len();
        (n >= 2).then(|| {
            let mean = self.differences.iter().sum::<f64>() / n as f64;
            1.96 * (self
                .differences
                .iter()
                .map(|d| (d - mean).powi(2))
                .sum::<f64>()
                / (n - 1) as f64
                / n as f64)
                .sqrt()
        })
    }
    fn wtl(&self) -> String {
        format!("{} / {} / {}", self.win, self.tie, self.loss)
    }
}
fn number(value: Option<f64>) -> String {
    value
        .map(|v| format!("{v:+.4}"))
        .unwrap_or_else(|| "—".into())
}

fn load(path: &Path, excluded: &[String]) -> Result<Run> {
    let manifest: Manifest = serde_json::from_reader(File::open(path.join("run.json"))?)?;
    let mut paths: Vec<_> = fs::read_dir(path)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .is_some_and(|s| s.to_string_lossy().ends_with(".pair.json"))
        })
        .collect();
    paths.sort();
    let mut pairs = paths
        .iter()
        .map(|p| serde_json::from_reader(File::open(p)?).map_err(anyhow::Error::from))
        .collect::<Result<Vec<Pair>>>()?;
    for pair in &pairs {
        ensure!(
            pair.seed == manifest.seed && pair.workers == manifest.workers,
            "pair options differ from manifest"
        );
        ensure!(
            manifest.task_ids.contains(&pair.task_id)
                && manifest.strategies.contains(&pair.strategy),
            "pair is outside manifest selection"
        );
    }
    let excluded_tasks: Vec<_> = excluded
        .iter()
        .filter(|id| manifest.task_ids.contains(id))
        .cloned()
        .collect();
    pairs.retain(|p| !excluded_tasks.contains(&p.task_id));
    Ok(Run {
        name: path
            .file_name()
            .context("run directory has no name")?
            .to_string_lossy()
            .into(),
        manifest,
        pairs,
        excluded_tasks,
    })
}

fn method_table(file: &mut File, rows: &[&Pair]) -> Result<()> {
    let mut grouped: BTreeMap<&str, Vec<&Pair>> = BTreeMap::new();
    for row in rows {
        grouped.entry(&row.strategy).or_default().push(row);
    }
    writeln!(
        file,
        "| Strategy | Valid pairs / attempts | vs plain mean pp | 95% approx. half-width pp | W / T / L | vs Lectra mean pp | W / T / L | Failed plain / variant | Timeout plain / variant | Mean wall plain / variant, s |\n|---|---:|---:|---:|---|---:|---|---|---|---|"
    )?;
    for (strategy, rows) in grouped {
        let mut plain = Stats::default();
        let mut lectra = Stats::default();
        for row in &rows {
            plain.add(row.variant.score(), row.plain.score());
            if row.lectra_pct.is_some() {
                lectra.add(row.variant.score(), row.lectra_pct);
            }
        }
        let n = rows.len() as f64;
        writeln!(
            file,
            "| {strategy} | {} / {} | {} | {} | {} | {} | {} | {} / {} | {} / {} | {:.2} / {:.2} |",
            plain.differences.len(),
            rows.len(),
            number(plain.mean()),
            number(plain.half_width()),
            plain.wtl(),
            number(lectra.mean()),
            if lectra.mean().is_some() {
                lectra.wtl()
            } else {
                "—".into()
            },
            rows.iter().filter(|r| !r.plain.valid).count(),
            rows.iter().filter(|r| !r.variant.valid).count(),
            rows.iter().filter(|r| r.plain.timeout).count(),
            rows.iter().filter(|r| r.variant.timeout).count(),
            rows.iter().map(|r| r.plain.wall_s).sum::<f64>() / n,
            rows.iter().map(|r| r.variant.wall_s).sum::<f64>() / n
        )?;
        ensure!(
            plain.win + plain.tie + plain.loss + plain.both_failed == rows.len(),
            "comparison count mismatch"
        );
    }
    Ok(())
}

fn write_run(file: &mut File, run: &Run) -> Result<()> {
    let expected = run.manifest.task_ids.len() * run.manifest.strategies.len();
    let complete = run.pairs.len() == expected;
    writeln!(
        file,
        "\n## {}\n\nRevision `{}`. Split: {}. {} of {} pairs complete. {}\n\nSeed {}; {} workers per trial; {} concurrent pairs. Wall grace: {} s.\n",
        run.name,
        run.manifest.revision,
        run.manifest.split,
        run.pairs.len(),
        expected,
        if complete {
            "Complete selected matrix."
        } else {
            "Incomplete: not eligible for strategy selection."
        },
        run.manifest.seed,
        run.manifest.workers,
        run.manifest.jobs,
        if run.manifest.wall_grace_s > 0.0 {
            run.manifest.wall_grace_s.to_string()
        } else {
            "uncapped legacy".into()
        }
    )?;
    if !run.excluded_tasks.is_empty() {
        writeln!(
            file,
            "Excluded whole task blocks: {}. All their strategies are excluded; original raw evidence remains untouched. The selected matrix remains incomplete.\n",
            run.excluded_tasks.join(", ")
        )?;
    }
    let mut budgets: Vec<_> = run.pairs.iter().map(|p| p.budget_s).collect();
    budgets.sort_by(f64::total_cmp);
    if let (Some(first), Some(last)) = (budgets.first(), budgets.last()) {
        let middle = budgets.len() / 2;
        let median = if budgets.len().is_multiple_of(2) {
            (budgets[middle - 1] + budgets[middle]) / 2.0
        } else {
            budgets[middle]
        };
        writeln!(
            file,
            "Search budget min / median / max: {first:.1} / {median:.1} / {last:.1} s.\n"
        )?;
    }
    method_table(file, &run.pairs.iter().collect::<Vec<_>>())?;
    writeln!(
        file,
        "\n| Strategy | Variant-first pairs | Plain fallbacks | Active pair construction |\n|---|---:|---:|---:|"
    )?;
    for strategy in &run.manifest.strategies {
        let rows: Vec<_> = run
            .pairs
            .iter()
            .filter(|p| p.strategy == *strategy)
            .collect();
        let fallback = rows
            .iter()
            .filter(|p| {
                p.variant
                    .warnings
                    .iter()
                    .any(|w| w.contains("candidate failed; plain fallback"))
            })
            .count();
        let paired = rows
            .iter()
            .filter(|p| {
                p.variant.warnings.iter().any(|w| {
                    w.strip_prefix("Cluster construction used ")
                        .and_then(|s| s.split_whitespace().next())
                        .and_then(|s| s.parse::<usize>().ok())
                        .is_some_and(|n| n > 0)
                })
            })
            .count();
        writeln!(
            file,
            "| {strategy} | {} | {fallback} | {paired} |",
            rows.iter().filter(|p| p.variant_first).count()
        )?;
    }
    if run.manifest.split == "freesewing" {
        writeln!(
            file,
            "\n| Style | Strategy | Budget s | Plain % | Variant % | Difference pp | Valid plain / variant |\n|---|---|---:|---:|---:|---:|---|"
        )?;
        for pair in &run.pairs {
            writeln!(
                file,
                "| {} | {} | {:.0} | {} | {} | {} | {} / {} |",
                pair.task_id,
                pair.strategy,
                pair.budget_s,
                number(pair.plain.score()),
                number(pair.variant.score()),
                number(
                    pair.variant
                        .score()
                        .zip(pair.plain.score())
                        .map(|(a, b)| a - b)
                ),
                pair.plain.valid,
                pair.variant.valid
            )?;
        }
    } else {
        let mut groups: BTreeMap<&str, Vec<&Pair>> = BTreeMap::new();
        for pair in &run.pairs {
            groups.entry(&pair.stratum).or_default().push(pair);
        }
        for (stratum, rows) in groups {
            writeln!(file, "\n### {stratum}\n")?;
            method_table(file, &rows)?;
        }
    }
    Ok(())
}

fn compare_budgets(file: &mut File, runs: &[Run]) -> Result<()> {
    for matched in runs.iter().filter(|r| r.manifest.split == "matched") {
        for short in runs.iter().filter(|r| {
            r.manifest.split == "confirm"
                && r.manifest.revision == matched.manifest.revision
                && r.manifest.seed == matched.manifest.seed
        }) {
            let mut plain = Stats::default();
            let mut variant = Stats::default();
            for long in &matched.pairs {
                if let Some(quick) = short
                    .pairs
                    .iter()
                    .find(|p| p.task_id == long.task_id && p.strategy == long.strategy)
                {
                    plain.add(long.plain.score(), quick.plain.score());
                    variant.add(long.variant.score(), quick.variant.score());
                }
            }
            writeln!(
                file,
                "\n## Paired effect of extra time\n\n{} versus {} on shared task IDs and strategy. Differences are long minus short.\n\n| Search | Valid pairs | Mean gain pp | W / T / L |\n|---|---:|---:|---|\n| Plain | {} | {} | {} |\n| Variant | {} | {} | {} |",
                matched.name,
                short.name,
                plain.differences.len(),
                number(plain.mean()),
                plain.wtl(),
                variant.differences.len(),
                number(variant.mean()),
                variant.wtl()
            )?;
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        args.len() >= 2,
        "usage: analyze OUTPUT.md RUN_DIRECTORY... [--exclude-task ID]..."
    );
    let output = PathBuf::from(&args[0]);
    let mut paths = Vec::new();
    let mut excluded = Vec::new();
    let mut arguments = args.iter().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--exclude-task" {
            let id = arguments.next().context("--exclude-task needs an ID")?;
            ensure!(!excluded.contains(id), "duplicate excluded task ID");
            excluded.push(id.clone());
        } else {
            paths.push(argument);
        }
    }
    ensure!(!paths.is_empty(), "at least one run directory is required");
    let runs = paths
        .iter()
        .map(|p| load(Path::new(p), &excluded))
        .collect::<Result<Vec<_>>>()?;
    ensure!(
        excluded
            .iter()
            .all(|id| runs.iter().any(|r| r.manifest.task_ids.contains(id))),
        "excluded task ID is outside all selections"
    );
    let mut file = File::create(&output)?;
    writeln!(
        file,
        "# Paired benchmark evidence\n\nAll arithmetic is computed in Rust from saved pair metadata. Ties are within 0.1 percentage point. Means and normal-approximation intervals use valid pairs only. Single-sided failures count as wins/losses; both-failed pairs are excluded from W/T/L and remain failures. These are fixed samples, not population estimates. Lectra comparisons retain the existing sheet-type and orientation assumptions.\n"
    )?;
    for run in &runs {
        write_run(&mut file, run)?;
    }
    compare_budgets(&mut file, &runs)?;
    Ok(())
}
