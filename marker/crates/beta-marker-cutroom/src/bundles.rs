//! Ply numbering and sewing-floor bundles. A marker garment is not a floor bundle.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// One physical piece cut at every ply. Expand folded placements before using this input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CutPart {
    /// Source job piece id.
    pub piece_id: String,
    /// Garment size.
    pub size: String,
    /// Marker garment number; shared by all parts of that garment.
    pub garment: u32,
    /// Distinguishes repeated parts of the same garment (for example two pockets).
    pub slot: u32,
    /// Handedness of this physical piece.
    pub mirrored: bool,
}

/// Contiguous, inclusive ply interval from one shade group. Ply numbers start at 1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlyRun {
    /// First ply, inclusive.
    pub first: u32,
    /// Last ply, inclusive.
    pub last: u32,
    /// Inventory shade/lot identity, not a geometric shade band.
    pub shade_group: String,
}

/// Small input contract. Planner and spreading adapters can supply this later.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lay {
    /// Unique lay id within the order.
    pub id: String,
    /// Style identifier.
    pub style: String,
    /// Order identifier.
    pub order: String,
    /// Fabric colour.
    pub colour: String,
    /// Shade runs must cover 1 through the final ply without gaps or overlap.
    pub plies: Vec<PlyRun>,
    /// Physical pieces per ply.
    pub parts: Vec<CutPart>,
}

/// Bundles always keep one size, part, handedness, shade and contiguous ply interval.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BundleRules {
    /// Maximum physical pieces per bundle. Final bundles can be smaller.
    pub pieces_per_bundle: u32,
    /// First floor bundle number. The caller reserves ranges between lays.
    pub first_bundle: u32,
    /// Numbers already issued for each size in earlier lays of this order.
    pub sticker_offset: u32,
}
impl Default for BundleRules {
    fn default() -> Self {
        Self {
            pieces_per_bundle: 20,
            first_bundle: 1,
            sticker_offset: 0,
        }
    }
}

/// Stable physical-piece identity within a lay; never use the printed number alone.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PieceKey {
    /// Source piece id.
    pub piece_id: String,
    /// Physical ply, starting at 1.
    pub ply: u32,
    /// Marker garment number.
    pub garment: u32,
    /// Repeated part index.
    pub slot: u32,
    /// Physical handedness.
    pub mirrored: bool,
}

/// One adhesive numbering sticker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sticker {
    /// Durable piece reference.
    pub piece: PieceKey,
    /// Same number on all parts of a garment from the same ply.
    pub number: u32,
    /// Final number issued for this size by this lay (including the offset).
    pub last_number: u32,
}
impl Sticker {
    /// Human label such as `1-60`: garment number, then final number in this lay/size run.
    pub fn label(&self) -> String {
        format!("{}-{}", self.number, self.last_number)
    }
}

/// One floor bundle, with explicit physical pieces for traceability and re-cuts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bundle {
    /// Lay identifier.
    pub lay: String,
    /// Style identifier.
    pub style: String,
    /// Order identifier.
    pub order: String,
    /// Fabric colour.
    pub colour: String,
    /// Size shared by all pieces.
    pub size: String,
    /// Part shared by all pieces.
    pub piece_id: String,
    /// Inventory shade group.
    pub shade_group: String,
    /// Sequential floor bundle number.
    pub number: u32,
    /// Inclusive ply range.
    pub plies: [u32; 2],
    /// Actual pieces; length is the quantity.
    pub stickers: Vec<Sticker>,
}

