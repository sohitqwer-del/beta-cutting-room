//! Reproduce a small geometry-free calibration. Run from the marker workspace root.
use anyhow::{Context, Result, ensure};
use beta_marker_core::Job;
use beta_marker_cutroom::costing::{
    Observation, accurate_estimate, consumption_mape, features, fit,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

fn main() -> Result<()> {
    let output = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .context("usage: calibrate <output-directory>")?;
    fs::create_dir_all(&output)?;
    let mut jobs: Vec<(String, Job, bool)> = Vec::new();
    for (name, holdout) in [
        ("freesewing/teagan-M", false),
        ("freesewing/titan-M", false),
        ("garmentcode/rand_07UUJUMCB1", false),
        ("garmentcode/rand_0B1IDD2JL0", true),
    ] {
        let job = serde_json::from_slice(&fs::read(format!("testdata/open/{name}.job.json"))?)?;
        jobs.push((name.into(), job, holdout));
    }
    let mut lectra_note = "BETA_DATA_DIR not set; no Lectra tasks used".to_string();
    if let Some(data) = std::env::var_os("BETA_DATA_DIR") {
        let path = PathBuf::from(data).join("marker-bench/lectra/sample-300.jsonl");
        if path.is_file() {
            let mut added = 0;
            for line in BufReader::new(fs::File::open(path)?).lines() {
                let v: Value = serde_json::from_str(&line?)?;
                if v["comparable"] != true || v["piece_count"].as_u64().unwrap_or(u64::MAX) > 25 {
                    continue;
                }
                let job: Job = serde_json::from_value(v["job"].clone())?;
                let id = format!(
                    "lectra/{}",
                    v["id"]
                        .as_str()
                        .map(str::to_string)
                        .unwrap_or_else(|| v["id"].to_string())
                );
                jobs.push((id, job, added == 1));
                added += 1;
                if added == 2 {
                    break;
                }
            }
            lectra_note = format!("{added} comparable CC BY 4.0 Lectra tasks; geometry not saved");
        } else {
            lectra_note = "Lectra sample-300.jsonl absent; no tasks used".into();
        }
    }
    let mut train = Vec::new();
    let mut holdout = Vec::new();
    let mut failures = Vec::new();
    for (source, base, is_holdout) in jobs {
        // Ratios 1 and 2 retain original size ratios; no Lectra orientation constraints change.
        for ratio in [1u32, 2] {
            for width in [1400.0, 1600.0] {
                let mut job = base.clone();
                job.fabric.width = width;
                let garments = if let Some(order) = &mut job.order {
                    for n in order.sizes.values_mut() {
                        *n = n.checked_mul(ratio).context("ratio overflow")?;
                    }
                    order.sizes.values().sum()
                } else {
                    // Each source task is treated as one complete set when no garment ratio exists.
                    for p in &mut job.pieces {
                        p.quantity = p.quantity.checked_mul(ratio).context("quantity overflow")?;
                        p.mirror_quantity = p
                            .mirror_quantity
                            .checked_mul(ratio)
                            .context("quantity overflow")?;
                    }
                    ratio
                };
                let seed = 7100 + train.len() as u64 + holdout.len() as u64;
                let result = (|| -> Result<Observation> {
                    let (f, _) = features(&job, width)?;
                    let actual = accurate_estimate(&job, width, garments, 1.0, seed)?;
                    Ok(Observation {
                        source: source.clone(),
                        features: f,
                        efficiency: actual.marker.efficiency,
                        length_mm: actual.marker.length,
                        width_mm: width,
                        garments,
                    })
                })();
                match result {
                    Ok(o) => {
                        eprintln!(
                            "{source} ratio {ratio} width {width}: {:.2}% ({:.0} mm)",
                            100.0 * o.efficiency,
                            o.length_mm
                        );
                        if is_holdout {
                            holdout.push(o);
                        } else {
                            train.push(o);
                        }
                    }
                    Err(e) => {
                        eprintln!("{source} ratio {ratio} width {width}: {e:#}");
                        failures.push(json!({"source":source,"ratio":ratio,"width_mm":width,"error":format!("{e:#}")}));
                    }
                }
                // Persist after each run, so a failed fit still leaves all successful observations.
                fs::write(
                    output.join("observations.json"),
                    serde_json::to_vec_pretty(
                        &json!({"train":train,"holdout":holdout,"failures":failures}),
                    )?,
                )?;
            }
        }
    }
    ensure!(!holdout.is_empty(), "no held-out nests succeeded");
    let mut model = fit(&train, 0.01)?;
    model.provenance = "Own 1-second sparrow nests, workers=2, widths 1400/1600 mm, source ratios multiplied by 1/2. Whole-pattern holdout; FreeSewing MIT, GarmentCodeData and Lectra CC BY 4.0. See calibration/README.md.".into();
    let train_mape = consumption_mape(&model, &train)?;
    let test_mape = consumption_mape(&model, &holdout)?;
    let mut by_source = BTreeMap::<String, Vec<Observation>>::new();
    for o in &holdout {
        by_source
            .entry(o.source.clone())
            .or_default()
            .push(o.clone());
    }
    let by_source = by_source
        .iter()
        .map(|(s, o)| Ok((s.clone(), consumption_mape(&model, o)?)))
        .collect::<Result<BTreeMap<_, _>>>()?;
    let summary = json!({"train_n":train.len(),"holdout_n":holdout.len(),"search_seconds":1.0,"workers":2,"ridge":0.01,"train_consumption_mape_pct":train_mape,"holdout_consumption_mape_pct":test_mape,"holdout_mape_by_source_pct":by_source,"lectra":lectra_note,"failed_n":failures.len(),"seed_policy":"7100 + number of successful prior observations","limits":"Small exploratory sample; real nests are short-run baselines, not optimal or production-accuracy labels."});
    fs::write(
        output.join("model.json"),
        serde_json::to_vec_pretty(&model)?,
    )?;
    fs::write(
        output.join("summary.json"),
        serde_json::to_vec_pretty(&summary)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}
