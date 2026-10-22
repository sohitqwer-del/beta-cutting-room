//! One implementation of the cut-plan, roll and cutting-room API routes.
//!
//! `beta-marker serve`, the desktop app and the browser WASM module all call
//! these functions, so one request gives the same bytes on every transport.
//! Hosts own only transport details: HTTP headers, Tauri IPC or a Worker.
use anyhow::{Result, ensure};
use beta_marker_core::{Job, MarkerResult};
use beta_marker_cutroom::bundles::{Bundle, BundleRules, CutPart, Lay, PieceKey, PlyRun};
use beta_marker_plan::{CutOrder, CutPlan, NestExecutor, PlanOptions};
use beta_marker_roll::{
    AllocationSettings, GroupingSettings, InspectionSettings, LayRequest, RollRegister,
    ShadePolicy, SpreadingPlan,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;

#[cfg(test)]
mod tests;

/// A route's reply body. Hosts add their own JSON content type and headers.
#[derive(Debug)]
pub enum Output {
    Json(Vec<u8>),
    File {
        mime: &'static str,
        extension: Option<&'static str>,
        body: Vec<u8>,
    },
}
impl Output {
    fn json(value: impl Serialize) -> Result<Self> {
        Ok(Self::Json(serde_json::to_vec(&value)?))
    }
    /// Body bytes, whatever the kind.
    pub fn body(&self) -> &[u8] {
        match self {
            Self::Json(body) | Self::File { body, .. } => body,
        }
    }
}

/// Every POST route answered by [`cutroom`].
pub const CUTROOM_ROUTES: &[&str] = &[
    "/api/roll/inspect",
    "/api/roll/spread",
    "/api/cutroom/lay",
    "/api/cutroom/from-spread",
    "/api/cutroom/bundles",
    "/api/cutroom/tickets",
    "/api/cutroom/stickers",
    "/api/cutroom/cost",
    "/api/cutroom/recut",
    "/api/cutroom/report",
    "/api/cutroom/report-marker",
];

#[derive(Deserialize)]
struct InspectRequest {
    register: RollRegister,
    #[serde(default)]
    settings: InspectionSettings,
    #[serde(default)]
    grouping: GroupingSettings,
}
#[derive(Deserialize)]
struct SpreadRequest {
    register: RollRegister,
    #[serde(default)]
    lays: Vec<LayRequest>,
    #[serde(default)]
    markers: Vec<SpreadMarker>,
    #[serde(default)]
    shade_rule: ShadePolicy,
    #[serde(default)]
    eligible_roll_ids: Vec<String>,
    #[serde(default)]
    settings: AllocationSettings,
}
/// A checked marker to spread. `passes` are lengths taken from rolls; on
/// folded or tubular fabric each pass is two plies. `plies` is the older name.
#[derive(Deserialize)]
struct SpreadMarker {
    id: String,
    job: Job,
    result: MarkerResult,
    #[serde(alias = "plies")]
    passes: u32,
    #[serde(default)]
    end_allowance_mm: f64,
}
#[derive(Deserialize)]
struct FromSpreadRequest {
    job: Job,
    result: MarkerResult,
    register: RollRegister,
    spread: SpreadingPlan,
    lay_id: String,
    order: String,
    colour: String,
}
#[derive(Deserialize)]
struct BundleRequest {
    lay: Lay,
    #[serde(default)]
    rules: BundleRules,
}
#[derive(Deserialize)]
struct TicketRequest {
    bundles: Vec<Bundle>,
    paper: Option<String>,
}
#[derive(Deserialize)]
struct CostRequest {
    job: Job,
    garments: u32,
    price_per_m: f64,
}
#[derive(Deserialize)]
struct RecutRequest {
    bundle: Bundle,
    piece: PieceKey,
    id: String,
    reason: String,
}
/// A checked marker and its shade runs. Runs count passes, as on the spreading table.
#[derive(Deserialize)]
pub struct PhysicalLayRequest {
    pub job: Job,
    pub result: MarkerResult,
    pub id: String,
    pub order: String,
    pub colour: String,
    pub plies: Vec<PlyRun>,
}
#[derive(Deserialize)]
struct MarkerActualsRequest {
    source: PhysicalLayRequest,
    issued_mm: f64,
    end_bits_mm: f64,
    actual_piece_area_mm2: f64,
    actual_pieces: u64,
    recut_count: u64,
    end_allowance_per_ply_mm: f64,
}

/// Physical-layer expansion and garment numbering. Surplus layers are not parts.
pub fn physical_lay(input: PhysicalLayRequest) -> Result<Lay> {
    ensure!(
        beta_marker_core::validate(&input.job, &input.result).ok,
        "The marker must pass validation before bundling"
    );
    let mut parts = Vec::new();
    let mut slots = BTreeMap::new();
    for placement in &input.result.placements {
        let piece = input
            .job
            .pieces
            .iter()
            .find(|p| p.id == placement.piece_id)
            .ok_or_else(|| anyhow::anyhow!("Unknown piece {}", placement.piece_id))?;
        let size = piece
            .size
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Piece needs a size"))?;
        for (garment, hand) in
            beta_marker_core::rules::layer_assignments(input.job.fabric.layout, placement)
        {
            let garment = garment.ok_or_else(|| {
                anyhow::anyhow!("Marker has no garment identities. Nest it again before bundling")
            })?;
            let slot = slots.entry((piece.id.clone(), garment, hand)).or_insert(0);
            parts.push(CutPart {
                piece_id: piece.id.clone(),
                size: size.clone(),
                garment,
                slot: *slot,
                mirrored: hand,
            });
            *slot += 1;
        }
    }
    Ok(Lay {
        id: input.id,
        style: input.job.name,
        order: input.order,
        colour: input.colour,
        plies: input.plies,
        parts,
    })
}

/// Answer a roll or cutting-room route. `None` means the path is not one of [`CUTROOM_ROUTES`].
pub fn cutroom(path: &str, body: &[u8]) -> Option<Result<Output>> {
    if !CUTROOM_ROUTES.contains(&path) {
        return None;
    }
    Some(cutroom_route(path, body))
}

fn cutroom_route(path: &str, body: &[u8]) -> Result<Output> {
    match path {
        "/api/roll/inspect" => {
            let input: InspectRequest = serde_json::from_slice(body)?;
            Output::json(
                json!({"inspection": beta_marker_roll::inspect(&input.register, &input.settings)?,
                "groups": beta_marker_roll::group_rolls(&input.register, &input.grouping)?}),
            )
        }
        "/api/roll/spread" => {
            let mut input: SpreadRequest = serde_json::from_slice(body)?;
            ensure!(
                input.markers.is_empty() || input.lays.is_empty(),
                "Supply marker sources or lay requests, not both"
            );
            for marker in input.markers {
                ensure!(
                    marker.end_allowance_mm.is_finite() && marker.end_allowance_mm >= 0.0,
                    "End allowance must be non-negative"
                );
                let validation = beta_marker_core::validate(&marker.job, &marker.result);
                ensure!(validation.ok, "Marker {} must pass validation", marker.id);
                input.lays.push(LayRequest {
                    id: marker.id,
                    marker_length_m: (validation.length + 2.0 * marker.end_allowance_mm) / 1000.0,
                    plies: marker.passes,
                    fabric_width_mm: marker.job.fabric.width,
                    shade_rule: input.shade_rule.clone(),
                    eligible_roll_ids: input.eligible_roll_ids.clone(),
                    allow_splicing: false,
                    splice_zones: vec![],
                });
            }
            ensure!(!input.lays.is_empty(), "Supply at least one lay");
            let plan = beta_marker_roll::allocate(&input.register, &input.lays, &input.settings)?;
            beta_marker_roll::validate_plan(&input.register, &plan)?;
            Output::json(plan)
        }
        "/api/cutroom/from-spread" => {
            let input: FromSpreadRequest = serde_json::from_slice(body)?;
            beta_marker_roll::validate_plan(&input.register, &input.spread)?;
            let lay = input
                .spread
                .lays
                .iter()
                .find(|s| s.lay.id == input.lay_id)
                .ok_or_else(|| anyhow::anyhow!("Lay is not in the spreading plan"))?;
            Output::json(beta_marker_cutroom::integration::from_spread(
                &input.job,
                &input.result,
                &input.register,
                lay,
                &input.order,
                &input.colour,
            )?)
        }
        "/api/cutroom/lay" => Output::json(physical_lay(serde_json::from_slice(body)?)?),
        "/api/cutroom/bundles" => {
            let input: BundleRequest = serde_json::from_slice(body)?;
            Output::json(beta_marker_cutroom::bundles::bundle_lay(
                &input.lay,
                input.rules,
            )?)
        }
        "/api/cutroom/tickets" | "/api/cutroom/stickers" => {
            let input: TicketRequest = serde_json::from_slice(body)?;
            let paper = match input.paper.as_deref().unwrap_or("a4") {
                "a4" => beta_marker_cutroom::tickets::TicketPaper::A4,
                "label100x50" => beta_marker_cutroom::tickets::TicketPaper::Label100x50,
                _ => anyhow::bail!("Paper must be a4 or label100x50"),
            };
            let pdf = if path.ends_with("tickets") {
                beta_marker_cutroom::tickets::ticket_pdf(&input.bundles, paper)?
            } else {
                beta_marker_cutroom::tickets::sticker_pdf(&input.bundles, paper)?
            };
            Ok(Output::File {
                mime: "application/pdf",
                extension: None,
                body: pdf,
            })
        }
        "/api/cutroom/recut" => {
            let input: RecutRequest = serde_json::from_slice(body)?;
            Output::json(beta_marker_cutroom::recuts::Damage::record(
                input.id,
                &input.bundle,
                input.piece,
                input.reason,
            )?)
        }
        "/api/cutroom/report-marker" => {
            let input: MarkerActualsRequest = serde_json::from_slice(body)?;
            let validation = beta_marker_core::validate(&input.source.job, &input.source.result);
            ensure!(validation.ok, "The source marker must pass validation");
            let layers = if input.source.job.fabric.layout == beta_marker_core::Layout::Open {
                1.0
            } else {
                2.0
            };
            let width_mm = input.source.job.fabric.width * layers;
            let lay = physical_lay(input.source)?;
            let plies = lay
                .plies
                .last()
                .ok_or_else(|| anyhow::anyhow!("Lay needs shade runs"))?
                .last;
            beta_marker_cutroom::bundles::bundle_lay(&lay, Default::default())?;
            let actuals = beta_marker_cutroom::reports::LayActuals {
                lay: lay.id,
                order: lay.order,
                width_mm,
                marker_length_mm: validation.length,
                plies,
                end_allowance_per_ply_mm: input.end_allowance_per_ply_mm,
                piece_area_per_ply_mm2: validation.piece_area * layers,
                pieces_per_ply: u32::try_from(lay.parts.len())?,
                actual_piece_area_mm2: input.actual_piece_area_mm2,
                actual_pieces: input.actual_pieces,
                recut_count: input.recut_count,
                spreading: vec![beta_marker_cutroom::reports::SpreadingRecord {
                    issued_mm: input.issued_mm,
                    end_bits_mm: input.end_bits_mm,
                }],
            };
            Output::json(beta_marker_cutroom::reports::lay_report(&actuals)?)
        }
        "/api/cutroom/report" => Output::json(beta_marker_cutroom::reports::lay_report(
            &serde_json::from_slice(body)?,
        )?),
        "/api/cutroom/cost" => {
            let input: CostRequest = serde_json::from_slice(body)?;
            ensure!(
                input.price_per_m.is_finite() && input.price_per_m >= 0.0,
                "Price must be non-negative"
            );
            let estimate = beta_marker_cutroom::costing::estimate(
                &input.job,
                input.job.fabric.width,
                input.garments,
                &Default::default(),
            )?;
            let cost = estimate.fabric_per_garment_mm / 1000.0 * input.price_per_m;
            Output::json(
                json!({"estimate":estimate,"fabric_cost_per_garment":cost,"price_per_m":input.price_per_m}),
            )
        }
        _ => anyhow::bail!("Unknown cutting-room route"),
    }
}

/// Reject copy counts that would overflow before any geometry work starts.
pub fn ensure_counts(job: &Job) -> std::result::Result<(), String> {
    for piece in &job.pieces {
        let multiplier = beta_marker_core::rules::multiplier(job, piece);
        if piece.quantity.checked_mul(multiplier).is_none()
            || piece.mirror_quantity.checked_mul(multiplier).is_none()
        {
            return Err("Required copy count exceeds u32".into());
        }
    }
    Ok(())
}

/// Import a plotter marker (HPGL/PLT) as a job and its placed result.
/// Errors are request errors (HTTP 400 in hosts that use HTTP).
pub fn import_plt(body: &[u8], width: Option<&str>) -> std::result::Result<Output, String> {
    let Some(width) = width
        .and_then(|w| w.parse::<f64>().ok())
        .filter(|w| w.is_finite() && *w > 0.0)
    else {
        return Err("PLT import needs a positive fabric width in mm".into());
    };
    let opts = beta_marker_dxf::HpglOptions {
        width: Some(width),
        ..Default::default()
    };
    let report = beta_marker_dxf::import_hpgl(body, &opts).map_err(|e| format!("{e:#}"))?;
    Output::json(json!({
        "job": report.job,
        "result": report.result,
        "warnings": report.warnings,
    }))
    .map_err(|e| e.to_string())
}

/// A marker already measured for one ratio, for example from an earlier plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownMarker {
    pub ratio: BTreeMap<String, u32>,
    pub result: MarkerResult,
}

