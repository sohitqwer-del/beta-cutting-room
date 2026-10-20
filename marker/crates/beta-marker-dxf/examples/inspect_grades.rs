//! Write local measurements and an overlaid SVG for an imported graded job.
//! Usage: cargo run -p beta-marker-dxf --example inspect_grades -- JOB.json OUT_DIR

use anyhow::{Context, Result, ensure};
use beta_marker_core::{
    Job,
    geometry::{area, bbox},
};
use geo::{Polygon, Validation};
use std::{collections::BTreeMap, fmt::Write, path::PathBuf};

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    ensure!(args.len() == 2, "expected JOB.json OUT_DIR");
    let job: Job = serde_json::from_slice(&std::fs::read(&args[0])?)?;
    let output = PathBuf::from(&args[1]);
    std::fs::create_dir_all(&output)?;
    let mut groups = BTreeMap::<String, Vec<_>>::new();
    for piece in &job.pieces {
        let key = piece
            .id
            .rsplit_once("-GRADE-")
            .map_or(piece.id.as_str(), |(a, _)| a);
        groups.entry(key.into()).or_default().push(piece);
    }
    let mut rows = vec![];
    let mut svg = String::new();
    let mut x = 20.0;
    let mut height = 0.0_f64;
    for (index, pieces) in groups.values().enumerate() {
        let points: Vec<_> = pieces
            .iter()
            .flat_map(|p| p.outline.iter().copied())
            .collect();
        let bounds = bbox(&points).context("empty piece group")?;
        let mut previous_area = None;
        let mut grows = true;
        write!(
            svg,
            "<g transform=\"translate({x} 30)\"><text x=\"0\" y=\"-8\" font-size=\"12\">Piece {}</text>",
            index + 1
        )?;
        for (size_index, piece) in pieces.iter().enumerate() {
            let polygon =
                Polygon::new(piece.outline.iter().map(|q| (q[0], q[1])).collect(), vec![]);
            let piece_area = area(&piece.outline);
            grows &= previous_area.is_none_or(|a| piece_area > a);
            previous_area = Some(piece_area);
            rows.push(serde_json::json!({"piece_id":piece.id, "size":piece.size,
                "area_mm2":piece_area,"bounds_mm":bbox(&piece.outline),
                "valid":polygon.is_valid(),"grain":piece.grain,
                "strictly_increasing_in_source_size_order_so_far":grows}));
            let colour = ["#0072b2", "#d55e00", "#009e73", "#cc79a7"][size_index % 4];
            write!(
                svg,
                "<polygon fill=\"none\" stroke=\"{colour}\" stroke-width=\"0.8\" points=\""
            )?;
            for q in &piece.outline {
                write!(svg, "{},{} ", q[0] - bounds[0], bounds[3] - q[1])?;
            }
            svg.push_str("\"/>");
        }
        svg.push_str("</g>");
        x += bounds[2] - bounds[0] + 30.0;
        height = height.max(bounds[3] - bounds[1] + 60.0);
    }
    std::fs::write(
        output.join("graded-measurements.json"),
        serde_json::to_vec_pretty(&rows)?,
    )?;
    let document = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {x} {height}\" role=\"img\" aria-label=\"Graded cut outlines overlaid by size\"><rect width=\"100%\" height=\"100%\" fill=\"white\"/>{svg}</svg>"
    );
    std::fs::write(output.join("graded-outlines.svg"), document)?;
    println!(
        "Measured {} outlines. Colour order repeats blue, orange, green, purple in source size order.",
        rows.len()
    );
    Ok(())
}
