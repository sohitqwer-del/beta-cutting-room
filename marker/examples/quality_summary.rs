//! Summarize independent development runs. Does not select on confirmation outcomes.
use anyhow::{Context, Result, ensure};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

#[path = "../crates/beta-marker-cli/src/cmd/bench/csv.rs"]
#[allow(dead_code)]
mod csv;

fn rows(path: &Path) -> Result<Vec<BTreeMap<String, String>>> {
    let records = csv::records(&fs::read_to_string(path)?)?;
    let header = records.first().context("empty CSV")?;
    ensure!(
        ["id", "seed", "valid", "our_efficiency", "difference_pp"]
            .iter()
            .all(|key| header.iter().any(|column| column == key)),
        "missing required benchmark CSV columns"
    );
    records
        .iter()
        .skip(1)
        .map(|r| {
            ensure!(r.len() == header.len(), "CSV column mismatch");
            Ok(header.iter().cloned().zip(r.iter().cloned()).collect())
        })
        .collect()
}
fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}
fn numeric(row: &BTreeMap<String, String>, key: &str) -> Option<f64> {
    row.get(key)?.parse::<f64>().ok().filter(|v| v.is_finite())
}
fn stats(rows: &[&BTreeMap<String, String>]) -> String {
    let mut values: Vec<f64> = rows
        .iter()
        .filter(|r| r.get("valid").is_some_and(|v| v == "true"))
        .filter_map(|r| numeric(r, "difference_pp"))
        .collect();
    values.sort_by(f64::total_cmp);
    let wins = values.iter().filter(|&&v| v > 0.100000001).count();
    let ties = values.iter().filter(|&&v| v.abs() <= 0.100000001).count();
    let finite_sheet = rows
        .iter()
        .filter(|r| {
            r.get("valid").is_some_and(|v| v != "true")
                && r.get("errors").is_some_and(|e| e.contains("too_long"))
        })
        .count();
    let failures = rows
        .iter()
        .filter(|r| r.get("valid").is_some_and(|v| v != "true"))
        .count()
        - finite_sheet;
    let median = if values.is_empty() {
        "?".into()
    } else {
        format!(
            "{:.4}",
            (values[(values.len() - 1) / 2] + values[values.len() / 2]) / 2.0
        )
    };
    format!(
        "{} / {} | {} | {median} | {wins} / {ties} / {} | {failures}",
        rows.len(),
        values.len(),
        mean_or_dash(&values),
        values.len() - wins - ties + finite_sheet
    )
}

fn mean_or_dash(values: &[f64]) -> String {
    if values.is_empty() {
        "?".into()
    } else {
        format!("{:.4}", mean(values))
    }
}
fn paired_values(
    base: &[BTreeMap<String, String>],
    variant: &[BTreeMap<String, String>],
    excluded: &BTreeSet<String>,
) -> Vec<f64> {
    let base: BTreeMap<_, _> = base
        .iter()
        .filter(|r| r["valid"] == "true" && !excluded.contains(&r["id"]))
        .filter_map(|r| {
            Some((
                (r["id"].clone(), r["seed"].clone()),
                numeric(r, "our_efficiency")?,
            ))
        })
        .collect();
    variant
        .iter()
        .filter(|r| r["valid"] == "true" && !excluded.contains(&r["id"]))
        .filter_map(|r| {
            Some(numeric(r, "our_efficiency")? - base.get(&(r["id"].clone(), r["seed"].clone()))?)
        })
        .collect()
}
fn root_for(path: &Path) -> PathBuf {
    if path.join("effective/results.csv").exists()
        || path.join("effective/variant/results.csv").exists()
    {
        path.join("effective")
    } else {
        path.to_owned()
    }
}

