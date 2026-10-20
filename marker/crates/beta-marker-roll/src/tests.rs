use super::*;
use beta_marker_core::Shrinkage;

fn roll(id: &str, length: f64) -> Roll {
    Roll {
        id: id.into(),
        lot: "lot".into(),
        dye_lot: "dye".into(),
        shade_group: "A".into(),
        ticket_length_m: length,
        measured_length_m: length,
        usable_width_mm: 1500.0,
        shrinkage: Shrinkage::default(),
        defects: Vec::new(),
    }
}

fn register(lengths: &[f64]) -> RollRegister {
    RollRegister {
        rolls: lengths
            .iter()
            .enumerate()
            .map(|(i, &len)| roll(&format!("R{i}"), len))
            .collect(),
    }
}

fn lay(id: &str, length: f64, plies: u32) -> LayRequest {
    LayRequest {
        id: id.into(),
        marker_length_m: length,
        plies,
        fabric_width_mm: 1500.0,
        shade_rule: ShadePolicy::SameGroup,
        eligible_roll_ids: Vec::new(),
        allow_splicing: false,
        splice_zones: Vec::new(),
    }
}

fn defect(id: &str, along: f64, points: u8) -> RollDefect {
    RollDefect {
        id: id.into(),
        along_m: along,
        across_mm: 100.0,
        length_mm: 100.0,
        width_mm: 20.0,
        points,
        defect_type: "hole".into(),
        action: DefectAction::CutAround,
    }
}

fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-7, "{a} != {b}");
}

#[test]
fn square_yard_formula_uses_measured_usable_area() {
    let s = InspectionSettings::default();
    let score = inspection_score(40, 40, 91.44, 1524.0, &s).unwrap();
    close(score.points_per_100_sq_yd, 24.0); // 100 yd x 60 inches = 166 2/3 sq yd
    close(score.points_per_100_m2 * SQUARE_YARD_M2, 24.0);
    close(score.area_m2, 91.44 * 1.524);
}

#[test]
fn metric_formula_limit_units_and_boundary() {
    let mut s = InspectionSettings {
        limit: 35.0,
        ..Default::default()
    };
    let score = inspection_score(60, 60, 100.0, 1500.0, &s).unwrap();
    close(score.points_per_100_m2, 40.0);
    assert!(score.passed);
    s.unit = InspectionUnit::Per100SquareMetres;
    assert!(!inspection_score(60, 60, 100.0, 1500.0, &s).unwrap().passed);
    s.limit = 40.0;
    assert!(inspection_score(60, 60, 100.0, 1500.0, &s).unwrap().passed);
    let narrow = inspection_score(60, 60, 100.0, 750.0, &s).unwrap();
    close(narrow.points_per_100_m2, 80.0);
}

#[test]
fn cap_policy_is_explicit_and_raw_points_remain_visible() {
    let mut r = register(&[10.0]);
    r.rolls[0].defects = vec![
        defect("a", 0.1, 4),
        defect("b", 0.2, 4),
        defect("c", 0.95, 4),
    ];
    let mut s = InspectionSettings::default();
    let report = inspect(&r, &s).unwrap();
    assert_eq!(report.rolls[0].score.raw_points, 12);
    assert_eq!(report.rolls[0].score.scored_points, 8);
    s.point_cap = PointCap::PerMetre;
    assert_eq!(inspect(&r, &s).unwrap().rolls[0].score.scored_points, 4);
    s.point_cap = PointCap::None;
    assert_eq!(inspect(&r, &s).unwrap().rolls[0].score.scored_points, 12);
}

#[test]
fn lot_score_is_area_weighted_and_reports_failed_rolls() {
    let mut r = register(&[1.0, 99.0, 5.0]);
    r.rolls[0].defects.push(defect("d", 0.0, 4));
    r.rolls[2].dye_lot = "other".into();
    r.rolls[0].ticket_length_m = 2.0;
    let report = inspect(&r, &InspectionSettings::default()).unwrap();
    let lot = report.lots.iter().find(|l| l.dye_lot == "dye").unwrap();
    close(lot.score.points_per_100_m2, 4.0 / 150.0 * 100.0);
    assert!(lot.score.passed);
    assert_eq!(lot.failed_rolls, 1);
    close(report.rolls[0].length_difference_m, -1.0);
}

