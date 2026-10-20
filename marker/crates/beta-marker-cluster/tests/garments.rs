//! Public API checks against the attributed MIT FreeSewing fixtures.
use anyhow::{Context, Result};
use beta_marker_cluster::{Strategy, nest_clustered};
use beta_marker_core::Job;
use beta_marker_nest::NestOptions;
use std::{fs::File, path::Path};

#[test]
fn freesewing_short_budget_results_are_valid() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testdata/open/freesewing");
    for style in ["simon", "teagan", "titan"] {
        let job: Job =
            serde_json::from_reader(File::open(root.join(format!("{style}-M.job.json")))?)?;
        let options = NestOptions {
            time_limit_s: 0.15,
            workers: Some(2),
            seed: Some(1),
            ..Default::default()
        };
        let result =
            nest_clustered(&job, &options, Strategy::Cluster).with_context(|| style.to_string())?;
        let validation = beta_marker_core::validate(&job, &result);
        assert!(validation.ok, "{style}: {:?}", validation.errors);
    }
    Ok(())
}
