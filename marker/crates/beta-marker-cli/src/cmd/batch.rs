//! Offline marker queue with width trials and independent result validation.
use anyhow::{Context, Result, ensure};
use beta_marker_core::{Job, MarkerResult};
use serde::Serialize;
use std::fs::{self, File};
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Instant;

/// Arguments for `beta-marker batch`.
#[derive(clap::Args)]
pub struct Args {
    /// Folder of job JSONs, one job JSON, or a text file of job paths.
    pub input: PathBuf,
    /// Trial widths in millimetres. Omit to use each job's width.
    #[arg(long, value_delimiter = ',')]
    pub widths: Vec<f64>,
    /// Search seconds per run. Construction and checking add overhead.
    #[arg(long = "time", default_value_t = 60.0)]
    pub time_s: f64,
    /// Independent seeds per job and width; keep the shortest valid marker.
    #[arg(long, default_value_t = 1)]
    pub runs: u32,
    /// Concurrent job/width trials. Each nest uses two separator workers.
    #[arg(long, default_value_t = 1)]
    pub jobs: usize,
    /// Base random seed.
    #[arg(long, default_value_t = 42)]
    pub seed: u64,
    /// Destination folder for results and summaries.
    #[arg(short, long)]
    pub output: PathBuf,
}

#[derive(Serialize)]
struct Row {
    job: String,
    width_mm: f64,
    length_mm: Option<f64>,
    efficiency: Option<f64>,
    runtime_s: f64,
    valid: bool,
    successful_runs: u32,
    result_file: Option<String>,
    errors: Vec<String>,
}

fn inputs(path: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let p = entry?.path();
            if !p.is_file()
                || !p
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("json"))
            {
                continue;
            }
            // A directory may contain reports and marker results. Those are not jobs.
            let value =
                serde_json::from_reader::<_, serde_json::Value>(BufReader::new(File::open(&p)?));
            if value.as_ref().is_ok_and(|v| {
                v.get("format").and_then(|f| f.as_str()) != Some(beta_marker_core::JOB_FORMAT)
            }) {
                continue;
            }
            paths.push(p);
        }
    } else if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("json"))
    {
        paths.push(path.to_owned());
    } else {
        for line in fs::read_to_string(path)
            .with_context(|| format!("could not read {}", path.display()))?
            .lines()
        {
            let line = line.trim_start_matches('\u{feff}').trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let p = PathBuf::from(line);
            paths.push(if p.is_absolute() {
                p
            } else {
                path.parent().unwrap_or(Path::new(".")).join(p)
            });
        }
    }
    paths.sort();
    paths.dedup();
    ensure!(
        !paths.is_empty(),
        "no job files found in {}",
        path.display()
    );
    Ok(paths)
}

fn json(path: &Path, value: &impl Serialize) -> Result<()> {
    let mut file = std::io::BufWriter::new(File::create(path)?);
    serde_json::to_writer_pretty(&mut file, value)?;
    file.write_all(b"\n")?;
    file.flush()?;
    Ok(())
}

