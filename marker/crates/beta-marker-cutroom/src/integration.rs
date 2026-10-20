//! Marker and spreading adapters. Stock shade identity travels with each ply.
use crate::bundles::{CutPart, Lay, PlyRun};
use anyhow::{Context, Result, ensure};
use beta_marker_core::rules::{bundle_sizes, layer_copies};
use beta_marker_core::{Job, MarkerResult};
use beta_marker_roll::{LaySpread, RollRegister, ShadeIdentity};
use std::collections::{BTreeMap, BTreeSet};

/// Trace a validated marker through an audited spread into physical cut parts.
/// Folded/tubular placements expand through `layer_copies`; an on-fold half
/// becomes one full piece. Repeated parts get distinct slots. Garment IDs use
/// core bundle order. Mixed inventory shades within one ply are rejected.
/// Call `beta_marker_roll::validate_plan` on the full spread before this adapter.
pub fn from_spread(
    job: &Job,
    marker: &MarkerResult,
    register: &RollRegister,
    spread: &LaySpread,
    order: &str,
    colour: &str,
) -> Result<Lay> {
    let report = beta_marker_core::validate(job, marker);
    ensure!(report.ok, "Marker failed validation: {:?}", report.errors);
    ensure!(
        spread.allocated_plies == spread.lay.plies && spread.allocated_plies > 0,
        "Spread is incomplete."
    );
    ensure!(
        spread.lay.marker_length_m.is_finite()
            && spread.lay.marker_length_m * 1000.0 + 0.01 >= report.length,
        "Spread cloth is shorter than the validated marker."
    );
    register.validate()?;
    let sizes = bundle_sizes(job);
    let pieces: BTreeMap<_, _> = job.pieces.iter().map(|p| (p.id.as_str(), p)).collect();
    let mut slots = BTreeMap::new();
    let mut parts = Vec::new();
    for pl in &marker.placements {
        let p = pieces
            .get(pl.piece_id.as_str())
            .context("Unknown marker piece.")?;
        let garment = pl
            .bundle
            .context("Marker pieces need garment bundle IDs. Re-nest the marker.")?;
        let expected = sizes
            .get(garment as usize)
            .context("Marker bundle ID is out of range.")?;
        ensure!(
            job.order.is_none() || expected == &p.size,
            "Bundle size differs from canonical garment order."
        );
        let (a, b) = layer_copies(job.fabric.layout, pl);
        for (mirrored, count) in [(false, a), (true, b)] {
            for _ in 0..count {
                let slot = slots.entry((p.id.clone(), garment, mirrored)).or_insert(0);
                parts.push(CutPart {
                    piece_id: p.id.clone(),
                    size: p.size.clone().unwrap_or_else(|| "unsized".into()),
                    garment,
                    slot: *slot,
                    mirrored,
                });
                *slot += 1;
            }
        }
    }
    let mut plies: Vec<PlyRun> = Vec::new();
    for ply in 1..=spread.allocated_plies {
        let mut identities = BTreeSet::new();
        for seg in spread.segments.iter().filter(|s| s.ply == ply) {
            let r = register
                .rolls
                .iter()
                .find(|r| r.id == seg.roll_id)
                .context("Spread uses an unknown roll.")?;
            identities.insert(ShadeIdentity {
                lot: r.lot.clone(),
                dye_lot: r.dye_lot.clone(),
                shade_group: r.shade_group.clone(),
            });
        }
        ensure!(
            identities.len() == 1,
            "Ply {ply} needs exactly one inventory shade identity."
        );
        let identity = identities.into_iter().next().context("Ply has no cloth.")?;
        // Encode the full identity without ambiguous delimiter joins.
        let shade =
            serde_json::to_string(&[&identity.lot, &identity.dye_lot, &identity.shade_group])?;
        if let Some(last) = plies.last_mut().filter(|r| r.shade_group == shade) {
            last.last = ply;
        } else {
            plies.push(PlyRun {
                first: ply,
                last: ply,
                shade_group: shade,
            });
        }
    }
    Ok(Lay {
        id: spread.lay.id.clone(),
        style: job
            .pieces
            .iter()
            .find_map(|p| p.style.clone())
            .unwrap_or_else(|| job.name.clone()),
        order: order.into(),
        colour: colour.into(),
        plies,
        parts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundles::{BundleRules, bundle_lay};
    use beta_marker_core::*;
    use beta_marker_roll::{LayRequest, ShadePolicy};
    fn marker(job: &Job, placements: Vec<Placement>) -> MarkerResult {
        let m = compute_metrics(job, &placements);
        MarkerResult {
            format: RESULT_FORMAT.into(),
            job_name: job.name.clone(),
            width: job.fabric.width,
            length: m.length,
            piece_area: m.piece_area,
            efficiency: m.efficiency,
            engine: "manual".into(),
            seed: None,
            runtime_s: 0.0,
            warnings: Vec::new(),
            placements,
        }
    }
    #[test]
    fn folded_and_on_fold_parts_have_physical_counts_and_shared_numbers() -> Result<()> {
        for on_fold in [false, true] {
            let job = Job {
                format: JOB_FORMAT.into(),
                name: "TEE".into(),
                fabric: Fabric {
                    width: 20.0,
                    layout: Layout::Tubular,
                    ..Default::default()
                },
                pieces: vec![Piece {
                    id: "front".into(),
                    size: Some("M".into()),
                    quantity: 1,
                    mirror_quantity: if on_fold { 0 } else { 1 },
                    outline: vec![[0.0, -5.0], [10.0, -5.0], [10.0, 5.0], [0.0, 5.0]],
                    fold_line: Some([[0.0, 0.0], [10.0, 0.0]]),
                    ..Default::default()
                }],
                order: Some(Order {
                    sizes: BTreeMap::from([("M".into(), 1)]),
                }),
                ..Default::default()
            };
            let marker = marker(
                &job,
                vec![Placement {
                    piece_id: "front".into(),
                    bundle: Some(0),
                    on_fold,
                    translate: [0.0, if on_fold { 0.0 } else { 5.0 }],
                    ..Default::default()
                }],
            );
            let mut register = beta_marker_roll::demo_register();
            register.rolls.truncate(1);
            let requests = [LayRequest {
                id: "L1".into(),
                marker_length_m: marker.length / 1000.0,
                plies: 3,
                fabric_width_mm: 20.0,
                shade_rule: ShadePolicy::SameGroup,
                eligible_roll_ids: Vec::new(),
                allow_splicing: false,
                splice_zones: Vec::new(),
            }];
            let spread = beta_marker_roll::allocate(&register, &requests, &Default::default())?;
            beta_marker_roll::validate_plan(&register, &spread)?;
            let floor = from_spread(&job, &marker, &register, &spread.lays[0], "O1", "Blue")?;
            assert_eq!(floor.parts.len(), if on_fold { 1 } else { 2 });
            let bundles = bundle_lay(&floor, BundleRules::default())?;
            let count: usize = bundles.iter().map(|b| b.stickers.len()).sum();
            assert_eq!(count, if on_fold { 3 } else { 6 });
            assert!(bundles.iter().all(|b| b.stickers[0].number == 1));
        }
        Ok(())
    }
}
