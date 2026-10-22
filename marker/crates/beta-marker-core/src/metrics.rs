//! Marker length, piece area and efficiency, computed from the geometry.

use crate::geometry::{area, bbox};
use crate::rules::{
    cut_outline, flip_allowed, layer_copies, layer_hands_interchangeable, multiplier, placed_outline,
};
use crate::model::{Job, Layout, MarkerResult, Placement};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Metrics {
    /// Largest x of any placed outline point, mm (0 when nothing is placed).
    pub length: f64,
    /// Sum of placed outline areas, mm^2.
    pub piece_area: f64,
    /// `piece_area / (width * length)`, 0 when the length is 0.
    pub efficiency: f64,
    /// Area of full piece copies that serve the requested garments, summed over
    /// every fabric layer, mm^2. Surplus copies are excluded.
    pub useful_piece_area: f64,
    /// `useful_piece_area / (layers * width * length)`, with 1 layer for open
    /// fabric and 2 for folded or tubular fabric.
    pub useful_yield: f64,
    /// Full copies cut through the layers that no requested garment uses.
    pub surplus_copies: u64,
}

impl Metrics {
    /// Refresh all metrics together after editing, nesting or restoring identities.
    pub fn apply(self, result: &mut MarkerResult) {
        result.length = self.length;
        result.piece_area = self.piece_area;
        result.efficiency = self.efficiency;
        result.useful_piece_area = self.useful_piece_area;
        result.useful_yield = self.useful_yield;
        result.surplus_copies = self.surplus_copies;
    }
}

/// Compute metrics. Placements whose `piece_id` is not in the job are ignored here
/// (the validator reports them).
pub fn compute_metrics(job: &Job, placements: &[Placement]) -> Metrics {
    let by_id: HashMap<&str, &crate::model::Piece> = job.pieces.iter().map(|p| (p.id.as_str(), p)).collect();
    let mut length: f64 = 0.0;
    let mut piece_area = 0.0;
    // Ordered, so native and WASM sum the useful area in the same order.
    let mut copies = BTreeMap::<&str, (u64, u64, u64)>::new();
    for pl in placements {
        let Some(piece) = by_id.get(pl.piece_id.as_str()) else { continue };
        let pts = placed_outline(piece, pl);
        if let Some(b) = bbox(&cut_outline(piece, pl)) {
            length = length.max(b[2]);
        }
        piece_area += area(&pts);
        let entry = copies.entry(&piece.id).or_default();
        let (normal, mirrored) = layer_copies(job.fabric.layout, pl);
        entry.0 += u64::from(normal);
        entry.1 += u64::from(mirrored);
        entry.2 += if job.fabric.layout == Layout::Open || pl.on_fold { 1 } else { 2 };
    }
    let efficiency = if length > 0.0 && job.fabric.width > 0.0 { piece_area / (job.fabric.width * length) } else { 0.0 };
    let mut useful_piece_area = 0.;
    let mut surplus_copies = 0;
    for (id, (normal, mirrored, physical)) in copies {
        let piece = by_id[id];
        let m = u64::from(multiplier(job, piece));
        let demand = (u64::from(piece.quantity) * m, u64::from(piece.mirror_quantity) * m);
        // Explicit layer assignments already name the demanded hand. A legacy
        // two-layer copy of a symmetric piece can serve either hand.
        let unbalanced = (normal > demand.0 && mirrored < demand.1)
            || (mirrored > demand.1 && normal < demand.0);
        let pooled = flip_allowed(job, piece)
            || (unbalanced
                && job.fabric.layout != Layout::Open
                && layer_hands_interchangeable(job, piece));
        let useful = if pooled {
            (normal + mirrored).min(demand.0 + demand.1)
        } else {
            normal.min(demand.0) + mirrored.min(demand.1)
        }
        .min(physical);
        useful_piece_area += area(&piece.outline) * useful as f64;
        surplus_copies += physical - useful;
    }
    let layers = if job.fabric.layout == Layout::Open { 1. } else { 2. };
    let useful_yield = if length > 0. && job.fabric.width > 0. { useful_piece_area / (layers * job.fabric.width * length) } else { 0. };
    Metrics { length, piece_area, efficiency, useful_piece_area, useful_yield, surplus_copies }
}

/// Relative fabric length saving when efficiency rises from `old` to `new`
/// (same pieces, same width): `1 - old / new`.
pub fn length_saving(old_efficiency: f64, new_efficiency: f64) -> f64 {
    if new_efficiency <= 0.0 { 0.0 } else { 1.0 - old_efficiency / new_efficiency }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_two_layer_copy_of_a_symmetric_piece_serves_two_garments() {
        use crate::model::{Fabric, Order, Piece};
        let mut job = Job {
            fabric: Fabric { width: 100.0, layout: Layout::Folded, ..Default::default() },
            pieces: vec![Piece {
                id: "p".into(),
                quantity: 1,
                size: Some("M".into()),
                outline: vec![[0.0, 0.0], [20.0, 0.0], [20.0, 10.0], [0.0, 10.0]],
                ..Default::default()
            }],
            order: Some(Order { sizes: std::collections::BTreeMap::from([("M".into(), 2)]) }),
            ..Default::default()
        };
        let pl = Placement { piece_id: "p".into(), translate: [0.0, 5.0], ..Default::default() };
        let m = compute_metrics(&job, std::slice::from_ref(&pl));
        assert_eq!((m.surplus_copies, m.useful_piece_area), (0, 400.0));
        assert!((m.useful_yield - 400.0 / (2.0 * 100.0 * 20.0)).abs() < 1e-12);
        // An asymmetric piece: the mirrored layer serves no garment.
        job.pieces[0].outline[3] = [5.0, 10.0];
        let m = compute_metrics(&job, &[pl]);
        assert_eq!((m.surplus_copies, m.useful_piece_area), (1, 175.0));
    }

    #[test]
    fn saving_formula() {
        // 85% -> 87% is about 2.30% shorter marker.
        assert!((length_saving(0.85, 0.87) - 0.022988).abs() < 1e-5);
    }
}
