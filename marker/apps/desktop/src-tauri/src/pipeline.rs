//! Whole-ply order -> inspected group -> plan -> spread -> bundles pipeline.
use anyhow::{Context, ensure};
use serde::Deserialize;
use serde_json::{Value, json};
#[derive(Deserialize)]
struct Input {
    order: beta_marker_plan::CutOrder,
    options: beta_marker_plan::PlanOptions,
    register: beta_marker_roll::RollRegister,
    #[serde(default)]
    group_index: usize,
    #[serde(default)]
    colour: String,
    #[serde(default)]
    bundle_rules: beta_marker_cutroom::bundles::BundleRules,
}
pub(crate) fn run(body: &[u8]) -> anyhow::Result<Value> {
    let mut input: Input = serde_json::from_slice(body)?;
    input.register.validate()?;
    let inspection = beta_marker_roll::inspect(&input.register, &Default::default())?;
    let groups = beta_marker_roll::group_rolls(&input.register, &Default::default())?;
    let group = groups
        .get(input.group_index)
        .context("Choose an available roll group")?;
    let mut eligible = beta_marker_plan::apply_roll_group(&mut input.order.job, group);
    eligible.retain(|id| {
        inspection
            .rolls
            .iter()
            .any(|roll| &roll.roll_id == id && roll.score.passed)
    });
    ensure!(
        !eligible.is_empty(),
        "Selected group has no rolls passing inspection"
    );
    let plan = beta_marker_plan::plan(&input.order, &input.options)?;
    ensure!(plan.complete, "Cut order has a shortfall");
    let requests =
        beta_marker_plan::spreading_requests(&plan, input.order.job.fabric.width, &eligible)?;
    let spread = beta_marker_roll::allocate(&input.register, &requests, &Default::default())?;
    beta_marker_roll::validate_plan(&input.register, &spread)?;
    ensure!(spread.complete, "Selected roll stock cannot fill all lays");
    let mut bundles = Vec::new();
    let mut markers = Vec::new();
    for lay in &spread.lays {
        let index = requests
            .iter()
            .position(|r| r.id == lay.lay.id)
            .context("Unknown spread lay")?;
        let mut job = input.order.job.clone();
        job.order = Some(beta_marker_core::Order {
            sizes: plan.lays[index].ratio.clone(),
        });
        let result = plan.lays[index]
            .result
            .as_ref()
            .context("Lay has no checked marker")?;
        let map = beta_marker_roll::map_defects(&input.register, lay, &Default::default())?;
        ensure!(
            !map.requires_replanning,
            "Lay needs defect replacement or replanning"
        );
        for mut zone in map.zones {
            for point in &mut zone.polygon {
                point[0] -= input.options.end_allowance_mm;
            }
            job.fabric.defects.push(zone);
        }
        let validation = beta_marker_core::validate(&job, result);
        ensure!(validation.ok, "Allocated roll defects conflict with marker");
        let physical = beta_marker_cutroom::integration::from_spread(
            &job,
            result,
            &input.register,
            lay,
            &input.order.name,
            &input.colour,
        )?;
        let rules = beta_marker_cutroom::bundles::BundleRules {
            first_bundle: u32::try_from(bundles.len())?
                .checked_add(1)
                .context("Bundle count overflow")?,
            ..input.bundle_rules
        };
        bundles.extend(beta_marker_cutroom::bundles::bundle_lay(&physical, rules)?);
        markers.push(json!({"id":lay.lay.id,"job":job,"result":result,"lay":physical,"validation":validation}));
    }
    Ok(
        json!({"order":input.order,"inspection":inspection,"plan":plan,"spread":spread,"bundles":bundles,"markers":markers}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_pipeline_is_rejected_before_planning() {
        assert!(run(b"{}").is_err());
    }
    #[test]
    fn original_fixture_pipeline_keeps_checked_markers_and_shade_identity() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../../packages/marker-wasm/test/fixtures/cutting-room.json"
        ))
        .unwrap();
        let body = json!({"order":{"format":"beta-marker.cutorder/v1","name":"Desktop pipeline test","job":fixture["job"],"quantities":{"M":2},"max_plies":2,"max_garments_per_marker":1,"overcut_pct":0},"options":{"time_per_marker_s":0.05,"max_candidates":2,"max_refinements":1,"search_nodes":100},"register":fixture["register"],"colour":"Blue"});
        let output = run(body.to_string().as_bytes()).expect("small pipeline");
        assert_eq!(output["plan"]["complete"], true);
        assert_eq!(output["spread"]["complete"], true);
        assert!(
            output["markers"]
                .as_array()
                .unwrap()
                .iter()
                .all(|m| m["validation"]["ok"] == true)
        );
        assert!(output["bundles"].as_array().unwrap().iter().all(|b| {
            b["shade_group"]
                .as_str()
                .is_some_and(|s| s.contains("LOT1"))
        }));
    }
}
