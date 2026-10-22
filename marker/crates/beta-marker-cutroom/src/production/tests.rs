use super::*;
use beta_marker_core::{Fabric, JOB_FORMAT, MarkerResult, Piece, Placement, RESULT_FORMAT};
use beta_marker_plan::{CUT_PLAN_FORMAT, Lay, PlanOptions};
use std::io::{Cursor, Read};

fn fixture() -> ReportInput {
    let job = Job {
        format: JOB_FORMAT.into(),
        name: "test".into(),
        fabric: Fabric {
            width: 500.0,
            ..Default::default()
        },
        pieces: vec![Piece {
            id: "front".into(),
            name: "Front".into(),
            quantity: 1,
            size: Some("M".into()),
            outline: vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]],
            ..Default::default()
        }],
        order: Some(Order {
            sizes: BTreeMap::from([("M".into(), 1)]),
        }),
        ..Default::default()
    };
    let placements = vec![Placement {
        piece_id: "front".into(),
        bundle: Some(0),
        translate: [10.0, 10.0],
        ..Default::default()
    }];
    let metrics = beta_marker_core::compute_metrics(&job, &placements);
    let marker = MarkerResult {
        format: RESULT_FORMAT.into(),
        job_name: job.name.clone(),
        width: 500.0,
        length: metrics.length,
        piece_area: metrics.piece_area,
        efficiency: metrics.efficiency,
        useful_piece_area: metrics.useful_piece_area,
        useful_yield: metrics.useful_yield,
        surplus_copies: metrics.surplus_copies,
        placements,
        engine: "synthetic report fixture".into(),
        seed: Some(42),
        runtime_s: 0.0,
        warnings: Vec::new(),
    };
    let options = PlanOptions {
        end_allowance_mm: 10.0,
        ..Default::default()
    };
    let plan = CutPlan {
        format: CUT_PLAN_FORMAT.into(),
        order_name: "O1".into(),
        options,
        lays: vec![Lay {
            ratio: BTreeMap::from([("M".into(), 1)]),
            plies: 10,
            passes: 10,
            marker_length_mm: 110.0,
            efficiency: marker.efficiency,
            fabric_m: 1.3,
            result: Some(marker),
        }],
        total_fabric_m: 1.3,
        ordered: BTreeMap::from([("M".into(), 10)]),
        planned: BTreeMap::from([("M".into(), 10)]),
        complete: true,
        ..Default::default()
    };
    let register = RollRegister {
        rolls: vec![beta_marker_roll::Roll {
            id: "=roll,1".into(),
            lot: "LOT".into(),
            dye_lot: "DYE".into(),
            shade_group: "A".into(),
            ticket_length_m: 5.0,
            measured_length_m: 5.0,
            usable_width_mm: 500.0,
            shrinkage: Default::default(),
            defects: vec![],
        }],
    };
    let requests = beta_marker_plan::spreading_requests(&plan, 500.0, &[]).expect("requests");
    let spread =
        beta_marker_roll::allocate(&register, &requests, &Default::default()).expect("spread");
    ReportInput {
        meta: ReportMeta {
            po: "PO1".into(),
            style: "Tee R1".into(),
            ..Default::default()
        },
        job,
        plan,
        gsm: 200.0,
        currency: "INR".into(),
        price_per_m: 100.0,
        lay_cost: 20.0,
        purchase_wastage_pct: 10.0,
        register: Some(register),
        spread: Some(spread),
        comparison: None,
        actuals: Some(OrderActuals {
            good_cut: BTreeMap::from([("M".into(), 9)]),
            rolls: vec![RollActual {
                roll_id: "=roll,1".into(),
                issued_m: 1.8,
                returned_m: 0.4,
                scrap_m: 0.1,
            }],
        }),
    }
}
#[test]
fn cost_units_stock_and_garments_reconcile() -> Result<()> {
    let report = build(&fixture())?;
    let c = &report.costing;
    assert!((c.net_marker_m - 1.1).abs() < 1e-9);
    assert!((c.end_loss_m - 0.2).abs() < 1e-9);
    assert!((c.consumed_m - 1.3).abs() < 1e-9);
    assert!((c.gross_purchase_m - 1.43).abs() < 1e-9);
    assert!((c.gross.m_per_piece - 0.143).abs() < 1e-9);
    assert!((c.gross.m_per_dozen - 1.716).abs() < 1e-9);
    assert!((c.gross.kg_per_piece - 0.0143).abs() < 1e-9);
    assert!((c.gross.kg_per_dozen - 0.1716).abs() < 1e-9);
    assert!((c.gross.yards_per_piece * 0.9144 - c.gross.m_per_piece).abs() < 1e-9);
    assert!((c.total_cost - 163.0).abs() < 1e-9);
    assert!((c.actual_consumed_m.expect("actual") - 1.4).abs() < 1e-9);
    assert!((report.rolls[0].balance_m - 3.6).abs() < 1e-9);
    assert_eq!(report.reconciliation[0].cut_balance, Some(-1));
    assert_eq!(report.lays[0].shades[0].lot, "LOT");
    assert_eq!(report.lays[0].shades[0].dye_lot, "DYE");
    assert_eq!(report.lays[0].shades[0].shade_group, "A");
    assert_eq!(report.lays[0].quantities["M"], 10);
    Ok(())
}
#[test]
fn actuals_stay_blank_and_invalid_balances_fail() -> Result<()> {
    let mut input = fixture();
    input.actuals = None;
    let pack = build(&input)?;
    assert!(
        pack.costing.actual.is_none()
            && pack.rolls[0].actual_used_m.is_none()
            && pack.reconciliation[0].cut.is_none()
    );
    input = fixture();
    input.actuals.as_mut().expect("actuals").rolls[0].returned_m = 2.0;
    assert!(build(&input).is_err());
    input = fixture();
    input.plan.planned.insert("M".into(), 11);
    assert!(build(&input).is_err());
    input = fixture();
    input.gsm = f64::INFINITY;
    assert!(build(&input).is_err());
    input = fixture();
    input.gsm = f64::MAX;
    assert!(build(&input).is_err());
    input = fixture();
    input.comparison = Some(CostComparison {
        currency: "USD".into(),
        trials: Vec::new(),
        chosen: None,
        reason: String::new(),
    });
    assert!(build(&input).is_err());
    Ok(())
}
#[test]
fn exports_preserve_numbers_and_escape_identifiers() -> Result<()> {
    let pack = build(&fixture())?;
    let csv = csv_files(&pack);
    assert_eq!(csv.len(), 6);
    assert!(csv["roll-register.csv"].contains("\"'=roll,1\""));
    let data = xlsx(&pack)?;
    let mut zip = zip::ZipArchive::new(Cursor::new(data))?;
    let mut xml = String::new();
    zip.by_name("xl/worksheets/sheet1.xml")?
        .read_to_string(&mut xml)?;
    assert!(xml.contains("<f>B23/B15</f>"));
    assert!(xml.contains("<f>B27*B13*B14/1000000</f>"));
    for (cell, expected) in [
        ("B13", 500.0),
        ("B25", 163.0),
        ("B26", 16.3),
        ("B27", 0.11),
        ("B34", 0.0143),
    ] {
        let start = xml
            .find(&format!("r=\"{cell}\""))
            .context("Missing costing cell")?;
        let body = xml[start..]
            .split("</c>")
            .next()
            .context("Missing cell end")?;
        let value = body
            .split_once("<v>")
            .and_then(|(_, s)| s.split_once("</v>"))
            .context("Missing cached formula value")?
            .0
            .parse::<f64>()?;
        assert!(
            (value - expected).abs() < 1e-8,
            "Cached {cell}: {value} vs {expected}"
        );
    }
    assert!(!xml.contains("#REF!") && !xml.contains("#DIV/0!"));
    let pdf = pdf(&pack)?;
    assert!(pdf.starts_with(b"%PDF-1.4"));
    assert!(String::from_utf8_lossy(&pdf).contains("/MediaBox [0 0 595.28 841.89]"));
    Ok(())
}

