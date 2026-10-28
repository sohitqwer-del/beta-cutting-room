//! Cost comparison uses measured metres, explicit stock recovery and one currency.
use crate::{CutOrder, CutPlan, PlanOptions, spreading_requests};
use anyhow::{Context, Result, ensure};
use beta_marker_core::Layout;
use beta_marker_roll::{AllocationSettings, RollRegister, SpreadingPlan};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Purchase price in the currency named on the request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "unit", rename_all = "snake_case")]
pub enum FabricPrice {
    /// Price per linear metre at the offered width.
    Metre {
        /// Currency units per metre.
        price: f64,
    },
    /// Price per kilogram; GSM is grams per square metre.
    Kilogram {
        /// Currency units per kilogram.
        price: f64,
        /// Fabric mass per area.
        gsm: f64,
    },
}
impl FabricPrice {
    /// Convert an open, physical width to currency per linear metre.
    pub fn per_metre(&self, physical_width_mm: f64) -> Result<f64> {
        positive(physical_width_mm, "physical width")?;
        let rate = match *self {
            Self::Metre { price } => {
                nonnegative(price, "metre price")?;
                price
            }
            Self::Kilogram { price, gsm } => {
                nonnegative(price, "kilogram price")?;
                positive(gsm, "GSM")?;
                price * gsm * physical_width_mm / 1_000_000.0
            }
        };
        nonnegative(rate, "price per metre")?;
        Ok(rate)
    }
}

/// One commercial fabric width and its eligible measured stock.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidthOffer {
    /// Stable alternative id.
    pub id: String,
    /// Usable laid-flat width in millimetres.
    pub width_mm: f64,
    /// Purchase price at this width.
    pub price: FabricPrice,
    /// Cost of starting a lay, in currency units.
    pub lay_cost: f64,
    /// Additional purchase allowance as percent of consumed cloth.
    #[serde(default)]
    pub purchase_wastage_pct: f64,
    /// Fraction of opened-roll remainders charged to this order (0 = returned
    /// at full value, 1 = charged fully). Recoverable stock is never called scrap.
    #[serde(default)]
    pub end_bit_charge_fraction: f64,
    /// Optional measured stock. Without stock, feasibility is geometry/demand only.
    #[serde(default)]
    pub rolls: Option<RollRegister>,
    /// Restrict stock to these approved rolls; empty means all compatible rolls.
    #[serde(default)]
    pub eligible_roll_ids: Vec<String>,
}

/// Ratio alternatives. Empty ratios enable automatic ratio search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatioSet {
    /// Stable alternative id.
    pub id: String,
    /// Permitted ratios; no automatic repair ratio is added to a nonempty menu.
    #[serde(default)]
    pub ratios: Vec<BTreeMap<String, u32>>,
}

/// Reproducible width-by-ratio comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostRequest {
    /// Same pattern geometry and order for all alternatives.
    pub order: CutOrder,
    /// Every price and cost uses this currency (for example INR).
    pub currency: String,
    /// Widths to compare serially, each using two nesting workers.
    pub offers: Vec<WidthOffer>,
    /// An empty list enables one automatic ratio set.
    #[serde(default)]
    pub ratio_sets: Vec<RatioSet>,
    /// Short-nest and integer-search controls.
    #[serde(default)]
    pub options: PlanOptions,
    /// Number of short-listed alternatives receiving longer final nests.
    pub finalists: usize,
    /// Seconds per final ratio. Must be positive and at least the short budget.
    pub final_time_s: f64,
    /// Roll allocator search controls.
    #[serde(default)]
    pub allocation: AllocationSettings,
}

/// Auditable cost components; lengths are linear metres, costs use request currency.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OrderCost {
    /// Marker length times plies, before end allowances.
    pub net_marker_m: f64,
    /// Head and tail allowance across all plies.
    pub end_allowance_m: f64,
    /// Consumed stock including allowances and physical splice overlap.
    pub consumed_m: f64,
    /// Recoverable remainder on opened rolls, kept separate from physical use.
    pub recoverable_end_bits_m: f64,
    /// Remainder charged to this order under the supplied policy.
    pub charged_end_bits_m: f64,
    /// Purchase allowance, additional to consumed stock.
    pub purchase_wastage_m: f64,
    /// Consumed cloth plus charged remainders plus purchase allowance.
    pub purchased_m: f64,
    /// Effective currency per purchased metre, using measured physical roll
    /// widths when stock is priced by kilogram.
    pub price_per_m: f64,
    /// Purchased metres times price per metre.
    pub fabric_cost: f64,
    /// Cost per lay times lay count.
    pub lay_cost: f64,
    /// Fabric plus lay cost.
    pub total: f64,
    /// Total cost divided by ordered garments, including required over-cut cost.
    pub per_ordered_garment: f64,
}

