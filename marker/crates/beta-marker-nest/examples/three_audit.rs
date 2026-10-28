//! Independently replay validation, fixed transforms and paired measurements.
//! three_audit CASES BEFORE_FILES_CSV AFTER_FILES_CSV SUMMARY_JSON
use anyhow::{Context, Result, ensure};
use beta_marker_core::{Job, MarkerResult, Placement};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};

#[derive(Deserialize)]
struct Case {
    id: String,
    job: Job,
    #[serde(default)]
    fixed: Vec<Placement>,
}
fn load(paths: &str, cases: &[Case]) -> Result<BTreeMap<(String, u64), Value>> {
    let mut out = BTreeMap::new();
    for path in paths.split(',') {
        for line in fs::read_to_string(path)?.lines().filter(|l| !l.is_empty()) {
            let row: Value = serde_json::from_str(line)?;
            let id = row["id"].as_str().context("case id")?;
            let seed = row["seed"].as_u64().context("seed")?;
            let case = cases.iter().find(|c| c.id == id).context("unknown case")?;
            let result: MarkerResult = serde_json::from_value(row["result"].clone())
                .with_context(|| format!("missing marker {id}: {}", row["error"]))?;
            let report = beta_marker_core::validate(&case.job, &result);
            ensure!(report.ok, "invalid {id}: {:?}", report.errors);
            ensure!(
                case.fixed.iter().all(|p| result.placements.contains(p)),
                "fixed transform changed: {id}"
            );
            let fixed_end = case
                .fixed
                .iter()
                .map(|p| {
                    let piece = case
                        .job
                        .pieces
                        .iter()
                        .find(|piece| piece.id == p.piece_id)
                        .unwrap();
                    beta_marker_core::geometry::bbox(&beta_marker_core::rules::cut_outline(
                        piece, p,
                    ))
                    .unwrap()[2]
                })
                .fold(0., f64::max);
            let value = json!({"id":id,"seed":seed,"budget_s":row["budget_s"],"length":report.length,
                "useful_pct":100.*report.useful_yield,"surplus":report.surplus_copies,"wall_s":row["wall_s"],
                "fixed_end":fixed_end,"fixed_preserved":true,"valid":true});
            ensure!(
                out.insert((id.into(), seed), value).is_none(),
                "duplicate trial"
            );
        }
    }
    Ok(out)
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    ensure!(args.len() == 5, "three_audit CASES BEFORE AFTER OUTPUT");
    let cases: Vec<Case> = serde_json::from_slice(&fs::read(&args[1])?)?;
    let before = load(&args[2], &cases)?;
    let after = load(&args[3], &cases)?;
    ensure!(before.len() == after.len(), "different trial counts");
    let mut trials = vec![];
    for (key, b) in &before {
        let n = after.get(key).context("missing paired result")?;
        ensure!(n["budget_s"] == b["budget_s"], "different budgets");
        let old = b["length"].as_f64().unwrap();
        let new = n["length"].as_f64().unwrap();
        trials.push(json!({"id":key.0,"seed":key.1,"requested_s":b["budget_s"],
            "before_m":old/1000.,"after_m":new/1000.,"saved_pct":100.*(1.-new/old),
            "before_useful_pct":b["useful_pct"],"after_useful_pct":n["useful_pct"],
            "before_wall_s":b["wall_s"],"after_wall_s":n["wall_s"],
            "before_surplus":b["surplus"],"after_surplus":n["surplus"],"locked_end_m":b["fixed_end"].as_f64().unwrap()/1000.,
            "fixed_preserved":true,"valid":true}));
    }
    let ids: BTreeSet<_> = trials.iter().map(|r| r["id"].as_str().unwrap()).collect();
    let mut groups = vec![];
    for id in ids {
        let rows: Vec<_> = trials.iter().filter(|r| r["id"] == id).collect();
        let mean =
            |k: &str| rows.iter().map(|r| r[k].as_f64().unwrap()).sum::<f64>() / rows.len() as f64;
        groups.push(json!({"id":id,"trials":rows.len(),"before_m":mean("before_m"),"after_m":mean("after_m"),
            "saved_pct":mean("saved_pct"),"before_useful_pct":mean("before_useful_pct"),"after_useful_pct":mean("after_useful_pct"),
            "before_wall_s":mean("before_wall_s"),"after_wall_s":mean("after_wall_s"),"locked_end_m":mean("locked_end_m"),
            "min_saved_pct":rows.iter().map(|r|r["saved_pct"].as_f64().unwrap()).fold(f64::INFINITY,f64::min),
            "max_saved_pct":rows.iter().map(|r|r["saved_pct"].as_f64().unwrap()).fold(f64::NEG_INFINITY,f64::max)}));
    }
    let report = json!({"cases":groups,"trials":trials,"all_valid":true,"all_fixed_transforms_preserved":true});
    fs::write(&args[4], serde_json::to_vec_pretty(&report)?)?;
    println!("{}", serde_json::to_string_pretty(&report["cases"])?);
    Ok(())
}