#[test]
fn measured_mass_uses_actual_roll_width() -> Result<()> {
    let mut input = fixture();
    input.register.as_mut().expect("register").rolls[0].usable_width_mm = 600.0;
    // Reallocation is needed after changing the measured register frame.
    let requests = beta_marker_plan::spreading_requests(&input.plan, 500.0, &[])?;
    input.spread = Some(beta_marker_roll::allocate(
        input.register.as_ref().expect("register"),
        &requests,
        &Default::default(),
    )?);
    let report = build(&input)?;
    let actual = report.costing.actual.expect("actual");
    assert!((actual.kg_per_piece - 1.4 * 0.6 * 0.2 / 9.0).abs() < 1e-9);
    assert!((report.costing.gross.kg_per_piece - 1.43 * 0.6 * 0.2 / 10.0).abs() < 1e-9);
    Ok(())
}

#[test]
fn folded_mass_doubles_physical_width_once() -> Result<()> {
    let mut input = fixture();
    input.job.fabric.layout = Layout::Folded;
    input.job.pieces[0].mirror_quantity = 1;
    // A normal folded placement cuts one normal and one mirrored part. The
    // order is still one garment per ply, not two garments.
    let marker = input.plan.lays[0].result.as_mut().expect("marker");
    let metrics = beta_marker_core::compute_metrics(&input.job, &marker.placements);
    marker.piece_area = metrics.piece_area;
    marker.efficiency = metrics.efficiency;
    input.plan.lays[0].efficiency = metrics.efficiency;
    let report = build(&input)?;
    assert_eq!(report.costing.mass_width_mm, 1000.0);
    assert!((report.costing.gross.kg_per_piece - 0.0286).abs() < 1e-9);
    assert_eq!(report.costing.planned_garments, 10);
    Ok(())
}