/// One alternative, including failures so a comparison cannot hide missing stock.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostTrial {
    /// Index of offer in the request.
    pub offer_index: usize,
    /// Offer id.
    pub offer_id: String,
    /// Ratio-set id.
    pub ratio_set: String,
    /// Width, millimetres.
    pub width_mm: f64,
    /// True after the final, longer search and fresh roll allocation.
    pub final_evaluated: bool,
    /// Complete demand and, when provided, complete stock allocation.
    pub feasible: bool,
    /// Actual validated plan, never calibration estimates.
    pub plan: Option<CutPlan>,
    /// Audited stock allocation, when stock was supplied.
    pub spread: Option<SpreadingPlan>,
    /// Price components, absent when planning failed.
    pub cost: Option<OrderCost>,
    /// Failure or incomplete-stock reason.
    pub error: Option<String>,
}

/// Full comparison and a human-readable selection reason.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostComparison {
    /// Price unit for all costs.
    pub currency: String,
    /// All short alternatives, updated with final measured results for finalists.
    pub trials: Vec<CostTrial>,
    /// Index in trials. Only feasible, finally nested alternatives can win.
    pub chosen: Option<usize>,
    /// Why this width and ratio set won, or why no feasible winner exists.
    pub reason: String,
}

fn positive(n: f64, label: &str) -> Result<()> {
    ensure!(
        n.is_finite() && n > 0.0,
        "{label} must be finite and positive"
    );
    Ok(())
}
fn nonnegative(n: f64, label: &str) -> Result<()> {
    ensure!(
        n.is_finite() && n >= 0.0,
        "{label} must be finite and nonnegative"
    );
    Ok(())
}

