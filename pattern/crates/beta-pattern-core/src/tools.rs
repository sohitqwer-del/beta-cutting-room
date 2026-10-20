//! Pure pattern-tool operations. Inputs remain unchanged when an operation fails.
use crate::{geometry::*, model::*};
use anyhow::{Context, Result, ensure};
use beta_marker_core::{MarkKind, Pt};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Compare two open seams, in millimetres. Positive means the first seam is longer.
pub fn walk(a: &[Pt], b: &[Pt]) -> Result<f64> {
    ensure!(
        a.len() >= 2 && b.len() >= 2 && a.iter().chain(b).flatten().all(|v| v.is_finite()),
        "two finite seams required"
    );
    Ok(length(a) - length(b))
}
/// True a seam to a target length by scaling it uniformly about its start.
pub fn true_seam(seam: &[Pt], target: f64) -> Result<Vec<Pt>> {
    let n = length(seam);
    ensure!(
        n.is_finite() && n > 1e-9 && target.is_finite() && target > 0.,
        "positive seam lengths required"
    );
    Ok(seam
        .iter()
        .map(|q| add(seam[0], mul(sub(*q, seam[0]), target / n)))
        .collect())
}
/// Position a notch and its inward endpoint on a directed seam (CCW outline by default).
pub fn notch(seam: &[Pt], distance_mm: f64, depth: f64, ccw: bool) -> Result<[Pt; 2]> {
    ensure!(
        depth.is_finite() && depth > 0.,
        "positive notch depth required"
    );
    let (p, t) = at_distance(seam, distance_mm)?;
    Ok([
        p,
        add(p, mul([-t[1], t[0]], if ccw { depth } else { -depth })),
    ])
}
/// Reflect points about an arbitrary infinite line.
pub fn mirror(points: &[Pt], a: Pt, b: Pt) -> Result<Vec<Pt>> {
    let u = unit(sub(b, a))?;
    ensure!(
        points.iter().flatten().all(|v| v.is_finite()),
        "non-finite point"
    );
    Ok(points
        .iter()
        .map(|p| {
            let v = sub(*p, a);
            add(a, sub(mul(u, 2. * dot(v, u)), v))
        })
        .collect())
}
/// Split a simple piece along an infinite line. Only a single connected cut is accepted.
pub fn split(poly: &[Pt], a: Pt, b: Pt) -> Result<[Vec<Pt>; 2]> {
    validate_polygon(poly)?;
    unit(sub(b, a))?;
    let closed: Vec<_> = poly.iter().copied().chain(poly.first().copied()).collect();
    let bounds = bbox(poly).context("empty polygon")?;
    let extent = (bounds[2] - bounds[0]).hypot(bounds[3] - bounds[1]) * 4. + distance(a, poly[0]);
    let u = unit(sub(b, a))?;
    let cuts = intersections(&closed, &[sub(a, mul(u, extent)), add(a, mul(u, extent))]);
    ensure!(
        cuts.len() == 2,
        "split requires exactly two boundary intersections"
    );
    let compact = |mut p: Vec<Pt>| {
        p.dedup_by(|a, b| distance(*a, *b) < 1e-9);
        if p.len() > 1 && distance(p[0], p[p.len() - 1]) < 1e-9 {
            p.pop();
        }
        p
    };
    let left = compact(beta_marker_core::geometry::clip_left_of_line(poly, a, b));
    let right = compact(beta_marker_core::geometry::clip_left_of_line(poly, b, a));
    validate_polygon(&left)?;
    validate_polygon(&right)?;
    Ok([left, right])
}
fn rotate(p: Pt, pivot: Pt, angle: f64) -> Pt {
    let (s, c) = angle.to_radians().sin_cos();
    let v = sub(p, pivot);
    add(pivot, [c * v[0] - s * v[1], s * v[0] + c * v[1]])
}
fn cut_transform(poly: &[Pt], a: Pt, b: Pt, transform: impl Fn(Pt) -> Pt) -> Result<Vec<Pt>> {
    split(poly, a, b)?;
    let side = |p| cross(sub(b, a), sub(p, a));
    let mut out = vec![];
    for i in 0..poly.len() {
        let p = poly[i];
        let q = poly[(i + 1) % poly.len()];
        let (sp, sq) = (side(p), side(q));
        out.push(if sp < -1e-9 { transform(p) } else { p });
        if (sp > 1e-9 && sq < -1e-9) || (sp < -1e-9 && sq > 1e-9) {
            let t = sp / (sp - sq);
            let cut = add(p, mul(sub(q, p), t));
            if sp > 0. {
                out.extend([cut, transform(cut)]);
            } else {
                out.extend([transform(cut), cut]);
            }
        } else if sp.abs() <= 1e-9 && sq < -1e-9 {
            out.push(transform(p));
        } else if sp < -1e-9 && sq.abs() <= 1e-9 {
            out.push(transform(q));
        }
    }
    out.dedup_by(|p, q| distance(*p, *q) < 1e-9);
    if out.len() > 1 && distance(out[0], out[out.len() - 1]) < 1e-9 {
        out.pop();
    }
    validate_polygon(&out)?;
    Ok(out)
}
/// Slash from a boundary pivot toward another boundary and rotate the right panel.
/// Positive angles rotate the right panel clockwise. A straight bridge fills the opening.
pub fn slash_spread(poly: &[Pt], pivot: Pt, toward: Pt, angle: f64) -> Result<Vec<Pt>> {
    ensure!(
        angle.is_finite() && angle > 0. && angle < 180.,
        "spread angle must lie between 0 and 180 degrees"
    );
    ensure!(
        poly.iter().enumerate().any(|(i, p)| {
            let q = poly[(i + 1) % poly.len()];
            (distance(*p, pivot) + distance(pivot, q) - distance(*p, q)).abs() < 1e-7
        }),
        "slash pivot must lie on boundary"
    );
    cut_transform(poly, pivot, toward, |q| rotate(q, pivot, -angle))
}
/// Pleat type. Depth is folded depth; knife adds 2*depth, box adds 4*depth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PleatKind {
    /// One fold direction.
    Knife,
    /// Opposed folds.
    Box,
}
/// Pleat cutting geometry and fold guide segments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pleat {
    /// Expanded outline.
    pub outline: Vec<Pt>,
    /// Fold guides on the inserted strip.
    pub folds: Vec<[Pt; 2]>,
}
/// Add a straight knife or box pleat by translating the right panel perpendicular to the cut.
pub fn pleat(poly: &[Pt], a: Pt, b: Pt, depth: f64, kind: PleatKind) -> Result<Pleat> {
    ensure!(
        depth.is_finite() && depth > 0.,
        "pleat depth must be positive"
    );
    let u = unit(sub(b, a))?;
    let normal = [u[1], -u[0]];
    let count = if kind == PleatKind::Knife { 2 } else { 4 };
    let width = depth * count as f64;
    let outline = cut_transform(poly, a, b, |q| add(q, mul(normal, width)))?;
    let closed: Vec<_> = poly.iter().copied().chain(poly.first().copied()).collect();
    let bb = bbox(poly).context("empty polygon")?;
    let extent = (bb[2] - bb[0]).hypot(bb[3] - bb[1]) * 4. + distance(a, poly[0]);
    let cuts = intersections(&closed, &[sub(a, mul(u, extent)), add(a, mul(u, extent))]);
    let folds = (0..=count)
        .map(|i| {
            [
                add(cuts[0], mul(normal, depth * i as f64)),
                add(cuts[1], mul(normal, depth * i as f64)),
            ]
        })
        .collect();
    Ok(Pleat { outline, folds })
}
/// A dart is a triangular intake between two seam legs meeting at an apex.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dart {
    /// Original outline before intake.
    pub original: Vec<Pt>,
    /// Outline containing the dart legs.
    pub outline: Vec<Pt>,
    /// Index of the original edge.
    pub edge: usize,
    /// Interior dart tip.
    pub apex: Pt,
    /// Mouth endpoints in traversal order.
    pub legs: [Pt; 2],
}
/// Create a dart centred at distance along an edge, with intake width measured on that edge.
pub fn dart(poly: &[Pt], edge: usize, center: f64, width: f64, apex: Pt) -> Result<Dart> {
    validate_polygon(poly)?;
    ensure!(edge < poly.len(), "dart edge index out of range");
    let (a, b) = (poly[edge], poly[(edge + 1) % poly.len()]);
    let n = distance(a, b);
    ensure!(
        width.is_finite() && width > 0. && center > width / 2. && center + width / 2. < n,
        "dart mouth must lie inside edge"
    );
    let u = unit(sub(b, a))?;
    let legs = [
        add(a, mul(u, center - width / 2.)),
        add(a, mul(u, center + width / 2.)),
    ];
    dart_legs(poly, edge, apex, legs)
}
fn dart_legs(poly: &[Pt], edge: usize, apex: Pt, legs: [Pt; 2]) -> Result<Dart> {
    let mut out = vec![];
    for (i, p) in poly.iter().enumerate() {
        out.push(*p);
        if i == edge {
            out.extend([legs[0], apex, legs[1]]);
        }
    }
    validate_polygon(&out)?;
    ensure!(area(&out) < area(poly), "dart apex must lie inside piece");
    Ok(Dart {
        original: poly.into(),
        outline: out,
        edge,
        apex,
        legs,
    })
}
/// Remove an open dart and restore the uncut outline (an editing operation, not folding simulation).
pub fn remove_dart(d: &Dart) -> Vec<Pt> {
    d.original.clone()
}
/// Close the old dart and open one at a point on another edge.
/// Rotates one panel about the apex, preserving fabric area and opening angle.
pub fn transfer_dart(d: &Dart, edge: usize, center: f64) -> Result<Dart> {
    ensure!(d.edge < d.original.len(), "source dart edge out of range");
    ensure!(edge < d.original.len(), "target dart edge out of range");
    ensure!(edge != d.edge, "transfer target must be another edge");
    let (a, b) = (d.original[edge], d.original[(edge + 1) % d.original.len()]);
    let u = unit(sub(b, a))?;
    ensure!(
        center > 0. && center < distance(a, b),
        "dart centre outside target edge"
    );
    let mut cut = d.clone();
    cut.original.insert(edge + 1, add(a, mul(u, center)));
    if edge < d.edge {
        cut.edge += 1;
    }
    let outline = close_dart(&cut, edge + 1)?;
    let apex_index = outline
        .iter()
        .position(|p| distance(*p, d.apex) < 1e-9)
        .context("transferred dart apex missing")?;
    ensure!(
        apex_index > 0 && apex_index + 1 < outline.len(),
        "transferred dart mouth is degenerate"
    );
    let legs = [outline[apex_index - 1], outline[apex_index + 1]];
    let mut original = outline.clone();
    original.remove(apex_index);
    validate_polygon(&original)?;
    Ok(Dart {
        original,
        outline,
        edge: apex_index - 1,
        apex: d.apex,
        legs,
    })
}
/// Close a dart by rotating the boundary chain from its second leg through `cut_vertex`.
/// The cut vertex opens a new slash to the apex, so the outline remains a flat pattern.
pub fn close_dart(d: &Dart, cut_vertex: usize) -> Result<Vec<Pt>> {
    ensure!(d.edge < d.original.len(), "dart edge out of range");
    validate_polygon(&d.original)?;
    ensure!(
        cut_vertex < d.original.len()
            && cut_vertex != d.edge
            && cut_vertex != (d.edge + 1) % d.original.len(),
        "closure cut must be away from dart edge"
    );
    ensure!(
        (distance(d.legs[0], d.apex) - distance(d.legs[1], d.apex)).abs() < 1e-6,
        "dart closure requires equal leg lengths"
    );
    let v0 = sub(d.legs[0], d.apex);
    let v1 = sub(d.legs[1], d.apex);
    let angle = cross(v1, v0).atan2(dot(v1, v0)).to_degrees();
    let mut out = vec![d.legs[0]];
    let mut i = (d.edge + 1) % d.original.len();
    loop {
        out.push(rotate(d.original[i], d.apex, angle));
        if i == cut_vertex {
            break;
        }
        i = (i + 1) % d.original.len();
    }
    out.extend([d.apex, d.original[cut_vertex]]);
    i = (cut_vertex + 1) % d.original.len();
    loop {
        out.push(d.original[i]);
        if i == d.edge {
            break;
        }
        i = (i + 1) % d.original.len();
    }
    out.dedup_by(|a, b| distance(*a, *b) < 1e-9);
    if out.len() > 1 && distance(out[0], out[out.len() - 1]) < 1e-9 {
        out.pop();
    }
    validate_polygon(&out)?;
    Ok(out)
}
/// Set or remove every edge allowance on a piece, returning a new document.
pub fn set_allowance(
    doc: &Document,
    piece_id: &str,
    width: Value,
    corner: Corner,
) -> Result<Document> {
    let mut out = doc.clone();
    let p = out
        .pieces
        .iter_mut()
        .find(|p| p.id == piece_id)
        .context("unknown piece for allowance edit")?;
    for e in &mut p.edges {
        e.allowance = width.clone();
        e.corner = corner;
    }
    Ok(out)
}
/// Insert a fixed polygon as drawing points/lines plus a piece. Rejects ID collisions atomically.
pub fn insert_polygon(doc: &mut Document, id: &str, name: &str, poly: &[Pt]) -> Result<()> {
    validate_polygon(poly)?;
    ensure!(
        !id.is_empty() && !doc.pieces.iter().any(|p| p.id == id),
        "piece ID empty or already used"
    );
    let mut drawing = BTreeMap::new();
    let mut edges = vec![];
    for (i, p) in poly.iter().enumerate() {
        let point = format!("{id}.p{i}");
        let line = format!("{id}.e{i}");
        drawing.insert(
            point,
            Object::Point {
                operation: PointOp::Absolute {
                    x: p[0].into(),
                    y: p[1].into(),
                },
            },
        );
        drawing.insert(
            line.clone(),
            Object::Line {
                from: format!("{id}.p{i}"),
                to: format!("{id}.p{}", (i + 1) % poly.len()),
            },
        );
        edges.push(Edge {
            object: line,
            reversed: false,
            allowance: 0.0.into(),
            corner: Corner::Miter,
        });
    }
    ensure!(
        drawing.keys().all(|k| !doc.drawing.contains_key(k)),
        "drawing ID collision"
    );
    doc.drawing.extend(drawing);
    doc.pieces.push(PatternPiece {
        internal_paths: vec![],
        anchors: BTreeMap::new(),
        grain_angle: None,
        label_point: None,
        id: id.into(),
        name: name.into(),
        edges,
        quantity: 1,
        mirror_quantity: 0,
        material: None,
        grain: None,
        fold_line: None,
        rotation: Default::default(),
        buffer: 0.0.into(),
        annotations: vec![],
        grade_points: BTreeMap::new(),
    });
    Ok(())
}
/// Add pleat fold guides to a fixed piece inserted with [`insert_polygon`].
pub fn add_fold_guides(doc: &mut Document, piece_id: &str, folds: &[[Pt; 2]]) -> Result<()> {
    let mut out = doc.clone();
    let p = out
        .pieces
        .iter_mut()
        .find(|p| p.id == piece_id)
        .context("unknown pleat piece")?;
    for (i, line) in folds.iter().enumerate() {
        let ids = [
            format!("{piece_id}.fold{i}a"),
            format!("{piece_id}.fold{i}b"),
        ];
        for (id, q) in ids.iter().zip(line) {
            ensure!(!out.drawing.contains_key(id), "fold ID collision");
            out.drawing.insert(
                id.clone(),
                Object::Point {
                    operation: PointOp::Absolute {
                        x: q[0].into(),
                        y: q[1].into(),
                    },
                },
            );
        }
        p.annotations.push(Annotation::Points {
            mark: MarkKind::Line,
            points: ids.into(),
            text: Some("pleat fold".into()),
        });
    }
    *doc = out;
    Ok(())
}

