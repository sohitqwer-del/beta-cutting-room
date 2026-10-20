//! `beta-marker import` (owner: agent `dxf`).

use anyhow::{Context, Result, ensure};
use beta_marker_core::{Fabric, JOB_FORMAT, Job, Nap, Order, Spacing, check_job, geometry::area};
use beta_marker_dxf::{
    GradeTable, HpglOptions, ImportOptions, MaterialKeywords, Units, classify_materials,
    filter_material, import_dxf, import_dxf_with_rules, import_hpgl, split_materials,
};
use std::{collections::BTreeMap, path::PathBuf};

/// Import an ASCII pattern drawing as a validated marker job.
#[derive(clap::Args)]
pub struct Args {
    /// ASCII DXF patterns or a plain HPGL/PLT marker.
    file: PathBuf,
    /// Cuttable fabric width in millimetres.
    #[arg(long)]
    width: Option<f64>,
    // Keep optional import controls indirect so the lead-owned command enum
    // does not grow with every format option.
    #[command(flatten)]
    extra: Box<ExtraArgs>,
    /// Source units; all output is in millimetres.
    #[arg(long, default_value="auto", value_parser=["auto","mm","cm","inch"])]
    units: String,
    /// Fabric direction rule.
    #[arg(long, default_value="none", value_parser=["none","one_way"])]
    nap: String,
    /// Minimum piece gap, mm.
    #[arg(long, default_value_t = 0.0)]
    gap: f64,
    /// Minimum fabric edge gap, mm.
    #[arg(long, default_value_t = 0.0)]
    edge_gap: f64,
    /// Allowed grain tilt, degrees.
    #[arg(long, default_value_t = 0.0)]
    tilt: f64,
    /// Garments per size, e.g. S=1,M=2.
    #[arg(long)]
    ratio: Option<String>,
    /// Split every even Quantity N into N/2 normal and N/2 mirrored copies.
    #[arg(long)]
    pairs: bool,
    /// Job name (defaults to the input file stem).
    #[arg(long)]
    name: Option<String>,
    /// Destination job JSON.
    #[arg(short, long)]
    output: PathBuf,
}

/// Optional plot, grade-table and material controls.
#[derive(clap::Args)]
struct ExtraArgs {
    /// Input format (auto uses the file extension).
    #[arg(long, default_value="auto", value_parser=["auto","dxf","plt","hpgl"])]
    format: String,
    /// Plotter units per millimetre for HPGL.
    #[arg(long, default_value_t = 40.0)]
    plu_per_mm: f64,
    /// Write the original HPGL marker placements for independent validation.
    #[arg(long)]
    result_out: Option<PathBuf>,
    /// Experimental grade table for base-size DXF files.
    #[arg(long)]
    rules: Option<PathBuf>,
    /// Keep only this material (case-insensitive).
    #[arg(long)]
    material: Option<String>,
    /// Write an additional job per material into this directory.
    #[arg(long)]
    split_materials: Option<PathBuf>,
    /// JSON object mapping material names to arrays of name keywords.
    #[arg(long)]
    material_keywords: Option<PathBuf>,
}