/// Re-price a measured plan and re-allocate its actual final lengths. Stock must
/// use the same laid-flat width and shrinkage convention as the source order.
pub fn price_plan(
    plan: &CutPlan,
    offer: &WidthOffer,
    allocation: &AllocationSettings,
) -> Result<(OrderCost, Option<SpreadingPlan>)> {
    ensure!(plan.complete, "Costing requires a complete measured plan.");
    let rate = offer.price.per_metre(offer.width_mm)?;
    nonnegative(offer.lay_cost, "lay cost")?;
    nonnegative(offer.purchase_wastage_pct, "purchase wastage")?;
    ensure!(
        offer.end_bit_charge_fraction.is_finite()
            && (0.0..=1.0).contains(&offer.end_bit_charge_fraction),
        "End-bit charge fraction must be in [0, 1]."
    );
    let net: f64 = plan
        .lays
        .iter()
        .map(|l| l.marker_length_mm * f64::from(l.passes) / 1000.0)
        .sum();
    let ends: f64 = plan
        .lays
        .iter()
        .map(|l| 2.0 * plan.options.end_allowance_mm * f64::from(l.passes) / 1000.0)
        .sum();
    ensure!(
        (plan.total_fabric_m - net - ends).abs() <= 1e-6 * (1.0 + net + ends),
        "Plan fabric balance does not reconcile."
    );
    let spread = if let Some(register) = &offer.rolls {
        let requests = spreading_requests(plan, offer.width_mm, &offer.eligible_roll_ids)?;
        let spread = beta_marker_roll::allocate(register, &requests, allocation)?;
        beta_marker_roll::validate_plan(register, &spread)?;
        Some(spread)
    } else {
        None
    };
    let consumed = spread
        .as_ref()
        .map_or(plan.total_fabric_m, |s| s.total_used_m);
    let recoverable = spread.as_ref().map_or(0.0, |s| s.total_end_bits_m);
    let charged = recoverable * offer.end_bit_charge_fraction;
    let wastage = consumed * offer.purchase_wastage_pct / 100.0;
    let purchased = consumed + charged + wastage;
    // A wider measured roll consumes more kilograms per metre than its nested
    // usable strip. Price physical stock width, including width trim, exactly.
    let fabric = if let (Some(register), Some(spread)) = (&offer.rolls, &spread) {
        let mut consumed_cost = 0.0;
        let mut end_cost = 0.0;
        for balance in &spread.rolls {
            let roll = register
                .rolls
                .iter()
                .find(|r| r.id == balance.roll_id)
                .context("Missing allocated stock")?;
            let roll_rate = offer.price.per_metre(roll.usable_width_mm)?;
            consumed_cost += balance.used_m * roll_rate;
            if balance.opened {
                end_cost += balance.remaining_m * offer.end_bit_charge_fraction * roll_rate;
            }
        }
        consumed_cost * (1.0 + offer.purchase_wastage_pct / 100.0) + end_cost
    } else {
        purchased * rate
    };
    let lay_cost = offer.lay_cost * plan.lays.len() as f64;
    let total = fabric + lay_cost;
    let count: u64 = plan.ordered.values().map(|q| u64::from(*q)).sum();
    for value in [
        net,
        ends,
        consumed,
        recoverable,
        charged,
        wastage,
        purchased,
        fabric,
        lay_cost,
        total,
    ] {
        nonnegative(value, "computed cost/length")?;
    }
    Ok((
        OrderCost {
            net_marker_m: net,
            end_allowance_m: ends,
            consumed_m: consumed,
            recoverable_end_bits_m: recoverable,
            charged_end_bits_m: charged,
            purchase_wastage_m: wastage,
            purchased_m: purchased,
            price_per_m: if purchased > 0.0 {
                fabric / purchased
            } else {
                rate
            },
            fabric_cost: fabric,
            lay_cost,
            total,
            per_ordered_garment: if count > 0 { total / count as f64 } else { 0.0 },
        },
        spread,
    ))
}

fn evaluate(
    request: &CostRequest,
    offer_index: usize,
    ratios: &RatioSet,
    incumbent: Option<&CutPlan>,
    progress: &mut impl FnMut(&CutPlan) -> Result<()>,
    engine: &mut crate::NestExecutor<'_>,
) -> Result<CostTrial> {
    let final_run = incumbent.is_some();
    let offer = &request.offers[offer_index];
    let mut order = request.order.clone();
    order.job.fabric.width = offer.width_mm;
    let mut opts = request.options.clone();
    let rate = offer.price.per_metre(offer.width_mm)?;
    // The integer solver works in metre equivalents. Free fabric cannot encode
    // a nonzero lay price with this linear objective.
    ensure!(
        rate > 0.0 || offer.lay_cost == 0.0,
        "A positive lay cost requires a positive fabric price."
    );
    opts.lay_cost_m = if rate > 0.0 {
        offer.lay_cost / (rate * (1.0 + offer.purchase_wastage_pct / 100.0))
    } else {
        0.0
    };
    opts.final_time_s = if final_run { request.final_time_s } else { 0.0 };
    let mut trial = CostTrial {
        offer_index,
        offer_id: offer.id.clone(),
        ratio_set: ratios.id.clone(),
        width_mm: offer.width_mm,
        final_evaluated: final_run,
        feasible: false,
        plan: None,
        spread: None,
        cost: None,
        error: None,
    };
    let plan = match incumbent {
        Some(plan) => crate::refine_plan_with_engine(&order, &opts, plan, progress, engine)?,
        None => crate::plan_ratios_with_engine(&order, &opts, &ratios.ratios, progress, engine)?,
    };
    if let Some(incumbent) = incumbent
        && plan
            .warnings
            .iter()
            .skip(incumbent.warnings.len())
            .any(|w| w.starts_with("Final ratio kept its validated incumbent:"))
    {
        trial.final_evaluated = false;
        trial.error=Some("A final engine call failed; retained short geometry is reviewable but cannot win the final comparison.".into());
        trial.plan = Some(plan);
        return Ok(trial);
    }
    if !plan.complete {
        trial.error = Some("Order quantities are incomplete.".into());
        trial.plan = Some(plan);
        return Ok(trial);
    }
    let (cost, spread) = price_plan(&plan, offer, &request.allocation)?;
    trial.feasible = spread.as_ref().is_none_or(|s| s.complete);
    if !trial.feasible {
        trial.error = Some("Eligible stock cannot fill the chosen lays.".into());
    }
    trial.cost = Some(cost);
    trial.plan = Some(plan);
    trial.spread = spread;
    Ok(trial)
}