/// `POST /api/plan` body. `markers` lets the planner reuse measured markers:
/// a marker is used only when it passes validation for the order's job.
#[derive(Deserialize)]
pub struct PlanRequest {
    pub order: CutOrder,
    pub options: PlanOptions,
    #[serde(default)]
    pub markers: Vec<KnownMarker>,
}

/// Input checks shared by every transport, before any nesting starts.
pub fn check_plan(input: &PlanRequest) -> std::result::Result<(), String> {
    let order = &input.order;
    let options = &input.options;
    if order.max_plies == 0
        || order.max_garments_per_marker == 0
        || order.quantities.values().all(|q| *q == 0)
        || !order.overcut_pct.is_finite()
        || order.overcut_pct < 0.0
        || !options.time_per_marker_s.is_finite()
        || options.time_per_marker_s <= 0.0
        || !options.final_time_s.is_finite()
        || !(0.0..=86400.0).contains(&options.final_time_s)
        || !options.end_allowance_mm.is_finite()
        || options.end_allowance_mm < 0.0
    {
        return Err("Plan needs positive quantities, ply and garment limits, positive time, and non-negative over-cut and end allowance".into());
    }
    ensure_counts(&order.job)
}

fn without_zeros(ratio: &BTreeMap<String, u32>) -> BTreeMap<&str, u32> {
    ratio
        .iter()
        .filter(|(_, n)| **n > 0)
        .map(|(s, n)| (s.as_str(), *n))
        .collect()
}

