//! Safe document edits and dependency-ordered reference lists for drafting editors.
use crate::{
    Document, Object, PointOp, Value,
    evaluate::{Evaluator, Geometry},
    formula,
    geometry::*,
};
use anyhow::{Context, Result, ensure};
use beta_marker_core::Pt;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;
use std::collections::BTreeSet;

/// Geometry or an object-specific error. Independent objects still evaluate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectEvaluation {
    /// Stable drawing ID.
    pub id: String,
    /// Evaluated geometry, absent on error.
    pub geometry: Option<Geometry>,
    /// Full error chain, absent on success.
    pub error: Option<String>,
}
/// Evaluate every object with a shared cache; errors do not hide independent geometry.
pub fn evaluate_document(
    doc: &Document,
    size: &str,
    tolerance: f64,
) -> Result<Vec<ObjectEvaluation>> {
    let mut e = Evaluator::new(doc, size, tolerance)?;
    Ok(doc
        .drawing
        .keys()
        .map(|id| match e.object(id) {
            Ok(g) => ObjectEvaluation {
                id: id.clone(),
                geometry: Some(g),
                error: None,
            },
            Err(err) => ObjectEvaluation {
                id: id.clone(),
                geometry: None,
                error: Some(format!("{err:#}")),
            },
        })
        .collect())
}
fn strings(value: &Json, f: &mut impl FnMut(&str)) {
    match value {
        Json::String(s) => f(s),
        Json::Array(a) => {
            for v in a {
                strings(v, f)
            }
        }
        Json::Object(o) => {
            for (k, v) in o {
                if !matches!(
                    k.as_str(),
                    "kind" | "op" | "mark" | "text" | "name" | "material" | "id" | "axis"
                ) {
                    strings(v, f)
                }
            }
        }
        _ => {}
    }
}
fn referenced(value: &Json, id: &str) -> bool {
    let mut found = false;
    strings(value, &mut |s| {
        found |=
            s == id || formula::references(s).is_ok_and(|(ids, _)| ids.iter().any(|s| s == id));
    });
    found
}
fn reference_kind(kind: &crate::catalog::ParameterType) -> bool {
    use crate::catalog::ParameterType::*;
    matches!(
        kind,
        PointRef | LineRef | CurveRef | ObjectRef | PointRefs | CubicSegments
    )
}
fn object_dependencies(doc: &Document, object: &Object) -> Result<BTreeSet<String>> {
    if let Object::DerivedPath { recipe, .. } = object {
        let mut refs = BTreeSet::new();
        let mut sources = vec![recipe.source.as_str()];
        if let crate::derived::Shape::Join { other } = &recipe.operation {
            sources.push(other);
        }
        for source in sources {
            let p = doc
                .pieces
                .iter()
                .find(|p| p.id == source)
                .context("Derived source piece missing")?;
            refs.extend(p.edges.iter().map(|e| e.object.clone()));
        }
        strings(&serde_json::to_value(&recipe.operation)?, &mut |s| {
            if doc.drawing.contains_key(s) {
                refs.insert(s.into());
            }
            if let Ok((ids, _)) = formula::references(s) {
                refs.extend(ids);
            }
        });
        return Ok(refs);
    }
    let json = serde_json::to_value(object)?;
    let fields = if json["kind"] == "point" {
        &json["operation"]
    } else {
        &json
    };
    let id = fields
        .get("op")
        .or_else(|| fields.get("kind"))
        .and_then(Json::as_str)
        .context("object tag missing")?;
    let spec = crate::catalog::catalog()
        .into_iter()
        .find(|s| s.id == id)
        .context("object missing catalog specification")?;
    let mut refs = BTreeSet::new();
    let mut vars = vec![];
    for p in spec.parameters {
        if let Some(value) = fields.get(&p.name) {
            if reference_kind(&p.kind) {
                strings(value, &mut |id| {
                    refs.insert(id.into());
                });
            } else if matches!(
                p.kind,
                crate::catalog::ParameterType::Formula
                    | crate::catalog::ParameterType::AngleFormula
                    | crate::catalog::ParameterType::Tangents
            ) {
                strings(value, &mut |s| {
                    if let Ok((ids, vs)) = formula::references(s) {
                        refs.extend(ids);
                        vars.extend(vs);
                    }
                });
            }
        }
    }
    fn variable(
        name: &str,
        doc: &Document,
        active: &mut BTreeSet<String>,
        done: &mut BTreeSet<String>,
        refs: &mut BTreeSet<String>,
    ) -> Result<()> {
        if done.contains(name) {
            return Ok(());
        }
        ensure!(
            active.len() < 128 && active.insert(name.into()),
            "variable dependency cycle at '{name}'"
        );
        if let Some(Value::Formula(s)) = doc.variables.get(name) {
            let (ids, vs) = formula::references(s)?;
            refs.extend(ids);
            for v in vs {
                variable(&v, doc, active, done, refs)?;
            }
        }
        active.remove(name);
        done.insert(name.into());
        Ok(())
    }
    let mut done = BTreeSet::new();
    for v in vars {
        variable(&v, doc, &mut BTreeSet::new(), &mut done, &mut refs)?;
    }
    Ok(refs)
}
/// Drawing, variable, piece or group users that prevent deleting an object.
pub fn dependents(doc: &Document, id: &str) -> Result<Vec<String>> {
    let mut out = vec![];
    for (key, o) in &doc.drawing {
        if key != id && object_dependencies(doc, o)?.contains(id) {
            out.push(format!("object:{key}"));
        }
    }
    for (key, v) in &doc.variables {
        if referenced(&serde_json::to_value(v)?, id) {
            out.push(format!("variable:{key}"));
        }
    }
    for p in &doc.pieces {
        if referenced(&serde_json::to_value(p)?, id) || p.grade_points.contains_key(id) {
            out.push(format!("piece:{}", p.id));
        }
    }
    for (key, g) in &doc.groups {
        if g.objects.iter().any(|s| s == id) {
            out.push(format!("group:{key}"));
        }
    }
    Ok(out)
}
/// Available drawing IDs in dependency order. `editing` excludes it and all its users.
pub fn reference_order(doc: &Document, editing: Option<&str>) -> Result<Vec<String>> {
    let graph = doc
        .drawing
        .iter()
        .map(|(id, o)| Ok((id.clone(), object_dependencies(doc, o)?)))
        .collect::<Result<std::collections::BTreeMap<_, _>>>()?;
    fn visit(
        id: &str,
        graph: &std::collections::BTreeMap<String, BTreeSet<String>>,
        active: &mut BTreeSet<String>,
        done: &mut BTreeSet<String>,
        out: &mut Vec<String>,
    ) -> Result<()> {
        if done.contains(id) {
            return Ok(());
        }
        ensure!(
            active.len() < 128 && active.insert(id.into()),
            "dependency cycle or depth limit at '{id}'"
        );
        for dep in graph
            .get(id)
            .with_context(|| format!("missing drawing dependency '{id}'"))?
        {
            visit(dep, graph, active, done, out)?;
        }
        active.remove(id);
        done.insert(id.into());
        out.push(id.into());
        Ok(())
    }
    let mut out = vec![];
    let mut done = BTreeSet::new();
    for id in graph.keys() {
        visit(id, &graph, &mut BTreeSet::new(), &mut done, &mut out)?;
    }
    if let Some(editing) = editing {
        let mut excluded = BTreeSet::from([editing.to_owned()]);
        for id in &out {
            if graph[id].iter().any(|dep| excluded.contains(dep)) {
                excluded.insert(id.clone());
            }
        }
        out.retain(|id| !excluded.contains(id));
    }
    Ok(out)
}
fn rewrite(value: &mut Json, old: &str, new: &str) -> Result<()> {
    match value {
        Json::String(s) if s == old => *s = new.into(),
        Json::String(s) => {
            if formula::references(s).is_ok_and(|(ids, _)| ids.iter().any(|id| id == old)) {
                *s = formula::rename_reference(s, old, new)?;
            }
        }
        Json::Array(a) => {
            for v in a {
                rewrite(v, old, new)?;
            }
        }
        Json::Object(o) => {
            for (k, v) in o {
                if !matches!(
                    k.as_str(),
                    "kind" | "op" | "mark" | "text" | "name" | "material" | "id" | "axis"
                ) {
                    rewrite(v, old, new)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}
/// Rename an object and all direct/formula references atomically.
pub fn rename(doc: &Document, old: &str, new: &str) -> Result<Document> {
    ensure!(doc.drawing.contains_key(old), "unknown object '{old}'");
    ensure!(
        valid_id(new) && !doc.drawing.contains_key(new),
        "new ID invalid or already exists"
    );
    let mut out = doc.clone();
    let object = out.drawing.remove(old).context("object missing")?;
    out.drawing.insert(new.into(), object);
    for o in out.drawing.values_mut() {
        let mut j = serde_json::to_value(&*o)?;
        rewrite(&mut j, old, new)?;
        *o = serde_json::from_value(j)?;
    }
    for v in out.variables.values_mut() {
        if let Value::Formula(s) = v
            && formula::references(s)?.0.iter().any(|id| id == old)
        {
            *s = formula::rename_reference(s, old, new)?;
        }
    }
    for p in &mut out.pieces {
        let mut j = serde_json::to_value(&*p)?;
        rewrite(&mut j, old, new)?;
        *p = serde_json::from_value(j)?;
        if let Some(rule) = p.grade_points.remove(old) {
            ensure!(
                !p.grade_points.contains_key(new),
                "grade point rename collision"
            );
            p.grade_points.insert(new.into(), rule);
        }
    }
    for g in out.groups.values_mut() {
        for id in &mut g.objects {
            if id == old {
                *id = new.into();
            }
        }
    }
    Ok(out)
}
/// Delete an unused object, rejecting all blocking users.
pub fn delete(doc: &Document, id: &str) -> Result<Document> {
    ensure!(doc.drawing.contains_key(id), "unknown object '{id}'");
    let users = dependents(doc, id)?;
    ensure!(
        users.is_empty(),
        "object '{id}' is used by {}",
        users.join(", ")
    );
    let mut out = doc.clone();
    out.drawing.remove(id);
    Ok(out)
}
/// IDs usable in geometry formulas.
pub fn valid_id(id: &str) -> bool {
    let mut c = id.chars();
    c.next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && c.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
}
/// Drag Absolute or Polar points, replacing their coordinate formulas with numeric values.
pub fn move_point(doc: &Document, id: &str, size: &str, target: Pt) -> Result<Document> {
    ensure!(
        target.iter().all(|v| v.is_finite()),
        "non-finite drag coordinate"
    );
    let mut out = doc.clone();
    let operation = match doc.drawing.get(id).context("unknown drag point")? {
        Object::Point { operation } => operation,
        _ => anyhow::bail!("object is not a point"),
    };
    let operation = match operation {
        PointOp::Absolute { .. } => PointOp::Absolute {
            x: target[0].into(),
            y: target[1].into(),
        },
        PointOp::Polar { from, .. } => {
            let origin = Evaluator::new(doc, size, 0.1)?.point(from)?;
            let delta = sub(target, origin);
            PointOp::Polar {
                from: from.clone(),
                distance: distance(origin, target).into(),
                angle: delta[1].atan2(delta[0]).to_degrees().into(),
            }
        }
        _ => anyhow::bail!("construction does not support interactive movement"),
    };
    out.drawing.insert(id.into(), Object::Point { operation });
    Ok(out)
}
/// Apply one generic catalog construction to a cloned document. Rejects ID collisions.
pub fn apply(
    doc: &Document,
    tool_id: &str,
    id: &str,
    params: serde_json::Map<String, Json>,
) -> Result<Document> {
    ensure!(
        valid_id(id) && !doc.drawing.contains_key(id),
        "drawing ID invalid or already used"
    );
    let object = crate::catalog::construct(tool_id, params)?;
    let mut out = doc.clone();
    out.drawing.insert(id.into(), object);
    reference_order(&out, None)?;
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn catalog_apply_and_safe_edits() {
        let d = apply(
            &Document::default(),
            "absolute",
            "A",
            json!({"x":1,"y":2}).as_object().unwrap().clone(),
        )
        .unwrap();
        let d = apply(
            &d,
            "polar",
            "B",
            json!({"from":"A","distance":10,"angle":90})
                .as_object()
                .unwrap()
                .clone(),
        )
        .unwrap();
        assert_eq!(reference_order(&d, None).unwrap(), ["A", "B"]);
        assert!(delete(&d, "A").is_err());
        assert_eq!(
            reference_order(&d, Some("A")).unwrap(),
            Vec::<String>::new()
        );
        let d = rename(&d, "A", "origin").unwrap();
        assert!(
            distance(
                Evaluator::new(&d, "M", 0.1).unwrap().point("B").unwrap(),
                [1., 12.]
            ) < 1e-9
        );
        let d = move_point(&d, "B", "M", [11., 2.]).unwrap();
        assert_eq!(
            Evaluator::new(&d, "M", 0.1).unwrap().point("B").unwrap(),
            [11., 2.]
        );
        assert_eq!(delete(&d, "B").unwrap().drawing.len(), 1);
        let mut d = d;
        d.drawing.insert(
            "bad".into(),
            Object::Point {
                operation: PointOp::Absolute {
                    x: "1/0".into(),
                    y: 0.0.into(),
                },
            },
        );
        assert_eq!(
            evaluate_document(&d, "M", 0.1)
                .unwrap()
                .iter()
                .filter(|r| r.error.is_some())
                .count(),
            1
        );
    }
    #[test]
    fn renames_geometry_arguments_without_substrings() {
        let mut d = apply(
            &Document::default(),
            "absolute",
            "A",
            json!({"x":1,"y":2}).as_object().unwrap().clone(),
        )
        .unwrap();
        d = apply(
            &d,
            "absolute",
            "AA",
            json!({"x":"x(A)+2","y":2}).as_object().unwrap().clone(),
        )
        .unwrap();
        let d = rename(&d, "A", "Z").unwrap();
        assert_eq!(
            Evaluator::new(&d, "M", 0.1).unwrap().point("AA").unwrap(),
            [3., 2.]
        );
        assert!(delete(&d, "Z").is_err());
    }
}
