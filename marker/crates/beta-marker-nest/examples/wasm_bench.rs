//! Native half of the browser comparison. Input/output paths are explicit.
//! Usage: wasm_bench <cases.json> <results.jsonl> [seconds=60] [workers=3]
use anyhow::{Context, Result};
use beta_marker_core::Job;
use beta_marker_nest::{NestOptions, nest};
use serde::Deserialize;
use std::{fs::File, io::Write, time::Instant};

#[derive(Deserialize)]
struct Case {
    id: String,
    job: Job,
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).is_some_and(|s| s == "--validate") {
        let cases: Vec<Case> =
            serde_json::from_reader(File::open(args.get(2).context("cases path")?)?)?;
        let input = std::fs::read_to_string(args.get(3).context("results path")?)?;
        let mut output = File::create(args.get(4).context("validated output path")?)?;
        for line in input.lines().filter(|line| !line.trim().is_empty()) {
            let mut row: serde_json::Value = serde_json::from_str(line)?;
            let case = cases
                .iter()
                .find(|case| row["id"] == case.id)
                .context("unknown case id")?;
            if let Some(value) = row.get("result") {
                let result = serde_json::from_value(value.clone())?;
                row["validation"] =
                    serde_json::to_value(beta_marker_core::validate(&case.job, &result))?;
            }
            writeln!(output, "{}", serde_json::to_string(&row)?)?;
        }
        return Ok(());
    }
    let cases: Vec<Case> =
        serde_json::from_reader(File::open(args.get(1).context("cases path")?)?)?;
    let mut output = File::create(args.get(2).context("output path")?)?;
    let seconds: f64 = args.get(3).map(|s| s.parse()).transpose()?.unwrap_or(60.0);
    let workers: usize = args.get(4).map(|s| s.parse()).transpose()?.unwrap_or(3);
    for case in cases {
        let started = Instant::now();
        let result = nest(
            &case.job,
            &NestOptions {
                time_limit_s: seconds,
                seed: Some(1),
                workers: Some(workers),
                ..Default::default()
            },
        );
        let row = match result {
            Ok(result) => serde_json::json!({
                "id": case.id, "budget_s": seconds, "workers": workers,
                "wall_s": started.elapsed().as_secs_f64(),
                "validation": beta_marker_core::validate(&case.job, &result),
                "result": result,
            }),
            Err(error) => serde_json::json!({
                "id": case.id, "budget_s": seconds, "workers": workers,
                "wall_s": started.elapsed().as_secs_f64(), "error": error.to_string(),
            }),
        };
        writeln!(output, "{}", serde_json::to_string(&row)?)?;
        output.flush()?;
        eprintln!(
            "{}: {:.2}s {}",
            row["id"],
            row["wall_s"],
            row.get("error").unwrap_or(&row["result"]["efficiency"])
        );
    }
    Ok(())
}
