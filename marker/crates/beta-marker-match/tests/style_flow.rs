//! Original full-flow regression: no factory files or external services.
use beta_marker_core::{
    self as marker, Fabric, Layout, MatchAxes, MatchRule, PointRef, Repeat, Spacing,
    style::{Face, LaySettings, OverlayOptions},
};
use beta_pattern_core::{
    blocks::{self, BlockKind, Create},
    evaluate, exchange, print,
    style::{self, Operation},
};
use serde_json::json;
use std::collections::BTreeMap;

#[test]
fn blank_piece_edits_are_atomic_and_exportable() {
    let blank = blocks::create(&Create::default()).unwrap();
    let mut operations = vec![];
    for (id, x, y) in [("A", 0, 0), ("B", 100, 0), ("C", 100, 150), ("D", 0, 150)] {
        operations.push(json!({"op":"put_object","id":id,"object":{"kind":"point","operation":{"op":"absolute","x":x,"y":y}}}));
    }
    for (id, a, b) in [
        ("bottom", "A", "B"),
        ("right", "B", "C"),
        ("top", "C", "D"),
        ("left", "D", "A"),
    ] {
        operations
            .push(json!({"op":"put_object","id":id,"object":{"kind":"line","from":a,"to":b}}));
    }
    operations.push(json!({"op":"put_piece","piece":{"id":"panel","name":"Blank sheet panel","edges":[{"object":"bottom"},{"object":"right"},{"object":"top"},{"object":"left"}],"grain":["A","D"],"material":"shell"}}));
    let operations: Vec<Operation> = operations
        .into_iter()
        .map(|v| serde_json::from_value(v).unwrap())
        .collect();
    let doc = style::edit(&blank, &operations).unwrap();
    assert!(blank.pieces.is_empty());
    let dx = exchange::export_dxf(&doc, &doc.sizes, 0.1).unwrap();
    let imported = beta_marker_dxf::import_dxf(dx.as_bytes(), &Default::default()).unwrap();
    assert_eq!(imported.pieces.len(), 5);
    assert!(
        print::tiled_pdf(&doc, &doc.sizes, &Default::default())
            .unwrap()
            .starts_with(b"%PDF")
    );
}

