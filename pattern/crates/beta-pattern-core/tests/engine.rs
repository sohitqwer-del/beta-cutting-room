//! End-to-end contracts across drafting, grading, annotations and exchange.
use beta_marker_core::{Fabric, MarkKind};
use beta_pattern_core::{
    evaluate::{self, Evaluator},
    exchange,
    geometry::*,
    starters, tools, *,
};
use std::collections::BTreeMap;

fn factory() -> Document {
    let mut d = Document {
        sizes: vec!["S".into(), "M".into(), "L".into()],
        base_size: "M".into(),
        ..Document::default()
    };
    tools::insert_polygon(
        &mut d,
        "panel",
        "Panel",
        &[[0., 0.], [50., 0.], [100., 0.], [100., 100.], [0., 100.]],
    )
    .unwrap();
    d.rules.insert(
        "left".into(),
        GradeRule {
            mode: GradeMode::Relative,
            deltas: BTreeMap::from([("S".into(), [-2., -3.]), ("L".into(), [2., 3.])]),
        },
    );
    d.rules.insert(
        "right".into(),
        GradeRule {
            mode: GradeMode::Incremental,
            deltas: BTreeMap::from([("M".into(), [4., 5.]), ("L".into(), [4., 5.])]),
        },
    );
    d.pieces[0].grade_points = BTreeMap::from([
        ("panel.p0".into(), "left".into()),
        ("panel.p2".into(), "right".into()),
        ("panel.p3".into(), "right".into()),
        ("panel.p4".into(), "left".into()),
    ]);
    d.pieces[0].grain = Some(["panel.p0".into(), "panel.p4".into()]);
    d.pieces[0].annotations = vec![
        Annotation::Notch {
            edge: 0,
            distance: 20.0.into(),
            depth: 3.0.into(),
        },
        Annotation::Button {
            point: "panel.p1".into(),
            diameter: 12.0.into(),
        },
    ];
    d
}
#[test]
fn factory_grades_base_lower_upper_and_nest() {
    let d = factory();
    let m = evaluate::grade(&d, "M", 0.01).unwrap();
    assert_eq!(m[0].seam[1], [50., 0.]);
    let l = evaluate::grade(&d, "L", 0.01).unwrap();
    assert_eq!(l[0].seam[0], [2., 3.]);
    assert_eq!(l[0].seam[1], [53., 4.]);
    assert_eq!(l[0].seam[2], [104., 5.]);
    let s = evaluate::grade(&d, "S", 0.01).unwrap();
    assert_eq!(s[0].seam[1], [47., -4.]);
    assert_eq!(l[0].piece.marks[1].kind, MarkKind::Drill);
    assert_eq!(l[0].piece.marks[1].points[0], [53., 4.]);
    let n = evaluate::nest(&d, &d.sizes, "panel.p0", 0.01).unwrap();
    for pieces in n.sizes.values() {
        assert_eq!(pieces[0].points["panel.p0"], [0., 0.]);
        assert_eq!(pieces[0].seam[0], [0., 0.]);
        assert_eq!(pieces[0].piece.grain.unwrap()[0], [0., 0.]);
    }
    let dx = exchange::export_dxf(&d, &d.sizes, 0.01).unwrap();
    let table = exchange::import_rule_table(&dx).unwrap();
    assert_eq!(table.rules, d.rules);
    assert_eq!(table.attachments["panel"], d.pieces[0].grade_points);
    assert_eq!(table.points["L"]["panel"]["panel.p1"], [53., 4.]);
    let round = beta_marker_dxf::import_dxf(dx.as_bytes(), &Default::default()).unwrap();
    assert_eq!(round.pieces.len(), 3);
    assert!(round.warnings.is_empty());
}
#[test]
fn allowance_edits_and_history_are_reversible() {
    let d = factory();
    let edit = tools::set_allowance(&d, "panel", 5.0.into(), Corner::Bevel).unwrap();
    assert_eq!(
        evaluate::grade(&d, "M", 0.1).unwrap()[0]
            .piece
            .outline
            .len(),
        5
    );
    let g = evaluate::grade(&edit, "M", 0.1).unwrap();
    assert!(area(&g[0].piece.outline) > 10000.);
    let removed = tools::set_allowance(&edit, "panel", 0.0.into(), Corner::Miter).unwrap();
    assert_eq!(
        evaluate::grade(&removed, "M", 0.1).unwrap()[0].seam,
        evaluate::grade(&d, "M", 0.1).unwrap()[0].seam
    );
    let mut h = history::History::new(d.clone());
    h.apply(edit);
    assert!(h.undo());
    assert_eq!(h.current(), &d);
}
#[test]
fn measurement_and_geometry_dependency_errors_have_names() {
    let mut d = starters::bodice();
    d.measurements.get_mut("S").unwrap().remove("chest");
    let err = format!("{:#}", evaluate::grade(&d, "S", 0.1).unwrap_err());
    assert!(
        err.contains("front") && err.contains("chest") && err.contains("S"),
        "{err}"
    );
    let mut d = Document::default();
    d.drawing.insert(
        "A".into(),
        Object::Point {
            operation: PointOp::Midpoint {
                a: "missing".into(),
                b: "missing".into(),
            },
        },
    );
    let err = format!(
        "{:#}",
        Evaluator::new(&d, "M", 0.1)
            .unwrap()
            .point("A")
            .unwrap_err()
    );
    assert!(err.contains("A") && err.contains("missing"));
}
#[test]
fn curve_point_intersection_and_tangents() {
    let mut d = Document::default();
    for (id, p) in [("O", [0., 0.]), ("A", [-20., 0.]), ("B", [20., 0.])] {
        d.drawing.insert(
            id.into(),
            Object::Point {
                operation: PointOp::Absolute {
                    x: p[0].into(),
                    y: p[1].into(),
                },
            },
        );
    }
    d.drawing.insert(
        "circle".into(),
        Object::Arc {
            center: "O".into(),
            radius: 10.0.into(),
            start: 0.0.into(),
            sweep: 360.0.into(),
        },
    );
    d.drawing.insert(
        "line".into(),
        Object::Line {
            from: "A".into(),
            to: "B".into(),
        },
    );
    d.drawing.insert(
        "crossing".into(),
        Object::Point {
            operation: PointOp::Intersection {
                a: "line".into(),
                b: "circle".into(),
                index: 0,
            },
        },
    );
    let mut e = Evaluator::new(&d, "M", 0.00001).unwrap();
    assert!((e.point("crossing").unwrap()[0] + 10.).abs() < 0.00002);
    let p = flatten_spline(&[[0., 0.], [10., 0.]], &[[10., 10.], [10., -10.]], 0.001).unwrap();
    assert!((bbox(&p).unwrap()[3] - 2.5).abs() < 1e-9);
    assert_eq!(p.first(), Some(&[0., 0.]));
    assert_eq!(p.last(), Some(&[10., 0.]));
    let p = flatten_cubic([[0., 0.], [0., 10.], [10., 10.], [10., 0.]], 0.0001).unwrap();
    assert!((length(&p) - 20.).abs() < 0.0002);
    assert_eq!(bbox(&p).unwrap(), [0., 0., 10., 7.5]);
}
#[test]
fn bad_jobs_and_invalid_tool_inputs_fail() {
    let d = factory();
    assert!(
        exchange::to_job(
            &d,
            Fabric {
                width: f64::INFINITY,
                ..Default::default()
            },
            BTreeMap::from([("M".into(), 1)]),
            0.1
        )
        .is_err()
    );
    assert!(
        exchange::to_job(
            &d,
            Fabric {
                width: 1500.,
                ..Default::default()
            },
            BTreeMap::from([("unknown".into(), 1)]),
            0.1
        )
        .is_err()
    );
    assert!(exchange::export_dxf(&d, &["M".into(), "M".into()], 0.1).is_err());
    assert!(tools::mirror(&[[1., 2.]], [0., 0.], [0., 0.]).is_err());
    assert!(
        tools::dart(
            &[[0., 0.], [10., 0.], [10., 10.], [0., 10.]],
            99,
            1.,
            1.,
            [5., 5.]
        )
        .is_err()
    );
    let mut d = d.clone();
    d.pieces[0].edges.pop();
    assert!(evaluate::grade(&d, "M", 0.1).is_err());
}