/// Import, validate, write the job, and print its area and warning summary.
pub fn run(args: Args) -> Result<()> {
    let extra = &args.extra;
    ensure!(
        args.output != args.file,
        "job output must differ from input"
    );
    if let Some(path) = &extra.result_out {
        ensure!(
            *path != args.output && *path != args.file,
            "result output must differ from job output and input"
        );
    }
    if let Some(width) = args.width {
        ensure!(
            width.is_finite() && width > 0.0,
            "width must be finite and positive"
        );
    }
    for (label, n) in [
        ("gap", args.gap),
        ("edge-gap", args.edge_gap),
        ("tilt", args.tilt),
    ] {
        ensure!(
            n.is_finite() && n >= 0.0,
            "{label} must be finite and nonnegative"
        );
    }
    ensure!(args.tilt <= 180.0, "tilt must not exceed 180 degrees");
    let units = match args.units.as_str() {
        "mm" => Units::Mm,
        "cm" => Units::Cm,
        "inch" => Units::Inch,
        _ => Units::Auto,
    };
    let bytes =
        std::fs::read(&args.file).with_context(|| format!("read {}", args.file.display()))?;
    let format = if extra.format == "auto" {
        args.file
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_ascii_lowercase()
    } else {
        extra.format.clone()
    };
    let name = args.name.clone().unwrap_or_else(|| {
        args.file
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    });
    let opts = ImportOptions {
        units,
        pairs: args.pairs,
    };
    let (mut report, mut imported_result) = match format.as_str() {
        "plt" | "hpgl" => {
            ensure!(
                extra.rules.is_none() && args.ratio.is_none() && !args.pairs,
                "PLT counts come from plotted copies; --rules, --ratio and --pairs require DXF"
            );
            ensure!(
                units == Units::Auto,
                "use --plu-per-mm, rather than --units, for PLT"
            );
            let imported = import_hpgl(
                &bytes,
                &HpglOptions {
                    width: args.width,
                    plu_per_mm: extra.plu_per_mm,
                    name: name.clone(),
                    ..Default::default()
                },
            )?;
            let sizes = imported
                .job
                .pieces
                .iter()
                .filter_map(|p| p.size.clone())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();
            (
                beta_marker_dxf::ImportReport {
                    pieces: imported.job.pieces,
                    sizes,
                    dialect: "hpgl".into(),
                    units: Units::Mm,
                    warnings: imported.warnings,
                },
                Some(imported.result),
            )
        }
        "dxf" => {
            ensure!(
                args.width.is_some(),
                "DXF pattern import requires --width in millimetres"
            );
            ensure!(
                extra.result_out.is_none(),
                "--result-out requires a plotted PLT/HPGL marker"
            );
            let report = if let Some(path) = &extra.rules {
                let table = GradeTable::parse(
                    &std::fs::read(path).with_context(|| format!("read {}", path.display()))?,
                )?;
                import_dxf_with_rules(&bytes, &opts, &table)?
            } else {
                import_dxf(&bytes, &opts)?
            };
            (report, None)
        }
        _ => anyhow::bail!("unknown input format; pass --format dxf or --format plt"),
    };
    let custom: MaterialKeywords = if let Some(path) = &extra.material_keywords {
        serde_json::from_slice(
            &std::fs::read(path).with_context(|| format!("read {}", path.display()))?,
        )
        .context("material keyword JSON must map names to arrays of keywords")?
    } else {
        MaterialKeywords::new()
    };
    report
        .warnings
        .extend(classify_materials(&mut report.pieces, &custom));
    let order = args.ratio.as_deref().map(parse_ratio).transpose()?;
    if let Some(order) = &order {
        for size in order.sizes.keys() {
            ensure!(
                report.sizes.contains(size),
                "ratio size {size:?} does not occur in the drawing"
            );
        }
        // The core multiplies u32 counts, so reject overflow before check_job.
        for p in &report.pieces {
            let m = p
                .size
                .as_ref()
                .and_then(|s| order.sizes.get(s))
                .copied()
                .unwrap_or(0);
            ensure!(
                p.quantity.checked_mul(m).is_some() && p.mirror_quantity.checked_mul(m).is_some(),
                "quantity overflow for {}",
                p.id
            );
        }
    }
    let mut meta = BTreeMap::from([
        (
            "source_file".into(),
            serde_json::json!(args.file.file_name().unwrap_or_default().to_string_lossy()),
        ),
        ("dxf_dialect".into(), serde_json::json!(report.dialect)),
        ("source_units".into(), serde_json::to_value(report.units)?),
        ("import_warnings".into(), serde_json::json!(report.warnings)),
    ]);
    meta.insert("source_format".into(), serde_json::json!(report.dialect));
    if imported_result.is_some() {
        meta.insert("plu_per_mm".into(), serde_json::json!(extra.plu_per_mm));
    }
    if let Some(path) = &extra.rules {
        meta.insert(
            "grade_rules_file".into(),
            serde_json::json!(path.file_name().unwrap_or_default().to_string_lossy()),
        );
    }
    let mut job = Job {
        match_rules: Vec::new(),
        format: JOB_FORMAT.into(),
        name,
        fabric: Fabric {
            width: args
                .width
                .or_else(|| imported_result.as_ref().map(|r| r.width))
                .context("missing width")?,
            max_length: None,
            nap: if args.nap == "one_way" {
                Nap::OneWay
            } else {
                Nap::None
            },
            allow_flip: false,
            tilt_deg: args.tilt,
            ..Default::default()
        },
        spacing: Spacing {
            piece_gap: args.gap,
            edge_gap: args.edge_gap,
        },
        pieces: report.pieces,
        order,
        meta,
    };
    if let Some(material) = &extra.material {
        job = filter_material(&job, material);
        ensure!(
            !job.pieces.is_empty(),
            "no pieces match material {material:?}"
        );
    }
    if let Some(result) = &mut imported_result {
        result
            .placements
            .retain(|pl| job.pieces.iter().any(|p| p.id == pl.piece_id));
        let m = beta_marker_core::compute_metrics(&job, &result.placements);
        result.length = m.length;
        result.piece_area = m.piece_area;
        result.efficiency = m.efficiency;
        result.warnings = report.warnings.clone();
    }
    let issues = check_job(&job);
    ensure!(
        issues.is_empty(),
        "invalid imported job: {}",
        issues
            .iter()
            .map(|i| i.message.as_str())
            .collect::<Vec<_>>()
            .join("; ")
    );
    write_json(&args.output, &job)?;
    if let (Some(path), Some(result)) = (&extra.result_out, &imported_result) {
        write_json(path, result)?;
        println!(
            "Original marker: {:.3} mm, {:.2}% efficiency, {} plotted copies",
            result.length,
            result.efficiency * 100.0,
            result.placements.len()
        );
        println!("Wrote {}", path.display());
    }
    if let Some(directory) = &extra.split_materials {
        for (i, (material, part)) in split_materials(&job).into_iter().enumerate() {
            let slug: String = material
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
                .collect();
            let path = directory.join(format!(
                "{:02}-{}.json",
                i + 1,
                slug.chars().take(80).collect::<String>()
            ));
            write_json(&path, &part)?;
            println!(
                "Material {material:?}: {} pieces -> {}",
                part.pieces.len(),
                path.display()
            );
        }
    }
    println!(
        "Imported {} pieces, {} sizes ({}, {:?})",
        job.pieces.len(),
        job.pieces
            .iter()
            .filter_map(|p| p.size.as_deref())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        report.dialect,
        report.units
    );
    println!(
        "{:<16} {:>8} {:>10} {:>22}",
        "Size", "Pieces", "Copies", "Area / garment (mm^2)"
    );
    let mut rows: BTreeMap<&str, (usize, u64, f64)> = BTreeMap::new();
    for p in &job.pieces {
        let row = rows
            .entry(p.size.as_deref().unwrap_or("(unsized)"))
            .or_default();
        let copies = u64::from(p.quantity) + u64::from(p.mirror_quantity);
        row.0 += 1;
        row.1 += copies;
        row.2 += area(&p.outline) * copies as f64;
    }
    for (size, (pieces, copies, area)) in rows {
        println!("{size:<16} {pieces:>8} {copies:>10} {area:>22.2}");
    }
    for warning in &report.warnings {
        println!("Warning: {warning}");
    }
    println!("Wrote {}", args.output.display());
    Ok(())
}