// Check a geometric impossibility independently of any search result or score.
fn verify_exclusions(root: &Path, eligibility: &Value) -> Result<()> {
    let proofs = eligibility["excluded_from_search_selection"]
        .as_object()
        .context("eligibility exclusions missing")?;
    let mut verified = BTreeSet::new();
    for line in BufReader::new(fs::File::open(
        root.join("conversion/development-40.jsonl"),
    )?)
    .lines()
    {
        let task: Value = serde_json::from_str(&line?)?;
        let id = task["id"].as_str().context("development task ID missing")?;
        let Some(proof) = proofs.get(id) else {
            continue;
        };
        let job: beta_marker_core::Job = serde_json::from_value(task["job"].clone())?;
        ensure!(
            job.fabric.layout == beta_marker_core::Layout::Open,
            "proof requires an open strip"
        );
        ensure!(
            (job.fabric.width
                - proof["fabric_width_mm"]
                    .as_f64()
                    .context("proof width missing")?)
            .abs()
                < 1e-8,
            "proof fabric width changed"
        );
        for entry in proof["pieces"].as_array().context("proof pieces missing")? {
            let piece = job
                .pieces
                .iter()
                .find(|p| Some(p.id.as_str()) == entry["id"].as_str())
                .context("proof piece missing")?;
            ensure!(
                piece.quantity > 0
                    && piece.mirror_quantity == 0
                    && !beta_marker_core::rules::flip_allowed(&job, piece),
                "proof requires an unmirrored demanded piece"
            );
            let rotations = beta_marker_core::allowed_rotations(&job, piece, false);
            ensure!(
                !rotations.free && rotations.tilt_deg == 0.0 && !rotations.angles.is_empty(),
                "proof requires discrete untilted angles"
            );
            let expected = entry["cross_width_span_mm"]
                .as_f64()
                .context("proof span missing")?;
            for angle in rotations.angles {
                let points = beta_marker_core::geometry::transform_points(
                    &piece.outline,
                    false,
                    angle,
                    [0.0, 0.0],
                );
                let bounds =
                    beta_marker_core::geometry::bbox(&points).context("proof outline empty")?;
                let span = bounds[3] - bounds[1];
                ensure!(
                    (span - expected).abs() < 1e-6 && span > job.fabric.width + 1e-6,
                    "excluded piece can fit or proof changed"
                );
            }
        }
        ensure!(verified.insert(id.to_owned()), "duplicate excluded task");
    }
    ensure!(
        verified.len() == proofs.len(),
        "an excluded task was not proved"
    );
    Ok(())
}
fn main() -> Result<()> {
    let root = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .context("usage: quality_summary <evidence-root>")?,
    );
    let mut folders: Vec<_> = fs::read_dir(&root)?.collect::<std::io::Result<_>>()?;
    folders.sort_by_key(|e| e.file_name());
    let eligibility: Value =
        serde_json::from_str(include_str!("../bench/experiments/v03/eligibility.json"))?;
    verify_exclusions(&root, &eligibility)?;
    let attempted = eligibility["attempted_tasks"]
        .as_u64()
        .context("eligibility task count missing")? as usize;
    let excluded: BTreeSet<String> = eligibility["excluded_from_search_selection"]
        .as_object()
        .context("eligibility exclusions missing")?
        .keys()
        .cloned()
        .collect();
    let eligible = attempted
        .checked_sub(excluded.len())
        .context("invalid eligibility count")?;
    let mut experiments = format!(
        "All {attempted} frozen tasks attempted per arm. Geometry-only exclusions from selection: {}.\n\n| Experiment | Valid eligible pairs / {eligible} | Baseline minus Lectra, pp | Variant minus Lectra, pp | Paired gain, pp | Load retries | Persistent load outliers |\n|---|---:|---:|---:|---:|---:|---:|\n",
        excluded.iter().cloned().collect::<Vec<_>>().join(", ")
    );
    let mut confirmation = String::from(
        "| Run / group | Attempts / valid comparisons | Mean difference, pp | Median difference, pp | Win / tie / loss | Other failures |\n|---|---:|---:|---:|---|---:|\n",
    );
    let mut features = String::from(
        "| Run | Job | Arm | Length | Occupied efficiency % | Demand efficiency % | Runtime s | Valid |\n|---|---|---|---:|---:|---:|---:|---|\n",
    );
    let mut candidates = Vec::new();
    let mut studies_seen = 0;
    let plan: Value =
        serde_json::from_str(include_str!("../bench/experiments/v03/study-plan.json"))?;
    let active: BTreeSet<String> = plan["active"]
        .as_array()
        .context("study plan missing")?
        .iter()
        .map(|name| {
            name.as_str()
                .map(|name| format!("paired-{name}"))
                .context("invalid study name")
        })
        .collect::<Result<_>>()?;
    let mut baseline_search = None;
    for entry in folders {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("interrupted-") || name.starts_with("tooling-") {
            continue;
        }
        let path = root_for(&entry.path());
        if name.starts_with("paired-") && path.join("variant/results.csv").exists() {
            if !active.contains(&name) {
                continue;
            }
            let audit: Value = serde_json::from_slice(&fs::read(path.join("load-audit.json"))?)?;
            ensure!(
                audit["rows"].as_array().is_some_and(|rows| !rows.is_empty()
                    && rows
                        .iter()
                        .all(|row| row.get("sampling_interrupted").is_some())),
                "Re-audit every development study for sampling gaps before selection: {name}"
            );
            studies_seen += 1;
            let baseline = rows(&path.join("baseline/results.csv"))?;
            let variant = rows(&path.join("variant/results.csv"))?;
            let changes = paired_values(&baseline, &variant, &excluded);
            let bdiff: Vec<_> = baseline
                .iter()
                .filter(|r| r["valid"] == "true" && !excluded.contains(&r["id"]))
                .filter_map(|r| numeric(r, "difference_pp"))
                .collect();
            let vdiff: Vec<_> = variant
                .iter()
                .filter(|r| r["valid"] == "true" && !excluded.contains(&r["id"]))
                .filter_map(|r| numeric(r, "difference_pp"))
                .collect();
            let retries = entry.path().join("reruns");
            let retry_count = if retries.exists() {
                fs::read_dir(&retries)?.count()
            } else {
                0
            };
            let selected = path.join("selection.json");
            let persistent = if selected.exists() {
                serde_json::from_slice::<Value>(&fs::read(selected)?)?["persistent_outliers"]
                    .as_array()
                    .map_or(0, Vec::len)
            } else {
                0
            };
            experiments.push_str(&format!(
                "| {name} | {} / {eligible} | {} | {} | {} | {retry_count} | {persistent} |\n",
                changes.len(),
                mean_or_dash(&bdiff),
                mean_or_dash(&vdiff),
                mean_or_dash(&changes)
            ));
            let bmanifest: Value =
                serde_json::from_slice(&fs::read(path.join("baseline/run.json"))?)?;
            baseline_search = Some(bmanifest["search"].clone());
            if changes.len() == eligible
                && baseline.len() == attempted
                && variant.len() == attempted
            {
                let manifest: Value =
                    serde_json::from_slice(&fs::read(path.join("variant/run.json"))?)?;
                candidates.push((mean(&changes), name.clone(), manifest["search"].clone()));
            }
            continue;
        }
        let arms: Vec<_> = if path.join("baseline/results.csv").exists() {
            vec![
                ("baseline", path.join("baseline")),
                ("variant", path.join("variant")),
            ]
        } else {
            vec![("run", path.clone())]
        };
        // Earlier unpaired, unaudited baselines remain raw evidence, not confirmation.
        if !path.join("load-audit.json").is_file() {
            continue;
        }
        for (arm, folder) in arms {
            if !folder.join("results.csv").exists() {
                continue;
            }
            let records = rows(&folder.join("results.csv"))?;
            if records
                .iter()
                .any(|r| numeric(r, "reference_efficiency").is_some())
            {
                confirmation.push_str(&format!(
                    "| {name} {arm}: all | {} |\n",
                    stats(&records.iter().collect::<Vec<_>>())
                ));
                for tier in ["A", "B"] {
                    let group: Vec<_> = records
                        .iter()
                        .filter(|r| r.get("tier").is_some_and(|t| t == tier))
                        .collect();
                    if !group.is_empty() {
                        confirmation.push_str(&format!(
                            "| {name} {arm}: tier {tier} | {} |\n",
                            stats(&group)
                        ));
                    }
                }
            } else {
                for row in records {
                    let num =
                        |key| numeric(&row, key).map_or_else(|| "?".into(), |v| format!("{v:.4}"));
                    features.push_str(&format!(
                        "| {name} | {} | {arm} | {} | {} | {} | {} | {} |\n",
                        row["id"],
                        num("our_length"),
                        num("our_efficiency"),
                        num("demand_efficiency"),
                        num("runtime_s"),
                        row["valid"]
                    ));
                }
            }
        }
    }
    fs::write(root.join("experiments-table.md"), &experiments)?;
    fs::write(root.join("confirmation-table.md"), &confirmation)?;
    fs::write(root.join("features-table.md"), &features)?;
    println!("{experiments}\n{confirmation}\n{features}");
    if studies_seen == active.len() {
        candidates.sort_by(|a, b| b.0.total_cmp(&a.0));
        let (name, change, search) =
            if let Some(winner) = candidates.first().filter(|winner| winner.0 > 0.1) {
                (winner.1.clone(), winner.0, winner.2.clone())
            } else {
                (
                    "baseline".into(),
                    0.0,
                    baseline_search.context("baseline settings missing")?,
                )
            };
        fs::write(
            root.join("selection.json"),
            serde_json::to_vec_pretty(
                &serde_json::json!({"experiment":name,"paired_change_pp":change,"search":search,"attempted_tasks":attempted,"eligible_pairs_required":eligible,"geometry_exclusions":excluded,"sampling_gap_audit_checked":true,"study_plan":plan,"rule":"All geometrically eligible pairs must be valid; all original tasks must be attempted. Select by paired gain only, never confirmation outcomes; retain baseline unless gain exceeds 0.1 pp. CPU and sampling-gap retries use first qualifying rerun, not best score."}),
            )?,
        )?;
        fs::write(
            root.join("selected-search.json"),
            serde_json::to_vec_pretty(&search["tuning"])?,
        )?;
    }
    Ok(())
}
