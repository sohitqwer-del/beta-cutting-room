//! Rebuild the small, attributed fixtures from repository-owned open test data.
use anyhow::{Context, Result};
use beta_marker_core::*;
use std::{fs, path::PathBuf};

fn reference(piece: &str, point: &str) -> PointRef {
    PointRef {
        piece_id: format!("SIMON-{piece}-M"),
        point_id: point.into(),
    }
}

fn main() -> Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = root.join("../../testdata/open/freesewing");
    let dest = root.join("tests/fixtures");
    fs::create_dir_all(&dest)?;
    fs::copy(source.join("LICENSE.MIT"), dest.join("LICENSE.MIT"))?;
    let mut job: Job = serde_json::from_slice(&fs::read(source.join("simon-M.job.json"))?)?;
    job.name = "Simon M: 40 mm checks".into();
    job.fabric.repeat = Some(Repeat {
        period_x: Some(40.0),
        period_y: Some(40.0),
        tolerance_mm: 1.0,
        ..Default::default()
    });
    let points = [
        ("FRONTLEFT", "centre", [0.0, -470.0]),
        ("FRONTRIGHT", "centre", [0.0, -470.0]),
        ("FRONTLEFT", "pocket", [120.0, -250.0]),
        ("FRONTLEFT", "armhole", [220.75, -157.5187]),
        ("SLEEVE", "armhole", [102.1587, 92.971]),
        ("YOKE", "join", [222.9582, -120.7893]),
        ("BACK", "join", [222.9582, -120.7893]),
    ];
    for (piece, id, point) in points {
        let p = job
            .pieces
            .iter_mut()
            .find(|p| p.id == format!("SIMON-{piece}-M"))
            .context("source piece missing")?;
        p.match_points.push(MatchPoint {
            id: id.into(),
            point,
        });
    }
    job.pieces.push(Piece {
        id: "SIMON-POCKET-M".into(),
        name: "own rectangular patch pocket".into(),
        style: Some("simon".into()),
        size: Some("M".into()),
        material: Some("fabric".into()),
        outline: vec![[0.0, 0.0], [130.0, 0.0], [130.0, -150.0], [0.0, -150.0]],
        grain: Some([[0.0, -150.0], [0.0, 0.0]]),
        quantity: 1,
        match_points: vec![MatchPoint {
            id: "top_left".into(),
            point: [0.0, 0.0],
        }],
        ..Default::default()
    });
    job.match_rules = vec![
        MatchRule {
            a: reference("FRONTLEFT", "centre"),
            b: Some(reference("FRONTRIGHT", "centre")),
            axes: MatchAxes::Both,
            offset: [0.0; 2],
        },
        MatchRule {
            a: reference("POCKET", "top_left"),
            b: Some(reference("FRONTLEFT", "pocket")),
            axes: MatchAxes::Both,
            offset: [0.0; 2],
        },
        MatchRule {
            a: reference("SLEEVE", "armhole"),
            b: Some(reference("FRONTLEFT", "armhole")),
            axes: MatchAxes::Y,
            offset: [0.0; 2],
        },
        MatchRule {
            a: reference("YOKE", "join"),
            b: Some(reference("BACK", "join")),
            axes: MatchAxes::X,
            offset: [0.0; 2],
        },
    ];
    job.meta.insert("fixture_notes".into(), serde_json::json!("FreeSewing Simon 4.10.2, MIT. Original cut outlines unchanged. Match points: centre front waist notches; own pocket top-left and corresponding front position; front/sleeve armhole notches; yoke/back joining notches. Added pocket is our 130 x 150 mm rectangular test piece, not a FreeSewing pattern."));
    fs::write(dest.join("simon-check.job.json"), serde_json::to_vec(&job)?)?;
    let ratio: Job = serde_json::from_slice(&fs::read(source.join("simon-ratio.job.json"))?)?;
    for (label, axis, band_mm) in [
        ("length", ShadeAxis::Length, 3000.0),
        ("width", ShadeAxis::Width, 750.0),
    ] {
        let mut shade = ratio.clone();
        shade.name = format!("Simon ratio: {label} shade bands");
        shade.fabric.shade = Some(ShadeRule { axis, band_mm });
        fs::write(
            dest.join(format!("simon-shade-{label}.job.json")),
            serde_json::to_vec(&shade)?,
        )?;
    }
    Ok(())
}
