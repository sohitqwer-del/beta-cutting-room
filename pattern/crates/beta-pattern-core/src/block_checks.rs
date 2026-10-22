//! Measured block quality and points of measure. Never certifies physical fit.
use crate::{
    blocks::BlockKind,
    evaluate::{self, Evaluator},
    geometry::*,
    model::*,
};
use anyhow::{Context, Result, ensure};
use beta_marker_core::Pt;
use serde::Serialize;
use std::collections::BTreeMap;

/// One measured assertion; ranges are inclusive. Units are explicit.
#[derive(Debug, Clone, Serialize)]
#[allow(missing_docs)]
pub struct CheckValue {
    pub id: String,
    pub category: String,
    pub value: f64,
    pub unit: String,
    pub min: f64,
    pub max: f64,
    pub pass: bool,
    pub note: String,
}
/// A size never disappears when its geometry fails.
#[derive(Debug, Clone, Serialize)]
#[allow(missing_docs)]
pub struct SizeChecks {
    pub size: String,
    pub pass: bool,
    pub checks: Vec<CheckValue>,
    pub errors: Vec<String>,
}
/// Recomputed graded geometry, plus explicit scope limits.
#[derive(Debug, Clone, Serialize)]
#[allow(missing_docs)]
pub struct BlockReport {
    pub block: BlockKind,
    pub sampling_mm: f64,
    pub pass: bool,
    pub sizes: Vec<SizeChecks>,
    pub limitations: Vec<String>,
}
/// A finished dimension and its change from the previous ordered size.
#[derive(Debug, Clone, Serialize)]
#[allow(missing_docs)]
pub struct SpecValue {
    pub size: String,
    pub measured_mm: f64,
    pub body_mm: Option<f64>,
    pub ease_mm: Option<f64>,
    pub target_mm: Option<f64>,
    pub grade_step_mm: Option<f64>,
    pub pass: Option<bool>,
}
/// Factory point of measure. Tolerance is a proposed QC tolerance, not a standard.
#[derive(Debug, Clone, Serialize)]
#[allow(missing_docs)]
pub struct PointOfMeasure {
    pub id: String,
    pub name: String,
    pub method: String,
    pub tolerance_mm: f64,
    pub values: Vec<SpecValue>,
}
/// Millimetre spec sheet measured on stitch lines, before shrinkage or wash.
#[derive(Debug, Clone, Serialize)]
#[allow(missing_docs)]
pub struct SpecSheet {
    pub schema: String,
    pub style: String,
    pub block: BlockKind,
    pub unit: String,
    pub sizes: Vec<String>,
    pub base_size: String,
    pub rows: Vec<PointOfMeasure>,
    pub notes: Vec<String>,
}

fn kind(doc: &Document) -> Result<BlockKind> {
    serde_json::from_value(
        doc.meta
            .get("block")
            .context("Choose a generated block for this report.")?
            .clone(),
    )
    .context("Unknown block")
}

