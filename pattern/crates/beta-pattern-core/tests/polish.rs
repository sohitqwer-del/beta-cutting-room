use beta_pattern_core::{
    blocks::{self, Create},
    evaluate::{self, Evaluator},
    geometry::*,
    style::{self, Operation},
    *,
};
use serde_json::json;

fn edit(doc: &Document, ops: serde_json::Value) -> Document {
    style::edit(doc, &serde_json::from_value::<Vec<Operation>>(ops).unwrap()).unwrap()
}
fn panel() -> Document {
    let mut doc = blocks::create(&Create::default()).unwrap();
    for (i, size) in doc.sizes.iter().enumerate() {
        doc.measurements
            .get_mut(size)
            .unwrap()
            .insert("w".into(), 100. + i as f64 * 10.);
        doc.measurements
            .get_mut(size)
            .unwrap()
            .insert("h".into(), 200. + i as f64 * 20.);
    }
    tools::insert_polygon(
        &mut doc,
        "panel",
        "Panel",
        &[[0., 0.], [100., 0.], [100., 200.], [0., 200.]],
    )
    .unwrap();
    edit(
        &doc,
        json!([
            {"op":"put_object","id":"panel.p1","replace":true,"object":{"kind":"point","operation":{"op":"absolute","x":"m.w","y":0}}},
            {"op":"put_object","id":"panel.p2","replace":true,"object":{"kind":"point","operation":{"op":"absolute","x":"m.w","y":"m.h"}}},
            {"op":"put_object","id":"panel.p3","replace":true,"object":{"kind":"point","operation":{"op":"absolute","x":0,"y":"m.h"}}},
            {"op":"put_object","id":"cut0","object":{"kind":"point","operation":{"op":"absolute","x":"m.w/2","y":0}}},
            {"op":"put_object","id":"cut1","object":{"kind":"point","operation":{"op":"absolute","x":"m.w/2","y":"m.h"}}},
            {"op":"put_object","id":"tip","object":{"kind":"point","operation":{"op":"absolute","x":"m.w/2","y":"m.h/3"}}}
        ]),
    )
}
#[test]
fn blocks_are_separate_in_every_size_and_after_chart_growth() {
    for block in blocks::catalog() {
        let mut doc = blocks::create(&Create {
            block: Some(block.id),
            ..Default::default()
        })
        .unwrap();
        for round in 0..2 {
            for size in &doc.sizes {
                let pieces = evaluate::grade(&doc, size, 0.1).unwrap();
                for pair in pieces.windows(2) {
                    let a = bbox(&pair[0].piece.outline).unwrap();
                    let b = bbox(&pair[1].piece.outline).unwrap();
                    assert!(b[0] - a[2] >= 49.999, "{:?} {size}: {a:?} {b:?}", block.id);
                }
                for p in pieces {
                    if let Some(g) = p.piece.grain {
                        let id = p.piece.id.strip_suffix(&format!("-{size}")).unwrap();
                        let horizontal = id == "waistband"
                            || block.id == blocks::BlockKind::Shirt
                                && matches!(id, "collar" | "stand" | "cuff");
                        if horizontal {
                            assert!((g[0][1] - g[1][1]).abs() < 1e-8);
                        } else if id == "neck_facing" {
                            assert!(
                                ((g[0][0] - g[1][0]).abs() - (g[0][1] - g[1][1]).abs()).abs()
                                    < 1e-8
                            );
                        } else {
                            assert!((g[0][0] - g[1][0]).abs() < 1e-8);
                        }
                    }
                }
            }
            if round == 0 {
                let mut chart = blocks::size_chart("indian_adult").unwrap();
                for row in chart.measurements.values_mut() {
                    for key in ["chest", "waist", "hip"] {
                        *row.get_mut(key).unwrap() += 60.;
                    }
                }
                doc = style::edit(&doc, &[Operation::SetChart { chart }]).unwrap();
            }
        }
    }
}
#[test]
fn split_join_dart_pleat_regrade_and_reopen_across_s_to_xxl() {
    let mut doc = panel();
    for request in [
        json!({"action":"split","piece":"panel","id":"half","a":"cut0","b":"cut1"}),
        json!({"action":"dart","piece":"panel","id":"darted","edge":1,"center":"m.w/2","width":20,"apex":"tip"}),
        json!({"action":"pleat","piece":"panel","id":"pleated","a":"cut0","b":"cut1","depth":10,"pleat_kind":"box"}),
    ] {
        doc = derived::piece_action(&doc, &request).unwrap();
    }
    doc = edit(
        &doc,
        json!([{"op":"join_pieces","pieces":["half","half_2"],"id":"joined","size":"M"}]),
    );
    for pass in 0..2 {
        for size in &doc.sizes {
            let pieces = evaluate::grade(&doc, size, 0.1).unwrap();
            let w = doc.measurements[size]["w"];
            let h = doc.measurements[size]["h"];
            for (id, expected) in [
                ("half", w * h / 2.),
                ("half_2", w * h / 2.),
                ("joined", w * h),
                ("darted", w * h - 10. * h / 3.),
                ("pleated", (w + 40.) * h),
            ] {
                let p = pieces
                    .iter()
                    .find(|p| p.piece.id == format!("{id}-{size}"))
                    .unwrap();
                assert!((area(&p.seam) - expected).abs() < 1e-6, "{id} {size}");
                validate_polygon(&p.piece.outline).unwrap();
            }
            let p = pieces
                .iter()
                .find(|p| p.piece.id == format!("pleated-{size}"))
                .unwrap();
            assert_eq!(
                p.piece
                    .marks
                    .iter()
                    .filter(|m| m.text.as_deref() == Some("pleat fold"))
                    .count(),
                5
            );
        }
        let mut chart = style::SizeChart {
            name: "changed".into(),
            sizes: doc.sizes.clone(),
            base_size: doc.base_size.clone(),
            measurements: doc.measurements.clone(),
        };
        for row in chart.measurements.values_mut() {
            *row.get_mut("w").unwrap() += 30.;
            *row.get_mut("h").unwrap() += 60.;
        }
        doc = style::edit(&doc, &[Operation::SetChart { chart }]).unwrap();
        if pass == 0 {
            doc = style::load(&style::save(&doc).unwrap()).unwrap();
        }
    }
    assert!(style::edit(&doc, &[Operation::DeletePiece { id: "panel".into() }]).is_err());
    let mut cycle = doc.clone();
    if let Object::DerivedPath { recipe, .. } = cycle.drawing.get_mut("half.boundary").unwrap() {
        recipe.source = "joined".into();
    }
    assert!(style::validate(&cycle).is_err());
}
#[test]
fn path_offsets_hit_parameters_reversed_edges_and_intersections() {
    let doc = edit(
        &panel(),
        json!([
            {"op":"offset_path","id":"left","source":"panel.e0","distance":12,"side":"left"},
            {"op":"offset_path","id":"right","source":"panel.e0","distance":12,"side":"right"},
            {"op":"put_object","id":"arc","object":{"kind":"arc","center":"panel.p0","radius":50,"start":0,"sweep":90}},
            {"op":"offset_path","id":"arc_in","source":"arc","distance":10,"side":"left"},
            {"op":"offset_path","id":"arc_out","source":"arc","distance":10,"side":"right"},
            {"op":"put_object","id":"cut_line","object":{"kind":"line","from":"cut0","to":"cut1"}}
        ]),
    );
    let mut e = Evaluator::new(&doc, "M", 0.01).unwrap();
    for (id, y) in [("left", 12.), ("right", -12.)] {
        assert!(e.path(id).unwrap().iter().all(|p| (p[1] - y).abs() < 1e-10));
    }
    for (id, r) in [("arc_in", 40.), ("arc_out", 60.)] {
        assert!(
            e.path(id)
                .unwrap()
                .iter()
                .all(|p| (p[0].hypot(p[1]) - r).abs() < 1e-10)
        );
    }
    let hit = style::hit_test(&doc, "M", [35., -1.], 2.)
        .unwrap()
        .into_iter()
        .find(|h| h.piece_id.as_deref() == Some("panel") && h.edge == Some(0))
        .unwrap();
    assert!((hit.along_mm.unwrap() - 35.).abs() < 1e-4);
    assert!((hit.parameter.unwrap() - 35. / 110.).abs() < 1e-5);
    assert_eq!(hit.parameter_kind, Some("native"));
    let request = json!({"op":"offset_path","id":"bad","source":"arc","distance":60,"side":"left"});
    assert!(style::edit(&doc, &[serde_json::from_value(request).unwrap()]).is_err());
    let trimmed = edit(
        &doc,
        json!([{"op":"extend_trim","id":"trimmed","source":"panel.e0","target":"cut_line","end":1}]),
    );
    assert_eq!(
        Evaluator::new(&trimmed, "M", 0.1)
            .unwrap()
            .path("trimmed")
            .unwrap(),
        vec![[0., 0.], [55., 0.]]
    );
    assert!(style::edit(&doc,&[serde_json::from_value(json!({"op":"extend_trim","id":"bad","source":"panel.e0","target":"panel.e2","end":1})).unwrap()]).is_err());
}