#[test]
fn fixed_starters_preserve_the_source_cut_geometry_and_marks() {
    for (native, source) in [
        (
            include_str!("../starters/freesewing-teagan-M.doc.json"),
            include_str!("../../../../marker/testdata/open/freesewing/teagan-M.job.json"),
        ),
        (
            include_str!("../starters/freesewing-titan-M.doc.json"),
            include_str!("../../../../marker/testdata/open/freesewing/titan-M.job.json"),
        ),
    ] {
        let d: Document = serde_json::from_str(native).unwrap();
        let source: beta_marker_core::Job = serde_json::from_str(source).unwrap();
        let graded = evaluate::grade(&d, "M", 0.1).unwrap();
        assert_eq!(graded.len(), source.pieces.len());
        for (p, q) in graded.iter().zip(&source.pieces) {
            assert_eq!(p.piece.outline, q.outline);
            assert_eq!(p.piece.grain, q.grain);
            assert_eq!(p.piece.marks, q.marks);
            assert_eq!(p.piece.quantity, q.quantity);
            assert_eq!(p.piece.mirror_quantity, q.mirror_quantity);
        }
        let job = exchange::to_job(
            &d,
            Fabric {
                width: 1500.,
                ..Default::default()
            },
            BTreeMap::from([("M".into(), 1)]),
            0.1,
        )
        .unwrap();
        assert!(beta_marker_core::check_job(&job).is_empty());
    }
}

#[test]
fn shared_pattern_evaluation_matches_grade_and_keeps_piece_errors_local() {
    let d = factory();
    for size in &d.sizes {
        let result = evaluate::evaluate_pattern(&d, size, 0.1).unwrap();
        let full = evaluate::grade(&d, size, 0.1).unwrap();
        for (compact, full) in result.pieces.iter().zip(full) {
            assert_eq!(compact.piece, full.piece);
            assert_eq!(compact.seam, full.seam);
            for (id, point) in &compact.points {
                assert_eq!(point, &full.points[id]);
            }
        }
        assert_eq!(result.objects.len(), d.drawing.len());
        assert!(result.piece_errors.is_empty());
    }
    let mut d = starters::bodice();
    d.pieces[0].edges.clear();
    let result = evaluate::evaluate_pattern(&d, "M", 0.1).unwrap();
    assert_eq!(result.pieces.len(), 1);
    assert!(result.piece_errors["front"].contains("no edges"));
    assert!(result.objects.iter().all(|o| o.error.is_none()));
}

#[test]
fn reusable_rule_library_rejects_incomplete_or_incompatible_sizes() {
    let d = factory();
    let lib = grading::rule_library(&d, "Factory standard");
    let lib = serde_json::from_str(&serde_json::to_string(&lib).unwrap()).unwrap();
    assert_eq!(grading::apply_library(&d, &lib).unwrap(), d);
    let mut bad = lib.clone();
    bad.sizes.reverse();
    assert!(grading::apply_library(&d, &bad).is_err());
    bad = lib;
    bad.rules.get_mut("right").unwrap().deltas.remove("L");
    assert!(grading::apply_library(&d, &bad).is_err());
    assert!(d.rules["right"].deltas.contains_key("L"));
}
