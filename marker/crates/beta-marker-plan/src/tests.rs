use super::*;

fn table(rows: &[(&[u32], f64)]) -> Vec<Candidate> {
    rows.iter()
        .map(|(ratio, length)| Candidate {
            ratio: ratio.to_vec(),
            length: *length,
            result: None,
        })
        .collect()
}

fn totals(c: &[Candidate], actions: &[(usize, u32)], n: usize) -> (Vec<u32>, f64) {
    let mut made = vec![0; n];
    let mut length = 0.0;
    for &(i, p) in actions {
        for (v, r) in made.iter_mut().zip(&c[i].ratio) {
            *v += r * p;
        }
        length += c[i].length * f64::from(p) / 1000.0;
    }
    (made, length)
}

#[test]
fn exact_mixed_ratio_beats_single_sizes_and_splits_plies() {
    let c = table(&[(&[1, 0], 1000.0), (&[0, 1], 1000.0), (&[1, 1], 1500.0)]);
    let opts = PlanOptions {
        search_nodes: 100_000,
        ..Default::default()
    };
    let s = optimizer::solve(&c, &[12, 12], &[12, 12], 5, &opts);
    let (made, length) = totals(&c, &s.actions, 2);
    assert_eq!(made, vec![12, 12]);
    assert_eq!(length, 18.0);
    assert_eq!(s.actions.len(), 3);
    assert!(s.actions.iter().all(|(_, p)| *p <= 5));
    assert!(s.exhausted);
}

#[test]
fn overcut_can_save_fabric_but_never_exceeds_cap() {
    let c = table(&[(&[1], 1000.0), (&[2], 1400.0)]);
    let opts = PlanOptions::default();
    let exact = optimizer::solve(&c, &[3], &[3], 80, &opts);
    assert_eq!(totals(&c, &exact.actions, 1), (vec![3], 2.4));
    let extra = optimizer::solve(&c, &[3], &[4], 80, &opts);
    assert_eq!(totals(&c, &extra.actions, 1), (vec![4], 2.8));
    // Lay cost makes a single two-ply lay cheaper than two separate markers.
    assert_eq!(extra.actions.len(), 1);
}

#[test]
fn allowance_is_per_end_per_ply_and_lay_cost_changes_choice() {
    let c = table(&[(&[1], 1000.0), (&[2], 2100.0)]);
    let opts = PlanOptions {
        end_allowance_mm: 100.0,
        lay_cost_m: 0.0,
        ..Default::default()
    };
    let s = optimizer::solve(&c, &[4], &[4], 2, &opts);
    assert_eq!(s.actions, vec![(1, 2)]);
    let order = simon_order(false);
    let p = assemble(&order, &opts, &["S".into()], &c, &[(1, 2)], true, vec![]);
    assert!((p.total_fabric_m - 4.6).abs() < 1e-9);
}

#[test]
fn impossible_table_reports_best_partial_and_budget_preserves_feasibility() {
    let c = table(&[(&[1, 0], 1000.0)]);
    let s = optimizer::solve(&c, &[3, 2], &[3, 2], 80, &PlanOptions::default());
    assert_eq!(totals(&c, &s.actions, 2).0, vec![3, 0]);
    let c = table(&[(&[1, 0], 1000.0), (&[0, 1], 1000.0), (&[1, 1], 1800.0)]);
    let s = optimizer::solve(
        &c,
        &[37, 113],
        &[38, 118],
        80,
        &PlanOptions {
            search_nodes: 1,
            ..Default::default()
        },
    );
    let (made, _) = totals(&c, &s.actions, 2);
    assert!(made[0] >= 37 && made[0] <= 38 && made[1] >= 113 && made[1] <= 118);
    assert!(!s.exhausted);
}

