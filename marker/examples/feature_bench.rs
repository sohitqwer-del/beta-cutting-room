//! Checked v0.2 measurements. No source dataset is changed.
//! Usage: feature_bench <job.json> <seconds> <output-dir> <feature> [runs=1]
use anyhow::{Context, Result, bail, ensure};
use beta_marker_core::{Job, Layout, Nap, Shrinkage, Zone, validate};
use beta_marker_nest::{NestOptions, nest, nest_with_progress};
use std::path::PathBuf;

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        (4..=5).contains(&args.len()),
        "usage: feature_bench <job.json> <seconds> <output-dir> <baseline|fixed|warm|defects|bundles|faceless|folded|tubular|blocks|shrinkage> [runs=1]"
    );
    let mut job: Job = serde_json::from_reader(std::fs::File::open(&args[0])?)?;
    let seconds: f64 = args[1].parse()?;
    let out = PathBuf::from(&args[2]);
    std::fs::create_dir_all(&out)?;
    let feature = &args[3];
    let runs = args.get(4).map(|s| s.parse()).transpose()?.unwrap_or(1);
    let mut options = NestOptions {
        time_limit_s: seconds,
        seed: Some(1),
        workers: Some(3),
        runs,
        ..Default::default()
    };
    match feature.as_str() {
        "baseline" => {}
        "fixed" | "warm" => {
            let mut previous = nest(
                &job,
                &NestOptions {
                    time_limit_s: 2.0,
                    seed: Some(1),
                    workers: Some(2),
                    ..Default::default()
                },
            )?;
            if feature == "fixed" {
                let count = previous.placements.len().div_ceil(3);
                for p in previous.placements.iter_mut().take(count) {
                    p.locked = true;
                }
            }
            std::fs::write(
                out.join("previous.json"),
                serde_json::to_vec_pretty(&previous)?,
            )?;
            if feature == "warm" {
                options.warm_start = Some(previous);
            } else {
                options.fixed = previous
                    .placements
                    .iter()
                    .filter(|p| p.locked)
                    .cloned()
                    .collect();
            }
        }
        "defects" => {
            job.fabric.defects.push(Zone {
                id: "test flaw".into(),
                polygon: vec![
                    [100.0, job.fabric.width * 0.35],
                    [300.0, job.fabric.width * 0.35],
                    [300.0, job.fabric.width * 0.55],
                    [100.0, job.fabric.width * 0.55],
                ],
                note: Some("Benchmark obstacle, not a source-data defect.".into()),
            });
        }
        "bundles" => job.fabric.nap = Nap::OneWayPerBundle,
        "faceless" => job.fabric.allow_flip = true,
        "folded" | "tubular" => {
            job.fabric.layout = if feature == "folded" {
                Layout::Folded
            } else {
                Layout::Tubular
            };
            job.fabric.width /= 2.0;
            // FreeSewing Teagan full front/back are symmetric about local x=0.
            // The fixture's sleeves already request a normal/mirrored pair.
            ensure!(
                job.name.to_lowercase().contains("teagan"),
                "fold conversion is defined for the open FreeSewing Teagan job"
            );
            for piece in &mut job.pieces {
                if matches!(piece.name.as_str(), "front" | "back") {
                    let b = beta_marker_core::geometry::bbox(&piece.outline)
                        .context("empty outline")?;
                    ensure!(
                        (b[0] + b[2]).abs() < 0.001,
                        "Teagan torso is not centered on x=0"
                    );
                    piece.fold_line = Some([[0.0, b[1]], [0.0, b[3]]]);
                }
            }
        }
        "blocks" => {
            for p in &mut job.pieces {
                p.block_margin = Some(5.0);
            }
        }
        "shrinkage" => {
            job.fabric.shrinkage = Shrinkage {
                length_pct: 2.0,
                width_pct: 1.0,
            }
        }
        _ => bail!("unknown feature '{feature}'"),
    }
    std::fs::write(out.join("job.json"), serde_json::to_vec_pretty(&job)?)?;
    let effective = beta_marker_core::rules::effective_job(&job);
    std::fs::write(
        out.join("effective-job.json"),
        serde_json::to_vec_pretty(&effective)?,
    )?;
    let mut progress = Vec::new();
    let result = nest_with_progress(&job, &options, |p| {
        println!("{:.3}s {:.3}mm", p.seconds, p.length);
        progress.push(p);
    })?;
    let report = validate(&job, &result);
    let layers = if job.fabric.layout == Layout::Open {
        1.0
    } else {
        2.0
    };
    let demanded_area: f64 = beta_marker_core::demands(&effective)
        .iter()
        .map(|d| {
            beta_marker_core::geometry::area(&effective.pieces[d.piece_index].outline)
                * f64::from(d.count)
                / layers
        })
        .sum();
    let demand_efficiency = demanded_area / (job.fabric.width * result.length);
    let summary = serde_json::json!({"feature":feature,"job":job.name,"budget_s":seconds,"runs":runs,
        "base_seed":1,"winning_seed":result.seed,"width":job.fabric.width,"length":result.length,"efficiency":result.efficiency,
        "runtime_s":result.runtime_s,"valid":report.ok,"demand_efficiency":demand_efficiency,
        "demanded_spread_area":demanded_area,"surplus_spread_area":(result.piece_area-demanded_area).max(0.0),"warnings":result.warnings});
    for (name, value) in [
        ("result.json", serde_json::to_value(&result)?),
        ("validation.json", serde_json::to_value(&report)?),
        ("progress.json", serde_json::to_value(progress)?),
        ("options.json", serde_json::to_value(options)?),
        ("summary.json", summary.clone()),
    ] {
        std::fs::write(out.join(name), serde_json::to_vec_pretty(&value)?)?;
    }
    println!("{}", serde_json::to_string_pretty(&summary)?);
    ensure!(report.ok, "invalid benchmark result");
    Ok(())
}
