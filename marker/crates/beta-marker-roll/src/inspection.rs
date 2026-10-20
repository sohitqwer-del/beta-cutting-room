//! Four-point area scores and independent shade/shrinkage groups.

use crate::RollRegister;
use crate::model::positive;
use anyhow::{Result, ensure};
use beta_marker_core::Shrinkage;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One square yard in square metres (the yard is exactly 0.9144 m).
pub const SQUARE_YARD_M2: f64 = 0.83612736;

/// Which area unit the buyer's acceptance limit uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InspectionUnit {
    /// Points per 100 square yards of usable cloth.
    Per100SquareYards,
    /// Points per 100 square metres of usable cloth.
    Per100SquareMetres,
}

/// Optional maximum of four accumulated points per linear inspection interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PointCap {
    /// Use the inspector's supplied scores without a linear cap.
    None,
    /// Group defect start positions into yard intervals from roll start; cap at 4.
    #[default]
    PerYard,
    /// Group defect start positions into metre intervals from roll start; cap at 4.
    PerMetre,
}

/// Buyer policy. Forty per 100 square yards is a default, not a universal standard.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct InspectionSettings {
    /// Inclusive acceptance limit in the selected area unit.
    pub limit: f64,
    /// Unit of the acceptance limit.
    pub unit: InspectionUnit,
    /// Policy for several scored defects within one linear interval.
    pub point_cap: PointCap,
}

impl Default for InspectionSettings {
    fn default() -> Self {
        Self {
            limit: 40.0,
            unit: InspectionUnit::Per100SquareYards,
            point_cap: PointCap::PerYard,
        }
    }
}

/// A score from the inspected usable area, with both area units explicit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InspectionScore {
    /// Sum of the inspector's individual defect points.
    pub raw_points: u64,
    /// Sum after the selected linear cap.
    pub scored_points: u64,
    /// Measured usable area in square metres.
    pub area_m2: f64,
    /// Normalised score in square-yard units.
    pub points_per_100_sq_yd: f64,
    /// Normalised score in square-metre units.
    pub points_per_100_m2: f64,
    /// Whether the score meets the recorded inclusive limit.
    pub passed: bool,
}

/// Receiving inspection result for one roll.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RollInspection {
    /// Roll identity from the register.
    pub roll_id: String,
    /// Receiving lot identity.
    pub lot: String,
    /// Dye batch identity.
    pub dye_lot: String,
    /// Measured minus ticket length, in metres (negative means short delivery).
    pub length_difference_m: f64,
    /// Scores and acceptance decision.
    pub score: InspectionScore,
}

/// Area-weighted lot result; a passing average never hides individual failed rolls.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LotInspection {
    /// Receiving lot identity.
    pub lot: String,
    /// Dye batch identity.
    pub dye_lot: String,
    /// Number of inspected rolls in this lot/dye batch.
    pub roll_count: usize,
    /// Number of individually rejected rolls, regardless of the lot average.
    pub failed_rolls: usize,
    /// Aggregate points divided by aggregate measured area.
    pub score: InspectionScore,
}

/// Complete inspection output, preserving the policy used to make it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InspectionReport {
    /// Policy used for this report.
    pub settings: InspectionSettings,
    /// Individual roll results in receiving order.
    pub rolls: Vec<RollInspection>,
    /// Lot/dye-batch summaries in lexical identity order.
    pub lots: Vec<LotInspection>,
}

/// Normalise penalty points by measured length times usable width.
pub fn inspection_score(
    raw_points: u64,
    scored_points: u64,
    length_m: f64,
    width_mm: f64,
    settings: &InspectionSettings,
) -> Result<InspectionScore> {
    positive(length_m, "inspected length")?;
    positive(width_mm, "usable width")?;
    crate::model::nonnegative(settings.limit, "inspection limit")?;
    ensure!(
        scored_points <= raw_points,
        "scored points exceed raw points"
    );
    score_area(
        raw_points,
        scored_points,
        length_m * width_mm / 1000.0,
        settings,
    )
}

fn score_area(
    raw_points: u64,
    scored_points: u64,
    area_m2: f64,
    s: &InspectionSettings,
) -> Result<InspectionScore> {
    positive(area_m2, "inspected area")?;
    let metric = scored_points as f64 / area_m2 * 100.0;
    let yards = metric * SQUARE_YARD_M2;
    ensure!(metric.is_finite(), "inspection score overflow");
    let compared = match s.unit {
        InspectionUnit::Per100SquareYards => yards,
        InspectionUnit::Per100SquareMetres => metric,
    };
    Ok(InspectionScore {
        raw_points,
        scored_points,
        area_m2,
        points_per_100_sq_yd: yards,
        points_per_100_m2: metric,
        passed: compared <= s.limit,
    })
}