#[test]
fn small_tables_match_independent_exhaustive_dynamic_program() {
    // DP adds one ply at a time with max_plies=1, so its objective is exact.
    for lay_cost in [0.0, 0.7] {
        let c = table(&[
            (&[1, 0], 900.0),
            (&[0, 1], 1100.0),
            (&[1, 1], 1550.0),
            (&[2, 1], 2400.0),
        ]);
        let opts = PlanOptions {
            lay_cost_m: lay_cost,
            search_nodes: 100_000,
            ..Default::default()
        };
        for a in 1..=5 {
            for b in 1..=5 {
                let mut dp = vec![vec![f64::INFINITY; 7]; 7];
                dp[0][0] = 0.0;
                for x in 0..=a + 1 {
                    for y in 0..=b + 1 {
                        for row in &c {
                            let nx = x + row.ratio[0] as usize;
                            let ny = y + row.ratio[1] as usize;
                            if nx <= a + 1 && ny <= b + 1 {
                                dp[nx][ny] =
                                    dp[nx][ny].min(dp[x][y] + row.length / 1000.0 + lay_cost);
                            }
                        }
                    }
                }
                let expected = (a..=a + 1)
                    .flat_map(|x| (b..=b + 1).map(move |y| (x, y)))
                    .map(|(x, y)| dp[x][y])
                    .fold(f64::INFINITY, f64::min);
                let s = optimizer::solve(
                    &c,
                    &[a as u32, b as u32],
                    &[(a + 1) as u32, (b + 1) as u32],
                    1,
                    &opts,
                );
                let actual = s
                    .actions
                    .iter()
                    .map(|(i, p)| c[*i].length / 1000.0 * f64::from(*p) + lay_cost)
                    .sum::<f64>();
                assert!(
                    (actual - expected).abs() < 1e-8,
                    "{a},{b}: {actual} != {expected}"
                );
                assert!(s.exhausted);
            }
        }
    }
}

#[test]
fn bounded_search_keeps_every_feasible_single_ratio_incumbent() {
    let c = table(&[
        (&[1, 1], 2000.0),
        (&[3, 1], 3000.0),
        (&[1, 0], 1000.0),
        (&[0, 1], 5000.0),
    ]);
    let opts = PlanOptions {
        search_nodes: 1,
        ..Default::default()
    };
    let s = optimizer::solve(&c, &[10, 10], &[10, 10], 2, &opts);
    let (made, length) = totals(&c, &s.actions, 2);
    assert_eq!(made, vec![10, 10]);
    assert!(length + opts.lay_cost_m * s.actions.len() as f64 <= 22.5);
}

fn simon_order(awkward: bool) -> CutOrder {
    let job: Job = serde_json::from_str(include_str!(
        "../../../testdata/open/freesewing/simon-ratio.job.json"
    ))
    .expect("MIT test fixture");
    CutOrder {
        format: CUT_ORDER_FORMAT.into(),
        name: if awkward {
            "Simon awkward"
        } else {
            "Simon 1200"
        }
        .into(),
        job,
        quantities: ["S", "M", "L", "XL"]
            .into_iter()
            .zip(if awkward {
                [37, 113, 96, 41]
            } else {
                [200, 400, 400, 200]
            })
            .map(|(s, q)| (s.into(), q))
            .collect(),
        max_plies: 80,
        max_garments_per_marker: 6,
        overcut_pct: if awkward { 5.0 } else { 0.0 },
    }
}

#[test]
fn examples_generate_ratios_and_reject_invalid_orders() {
    let mut order = simon_order(true);
    let opts = PlanOptions::default();
    validate_input(&order, &opts).expect("valid order");
    let sizes: Vec<_> = order.quantities.keys().cloned().collect();
    let (c, _) = generate(&order, &opts, &sizes, &[1000; 4]).expect("ratios");
    assert!(c.iter().all(|c| c.ratio.iter().sum::<u32>() <= 6));
    for i in 0..4 {
        assert!(
            c.iter()
                .any(|c| c.ratio[i] == 1 && c.ratio.iter().sum::<u32>() == 1)
        );
    }
    let unique: std::collections::BTreeSet<_> = c.iter().map(|c| c.ratio.clone()).collect();
    assert_eq!(unique.len(), c.len());
    order.quantities.insert("missing".into(), 1);
    assert!(validate_input(&order, &opts).is_err());
    order = simon_order(false);
    order.overcut_pct = f64::NAN;
    assert!(plan(&order, &opts).is_err());
}

#[test]
fn zero_order_and_partial_balance_are_explicit() -> Result<()> {
    let mut order = simon_order(false);
    order.quantities.values_mut().for_each(|q| *q = 0);
    let p = plan(&order, &PlanOptions::default())?;
    assert!(p.complete && p.lays.is_empty());
    assert_eq!(p.total_fabric_m, 0.0);
    order = simon_order(false);
    let c = table(&[(&[1], 1000.0)]);
    let p = assemble(
        &order,
        &PlanOptions::default(),
        &["S".into()],
        &c,
        &[(0, 80)],
        true,
        vec![],
    );
    assert!(!p.complete);
    assert_eq!(p.planned["S"], 80);
    assert_eq!(p.shortfall["S"], 120);
    assert_eq!(p.shortfall["M"], 400);
    Ok(())
}