fn write_json(path: &std::path::Path, value: &impl serde::Serialize) -> Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_vec_pretty(value)?)
        .with_context(|| format!("write {}", path.display()))
}

fn parse_ratio(text: &str) -> Result<Order> {
    let mut sizes = BTreeMap::new();
    for item in text.split(',') {
        let (size, count) = item
            .split_once('=')
            .context("ratio must be SIZE=N, for example S=1,M=2")?;
        let size = size.trim();
        ensure!(!size.is_empty(), "ratio size must not be empty");
        let count: u32 = count
            .trim()
            .parse()
            .context("ratio counts must be nonnegative integers")?;
        ensure!(
            sizes.insert(size.to_string(), count).is_none(),
            "duplicate ratio size {size}"
        );
    }
    ensure!(
        sizes.values().any(|&n| n > 0),
        "ratio must include at least one garment"
    );
    Ok(Order { sizes })
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct ImportParser {
        #[command(flatten)]
        args: Args,
    }

    #[test]
    fn parses_plot_and_experimental_pattern_options() {
        let plt = ImportParser::try_parse_from([
            "import",
            "marker.plt",
            "--format",
            "plt",
            "--plu-per-mm",
            "40",
            "--result-out",
            "their.json",
            "-o",
            "job.json",
        ])
        .expect("PLT args")
        .args;
        assert_eq!(plt.width, None);
        assert_eq!(plt.extra.result_out, Some("their.json".into()));
        let dxf = ImportParser::try_parse_from([
            "import",
            "base.dxf",
            "--width",
            "1500",
            "--rules",
            "sizes.rul",
            "--material",
            "lining",
            "--split-materials",
            "out",
            "--material-keywords",
            "words.json",
            "-o",
            "job.json",
        ])
        .expect("DXF args")
        .args;
        assert_eq!(dxf.width, Some(1500.0));
        assert_eq!(dxf.extra.rules, Some("sizes.rul".into()));
        assert_eq!(dxf.extra.material.as_deref(), Some("lining"));
    }

    #[test]
    fn plot_cli_writes_reusable_job_and_measured_original_result() {
        let target = std::env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target"));
        let out = target.join(format!("import-smoke-{}", std::process::id()));
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../testdata/dxf/comparison-marker.plt");
        let job_path = out.join("job.json");
        let result_path = out.join("their-marker.json");
        let args = ImportParser::try_parse_from([
            std::ffi::OsStr::new("import"),
            fixture.as_os_str(),
            std::ffi::OsStr::new("--format"),
            std::ffi::OsStr::new("plt"),
            std::ffi::OsStr::new("--width"),
            std::ffi::OsStr::new("150"),
            std::ffi::OsStr::new("-o"),
            job_path.as_os_str(),
            std::ffi::OsStr::new("--result-out"),
            result_path.as_os_str(),
        ])
        .expect("CLI arguments")
        .args;
        run(args).expect("CLI import");
        let job: Job =
            serde_json::from_slice(&std::fs::read(job_path).expect("job bytes")).expect("job");
        let result: beta_marker_core::MarkerResult =
            serde_json::from_slice(&std::fs::read(result_path).expect("result bytes"))
                .expect("result");
        assert_eq!(job.pieces.len(), 1);
        assert_eq!(result.engine, "imported");
        assert_eq!(result.length, 230.0);
        assert_eq!(result.piece_area, 9000.0);
        assert!(beta_marker_core::validate(&job, &result).ok);
    }
    #[test]
    fn ratio_validates_counts_and_duplicates() {
        assert_eq!(parse_ratio("S=1, M=2").expect("ratio").sizes["M"], 2);
        for bad in ["", "M", "M=-1", "M=0", "M=1,M=2", "=1"] {
            assert!(parse_ratio(bad).is_err(), "{bad}");
        }
    }
}
