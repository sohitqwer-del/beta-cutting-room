//! Original synthetic factory-workflow regressions. No supplier pattern data.
use beta_marker_core::{Fabric, MarkKind};
use beta_pattern_core::{evaluate, exchange, geometry::*, production::*, symmetry, tools, *};
use std::collections::BTreeMap;

fn square(id: &str) -> Document {
    let mut doc = Document {
        sizes: vec!["S".into(), "M".into(), "L".into()],
        ..Document::default()
    };
    tools::insert_polygon(
        &mut doc,
        id,
        "Public test panel",
        &[[0., 0.], [100., 0.], [100., 200.], [0., 200.]],
    )
    .unwrap();
    doc
}
fn factory() -> Document {
    let mut d = square("front");
    tools::insert_polygon(
        &mut d,
        "back",
        "Back",
        &[[200., 0.], [300., 0.], [300., 200.], [200., 200.]],
    )
    .unwrap();
    for (id, values) in [
        ("fixed", [[0., 0.]; 3]),
        ("grow", [[-10., 0.], [0., 0.], [20., 0.]]),
    ] {
        d.rules.insert(
            id.into(),
            GradeRule {
                mode: GradeMode::Relative,
                deltas: d.sizes.iter().cloned().zip(values).collect(),
            },
        );
    }
    for i in 0..4 {
        d.pieces[0].grade_points.insert(
            format!("front.p{i}"),
            if i == 1 || i == 2 { "grow" } else { "fixed" }.into(),
        );
    }
    d
}
fn check() -> SeamCheck {
    SeamCheck {
        name: "Hem join".into(),
        a: SeamRun {
            piece: "front".into(),
            edges: vec![0],
            reversed: false,
        },
        b: SeamRun {
            piece: "back".into(),
            edges: vec![0],
            reversed: true,
        },
        ease_mm: 0.,
        tolerance_mm: 1.,
    }
}
#[test]
fn seam_report_measures_factory_rules_not_draft_lengths() {
    let d = factory();
    let report = check_seams(&d, &check(), 0.05).unwrap();
    assert_eq!(
        report.rows.iter().map(|r| r.a_mm).collect::<Vec<_>>(),
        vec![90., 100., 120.]
    );
    assert_eq!(
        report.rows.iter().map(|r| r.pass).collect::<Vec<_>>(),
        vec![false, true, false]
    );
    assert_eq!(report.rows[2].error_mm, 20.);
    assert_eq!(length(&report.rows[2].b_path), 100.);
    assert_eq!(report.rows[2].b_path[0], [0., 0.]);
    assert!((report.rows[2].b_path[1][0] - 100.).abs() < 1e-9);
    assert!(
        seam_csv(&report)
            .contains("\"Hem join\",\"L\",120.000,100.000,20.000,0.000,20.000,1.000,CHECK")
    );
    let mut c = check();
    c.ease_mm = 20.;
    c.tolerance_mm = 0.;
    assert!(check_seams(&d, &c, 0.05).unwrap().rows[2].pass);
}
#[test]
fn seam_chains_wrap_and_reject_disconnected_duplicate_or_invalid_edges() {
    let d = factory();
    let mut c = check();
    c.a.edges = vec![3, 0];
    c.b.edges = vec![3, 0];
    let report = check_seams(&d, &c, 0.05).unwrap();
    assert_eq!(report.rows[1].a_mm, 300.);
    for bad in [vec![], vec![4], vec![0, 2], vec![0, 1, 0]] {
        c.a.edges = bad;
        assert!(check_seams(&d, &c, 0.05).is_err());
    }
    c = check();
    c.tolerance_mm = -1.;
    assert!(check_seams(&d, &c, 0.05).is_err());
    c = check();
    c.ease_mm = f64::NAN;
    assert!(check_seams(&d, &c, 0.05).is_err());
}
#[test]
fn rule_import_rejects_a_library_that_collapses_an_attached_piece() {
    let d = factory();
    let mut library = grading::rule_library(&d, "Invalid width");
    library
        .rules
        .get_mut("grow")
        .unwrap()
        .deltas
        .insert("L".into(), [-100., 0.]);
    assert!(rule_exchange::install(&d, &library, true).is_err());
    assert_eq!(d.rules["grow"].deltas["L"], [20., 0.]);
}
#[test]
fn curved_seams_and_allowances_are_measured_independently() {
    let mut d = square("front");
    d.drawing.insert(
        "curve".into(),
        Object::Spline {
            points: vec!["front.p0".into(), "front.p1".into()],
            tangents: vec![[50.0.into(), (-40.).into()], [50.0.into(), 40.0.into()]],
        },
    );
    d.pieces[0].edges[0].object = "curve".into();
    for e in &mut d.pieces[0].edges {
        e.allowance = 15.0.into();
    }
    let c = SeamCheck {
        a: SeamRun {
            piece: "front".into(),
            edges: vec![0],
            reversed: false,
        },
        b: SeamRun {
            piece: "front".into(),
            edges: vec![2],
            reversed: false,
        },
        ..check()
    };
    let report = check_seams(&d, &c, 0.01).unwrap();
    assert!(report.rows[0].a_mm > 100.);
    assert!(report.rows[0].a_mm < 120.);
    assert_eq!(report.rows[0].b_mm, 100.);
}
fn marked() -> Document {
    let mut d = square("half");
    let p = &mut d.pieces[0];
    p.quantity = 2;
    p.mirror_quantity = 1;
    p.material = Some("cotton".into());
    p.grain = Some(["half.p0".into(), "half.p3".into()]);
    p.label_point = Some("half.p1".into());
    p.anchors.insert("pocket".into(), "half.p2".into());
    p.edges[0].allowance = 10.0.into();
    p.edges[1].allowance = 8.0.into();
    p.edges[2].allowance = 20.0.into();
    p.annotations = vec![
        Annotation::ShapedNotch {
            edge: 1,
            distance: 70.0.into(),
            depth: 3.0.into(),
            width: 4.0.into(),
            shape: NotchType::V,
        },
        Annotation::Points {
            mark: MarkKind::Drill,
            points: vec!["half.p2".into()],
            text: Some("drill".into()),
        },
    ];
    d
}
#[test]
fn mirror_keeps_grain_marks_quantities_and_unequal_allowances() {
    let d = marked();
    let next =
        symmetry::mirror_piece(&d, "half", "left", ["half.p0".into(), "half.p3".into()]).unwrap();
    assert_eq!(next.pieces[0], d.pieces[0]);
    for s in &d.sizes {
        let g = evaluate::grade(&next, s, 0.05).unwrap();
        assert!((area(&g[0].piece.outline) - area(&g[1].piece.outline)).abs() < 1e-6);
        assert_eq!(g[1].piece.quantity, 2);
        assert_eq!(g[1].piece.mirror_quantity, 1);
        assert_eq!(g[0].piece.marks.len(), g[1].piece.marks.len());
        let a = &g[0].piece.marks[0].points;
        let b = &g[1].piece.marks[0].points;
        // V orientation reverses along reflection but shape coordinates match.
        for (a, b) in a.iter().zip(b) {
            assert!((a[0] + b[0]).abs() < 1e-6);
            assert!((a[1] - b[1]).abs() < 1e-6);
        }
    }
}
#[test]
fn mirror_factory_rules_reflect_delta_vectors_and_lengths() {
    let d = factory();
    let next = symmetry::mirror_piece(
        &d,
        "front",
        "mirror",
        ["front.p0".into(), "front.p3".into()],
    )
    .unwrap();
    for s in &d.sizes {
        let g = evaluate::grade(&next, s, 0.05).unwrap();
        for (a, b) in g[0].seam.iter().zip(&g[2].seam) {
            assert!((a[0] + b[0]).abs() < 1e-7);
            assert!((a[1] - b[1]).abs() < 1e-7);
        }
        assert_eq!(length(&g[0].seam_edges[0]), length(&g[2].seam_edges[0]));
    }
}
#[test]
fn unfold_keeps_half_details_and_remains_linked_to_measurements() {
    let mut d = marked();
    for (s, w) in [("S", 90.), ("M", 100.), ("L", 120.)] {
        d.measurements
            .insert(s.into(), BTreeMap::from([("width".into(), w)]));
    }
    for point in ["half.p1", "half.p2"] {
        if let Object::Point {
            operation: PointOp::Absolute { x, .. },
        } = d.drawing.get_mut(point).unwrap()
        {
            *x = "m.width".into();
        }
    }
    let next = symmetry::unfold_piece(&d, "half", "full", 3).unwrap();
    assert_eq!(next.pieces[1].edges.len(), 6);
    for s in &d.sizes {
        let g = evaluate::grade(&next, s, 0.05).unwrap();
        assert!((area(&g[1].seam) - 2. * area(&g[0].seam)).abs() < 1e-6);
        assert_eq!(g[1].piece.fold_line, Some([[0., 200.], [0., 0.]]));
        let notches: Vec<_> = g[1]
            .piece
            .marks
            .iter()
            .filter(|m| m.kind == MarkKind::Notch)
            .collect();
        assert_eq!(notches.len(), 2);
        assert!((notches[0].points[1][1] - notches[1].points[1][1]).abs() < 1e-6);
        assert!((notches[0].points[1][0] + notches[1].points[1][0]).abs() < 1e-6);
    }
}
#[test]
fn unfold_rejects_nonzero_fold_allowance_rules_and_collisions_atomically() {
    let d = marked();
    let before = d.clone();
    assert!(symmetry::unfold_piece(&d, "half", "full", 0).is_err());
    assert!(symmetry::unfold_piece(&d, "half", "half", 3).is_err());
    assert!(symmetry::unfold_piece(&d, "half", "full", 99).is_err());
    let factory = factory();
    assert!(symmetry::unfold_piece(&factory, "front", "full", 3).is_err());
    assert_eq!(d, before);
    let mut collision = d;
    collision.pieces[0]
        .anchors
        .insert("mirrored.pocket".into(), "half.p1".into());
    assert!(symmetry::unfold_piece(&collision, "half", "full", 3).is_err());
}
#[test]
fn unfolding_does_not_duplicate_a_drill_or_guide_on_the_fold() {
    let mut d = square("half");
    d.pieces[0].annotations.push(Annotation::Points {
        mark: MarkKind::Drill,
        points: vec!["half.p0".into()],
        text: None,
    });
    let fold = d.pieces[0].edges[3].clone();
    d.pieces[0].internal_paths.push(InternalPath {
        name: "centre".into(),
        edges: vec![fold],
        closed: false,
    });
    let next = symmetry::unfold_piece(&d, "half", "full", 3).unwrap();
    assert_eq!(next.pieces[1].annotations.len(), 1);
    assert_eq!(next.pieces[1].internal_paths.len(), 1);
    let g = evaluate::grade(&next, "M", 0.1).unwrap();
    assert_eq!(
        g[1].piece
            .marks
            .iter()
            .filter(|m| m.kind == MarkKind::Drill)
            .count(),
        1
    );
}
#[test]
fn unfolded_crossing_guides_mark_each_half_once_and_skip_removed_fold_edges() {
    let mut d = square("half");
    for (id, x) in [("guide_start", -200.), ("guide_end", 200.)] {
        d.drawing.insert(
            id.into(),
            Object::Point {
                operation: PointOp::Absolute {
                    x: x.into(),
                    y: 100.0.into(),
                },
            },
        );
    }
    d.drawing.insert(
        "guide".into(),
        Object::Line {
            from: "guide_start".into(),
            to: "guide_end".into(),
        },
    );
    d.pieces[0].annotations.push(Annotation::SeamIntersections {
        path: "guide".into(),
        edges: None,
        depth: 3.0.into(),
        width: 4.0.into(),
        shape: NotchType::V,
    });
    let next = symmetry::unfold_piece(&d, "half", "full", 3).unwrap();
    let g = evaluate::grade(&next, "M", 0.05).unwrap();
    let marks: Vec<_> = g[1]
        .piece
        .marks
        .iter()
        .filter(|m| m.kind == MarkKind::Notch)
        .collect();
    assert_eq!(marks.len(), 2);
    assert!(
        marks
            .iter()
            .all(|m| (m.points[1][0].abs() - 97.).abs() < 1e-9)
    );
    assert!(marks.iter().all(|m| (m.points[1][1] - 100.).abs() < 1e-9));
}
#[test]
fn mirror_handles_diagonal_axes_and_rejects_degenerate_axes() {
    let d = factory();
    let next = symmetry::mirror_piece(
        &d,
        "front",
        "diagonal",
        ["front.p0".into(), "front.p2".into()],
    )
    .unwrap();
    let a = [0., 0.];
    let b = [100., 200.];
    for size in &d.sizes {
        let g = evaluate::grade(&next, size, 0.1).unwrap();
        let expected = tools::mirror(&g[0].seam, a, b).unwrap();
        for (a, b) in expected.iter().zip(&g[2].seam) {
            assert!(distance(*a, *b) < 1e-6);
        }
    }
    assert!(
        symmetry::mirror_piece(&d, "front", "bad", ["front.p0".into(), "front.p0".into()]).is_err()
    );
}
fn row(kind: RowMark) -> MarkRow {
    MarkRow {
        piece: "half".into(),
        id: "placket".into(),
        origin: "half.p0".into(),
        count: 3,
        spacing: 50.0.into(),
        direction: 90.0.into(),
        kind,
        length: 15.0.into(),
        angle: 0.0.into(),
    }
}
#[test]
fn repeated_buttons_buttonholes_and_drills_remain_parametric() {
    for (kind, count) in [
        (RowMark::Button, 3),
        (RowMark::Buttonhole, 9),
        (RowMark::Drill, 3),
    ] {
        let d = square("half");
        let mut r = row(kind);
        r.spacing = "v.pitch".into();
        let mut d = d;
        d.variables.insert("pitch".into(), 40.0.into());
        let next = add_mark_row(&d, &r).unwrap();
        assert_eq!(next.pieces[0].annotations.len(), count);
        let g = evaluate::grade(&next, "M", 0.1).unwrap();
        assert!(distance(g[0].points["placket.c2"], [0., 80.]) < 1e-9);
    }
}
#[test]
fn marks_reject_bad_counts_lengths_and_duplicate_prefixes() {
    let d = square("half");
    let mut r = row(RowMark::Buttonhole);
    r.count = 0;
    assert!(add_mark_row(&d, &r).is_err());
    r.count = 3;
    r.length = (-1.).into();
    assert!(add_mark_row(&d, &r).is_err());
    r.length = 15.0.into();
    let next = add_mark_row(&d, &r).unwrap();
    assert!(add_mark_row(&next, &r).is_err());
}
#[test]
fn production_marks_and_piece_info_survive_marker_and_dxf() {
    let d = add_mark_row(&marked(), &row(RowMark::Buttonhole)).unwrap();
    let next = symmetry::unfold_piece(&d, "half", "full", 3).unwrap();
    let job = exchange::to_job(
        &next,
        Fabric {
            width: 1500.,
            ..Default::default()
        },
        BTreeMap::from([("M".into(), 2)]),
        0.1,
    )
    .unwrap();
    assert!(beta_marker_core::check_job(&job).is_empty());
    assert!(job.pieces[1].fold_line.is_some());
    assert_eq!(job.pieces[1].mirror_quantity, 1);
    let dxf = exchange::export_dxf(&next, &["M".into()], 0.1).unwrap();
    let imported = beta_marker_dxf::import_dxf(dxf.as_bytes(), &Default::default()).unwrap();
    assert_eq!(imported.pieces.len(), 2);
    for (a, b) in job.pieces.iter().zip(&imported.pieces) {
        assert!((area(&a.outline) - area(&b.outline)).abs() < 1e-4);
        assert_eq!(a.name, b.name);
        assert_eq!(a.material, b.material);
        assert_eq!(a.quantity, b.quantity);
        assert_eq!(a.mirror_quantity, b.mirror_quantity);
        for kind in [MarkKind::Notch, MarkKind::Drill, MarkKind::Line] {
            let fold = usize::from(kind == MarkKind::Line && a.fold_line.is_some());
            assert_eq!(
                a.marks.iter().filter(|m| m.kind == kind).count() + fold,
                b.marks.iter().filter(|m| m.kind == kind).count()
            );
        }
    }
    let pdf = print::tiled_pdf(&next, &["M".into()], &Default::default()).unwrap();
    assert!(String::from_utf8_lossy(&pdf).contains("[3 2] 0 d"));
}
