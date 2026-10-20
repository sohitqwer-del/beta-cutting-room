//! `beta-marker validate`: check a result independently of the nesting engine.
use anyhow::Context;
use beta_marker_core::{Job, MarkerResult, validate};
use std::{fs::File, path::PathBuf};

/// Inputs and output style for marker validation.
#[derive(clap::Args)]
pub struct Args {
    /// Job JSON file.
    pub job: PathBuf,
    /// Marker result JSON file.
    pub result: PathBuf,
    /// Print the machine-readable validation report.
    #[arg(long)]
    pub json: bool,
}
/// Print validation findings. Invalid markers return an error (CLI exit code 1).
pub fn run(args: Args) -> anyhow::Result<()> {
    let job: Job = serde_json::from_reader(
        File::open(&args.job).with_context(|| format!("Cannot open {}", args.job.display()))?,
    )
    .context("Cannot read job JSON")?;
    let result: MarkerResult = serde_json::from_reader(
        File::open(&args.result)
            .with_context(|| format!("Cannot open {}", args.result.display()))?,
    )
    .context("Cannot read result JSON")?;
    let report = validate(&job, &result);
    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!(
            "{}: length {:.3} mm, efficiency {:.2}%",
            if report.ok { "Valid" } else { "Invalid" },
            report.length,
            report.efficiency * 100.0
        );
        for issue in &report.errors {
            println!("Error [{}]: {}", issue.code, issue.message);
        }
        for issue in &report.warnings {
            println!("Warning [{}]: {}", issue.code, issue.message);
        }
    }
    anyhow::ensure!(
        report.ok,
        "Marker validation failed with {} error(s).",
        report.errors.len()
    );
    Ok(())
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
    fn parses_json_flag_and_propagates_validation_errors() {
        let fixtures =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../beta-marker-export/tests/fixtures");
        let command = Command::try_parse_from(["validate", "job.json", "result.json", "--json"])
            .expect("validate arguments");
        assert!(command.args.json);
        assert!(
            run(Args {
                job: fixtures.join("calibration-job.json"),
                result: fixtures.join("calibration-result.json"),
                json: true
            })
            .is_ok()
        );
        assert!(
            run(Args {
                job: fixtures.join("calibration-job.json"),
                result: fixtures.join("missing-result.json"),
                json: false
            })
            .is_err()
        );
    }
}
