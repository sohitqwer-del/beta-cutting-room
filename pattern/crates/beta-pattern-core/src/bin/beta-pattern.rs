//! Offline BETA Pattern command line interface.
use anyhow::{Context, Result, bail, ensure};
use beta_marker_core::Fabric;
use beta_pattern_core::{Document, evaluate, exchange, geometry, measurements, print, seamly};
use std::{collections::BTreeMap, fs, path::Path, process::ExitCode};

const USAGE: &str = "beta-pattern grade <doc.json> -o <directory>\nbeta-pattern export-dxf <doc.json> --sizes S,M,L -o <file.dxf>\nbeta-pattern to-job <doc.json> --width 1500 --ratio S=1,M=2 -o <job.json>\nbeta-pattern import-seamly <file.sm2d> --measurements <file.smis|file.smms> [--copy-rules <rules.json>] -o <doc.json>\nbeta-pattern export-pdf <doc.json> --sizes S,M --paper A4 --overlap 10 -o <tiles.pdf>\nbeta-pattern export-plt <doc.json> --sizes S,M -o <pattern.plt>\nbeta-pattern import-measurements <file.smis> --size M -o <measurements.json>\nbeta-pattern export-measurements <doc.json> -o <measurements.csv>\nGeometry commands accept --tolerance <mm> (default 0.1).";
fn read(path: &str) -> Result<String> {
    let meta = fs::metadata(path).with_context(|| format!("read {path}"))?;
    ensure!(meta.len() <= 64 * 1024 * 1024, "input exceeds 64 MiB");
    fs::read_to_string(path).with_context(|| format!("read {path}"))
}
fn json(path: impl AsRef<Path>, v: &impl serde::Serialize) -> Result<()> {
    fs::write(path.as_ref(), serde_json::to_string_pretty(v)?)
        .with_context(|| format!("write {}", path.as_ref().display()))
}
fn required<'a>(options: &'a BTreeMap<String, String>, key: &str) -> Result<&'a str> {
    options
        .get(key)
        .map(String::as_str)
        .with_context(|| format!("missing {key}\n{USAGE}"))
}
fn safe_name(s: &str) -> String {
    s.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b == b'_' || b == b'-' {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}
fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
        println!("{USAGE}");
        return Ok(());
    }
    // Command-level help: show the usage line for that command without reading files.
    if args[1..].iter().any(|a| a == "--help" || a == "-h") {
        let line = USAGE
            .lines()
            .find(|l| l.starts_with(&format!("beta-pattern {} ", args[0])))
            .with_context(|| format!("unknown command '{}'\n{USAGE}", args[0]))?;
        println!("{line}\nGeometry commands accept --tolerance <mm> (default 0.1).");
        return Ok(());
    }
    ensure!(args.len() >= 2, "missing input file\n{USAGE}");
    let command = args[0].as_str();
    let allowed = match command {
        "grade" => vec!["-o", "--tolerance"],
        "export-dxf" | "export-plt" => vec!["-o", "--sizes", "--tolerance"],
        "export-pdf" => vec!["-o", "--sizes", "--paper", "--overlap", "--tolerance"],
        "import-measurements" => vec!["-o", "--size"],
        "export-measurements" => vec!["-o"],
        "to-job" => vec!["-o", "--width", "--ratio", "--tolerance"],
        "import-seamly" => vec!["-o", "--measurements", "--copy-rules"],
        _ => bail!("unknown command '{command}'\n{USAGE}"),
    };
    let mut options = BTreeMap::new();
    for pair in args[2..].chunks(2) {
        ensure!(pair.len() == 2, "missing value for {}", pair[0]);
        ensure!(
            allowed.contains(&pair[0].as_str()),
            "unknown option {}",
            pair[0]
        );
        ensure!(
            options.insert(pair[0].clone(), pair[1].clone()).is_none(),
            "duplicate option {}",
            pair[0]
        );
    }
    let out = required(&options, "-o")?;
    if command == "import-seamly" {
        let pattern = read(&args[1])?;
        let ms = options.get("--measurements").map(|p| read(p)).transpose()?;
        let import_options = seamly::ImportOptions {
            copy_rules: options
                .get("--copy-rules")
                .map(|path| -> Result<_> {
                    serde_json::from_str(&read(path)?).context("parse copy-rule mapping JSON")
                })
                .transpose()?
                .unwrap_or_default(),
        };
        let report = if let Some(ms) = ms
            .as_deref()
            .filter(|s| s.contains("<smms") || s.contains("<vst"))
        {
            seamly::import_multisize_with_options(&pattern, ms, &import_options)?
        } else {
            seamly::import_with_options(&pattern, ms.as_deref(), "M", &import_options)?
        };
        for warning in report.warnings {
            eprintln!("warning: {warning}");
        }
        return json(out, &report.document);
    }
    if command == "import-measurements" {
        let text = read(&args[1])?;
        let set = if args[1].to_ascii_lowercase().ends_with(".csv") {
            measurements::import_csv(&text)?
        } else {
            measurements::import_seamly(
                &text,
                options.get("--size").map(String::as_str).unwrap_or("M"),
            )?
        };
        return json(out, &set);
    }
    let doc: Document =
        serde_json::from_str(&read(&args[1])?).context("parse BETA document JSON")?;
    if let Some(warnings) = doc
        .meta
        .get("seamly_import_warnings")
        .and_then(|v| v.as_array())
    {
        for warning in warnings.iter().filter_map(|v| v.as_str()) {
            eprintln!("warning: {warning}");
        }
    }
    let tolerance = options
        .get("--tolerance")
        .map(|s| s.parse::<f64>())
        .transpose()
        .context("tolerance must be numeric millimetres")?
        .unwrap_or(0.1);
    ensure!(
        tolerance.is_finite() && tolerance > 0.,
        "tolerance must be positive finite millimetres"
    );
    match command {
        "grade" => {
            // Evaluate all sizes before creating any output.
            let all = doc
                .sizes
                .iter()
                .map(|s| Ok((s.clone(), evaluate::grade(&doc, s, tolerance)?)))
                .collect::<Result<Vec<_>>>()?;
            fs::create_dir_all(out)?;
            let mut summary = vec![];
            for (size, pieces) in all {
                json(
                    Path::new(out).join(format!("{}.pieces.json", safe_name(&size))),
                    &pieces,
                )?;
                for p in pieces {
                    summary.push(serde_json::json!({"size":size,"piece":p.piece.id,"stitch_area_mm2":geometry::area(&p.seam),"cut_area_mm2":geometry::area(&p.piece.outline),"stitch_bbox_mm":geometry::bbox(&p.seam),"cut_bbox_mm":geometry::bbox(&p.piece.outline)}));
                }
            }
            json(Path::new(out).join("summary.json"), &summary)
        }
        "export-dxf" => {
            let sizes = options
                .get("--sizes")
                .map(|s| s.split(',').map(str::to_owned).collect())
                .unwrap_or_else(|| doc.sizes.clone());
            fs::write(out, exchange::export_dxf(&doc, &sizes, tolerance)?)?;
            Ok(())
        }
        "export-pdf" | "export-plt" => {
            let sizes = options
                .get("--sizes")
                .map(|s| s.split(',').map(str::to_owned).collect())
                .unwrap_or_else(|| doc.sizes.clone());
            if command == "export-pdf" {
                let paper = match options
                    .get("--paper")
                    .map(|s| s.to_ascii_uppercase())
                    .as_deref()
                    .unwrap_or("A4")
                {
                    "A4" => print::Paper::A4,
                    "A3" => print::Paper::A3,
                    _ => bail!("paper must be A4 or A3"),
                };
                let overlap_mm = options
                    .get("--overlap")
                    .map(|s| s.parse::<f64>())
                    .transpose()
                    .context("overlap must be millimetres")?
                    .unwrap_or(10.);
                fs::write(
                    out,
                    print::tiled_pdf(
                        &doc,
                        &sizes,
                        &print::PrintOptions {
                            paper,
                            overlap_mm,
                            tolerance_mm: tolerance,
                            ..Default::default()
                        },
                    )?,
                )?;
            } else {
                fs::write(out, print::plotter_plt(&doc, &sizes, tolerance)?)?;
            }
            Ok(())
        }
        "export-measurements" => {
            let set = measurements::MeasurementSet {
                sizes: doc.sizes.clone(),
                base_size: doc.base_size.clone(),
                values: doc.measurements.clone(),
                heights: Default::default(),
            };
            fs::write(out, measurements::export_csv(&set)?)?;
            Ok(())
        }
        "to-job" => {
            let width = required(&options, "--width")?
                .parse::<f64>()
                .context("width must be numeric millimetres")?;
            let mut ratio = BTreeMap::new();
            for entry in required(&options, "--ratio")?.split(',') {
                let (size, count) = entry
                    .split_once('=')
                    .context("ratio entries must be SIZE=count")?;
                ensure!(
                    !size.is_empty()
                        && ratio
                            .insert(
                                size.into(),
                                count
                                    .parse::<u32>()
                                    .context("ratio count must be a nonnegative integer")?
                            )
                            .is_none(),
                    "empty or duplicate ratio size"
                );
            }
            let job = exchange::to_job(
                &doc,
                Fabric {
                    width,
                    ..Default::default()
                },
                ratio,
                tolerance,
            )?;
            json(out, &job)
        }
        _ => bail!("unhandled command"),
    }
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("beta-pattern: {e:#}");
            ExitCode::FAILURE
        }
    }
}
