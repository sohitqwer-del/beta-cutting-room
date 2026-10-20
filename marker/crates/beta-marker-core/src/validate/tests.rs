use super::*;
use crate::metrics::compute_metrics;
use crate::model::*;
use std::collections::BTreeMap;

fn fixture() -> (Job, MarkerResult) {
    let p = Piece {
        id: "square".into(),
        name: "Square".into(),
        style: None,
        size: Some("M".into()),
        material: None,
        outline: vec![[0., 0.], [10., 0.], [10., 10.], [0., 10.]],
        grain: None,
        quantity: 2,
        mirror_quantity: 0,
        rotation: RotationRule::Flip180,
        buffer: 0.,
        marks: vec![],
        ..Default::default()
    };
    let j = Job {
        match_rules: Vec::new(),
        format: JOB_FORMAT.into(),
        name: "Test".into(),
        fabric: Fabric {
            width: 100.,
            max_length: None,
            nap: Nap::None,
            allow_flip: false,
            tilt_deg: 0.,
            ..Default::default()
        },
        spacing: Spacing::default(),
        pieces: vec![p],
        order: None,
        meta: BTreeMap::new(),
    };
    let r = MarkerResult {
        format: RESULT_FORMAT.into(),
        job_name: j.name.clone(),
        width: 100.,
        length: 30.,
        piece_area: 200.,
        efficiency: 200. / 3000.,
        engine: "handmade".into(),
        seed: None,
        runtime_s: 0.,
        warnings: vec![],
        placements: vec![pl(0, [0., 0.]), pl(1, [20., 0.])],
    };
    (j, r)
}
fn pl(copy: u32, translate: Pt) -> Placement {
    Placement {
        piece_id: "square".into(),
        copy,
        mirrored: false,
        rotation_deg: 0.,
        translate,
        locked: false,
        ..Default::default()
    }
}
fn codes(j: &Job, r: &MarkerResult) -> Vec<String> {
    validate(j, r).errors.into_iter().map(|e| e.code).collect()
}
fn has(j: &Job, r: &MarkerResult, code: &str) -> bool {
    codes(j, r).iter().any(|c| c == code)
}

#[test]
fn explicit_layers_serve_two_garments_with_exact_handed_counts() {
    let (mut j, mut r) = fixture();
    j.fabric.layout = Layout::Folded;
    j.pieces[0].quantity = 1;
    j.pieces[0].mirror_quantity = 1;
    j.order = Some(Order { sizes: BTreeMap::from([("M".into(), 2)]) });
    for (i, p) in r.placements.iter_mut().enumerate() {
        p.bundle = Some(i as u32);
        p.layer_allocations = vec![
            Some(LayerAllocation { bundle: i as u32, mirrored: false }),
            Some(LayerAllocation { bundle: 1 - i as u32, mirrored: true }),
        ];
    }
    assert!(validate(&j, &r).ok, "{:?}", validate(&j, &r).errors);
    r.placements[1].layer_allocations[1] = None;
    assert!(has(&j, &r, "missing_piece"));
    assert!(has(&j, &r, "bundle"));
    r.placements[1].layer_allocations[1] = Some(LayerAllocation { bundle: 1, mirrored: true });
    assert!(has(&j, &r, "bundle")); // Global count alone cannot catch this.
    r.placements[1].layer_allocations[1] = Some(LayerAllocation { bundle: 0, mirrored: false });
    assert!(has(&j, &r, "layer_allocation"));
    assert!(has(&j, &r, "extra_piece"));
}

#[test]
fn shared_layers_participate_in_both_garments_shade_nap_and_match_checks() {
    let (mut j, mut r) = fixture();
    j.fabric.layout = Layout::Folded;
    j.pieces[0].quantity = 1;
    j.pieces[0].mirror_quantity = 1;
    j.order = Some(Order { sizes: BTreeMap::from([("M".into(), 2)]) });
    for (i, p) in r.placements.iter_mut().enumerate() {
        p.bundle = Some(i as u32);
        p.layer_allocations = vec![
            Some(LayerAllocation { bundle: i as u32, mirrored: false }),
            Some(LayerAllocation { bundle: 1 - i as u32, mirrored: true }),
        ];
    }
    j.fabric.shade = Some(ShadeRule { axis: ShadeAxis::Length, band_mm: 15. });
    assert!(has(&j, &r, "shade"));
    j.fabric.shade = None;
    j.fabric.nap = Nap::OneWayPerBundle;
    r.placements[1].rotation_deg = 180.;
    r.placements[1].translate = [30., 10.];
    assert!(has(&j, &r, "nap_bundle"));
    j.fabric.nap = Nap::None;
    r.placements[1].rotation_deg = 0.;
    r.placements[1].translate = [25., 0.];
    j.fabric.repeat = Some(Repeat { period_x: Some(20.), ..Default::default() });
    j.pieces[0].match_points = vec![MatchPoint { id: "stripe".into(), point: [0., 0.] }];
    j.match_rules = vec![MatchRule {
        a: PointRef { piece_id: "square".into(), point_id: "stripe".into() },
        b: None, axes: MatchAxes::X, offset: [0., 0.],
    }];
    assert!(has(&j, &r, "match"));
}

