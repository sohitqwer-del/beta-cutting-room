//! Convert the defects actually encountered by a spread into marker coordinates.

use crate::{DefectAction, LaySpread, RollRegister};
use anyhow::{Result, ensure};
use beta_marker_core::Zone;
use serde::{Deserialize, Serialize};

/// Low-ply cut-around policy; higher-ply lays require physical repair instructions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DefectMappingSettings {
    /// Largest requested lay ply count eligible for marker cut-around zones.
    pub cut_around_max_plies: u32,
    /// Extra exclusion around each flaw, in marker millimetres.
    pub margin_mm: f64,
}

impl Default for DefectMappingSettings {
    fn default() -> Self {
        Self {
            cut_around_max_plies: 4,
            margin_mm: 0.0,
        }
    }
}

/// A defect removal that needs replacement cloth and a reviewed revised spread.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DefectCutInstruction {
    /// Source stock identity.
    pub roll_id: String,
    /// Inspector's flaw identity within that roll.
    pub defect_id: String,
    /// One-based affected ply number.
    pub ply: u32,
    /// Start of the affected cloth on the roll, in metres.
    pub roll_start_m: f64,
    /// End of the affected cloth on the roll, in metres.
    pub roll_end_m: f64,
    /// Start of the affected interval along the marker, in metres.
    pub marker_start_m: f64,
    /// End of the affected interval along the marker, in metres.
    pub marker_end_m: f64,
    /// A supplied safe zone contains the full cut-out interval.
    pub splice_zone_available: bool,
    /// Human-readable next action; these instructions do not silently alter stock.
    pub instruction: String,
}

/// `zones` can be assigned directly to `Job.fabric.defects` before nesting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayDefectMap {
    /// Cut-plan lay identity.
    pub lay_id: String,
    /// Conservative marker exclusions in core millimetre coordinates.
    pub zones: Vec<Zone>,
    /// Pending full-width cut-outs requiring replacement stock and replanning.
    pub cut_instructions: Vec<DefectCutInstruction>,
    /// True when physical cut-outs need replacement material and replanning.
    pub requires_replanning: bool,
    /// Reasons why the initial allocation cannot yet account for a repair.
    pub warnings: Vec<String>,
}

/// Union the per-ply flaws by returning all exclusion rectangles. A low-ply marker
/// avoids every returned rectangle on every ply, which is conservative and safe.
/// All spread segments run forward, face up, from the usable left edge.
pub fn map_defects(
    register: &RollRegister,
    spread: &LaySpread,
    settings: &DefectMappingSettings,
) -> Result<LayDefectMap> {
    register.validate()?;
    crate::model::positive(spread.lay.marker_length_m, "marker length")?;
    crate::model::positive(spread.lay.fabric_width_mm, "marker width")?;
    crate::model::nonnegative(settings.margin_mm, "defect margin")?;
    let mut out = LayDefectMap {
        lay_id: spread.lay.id.clone(),
        zones: Vec::new(),
        cut_instructions: Vec::new(),
        requires_replanning: false,
        warnings: Vec::new(),
    };
    for (si, seg) in spread.segments.iter().enumerate() {
        let r = register
            .rolls
            .iter()
            .find(|r| r.id == seg.roll_id)
            .ok_or_else(|| anyhow::anyhow!("unknown spread roll {}", seg.roll_id))?;
        crate::model::nonnegative(seg.roll_start_m, "roll start")?;
        crate::model::nonnegative(seg.marker_start_m, "marker start")?;
        crate::model::positive(seg.length_m, "spread length")?;
        ensure!(
            seg.ply > 0
                && seg.ply <= spread.allocated_plies
                && spread.allocated_plies <= spread.lay.plies,
            "spread ply number is invalid"
        );
        ensure!(
            seg.roll_start_m + seg.length_m <= r.measured_length_m + 1e-8
                && seg.marker_start_m + seg.length_m <= spread.lay.marker_length_m + 1e-8,
            "spread segment is out of bounds"
        );
        ensure!(
            r.usable_width_mm + 1e-8 >= spread.lay.fabric_width_mm,
            "spread roll is narrower than marker"
        );
        for d in &r.defects {
            if d.action == DefectAction::Ignore {
                continue;
            }
            let a = d.along_m.max(seg.roll_start_m);
            let b = (d.along_m + d.length_mm / 1000.0).min(seg.roll_start_m + seg.length_m);
            let y0 = (d.across_mm - settings.margin_mm).max(0.0);
            let y1 =
                (d.across_mm + d.width_mm + settings.margin_mm).min(spread.lay.fabric_width_mm);
            if b <= a || y1 <= y0 {
                continue;
            }
            let x0 = (seg.marker_start_m + a - seg.roll_start_m).max(0.0);
            let x1 = (seg.marker_start_m + b - seg.roll_start_m).min(spread.lay.marker_length_m);
            if d.action == DefectAction::CutAround
                && spread.lay.plies <= settings.cut_around_max_plies
            {
                let left = (x0 * 1000.0 - settings.margin_mm).max(0.0);
                let right =
                    (x1 * 1000.0 + settings.margin_mm).min(spread.lay.marker_length_m * 1000.0);
                out.zones.push(Zone {
                    id: format!("{}:{}:{}:{}:{si}", spread.lay.id, r.id, d.id, seg.ply),
                    polygon: vec![[left, y0], [right, y0], [right, y1], [left, y1]],
                    note: Some(format!("{}; roll {}, ply {}", d.defect_type, r.id, seg.ply)),
                });
            } else {
                let safe = spread.lay.allow_splicing
                    && spread
                        .lay
                        .splice_zones
                        .iter()
                        .any(|z| x0 >= z.start_m && x1 <= z.end_m);
                out.cut_instructions.push(DefectCutInstruction { roll_id: r.id.clone(), defect_id: d.id.clone(), ply: seg.ply, roll_start_m: a, roll_end_m: b, marker_start_m: x0, marker_end_m: x1, splice_zone_available: safe, instruction: if safe { "Cut out the defect; insert sound replacement cloth with the configured overlap. Recompute the roll balance before spreading.".into() } else { "No supplied splice zone covers this defect. Stop this ply; re-cut or revise the marker and spreading plan.".into() } });
                out.requires_replanning = true;
            }
        }
    }
    if out.requires_replanning {
        out.warnings.push("Defect cut-outs are pending instructions. Their replacement cloth and overlap are not included in the allocation totals; replan before production.".into());
    }
    Ok(out)
}
