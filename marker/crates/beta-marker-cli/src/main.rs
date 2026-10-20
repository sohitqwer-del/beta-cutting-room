//! `beta-marker` command line. Each subcommand lives in its own file under `cmd/`
//! so that different agents can work on them without conflicts.

use clap::{Parser, Subcommand};

mod cmd;

#[derive(Parser)]
#[command(name = "beta-marker", version, about = "BETA Marker: open-source garment marker planner")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Import a DXF-AAMA/ASTM pattern file and write a job JSON
    Import(cmd::import::Args),
    /// Nest a job JSON and write a result JSON
    Nest(cmd::nest::Args),
    /// Check a result against its job with the independent validator
    Validate(cmd::validate::Args),
    /// Write a marker as SVG, PLT, PDF, ISO cutter, DXF, Excel, labels or mini-marker
    Export(cmd::export::Args),
    /// Run the local web app and HTTP API
    Serve(cmd::serve::Args),
    /// Run benchmarks (academic instances, Lectra dataset sample)
    Bench(cmd::bench::Args),
    /// Plan a cut order: lays (size ratios and plies) and fabric use
    Plan(cmd::plan::Args),
    /// Whole cutting-room run: rolls -> plan -> final nests -> spreading -> bundles -> files
    Pipeline(cmd::plan::pipeline::Args),
    /// Nest many jobs (a folder or list), optionally at several widths
    Batch(cmd::batch::Args),
    /// Fabric rolls: inspection, shade/shrinkage groups, spreading plans
    Roll(cmd::roll::Args),
    /// Cutting room: bundles, tickets, re-cuts, reports, costing
    Cutroom(cmd::cutroom::Args),
    /// Persistent unattended production jobs, checkpoints and recovery
    Queue(cmd::queue::Args),
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Command::Import(a) => cmd::import::run(a),
        Command::Nest(a) => cmd::nest::run(a),
        Command::Validate(a) => cmd::validate::run(a),
        Command::Export(a) => cmd::export::run(a),
        Command::Serve(a) => cmd::serve::run(a),
        Command::Bench(a) => cmd::bench::run(a),
        Command::Plan(a) => cmd::plan::run(a),
        Command::Pipeline(a) => cmd::plan::pipeline::run(a),
        Command::Batch(a) => cmd::batch::run(a),
        Command::Roll(a) => cmd::roll::run(a),
        Command::Cutroom(a) => cmd::cutroom::run(a),
        Command::Queue(a) => cmd::queue::run(a),
    }
}