/// Default edge/POM mappings apply only to the original set of active block pieces.
/// Linked topology changes require new operator seam mappings and a reviewed spec.
pub fn original_topology(doc: &Document) -> Result<()> {
    if let Some(reason) = doc
        .meta
        .get("block_review_required")
        .and_then(|v| v.as_str())
    {
        anyhow::bail!("{reason}");
    }
    let k = kind(doc)?;
    let expected = crate::blocks::catalog()
        .into_iter()
        .find(|b| b.id == k)
        .context("Unknown block")?
        .pieces;
    let active: Vec<_> = doc
        .pieces
        .iter()
        .filter(|p| p.quantity > 0 || p.mirror_quantity > 0)
        .collect();
    ensure!(
        active.len() == expected.len()
            && expected.iter().all(|id| active.iter().any(|p| p.id == *id)),
        "Active block pieces changed. Review seam mappings and points of measure for the production pieces; archived source checks are not approval."
    );
    for p in active {
        ensure!(
            !p.edges
                .iter()
                .any(|e| matches!(doc.drawing.get(&e.object), Some(Object::DerivedPath { .. }))),
            "Block topology changed. Review replacement seam mappings and points of measure."
        );
    }
    Ok(())
}
fn upper(k: BlockKind) -> bool {
    matches!(
        k,
        BlockKind::TShirt | BlockKind::Shirt | BlockKind::Polo | BlockKind::Kurta
    )
}
fn piece<'a>(g: &'a [GradedPiece], id: &str) -> Result<&'a GradedPiece> {
    g.iter()
        .find(|p| p.piece.id.rsplit_once('-').is_some_and(|(n, _)| n == id))
        .or_else(|| g.iter().find(|p| p.piece.id == id))
        .with_context(|| format!("Missing block piece {id}"))
}
fn edge(g: &GradedPiece, i: usize) -> Result<&[Pt]> {
    g.seam_edges
        .get(i)
        .map(Vec::as_slice)
        .filter(|p| p.len() > 1)
        .context("Block stitch edge missing")
}
fn start(g: &GradedPiece, i: usize) -> Result<Pt> {
    Ok(edge(g, i)?[0])
}
fn end(g: &GradedPiece, i: usize) -> Result<Pt> {
    Ok(*edge(g, i)?.last().unwrap())
}
fn len(g: &GradedPiece, i: usize) -> Result<f64> {
    Ok(length(edge(g, i)?))
}
fn v(e: &mut Evaluator<'_>, name: &str) -> Result<f64> {
    e.value(&Value::Formula(format!("v.{name}")))
}
fn m(doc: &Document, size: &str, name: &str) -> Result<f64> {
    doc.measurements
        .get(size)
        .and_then(|r| r.get(name))
        .copied()
        .with_context(|| format!("Missing {name} in {size}"))
}
/// Width of the stitch polygon at a specified y. End vertices count once.
fn width(g: &GradedPiece, y: f64) -> Result<f64> {
    let mut xs = vec![];
    for (a, b) in g
        .seam
        .iter()
        .zip(g.seam.iter().cycle().skip(1))
        .take(g.seam.len())
    {
        if (a[1] - b[1]).abs() < 1e-8 {
            if (y - a[1]).abs() < 1e-7 {
                xs.extend([a[0], b[0]]);
            }
        } else if y >= a[1].min(b[1]) - 1e-8 && y <= a[1].max(b[1]) + 1e-8 {
            xs.push(a[0] + (y - a[1]) / (b[1] - a[1]) * (b[0] - a[0]));
        }
    }
    ensure!(
        xs.len() >= 2,
        "POM section does not cross the stitch outline"
    );
    Ok(xs.iter().copied().fold(f64::NEG_INFINITY, f64::max)
        - xs.into_iter().fold(f64::INFINITY, f64::min))
}
/// One side's complete armhole stitch length: front plus back (and the yoke part on a shirt).
fn armhole(g: &[GradedPiece], k: BlockKind) -> Result<f64> {
    let base = len(piece(g, "front")?, 2)? + len(piece(g, "back")?, 2)?;
    Ok(match yoke_edges(g, k)? {
        Some((_, Some(arm))) => base + len(piece(g, "yoke")?, arm)?,
        _ => base,
    })
}
/// Shirt yoke seam edge and yoke armhole edge. Shirts saved by 0.6 have a 5-edge yoke
/// whose seam passes through the shoulder point, so it has no armhole part.
fn yoke_edges(g: &[GradedPiece], k: BlockKind) -> Result<Option<(usize, Option<usize>)>> {
    if k != BlockKind::Shirt {
        return Ok(None);
    }
    Ok(Some(match piece(g, "yoke")?.seam_edges.len() {
        5 => (4, None),
        7 => (5, Some(6)),
        _ => anyhow::bail!("Unknown yoke topology"),
    }))
}
/// Crotch extension beyond the hip line and the panel's hip width (trousers only).
fn extension(g: &GradedPiece) -> Result<(f64, f64)> {
    let hip = width(g, start(g, 3)?[1] - 200.)?;
    Ok((start(g, 2)?[0] - start(g, 5)?[0] - hip, hip))
}
type Dimension = (String, String, String, f64, f64, Option<(f64, f64)>);
fn dimensions(
    doc: &Document,
    size: &str,
    g: &[GradedPiece],
    k: BlockKind,
) -> Result<Vec<Dimension>> {
    let mut e = Evaluator::new(doc, size, 0.05)?;
    let f = piece(g, "front")?;
    let b = piece(g, "back")?;
    let mut out = vec![];
    let mut add =
        |id: &str, name: &str, method: &str, tol: f64, actual: f64, target: Option<(f64, f64)>| {
            out.push((id.into(), name.into(), method.into(), tol, actual, target));
        };
    if upper(k) {
        let front_factor = if k == BlockKind::Shirt { 2. } else { 1. };
        let chest =
            width(f, start(f, 2)?[1] - 10.)? * front_factor + width(b, start(b, 2)?[1] - 10.)?;
        add(
            "chest_half",
            "Half chest",
            "Flat, 10 mm below each underarm stitch point; total front/back circumference divided by 2.",
            6.,
            chest / 2.,
            Some((m(doc, size, "chest")? / 2., v(&mut e, "ease_chest")? / 2.)),
        );
        add(
            "body_length_hps",
            "Body length HPS",
            "Vertical from high shoulder point (neck/shoulder join) to centre-front hem stitch line.",
            10.,
            end(f, 3)?[1] - start(f, 0)?[1],
            Some((
                m(
                    doc,
                    size,
                    if k == BlockKind::Kurta {
                        "tunic_length"
                    } else {
                        "body_length"
                    },
                )?,
                0.,
            )),
        );
        let hem = len(f, 0)? * 2. + len(b, 0)? * 2.;
        add(
            "hem_half",
            "Half hem sweep",
            "Finished front plus back hem stitch lengths divided by 2.",
            6.,
            hem / 2.,
            None,
        );
        add(
            "neck_width",
            "Neck width",
            "Horizontal between high shoulder points on the front, excluding binding or collar.",
            3.,
            2. * (end(f, 3)?[0] - end(f, 4)?[0]).abs(),
            None,
        );
        add(
            "shoulder",
            "Shoulder seam",
            "Along one shoulder stitch seam, neckline to armhole.",
            3.,
            len(f, 3)?,
            Some((m(doc, size, "shoulder")?, 0.)),
        );
        let woven = matches!(k, BlockKind::Shirt | BlockKind::Kurta);
        add(
            "armhole_depth",
            "Armhole depth",
            "Vertical from high shoulder point to the underarm stitch point.",
            5.,
            end(f, 3)?[1] - start(f, 2)?[1],
            Some((m(doc, size, "arm_depth")?, if woven { 20. } else { 0. })),
        );
        add(
            "armhole",
            "Armhole, curved",
            "Along one complete armhole stitch line, front plus back (and yoke on a shirt).",
            6.,
            armhole(g, k)?,
            None,
        );
        add(
            "across_shoulder",
            "Across shoulder",
            "Straight between the two shoulder points (shoulder/armhole joins), front.",
            5.,
            if k == BlockKind::Shirt {
                2. * (end(f, 2)?[0] - start(f, 5)?[0]).abs()
            } else {
                (end(f, 2)?[0] - start(f, 7)?[0]).abs()
            },
            None,
        );
        add(
            "front_neck_drop",
            "Front neck drop",
            "Vertical from high shoulder point to centre-front neck stitch line.",
            3.,
            end(f, 3)?[1] - end(f, 4)?[1],
            None,
        );
        let (bn, bs, be) = if k == BlockKind::Shirt {
            (piece(g, "yoke")?, 0, 1)
        } else {
            (b, 3, 4)
        };
        add(
            "back_neck_drop",
            "Back neck drop",
            "Vertical from high shoulder point to centre-back neck stitch line.",
            3.,
            end(bn, bs)?[1] - end(bn, be)?[1],
            None,
        );
        let s = piece(g, "sleeve")?;
        let cuff = if k == BlockKind::Shirt {
            len(piece(g, "cuff")?, 1)? / 2.
        } else {
            0.
        };
        add(
            "sleeve_length",
            "Sleeve length",
            "Vertical from sleeve crown to hem; includes half the unfolded cuff depth on a shirt.",
            if matches!(k, BlockKind::Shirt | BlockKind::Kurta) {
                10.
            } else {
                5.
            },
            end(s, 2)?[1] - start(s, 0)?[1] + cuff,
            Some((
                m(
                    doc,
                    size,
                    if matches!(k, BlockKind::Shirt | BlockKind::Kurta) {
                        "sleeve_length"
                    } else {
                        "short_sleeve"
                    },
                )?,
                0.,
            )),
        );
        let bicep = (start(s, 2)?[0] - end(s, 3)?[0]).abs();
        let ease = doc
            .variables
            .get("ease_bicep")
            .map(|x| e.value(x))
            .transpose()?
            .unwrap_or(if matches!(k, BlockKind::Shirt | BlockKind::Kurta) {
                80.
            } else {
                60.
            });
        add(
            "bicep_half",
            "Half bicep",
            "Across sleeve at cap base, stitch line to stitch line, divided by 2.",
            4.,
            bicep / 2.,
            Some((m(doc, size, "bicep")? / 2., ease / 2.)),
        );
        add(
            "sleeve_opening_half",
            "Half sleeve opening",
            "Sleeve hem stitch length divided by 2; shirt cuff joining edge, before fastening overlap.",
            3.,
            len(s, 0)? / 2.,
            Some((
                m(
                    doc,
                    size,
                    if matches!(k, BlockKind::Shirt | BlockKind::Kurta) {
                        "wrist"
                    } else {
                        "bicep"
                    },
                )? / 2.,
                if matches!(k, BlockKind::Shirt | BlockKind::Kurta) {
                    30.
                } else {
                    20.
                },
            )),
        );
    } else {
        let skirt = k == BlockKind::ALineSkirt;
        let track = k == BlockKind::TrackPant;
        let wi = if skirt { 0 } else { 3 };
        let hi = if skirt { 3 } else { 0 };
        let factor = if skirt { 1. } else { 2. };
        let waist = factor * (len(f, wi)? + len(b, wi)?);
        add(
            "waist_half",
            "Half waist joining line",
            "Waist seam circumference divided by 2; track pant is the extended casing, not relaxed elastic.",
            6.,
            waist / 2.,
            Some((
                m(doc, size, if track { "hip" } else { "waist" })? / 2.,
                v(&mut e, if track { "ease_hip" } else { "ease_waist" })? / 2.,
            )),
        );
        let hip =
            factor * (width(f, start(f, wi)?[1] - 200.)? + width(b, start(b, wi)?[1] - 200.)?);
        add(
            "hip_half",
            "Half hip",
            "Horizontal 200 mm below waist joining line, circumference divided by 2.",
            6.,
            hip / 2.,
            Some((m(doc, size, "hip")? / 2., v(&mut e, "ease_hip")? / 2.)),
        );
        add(
            "body_length",
            "Length below waistband",
            "Vertical from waist joining line to hem stitch line; waistband depth excluded.",
            10.,
            start(f, wi)?[1] - start(f, hi)?[1],
            Some((
                m(
                    doc,
                    size,
                    if skirt {
                        "skirt_length"
                    } else {
                        "trouser_length"
                    },
                )?,
                0.,
            )),
        );
        add(
            "waistband_depth",
            "Finished waistband depth",
            "Half the unfolded waistband depth, excluding joining allowances.",
            2.,
            len(piece(g, "waistband")?, 1)? / 2.,
            None,
        );
        add(
            "hem_half",
            "Half hem sweep",
            "Skirt: half total hem sweep. Trousers: half one leg opening.",
            if skirt { 6. } else { 4. },
            (len(f, hi)? + len(b, hi)?) / 2.,
            if skirt {
                None
            } else {
                Some((m(doc, size, "ankle")? / 2., 40.))
            },
        );
        if !skirt {
            add(
                "inseam",
                "Inseam",
                "Along front inner-leg stitch seam, crotch to hem.",
                10.,
                len(f, 5)?,
                Some((m(doc, size, "inseam")?, 0.)),
            );
            add(
                "outseam",
                "Outseam below waistband",
                "Along front outer-leg stitch seam, waist joining line to hem.",
                10.,
                len(f, 1)? + len(f, 2)?,
                None,
            );
            add(
                "thigh_half",
                "Half thigh",
                "Across one leg 25 mm below the crotch point, front plus back panel, divided by 2.",
                4.,
                (width(f, start(f, 5)?[1] - 25.)? + width(b, start(b, 5)?[1] - 25.)?) / 2.,
                None,
            );
            add(
                "front_rise",
                "Front rise",
                "Along front crotch stitch curve, waist joining line to crotch point; waistband excluded.",
                5.,
                len(f, 4)?,
                None,
            );
            add(
                "back_rise",
                "Back rise",
                "Along back crotch stitch curve, waist joining line to crotch point; waistband excluded.",
                5.,
                len(b, 4)?,
                None,
            );
        }
    }
    Ok(out)
}
/// Read actual graded stitch geometry, never calculate a spec from chart values alone.
pub fn spec_sheet(doc: &Document) -> Result<SpecSheet> {
    original_topology(doc)?;
    let k = kind(doc)?;
    let mut rows: Vec<PointOfMeasure> = vec![];
    for size in &doc.sizes {
        let g = evaluate::grade(doc, size, 0.05)?;
        for (id, name, method, tolerance_mm, measured_mm, target) in dimensions(doc, size, &g, k)? {
            let index = rows.iter().position(|r| r.id == id).unwrap_or_else(|| {
                rows.push(PointOfMeasure {
                    id,
                    name,
                    method,
                    tolerance_mm,
                    values: vec![],
                });
                rows.len() - 1
            });
            let row = &mut rows[index];
            let grade_step_mm = row.values.last().map(|p| measured_mm - p.measured_mm);
            let target_mm = target.map(|(b, e)| b + e);
            row.values.push(SpecValue {
                size: size.clone(),
                measured_mm,
                body_mm: target.map(|x| x.0),
                ease_mm: target.map(|x| x.1),
                target_mm,
                grade_step_mm,
                pass: target_mm.map(|t| (t - measured_mm).abs() <= tolerance_mm),
            });
        }
    }
    Ok(SpecSheet{schema:"beta-pattern.spec/v1".into(),style:doc.name.clone(),block:k,unit:"mm".into(),sizes:doc.sizes.clone(),base_size:doc.base_size.clone(),rows,notes:vec![
        "Proposed QC tolerances need buyer approval. Measure sewn, unwashed, unstretched garments flat; no shrinkage compensation is included.".into(),
        "Values are stitch-line design measurements, not cut-outline dimensions. Grade steps follow the chart's size order; the first is null.".into(),
        "Design-only dimensions have no body/ease target and pass is null. An elastic casing is reported extended; relaxed elastic length is not specified.".into(),
        "Blocks require a sewn sample and pattern-master review of balance, closures, collar shaping and fabric behaviour.".into()]})
}
#[allow(clippy::too_many_arguments)] fn add( // 0.7.0: tested code kept; group arguments next round.
    out: &mut Vec<CheckValue>,
    id: impl Into<String>,
    cat: &str,
    value: f64,
    unit: &str,
    min: f64,
    max: f64,
    note: &str,
) {
    out.push(CheckValue {
        id: id.into(),
        category: cat.into(),
        value,
        unit: unit.into(),
        min,
        max,
        pass: value.is_finite() && value >= min - 1e-6 && value <= max + 1e-6,
        note: note.into(),
    });
}
fn endpoint_tangent(
    doc: &Document,
    size: &str,
    g: &GradedPiece,
    pi: usize,
    at_end: bool,
) -> Result<Pt> {
    let source = doc
        .pieces
        .iter()
        .find(|p| g.piece.id == format!("{}-{size}", p.id) || g.piece.id == p.id)
        .context("Source missing")?;
    let ed = source
        .edges
        .get(pi)
        .context("Block edge missing after edit")?;
    if source.grade_points.is_empty() {
        let mut e = Evaluator::new(doc, size, 0.001)?;
        let curve = crate::paths::Curve::from_evaluator(doc, &mut e, &ed.object)?;
        if curve.native() {
            let (_, d) = curve.sample(if at_end != ed.reversed { 1. } else { 0. })?;
            return unit(mul(d, if ed.reversed { -1. } else { 1. }));
        }
        if let Some(Object::CubicPath { segments }) = doc.drawing.get(&ed.object) {
            let last = at_end != ed.reversed;
            let s = if last {
                segments.last()
            } else {
                segments.first()
            }
            .context("Empty path")?;
            let d = if last {
                sub(e.point(&s[3])?, e.point(&s[2])?)
            } else {
                sub(e.point(&s[1])?, e.point(&s[0])?)
            };
            return unit(mul(d, if ed.reversed { -1. } else { 1. }));
        }
    }
    let p = edge(g, pi)?;
    unit(if at_end {
        sub(p[p.len() - 1], p[p.len() - 2])
    } else {
        sub(p[1], p[0])
    })
}
fn corner(doc: &Document, size: &str, g: &GradedPiece, a: usize, b: usize) -> Result<f64> {
    let u = mul(endpoint_tangent(doc, size, g, a, true)?, -1.);
    let v = endpoint_tangent(doc, size, g, b, false)?;
    Ok(dot(u, v).clamp(-1., 1.).acos().to_degrees())
}
fn size_checks(doc: &Document, size: &str, k: BlockKind) -> Result<Vec<CheckValue>> {
    let g = evaluate::grade(doc, size, 0.05)?;
    let mut out = vec![];
    let f = piece(&g, "front")?;
    let b = piece(&g, "back")?;
    let mut e = Evaluator::new(doc, size, 0.05)?;
    let mut seam = |id: &str, a: f64, b: f64| {
        add(
            &mut out,
            id,
            "seam",
            a - b,
            "mm",
            -1.,
            1.,
            "Actual joining stitch lengths; allowance excluded.",
        )
    };
    if upper(k) {
        seam("side", len(f, 1)?, len(b, 1)?);
        seam(
            "shoulder",
            len(f, 3)?,
            len(
                piece(
                    &g,
                    if k == BlockKind::Shirt {
                        "yoke"
                    } else {
                        "back"
                    },
                )?,
                if k == BlockKind::Shirt { 0 } else { 3 },
            )?,
        );
        let s = piece(&g, "sleeve")?;
        seam("sleeve_underarm", len(s, 1)?, len(s, 4)?);
        if k == BlockKind::Shirt {
            let (join, _) = yoke_edges(&g, k)?.context("Shirt yoke")?;
            seam("yoke", len(b, 3)?, len(piece(&g, "yoke")?, join)?);
            seam("cuff", len(s, 0)?, len(piece(&g, "cuff")?, 0)?);
            seam(
                "collar_stand",
                len(piece(&g, "collar")?, 0)?,
                len(piece(&g, "stand")?, 0)?,
            );
        }
        let neck = 2. * len(f, 4)?
            + if k == BlockKind::Shirt {
                len(piece(&g, "yoke")?, 1)? + len(piece(&g, "yoke")?, 2)?
            } else {
                2. * len(b, 4)?
            };
        let component = piece(
            &g,
            match k {
                BlockKind::TShirt => "neck_rib",
                BlockKind::Shirt => "stand",
                BlockKind::Polo => "collar",
                _ => "neck_facing",
            },
        )?;
        let ratio = if k == BlockKind::TShirt { 0.85 } else { 1. };
        seam("neck_join", len(component, 0)?, neck * ratio);
        add(
            &mut out,
            "neck_join_ratio",
            "ease",
            len(component, 0)? / neck,
            "ratio",
            ratio - 0.003,
            ratio + 0.003,
            "T-shirt rib is 0.85 times the neckline (15% shorter; about 17.65% extension to sew). Others 1:1.",
        );
        let cap = len(s, 2)? + len(s, 3)?;
        let arm = armhole(&g, k)?;
        let requested = v(&mut e, "cap_ease")?;
        add(
            &mut out,
            "cap_ease_error",
            "seam",
            cap - arm - requested,
            "mm",
            -1.,
            1.,
            "Sleeve cap minus total armhole minus requested ease.",
        );
        for (id, cap, arm) in [
            ("cap_front", len(s, 2)?, len(f, 2)?),
            ("cap_back", len(s, 3)?, arm - len(f, 2)?),
        ] {
            add(
                &mut out,
                id,
                "seam",
                cap - arm - requested / 2.,
                "mm",
                -1.,
                1.,
                "Equal front/back cap ease allocation in this symmetric starter.",
            );
        }
        let woven = matches!(k, BlockKind::Shirt | BlockKind::Kurta);
        add(
            &mut out,
            "cap_ease_percent",
            "ease",
            100. * (cap - arm) / arm,
            "percent",
            if woven { 1. } else { -1. },
            if woven { 5. } else { 2. },
            "BETA starting range: woven 1-5%; knit -1 to +2%. Fabric and sample approval required.",
        );
        for (id, p, a, b, multiplier) in [
            (
                "front_neck_at_center",
                f,
                4,
                5,
                if k == BlockKind::Shirt { 2. } else { 1. },
            ),
            (
                "back_neck_at_center",
                piece(
                    &g,
                    if k == BlockKind::Shirt {
                        "yoke"
                    } else {
                        "back"
                    },
                )?,
                if k == BlockKind::Shirt { 1 } else { 4 },
                if k == BlockKind::Shirt { 2 } else { 5 },
                1.,
            ),
            ("sleeve_crown", s, 2, 3, 1.),
        ] {
            add(
                &mut out,
                id,
                "tangent",
                (180. - corner(doc, size, p, a, b)? * multiplier).abs(),
                "deg",
                0.,
                2.,
                "Tangent across the mirrored centre/crown, or after joining mirrored shirt fronts.",
            );
        }
        for (id, a, bv) in [
            (
                "armhole_at_side",
                corner(doc, size, f, 1, 2)?,
                corner(doc, size, b, 1, 2)?,
            ),
            (
                "armhole_at_shoulder",
                corner(doc, size, f, 2, 3)?,
                match yoke_edges(&g, k)? {
                    Some((_, Some(arm))) => corner(doc, size, piece(&g, "yoke")?, arm, 0)?,
                    Some((join, None)) => {
                        corner(doc, size, b, 2, 3)?
                            + corner(doc, size, piece(&g, "yoke")?, join, 0)?
                    }
                    None => corner(doc, size, b, 2, 3)?,
                },
            ),
            (
                "neck_at_shoulder",
                corner(doc, size, f, 3, 4)?,
                corner(
                    doc,
                    size,
                    piece(
                        &g,
                        if k == BlockKind::Shirt {
                            "yoke"
                        } else {
                            "back"
                        },
                    )?,
                    if k == BlockKind::Shirt { 0 } else { 3 },
                    if k == BlockKind::Shirt { 1 } else { 4 },
                )?,
            ),
            (
                "hem_at_side",
                corner(doc, size, f, 0, 1)?,
                corner(doc, size, b, 0, 1)?,
            ),
            (
                "sleeve_cap_at_underarm",
                corner(doc, size, s, 1, 2)?,
                corner(doc, size, s, 3, 4)?,
            ),
            (
                "sleeve_hem_at_underarm",
                corner(doc, size, s, 4, 0)?,
                corner(doc, size, s, 0, 1)?,
            ),
        ] {
            add(
                &mut out,
                id,
                "tangent",
                (180. - a - bv).abs(),
                "deg",
                0.,
                2.,
                "Angular kink after opening the joined seam flat; 0 degrees is tangent continuous.",
            );
        }
        if let Some((join, arm)) = yoke_edges(&g, k)? {
            let y = piece(&g, "yoke")?;
            if let Some(arm) = arm {
                add(
                    &mut out,
                    "armhole_at_yoke",
                    "tangent",
                    (180. - corner(doc, size, b, 2, 3)? - corner(doc, size, y, join, arm)?).abs(),
                    "deg",
                    0.,
                    2.,
                    "Back armhole continues smoothly into the yoke armhole across the yoke seam.",
                );
            }
            add(
                &mut out,
                "yoke_depth_cb",
                "proportion",
                end(y, 1)?[1] - start(y, join)?[1],
                "mm",
                50.,
                90.,
                "Centre-back neck to yoke seam. BETA starting range for a casual shirt yoke.",
            );
        }
    } else {
        let skirt = k == BlockKind::ALineSkirt;
        seam("outseam", len(f, 1)? + len(f, 2)?, len(b, 1)? + len(b, 2)?);
        if !skirt {
            seam("inseam", len(f, 5)?, len(b, 5)?);
        }
        let wi = if skirt { 0 } else { 3 };
        let hi = if skirt { 3 } else { 0 };
        seam(
            "waistband",
            len(piece(&g, "waistband")?, 0)?,
            (len(f, wi)? + len(b, wi)?) * if skirt { 1. } else { 2. },
        );
        for (id, a, bv) in if skirt {
            vec![
                (
                    "waist_at_side",
                    corner(doc, size, f, 0, 1)?,
                    corner(doc, size, b, 0, 1)?,
                ),
                (
                    "hem_at_side",
                    corner(doc, size, f, 2, 3)?,
                    corner(doc, size, b, 2, 3)?,
                ),
                ("hip_curve_join", corner(doc, size, f, 1, 2)?, 0.),
            ]
        } else {
            vec![
                (
                    "hem_at_outseam",
                    corner(doc, size, f, 0, 1)?,
                    corner(doc, size, b, 0, 1)?,
                ),
                (
                    "hem_at_inseam",
                    corner(doc, size, f, 5, 0)?,
                    corner(doc, size, b, 5, 0)?,
                ),
                (
                    "crotch_at_inseam",
                    corner(doc, size, f, 4, 5)?,
                    corner(doc, size, b, 4, 5)?,
                ),
                (
                    "waist_at_front_crotch",
                    corner(doc, size, f, 3, 4)?,
                    corner(doc, size, f, 3, 4)?,
                ),
                (
                    "waist_at_back_crotch",
                    corner(doc, size, b, 3, 4)?,
                    corner(doc, size, b, 3, 4)?,
                ),
            ]
        } {
            add(
                &mut out,
                id,
                "tangent",
                (180. - a - bv).abs(),
                "deg",
                0.,
                2.,
                "Joined boundary tangent residual, after seam alignment.",
            );
        }
        let _ = hi;
        if !skirt {
            add(
                &mut out,
                "waist_at_side",
                "tangent",
                (180. - corner(doc, size, f, 2, 3)? - corner(doc, size, b, 2, 3)?).abs(),
                "deg",
                0.,
                2.,
                "Front and back waist lines meet the side seam without a kink.",
            );
            let ((ef, hf), (eb, hb)) = (extension(f)?, extension(b)?);
            let hip = 2. * (hf + hb);
            for (id, value, unit, lo, hi, note) in [
                (
                    "crotch_extension_total",
                    (ef + eb) / hip,
                    "ratio",
                    0.12,
                    0.18,
                    "Front plus back crotch extension beyond the hip line, as a fraction of garment hip. Zero extension cannot pass between the legs.",
                ),
                (
                    "crotch_extension_back_front",
                    eb / ef.max(1e-6),
                    "ratio",
                    1.6,
                    2.4,
                    "The back extension is longer than the front to cover the seat.",
                ),
                (
                    "back_front_rise",
                    len(b, 4)? / len(f, 4)?,
                    "ratio",
                    1.1,
                    1.5,
                    "Back crotch seam is longer than the front: raised and slanted centre back plus a longer extension.",
                ),
                (
                    "thigh_hip",
                    (width(f, start(f, 5)?[1] - 25.)? + width(b, start(b, 5)?[1] - 25.)?) / hip,
                    "ratio",
                    0.58,
                    0.72,
                    "Leg circumference 25 mm below the crotch over garment hip. Below about 0.58 the thigh is tight for a woven or loose knit leg.",
                ),
            ] {
                add(&mut out, id, "proportion", value, unit, lo, hi, note);
            }
            for (id, p) in [("front", f), ("back", b)] {
                let grain = p.piece.grain.context("Leg grain missing")?;
                let crotch = (start(p, 5)?[0] + start(p, 2)?[0]) / 2.;
                let hem = (start(p, 0)?[0] + end(p, 0)?[0]) / 2.;
                add(
                    &mut out,
                    format!("{id}.crease_offset"),
                    "grain",
                    (grain[0][0] - crotch).abs().max((grain[0][0] - hem).abs()),
                    "mm",
                    0.,
                    1.,
                    "The grain line is the crease: centred between crotch point and side at crotch level, and on the hem.",
                );
            }
        }
    }
    for (id, _, _, _, actual, target) in dimensions(doc, size, &g, k)? {
        if let Some((body, ease)) = target {
            add(
                &mut out,
                format!("pom_{id}"),
                "finished_measurement",
                actual - body - ease,
                "mm",
                -1.,
                1.,
                "Actual stitch-line POM minus independent body/design measurement and declared ease. Engineering tolerance 1 mm.",
            );
        }
    }
    for (source, p) in doc.pieces.iter().zip(&g) {
        add(
            &mut out,
            format!("{}.self_intersections", source.id),
            "geometry",
            f64::from(self_intersects(&p.seam)) + f64::from(self_intersects(&p.piece.outline)),
            "count",
            0.,
            0.,
            "Both stitch and allowance boundaries must be simple.",
        );
        let grain = p
            .piece
            .grain
            .context("Every block piece needs a grain line")?;
        let grain_target = if source.id == "waistband"
            || k == BlockKind::Shirt && matches!(source.id.as_str(), "collar" | "stand" | "cuff")
        {
            90.
        } else if source.id == "neck_facing" {
            45.
        } else {
            0.
        };
        add(
            &mut out,
            format!("{}.grain_length", source.id),
            "grain",
            distance(grain[0], grain[1]),
            "mm",
            10.,
            5000.,
            "Usable, nonzero grain reference within the piece.",
        );
        add(
            &mut out,
            format!("{}.grain_angle", source.id),
            "grain",
            (grain[1][0] - grain[0][0])
                .abs()
                .atan2((grain[1][1] - grain[0][1]).abs())
                .to_degrees(),
            "deg",
            grain_target - 0.1,
            grain_target + 0.1,
            "Main pieces: vertical. Woven collar/stand/cuff and waistbands: along joining length. Woven binding: bias. Knit rib/collar: stretch across joining length.",
        );
        let inside = inside;
        add(
            &mut out,
            format!("{}.grain_inside", source.id),
            "grain",
            if inside(grain[0], &p.seam) && inside(grain[1], &p.seam) {
                1.
            } else {
                0.
            },
            "bool",
            1.,
            1.,
            "Both grain endpoints lie inside the stitch outline.",
        );
        for (i, ed) in source.edges.iter().enumerate() {
            if source.grade_points.is_empty()
                && let Some(Object::CubicPath { segments }) = doc.drawing.get(&ed.object)
            {
                for (j, pair) in segments.windows(2).enumerate() {
                    let a = unit(sub(e.point(&pair[0][3])?, e.point(&pair[0][2])?))?;
                    let b = unit(sub(e.point(&pair[1][1])?, e.point(&pair[1][0])?))?;
                    add(
                        &mut out,
                        format!("{}.curve_{i}_join_{j}", source.id),
                        "tangent",
                        dot(a, b).clamp(-1., 1.).acos().to_degrees(),
                        "deg",
                        0.,
                        2.,
                        "Internal cubic segments share a tangent at the hip section.",
                    );
                }
            }
            let width = e.value(&ed.allowance)?;
            if width > 0. {
                add(
                    &mut out,
                    format!("{}.concave_radius_{i}", source.id),
                    "geometry",
                    (concave_radius(p, i)? / width).min(99.),
                    "ratio",
                    1.25,
                    99.,
                    "Smallest concave stitch radius over this edge's allowance. Below 1 the cut allowance folds over itself.",
                );
            }
            let hem = if upper(k) {
                (source.id == "front" || source.id == "back")
                    && (i == 0 || i == source.edges.len() - 1)
                    || source.id == "sleeve" && i == 0 && k != BlockKind::Shirt
            } else {
                (source.id == "front" || source.id == "back")
                    && i == if k == BlockKind::ALineSkirt { 3 } else { 0 }
            };
            // Half shirt front ends in a centre seam, not a hem.
            let hem = hem && !(source.id == "front" && k == BlockKind::Shirt && i == 5);
            let expected = v(
                &mut e,
                if hem {
                    "hem_allowance"
                } else {
                    "seam_allowance"
                },
            )?;
            add(
                &mut out,
                format!("{}.allowance_{i}", source.id),
                "allowance",
                e.value(&ed.allowance)? - expected,
                "mm",
                -0.01,
                0.01,
                "Per-edge width compared with this block's declared seam/hem allowance.",
            );
        }
        let mut notches = 0.;
        for a in &source.annotations {
            if let Annotation::Notch {
                edge: ei,
                distance: d,
                depth,
            } = a
            {
                notches += 1.;
                let d = e.value(d)?;
                let l = len(p, *ei)?;
                let depth = e.value(depth)?;
                add(
                    &mut out,
                    format!("{}.notch_{}_position", source.id, notches as usize),
                    "notch",
                    d / l,
                    "fraction",
                    0.,
                    1.,
                    "Distance along actual graded stitch edge, including reversed direction.",
                );
                add(
                    &mut out,
                    format!("{}.notch_{}_depth", source.id, notches as usize),
                    "notch",
                    depth,
                    "mm",
                    1.,
                    e.value(&source.edges[*ei].allowance)?.min(5.),
                    "Cut notch must stay within the seam allowance.",
                );
            }
        }
        if matches!(
            source.id.as_str(),
            "front"
                | "back"
                | "sleeve"
                | "waistband"
                | "neck_rib"
                | "collar"
                | "stand"
                | "cuff"
                | "neck_facing"
                | "yoke"
        ) {
            add(
                &mut out,
                format!("{}.assembly_notches", source.id),
                "notch",
                notches,
                "count",
                1.,
                100.,
                "At least one assembly reference on each joining piece; positions are checked separately.",
            );
        }
        if let Some(raw) = doc
            .meta
            .get("block_notches")
            .and_then(|r| r.get(&source.id))
        {
            let expected: Vec<Annotation> = serde_json::from_value(raw.clone())?;
            for (i, a) in expected.iter().enumerate() {
                if let Annotation::Notch {
                    edge: ei,
                    distance: d,
                    ..
                } = a
                {
                    let expected = e.value(d)?;
                    let mut nearest = f64::INFINITY;
                    for actual in &source.annotations {
                        #[allow(clippy::collapsible_if)] if let Annotation::Notch {
                            edge: ae,
                            distance: ad,
                            ..
                        } = actual
                        {
                            if ae == ei {
                                nearest = nearest.min((e.value(ad)? - expected).abs());
                            }
                        }
                    }
                    add(
                        &mut out,
                        format!("{}.assembly_reference_{i}", source.id),
                        "notch",
                        if nearest.is_finite() {
                            nearest
                        } else {
                            len(p, *ei)?
                        },
                        "mm",
                        0.,
                        1.,
                        "Required reference: side midpoint, front single/back double armhole thirds, crown, and component join centres/waist quarters. Extra marks are allowed.",
                    );
                }
            }
        }
    }
    Ok(out)
}

