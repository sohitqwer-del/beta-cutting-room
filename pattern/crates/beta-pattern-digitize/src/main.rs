//! Command-line entry point for offline target generation and photo digitizing.
use anyhow::{Context, Result, bail, ensure};
use beta_marker_core::Job;
use beta_pattern_digitize::{
    ScanOptions, scan_file,
    synthetic::{self, PhotoOptions},
    target::{SheetSize, Target},
};
use std::{collections::BTreeMap, path::PathBuf};

const HELP: &str =
    "beta-digitize target --size a3|a2 [--tiles 2x2] [--table] -o target.pdf|target.svg
beta-digitize scan photo.jpg [--target target.target.json] [--size a3|a2]
    [--radial] [--mm-per-pixel 0.5] [--tolerance 0.25] [--min-area 100]
    [--grain x1,y1,x2,y2] [--threshold 170] [--no-smooth]
    [--debug overlay.png] [--rectified metric.png] [--report scan.json]
    [--job --width 1500] -o pieces.json
beta-digitize benchmark --input source.job.json --output-dir data-directory [--quick]

Print at 100% / actual size. Ten checker cells must measure 50 mm in both axes.
Keep a 36 mm empty border. All grain input coordinates are board mm, y DOWN.
Default scan target is one landscape A3 sheet. Use its manifest for other layouts.";

