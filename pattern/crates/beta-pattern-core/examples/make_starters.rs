//! Regenerate the small native starter documents from the repository's open fixtures.
use anyhow::{Context, Result};
use beta_marker_core::Job;
use beta_pattern_core::{evaluate, exchange, geometry, starters};
use std::{collections::BTreeMap, fs, path::Path};
fn main() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = root.join("starters");
    fs::create_dir_all(&out)?;
    let bodice = starters::bodice();
    fs::write(
        out.join("beta-bodice.doc.json"),
        serde_json::to_string_pretty(&bodice)?,
    )?;
    let mut report = vec![];
    for s in &bodice.sizes {
        for p in evaluate::grade(&bodice, s, 0.1)? {
            let b = geometry::bbox(&p.seam).context("empty seam")?;
            report.push(serde_json::json!({"size":s,"piece":p.piece.id,"width_mm":b[2]-b[0],"height_mm":b[3]-b[1],"stitch_area_mm2":geometry::area(&p.seam),"cut_area_mm2":geometry::area(&p.piece.outline)}));
        }
    }
    let dxf = exchange::export_dxf(&bodice, &bodice.sizes, 0.1)?;
    let imported = beta_marker_dxf::import_dxf(dxf.as_bytes(), &Default::default())?;
    let job = exchange::to_job(
        &bodice,
        beta_marker_core::Fabric {
            width: 1500.,
            ..Default::default()
        },
        BTreeMap::from([
            ("S".into(), 1),
            ("M".into(), 2),
            ("L".into(), 2),
            ("XL".into(), 1),
        ]),
        0.1,
    )?;
    fs::write(
        out.join("verification.json"),
        serde_json::to_string_pretty(
            &serde_json::json!({"bodice":report,"dxf_roundtrip_piece_count":imported.pieces.len(),"dxf_warnings":imported.warnings,"marker_issues":beta_marker_core::check_job(&job)}),
        )?,
    )?;
    for name in ["teagan", "titan"] {
        let input = root.join(format!(
            "../../../marker/testdata/open/freesewing/{name}-M.job.json"
        ));
        let job: Job = serde_json::from_str(
            &fs::read_to_string(&input).with_context(|| input.display().to_string())?,
        )?;
        let mut doc = starters::from_marker_job(&job)?;
        doc.meta.insert("attribution".into(),serde_json::json!("FreeSewing 4.10.2, copyright Joost De Cock and FreeSewing contributors, MIT. See LICENSE.FreeSewing.MIT."));
        fs::write(
            out.join(format!("freesewing-{name}-M.doc.json")),
            serde_json::to_string_pretty(&doc)?,
        )?;
    }
    Ok(())
}
