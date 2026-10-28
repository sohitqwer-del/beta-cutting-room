//! Paired harness for mixed layers, obstacle repair and browser continuity.
//! three_study --prepare MARKER_ROOT OUT_CASES
//! three_study CASES OUTPUT_JSONL SECONDS SEED
use anyhow::{Context, Result, ensure};
use beta_marker_core::{Job, Layout, Placement, Zone};
use beta_marker_nest::{
    NestOptions,
    browser::{BrowserOptions, BrowserSession},
    nest,
};
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::Path, time::Instant};

#[derive(Serialize, Deserialize)]
struct Case {
    id: String,
    job: Job,
    #[serde(default)]
    fixed: Vec<Placement>,
    #[serde(default)]
    browser: bool,
}

fn prepare(root: &Path, out: &Path) -> Result<()> {
    let mut cases = vec![];
    for name in ["teagan", "simon", "titan", "hugo"] {
        let job: Job = serde_json::from_slice(&fs::read(
            root.join(format!("testdata/open/freesewing/{name}-ratio.job.json")),
        )?)?;
        if name == "teagan" || name == "hugo" {
            for layout in [Layout::Folded, Layout::Tubular] {
                let mut folded = job.clone();
                folded.fabric.width /= 2.;
                folded.fabric.layout = layout;
                for piece in &mut folded.pieces {
                    if piece.mirror_quantity == 0
                        && beta_marker_core::rules::mirror_symmetric(
                            &piece.outline,
                            piece.grain,
                            false,
                        )
                    {
                        piece.fold_line = piece.grain;
                    }
                }
                cases.push(Case {
                    id: format!("{name}-{layout:?}"),
                    job: folded,
                    fixed: vec![],
                    browser: false,
                });
            }
        }
        if name == "teagan" || name == "simon" {
            let mut damaged = job.clone();
            damaged.fabric.defects.push(Zone {
                id: "interior study defect".into(),
                polygon: vec![[1500., 300.], [1580., 300.], [1580., 420.], [1500., 420.]],
                note: None,
            });
            cases.push(Case {
                id: format!("{name}-interior"),
                job: damaged,
                fixed: vec![],
                browser: false,
            });
            let seed = nest(
                &job,
                &NestOptions {
                    time_limit_s: 0.1,
                    seed: Some(1),
                    ..Default::default()
                },
            )?;
            let mut fixed = vec![seed.placements[0].clone()];
            fixed[0].locked = true;
            let mut early = seed
                .placements
                .iter()
                .min_by(|a, b| {
                    let end = |pl: &Placement| {
                        let piece = job.pieces.iter().find(|p| p.id == pl.piece_id).unwrap();
                        beta_marker_core::geometry::bbox(&beta_marker_core::rules::cut_outline(
                            piece, pl,
                        ))
                        .unwrap()[2]
                    };
                    end(a).total_cmp(&end(b))
                })
                .unwrap()
                .clone();
            early.locked = true;
            cases.push(Case {
                id: format!("{name}-front-locked"),
                job: job.clone(),
                fixed: vec![early],
                browser: false,
            });
            cases.push(Case {
                id: format!("{name}-locked"),
                job: job.clone(),
                fixed,
                browser: false,
            });
            cases.push(Case {
                id: format!("{name}-browser"),
                job,
                fixed: vec![],
                browser: true,
            });
        }
    }
    fs::write(out, serde_json::to_vec_pretty(&cases)?)?;
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).is_some_and(|s| s == "--prepare") {
        return prepare(
            Path::new(args.get(2).context("marker root")?),
            Path::new(args.get(3).context("output")?),
        );
    }
    ensure!(args.len() == 5, "three_study CASES OUTPUT SECONDS SEED");
    let cases: Vec<Case> = serde_json::from_slice(&fs::read(&args[1])?)?;
    let seconds: f64 = args[3].parse()?;
    let seed: u64 = args[4].parse()?;
    let mut out = fs::File::create(&args[2])?;
    for case in cases {
        let started = Instant::now();
        let result = if case.browser {
            let mut session = BrowserSession::new(
                case.job.clone(),
                BrowserOptions {
                    time_limit_s: seconds,
                    round_time_s: 1.,
                    seed: Some(seed),
                    fixed: case.fixed.clone(),
                    ..Default::default()
                },
            )?;
            loop {
                let step = session.step()?;
                if step.done {
                    break Ok(step.result);
                }
            }
        } else {
            nest(
                &case.job,
                &NestOptions {
                    time_limit_s: seconds,
                    seed: Some(seed),
                    workers: Some(1),
                    fixed: case.fixed.clone(),
                    ..Default::default()
                },
            )
        };
        let row = match result {
            Ok(result) => {
                serde_json::json!({"id":case.id,"seed":seed,"budget_s":seconds,"wall_s":started.elapsed().as_secs_f64(),"validation":beta_marker_core::validate(&case.job,&result),"fixed_preserved":case.fixed.iter().all(|p|result.placements.contains(p)),"result":result})
            }
            Err(error) => serde_json::json!({"id":case.id,"seed":seed,"error":error.to_string()}),
        };
        writeln!(out, "{row}")?;
        out.flush()?;
        eprintln!(
            "{}: length {}, valid {}",
            row["id"], row["result"]["length"], row["validation"]["ok"]
        );
    }
    Ok(())
}
