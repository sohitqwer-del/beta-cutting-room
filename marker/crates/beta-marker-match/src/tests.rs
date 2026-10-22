use super::*;
use beta_marker_core::*;

fn opts() -> NestOptions {
    NestOptions {
        time_limit_s: 0.06,
        seed: Some(42),
        workers: Some(2),
        ..Default::default()
    }
}

#[test]
fn shared_layer_matching_and_shade_include_the_second_garment() {
    let mut j = job();
    j.fabric.layout = Layout::Folded;
    let mut r = result(&j, &[[0., 0.], [80., 40.]]);
    r.placements[0].layer_allocations = vec![
        Some(LayerAllocation {
            bundle: 0,
            mirrored: false,
        }),
        Some(LayerAllocation {
            bundle: 1,
            mirrored: true,
        }),
    ];
    r.placements[1].layer_allocations = vec![
        Some(LayerAllocation {
            bundle: 0,
            mirrored: false,
        }),
        Some(LayerAllocation {
            bundle: 1,
            mirrored: true,
        }),
    ];
    assert!(match_errors(&j, &r).is_empty());
    r.placements[1].layer_allocations[1] = None;
    assert!(
        match_errors(&j, &r)
            .iter()
            .any(|e| e.code == "match_partner")
    );
    let mut second = r.placements[1].clone();
    second.copy = 1;
    second.bundle = Some(1);
    second.layer_allocations.clear();
    second.translate = [165., 40.];
    r.placements.push(second);
    assert!(match_errors(&j, &r).iter().any(|e| e.code == "match"));
    j.fabric.shade = Some(ShadeRule {
        axis: ShadeAxis::Length,
        band_mm: 150.,
    });
    assert!(
        checks::shade_errors(&j, &r)
            .iter()
            .any(|e| e.code == "shade")
    );
}

fn rect(id: &str, width: f64, height: f64) -> Piece {
    Piece {
        id: id.into(),
        name: id.into(),
        quantity: 1,
        outline: vec![[0.0, 0.0], [width, 0.0], [width, height], [0.0, height]],
        match_points: vec![MatchPoint {
            id: "p".into(),
            point: [0.0; 2],
        }],
        rotation: RotationRule::Fixed,
        ..Default::default()
    }
}

fn reference(id: &str) -> PointRef {
    PointRef {
        piece_id: id.into(),
        point_id: "p".into(),
    }
}

pub(crate) fn job() -> Job {
    Job {
        format: JOB_FORMAT.into(),
        name: "matching rectangles".into(),
        fabric: Fabric {
            width: 200.0,
            repeat: Some(Repeat {
                period_x: Some(40.0),
                period_y: Some(40.0),
                tolerance_mm: 0.5,
                ..Default::default()
            }),
            ..Default::default()
        },
        pieces: vec![rect("a", 50.0, 35.0), rect("b", 50.0, 35.0)],
        match_rules: vec![MatchRule {
            a: reference("a"),
            b: Some(reference("b")),
            axes: MatchAxes::Both,
            offset: [0.0; 2],
        }],
        ..Default::default()
    }
}

fn result(job: &Job, positions: &[[f64; 2]]) -> MarkerResult {
    let placements = job
        .pieces
        .iter()
        .zip(positions)
        .map(|(p, &translate)| Placement {
            piece_id: p.id.clone(),
            translate,
            bundle: Some(0),
            ..Default::default()
        })
        .collect();
    let mut r = MarkerResult {
        format: RESULT_FORMAT.into(),
        job_name: job.name.clone(),
        width: job.fabric.width,
        length: 0.0,
        piece_area: 0.0,
        efficiency: 0.0,
        useful_piece_area: 0.0,
        useful_yield: 0.0,
        surplus_copies: 0,
        engine: "test".into(),
        seed: None,
        runtime_s: 0.0,
        placements,
        warnings: vec![],
    };
    refresh(job, &mut r);
    r
}

