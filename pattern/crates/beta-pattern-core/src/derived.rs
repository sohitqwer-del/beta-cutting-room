//! Associative piece constructions. Recipes are evaluated again in every size.
use crate::{
    evaluate::{self, Evaluator},
    geometry,
    model::*,
    tools,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};

/// Source piece and a size-independent construction recipe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recipe {
    /// Retained source piece ID.
    pub source: String,
    /// Construction evaluated from the current source and chart.
    pub operation: Shape,
    /// Keep exterior source allowances for a joined boundary until explicitly edited.
    #[serde(default)]
    pub inherit_allowances: bool,
}
/// Linked piece operation. Distances may be measurement formulas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum Shape {
    Split {
        a: String,
        b: String,
        part: usize,
    },
    Join {
        other: String,
    },
    Dart {
        edge: usize,
        center: Value,
        width: Value,
        apex: String,
    },
    Pleat {
        a: String,
        b: String,
        depth: Value,
        box_pleat: bool,
    },
    DartTransfer {
        edge: usize,
        center: Value,
    },
    DartClose {
        vertex: usize,
    },
    Offset {
        distance: Value,
    },
}
/// Add a linked boundary and its piece metadata. Source pieces are retained.
pub fn insert(doc: &Document, id: &str, recipe: Recipe) -> Result<Document> {
    ensure!(
        crate::editor::valid_id(id) && !doc.pieces.iter().any(|p| p.id == id),
        "Choose an unused piece ID"
    );
    let source = doc
        .pieces
        .iter()
        .find(|p| p.id == recipe.source)
        .context("Source piece missing")?;
    if let Shape::Join { other } = &recipe.operation {
        let b = doc
            .pieces
            .iter()
            .find(|p| p.id == *other)
            .context("Second piece missing")?;
        ensure!(source.material == b.material, "Piece materials differ");
    }
    let mut next = doc.clone();
    let path = format!("{id}.boundary");
    ensure!(
        !next.drawing.contains_key(&path),
        "Boundary ID already exists"
    );
    next.drawing.insert(
        path.clone(),
        Object::DerivedPath {
            recipe: Box::new(recipe.clone()),
            guide: None,
        },
    );
    let mut piece = source.clone();
    piece.id = id.into();
    piece.name = match &recipe.operation {
        Shape::Join { other } => format!(
            "{} + {}",
            source.name,
            doc.pieces
                .iter()
                .find(|p| p.id == *other)
                .context("Join source missing")?
                .name
        ),
        operation => format!(
            "{} {}",
            source.name,
            match operation {
                Shape::Split { part: 0, .. } => "split",
                Shape::Split { .. } => "split 2",
                Shape::Dart { .. } => "dart",
                Shape::DartTransfer { .. } => "dart transfer",
                Shape::DartClose { .. } => "closed dart",
                Shape::Pleat { .. } => "pleat",
                Shape::Offset { .. } => "offset",
                Shape::Join { .. } => unreachable!(),
            }
        ),
    };
    let first = source.edges.first().context("Source piece has no edges")?;
    let uniform = source
        .edges
        .iter()
        .all(|e| e.allowance == first.allowance && e.corner == first.corner);
    piece.edges = vec![Edge {
        object: path,
        reversed: false,
        allowance: if uniform {
            first.allowance.clone()
        } else {
            0.0.into()
        },
        corner: first.corner,
    }];
    // A shape change does not imply symmetry or a safe location for old seam marks.
    piece.fold_line = None;
    piece.annotations.clear();
    piece.internal_paths.clear();
    piece.anchors.clear();
    if let Shape::Join { other } = &recipe.operation {
        for id in [&recipe.source, other] {
            let source = doc
                .pieces
                .iter()
                .find(|p| p.id == *id)
                .context("Join source missing")?;
            piece.anchors.extend(
                source
                    .anchors
                    .iter()
                    .map(|(name, point)| (format!("{id}.{name}"), point.clone())),
            );
        }
    }
    piece.grade_points.clear();
    piece.label_point = None;
    if matches!(recipe.operation, Shape::Pleat { .. }) {
        let count = if matches!(
            recipe.operation,
            Shape::Pleat {
                box_pleat: true,
                ..
            }
        ) {
            5
        } else {
            3
        };
        for index in 0..count {
            let path = format!("{id}.fold{index}");
            ensure!(
                !next.drawing.contains_key(&path),
                "Fold guide ID already exists"
            );
            next.drawing.insert(
                path.clone(),
                Object::DerivedPath {
                    recipe: Box::new(recipe.clone()),
                    guide: Some(index),
                },
            );
            piece.internal_paths.push(InternalPath {
                name: "pleat fold".into(),
                edges: vec![Edge {
                    object: path,
                    reversed: false,
                    allowance: 0.0.into(),
                    corner: Corner::Miter,
                }],
                closed: false,
            });
        }
    }
    next.pieces.push(piece);
    Ok(next)
}
/// Apply the existing split/dart/pleat request, retaining construction references.
pub fn piece_action(doc: &Document, request: &serde_json::Value) -> Result<Document> {
    let text = |key: &str| {
        request[key]
            .as_str()
            .with_context(|| format!("Choose {key}"))
    };
    let value = |key: &str| -> Result<Value> { Ok(serde_json::from_value(request[key].clone())?) };
    let id = text("id")?;
    let source = text("piece")?;
    let shape = match text("action")? {
        "split" => Shape::Split {
            a: text("a")?.into(),
            b: text("b")?.into(),
            part: 0,
        },
        "pleat" => Shape::Pleat {
            a: text("a")?.into(),
            b: text("b")?.into(),
            depth: value("depth")?,
            box_pleat: request["pleat_kind"] == "box",
        },
        "dart_transfer" => {
            let edge = request["edge"]
                .as_u64()
                .filter(|n| *n > 0)
                .context("Choose an edge starting at 1")?;
            Shape::DartTransfer {
                edge: usize::try_from(edge).context("Edge number is too large")? - 1,
                center: value("center")?,
            }
        }
        "dart_close" => {
            let vertex = request["vertex"]
                .as_u64()
                .filter(|n| *n > 0)
                .context("Choose a vertex starting at 1")?;
            Shape::DartClose {
                vertex: usize::try_from(vertex).context("Vertex number is too large")? - 1,
            }
        }
        "dart" => {
            let size = request["size"].as_str().unwrap_or(&doc.base_size);
            let mut e = Evaluator::new(doc, size, 0.05)?;
            let index = doc
                .pieces
                .iter()
                .position(|p| p.id == source)
                .context("Source piece missing")?;
            let graded = evaluate::grade(doc, size, 0.05)?;
            let edges = &graded[index].seam_edges;
            let selected = if let Some(raw) = request.get("seam_edge") {
                usize::try_from(
                    raw.as_u64()
                        .context("Use a zero-based whole seam edge number")?,
                )
                .context("Seam edge is too large")?
            } else {
                let n = e.value(&value("edge")?)?;
                ensure!(
                    n >= 1. && n.fract() == 0.,
                    "Use a whole edge number starting at 1"
                );
                let mut remaining = n as usize - 1;
                let mut selected = None;
                for (i, path) in edges.iter().enumerate() {
                    if remaining < path.len() - 1 {
                        selected = Some(i);
                        break;
                    }
                    remaining -= path.len() - 1;
                }
                selected.context("Dart edge out of range")?
            };
            ensure!(
                edges
                    .get(selected)
                    .context("Dart seam edge out of range")?
                    .len()
                    == 2,
                "A dart needs a straight source edge; split or straighten this edge first"
            );
            Shape::Dart {
                edge: selected,
                center: value("center")?,
                width: value("width")?,
                apex: text("apex")?.into(),
            }
        }
        _ => anyhow::bail!("Unsupported linked shape"),
    };
    let mut out = insert(
        doc,
        id,
        Recipe {
            source: source.into(),
            operation: shape.clone(),
            inherit_allowances: false,
        },
    )?;
    if let Shape::Split { a, b, .. } = shape {
        out = insert(
            &out,
            &format!("{id}_2"),
            Recipe {
                source: source.into(),
                operation: Shape::Split { a, b, part: 1 },
                inherit_allowances: false,
            },
        )?;
    }
    Ok(out)
}

