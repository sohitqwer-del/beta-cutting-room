//! Production report packs. Computation uses full precision; writers round only
//! display values. Planned stock is never presented as measured actual use.
use anyhow::{Context, Result, ensure};
use beta_marker_core::{Job, Layout, Order};
use beta_marker_plan::{CutPlan, cost::CostComparison};
use beta_marker_roll::{RollRegister, ShadeIdentity, SpreadingPlan};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

mod writers;
pub use writers::{csv_files, pdf, xlsx};
#[cfg(test)]
mod tests;

/// Buyer/job metadata printed consistently across every report.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ReportMeta {
    /// Purchase order / external order id.
    pub po: String,
    /// Buyer name or external id.
    pub buyer: String,
    /// Style / revision.
    pub style: String,
    /// Colour.
    pub colour: String,
    /// Fabric composition / stock code.
    pub fabric: String,
    /// Reporting date, supplied by caller (not a moving system date).
    pub date: String,
    /// Preparer name.
    pub prepared_by: String,
    /// Approver name; blank means no recorded approval.
    pub approved_by: String,
}

/// Measured order output and inventory movements. Good garments exclude rejects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderActuals {
    /// Good garments actually cut, by size. Absence of actuals is not zero output.
    pub good_cut: BTreeMap<String, u32>,
    /// Measured roll withdrawals and reusable returns.
    pub rolls: Vec<RollActual>,
}

/// One measured roll issue/return record. Multiple events for a roll may be summed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollActual {
    /// Register identity.
    pub roll_id: String,
    /// Total length withdrawn from stock, including reusable returned remnants.
    pub issued_m: f64,
    /// Usable remnants returned to stock (same lot/dye/shade identity).
    pub returned_m: f64,
    /// Identified irrecoverable scrap within `issued - returned`, not added again.
    pub scrap_m: f64,
}

/// Inputs to one order report pack. Plan geometry is independently checked.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportInput {
    /// Human and external identifiers.
    pub meta: ReportMeta,
    /// Original patterns and fabric rules used for the plan.
    pub job: Job,
    /// Planned garment and fabric quantities.
    pub plan: CutPlan,
    /// Fabric mass in grams per square metre.
    pub gsm: f64,
    /// Currency label, e.g. INR.
    pub currency: String,
    /// Price per linear metre at the laid-flat stock width.
    pub price_per_m: f64,
    /// Cost per lay in the named currency.
    pub lay_cost: f64,
    /// Purchase wastage allowance, percent of planned consumed cloth.
    pub purchase_wastage_pct: f64,
    /// Measured inventory, required with spread or actual roll records.
    #[serde(default)]
    pub register: Option<RollRegister>,
    /// Planned stock allocation, independently audited before reporting.
    #[serde(default)]
    pub spread: Option<SpreadingPlan>,
    /// Optional width comparison, kept separate from actual cutting.
    #[serde(default)]
    pub comparison: Option<CostComparison>,
    /// Actual issue/returns and good cut quantities. None means unmeasured.
    #[serde(default)]
    pub actuals: Option<OrderActuals>,
}

/// Consumption in the common merchandiser units.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Consumption {
    /// Linear metres per garment.
    pub m_per_piece: f64,
    /// Linear metres per dozen garments.
    pub m_per_dozen: f64,
    /// Estimated kilograms per garment from GSM and physical fabric width.
    pub kg_per_piece: f64,
    /// Estimated kilograms per dozen garments.
    pub kg_per_dozen: f64,
    /// Linear yards per garment, exact conversion 1 yd = 0.9144 m.
    pub yards_per_piece: f64,
}