#[test]
fn explicit_surplus_is_not_useful_and_cannot_cross_sizes_or_open_fabric() {
    let (mut j, mut r) = fixture();
    j.fabric.layout = Layout::Tubular;
    j.pieces[0].quantity = 1;
    r.placements.truncate(1);
    r.placements[0].bundle = Some(0);
    r.placements[0].layer_allocations = vec![Some(LayerAllocation { bundle: 0, mirrored: false }), None];
    let report = validate(&j, &r);
    assert!(report.ok, "{:?}", report.errors);
    assert!(report.warnings.iter().any(|w| w.code == "surplus_copy"));
    assert_eq!(layer_copies(j.fabric.layout, &r.placements[0]), (1, 0));
    j.order = Some(Order { sizes: BTreeMap::from([("M".into(), 1), ("L".into(), 1)]) });
    r.placements[0].layer_allocations[0].as_mut().unwrap().bundle = 1;
    assert!(has(&j, &r, "layer_allocation"));
    j.fabric.layout = Layout::Open;
    assert!(has(&j, &r, "layer_allocation"));
}

#[test]
fn declared_symmetric_full_layers_can_fill_two_normal_demands() {
    let (mut j, mut r) = fixture();
    j.fabric.layout = Layout::Folded;
    j.pieces[0].quantity = 1;
    j.pieces[0].fold_line = Some([[0., 5.], [10., 5.]]);
    j.order = Some(Order { sizes: BTreeMap::from([("M".into(), 2)]) });
    r.placements.truncate(1);
    r.placements[0].bundle = Some(0);
    r.placements[0].layer_allocations = vec![
        Some(LayerAllocation { bundle: 0, mirrored: false }),
        Some(LayerAllocation { bundle: 1, mirrored: false }),
    ];
    assert!(validate(&j, &r).ok, "{:?}", validate(&j, &r).errors);
    j.pieces[0].fold_line = None;
    assert!(has(&j, &r, "layer_allocation"));
}

#[test]
fn symmetric_on_fold_mirror_is_half_choice_not_full_flip_permission() {
    let (mut j, mut r) = fixture();
    j.fabric.layout = Layout::Folded;
    j.fabric.nap = Nap::OneWay;
    j.pieces[0].quantity = 1;
    j.pieces[0].grain = Some([[0., 0.], [10., 0.]]);
    j.pieces[0].fold_line = Some([[0., 5.], [10., 5.]]);
    r.placements.truncate(1);
    let p = &mut r.placements[0];
    p.on_fold = true;
    p.mirrored = true;
    p.rotation_deg = 180.;
    // Mirroring flips the grain as well: a 180-degree turn restores +x.
    p.translate = [0., 105.];
    assert!(validate(&j, &r).ok, "{:?}", validate(&j, &r).errors);
    r.placements[0].on_fold = false;
    assert!(has(&j, &r, "mirror"));
}

#[test]
fn layer_wire_fixture_round_trips_and_old_json_defaults() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../packages/marker-wasm/test/fixtures/contract.json"
    )).unwrap();
    for raw in fixture["layer_allocation_examples"].as_array().unwrap() {
        let p: Placement = serde_json::from_value(raw.clone()).unwrap();
        assert_eq!(serde_json::to_value(&p).unwrap()["layer_allocations"], raw["layer_allocations"]);
    }
    let old: Placement = serde_json::from_value(fixture["result"]["placements"][0].clone()).unwrap();
    assert!(old.layer_allocations.is_empty());
}