pub(crate) fn construct(
    e: &mut Evaluator<'_>,
    recipe: &Recipe,
    guide: Option<usize>,
) -> Result<Vec<beta_marker_core::Pt>> {
    let source = e.derived_source(&recipe.source)?;
    let poly = &source.seam;
    let mut folds = vec![];
    let mut outline = match &recipe.operation {
        Shape::Split { a, b, part } => {
            ensure!(*part < 2, "Split part must be 0 or 1");
            tools::split(poly, e.point(a)?, e.point(b)?)?[*part].clone()
        }
        Shape::Join { other } => tools::union_shared_seam(poly, &e.derived_source(other)?.seam)?,
        Shape::Dart {
            edge,
            center,
            width,
            apex,
        } => {
            let path = source
                .seam_edges
                .get(*edge)
                .context("Dart source edge missing")?;
            ensure!(path.len() == 2, "Dart source edge must remain straight");
            let index = source
                .seam_edges
                .iter()
                .take(*edge)
                .map(|p| p.len() - 1)
                .sum();
            tools::dart(
                poly,
                index,
                e.value(center)?,
                e.value(width)?,
                e.point(apex)?,
            )?
            .outline
        }
        Shape::DartTransfer { .. } => dart(e, recipe)?.outline,
        Shape::DartClose { vertex } => {
            let parent = e.derived_recipe(&recipe.source)?;
            tools::close_dart(&dart(e, &parent)?, *vertex)?
        }
        Shape::Pleat {
            a,
            b,
            depth,
            box_pleat,
        } => {
            let p = tools::pleat(
                poly,
                e.point(a)?,
                e.point(b)?,
                e.value(depth)?,
                if *box_pleat {
                    tools::PleatKind::Box
                } else {
                    tools::PleatKind::Knife
                },
            )?;
            folds = p.folds;
            p.outline
        }
        Shape::Offset { distance } => {
            let d = e.value(distance)?;
            ensure!(d >= 0., "Offset must be non-negative");
            geometry::offset(poly, &vec![d; poly.len()], &vec![Corner::Miter; poly.len()])?
        }
    };
    if let Some(index) = guide {
        return Ok(folds.get(index).context("Pleat guide missing")?.to_vec());
    }
    geometry::validate_polygon(&outline)?;
    outline.push(outline[0]);
    Ok(outline)
}

fn dart(e: &mut Evaluator<'_>, recipe: &Recipe) -> Result<tools::Dart> {
    match &recipe.operation {
        Shape::Dart {
            edge,
            center,
            width,
            apex,
        } => {
            let g = e.derived_source(&recipe.source)?;
            let index = g.seam_edges.iter().take(*edge).map(|p| p.len() - 1).sum();
            tools::dart(
                &g.seam,
                index,
                e.value(center)?,
                e.value(width)?,
                e.point(apex)?,
            )
        }
        Shape::DartTransfer { edge, center } => {
            let parent = e.derived_recipe(&recipe.source)?;
            tools::transfer_dart(&dart(e, &parent)?, *edge, e.value(center)?)
        }
        _ => anyhow::bail!("Choose a piece created by the dart tool"),
    }
}
