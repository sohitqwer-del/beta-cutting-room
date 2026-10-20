//! `beta-marker export`: write cutting-room output from a job and result.
use anyhow::Context;
use beta_marker_core::{Job, MarkerResult};
use beta_marker_export::{
    ExportOptions, IsoProfile, LabelOptions, to_dxf, to_hpgl, to_iso, to_labels_pdf,
    to_mini_marker_pdf, to_pdf, to_report, to_svg, to_xlsx,
};
use std::{
    fs::{self, File},
    path::PathBuf,
};

/// Supported output file formats.
#[derive(Clone, Copy, clap::ValueEnum)]
pub enum Format {
    /// Scalable vector preview.
    Svg,
    /// HPGL pen plotter file.
    Plt,
    /// Vector PDF at actual scale.
    Pdf,
    /// Plain-text cutting report.
    Report,
    /// Generic ISO 6983-style knife-table program (requires a verified profile).
    Iso,
    /// R12 marker DXF, in millimetres.
    Dxf,
    /// Excel cutting report.
    Xlsx,
    /// A4 placement sticker sheet.
    Labels,
    /// A4 mini-marker overview.
    Mini,
}
/// Output format, drawing controls and paths.
#[derive(clap::Args)]
pub struct Args {
    /// Job JSON file.
    pub job: PathBuf,
    /// Marker result JSON file.
    pub result: PathBuf,
    /// Output file format.
    #[arg(long, value_enum)]
    pub format: Format,
    /// Omit pattern marks.
    #[arg(long)]
    pub no_marks: bool,
    /// Omit piece labels (the job title is always included).
    #[arg(long)]
    pub no_labels: bool,
    /// PDF tile side in mm (must exceed 10 mm); pages never exceed 5080 mm.
    #[arg(long)]
    pub tile: Option<f64>,
    /// Rotate HPGL 90 degrees to feed marker length along plotter y.
    #[arg(long)]
    pub rotate: bool,
    /// Factory post-processor profile JSON (ISO output only).
    #[arg(long)]
    pub profile: Option<PathBuf>,
    /// Destination file.
    #[arg(short, long)]
    pub output: PathBuf,
}
/// Export using the shared geometry transform and recomputed marker metrics.
pub fn run(args: Args) -> anyhow::Result<()> {
    let job: Job = serde_json::from_reader(
        File::open(&args.job).with_context(|| format!("Cannot open {}", args.job.display()))?,
    )
    .context("Cannot read job JSON")?;
    let result: MarkerResult = serde_json::from_reader(
        File::open(&args.result)
            .with_context(|| format!("Cannot open {}", args.result.display()))?,
    )
    .context("Cannot read result JSON")?;
    let opts = ExportOptions {
        marks: !args.no_marks,
        labels: !args.no_labels,
        pdf_tile_mm: args.tile,
        rotate: args.rotate,
    };
    anyhow::ensure!(
        args.profile.is_none() || matches!(args.format, Format::Iso),
        "--profile is only supported for ISO output."
    );
    let bytes = match args.format {
        Format::Svg => to_svg(&job, &result, &opts)?.into_bytes(),
        Format::Plt => to_hpgl(&job, &result, &opts)?.into_bytes(),
        Format::Pdf => to_pdf(&job, &result, &opts)?,
        Format::Report => to_report(&job, &result).into_bytes(),
        Format::Iso => {
            let profile = if let Some(path) = &args.profile {
                serde_json::from_reader(
                    File::open(path).with_context(|| format!("Cannot open {}", path.display()))?,
                )
                .context("Cannot read ISO profile JSON")?
            } else {
                IsoProfile::default()
            };
            to_iso(&job, &result, &profile)?.into_bytes()
        }
        Format::Dxf => to_dxf(&job, &result, &opts)?.into_bytes(),
        Format::Xlsx => to_xlsx(&job, &result, None)?,
        Format::Labels => to_labels_pdf(&job, &result, &LabelOptions::default())?,
        Format::Mini => to_mini_marker_pdf(&job, &result)?,
    };
    fs::write(&args.output, bytes)
        .with_context(|| format!("Cannot write {}", args.output.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct Command {
        #[command(flatten)]
        args: Args,
    }

    #[test]
    fn parses_all_drawing_controls() {
        let command = Command::try_parse_from([
            "export",
            "job.json",
            "result.json",
            "--format",
            "pdf",
            "--no-marks",
            "--no-labels",
            "--tile",
            "297",
            "--rotate",
            "-o",
            "marker.pdf",
        ])
        .expect("export arguments");
        assert!(matches!(command.args.format, Format::Pdf));
        assert!(command.args.no_marks && command.args.no_labels && command.args.rotate);
        assert_eq!(command.args.tile, Some(297.0));
        assert_eq!(command.args.output, PathBuf::from("marker.pdf"));
        assert!(
            Command::try_parse_from(["export", "j", "r", "--format", "unknown", "-o", "x"])
                .is_err()
        );
    }
}

#[cfg(test)]
mod v02_tests {
    use super::*;
    use clap::Parser;
    #[derive(Parser)]
    struct Command {
        #[command(flatten)]
        args: Args,
    }
    #[test]
    fn parses_new_formats_and_iso_profile() {
        for format in ["iso", "dxf", "xlsx", "labels", "mini"] {
            let command = Command::try_parse_from([
                "export",
                "job.json",
                "result.json",
                "--format",
                format,
                "-o",
                "output",
            ])
            .expect("format");
            assert!(command.args.profile.is_none());
        }
        let command = Command::try_parse_from([
            "export",
            "j",
            "r",
            "--format",
            "iso",
            "--profile",
            "cutter.json",
            "-o",
            "output",
        ])
        .expect("ISO profile");
        assert_eq!(command.args.profile, Some(PathBuf::from("cutter.json")));
    }
}