#[test]
fn counts_unknown_and_duplicate_copy() {
    let (mut j, mut r) = fixture();
    assert!(validate(&j, &r).ok);
    r.placements.pop();
    assert!(has(&j, &r, "missing_piece"));
    r.placements.push(pl(0, [20., 0.]));
    assert!(
        validate(&j, &r)
            .warnings
            .iter()
            .any(|i| i.code == "duplicate_copy")
    );
    r.placements.push(pl(2, [40., 0.]));
    assert!(has(&j, &r, "extra_piece"));
    r.placements[2].piece_id = "ghost".into();
    assert!(has(&j, &r, "unknown_piece"));
    j.order = Some(Order {
        sizes: BTreeMap::from([("M".into(), 2)]),
    });
    assert!(has(&j, &r, "missing_piece"));
}
#[test]
fn mirror_and_grain() {
    let (mut j, mut r) = fixture();
    r.placements[1].mirrored = true;
    r.placements[1].translate = [30., 0.];
    assert!(has(&j, &r, "mirror"));
    assert!(has(&j, &r, "missing_piece") && has(&j, &r, "extra_piece"));
    j.fabric.allow_flip = true;
    assert!(!has(&j, &r, "mirror"));
    j.fabric.allow_flip = false;
    j.pieces[0].quantity = 1;
    j.pieces[0].mirror_quantity = 1;
    assert!(validate(&j, &r).ok);
    r.placements[0].rotation_deg = 90.;
    assert!(has(&j, &r, "grain"));
    j.pieces[0].grain = Some([[0., 0.], [0., 10.]]);
    r.placements[0].rotation_deg = 270.;
    r.placements[1].rotation_deg = 270.;
    assert!(!has(&j, &r, "grain"));
}
#[test]
fn edges_length_and_metrics() {
    let (mut j, mut r) = fixture();
    r.placements[0].translate = [-0.009, 90.009];
    assert!(!has(&j, &r, "outside_width") && !has(&j, &r, "outside_start"));
    r.placements[0].translate = [-0.011, 90.011];
    assert!(has(&j, &r, "outside_width") && has(&j, &r, "outside_start"));
    r.placements[0].translate = [0., 0.];
    j.spacing.edge_gap = 1.;
    j.pieces[0].buffer = 1.;
    assert!(has(&j, &r, "outside_width") && has(&j, &r, "outside_start"));
    j.fabric.max_length = Some(29.99);
    assert!(has(&j, &r, "too_long"));
    r.length = 999.;
    r.efficiency = 1.;
    let v = validate(&j, &r);
    assert!(v.warnings.iter().any(|i| i.code == "length"));
    assert!(v.warnings.iter().any(|i| i.code == "efficiency"));
    assert_eq!(v.length, 30.);
}
#[test]
fn overlap_touch_gap_and_identical() {
    let (mut j, mut r) = fixture();
    for t in [[5., 5.], [5., 0.], [0., 0.]] {
        r.placements[1].translate = t;
        assert!(has(&j, &r, "overlap"), "{t:?}");
    }
    for t in [[10., 0.], [10., 10.]] {
        r.placements[1].translate = t;
        assert!(!has(&j, &r, "overlap"), "{t:?}");
    }
    j.spacing.piece_gap = 0.005;
    assert!(has(&j, &r, "gap"));
    j.spacing.piece_gap = 2.;
    assert!(has(&j, &r, "gap"));
    r.placements[1].translate = [11., 0.];
    assert!(has(&j, &r, "gap") && !has(&j, &r, "overlap"));
    r.placements[1].translate = [11.991, 0.];
    assert!(!has(&j, &r, "gap"));
    j.pieces[0].buffer = 1.;
    assert!(has(&j, &r, "gap"));
    r.placements[1].translate = [14., 0.];
    assert!(!has(&j, &r, "gap"));
}
#[test]
fn containment_and_concave_disjoint_boxes() {
    let (mut j, mut r) = fixture();
    let mut inner = j.pieces[0].clone();
    inner.id = "inner".into();
    inner.quantity = 1;
    inner.outline = vec![[0., 0.], [2., 0.], [2., 2.], [0., 2.]];
    j.pieces[0].quantity = 1;
    j.pieces.push(inner);
    r.placements[1].piece_id = "inner".into();
    r.placements[1].translate = [4., 4.];
    assert!(has(&j, &r, "overlap"));
    j.pieces[0].outline = vec![
        [0., 0.],
        [10., 0.],
        [10., 3.],
        [3., 3.],
        [3., 10.],
        [0., 10.],
    ];
    assert!(!has(&j, &r, "overlap"));
    j.spacing.piece_gap = 2.;
    assert!(has(&j, &r, "gap"));
    r.placements[1].translate = [5., 5.];
    assert!(!has(&j, &r, "gap"));
}
#[test]
fn collinear_duplicate_winding_and_t_junctions() {
    let (mut j, mut r) = fixture();
    j.pieces[0].outline = vec![
        [0., 0.],
        [5., 0.],
        [5., 0.],
        [10., 0.],
        [10., 10.],
        [0., 10.],
        [0., 0.],
    ];
    r.placements[1].translate = [10., 0.];
    assert!(!has(&j, &r, "overlap"));
    r.placements[1].translate = [9., 0.];
    assert!(has(&j, &r, "overlap"));
    j.pieces[0].outline.reverse();
    assert!(has(&j, &r, "overlap"));
    let mut p = j.pieces[0].clone();
    p.id = "cross".into();
    p.quantity = 1;
    p.outline = vec![[0., 0.], [20., 0.], [20., 2.], [0., 2.]];
    j.pieces[0].quantity = 1;
    j.pieces.push(p);
    r.placements[1].piece_id = "cross".into();
    r.placements[1].translate = [-5., 4.];
    assert!(has(&j, &r, "overlap"));
}
#[test]
fn invalid_coordinates_cannot_pass() {
    let (j, mut r) = fixture();
    r.placements[0].translate[0] = f64::NAN;
    assert!(!validate(&j, &r).ok);
}
#[test]
#[ignore = "run with cargo test -p beta-marker-core --release validation_speed -- --ignored --nocapture"]
fn validation_speed() {
    let (mut j, mut r) = fixture();
    j.fabric.width = 25. * 21.;
    j.spacing.piece_gap = 1.;
    j.pieces[0].quantity = 1000;
    j.pieces[0].outline = (0..100)
        .map(|i| {
            let a = i as f64 * std::f64::consts::TAU / 100.;
            [10. + 10. * a.cos(), 10. + 10. * a.sin()]
        })
        .collect();
    r.placements = (0..1000)
        .map(|i| pl(i, [(i / 25) as f64 * 21., (i % 25) as f64 * 21.]))
        .collect();
    let m = compute_metrics(&j, &r.placements);
    r.length = m.length;
    r.efficiency = m.efficiency;
    let start = std::time::Instant::now();
    let v = validate(&j, &r);
    let elapsed = start.elapsed();
    println!("1000 pieces x 100 vertices: {elapsed:?}");
    assert!(v.ok, "{:?}", v.errors);
    if !cfg!(debug_assertions) {
        assert!(elapsed.as_secs_f64() < 1.0);
    }
}

