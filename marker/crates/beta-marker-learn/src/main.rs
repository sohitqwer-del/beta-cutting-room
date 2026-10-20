//! Small offline command-line interface; outputs contain no source outlines.
use anyhow::{Context, Result, bail, ensure};
use beta_marker_learn::{
    Features, Model, PieceSummary, Shape, TaskSummary, feature_names, summarize,
    summarize_rotations,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env,
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    path::Path,
    time::Instant,
};

fn read_lines<T: serde::de::DeserializeOwned>(
    path: &str,
    mut f: impl FnMut(T) -> Result<()>,
) -> Result<()> {
    for (n, line) in BufReader::new(File::open(path)?).lines().enumerate() {
        let row = serde_json::from_str(&line?).with_context(|| format!("{path} line {}", n + 1))?;
        f(row)?;
    }
    Ok(())
}
fn output(path: &str) -> Result<BufWriter<File>> {
    Ok(BufWriter::new(File::create_new(path)?))
}
fn write_row<T: Serialize>(out: &mut impl Write, value: &T) -> Result<()> {
    serde_json::to_writer(&mut *out, value)?;
    writeln!(out)?;
    Ok(())
}
#[derive(Deserialize)]
struct ShapeInput {
    id: i64,
    outline: Vec<[f64; 2]>,
}
#[derive(Serialize, Deserialize)]
struct ShapeRow {
    id: i64,
    shape: Shape,
}
#[derive(Deserialize)]
struct RotationRow {
    id: usize,
    intervals: Vec<[f64; 3]>,
}
#[derive(Deserialize)]
struct TaskRow {
    id: u64,
    split: String,
    target: f64,
    width: f64,
    sheet_code: u8,
    extra_constraints: usize,
    pieces: Vec<(i64, u32, usize)>,
    seen_shape_fraction: f64,
}
#[derive(Serialize, Deserialize)]
struct FeatureRow {
    id: u64,
    split: String,
    target: f64,
    seen_shape_fraction: f64,
    features: Features,
}
fn main() -> Result<()> {
    let args: Vec<_> = env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("shapes") if args.len() == 4 => {
            let mut out = output(&args[3])?;
            let mut n = 0;
            read_lines(&args[2], |row: ShapeInput| {
                let shape = Shape::from_outline(&row.outline)
                    .with_context(|| format!("shape {}", row.id))?;
                write_row(&mut out, &ShapeRow { id: row.id, shape })?;
                n += 1;
                Ok(())
            })?;
            out.flush()?;
            eprintln!("measured {n} shapes");
        }
        Some("features") if args.len() == 6 => {
            let mut shapes = HashMap::new();
            read_lines(&args[2], |r: ShapeRow| {
                ensure!(shapes.insert(r.id, r.shape).is_none(), "duplicate shape");
                Ok(())
            })?;
            let mut rotations = HashMap::new();
            read_lines(&args[3], |r: RotationRow| {
                ensure!(
                    rotations
                        .insert(r.id, summarize_rotations(&r.intervals)?)
                        .is_none(),
                    "duplicate rotation"
                );
                Ok(())
            })?;
            let mut out = output(&args[5])?;
            let mut n = 0;
            read_lines(&args[4], |r: TaskRow| {
                ensure!(
                    ["train", "val", "test"].contains(&r.split.as_str())
                        && r.target > 0.0
                        && r.target <= 1.0,
                    "invalid label or split"
                );
                let pieces = r
                    .pieces
                    .iter()
                    .map(|(s, c, a)| {
                        Ok(PieceSummary {
                            shape: *shapes.get(s).context("missing shape")?,
                            count: *c,
                            rotations: *rotations.get(a).context("missing rotations")?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                let features = summarize(&TaskSummary {
                    width: r.width,
                    sheet_code: r.sheet_code,
                    extra_constraints: r.extra_constraints,
                    pieces,
                })?;
                write_row(
                    &mut out,
                    &FeatureRow {
                        id: r.id,
                        split: r.split,
                        target: r.target,
                        seen_shape_fraction: r.seen_shape_fraction,
                        features,
                    },
                )?;
                n += 1;
                Ok(())
            })?;
            out.flush()?;
            eprintln!("extracted {n} tasks; {} features", feature_names().len());
        }
        Some("predict") if args.len() == 4 || args.len() == 5 => {
            let model = Model::from_json(&std::fs::read_to_string(&args[2])?)?;
            let job = serde_json::from_reader(File::open(&args[3])?)?;
            let estimate = model.estimate_job(&job)?;
            if let Some(path) = args.get(4) {
                write_row(&mut output(path)?, &estimate)?;
            } else {
                println!("{}", serde_json::to_string_pretty(&estimate)?);
            }
        }
        Some("evaluate") if args.len() == 5 => {
            let model = Model::from_json(&std::fs::read_to_string(&args[2])?)?;
            let mut out = output(&args[4])?;
            let mut n = 0;
            let mut nanos = 0u128;
            read_lines(&args[3], |r: FeatureRow| {
                let start = Instant::now();
                let estimate = model.predict(&r.features)?;
                nanos += start.elapsed().as_nanos();
                write_row(
                    &mut out,
                    &serde_json::json!({"id":r.id,"prediction":estimate.efficiency,"interval":estimate.efficiency_interval,"outside_range":estimate.outside_training_range}),
                )?;
                n += 1;
                Ok(())
            })?;
            out.flush()?;
            eprintln!(
                "{n} predictions; inference mean {:.3} microseconds (excludes JSON I/O)",
                nanos as f64 / n as f64 / 1000.0
            );
        }
        Some("schema") if args.len() == 2 => {
            println!("{}", serde_json::to_string(&feature_names())?);
        }
        Some("audit-jobs") if args.len() == 5 => {
            let mut reference = HashMap::new();
            read_lines(&args[3], |r: FeatureRow| {
                reference.insert(r.id, r.features);
                Ok(())
            })?;
            let mut out = output(&args[4])?;
            read_lines(&args[2], |row: serde_json::Value| {
                let id = row["id"]
                    .as_str()
                    .context("missing sample id")?
                    .parse::<u64>()?;
                if row["tier"] != "A" {
                    return write_row(
                        &mut out,
                        &serde_json::json!({"id":id,"skipped":"orientation handicap"}),
                    );
                }
                let job = serde_json::from_value(row["job"].clone())?;
                match beta_marker_learn::features_from_job(&job) {
                    Ok(features) => {
                        let expected = reference.get(&id).context("missing task reference")?;
                        let max_difference = features
                            .values
                            .iter()
                            .zip(&expected.values)
                            .map(|(a, b)| (a - b).abs() / (1.0 + b.abs()))
                            .fold(0.0, f64::max);
                        write_row(
                            &mut out,
                            &serde_json::json!({"id":id,"max_relative_feature_difference":max_difference}),
                        )
                    }
                    Err(error) => write_row(
                        &mut out,
                        &serde_json::json!({"id":id,"skipped":error.to_string()}),
                    ),
                }
            })?;
            out.flush()?;
        }
        _ => bail!(
            "usage: {} shapes IN OUT | features SHAPE_STATS ROTATIONS TASKS OUT | predict MODEL JOB [OUT] | evaluate MODEL FEATURES OUT | audit-jobs SAMPLE FEATURES OUT | schema",
            Path::new(&args[0]).display()
        ),
    }
    Ok(())
}