#[test]
fn wrap_around_offsets_axes_and_default_tolerance() {
    let mut j = job();
    let mut r = result(&j, &[[39.8, -39.8], [0.2, 0.2]]);
    assert!(match_errors(&j, &r).is_empty());
    r.placements[0].translate[0] = 39.6;
    assert!(!match_errors(&j, &r).is_empty());
    j.fabric.repeat.as_mut().expect("repeat").tolerance_mm = 0.0;
    assert!(match_errors(&j, &r).is_empty());
    j.match_rules[0].offset = [-0.6, 0.0];
    j.fabric.repeat.as_mut().expect("repeat").tolerance_mm = 0.01;
    assert!(match_errors(&j, &r).is_empty());
    j.fabric.repeat.as_mut().expect("repeat").period_y = None;
    r.placements[0].translate[1] = 123.0;
    assert!(match_errors(&j, &r).is_empty());
}

#[test]
fn absolute_rule_uses_repeat_offset_and_transforms_match_point() {
    let mut j = job();
    j.match_rules[0].b = None;
    j.match_rules[0].offset = [3.0, -2.0];
    j.fabric.repeat.as_mut().expect("repeat").offset_x = 7.0;
    j.fabric.repeat.as_mut().expect("repeat").offset_y = 9.0;
    j.pieces[0].match_points[0].point = [4.0, 5.0];
    let mut r = result(&j, &[[15.0, 11.0], [0.0, 0.0]]);
    r.placements[0].mirrored = true;
    r.placements[0].rotation_deg = 90.0;
    // mirror+rotate (4,5) = (-5,-4), then (15,11) gives (10,7).
    assert!(match_errors(&j, &r).is_empty());
    r.placements[0].translate[1] += 1.0;
    assert!(!match_errors(&j, &r).is_empty());
}

#[test]
fn every_copy_and_bundle_is_checked() {
    let j = job();
    let mut r = result(&j, &[[0.0, 0.0], [80.0, 40.0]]);
    let mut extra = r.placements[1].clone();
    extra.copy = 1;
    extra.translate[0] += 5.0;
    r.placements.push(extra);
    assert!(!match_errors(&j, &r).is_empty());
    r.placements.pop();
    r.placements[1].bundle = Some(1);
    assert!(
        match_errors(&j, &r)
            .iter()
            .any(|e| e.code == "match_partner")
    );
    r.placements[1].bundle = None;
    assert!(match_errors(&j, &r).iter().any(|e| e.code == "bundle"));
}

#[test]
fn phase_search_is_independent_and_respects_user_fixed() -> anyhow::Result<()> {
    let j = job();
    let mut r = result(&j, &[[1.0, 1.0], [55.0, 45.0]]);
    r.placements[0].locked = true;
    let fixed = vec![r.placements[0].clone()];
    let placements = search::place_matched(&j, &r, &fixed)?;
    let mut actual = r.clone();
    actual.placements = placements;
    refresh(&j, &mut actual);
    assert!(match_errors(&j, &actual).is_empty());
    assert!(beta_marker_core::validate(&j, &actual).ok);
    assert_eq!(
        actual.placements.iter().find(|p| p.piece_id == "a"),
        fixed.first()
    );
    Ok(())
}

#[test]
fn inconsistent_cycles_are_errors() {
    let mut j = job();
    let mut incompatible = j.match_rules[0].clone();
    incompatible.offset[0] = 10.0;
    j.match_rules.push(incompatible);
    let r = result(&j, &[[1.0, 1.0], [55.0, 45.0]]);
    assert!(search::place_matched(&j, &r, &[]).is_err());
}

#[test]
fn phase_cycles_can_require_a_different_allowed_rotation() -> anyhow::Result<()> {
    let mut j = job();
    j.pieces[0].rotation = RotationRule::Flip180;
    j.pieces[1].rotation = RotationRule::Flip180;
    j.pieces[0].match_points.push(MatchPoint {
        id: "q".into(),
        point: [10.0, 0.0],
    });
    j.pieces[1].match_points.push(MatchPoint {
        id: "q".into(),
        point: [-10.0, 0.0],
    });
    j.match_rules.push(MatchRule {
        a: PointRef {
            piece_id: "a".into(),
            point_id: "q".into(),
        },
        b: Some(PointRef {
            piece_id: "b".into(),
            point_id: "q".into(),
        }),
        axes: MatchAxes::X,
        offset: [0.0; 2],
    });
    let baseline = result(&j, &[[1.0, 1.0], [55.0, 45.0]]);
    let placements = search::place_matched(&j, &baseline, &[])?;
    let mut r = baseline;
    r.placements = placements;
    refresh(&j, &mut r);
    assert!(match_errors(&j, &r).is_empty());
    assert!(beta_marker_core::validate(&j, &r).ok);
    assert!(r.placements.iter().any(|p| p.rotation_deg == 180.0));
    Ok(())
}

