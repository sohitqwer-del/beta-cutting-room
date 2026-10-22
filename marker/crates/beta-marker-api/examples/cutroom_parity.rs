//! Native reference files for the cutting-room WASM parity harness.
//!
//! cargo run -p beta-marker-api --example cutroom_parity -- <output-directory>
//! node packages/marker-wasm/scripts/cutroom-parity.mjs <same-directory>
//!
//! Inputs: the MIT FreeSewing Simon fixture and the synthetic demo roll register.
//! Nesting is time-based, so it is not compared. A first native run records the
//! first marker measured for each ratio; both native and WASM then plan again with
//! those markers. Every later step goes through the same route on both sides.
use anyhow::{Context, Result, ensure};
use beta_marker_api::{KnownMarker, Output, PlanRequest};
use beta_marker_core::{Job, Layout, MarkerResult};
use beta_marker_plan::{CUT_ORDER_FORMAT, CutOrder, CutPlan, PlanOptions};
use serde_json::{Value, json};
use std::{fs, path::Path, path::PathBuf};

fn without_runtime(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find("\"runtime_s\":") {
        let (head, tail) = rest.split_at(i + "\"runtime_s\":".len());
        out.push_str(head);
        out.push('0');
        let end = tail.find([',', '}']).unwrap_or(tail.len());
        rest = &tail[end..];
    }
    out.push_str(rest);
    out
}

fn order(layout: Layout) -> Result<CutOrder> {
    let mut job: Job = serde_json::from_str(include_str!(
        "../../../testdata/open/freesewing/simon-ratio.job.json"
    ))?;
    job.fabric.layout = layout;
    let max_plies = if layout == Layout::Open {
        job.name = "Simon open".into();
        6
    } else {
        // Folded fabric doubles the plies per pass: 12 plies = 6 passes.
        job.name = "Simon folded".into();
        job.fabric.width = 800.0;
        12
    };
    Ok(CutOrder {
        format: CUT_ORDER_FORMAT.into(),
        name: job.name.clone(),
        job,
        quantities: [("S", 6), ("M", 10), ("L", 8), ("XL", 4)]
            .into_iter()
            .map(|(s, q)| (s.to_string(), q))
            .collect(),
        max_plies,
        max_garments_per_marker: 2,
        overcut_pct: 0.0,
    })
}

fn options() -> PlanOptions {
    PlanOptions {
        time_per_marker_s: 1.0,
        end_allowance_mm: 20.0,
        max_candidates: 16,
        max_refinements: 6,
        ..Default::default()
    }
}

/// Native plan with a recording engine: a ratio is nested once, then reused.
fn record(order: CutOrder) -> Result<(PlanRequest, CutPlan)> {
    let mut known: Vec<KnownMarker> = vec![];
    let mut engine = |job: &Job, options: &beta_marker_nest::NestOptions| -> Result<MarkerResult> {
        let ratio = job
            .order
            .as_ref()
            .map(|o| o.sizes.clone())
            .unwrap_or_default();
        if let Some(found) = known.iter().find(|k| k.ratio == ratio) {
            return Ok(found.result.clone());
        }
        let result = beta_marker_plan::checked_nest(job, options)?;
        known.push(KnownMarker {
            ratio,
            result: result.clone(),
        });
        Ok(result)
    };
    let mut request = PlanRequest {
        order,
        options: options(),
        markers: vec![],
    };
    let first = beta_marker_api::plan(&request, &mut engine, |_| Ok(()))?;
    request.markers = known;
    let mut never = |_: &Job, _: &beta_marker_nest::NestOptions| -> Result<MarkerResult> {
        anyhow::bail!("every ratio must come from the recorded markers")
    };
    let replay = beta_marker_api::plan(&request, &mut never, |_| Ok(()))?;
    ensure!(
        without_runtime(&serde_json::to_string(&first)?)
            == without_runtime(&serde_json::to_string(&replay)?),
        "native replay differs from the recorded plan"
    );
    Ok((request, replay))
}

struct Cases<'a> {
    dir: &'a Path,
    list: Vec<Value>,
}
impl Cases<'_> {
    /// Call a route natively, keep its bytes as the reference, and return them.
    fn route(&mut self, name: &str, url: &str, body: &[u8]) -> Result<Vec<u8>> {
        let (path, query) = url.split_once('?').unwrap_or((url, ""));
        let output = if let Some(output) = beta_marker_api::cutroom(path, body) {
            output?
        } else if path == "/api/import" {
            let width = query.split('&').find_map(|p| p.strip_prefix("width="));
            beta_marker_api::import_plt(body, width).map_err(anyhow::Error::msg)?
        } else {
            let format = query.trim_start_matches("format=");
            let input: Value = serde_json::from_slice(body)?;
            let job: Job = serde_json::from_value(input["job"].clone())?;
            let result: MarkerResult = serde_json::from_value(input["result"].clone())?;
            let (bytes, mime, extension) =
                beta_marker_export::export(format, &job, &result, &Default::default())?;
            Output::File {
                mime,
                extension: Some(extension),
                body: bytes,
            }
        };
        let (mime, bytes) = match output {
            Output::Json(body) => ("application/json", body),
            Output::File { mime, body, .. } => (mime, body),
        };
        let file = format!("{name}.bin");
        fs::write(self.dir.join(&file), &bytes)?;
        fs::write(self.dir.join(format!("{name}.request")), body)?;
        self.list
            .push(json!({"kind":"route","name":name,"url":url,"mime":mime,"file":file}));
        Ok(bytes)
    }
}

