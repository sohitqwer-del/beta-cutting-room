//! Plan a cut order and print its lay and quantity tables.
use anyhow::{Context, Result, ensure};
use beta_marker_plan::{CutOrder, PlanOptions};
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;
pub mod pipeline;

/// Arguments for `beta-marker plan`.
#[derive(clap::Args)]
#[command(
    subcommand_precedence_over_arg = true,
    subcommand_negates_reqs = true,
    after_help = "Examples:\n  beta-marker plan order.json --final-time 120 -o plan.json\n  beta-marker plan pipeline order.json --rolls rolls.json -o cutting-room"
)]
pub struct Args {
    /// Cut order JSON containing the pattern job and quantities.
    #[arg(required = true)]
    pub cutorder: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Option<Command>,
    /// Final nesting seconds per chosen ratio. Zero keeps short markers.
    #[arg(long, default_value_t = 0.0)]
    pub final_time: f64,
    /// Short nesting budget per distinct ratio, seconds.
    #[arg(long, default_value_t = 20.0)]
    pub time_per_marker: f64,
    /// Base random seed for repeatable candidate and final searches.
    #[arg(long, default_value_t = 42)]
    pub seed: u64,
    /// Cost of starting one lay, expressed as metres of fabric.
    #[arg(long, default_value_t = 0.5)]
    pub lay_cost: f64,
    /// Fabric allowance at each end of each ply, millimetres.
    #[arg(long, default_value_t = 0.0)]
    pub end_allowance: f64,
    /// Maximum number of distinct real markers to evaluate.
    #[arg(long, default_value_t = 16)]
    pub max_markers: usize,
    /// Maximum integer search states per optimisation pass.
    #[arg(long, default_value_t = 20_000)]
    pub search_nodes: usize,
    /// Output cut plan JSON.
    #[arg(short, long, required = true)]
    pub output: Option<PathBuf>,
}

/// Plan operations kept here until the lead adds the top-level pipeline alias.
#[derive(clap::Subcommand)]
pub enum Command {
    /// Plan, re-nest, validate, allocate rolls and write cutting-room files.
    Pipeline(pipeline::Args),
}

/// Write the plan even when incomplete, then return a failure for a shortfall.
pub fn run(args: Args) -> Result<()> {
    if let Some(Command::Pipeline(args)) = args.command {
        return pipeline::run(args);
    }
    let cutorder = args.cutorder.context("A cut order JSON is required.")?;
    let output = args.output.context("An output path is required.")?;
    let order: CutOrder = serde_json::from_reader(BufReader::new(
        File::open(&cutorder).with_context(|| format!("could not read {}", cutorder.display()))?,
    ))
    .context("invalid cut order JSON")?;
    let p = beta_marker_plan::plan(
        &order,
        &PlanOptions {
            time_per_marker_s: args.time_per_marker,
            seed: args.seed,
            final_time_s: args.final_time,
            lay_cost_m: args.lay_cost,
            end_allowance_mm: args.end_allowance,
            max_refinements: args.max_markers,
            search_nodes: args.search_nodes,
            ..Default::default()
        },
    )?;
    let mut writer = BufWriter::new(
        File::create(&output).with_context(|| format!("could not write {}", output.display()))?,
    );
    serde_json::to_writer_pretty(&mut writer, &p)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    println!("Lay | Ratio | Plies | Length (m) | Efficiency | Fabric (m)");
    for (i, l) in p.lays.iter().enumerate() {
        let ratio = l
            .ratio
            .iter()
            .map(|(s, n)| format!("{s}:{n}"))
            .collect::<Vec<_>>()
            .join(" ");
        println!(
            "{} | {} | {} | {:.3} | {:.2}% | {:.3}",
            i + 1,
            ratio,
            l.plies,
            l.marker_length_mm / 1000.0,
            l.efficiency * 100.0,
            l.fabric_m
        );
    }
    println!("\nSize | Ordered | Planned | Extra | Missing");
    for (s, q) in &p.ordered {
        println!(
            "{s} | {q} | {} | {} | {}",
            p.planned[s],
            p.overcut.get(s).copied().unwrap_or(0),
            p.shortfall.get(s).copied().unwrap_or(0)
        );
    }
    println!(
        "\nFabric: {:.3} m; objective: {:.3} m; lays: {}; runtime: {:.2} s",
        p.total_fabric_m,
        p.objective_m,
        p.lays.len(),
        p.runtime_s
    );
    for warning in &p.warnings {
        eprintln!("Warning: {warning}");
    }
    ensure!(
        p.complete,
        "cut plan has a shortfall; see {}",
        output.display()
    );
    Ok(())
}
