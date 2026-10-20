//! Original parametric teaching block and conversion of existing flattened Marker jobs.
use crate::{model::*, tools::insert_polygon};
use anyhow::{Context, Result, ensure};
use beta_marker_core::{Job, MarkKind, Pt};
use std::collections::BTreeMap;

/// Convert a single-size Marker job into a fixed pattern. Existing cut allowances are not added again.
pub fn from_marker_job(job: &Job) -> Result<Document> {
    let mut sizes: Vec<_> = job.pieces.iter().filter_map(|p| p.size.clone()).collect();
    sizes.sort();
    sizes.dedup();
    ensure!(
        sizes.len() <= 1,
        "fixed starter conversion requires a single-size job"
    );
    if sizes.is_empty() {
        sizes.push("M".into());
    }
    let mut doc = Document {
        name: job.name.clone(),
        sizes: sizes.clone(),
        base_size: sizes[0].clone(),
        meta: job.meta.clone(),
        ..Document::default()
    };
    for p in &job.pieces {
        insert_polygon(&mut doc, &p.id, &p.name, &p.outline)?;
        let add_point = |d: &mut Document, id: String, q: Pt| -> String {
            d.drawing.insert(
                id.clone(),
                Object::Point {
                    operation: PointOp::Absolute {
                        x: q[0].into(),
                        y: q[1].into(),
                    },
                },
            );
            id
        };
        let grain = p.grain.map(|g| {
            [
                add_point(&mut doc, format!("{}.grain0", p.id), g[0]),
                add_point(&mut doc, format!("{}.grain1", p.id), g[1]),
            ]
        });
        let fold = p.fold_line.map(|g| {
            [
                add_point(&mut doc, format!("{}.fold0", p.id), g[0]),
                add_point(&mut doc, format!("{}.fold1", p.id), g[1]),
            ]
        });
        let mut annotations = vec![];
        for (i, m) in p.marks.iter().enumerate() {
            let ids = m
                .points
                .iter()
                .enumerate()
                .map(|(j, q)| add_point(&mut doc, format!("{}.mark{i}.{j}", p.id), *q))
                .collect();
            annotations.push(Annotation::Points {
                mark: m.kind,
                points: ids,
                text: m.text.clone(),
            });
        }
        let piece = doc.pieces.last_mut().context("converted piece missing")?;
        piece.quantity = p.quantity;
        piece.mirror_quantity = p.mirror_quantity;
        piece.grain = grain;
        piece.fold_line = fold;
        piece.material = p.material.clone();
        piece.rotation = p.rotation;
        piece.buffer = p.buffer.into();
        piece.annotations = annotations;
    }
    Ok(doc)
}
fn point(doc: &mut Document, id: &str, x: &str, y: &str) {
    doc.drawing.insert(
        id.into(),
        Object::Point {
            operation: PointOp::Absolute {
                x: x.into(),
                y: y.into(),
            },
        },
    );
}
/// Original front/back quarter-bodice teaching block, S through XL. Not a fit-certified production block.
pub fn bodice() -> Document {
    let mut doc = Document {
        name: "BETA original basic bodice".into(),
        sizes: vec!["S".into(), "M".into(), "L".into(), "XL".into()],
        base_size: "M".into(),
        ..Document::default()
    };
    for (size, chest, waist, length, neck, shoulder, arm) in [
        ("S", 880., 680., 400., 350., 115., 190.),
        ("M", 920., 720., 410., 360., 120., 195.),
        ("L", 960., 760., 420., 370., 125., 200.),
        ("XL", 1000., 800., 430., 380., 130., 205.),
    ] {
        doc.measurements.insert(
            size.into(),
            BTreeMap::from([
                ("chest".into(), chest),
                ("waist".into(), waist),
                ("back_length".into(), length),
                ("neck".into(), neck),
                ("shoulder".into(), shoulder),
                ("arm_depth".into(), arm),
            ]),
        );
    }
    doc.variables = BTreeMap::from([
        ("ease".into(), 80.0.into()),
        ("width".into(), "(m.chest+v.ease)/4".into()),
        ("waist_width".into(), "(m.waist+v.ease)/4".into()),
        ("neck_width".into(), "m.neck/6".into()),
    ]);
    for (id, neck_depth) in [("front", "v.neck_width+10"), ("back", "20")] {
        let n = |s: &str| format!("{id}.{s}");
        for (s, x, y) in [
            ("A", "0", "0"),
            ("B", "v.waist_width", "0"),
            ("C", "v.width", "m.back_length-m.arm_depth"),
            ("D", "v.neck_width+m.shoulder", "m.back_length-25"),
            ("E", "v.neck_width", "m.back_length"),
            ("F", "0", &format!("m.back_length-({neck_depth})")),
            ("arm1", "v.width-30", "m.back_length-m.arm_depth+20"),
            ("arm2", "v.neck_width+m.shoulder-15", "m.back_length-65"),
            ("neck1", "v.neck_width*0.45", "m.back_length"),
            ("neck2", "0", &format!("m.back_length-({neck_depth})*0.55")),
            ("grain0", "v.width/2", "80"),
            ("grain1", "v.width/2", "m.back_length-100"),
        ] {
            point(&mut doc, &n(s), x, y);
        }
        let objects = [
            (
                "waist",
                Object::Line {
                    from: n("A"),
                    to: n("B"),
                },
            ),
            (
                "side",
                Object::Line {
                    from: n("B"),
                    to: n("C"),
                },
            ),
            (
                "arm",
                Object::Cubic {
                    points: [n("C"), n("arm1"), n("arm2"), n("D")],
                },
            ),
            (
                "shoulder",
                Object::Line {
                    from: n("D"),
                    to: n("E"),
                },
            ),
            (
                "neck",
                Object::Cubic {
                    points: [n("E"), n("neck1"), n("neck2"), n("F")],
                },
            ),
            (
                "center",
                Object::Line {
                    from: n("F"),
                    to: n("A"),
                },
            ),
        ];
        let mut edges = vec![];
        for (s, o) in objects {
            let obj = n(s);
            doc.drawing.insert(obj.clone(), o);
            edges.push(Edge {
                object: obj,
                reversed: false,
                allowance: 10.0.into(),
                corner: Corner::Miter,
            });
        }
        doc.pieces.push(PatternPiece {
            internal_paths: vec![],
            anchors: BTreeMap::new(),
            grain_angle: None,
            label_point: None,
            id: id.into(),
            name: format!("Bodice {id}"),
            edges,
            quantity: 1,
            mirror_quantity: 1,
            material: Some("fabric".into()),
            grain: Some([n("grain0"), n("grain1")]),
            fold_line: None,
            rotation: Default::default(),
            buffer: 0.0.into(),
            annotations: vec![
                Annotation::Notch {
                    edge: 1,
                    distance: "length(front.side)/2".replace("front", id).as_str().into(),
                    depth: 3.0.into(),
                },
                Annotation::Points {
                    mark: MarkKind::Text,
                    points: vec![n("grain0")],
                    text: Some(format!("BETA {id} - cut one pair")),
                },
            ],
            grade_points: BTreeMap::new(),
        });
    }
    doc.meta.insert(
        "drafting".into(),
        serde_json::json!(
            "Original BETA teaching formulas; not a fit-certified production block. See README."
        ),
    );
    doc
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{evaluate, exchange, geometry::*};
    #[test]
    fn bodice_sizes_marker_and_dxf() {
        let d = bodice();
        let mut previous = 0.;
        for s in &d.sizes {
            let pieces = evaluate::grade(&d, s, 0.1).unwrap();
            let a: f64 = pieces.iter().map(|p| area(&p.piece.outline)).sum();
            assert!(a > previous);
            previous = a;
            let bb = bbox(&pieces[0].seam).unwrap();
            let expected = match s.as_str() {
                "S" => 240.,
                "M" => 250.,
                "L" => 260.,
                _ => 270.,
            };
            assert_eq!(bb[2] - bb[0], expected);
            let job = exchange::to_job(
                &d,
                beta_marker_core::Fabric {
                    width: 1500.,
                    ..Default::default()
                },
                BTreeMap::from([(s.clone(), 2)]),
                0.1,
            )
            .unwrap();
            assert!(beta_marker_core::check_job(&job).is_empty());
        }
        let dxf = exchange::export_dxf(&d, &d.sizes, 0.1).unwrap();
        let report = beta_marker_dxf::import_dxf(dxf.as_bytes(), &Default::default()).unwrap();
        assert_eq!(report.pieces.len(), 8);
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
        let original: Vec<_> = d
            .sizes
            .iter()
            .flat_map(|s| {
                evaluate::grade(&d, s, 0.1)
                    .unwrap()
                    .into_iter()
                    .map(|p| p.piece)
            })
            .collect();
        for p in &original {
            let q = report.pieces.iter().find(|q| q.id == p.id).unwrap();
            assert!((area(&p.outline) - area(&q.outline)).abs() < 0.001);
            assert_eq!(p.outline.len(), q.outline.len());
            for (a, b) in p.outline.iter().zip(&q.outline) {
                assert!(distance(*a, *b) < 1e-7);
            }
            assert_eq!(p.quantity, q.quantity);
            assert_eq!(p.mirror_quantity, q.mirror_quantity);
            assert_eq!(p.size, q.size);
            assert_eq!(p.material, q.material);
            assert_eq!(p.rotation, q.rotation);
            if let (Some(a), Some(b)) = (p.grain, q.grain) {
                for (a, b) in a.into_iter().zip(b) {
                    assert!(distance(a, b) < 1e-7);
                }
            } else {
                assert_eq!(p.grain, q.grain);
            }
            assert_eq!(p.marks.len(), q.marks.len());
            for (a, b) in p.marks.iter().zip(&q.marks) {
                assert_eq!(a.kind, b.kind);
                assert_eq!(a.text, b.text);
                assert_eq!(a.points.len(), b.points.len());
                for (a, b) in a.points.iter().zip(&b.points) {
                    assert!(distance(*a, *b) < 1e-7);
                }
            }
        }
        let table = exchange::import_rule_table(&dxf).unwrap();
        assert_eq!(table.sizes, d.sizes);
        assert_eq!(table.points.len(), 4);
    }
    #[test]
    fn json_roundtrip() {
        let d = bodice();
        let s = serde_json::to_string(&d).unwrap();
        assert_eq!(d, serde_json::from_str::<Document>(&s).unwrap());
    }
}
