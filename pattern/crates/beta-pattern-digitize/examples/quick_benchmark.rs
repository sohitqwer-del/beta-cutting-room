use anyhow::Result;
use beta_marker_core::Job;
use beta_pattern_digitize::{
    quick::{self, Options, Reference},
    quick_synthetic,
    synthetic::{self, PhotoOptions},
    target::{SheetSize, Target},
};
use std::{path::Path, time::Instant};
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let input = args.get(1).expect("FreeSewing job");
    let out = Path::new(args.get(2).expect("output directory"));
    std::fs::create_dir_all(out)?;
    let job: Job = serde_json::from_slice(&std::fs::read(input)?)?;
    let target = Target::tiled(SheetSize::A3, 1, 1, false)?;
    let mut rows = Vec::new();
    let quick = args.iter().any(|a| a == "--quick");
    let dimensions = if quick { [1600, 1200] } else { [4000, 3000] };
    for (index, source) in job.pieces.iter().take(3).enumerate() {
        for (name, options, brown) in [
            (
                "frontal",
                PhotoOptions {
                    dimensions,
                    perspective: false,
                    blur_px: 0.5,
                    lighting: 0.08,
                    noise: 1.,
                    k1: 0.,
                    ..Default::default()
                },
                false,
            ),
            (
                "perspective-shadow-texture",
                PhotoOptions {
                    dimensions,
                    blur_px: 1.2,
                    lighting: 0.40,
                    noise: 5.,
                    k1: 0.,
                    ..Default::default()
                },
                false,
            ),
            (
                "blur-brown",
                PhotoOptions {
                    dimensions,
                    blur_px: 3.,
                    lighting: 0.30,
                    noise: 4.,
                    k1: 0.,
                    ..Default::default()
                },
                true,
            ),
            (
                "radial-lens",
                PhotoOptions {
                    dimensions,
                    blur_px: 0.9,
                    lighting: 0.28,
                    noise: 4.,
                    k1: 0.035,
                    ..Default::default()
                },
                false,
            ),
        ] {
            let photo = quick_synthetic::photograph(source, &options, brown)?;
            let stem = format!("a4-{index}-{name}");
            let path = out.join(format!("{stem}.jpg"));
            photo.image.save(&path)?;
            let bytes = std::fs::read(&path)?;
            let start = Instant::now();
            match quick::scan_bytes(&bytes, &Options::default()) {
                Ok(scan) => {
                    let elapsed = start.elapsed().as_secs_f64();
                    std::fs::write(out.join(format!("{stem}-overlay.png")), &scan.overlay_png)?;
                    let recovered = scan
                        .pieces
                        .iter()
                        .max_by(|a, b| {
                            beta_marker_core::geometry::signed_area(&a.piece.outline)
                                .abs()
                                .total_cmp(
                                    &beta_marker_core::geometry::signed_area(&b.piece.outline)
                                        .abs(),
                                )
                        })
                        .unwrap();
                    let score = synthetic::measure(&photo.truth, &recovered.piece)?;
                    println!(
                        "A4 {} {name}: mean {:.3} max {:.3} mm, {:.3} s",
                        source.name, score.mean_mm, score.max_mm, elapsed
                    );
                    rows.push(serde_json::json!({"mode":"a4-sheet","piece":source.name,"case":name,"score":score,"seconds":elapsed,"source_scale":photo.source_scale,"pieces":scan.pieces.len(),"estimated_error_mm":scan.scale.expected_error_mm,"warnings":scan.warnings}));
                }
                Err(e) => {
                    println!("A4 {} {name}: FAILED {e}", source.name);
                    rows.push(serde_json::json!({"mode":"a4-sheet","piece":source.name,"case":name,"error":e.to_string()}));
                }
            }
        }
        for (name, perspective) in [("frontal", false), ("phone", true)] {
            let options = PhotoOptions {
                dimensions,
                perspective,
                k1: if perspective { 0.035 } else { 0. },
                ..Default::default()
            };
            let photo = synthetic::photograph(&target, source, &options)?;
            let stem = format!("target-{index}-{name}");
            let path = out.join(format!("{stem}.jpg"));
            photo.image.save(&path)?;
            let bytes = std::fs::read(&path)?;
            let start = Instant::now();
            let scan = quick::scan_bytes(
                &bytes,
                &Options {
                    reference: Reference::BetaTarget,
                    ..Default::default()
                },
            )?;
            let elapsed = start.elapsed().as_secs_f64();
            std::fs::write(out.join(format!("{stem}-overlay.png")), &scan.overlay_png)?;
            let score = synthetic::measure(&photo.truth, &scan.pieces[0].piece)?;
            println!(
                "Target {} {name}: mean {:.3} max {:.3} mm, {:.3} s",
                source.name, score.mean_mm, score.max_mm, elapsed
            );
            rows.push(serde_json::json!({"mode":"beta-target","piece":source.name,"case":name,"score":score,"seconds":elapsed,"source_scale":photo.source_scale,"warnings":scan.warnings}));
        }
    }
    std::fs::write(
        out.join("accuracy.json"),
        serde_json::to_string_pretty(&rows)?,
    )?;
    Ok(())
}