#[test]
fn invalid_measurements_defects_and_limits_are_rejected() {
    for v in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(inspection_score(4, 4, v, 1500.0, &InspectionSettings::default()).is_err());
        assert!(inspection_score(4, 4, 10.0, v, &InspectionSettings::default()).is_err());
    }
    assert!(inspection_score(1, 2, 10.0, 1500.0, &InspectionSettings::default()).is_err());
    let s = InspectionSettings {
        limit: -1.0,
        ..Default::default()
    };
    assert!(inspect(&RollRegister::default(), &s).is_err());
    let mut r = register(&[10.0]);
    r.rolls[0].defects.push(defect("d", 9.95, 2));
    assert!(r.validate().is_err());
    r.rolls[0].defects[0].along_m = 1.0;
    r.rolls[0].defects[0].points = 0;
    assert!(r.validate().is_err());
    r.rolls[0].defects[0].points = 5;
    assert!(r.validate().is_err());
    r.rolls[0].defects[0].points = 4;
    r.rolls[0].defects[0].across_mm = 1499.0;
    assert!(r.validate().is_err());
}

#[test]
fn shade_shrinkage_bands_average_and_negative_boundaries() {
    let mut r = register(&[10.0; 6]);
    for (i, pct) in [-1.01, -1.0, 0.0, 0.99, 1.0, 0.0].iter().enumerate() {
        r.rolls[i].shrinkage.length_pct = *pct;
    }
    r.rolls[5].shade_group = "B".into();
    let groups = group_rolls(&r, &GroupingSettings::default()).unwrap();
    assert_eq!(groups.len(), 4);
    let middle = groups
        .iter()
        .find(|g| g.shade_group == "A" && g.length_band == 0)
        .unwrap();
    assert_eq!(middle.roll_ids, ["R1", "R2", "R3"]);
    close(middle.average_shrinkage.length_pct, -0.01 / 3.0);
    let mut job = beta_marker_core::Job::default();
    job.fabric.shrinkage = middle.average_shrinkage;
    close(job.fabric.shrinkage.width_pct, 0.0);
    let finer = group_rolls(&r, &GroupingSettings { band_step_pct: 1.0 }).unwrap();
    assert_eq!(finer.len(), 4);
    assert_eq!(
        finer
            .iter()
            .find(|g| g.shade_group == "A" && g.length_band == 0)
            .unwrap()
            .roll_ids,
        ["R2"]
    );
    assert!(group_rolls(&r, &GroupingSettings { band_step_pct: 0.0 }).is_err());
}

#[test]
fn grouping_keeps_dye_lots_and_both_axes_separate() {
    let mut r = register(&[10.0; 3]);
    r.rolls[1].dye_lot = "dye2".into();
    r.rolls[2].shrinkage.width_pct = 2.0;
    assert_eq!(
        group_rolls(&r, &GroupingSettings::default()).unwrap().len(),
        3
    );
}

#[test]
fn exact_allocator_finds_zero_waste_mixed_lengths() {
    let r = register(&[10.0, 10.0, 12.0]);
    let requests = vec![lay("six", 6.0, 2), lay("four", 4.0, 2)];
    let plan = allocate(&r, &requests, &AllocationSettings::default()).unwrap();
    assert!(plan.complete && plan.search.optimal_whole_ply);
    close(plan.total_waste_m, 0.0);
    close(plan.unopened_stock_m, 12.0);
    validate_plan(&r, &plan).unwrap();
}

#[test]
fn exact_allocator_can_recover_from_greedy_failure() {
    let r = register(&[10.0, 10.0]);
    let requests = vec![lay("four", 4.0, 2), lay("six", 6.0, 2)];
    let comparison = compare_allocations(&r, &requests, &AllocationSettings::default()).unwrap();
    assert!(!comparison.first_come.complete);
    assert!(comparison.optimized.complete);
    assert_eq!(comparison.waste_saved_m, None);
    validate_plan(&r, &comparison.optimized).unwrap();
}

#[test]
fn search_budget_does_not_claim_optimality() {
    let r = register(&[10.0, 10.0, 12.0]);
    let s = AllocationSettings {
        exact_node_limit: 1,
        ..Default::default()
    };
    let plan = allocate(&r, &[lay("a", 4.0, 2)], &s).unwrap();
    assert!(!plan.search.optimal_whole_ply);
    assert_eq!(plan.search.exact_nodes, 1);
    validate_plan(&r, &plan).unwrap();
}