#[test]
fn cubic_offsets_use_normals_and_hits_return_native_t() {
    let mut doc = Document::default();
    for (id, p) in [
        ("a", [0., 0.]),
        ("b", [0., 100.]),
        ("c", [100., 100.]),
        ("d", [100., 0.]),
    ] {
        doc.drawing.insert(
            id.into(),
            Object::Point {
                operation: PointOp::Absolute {
                    x: p[0].into(),
                    y: p[1].into(),
                },
            },
        );
    }
    doc.drawing.insert(
        "curve".into(),
        Object::Cubic {
            points: ["a".into(), "b".into(), "c".into(), "d".into()],
        },
    );
    let doc = edit(
        &doc,
        json!([
            {"op":"offset_path","id":"outer","source":"curve","distance":10,"side":"left"},
            {"op":"offset_path","id":"inner","source":"curve","distance":10,"side":"right"}
        ]),
    );
    let mut e = Evaluator::new(&doc, "M", 0.01).unwrap();
    for (id, expected) in [("outer", [50., 85.]), ("inner", [50., 65.])] {
        assert!(
            e.path(id)
                .unwrap()
                .iter()
                .any(|p| distance(*p, expected) < 1e-8)
        );
    }
    let hits = style::hit_test(&doc, "M", [50., 80.], 6.).unwrap();
    let hit = hits.iter().find(|h| h.id == "curve").unwrap();
    assert!((hit.parameter.unwrap() - 0.5).abs() < 1e-8);
    assert!(distance(hit.point, [50., 75.]) < 1e-8);
    assert!((hit.distance_mm - 5.).abs() < 1e-8);
    let renamed = editor::rename(&doc, "curve", "armhole").unwrap();
    assert!(style::save(&renamed).is_ok());
    assert!(editor::delete(&renamed, "armhole").is_err());
}