/// Make deterministic bundles. Reject duplicate physical parts and invalid shade coverage.
/// A safety limit of one million physical pieces avoids accidental unbounded allocations.
pub fn bundle_lay(lay: &Lay, rules: BundleRules) -> Result<Vec<Bundle>> {
    ensure!(
        !lay.id.trim().is_empty()
            && !lay.style.trim().is_empty()
            && !lay.order.trim().is_empty()
            && !lay.colour.trim().is_empty(),
        "lay, style, order and colour are required"
    );
    ensure!(
        rules.pieces_per_bundle > 0 && rules.first_bundle > 0,
        "bundle size and first number must be positive"
    );
    ensure!(
        !lay.plies.is_empty() && !lay.parts.is_empty(),
        "lay needs plies and parts"
    );
    let mut next = 1u64;
    for run in &lay.plies {
        ensure!(
            u64::from(run.first) == next
                && run.last >= run.first
                && !run.shade_group.trim().is_empty(),
            "shade runs must cover consecutive plies from 1"
        );
        next = u64::from(run.last) + 1;
    }
    let total_plies = next - 1;
    ensure!(
        total_plies * lay.parts.len() as u64 <= 1_000_000,
        "lay exceeds one million physical pieces"
    );
    let mut seen = BTreeSet::new();
    let mut garments: BTreeMap<&str, BTreeSet<u32>> = BTreeMap::new();
    let mut groups: BTreeMap<(&str, &str, bool), Vec<&CutPart>> = BTreeMap::new();
    for p in &lay.parts {
        ensure!(
            !p.size.trim().is_empty() && !p.piece_id.trim().is_empty(),
            "part id and size are required"
        );
        ensure!(
            seen.insert((&p.piece_id, &p.size, p.garment, p.slot, p.mirrored)),
            "duplicate physical part"
        );
        garments.entry(&p.size).or_default().insert(p.garment);
        groups
            .entry((&p.size, &p.piece_id, p.mirrored))
            .or_default()
            .push(p);
    }
    // A garment id must identify exactly one size within the marker.
    let mut sizes = BTreeMap::new();
    for p in &lay.parts {
        if let Some(previous) = sizes.insert(p.garment, &p.size) {
            ensure!(
                previous == &p.size,
                "marker garment id belongs to more than one size"
            );
        }
    }
    // This input has physical parts rather than a Job. Project its distinct
    // sizes into the same core bundle-order helper used by marker garments.
    let size_job = beta_marker_core::Job {
        order: Some(beta_marker_core::Order {
            sizes: garments.keys().map(|s| (s.to_string(), 1)).collect(),
        }),
        ..Default::default()
    };
    let ordered_sizes: BTreeMap<_, _> = beta_marker_core::rules::bundle_sizes(&size_job)
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(i, s)| (s, i))
        .collect();
    let mut groups: Vec<_> = groups.into_iter().collect();
    groups.sort_by_key(|((size, piece, mirrored), _)| (ordered_sizes[*size], *piece, *mirrored));
    let mut out = Vec::new();
    for ((size, piece_id, _), mut parts) in groups {
        parts.sort_by_key(|p| (p.garment, p.slot));
        let ids: Vec<_> = garments[size].iter().copied().collect();
        let count = ids.len() as u64;
        let last = u64::from(rules.sticker_offset) + total_plies * count;
        ensure!(last <= u64::from(u32::MAX), "sticker numbering overflow");
        for run in &lay.plies {
            let mut stickers = Vec::new();
            for ply in run.first..=run.last {
                for p in &parts {
                    let rank = ids
                        .binary_search(&p.garment)
                        .map_err(|_| anyhow::anyhow!("missing garment"))?
                        as u64;
                    let number =
                        u64::from(rules.sticker_offset) + u64::from(ply - 1) * count + rank + 1;
                    stickers.push(Sticker {
                        piece: PieceKey {
                            piece_id: piece_id.into(),
                            ply,
                            garment: p.garment,
                            slot: p.slot,
                            mirrored: p.mirrored,
                        },
                        number: number as u32,
                        last_number: last as u32,
                    });
                }
            }
            for chunk in stickers.chunks(rules.pieces_per_bundle as usize) {
                let n = u64::from(rules.first_bundle) + out.len() as u64;
                ensure!(n <= u64::from(u32::MAX), "bundle numbering overflow");
                out.push(Bundle {
                    lay: lay.id.clone(),
                    style: lay.style.clone(),
                    order: lay.order.clone(),
                    colour: lay.colour.clone(),
                    size: size.into(),
                    piece_id: piece_id.into(),
                    shade_group: run.shade_group.clone(),
                    number: n as u32,
                    plies: [chunk[0].piece.ply, chunk[chunk.len() - 1].piece.ply],
                    stickers: chunk.to_vec(),
                });
            }
        }
    }
    Ok(out)
}