// The generic editor contract lives separately from polygon editing tools.
pub use crate::catalog::{ParameterType, ToolParameter, ToolSpec, catalog, construct};

/// Generic construction and dependency-safe editor operations.
pub use crate::editor::{apply, delete, dependents, move_point, reference_order, rename};

/// Apply a catalog transformation to each selected object and create a visible group.
/// Copies retain parametric references to source geometry; ID collisions fail atomically.
pub fn transform_group(
    doc: &Document,
    selection: &[String],
    suffix: &str,
    tool_id: &str,
    params: serde_json::Map<String, serde_json::Value>,
) -> Result<Document> {
    ensure!(
        matches!(tool_id, "move" | "rotate" | "mirror_line" | "mirror_axis"),
        "not a group transformation"
    );
    ensure!(
        !selection.is_empty() && !suffix.is_empty(),
        "selection and suffix required"
    );
    ensure!(
        selection
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            == selection.len(),
        "duplicate selection ID"
    );
    let mut out = doc.clone();
    let mut objects = vec![];
    for source in selection {
        ensure!(
            doc.drawing.contains_key(source),
            "unknown selected object '{source}'"
        );
        let id = format!("{source}{suffix}");
        let mut fields = params.clone();
        fields.insert("source".into(), serde_json::json!(source));
        out = apply(&out, tool_id, &id, fields)?;
        objects.push(id);
    }
    ensure!(!out.groups.contains_key(suffix), "group ID collision");
    out.groups.insert(
        suffix.into(),
        DrawingGroup {
            name: suffix.into(),
            objects,
            visible: true,
        },
    );
    Ok(out)
}
/// Change display visibility without changing evaluation or exports.
pub fn set_group_visibility(doc: &Document, id: &str, visible: bool) -> Result<Document> {
    let mut out = doc.clone();
    out.groups
        .get_mut(id)
        .context("unknown drawing group")?
        .visible = visible;
    Ok(out)
}
/// Union two simple polygons that share complete, oppositely directed boundary segments.
/// Rejects overlaps, disconnected regions, holes and partial-edge joins.
pub fn union_shared_seam(a: &[Pt], b: &[Pt]) -> Result<Vec<Pt>> {
    validate_polygon(a)?;
    validate_polygon(b)?;
    let mut b = b.to_vec();
    if signed_area(a).signum() != signed_area(&b).signum() {
        b.reverse();
    }
    let mut edges: Vec<(Pt, Pt)> = a
        .iter()
        .enumerate()
        .map(|(i, p)| (*p, a[(i + 1) % a.len()]))
        .collect();
    let mut shared = 0;
    for (i, p) in b.iter().enumerate() {
        let q = b[(i + 1) % b.len()];
        if let Some(j) = edges
            .iter()
            .position(|(x, y)| distance(*x, q) < 1e-6 && distance(*y, *p) < 1e-6)
        {
            edges.remove(j);
            shared += 1;
        } else {
            edges.push((*p, q));
        }
    }
    ensure!(
        shared > 0,
        "pieces do not share an oppositely directed seam"
    );
    ensure!(!edges.is_empty(), "union has no exterior");
    let (start, mut end) = edges.remove(0);
    let mut out = vec![start];
    while distance(end, start) > 1e-6 {
        out.push(end);
        let matches: Vec<_> = edges
            .iter()
            .enumerate()
            .filter(|(_, e)| distance(e.0, end) < 1e-6)
            .map(|(i, _)| i)
            .collect();
        ensure!(
            matches.len() == 1,
            "union boundary is disconnected or branches"
        );
        let edge = edges.remove(matches[0]);
        end = edge.1;
    }
    ensure!(
        edges.is_empty(),
        "union contains a hole or disconnected boundary"
    );
    validate_polygon(&out)?;
    ensure!(
        (area(&out) - area(a) - area(&b)).abs() < 1e-5 * (1. + area(&out)),
        "pieces overlap away from shared seam"
    );
    Ok(out)
}
/// Join two pieces in one evaluated size into a fixed stitch outline.
/// Exterior allowances follow source edge widths. Marks become fixed geometry;
/// notches on the removed shared seam are dropped. Source pieces remain unchanged.
pub fn union_pieces(
    doc: &Document,
    a: &str,
    b: &str,
    id: &str,
    size: &str,
    tolerance: f64,
) -> Result<Document> {
    ensure!(a != b, "union needs different pieces");
    let pa = doc
        .pieces
        .iter()
        .find(|p| p.id == a)
        .context("first piece missing")?;
    let pb = doc
        .pieces
        .iter()
        .find(|p| p.id == b)
        .context("second piece missing")?;
    ensure!(pa.material == pb.material, "piece materials differ");
    let graded = crate::evaluate::grade(doc, size, tolerance)?;
    let ga = &graded[doc
        .pieces
        .iter()
        .position(|p| p.id == a)
        .context("first piece missing")?];
    let gb = &graded[doc
        .pieces
        .iter()
        .position(|p| p.id == b)
        .context("second piece missing")?];
    let poly = union_shared_seam(&ga.seam, &gb.seam)?;
    let mut out = doc.clone();
    insert_polygon(&mut out, id, &format!("{} + {}", pa.name, pb.name), &poly)?;
    let mut e = crate::evaluate::Evaluator::new(doc, size, tolerance)?;
    let mut sources = vec![];
    for p in [pa, pb] {
        for edge in &p.edges {
            let path = e.path(&edge.object)?;
            for w in path.windows(2) {
                sources.push((w[0], w[1], e.value(&edge.allowance)?));
            }
        }
    }
    let merged = out.pieces.last_mut().context("union piece missing")?;
    for (i, edge) in merged.edges.iter_mut().enumerate() {
        let mid = mul(add(poly[i], poly[(i + 1) % poly.len()]), 0.5);
        if let Some((_, _, width)) = sources.iter().find(|(a, b, _)| {
            (distance(*a, mid) + distance(mid, *b) - distance(*a, *b)).abs() < 1e-6
        }) {
            edge.allowance = (*width).into();
        }
    }
    merged.material = pa.material.clone();
    merged.quantity = pa.quantity;
    merged.mirror_quantity = pa.mirror_quantity;
    merged.internal_paths = pa
        .internal_paths
        .iter()
        .chain(&pb.internal_paths)
        .cloned()
        .collect();
    merged.grain = pa.grain.clone();
    merged.grain_angle = pa.grain_angle.clone();
    merged.anchors = pa
        .anchors
        .iter()
        .map(|(k, v)| (format!("{a}.{k}"), v.clone()))
        .chain(
            pb.anchors
                .iter()
                .map(|(k, v)| (format!("{b}.{k}"), v.clone())),
        )
        .collect();
    // Preserve evaluated marks as fixed drawing references. Notches on the
    // removed shared seam are dropped; exterior notches and internal guides stay.
    let mut annotations = vec![];
    for (i, mark) in ga.piece.marks.iter().chain(&gb.piece.marks).enumerate() {
        if mark.kind == MarkKind::Notch
            && let Some(q) = mark.points.first()
        {
            let on_boundary = poly.iter().enumerate().any(|(j, a)| {
                let b = poly[(j + 1) % poly.len()];
                (distance(*a, *q) + distance(*q, b) - distance(*a, b)).abs() < 1e-6
            });
            if !on_boundary {
                continue;
            }
        }
        let mut ids = vec![];
        for (j, q) in mark.points.iter().enumerate() {
            let point = format!("{id}.mark{i}.{j}");
            ensure!(!out.drawing.contains_key(&point), "union mark ID collision");
            out.drawing.insert(
                point.clone(),
                Object::Point {
                    operation: PointOp::Absolute {
                        x: q[0].into(),
                        y: q[1].into(),
                    },
                },
            );
            ids.push(point);
        }
        annotations.push(Annotation::Points {
            mark: mark.kind,
            points: ids,
            text: mark.text.clone(),
        });
    }
    // Internal path geometry already exists in evaluated marks; copying both
    // representations would print each guide twice.
    let merged = out.pieces.last_mut().context("union piece missing")?;
    merged.internal_paths.clear();
    merged.annotations = annotations;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    const S: [Pt; 4] = [[0., 0.], [100., 0.], [100., 100.], [0., 100.]];
    #[test]
    fn split_mirror_walk_notch() {
        let p = split(&S, [50., 0.], [50., 100.]).unwrap();
        assert_eq!(area(&p[0]), 5000.);
        assert_eq!(area(&p[1]), 5000.);
        let diagonal = split(&S, [0., 0.], [100., 100.]).unwrap();
        assert_eq!(area(&diagonal[0]), 5000.);
        assert_eq!(area(&diagonal[1]), 5000.);
        let m = mirror(&S, [0., 0.], [0., 1.]).unwrap();
        assert_eq!(m[1], [-100., 0.]);
        assert_eq!(area(&m), 10000.);
        assert_eq!(
            walk(&[[0., 0.], [3., 4.]], &[[0., 0.], [3., 0.]]).unwrap(),
            2.
        );
        assert_eq!(length(&true_seam(&[[0., 0.], [3., 4.]], 10.).unwrap()), 10.);
        assert_eq!(
            notch(&S[..2], 20., 3., true).unwrap(),
            [[20., 0.], [20., 3.]]
        );
    }
    #[test]
    fn fullness_and_pleats() {
        for (k, expected) in [(PleatKind::Knife, 12000.), (PleatKind::Box, 14000.)] {
            let p = pleat(&S, [50., 0.], [50., 100.], 10., k).unwrap();
            assert_eq!(area(&p.outline), expected);
        }
        let p = slash_spread(&S, [50., 0.], [50., 100.], 30.).unwrap();
        assert!((area(&p) - 12500.).abs() < 1e-7, "{}", area(&p));
    }
    #[test]
    fn dart_transfer_and_close() {
        let d = dart(&S, 0, 50., 20., [50., 50.]).unwrap();
        assert_eq!(area(&d.outline), 9500.);
        assert_eq!(remove_dart(&d), S);
        let t = transfer_dart(&d, 2, 50.).unwrap();
        assert!((area(&t.outline) - 9500.).abs() < 1e-8);
        let side = transfer_dart(&d, 1, 25.).unwrap();
        assert!((area(&side.outline) - 9500.).abs() < 1e-8);
        assert!(
            (distance(side.legs[0], side.apex) - distance(side.legs[1], side.apex)).abs() < 1e-8
        );
        let closed = close_dart(&d, 2).unwrap();
        assert!((area(&closed) - area(&d.outline)).abs() < 1e-8);
        let a = sub(d.legs[0], d.apex);
        let b = sub(d.legs[1], d.apex);
        let c = sub(t.legs[0], t.apex);
        let e = sub(t.legs[1], t.apex);
        assert!(
            (dot(unit(a).unwrap(), unit(b).unwrap()) - dot(unit(c).unwrap(), unit(e).unwrap()))
                .abs()
                < 1e-9
        );
    }
}