#[test]
fn phase_search_checks_defect_polygons_and_clearance() -> anyhow::Result<()> {
    let mut j = job();
    j.spacing.piece_gap = 8.0;
    j.spacing.edge_gap = 3.0;
    j.pieces[0].buffer = 2.0;
    j.fabric.defects.push(Zone {
        polygon: vec![[0.0, 0.0], [80.0, 0.0], [80.0, 70.0], [0.0, 70.0]],
        ..Default::default()
    });
    let baseline = result(&j, &[[1.0, 1.0], [55.0, 45.0]]);
    let placements = search::place_matched(&j, &baseline, &[])?;
    let mut r = baseline;
    r.placements = placements;
    refresh(&j, &mut r);
    assert!(beta_marker_core::validate(&j, &r).ok);
    for p in &r.placements {
        let piece = j
            .pieces
            .iter()
            .find(|piece| piece.id == p.piece_id)
            .expect("piece");
        let b =
            beta_marker_core::geometry::bbox(&beta_marker_core::rules::placed_outline(piece, p))
                .expect("bounds");
        assert!(b[0] >= 80.0 || b[1] >= 70.0);
    }
    Ok(())
}

#[test]
fn shade_sections_preserve_global_piece_gaps() -> anyhow::Result<()> {
    let mut j = job();
    j.match_rules.clear();
    j.spacing.piece_gap = 12.0;
    j.pieces[0].buffer = 2.0;
    for p in &mut j.pieces {
        p.size = Some("M".into());
    }
    j.order = Some(Order {
        sizes: std::collections::BTreeMap::from([("M".into(), 4)]),
    });
    for axis in [ShadeAxis::Length, ShadeAxis::Width] {
        j.fabric.shade = Some(ShadeRule {
            axis,
            band_mm: 100.0,
        });
        let r = nest_with_rules(&j, &opts())?;
        assert!(beta_marker_core::validate(&j, &r).ok);
        assert!(checks::shade_errors(&j, &r).is_empty());
    }
    Ok(())
}

#[test]
fn checker_applies_shrinkage_to_match_points() {
    let mut j = job();
    j.pieces[0].match_points[0].point = [10.0, 0.0];
    j.fabric.shrinkage.length_pct = 10.0;
    j.fabric.repeat.as_mut().expect("repeat").tolerance_mm = 0.01;
    let r = result(&j, &[[0.0, 0.0], [11.0, 0.0]]);
    assert!(match_errors(&j, &r).is_empty());
}

#[test]
fn buffered_snap_keeps_unmatched_positions() -> anyhow::Result<()> {
    let mut j = job();
    j.pieces.push(rect("unmatched", 30.0, 20.0));
    let baseline = result(&j, &[[1.0, 1.0], [90.0, 81.0], [55.0, 120.0]]);
    let r = search::snap_baseline(&j, &baseline)?.expect("clear snapped layout");
    assert!(match_errors(&j, &r).is_empty());
    assert_eq!(r.placements[2], baseline.placements[2]);
    for (a, b) in r.placements.iter().zip(&baseline.placements) {
        for axis in 0..2 {
            assert!((a.translate[axis] - b.translate[axis]).abs() <= 20.0);
        }
    }
    Ok(())
}

#[test]
fn oversized_shade_bundle_and_count_overflow_are_errors() {
    let mut j = job();
    j.match_rules.clear();
    j.fabric.shade = Some(ShadeRule {
        axis: ShadeAxis::Length,
        band_mm: 20.0,
    });
    assert!(nest_with_rules(&j, &opts()).is_err());
    j.pieces[0].quantity = u32::MAX;
    j.pieces[0].mirror_quantity = 1;
    assert!(nest_with_rules(&j, &opts()).is_err());
}

