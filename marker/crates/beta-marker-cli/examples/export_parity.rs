//! Native reference files for the WASM byte-parity harness. Synthetic data only.
use anyhow::Result;
use beta_marker_core::{Job, MarkerResult};
use beta_marker_export as output;
use serde_json::json;
use std::{fs, path::PathBuf};

fn main() -> Result<()> {
    let dir = PathBuf::from(std::env::args().nth(1).expect("output directory"));
    fs::create_dir_all(&dir)?;
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../packages/marker-wasm/test/fixtures/contract.json"
    ))?;
    let mut cases = vec![];
    for variant in 0..3 {
        let mut job: Job = serde_json::from_value(fixture["job"].clone())?;
        let mut result: MarkerResult = serde_json::from_value(fixture["result"].clone())?;
        // A complete one-piece synthetic job, independent of the contract's ratio.
        job.order = None;
        job.pieces[0].quantity = 1;
        job.pieces[0].mirror_quantity = 0;
        job.fabric.width = 600.;
        if variant > 0 {
            job.pieces[0].outline = vec![
                [0., 0.],
                [400., 0.],
                [400., 60.],
                [100., 60.],
                [100., 250.],
                [0., 250.],
            ];
            job.pieces[0].rotation = beta_marker_core::RotationRule::Free;
            job.pieces[0].grain = None;
            job.fabric.shrinkage.length_pct = 2.;
            job.fabric.shrinkage.width_pct = 1.;
            result.placements[0].rotation_deg = 37.;
            result.placements[0].translate = [180., 10.];
        }
        if variant == 2 {
            job.pieces[0].quantity = 0;
            job.pieces[0].mirror_quantity = 1;
            result.placements[0].mirrored = true;
            result.placements[0].translate = [600., 280.];
            job.meta.insert("beta_stripe_guides".into(), json!(true));
            job.fabric.repeat = Some(serde_json::from_value(
                json!({"period_x":25,"offset_x":0,"offset_y":0,"tolerance_mm":1}),
            )?);
        }
        let metrics = beta_marker_core::compute_metrics(
            &beta_marker_core::rules::effective_job(&job),
            &result.placements,
        );
        result.width = job.fabric.width;
        result.length = metrics.length;
        result.piece_area = metrics.piece_area;
        result.efficiency = metrics.efficiency;
        metrics.apply(&mut result);
        anyhow::ensure!(
            beta_marker_core::validate(&job, &result).ok,
            "invalid synthetic parity layout {variant}"
        );
        for format in ["svg", "plt", "hpgl", "pdf", "dxf", "iso"] {
            for (n, options) in [
                json!({}),
                json!({"labels":false,"marks":false,"rotate":true,"pdf_tile_mm":210}),
            ]
            .into_iter()
            .enumerate()
            {
                let file = format!("marker-{variant}-{format}-{n}.bin");
                fs::write(
                    dir.join(&file),
                    output::export(
                        format,
                        &job,
                        &result,
                        &serde_json::from_value(options.clone())?,
                    )?
                    .0,
                )?;
                cases.push(json!({"kind":"marker","file":file,"job":job,"result":result,"format":format,"options":options}));
            }
        }
        for paper in ["A4", "A3"] {
            let file = format!("marker-{variant}-{paper}.pdf");
            let options = json!({"paper":paper,"overlap_mm":10});
            fs::write(
                dir.join(&file),
                output::tiled_marker_pdf(&job, &result, &serde_json::from_value(options.clone())?)?,
            )?;
            cases.push(
                json!({"kind":"tiles","file":file,"job":job,"result":result,"options":options}),
            );
        }
    }
    let document = beta_pattern_core::starters::bodice();
    for format in ["dxf", "plt", "A4", "A3"] {
        let file = format!("pattern-{format}.bin");
        let bytes = match format {
            "dxf" => beta_pattern_core::exchange::export_dxf(&document, &document.sizes, 0.1)?
                .into_bytes(),
            "plt" => {
                beta_pattern_core::print::plotter_plt(&document, &document.sizes, 0.1)?.into_bytes()
            }
            paper => beta_pattern_core::print::tiled_pdf(
                &document,
                &document.sizes,
                &beta_pattern_core::print::PrintOptions {
                    paper: if paper == "A4" {
                        beta_pattern_core::print::Paper::A4
                    } else {
                        beta_pattern_core::print::Paper::A3
                    },
                    ..Default::default()
                },
            )?,
        };
        fs::write(dir.join(&file), bytes)?;
        cases.push(json!({"kind":"pattern","file":file,"document":document,"format":format}));
    }
    fs::write(dir.join("cases.json"), serde_json::to_vec(&cases)?)?;
    let mut imports = vec![];
    for (name, bytes) in [
        (
            "shirt-aama-mm",
            include_bytes!("../../../testdata/dxf/shirt-aama-mm.dxf").as_slice(),
        ),
        (
            "tee-astm-mm",
            include_bytes!("../../../testdata/dxf/tee-astm-mm.dxf").as_slice(),
        ),
        (
            "trouser-aama-inch",
            include_bytes!("../../../testdata/dxf/trouser-aama-inch.dxf").as_slice(),
        ),
    ] {
        let report = beta_marker_dxf::import_dxf(bytes, &Default::default())?;
        imports.push(json!({"name":name,"source":std::str::from_utf8(bytes)?,"report":report}));
    }
    for (name, options, rules) in [
        (
            "grade-base-mm-pairs",
            json!({"units":"mm","pairs":true}),
            None,
        ),
        (
            "grade-cumulative",
            json!({}),
            Some(include_str!("../../../testdata/dxf/grade-cumulative.rul")),
        ),
        (
            "grade-incremental",
            json!({}),
            Some(include_str!("../../../testdata/dxf/grade-incremental.rul")),
        ),
    ] {
        let source = include_str!("../../../testdata/dxf/grade-base-mm.dxf");
        let opts = serde_json::from_value(options.clone())?;
        let report = if let Some(rules) = rules {
            beta_marker_dxf::import_dxf_with_rules(
                source.as_bytes(),
                &opts,
                &beta_marker_dxf::GradeTable::parse(rules.as_bytes())?,
            )?
        } else {
            beta_marker_dxf::import_dxf(source.as_bytes(), &opts)?
        };
        imports.push(
            json!({"name":name,"source":source,"options":options,"rules":rules,"report":report}),
        );
    }
    fs::write(dir.join("imports.json"), serde_json::to_vec(&imports)?)?;
    println!("{} native reference files", cases.len());
    Ok(())
}