/// Inspect every roll and aggregate by the pair (lot, dye lot).
pub fn inspect(register: &RollRegister, settings: &InspectionSettings) -> Result<InspectionReport> {
    register.validate()?;
    crate::model::nonnegative(settings.limit, "inspection limit")?;
    let mut rolls = Vec::new();
    let mut lots: BTreeMap<(String, String), LotInspection> = BTreeMap::new();
    for r in &register.rolls {
        let raw: u64 = r.defects.iter().map(|d| u64::from(d.points)).sum();
        let scored = if settings.point_cap == PointCap::None {
            raw
        } else {
            let interval = if settings.point_cap == PointCap::PerYard {
                0.9144
            } else {
                1.0
            };
            let mut buckets = BTreeMap::<u64, u64>::new();
            for d in &r.defects {
                ensure!(
                    d.along_m / interval < u64::MAX as f64,
                    "roll too long for inspection intervals"
                );
                *buckets
                    .entry((d.along_m / interval).floor() as u64)
                    .or_default() += u64::from(d.points);
            }
            buckets.values().map(|p| (*p).min(4)).sum()
        };
        let score = inspection_score(
            raw,
            scored,
            r.measured_length_m,
            r.usable_width_mm,
            settings,
        )?;
        rolls.push(RollInspection {
            roll_id: r.id.clone(),
            lot: r.lot.clone(),
            dye_lot: r.dye_lot.clone(),
            length_difference_m: r.measured_length_m - r.ticket_length_m,
            score: score.clone(),
        });
        let lot = lots
            .entry((r.lot.clone(), r.dye_lot.clone()))
            .or_insert_with(|| LotInspection {
                lot: r.lot.clone(),
                dye_lot: r.dye_lot.clone(),
                roll_count: 0,
                failed_rolls: 0,
                score: InspectionScore {
                    raw_points: 0,
                    scored_points: 0,
                    area_m2: 0.0,
                    points_per_100_sq_yd: 0.0,
                    points_per_100_m2: 0.0,
                    passed: true,
                },
            });
        lot.roll_count += 1;
        lot.failed_rolls += usize::from(!score.passed);
        lot.score.raw_points += raw;
        lot.score.scored_points += scored;
        lot.score.area_m2 += score.area_m2;
    }
    let mut lots: Vec<_> = lots.into_values().collect();
    for lot in &mut lots {
        lot.score = score_area(
            lot.score.raw_points,
            lot.score.scored_points,
            lot.score.area_m2,
            settings,
        )?;
    }
    Ok(InspectionReport {
        settings: settings.clone(),
        rolls,
        lots,
    })
}

/// Grouping bandwidth in percentage points on each shrinkage axis.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GroupingSettings {
    /// A value of 2 gives bands centred on 0, 2, 4, ... with +/-1% half-width.
    pub band_step_pct: f64,
}

impl Default for GroupingSettings {
    fn default() -> Self {
        Self { band_step_pct: 2.0 }
    }
}

/// Compatible shade and shrinkage tests, kept within the same lot and dye lot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RollGroup {
    /// Receiving lot identity.
    pub lot: String,
    /// Dye batch identity.
    pub dye_lot: String,
    /// Colour group within the dye batch.
    pub shade_group: String,
    /// Band index on the length shrinkage axis.
    pub length_band: i64,
    /// Band index on the width shrinkage axis.
    pub width_band: i64,
    /// Compatible stock identities in receiving order.
    pub roll_ids: Vec<String>,
    /// Arithmetic mean of the roll tests, ready for `Job.fabric.shrinkage`.
    pub average_shrinkage: Shrinkage,
}

/// Use half-open bands [centre-step/2, centre+step/2); exact ties enter the upper band.
pub fn group_rolls(register: &RollRegister, settings: &GroupingSettings) -> Result<Vec<RollGroup>> {
    register.validate()?;
    positive(settings.band_step_pct, "shrinkage band step")?;
    let band = |v: f64| -> Result<i64> {
        let n = (v / settings.band_step_pct + 0.5).floor();
        ensure!(
            n.is_finite() && n > i64::MIN as f64 && n < i64::MAX as f64,
            "shrinkage band step is too small"
        );
        Ok(n as i64)
    };
    let mut groups = BTreeMap::new();
    for r in &register.rolls {
        let lb = band(r.shrinkage.length_pct)?;
        let wb = band(r.shrinkage.width_pct)?;
        let g = groups
            .entry((
                r.lot.clone(),
                r.dye_lot.clone(),
                r.shade_group.clone(),
                lb,
                wb,
            ))
            .or_insert_with(|| RollGroup {
                lot: r.lot.clone(),
                dye_lot: r.dye_lot.clone(),
                shade_group: r.shade_group.clone(),
                length_band: lb,
                width_band: wb,
                roll_ids: Vec::new(),
                average_shrinkage: Shrinkage::default(),
            });
        g.roll_ids.push(r.id.clone());
        let n = g.roll_ids.len() as f64;
        g.average_shrinkage.length_pct +=
            (r.shrinkage.length_pct - g.average_shrinkage.length_pct) / n;
        g.average_shrinkage.width_pct +=
            (r.shrinkage.width_pct - g.average_shrinkage.width_pct) / n;
    }
    Ok(groups.into_values().collect())
}
