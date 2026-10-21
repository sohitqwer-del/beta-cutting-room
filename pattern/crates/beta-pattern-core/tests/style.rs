use beta_pattern_core::{
    Value,
    blocks::{self, BlockKind, Create, Ease},
    evaluate, geometry,
    style::{self, Operation},
};
use serde_json::json;

#[test]
fn folded_handoff_completes_one_sided_cut_marks_but_open_handoff_keeps_them() {
    let doc = blocks::create(&Create {
        block: Some(BlockKind::TShirt),
        ..Default::default()
    })
    .unwrap();
    let mut settings:style::JobSettings=serde_json::from_value(json!({"fabric":{"width":750,"layout":"folded"},"spacing":{"piece_gap":3,"edge_gap":5},"ratio":{"M":1}})).unwrap();
    assert!(style::to_job(&doc, &settings).is_ok());
    let op=serde_json::from_value(json!({"op":"add_annotation","piece":"front","annotation":{"kind":"notch","edge":1,"distance":100,"depth":3}})).unwrap();
    let edited = style::edit(&doc, &[op]).unwrap();
    let folded = style::to_job(&edited, &settings).unwrap();
    let folded_marks = folded
        .pieces
        .iter()
        .find(|p| p.id == "front-M")
        .unwrap()
        .marks
        .iter()
        .filter(|m| m.kind == beta_marker_core::MarkKind::Notch)
        .count();
    assert_eq!(folded_marks, 4);
    assert_eq!(edited.pieces[0].annotations.len(), 3);
    assert_eq!(style::to_job(&edited, &settings).unwrap(), folded);
    settings.fabric.layout = beta_marker_core::Layout::Open;
    let open = style::to_job(&edited, &settings).unwrap();
    assert_eq!(
        open.pieces
            .iter()
            .find(|p| p.id == "front-M")
            .unwrap()
            .marks
            .iter()
            .filter(|m| m.kind == beta_marker_core::MarkKind::Notch)
            .count(),
        3
    );
}

