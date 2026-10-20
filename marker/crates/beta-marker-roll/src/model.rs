//! JSON contracts and validation for a measured roll register.

use anyhow::{Result, ensure};
use beta_marker_core::Shrinkage;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// One flaw. The position is the lower-left corner of its measured rectangle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RollDefect {
    /// Unique within this roll.
    pub id: String,
    /// Distance from the measured roll start, in metres.
    pub along_m: f64,
    /// Distance from the usable left edge, in millimetres.
    pub across_mm: f64,
    /// Extent along the roll, in millimetres.
    pub length_mm: f64,
    /// Extent across the roll, in millimetres.
    pub width_mm: f64,
    /// Inspector-assigned penalty, from one to four.
    pub points: u8,
    /// Plain-text classification, such as hole, stain or slub.
    pub defect_type: String,
    /// How to handle this flaw in a spread.
    #[serde(default)]
    pub action: DefectAction,
}

/// A flaw can stay in the cloth, become a marker exclusion or be removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DefectAction {
    /// Score it during inspection; no cutting instruction is needed.
    Ignore,
    /// Exclude its footprint in a low-ply marker; remove it in a high-ply lay.
    #[default]
    CutAround,
    /// Always flag a cut-out and replacement instruction.
    CutOut,
}

/// One roll. All lengths refer to the usable cloth, with selvedges excluded.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Roll {
    /// Stock identity, unique across the register.
    pub id: String,
    /// Receiving or supplier lot identity.
    pub lot: String,
    /// Dye batch identity within the lot.
    pub dye_lot: String,
    /// Inspector-assigned colour group within the dye batch.
    pub shade_group: String,
    /// Supplier's stated usable length, in metres.
    pub ticket_length_m: f64,
    /// Inspected usable length, in metres; allocation uses this value.
    pub measured_length_m: f64,
    /// Cuttable width after removing selvedges, in millimetres.
    pub usable_width_mm: f64,
    /// Percent in the core convention: positive values enlarge marker pieces.
    pub shrinkage: Shrinkage,
    /// Recorded flaws, including those ignored for cutting but scored for inspection.
    #[serde(default)]
    pub defects: Vec<RollDefect>,
}

/// Measured stock, in receiving order (used by the first-come baseline).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RollRegister {
    /// Rolls in receiving order.
    pub rolls: Vec<Roll>,
}

pub(crate) fn positive(value: f64, name: &str) -> Result<()> {
    ensure!(
        value.is_finite() && value > 0.0,
        "{name} must be finite and positive"
    );
    Ok(())
}

pub(crate) fn nonnegative(value: f64, name: &str) -> Result<()> {
    ensure!(
        value.is_finite() && value >= 0.0,
        "{name} must be finite and nonnegative"
    );
    Ok(())
}

impl RollRegister {
    /// Reject duplicate identities, non-finite measurements and out-of-roll flaws.
    pub fn validate(&self) -> Result<()> {
        let mut ids = BTreeSet::new();
        for r in &self.rolls {
            ensure!(
                !r.id.trim().is_empty() && ids.insert(&r.id),
                "roll ids must be nonempty and unique: {}",
                r.id
            );
            ensure!(
                !r.lot.trim().is_empty()
                    && !r.dye_lot.trim().is_empty()
                    && !r.shade_group.trim().is_empty(),
                "roll {} needs lot, dye lot and shade group",
                r.id
            );
            positive(r.ticket_length_m, "ticket length")?;
            positive(r.measured_length_m, "measured length")?;
            positive(r.usable_width_mm, "usable width")?;
            positive(
                r.measured_length_m * r.usable_width_mm / 1000.0,
                "measured area",
            )?;
            for pct in [r.shrinkage.length_pct, r.shrinkage.width_pct] {
                ensure!(
                    pct.is_finite() && pct > -100.0 && pct < 100.0,
                    "shrinkage percent must be between -100 and 100"
                );
            }
            let mut defects = BTreeSet::new();
            for d in &r.defects {
                ensure!(
                    !d.id.trim().is_empty() && defects.insert(&d.id),
                    "defect ids must be nonempty and unique within roll {}",
                    r.id
                );
                ensure!(
                    (1..=4).contains(&d.points),
                    "defect {} needs 1 to 4 points",
                    d.id
                );
                nonnegative(d.along_m, "defect along position")?;
                nonnegative(d.across_mm, "defect across position")?;
                positive(d.length_mm, "defect length")?;
                positive(d.width_mm, "defect width")?;
                ensure!(
                    d.along_m + d.length_mm / 1000.0 <= r.measured_length_m + 1e-9
                        && d.across_mm + d.width_mm <= r.usable_width_mm + 1e-6,
                    "defect {} lies outside roll {}",
                    d.id,
                    r.id
                );
            }
        }
        Ok(())
    }
}
