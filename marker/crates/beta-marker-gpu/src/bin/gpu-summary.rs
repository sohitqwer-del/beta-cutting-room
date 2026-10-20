//! Aggregate raw paired evidence without selecting successful trials.
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

fn records(path: PathBuf) -> Result<Vec<Value>> {
    fs::read_to_string(path)?
        .lines()
        .map(|line| Ok(serde_json::from_str(line)?))
        .collect()
}
fn mean(values: &[f64]) -> Option<f64> {
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}
fn range(values: &[f64]) -> Value {
    json!({ "n": values.len(), "mean": mean(values), "min": values.iter().copied().reduce(f64::min), "max": values.iter().copied().reduce(f64::max) })
}
fn main() -> Result<()> {
    let out = PathBuf::from(
        std::env::args()
            .nth(1)
            .context("usage: gpu-summary <evidence-directory>")?,
    );
    let runs = records(out.join("runs.jsonl"))?;
    let mut pairs = BTreeMap::<(String, u64), BTreeMap<String, &Value>>::new();
    for row in &runs {
        let budget = row["budget_s"].as_f64().context("budget")?.to_bits();
        pairs
            .entry((row["id"].as_str().context("id")?.into(), budget))
            .or_default()
            .insert(row["variant"].as_str().context("variant")?.into(), row);
    }
    let budgets: BTreeSet<_> = pairs.keys().map(|(_, b)| *b).collect();
    let paired_details: Vec<_> = pairs.iter().filter_map(|((id, budget), pair)| {
        let (plain, warm) = (pair.get("plain")?, pair.get("gpu_warm")?);
        let delta = plain["efficiency"].as_f64().zip(warm["efficiency"].as_f64()).map(|(p, w)| (w - p) * 100.0);
        Some(json!({ "id": id, "budget_s": f64::from_bits(*budget), "plain_efficiency": plain["efficiency"], "warm_efficiency": warm["efficiency"], "delta_pp": delta, "plain_ok": plain["ok"], "warm_ok": warm["ok"], "proposal_error": warm["proposal_error"] }))
    }).collect();
    let mut groups = Vec::new();
    for budget in budgets {
        for family in ["all", "Lectra", "FreeSewing"] {
            let mut deltas = Vec::new();
            let mut plain = Vec::new();
            let mut warm = Vec::new();
            let mut wins = 0;
            let mut ties = 0;
            let mut losses = 0;
            let mut incomplete = 0;
            let mut failed_plain = 0;
            let mut failed_warm = 0;
            let mut proposals = 0;
            let mut proposal_failures = 0;
            let mut proposal_s = Vec::new();
            let mut scan_s = Vec::new();
            let mut kernel_s = Vec::new();
            let mut wall_plain = Vec::new();
            let mut wall_warm = Vec::new();
            for ((id, b), pair) in &pairs {
                if *b != budget
                    || (family == "Lectra" && !id.starts_with("lectra-"))
                    || (family == "FreeSewing" && id.starts_with("lectra-"))
                {
                    continue;
                }
                let (Some(p), Some(w)) = (pair.get("plain"), pair.get("gpu_warm")) else {
                    incomplete += 1;
                    continue;
                };
                let p_ok = p["ok"] == true && p["valid"] == true;
                let w_ok = w["ok"] == true && w["valid"] == true;
                failed_plain += usize::from(!p_ok);
                failed_warm += usize::from(!w_ok);
                if w["proposal_stats"].is_object() {
                    proposals += 1;
                    if let Some(v) = w["proposal_stats"]["proposal_s"].as_f64() {
                        proposal_s.push(v);
                    }
                    if let Some(v) = w["proposal_stats"]["scan_s"].as_f64() {
                        scan_s.push(v);
                    }
                    if let Some(v) = w["proposal_stats"]["gpu_kernel_s"].as_f64() {
                        kernel_s.push(v);
                    }
                } else {
                    proposal_failures += 1;
                }
                if let Some(v) = p["wall_s"].as_f64() {
                    wall_plain.push(v);
                }
                if let Some(v) = w["wall_s"].as_f64() {
                    wall_warm.push(v);
                }
                if p_ok && w_ok {
                    let pv = p["efficiency"].as_f64().context("plain efficiency")? * 100.0;
                    let wv = w["efficiency"].as_f64().context("warm efficiency")? * 100.0;
                    let delta = wv - pv;
                    plain.push(pv);
                    warm.push(wv);
                    deltas.push(delta);
                    if delta > 0.1 {
                        wins += 1;
                    } else if delta < -0.1 {
                        losses += 1;
                    } else {
                        ties += 1;
                    }
                } else if p_ok {
                    losses += 1;
                } else if w_ok {
                    wins += 1;
                }
            }
            groups.push(json!({ "family": family, "budget_s": f64::from_bits(budget), "valid_pairs": deltas.len(), "plain_mean_pct": mean(&plain), "warm_mean_pct": mean(&warm), "delta_pp": range(&deltas), "win_tie_loss_at_0_1pp": [wins, ties, losses], "failed_plain": failed_plain, "failed_warm": failed_warm, "incomplete_pairs": incomplete, "successful_proposals": proposals, "failed_proposals": proposal_failures, "proposal_s": range(&proposal_s), "scan_s": range(&scan_s), "gpu_kernel_s": range(&kernel_s), "plain_wall_s": range(&wall_plain), "warm_wall_s": range(&wall_warm) }));
        }
    }
    let cpu = records(out.join("cpu-fallback.jsonl"))?;
    let fills = records(out.join("fill-gaps.jsonl"))?;
    let windows: Vec<_> = runs
        .iter()
        .filter(|v| v["variant"] == "gpu_warm")
        .filter_map(|v| {
            Some((
                v["start_unix_ms"].as_f64()?,
                v["proposal_wall_s"].as_f64()? * 1000.0,
            ))
        })
        .collect();
    let mut all_util = Vec::new();
    let mut proposal_util = Vec::new();
    let mut memory = Vec::new();
    for filename in ["nvidia-smi.csv", "prior-nvidia-smi.csv"] {
        if !out.join(filename).exists() {
            continue;
        }
        for line in fs::read_to_string(out.join(filename))?.lines().skip(1) {
            let fields: Vec<_> = line.split(',').map(str::trim).collect();
            if fields.len() != 6 {
                continue;
            }
            if let (Ok(time), Ok(util)) = (fields[0].parse::<f64>(), fields[2].parse::<f64>()) {
                all_util.push(util);
                if windows
                    .iter()
                    .any(|&(start, span)| time >= start && time <= start + span)
                {
                    proposal_util.push(util);
                }
            }
            if let Ok(value) = fields[3].parse::<f64>() {
                memory.push(value);
            }
        }
    }
    let fallback_s: Vec<_> = cpu
        .iter()
        .filter_map(|v| v["stats"]["proposal_s"].as_f64())
        .collect();
    let fill_s: Vec<_> = fills
        .iter()
        .filter_map(|v| v["stats"]["proposal_s"].as_f64())
        .collect();
    let fill_delta: Vec<_> = fills
        .iter()
        .filter_map(|v| {
            Some((v["after_efficiency"].as_f64()? - v["before_efficiency"].as_f64()?) * 100.0)
        })
        .collect();
    ensure!(
        fill_delta.iter().all(|v| *v >= -1e-8),
        "fill gaps regressed an incumbent"
    );
    let summary = json!({ "rows": runs.len(), "groups": groups, "paired_details": paired_details, "cpu_fallback_s": range(&fallback_s), "cpu_fallback_failures": cpu.iter().filter(|v| v["ok"] != true).count(), "fill_gaps_s": range(&fill_s), "fill_gaps_delta_pp": range(&fill_delta), "fill_gaps_improved": fill_delta.iter().filter(|v| **v > 1e-6).count(), "fill_gaps_failures": fills.iter().filter(|v| v["ok"] != true).count(), "gpu_util_all_pct": range(&all_util), "gpu_util_proposal_windows_pct": range(&proposal_util), "gpu_memory_MiB": range(&memory) });
    fs::write(
        out.join("summary.json"),
        serde_json::to_vec_pretty(&summary)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}