fn main() -> Result<()> {
    let dir = PathBuf::from(std::env::args().nth(1).context("output directory")?);
    fs::create_dir_all(&dir)?;
    let mut cases = Cases {
        dir: &dir,
        list: vec![],
    };
    let register = beta_marker_roll::demo_register();
    for layout in [Layout::Open, Layout::Folded] {
        let label = if layout == Layout::Open {
            "open"
        } else {
            "folded"
        };
        let (request, plan) = record(order(layout)?)?;
        ensure!(plan.complete, "{label} plan is incomplete");
        for lay in &plan.lays {
            ensure!(
                lay.plies == lay.passes * beta_marker_plan::plies_per_pass(layout)
                    && lay.plies <= request.order.max_plies,
                "{label}: plies and passes disagree"
            );
        }
        let request_json =
            json!({"order": request.order, "options": request.options, "markers": request.markers});
        let file = format!("plan-{label}.json");
        fs::write(
            dir.join(&file),
            without_runtime(&serde_json::to_string(&plan)?),
        )?;
        fs::write(
            dir.join(format!("plan-{label}.request")),
            request_json.to_string(),
        )?;
        cases
            .list
            .push(json!({"kind":"plan","name":format!("plan-{label}"),"file":file}));

        // Rolls: inspect, then allocate every lay's passes from the demo stock.
        cases.route(
            &format!("{label}-inspect"),
            "/api/roll/inspect",
            json!({"register": register}).to_string().as_bytes(),
        )?;
        let markers: Vec<Value> = plan
            .lays
            .iter()
            .enumerate()
            .map(|(i, lay)| {
                let mut job = request.order.job.clone();
                job.order = Some(beta_marker_core::Order {
                    sizes: lay.ratio.clone(),
                });
                json!({"id": format!("LAY-{:02}", i + 1), "job": job, "result": lay.result,
                    "passes": lay.passes, "end_allowance_mm": request.options.end_allowance_mm})
            })
            .collect();
        let spread_bytes = cases.route(
            &format!("{label}-spread"),
            "/api/roll/spread",
            json!({"register": register, "markers": markers})
                .to_string()
                .as_bytes(),
        )?;
        let spread: beta_marker_roll::SpreadingPlan = serde_json::from_slice(&spread_bytes)?;
        ensure!(spread.complete, "{label}: demo stock must cover the plan");
        ensure!(
            spread.lays.iter().all(|s| {
                let i: usize = s.lay.id[4..].parse().unwrap_or(0);
                i > 0 && s.lay.plies == plan.lays[i - 1].passes
            }),
            "{label}: rolls must supply passes"
        );

        // Lay 1: physical lay from the spread, bundles, tickets and stickers, report.
        let first = &markers[0];
        let lay_bytes = cases.route(
            &format!("{label}-from-spread"),
            "/api/cutroom/from-spread",
            json!({"job": first["job"], "result": first["result"], "register": register,
                "spread": spread, "lay_id": "LAY-01", "order": "PO-PARITY", "colour": "Navy"})
            .to_string()
            .as_bytes(),
        )?;
        let lay: Value = serde_json::from_slice(&lay_bytes)?;
        let bundle_bytes = cases.route(
            &format!("{label}-bundles"),
            "/api/cutroom/bundles",
            json!({"lay": lay}).to_string().as_bytes(),
        )?;
        let bundles: Value = serde_json::from_slice(&bundle_bytes)?;
        for (route, paper) in [
            ("tickets", "a4"),
            ("tickets", "label100x50"),
            ("stickers", "a4"),
        ] {
            cases.route(
                &format!("{label}-{route}-{paper}"),
                &format!("/api/cutroom/{route}"),
                json!({"bundles": bundles, "paper": paper})
                    .to_string()
                    .as_bytes(),
            )?;
        }
        cases.route(
            &format!("{label}-recut"),
            "/api/cutroom/recut",
            json!({"id": "D-1", "bundle": bundles[0], "piece": bundles[0]["stickers"][0]["piece"],
                "reason": "Hole near the notch"})
            .to_string()
            .as_bytes(),
        )?;
        cases.route(
            &format!("{label}-report-marker"),
            "/api/cutroom/report-marker",
            json!({"source": {"job": first["job"], "result": first["result"], "id": "LAY-01",
                "order": "PO-PARITY", "colour": "Navy", "plies": lay["plies"]},
                "issued_mm": 1.0e5, "end_bits_mm": 900.0, "actual_piece_area_mm2": 1.0e7,
                "actual_pieces": 100, "recut_count": 1, "end_allowance_per_ply_mm": 20.0})
            .to_string()
            .as_bytes(),
        )?;
        // Costing estimates open fabric only (folded jobs must be expanded first).
        if layout == Layout::Open {
            cases.route(
                &format!("{label}-cost"),
                "/api/cutroom/cost",
                json!({"job": first["job"], "price_per_m": 180.0,
                    "garments": plan.lays[0].ratio.values().sum::<u32>()})
                .to_string()
                .as_bytes(),
            )?;
        }
        // Marker report formats, and a PLT round trip for Compare.
        let marker = json!({"job": first["job"], "result": first["result"]}).to_string();
        for format in ["report", "mini", "labels", "xlsx"] {
            cases.route(
                &format!("{label}-export-{format}"),
                &format!("/api/export?format={format}"),
                marker.as_bytes(),
            )?;
        }
        let plt = cases.route(
            &format!("{label}-export-plt"),
            "/api/export?format=plt",
            marker.as_bytes(),
        )?;
        let width = request.order.job.fabric.width;
        cases.route(
            &format!("{label}-import-plt"),
            &format!("/api/import?format=plt&width={width}"),
            &plt,
        )?;
    }
    fs::write(dir.join("cases.json"), serde_json::to_vec(&cases.list)?)?;
    println!("{} native reference files", cases.list.len());
    Ok(())
}