/// Smallest concave radius inside one stitch edge, sampled every 3 mm. A seam allowance
/// wider than this radius folds over itself when it is cut. Convex bends never fold.
fn concave_radius(g: &GradedPiece, i: usize) -> Result<f64> {
    let path = edge(g, i)?;
    let total = length(path);
    let sign = signed_area(&g.seam).signum();
    let steps = (total / 3.).floor() as usize;
    let at = |d: f64| Ok::<_, anyhow::Error>(at_distance(path, d)?.0);
    let mut best = f64::INFINITY;
    for k in 1..steps.saturating_sub(1) {
        let (a, b, c) = (
            at((k - 1) as f64 * 3.)?,
            at(k as f64 * 3.)?,
            at((k + 1) as f64 * 3.)?,
        );
        let turn = (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0]);
        if turn * sign >= 0. {
            continue;
        }
        // Circumradius of the three samples.
        best = best.min(distance(a, b) * distance(b, c) * distance(a, c) / (2. * turn.abs()));
    }
    Ok(best)
}
fn inside(p: Pt, poly: &[Pt]) -> bool {
    let mut yes = false;
    for (a, b) in poly
        .iter()
        .zip(poly.iter().cycle().skip(1))
        .take(poly.len())
    {
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            yes = !yes;
        }
    }
    yes
}
/// All sizes, including failures; errors do not masquerade as passing checks.
pub fn report(doc: &Document) -> Result<BlockReport> {
    let k = kind(doc)?;
    let mut sizes = vec![];
    let chart = chart_checks(&crate::style::SizeChart {
        name: "Style chart".into(),
        sizes: doc.sizes.clone(),
        base_size: doc.base_size.clone(),
        measurements: doc.measurements.clone(),
    })?;
    for size in &doc.sizes {
        match original_topology(doc).and_then(|()| size_checks(doc, size, k)) {
            Ok(mut checks) => {
                checks.extend(chart[size].clone());
                sizes.push(SizeChecks {
                    size: size.clone(),
                    pass: checks.iter().all(|c| c.pass),
                    checks,
                    errors: vec![],
                })
            }
            Err(e) => sizes.push(SizeChecks {
                size: size.clone(),
                pass: false,
                checks: vec![],
                errors: vec![format!("{e:#}")],
            }),
        }
    }
    Ok(BlockReport{block:k,sampling_mm:0.05,pass:!sizes.is_empty()&&sizes.iter().all(|s|s.pass),sizes,limitations:vec!["Geometric checks do not establish wearer fit, fabric stretch recovery, shrinkage or sewing-machine suitability.".into(),"Symmetric front/back armholes and simple collar, pocket, closure and crotch shaping require a pattern master's sample review.".into()]})
}