/// Costing values and their explicit denominators.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostingSheet {
    /// Laid-flat cuttable width, millimetres.
    pub usable_width_mm: f64,
    /// Laid-flat cuttable width / 25.4. Tubular stock uses laid-flat dia convention.
    pub width_inches: f64,
    /// Physical width for GSM mass: twice laid-flat width on folded/tubular stock.
    pub mass_width_mm: f64,
    /// Physical cloth layers across the laid-flat frame: 1 open, 2 folded/tubular.
    pub fabric_layers: u8,
    /// Grams per square metre.
    pub gsm: f64,
    /// Required garments; gross planned consumption uses this denominator.
    pub ordered_garments: u64,
    /// Planned garments including over-cut; net marker consumption uses this denominator.
    pub planned_garments: u64,
    /// Marker length times plies, excluding end allowance.
    pub net_marker_m: f64,
    /// Head/tail allowance across all plies.
    pub end_loss_m: f64,
    /// Physical stock overlap at splices.
    pub splice_loss_m: f64,
    /// Net marker + end loss + splice loss.
    pub consumed_m: f64,
    /// Additional purchase allowance in metres.
    pub purchase_wastage_m: f64,
    /// Consumption plus purchase allowance; reusable roll ends are excluded.
    pub gross_purchase_m: f64,
    /// Specified purchase allowance percent.
    pub purchase_wastage_pct: f64,
    /// Extra gross purchase compared with net marker length, percent of net.
    pub total_extra_pct_of_net: f64,
    /// Net marker metres divided by planned garments.
    pub net: Consumption,
    /// Gross purchased metres divided by ordered garments.
    pub gross: Consumption,
    /// Actual consumed metres divided by measured good garments, if measured.
    pub actual: Option<Consumption>,
    /// Measured issues minus reusable returns, if measured.
    pub actual_consumed_m: Option<f64>,
    /// Full order purchase cost plus lay cost.
    pub total_cost: f64,
    /// Full order cost per ordered garment.
    pub cost_per_ordered_piece: f64,
    /// Editable purchase price per linear metre in the named currency.
    pub price_per_m: f64,
    /// Editable cost of starting one lay in the named currency.
    pub price_per_lay: f64,
}

/// One size's demand, planned production and measured good cut balance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationRow {
    /// Garment size.
    pub size: String,
    /// Buyer requirement.
    pub ordered: u32,
    /// Planned garments including any permitted extra.
    pub planned: u32,
    /// Measured good cut garments; absent until measured.
    pub cut: Option<u32>,
    /// Planned minus ordered (signed).
    pub planned_balance: i64,
    /// Actual good cut minus ordered; absent until measured.
    pub cut_balance: Option<i64>,
}

/// One physical lay ticket, including stock identity and size quantities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayTicket {
    /// Stable L001 etc., independent of roll execution order.
    pub lay: String,
    /// Marker identifier, stable within the report.
    pub marker: String,
    /// Plies / spreads as used by the source plan.
    pub plies: u32,
    /// Marker length, metres.
    pub marker_m: f64,
    /// Head allowance per ply, metres.
    pub head_m: f64,
    /// Tail allowance per ply, metres.
    pub tail_m: f64,
    /// Garments per ply, by size.
    pub ratio: BTreeMap<String, u32>,
    /// Output garments, by size.
    pub quantities: BTreeMap<String, u32>,
    /// Referenced roll ids.
    pub rolls: Vec<String>,
    /// Full lot/dye/shade identity; no implicit approval of mixed shades.
    pub shades: Vec<ShadeIdentity>,
    /// Final length of reusable ends reported at this lay, metres.
    pub recoverable_ends_m: f64,
}

/// Register balance, including identity on every reusable end bit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollRow {
    /// Stock identity.
    pub roll: String,
    /// Lot.
    pub lot: String,
    /// Dye lot.
    pub dye_lot: String,
    /// Shade group.
    pub shade: String,
    /// Usable laid-flat width / 25.4.
    pub width_inches: f64,
    /// Initial measured usable length.
    pub initial_m: f64,
    /// Planned stock consumption; absent without allocation.
    pub planned_used_m: Option<f64>,
    /// Planned recoverable remainder on opened rolls.
    pub planned_end_bit_m: Option<f64>,
    /// Measured issues.
    pub issued_m: Option<f64>,
    /// Measured reusable returns.
    pub returned_m: Option<f64>,
    /// Actual consumed cloth, including identified scrap.
    pub actual_used_m: Option<f64>,
    /// Identified scrap within actual used cloth.
    pub scrap_m: Option<f64>,
    /// Initial - issued + returned, if measured; otherwise initial - planned use.
    pub balance_m: f64,
}