/// Compare widths and ratio menus. Shortlist by actual short-plan cost, then
/// re-nest finalists and recompute integer lays and stock from measured lengths.
pub fn compare(request: &CostRequest) -> Result<CostComparison> {
    compare_with_progress(request, |_| Ok(()))
}

/// As [`compare`], with partial-plan checkpoints between nesting calls.
pub fn compare_with_progress(
    request: &CostRequest,
    progress: impl FnMut(&CutPlan) -> Result<()>,
) -> Result<CostComparison> {
    compare_with_engine(request, progress, &mut crate::quality::checked_nest)
}

/// Compare using an application-owned engine executor for queue checkpoints and
/// cancellation between bounded engine slices.
pub fn compare_with_engine(
    request: &CostRequest,
    mut progress: impl FnMut(&CutPlan) -> Result<()>,
    engine: &mut crate::NestExecutor<'_>,
) -> Result<CostComparison> {
    ensure!(
        request.order.job.fabric.layout == Layout::Open,
        "Cost width trials currently require open fabric; folded/tubular stock needs explicit physical-width accounting."
    );
    ensure!(
        !request.currency.trim().is_empty() && !request.offers.is_empty() && request.finalists > 0,
        "Currency, offers and at least one finalist are required."
    );
    positive(request.final_time_s, "final time")?;
    ensure!(
        request.final_time_s >= request.options.time_per_marker_s,
        "Final budget must be at least the short budget."
    );
    let automatic = vec![RatioSet {
        id: "automatic".into(),
        ratios: Vec::new(),
    }];
    let sets = if request.ratio_sets.is_empty() {
        &automatic
    } else {
        &request.ratio_sets
    };
    let mut ratio_ids = std::collections::BTreeSet::new();
    ensure!(
        sets.iter()
            .all(|s| !s.id.trim().is_empty() && ratio_ids.insert(&s.id)),
        "Ratio-set ids must be nonempty and unique."
    );
    let mut ids = std::collections::BTreeSet::new();
    let mut offer_errors = Vec::new();
    for offer in &request.offers {
        ensure!(
            !offer.id.trim().is_empty() && ids.insert(&offer.id),
            "Offer ids must be nonempty and unique."
        );
        let checked = (|| -> Result<()> {
            offer.price.per_metre(offer.width_mm)?;
            if let Some(register) = &offer.rolls {
                register.validate()?;
                let mut count = 0;
                for roll in register.rolls.iter().filter(|r| {
                    r.usable_width_mm >= offer.width_mm
                        && (offer.eligible_roll_ids.is_empty()
                            || offer.eligible_roll_ids.contains(&r.id))
                }) {
                    count += 1;
                    ensure!(
                        roll.shrinkage.length_pct
                            <= request.order.job.fabric.shrinkage.length_pct + 1e-9
                            && roll.shrinkage.width_pct
                                <= request.order.job.fabric.shrinkage.width_pct + 1e-9,
                        "Offer {} roll {} needs more shrinkage compensation than the common order; prepare compatible stock before comparing widths.",
                        offer.id,
                        roll.id
                    );
                }
                ensure!(
                    count > 0,
                    "Offer {} has no eligible stock at this width.",
                    offer.id
                );
                let eligible: std::collections::BTreeSet<_> = register
                    .rolls
                    .iter()
                    .filter(|r| {
                        r.usable_width_mm >= offer.width_mm
                            && (offer.eligible_roll_ids.is_empty()
                                || offer.eligible_roll_ids.contains(&r.id))
                    })
                    .map(|r| r.id.as_str())
                    .collect();
                let bands: std::collections::BTreeSet<_> =
                    beta_marker_roll::group_rolls(register, &Default::default())?
                        .into_iter()
                        .filter(|g| g.roll_ids.iter().any(|id| eligible.contains(id.as_str())))
                        .map(|g| (g.length_band, g.width_band))
                        .collect();
                ensure!(
                    bands.len() <= 1,
                    "Offer {} spans incompatible shrinkage bands; restrict approved roll IDs to one band pair.",
                    offer.id
                );
            }
            Ok(())
        })();
        offer_errors.push(checked.err().map(|error| format!("{error:#}")));
    }
    let mut trials = Vec::new();
    for (i, offer_error) in offer_errors.iter().enumerate() {
        for ratios in sets {
            // Input/geometry failures isolate alternatives; callback errors must
            // propagate so callers can cancel rather than finishing the matrix.
            let mut callback_error = None;
            let result = match offer_error {
                Some(error) => Err(anyhow::anyhow!(error.clone())),
                None => evaluate(
                    request,
                    i,
                    ratios,
                    None,
                    &mut |p| match progress(p) {
                        Ok(()) => Ok(()),
                        Err(e) => {
                            callback_error = Some(e.to_string());
                            Err(e)
                        }
                    },
                    engine,
                ),
            };
            if let Some(e) = callback_error {
                anyhow::bail!(e);
            }
            trials.push(match result {
                Ok(t) => t,
                Err(e) => CostTrial {
                    offer_index: i,
                    offer_id: request.offers[i].id.clone(),
                    ratio_set: ratios.id.clone(),
                    width_mm: request.offers[i].width_mm,
                    final_evaluated: false,
                    feasible: false,
                    plan: None,
                    spread: None,
                    cost: None,
                    error: Some(format!("{e:#}")),
                },
            });
        }
    }
    let mut shortlist: Vec<_> = (0..trials.len()).filter(|i| trials[*i].feasible).collect();
    shortlist.sort_by(|a, b| {
        trials[*a]
            .cost
            .as_ref()
            .map_or(f64::INFINITY, |c| c.total)
            .total_cmp(&trials[*b].cost.as_ref().map_or(f64::INFINITY, |c| c.total))
            .then(a.cmp(b))
    });
    shortlist.truncate(request.finalists);
    for i in shortlist {
        let ratio_set = sets
            .iter()
            .find(|s| s.id == trials[i].ratio_set)
            .context("Unknown ratio set")?;
        let mut failure = None;
        let result = evaluate(
            request,
            trials[i].offer_index,
            ratio_set,
            trials[i].plan.as_ref(),
            &mut |p| match progress(p) {
                Ok(()) => Ok(()),
                Err(e) => {
                    failure = Some(e.to_string());
                    Err(e)
                }
            },
            engine,
        );
        if let Some(e) = failure {
            anyhow::bail!(e);
        }
        match result {
            Ok(t) => trials[i] = t,
            Err(e) => {
                trials[i].error = Some(format!("Final evaluation failed: {e:#}"));
                trials[i].feasible = false;
            }
        }
    }
    let chosen = (0..trials.len())
        .filter(|i| trials[*i].final_evaluated && trials[*i].feasible)
        .min_by(|a, b| {
            trials[*a]
                .cost
                .as_ref()
                .map_or(f64::INFINITY, |c| c.total)
                .total_cmp(&trials[*b].cost.as_ref().map_or(f64::INFINITY, |c| c.total))
                .then(a.cmp(b))
        });
    let reason = match chosen {
        Some(i) => {
            let t = &trials[i];
            let c = t.cost.as_ref().context("Missing chosen cost")?;
            format!(
                "Choose {} at {:.1} mm, ratio set {}: lowest feasible final cost {:.2} {} ({:.4} per ordered garment), {:.3} consumed m, {:.3} recoverable end-bit m, {:.3} charged end-bit m. Actual final lengths and fresh roll allocation decide this shortlist; unfinalized alternatives are not production choices.",
                t.offer_id,
                t.width_mm,
                t.ratio_set,
                c.total,
                request.currency,
                c.per_ordered_garment,
                c.consumed_m,
                c.recoverable_end_bits_m,
                c.charged_end_bits_m
            )
        }
        None => {
            "No feasible finally nested alternative; review quantity, stock and trial failures."
                .into()
        }
    };
    Ok(CostComparison {
        currency: request.currency.clone(),
        trials,
        chosen,
        reason,
    })
}
