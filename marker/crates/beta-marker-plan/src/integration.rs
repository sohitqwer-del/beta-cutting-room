//! Unit-safe adapters between the cutting plan and measured cloth stock.
use crate::CutPlan;
use anyhow::{Result, ensure};
use beta_marker_core::Job;
use beta_marker_roll::{LayRequest, RollGroup, ShadePolicy};

/// Apply a shade/shrinkage group's mean test result before planning or nesting.
/// The returned stock restriction must also be used during spreading.
pub fn apply_roll_group(job: &mut Job, group: &RollGroup) -> Vec<String> {
    job.fabric.shrinkage = group.average_shrinkage;
    group.roll_ids.clone()
}

/// Convert chosen lays into whole-ply spread requests. Length includes both
/// end allowances, converted from millimetres to metres. IDs are stable even
/// when the allocator changes execution order. Splicing is disabled.
/// `eligible_roll_ids` must describe the stock used to set job shrinkage.
pub fn spreading_requests(
    plan: &CutPlan,
    width_mm: f64,
    eligible_roll_ids: &[String],
) -> Result<Vec<LayRequest>> {
    ensure!(plan.complete, "Cannot spread an incomplete cutting plan.");
    ensure!(
        width_mm.is_finite() && width_mm > 0.0,
        "Width must be finite and positive."
    );
    ensure!(
        plan.options.end_allowance_mm.is_finite() && plan.options.end_allowance_mm >= 0.0,
        "End allowance must be finite and nonnegative."
    );
    plan.lays
        .iter()
        .enumerate()
        .map(|(i, lay)| {
            ensure!(
                lay.plies > 0 && lay.marker_length_mm.is_finite() && lay.marker_length_mm > 0.0,
                "Lay {} has invalid plies or length.",
                i + 1
            );
            Ok(LayRequest {
                id: format!("L{:03}", i + 1),
                marker_length_m: (lay.marker_length_mm + 2.0 * plan.options.end_allowance_mm)
                    / 1000.0,
                fabric_width_mm: width_mm,
                plies: lay.plies,
                shade_rule: ShadePolicy::SameGroup,
                eligible_roll_ids: eligible_roll_ids.to_vec(),
                allow_splicing: false,
                splice_zones: Vec::new(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spread_units_allowances_and_stock_restrictions() {
        let mut plan = CutPlan {
            complete: true,
            ..Default::default()
        };
        plan.options.end_allowance_mm = 25.0;
        plan.lays.push(crate::Lay {
            plies: 7,
            marker_length_mm: 2000.0,
            ..Default::default()
        });
        let lays = spreading_requests(&plan, 1500.0, &["R1".into()]).expect("adapter");
        assert_eq!(lays[0].marker_length_m, 2.05);
        assert_eq!(lays[0].plies, 7);
        assert_eq!(lays[0].eligible_roll_ids, ["R1"]);
        plan.complete = false;
        assert!(spreading_requests(&plan, 1500.0, &[]).is_err());
    }
}

#[cfg(test)]
mod stock_tests {
    use super::*;
    #[test]
    fn group_shrinkage_keeps_the_stock_restriction() {
        let register = beta_marker_roll::demo_register();
        let groups = beta_marker_roll::group_rolls(&register, &Default::default()).expect("groups");
        let mut job = Job::default();
        let ids = apply_roll_group(&mut job, &groups[0]);
        assert_eq!(job.fabric.shrinkage, groups[0].average_shrinkage);
        assert_eq!(ids, groups[0].roll_ids);
    }
}
