//! Editor geometry and input helpers shared by all browser modes.
use beta_marker_core::Pt;
use beta_pattern_core::{
    Document, Object, Value,
    evaluate::{self, Evaluator, Geometry},
};
use serde_json::{Value as Json, json};

/// Evaluate every construction, including objects which do not belong to a piece.
/// Piece failures are returned inline so an unfinished draft can still be drawn.
pub fn inspect(doc: &Document, size: &str) -> Result<Json, String> {
    let evaluated = evaluate::evaluate_pattern(doc, size, 0.1).map_err(|e| format!("{e:#}"))?;
    let mut variables = Evaluator::new(doc, size, 0.1).map_err(|e| format!("{e:#}"))?;
    for key in doc.variables.keys() {
        variables
            .value(&Value::Formula(format!("v.{key}")))
            .map_err(|e| format!("{e:#}"))?;
    }
    let mut objects = serde_json::Map::new();
    let mut lengths = std::collections::BTreeMap::new();
    for item in evaluated.objects {
        if let Some(error) = item.error {
            return Err(error);
        }
        let object = match item.geometry.ok_or("Drawing object has no geometry")? {
            Geometry::Point(p) => json!({"kind":"point","points":[p]}),
            Geometry::Path(points) => {
                lengths.insert(
                    item.id.clone(),
                    beta_pattern_core::geometry::length(&points),
                );
                json!({"kind":"path","points":points})
            }
        };
        objects.insert(item.id, object);
    }
    let pieces: Vec<_> = doc
        .pieces
        .iter()
        .map(|piece| {
            if let Some(error) = evaluated.piece_errors.get(&piece.id) {
                return json!({"id":piece.id,"ok":false,"error":error});
            }
            let id = format!("{}-{size}", piece.id);
            let graded = evaluated.pieces.iter().find(|g| g.piece.id == id);
            let edges: Vec<_> = piece
                .edges
                .iter()
                .enumerate()
                .map(|(i, edge)| {
                    let length = graded
                        .and_then(|g| g.seam_edges.get(i))
                        .map(|path| beta_pattern_core::geometry::length(path))
                        .unwrap_or_else(|| lengths.get(&edge.object).copied().unwrap_or(0.));
                    json!({"object":edge.object,"length_mm":length})
                })
                .collect();
            json!({"id":piece.id,"ok":true,"graded":graded,"edges":edges})
        })
        .collect();
    Ok(json!({"objects":objects,"pieces":pieces}))
}

/// Check one safe expression in the current size context.
pub fn check_formula(doc: &Document, size: &str, formula: &str) -> Result<f64, String> {
    Evaluator::new(doc, size, 0.1)
        .and_then(|mut e| e.value(&Value::Formula(formula.into())))
        .map_err(|e| format!("{e:#}"))
}

/// Options for snapping a pointer in document millimetres.
pub struct SnapSettings<'a> {
    /// Pointer position.
    pub at: Pt,
    /// Maximum point/path attraction radius.
    pub radius: f64,
    /// Grid step; zero disables grid snapping.
    pub grid: f64,
    /// Enable point attraction.
    pub points: bool,
    /// Enable line/curve projection.
    pub paths: bool,
    /// Exclude a dragged point and its dependent paths.
    pub ignore: Option<&'a str>,
}
/// Snap to the nearest point or sampled path within radius, then to the grid.
pub fn snap(doc: &Document, size: &str, settings: SnapSettings<'_>) -> Result<Json, String> {
    let SnapSettings {
        at,
        radius,
        grid,
        points,
        paths,
        ignore,
    } = settings;
    let allowed = ignore
        .map(|id| beta_pattern_core::tools::reference_order(doc, Some(id)))
        .transpose()
        .map_err(|e| format!("{e:#}"))?;
    if !at.iter().all(|v| v.is_finite())
        || !radius.is_finite()
        || radius <= 0.
        || !grid.is_finite()
        || grid < 0.
    {
        return Err("Use finite coordinates, positive snap radius and non-negative grid".into());
    }
    let mut ev = Evaluator::new(doc, size, 0.1).map_err(|e| format!("{e:#}"))?;
    let mut best = radius;
    let mut target = at;
    let mut source: Option<String> = None;
    for (id, object) in &doc.drawing {
        if allowed.as_ref().is_some_and(|ids| !ids.contains(id)) {
            continue;
        }
        if points && matches!(object, Object::Point { .. }) {
            let p = ev.point(id).map_err(|e| format!("{e:#}"))?;
            let d = (p[0] - at[0]).hypot(p[1] - at[1]);
            if d < best {
                best = d;
                target = p;
                source = Some(id.clone());
            }
        }
    }
    // A nearby point takes precedence over a line projection.
    if source.is_none() && paths {
        for (id, object) in &doc.drawing {
            if allowed.as_ref().is_some_and(|ids| !ids.contains(id)) {
                continue;
            }
            if matches!(object, Object::Point { .. }) {
                continue;
            }
            for ab in ev.path(id).map_err(|e| format!("{e:#}"))?.windows(2) {
                let v = [ab[1][0] - ab[0][0], ab[1][1] - ab[0][1]];
                let n = v[0] * v[0] + v[1] * v[1];
                if n < 1e-12 {
                    continue;
                }
                let t = (((at[0] - ab[0][0]) * v[0] + (at[1] - ab[0][1]) * v[1]) / n).clamp(0., 1.);
                let p = [ab[0][0] + t * v[0], ab[0][1] + t * v[1]];
                let d = (p[0] - at[0]).hypot(p[1] - at[1]);
                if d < best {
                    best = d;
                    target = p;
                    source = Some(id.clone());
                }
            }
        }
    }
    if source.is_none() && grid > 0. {
        target = [(at[0] / grid).round() * grid, (at[1] / grid).round() * grid];
    }
    Ok(json!({"point":target,"object":source}))
}