#[test]
fn stock_is_shared_between_lays_and_end_bits_are_counted_once() {
    let r = register(&[13.0]);
    let plan = allocate(
        &r,
        &[lay("a", 6.0, 1), lay("b", 5.0, 1)],
        &AllocationSettings::default(),
    )
    .unwrap();
    close(plan.total_end_bits_m, 2.0);
    close(
        plan.lays
            .iter()
            .flat_map(|l| &l.allocations)
            .map(|a| a.end_bit_m)
            .sum(),
        2.0,
    );
    close(plan.total_used_m, 11.0);
    assert_eq!(plan.roll_changes, 0);
    validate_plan(&r, &plan).unwrap();
}

#[test]
fn width_and_shade_cannot_be_relaxed_silently() {
    let mut r = register(&[100.0, 6.0, 6.0]);
    r.rolls[0].usable_width_mm = 1499.0;
    r.rolls[2].shade_group = "B".into();
    let mut request = lay("a", 6.0, 2);
    let plan = allocate(&r, &[request.clone()], &AllocationSettings::default()).unwrap();
    assert!(!plan.complete);
    assert_eq!(plan.lays[0].allocated_plies, 1);
    assert!(!plan.rolls[0].opened);
    assert!(plan.warnings.iter().any(|w| w.contains("too narrow")));
    request.shade_rule = ShadePolicy::AllowMixing;
    let plan = allocate(&r, &[request], &AllocationSettings::default()).unwrap();
    assert!(plan.complete);
    assert!(plan.warnings.iter().any(|w| w.contains("shade mixing")));
    validate_plan(&r, &plan).unwrap();
}

#[test]
fn required_shade_and_shrinkage_group_stock_restriction() {
    let mut r = register(&[20.0, 20.0]);
    r.rolls[1].shade_group = "B".into();
    let mut request = lay("a", 5.0, 2);
    request.shade_rule = ShadePolicy::Required {
        identity: ShadeIdentity {
            lot: "lot".into(),
            dye_lot: "dye".into(),
            shade_group: "B".into(),
        },
    };
    let plan = allocate(&r, &[request.clone()], &AllocationSettings::default()).unwrap();
    assert_eq!(plan.lays[0].allocations[0].roll_id, "R1");
    request.eligible_roll_ids = vec!["R0".into()];
    assert!(
        !allocate(&r, &[request], &AllocationSettings::default())
            .unwrap()
            .complete
    );
}

#[test]
fn three_piece_splice_covers_marker_and_accounts_for_overlap() {
    let r = register(&[4.1, 4.1, 4.0]);
    let mut request = lay("splice", 12.0, 1);
    request.allow_splicing = true;
    request.splice_zones = vec![
        SpliceZone {
            start_m: 4.0,
            end_m: 4.1,
        },
        SpliceZone {
            start_m: 8.0,
            end_m: 8.1,
        },
    ];
    let plan = allocate(&r, &[request], &AllocationSettings::default()).unwrap();
    assert!(plan.complete);
    assert_eq!(plan.lays[0].splices.len(), 2);
    close(plan.total_used_m, 12.2);
    close(plan.splice_overlap_waste_m, 0.2);
    close(plan.total_end_bits_m, 0.0);
    validate_plan(&r, &plan).unwrap();
}

#[test]
fn no_splice_without_permission_or_suitable_zone() {
    let r = register(&[6.0, 6.0]);
    let mut request = lay("a", 10.0, 1);
    request.splice_zones = vec![SpliceZone {
        start_m: 4.0,
        end_m: 4.5,
    }];
    assert!(
        !allocate(&r, &[request.clone()], &AllocationSettings::default())
            .unwrap()
            .complete
    );
    request.allow_splicing = true;
    request.splice_zones = vec![SpliceZone {
        start_m: 1.0,
        end_m: 2.0,
    }];
    let plan = allocate(&r, &[request], &AllocationSettings::default()).unwrap();
    assert!(!plan.complete);
    close(plan.total_used_m, 0.0); // unsuccessful paths consume no stock
    validate_plan(&r, &plan).unwrap();
}