fn trial(index: usize, path: &Path, width: Option<f64>, args: &Args) -> Row {
    let started = Instant::now();
    let mut row = Row {
        job: path.to_string_lossy().into_owned(),
        width_mm: width.unwrap_or(0.0),
        length_mm: None,
        efficiency: None,
        runtime_s: 0.0,
        valid: false,
        successful_runs: 0,
        result_file: None,
        errors: Vec::new(),
    };
    let outcome = (|| -> Result<()> {
        let mut job: Job = serde_json::from_reader(BufReader::new(File::open(path)?))
            .context("invalid job JSON")?;
        if let Some(w) = width {
            job.fabric.width = w;
        }
        row.width_mm = job.fabric.width;
        for p in &job.pieces {
            let m = beta_marker_core::rules::multiplier(&job, p);
            ensure!(
                p.quantity.checked_mul(m).is_some() && p.mirror_quantity.checked_mul(m).is_some(),
                "piece count overflow: {}",
                p.id
            );
        }
        let issues = beta_marker_core::check_job(&job);
        ensure!(
            issues.is_empty(),
            "invalid job: {}",
            issues
                .iter()
                .map(|i| i.message.as_str())
                .collect::<Vec<_>>()
                .join("; ")
        );
        let mut best: Option<MarkerResult> = None;
        for run in 0..args.runs {
            let result = beta_marker_match::nest_with_rules(
                &job,
                &beta_marker_nest::NestOptions {
                    time_limit_s: args.time_s,
                    seed: Some(
                        args.seed
                            .wrapping_add((index as u64).wrapping_mul(u64::from(args.runs)))
                            .wrapping_add(u64::from(run)),
                    ),
                    workers: Some(2),
                    runs: 1,
                    ..Default::default()
                },
            );
            match result {
                Ok(mut r) => {
                    let report = beta_marker_core::validate(&job, &r);
                    if report.ok
                        && r.length.is_finite()
                        && r.length > 0.0
                        && r.efficiency.is_finite()
                    {
                        r.length = report.length;
                        r.piece_area = report.piece_area;
                        r.efficiency = report.efficiency;
                        r.warnings
                            .extend(report.warnings.into_iter().map(|w| w.message));
                        row.successful_runs += 1;
                        if best.as_ref().is_none_or(|b| r.length < b.length) {
                            best = Some(r);
                        }
                    } else {
                        row.errors.push(format!(
                            "run {} failed independent validation: {}",
                            run + 1,
                            serde_json::to_string(&report)?
                        ));
                    }
                }
                Err(e) => row.errors.push(format!("run {}: {e:#}", run + 1)),
            }
        }
        if let Some(r) = best {
            let stem = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect::<String>();
            let filename = format!("{index:04}-{stem}-w{:.3}.result.json", job.fabric.width);
            // Save the width-adjusted job beside the result for later validation.
            json(
                &args.output.join(format!(
                    "{index:04}-{stem}-w{:.3}.job.json",
                    job.fabric.width
                )),
                &job,
            )?;
            json(&args.output.join(&filename), &r)?;
            row.valid = true;
            row.length_mm = Some(r.length);
            row.efficiency = Some(r.efficiency);
            row.result_file = Some(filename);
        }
        Ok(())
    })();
    if let Err(e) = outcome {
        row.errors.push(format!("{e:#}"));
    }
    if !row.width_mm.is_finite() {
        row.width_mm = 0.0;
    }
    row.runtime_s = started.elapsed().as_secs_f64();
    row
}

fn csv(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}
fn md(s: &str) -> String {
    s.replace('|', "\\|").replace(['\n', '\r'], " ")
}

