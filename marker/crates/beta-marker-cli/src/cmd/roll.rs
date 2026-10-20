//! Inspect cloth and turn measured roll stock into spreading instructions.
use anyhow::{Context, Result, ensure};
use beta_marker_roll::*;
use serde::{Serialize, de::DeserializeOwned};
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

#[derive(clap::Args)]
#[command(
    after_help = "Examples:\n  beta-marker roll inspect rolls.json --format text\n  beta-marker roll group rolls.json -o groups.json\n  beta-marker roll spread rolls.json lays.json -o spread.json\n  beta-marker roll defects-to-marker rolls.json spread.json -o defects.json"
)]
pub struct Args {
    #[command(subcommand)]
    command: Command,
}
#[derive(clap::Subcommand)]
enum Command {
    /// Score measured rolls and lots with the four-point inspection system.
    Inspect(Single),
    /// Keep shades apart and group similar shrinkage tests (JSON output).
    Group(Single),
    /// Assign rolls to lays. A lay is a marker spread over fabric plies.
    #[command(alias = "spread")]
    Allocate(Allocate),
    /// Map roll flaws to marker zones or pending replacement instructions.
    #[command(alias = "defects")]
    DefectsToMarker(Defects),
}
#[derive(clap::Args)]
struct Single {
    /// Roll register JSON, with a rolls array.
    register: PathBuf,
    #[command(flatten)]
    output: Output,
    /// Optional JSON settings for this operation.
    #[arg(long)]
    settings: Option<PathBuf>,
}
#[derive(clap::Args)]
struct Allocate {
    /// Measured roll register JSON.
    register: PathBuf,
    /// JSON array of LayRequest records, or a CutPlan JSON with --width.
    lays: PathBuf,
    /// Cuttable width in mm, required when lays is a CutPlan.
    #[arg(long)]
    width: Option<f64>,
    /// Optional allocation settings JSON.
    #[arg(long)]
    settings: Option<PathBuf>,
    #[command(flatten)]
    output: Output,
}
#[derive(clap::Args)]
struct Defects {
    /// Measured roll register JSON.
    register: PathBuf,
    /// Spreading plan JSON.
    spread: PathBuf,
    /// Apply one lay's exclusion zones to this job and write an updated Job JSON.
    #[arg(long)]
    job: Option<PathBuf>,
    /// Lay ID to select when using --job with a multi-lay spread.
    #[arg(long)]
    lay: Option<String>,
    /// Head allowance before marker x=0, in mm, when applying zones to a job.
    #[arg(long, default_value_t = 0.0)]
    marker_offset_mm: f64,
    /// Optional defect mapping settings JSON.
    #[arg(long)]
    settings: Option<PathBuf>,
    #[command(flatten)]
    output: Output,
}
#[derive(Clone, Copy, clap::ValueEnum)]
pub(crate) enum Format {
    Json,
    Csv,
    Text,
}
#[derive(clap::Args)]
pub(crate) struct Output {
    /// Output representation. Grouping and defect maps use JSON only.
    #[arg(long, value_enum, default_value = "json")]
    pub format: Format,
    /// Destination file. Omit to write to standard output.
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}
pub(crate) fn read<T: DeserializeOwned>(path: &Path) -> Result<T> {
    serde_json::from_slice(
        &fs::read(path).with_context(|| format!("Cannot read {}", path.display()))?,
    )
    .with_context(|| format!("Invalid JSON in {}", path.display()))
}
pub(crate) fn settings<T: DeserializeOwned + Default>(path: &Option<PathBuf>) -> Result<T> {
    path.as_deref().map_or_else(|| Ok(T::default()), read)
}
pub(crate) fn write(path: Option<&Path>, bytes: &[u8]) -> Result<()> {
    if let Some(path) = path {
        fs::write(path, bytes).with_context(|| format!("Cannot write {}", path.display()))?;
    } else {
        io::stdout().lock().write_all(bytes)?;
    }
    Ok(())
}
pub(crate) fn json(path: Option<&Path>, value: &impl Serialize) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    write(path, &bytes)
}
fn only_json(output: &Output) -> Result<()> {
    ensure!(
        matches!(output.format, Format::Json),
        "This operation supports --format json only."
    );
    Ok(())
}
/// Run the selected operation. Incomplete allocations are saved, then fail.
pub fn run(args: Args) -> Result<()> {
    match args.command {
        Command::Inspect(a) => {
            let report = inspect(&read(&a.register)?, &settings(&a.settings)?)?;
            match a.output.format {
                Format::Json => json(a.output.output.as_deref(), &report),
                Format::Csv => write(a.output.output.as_deref(), report.to_csv().as_bytes()),
                Format::Text => write(a.output.output.as_deref(), report.to_text().as_bytes()),
            }
        }
        Command::Group(a) => {
            only_json(&a.output)?;
            json(
                a.output.output.as_deref(),
                &group_rolls(&read(&a.register)?, &settings(&a.settings)?)?,
            )
        }
        Command::Allocate(a) => {
            let value: serde_json::Value = read(&a.lays)?;
            let lays = if value.is_array() {
                serde_json::from_value(value)?
            } else {
                let plan: beta_marker_plan::CutPlan = serde_json::from_value(value)?;
                beta_marker_plan::spreading_requests(
                    &plan,
                    a.width.context("A CutPlan needs --width in mm.")?,
                    &[],
                )?
            };
            let plan = allocate(&read(&a.register)?, &lays, &settings(&a.settings)?)?;
            match a.output.format {
                Format::Json => json(a.output.output.as_deref(), &plan)?,
                Format::Csv => write(a.output.output.as_deref(), plan.to_csv().as_bytes())?,
                Format::Text => write(a.output.output.as_deref(), plan.to_text().as_bytes())?,
            }
            ensure!(
                plan.complete,
                "Stock allocation is incomplete; see the saved spreading plan."
            );
            Ok(())
        }
        Command::DefectsToMarker(a) => {
            only_json(&a.output)?;
            let register = read(&a.register)?;
            let plan: SpreadingPlan = read(&a.spread)?;
            validate_plan(&register, &plan)?;
            let opts = settings(&a.settings)?;
            let maps: Result<Vec<_>> = plan
                .lays
                .iter()
                .map(|l| map_defects(&register, l, &opts))
                .collect();
            let maps = maps?;
            if let Some(path) = a.job {
                ensure!(
                    a.marker_offset_mm.is_finite() && a.marker_offset_mm >= 0.0,
                    "Marker offset must be finite and nonnegative."
                );
                ensure!(
                    maps.len() == 1 || a.lay.is_some(),
                    "Select --lay when applying a multi-lay spread to a job."
                );
                let map = maps
                    .iter()
                    .find(|m| a.lay.as_ref().is_none_or(|id| *id == m.lay_id))
                    .context("Lay ID was not found.")?;
                ensure!(
                    !map.requires_replanning,
                    "This lay needs replacement cloth. Export the defect map without --job to review its instructions."
                );
                let mut job: beta_marker_core::Job = read(&path)?;
                for mut zone in map.zones.clone() {
                    for pt in &mut zone.polygon {
                        pt[0] -= a.marker_offset_mm;
                    }
                    job.fabric.defects.push(zone);
                }
                json(a.output.output.as_deref(), &job)
            } else {
                ensure!(
                    a.lay.is_none() && a.marker_offset_mm == 0.0,
                    "--lay and --marker-offset-mm need --job."
                );
                json(a.output.output.as_deref(), &maps)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    #[derive(Parser)]
    struct Cli {
        #[command(flatten)]
        args: Args,
    }
    #[test]
    fn allocation_saves_auditable_shortfall_and_reports_failure() -> Result<()> {
        let root = std::env::var_os("BETA_DATA_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join(format!(
            "marker-bench/integrate/roll-shortfall-{}",
            std::process::id()
        ));
        fs::create_dir_all(&root)?;
        let mut register = demo_register();
        register.rolls.truncate(1);
        register.rolls[0].measured_length_m = 1.0;
        let path = root.join("rolls.json");
        json(Some(&path), &register)?;
        let lays = root.join("lays.json");
        json(
            Some(&lays),
            &vec![LayRequest {
                id: "L1".into(),
                marker_length_m: 0.8,
                plies: 3,
                fabric_width_mm: 1500.0,
                shade_rule: ShadePolicy::SameGroup,
                eligible_roll_ids: Vec::new(),
                allow_splicing: false,
                splice_zones: Vec::new(),
            }],
        )?;
        let output = root.join("spread.json");
        let parsed = Cli::try_parse_from([
            "roll",
            "spread",
            path.to_str().context("path")?,
            lays.to_str().context("path")?,
            "-o",
            output.to_str().context("path")?,
        ])?;
        assert!(run(parsed.args).is_err());
        let saved: SpreadingPlan = read(&output)?;
        assert!(!saved.complete);
        validate_plan(&register, &saved)?;
        for command in ["inspect", "group"] {
            let output = root.join(format!("{command}.json"));
            run(Cli::try_parse_from([
                "roll",
                command,
                path.to_str().context("path")?,
                "-o",
                output.to_str().context("path")?,
            ])?
            .args)?;
            assert!(fs::metadata(output)?.len() > 0);
        }
        Ok(())
    }
}