#[test]
fn splice_reuses_opened_tails_and_reduces_new_stock() {
    let r = register(&[6.0, 6.0, 10.0]);
    let mut first = lay("first", 4.0, 2);
    first.eligible_roll_ids = vec!["R0".into(), "R1".into()];
    let mut last = lay("last", 3.5, 1);
    last.allow_splicing = true;
    last.splice_zones = vec![SpliceZone {
        start_m: 1.5,
        end_m: 2.0,
    }];
    let comparison =
        compare_allocations(&r, &[first, last], &AllocationSettings::default()).unwrap();
    assert!(comparison.optimized.complete);
    assert!(!comparison.optimized.rolls[2].opened);
    assert!(comparison.waste_saved_m.unwrap() > 5.0);
    validate_plan(&r, &comparison.optimized).unwrap();
}

#[test]
fn defect_mapping_clips_across_ply_boundary_and_converts_metres_to_mm() {
    let mut r = register(&[10.0]);
    let mut d = defect("d", 4.8, 4);
    d.length_mm = 400.0;
    r.rolls[0].defects.push(d);
    let plan = allocate(&r, &[lay("a", 5.0, 2)], &AllocationSettings::default()).unwrap();
    let map = map_defects(&r, &plan.lays[0], &DefectMappingSettings::default()).unwrap();
    assert_eq!(map.zones.len(), 2);
    close(map.zones[0].polygon[0][0], 4800.0);
    close(map.zones[0].polygon[1][0], 5000.0);
    close(map.zones[1].polygon[0][0], 0.0);
    close(map.zones[1].polygon[1][0], 200.0);
    close(map.zones[0].polygon[0][1], 100.0);
    assert!(!map.requires_replanning);
}

#[test]
fn high_ply_defects_become_explicit_pending_cutouts() {
    let mut r = register(&[50.0]);
    r.rolls[0].defects.push(defect("d", 1.2, 4));
    let mut request = lay("a", 5.0, 8);
    request.allow_splicing = true;
    request.splice_zones = vec![SpliceZone {
        start_m: 1.0,
        end_m: 2.0,
    }];
    let plan = allocate(&r, &[request], &AllocationSettings::default()).unwrap();
    let map = map_defects(&r, &plan.lays[0], &DefectMappingSettings::default()).unwrap();
    assert!(map.zones.is_empty());
    assert_eq!(map.cut_instructions.len(), 1);
    assert!(map.requires_replanning && map.cut_instructions[0].splice_zone_available);
    assert!(!map.warnings.is_empty());
}

#[test]
fn explicit_cutout_and_ignored_defects_follow_policy() {
    let mut r = register(&[10.0]);
    let mut d = defect("d", 1.2, 4);
    d.action = DefectAction::CutOut;
    r.rolls[0].defects.push(d);
    let plan = allocate(&r, &[lay("a", 5.0, 1)], &AllocationSettings::default()).unwrap();
    let map = map_defects(&r, &plan.lays[0], &DefectMappingSettings::default()).unwrap();
    assert!(map.requires_replanning && !map.cut_instructions[0].splice_zone_available);
    r.rolls[0].defects[0].action = DefectAction::Ignore;
    assert!(
        map_defects(&r, &plan.lays[0], &DefectMappingSettings::default())
            .unwrap()
            .cut_instructions
            .is_empty()
    );
}

#[test]
fn json_round_trips_and_tables_escape_identifiers() {
    let mut r = register(&[10.0]);
    r.rolls[0].id = "roll,\"one\"".into();
    let mut l = lay("lay\n1", 6.0, 1);
    l.shade_rule = ShadePolicy::AllowMixing;
    let plan = allocate(&r, &[l], &AllocationSettings::default()).unwrap();
    let json = serde_json::to_string(&plan).unwrap();
    assert_eq!(serde_json::from_str::<SpreadingPlan>(&json).unwrap(), plan);
    assert!(plan.to_csv().contains("\"roll,\"\"one\"\"\""));
    assert!(plan.to_text().contains("lay 1"));
    let report = inspect(&r, &InspectionSettings::default()).unwrap();
    let json = serde_json::to_string(&report).unwrap();
    assert_eq!(
        serde_json::from_str::<InspectionReport>(&json).unwrap(),
        report
    );
    assert!(report.to_csv().contains("points_per_100_sq_yd"));
    assert!(report.to_text().contains("Dye lot"));
    assert_eq!(
        serde_json::from_str::<RollRegister>(&serde_json::to_string(&r).unwrap()).unwrap(),
        r
    );
}