fn summaries(output: &Path, rows: &[Row]) -> Result<()> {
    json(&output.join("summary.json"), &rows)?;
    let mut csv_out = String::from(
        "job,width_mm,length_mm,efficiency,runtime_s,valid,successful_runs,best_width,result_file,error\n",
    );
    let mut markdown = String::from(
        "# Marker queue\n\nEfficiency is the fraction of fabric occupied by pattern pieces. Runtime includes all runs and validation. Best width uses the least fabric area (width × length); length breaks ties.\n\n| Job | Width (mm) | Length (m) | Efficiency | Runtime (s) | Valid | Best width |\n|---|---:|---:|---:|---:|---|---|\n",
    );
    let mut best: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for (i, r) in rows.iter().enumerate().filter(|(_, r)| r.valid) {
        let area = r.width_mm * r.length_mm.unwrap_or(f64::INFINITY);
        if best.get(r.job.as_str()).is_none_or(|j| {
            let b = &rows[*j];
            area.total_cmp(&(b.width_mm * b.length_mm.unwrap_or(f64::INFINITY)))
                .then_with(|| {
                    r.length_mm
                        .unwrap_or(f64::INFINITY)
                        .total_cmp(&b.length_mm.unwrap_or(f64::INFINITY))
                })
                .is_lt()
        }) {
            best.insert(&r.job, i);
        }
    }
    for (i, r) in rows.iter().enumerate() {
        let is_best = best.get(r.job.as_str()) == Some(&i);
        let length = r.length_mm.map(|n| format!("{n:.3}")).unwrap_or_default();
        let efficiency = r.efficiency.map(|n| format!("{n:.6}")).unwrap_or_default();
        csv_out.push_str(&format!(
            "{},{:.3},{},{},{:.3},{},{},{},{},{}\n",
            csv(&r.job),
            r.width_mm,
            length,
            efficiency,
            r.runtime_s,
            r.valid,
            r.successful_runs,
            is_best,
            csv(r.result_file.as_deref().unwrap_or("")),
            csv(&r.errors.join("; "))
        ));
        markdown.push_str(&format!(
            "| {} | {:.1} | {} | {} | {:.2} | {} | {} |\n",
            md(&r.job),
            r.width_mm,
            r.length_mm
                .map(|n| format!("{:.3}", n / 1000.0))
                .unwrap_or_else(|| "—".into()),
            r.efficiency
                .map(|n| format!("{:.2}%", n * 100.0))
                .unwrap_or_else(|| "—".into()),
            r.runtime_s,
            r.valid,
            if is_best { "best" } else { "" }
        ));
    }
    markdown.push_str("\n## Best width per job\n\n");
    for (job, i) in &best {
        let r = &rows[*i];
        markdown.push_str(&format!(
            "- {}: {:.1} mm; {:.3} m² per marker.\n",
            md(job),
            r.width_mm,
            r.width_mm * r.length_mm.unwrap_or(0.0) / 1e6
        ));
    }
    for r in rows.iter().filter(|r| !r.errors.is_empty()) {
        markdown.push_str(&format!(
            "\n- {} at {:.1} mm: {}\n",
            md(&r.job),
            r.width_mm,
            md(&r.errors.join("; "))
        ));
    }
    json(
        &output.join("best-widths.json"),
        &best.values().map(|i| &rows[*i]).collect::<Vec<_>>(),
    )?;
    fs::write(output.join("summary.csv"), csv_out)?;
    fs::write(output.join("summary.md"), markdown)?;
    Ok(())
}

