use super::*;
use crate::cost::*;
use beta_marker_core::{Fabric, JOB_FORMAT, Piece, Placement, RESULT_FORMAT};
use beta_marker_roll::RollRegister;

fn order() -> CutOrder {
    CutOrder {
        format: CUT_ORDER_FORMAT.into(),
        name: "rectangles".into(),
        job: Job {
            format: JOB_FORMAT.into(),
            name: "rectangles".into(),
            fabric: Fabric {
                width: 220.0,
                ..Default::default()
            },
            pieces: vec![Piece {
                id: "p".into(),
                name: "p".into(),
                quantity: 1,
                size: Some("M".into()),
                outline: vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]],
                ..Default::default()
            }],
            ..Default::default()
        },
        quantities: BTreeMap::from([("M".into(), 4)]),
        max_plies: 10,
        max_garments_per_marker: 2,
        overcut_pct: 0.0,
    }
}
fn fake(job: &Job, seconds: f64) -> MarkerResult {
    let count = job.order.as_ref().expect("ratio").sizes["M"];
    let placements = (0..count)
        .map(|i| Placement {
            piece_id: "p".into(),
            copy: i,
            bundle: Some(i),
            translate: if seconds < 0.02 {
                [1.0 + 119.0 * f64::from(i), 1.0]
            } else {
                [1.0, 1.0 + 110.0 * f64::from(i)]
            },
            ..Default::default()
        })
        .collect::<Vec<_>>();
    let m = beta_marker_core::compute_metrics(job, &placements);
    MarkerResult {
        format: RESULT_FORMAT.into(),
        job_name: job.name.clone(),
        width: job.fabric.width,
        length: m.length,
        piece_area: m.piece_area,
        efficiency: m.efficiency,
        useful_piece_area: m.useful_piece_area,
        useful_yield: m.useful_yield,
        surplus_copies: m.surplus_copies,
        placements,
        engine: "synthetic test table".into(),
        seed: Some(42),
        runtime_s: 0.0,
        warnings: Vec::new(),
    }
}
#[test]
fn class_probes_fix_short_search_bias_with_actual_final_lengths() -> Result<()> {
    let order = order();
    let opts = PlanOptions {
        time_per_marker_s: 0.01,
        final_time_s: 0.03,
        max_candidates: 2,
        max_refinements: 2,
        lay_cost_m: 0.0,
        ..Default::default()
    };
    let mut calls = Vec::new();
    let plan = plan_ratios_with_engine(&order, &opts, &[], |_| Ok(()), &mut |job, options| {
        calls.push((
            job.order.as_ref().expect("ratio").sizes["M"],
            options.time_limit_s,
        ));
        Ok(fake(job, options.time_limit_s))
    })?;
    assert!(plan.complete);
    assert_eq!(plan.lays.len(), 1);
    assert_eq!(plan.lays[0].ratio["M"], 2);
    assert!((plan.total_fabric_m - 0.202).abs() < 1e-9);
    assert!(calls.contains(&(1, 0.03)) && calls.contains(&(2, 0.03)));
    assert!(
        beta_marker_core::validate(
            &Job {
                order: Some(Order {
                    sizes: plan.lays[0].ratio.clone()
                }),
                ..order.job
            },
            plan.lays[0].result.as_ref().expect("marker")
        )
        .ok
    );
    Ok(())
}
fn offer() -> WidthOffer {
    WidthOffer {
        id: "stock".into(),
        width_mm: 220.0,
        price: FabricPrice::Metre { price: 100.0 },
        lay_cost: 10.0,
        purchase_wastage_pct: 10.0,
        end_bit_charge_fraction: 0.5,
        rolls: None,
        eligible_roll_ids: Vec::new(),
    }
}
#[test]
fn cost_balance_charges_remainder_once_and_kg_uses_width() -> Result<()> {
    let order = order();
    let options = PlanOptions {
        time_per_marker_s: 0.01,
        final_time_s: 0.03,
        max_candidates: 2,
        max_refinements: 2,
        lay_cost_m: 0.0,
        end_allowance_mm: 10.0,
        ..Default::default()
    };
    let plan = plan_ratios_with_engine(&order, &options, &[], |_| Ok(()), &mut |job, opts| {
        Ok(fake(job, opts.time_limit_s))
    })?;
    let mut offer = offer();
    offer.rolls = Some(RollRegister {
        rolls: vec![beta_marker_roll::Roll {
            id: "r".into(),
            lot: "lot".into(),
            dye_lot: "dye".into(),
            shade_group: "A".into(),
            ticket_length_m: 1.0,
            measured_length_m: 1.0,
            usable_width_mm: 220.0,
            shrinkage: Default::default(),
            defects: vec![],
        }],
    });
    let (cost, spread) = price_plan(&plan, &offer, &Default::default())?;
    assert!(spread.expect("stock").complete);
    assert!((cost.consumed_m - 0.242).abs() < 1e-9);
    assert!((cost.recoverable_end_bits_m - 0.758).abs() < 1e-9);
    assert!((cost.purchased_m - (0.242 + 0.379 + 0.0242)).abs() < 1e-9);
    assert!((cost.total - 74.52).abs() < 1e-8);
    let kg = FabricPrice::Kilogram {
        price: 200.0,
        gsm: 180.0,
    };
    assert!((kg.per_metre(1500.0)? - 54.0).abs() < 1e-9);
    assert!(
        FabricPrice::Metre { price: f64::NAN }
            .per_metre(1500.0)
            .is_err()
    );
    Ok(())
}
#[test]
fn final_width_selection_uses_cost_not_efficiency_and_keeps_failures() -> Result<()> {
    let mut cheap = offer();
    cheap.id = "cheap-wide".into();
    cheap.width_mm = 230.0;
    cheap.price = FabricPrice::Metre { price: 40.0 };
    cheap.lay_cost = 0.0;
    cheap.purchase_wastage_pct = 0.0;
    let mut expensive = cheap.clone();
    expensive.id = "expensive-narrow".into();
    expensive.width_mm = 220.0;
    expensive.price = FabricPrice::Metre { price: 100.0 };
    let mut failed = expensive.clone();
    failed.id = "unavailable".into();
    failed.width_mm = 210.0;
    let request = CostRequest {
        order: order(),
        currency: "INR".into(),
        offers: vec![cheap, expensive, failed],
        ratio_sets: vec![],
        options: PlanOptions {
            time_per_marker_s: 0.01,
            max_candidates: 2,
            max_refinements: 2,
            ..Default::default()
        },
        finalists: 2,
        final_time_s: 0.03,
        allocation: Default::default(),
    };
    let result = compare_with_engine(&request, |_| Ok(()), &mut |job, opts| {
        if job.fabric.width < 220.0 {
            anyhow::bail!("Synthetic width failure");
        }
        Ok(fake(job, opts.time_limit_s))
    })?;
    let selected = &result.trials[result.chosen.expect("chosen")];
    assert_eq!(selected.offer_id, "cheap-wide");
    assert!(selected.final_evaluated);
    assert!(!result.trials[2].feasible && result.trials[2].error.is_some());
    assert!(
        selected.plan.as_ref().expect("chosen plan").lays[0].efficiency
            < result.trials[1]
                .plan
                .as_ref()
                .expect("valid narrower plan")
                .lays[0]
                .efficiency
    );
    assert!(result.reason.contains("lowest feasible final cost"));
    Ok(())
}