#[test]
fn shrinkage_and_faceless_counts() {
    let (mut j, mut r) = fixture();
    r.placements[1].mirrored = true;
    r.placements[1].translate = [30., 0.];
    j.fabric.allow_flip = true;
    assert!(validate(&j, &r).ok);
    r.placements.pop();
    assert!(has(&j, &r, "missing_piece"));
    let (mut j, mut r) = fixture();
    j.fabric.shrinkage.length_pct = 110.;
    assert!(has(&j, &r, "overlap"));
    r.placements[1].translate = [22., 0.];
    assert!(!has(&j, &r, "overlap"));
    assert_eq!(validate(&j, &r).length, 43.);
}
#[test]
fn layers_and_fold_geometry() {
    let (mut j, mut r) = fixture();
    j.fabric.layout = Layout::Folded;
    j.pieces[0].quantity = 1;
    j.pieces[0].mirror_quantity = 1;
    r.placements.truncate(1);
    assert!(validate(&j, &r).ok);
    j.pieces[0].mirror_quantity = 0;
    assert!(
        validate(&j, &r)
            .warnings
            .iter()
            .any(|i| i.code == "surplus_copy")
    );
    j.pieces[0].quantity = 2;
    assert!(has(&j, &r, "missing_piece"));
    j.pieces[0].quantity = 1;
    j.pieces[0].fold_line = Some([[10., 5.], [0., 5.]]);
    j.spacing.edge_gap = 2.;
    r.placements[0].on_fold = true;
    r.placements[0].translate = [2., 95.];
    assert!(validate(&j, &r).ok, "{:?}", validate(&j, &r).errors);
    assert_eq!(validate(&j, &r).piece_area, 50.);
    r.placements[0].translate[1] = 95.009;
    assert!(!has(&j, &r, "fold"));
    r.placements[0].translate[1] = 95.011;
    assert!(has(&j, &r, "fold"));
    j.fabric.layout = Layout::Open;
    assert!(has(&j, &r, "fold"));
    j.fabric.layout = Layout::Tubular;
    j.pieces[0].fold_line = Some([[0., 5.], [10., 5.]]);
    r.placements[0].translate[1] = -5.;
    assert!(validate(&j, &r).ok);
    j.pieces[0].fold_line = None;
    assert!(has(&j, &r, "fold"));
}
#[test]
fn defects_and_fusing_rectangles() {
    let (mut j, mut r) = fixture();
    j.fabric.defects = vec![Zone {
        id: "hole".into(),
        polygon: vec![[10., 0.], [12., 0.], [12., 2.], [10., 2.]],
        note: None,
    }];
    assert!(!has(&j, &r, "defect"));
    j.fabric.defects[0].polygon[0][0] = 9.;
    assert!(has(&j, &r, "defect"));
    j.fabric.defects.clear();
    j.pieces[0].outline = vec![[0., 0.], [10., 0.], [0., 10.]];
    j.pieces[0].block_margin = Some(2.);
    r.placements[0].translate = [2., 2.];
    r.placements[1].translate = [15., 2.];
    assert!(has(&j, &r, "overlap"));
    r.placements[1].translate = [16., 2.];
    assert!(!has(&j, &r, "overlap"));
    j.spacing.piece_gap = 1.;
    assert!(has(&j, &r, "gap"));
    r.placements[1].translate = [17., 2.];
    assert!(!has(&j, &r, "gap"));
    r.placements[0].translate[1] = 89.;
    assert!(has(&j, &r, "outside_width"));
    j.pieces[0].grain = Some([[0., 0.], [1., 1.]]);
    let b = crate::rules::cut_outline(
        &j.pieces[0],
        &Placement {
            rotation_deg: 315.,
            ..pl(0, [20., 20.])
        },
    );
    assert!((b[0][1] - b[1][1]).abs() < 1e-8);
}
#[test]
fn bundle_ratio_completeness_and_direction() {
    let (mut j, mut r) = fixture();
    j.fabric.nap = Nap::OneWayPerBundle;
    assert!(has(&j, &r, "bundle_missing"));
    for p in &mut r.placements {
        p.bundle = Some(0);
    }
    assert!(validate(&j, &r).ok);
    r.placements[1].bundle = Some(1);
    assert!(has(&j, &r, "bundle"));
    r.placements[1].bundle = Some(0);
    r.placements[1].rotation_deg = 180.;
    r.placements[1].translate = [30., 10.];
    assert!(has(&j, &r, "nap_bundle"));
    r.placements[0].rotation_deg = 180.;
    r.placements[0].translate = [10., 10.];
    assert!(!has(&j, &r, "nap_bundle"));
    j.order = Some(Order {
        sizes: BTreeMap::from([("M".into(), 2)]),
    });
    assert!(has(&j, &r, "bundle"));
    j.order = None;
    let mut second = j.pieces[0].clone();
    second.id = "other-size".into();
    second.size = Some("L".into());
    second.quantity = 1;
    j.pieces.push(second);
    r.placements[1].piece_id = "other-size".into();
    assert!(has(&j, &r, "bundle"));
}
#[test]
fn shade_requires_whole_bundle_in_one_band() {
    let (mut j, mut r) = fixture();
    for p in &mut r.placements {
        p.bundle = Some(0);
    }
    j.fabric.shade = Some(ShadeRule {
        axis: ShadeAxis::Length,
        band_mm: 30.,
    });
    assert!(!has(&j, &r, "shade"));
    r.placements[1].translate[0] = 21.;
    assert!(has(&j, &r, "shade"));
    j.fabric.shade = Some(ShadeRule {
        axis: ShadeAxis::Width,
        band_mm: 10.,
    });
    assert!(!has(&j, &r, "shade"));
    r.placements[1].translate[1] = 10.;
    assert!(has(&j, &r, "shade"));
}
#[test]
fn matching_phase_wrap_axes_offsets_and_bundles() {
    let (mut j, mut r) = fixture();
    j.pieces[0].quantity = 1;
    let mut p = j.pieces[0].clone();
    p.id = "back".into();
    j.pieces.push(p);
    for p in &mut j.pieces {
        p.match_points = vec![MatchPoint {
            id: "seam".into(),
            point: [1., 2.],
        }];
    }
    r.placements[1].piece_id = "back".into();
    for p in &mut r.placements {
        p.bundle = Some(0);
    }
    j.fabric.repeat = Some(Repeat {
        period_x: Some(10.),
        period_y: Some(10.),
        ..Default::default()
    });
    let reference = |id: &str| PointRef {
        piece_id: id.into(),
        point_id: "seam".into(),
    };
    j.match_rules = vec![MatchRule {
        a: reference("square"),
        b: Some(reference("back")),
        axes: MatchAxes::Both,
        offset: [0., 0.],
    }];
    assert!(!has(&j, &r, "match"));
    r.placements[1].translate[0] = 20.9;
    assert!(!has(&j, &r, "match"));
    r.placements[1].translate[0] = 21.1;
    assert!(has(&j, &r, "match"));
    j.match_rules[0].offset = [-1.1, 0.];
    assert!(!has(&j, &r, "match"));
    r.placements[1].translate[1] = 2.;
    assert!(has(&j, &r, "match"));
    j.match_rules[0].axes = MatchAxes::X;
    assert!(!has(&j, &r, "match"));
    j.match_rules[0].b = None;
    j.match_rules[0].offset = [1., 2.];
    assert!(!has(&j, &r, "match"));
    j.fabric.repeat.as_mut().expect("repeat").offset_x = 5.;
    assert!(has(&j, &r, "match"));
    j.fabric.repeat.as_mut().expect("repeat").offset_x = 0.;
    j.match_rules[0].axes = MatchAxes::Y;
    assert!(!has(&j, &r, "match"));
    j.fabric.repeat.as_mut().expect("repeat").period_y = None;
    assert!(has(&j, &r, "match"));
}

