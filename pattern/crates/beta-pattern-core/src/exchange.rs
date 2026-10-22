//! Marker integration and AAMA-style DXF with an explicit BETA grade-table extension.
use crate::{evaluate::grade, geometry::validate_polygon, model::*};
use anyhow::{Context, Result, ensure};
use beta_marker_core::{Fabric, JOB_FORMAT, Job, Order};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Build and validate a Marker job using selected sizes and a garment ratio.
pub fn to_job(
    doc: &Document,
    fabric: Fabric,
    ratio: BTreeMap<String, u32>,
    tolerance: f64,
) -> Result<Job> {
    ensure!(
        fabric.width.is_finite() && fabric.width > 0.,
        "fabric width must be finite and positive"
    );
    ensure!(
        !ratio.is_empty() && ratio.values().any(|n| *n > 0),
        "ratio must contain positive garment counts"
    );
    let mut pieces = vec![];
    for (size, count) in &ratio {
        for p in grade(doc, size, tolerance)? {
            if p.piece.quantity == 0 && p.piece.mirror_quantity == 0 {
                continue;
            }
            p.piece
                .quantity
                .checked_mul(*count)
                .context("normal copy count exceeds u32")?;
            p.piece
                .mirror_quantity
                .checked_mul(*count)
                .context("mirrored copy count exceeds u32")?;
            validate_polygon(&p.piece.outline)?;
            pieces.push(p.piece);
        }
    }
    let job = Job {
        format: JOB_FORMAT.into(),
        name: doc.name.clone(),
        fabric,
        pieces,
        order: Some(Order { sizes: ratio }),
        ..Job::default()
    };
    let issues = beta_marker_core::check_job(&job);
    ensure!(
        issues.is_empty(),
        "invalid Marker job: {}",
        issues
            .iter()
            .map(|i| i.message.as_str())
            .collect::<Vec<_>>()
            .join("; ")
    );
    Ok(job)
}
/// BETA's public grade-table extension. This is not a proprietary RUL format.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleTable {
    /// Always `beta-pattern.rules/v1`.
    pub format: String,
    /// Base size for relative displacements.
    pub base_size: String,
    /// Ordered sizes.
    pub sizes: Vec<String>,
    /// Named rule definitions.
    pub rules: BTreeMap<String, GradeRule>,
    /// Piece ID -> point ID -> rule ID.
    pub attachments: BTreeMap<String, BTreeMap<String, String>>,
    /// Size -> piece ID -> point ID -> evaluated point coordinates.
    pub points: BTreeMap<String, BTreeMap<String, BTreeMap<String, beta_marker_core::Pt>>>,
}
/// Export selected sizes together as inserted AAMA-style blocks with BETA comment metadata.
pub fn export_dxf(doc: &Document, sizes: &[String], tolerance: f64) -> Result<String> {
    ensure!(!sizes.is_empty(), "select at least one size");
    let mut pieces = vec![];
    let mut table = RuleTable {
        format: "beta-pattern.rules/v1".into(),
        base_size: doc.base_size.clone(),
        sizes: doc.sizes.clone(),
        rules: doc.rules.clone(),
        attachments: doc
            .pieces
            .iter()
            .map(|p| (p.id.clone(), p.grade_points.clone()))
            .collect(),
        points: BTreeMap::new(),
    };
    let mut seen = std::collections::BTreeSet::new();
    for size in sizes {
        ensure!(seen.insert(size), "duplicate export size '{size}'");
        let graded = grade(doc, size, tolerance)?;
        let mut point_map = BTreeMap::new();
        for (p, g) in doc.pieces.iter().zip(graded) {
            if g.piece.quantity == 0 && g.piece.mirror_quantity == 0 {
                continue;
            }
            point_map.insert(p.id.clone(), g.points);
            let mut piece = g.piece;
            // Layer 6 denotes a half pattern in our reader. This is already a
            // full piece, so retain the fold as an internal guide instead of
            // asking downstream readers to unfold the outline a second time.
            if let Some(axis) = piece.fold_line {
                piece.marks.push(beta_marker_core::Mark {
                    kind: beta_marker_core::MarkKind::Line,
                    points: axis.to_vec(),
                    text: Some("fold line (full symmetric piece)".into()),
                });
            }
            pieces.push(piece);
        }
        table.points.insert(size.clone(), point_map);
    }
    let json = serde_json::to_string(&table)?;
    let mut out = String::new();
    // A comment per Unicode-safe chunk avoids unbounded physical DXF lines.
    out.push_str("999\nBETA_PATTERN_RULES_BEGIN beta-pattern.rules/v1\n");
    let mut rest = json.as_str();
    while !rest.is_empty() {
        let mut end = rest.len().min(200);
        while !rest.is_char_boundary(end) {
            end -= 1;
        }
        out.push_str("999\nBETA_PATTERN_RULES_DATA ");
        out.push_str(&rest[..end]);
        out.push('\n');
        rest = &rest[end..];
    }
    out.push_str("999\nBETA_PATTERN_RULES_END\n");
    out.push_str(&beta_marker_dxf::export_aama(&pieces)?);
    Ok(out)
}
/// Read BETA grade metadata from DXF comments. Other readers may ignore these comments.
pub fn import_rule_table(dxf: &str) -> Result<RuleTable> {
    ensure!(dxf.len() <= 64 * 1024 * 1024, "DXF exceeds 64 MiB");
    let lines: Vec<_> = dxf.lines().collect();
    ensure!(lines.len() % 2 == 0, "incomplete DXF group pair");
    let mut active = false;
    let mut ended = false;
    let mut json = String::new();
    for pair in lines.as_chunks::<2>().0 {
        if pair[0].trim() != "999" {
            continue;
        }
        let s = pair[1];
        if s.starts_with("BETA_PATTERN_RULES_BEGIN ") {
            ensure!(!active && !ended, "duplicate rule table");
            active = true;
        } else if s == "BETA_PATTERN_RULES_END" {
            ensure!(active, "unexpected rule table end");
            active = false;
            ended = true;
        } else if let Some(data) = s.strip_prefix("BETA_PATTERN_RULES_DATA ") {
            ensure!(active, "rule data outside table");
            json.push_str(data);
        }
    }
    ensure!(ended && !active, "missing or incomplete BETA rule table");
    let table: RuleTable = serde_json::from_str(&json)?;
    ensure!(
        table.format == "beta-pattern.rules/v1",
        "unsupported rule table version"
    );
    Ok(table)
}
