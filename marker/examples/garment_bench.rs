//! Seed-1 garment benchmark. Usage: garment_bench <job.json> <seconds> <output-dir>
//! Writes the result, independent validation, every valid improvement and the
//! best valid efficiency available at each requested wall-clock checkpoint.
use anyhow::{Result, ensure};
use beta_marker_core::{Job, compute_metrics, validate};
use beta_marker_nest::{NestOptions, nest_with_progress};
use std::path::PathBuf;
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        args.len() == 3,
        "usage: garment_bench <job.json> <seconds> <output-dir>"
    );
    let job: Job = serde_json::from_reader(std::fs::File::open(&args[0])?)?;
    let seconds: f64 = args[1].parse()?;
    let output = PathBuf::from(&args[2]);
    std::fs::create_dir_all(&output)?;
    let mut progress = vec![];
    let result = nest_with_progress(
        &job,
        &NestOptions {
            time_limit_s: seconds,
            seed: Some(1),
            workers: Some(3),
            early_termination: false,
            ..Default::default()
        },
        |p| {
            println!("{:.3}s {:.3}mm", p.seconds, p.length);
            progress.push(p);
        },
    )?;
    let report = validate(&job, &result);
    let checkpoints:Vec<_>=[10.,30.,60.,120.,300.].iter().filter(|&&t|t<=seconds).map(|&t|{
        let best=progress.iter().filter(|p|p.seconds<=t).min_by(|a,b|a.length.total_cmp(&b.length));
        serde_json::json!({"seconds":t,"length":best.map(|p|p.length),"efficiency":best.map(|p|result.piece_area/(job.fabric.width*p.length))})
    }).collect();
    let metrics = compute_metrics(&job, &result.placements);
    let summary = serde_json::json!({"job":job.name,"width":job.fabric.width,"budget_s":seconds,"seed":1,"workers":3,"length":metrics.length,"efficiency":metrics.efficiency,"runtime_s":result.runtime_s,"valid":report.ok,"warnings":result.warnings,"checkpoints":checkpoints});
    for (name, value) in [
        ("result.json", serde_json::to_value(&result)?),
        ("validation.json", serde_json::to_value(&report)?),
        ("progress.json", serde_json::to_value(&progress)?),
        ("summary.json", summary.clone()),
    ] {
        std::fs::write(output.join(name), serde_json::to_vec_pretty(&value)?)?;
    }
    println!("{}", serde_json::to_string_pretty(&summary)?);
    ensure!(report.ok, "invalid benchmark result");
    Ok(())
}