#[test]
fn folded_bundle_cannot_hide_an_extra_garment() {
    let (mut j, mut r) = fixture();
    j.fabric.layout = Layout::Folded;
    j.fabric.nap = Nap::OneWayPerBundle;
    j.pieces[0].quantity = 1;
    j.pieces[0].mirror_quantity = 1;
    for pl in &mut r.placements {
        pl.bundle = Some(0);
    }
    assert!(has(&j, &r, "bundle"));
    r.placements.truncate(1);
    assert!(validate(&j, &r).ok);
    j.pieces[0].mirror_quantity = 0;
    assert!(validate(&j, &r).ok); // only the unavoidable opposite layer is surplus
}
#[test]
fn matching_is_independent_between_bundles_and_transforms_points() {
    let (mut j, mut r) = fixture();
    j.pieces[0].quantity = 1;
    j.pieces[0].match_points = vec![MatchPoint {
        id: "p".into(),
        point: [2., 3.],
    }];
    j.order = Some(Order {
        sizes: BTreeMap::from([("M".into(), 2)]),
    });
    j.fabric.repeat = Some(Repeat {
        period_x: Some(10.),
        period_y: Some(10.),
        tolerance_mm: 0.1,
        ..Default::default()
    });
    j.match_rules = vec![MatchRule {
        a: PointRef {
            piece_id: "square".into(),
            point_id: "p".into(),
        },
        b: None,
        axes: MatchAxes::Both,
        offset: [2., 3.],
    }];
    r.placements[0].bundle = Some(0);
    r.placements[1].bundle = Some(1);
    assert!(validate(&j, &r).ok);
    r.placements[1].rotation_deg = 180.;
    r.placements[1].translate = [34., 16.];
    assert!(!has(&j, &r, "match")); // transformed point (32,13) matches (2,3)
    r.placements[1].translate[1] += 0.2;
    assert!(has(&j, &r, "match"));
}

