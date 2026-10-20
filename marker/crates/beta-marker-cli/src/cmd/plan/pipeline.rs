//! Offline order-to-cutting-room pipeline. No machine is started by this command.
use super::super::roll::{json, read};
use anyhow::{Context, Result, ensure};
use beta_marker_core::{Job, Order, Shrinkage};
use beta_marker_cutroom::{
    bundles::{Bundle, BundleRules, bundle_lay},
    integration::from_spread,
    tickets::{TicketPaper, sticker_pdf, ticket_pdf},
};
use beta_marker_export::{CutPlanTable, ExportOptions};
use beta_marker_plan::{CutOrder, CutPlan, PlanOptions};
use beta_marker_roll::{AllocationSettings, GroupingSettings, RollRegister};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

/// Arguments are public so the lead can wire `Pipeline(cmd::plan::pipeline::Args)`.
#[derive(clap::Args)]
#[command(
    after_help = "Example:\n  beta-marker plan pipeline order.json --rolls rolls.json --time-per-marker 2 --final-time 120 -o cutting-room\n\nThe largest compatible shrinkage band supplies stock. Each lay keeps one lot/dye/shade.\nFiles describe planned production; measured cutting-room reports need actuals."
)]
pub struct Args {
    /// Cut order JSON with patterns and required garments per size.
    pub cutorder: PathBuf,
    /// Inspected roll register JSON.
    #[arg(long)]
    pub rolls: PathBuf,
    /// Search seconds per candidate ratio.
    #[arg(long, default_value_t = 20.0)]
    pub time_per_marker: f64,
    /// Base random seed for repeatable candidate and final searches.
    #[arg(long, default_value_t = 42)]
    pub seed: u64,
    /// Search seconds per chosen ratio after planning.
    #[arg(long, default_value_t = 120.0)]
    pub final_time: f64,
    #[arg(long, default_value_t = 16)]
    pub max_markers: usize,
    #[arg(long, default_value_t = 20_000)]
    pub search_nodes: usize,
    /// Fabric allowance at each end of every ply, mm.
    #[arg(long, default_value_t = 0.0)]
    pub end_allowance: f64,
    #[arg(long, default_value_t = 20)]
    pub pieces_per_bundle: u32,
    /// Fabric colour printed on tickets.
    #[arg(long, default_value = "Unspecified")]
    pub colour: String,
    /// Output directory. Use a fresh directory to keep each run separate.
    #[arg(short, long)]
    pub output: PathBuf,
}