/// Original example-chart grade review, not population validation.
pub fn chart_checks(chart: &crate::style::SizeChart) -> Result<BTreeMap<String, Vec<CheckValue>>> {
    chart.validate()?;
    let mut out = BTreeMap::new();
    for (i, size) in chart.sizes.iter().enumerate() {
        let mut checks = vec![];
        if i > 0 {
            for (name, value) in &chart.measurements[size] {
                let step = value - chart.measurements[&chart.sizes[i - 1]][name];
                let max = if matches!(name.as_str(), "chest" | "waist" | "hip") {
                    80.
                } else {
                    25.
                };
                add(
                    &mut checks,
                    format!("grade_{name}"),
                    "chart",
                    step,
                    "mm",
                    0.,
                    max,
                    "Non-decreasing example grade; advisory ceiling, not a universal size standard.",
                );
            }
        }
        let r = &chart.measurements[size];
        if let (Some(l), Some(i)) = (r.get("trouser_length"), r.get("inseam")) {
            add(
                &mut checks,
                "rise",
                "chart",
                l - i,
                "mm",
                200.,
                360.,
                "Example adult construction rise, before waistband.",
            );
        }
        for (id, a, b, lo, hi) in [
            ("hip_waist", "hip", "waist", 0., 350.),
            ("neck_chest_ratio", "neck", "chest", 0.28, 0.48),
            ("bicep_chest_ratio", "bicep", "chest", 0.24, 0.42),
        ] {
            if let (Some(a), Some(b)) = (r.get(a), r.get(b)) {
                let ratio = id.ends_with("ratio");
                add(
                    &mut checks,
                    id,
                    "chart",
                    if ratio { a / b } else { a - b },
                    if ratio { "ratio" } else { "mm" },
                    lo,
                    hi,
                    "Advisory plausibility screen for these adult examples, not population evidence.",
                );
            }
        }
        out.insert(size.clone(), checks);
    }
    Ok(out)
}
