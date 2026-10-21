use crate::{style::*, *};
use serde_json::json;

fn job() -> Job {
    Job {
        format: JOB_FORMAT.into(),
        fabric: Fabric {
            width: 100.,
            repeat: Some(Repeat {
                period_x: Some(10.),
                period_y: Some(20.),
                offset_x: 2.,
                ..Default::default()
            }),
            ..Default::default()
        },
        pieces: vec![Piece {
            id: "panel".into(),
            name: "Panel".into(),
            outline: vec![
                [0., 0.],
                [40., 0.],
                [40., 40.],
                [25., 40.],
                [25., 15.],
                [15., 15.],
                [15., 40.],
                [0., 40.],
            ],
            quantity: 1,
            ..Default::default()
        }],
        ..Default::default()
    }
}
fn result(j: &Job) -> MarkerResult {
    let placements = vec![Placement {
        piece_id: "panel".into(),
        translate: [5., 5.],
        ..Default::default()
    }];
    let m = compute_metrics(j, &placements);
    MarkerResult {
        format: RESULT_FORMAT.into(),
        job_name: j.name.clone(),
        width: j.fabric.width,
        length: m.length,
        piece_area: m.piece_area,
        efficiency: m.efficiency,
        engine: "test".into(),
        seed: None,
        runtime_s: 0.,
        placements,
        warnings: vec![],
    }
}
#[test]
fn match_report_has_signed_axes_missing_rows_and_validator_parity() {
    let mut j = job();
    j.pieces[0].match_points = vec![MatchPoint {
        id: "hem".into(),
        point: [0., 0.],
    }];
    j.match_rules = vec![MatchRule {
        a: PointRef {
            piece_id: "panel".into(),
            point_id: "hem".into(),
        },
        b: None,
        axes: MatchAxes::Both,
        offset: [0., 0.],
    }];
    let mut r = result(&j);
    r.placements[0].bundle = Some(0);
    r.placements[0].translate = [11., 19.]; // fabric phase [2,0], signed errors [-1,-1]
    let rows = overlay(&j, &r, &Default::default()).unwrap().matches;
    assert_eq!(rows, validate(&j, &r).matches);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].offset_mm, [Some(-1.), Some(-1.)]);
    assert_eq!(rows[0].status, MatchStatus::Matched); // default 1 mm tolerance, inclusive
    assert_eq!(rows[0].a, Some([11., 19.]));
    assert_eq!(rows[0].b, Some([2., 0.]));
    r.placements[0].translate[1] = 18.;
    let report = validate(&j, &r);
    assert_eq!(report.matches[0].status, MatchStatus::Off);
    assert_eq!(report.matches[0].error_mm, Some(2.));
    assert!(report.errors.iter().any(|e| e.code == "match"));
    j.match_rules[0].axes = MatchAxes::X;
    assert_eq!(validate(&j, &r).matches[0].offset_mm, [Some(-1.), None]);
    r.placements.clear();
    let rows = overlay(&j, &r, &Default::default()).unwrap().matches;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].status, MatchStatus::NotPlaced);
    assert_eq!(rows[0].error_mm, None);
    assert_eq!(rows[0].a, None);
    assert!(!rows[0].pass);
}
#[test]
fn concave_repeat_lines_are_clipped_and_offsets_are_respected() {
    let j = job();
    let r = result(&j);
    let overlay = overlay(&j, &r, &Default::default()).unwrap();
    assert!(!overlay.stripes.is_empty());
    for line in &overlay.stripes {
        let poly = &overlay.outlines[line.placement].points;
        for t in [0., 0.25, 0.5, 0.75, 1.] {
            let p = [
                line.points[0][0] * (1. - t) + line.points[1][0] * t,
                line.points[0][1] * (1. - t) + line.points[1][1] * t,
            ];
            assert!(contains(p, poly), "{p:?}");
        }
        if line.axis == Axis::X {
            assert!((line.points[0][0] - 2.).rem_euclid(10.) < 1e-8);
        }
    }
    let y20: Vec<_> = overlay
        .stripes
        .iter()
        .filter(|s| s.axis == Axis::Y && s.points[0][1] == 20.)
        .collect();
    assert_eq!(y20.len(), 2);
}
#[test]
fn fold_halves_fusing_envelopes_and_version_differences_use_display_geometry() {
    let mut j = job();
    j.fabric.layout = Layout::Folded;
    j.pieces[0].outline = vec![[0., 0.], [40., 0.], [40., 40.], [0., 40.]];
    j.pieces[0].fold_line = Some([[0., 20.], [40., 20.]]);
    let mut r = result(&j);
    r.placements[0].on_fold = true;
    let old = PreviousMarker {
        job: j.clone(),
        result: r.clone(),
    };
    r.placements[0].translate[0] += 10.;
    let o = overlay(
        &j,
        &r,
        &OverlayOptions {
            previous: Some(old),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(o.previous[0].kind, "moved");
    assert_eq!(geometry::area(&o.outlines[0].points), 800.);
    for s in o.stripes {
        assert!(contains(s.points[0], &o.outlines[0].points));
    }
    r.placements[0].on_fold = false;
    j.pieces[0].block_margin = Some(5.);
    let o = overlay(&j, &r, &Default::default()).unwrap();
    assert_eq!(geometry::area(&o.outlines[0].points), 2500.);
}
#[test]
fn lay_and_repeat_metadata_are_validated_and_bounded() {
    let mut j = job();
    let r = result(&j);
    j.meta.insert(
        "beta_style_lay".into(),
        json!({"face":"up","plies":10,"end_allowance_mm":15}),
    );
    assert_eq!(
        overlay(&j, &r, &Default::default())
            .unwrap()
            .spread_length_mm,
        r.length + 30.
    );
    j.fabric.layout = Layout::Tubular;
    j.meta.get_mut("beta_style_lay").unwrap()["plies"] = json!(3);
    assert!(check_job(&j).iter().any(|e| e.code == "lay"));
    j.meta.get_mut("beta_style_lay").unwrap()["plies"] = json!(4);
    j.meta.insert(
        "beta_repeat_bands".into(),
        json!([{"axis":"x","offset_mm":0,"width_mm":1,"colour":"red;bad"}]),
    );
    assert!(check_job(&j).iter().any(|e| e.code == "repeat_bands"));
    j.meta.remove("beta_repeat_bands");
    j.fabric.repeat.as_mut().unwrap().period_x = Some(0.00001);
    assert!(overlay(&j, &r, &Default::default()).is_err());
    j.spacing.edge_gap = f64::NAN;
    assert!(!validate(&j, &r).ok);
}
#[test]
fn bad_counts_and_numeric_rules_return_errors_without_panicking() {
    let mut j = job();
    let r = result(&j);
    j.pieces[0].quantity = u32::MAX;
    j.pieces[0].mirror_quantity = 1;
    assert!(check_job(&j).iter().any(|i| i.code == "quantity"));
    assert!(!validate(&j, &r).ok);
    j.pieces[0].mirror_quantity = 0;
    assert!(check_job(&j).iter().any(|i| i.code == "quantity"));
    assert!(!validate(&j, &r).ok);
    j.pieces[0].quantity = 1;
    j.pieces[0].mirror_quantity = 0;
    j.fabric.repeat.as_mut().unwrap().offset_x = f64::INFINITY;
    assert!(check_job(&j).iter().any(|i| i.code == "repeat"));
    assert!(!validate(&j, &r).ok);
}
