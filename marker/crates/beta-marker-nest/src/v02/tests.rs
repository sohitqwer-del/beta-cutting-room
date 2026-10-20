use super::*;
use crate::SearchTuning;
use beta_marker_core::geometry::area;
use beta_marker_core::{Fabric, JOB_FORMAT, Order, Shrinkage, Spacing, Zone};

fn job() -> Job {
    Job {
        format: JOB_FORMAT.into(),
        name: "v02 fixture".into(),
        fabric: Fabric {
            width: 100.0,
            ..Default::default()
        },
        pieces: vec![Piece {
            id: "panel".into(),
            name: "panel".into(),
            quantity: 2,
            outline: vec![[0.0, 0.0], [60.0, 0.0], [60.0, 40.0], [0.0, 40.0]],
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn options() -> NestOptions {
    NestOptions {
        time_limit_s: 0.05,
        workers: Some(2),
        seed: Some(1),
        ..Default::default()
    }
}

fn internally_nest(job: &Job, options: &NestOptions, variant: usize) -> Result<MarkerResult> {
    let templates = slots(job)?;
    let fixed = fixed_placements(job, &templates, &options.fixed)?;
    checks::geometry(job, &fixed)?;
    run(
        job,
        &templates,
        &fixed,
        options,
        variant,
        &SearchTuning::default(),
        &mut |_| {},
        None,
    )
}

#[test]
fn fixed_transforms_count_towards_demand_and_leave_holes_available() -> Result<()> {
    let mut j = job();
    j.fabric.width = 45.0;
    let fixed = Placement {
        piece_id: "panel".into(),
        translate: [100.0, 0.0],
        bundle: Some(0),
        locked: true,
        ..Default::default()
    };
    let options = NestOptions {
        fixed: vec![fixed.clone()],
        ..options()
    };
    let result = super::nest(&j, &options, &mut |_| {})?;
    assert!(result.placements.contains(&fixed));
    assert_eq!(result.placements.len(), 2);
    assert_eq!(result.length, 160.0);
    assert!(result.placements.iter().any(|p| p.translate[0] < 1e-6));
    assert!(beta_marker_core::validate(&j, &result).ok);
    Ok(())
}

#[test]
fn invalid_fixed_placements_fail_without_moving_them() -> Result<()> {
    let j = job();
    let fixed = Placement {
        piece_id: "panel".into(),
        bundle: Some(0),
        ..Default::default()
    };
    for placements in [
        vec![fixed.clone(), fixed.clone()],
        vec![Placement {
            translate: [-10.0, 0.0],
            ..fixed.clone()
        }],
        vec![Placement {
            piece_id: "unknown".into(),
            ..fixed.clone()
        }],
    ] {
        assert!(
            super::nest(
                &j,
                &NestOptions {
                    fixed: placements,
                    ..options()
                },
                &mut |_| {}
            )
            .is_err()
        );
    }
    let mut j = j;
    j.fabric.max_length = Some(10.0);
    assert!(
        super::nest(
            &j,
            &NestOptions {
                fixed: vec![fixed],
                ..options()
            },
            &mut |_| {}
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn defects_are_obstacles_without_piece_gap_inflation() -> Result<()> {
    let mut j = job();
    j.spacing.piece_gap = 4.0;
    j.fabric.defects = vec![Zone {
        id: "hole".into(),
        polygon: vec![[0.0, 0.0], [30.0, 0.0], [30.0, 100.0], [0.0, 100.0]],
        note: None,
    }];
    let result = super::nest(&j, &options(), &mut |_| {})?;
    checks::complete(&j, &result)?;
    for p in &result.placements {
        let outline = checks::outline(&j.pieces[0], p)?;
        assert!(!checks::conflict(
            &outline,
            &j.fabric.defects[0].polygon,
            0.0
        ));
    }
    let fixed = Placement {
        piece_id: "panel".into(),
        bundle: Some(0),
        ..Default::default()
    };
    assert!(
        super::nest(
            &j,
            &NestOptions {
                fixed: vec![fixed],
                ..options()
            },
            &mut |_| {}
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn portfolio_and_native_warm_start_keep_a_valid_incumbent() -> Result<()> {
    let j = job();
    let first = super::nest(&j, &options(), &mut |_| {})?;
    let result = super::nest(
        &j,
        &NestOptions {
            runs: 3,
            warm_start: Some(first.clone()),
            ..options()
        },
        &mut |_| {},
    )?;
    assert!(result.length <= first.length);
    assert!(beta_marker_core::validate(&j, &result).ok);
    let mut bad = first;
    bad.placements[0].translate = [-100.0, -100.0];
    assert!(
        super::nest(
            &j,
            &NestOptions {
                warm_start: Some(bad),
                ..options()
            },
            &mut |_| {}
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn bundles_follow_s1_m2_and_share_one_direction() -> Result<()> {
    let mut j = job();
    j.fabric.nap = Nap::OneWayPerBundle;
    j.pieces[0].quantity = 1;
    j.pieces[0].size = Some("S".into());
    let mut m = j.pieces[0].clone();
    m.id = "panel-m".into();
    m.size = Some("M".into());
    j.pieces.push(m.clone());
    m.id = "facing-m".into();
    j.pieces.push(m);
    j.order = Some(Order {
        sizes: BTreeMap::from([("S".into(), 1), ("M".into(), 2)]),
    });
    let result = super::nest(
        &j,
        &NestOptions {
            runs: 3,
            ..options()
        },
        &mut |_| {},
    )?;
    checks::complete(&j, &result)?;
    assert_eq!(
        result
            .placements
            .iter()
            .find(|p| p.piece_id == "panel")
            .and_then(|p| p.bundle),
        Some(0)
    );
    for name in ["panel-m", "facing-m"] {
        let bundles: BTreeSet<_> = result
            .placements
            .iter()
            .filter(|p| p.piece_id == name)
            .filter_map(|p| p.bundle)
            .collect();
        assert_eq!(bundles, BTreeSet::from([1, 2]));
    }
    Ok(())
}

#[test]
fn faceless_allocations_keep_total_counts_and_unique_copy_numbers() -> Result<()> {
    let mut j = job();
    j.fabric.allow_flip = true;
    let result = internally_nest(&j, &options(), 1)?;
    assert!(result.placements.iter().all(|p| p.mirrored));
    checks::complete(&j, &result)?;
    assert_eq!(result.placements.len(), 2);
    assert!(beta_marker_core::validate(&j, &result).ok);
    Ok(())
}

#[test]
fn layered_pairs_use_one_placement_and_odd_demand_has_surplus() -> Result<()> {
    let mut j = job();
    j.fabric.layout = Layout::Folded;
    j.pieces[0].quantity = 1;
    j.pieces[0].mirror_quantity = 1;
    let result = super::nest(&j, &options(), &mut |_| {})?;
    assert_eq!(result.placements.len(), 1);
    checks::complete(&j, &result)?;
    j.pieces[0].mirror_quantity = 0;
    let result = super::nest(&j, &options(), &mut |_| {})?;
    assert_eq!(result.placements.len(), 1);
    assert_eq!(
        beta_marker_core::rules::layer_copies(Layout::Folded, &result.placements[0]),
        (1, 0)
    );
    assert!(result.warnings.iter().any(|w| w.contains("surplus")));
    Ok(())
}

#[test]
fn fold_halves_touch_an_exact_fold_edge_without_edge_gap() -> Result<()> {
    for layout in [Layout::Folded, Layout::Tubular] {
        let mut j = job();
        j.fabric.layout = layout;
        j.fabric.width = 21.0;
        j.spacing = Spacing {
            piece_gap: 2.0,
            edge_gap: 1.0,
        };
        j.pieces[0].quantity = 1;
        // Left of this line is the upper half of the symmetric rectangle.
        j.pieces[0].fold_line = Some([[0.0, 20.0], [60.0, 20.0]]);
        let result = super::nest(&j, &options(), &mut |_| {})?;
        assert_eq!(result.placements.len(), 1);
        assert!(result.placements[0].on_fold);
        checks::complete(&j, &result)?;
        assert!((result.piece_area - 1200.0).abs() < 1e-6);
        let mut bad = result;
        bad.placements[0].translate[1] -= 0.0001;
        assert!(checks::geometry(&j, &bad.placements).is_err());
    }
    Ok(())
}

#[test]
fn one_way_faceless_fold_can_use_the_other_symmetric_half() -> Result<()> {
    let mut j = job();
    j.fabric.layout = Layout::Folded;
    j.fabric.width = 21.0;
    j.fabric.nap = Nap::OneWay;
    j.fabric.allow_flip = true;
    j.spacing.edge_gap = 1.0;
    j.pieces[0].quantity = 1;
    j.pieces[0].grain = Some([[0.0, 0.0], [60.0, 0.0]]);
    j.pieces[0].fold_line = Some([[0.0, 20.0], [60.0, 20.0]]);
    let result = super::nest(&j, &options(), &mut |_| {})?;
    assert!(result.placements[0].on_fold && result.placements[0].mirrored);
    checks::complete(&j, &result)?;
    Ok(())
}

#[test]
fn one_way_fold_half_does_not_need_optional_flip() -> Result<()> {
    let mut j = job();
    j.fabric.layout = Layout::Folded;
    j.fabric.width = 21.0;
    j.fabric.nap = Nap::OneWay;
    j.spacing.edge_gap = 1.0;
    j.pieces[0].quantity = 1;
    j.pieces[0].grain = Some([[0., 0.], [60., 0.]]);
    j.pieces[0].fold_line = Some([[0., 20.], [60., 20.]]);
    let result = super::nest(&j, &options(), &mut |_| {})?;
    assert!(result.placements[0].on_fold && result.placements[0].mirrored);
    assert!(beta_marker_core::validate(&j, &result).ok);
    Ok(())
}

#[test]
fn full_symmetric_layers_share_garments_and_survive_warm_fixed_refinement() -> Result<()> {
    let mut j = job();
    j.fabric.layout = Layout::Folded;
    // A shared full cut fits; two separate full cuts cannot sit side by side.
    // This avoids making allocation correctness depend on a tiny guard/timing tie.
    j.fabric.width = 41.;
    j.pieces[0].quantity = 1;
    j.pieces[0].size = Some("M".into());
    j.pieces[0].fold_line = Some([[0., 20.], [60., 20.]]);
    j.order = Some(Order { sizes: BTreeMap::from([("M".into(), 2)]) });
    let mut result = super::nest(&j, &options(), &mut |_| {})?;
    assert_eq!(result.placements.len(), 1);
    assert_eq!(beta_marker_core::rules::placement_bundles(&result.placements[0]), BTreeSet::from([0, 1]));
    assert!(!result.warnings.iter().any(|w| w.contains("surplus")));
    let warm = super::nest(&j, &NestOptions { warm_start: Some(result.clone()), ..options() }, &mut |_| {})?;
    assert!(warm.length <= result.length);
    result.placements[0].locked = true;
    let fixed = result.placements[0].clone();
    let refined = super::nest(&j, &NestOptions { fixed: vec![fixed.clone()], ..options() }, &mut |_| {})?;
    assert_eq!(refined.placements, vec![fixed]);
    assert!(beta_marker_core::validate(&j, &refined).ok);
    Ok(())
}

#[test]
fn asymmetric_layers_never_replace_an_unrequested_hand() -> Result<()> {
    let mut j = job();
    j.fabric.layout = Layout::Tubular;
    j.pieces[0].quantity = 1;
    j.pieces[0].size = Some("M".into());
    j.order = Some(Order { sizes: BTreeMap::from([("M".into(), 2)]) });
    let result = super::nest(&j, &options(), &mut |_| {})?;
    assert_eq!(result.placements.len(), 2);
    assert!(result.placements.iter().all(|p| p.layer_allocations[1].is_none()));
    assert!(beta_marker_core::validate(&j, &result).ok);
    Ok(())
}

#[test]
fn explicit_fixed_faceless_layers_consume_pooled_hands() -> Result<()> {
    let mut j = job();
    j.fabric.layout = Layout::Folded;
    j.fabric.allow_flip = true;
    j.pieces[0].quantity = 1;
    j.pieces[0].mirror_quantity = 1;
    let fixed = Placement {
        piece_id: "panel".into(), bundle: Some(0), locked: true,
        layer_allocations: vec![
            Some(beta_marker_core::LayerAllocation { bundle: 0, mirrored: false }),
            Some(beta_marker_core::LayerAllocation { bundle: 0, mirrored: false }),
        ],
        ..Default::default()
    };
    let result = super::nest(&j, &NestOptions { fixed: vec![fixed.clone()], ..options() }, &mut |_| {})?;
    assert_eq!(result.placements, vec![fixed]);
    assert!(beta_marker_core::validate(&j, &result).ok);
    Ok(())
}

#[test]
fn symmetric_mirror_pair_can_use_two_halves_when_a_full_cut_is_too_wide() -> Result<()> {
    let mut j = job();
    j.fabric.layout = Layout::Tubular;
    j.fabric.width = 21.;
    j.pieces[0].quantity = 1;
    j.pieces[0].mirror_quantity = 1;
    j.pieces[0].fold_line = Some([[0., 20.], [60., 20.]]);
    let result = super::nest(&j, &options(), &mut |_| {})?;
    assert_eq!(result.placements.len(), 2);
    assert!(result.placements.iter().all(|p| p.on_fold));
    assert_eq!(result.placements.iter().map(|p| beta_marker_core::rules::layer_copies(j.fabric.layout, p))
        .fold((0, 0), |(a, b), (c, d)| (a + c, b + d)), (1, 1));
    assert!(beta_marker_core::validate(&j, &result).ok);
    Ok(())
}

#[test]
fn fusing_rectangle_rotates_with_grain_and_keeps_its_margin() -> Result<()> {
    let mut j = job();
    j.pieces[0].outline = vec![[0.0, 0.0], [60.0, 0.0], [0.0, 40.0]];
    j.pieces[0].grain = Some([[0.0, 0.0], [1.0, 1.0]]);
    j.pieces[0].block_margin = Some(3.0);
    let cut = cut_piece(&j.pieces[0])?;
    assert_eq!(cut.outline.len(), 4);
    assert!(area(&cut.outline) > area(&j.pieces[0].outline));
    let result = super::nest(&j, &options(), &mut |_| {})?;
    checks::complete(&j, &result)?;
    Ok(())
}

#[test]
fn shrinkage_uses_effective_coordinates_and_reports_it() -> Result<()> {
    let mut j = job();
    j.fabric.shrinkage = Shrinkage {
        length_pct: 2.0,
        width_pct: 1.0,
    };
    let effective = beta_marker_core::rules::effective_job(&j);
    assert!((effective.pieces[0].outline[1][0] - 61.2).abs() < 1e-6);
    let result = super::nest(&j, &options(), &mut |_| {})?;
    assert!(result.warnings.iter().any(|w| w.contains("effective_job")));
    checks::complete(&effective, &result)?;
    Ok(())
}

#[test]
fn narrow_sliver_and_coincident_shapes_are_not_accepted() {
    let a = vec![
        [0.0, 0.0],
        [10.0000004, 0.0],
        [10.0000004, 10.0],
        [0.0, 10.0],
    ];
    let b = vec![[10.0, 0.0], [20.0, 0.0], [20.0, 10.0], [10.0, 10.0]];
    assert!(checks::conflict(&a, &b, 0.0));
    assert!(checks::conflict(&a, &a, 0.0));
}

#[test]
fn portfolios_fit_both_engine_pools_inside_the_thread_budget() {
    for cores in 1..=64 {
        for runs in 1..=70 {
            for requested in [1, 2, 3, usize::MAX] {
                let (parallel, workers, waves) = thread_budget(cores, runs, Some(requested));
                let total = if cores < 3 {
                    1
                } else if parallel == 1 {
                    1 + 2 * workers
                } else {
                    1 + parallel * (1 + 2 * workers)
                };
                assert!(
                    total <= cores,
                    "{cores} cores, {runs} runs, {workers} workers: {total} threads"
                );
                assert_eq!(waves, runs.div_ceil(parallel));
                assert!(workers <= requested);
            }
        }
    }
    assert_eq!(thread_budget(16, 1, None), (1, 3, 1));
    assert_eq!(thread_budget(16, 3, None), (3, 2, 1));
    assert_eq!(thread_budget(16, 5, None), (5, 1, 1));
}

#[test]
fn new_feature_inputs_reject_nonfinite_and_degenerate_geometry() {
    let mut cases = Vec::new();
    let mut bad = job();
    bad.pieces[0].block_margin = Some(-1.0);
    cases.push(bad);
    let mut bad = job();
    bad.pieces[0].fold_line = Some([[0.0, 0.0], [0.0, 0.0]]);
    cases.push(bad);
    let mut bad = job();
    bad.fabric.shrinkage.length_pct = -100.0;
    cases.push(bad);
    let mut bad = job();
    bad.fabric.shrinkage.width_pct = f64::NAN;
    cases.push(bad);
    let mut bad = job();
    bad.fabric.defects.push(Zone {
        id: "crossed".into(),
        polygon: vec![[0.0, 0.0], [10.0, 10.0], [0.0, 10.0], [10.0, 0.0]],
        note: None,
    });
    cases.push(bad);
    for j in cases {
        assert!(super::nest(&j, &options(), &mut |_| {}).is_err());
    }
    let fixed = Placement {
        piece_id: "panel".into(),
        bundle: Some(0),
        translate: [f64::INFINITY, 0.0],
        ..Default::default()
    };
    assert!(
        super::nest(
            &job(),
            &NestOptions {
                fixed: vec![fixed],
                ..options()
            },
            &mut |_| {}
        )
        .is_err()
    );
}

#[test]
fn faceless_layers_pool_demand_without_unnecessary_surplus() -> Result<()> {
    let mut j = job();
    j.fabric.layout = Layout::Folded;
    j.fabric.allow_flip = true;
    let result = super::nest(&j, &options(), &mut |_| {})?;
    assert_eq!(result.placements.len(), 1);
    assert!(!result.warnings.iter().any(|w| w.contains("surplus")));
    Ok(())
}

#[test]
fn no_order_uses_one_garment_per_size() -> Result<()> {
    let mut j = job();
    j.pieces[0].size = Some("S".into());
    let mut medium = j.pieces[0].clone();
    medium.id = "medium".into();
    medium.size = Some("M".into());
    j.pieces.push(medium);
    let result = super::nest(&j, &options(), &mut |_| {})?;
    assert!(result.placements.iter().filter(|p| p.piece_id == "panel").all(|p| p.bundle == Some(0)));
    assert!(result.placements.iter().filter(|p| p.piece_id == "medium").all(|p| p.bundle == Some(1)));
    assert!(beta_marker_core::validate(&j, &result).ok);
    Ok(())
}

#[test]
fn locked_full_cut_can_replace_a_suggested_fold_half() -> Result<()> {
    let mut j = job();
    j.fabric.layout = Layout::Folded;
    j.pieces[0].quantity = 1;
    j.pieces[0].fold_line = Some([[0.0, 20.0], [60.0, 20.0]]);
    let fixed = Placement {
        piece_id: "panel".into(),
        bundle: Some(0),
        locked: true,
        ..Default::default()
    };
    let result = super::nest(
        &j,
        &NestOptions {
            fixed: vec![fixed.clone()],
            ..options()
        },
        &mut |_| {},
    )?;
    assert_eq!(result.placements, vec![fixed]);
    assert!(result.warnings.iter().any(|w| w.contains("surplus")));
    Ok(())
}

#[test]
fn progress_never_reports_a_candidate_rejected_by_the_independent_gate() {
    let mut j = job();
    j.fabric.shade = Some(beta_marker_core::ShadeRule {
        axis: beta_marker_core::ShadeAxis::Width,
        band_mm: 20.0,
    });
    let fixed = Placement {
        piece_id: "panel".into(),
        bundle: Some(0),
        ..Default::default()
    };
    let mut progress = Vec::new();
    let outcome = super::nest(
        &j,
        &NestOptions {
            fixed: vec![fixed],
            ..options()
        },
        &mut |p| progress.push(p),
    );
    assert!(outcome.is_err());
    assert!(progress.is_empty());
}

#[test]
fn per_piece_flip_overrides_both_fabric_defaults() -> Result<()> {
    let mut j = job();
    j.fabric.allow_flip = false;
    j.pieces[0].flip = Some(true);
    let templates = slots(&j)?;
    assert!(mirror_state(&j, &templates[0], 1));
    let mirrored = Placement {
        piece_id: "panel".into(),
        mirrored: true,
        bundle: Some(0),
        ..Default::default()
    };
    assert!(fixed_placements(&j, &templates, std::slice::from_ref(&mirrored)).is_ok());
    let allowed_result = internally_nest(&j, &options(), 1)?;
    assert!(allowed_result.placements.iter().all(|p| p.mirrored));
    j.fabric.allow_flip = true;
    j.pieces[0].flip = Some(false);
    let templates = slots(&j)?;
    assert!(!mirror_state(&j, &templates[0], 1));
    assert!(fixed_placements(&j, &templates, &[mirrored]).is_err());
    let forbidden_result = internally_nest(&j, &options(), 1)?;
    assert!(forbidden_result.placements.iter().all(|p| !p.mirrored));
    Ok(())
}

#[test]
fn numeric_bundle_sizes_use_the_shared_order() {
    let mut j = job();
    j.order = Some(Order {
        sizes: BTreeMap::from([("100".into(), 1), ("30".into(), 1), ("34".into(), 1)]),
    });
    j.pieces[0].size = Some("100".into());
    assert_eq!(bundles(&j, &j.pieces[0]), vec![2]);
}

#[test]
fn invalid_piece_tilts_are_rejected_before_search() {
    for tilt in [f64::NAN, f64::INFINITY, -0.1, 180.1] {
        let mut j = job();
        j.pieces[0].tilt_deg = Some(tilt);
        assert!(super::nest(&j, &options(), &mut |_| {}).is_err());
    }
}

#[test]
fn fold_halves_can_use_full_two_layer_cuts_to_share_the_width() -> Result<()> {
    let mut j = job();
    j.fabric.layout = Layout::Folded;
    j.pieces[0].quantity = 2;
    j.pieces[0].fold_line = Some([[0.0, 20.0], [60.0, 20.0]]);
    let result = super::nest(&j, &options(), &mut |_| {})?;
    checks::complete(&j, &result)?;
    assert!(
        result.length < 100.0,
        "fold-only construction needs two 60 mm slots"
    );
    assert!(result.placements.iter().any(|p| !p.on_fold));
    assert_eq!(
        result.placements.iter().map(|p| beta_marker_core::rules::layer_copies(j.fabric.layout, p))
            .fold((0, 0), |(a, b), (c, d)| (a + c, b + d)),
        (2, 0)
    );
    let surplus = result.placements.iter().flat_map(|p| &p.layer_allocations).any(Option::is_none);
    assert_eq!(result.warnings.iter().any(|w| w.contains("surplus")), surplus);
    Ok(())
}

#[test]
fn pending_flip_validator_is_reported_without_accepting_the_result() -> Result<()> {
    let mut j = job();
    j.pieces[0].flip = Some(true);
    let placements = vec![
        Placement {
            piece_id: "panel".into(),
            mirrored: true,
            bundle: Some(0),
            translate: [60.0, 0.0],
            ..Default::default()
        },
        Placement {
            piece_id: "panel".into(),
            mirrored: true,
            copy: 1,
            bundle: Some(0),
            translate: [120.0, 0.0],
            ..Default::default()
        },
    ];
    let mut result = MarkerResult {
        format: beta_marker_core::RESULT_FORMAT.into(),
        job_name: j.name.clone(),
        width: j.fabric.width,
        length: 0.0,
        piece_area: 0.0,
        efficiency: 0.0,
        engine: "test".into(),
        seed: None,
        runtime_s: 0.0,
        placements,
        warnings: vec![],
    };
    checks::refresh(&j, &mut result)?;
    checks::complete(&j, &result)?;
    let gate = checks::independently_validate(&j, &result);
    if beta_marker_core::validate(&j, &result).ok {
        gate?;
    } else {
        assert!(gate.unwrap_err().to_string().contains("pending_validator"));
    }
    Ok(())
}
