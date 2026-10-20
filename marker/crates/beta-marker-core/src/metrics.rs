//! Marker length, piece area and efficiency, computed from the geometry.

use crate::geometry::{area, bbox};
use crate::rules::{cut_outline, placed_outline};
use crate::model::{Job, Placement};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Metrics {
    /// Largest x of any placed outline point, mm (0 when nothing is placed).
    pub length: f64,
    /// Sum of placed outline areas, mm^2.
    pub piece_area: f64,
    /// `piece_area / (width * length)`, 0 when the length is 0.
    pub efficiency: f64,
}

/// Compute metrics. Placements whose `piece_id` is not in the job are ignored here
/// (the validator reports them).
pub fn compute_metrics(job: &Job, placements: &[Placement]) -> Metrics {
    let by_id: HashMap<&str, &crate::model::Piece> = job.pieces.iter().map(|p| (p.id.as_str(), p)).collect();
    let mut length: f64 = 0.0;
    let mut piece_area = 0.0;
    for pl in placements {
        let Some(piece) = by_id.get(pl.piece_id.as_str()) else { continue };
        let pts = placed_outline(piece, pl);
        if let Some(b) = bbox(&cut_outline(piece, pl)) {
            length = length.max(b[2]);
        }
        piece_area += area(&pts);
    }
    let efficiency = if length > 0.0 && job.fabric.width > 0.0 { piece_area / (job.fabric.width * length) } else { 0.0 };
    Metrics { length, piece_area, efficiency }
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
    fn saving_formula() {
        // 85% -> 87% is about 2.30% shorter marker.
        assert!((length_saving(0.85, 0.87) - 0.022988).abs() < 1e-5);
    }
}