/// Choose one shrinkage band, retaining separate inventory shades within it.
/// Use the largest measured shrinkage on each axis to cover all selected stock.
fn stock(order: &mut CutOrder, register: &RollRegister) -> Result<Vec<String>> {
    let groups = beta_marker_roll::group_rolls(register, &GroupingSettings::default())?;
    let mut bands = BTreeMap::<(i64, i64), Vec<String>>::new();
    for g in groups {
        bands
            .entry((g.length_band, g.width_band))
            .or_default()
            .extend(g.roll_ids);
    }
    let capacity = |ids: &[String]| {
        register
            .rolls
            .iter()
            .filter(|r| ids.contains(&r.id) && r.usable_width_mm >= order.job.fabric.width)
            .map(|r| r.measured_length_m)
            .sum::<f64>()
    };
    let ids = bands
        .into_values()
        .max_by(|a, b| capacity(a).total_cmp(&capacity(b)))
        .context("No roll stock was supplied.")?;
    let selected: Vec<_> = register
        .rolls
        .iter()
        .filter(|r| ids.contains(&r.id) && r.usable_width_mm >= order.job.fabric.width)
        .collect();
    ensure!(
        !selected.is_empty(),
        "No selected roll is wide enough for the marker."
    );
    order.job.fabric.shrinkage = Shrinkage {
        length_pct: selected
            .iter()
            .map(|r| r.shrinkage.length_pct)
            .fold(f64::NEG_INFINITY, f64::max),
        width_pct: selected
            .iter()
            .map(|r| r.shrinkage.width_pct)
            .fold(f64::NEG_INFINITY, f64::max),
    };
    Ok(selected.iter().map(|r| r.id.clone()).collect())
}
fn save(path: &Path, name: &str, value: &impl serde::Serialize) -> Result<()> {
    json(Some(&path.join(name)), value)
}
fn plan_table(plan: &CutPlan) -> CutPlanTable {
    CutPlanTable {
        headers: [
            "Lay",
            "Ratio",
            "Plies",
            "Marker length (mm)",
            "Efficiency (%)",
            "Fabric (m)",
        ]
        .map(String::from)
        .to_vec(),
        rows: plan
            .lays
            .iter()
            .enumerate()
            .map(|(i, l)| {
                vec![
                    format!("L{:03}", i + 1),
                    format!("{:?}", l.ratio),
                    l.plies.to_string(),
                    l.marker_length_mm.to_string(),
                    (100.0 * l.efficiency).to_string(),
                    l.fabric_m.to_string(),
                ]
            })
            .collect(),
    }
}
/// Run every stage and save checked plans, print files and per-lay cutter output.
pub fn run(args: Args) -> Result<()> {
    ensure!(
        !args.output.exists() || fs::read_dir(&args.output)?.next().is_none(),
        "Output directory must be empty; use a fresh directory for this run."
    );
    ensure!(
        args.pieces_per_bundle > 0,
        "Pieces per bundle must be positive."
    );
    let mut order: CutOrder = read(&args.cutorder)?;
    let register: RollRegister = read(&args.rolls)?;
    register.validate()?;
    let eligible = stock(&mut order, &register)?;
    let plan = beta_marker_plan::plan(
        &order,
        &PlanOptions {
            time_per_marker_s: args.time_per_marker,
            seed: args.seed,
            final_time_s: args.final_time,
            max_refinements: args.max_markers,
            search_nodes: args.search_nodes,
            end_allowance_mm: args.end_allowance,
            ..Default::default()
        },
    )?;
    fs::create_dir_all(&args.output)?;
    save(&args.output, "cutorder.json", &order)?;
    save(&args.output, "cutplan.json", &plan)?;
    ensure!(
        plan.complete,
        "Cut order has a shortfall; see cutplan.json."
    );
    let requests = beta_marker_plan::spreading_requests(&plan, order.job.fabric.width, &eligible)?;
    save(&args.output, "lay-requests.json", &requests)?;
    let spread = beta_marker_roll::allocate(&register, &requests, &AllocationSettings::default())?;
    save(&args.output, "spreading-plan.json", &spread)?;
    beta_marker_roll::validate_plan(&register, &spread)?;
    ensure!(
        spread.complete,
        "Roll stock cannot fill every chosen lay; see spreading-plan.json. Split lays or supply compatible rolls."
    );
    let mut bundles: Vec<Bundle> = Vec::new();
    let mut lays = Vec::new();
    // Validate all physical cloth defects before producing any cutter or print file.
    for lay in &spread.lays {
        let i = requests
            .iter()
            .position(|r| r.id == lay.lay.id)
            .context("Unknown spread lay ID.")?;
        let mut job: Job = order.job.clone();
        job.order = Some(Order {
            sizes: plan.lays[i].ratio.clone(),
        });
        let result = plan.lays[i].result.as_ref().context("Lay has no marker.")?;
        let map = beta_marker_roll::map_defects(&register, lay, &Default::default())?;
        save(&args.output, &format!("{}-defects.json", lay.lay.id), &map)?;
        ensure!(
            !map.requires_replanning,
            "Lay {} needs defect replacement/replanning; see its defect map.",
            lay.lay.id
        );
        for mut zone in map.zones {
            for pt in &mut zone.polygon {
                pt[0] -= args.end_allowance;
            }
            job.fabric.defects.push(zone);
        }
        let validation = beta_marker_core::validate(&job, result);
        save(
            &args.output,
            &format!("{}-validation.json", lay.lay.id),
            &validation,
        )?;
        ensure!(
            validation.ok,
            "Lay {} conflicts with allocated roll defects: {:?}",
            lay.lay.id,
            validation.errors
        );
        let floor = from_spread(&job, result, &register, lay, &order.name, &args.colour)?;
        let first_bundle = u32::try_from(bundles.len())?
            .checked_add(1)
            .context("Bundle number overflow.")?;
        let made = bundle_lay(
            &floor,
            BundleRules {
                pieces_per_bundle: args.pieces_per_bundle,
                first_bundle,
                sticker_offset: 0,
            },
        )?;
        save(
            &args.output,
            &format!("{}-cutroom.json", lay.lay.id),
            &floor,
        )?;
        bundles.extend(made);
        lays.push((lay.lay.id.clone(), job, result));
    }
    // Render PDFs before writing them so an invalid ticket cannot leave a print batch.
    let tickets = ticket_pdf(&bundles, TicketPaper::A4)?;
    let stickers = sticker_pdf(&bundles, TicketPaper::A4)?;
    let table = plan_table(&plan);
    let bundle_table = CutPlanTable {
        headers: ["Lay", "Bundle", "Size", "Part", "Shade identity", "Pieces"]
            .map(String::from)
            .to_vec(),
        rows: bundles
            .iter()
            .map(|b| {
                vec![
                    b.lay.clone(),
                    b.number.to_string(),
                    b.size.clone(),
                    b.piece_id.clone(),
                    b.shade_group.clone(),
                    b.stickers.len().to_string(),
                ]
            })
            .collect(),
    };
    let summary = CutPlanTable {
        headers: vec!["Planned production".into(), "Value".into()],
        rows: vec![
            vec!["Fabric (m)".into(), plan.total_fabric_m.to_string()],
            vec![
                "Allocated cloth (m)".into(),
                spread.total_used_m.to_string(),
            ],
            vec![
                "Opened-roll remaining stock (m)".into(),
                spread.total_end_bits_m.to_string(),
            ],
            vec!["Lays".into(), plan.lays.len().to_string()],
            vec!["Bundles".into(), bundles.len().to_string()],
        ],
    };
    let xlsx = beta_marker_export::to_tables_xlsx(&[
        ("Summary", summary),
        ("Cut plan", table),
        ("Bundles", bundle_table),
    ])?;
    save(&args.output, "bundles.json", &bundles)?;
    fs::write(args.output.join("tickets.pdf"), tickets)?;
    fs::write(args.output.join("stickers.pdf"), stickers)?;
    fs::write(args.output.join("report.xlsx"), xlsx)?;
    fs::write(args.output.join("spreading-plan.csv"), spread.to_csv())?;
    for (id, job, result) in lays {
        save(&args.output, &format!("{id}-job.json"), &job)?;
        save(&args.output, &format!("{id}-result.json"), result)?;
        for format in ["plt", "iso", "mini"] {
            let (bytes, _, ext) =
                beta_marker_export::export(format, &job, result, &ExportOptions::default())?;
            fs::write(args.output.join(format!("{id}-{format}.{ext}")), bytes)?;
        }
    }
    println!(
        "Fabric {:.3} m; allocated {:.3} m; {} lays; {} bundles; {} physical pieces.",
        plan.total_fabric_m,
        spread.total_used_m,
        plan.lays.len(),
        bundles.len(),
        bundles.iter().map(|b| b.stickers.len()).sum::<usize>()
    );
    for (i, l) in plan.lays.iter().enumerate() {
        println!(
            "L{:03}: {:?} x {} plies, {:.3} m, {:.2}% efficiency",
            i + 1,
            l.ratio,
            l.plies,
            l.marker_length_mm / 1000.0,
            100.0 * l.efficiency
        );
    }
    for w in &plan.warnings {
        eprintln!("Warning: {w}");
    }
    println!("Files written to {}", args.output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    #[derive(Parser)]
    struct Cli {
        #[command(flatten)]
        args: super::super::Args,
    }
    #[test]
    fn pipeline_help_and_existing_plan_syntax() {
        let parsed = Cli::try_parse_from([
            "plan",
            "pipeline",
            "order.json",
            "--rolls",
            "rolls.json",
            "--seed",
            "1",
            "-o",
            "out",
        ])
        .expect("pipeline arguments");
        let Some(super::super::Command::Pipeline(a)) = parsed.args.command else {
            panic!("pipeline command")
        };
        assert_eq!(a.final_time, 120.0);
        assert_eq!(a.seed, 1);
        let parsed =
            Cli::try_parse_from(["plan", "order.json", "-o", "plan.json"]).expect("legacy plan");
        assert!(parsed.args.command.is_none());
        assert!(Cli::try_parse_from(["plan", "order.json"]).is_err());
    }
    #[test]
    fn shrinkage_selection_is_conservative_and_stock_scoped() {
        let mut order = CutOrder {
            format: beta_marker_plan::CUT_ORDER_FORMAT.into(),
            name: "O1".into(),
            job: Job {
                fabric: beta_marker_core::Fabric {
                    width: 1500.0,
                    ..Default::default()
                },
                ..Default::default()
            },
            quantities: BTreeMap::new(),
            max_plies: 80,
            max_garments_per_marker: 6,
            overcut_pct: 0.0,
        };
        let mut register = beta_marker_roll::demo_register();
        register.rolls[0].shrinkage.length_pct = 2.5;
        register.rolls[1].shrinkage.length_pct = 8.0; // different band, excluded
        let ids = stock(&mut order, &register).expect("stock selection");
        assert_eq!(order.job.fabric.shrinkage.length_pct, 2.5);
        assert_eq!(order.job.fabric.shrinkage.width_pct, 1.0);
        assert!(ids.contains(&"R00".into()));
        assert!(!ids.contains(&"R01".into()));
    }
    #[test]
    fn small_pipeline_writes_checked_print_and_cutter_artifacts() -> Result<()> {
        let root = std::env::var_os("BETA_DATA_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join(format!(
            "marker-bench/integrate/fast-{}",
            std::process::id()
        ));
        fs::create_dir_all(&root)?;
        let cutorder = root.join("order.json");
        let rolls = root.join("rolls.json");
        let job = Job {
            format: beta_marker_core::JOB_FORMAT.into(),
            name: "TEE".into(),
            fabric: beta_marker_core::Fabric {
                width: 1500.0,
                ..Default::default()
            },
            pieces: vec![beta_marker_core::Piece {
                id: "front".into(),
                style: Some("TEE".into()),
                size: Some("M".into()),
                quantity: 1,
                outline: vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]],
                ..Default::default()
            }],
            ..Default::default()
        };
        let order = CutOrder {
            format: beta_marker_plan::CUT_ORDER_FORMAT.into(),
            name: "O1".into(),
            job,
            quantities: BTreeMap::from([("M".into(), 3)]),
            max_plies: 5,
            max_garments_per_marker: 1,
            overcut_pct: 0.0,
        };
        json(Some(&cutorder), &order)?;
        json(Some(&rolls), &beta_marker_roll::demo_register())?;
        let output = root.join("output");
        run(Args {
            cutorder,
            rolls,
            time_per_marker: 0.01,
            seed: 42,
            final_time: 0.02,
            max_markers: 1,
            search_nodes: 100,
            end_allowance: 5.0,
            pieces_per_bundle: 20,
            colour: "Blue".into(),
            output: output.clone(),
        })?;
        for name in [
            "tickets.pdf",
            "stickers.pdf",
            "report.xlsx",
            "L001-plt.plt",
            "L001-iso.iso",
            "L001-validation.json",
        ] {
            assert!(fs::metadata(output.join(name))?.len() > 0, "{name}");
        }
        let bundles: Vec<Bundle> = read(&output.join("bundles.json"))?;
        assert_eq!(bundles.iter().map(|b| b.stickers.len()).sum::<usize>(), 3);
        let p: CutPlan = read(&output.join("cutplan.json"))?;
        let spread: beta_marker_roll::SpreadingPlan = read(&output.join("spreading-plan.json"))?;
        assert!((p.total_fabric_m - spread.total_used_m).abs() < 1e-8);
        Ok(())
    }
    #[test]
    #[ignore = "full Simon production pipeline: long nests, PDFs and cutters under BETA_DATA_DIR"]
    fn simon_long_pipeline() -> Result<()> {
        let root = std::env::var_os("BETA_DATA_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("marker-bench/integrate");
        fs::create_dir_all(&root)?;
        let rolls = root.join("demo-rolls.json");
        json(Some(&rolls), &beta_marker_roll::demo_register())?;
        let orders = root.parent().context("Data parent")?.join("plan");
        let stem = "simon-1200";
        let output = root.join(format!("{stem}-long-{}", std::process::id()));
        run(Args {
            cutorder: orders.join(format!("{stem}.cutorder.json")),
            rolls,
            time_per_marker: 6.0,
            seed: 42,
            final_time: 120.0,
            max_markers: 5,
            search_nodes: 5000,
            end_allowance: 10.0,
            pieces_per_bundle: 20,
            colour: "Blue".into(),
            output,
        })?;
        Ok(())
    }
}
