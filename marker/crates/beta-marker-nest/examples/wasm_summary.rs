//! Geometry-free Markdown summary of the paired browser benchmark JSONL.
use anyhow::{Context, Result};
use serde_json::Value;
use std::{collections::BTreeMap, fs};

fn rows(path: &str) -> Result<Vec<Value>> {
    fs::read_to_string(path)?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| Ok(serde_json::from_str(line)?))
        .collect()
}
fn number(row: &Value, pointer: &str) -> Result<f64> {
    row.pointer(pointer)
        .and_then(Value::as_f64)
        .with_context(|| format!("missing {pointer} in {}", row["id"]))
}
fn mean(xs: &[f64]) -> f64 {
    xs.iter().sum::<f64>() / xs.len() as f64
}
fn range(xs: &[f64]) -> (f64, f64) {
    (
        xs.iter().copied().fold(f64::INFINITY, f64::min),
        xs.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    )
}
fn quality(row: &Value) -> Result<String> {
    if row["validation"]["ok"] != true {
        return Ok("FAILED".into());
    }
    Ok(format!(
        "{:.2}% / {:.1}",
        100.0 * number(row, "/result/efficiency")?,
        number(row, "/result/length")?
    ))
}
fn memory_cell(row: &Value) -> String {
    row["memory_bytes"]
        .as_f64()
        .map_or_else(|| "—".into(), |n| format!("{:.2}", n / 1048576.0))
}
fn wall_cell(row: &Value) -> Result<String> {
    Ok(format!(
        "{:.2}{}",
        number(row, "/wall_s")?,
        if row["watchdog"] == true { "*" } else { "" }
    ))
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let native = rows(args.get(1).context("native JSONL path")?)?;
    let wasm = rows(args.get(2).context("WASM JSONL path")?)?;
    let native: BTreeMap<_, _> = native
        .iter()
        .map(|row| (row["id"].as_str().unwrap(), row))
        .collect();
    println!(
        "| Case | Native 60 s: efficiency / length mm | WASM 30 s: efficiency / length mm | WASM 60 s: efficiency / length mm |"
    );
    println!("|---|---:|---:|---:|");
    for (&id, row) in &native {
        print!("| {id} | {} |", quality(row)?);
        for seconds in [30, 60] {
            let w = wasm
                .iter()
                .find(|w| w["id"] == id && w["budget_s"] == seconds)
                .context("missing paired row")?;
            print!(" {} |", quality(w)?);
        }
        println!();
    }
    for seconds in [30, 60] {
        for prefix in ["freesewing-", "lectra-"] {
            let selected: Vec<_> = wasm
                .iter()
                .filter(|w| {
                    w["budget_s"] == seconds
                        && w["id"].as_str().unwrap().starts_with(prefix)
                        && w["validation"]["ok"] == true
                        && native[w["id"].as_str().unwrap()]["validation"]["ok"] == true
                })
                .collect();
            let delta: Vec<_> = selected
                .iter()
                .map(|w| {
                    let n = native[w["id"].as_str().unwrap()];
                    Ok(100.0
                        * (number(w, "/result/efficiency")? - number(n, "/result/efficiency")?))
                })
                .collect::<Result<_>>()?;
            println!(
                "\n{prefix} at {seconds} s: mean efficiency difference from native = {:+.2} percentage points ({} valid pairs; failures excluded from the mean).",
                mean(&delta),
                selected.len()
            );
        }
    }
    println!(
        "\n| Case | Native wall s | WASM wall s (30 / 60) | WASM memory MiB (30 / 60) | 60 s run: rounds / mean round s / max round s |"
    );
    println!("|---|---:|---:|---:|---:|");
    for (&id, n) in &native {
        let a = wasm
            .iter()
            .find(|w| w["id"] == id && w["budget_s"] == 30)
            .context("missing 30 s row")?;
        let b = wasm
            .iter()
            .find(|w| w["id"] == id && w["budget_s"] == 60)
            .context("missing 60 s row")?;
        let rounds: Vec<_> = b["rounds"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|r| number(r, "/seconds"))
            .collect::<Result<_>>()?;
        let timing = if rounds.is_empty() {
            "—".into()
        } else {
            format!(
                "{} / {:.3} / {:.3}",
                rounds.len(),
                mean(&rounds),
                range(&rounds).1
            )
        };
        println!(
            "| {id} | {:.2} | {} / {} | {} / {} | {} |",
            number(n, "/wall_s")?,
            wall_cell(a)?,
            wall_cell(b)?,
            memory_cell(a),
            memory_cell(b),
            timing,
        );
    }
    println!(
        "\n* Host watchdog retained the last checked checkpoint. Round timings and memory samples cover completed rounds only."
    );
    let wall: Vec<_> = wasm
        .iter()
        .map(|r| number(r, "/wall_s"))
        .collect::<Result<_>>()?;
    let memory: Vec<_> = wasm
        .iter()
        .filter_map(|r| r["memory_bytes"].as_f64().map(|n| n / 1048576.0))
        .collect();
    let mut rounds: Vec<_> = wasm
        .iter()
        .flat_map(|r| r["rounds"].as_array().into_iter().flatten())
        .map(|r| number(r, "/seconds"))
        .collect::<Result<_>>()?;
    rounds.sort_by(f64::total_cmp);
    let (wall_min, wall_max) = range(&wall);
    let (mem_min, mem_max) = range(&memory);
    let native_wall: Vec<_> = native
        .values()
        .map(|r| number(r, "/wall_s"))
        .collect::<Result<_>>()?;
    let (native_min, native_max) = range(&native_wall);
    println!("\nNative wall time: {native_min:.2}–{native_max:.2} s.");
    println!("WASM wall time (30/60 s budgets combined): {wall_min:.2}–{wall_max:.2} s.");
    let mid = rounds.len() / 2;
    let median = if rounds.len().is_multiple_of(2) {
        (rounds[mid - 1] + rounds[mid]) / 2.0
    } else {
        rounds[mid]
    };
    println!(
        "WASM allocated linear memory: {mem_min:.2}–{mem_max:.2} MiB (reused instance high-water mark)."
    );
    println!(
        "Round time: mean {:.3} s, median {:.3} s, max {:.3} s ({} rounds).",
        mean(&rounds),
        median,
        rounds.last().unwrap(),
        rounds.len()
    );
    println!(
        "Valid native results: {}/{}; valid WASM results: {}/{}.",
        native
            .values()
            .filter(|r| r["validation"]["ok"] == true)
            .count(),
        native.len(),
        wasm.iter()
            .filter(|r| r["validation"]["ok"] == true)
            .count(),
        wasm.len()
    );
    println!(
        "Watchdog recoveries: {} / {}. Results whose winning marker came from the BETA bounded packer: {}.",
        wasm.iter().filter(|r| r["watchdog"] == true).count(),
        wasm.len(),
        wasm.iter()
            .filter(|r| r["result"]["engine"]
                .as_str()
                .is_some_and(|s| s.contains("bounded")))
            .count()
    );
    Ok(())
}