fn main() {
    if let Err(e) = run() {
        eprintln!("beta-digitize: {e:#}");
        std::process::exit(1);
    }
}
fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let Some(command) = args.next() else {
        println!("{HELP}");
        return Ok(());
    };
    if command == "--help" || command == "-h" {
        println!("{HELP}");
        return Ok(());
    }
    let mut options = BTreeMap::new();
    let mut positional = Vec::new();
    let args: Vec<_> = args.collect();
    let mut i = 0;
    while i < args.len() {
        let key = &args[i];
        if key.starts_with('-') {
            let flag = matches!(
                key.as_str(),
                "--job" | "--radial" | "--table" | "--quick" | "--no-smooth"
            );
            let value = if flag {
                "true".to_string()
            } else {
                i += 1;
                args.get(i).context("missing option value")?.clone()
            };
            ensure!(
                options.insert(key.clone(), value).is_none(),
                "duplicate option {key}"
            );
        } else {
            positional.push(key.clone());
        }
        i += 1;
    }
    let mut take = |key: &str| options.remove(key);
    let out = take("-o").map(PathBuf::from);
    let size = match take("--size").as_deref().unwrap_or("a3") {
        "a3" => SheetSize::A3,
        "a2" => SheetSize::A2,
        _ => bail!("size must be a3 or a2"),
    };
    match command.as_str() {
        "target" => {
            ensure!(
                positional.is_empty(),
                "target takes no positional arguments"
            );
            let layout = take("--tiles").unwrap_or_else(|| "1x1".into());
            let (c, r) = layout.split_once('x').context("tiles must be COLSxROWS")?;
            let target = Target::tiled(size, c.parse()?, r.parse()?, take("--table").is_some())?;
            ensure!(options.is_empty(), "unknown options: {:?}", options.keys());
            let output = out.context("supply -o target.pdf or target.svg")?;
            target.save(&output)?;
            eprintln!(
                "Target: {} x {} mm, {} codes. Saved target and manifest.",
                target.width_mm,
                target.height_mm,
                target.markers.len()
            );
        }
        "scan" => {
            ensure!(positional.len() == 1, "scan needs one input photo");
            let target = if let Some(path) = take("--target") {
                let bytes = std::fs::read(path)?;
                ensure!(bytes.len() <= 1_000_000, "target manifest too large");
                serde_json::from_slice(&bytes)?
            } else {
                Target::tiled(size, 1, 1, false)?
            };
            let parse = |text: Option<String>, default: f64| -> Result<f64> {
                text.map(|s| s.parse().map_err(Into::into))
                    .unwrap_or(Ok(default))
            };
            let opts = ScanOptions {
                mm_per_pixel: parse(take("--mm-per-pixel"), 0.5)?,
                simplify_mm: parse(take("--tolerance"), 0.25)?,
                min_area_mm2: parse(take("--min-area"), 100.)?,
                fit_radial: take("--radial").is_some(),
                smooth: take("--no-smooth").is_none(),
                threshold: take("--threshold").map(|s| s.parse()).transpose()?,
                grain: take("--grain")
                    .map(|s| -> Result<_> {
                        let p: Vec<f64> = s
                            .split(',')
                            .map(str::parse)
                            .collect::<std::result::Result<_, _>>()?;
                        ensure!(p.len() == 4, "grain needs x1,y1,x2,y2");
                        Ok([[p[0], p[1]], [p[2], p[3]]])
                    })
                    .transpose()?,
            };
            let debug = take("--debug");
            let metric = take("--rectified");
            let report = take("--report");
            let job = take("--job").is_some();
            let width = parse(take("--width"), 1500.)?;
            ensure!(options.is_empty(), "unknown options: {:?}", options.keys());
            let output = out.context("supply -o pieces.json")?;
            let result = scan_file(&PathBuf::from(&positional[0]), &target, &opts)?;
            let json = if job {
                serde_json::to_vec_pretty(&result.job(width)?)?
            } else {
                serde_json::to_vec_pretty(&result.pieces)?
            };
            std::fs::write(output, json)?;
            if let Some(path) = debug {
                result.overlay.save(path)?;
            }
            if let Some(path) = metric {
                result.rectified.save(path)?;
            }
            if let Some(path) = report {
                std::fs::write(path, serde_json::to_vec_pretty(&result.report)?)?;
            }
            eprintln!(
                "{} pieces; {} markers; reprojection RMS {:.2} px; {:.3} s",
                result.pieces.len(),
                result.report.detected_markers,
                result.report.calibration.rms_px,
                result.report.runtime_s
            );
            for warning in &result.report.warnings {
                eprintln!("Warning: {warning}");
            }
        }
        "benchmark" => {
            ensure!(
                positional.is_empty() && out.is_none(),
                "benchmark uses --input and --output-dir"
            );
            let input = take("--input").context("supply --input source.job.json")?;
            let directory =
                PathBuf::from(take("--output-dir").context("supply --output-dir (outside git)")?);
            let quick = take("--quick").is_some();
            ensure!(options.is_empty(), "unknown options: {:?}", options.keys());
            std::fs::create_dir_all(&directory)?;
            let job: Job = serde_json::from_slice(&std::fs::read(input)?)?;
            let target = Target::tiled(size, 1, 1, false)?;
            target.save(&directory.join("target.pdf"))?;
            let mut rows = Vec::new();
            let mut passed = true;
            for source in job.pieces.iter().take(3) {
                for degraded in [false, true] {
                    let label = if degraded { "phone" } else { "scan" };
                    let cfg = if degraded {
                        PhotoOptions::default()
                    } else {
                        PhotoOptions {
                            perspective: false,
                            blur_px: 0.4,
                            noise: 1.,
                            lighting: 0.08,
                            k1: 0.,
                            ..Default::default()
                        }
                    };
                    let cfg = if quick {
                        PhotoOptions {
                            dimensions: [1800, 1350],
                            ..cfg
                        }
                    } else {
                        cfg
                    };
                    let photo = synthetic::photograph(&target, source, &cfg)?;
                    let stem = format!("{}-{label}", source.id);
                    let path = directory.join(format!("{stem}.jpg"));
                    photo.image.save(&path)?;
                    let result = scan_file(
                        &path,
                        &target,
                        &ScanOptions {
                            fit_radial: degraded,
                            ..Default::default()
                        },
                    )?;
                    ensure!(
                        result.pieces.len() == 1,
                        "benchmark expected one piece, found {}",
                        result.pieces.len()
                    );
                    let score = synthetic::measure(&photo.truth, &result.pieces[0])?;
                    result
                        .overlay
                        .save(directory.join(format!("{stem}-overlay.png")))?;
                    std::fs::write(
                        directory.join(format!("{stem}-truth.json")),
                        serde_json::to_vec_pretty(&photo.truth)?,
                    )?;
                    std::fs::write(
                        directory.join(format!("{stem}-pieces.json")),
                        serde_json::to_vec_pretty(&result.pieces)?,
                    )?;
                    eprintln!(
                        "{stem}: mean {:.3}, max {:.3} mm, area {:.3}%, notches {}/{}, FP {}, runtime {:.3}s",
                        score.mean_mm,
                        score.max_mm,
                        score.area_error_pct,
                        score.notches_found,
                        score.notches_expected,
                        score.notch_false_positives,
                        result.report.runtime_s
                    );
                    passed &= score.mean_mm < 1. && score.max_mm < 2.;
                    rows.push(serde_json::json!({"piece":source.id,"case":label,"dimensions":cfg.dimensions,"source_scale":photo.source_scale,"accuracy":score,"scan":result.report}));
                }
            }
            ensure!(!rows.is_empty(), "source job contains no pieces");
            std::fs::write(
                directory.join("accuracy.json"),
                serde_json::to_vec_pretty(&rows)?,
            )?;
            ensure!(
                passed,
                "one or more synthetic outlines missed the 1 mm mean / 2 mm max goal"
            );
        }
        _ => bail!("unknown command {command}\n{HELP}"),
    }
    Ok(())
}
