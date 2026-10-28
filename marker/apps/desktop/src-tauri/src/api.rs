//! Direct native adapters matching the loopback API. No HTTP server is linked.
#[path = "../../../../../pattern/crates/beta-pattern-wasm/src/editor.rs"]
mod pattern_editor;
#[path = "../../../../../pattern/crates/beta-pattern-wasm/src/operations.rs"]
mod pattern_operations;
#[path = "../../../../../pattern/crates/beta-pattern-wasm/src/output.rs"]
mod pattern_output;
use anyhow::Context;
use beta_marker_core::{Job, MarkerResult};
use beta_marker_dxf::{ImportOptions, Units};
use beta_marker_export::ExportOptions;
use beta_marker_nest::NestOptions;
use serde::{Deserialize, Serialize};
use serde_json::json;
const MAX_BODY: usize = 50 * 1024 * 1024;
const PHOTO_BODY: usize = 30 * 1024 * 1024;
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Reply {
    pub status: u16,
    pub mime: String,
    pub extension: Option<String>,
    pub body: Vec<u8>,
}
impl Reply {
    fn bytes(mime: &str, body: Vec<u8>) -> Self {
        Self {
            status: 200,
            mime: mime.into(),
            extension: None,
            body,
        }
    }
    pub(crate) fn json(value: impl Serialize) -> Self {
        match serde_json::to_vec(&value) {
            Ok(body) => Self::bytes("application/json", body),
            Err(e) => Self::error(500, &e.to_string()),
        }
    }
    fn from_output(output: beta_marker_api::Output) -> Self {
        match output {
            beta_marker_api::Output::Json(body) => Self::bytes("application/json", body),
            beta_marker_api::Output::File {
                mime,
                extension,
                body,
            } => Self {
                extension: extension.map(Into::into),
                ..Self::bytes(mime, body)
            },
        }
    }
    pub(crate) fn error(status: u16, message: &str) -> Self {
        let mut r = Self::json(json!({"error":message}));
        r.status = status;
        r
    }
}
#[derive(PartialEq, Eq)]
pub(crate) enum Method {
    Get,
    Post,
}
#[derive(Deserialize)]
struct MarkerRequest {
    job: Job,
    result: MarkerResult,
}
#[derive(Deserialize)]
pub(crate) struct NestRequest {
    pub job: Job,
    pub options: NestOptions,
    #[serde(default)]
    pub adaptive: bool,
}
pub(crate) use beta_marker_api::PlanRequest;
#[derive(Deserialize)]
struct ExportRequest {
    job: Job,
    result: MarkerResult,
    #[serde(default)]
    options: ExportOptions,
}
pub(crate) enum Route {
    Reply(Reply),
    Nest(Box<NestRequest>),
    Plan(Box<PlanRequest>),
    Status,
}
impl From<Reply> for Route {
    fn from(reply: Reply) -> Self {
        Self::Reply(reply)
    }
}

fn parse<T: serde::de::DeserializeOwned>(body: &[u8]) -> Result<T, Reply> {
    serde_json::from_slice(body)
        .map_err(|error| Reply::error(400, &format!("Invalid JSON: {error}")))
}

fn ensure_counts(job: &Job) -> Result<(), Reply> {
    for piece in &job.pieces {
        let multiplier = beta_marker_core::rules::multiplier(job, piece);
        if piece.quantity.checked_mul(multiplier).is_none()
            || piece.mirror_quantity.checked_mul(multiplier).is_none()
        {
            return Err(Reply::error(400, "Required copy count exceeds u32"));
        }
    }
    Ok(())
}