/// Plan a cut order. `nest` is the host's engine: native threads on a server or
/// desktop, one thread in a browser Worker. Known markers that pass validation
/// replace engine calls for their ratio. `progress` receives checked partial plans.
pub fn plan(
    input: &PlanRequest,
    nest: &mut NestExecutor<'_>,
    progress: impl FnMut(&CutPlan) -> Result<()>,
) -> Result<CutPlan> {
    check_plan(input).map_err(anyhow::Error::msg)?;
    let mut engine = |job: &Job, options: &beta_marker_nest::NestOptions| {
        let wanted = job.order.as_ref().map(|o| without_zeros(&o.sizes));
        let known = input
            .markers
            .iter()
            .find(|m| Some(without_zeros(&m.ratio)) == wanted)
            .filter(|m| beta_marker_core::validate(job, &m.result).ok);
        match known {
            Some(known) => Ok(known.result.clone()),
            None => nest(job, options),
        }
    };
    beta_marker_plan::plan_ratios_with_engine(
        &input.order,
        &input.options,
        &[],
        progress,
        &mut engine,
    )
}

/// The measured markers of a plan, ready to pass back as `PlanRequest.markers`.
pub fn known_markers(plan: &CutPlan) -> Vec<KnownMarker> {
    plan.lays
        .iter()
        .filter_map(|lay| {
            lay.result.as_ref().map(|result| KnownMarker {
                ratio: lay.ratio.clone(),
                result: result.clone(),
            })
        })
        .collect()
}
