//! Paired, independently validated efficiency experiments on public/synthetic jobs.
//! prepare: efficiency_study --prepare MARKER_ROOT CASES_JSON
//! run: efficiency_study CASES_JSON OUTPUT_JSONL SECONDS SEED
use anyhow::{Context, Result, ensure};
use beta_marker_core::{Fabric, JOB_FORMAT, Job, Piece, Placement, RotationRule, Zone};
use beta_marker_nest::{NestOptions, nest};
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::Path, time::Instant};

#[derive(Serialize, Deserialize)]
struct Case {
    id: String,
    job: Job,
    #[serde(default)]
    fixed: Vec<Placement>,
}

fn prepare(root: &Path, output: &Path) -> Result<()> {
    let pocket = Job {
        format: JOB_FORMAT.into(),
        name: "original concave pocket regression".into(),
        fabric: Fabric {
            width: 100.0,
            ..Default::default()
        },
        pieces: vec![
            Piece {
                id: "concave".into(),
                quantity: 1,
                rotation: RotationRule::Fixed,
                outline: vec![
                    [0., 0.],
                    [100., 0.],
                    [100., 100.],
                    [40., 100.],
                    [40., 20.],
                    [20., 20.],
                    [20., 100.],
                    [0., 100.],
                ],
                ..Default::default()
            },
            Piece {
                id: "small".into(),
                quantity: 1,
                rotation: RotationRule::Fixed,
                outline: vec![[0., 0.], [10., 0.], [10., 20.], [0., 20.]],
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    let mut cases = vec![Case {
        id: "synthetic-pocket".into(),
        job: pocket,
        fixed: vec![Placement {
            piece_id: "concave".into(),
            locked: true,
            ..Default::default()
        }],
    }];
    for name in ["teagan", "simon", "titan", "hugo"] {
        let mut job: Job = serde_json::from_slice(&fs::read(
            root.join(format!("testdata/open/freesewing/{name}-ratio.job.json")),
        )?)?;
        // One declared original defect selects the same constrained path used
        // for damaged fabric; piece geometry, demand, gaps and grain rules stay unchanged.
        job.fabric.defects.push(Zone {
            id: "study-hole".into(),
            polygon: vec![[0., 0.], [20., 0.], [20., 20.], [0., 20.]],
            note: Some("Original synthetic 20 mm defect; no factory data".into()),
        });
        cases.push(Case {
            id: format!("{name}-defect"),
            job,
            fixed: vec![],
        });
    }
    fs::write(output, serde_json::to_vec_pretty(&cases)?)?;
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).is_some_and(|a| a == "--summarize") {
        return summarize(&args[2], &args[3], Path::new(&args[4]));
    }
    if args.get(1).is_some_and(|a| a == "--prepare") {
        return prepare(
            Path::new(args.get(2).context("marker root")?),
            Path::new(args.get(3).context("cases output")?),
        );
    }
    ensure!(
        args.len() == 5,
        "usage: efficiency_study CASES OUTPUT SECONDS SEED"
    );
    let cases: Vec<Case> = serde_json::from_slice(&fs::read(&args[1])?)?;
    let seconds: f64 = args[3].parse()?;
    let seed: u64 = args[4].parse()?;
    let mut output = fs::File::create(&args[2])?;
    for case in cases {
        let started = Instant::now();
        let result = nest(
            &case.job,
            &NestOptions {
                time_limit_s: seconds,
                seed: Some(seed),
                workers: Some(1),
                fixed: case.fixed,
                ..Default::default()
            },
        );
        let row = match result {
            Ok(result) => serde_json::json!({
                "id": case.id, "seed": seed, "budget_s": seconds,
                "wall_s": started.elapsed().as_secs_f64(),
                "validation": beta_marker_core::validate(&case.job, &result),
                "result": result,
            }),
            Err(error) => serde_json::json!({"id":case.id,"seed":seed,"error":error.to_string()}),
        };
        writeln!(output, "{row}")?;
        output.flush()?;
        eprintln!(
            "{} seed {seed}: length {}, valid {}",
            row["id"], row["result"]["length"], row["validation"]["ok"]
        );
    }
    Ok(())
}

fn summarize(before_paths: &str, after_paths: &str, output: &Path) -> Result<()> {
    fn load(paths: &str) -> Result<Vec<serde_json::Value>> {
        let mut rows = vec![];
        for path in paths.split(',') {
            for line in fs::read_to_string(path)?
                .lines()
                .filter(|l| !l.trim().is_empty())
            {
                rows.push(serde_json::from_str(line)?);
            }
        }
        Ok(rows)
    }
    let (before, after) = (load(before_paths)?, load(after_paths)?);
    ensure!(before.len() == after.len(), "paired trial counts differ");
    let mut trials = vec![];
    for old in &before {
        let new = after
            .iter()
            .find(|row| row["id"] == old["id"] && row["seed"] == old["seed"])
            .context("missing paired trial")?;
        ensure!(
            old["budget_s"] == new["budget_s"],
            "requested search budgets differ"
        );
        ensure!(
            old["validation"]["ok"] == true && new["validation"]["ok"] == true,
            "invalid trial"
        );
        let b = old["result"]["length"]
            .as_f64()
            .context("baseline length")?;
        let n = new["result"]["length"]
            .as_f64()
            .context("candidate length")?;
        trials.push(serde_json::json!({
            "id": old["id"], "seed": old["seed"], "budget_s": old["budget_s"],
            "before_length_m": b/1000., "after_length_m": n/1000.,
            "before_efficiency_pct": 100.*old["result"]["efficiency"].as_f64().context("before efficiency")?,
            "after_efficiency_pct": 100.*new["result"]["efficiency"].as_f64().context("after efficiency")?,
            "fabric_saved_pct": 100.*(1.-n/b),
            "before_wall_s":old["wall_s"], "after_wall_s":new["wall_s"], "valid":true,
        }));
    }
    let ids: std::collections::BTreeSet<_> =
        trials.iter().map(|r| r["id"].as_str().unwrap()).collect();
    let mut cases = vec![];
    for id in ids {
        let rows: Vec<_> = trials.iter().filter(|r| r["id"] == id).collect();
        let mean = |key: &str| {
            rows.iter().map(|r| r[key].as_f64().unwrap()).sum::<f64>() / rows.len() as f64
        };
        cases.push(serde_json::json!({"id":id,"trials":rows.len(),
            "before_efficiency_pct":mean("before_efficiency_pct"),"after_efficiency_pct":mean("after_efficiency_pct"),
            "before_length_m":mean("before_length_m"),"after_length_m":mean("after_length_m"),
            "fabric_saved_pct":mean("fabric_saved_pct"),"before_wall_s":mean("before_wall_s"),"after_wall_s":mean("after_wall_s"),
            "saving_min_pct":rows.iter().map(|r|r["fabric_saved_pct"].as_f64().unwrap()).fold(f64::INFINITY,f64::min),
            "saving_max_pct":rows.iter().map(|r|r["fabric_saved_pct"].as_f64().unwrap()).fold(f64::NEG_INFINITY,f64::max),
        }));
    }
    let report = serde_json::json!({"cases":cases,"trials":trials,"all_valid":true});
    fs::write(output, serde_json::to_vec_pretty(&report)?)?;
    println!("{}", serde_json::to_string_pretty(&report["cases"])?);
    Ok(())
}