/// Calibrate a background image from two pixel positions and a known millimetre length.
pub fn image_scale(a: Pt, b: Pt, distance_mm: f64) -> Result<f64, String> {
    let pixels = (a[0] - b[0]).hypot(a[1] - b[1]);
    if !a.iter().chain(b.iter()).all(|v| v.is_finite())
        || !pixels.is_finite()
        || pixels <= 1e-6
        || !distance_mm.is_finite()
        || distance_mm <= 0.
    {
        return Err("Choose two different image points and a positive known distance".into());
    }
    Ok(distance_mm / pixels)
}

/// Import individual/multisize Seamly measurements or the engine's CSV table.
pub fn measurements(
    doc: &Document,
    source: &str,
    format: &str,
    size: &str,
) -> Result<Document, String> {
    let set = if format == "csv" {
        beta_pattern_core::measurements::import_csv(source)
    } else {
        beta_pattern_core::measurements::import_seamly(source, size)
    }
    .map_err(|e| format!("{e:#}"))?;
    if set.sizes.len() == 1 && doc.sizes.contains(&set.sizes[0]) {
        let mut next = doc.clone();
        let key = &set.sizes[0];
        next.measurements
            .entry(key.clone())
            .or_default()
            .extend(set.values[key].clone());
        Ok(next)
    } else {
        set.apply(doc).map_err(|e| format!("{e:#}"))
    }
}
/// Export measurements as the engine's documented CSV interchange table.
pub fn measurement_csv(doc: &Document) -> Result<String, String> {
    beta_pattern_core::measurements::export_csv(&beta_pattern_core::measurements::MeasurementSet {
        sizes: doc.sizes.clone(),
        base_size: doc.base_size.clone(),
        values: doc.measurements.clone(),
        heights: Default::default(),
    })
    .map_err(|e| format!("{e:#}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn piece_lengths_use_factory_grades_while_draft_objects_keep_their_coordinates() {
        use beta_pattern_core::{GradeMode, GradeRule, tools};
        use std::collections::BTreeMap;
        let mut doc = Document {
            sizes: vec!["M".into(), "L".into()],
            ..Default::default()
        };
        tools::insert_polygon(
            &mut doc,
            "panel",
            "Panel",
            &[[0., 0.], [100., 0.], [100., 200.], [0., 200.]],
        )
        .unwrap();
        for (name, delta) in [("fixed", [0., 0.]), ("grow", [20., 0.])] {
            doc.rules.insert(
                name.into(),
                GradeRule {
                    mode: GradeMode::Relative,
                    deltas: BTreeMap::from([("L".into(), delta)]),
                },
            );
        }
        for i in 0..4 {
            doc.pieces[0].grade_points.insert(
                format!("panel.p{i}"),
                if i == 1 || i == 2 { "grow" } else { "fixed" }.into(),
            );
        }
        let result = inspect(&doc, "L").unwrap();
        assert_eq!(result["pieces"][0]["edges"][0]["length_mm"], 120.);
        assert_eq!(
            result["objects"]["panel.p1"]["points"][0],
            json!([100., 0.])
        );
    }
    #[test]
    fn unfinished_objects_and_formula_errors_are_checked() {
        let mut d = Document::default();
        d.drawing.insert(
            "A".into(),
            Object::Point {
                operation: beta_pattern_core::PointOp::Absolute {
                    x: 10.0.into(),
                    y: 20.0.into(),
                },
            },
        );
        assert_eq!(
            inspect(&d, "M").unwrap()["objects"]["A"]["points"][0],
            json!([10., 20.])
        );
        assert_eq!(check_formula(&d, "M", "x(A)+5").unwrap(), 15.);
        assert!(check_formula(&d, "M", "v.missing").is_err());
        assert_eq!(
            snap(
                &d,
                "M",
                SnapSettings {
                    at: [11., 20.],
                    radius: 5.,
                    grid: 10.,
                    points: true,
                    paths: true,
                    ignore: None
                }
            )
            .unwrap()["object"],
            "A"
        );
        assert_eq!(
            snap(
                &d,
                "M",
                SnapSettings {
                    at: [27., 31.],
                    radius: 5.,
                    grid: 10.,
                    points: true,
                    paths: true,
                    ignore: None
                }
            )
            .unwrap()["point"],
            json!([30., 30.])
        );
        assert_eq!(image_scale([0., 0.], [100., 0.], 200.).unwrap(), 2.);
        assert!(image_scale([0., 0.], [0., 0.], 200.).is_err());
    }
}