#[test]
fn explicit_ratio_menu_needs_no_unrequested_single_size_repairs() -> Result<()> {
    let mut order = order();
    order.quantities.insert("S".into(), 4);
    let mut second = order.job.pieces[0].clone();
    second.id = "s".into();
    second.size = Some("S".into());
    order.job.pieces.push(second);
    let ratio = BTreeMap::from([("M".into(), 1), ("S".into(), 1)]);
    let opts = PlanOptions {
        max_candidates: 1,
        max_refinements: 1,
        ..Default::default()
    };
    let mut seen = Vec::new();
    let plan = plan_ratios_with_engine(
        &order,
        &opts,
        std::slice::from_ref(&ratio),
        |_| Ok(()),
        &mut |job, _| {
            seen.push(job.order.as_ref().expect("ratio").sizes.clone());
            anyhow::bail!("Synthetic unavailable ratio")
        },
    )?;
    assert_eq!(seen, vec![ratio]);
    assert!(!plan.complete);
    Ok(())
}

#[test]
fn stock_must_not_mix_incompatible_shrinkage_bands() -> Result<()> {
    let mut order = order();
    order.job.fabric.shrinkage = beta_marker_core::Shrinkage {
        length_pct: 2.0,
        width_pct: 2.0,
    };
    let mut stock = offer();
    stock.rolls = Some(RollRegister {
        rolls: [0.0, 2.0]
            .into_iter()
            .enumerate()
            .map(|(i, length_pct)| beta_marker_roll::Roll {
                id: format!("roll-{i}"),
                lot: "L".into(),
                dye_lot: "D".into(),
                shade_group: "A".into(),
                ticket_length_m: 10.0,
                measured_length_m: 10.0,
                usable_width_mm: 220.0,
                shrinkage: beta_marker_core::Shrinkage {
                    length_pct,
                    width_pct: 0.0,
                },
                defects: Vec::new(),
            })
            .collect(),
    });
    let request = CostRequest {
        order,
        currency: "INR".into(),
        offers: vec![stock],
        ratio_sets: vec![],
        options: PlanOptions {
            time_per_marker_s: 0.01,
            max_candidates: 2,
            max_refinements: 2,
            ..Default::default()
        },
        finalists: 1,
        final_time_s: 0.03,
        allocation: Default::default(),
    };
    let mut calls = 0;
    let err = compare_with_engine(&request, |_| Ok(()), &mut |_, _| {
        calls += 1;
        anyhow::bail!("Unexpected nest")
    })
    .expect_err("incompatible stock");
    assert!(err.to_string().contains("shrinkage bands"));
    assert_eq!(calls, 0);
    Ok(())
}