/// Run all trials, write results and summaries, then fail if any trial failed.
pub fn run(args: Args) -> Result<()> {
    ensure!(
        args.time_s.is_finite() && args.time_s > 0.0 && args.time_s <= 86_400.0,
        "time must be in (0, 86400]"
    );
    ensure!(
        args.runs > 0 && args.jobs > 0,
        "runs and jobs must be positive"
    );
    ensure!(
        args.widths.iter().all(|w| w.is_finite() && *w > 0.0),
        "widths must be finite and positive"
    );
    let paths = inputs(&args.input)?;
    let mut widths = args.widths.clone();
    widths.sort_by(f64::total_cmp);
    widths.dedup();
    let widths: Vec<_> = if widths.is_empty() {
        vec![None]
    } else {
        widths.into_iter().map(Some).collect()
    };
    let tasks: Vec<_> = paths
        .iter()
        .flat_map(|p| widths.iter().map(move |w| (p, *w)))
        .collect();
    fs::create_dir_all(&args.output)?;
    let next = AtomicUsize::new(0);
    let results = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..args.jobs.min(tasks.len()) {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some((path, width)) = tasks.get(index) else {
                        break;
                    };
                    let row = trial(index, path, *width, &args);
                    eprintln!(
                        "{} at {:.1} mm: {} ({:.2}s)",
                        path.display(),
                        row.width_mm,
                        if row.valid { "valid" } else { "failed" },
                        row.runtime_s
                    );
                    if let Ok(mut rows) = results.lock() {
                        rows.push((index, row));
                    }
                }
            });
        }
    });
    let mut rows = results
        .into_inner()
        .map_err(|_| anyhow::anyhow!("batch results lock was poisoned"))?;
    rows.sort_by_key(|(i, _)| *i);
    let rows: Vec<_> = rows.into_iter().map(|(_, r)| r).collect();
    summaries(&args.output, &rows)?;
    let failed = rows.iter().filter(|r| !r.valid).count();
    println!(
        "Trials: {}; valid: {}; failed: {}; summary: {}",
        rows.len(),
        rows.len() - failed,
        failed,
        args.output.join("summary.md").display()
    );
    ensure!(
        failed == 0,
        "{failed} batch trials failed; summaries include error details"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn test_dir(suffix: &str) -> PathBuf {
        std::env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join(format!("beta-plan-{suffix}-{}", std::process::id()))
    }
    #[test]
    fn escapes_output_fields() {
        assert_eq!(csv("a,\"b"), "\"a,\"\"b\"");
        assert_eq!(md("a|b\nc"), "a\\|b c");
    }
    #[test]
    fn best_width_uses_fabric_area_and_keeps_failures() -> Result<()> {
        let dir = test_dir("summary");
        fs::create_dir_all(&dir)?;
        let rows = vec![
            Row {
                job: "shirt|a".into(),
                width_mm: 1000.0,
                length_mm: Some(2000.0),
                efficiency: Some(0.7),
                runtime_s: 1.0,
                valid: true,
                successful_runs: 1,
                result_file: Some("a.json".into()),
                errors: vec![],
            },
            Row {
                job: "shirt|a".into(),
                width_mm: 1500.0,
                length_mm: Some(1500.0),
                efficiency: Some(0.6),
                runtime_s: 1.0,
                valid: true,
                successful_runs: 1,
                result_file: Some("b.json".into()),
                errors: vec![],
            },
            Row {
                job: "bad".into(),
                width_mm: 1500.0,
                length_mm: None,
                efficiency: None,
                runtime_s: 1.0,
                valid: false,
                successful_runs: 0,
                result_file: None,
                errors: vec!["bad input".into()],
            },
        ];
        summaries(&dir, &rows)?;
        let best: serde_json::Value =
            serde_json::from_reader(File::open(dir.join("best-widths.json"))?)?;
        assert_eq!(best[0]["width_mm"], 1000.0);
        let text = fs::read_to_string(dir.join("summary.csv"))?;
        assert!(text.contains("bad input"));
        for f in [
            "summary.json",
            "summary.csv",
            "summary.md",
            "best-widths.json",
        ] {
            fs::remove_file(dir.join(f))?;
        }
        fs::remove_dir(dir)?;
        Ok(())
    }

    #[test]
    fn folder_and_list_inputs_and_failed_trials_are_preserved() -> Result<()> {
        let dir = test_dir("inputs");
        fs::create_dir_all(&dir)?;
        let job = dir.join("bad.job.json");
        fs::write(&job, r#"{"format":"beta-marker.job/v1"}"#)?;
        let report = dir.join("report.json");
        fs::write(&report, r#"{"format":"beta-marker.result/v1"}"#)?;
        assert_eq!(inputs(&dir)?, vec![job.clone()]);
        let list = dir.join("list.txt");
        fs::write(&list, "\u{feff}# jobs\n\nbad.job.json\nbad.job.json\n")?;
        assert_eq!(inputs(&list)?, vec![job.clone()]);
        let out = dir.join("out");
        assert!(
            run(Args {
                input: list.clone(),
                widths: vec![1450.0, 1500.0],
                time_s: 0.01,
                runs: 1,
                jobs: 2,
                seed: 42,
                output: out.clone()
            })
            .is_err()
        );
        let rows: serde_json::Value =
            serde_json::from_reader(File::open(out.join("summary.json"))?)?;
        assert_eq!(rows.as_array().map(Vec::len), Some(2));
        assert_eq!(rows[0]["valid"], false);
        assert_eq!(rows[1]["valid"], false);
        for f in [
            "summary.json",
            "summary.csv",
            "summary.md",
            "best-widths.json",
        ] {
            fs::remove_file(out.join(f))?;
        }
        fs::remove_dir(out)?;
        fs::remove_file(job)?;
        fs::remove_file(report)?;
        fs::remove_file(list)?;
        fs::remove_dir(dir)?;
        Ok(())
    }
}