/// Entire versioned report result; renderers contain no business calculations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportPack {
    /// beta-marker.production-report/v1.
    pub format: String,
    /// Source metadata.
    pub meta: ReportMeta,
    /// Source order id/name.
    pub order: String,
    /// Currency label.
    pub currency: String,
    /// Cost and consumption sheet.
    pub costing: CostingSheet,
    /// Whether planned quantities cover the complete order.
    pub plan_complete: bool,
    /// One ticket per lay.
    pub lays: Vec<LayTicket>,
    /// Width alternatives and selected reason, when supplied.
    pub widths: Option<CostComparison>,
    /// Measured register identities and stock balances.
    pub rolls: Vec<RollRow>,
    /// Ordered / planned / cut garments per size.
    pub reconciliation: Vec<ReconciliationRow>,
    /// Assumptions and scope.
    pub notes: Vec<String>,
}

fn nn(n: f64) -> bool {
    n.is_finite() && n >= 0.0
}
fn consumption(m: f64, garments: u64, width: f64, gsm: f64) -> Result<Consumption> {
    let per = if garments == 0 {
        0.0
    } else {
        m / garments as f64
    };
    let kg = per * width * gsm / 1_000_000.0;
    let result = Consumption {
        m_per_piece: per,
        m_per_dozen: 12.0 * per,
        kg_per_piece: kg,
        kg_per_dozen: 12.0 * kg,
        yards_per_piece: per / 0.9144,
    };
    ensure!(
        [
            result.m_per_piece,
            result.m_per_dozen,
            result.kg_per_piece,
            result.kg_per_dozen,
            result.yards_per_piece
        ]
        .into_iter()
        .all(nn),
        "Consumption numeric overflow."
    );
    Ok(result)
}

