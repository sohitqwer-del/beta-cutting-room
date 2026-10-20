//! Reproduce the CC BY 4.0 drafting exercises, without network access.

#[path = "support/patterns.rs"]
mod patterns;

use anyhow::Result;
use beta_marker_core::{MarkKind, Piece, Pt, geometry::area};
use std::{fmt::Write, path::PathBuf};

struct Dxf(String);
impl Dxf {
    fn pair(&mut self, code: i32, value: impl std::fmt::Display) {
        let _ = writeln!(self.0, "{code}\n{value}");
    }
    fn point(&mut self, code: i32, p: Pt, scale: f64) {
        self.pair(code, format_args!("{:.8}", p[0] * scale));
        self.pair(code + 10, format_args!("{:.8}", p[1] * scale));
    }
    fn text(&mut self, label: &str) {
        self.pair(0, "TEXT");
        self.pair(8, 15);
        self.point(10, [0.0, 0.0], 1.0);
        self.pair(40, 3);
        self.pair(1, label);
    }
    fn line(&mut self, layer: i32, a: Pt, b: Pt, scale: f64) {
        self.pair(0, "LINE");
        self.pair(8, layer);
        self.point(10, a, scale);
        self.point(11, b, scale);
    }
    fn poly(&mut self, layer: i32, points: &[Pt], scale: f64) {
        self.pair(0, "POLYLINE");
        self.pair(8, layer);
        self.pair(66, 1);
        self.point(10, [0.0, 0.0], scale);
        self.pair(70, 1);
        for &p in points {
            self.pair(0, "VERTEX");
            self.pair(8, layer);
            self.point(10, p, scale);
        }
        self.pair(0, "SEQEND");
        self.pair(8, layer);
    }
    fn header(&mut self, units: i32, dialect: &str) {
        self.pair(
            999,
            format!("Original Project BETA parametric {dialect} exercise; CC BY 4.0"),
        );
        self.pair(0, "SECTION");
        self.pair(2, "HEADER");
        self.pair(9, "$ACADVER");
        self.pair(1, "AC1009");
        self.pair(9, "$INSUNITS");
        self.pair(70, units);
        self.pair(0, "ENDSEC");
    }
}

fn drawing(pieces: &[Piece], inch: bool, dialect: &str) -> String {
    let scale = if inch { 1.0 / 25.4 } else { 1.0 };
    let mut d = Dxf(String::new());
    d.header(if inch { 1 } else { 4 }, dialect);
    d.pair(0, "SECTION");
    d.pair(2, "BLOCKS");
    for (i, p) in pieces.iter().enumerate() {
        d.pair(0, "BLOCK");
        d.pair(8, 0);
        d.pair(2, format!("PATTERN_{i}"));
        d.pair(70, 0);
        d.point(10, [0.0, 0.0], scale);
        d.poly(1, &p.outline, scale);
        if let Some([a, b]) = p.grain {
            d.line(7, a, b, scale);
        }
        for m in &p.marks {
            let layer = if m.kind == MarkKind::Drill { 13 } else { 4 };
            if m.points.len() == 2 {
                d.line(layer, m.points[0], m.points[1], scale);
            } else {
                d.pair(0, "POINT");
                d.pair(8, layer);
                d.point(10, m.points[0], scale);
            }
        }
        d.text(&format!("Piece Name: {}", p.name));
        d.text(&format!("Size: {}", p.size.as_deref().unwrap_or("")));
        d.text(&format!("Quantity: {}", p.quantity));
        d.text("Category: Original drafting exercise");
        d.text("Material: Cotton");
        if p.quantity == 2 && matches!(p.name.as_str(), "Front" | "Back" | "Sleeve") {
            d.text("Annotation: Intended pair; Quantity 2; use --pairs to split normal/mirrored copies");
        }
        d.pair(0, "ENDBLK");
    }
    d.pair(0, "ENDSEC");
    d.pair(0, "SECTION");
    d.pair(2, "ENTITIES");
    d.text(&format!(
        "Style Name: {}",
        pieces
            .first()
            .and_then(|p| p.style.as_deref())
            .unwrap_or("BETA")
    ));
    d.text(&format!(
        "Sample Size: {}",
        pieces
            .first()
            .and_then(|p| p.size.as_deref())
            .unwrap_or("M")
    ));
    d.text(if inch {
        "UNITS: ENGLISH"
    } else {
        "UNITS: METRIC"
    });
    for i in 0..pieces.len() {
        d.pair(0, "INSERT");
        d.pair(8, 0);
        d.pair(2, format!("PATTERN_{i}"));
        d.point(10, [0.0, 0.0], 1.0);
    }
    d.pair(0, "ENDSEC");
    d.pair(0, "EOF");
    d.0
}

fn generic() -> String {
    let mut d = Dxf(String::new());
    d.header(4, "generic");
    d.pair(0, "SECTION");
    d.pair(2, "ENTITIES");
    d.poly(
        0,
        &[[0.0, 0.0], [300.0, 0.0], [300.0, 500.0], [0.0, 500.0]],
        1.0,
    );
    d.poly(
        0,
        &[[40.0, 40.0], [80.0, 40.0], [80.0, 80.0], [40.0, 80.0]],
        1.0,
    );
    d.line(7, [150.0, 100.0], [150.0, 400.0], 1.0);
    d.poly(
        0,
        &[[400.0, 0.0], [650.0, 0.0], [600.0, 400.0], [400.0, 350.0]],
        1.0,
    );
    d.line(7, [500.0, 80.0], [500.0, 300.0], 1.0);
    d.pair(0, "ENDSEC");
    d.pair(0, "EOF");
    d.0
}

fn main() -> Result<()> {
    let destination = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/dxf");
    std::fs::create_dir_all(&destination)?;
    let mut manifest = serde_json::Map::new();
    for (name, pieces, inch, dialect) in [
        ("shirt-aama-mm.dxf", patterns::shirt(), false, "AAMA"),
        ("trouser-aama-inch.dxf", patterns::trouser(), true, "AAMA"),
        ("tee-astm-mm.dxf", patterns::tee(), false, "ASTM D6673"),
    ] {
        std::fs::write(destination.join(name), drawing(&pieces, inch, dialect))?;
        let expected: Vec<_> = pieces
            .iter()
            .map(|p| {
                serde_json::json!({
                    "name":p.name,"size":p.size,"area_mm2":area(&p.outline),"quantity":p.quantity,
                    "notches":p.marks.iter().filter(|m| m.kind == MarkKind::Notch).count(),
                    "drills":p.marks.iter().filter(|m| m.kind == MarkKind::Drill).count()
                })
            })
            .collect();
        manifest.insert(name.into(), serde_json::json!(expected));
        println!("{name}: {} pieces", pieces.len());
    }
    std::fs::write(destination.join("generic-polylines.dxf"), generic())?;
    std::fs::write(
        destination.join("expected.json"),
        serde_json::to_string_pretty(&manifest)?,
    )?;
    Ok(())
}