#[test]
fn edge_hits_follow_reversed_and_factory_graded_stitch_paths() {
    let mut doc = panel();
    doc.pieces[0].edges.reverse();
    for edge in &mut doc.pieces[0].edges {
        edge.reversed = true;
    }
    let hit = style::hit_test(&doc, "M", [35., -1.], 2.)
        .unwrap()
        .into_iter()
        .find(|h| h.piece_id.as_deref() == Some("panel") && h.edge == Some(3))
        .unwrap();
    assert!((hit.along_mm.unwrap() - 75.).abs() < 1e-4);
    assert!((hit.parameter.unwrap() - 35. / 110.).abs() < 1e-5); // native direction unchanged
    let deltas = doc
        .sizes
        .iter()
        .enumerate()
        .map(|(i, s)| (s.clone(), [i as f64 * 10. - 10., 0.]))
        .collect();
    doc.rules.insert(
        "grow".into(),
        GradeRule {
            mode: GradeMode::Relative,
            deltas,
        },
    );
    for p in ["panel.p1", "panel.p2"] {
        doc.pieces[0].grade_points.insert(p.into(), "grow".into());
    }
    let doc = derived::piece_action(
        &doc,
        &json!({"action":"split","piece":"panel","id":"split","a":"cut0","b":"cut1"}),
    )
    .unwrap();
    for size in &doc.sizes {
        let g = evaluate::grade(&doc, size, 0.1).unwrap();
        assert!((area(&g[0].seam) - area(&g[1].seam) - area(&g[2].seam)).abs() < 1e-6);
    }
    let hit = style::hit_test(&doc, "XXL", [139., 100.], 2.)
        .unwrap()
        .into_iter()
        .find(|h| h.piece_id.as_deref() == Some("panel") && h.edge == Some(2))
        .unwrap();
    assert_eq!(hit.parameter_kind, Some("edge_length_fraction"));
    assert!((hit.point[0] - 140.).abs() < 1e-8);
}

