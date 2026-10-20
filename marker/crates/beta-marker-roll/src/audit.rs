//! Independent checks on stock, shade, ply coverage and reported material totals.

use crate::{RollRegister, ShadePolicy, SpreadingPlan};
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

fn close(a: f64, b: f64) -> bool {
    a.is_finite() && b.is_finite() && (a - b).abs() <= 1e-7 * (1.0 + a.abs().max(b.abs()))
}

/// Check a deserialised plan independently of the allocation algorithm.
/// A valid partial plan still needs `complete == true` before production.
pub fn validate_plan(register: &RollRegister, plan: &SpreadingPlan) -> Result<()> {
    register.validate()?;
    let mut intervals: BTreeMap<&str, Vec<(f64, f64)>> = BTreeMap::new();
    let mut live_cursor = BTreeMap::<&str, f64>::new();
    let mut end_rows = BTreeMap::<&str, f64>::new();
    let mut total_overlap = 0.0;
    let mut prev: Option<&str> = None;
    let mut changes = 0;
    let mut lay_ids = BTreeSet::new();
    for l in &plan.lays {
        ensure!(lay_ids.insert(&l.lay.id), "duplicate lay in plan");
        crate::model::positive(l.lay.marker_length_m, "marker length")?;
        crate::model::positive(l.lay.fabric_width_mm, "marker width")?;
        ensure!(
            l.lay.plies > 0 && l.allocated_plies <= l.lay.plies,
            "allocated ply count is invalid"
        );
        let mut shades = BTreeSet::new();
        let mut by_ply = BTreeMap::<u32, Vec<&crate::SpreadSegment>>::new();
        let mut rows = BTreeMap::<&str, Vec<&crate::SpreadSegment>>::new();
        for seg in &l.segments {
            let r = register
                .rolls
                .iter()
                .find(|r| r.id == seg.roll_id)
                .ok_or_else(|| anyhow::anyhow!("unknown roll {}", seg.roll_id))?;
            crate::model::nonnegative(seg.roll_start_m, "roll start")?;
            crate::model::nonnegative(seg.marker_start_m, "marker start")?;
            crate::model::positive(seg.length_m, "segment length")?;
            ensure!(
                seg.ply > 0 && seg.ply <= l.allocated_plies,
                "segment has invalid ply"
            );
            ensure!(
                seg.roll_start_m + seg.length_m <= r.measured_length_m + 1e-7,
                "segment exceeds roll stock"
            );
            let cursor = live_cursor.entry(&seg.roll_id).or_default();
            ensure!(
                close(*cursor, seg.roll_start_m),
                "roll segments are not in physical execution order"
            );
            *cursor += seg.length_m;
            ensure!(
                r.usable_width_mm + 1e-8 >= l.lay.fabric_width_mm,
                "roll is narrower than marker"
            );
            ensure!(
                l.lay.eligible_roll_ids.is_empty() || l.lay.eligible_roll_ids.contains(&r.id),
                "roll is outside lay stock restriction"
            );
            let shade = (&r.lot, &r.dye_lot, &r.shade_group);
            if let ShadePolicy::Required { identity } = &l.lay.shade_rule {
                ensure!(
                    shade == (&identity.lot, &identity.dye_lot, &identity.shade_group),
                    "roll shade does not match required identity"
                );
            }
            shades.insert(shade);
            by_ply.entry(seg.ply).or_default().push(seg);
            rows.entry(&seg.roll_id).or_default().push(seg);
            intervals
                .entry(&seg.roll_id)
                .or_default()
                .push((seg.roll_start_m, seg.roll_start_m + seg.length_m));
            if prev.is_some_and(|id| id != seg.roll_id) {
                changes += 1;
            }
            prev = Some(&seg.roll_id);
        }
        ensure!(
            l.lay.shade_rule == ShadePolicy::AllowMixing || shades.len() <= 1,
            "lay mixes shade identities"
        );
        ensure!(
            by_ply.len() == l.allocated_plies as usize,
            "allocated ply is missing segments"
        );
        let mut expected_splices = 0;
        for (ply, mut segments) in by_ply {
            segments.sort_by(|a, b| a.marker_start_m.total_cmp(&b.marker_start_m));
            ensure!(
                close(segments[0].marker_start_m, 0.0),
                "ply does not start at marker zero"
            );
            let last = segments[segments.len() - 1];
            ensure!(
                close(last.marker_start_m + last.length_m, l.lay.marker_length_m),
                "ply does not cover marker end"
            );
            for pair in segments.windows(2) {
                let joins: Vec<_> = l
                    .splices
                    .iter()
                    .filter(|s| {
                        s.ply == ply
                            && s.from_roll == pair[0].roll_id
                            && s.to_roll == pair[1].roll_id
                            && close(s.marker_position_m, pair[1].marker_start_m)
                    })
                    .collect();
                ensure!(joins.len() == 1, "splice instruction missing or duplicated");
                let s = joins[0];
                crate::model::nonnegative(s.overlap_m, "splice overlap")?;
                ensure!(
                    close(
                        pair[0].marker_start_m + pair[0].length_m,
                        s.marker_position_m + s.overlap_m
                    ),
                    "splice overlap differs from coverage"
                );
                ensure!(
                    l.lay.allow_splicing
                        && l.lay
                            .splice_zones
                            .iter()
                            .any(|z| s.marker_position_m >= z.start_m - 1e-8
                                && s.marker_position_m + s.overlap_m <= z.end_m + 1e-8),
                    "splice falls outside an allowed zone"
                );
                ensure!(
                    pair[0].marker_start_m < pair[1].marker_start_m,
                    "splice makes no progress"
                );
                total_overlap += s.overlap_m;
                expected_splices += 1;
            }
        }
        ensure!(
            expected_splices == l.splices.len(),
            "unexpected splice instruction"
        );
        ensure!(
            rows.len() == l.allocations.len(),
            "allocation rows do not match spread rolls"
        );
        let mut row_ids = BTreeSet::new();
        for row in &l.allocations {
            ensure!(row_ids.insert(&row.roll_id), "duplicate allocation row");
            let segs = rows
                .get(row.roll_id.as_str())
                .ok_or_else(|| anyhow::anyhow!("allocation row has no segments"))?;
            let r = register
                .rolls
                .iter()
                .find(|r| r.id == row.roll_id)
                .ok_or_else(|| anyhow::anyhow!("allocation row has unknown roll"))?;
            ensure!(
                row.shade_group == r.shade_group,
                "allocation row shade is incorrect"
            );
            ensure!(
                row.plies as usize == segs.iter().map(|s| s.ply).collect::<BTreeSet<_>>().len(),
                "allocation row ply count is incorrect"
            );
            ensure!(
                row.full_plies as usize
                    == segs
                        .iter()
                        .filter(|s| close(s.marker_start_m, 0.0)
                            && close(s.length_m, l.lay.marker_length_m))
                        .count(),
                "allocation row full-ply count is incorrect"
            );
            ensure!(
                close(row.used_m, segs.iter().map(|s| s.length_m).sum()),
                "allocation row used length is incorrect"
            );
            crate::model::nonnegative(row.end_bit_m, "end bit")?;
            *end_rows.entry(&row.roll_id).or_default() += row.end_bit_m;
        }
    }
    ensure!(
        plan.rolls.len() == register.rolls.len(),
        "roll balance count differs from register"
    );
    let mut seen = BTreeSet::new();
    let mut used = 0.0;
    let mut end = 0.0;
    let mut unopened = 0.0;
    for balance in &plan.rolls {
        ensure!(seen.insert(&balance.roll_id), "duplicate roll balance");
        let r = register
            .rolls
            .iter()
            .find(|r| r.id == balance.roll_id)
            .ok_or_else(|| anyhow::anyhow!("unknown balance roll"))?;
        let mut cuts = intervals
            .remove(balance.roll_id.as_str())
            .unwrap_or_default();
        cuts.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut cursor = 0.0;
        for (a, b) in &cuts {
            ensure!(close(*a, cursor), "roll stock is skipped or reused");
            cursor = *b;
        }
        ensure!(
            balance.opened != cuts.is_empty(),
            "opened flag is incorrect"
        );
        ensure!(
            close(cursor, balance.used_m)
                && close(balance.used_m + balance.remaining_m, r.measured_length_m),
            "roll material balance is incorrect"
        );
        crate::model::nonnegative(balance.remaining_m, "remaining stock")?;
        ensure!(
            close(
                *end_rows.get(balance.roll_id.as_str()).unwrap_or(&0.0),
                if balance.opened {
                    balance.remaining_m
                } else {
                    0.0
                }
            ),
            "table end bits do not match balance"
        );
        used += cursor;
        if balance.opened {
            end += balance.remaining_m;
        } else {
            unopened += balance.remaining_m;
        }
    }
    let demand: f64 = plan
        .lays
        .iter()
        .map(|l| f64::from(l.allocated_plies) * l.lay.marker_length_m)
        .sum();
    ensure!(
        close(used, demand + total_overlap),
        "spread material does not equal demand plus overlap"
    );
    ensure!(
        close(plan.total_used_m, used)
            && close(plan.total_end_bits_m, end)
            && close(plan.splice_overlap_waste_m, total_overlap)
            && close(plan.total_waste_m, end + total_overlap)
            && close(plan.unopened_stock_m, unopened),
        "plan totals are incorrect"
    );
    ensure!(
        plan.roll_changes == changes,
        "roll change count is incorrect"
    );
    ensure!(
        plan.complete == plan.lays.iter().all(|l| l.allocated_plies == l.lay.plies),
        "complete flag is incorrect"
    );
    Ok(())
}