#[test]
fn fold_edges_waive_clearance_but_not_the_fabric_boundary() {
    let (mut j, mut r) = fixture();
    j.fabric.layout = Layout::Folded;
    j.spacing.edge_gap = 2.;
    r.placements[0].translate = [2., 90.];
    r.placements[1].translate = [22., 90.];
    assert!(!has(&j, &r, "outside_width"));
    r.placements[0].translate[1] = 0.;
    assert!(has(&j, &r, "outside_width"));
    j.fabric.layout = Layout::Tubular;
    assert!(!has(&j, &r, "outside_width"));
    r.placements[0].translate[1] = -0.02;
    assert!(has(&j, &r, "outside_width"));
    j.fabric.layout = Layout::Open;
    r.placements[0].translate[1] = 90.;
    assert!(has(&j, &r, "outside_width"));
}
#[test]
fn on_fold_halves_do_not_collide_with_the_discarded_half() {
    let (mut j, mut r) = fixture();
    j.fabric.layout = Layout::Tubular;
    j.pieces[0].quantity = 1;
    j.pieces[0].fold_line = Some([[0., 5.], [10., 5.]]);
    r.placements[0].on_fold = true;
    r.placements[0].translate = [0., -5.];
    let mut p = j.pieces[0].clone();
    p.id = "small".into();
    p.outline = vec![[0., 0.], [2., 0.], [2., 2.], [0., 2.]];
    p.fold_line = None;
    j.pieces.push(p);
    r.placements[1].piece_id = "small".into();
    r.placements[1].translate = [0., 6.];
    assert!(!has(&j, &r, "overlap"));
    r.placements[1].translate[1] = 4.;
    assert!(has(&j, &r, "overlap"));
}