#[test]
fn matching_integrates_fixed_and_unmatched_pieces() -> anyhow::Result<()> {
    let mut j = job();
    j.pieces.push(rect("unmatched", 60.0, 23.0));
    let r = nest_with_rules(&j, &opts())?;
    assert!(beta_marker_core::validate(&j, &r).ok);
    assert!(match_errors(&j, &r).is_empty());
    assert!(r.placements.iter().all(|p| p.bundle.is_some()));
    Ok(())
}

#[test]
fn public_entry_keeps_user_locked_match_point() -> anyhow::Result<()> {
    let mut j = job();
    j.pieces.push(rect("unmatched", 60.0, 23.0));
    let fixed = Placement {
        piece_id: "a".into(),
        translate: [1.0, 1.0],
        bundle: Some(0),
        locked: true,
        ..Default::default()
    };
    let mut options = opts();
    options.fixed.push(fixed.clone());
    let r = nest_with_rules(&j, &options)?;
    assert_eq!(
        r.placements.iter().find(|p| p.piece_id == "a"),
        Some(&fixed)
    );
    assert!(match_errors(&j, &r).is_empty());
    Ok(())
}

#[test]
fn shade_band_remapping_preserves_user_fixed_identity() -> anyhow::Result<()> {
    let mut j = job();
    j.match_rules.clear();
    j.fabric.shade = Some(ShadeRule {
        axis: ShadeAxis::Length,
        band_mm: 117.0,
    });
    for p in &mut j.pieces {
        p.size = Some("M".into());
    }
    j.order = Some(Order {
        sizes: std::collections::BTreeMap::from([("M".into(), 3)]),
    });
    let fixed = Placement {
        piece_id: "a".into(),
        copy: 1,
        translate: [118.0, 1.0],
        bundle: Some(1),
        locked: true,
        ..Default::default()
    };
    let mut options = opts();
    options.fixed.push(fixed.clone());
    let r = nest_with_rules(&j, &options)?;
    assert_eq!(
        r.placements
            .iter()
            .find(|p| p.piece_id == "a" && p.copy == 1),
        Some(&fixed)
    );
    assert!(beta_marker_core::validate(&j, &r).ok);
    Ok(())
}

#[test]
fn shade_bands_keep_whole_bundles_and_copy_ids() -> anyhow::Result<()> {
    let mut j = job();
    j.match_rules.clear();
    j.pieces[0].size = Some("M".into());
    j.pieces[1].size = Some("M".into());
    j.pieces[0].quantity = 2;
    j.order = Some(Order {
        sizes: std::collections::BTreeMap::from([("M".into(), 3)]),
    });
    for axis in [ShadeAxis::Length, ShadeAxis::Width] {
        j.fabric.shade = Some(ShadeRule {
            axis,
            band_mm: 100.0,
        });
        let r = nest_with_rules(&j, &opts())?;
        assert!(beta_marker_core::validate(&j, &r).ok);
        assert!(checks::shade_errors(&j, &r).is_empty());
        assert_eq!(r.placements.len(), 9);
        let copies: std::collections::BTreeSet<_> = r.placements.iter().map(identity).collect();
        assert_eq!(copies.len(), 9);
        assert_eq!(
            r.placements
                .iter()
                .map(|p| p.bundle)
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            3
        );
    }
    Ok(())
}

#[test]
fn shade_remapping_uses_natural_size_order() -> anyhow::Result<()> {
    let mut j = job();
    j.match_rules.clear();
    j.pieces.clear();
    for size in ["S", "M", "L"] {
        let mut p = rect(size, 40.0, 20.0);
        p.size = Some(size.into());
        p.mirror_quantity = 1;
        j.pieces.push(p);
    }
    j.order = Some(Order {
        sizes: std::collections::BTreeMap::from([
            ("S".into(), 1),
            ("M".into(), 2),
            ("L".into(), 1),
        ]),
    });
    j.fabric.shade = Some(ShadeRule {
        axis: ShadeAxis::Length,
        band_mm: 117.0,
    });
    let r = nest_with_rules(&j, &opts())?;
    for pl in &r.placements {
        let expected = match pl.piece_id.as_str() {
            "S" => 0,
            "M" => 1 + pl.copy,
            "L" => 3,
            _ => unreachable!("test piece"),
        };
        assert_eq!(pl.bundle, Some(expected));
    }
    assert!(beta_marker_core::validate(&j, &r).ok);
    Ok(())
}

