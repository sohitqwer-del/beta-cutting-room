use super::*;
use beta_marker_core::{Fabric, Piece, RotationRule, Spacing};
fn fixture() -> (Job, MarkerResult) {
    let job = Job {
        format: beta_marker_core::JOB_FORMAT.into(),
        name: "Original editor test".into(),
        fabric: Fabric {
            width: 200.,
            tilt_deg: 3.,
            allow_flip: true,
            ..Default::default()
        },
        spacing: Spacing {
            piece_gap: 2.,
            edge_gap: 1.,
        },
        pieces: vec![Piece {
            id: "part".into(),
            name: "Pocket".into(),
            quantity: 2,
            outline: vec![[0., 0.], [10., 0.], [10., 10.], [0., 10.]],
            rotation: RotationRule::Quarter,
            ..Default::default()
        }],
        ..Default::default()
    };
    let mut result = MarkerResult {
        format: beta_marker_core::RESULT_FORMAT.into(),
        job_name: job.name.clone(),
        width: job.fabric.width,
        length: 0.,
        piece_area: 0.,
        efficiency: 0.,
        useful_piece_area: 0.,
        useful_yield: 0.,
        surplus_copies: 0,
        engine: "test".into(),
        seed: None,
        runtime_s: 0.,
        warnings: vec![],
        placements: vec![
            Placement {
                piece_id: "part".into(),
                translate: [1., 1.],
                bundle: Some(0),
                ..Default::default()
            },
            Placement {
                piece_id: "part".into(),
                copy: 1,
                translate: [40., 1.],
                bundle: Some(0),
                ..Default::default()
            },
        ],
    };
    metrics(&job, &mut result);
    (job, result)
}

#[test]
fn fresh_search_keeps_shorter_checked_marker_without_trusting_cached_lengths() {
    let (j, r) = fixture();
    let mut longer = r.clone();
    longer.placements[1].translate[0] = 80.;
    longer.length = 1.; // Recomputed in Rust; imported metadata cannot win.
    let chosen = choose_search_result(&j, &r, &longer, &[]).unwrap();
    assert!(chosen.retained);
    assert_eq!(chosen.result.placements, r.placements);
    assert_eq!(chosen.result.engine, r.engine);
    let mut stale = r.clone();
    stale.placements.pop();
    stale.length = 0.;
    let chosen = choose_search_result(&j, &stale, &longer, &[]).unwrap();
    assert!(!chosen.retained);
    assert!(chosen.result.length >= 90.);
    let mut invalid = longer.clone();
    invalid.placements[1] = invalid.placements[0].clone();
    assert!(choose_search_result(&j, &r, &invalid, &[]).is_err());
    let moved_lock = longer.placements[1].clone();
    let chosen = choose_search_result(&j, &r, &longer, &[moved_lock]).unwrap();
    assert!(!chosen.retained);
}

#[test]
fn improve_area_preserves_outside_and_locked_records() {
    let (j, mut r) = fixture();
    r.placements[0].locked = true;
    let original = r.clone();
    let out = run(
        &j,
        &r,
        &[],
        Action::ImproveRegion {
            from: [30., 0.],
            to: [80., 100.],
        },
    )
    .unwrap();
    assert_eq!(out.selected, vec![1]);
    assert_eq!(out.result.placements[0], original.placements[0]);
    assert!(out.result.placements[1].translate[0] >= 30. - EPS);
    assert!(out.result.length < original.length);
    assert!(beta_marker_core::validate(&j, &out.result).ok);
    assert_eq!(r, original);
}

#[test]
fn improve_area_ignores_crossing_pieces_and_respects_defects() {
    let (mut j, r) = fixture();
    j.fabric.defects.push(beta_marker_core::Zone {
        id: "flaw".into(),
        polygon: vec![[30., 0.], [35., 0.], [35., 200.], [30., 200.]],
        note: None,
    });
    let out = run(
        &j,
        &r,
        &[],
        Action::ImproveRegion {
            from: [5., 0.],
            to: [100., 100.],
        },
    )
    .unwrap();
    assert_eq!(out.selected, vec![1]);
    assert_eq!(out.result.placements[0], r.placements[0]);
    assert!(out.result.placements[1].translate[0] >= 35. - EPS);
    assert!(beta_marker_core::validate(&j, &out.result).ok);
}

