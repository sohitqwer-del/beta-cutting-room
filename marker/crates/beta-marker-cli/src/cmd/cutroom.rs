//! Bundle and number cut pieces, print offline tickets, and reconcile the room.
use super::roll::{Format, Output, json, read, write};
use anyhow::{Result, ensure};
use beta_marker_cutroom::{bundles::*, costing::*, recuts::*, reports::*, tickets::*};
use std::path::PathBuf;

#[derive(clap::Args)]
#[command(
    after_help = "Examples:\n  beta-marker cutroom bundle lay.json -o bundles.json\n  beta-marker cutroom tickets bundles.json -o tickets.pdf\n  beta-marker cutroom stickers bundles.json --paper label -o stickers.pdf\n  beta-marker cutroom recut job.json bundles.json damage.json -o recuts.json\n  beta-marker cutroom report actuals.json --format csv -o report.csv\n  beta-marker cutroom cost job.json --garments 6 -o estimate.json"
)]
pub struct Args {
    #[command(subcommand)]
    command: Command,
}
#[derive(clap::Subcommand)]
enum Command {
    /// Group physical parts by size, shade and handedness. Input is a cutroom Lay.
    Bundle(BundleArgs),
    /// Print a QR ticket for each sewing-floor bundle.
    Tickets(PrintArgs),
    /// Print one QR numbering sticker for each physical piece.
    Stickers(PrintArgs),
    /// Schedule damaged pieces for replacement from their original shade.
    Recut(RecutArgs),
    /// Reconcile a JSON array of measured LayActuals; include an order total.
    Report(ReportArgs),
    /// Estimate fabric consumption from geometry or a short real nest.
    Cost(CostArgs),
}
#[derive(clap::Args)]
struct BundleArgs {
    lay: PathBuf,
    #[arg(long, default_value_t = 20)]
    pieces_per_bundle: u32,
    #[arg(long, default_value_t = 1)]
    first_bundle: u32,
    #[arg(long, default_value_t = 0)]
    sticker_offset: u32,
    #[arg(short, long)]
    output: PathBuf,
}
#[derive(Clone, Copy, clap::ValueEnum)]
enum Paper {
    A4,
    Label,
}
#[derive(clap::Args)]
struct PrintArgs {
    bundles: PathBuf,
    /// A4 has ten 100 x 50 mm labels; label uses one 100 x 50 mm page.
    #[arg(long, value_enum, default_value = "a4")]
    paper: Paper,
    #[arg(short, long)]
    output: PathBuf,
}
#[derive(clap::Args)]
struct RecutArgs {
    job: PathBuf,
    bundles: PathBuf,
    damage: PathBuf,
    #[arg(long, default_value = "R1")]
    batch_id: String,
    /// Save both scheduled jobs and the updated ledger in one JSON object.
    #[arg(short, long)]
    output: PathBuf,
}
#[derive(clap::Args)]
struct ReportArgs {
    actuals: PathBuf,
    #[command(flatten)]
    output: Output,
}
#[derive(clap::Args)]
struct CostArgs {
    job: PathBuf,
    /// Complete garments represented by this job's size ratio.
    #[arg(long)]
    garments: u32,
    /// Override fabric width, in mm.
    #[arg(long)]
    width: Option<f64>,
    /// Fitted efficiency model JSON. Omit for the uncalibrated heuristic.
    #[arg(long)]
    model: Option<PathBuf>,
    /// Use a real nest with this search budget, seconds (at most 20).
    #[arg(long)]
    accurate: Option<f64>,
    #[arg(long, default_value_t = 42)]
    seed: u64,
    #[arg(short, long)]
    output: PathBuf,
}
/// Execute one offline cutting-room operation.
pub fn run(args: Args) -> Result<()> {
    let tickets = matches!(args.command, Command::Tickets(_));
    match args.command {
        Command::Bundle(a) => json(
            Some(&a.output),
            &bundle_lay(
                &read(&a.lay)?,
                BundleRules {
                    pieces_per_bundle: a.pieces_per_bundle,
                    first_bundle: a.first_bundle,
                    sticker_offset: a.sticker_offset,
                },
            )?,
        ),
        Command::Tickets(a) | Command::Stickers(a) => {
            let bundles: Vec<Bundle> = read(&a.bundles)?;
            let paper = match a.paper {
                Paper::A4 => TicketPaper::A4,
                Paper::Label => TicketPaper::Label100x50,
            };
            // The two commands share validated input and paper controls.
            let bytes = if tickets {
                ticket_pdf(&bundles, paper)?
            } else {
                sticker_pdf(&bundles, paper)?
            };
            write(Some(&a.output), &bytes)
        }
        Command::Recut(a) => {
            let bundles: Vec<Bundle> = read(&a.bundles)?;
            let mut ledger: Vec<Damage> = read(&a.damage)?;
            let jobs = schedule_recuts(&read(&a.job)?, &bundles, &mut ledger, &a.batch_id)?;
            json(
                Some(&a.output),
                &serde_json::json!({"jobs": jobs, "ledger": ledger}),
            )
        }
        Command::Report(a) => {
            let actuals: Vec<LayActuals> = read(&a.actuals)?;
            let mut reports: Vec<_> = actuals.iter().map(lay_report).collect::<Result<_>>()?;
            reports.push(order_report(&actuals)?);
            match a.output.format {
                Format::Json => json(a.output.output.as_deref(), &reports),
                Format::Csv => write(a.output.output.as_deref(), reports_csv(&reports).as_bytes()),
                Format::Text => write(
                    a.output.output.as_deref(),
                    reports
                        .iter()
                        .map(report_text)
                        .collect::<Vec<_>>()
                        .join("\n")
                        .as_bytes(),
                ),
            }
        }
        Command::Cost(a) => {
            let job: beta_marker_core::Job = read(&a.job)?;
            let width = a.width.unwrap_or(job.fabric.width);
            ensure!(
                a.model.is_none() || a.accurate.is_none(),
                "Choose --model or --accurate."
            );
            if let Some(time) = a.accurate {
                json(
                    Some(&a.output),
                    &accurate_estimate(&job, width, a.garments, time, a.seed)?,
                )
            } else {
                let model = a
                    .model
                    .as_deref()
                    .map(read)
                    .transpose()?
                    .unwrap_or_default();
                json(Some(&a.output), &estimate(&job, width, a.garments, &model)?)
            }
        }
    }
}
