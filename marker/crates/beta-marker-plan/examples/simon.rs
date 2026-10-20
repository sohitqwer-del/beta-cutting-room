//! Generate MIT FreeSewing Simon cut orders without duplicating the pattern dataset.
use anyhow::{Context, Result};
use beta_marker_core::Job;
use beta_marker_plan::{CUT_ORDER_FORMAT, CutOrder};
use std::path::PathBuf;

fn main() -> Result<()> {
    let destination = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .context("usage: cargo run -p beta-marker-plan --example simon -- <output folder>")?;
    std::fs::create_dir_all(&destination)?;
    let job: Job = serde_json::from_str(include_str!(
        "../../../testdata/open/freesewing/simon-ratio.job.json"
    ))?;
    for (name, quantities, overcut_pct) in [
        ("simon-1200", [200, 400, 400, 200], 0.0),
        ("simon-awkward", [37, 113, 96, 41], 5.0),
    ] {
        let order = CutOrder {
            format: CUT_ORDER_FORMAT.into(),
            name: name.into(),
            job: job.clone(),
            quantities: ["S", "M", "L", "XL"]
                .into_iter()
                .zip(quantities)
                .map(|(s, q)| (s.into(), q))
                .collect(),
            max_plies: 80,
            max_garments_per_marker: 6,
            overcut_pct,
        };
        let path = destination.join(format!("{name}.cutorder.json"));
        std::fs::write(&path, serde_json::to_vec_pretty(&order)?)?;
        println!("{}", path.display());
    }
    Ok(())
}
