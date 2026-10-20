//! Piece editing commands. Geometry stays in the core; edits keep their source pieces.
use beta_pattern_core::{
    Document, Value,
    evaluate::{self, Evaluator},
    tools,
};
use serde_json::{Value as Json, json};

/// Apply a piece menu action. New geometry is inserted as an editable fixed outline.
pub fn apply(doc: &Document, request: &Json) -> Result<Json, String> {
    let s = |key: &str| request[key].as_str().ok_or_else(|| format!("Choose {key}"));
    let action = s("action")?;
    let size = request["size"].as_str().unwrap_or(&doc.base_size);
    let source = s("piece")?;
    let piece = doc
        .pieces
        .iter()
        .find(|p| p.id == source)
        .ok_or("Choose a piece")?;
    let mut ev = Evaluator::new(doc, size, 0.1).map_err(|e| format!("{e:#}"))?;
    let mut number = |key: &str| -> Result<f64, String> {
        let v: Value =
            serde_json::from_value(request[key].clone()).map_err(|_| format!("Enter {key}"))?;
        ev.value(&v).map_err(|e| format!("{e:#}"))
    };
    if action == "walk" || action == "true_seam" {
        let mut e = Evaluator::new(doc, size, 0.05).map_err(|e| format!("{e:#}"))?;
        let a = e.path(s("a")?).map_err(|e| format!("{e:#}"))?;
        if action == "walk" {
            let b = e.path(s("b")?).map_err(|e| format!("{e:#}"))?;
            let difference = tools::walk(&a, &b).map_err(|e| format!("{e:#}"))?;
            return Ok(
                json!({"document":doc,"message":format!("First seam minus second: {difference:.3} mm"),"difference_mm":difference}),
            );
        }
        let path = tools::true_seam(&a, number("target")?).map_err(|e| format!("{e:#}"))?;
        let mut out = doc.clone();
        let id = s("id")?;
        if !beta_pattern_core::editor::valid_id(id) {
            return Err("Use a valid output ID".into());
        }
        if out.drawing.contains_key(id) {
            return Err("Output ID already exists".into());
        }
        let mut points = vec![];
        for (i, p) in path.iter().enumerate() {
            let key = format!("{id}.p{i}");
            if out.drawing.contains_key(&key) {
                return Err("Output point ID already exists".into());
            }
            out.drawing.insert(
                key.clone(),
                beta_pattern_core::Object::Point {
                    operation: beta_pattern_core::PointOp::Absolute {
                        x: p[0].into(),
                        y: p[1].into(),
                    },
                },
            );
            points.push(key);
        }
        out.drawing.insert(
            id.into(),
            beta_pattern_core::Object::Spline {
                tangents: vec![[0.0.into(), 0.0.into()]; points.len()],
                points,
            },
        );
        return Ok(
            json!({"document":out,"message":"Trued seam added as a separate path. Connect and review it before replacing a piece edge."}),
        );
    }
    let mut single = doc.clone();
    single.pieces = vec![piece.clone()];
    let graded = evaluate::grade(&single, size, 0.05).map_err(|e| format!("{e:#}"))?;
    let poly = &graded.first().ok_or("Piece could not be evaluated")?.seam;
    let point = |key: &str| -> Result<[f64; 2], String> {
        Evaluator::new(doc, size, 0.1)
            .and_then(|mut e| e.point(s(key).map_err(anyhow_error)?))
            .map_err(|e| format!("{e:#}"))
    };
    let id = s("id")?;
    if !beta_pattern_core::editor::valid_id(id) {
        return Err("Use a valid output ID".into());
    }
    let mut out = doc.clone();
    let mut dart = None;
    let mut folds = vec![];
    let result = match action {
        "mirror" => tools::mirror(poly, point("a")?, point("b")?),
        "split" => {
            let parts =
                tools::split(poly, point("a")?, point("b")?).map_err(|e| format!("{e:#}"))?;
            tools::insert_polygon(
                &mut out,
                &format!("{id}_2"),
                &format!("{} split 2", piece.name),
                &parts[1],
            )
            .map_err(|e| format!("{e:#}"))?;
            Ok(parts[0].clone())
        }
        "slash_spread" => tools::slash_spread(poly, point("a")?, point("b")?, number("angle")?),
        "pleat" => {
            let p = tools::pleat(
                poly,
                point("a")?,
                point("b")?,
                number("depth")?,
                if request["pleat_kind"] == "box" {
                    tools::PleatKind::Box
                } else {
                    tools::PleatKind::Knife
                },
            )
            .map_err(|e| format!("{e:#}"))?;
            folds = p.folds;
            Ok(p.outline)
        }
        "dart" => {
            let edge = number("edge")?;
            if edge < 1. || edge.fract() != 0. {
                return Err("Use a whole edge number starting at 1".into());
            }
            let d = tools::dart(
                poly,
                (edge as usize) - 1,
                number("center")?,
                number("width")?,
                point("apex")?,
            )
            .map_err(|e| format!("{e:#}"))?;
            let p = d.outline.clone();
            dart = Some(d);
            Ok(p)
        }
        "dart_transfer" | "dart_close" => {
            let d: tools::Dart = serde_json::from_value(
                doc.meta
                    .get(&format!("dart:{source}"))
                    .cloned()
                    .ok_or("Choose a piece created by the dart tool")?,
            )
            .map_err(|e| e.to_string())?;
            if action == "dart_transfer" {
                let edge = number("edge")?;
                if edge < 1. || edge.fract() != 0. {
                    return Err("Use a whole edge number starting at 1".into());
                }
                let d = tools::transfer_dart(&d, (edge as usize) - 1, number("center")?)
                    .map_err(|e| format!("{e:#}"))?;
                let p = d.outline.clone();
                dart = Some(d);
                Ok(p)
            } else {
                let v = number("vertex")?;
                if v < 1. || v.fract() != 0. {
                    return Err("Use a whole vertex number starting at 1".into());
                }
                tools::close_dart(&d, (v as usize) - 1)
            }
        }
        _ => return Err("Choose a supported piece action".into()),
    }
    .map_err(|e| format!("{e:#}"))?;
    tools::insert_polygon(&mut out, id, &format!("{} {action}", piece.name), &result)
        .map_err(|e| format!("{e:#}"))?;
    for new in out.pieces.iter_mut().skip(doc.pieces.len()) {
        new.quantity = piece.quantity;
        new.mirror_quantity = piece.mirror_quantity;
        new.material = piece.material.clone();
        new.rotation = piece.rotation;
        new.buffer = piece.buffer.clone();
        if let Some(first) = piece.edges.first()
            && piece
                .edges
                .iter()
                .all(|e| e.allowance == first.allowance && e.corner == first.corner)
        {
            for edge in &mut new.edges {
                edge.allowance = first.allowance.clone();
                edge.corner = first.corner;
            }
        }
    }
    if !folds.is_empty() {
        tools::add_fold_guides(&mut out, id, &folds).map_err(|e| format!("{e:#}"))?;
    }
    if let Some(d) = dart {
        out.meta.insert(
            format!("dart:{id}"),
            serde_json::to_value(d).map_err(|e| e.to_string())?,
        );
    }
    Ok(
        json!({"document":out,"message":"New piece added. Source retained. The new outline uses fixed geometry: check measurements, seam allowance, marks, grain and grading before cutting."}),
    )
}