#[derive(Deserialize)]
struct PatternRequest {
    document: beta_pattern_core::Document,
    #[serde(default)]
    sizes: Vec<String>,
    #[serde(default = "pattern_tolerance")]
    tolerance_mm: f64,
    #[serde(default)]
    fabric: beta_marker_core::Fabric,
    #[serde(default)]
    ratio: std::collections::BTreeMap<String, u32>,
}
fn pattern_tolerance() -> f64 {
    0.1
}
#[derive(Deserialize)]
struct SeamlyRequest {
    pattern: String,
    measurements: Option<String>,
    size: Option<String>,
}
fn cutting_route(path: &str, body: &[u8]) -> Option<anyhow::Result<Reply>> {
    // Roll, spreading and cutting-room routes are shared with desktop and WASM.
    if let Some(output) = beta_marker_api::cutroom(path, body) {
        return Some(output.map(Reply::from_output));
    }
    let known = [
        "/api/pattern/starter",
        "/api/pattern/catalog",
        "/api/pattern/inspect",
        "/api/pattern/formula",
        "/api/pattern/snap",
        "/api/pattern/image-scale",
        "/api/pattern/measurements-import",
        "/api/pattern/measurements-export",
        "/api/pattern/move-point",
        "/api/pattern/nest",
        "/api/pattern/tool",
        "/api/pattern/piece-action",
        "/api/pattern/export-pdf",
        "/api/pattern/export-plt",
        "/api/pattern/import",
        "/api/pattern/from-job",
        "/api/pattern/grade",
        "/api/pattern/export-dxf",
        "/api/pattern/to-job",
        "/api/digitize",
        "/api/digitize/target",
    ];
    if !known.contains(&path) {
        return None;
    }
    Some((|| -> anyhow::Result<Reply> {
        match path {
            "/api/pattern/catalog" => Ok(Reply::json(beta_pattern_core::tools::catalog())),
            "/api/pattern/inspect"
            | "/api/pattern/formula"
            | "/api/pattern/snap"
            | "/api/pattern/image-scale"
            | "/api/pattern/measurements-import"
            | "/api/pattern/measurements-export"
            | "/api/pattern/move-point"
            | "/api/pattern/nest"
            | "/api/pattern/tool"
            | "/api/pattern/piece-action"
            | "/api/pattern/export-pdf"
            | "/api/pattern/export-plt" => {
                let r: serde_json::Value = serde_json::from_slice(body)?;
                let document: beta_pattern_core::Document =
                    serde_json::from_value(r["document"].clone())?;
                let size = r["size"].as_str().unwrap_or(&document.base_size);
                let adapter_error = |s: String| anyhow::anyhow!(s);
                match path {
                    "/api/pattern/measurements-import" => Ok(Reply::json(
                        pattern_editor::measurements(
                            &document,
                            r["source"].as_str().context("Choose a measurement file")?,
                            r["format"].as_str().unwrap_or("xml"),
                            size,
                        )
                        .map_err(adapter_error)?,
                    )),
                    "/api/pattern/measurements-export" => Ok(Reply::bytes(
                        "text/csv",
                        pattern_editor::measurement_csv(&document)
                            .map_err(adapter_error)?
                            .into_bytes(),
                    )),
                    "/api/pattern/move-point" => {
                        Ok(Reply::json(beta_pattern_core::tools::move_point(
                            &document,
                            r["id"].as_str().context("Choose a point")?,
                            size,
                            serde_json::from_value(r["target"].clone())?,
                        )?))
                    }
                    "/api/pattern/nest" => Ok(Reply::json(beta_pattern_core::evaluate::nest(
                        &document,
                        &document.sizes,
                        r["reference"]
                            .as_str()
                            .context("Choose a reference point")?,
                        0.1,
                    )?)),
                    "/api/pattern/inspect" => Ok(Reply::json(
                        pattern_editor::inspect(&document, size).map_err(adapter_error)?,
                    )),
                    "/api/pattern/snap" => Ok(Reply::json(
                        pattern_editor::snap(
                            &document,
                            size,
                            pattern_editor::SnapSettings {
                                at: serde_json::from_value(r["at"].clone())?,
                                radius: r["radius"].as_f64().unwrap_or(8.),
                                grid: r["grid"].as_f64().unwrap_or(10.),
                                points: r["points"].as_bool().unwrap_or(true),
                                paths: r["paths"].as_bool().unwrap_or(true),
                                ignore: r["ignore"].as_str(),
                            },
                        )
                        .map_err(adapter_error)?,
                    )),
                    "/api/pattern/image-scale" => Ok(Reply::json(
                        json!({"scale":pattern_editor::image_scale(serde_json::from_value(r["a"].clone())?,serde_json::from_value(r["b"].clone())?,r["distance_mm"].as_f64().context("Enter a known distance")?).map_err(adapter_error)?}),
                    )),
                    "/api/pattern/formula" => Ok(Reply::json(
                        json!({"value":pattern_editor::check_formula(&document,size,r["expression"].as_str().context("Enter a formula")?).map_err(adapter_error)?}),
                    )),
                    "/api/pattern/tool" => {
                        let tool = r["tool"].as_str().context("Choose a tool")?;
                        let id = r["id"].as_str().context("Enter an object ID")?;
                        let mut params = r["parameters"]
                            .as_object()
                            .context("Enter tool parameters")?
                            .clone();
                        let next = if let Some(selection) = r["selection"].as_array() {
                            let selection: Vec<String> = selection
                                .iter()
                                .map(|s| {
                                    s.as_str()
                                        .map(str::to_owned)
                                        .context("Choose valid group objects")
                                })
                                .collect::<anyhow::Result<_>>()?;
                            params.remove("source");
                            beta_pattern_core::tools::transform_group(
                                &document, &selection, id, tool, params,
                            )?
                        } else {
                            beta_pattern_core::tools::apply(&document, tool, id, params)?
                        };
                        pattern_editor::inspect(&next, size).map_err(adapter_error)?;
                        Ok(Reply::json(json!({"document":next})))
                    }
                    "/api/pattern/piece-action" => Ok(Reply::json(
                        pattern_operations::apply(&document, &r).map_err(adapter_error)?,
                    )),
                    "/api/pattern/export-pdf" => Ok(Reply::bytes(
                        "application/pdf",
                        pattern_output::pdf(&document, r["paper"].as_str().unwrap_or("A4"))
                            .map_err(adapter_error)?,
                    )),
                    _ => Ok(Reply::bytes(
                        "application/vnd.hp-hpgl",
                        pattern_output::plt(&document)
                            .map_err(adapter_error)?
                            .into_bytes(),
                    )),
                }
            }
            "/api/pattern/starter" => Ok(Reply::json(beta_pattern_core::starters::bodice())),
            "/api/pattern/import" => {
                let input: SeamlyRequest = serde_json::from_slice(body)?;
                Ok(Reply::json(beta_pattern_core::seamly::import(
                    &input.pattern,
                    input.measurements.as_deref(),
                    input.size.as_deref().unwrap_or("M"),
                )?))
            }
            "/api/pattern/from-job" => {
                let job: Job = serde_json::from_slice(body)?;
                Ok(Reply::json(beta_pattern_core::starters::from_marker_job(
                    &job,
                )?))
            }
            "/api/pattern/grade" | "/api/pattern/export-dxf" | "/api/pattern/to-job" => {
                let input: PatternRequest = serde_json::from_slice(body)?;
                anyhow::ensure!(
                    input.tolerance_mm.is_finite() && (0.01..=2.0).contains(&input.tolerance_mm),
                    "Pattern tolerance must be 0.01 to 2 mm"
                );
                let sizes = if input.sizes.is_empty() {
                    &input.document.sizes
                } else {
                    &input.sizes
                };
                anyhow::ensure!(
                    !sizes.is_empty() && sizes.len() <= 100,
                    "Choose 1 to 100 sizes"
                );
                match path {
                    "/api/pattern/export-dxf" => Ok(Reply::bytes(
                        "application/dxf",
                        beta_pattern_core::exchange::export_dxf(
                            &input.document,
                            sizes,
                            input.tolerance_mm,
                        )?
                        .into_bytes(),
                    )),
                    "/api/pattern/to-job" => Ok(Reply::json(beta_pattern_core::exchange::to_job(
                        &input.document,
                        input.fabric,
                        input.ratio,
                        input.tolerance_mm,
                    )?)),
                    _ => {
                        let mut graded = std::collections::BTreeMap::new();
                        for size in sizes {
                            graded.insert(
                                size,
                                beta_pattern_core::evaluate::grade(
                                    &input.document,
                                    size,
                                    input.tolerance_mm,
                                )?,
                            );
                        }
                        Ok(Reply::json(json!({"sizes":graded})))
                    }
                }
            }
            "/api/digitize/target" => {
                let target = beta_pattern_digitize::target::Target::tiled(
                    beta_pattern_digitize::target::SheetSize::A3,
                    1,
                    1,
                    false,
                )?;
                Ok(Reply::bytes("application/pdf", target.pdf()?))
            }
            "/api/digitize" => {
                use base64::Engine as _;
                let mut reader =
                    image::ImageReader::new(std::io::Cursor::new(body)).with_guessed_format()?;
                let mut limits = image::Limits::default();
                limits.max_image_width = Some(16000);
                limits.max_image_height = Some(16000);
                limits.max_alloc = Some(256 * 1024 * 1024);
                reader.limits(limits);
                let image = reader.decode()?.to_luma8();
                let target = beta_pattern_digitize::target::Target::tiled(
                    beta_pattern_digitize::target::SheetSize::A3,
                    1,
                    1,
                    false,
                )?;
                let scan = beta_pattern_digitize::scan(&image, &target, &Default::default())?;
                let mut png = std::io::Cursor::new(Vec::new());
                scan.overlay.write_to(&mut png, image::ImageFormat::Png)?;
                let overlay = format!(
                    "data:image/png;base64,{}",
                    base64::engine::general_purpose::STANDARD.encode(png.into_inner())
                );
                Ok(Reply::json(
                    json!({"pieces":scan.pieces,"report":scan.report,"overlay_png":overlay}),
                ))
            }
            _ => anyhow::bail!("Unknown cutting-room route"),
        }
    })())
}

