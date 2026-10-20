//! R12 writer using the public numbered-layer AAMA conventions.

use crate::pattern::BetaFields;
use anyhow::{Context, Result, ensure};
use beta_marker_core::{MarkKind, Piece, Pt};
use std::fmt::Write;

pub(crate) struct Writer {
    pub out: String,
}

impl Writer {
    pub fn new() -> Self {
        Self { out: String::new() }
    }
    pub fn pair(&mut self, code: i32, value: impl std::fmt::Display) {
        // Writing into a String cannot fail.
        let _ = writeln!(self.out, "{code}\n{value}");
    }
    pub fn point(&mut self, code: i32, p: Pt) {
        self.pair(code, format_args!("{:.8}", p[0]));
        self.pair(code + 10, format_args!("{:.8}", p[1]));
    }
    pub fn text(&mut self, p: Pt, text: &str) {
        self.pair(0, "TEXT");
        self.pair(8, 15);
        self.point(10, p);
        self.pair(40, 3);
        self.pair(1, text.replace('\r', "").replace('\n', "\\P"));
    }
    pub fn line(&mut self, layer: i32, a: Pt, b: Pt) {
        self.pair(0, "LINE");
        self.pair(8, layer);
        self.point(10, a);
        self.point(11, b);
    }
    pub fn polyline(&mut self, layer: i32, points: &[Pt], closed: bool) {
        self.pair(0, "POLYLINE");
        self.pair(8, layer);
        self.pair(66, 1);
        self.point(10, [0.0, 0.0]);
        self.pair(70, i32::from(closed));
        for &p in points {
            self.pair(0, "VERTEX");
            self.pair(8, layer);
            self.point(10, p);
        }
        self.pair(0, "SEQEND");
        self.pair(8, layer);
    }
    pub fn header(&mut self, inches: bool, dialect: &str) {
        self.pair(
            999,
            format!(
                "BETA Marker {dialect}; original parametric patterns when used by make_testdata"
            ),
        );
        self.pair(0, "SECTION");
        self.pair(2, "HEADER");
        self.pair(9, "$ACADVER");
        self.pair(1, "AC1009");
        self.pair(9, "$INSUNITS");
        self.pair(70, if inches { 1 } else { 4 });
        self.pair(9, "$MEASUREMENT");
        self.pair(70, if inches { 0 } else { 1 });
        self.pair(0, "ENDSEC");
        self.pair(0, "SECTION");
        self.pair(2, "TABLES");
        self.pair(0, "TABLE");
        self.pair(2, "LAYER");
        self.pair(70, 10);
        for layer in [0, 1, 2, 3, 4, 6, 7, 8, 13, 15] {
            self.pair(0, "LAYER");
            self.pair(2, layer);
            self.pair(70, 0);
            self.pair(62, 7);
            self.pair(6, "CONTINUOUS");
        }
        self.pair(0, "ENDTAB");
        self.pair(0, "TABLE");
        self.pair(2, "APPID");
        self.pair(70, 1);
        self.pair(0, "APPID");
        self.pair(2, "BETA_MARK");
        self.pair(70, 0);
        self.pair(0, "ENDTAB");
        self.pair(0, "ENDSEC");
    }
}