#[test]
fn shade_identity_keeps_lot_and_dye_separate_even_with_slashes() -> Result<()> {
    let mut input = fixture();
    input.actuals = None;
    let register = input.register.as_mut().expect("register");
    let first = &mut register.rolls[0];
    first.ticket_length_m = 0.7;
    first.measured_length_m = 0.7;
    first.lot = "A/B".into();
    first.dye_lot = "C".into();
    first.shade_group = "D".into();
    let mut second = first.clone();
    second.id = "second".into();
    second.lot = "A".into();
    second.dye_lot = "B/C".into();
    register.rolls.push(second);
    let mut requests = beta_marker_plan::spreading_requests(&input.plan, 500.0, &[])?;
    requests[0].shade_rule = beta_marker_roll::ShadePolicy::AllowMixing;
    input.spread = Some(beta_marker_roll::allocate(
        register,
        &requests,
        &Default::default(),
    )?);
    let report = build(&input)?;
    assert_eq!(report.lays[0].shades.len(), 2);
    assert_ne!(report.lays[0].shades[0], report.lays[0].shades[1]);
    Ok(())
}

#[test]
#[ignore = "writes synthetic XLSX/CSV/PDF for visual inspection under BETA_DATA_DIR"]
fn write_report_pack_for_review() -> Result<()> {
    let dir = std::path::PathBuf::from(std::env::var("BETA_DATA_DIR")?)
        .join("marker-bench/ops/report-sample");
    std::fs::create_dir_all(&dir)?;
    let report = build(&fixture())?;
    std::fs::write(dir.join("report.json"), serde_json::to_vec_pretty(&report)?)?;
    std::fs::write(dir.join("report.pdf"), pdf(&report)?)?;
    std::fs::write(dir.join("report.xlsx"), xlsx(&report)?)?;
    for (name, csv) in csv_files(&report) {
        std::fs::write(dir.join(name), csv)?;
    }
    Ok(())
}