#[test]
#[ignore = "real Simon width/cost measurements under BETA_DATA_DIR; 2 workers, <=5 s searches"]
fn simon_width_cost_report() -> Result<()> {
    let root = std::path::PathBuf::from(std::env::var("BETA_DATA_DIR")?).join("marker-bench/plan");
    for stem in ["simon-1200", "simon-awkward"] {
        let mut order: CutOrder =
            serde_json::from_slice(&std::fs::read(root.join(format!("{stem}.cutorder.json")))?)?;
        let register = beta_marker_roll::demo_register();
        order.job.fabric.shrinkage = beta_marker_core::Shrinkage {
            length_pct: 2.0,
            width_pct: 1.0,
        };
        let request = CostRequest {
            order,
            currency: "INR".into(),
            offers: [1450.0, 1500.0]
                .into_iter()
                .map(|width| WidthOffer {
                    id: format!("width-{width}"),
                    width_mm: width,
                    price: FabricPrice::Kilogram {
                        price: 240.0,
                        gsm: 150.0,
                    },
                    lay_cost: 150.0,
                    purchase_wastage_pct: 2.0,
                    end_bit_charge_fraction: 0.25,
                    rolls: Some(register.clone()),
                    eligible_roll_ids: vec![],
                })
                .collect(),
            ratio_sets: vec![],
            options: PlanOptions {
                time_per_marker_s: 0.05,
                final_time_s: 0.0,
                max_refinements: 5,
                max_candidates: 32,
                search_nodes: 3000,
                ..Default::default()
            },
            finalists: 2,
            final_time_s: 0.3,
            allocation: Default::default(),
        };
        let compared = compare(&request)?;
        assert!(compared.chosen.is_some());
        std::fs::write(
            root.join(format!("{stem}-ops-width-cost.json")),
            serde_json::to_vec_pretty(&compared)?,
        )?;
        println!("{stem}: {}", compared.reason);
        for trial in &compared.trials {
            if let (Some(p), Some(c)) = (&trial.plan, &trial.cost) {
                println!(
                    "{} {:.3} m {:.2} INR {} lays: {:?}",
                    trial.offer_id,
                    p.total_fabric_m,
                    c.total,
                    p.lays.len(),
                    p.lays.iter().map(|l| l.efficiency).collect::<Vec<_>>()
                );
            }
        }
    }
    Ok(())
}
