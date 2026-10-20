//! Short, reproducible comparison against the same pieces without special rules.
use anyhow::{Result, ensure};
use beta_marker_core::Job;
use beta_marker_match::{match_errors, nest_with_shade_search};
use beta_marker_nest::NestOptions;
use std::{fs, path::PathBuf};

fn main() -> Result<()> {
    let seconds: f64 = std::env::args()
        .nth(1)
        .map(|s| s.parse())
        .transpose()?
        .unwrap_or(0.5);
    let opts = NestOptions {
        time_limit_s: seconds,
        seed: Some(42),
        workers: Some(2),
        ..Default::default()
    };
    let filter = std::env::args().nth(2);
    let args: Vec<_> = std::env::args().collect();
    let output = args
        .windows(2)
        .find(|pair| pair[0] == "--out")
        .map(|pair| PathBuf::from(&pair[1]));
    if let Some(path) = &output {
        fs::create_dir_all(path)?;
    }
    let attempts = std::env::args()
        .nth(3)
        .map(|s| s.parse())
        .transpose()?
        .unwrap_or(1);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    println!(
        "| Job | Rules | Efficiency without | With | Loss (points) | Runtime without / with (s) | Valid |\n|---|---|---:|---:|---:|---:|---|"
    );
    for (file, label) in [
        ("simon-check.job.json", "40 mm x/y checks, 4 rules"),
        ("simon-shade-length.job.json", "3000 mm length bands"),
        ("simon-shade-width.job.json", "750 mm width lanes"),
    ] {
        if filter
            .as_ref()
            .is_some_and(|filter| !file.contains(filter.as_str()))
        {
            continue;
        }
        let job: Job = serde_json::from_slice(&fs::read(root.join(file))?)?;
        let mut free = job.clone();
        free.fabric.shade = None;
        free.match_rules.clear();
        let base = beta_marker_nest::nest(&free, &opts)?;
        let result = nest_with_shade_search(&job, &opts, attempts)?;
        let valid =
            beta_marker_core::validate(&job, &result).ok && match_errors(&job, &result).is_empty();
        ensure!(valid, "invalid measured result for {file}");
        if let Some(path) = &output {
            let folder = path.join(file.trim_end_matches(".job.json"));
            fs::create_dir_all(&folder)?;
            for (name, value) in [
                ("job.json", serde_json::to_value(&job)?),
                ("baseline.json", serde_json::to_value(&base)?),
                ("result.json", serde_json::to_value(&result)?),
                (
                    "validation.json",
                    serde_json::to_value(beta_marker_core::validate(&job, &result))?,
                ),
                (
                    "summary.json",
                    serde_json::json!({"budget_s":seconds,"attempts":attempts,"baseline_efficiency":base.efficiency,"efficiency":result.efficiency,"loss_pp":100.0*(base.efficiency-result.efficiency),"length":result.length,"runtime_s":result.runtime_s,"valid":valid}),
                ),
            ] {
                fs::write(folder.join(name), serde_json::to_vec_pretty(&value)?)?;
            }
        }
        eprintln!(
            "{file}: length {:.1} -> {:.1} mm",
            base.length, result.length
        );
        if let Some(shade) = job.fabric.shade {
            let axis = usize::from(shade.axis == beta_marker_core::ShadeAxis::Width);
            let mut bands =
                std::collections::BTreeMap::<i64, (std::collections::BTreeSet<u32>, f64)>::new();
            for pl in &result.placements {
                let Some(piece) = job.pieces.iter().find(|p| p.id == pl.piece_id) else {
                    continue;
                };
                let Some(b) = beta_marker_core::geometry::bbox(
                    &beta_marker_core::rules::placed_outline(piece, pl),
                ) else {
                    continue;
                };
                let band = (b[axis] / shade.band_mm).floor() as i64;
                let entry = bands.entry(band).or_default();
                if let Some(bundle) = pl.bundle {
                    entry.0.insert(bundle);
                }
                entry.1 = entry.1.max(b[2]);
            }
            for (band, (bundles, end)) in bands {
                let extent = if axis == 0 {
                    end - band as f64 * shade.band_mm
                } else {
                    end
                };
                eprintln!("  band {band}: bundles {bundles:?}, used length {extent:.1} mm");
            }
        }
        println!(
            "| {} | {label} | {:.2}% | {:.2}% | {:.2} | {:.3} / {:.3} | yes |",
            job.name,
            base.efficiency * 100.0,
            result.efficiency * 100.0,
            (base.efficiency - result.efficiency) * 100.0,
            base.runtime_s,
            result.runtime_s
        );
    }
    Ok(())
}