#[test]
fn improve_area_rejects_incomplete_markers_and_bad_bounds() {
    let (j, mut r) = fixture();
    assert!(
        run(
            &j,
            &r,
            &[],
            Action::ImproveRegion {
                from: [0., 0.],
                to: [f64::NAN, 100.]
            }
        )
        .is_err()
    );
    r.placements.pop();
    assert!(
        run(
            &j,
            &r,
            &[],
            Action::ImproveRegion {
                from: [0., 0.],
                to: [100., 100.]
            }
        )
        .is_err()
    );
}
fn run(j: &Job, r: &MarkerResult, selected: &[usize], action: Action) -> Result<Output, String> {
    apply(
        j,
        r,
        Request {
            selected: selected.to_vec(),
            action,
        },
    )
}
#[test]
fn bump_all_directions_with_buffers() {
    let (mut j, mut r) = fixture();
    j.pieces[0].buffer = 1.;
    r.placements[0].translate = [2., 2.];
    r.placements[1].translate = [40., 2.];
    let s = Settings {
        table_length: Some(100.),
        ..Default::default()
    };
    assert!((bump(&j, &mut r, &[1], Direction::Left, &s, false).unwrap() - 24.).abs() < EPS);
    assert!((r.placements[1].translate[0] - 16.).abs() < EPS);
    assert!((bump(&j, &mut r, &[1], Direction::Right, &s, false).unwrap() - 74.).abs() < EPS);
    assert!((bump(&j, &mut r, &[1], Direction::Up, &s, false).unwrap() - 186.).abs() < EPS);
    assert!((bump(&j, &mut r, &[1], Direction::Down, &s, false).unwrap() - 186.).abs() < EPS);
}
#[test]
fn swept_contact_does_not_tunnel_through_thin_polygon() {
    let (mut j, mut r) = fixture();
    j.spacing.piece_gap = 0.;
    j.pieces.push(Piece {
        id: "sliver".into(),
        name: "Sliver".into(),
        quantity: 1,
        outline: vec![[0., 0.], [0.001, 0.], [0.001, 50.], [0., 50.]],
        ..Default::default()
    });
    r.placements[0] = Placement {
        piece_id: "sliver".into(),
        translate: [25., 1.],
        ..Default::default()
    };
    let d = bump(
        &j,
        &mut r,
        &[1],
        Direction::Left,
        &Settings::default(),
        false,
    )
    .unwrap();
    assert!((d - 14.999).abs() < EPS);
}
#[test]
fn bump_uses_concave_cut_line_not_box() {
    let (mut j, mut r) = fixture();
    j.spacing.piece_gap = 0.;
    j.pieces[0].outline = vec![
        [0., 0.],
        [30., 0.],
        [30., 5.],
        [5., 5.],
        [5., 30.],
        [0., 30.],
    ];
    j.pieces.push(Piece {
        id: "small".into(),
        name: "Small".into(),
        quantity: 1,
        outline: vec![[0., 0.], [3., 0.], [3., 3.], [0., 3.]],
        ..Default::default()
    });
    r.placements[1] = Placement {
        piece_id: "small".into(),
        translate: [40., 10.],
        ..Default::default()
    };
    let d = bump(
        &j,
        &mut r,
        &[1],
        Direction::Left,
        &Settings::default(),
        false,
    )
    .unwrap();
    assert!((d - 34.).abs() < EPS);
}
#[test]
fn overlap_area_and_resolve() {
    let (j, mut r) = fixture();
    r.placements[1].translate = [6., 1.];
    let i = inspect(&j, &r).unwrap();
    assert_eq!(i.pairs.len(), 1);
    assert!((i.pairs[0].overlap_mm2 - 50.).abs() < EPS);
    assert!(
        bump(
            &j,
            &mut r,
            &[1],
            Direction::Right,
            &Settings {
                table_length: Some(100.),
                ..Default::default()
            },
            false
        )
        .is_err()
    );
    bump(
        &j,
        &mut r,
        &[1],
        Direction::Right,
        &Settings {
            table_length: Some(100.),
            ..Default::default()
        },
        true,
    )
    .unwrap();
    assert!(inspect(&j, &r).unwrap().pairs.is_empty());
}
#[test]
fn group_move_lock_swap_and_rotation_are_atomic() {
    let (j, r) = fixture();
    let o = run(&j, &r, &[0, 1], Action::Move { delta: [5., 5.] }).unwrap();
    assert_eq!(o.result.placements[0].translate, [6., 6.]);
    assert_eq!(o.result.placements[1].translate, [45., 6.]);
    let o = run(&j, &r, &[0, 1], Action::Swap).unwrap();
    assert_eq!(o.result.placements[0].translate, [40., 1.]);
    let mut locked = r.clone();
    locked.placements[1].locked = true;
    assert!(run(&j, &locked, &[0, 1], Action::Move { delta: [1., 0.] }).is_err());
    assert!(run(&j, &r, &[0], Action::Swap).is_err());
    assert_eq!(r.placements[0].translate, [1., 1.]);
}
#[test]
fn allowed_rotation_tilt_flip_pivot() {
    let (j, mut r) = fixture();
    r.placements[0].translate = [30., 30.];
    r.placements[1].translate = [60., 30.];
    let o = run(&j, &r, &[0, 1], Action::Rotate { degrees: 90. }).unwrap();
    assert_eq!(o.result.placements[0].rotation_deg, 90.);
    assert_eq!(o.result.placements[1].rotation_deg, 90.);
    let o = run(&j, &r, &[0], Action::NextRotation).unwrap();
    assert_eq!(o.result.placements[0].rotation_deg, 90.);
    assert!(run(&j, &r, &[0], Action::Tilt { degrees: 3. }).is_ok());
    assert!(run(&j, &r, &[0], Action::Tilt { degrees: 4. }).is_err());
    let o = run(&j, &r, &[0], Action::Flip).unwrap();
    assert!(o.result.placements[0].mirrored);
    let mut paired = r.clone();
    paired.placements[1].mirrored = true;
    paired.placements[1].copy = 0;
    paired.placements[1].translate[0] = 80.;
    let o = run(&j, &paired, &[0], Action::Flip).unwrap();
    assert_ne!(o.result.placements[0].copy, o.result.placements[1].copy);
    let mut no = j.clone();
    no.fabric.allow_flip = false;
    assert!(run(&no, &r, &[0], Action::Flip).is_err());
}
#[test]
fn compact_close_end_and_group_contact() {
    let (j, r) = fixture();
    let o = run(&j, &r, &[], Action::Compact).unwrap();
    assert!((o.result.placements[1].translate[0] - 13.).abs() < EPS);
    let o = run(&j, &r, &[], Action::CloseEnd).unwrap();
    assert!((o.result.placements[1].translate[0] - 13.).abs() < EPS);
    let mut r = r;
    for p in &mut r.placements {
        p.translate[0] += 30.;
    }
    bump(
        &j,
        &mut r,
        &[0, 1],
        Direction::Left,
        &Settings::default(),
        false,
    )
    .unwrap();
    assert_eq!(r.placements[0].translate[0], 1.);
    assert_eq!(r.placements[1].translate[0], 40.);
}
#[test]
fn sections_reject_crossing_and_stop_bump() {
    let (j, r) = fixture();
    assert!(
        run(
            &j,
            &r,
            &[],
            Action::Section {
                x: 5.,
                remove: false
            }
        )
        .is_err()
    );
    let o = run(
        &j,
        &r,
        &[],
        Action::Section {
            x: 25.,
            remove: false,
        },
    )
    .unwrap();
    let o = run(
        &o.job,
        &o.result,
        &[1],
        Action::Bump {
            direction: Direction::Left,
        },
    )
    .unwrap();
    assert!((o.result.placements[1].translate[0] - 25.).abs() < EPS);
    assert!(run(&o.job, &o.result, &[1], Action::Move { delta: [-1., 0.] }).is_err());
    let o = run(
        &o.job,
        &o.result,
        &[],
        Action::Section {
            x: 25.,
            remove: true,
        },
    )
    .unwrap();
    assert!(settings(&o.job).unwrap().sections.is_empty());
}
#[test]
fn end_length_measure_selection() {
    let (j, r) = fixture();
    assert!(run(&j, &r, &[], Action::Length { length: 49. }).is_err());
    let o = run(&j, &r, &[], Action::Length { length: 100. }).unwrap();
    assert!((inspect(&o.job, &o.result).unwrap().table_efficiency - 0.01).abs() < EPS);
    assert_eq!(
        run(
            &j,
            &r,
            &[],
            Action::Measure {
                from: [0., 0.],
                to: [3., 4.]
            }
        )
        .unwrap()
        .measurement,
        Some(5.)
    );
    assert_eq!(
        run(
            &j,
            &r,
            &[],
            Action::Rectangle {
                from: [0., 0.],
                to: [20., 20.]
            }
        )
        .unwrap()
        .selected,
        vec![0]
    );
    for (field, value) in [("name", "poc"), ("bundle", "1"), ("all", "")] {
        assert_eq!(
            run(
                &j,
                &r,
                &[],
                Action::Select {
                    field: field.into(),
                    value: value.into()
                }
            )
            .unwrap()
            .selected,
            vec![0, 1]
        );
    }
}
#[test]
fn remove_place_fill_and_copy_identity() {
    let (j, r) = fixture();
    let o = run(&j, &r, &[0], Action::Remove).unwrap();
    assert_eq!(
        unplaced(&j, &o.result, &settings(&o.job).unwrap())
            .unwrap()
            .len(),
        1
    );
    let o = run(
        &o.job,
        &o.result,
        &[],
        Action::Place {
            unplaced: 0,
            at: [20., 30.],
        },
    )
    .unwrap();
    assert_eq!(o.result.placements.len(), 2);
    assert!(
        unplaced(&j, &o.result, &settings(&o.job).unwrap())
            .unwrap()
            .is_empty()
    );
    let o = run(&j, &r, &[0, 1], Action::Remove).unwrap();
    let o = run(&o.job, &o.result, &[], Action::FillGaps).unwrap();
    assert_eq!(o.result.placements.len(), 2);
    assert!(inspect(&o.job, &o.result).unwrap().validation.ok);
    assert_ne!(o.result.placements[0].copy, o.result.placements[1].copy);
}
#[test]
fn demand_for_folded_and_faceless_layout() {
    let (mut j, mut r) = fixture();
    j.fabric.layout = Layout::Folded;
    j.pieces[0].quantity = 1;
    j.pieces[0].mirror_quantity = 1;
    r.placements.truncate(1);
    assert!(unplaced(&j, &r, &Settings::default()).unwrap().is_empty());
    r.placements.clear();
    assert_eq!(unplaced(&j, &r, &Settings::default()).unwrap().len(), 1);
}
#[test]
fn reuse_requires_unchanged_shapes_and_ratio() {
    let (j, r) = fixture();
    let mut renamed = j.clone();
    renamed.name = "Another run".into();
    renamed.meta.insert(
        "source".into(),
        serde_json::json!("A later import of the same style"),
    );
    assert_eq!(reuse(&renamed, &j, &r).unwrap().job_name, "Another run");
    renamed.pieces[0].outline[0][0] += 0.1;
    assert!(reuse(&renamed, &j, &r).is_err());
}
#[test]
fn defect_boundary_and_bad_input() {
    let (mut j, mut r) = fixture();
    j.fabric.defects.push(beta_marker_core::Zone {
        id: "hole".into(),
        polygon: vec![[25., 0.], [26., 0.], [26., 20.], [25., 20.]],
        ..Default::default()
    });
    bump(
        &j,
        &mut r,
        &[1],
        Direction::Left,
        &Settings::default(),
        false,
    )
    .unwrap();
    assert!((r.placements[1].translate[0] - 26.).abs() < EPS);
    assert!(run(&j, &r, &[99], Action::Flip).is_err());
    assert!(
        run(
            &j,
            &r,
            &[1],
            Action::Move {
                delta: [f64::NAN, 0.]
            }
        )
        .is_err()
    );
}
#[test]
fn corrupted_metadata_counts_and_internal_group_overlap_are_rejected() {
    let (mut j, r) = fixture();
    j.pieces[0].quantity = u32::MAX;
    assert!(inspect(&j, &r).is_err());
    assert!(run(&j, &r, &[0], Action::Flip).is_err());
    let (mut j, mut r) = fixture();
    r.placements[1].translate = [6., 1.];
    assert!(
        bump(
            &j,
            &mut r,
            &[0, 1],
            Direction::Right,
            &Settings {
                table_length: Some(100.),
                ..Default::default()
            },
            true
        )
        .is_err()
    );
    save_settings(
        &mut j,
        &Settings {
            table_length: Some(8.),
            ..Default::default()
        },
    )
    .unwrap();
    let check = inspect(&j, &r).unwrap();
    assert_eq!(check.end_violations, vec![0, 1]);
    assert!(!check.geometry_ok);
    assert!(reuse(&j, &j, &r).is_err());
}
#[test]
fn size_filter_shrinkage_blocks_and_fold_edges_use_effective_cut_geometry() {
    let (mut j, mut r) = fixture();
    j.pieces[0].size = Some("M".into());
    assert_eq!(
        run(
            &j,
            &r,
            &[],
            Action::Select {
                field: "size".into(),
                value: "M".into()
            }
        )
        .unwrap()
        .selected,
        vec![0, 1]
    );
    j.fabric.shrinkage.length_pct = 10.;
    j.pieces[0].block_margin = Some(2.);
    r.placements[0].translate = [4., 4.];
    r.placements[1].translate = [40., 4.];
    let o = run(
        &j,
        &r,
        &[1],
        Action::Bump {
            direction: Direction::Left,
        },
    )
    .unwrap();
    assert!((o.result.placements[1].translate[0] - 21.).abs() < EPS);
    let (mut j, mut r) = fixture();
    j.fabric.layout = Layout::Folded;
    r.placements.truncate(1);
    let o = run(
        &j,
        &r,
        &[0],
        Action::Bump {
            direction: Direction::Up,
        },
    )
    .unwrap();
    assert!((o.result.placements[0].translate[1] - 190.).abs() < EPS);
    j.fabric.layout = Layout::Tubular;
    let o = run(
        &j,
        &r,
        &[0],
        Action::Bump {
            direction: Direction::Down,
        },
    )
    .unwrap();
    assert!(o.result.placements[0].translate[1].abs() < EPS);
}
#[test]
fn diagonal_clearance_uses_circular_gap_not_square_offsets() {
    let (mut j, mut r) = fixture();
    j.spacing.piece_gap = 6.;
    r.placements[1].translate = [40., 16.];
    bump(
        &j,
        &mut r,
        &[1],
        Direction::Left,
        &Settings::default(),
        false,
    )
    .unwrap();
    assert!((r.placements[1].translate[0] - (11. + 11_f64.sqrt())).abs() < EPS);
    assert!(inspect(&j, &r).unwrap().pairs.is_empty());
}
#[test]
fn new_unplaced_copies_use_size_bundles_and_physical_fold_counts() {
    let (mut j, mut r) = fixture();
    r.placements.clear();
    j.pieces[0].size = Some("M".into());
    let mut l = j.pieces[0].clone();
    l.id = "large".into();
    l.size = Some("L".into());
    j.pieces.push(l);
    let copies = unplaced(&j, &r, &Settings::default()).unwrap();
    assert!(
        copies
            .iter()
            .filter(|p| p.piece_id == "part")
            .all(|p| p.bundle == Some(0))
    );
    assert!(
        copies
            .iter()
            .filter(|p| p.piece_id == "large")
            .all(|p| p.bundle == Some(1))
    );
    j.pieces.truncate(1);
    j.pieces[0].mirror_quantity = 2;
    j.fabric.layout = Layout::Folded;
    j.order = Some(beta_marker_core::Order {
        sizes: BTreeMap::from([("M".into(), 2)]),
    });
    let copies = unplaced(&j, &r, &Settings::default()).unwrap();
    assert_eq!(copies.len(), 4);
    assert_eq!(copies.iter().filter(|p| p.bundle == Some(0)).count(), 2);
    assert_eq!(copies.iter().filter(|p| p.bundle == Some(1)).count(), 2);
}
#[test]
fn placing_consumes_the_removal_record_so_later_edits_keep_their_new_orientation() {
    let (j, r) = fixture();
    let o = run(&j, &r, &[1], Action::Remove).unwrap();
    assert_eq!(settings(&o.job).unwrap().removed.len(), 1);
    let o = run(
        &o.job,
        &o.result,
        &[],
        Action::Place {
            unplaced: 0,
            at: [40., 30.],
        },
    )
    .unwrap();
    assert!(settings(&o.job).unwrap().removed.is_empty());
    let o = run(&o.job, &o.result, &[1], Action::Flip).unwrap();
    let o = run(&o.job, &o.result, &[1], Action::Remove).unwrap();
    assert!(unplaced(&o.job, &o.result, &settings(&o.job).unwrap()).unwrap()[0].mirrored);
}
/// Deterministic 300-piece benchmark. Run release with --ignored --nocapture.
#[test]
#[ignore = "performance measurement; run in release on the assigned machine"]
fn benchmark_bump_300() {
    let (mut j, mut r) = fixture();
    j.pieces[0].quantity = 300;
    j.fabric.width = 500.;
    r.placements = (0..300)
        .map(|i| Placement {
            piece_id: "part".into(),
            copy: i,
            translate: [1. + f64::from(i % 30) * 30., 1. + f64::from(i / 30) * 30.],
            bundle: Some(0),
            ..Default::default()
        })
        .collect();
    metrics(&j, &mut r);
    let mut ms = vec![];
    for _ in 0..100 {
        let mut r = r.clone();
        let start = std::time::Instant::now();
        bump(
            &j,
            &mut r,
            &[299],
            Direction::Left,
            &Settings::default(),
            false,
        )
        .unwrap();
        ms.push(start.elapsed().as_secs_f64() * 1000.);
    }
    ms.sort_by(f64::total_cmp);
    println!(
        "bump_300 native: median={:.3}ms p95={:.3}ms max={:.3}ms",
        ms[50], ms[95], ms[99]
    );
    assert!(ms[95] < 20., "p95 must be below 20 ms");
}
