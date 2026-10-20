//! Summarize paired quality without treating incomplete/failed cells as wins.
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct Row {
    job: String,
    seed: u64,
    strategy: String,
    efficiency_pct: Option<f64>,
    wall_s: f64,
    valid: bool,
}
#[derive(Serialize)]
struct Pair {
    job: String,
    seed: u64,
    continuous_pct: f64,
    warm5_pct: f64,
    warm15_pct: f64,
    loss5_pp: f64,
    loss15_pp: f64,
    continuous_wall_s: f64,
    warm5_wall_s: f64,
    warm15_wall_s: f64,
}
#[derive(Serialize)]
struct Comparison {
    pairs: Vec<Pair>,
    mean_loss5_pp: Option<f64>,
    mean_loss15_pp: Option<f64>,
    incomplete: Vec<String>,
    recommendation: String,
}

fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let input = args
        .first()
        .context("Pass slice-bench summary.json and optional output.json")?;
    let rows: Vec<Row> = serde_json::from_slice(&std::fs::read(input)?)?;
    let mut groups = BTreeMap::<(String, u64), BTreeMap<String, Row>>::new();
    for row in rows {
        groups
            .entry((row.job.clone(), row.seed))
            .or_default()
            .insert(row.strategy.clone(), row);
    }
    let mut pairs = Vec::new();
    let mut incomplete = Vec::new();
    for ((job, seed), cells) in groups {
        let get = |name: &str| {
            cells
                .get(name)
                .filter(|r| r.valid)
                .and_then(|r| r.efficiency_pct.map(|p| (p, r.wall_s)))
        };
        if let (Some((continuous, a)), Some((warm5, b)), Some((warm15, c))) =
            (get("continuous60"), get("warm12x5"), get("warm4x15"))
        {
            pairs.push(Pair {
                job,
                seed,
                continuous_pct: continuous,
                warm5_pct: warm5,
                warm15_pct: warm15,
                loss5_pp: continuous - warm5,
                loss15_pp: continuous - warm15,
                continuous_wall_s: a,
                warm5_wall_s: b,
                warm15_wall_s: c,
            });
        } else {
            incomplete.push(format!("{job} seed {seed}: missing or failed paired cell"));
        }
    }
    let mean_loss5_pp = if pairs.is_empty() {
        None
    } else {
        Some(pairs.iter().map(|p| p.loss5_pp).sum::<f64>() / pairs.len() as f64)
    };
    let mean_loss15_pp = if pairs.is_empty() {
        None
    } else {
        Some(pairs.iter().map(|p| p.loss15_pp).sum::<f64>() / pairs.len() as f64)
    };
    let recommendation=if mean_loss5_pp.is_some_and(|n|n>0.2) {
        "Five-second slicing exceeds the mean-loss guard. Run continuously with this engine. A future checkpoint slice must retain construction and use at least max(60 seconds, measured construction time)."
    }else{
        "This finite paired sample does not prove equivalence. Run continuously to avoid repeated construction. A future checkpoint slice must retain construction and use at least max(60 seconds, measured construction time)."
    }.to_string();
    let out = serde_json::to_string_pretty(&Comparison {
        pairs,
        mean_loss5_pp,
        mean_loss15_pp,
        incomplete,
        recommendation,
    })?;
    if let Some(path) = args.get(1) {
        std::fs::write(path, &out)?;
    }
    println!("{out}");
    Ok(())
}
