//! Parametric piece reflection and half-pattern unfolding.
use crate::{
    evaluate::{self, Evaluator},
    geometry::*,
    *,
};
use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;

fn on_axis(doc: &Document, refs: &[&str], axis: &[String; 2]) -> Result<bool> {
    if refs.is_empty() {
        return Ok(false);
    }
    for size in &doc.sizes {
        let mut e = Evaluator::new(doc, size, 0.05)?;
        let a = e.point(&axis[0])?;
        let b = e.point(&axis[1])?;
        let u = unit(sub(b, a))?;
        for name in refs {
            let points = match e.object(name)? {
                evaluate::Geometry::Point(p) => vec![p],
                evaluate::Geometry::Path(p) => p,
            };
            if points.iter().any(|p| cross(u, sub(*p, a)).abs() > 1e-6) {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

fn reflect_ref(
    out: &mut Document,
    prefix: &str,
    source: &str,
    axis: &[String; 2],
    cache: &mut BTreeMap<String, String>,
) -> Result<String> {
    if let Some(id) = cache.get(source) {
        return Ok(id.clone());
    }
    let id = format!("{prefix}.mirror.{source}");
    ensure!(
        !out.drawing.contains_key(&id),
        "reflection ID collision: {id}"
    );
    ensure!(
        out.drawing.contains_key(source),
        "missing reflection source: {source}"
    );
    out.drawing.insert(
        id.clone(),
        Object::MirrorLine {
            source: source.into(),
            a: axis[0].clone(),
            b: axis[1].clone(),
        },
    );
    cache.insert(source.into(), id.clone());
    Ok(id)
}

/// Reflect a complete piece while retaining formulas, marks, grain, allowances,
/// quantities and factory rules. Source geometry remains linked and unchanged.
/// For factory grading the reflection axis is evaluated in the base size.
pub fn mirror_piece(doc: &Document, source: &str, id: &str, axis: [String; 2]) -> Result<Document> {
    ensure!(editor::valid_id(id), "use a valid piece ID");
    ensure!(
        !doc.pieces.iter().any(|p| p.id == id),
        "piece ID already exists"
    );
    let source_piece = doc
        .pieces
        .iter()
        .find(|p| p.id == source)
        .context("unknown source piece")?;
    let mut out = doc.clone();
    let mut p = source_piece.clone();
    p.id = id.into();
    p.name = format!("{} mirrored", p.name);
    let mut cache = BTreeMap::new();
    // An angled grain is turned into a parametric endpoint before reflection.
    if let Some(g) = p.grain_angle.take().filter(|_| p.grain.is_none()) {
        let end = format!("{id}.grain_end");
        ensure!(!out.drawing.contains_key(&end), "grain point ID collision");
        out.drawing.insert(
            end.clone(),
            Object::Point {
                operation: PointOp::Polar {
                    from: g.origin.clone(),
                    angle: g.angle,
                    distance: g.length,
                },
            },
        );
        p.grain = Some([g.origin, end]);
    }
    let mut reflect = |s: &str| reflect_ref(&mut out, id, s, &axis, &mut cache);
    for edge in p
        .edges
        .iter_mut()
        .chain(p.internal_paths.iter_mut().flat_map(|p| &mut p.edges))
    {
        edge.object = reflect(&edge.object)?;
    }
    for name in p
        .grain
        .iter_mut()
        .flatten()
        .chain(p.fold_line.iter_mut().flatten())
        .chain(p.anchors.values_mut())
        .chain(p.label_point.iter_mut())
    {
        *name = reflect(name)?;
    }
    for annotation in &mut p.annotations {
        match annotation {
            Annotation::Points { points, .. } => {
                for name in points {
                    *name = reflect(name)?;
                }
            }
            Annotation::Button { point, .. } => *point = reflect(point)?,
            Annotation::SeamIntersections { path, .. } => *path = reflect(path)?,
            Annotation::Notch { .. } | Annotation::ShapedNotch { .. } => {}
        }
    }
    let mut attachments = BTreeMap::new();
    let mut e = Evaluator::new(doc, &doc.base_size, 0.1)?;
    let a = e.point(&axis[0])?;
    let b = e.point(&axis[1])?;
    let u = unit(sub(b, a))?;
    // Reflect displacement vectors (not coordinates) about the base axis.
    for (point, rule) in &source_piece.grade_points {
        let point = reflect_ref(&mut out, id, point, &axis, &mut cache)?;
        let rule_id = format!("{id}.rule.{rule}");
        if !attachments.values().any(|r| r == &rule_id) {
            ensure!(
                !out.rules.contains_key(&rule_id),
                "reflected rule ID collision"
            );
            let rule = doc.rules.get(rule).context("missing grade rule")?;
            let deltas = doc
                .sizes
                .iter()
                .map(|s| {
                    let d = grading::rule_delta(doc, rule, s)?;
                    Ok((s.clone(), sub(mul(u, 2. * dot(d, u)), d)))
                })
                .collect::<Result<_>>()?;
            out.rules.insert(
                rule_id.clone(),
                GradeRule {
                    mode: GradeMode::Relative,
                    deltas,
                },
            );
        }
        attachments.insert(point, rule_id);
    }
    p.grade_points = attachments;
    out.pieces.push(p);
    for s in &out.sizes {
        evaluate::grade(&out, s, 0.1)?;
    }
    Ok(out)
}

/// Unfold one straight, zero-allowance edge into a full symmetric piece.
/// The two halves remain linked to source measurements and constructions.
/// Factory-rule half patterns need conversion to measurement grading first:
/// reflecting their interpolated internal deformation is not yet supported.
pub fn unfold_piece(doc: &Document, source: &str, id: &str, fold_edge: usize) -> Result<Document> {
    let p = doc
        .pieces
        .iter()
        .find(|p| p.id == source)
        .context("unknown source piece")?;
    ensure!(
        p.grade_points.is_empty(),
        "unfold currently requires measurement grading; factory-rule half patterns are not supported"
    );
    ensure!(fold_edge < p.edges.len(), "fold edge index out of range");
    ensure!(p.edges.len() >= 3, "unfold needs at least three edges");
    let mut out = doc.clone();
    let axis = [format!("{id}.fold_start"), format!("{id}.fold_end")];
    for (i, name) in axis.iter().enumerate() {
        ensure!(!out.drawing.contains_key(name), "fold point ID collision");
        out.drawing.insert(
            name.clone(),
            Object::Point {
                operation: PointOp::PathEndpoint {
                    curve: p.edges[fold_edge].object.clone(),
                    index: i,
                },
            },
        );
    }
    for size in &doc.sizes {
        let mut e = Evaluator::new(&out, size, 0.05)?;
        ensure!(
            e.value(&p.edges[fold_edge].allowance)?.abs() < 1e-9,
            "set fold edge allowance to zero before unfolding"
        );
        let a = e.point(&axis[0])?;
        let b = e.point(&axis[1])?;
        let u = unit(sub(b, a))?;
        let graded = evaluate::grade(doc, size, 0.05)?;
        let g = &graded[doc
            .pieces
            .iter()
            .position(|q| q.id == source)
            .context("source missing")?];
        ensure!(
            g.seam_edges[fold_edge]
                .iter()
                .all(|q| cross(u, sub(*q, a)).abs() < 1e-6),
            "fold edge must be straight in every size"
        );
        let sides: Vec<_> = g.seam.iter().map(|q| cross(u, sub(*q, a))).collect();
        ensure!(
            sides.iter().all(|d| *d >= -1e-6) || sides.iter().all(|d| *d <= 1e-6),
            "half pattern crosses its fold axis"
        );
    }
    out = mirror_piece(&out, source, id, axis.clone())?;
    let reflected = out.pieces.pop().context("reflected piece missing")?;
    let n = p.edges.len();
    let order: Vec<_> = (1..n).map(|i| (fold_edge + i) % n).collect();
    let mut full = p.clone();
    full.id = id.into();
    full.name = format!("{} unfolded", p.name);
    full.fold_line = Some(axis.clone());
    full.edges = order.iter().map(|i| p.edges[*i].clone()).collect();
    for i in order.iter().rev() {
        let mut edge = reflected.edges[*i].clone();
        edge.reversed = !edge.reversed;
        edge.corner = p.edges[(*i + 1) % n].corner;
        full.edges.push(edge);
    }
    let remap = |annotations: &[Annotation], mirrored: bool| -> Result<Vec<Annotation>> {
        let mut result = vec![];
        for a in annotations {
            let mut a = a.clone();
            match &mut a {
                Annotation::SeamIntersections { edges, .. } => {
                    // A guide may extend across both halves. Restrict each
                    // copy to its source half to avoid duplicate/extra cuts.
                    let selected = edges.clone().unwrap_or_else(|| (0..n).collect());
                    ensure!(
                        selected.iter().all(|i| *i < n),
                        "invalid crossing-notch edge"
                    );
                    *edges = Some(
                        selected
                            .into_iter()
                            .filter(|i| *i != fold_edge)
                            .map(|original| {
                                let pos = (original + n - fold_edge - 1) % n;
                                if mirrored {
                                    2 * order.len() - 1 - pos
                                } else {
                                    pos
                                }
                            })
                            .collect(),
                    );
                }
                Annotation::Notch { edge, distance, .. }
                | Annotation::ShapedNotch { edge, distance, .. } => {
                    ensure!(
                        *edge != fold_edge,
                        "remove notches from the fold edge before unfolding"
                    );
                    let original = *edge;
                    let pos = order
                        .iter()
                        .position(|i| *i == original)
                        .context("invalid notch edge")?;
                    *edge = if mirrored {
                        2 * order.len() - 1 - pos
                    } else {
                        pos
                    };
                    if mirrored {
                        let d = match distance {
                            Value::Number(n) => n.to_string(),
                            Value::Formula(s) => s.clone(),
                        };
                        *distance =
                            Value::Formula(format!("length({})-({d})", p.edges[original].object));
                    }
                }
                _ => {}
            }
            result.push(a);
        }
        Ok(result)
    };
    full.annotations = remap(&p.annotations, false)?;
    for (original, mirrored) in p
        .annotations
        .iter()
        .zip(remap(&reflected.annotations, true)?)
    {
        let refs = match original {
            Annotation::Points { points, .. } => points.iter().map(String::as_str).collect(),
            Annotation::Button { point, .. } => vec![point.as_str()],
            Annotation::SeamIntersections { path, .. } => vec![path.as_str()],
            _ => vec![],
        };
        // A mark exactly on the fold is already present. Duplicating it can
        // cause a cutter/plotter to visit the same drill or line twice.
        if !on_axis(&out, &refs, &axis)? {
            full.annotations.push(mirrored);
        }
    }
    for (original, mirrored) in p.internal_paths.iter().zip(reflected.internal_paths) {
        let refs: Vec<_> = original.edges.iter().map(|e| e.object.as_str()).collect();
        if !on_axis(&out, &refs, &axis)? {
            full.internal_paths.push(mirrored);
        }
    }
    for (name, point) in reflected.anchors {
        let name = format!("mirrored.{name}");
        ensure!(
            !full.anchors.contains_key(&name),
            "unfolded anchor name collision: {name}"
        );
        full.anchors.insert(name, point);
    }
    out.pieces.push(full);
    for size in &out.sizes {
        evaluate::grade(&out, size, 0.1)?;
    }
    Ok(out)
}