fn anyhow_error(message: String) -> std::io::Error {
    std::io::Error::other(message)
}
#[cfg(test)]
mod tests {
    use super::*;
    use beta_pattern_core::{Object, PointOp};
    fn square() -> Document {
        let mut d = Document::default();
        tools::insert_polygon(
            &mut d,
            "square",
            "Square",
            &[[0., 0.], [100., 0.], [100., 100.], [0., 100.]],
        )
        .unwrap();
        for (name, p) in [
            ("bottom", [50., 0.]),
            ("top", [50., 100.]),
            ("apex", [50., 50.]),
        ] {
            d.drawing.insert(
                name.into(),
                Object::Point {
                    operation: PointOp::Absolute {
                        x: p[0].into(),
                        y: p[1].into(),
                    },
                },
            );
        }
        d
    }
    #[test]
    fn menu_operations_retain_sources_and_reject_bad_fields() {
        let d = square();
        for action in ["mirror", "split", "slash_spread", "pleat"] {
            let r=apply(&d,&json!({"action":action,"piece":"square","id":"new_piece","a":"bottom","b":"top","angle":30,"depth":10,"pleat_kind":"box"})).unwrap();
            let next: Document = serde_json::from_value(r["document"].clone()).unwrap();
            assert_eq!(next.pieces[0], d.pieces[0]);
            assert!(next.pieces.len() >= 2);
            assert!(evaluate::grade(&next, "M", 0.1).is_ok());
        }
        let created=apply(&d,&json!({"action":"dart","piece":"square","id":"dart_piece","edge":1,"center":50,"width":20,"apex":"apex"})).unwrap();
        let dart: Document = serde_json::from_value(created["document"].clone()).unwrap();
        assert!(apply(&dart,&json!({"action":"dart_transfer","piece":"dart_piece","id":"transferred","edge":3,"center":50})).is_ok());
        assert!(
            apply(
                &dart,
                &json!({"action":"dart_close","piece":"dart_piece","id":"closed","vertex":3})
            )
            .is_ok()
        );
        assert!(apply(&d,&json!({"action":"dart","piece":"square","id":"bad","edge":0,"center":50,"width":20,"apex":"apex"})).is_err());
        let trued=apply(&d,&json!({"action":"true_seam","piece":"square","id":"trued","a":"square.e0","target":125})).unwrap();
        let next: Document = serde_json::from_value(trued["document"].clone()).unwrap();
        let path = Evaluator::new(&next, "M", 0.01)
            .unwrap()
            .path("trued")
            .unwrap();
        assert!((beta_pattern_core::geometry::length(&path) - 125.).abs() < 0.001);
        assert!(
            apply(
                &d,
                &json!({"action":"walk","piece":"square","a":"square.e0","b":"square.e2"})
            )
            .unwrap()["difference_mm"]
                .as_f64()
                .unwrap()
                .abs()
                < 1e-9
        );
    }
}