pub(crate) fn route(method: &Method, url: &str, body: &[u8]) -> Route {
    if body.len() > MAX_BODY {
        return Reply::error(413, "Request body exceeds 50 MB").into();
    }
    let (path, query) = url.split_once('?').unwrap_or((url, ""));
    let mut params = std::collections::BTreeMap::new();
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let Some((key, value)) = pair.split_once('=') else {
            return Reply::error(400, "Invalid query parameter").into();
        };
        if params.insert(key, value).is_some() {
            return Reply::error(400, "Duplicate query parameter").into();
        }
    }
    if method == &Method::Get && path == "/api/health" {
        return Reply::json(json!({"ok": true, "version": env!("CARGO_PKG_VERSION"), "core_version": beta_marker_core::VERSION})).into();
    }
    if method == &Method::Get && path == "/api/nest/status" {
        return Route::Status;
    }
    if path.starts_with("/api/") && method != &Method::Post {
        return Reply::error(405, "Use POST for this API route").into();
    }
    if method == &Method::Post {
        if path == "/api/digitize" && body.len() > PHOTO_BODY {
            return Reply::error(413, "Photo exceeds 30 MB").into();
        }
        if let Some(reply) = cutting_route(path, body) {
            return reply
                .unwrap_or_else(|e| Reply::error(400, &format!("{e:#}")))
                .into();
        }
    }
    match (method, path) {
        (&Method::Post, "/api/import") => {
            match params.get("format").copied().unwrap_or("dxf") {
                "dxf" => {}
                "plt" => {
                    return match beta_marker_api::import_plt(body, params.get("width").copied()) {
                        Ok(output) => Reply::from_output(output).into(),
                        Err(error) => Reply::error(400, &error).into(),
                    };
                }
                _ => return Reply::error(400, "Import format must be dxf or plt").into(),
            }
            let units = match params.get("units").copied().unwrap_or("auto") {
                "auto" => Units::Auto,
                "mm" => Units::Mm,
                "cm" => Units::Cm,
                "inch" => Units::Inch,
                _ => return Reply::error(400, "Units must be auto, mm, cm or inch").into(),
            };
            // Matches ImportOptions::default and WASM: left/right pairs unless pairs=false.
            let pairs = match params.get("pairs").copied().unwrap_or("true") {
                "true" => true,
                "false" => false,
                _ => return Reply::error(400, "Pairs must be true or false").into(),
            };
            // The importer splits an even Quantity N into N/2 normal + N/2 mirrored copies.
            match beta_marker_dxf::import_dxf(body, &ImportOptions { units, pairs }) {
                Ok(report) => Reply::json(json!({"report":report})).into(),
                Err(error) => Reply::error(400, &error.to_string()).into(),
            }
        }
        (&Method::Post, "/api/nest") => match parse::<NestRequest>(body) {
            Ok(input) => {
                if let Err(error) = ensure_counts(&input.job) {
                    return error.into();
                }
                if !input.options.time_limit_s.is_finite() || input.options.time_limit_s <= 0.0 {
                    return Reply::error(400, "Time limit must be positive and finite").into();
                }
                if input.options.workers == Some(0) {
                    return Reply::error(400, "Workers must be greater than zero").into();
                }
                let issues = beta_marker_core::check_job(&input.job);
                if !issues.is_empty() {
                    return Reply::error(
                        400,
                        &issues
                            .iter()
                            .map(|issue| issue.message.as_str())
                            .collect::<Vec<_>>()
                            .join("; "),
                    )
                    .into();
                }
                Route::Nest(Box::new(input))
            }
            Err(error) => error.into(),
        },
        (&Method::Post, "/api/plan") => match parse::<PlanRequest>(body) {
            Ok(input) => {
                if let Err(error) = beta_marker_api::check_plan(&input) {
                    return Reply::error(400, &error).into();
                }
                Route::Plan(Box::new(input))
            }
            Err(error) => error.into(),
        },
        (&Method::Post, "/api/validate") => match parse::<MarkerRequest>(body) {
            Ok(input) => match ensure_counts(&input.job) {
                Ok(()) => Reply::json(beta_marker_core::validate(&input.job, &input.result)).into(),
                Err(error) => error.into(),
            },
            Err(error) => error.into(),
        },
        (&Method::Post, "/api/export") => {
            let input = match parse::<ExportRequest>(body) {
                Ok(input) => input,
                Err(error) => return error.into(),
            };
            if let Err(error) = ensure_counts(&input.job) {
                return error.into();
            }
            let format = params.get("format").copied().unwrap_or("svg");
            match beta_marker_export::export(format, &input.job, &input.result, &input.options) {
                Ok((bytes, mime, extension)) => {
                    let mut reply = Reply::bytes(mime, bytes);
                    reply.extension = Some(extension.into());
                    reply.into()
                }
                Err(error) => Reply::error(400, &format!("{error:#}")).into(),
            }
        }
        _ => Reply::error(404, "Unknown route").into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    fn assert_wire(expected: &serde_json::Value, actual: &serde_json::Value) {
        match (expected, actual) {
            (serde_json::Value::Number(a), serde_json::Value::Number(b)) => {
                assert_eq!(a.as_f64(), b.as_f64())
            }
            (serde_json::Value::Array(a), serde_json::Value::Array(b)) => {
                assert_eq!(a.len(), b.len());
                for (a, b) in a.iter().zip(b) {
                    assert_wire(a, b);
                }
            }
            (serde_json::Value::Object(a), serde_json::Value::Object(b)) => {
                assert_eq!(a.keys().collect::<Vec<_>>(), b.keys().collect::<Vec<_>>());
                for (k, a) in a {
                    assert_wire(a, &b[k]);
                }
            }
            _ => assert_eq!(expected, actual),
        }
    }
    fn reply(method: Method, url: &str, body: &[u8]) -> Reply {
        match route(&method, url, body) {
            Route::Reply(reply) => reply,
            Route::Nest(_) => panic!("unexpected nest"),
            Route::Plan(_) => panic!("unexpected plan"),
            Route::Status => Reply::json(
                json!({"running":false,"elapsed_s":0.0,"best_length_mm":null,"best_efficiency":null}),
            ),
        }
    }
    #[test]
    fn bounds_native_request_bodies() {
        let bytes = vec![0; MAX_BODY + 1];
        assert_eq!(reply(Method::Post, "/api/validate", &bytes).status, 413);
    }
    #[test]
    fn routes_shared_contract_without_opening_a_socket() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../packages/marker-wasm/test/fixtures/contract.json"
        ))
        .unwrap();
        let body = fixture.to_string();
        let validation = reply(Method::Post, "/api/validate", body.as_bytes());
        assert_eq!(validation.status, 200);
        let report: beta_marker_core::ValidationReport =
            serde_json::from_slice(&validation.body).unwrap();
        assert_eq!(report.length, 21.0);
        let exported = reply(Method::Post, "/api/export?format=report", body.as_bytes());
        assert_eq!(exported.status, 200);
        assert_eq!(exported.mime, "text/plain; charset=utf-8");
        assert_eq!(exported.extension.as_deref(), Some("txt"));
        assert!(String::from_utf8_lossy(&exported.body).contains("Job:"));
        assert_eq!(
            reply(Method::Post, "/api/export?format=unknown", body.as_bytes()).status,
            400
        );
        let body = json!({"job":fixture["job"],"options":{"time_limit_s":1}}).to_string();
        assert!(matches!(
            route(&Method::Post, "/api/nest", body.as_bytes()),
            Route::Nest(_)
        ));
        let mut overflow = fixture;
        overflow["job"]["pieces"][0]["quantity"] = json!(u32::MAX);
        assert_eq!(
            reply(
                Method::Post,
                "/api/validate",
                overflow.to_string().as_bytes()
            )
            .status,
            400
        );
    }
    #[test]
    fn health_bad_input_and_missing_static() {
        let health = reply(Method::Get, "/api/health", b"");
        assert_eq!(health.status, 200);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&health.body).unwrap()["ok"],
            true
        );
        assert_eq!(reply(Method::Post, "/api/validate", b"{}").status, 400);
        assert_eq!(reply(Method::Post, "/api/nest", b"oops").status, 400);
        assert_eq!(reply(Method::Get, "/missing.js", b"").status, 404);
        assert_eq!(reply(Method::Get, "/api/nest", b"").status, 405);
        assert_eq!(
            reply(Method::Post, "/api/import?units=feet", b"").status,
            400
        );
        assert_eq!(
            reply(Method::Post, "/api/import?pairs=yes", b"").status,
            400
        );
        assert_eq!(
            reply(Method::Post, "/api/import?pairs=true&pairs=false", b"").status,
            400
        );
    }
    #[test]
    fn v02_routes_preserve_options_and_plan_contract() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../packages/marker-wasm/test/fixtures/contract.json"
        ))
        .unwrap();
        let v = &fixture["v02"];
        let body = json!({"job":v["job"],"options":v["options"]}).to_string();
        let Route::Nest(input) = route(&Method::Post, "/api/nest", body.as_bytes()) else {
            panic!("expected nesting work")
        };
        assert_eq!(input.options.runs, 3);
        assert_eq!(input.options.fixed[0].bundle, Some(0));
        assert!(input.options.fixed[0].on_fold);
        assert!(input.options.warm_start.is_some());
        assert_wire(
            &v["options"],
            &serde_json::to_value(&input.options).unwrap(),
        );
        let body = json!({"order":v["order"],"options":v["plan_options"]}).to_string();
        let Route::Plan(input) = route(&Method::Post, "/api/plan", body.as_bytes()) else {
            panic!("expected planning work")
        };
        assert_eq!(input.order.quantities["M"], 20);
        assert_eq!(input.options.end_allowance_mm, 10.0);
        assert_wire(&v["order"], &serde_json::to_value(&input.order).unwrap());
        assert_wire(
            &v["plan_options"],
            &serde_json::to_value(&input.options).unwrap(),
        );
        let plan: beta_marker_plan::CutPlan = serde_json::from_value(v["plan"].clone()).unwrap();
        assert_wire(&v["plan"], &serde_json::to_value(&plan).unwrap());
        assert_eq!(
            plan.lays[0].result.as_ref().unwrap().placements[0].bundle,
            Some(0)
        );
        let mut invalid: serde_json::Value = serde_json::from_str(&body).unwrap();
        invalid["order"]["max_plies"] = json!(0);
        assert_eq!(
            reply(Method::Post, "/api/plan", invalid.to_string().as_bytes()).status,
            400
        );
        assert_eq!(reply(Method::Post, "/api/plan", b"{}").status, 400);
        let square = b"IN;SP1;PU0,0;PD4000,0,4000,4000,0,4000,0,0;PU;SP0;";
        let plt = reply(Method::Post, "/api/import?format=plt&width=1500", square);
        assert_eq!(plt.status, 200);
        let plt: serde_json::Value = serde_json::from_slice(&plt.body).unwrap();
        assert_eq!(plt["job"]["pieces"].as_array().map(Vec::len), Some(1));
        assert_eq!(
            plt["result"]["placements"].as_array().map(Vec::len),
            Some(1)
        );
        assert_eq!(
            reply(Method::Post, "/api/import?format=plt&width=0", b"IN;").status,
            400
        );
        assert_eq!(
            reply(Method::Post, "/api/import?format=unknown", b"").status,
            400
        );
        let status = reply(Method::Get, "/api/nest/status", b"");
        let status: serde_json::Value = serde_json::from_slice(&status.body).unwrap();
        assert_eq!(status["running"], false);
        assert!(status["best_length_mm"].is_null());
        assert_eq!(status["elapsed_s"], 0.0);
        let imported = reply(
            Method::Post,
            "/api/import?format=dxf&width=1500",
            include_bytes!("../../../../testdata/dxf/shirt-aama-mm.dxf"),
        );
        assert_eq!(imported.status, 200);
        let imported: serde_json::Value = serde_json::from_slice(&imported.body).unwrap();
        assert!(imported["report"]["pieces"].as_array().unwrap().len() > 1);
        assert_eq!(
            crate::worker::progress_area(
                &serde_json::from_value::<Job>(fixture["job"].clone()).unwrap()
            ),
            Some(800.0)
        );
    }

    fn floor_fixture() -> serde_json::Value {
        serde_json::from_str(include_str!(
            "../../../../packages/marker-wasm/test/fixtures/cutting-room.json"
        ))
        .unwrap()
    }
    fn post(path: &str, body: serde_json::Value) -> Reply {
        reply(Method::Post, path, body.to_string().as_bytes())
    }
    #[test]
    fn pattern_editor_routes_validate_catalog_forms_and_physical_exports() {
        let doc = beta_pattern_core::starters::bodice();
        let catalog = post("/api/pattern/catalog", json!({}));
        assert_eq!(catalog.status, 200);
        assert!(
            serde_json::from_slice::<serde_json::Value>(&catalog.body)
                .unwrap()
                .as_array()
                .unwrap()
                .len()
                >= 35
        );
        let inspected = post("/api/pattern/inspect", json!({"document":doc,"size":"M"}));
        assert_eq!(inspected.status, 200);
        assert!(
            serde_json::from_slice::<serde_json::Value>(&inspected.body).unwrap()["objects"]
                .as_object()
                .unwrap()
                .len()
                > 20
        );
        assert_eq!(
            post(
                "/api/pattern/formula",
                json!({"document":doc,"size":"M","expression":"m.chest/4"})
            )
            .status,
            200
        );
        assert_eq!(
            post(
                "/api/pattern/formula",
                json!({"document":doc,"size":"M","expression":"m.missing"})
            )
            .status,
            400
        );
        let added = post(
            "/api/pattern/tool",
            json!({"document":doc,"tool":"absolute","id":"user_origin","parameters":{"x":0,"y":0}}),
        );
        assert_eq!(added.status, 200);
        let changed: serde_json::Value = serde_json::from_slice(&added.body).unwrap();
        assert!(changed["document"]["drawing"]["user_origin"].is_object());
        let pdf = post(
            "/api/pattern/export-pdf",
            json!({"document":doc,"paper":"A4"}),
        );
        assert_eq!(pdf.status, 200);
        assert_eq!(pdf.mime, "application/pdf");
        assert!(pdf.body.starts_with(b"%PDF"));
        let plt = post("/api/pattern/export-plt", json!({"document":doc}));
        assert_eq!(plt.status, 200);
        assert!(plt.body.starts_with(b"IN;"));
        let csv = post("/api/pattern/measurements-export", json!({"document":doc}));
        assert_eq!(csv.status, 200);
        assert_eq!(
            post(
                "/api/pattern/measurements-import",
                json!({"document":doc,"format":"csv","source":String::from_utf8(csv.body).unwrap()})
            )
            .status,
            200
        );
        assert_eq!(
            post(
                "/api/pattern/export-pdf",
                json!({"document":doc,"paper":"invalid"})
            )
            .status,
            400
        );
    }
    #[test]
    fn pattern_routes_grade_edit_import_and_export() {
        let document: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../packages/marker-wasm/test/fixtures/pattern.doc.json"
        ))
        .unwrap();
        let graded = post("/api/pattern/grade", json!({"document":document}));
        assert_eq!(graded.status, 200);
        let graded: serde_json::Value = serde_json::from_slice(&graded.body).unwrap();
        assert_eq!(graded["sizes"].as_object().unwrap().len(), 4);
        let mut edited = document.clone();
        edited["measurements"]["M"]["chest"] = json!(1000);
        let after = post("/api/pattern/grade", json!({"document":edited}));
        assert_eq!(after.status, 200);
        assert_ne!(
            graded,
            serde_json::from_slice::<serde_json::Value>(&after.body).unwrap()
        );
        let exported = post("/api/pattern/export-dxf", json!({"document":document}));
        assert_eq!(exported.status, 200);
        assert!(String::from_utf8_lossy(&exported.body).contains("BETA_PATTERN_RULES_BEGIN"));
        let job = post(
            "/api/pattern/to-job",
            json!({"document":document,"fabric":{"width":1500},"ratio":{"M":1}}),
        );
        assert_eq!(job.status, 200);
        let job: Job = serde_json::from_slice(&job.body).unwrap();
        assert!(!job.pieces.is_empty());
        assert_eq!(
            post("/api/pattern/to-job", json!({"document":document})).status,
            400
        );
        assert_eq!(
            post(
                "/api/pattern/grade",
                json!({"document":document,"tolerance_mm":0})
            )
            .status,
            400
        );
        let imported = post(
            "/api/pattern/import",
            json!({"pattern":include_str!("../../../../../pattern/crates/beta-pattern-core/tests/fixtures/basic.sm2d"),"measurements":include_str!("../../../../../pattern/crates/beta-pattern-core/tests/fixtures/basic.smis")}),
        );
        assert_eq!(imported.status, 200);
        assert_eq!(
            post("/api/pattern/import", json!({"pattern":"bad XML"})).status,
            400
        );
    }
    #[test]
    fn cutting_room_shared_fixture_routes_and_pdf_bytes() {
        let f = floor_fixture();
        let inspected = post("/api/roll/inspect", json!({"register":f["register"]}));
        assert_eq!(inspected.status, 200);
        let inspected: serde_json::Value = serde_json::from_slice(&inspected.body).unwrap();
        assert_eq!(inspected["inspection"]["rolls"][0]["score"]["passed"], true);
        assert_eq!(inspected["groups"][0]["roll_ids"][0], "R1");
        let allocated = post(
            "/api/roll/spread",
            json!({"register":f["register"],"lays":f["lays"]}),
        );
        assert_eq!(allocated.status, 200);
        let spread: beta_marker_roll::SpreadingPlan =
            serde_json::from_slice(&allocated.body).unwrap();
        assert!(spread.complete);
        assert_eq!(spread.total_used_m, 12.0);
        let from_spread = post(
            "/api/cutroom/from-spread",
            json!({"job":f["job"],"result":f["result"],"register":f["register"],"spread":spread,"lay_id":"LAY-01","order":"ORDER-01","colour":"Blue"}),
        );
        assert_eq!(
            from_spread.status,
            200,
            "{}",
            String::from_utf8_lossy(&from_spread.body)
        );
        let from_spread: serde_json::Value = serde_json::from_slice(&from_spread.body).unwrap();
        assert_eq!(
            from_spread["plies"][0]["shade_group"],
            r#"["LOT1","DYE1","A"]"#
        );

        let physical = post(
            "/api/cutroom/lay",
            json!({"job":f["job"],"result":f["result"],"id":"LAY-01","order":"ORDER-01","colour":"Blue","plies":f["lay"]["plies"]}),
        );
        assert_eq!(physical.status, 200);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&physical.body).unwrap(),
            f["lay"]
        );
        let bundled = post("/api/cutroom/bundles", json!({"lay":f["lay"]}));
        assert_eq!(bundled.status, 200);
        let bundles: serde_json::Value = serde_json::from_slice(&bundled.body).unwrap();
        assert_eq!(bundles[0]["stickers"].as_array().unwrap().len(), 2);
        for path in ["/api/cutroom/tickets", "/api/cutroom/stickers"] {
            for paper in ["a4", "label100x50"] {
                let pdf = post(path, json!({"bundles":bundles,"paper":paper}));
                assert_eq!(pdf.status, 200);
                assert!(pdf.body.starts_with(b"%PDF-1.4"));
            }
        }
        let damage = post(
            "/api/cutroom/recut",
            json!({"id":"DAMAGE1","bundle":bundles[0],"piece":bundles[0]["stickers"][0]["piece"],"reason":"Knife damage"}),
        );
        assert_eq!(damage.status, 200);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&damage.body).unwrap()["status"],
            "open"
        );
        let report = post("/api/cutroom/report", f["actuals"].clone());
        assert_eq!(report.status, 200);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&report.body).unwrap()["variance_mm"],
            0.0
        );
        let cost = post(
            "/api/cutroom/cost",
            json!({"job":f["job"],"garments":1,"price_per_m":100}),
        );
        assert_eq!(cost.status, 200);
        let mut bad = f["register"].clone();
        bad["rolls"][0]["measured_length_m"] = json!(-1);
        assert_eq!(
            post("/api/roll/inspect", json!({"register":bad})).status,
            400
        );
        for path in [
            "/api/roll/inspect",
            "/api/roll/spread",
            "/api/cutroom/bundles",
            "/api/cutroom/cost",
            "/api/cutroom/tickets",
            "/api/cutroom/recut",
            "/api/cutroom/report",
            "/api/cutroom/report-marker",
        ] {
            assert_eq!(post(path, json!({})).status, 400, "{path}");
        }
    }

    #[test]
    fn marker_pipeline_recomputes_end_allowance_and_physical_report() {
        let f = floor_fixture();
        let spread = post(
            "/api/roll/spread",
            json!({"register":f["register"],"markers":[{"id":"L1","job":f["job"],"result":f["result"],"plies":2,"end_allowance_mm":10}]}),
        );
        assert_eq!(
            spread.status,
            200,
            "{}",
            String::from_utf8_lossy(&spread.body)
        );
        let spread: beta_marker_roll::SpreadingPlan = serde_json::from_slice(&spread.body).unwrap();
        assert!((spread.total_used_m - 0.082).abs() < 1e-10);
        let source = json!({"job":f["job"],"result":f["result"],"id":"LAY-01","order":"ORDER-01","colour":"Blue","plies":f["lay"]["plies"]});
        let actuals = json!({"source":source,"issued_mm":50,"end_bits_mm":8,"actual_piece_area_mm2":800,"actual_pieces":2,"recut_count":0,"end_allowance_per_ply_mm":0});
        let report = post("/api/cutroom/report-marker", actuals.clone());
        assert_eq!(
            report.status,
            200,
            "{}",
            String::from_utf8_lossy(&report.body)
        );
        let report: serde_json::Value = serde_json::from_slice(&report.body).unwrap();
        assert_eq!(report["planned_pieces"], 2);
        assert_eq!(report["variance_mm"], 0.0);
        let mut folded = actuals;
        folded["source"]["job"]["fabric"]["layout"] = json!("folded");
        folded["actual_pieces"] = json!(4);
        folded["actual_piece_area_mm2"] = json!(1600);
        let report = post("/api/cutroom/report-marker", folded);
        assert_eq!(
            report.status,
            200,
            "{}",
            String::from_utf8_lossy(&report.body)
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&report.body).unwrap()["planned_pieces"],
            4
        );
    }
    #[test]
    fn all_marker_exports_return_real_bytes() {
        let f = floor_fixture();
        for format in [
            "svg", "plt", "pdf", "iso", "dxf", "xlsx", "labels", "mini", "report",
        ] {
            let output = post(&format!("/api/export?format={format}"), f.clone());
            assert_eq!(
                output.status,
                200,
                "{format}: {}",
                String::from_utf8_lossy(&output.body)
            );
            assert!(!output.body.is_empty());
            assert!(output.extension.is_some());
            if ["pdf", "labels", "mini"].contains(&format) {
                assert!(output.body.starts_with(b"%PDF"));
            }
            if format == "xlsx" {
                assert!(output.body.starts_with(b"PK"));
            }
        }
    }

    #[test]
    fn digitize_photo_returns_pieces_and_a_real_overlay_png() {
        let fixture = floor_fixture();
        let job: Job = serde_json::from_value(fixture["job"].clone()).unwrap();
        let target = beta_pattern_digitize::target::Target::tiled(
            beta_pattern_digitize::target::SheetSize::A3,
            1,
            1,
            false,
        )
        .unwrap();
        let options = beta_pattern_digitize::synthetic::PhotoOptions {
            dimensions: [1600, 1200],
            perspective: false,
            blur_px: 0.3,
            noise: 0.0,
            lighting: 0.0,
            k1: 0.0,
            ..Default::default()
        };
        let mut source = job.pieces[0].clone();
        source.outline = vec![[0.0, 0.0], [300.0, 0.0], [300.0, 160.0], [0.0, 160.0]];
        let photo =
            beta_pattern_digitize::synthetic::photograph(&target, &source, &options).unwrap();
        let mut bytes = std::io::Cursor::new(Vec::new());
        photo
            .image
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        let response = reply(Method::Post, "/api/digitize", bytes.get_ref());
        assert_eq!(
            response.status,
            200,
            "{}",
            String::from_utf8_lossy(&response.body)
        );
        let output: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
        assert!(!output["pieces"].as_array().unwrap().is_empty());
        assert!(
            output["overlay_png"]
                .as_str()
                .unwrap()
                .starts_with("data:image/png;base64,iVBOR")
        );
        if let Ok(dir) = std::env::var("BETA_UI_SMOKE_DIR") {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                Path::new(&dir).join("synthetic-a3-photo.png"),
                bytes.into_inner(),
            )
            .unwrap();
        }
    }
    #[test]
    fn digitize_target_and_upload_errors_are_bounded() {
        let target = post("/api/digitize/target", json!({}));
        assert_eq!(target.status, 200);
        assert!(target.body.starts_with(b"%PDF"));
        let invalid = reply(Method::Post, "/api/digitize", b"not an image");
        assert_eq!(invalid.status, 400);
        let oversized = reply(Method::Post, "/api/digitize", &vec![0; PHOTO_BODY + 1]);
        assert_eq!(oversized.status, 413);
        let f = floor_fixture();
        let mut invalid = f["result"].clone();
        invalid["placements"][0]["bundle"] = json!(null);
        assert_eq!(post("/api/cutroom/lay",json!({"job":f["job"],"result":invalid,"id":"L1","order":"O1","colour":"Blue","plies":f["lay"]["plies"]})).status,400);
    }
}
