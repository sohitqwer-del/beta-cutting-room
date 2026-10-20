//! Paired continuous/sliced search quality. All output stays under BETA_DATA_DIR.
//! Run with a JSON array of six public job paths (Simon + five Lectra dev jobs).
use anyhow::{Context, Result};
use beta_marker_core::{Job, MarkerResult};
use beta_marker_nest::NestOptions;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, time::Instant};

#[derive(Serialize, Deserialize)]
struct Row {
    job: String,
    seed: u64,
    strategy: String,
    search_s: f64,
    wall_s: f64,
    length_mm: Option<f64>,
    efficiency_pct: Option<f64>,
    valid: bool,
    error: Option<String>,
    slice_best_pct: Vec<f64>,
}
fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let manifest = PathBuf::from(args.first().context("Pass input-paths.json")?);
    let jobs: Vec<PathBuf> = serde_json::from_slice(&std::fs::read(manifest)?)?;
    let root = PathBuf::from(std::env::var("BETA_DATA_DIR")?).join("marker-bench/ops/slice-bench");
    std::fs::create_dir_all(&root)?;
    // Explicit resume keeps completed failures as well as successes. Use the
    // same immutable input manifest; never silently re-run only losing cells.
    let mut rows: Vec<Row> =
        if args.iter().any(|a| a == "--resume") && root.join("summary.json").exists() {
            serde_json::from_slice(&std::fs::read(root.join("summary.json"))?)?
        } else {
            Vec::new()
        };
    let seed = 42;
    for (i, path) in jobs.iter().enumerate() {
        let job: Job = serde_json::from_slice(&std::fs::read(path)?)?;
        // Rotate strategy order across jobs to reduce order/load confounding.
        let strategies = [
            ("continuous60", 60.0, 1usize),
            ("warm12x5", 5.0, 12),
            ("warm4x15", 15.0, 4),
        ];
        for k in 0..3 {
            let (strategy, seconds, slices) = strategies[(i + k) % 3];
            if rows
                .iter()
                .any(|r| r.job == job.name && r.seed == seed && r.strategy == strategy)
            {
                continue;
            }
            let started = Instant::now();
            let mut best: Option<MarkerResult> = None;
            let mut curve = Vec::new();
            let mut error = None;
            for slice in 0..slices {
                let options = NestOptions {
                    time_limit_s: seconds,
                    workers: Some(2),
                    seed: Some(seed),
                    warm_start: best.clone(),
                    ..Default::default()
                };
                let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    beta_marker_match::nest_with_rules(&job, &options)
                }));
                let outcome = match outcome {
                    Ok(value) => value,
                    Err(_) => Err(anyhow::anyhow!("Engine panicked")),
                };
                match outcome {
                    Ok(result) => {
                        let check = beta_marker_core::validate(&job, &result);
                        if !check.ok {
                            error = Some(format!("Invalid benchmark result: {:?}", check.errors));
                            break;
                        }
                        if best.as_ref().is_none_or(|b| result.length < b.length) {
                            best = Some(result);
                        }
                        let retained = best.as_ref().context("No checked best")?;
                        curve.push(retained.efficiency * 100.0);
                        std::fs::write(
                            root.join(format!("{i}-{strategy}-checkpoint.json")),
                            serde_json::to_vec_pretty(retained)?,
                        )?;
                        eprintln!(
                            "{} {} slice {}/{}: {:.4}% ({:.2} wall s)",
                            job.name,
                            strategy,
                            slice + 1,
                            slices,
                            retained.efficiency * 100.0,
                            started.elapsed().as_secs_f64()
                        );
                    }
                    Err(e) => {
                        error = Some(format!("{e:#}"));
                        break;
                    }
                }
            }
            rows.push(Row {
                job: job.name.clone(),
                seed,
                strategy: strategy.into(),
                search_s: seconds * curve.len() as f64,
                wall_s: started.elapsed().as_secs_f64(),
                length_mm: best.as_ref().map(|m| m.length),
                efficiency_pct: best.as_ref().map(|m| m.efficiency * 100.0),
                valid: best.is_some() && error.is_none(),
                error,
                slice_best_pct: curve,
            });
            std::fs::write(root.join("summary.json"), serde_json::to_vec_pretty(&rows)?)?;
        }
    }
    Ok(())
}