#[test]
fn invalid_allocation_requests_and_tampered_plans_are_rejected() {
    let r = register(&[10.0]);
    let mut l = lay("a", 5.0, 0);
    assert!(allocate(&r, &[l.clone()], &AllocationSettings::default()).is_err());
    l.plies = 1;
    l.eligible_roll_ids = vec!["missing".into()];
    assert!(allocate(&r, &[l], &AllocationSettings::default()).is_err());
    let mut p = allocate(&r, &[lay("a", 5.0, 1)], &AllocationSettings::default()).unwrap();
    p.lays[0].segments[0].roll_start_m = 6.0;
    assert!(validate_plan(&r, &p).is_err());
    assert!(map_defects(&r, &p.lays[0], &DefectMappingSettings::default()).is_err());
}

#[test]
fn empty_request_keeps_all_stock_unopened() {
    let r = register(&[10.0]);
    let p = allocate(&r, &[], &AllocationSettings::default()).unwrap();
    assert!(p.complete);
    close(p.unopened_stock_m, 10.0);
    close(p.total_waste_m, 0.0);
    validate_plan(&r, &p).unwrap();
}

// Independent exhaustive assignment oracle. Each small lay has one ply.
fn oracle(
    lengths: &[f64],
    items: &[f64],
    index: usize,
    remaining: &mut [f64],
    opened: &mut [bool],
    best: &mut f64,
) {
    if index == items.len() {
        let cost: f64 = lengths
            .iter()
            .zip(opened.iter())
            .filter(|(_, o)| **o)
            .map(|(v, _)| v)
            .sum::<f64>()
            - items.iter().sum::<f64>();
        *best = best.min(cost);
        return;
    }
    for i in 0..lengths.len() {
        if remaining[i] >= items[index] {
            let old = opened[i];
            opened[i] = true;
            remaining[i] -= items[index];
            oracle(lengths, items, index + 1, remaining, opened, best);
            remaining[i] += items[index];
            opened[i] = old;
        }
    }
}

#[test]
fn exact_search_matches_independent_optimum_on_small_instances() {
    for seed in 0..32 {
        let lengths: Vec<_> = (0..4)
            .map(|i| (6 + (seed * 7 + i * 3) % 9) as f64)
            .collect();
        let items: Vec<_> = (0..5)
            .map(|i| (2 + (seed * 3 + i * 2) % 6) as f64)
            .collect();
        let requests: Vec<_> = items
            .iter()
            .enumerate()
            .map(|(i, &n)| lay(&format!("L{i}"), n, 1))
            .collect();
        let r = register(&lengths);
        let p = allocate(&r, &requests, &AllocationSettings::default()).unwrap();
        let mut optimum = f64::INFINITY;
        oracle(
            &lengths,
            &items,
            0,
            &mut lengths.clone(),
            &mut [false; 4],
            &mut optimum,
        );
        assert_eq!(p.complete, optimum.is_finite());
        if p.complete {
            close(p.total_waste_m, optimum);
            assert!(p.search.optimal_whole_ply);
        }
        validate_plan(&r, &p).unwrap();
    }
}

#[test]
fn realistic_forty_roll_portfolio_never_loses_to_first_come() {
    let mut seed = 20261019_u64;
    let mut r = RollRegister::default();
    for i in 0..40 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let mut roll = roll(
            &format!("R{i:02}"),
            40.0 + ((seed >> 32) % 8001) as f64 / 100.0,
        );
        roll.shade_group = format!("S{}", i % 3);
        r.rolls.push(roll);
    }
    let requests: Vec<_> = (0..18)
        .map(|i| {
            let mut l = lay(
                &format!("L{i:02}"),
                [5.7, 7.3, 9.4, 6.1, 8.6, 10.2][i % 6],
                14,
            );
            l.shade_rule = ShadePolicy::Required {
                identity: ShadeIdentity {
                    lot: "lot".into(),
                    dye_lot: "dye".into(),
                    shade_group: format!("S{}", i % 3),
                },
            };
            l
        })
        .collect();
    let c = compare_allocations(&r, &requests, &AllocationSettings::default()).unwrap();
    validate_plan(&r, &c.first_come).unwrap();
    validate_plan(&r, &c.optimized).unwrap();
    assert!(c.first_come.complete && c.optimized.complete);
    assert!(c.waste_saved_m.unwrap() > 0.0);
    println!(
        "40 rolls: baseline {:.2} m, optimized {:.2} m, saved {:.2} m ({:.2}%)",
        c.first_come.total_waste_m,
        c.optimized.total_waste_m,
        c.waste_saved_m.unwrap(),
        c.waste_saved_pct.unwrap()
    );
}