#[test]
#[ignore = "real Simon nests; writes measurement artifacts under BETA_DATA_DIR"]
fn simon_real_plans() -> Result<()> {
    let root = std::path::PathBuf::from(std::env::var("BETA_DATA_DIR")?).join("marker-bench/plan");
    std::fs::create_dir_all(&root)?;
    let baseline_started = Instant::now();
    let baseline_job = simon_order(false).job;
    let baseline = beta_marker_match::nest_with_rules(
        &baseline_job,
        &beta_marker_nest::NestOptions {
            time_limit_s: 0.3,
            workers: Some(2),
            seed: Some(42),
            ..Default::default()
        },
    )?;
    assert!(beta_marker_core::validate(&baseline_job, &baseline).ok);
    let baseline_runtime = baseline_started.elapsed().as_secs_f64();
    std::fs::write(
        root.join("naive.result.json"),
        serde_json::to_vec_pretty(&baseline)?,
    )?;
    for awkward in [false, true] {
        let order = simon_order(awkward);
        let opts = PlanOptions {
            time_per_marker_s: 0.3,
            max_refinements: 10,
            search_nodes: 5000,
            ..Default::default()
        };
        let p = plan(&order, &opts)?;
        assert!(p.complete, "{:?}", p.warnings);
        for lay in &p.lays {
            assert!(lay.plies <= 80 && lay.ratio.values().sum::<u32>() <= 6);
            let mut job = order.job.clone();
            job.order = Some(Order {
                sizes: lay.ratio.clone(),
            });
            assert!(beta_marker_core::validate(&job, lay.result.as_ref().expect("real marker")).ok);
        }
        for (s, n) in &p.planned {
            assert!(
                *n >= order.quantities[s]
                    && *n
                        <= (f64::from(order.quantities[s]) * (1.0 + order.overcut_pct / 100.0))
                            .floor() as u32
            );
        }
        let stem = if awkward {
            "simon-awkward"
        } else {
            "simon-1200"
        };
        let naive_ratio = BTreeMap::from([
            ("S".to_string(), 1u32),
            ("M".to_string(), 2),
            ("L".to_string(), 2),
            ("XL".to_string(), 1),
        ]);
        let naive_total_plies = order
            .quantities
            .iter()
            .map(|(s, q)| q.div_ceil(naive_ratio[s]))
            .max()
            .unwrap_or(0);
        let naive_planned: BTreeMap<_, _> = naive_ratio
            .iter()
            .map(|(s, n)| (s.clone(), n * naive_total_plies))
            .collect();
        let naive_valid = naive_planned.iter().all(|(s, n)| {
            *n <= (f64::from(order.quantities[s]) * (1.0 + order.overcut_pct / 100.0)).floor()
                as u32
        });
        let naive_fabric = baseline.length / 1000.0 * f64::from(naive_total_plies);
        let naive = serde_json::json!({"ratio":naive_ratio,"total_plies":naive_total_plies,"lays":naive_total_plies.div_ceil(order.max_plies),"planned":naive_planned,"within_overcut":naive_valid,"marker_length_mm":baseline.length,"efficiency":baseline.efficiency,"total_fabric_m":naive_fabric,"runtime_s":baseline_runtime});
        std::fs::write(
            root.join(format!("{stem}.naive.json")),
            serde_json::to_vec_pretty(&naive)?,
        )?;
        std::fs::write(
            root.join(format!("{stem}.cutorder.json")),
            serde_json::to_vec_pretty(&order)?,
        )?;
        std::fs::write(
            root.join(format!("{stem}.cutplan.json")),
            serde_json::to_vec_pretty(&p)?,
        )?;
        println!(
            "{stem}: {:.3} m, {} lays, {:.3} s; planned {:?}",
            p.total_fabric_m,
            p.lays.len(),
            p.runtime_s,
            p.planned
        );
        println!(
            "naive: {naive_fabric:.3}m; valid overcut: {naive_valid}; marker runtime {baseline_runtime:.3}s"
        );
        for (i, l) in p.lays.iter().enumerate() {
            println!(
                "{} {:?} x{} {:.3}m {:.2}% {:.3}m",
                i + 1,
                l.ratio,
                l.plies,
                l.marker_length_mm / 1000.0,
                l.efficiency * 100.0,
                l.fabric_m
            );
        }
    }
    Ok(())
}