#[test]
fn fusing_block_keeps_the_whole_cut_rectangle_clear_of_defects() {
    let (mut j, mut r) = fixture();
    j.pieces[0].quantity = 1;
    r.placements.truncate(1);
    j.pieces[0].outline = vec![[0., 0.], [10., 0.], [0., 10.]];
    r.placements[0].translate = [2., 2.];
    j.fabric.defects = vec![Zone {
        id: "hole".into(),
        polygon: vec![[10., 10.], [11., 10.], [11., 11.], [10., 11.]],
        note: None,
    }];
    assert!(!has(&j, &r, "defect"));
    j.pieces[0].block_margin = Some(2.);
    assert!(has(&j, &r, "defect"));
}

#[test]
fn per_piece_flip_overrides_fabric_counts_and_bundles() {
    let (mut j, mut r) = fixture();
    // Use the fixture's single normal demand, placed face down.
    j.pieces.truncate(1);
    j.order = None;
    j.pieces[0].quantity = 1;
    j.pieces[0].mirror_quantity = 0;
    j.pieces[0].flip = Some(true);
    let pl = Placement {
        piece_id: j.pieces[0].id.clone(),
        mirrored: true,
        translate: [100.0, 0.0],
        bundle: Some(0),
        ..Default::default()
    };
    r.placements = vec![pl];
    let report = validate(&j, &r);
    assert!(
        !report.errors.iter().any(
            |i| ["mirror", "missing_piece", "extra_piece", "bundle"].contains(&i.code.as_str())
        ),
        "{:?}",
        report.errors
    );
    j.fabric.allow_flip = true;
    j.pieces[0].flip = Some(false);
    assert!(validate(&j, &r).errors.iter().any(|i| i.code == "mirror"));
}

#[test]
fn metrics_and_validation_share_block_length_and_outline_area() {
    let (mut j, mut r) = fixture();
    j.pieces[0].block_margin = Some(2.0);
    r.placements = vec![pl(0, [2.0, 2.0]), pl(1, [22.0, 2.0])];
    let m = compute_metrics(&j, &r.placements);
    let report = validate(&j, &r);
    assert!(report.ok, "{:?}", report.errors);
    assert_eq!(m.length, 34.0);
    assert_eq!(m.piece_area, 200.0);
    assert_eq!(report.length, m.length);
    assert_eq!(report.efficiency, m.efficiency);
}

#[test]
fn bundle_ids_follow_shared_garment_order() {
    let (mut j, mut r) = fixture();
    j.pieces[0].quantity = 1;
    j.pieces[0].size = Some("S".into());
    let mut large = j.pieces[0].clone();
    large.id = "large".into();
    large.size = Some("XL".into());
    j.pieces.push(large);
    j.order = Some(Order {
        sizes: BTreeMap::from([("S".into(), 1), ("XL".into(), 1)]),
    });
    r.placements[0].bundle = Some(0);
    r.placements[1].piece_id = "large".into();
    r.placements[1].bundle = Some(1);
    assert!(validate(&j, &r).ok);
    r.placements[0].bundle = Some(1);
    r.placements[1].bundle = Some(0);
    assert!(has(&j, &r, "bundle"));
}