/// Write AAMA-style R12 ASCII DXF in millimetres, with one inserted BLOCK per
/// piece. Standard TEXT stores name, size, quantity and material. A `BETA:` TEXT
/// extension preserves ID, mirrored count, rotation and buffer for round trips.
pub fn export_aama(pieces: &[Piece]) -> Result<String> {
    let mut w = Writer::new();
    w.header(false, "AAMA");
    w.pair(0, "SECTION");
    w.pair(2, "BLOCKS");
    for (i, p) in pieces.iter().enumerate() {
        ensure!(
            p.outline.len() >= 3,
            "piece {} has fewer than three outline points",
            p.id
        );
        ensure!(
            p.outline
                .iter()
                .chain(p.grain.iter().flatten())
                .chain(p.marks.iter().flat_map(|m| &m.points))
                .flatten()
                .all(|v| v.is_finite() && v.abs() <= 1e12),
            "piece {} has invalid coordinates",
            p.id
        );
        ensure!(
            p.buffer.is_finite() && p.buffer >= 0.0,
            "piece {} has invalid buffer",
            p.id
        );
        let name = format!("BETA_{i}");
        w.pair(0, "BLOCK");
        w.pair(8, 0);
        w.pair(2, &name);
        w.pair(70, 0);
        w.point(10, [0.0, 0.0]);
        w.pair(3, &name);
        w.pair(1, "");
        w.polyline(1, &p.outline, true);
        if let Some([a, b]) = p.grain {
            w.line(7, a, b);
        }
        for m in &p.marks {
            ensure!(!m.points.is_empty(), "piece {} has an empty mark", p.id);
            ensure!(
                m.kind != MarkKind::Drill || m.points.len() == 1,
                "piece {}: a drill must have one centre",
                p.id
            );
            ensure!(
                m.kind != MarkKind::Text || m.points.len() == 1,
                "piece {}: text must have one anchor",
                p.id
            );
            match m.kind {
                MarkKind::Text => {
                    if let Some(anchor) = m.points.first() {
                        w.text(*anchor, m.text.as_deref().unwrap_or(""));
                    }
                }
                MarkKind::Drill | MarkKind::Notch if m.points.len() == 1 => {
                    w.pair(0, "POINT");
                    w.pair(8, if m.kind == MarkKind::Drill { 13 } else { 4 });
                    w.point(10, m.points[0]);
                }
                MarkKind::Notch if m.points.len() == 2 => w.line(4, m.points[0], m.points[1]),
                MarkKind::Line if m.points.len() == 1 => {
                    w.pair(0, "POINT");
                    w.pair(8, 8);
                    w.point(10, m.points[0]);
                }
                MarkKind::Line | MarkKind::Notch | MarkKind::Drill => {
                    if !m.points.is_empty() {
                        w.polyline(
                            if m.kind == MarkKind::Line {
                                8
                            } else if m.kind == MarkKind::Drill {
                                13
                            } else {
                                4
                            },
                            &m.points,
                            false,
                        );
                    }
                }
            }
            w.pair(1001, "BETA_MARK");
            // XDATA strings have a 255-byte limit. JSON escapes preserve literal
            // paragraph markers, empty/None text, and metadata-looking labels.
            let json = serde_json::to_string(&m.text)?;
            let mut rest = json.as_str();
            while !rest.is_empty() {
                let mut end = rest.len().min(250);
                while !rest.is_char_boundary(end) {
                    end -= 1;
                }
                w.pair(1000, &rest[..end]);
                rest = &rest[end..];
            }
        }
        w.text([0.0, 0.0], &format!("Piece Name: {}", p.name));
        if let Some(s) = &p.size {
            w.text([0.0, 0.0], &format!("Size: {s}"));
        }
        if let Some(s) = &p.style {
            w.text([0.0, 0.0], &format!("Style Name: {s}"));
        }
        if let Some(s) = &p.material {
            w.text([0.0, 0.0], &format!("Material: {s}"));
        }
        let total = p
            .quantity
            .checked_add(p.mirror_quantity)
            .context("export quantity exceeds u32")?;
        w.text([0.0, 0.0], &format!("Quantity: {total}"));
        let beta = BetaFields {
            id: p.id.clone(),
            style: p.style.clone(),
            quantity: p.quantity,
            mirror_quantity: p.mirror_quantity,
            rotation: p.rotation,
            buffer: p.buffer,
        };
        w.text(
            [0.0, 0.0],
            &format!("BETA: {}", serde_json::to_string(&beta)?),
        );
        w.pair(0, "ENDBLK");
        w.pair(8, 0);
    }
    w.pair(0, "ENDSEC");
    w.pair(0, "SECTION");
    w.pair(2, "ENTITIES");
    w.text([0.0, 0.0], "Units: METRIC");
    for i in 0..pieces.len() {
        w.pair(0, "INSERT");
        w.pair(8, 0);
        w.pair(2, format!("BETA_{i}"));
        w.point(10, [0.0, 0.0]);
    }
    w.pair(0, "ENDSEC");
    w.pair(0, "EOF");
    Ok(w.out)
}