#[test]
fn skirt_accepts_a_chart_with_only_its_required_measurements() {
    let chart=serde_json::from_value(json!({"name":"Buyer skirt","sizes":["M"],"base_size":"M","measurements":{"M":{"waist":800,"hip":1000,"skirt_length":600}}})).unwrap();
    let doc = blocks::create(&Create {
        block: Some(BlockKind::ALineSkirt),
        chart: Some(chart),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(doc.pieces.len(), 3);
    assert!(
        blocks::seam_reports(&doc)
            .unwrap()
            .iter()
            .flat_map(|r| &r.rows)
            .all(|r| r.pass)
    );
}

#[test]
fn seam_notches_reach_the_cut_edge_and_keep_the_requested_depth() {
    let mut doc = beta_pattern_core::Document::default();
    beta_pattern_core::tools::insert_polygon(
        &mut doc,
        "panel",
        "Panel",
        &[[0., 0.], [100., 0.], [100., 100.], [0., 100.]],
    )
    .unwrap();
    for edge in &mut doc.pieces[0].edges {
        edge.allowance = 10.0.into();
    }
    doc.pieces[0]
        .annotations
        .push(beta_pattern_core::Annotation::Notch {
            edge: 0,
            distance: 50.0.into(),
            depth: 5.0.into(),
        });
    let graded = evaluate::grade(&doc, "M", 0.1).unwrap();
    let notch = &graded[0].piece.marks[0];
    assert_eq!(notch.points, vec![[50., -10.], [50., -5.]]);
    assert_eq!(notch.text.as_deref(), Some("slit"));
}

#[test]
fn seven_blocks_have_closed_outlines_and_sewing_matches_in_both_charts() {
    let mut results = vec![];
    for preset in ["indian_adult", "iso_example"] {
        for info in blocks::catalog() {
            let doc = blocks::create(&Create {
                block: Some(info.id),
                chart: Some(blocks::size_chart(preset).unwrap()),
                ..Default::default()
            })
            .unwrap_or_else(|e| panic!("{preset} {:?}: {e:#}", info.id));
            assert_eq!(doc.pieces.len(), info.pieces.len());
            for size in &doc.sizes {
                let graded = evaluate::grade(&doc, size, 0.05).unwrap();
                for g in &graded {
                    geometry::validate_polygon(&g.seam).unwrap();
                    geometry::validate_polygon(&g.piece.outline).unwrap();
                }
            }
            let reports = blocks::seam_reports(&doc).unwrap();
            let max_error = reports
                .iter()
                .flat_map(|r| &r.rows)
                .map(|r| r.error_mm.abs())
                .fold(0., f64::max);
            results.push(json!({"preset":preset,"block":info.id,"sizes":doc.sizes.len(),"pieces_per_size":doc.pieces.len(),"seam_rows":reports.iter().map(|r|r.rows.len()).sum::<usize>(),"max_error_mm":max_error}));
            for report in reports {
                for row in report.rows {
                    assert!(
                        row.pass,
                        "{preset} {:?} {} {}: {} mm",
                        info.id, report.check.name, row.size, row.error_mm
                    );
                }
            }
            assert_eq!(style::load(&style::save(&doc).unwrap()).unwrap(), doc);
        }
    }
    if let Ok(root) = std::env::var("BETA_STYLE_EVIDENCE_DIR") {
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            std::path::Path::new(&root).join("block-matrix.json"),
            serde_json::to_string_pretty(&results).unwrap(),
        )
        .unwrap();
    }
}
#[test]
fn cap_ease_and_measurement_edits_regrade_without_losing_edits() {
    let doc = blocks::create(&Create {
        block: Some(BlockKind::TShirt),
        ease: Ease {
            sleeve_cap: Some(8.),
            ..Default::default()
        },
        ..Default::default()
    })
    .unwrap();
    for report in blocks::seam_reports(&doc).unwrap() {
        assert!(report.rows.iter().all(|r| r.pass));
    }
    let updated_ease = style::edit(
        &doc,
        &[Operation::SetVariable {
            name: "cap_ease".into(),
            value: 20.0.into(),
        }],
    )
    .unwrap();
    for report in blocks::seam_reports(&updated_ease).unwrap() {
        assert!(report.rows.iter().all(|r| r.pass));
        if report.check.name.ends_with("sleeve cap") {
            assert_eq!(report.check.ease_mm, 10.);
        }
    }
    let old = evaluate::Evaluator::new(&doc, "M", 0.1)
        .unwrap()
        .point("front.arm1")
        .unwrap();
    let changed = style::edit(
        &doc,
        &[Operation::MovePoint {
            id: "front.arm1".into(),
            size: "M".into(),
            target: [old[0] + 2., old[1] + 1.],
            preserve_formulas: true,
        }],
    )
    .unwrap();
    assert_ne!(doc, changed);
    assert!(matches!(
        &changed.drawing["front.arm1"],
        beta_pattern_core::Object::Point {
            operation: beta_pattern_core::PointOp::Absolute {
                x: Value::Formula(_),
                ..
            }
        }
    ));
    let mut chart = blocks::size_chart("indian_adult").unwrap();
    chart
        .measurements
        .get_mut("M")
        .unwrap()
        .insert("chest".into(), 1020.);
    let regraded = style::edit(&changed, &[Operation::SetChart { chart }]).unwrap();
    let new = evaluate::Evaluator::new(&regraded, "M", 0.1)
        .unwrap()
        .point("front.arm1")
        .unwrap();
    assert!(new[0] > old[0] + 2.);
}
#[test]
fn invalid_transactions_leave_the_snapshot_unchanged() {
    let doc = blocks::create(&Create {
        block: Some(BlockKind::TShirt),
        ..Default::default()
    })
    .unwrap();
    let source = style::save(&doc).unwrap();
    let asymmetric: Operation = serde_json::from_value(
        json!({"op":"set_allowance","piece":"front","edges":[0],"width":30,"corner":"miter"}),
    )
    .unwrap();
    assert!(
        style::edit(&doc, &[asymmetric])
            .unwrap_err()
            .to_string()
            .contains("not symmetric")
    );
    for request in [
        json!({"op":"delete_object","id":"front.C"}),
        json!({"op":"set_allowance","piece":"front","edges":[99],"width":10,"corner":"bevel"}),
        json!({"op":"set_variable","name":"bad","value":"1/0"}),
        json!({"op":"add_annotation","piece":"front","annotation":{"kind":"notch","edge":1,"distance":999999,"depth":3}}),
    ] {
        let op: Operation = serde_json::from_value(request).unwrap();
        assert!(style::edit(&doc, &[op]).is_err());
        assert_eq!(style::save(&doc).unwrap(), source);
    }
    assert!(style::load(&source.replace("beta-pattern.doc/v1", "beta-pattern.doc/v99")).is_err());
    assert!(style::hit_test(&doc, "M", [0., 0.], -1.).is_err());
    assert!(
        style::hit_test(
            &doc,
            "M",
            evaluate::Evaluator::new(&doc, "M", 0.1)
                .unwrap()
                .point("front.A")
                .unwrap(),
            2.
        )
        .unwrap()
        .iter()
        .any(|h| h.id == "front.A")
    );
    let length = style::measure(
        &doc,
        "M",
        &style::Measurement::Length {
            object: "front.side".into(),
        },
    )
    .unwrap();
    assert!(length.value > 400.);
}