#[test]
fn tshirt_edit_grade_fold_match_nest_30_seconds_validate_and_export() {
    let mut chart = blocks::size_chart("indian_adult").unwrap();
    chart.sizes.retain(|s| s != "XXL");
    chart.measurements.remove("XXL");
    let doc = blocks::create(&Create {
        block: Some(BlockKind::TShirt),
        chart: Some(chart),
        ..Default::default()
    })
    .unwrap();
    let mut operations = vec![];
    for id in ["front.arm1", "back.arm1"] {
        let p = evaluate::Evaluator::new(&doc, "M", 0.1)
            .unwrap()
            .point(id)
            .unwrap();
        operations.push(Operation::MovePoint {
            id: id.into(),
            size: "M".into(),
            target: [p[0] + 2., p[1] + 1.],
            preserve_formulas: true,
        });
    }
    operations.push(serde_json::from_value(json!({"op":"add_annotation","piece":"front","annotation":{"kind":"shaped_notch","edge":1,"distance":100,"depth":3,"width":2,"shape":"v"}})).unwrap());
    operations.push(
        serde_json::from_value(
            json!({"op":"set_allowance","piece":"front","edges":[0,9],"width":25,"corner":"miter"}),
        )
        .unwrap(),
    );
    for (id, side) in [("arm_left", "left"), ("arm_right", "right")] {
        operations.push(
            serde_json::from_value(
                json!({"op":"offset_path","id":id,"source":"front.arm","distance":3,"side":side}),
            )
            .unwrap(),
        );
    }
    let doc = style::edit(&doc, &operations).unwrap();
    let mut evaluator = evaluate::Evaluator::new(&doc, "M", 0.1).unwrap();
    let path = evaluator.path("front.side").unwrap();
    let click = beta_pattern_core::geometry::at_distance(&path, 100.)
        .unwrap()
        .0;
    let hits = style::hit_test(&doc, "M", click, 2.).unwrap();
    let edge = hits
        .iter()
        .find(|h| h.piece_id.as_deref() == Some("front") && h.edge == Some(1))
        .unwrap();
    assert!((edge.along_mm.unwrap() - 100.).abs() < 0.001);
    let checks = blocks::seam_reports(&doc).unwrap();
    assert!(checks.iter().flat_map(|c| &c.rows).all(|r| r.pass));
    let settings = style::JobSettings {
        fabric: Fabric {
            width: 750.,
            layout: Layout::Folded,
            nap: marker::Nap::OneWay,
            repeat: Some(Repeat {
                period_x: Some(10.),
                tolerance_mm: 0.1,
                ..Default::default()
            }),
            ..Default::default()
        },
        spacing: Spacing {
            piece_gap: 3.,
            edge_gap: 5.,
        },
        ratio: BTreeMap::from([
            ("S".into(), 1),
            ("M".into(), 1),
            ("L".into(), 1),
            ("XL".into(), 1),
        ]),
        lay: Some(LaySettings {
            face: Face::Up,
            plies: 20,
            end_allowance_mm: 15.,
        }),
        repeat_bands: vec![],
        piece_settings: BTreeMap::new(),
        material: None,
        matches: vec![MatchRule {
            a: PointRef {
                piece_id: "front".into(),
                point_id: "side_hem".into(),
            },
            b: Some(PointRef {
                piece_id: "back".into(),
                point_id: "side_hem".into(),
            }),
            axes: MatchAxes::X,
            offset: [0., 0.],
        }],
    };
    let job = style::to_job(&doc, &settings).unwrap();
    assert_eq!(job.match_rules.len(), 4);
    let started = std::time::Instant::now();
    let result = beta_marker_match::nest_with_rules(
        &job,
        &beta_marker_nest::NestOptions {
            time_limit_s: 30.,
            seed: Some(42),
            workers: Some(1),
            ..Default::default()
        },
    )
    .unwrap();
    let validation = marker::validate(&job, &result);
    assert!(validation.ok, "{:?}", validation.errors);
    assert!(beta_marker_match::match_errors(&job, &result).is_empty());
    let overlay = marker::style::overlay(&job, &result, &OverlayOptions::default()).unwrap();
    assert!(!overlay.stripes.is_empty());
    assert_eq!(overlay.matches, validation.matches);
    assert_eq!(overlay.matches.len(), 4);
    assert!(
        overlay
            .matches
            .iter()
            .all(|m| m.pass && m.error_mm.unwrap() <= 0.1 + 1e-9 && m.offset_mm[1].is_none())
    );
    assert!((overlay.spread_length_mm - result.length - 30.).abs() < 0.01);
    for line in &overlay.stripes {
        let poly = &overlay.outlines[line.placement].points;
        assert!(marker::style::contains(line.points[0], poly));
        assert!(marker::style::contains(line.points[1], poly));
    }
    let options = beta_marker_export::ExportOptions::default();
    let pdf = beta_marker_export::export_with_stripe_guides("pdf", &job, &result, &options)
        .unwrap()
        .0;
    let plt = beta_marker_export::export_with_stripe_guides("plt", &job, &result, &options)
        .unwrap()
        .0;
    let dxf = exchange::export_dxf(&doc, &doc.sizes, 0.1).unwrap();
    assert!(pdf.starts_with(b"%PDF-1.4") && pdf.ends_with(b"%%EOF\n"));
    let imported = beta_marker_dxf::import_dxf(dxf.as_bytes(), &Default::default()).unwrap();
    assert_eq!(imported.pieces.len(), 16);
    for piece in imported.pieces {
        beta_pattern_core::geometry::validate_polygon(&piece.outline).unwrap();
    }
    let recovered = beta_marker_dxf::import_hpgl(
        &plt,
        &beta_marker_dxf::HpglOptions {
            width: Some(750.),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(marker::validate(&recovered.job, &recovered.result).ok);
    eprintln!(
        "Style E2E: sizes=4 pieces=16 placements={} length_mm={:.3} stripe_segments={} wall_s={:.3}",
        result.placements.len(),
        result.length,
        overlay.stripes.len(),
        started.elapsed().as_secs_f64()
    );
    if let Ok(root) = std::env::var("BETA_STYLE_EVIDENCE_DIR") {
        let root = std::path::PathBuf::from(root);
        std::fs::create_dir_all(&root).unwrap();
        for (file, data) in [
            ("tshirt.doc.json", style::save(&doc).unwrap()),
            ("job.json", serde_json::to_string_pretty(&job).unwrap()),
            (
                "result.json",
                serde_json::to_string_pretty(&result).unwrap(),
            ),
            (
                "validation.json",
                serde_json::to_string_pretty(&validation).unwrap(),
            ),
            ("seams.json", serde_json::to_string_pretty(&checks).unwrap()),
            ("overlay.json", serde_json::to_string(&overlay).unwrap()),
            ("tshirt.dxf", dxf),
        ] {
            std::fs::write(root.join(file), data).unwrap();
        }
        std::fs::write(root.join("marker.pdf"), pdf).unwrap();
        std::fs::write(root.join("marker.plt"), plt).unwrap();
        std::fs::write(
            root.join("marker.svg"),
            beta_marker_export::export_with_stripe_guides("svg", &job, &result, &options)
                .unwrap()
                .0,
        )
        .unwrap();
    }
}