#[test]
fn absolute_matching_stays_in_global_phase_after_band_shift() -> anyhow::Result<()> {
    let mut j = job();
    j.fabric.width = 100.0;
    j.match_rules[0].b = None;
    j.match_rules[0].offset = [7.0, 3.0];
    j.fabric.shade = Some(ShadeRule {
        axis: ShadeAxis::Length,
        band_mm: 117.0,
    });
    for p in &mut j.pieces {
        p.size = Some("M".into());
    }
    j.order = Some(Order {
        sizes: std::collections::BTreeMap::from([("M".into(), 3)]),
    });
    let r = nest_with_rules(&j, &opts())?;
    assert!(r.length > 117.0, "the fixture must exercise a shifted band");
    assert!(match_errors(&j, &r).is_empty());
    assert!(checks::shade_errors(&j, &r).is_empty());
    Ok(())
}

#[test]
fn malformed_repeat_and_nonfinite_offsets_return_errors() {
    assert!(
        nest_with_rules(
            &job(),
            &NestOptions {
                time_limit_s: f64::MAX,
                ..opts()
            }
        )
        .is_err()
    );
    let mut j = job();
    j.fabric.repeat.as_mut().expect("repeat").period_x = Some(f64::INFINITY);
    assert!(nest_with_rules(&j, &opts()).is_err());
    assert!(!match_errors(&j, &result(&j, &[[0.0; 2], [0.0; 2]])).is_empty());
    j.fabric.repeat.as_mut().expect("repeat").period_x = Some(40.0);
    j.match_rules[0].offset[0] = f64::NAN;
    assert!(nest_with_rules(&j, &opts()).is_err());
}

#[test]
fn simon_check_fixture() -> anyhow::Result<()> {
    let job: Job = serde_json::from_str(include_str!("../tests/fixtures/simon-check.job.json"))?;
    let result = nest_with_rules(&job, &opts())?;
    assert!(beta_marker_core::validate(&job, &result).ok);
    assert!(match_errors(&job, &result).is_empty());
    Ok(())
}

#[test]
fn simon_ratio_shade_fixtures() -> anyhow::Result<()> {
    for json in [
        include_str!("../tests/fixtures/simon-shade-length.job.json"),
        include_str!("../tests/fixtures/simon-shade-width.job.json"),
    ] {
        let job: Job = serde_json::from_str(json)?;
        let result = nest_with_rules(&job, &opts())?;
        assert!(beta_marker_core::validate(&job, &result).ok);
        assert!(checks::shade_errors(&job, &result).is_empty());
    }
    Ok(())
}

#[test]
fn shade_fixed_optional_mirror_does_not_require_mirror_quantity() -> anyhow::Result<()> {
    let mut j = job();
    j.match_rules.clear();
    j.fabric.shade = Some(ShadeRule {
        axis: ShadeAxis::Length,
        band_mm: 117.0,
    });
    for p in &mut j.pieces {
        p.size = Some("M".into());
    }
    j.pieces[0].flip = Some(true);
    j.order = Some(Order {
        sizes: std::collections::BTreeMap::from([("M".into(), 2)]),
    });
    let fixed = Placement {
        piece_id: "a".into(),
        copy: 0,
        mirrored: true,
        translate: [170.0, 1.0],
        bundle: Some(1),
        locked: true,
        ..Default::default()
    };
    let result = nest_with_rules(
        &j,
        &NestOptions {
            fixed: vec![fixed.clone()],
            ..opts()
        },
    )?;
    assert!(result.placements.contains(&fixed));
    assert!(beta_marker_core::validate(&j, &result).ok);
    let identities: std::collections::BTreeSet<_> =
        result.placements.iter().map(identity).collect();
    assert_eq!(identities.len(), result.placements.len());
    Ok(())
}