/// Build reconciled reports. Actuals use physical roll lengths and good garments;
/// folded/tubular mass uses twice the laid-flat width exactly once.
pub fn build(input: &ReportInput) -> Result<ReportPack> {
    ensure!(
        input.gsm.is_finite()
            && input.gsm > 0.0
            && input.job.fabric.width.is_finite()
            && input.job.fabric.width > 0.0,
        "GSM and width must be finite and positive."
    );
    ensure!(
        [
            input.price_per_m,
            input.lay_cost,
            input.purchase_wastage_pct,
            input.plan.options.end_allowance_mm
        ]
        .into_iter()
        .all(nn),
        "Prices, allowance and wastage must be finite and nonnegative."
    );
    ensure!(!input.currency.trim().is_empty(), "Currency is required.");
    if let Some(comparison) = &input.comparison {
        ensure!(
            comparison.currency == input.currency,
            "Width comparison uses a different currency."
        );
        for trial in &comparison.trials {
            if let Some(plan) = &trial.plan {
                ensure!(
                    plan.order_name == input.plan.order_name && plan.ordered == input.plan.ordered,
                    "Width comparison belongs to a different order."
                );
            }
        }
        if let Some(i) = comparison.chosen {
            let chosen = comparison
                .trials
                .get(i)
                .context("Width comparison chosen index is invalid")?;
            ensure!(
                chosen.feasible && chosen.final_evaluated,
                "Width comparison winner must be feasible and finally evaluated."
            );
        }
    }
    let mut made = BTreeMap::<String, u64>::new();
    let mut net = 0.0;
    let mut end_loss = 0.0;
    let mut lays = Vec::new();
    if let Some(register) = &input.register {
        register.validate()?;
    }
    if let Some(spread) = &input.spread {
        beta_marker_roll::validate_plan(
            input
                .register
                .as_ref()
                .context("Spread requires a roll register")?,
            spread,
        )?;
    }
    for (i, lay) in input.plan.lays.iter().enumerate() {
        ensure!(lay.plies > 0, "Lay plies must be positive.");
        let mut job = input.job.clone();
        job.order = Some(Order {
            sizes: lay.ratio.clone(),
        });
        let result = lay
            .result
            .as_ref()
            .context("Report requires actual validated marker geometry")?;
        let checked = beta_marker_core::validate(&job, result);
        ensure!(
            checked.ok && (checked.length - lay.marker_length_mm).abs() < 0.01,
            "Lay marker failed independent validation or length does not match."
        );
        let id = format!("L{:03}", i + 1);
        let allocated = input
            .spread
            .as_ref()
            .map(|s| {
                s.lays
                    .iter()
                    .find(|l| l.lay.id == id)
                    .context("Spread is missing a source lay")
            })
            .transpose()?;
        if let Some(spread) = allocated {
            ensure!(
                spread.lay.plies == lay.plies
                    && (spread.lay.fabric_width_mm - input.job.fabric.width).abs() < 0.01
                    && (spread.lay.marker_length_m
                        - (lay.marker_length_mm + 2.0 * input.plan.options.end_allowance_mm)
                            / 1000.0)
                        .abs()
                        < 1e-7,
                "Spread request differs from the source plan."
            );
        }
        let mut quantities = BTreeMap::new();
        for (s, n) in &lay.ratio {
            let n = n.checked_mul(lay.plies).context("Lay quantity overflow")?;
            quantities.insert(s.clone(), n);
            *made.entry(s.clone()).or_default() += u64::from(n);
        }
        net += checked.length * f64::from(lay.plies) / 1000.0;
        end_loss += 2.0 * input.plan.options.end_allowance_mm * f64::from(lay.plies) / 1000.0;
        let rolls: Vec<_> = allocated
            .into_iter()
            .flat_map(|l| l.allocations.iter().map(|r| r.roll_id.clone()))
            .collect();
        let mut shades = BTreeSet::new();
        for id in &rolls {
            let roll = input
                .register
                .as_ref()
                .and_then(|r| r.rolls.iter().find(|r| &r.id == id))
                .context("Unknown allocated roll")?;
            shades.insert(ShadeIdentity {
                lot: roll.lot.clone(),
                dye_lot: roll.dye_lot.clone(),
                shade_group: roll.shade_group.clone(),
            });
        }
        lays.push(LayTicket {
            lay: id,
            marker: format!("M{:03}", i + 1),
            plies: lay.plies,
            marker_m: checked.length / 1000.0,
            head_m: input.plan.options.end_allowance_mm / 1000.0,
            tail_m: input.plan.options.end_allowance_mm / 1000.0,
            ratio: lay.ratio.clone(),
            quantities,
            rolls,
            shades: shades.into_iter().collect(),
            recoverable_ends_m: allocated
                .map_or(0.0, |l| l.allocations.iter().map(|r| r.end_bit_m).sum()),
        });
    }
    ensure!(
        (input.plan.total_fabric_m - net - end_loss).abs() < 1e-6 * (1.0 + net + end_loss),
        "Plan fabric does not reconcile."
    );
    for (size, n) in &input.plan.planned {
        ensure!(
            made.remove(size).unwrap_or(0) == u64::from(*n),
            "Plan quantity mismatch for {size}."
        );
    }
    ensure!(
        made.values().all(|n| *n == 0),
        "Plan has extra unreported sizes."
    );
    ensure!(
        input
            .plan
            .ordered
            .keys()
            .all(|s| input.plan.planned.contains_key(s)),
        "Plan is missing an ordered size."
    );
    let ordered: u64 = input.plan.ordered.values().map(|n| u64::from(*n)).sum();
    let planned: u64 = input.plan.planned.values().map(|n| u64::from(*n)).sum();
    ensure!(
        ordered > 0 && planned > 0,
        "Report needs nonzero ordered and planned garments."
    );
    let mut actual_by_roll = BTreeMap::<String, (f64, f64, f64)>::new();
    if let Some(actuals) = &input.actuals {
        let register = input
            .register
            .as_ref()
            .context("Actual roll movements require a register")?;
        ensure!(
            actuals
                .good_cut
                .keys()
                .all(|s| input.plan.ordered.contains_key(s)),
            "Actuals contain an unordered size."
        );
        for actual in &actuals.rolls {
            ensure!(
                register.rolls.iter().any(|r| r.id == actual.roll_id),
                "Unknown actual roll {}",
                actual.roll_id
            );
            ensure!(
                [actual.issued_m, actual.returned_m, actual.scrap_m]
                    .into_iter()
                    .all(nn)
                    && actual.returned_m + actual.scrap_m <= actual.issued_m + 1e-8,
                "Invalid issue/return/scrap balance."
            );
            let row = actual_by_roll.entry(actual.roll_id.clone()).or_default();
            row.0 += actual.issued_m;
            row.1 += actual.returned_m;
            row.2 += actual.scrap_m;
        }
    }
    let mut rows = Vec::new();
    if let Some(register) = &input.register {
        for roll in &register.rolls {
            let balance = input
                .spread
                .as_ref()
                .and_then(|s| s.rolls.iter().find(|b| b.roll_id == roll.id));
            let actual = actual_by_roll
                .get(&roll.id)
                .copied()
                .or_else(|| input.actuals.as_ref().map(|_| (0.0, 0.0, 0.0)));
            if let Some((issued, returned, _)) = actual {
                ensure!(
                    issued - returned <= roll.measured_length_m + 1e-7,
                    "Actual use exceeds measured roll length."
                );
            }
            rows.push(RollRow {
                roll: roll.id.clone(),
                lot: roll.lot.clone(),
                dye_lot: roll.dye_lot.clone(),
                shade: roll.shade_group.clone(),
                width_inches: roll.usable_width_mm / 25.4,
                initial_m: roll.measured_length_m,
                planned_used_m: balance.map(|b| b.used_m),
                planned_end_bit_m: balance.map(|b| if b.opened { b.remaining_m } else { 0.0 }),
                issued_m: actual.map(|a| a.0),
                returned_m: actual.map(|a| a.1),
                actual_used_m: actual.map(|a| a.0 - a.1),
                scrap_m: actual.map(|a| a.2),
                balance_m: actual.map_or_else(
                    || balance.map_or(roll.measured_length_m, |b| b.remaining_m),
                    |a| roll.measured_length_m - a.0 + a.1,
                ),
            });
        }
    }
    let mut actual_m = input
        .actuals
        .as_ref()
        .map(|_| actual_by_roll.values().map(|a| a.0 - a.1).sum::<f64>());
    let good = input
        .actuals
        .as_ref()
        .map(|a| a.good_cut.values().map(|n| u64::from(*n)).sum::<u64>());
    if let Some(m) = actual_m {
        ensure!(nn(m), "Actual total overflow.");
        ensure!(
            good.unwrap_or(0) == 0 || m > 0.0,
            "Good cut output requires measured fabric use."
        );
    }
    let fabric_layers = if input.job.fabric.layout == Layout::Open {
        1u8
    } else {
        2u8
    };
    let purchase_width = if let (Some(spread), Some(register)) = (&input.spread, &input.register) {
        let mut weighted = 0.0;
        for balance in &spread.rolls {
            let roll = register
                .rolls
                .iter()
                .find(|r| r.id == balance.roll_id)
                .context("Unknown stock balance")?;
            weighted += balance.used_m * roll.usable_width_mm;
        }
        if spread.total_used_m > 0.0 {
            weighted / spread.total_used_m
        } else {
            input.job.fabric.width
        }
    } else {
        input.job.fabric.width
    };
    let mass_width = purchase_width * f64::from(fabric_layers);
    // Weight actual use by each measured roll width, not the marker width.
    let actual = match (actual_m, good) {
        (Some(m), Some(n)) if n > 0 => {
            let mut c = consumption(m, n, mass_width, input.gsm)?;
            let kg: f64 = rows
                .iter()
                .map(|r| {
                    r.actual_used_m.unwrap_or(0.0) * r.width_inches * 25.4 * input.gsm / 1_000_000.0
                        * if input.job.fabric.layout == Layout::Open {
                            1.0
                        } else {
                            2.0
                        }
                })
                .sum();
            c.kg_per_piece = kg / n as f64;
            c.kg_per_dozen = 12.0 * c.kg_per_piece;
            ensure!(
                nn(c.kg_per_piece) && nn(c.kg_per_dozen),
                "Actual mass numeric overflow."
            );
            Some(c)
        }
        _ => None,
    };
    let splice = input
        .spread
        .as_ref()
        .map_or(0.0, |s| s.splice_overlap_waste_m);
    let consumed = net + end_loss + splice;
    if let Some(s) = &input.spread {
        ensure!(
            s.complete && (s.total_used_m - consumed).abs() < 1e-6 * (1.0 + consumed),
            "Report needs a complete reconciled spread."
        );
    }
    let waste = consumed * input.purchase_wastage_pct / 100.0;
    let gross = consumed + waste;
    let total = gross * input.price_per_m + lays.len() as f64 * input.lay_cost;
    ensure!(
        [
            net, end_loss, splice, consumed, waste, gross, total, mass_width
        ]
        .into_iter()
        .all(nn),
        "Report total overflow."
    );
    let reconciliation = input
        .plan
        .ordered
        .iter()
        .map(|(size, &n)| {
            let p = input.plan.planned.get(size).copied().unwrap_or(0);
            let c = input
                .actuals
                .as_ref()
                .map(|a| a.good_cut.get(size).copied().unwrap_or(0));
            ReconciliationRow {
                size: size.clone(),
                ordered: n,
                planned: p,
                cut: c,
                planned_balance: i64::from(p) - i64::from(n),
                cut_balance: c.map(|v| i64::from(v) - i64::from(n)),
            }
        })
        .collect();
    let mut notes=vec!["Net consumption uses planned garments. Gross consumption uses ordered garments, including the cost of any over-cut.".into(),
        "Gross purchase includes end/splice loss and purchase allowance. Recoverable roll ends stay in inventory and are not charged again here.".into(),
        "Mass is estimated from GSM and physical cloth width. Folded/tubular mass uses twice laid-flat width once.".into(),
        "Actuals use measured issues minus reusable returns and measured good garments. Blank actuals mean unmeasured.".into()];
    if !input.plan.complete {
        notes.push("Plan is incomplete. Gross cost and consumption cover only planned cloth; review missing sizes before quoting or cutting.".into());
    }
    Ok(ReportPack {
        format: "beta-marker.production-report/v1".into(),
        meta: input.meta.clone(),
        order: input.plan.order_name.clone(),
        currency: input.currency.clone(),
        plan_complete: input.plan.complete,
        costing: CostingSheet {
            usable_width_mm: input.job.fabric.width,
            width_inches: input.job.fabric.width / 25.4,
            mass_width_mm: mass_width,
            fabric_layers,
            gsm: input.gsm,
            ordered_garments: ordered,
            planned_garments: planned,
            net_marker_m: net,
            end_loss_m: end_loss,
            splice_loss_m: splice,
            consumed_m: consumed,
            purchase_wastage_m: waste,
            gross_purchase_m: gross,
            purchase_wastage_pct: input.purchase_wastage_pct,
            total_extra_pct_of_net: if net > 0.0 {
                100.0 * (gross - net) / net
            } else {
                0.0
            },
            net: consumption(net, planned, mass_width, input.gsm)?,
            gross: consumption(gross, ordered, mass_width, input.gsm)?,
            actual,
            actual_consumed_m: actual_m.take(),
            total_cost: total,
            cost_per_ordered_piece: total / ordered as f64,
            price_per_m: input.price_per_m,
            price_per_lay: input.lay_cost,
        },
        lays,
        widths: input.comparison.clone(),
        rolls: rows,
        reconciliation,
        notes,
    })
}
