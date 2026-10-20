//! Read a garment job and write a checked marker.
use anyhow::{Context, Result, ensure};
use beta_marker_core::Job;
use beta_marker_nest::{NestOptions, nest};
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

/// Options for `beta-marker nest`.
#[derive(clap::Args)]
pub struct Args {
    /// Input job JSON.
    pub job: PathBuf,
    /// Search budget in seconds.
    #[arg(long = "time", default_value_t = 60.0)]
    pub time_limit_s: f64,
    /// Random seed.
    #[arg(long)]
    pub seed: Option<u64>,
    /// Separator worker threads.
    #[arg(long)]
    pub workers: Option<usize>,
    /// Stop after repeated unsuccessful search attempts.
    #[arg(long)]
    pub early_termination: bool,
    /// Independent seeded runs sharing the search budget.
    #[arg(long, default_value_t = 1)]
    pub runs: u32,
    /// Keep the locked placements from this result JSON.
    #[arg(long)]
    pub fixed: Option<PathBuf>,
    /// Start the search from this earlier result JSON.
    #[arg(long)]
    pub warm_start: Option<PathBuf>,
    /// Output result JSON.
    #[arg(short, long)]
    pub output: PathBuf,
}

/// Run the command. Errors reach main and produce exit status 1.
pub fn run(args: Args) -> Result<()> {
    let input =
        File::open(&args.job).with_context(|| format!("could not read {}", args.job.display()))?;
    let job: Job = serde_json::from_reader(BufReader::new(input)).context("invalid job JSON")?;
    // Avoid overflowing the core helper's u32 demand multiplication.
    for p in &job.pieces {
        let m = beta_marker_core::rules::multiplier(&job, p);
        ensure!(
            p.quantity.checked_mul(m).is_some() && p.mirror_quantity.checked_mul(m).is_some(),
            "invalid job: quantity overflows for {}",
            p.id
        );
    }
    let issues = beta_marker_core::check_job(&job);
    ensure!(
        issues.is_empty(),
        "invalid job: {}",
        issues
            .iter()
            .map(|i| format!("{}: {}", i.code, i.message))
            .collect::<Vec<_>>()
            .join("; ")
    );
    let result = nest(
        &job,
        &NestOptions {
            time_limit_s: args.time_limit_s,
            seed: args.seed,
            workers: args.workers,
            early_termination: args.early_termination,
            runs: args.runs,
            fixed: args
                .fixed
                .as_deref()
                .map(read_result)
                .transpose()?
                .map(|r| r.placements.into_iter().filter(|p| p.locked).collect())
                .unwrap_or_default(),
            warm_start: args.warm_start.as_deref().map(read_result).transpose()?,
        },
    )?;
    let mut writer = BufWriter::new(
        File::create(&args.output)
            .with_context(|| format!("could not write {}", args.output.display()))?,
    );
    serde_json::to_writer_pretty(&mut writer, &result)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    println!(
        "Length: {:.2} mm ({:.3} m)\nEfficiency: {:.2}%\nRuntime: {:.2} s",
        result.length,
        result.length / 1000.0,
        result.efficiency * 100.0,
        result.runtime_s
    );
    for warning in &result.warnings {
        eprintln!("Warning: {warning}");
    }
    Ok(())
}

fn read_result(path: &Path) -> Result<beta_marker_core::MarkerResult> {
    let file = File::open(path).with_context(|| format!("could not read {}", path.display()))?;
    serde_json::from_reader(BufReader::new(file))
        .with_context(|| format!("invalid result JSON in {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct Command {
        #[command(flatten)]
        args: Args,
    }

    #[test]
    fn accepts_portfolio_and_result_paths() {
        let parsed = Command::try_parse_from([
            "nest",
            "job.json",
            "--runs",
            "5",
            "--fixed",
            "fixed.json",
            "--warm-start",
            "warm.json",
            "-o",
            "result.json",
        ])
        .expect("valid command line");
        assert_eq!(parsed.args.runs, 5);
        assert_eq!(parsed.args.fixed, Some(PathBuf::from("fixed.json")));
        assert_eq!(parsed.args.warm_start, Some(PathBuf::from("warm.json")));
    }
}
