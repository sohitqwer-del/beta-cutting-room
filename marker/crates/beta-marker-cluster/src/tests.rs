use super::*;
use beta_marker_core::geometry::{area, transform_points};
use beta_marker_core::{Fabric, JOB_FORMAT, Piece, Placement, RotationRule, Spacing};

fn triangle_job() -> Job {
    Job {
        format: JOB_FORMAT.into(),
        name: "triangle pairs".into(),
        fabric: Fabric {
            width: 210.0,
            ..Default::default()
        },
        spacing: Spacing {
            piece_gap: 0.5,
            edge_gap: 1.0,
        },
        pieces: vec![Piece {
            id: "panel".into(),
            name: "Panel".into(),
            quantity: 4,
            rotation: RotationRule::Flip180,
            outline: vec![[0.0, 0.0], [100.0, 0.0], [0.0, 100.0]],
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn opts() -> NestOptions {
    NestOptions {
        time_limit_s: 0.08,
        seed: Some(1),
        workers: Some(2),
        ..Default::default()
    }
}

#[test]
fn exact_distance_rejects_overlap_and_insufficient_gap() {
    let a = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    assert!(!geometry::separated(&a, &a, 0.0));
    assert!(!geometry::separated(
        &a,
        &transform_points(&a, false, 0.0, [10.5, 0.0]),
        1.0
    ));
    assert!(geometry::separated(
        &a,
        &transform_points(&a, false, 0.0, [11.01, 0.0]),
        1.0
    ));
}

#[test]
fn pair_hull_is_compact_and_contains_both_members() -> Result<()> {
    let job = triangle_job();
    let prepared = cluster::prepare(&job, 0.5)?;
    assert_eq!(prepared.pairs, 2);
    assert_eq!(prepared.job.pieces.len(), 1);
    assert_eq!(prepared.job.pieces[0].quantity, 2);
    assert!(area(&prepared.job.pieces[0].outline) < 12000.0);
    for members in &prepared.mapping[0] {
        let a = &members[0];
        let b = &members[1];
        let world = |m: &cluster::Member| {
            transform_points(
                &job.pieces[m.piece].outline,
                m.placement.mirrored,
                m.placement.rotation_deg,
                m.placement.translate,
            )
        };
        assert!(geometry::separated(
            &world(a),
            &world(b),
            job.spacing.piece_gap
        ));
        use geo::Contains;
        let envelope = geometry::polygon(&prepared.job.pieces[0].outline);
        for member in members {
            assert!(envelope.contains(&geometry::polygon(&world(member))));
        }
    }
    Ok(())
}

#[test]
fn congruent_physical_ids_share_search_and_keep_original_identities() -> Result<()> {
    let mut job = triangle_job();
    job.pieces[0].quantity = 2;
    let mut second = job.pieces[0].clone();
    second.id = "other physical IDs".into();
    job.pieces.push(second);
    let prepared = cluster::prepare(&job, 0.5)?;
    assert_eq!(prepared.pairs, 2);
    assert_eq!(prepared.job.pieces.len(), 1);
    let composite = nest(&prepared.job, &opts())?;
    let expanded = cluster::expand(&job, &prepared, composite)?;
    assert!(beta_marker_core::validate(&job, &expanded).ok);
    for piece in &job.pieces {
        assert_eq!(
            expanded
                .placements
                .iter()
                .filter(|p| p.piece_id == piece.id)
                .count(),
            2
        );
    }
    Ok(())
}

#[test]
fn expansion_preserves_mirror_copy_bundle_and_grain() -> Result<()> {
    let mut job = triangle_job();
    job.pieces[0].quantity = 2;
    job.pieces[0].mirror_quantity = 2;
    job.pieces[0].grain = Some([[0.0, 0.0], [0.0, 1.0]]);
    let prepared = cluster::prepare(&job, 0.4)?;
    let composite = nest(&prepared.job, &opts())?;
    let result = cluster::expand(&job, &prepared, composite)?;
    assert!(beta_marker_core::validate(&job, &result).ok);
    assert_eq!(result.placements.len(), 4);
    for mirrored in [false, true] {
        let mut copies: Vec<_> = result
            .placements
            .iter()
            .filter(|p| p.mirrored == mirrored)
            .map(|p| p.copy)
            .collect();
        copies.sort();
        assert_eq!(copies, vec![0, 1]);
    }
    assert!(result.placements.iter().all(|p| p.bundle == Some(0)));
    Ok(())
}

#[test]
fn one_way_and_member_rotation_intersection_are_preserved() -> Result<()> {
    let mut job = triangle_job();
    job.fabric.nap = Nap::OneWay;
    let result = nest_clustered(&job, &opts(), Strategy::Cluster)?;
    assert!(
        result
            .placements
            .iter()
            .all(|p| p.rotation_deg.abs() < 1e-3)
    );
    assert!(beta_marker_core::validate(&job, &result).ok);
    Ok(())
}

#[test]
fn empty_space_insertion_fills_a_concave_pocket() -> Result<()> {
    let mut job = triangle_job();
    job.pieces[0].outline = vec![
        [0.0, 0.0],
        [100.0, 0.0],
        [100.0, 30.0],
        [30.0, 30.0],
        [30.0, 100.0],
        [0.0, 100.0],
    ];
    job.pieces[0].quantity = 1;
    job.pieces.push(Piece {
        id: "pocket".into(),
        name: "Pocket".into(),
        quantity: 1,
        outline: vec![[0.0, 0.0], [20.0, 0.0], [20.0, 20.0], [0.0, 20.0]],
        ..Default::default()
    });
    let large = vec![Placement {
        piece_id: "panel".into(),
        translate: [2.0, 2.0],
        bundle: Some(0),
        ..Default::default()
    }];
    let small = vec![Placement {
        piece_id: "pocket".into(),
        bundle: Some(0),
        ..Default::default()
    }];
    let result = fill::insert(&job, large, small, 0.3)?;
    assert!(result.length <= 102.0 + 1e-6);
    assert!(beta_marker_core::validate(&job, &result).ok);
    Ok(())
}

#[test]
fn all_strategies_pass_independent_validation() -> Result<()> {
    let mut job = triangle_job();
    job.pieces.push(Piece {
        id: "cuff".into(),
        name: "Cuff".into(),
        quantity: 2,
        outline: vec![[0.0, 0.0], [10.0, 0.0], [10.0, 5.0], [0.0, 5.0]],
        ..Default::default()
    });
    for strategy in [
        Strategy::Plain,
        Strategy::Cluster,
        Strategy::ClusterContour,
        Strategy::Aggregate,
        Strategy::Restart,
        Strategy::GapFill,
        Strategy::GapFillFixed,
        Strategy::TwoStage,
        Strategy::TwoStageFixed,
        Strategy::Auto,
        Strategy::Race,
    ] {
        let result = nest_clustered(&job, &opts(), strategy)?;
        assert!(beta_marker_core::validate(&job, &result).ok, "{strategy:?}");
    }
    Ok(())
}

#[test]
fn connected_envelope_covers_members_without_filling_their_hull() -> Result<()> {
    let a = vec![[0.0, 0.0], [20.0, 0.0], [20.0, 10.0], [0.0, 10.0]];
    let b = transform_points(&a, false, 0.0, [22.0, 20.0]);
    let envelope = geometry::connected_envelope(&a, &b)
        .ok_or_else(|| anyhow::anyhow!("no connected envelope"))?;
    use geo::Covers;
    assert!(geometry::polygon(&envelope).covers(&geometry::polygon(&a)));
    assert!(geometry::polygon(&envelope).covers(&geometry::polygon(&b)));
    let mut both = a;
    both.extend(b);
    assert!(area(&envelope) < area(&geometry::hull(&both)) * 0.75);
    Ok(())
}

#[test]
fn aggregate_keeps_independent_members_and_one_way_quarter_turns() -> Result<()> {
    let mut job = triangle_job();
    job.fabric.nap = Nap::OneWay;
    job.pieces[0].rotation = RotationRule::Quarter;
    let mut other = job.pieces[0].clone();
    other.id = "distinct physical ID".into();
    job.pieces.push(other);
    let prepared = cluster::prepare_mode(&job, 0.01, cluster::Mode::Aggregate)?;
    assert_eq!(prepared.pairs, 0);
    assert_eq!(prepared.job.pieces.len(), 1);
    assert_eq!(prepared.job.pieces[0].quantity, 8);
    let allowed =
        beta_marker_core::allowed_rotations(&prepared.job, &prepared.job.pieces[0], false);
    assert!(allowed.allows(0.0) && allowed.allows(90.0));
    assert!(!allowed.allows(180.0));
    let result = nest_clustered(&job, &opts(), Strategy::Aggregate)?;
    assert!(beta_marker_core::validate(&job, &result).ok);
    assert_eq!(result.placements.len(), 8);
    Ok(())
}

#[test]
fn fixed_placements_and_shrinkage_use_safe_paths() -> Result<()> {
    let mut job = triangle_job();
    job.fabric.shrinkage.length_pct = 2.0;
    job.fabric.shrinkage.width_pct = 1.0;
    let initial = nest_clustered(&job, &opts(), Strategy::Cluster)?;
    assert!(
        initial
            .warnings
            .iter()
            .any(|w| w.starts_with("Shrinkage applied:"))
    );
    let fixed = initial.placements[0].clone();
    let mut options = opts();
    options.fixed = vec![fixed.clone()];
    options.warm_start = Some(initial);
    let result = nest_clustered(&job, &options, Strategy::Auto)?;
    assert!(result.placements.contains(&fixed));
    assert!(beta_marker_core::validate(&job, &result).ok);
    Ok(())
}

#[test]
fn invalid_budgets_and_invalid_warm_starts_return_errors() -> Result<()> {
    let job = triangle_job();
    for seconds in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let mut options = opts();
        options.time_limit_s = seconds;
        assert!(nest_clustered(&job, &options, Strategy::Auto).is_err());
    }
    let mut options = opts();
    let mut result = nest(&job, &options)?;
    result.placements.clear();
    options.warm_start = Some(result);
    assert!(nest_clustered(&job, &options, Strategy::Cluster).is_err());
    Ok(())
}

#[test]
fn input_overflow_nonfinite_grain_and_crossing_edges_return_errors() {
    let mut job = triangle_job();
    job.pieces[0].quantity = u32::MAX;
    job.pieces[0].mirror_quantity = 1;
    assert!(nest_clustered(&job, &opts(), Strategy::Cluster).is_err());
    let mut job = triangle_job();
    job.pieces[0].grain = Some([[0.0, 0.0], [f64::NAN, 1.0]]);
    assert!(nest_clustered(&job, &opts(), Strategy::Cluster).is_err());
    let mut job = triangle_job();
    job.pieces[0].outline = vec![
        [0.0, 0.0],
        [100.0, 100.0],
        [0.0, 100.0],
        [100.0, 0.0],
        [150.0, 0.0],
    ];
    assert!(nest_clustered(&job, &opts(), Strategy::Cluster).is_err());
    let mut job = triangle_job();
    job.fabric.shrinkage.length_pct = f64::MAX;
    assert!(nest_clustered(&job, &opts(), Strategy::Cluster).is_err());
}

#[test]
fn small_piece_threshold_ignores_undemanded_geometry() {
    let mut job = triangle_job();
    job.pieces.push(Piece {
        id: "inactive".into(),
        quantity: 0,
        outline: vec![[0.0, 0.0], [1000.0, 0.0], [1000.0, 1000.0], [0.0, 1000.0]],
        ..Default::default()
    });
    job.pieces.push(Piece {
        id: "cuff".into(),
        quantity: 1,
        outline: vec![[0.0, 0.0], [10.0, 0.0], [10.0, 5.0], [0.0, 5.0]],
        ..Default::default()
    });
    let small = fill::small_ids(&job);
    assert!(!small.contains("panel"));
    assert!(small.contains("cuff"));
}

#[test]
fn restart_retains_a_valid_warm_incumbent() -> Result<()> {
    let mut job = triangle_job();
    job.pieces.push(Piece {
        id: "restart cuff".into(),
        quantity: 2,
        outline: vec![[0.0, 0.0], [10.0, 0.0], [10.0, 5.0], [0.0, 5.0]],
        ..Default::default()
    });
    assert!(!fill::small_ids(&job).is_empty());
    let mut options = opts();
    options.time_limit_s = 0.5;
    let previous = nest(&job, &options)?;
    let length = previous.length;
    options.warm_start = Some(previous);
    let result = nest_clustered(&job, &options, Strategy::Restart)?;
    assert!(beta_marker_core::validate(&job, &result).ok);
    assert!(result.length <= length + 1e-7);
    Ok(())
}

#[test]
fn unequal_buffers_and_finite_length_limits_are_validated() -> Result<()> {
    let mut job = triangle_job();
    job.pieces[0].buffer = 0.7;
    let mut second = job.pieces[0].clone();
    second.id = "second panel".into();
    second.quantity = 2;
    second.buffer = 1.2;
    job.pieces.push(second);
    let result = nest_clustered(&job, &opts(), Strategy::Cluster)?;
    assert!(beta_marker_core::validate(&job, &result).ok);
    job.fabric.max_length = Some(1.0);
    assert!(nest_clustered(&job, &opts(), Strategy::Cluster).is_err());
    Ok(())
}