#[test]
fn folded_handoff_mirrors_shaped_marks_and_deduplicates_axis_drills_in_all_sizes() {
    let mut doc = panel();
    doc.pieces[0].fold_line = Some(["cut0".into(), "cut1".into()]);
    doc.pieces[0].grain = Some(["cut0".into(), "cut1".into()]);
    for (i, shape) in [
        NotchType::Slit,
        NotchType::V,
        NotchType::T,
        NotchType::Castle,
    ]
    .into_iter()
    .enumerate()
    {
        doc.pieces[0].annotations.push(Annotation::ShapedNotch {
            edge: 1,
            distance: (30. + 30. * i as f64).into(),
            depth: 3.0.into(),
            width: 4.0.into(),
            shape,
        });
    }
    doc.pieces[0].annotations.push(Annotation::Points {
        mark: beta_marker_core::MarkKind::Drill,
        points: vec!["tip".into()],
        text: None,
    });
    let settings:style::JobSettings=serde_json::from_value(json!({"fabric":{"width":750,"layout":"folded"},"ratio":{"S":1,"M":1,"L":1,"XL":1,"XXL":1}})).unwrap();
    let saved = style::save(&doc).unwrap();
    let job = style::to_job(&doc, &settings).unwrap();
    for p in &job.pieces {
        assert_eq!(p.marks.len(), 9); // eight shaped notches plus the one axis drill
        let axis = p.fold_line.unwrap();
        for m in &p.marks {
            let twin = tools::mirror(&m.points, axis[0], axis[1]).unwrap();
            assert!(p.marks.iter().any(|other| {
                other.kind == m.kind
                    && other.text == m.text
                    && other.points.len() == twin.len()
                    && twin
                        .iter()
                        .all(|q| other.points.iter().any(|r| distance(*q, *r) < 1e-8))
            }));
        }
    }
    assert_eq!(style::save(&doc).unwrap(), saved);
    let mut bad = doc.clone();
    bad.pieces[0].edges[1].allowance = 10.0.into();
    assert!(style::to_job(&bad, &settings).is_err());
}

#[test]
fn dart_seam_edge_is_stable_after_a_curve() {
    let doc = blocks::create(&Create {
        block: Some(blocks::BlockKind::TShirt),
        ..Default::default()
    })
    .unwrap();
    let dart=derived::piece_action(&doc,&json!({"action":"dart","piece":"front","id":"darted","seam_edge":3,"center":40,"width":10,"apex":"front.grain1"})).unwrap();
    style::validate(&dart).unwrap();
    for size in &doc.sizes {
        let original = evaluate::grade(&doc, size, 0.1).unwrap();
        let result = evaluate::grade(&dart, size, 0.1).unwrap();
        assert!(area(&result.last().unwrap().seam) < area(&original[0].seam));
    }
    assert!(derived::piece_action(&doc,&json!({"action":"dart","piece":"front","id":"bad","seam_edge":2,"center":40,"width":10,"apex":"front.grain1"})).is_err());
}
