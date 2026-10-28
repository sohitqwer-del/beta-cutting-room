//! Browser module for the cutting room: cut plans, rolls, spreading, bundles,
//! tickets, marker reports and PLT import. It is loaded only when a cutting-room
//! screen needs it. Every route calls the same Rust as `beta-marker serve`.
use beta_marker_api::Output;
use beta_marker_core::{Job, MarkerResult};
use beta_marker_export::ExportOptions;
use serde::Deserialize;
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

/// One local API reply. JSON bodies use `application/json`.
#[wasm_bindgen]
pub struct ApiReply {
    status: u16,
    mime: String,
    extension: Option<String>,
    body: Vec<u8>,
}
#[wasm_bindgen]
impl ApiReply {
    #[wasm_bindgen(getter)]
    pub fn status(&self) -> u16 {
        self.status
    }
    #[wasm_bindgen(getter)]
    pub fn mime(&self) -> String {
        self.mime.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn extension(&self) -> Option<String> {
        self.extension.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn body(&self) -> Vec<u8> {
        self.body.clone()
    }
}
impl ApiReply {
    fn from_output(output: Output) -> Self {
        match output {
            Output::Json(body) => Self::bytes("application/json", None, body),
            Output::File {
                mime,
                extension,
                body,
            } => Self::bytes(mime, extension, body),
        }
    }
    fn bytes(mime: &str, extension: Option<&str>, body: Vec<u8>) -> Self {
        Self {
            status: 200,
            mime: mime.into(),
            extension: extension.map(Into::into),
            body,
        }
    }
    fn error(status: u16, message: &str) -> Self {
        Self {
            status,
            mime: "application/json".into(),
            extension: None,
            body: serde_json::json!({ "error": message })
                .to_string()
                .into_bytes(),
        }
    }
}

#[derive(Deserialize)]
struct MarkerRequest {
    job: Job,
    result: MarkerResult,
    #[serde(default)]
    options: ExportOptions,
}

fn query(url: &str) -> Result<(&str, BTreeMap<&str, &str>), ApiReply> {
    let (path, query) = url.split_once('?').unwrap_or((url, ""));
    let mut params = BTreeMap::new();
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let Some((key, value)) = pair.split_once('=') else {
            return Err(ApiReply::error(400, "Invalid query parameter"));
        };
        if params.insert(key, value).is_some() {
            return Err(ApiReply::error(400, "Duplicate query parameter"));
        }
    }
    Ok((path, params))
}

/// Answer one POST route, exactly as the native server does.
/// Covers roll and cutting-room routes, `import?format=plt`, `export` and `validate`.
#[wasm_bindgen]
pub fn api_request(url: &str, body: &[u8]) -> ApiReply {
    let (path, params) = match query(url) {
        Ok(parts) => parts,
        Err(reply) => return reply,
    };
    if let Some(reply) = beta_marker_api::cutroom(path, body) {
        return match reply {
            Ok(output) => ApiReply::from_output(output),
            Err(error) => ApiReply::error(400, &format!("{error:#}")),
        };
    }
    match path {
        "/api/import" => match params.get("format").copied().unwrap_or("dxf") {
            "plt" => match beta_marker_api::import_plt(body, params.get("width").copied()) {
                Ok(output) => ApiReply::from_output(output),
                Err(error) => ApiReply::error(400, &error),
            },
            _ => ApiReply::error(400, "This module imports PLT markers only"),
        },
        "/api/export" | "/api/validate" => {
            let input: MarkerRequest = match serde_json::from_slice(body) {
                Ok(input) => input,
                Err(error) => return ApiReply::error(400, &format!("Invalid JSON: {error}")),
            };
            if let Err(error) = beta_marker_api::ensure_counts(&input.job) {
                return ApiReply::error(400, &error);
            }
            if path == "/api/validate" {
                return match serde_json::to_vec(&beta_marker_core::validate(
                    &input.job,
                    &input.result,
                )) {
                    Ok(body) => ApiReply::bytes("application/json", None, body),
                    Err(error) => ApiReply::error(500, &error.to_string()),
                };
            }
            let format = params.get("format").copied().unwrap_or("svg");
            match beta_marker_export::export(format, &input.job, &input.result, &input.options) {
                Ok((bytes, mime, extension)) => ApiReply::bytes(mime, Some(extension), bytes),
                Err(error) => ApiReply::error(400, &format!("{error:#}")),
            }
        }
        _ => ApiReply::error(404, "Unknown route"),
    }
}

/// Browser engine for the planner: one thread, the same garment rules and checks
/// as the browser nester. The planner validates every returned marker again.
pub fn browser_nest(
    job: &Job,
    options: &beta_marker_nest::NestOptions,
) -> anyhow::Result<MarkerResult> {
    let mut options = options.clone();
    options.workers = Some(1);
    options.runs = 1;
    beta_marker_match::nest_with_rules_and_progress(job, &options, |_| {})
}

/// Compare measured width/ratio alternatives. Call in a Worker; progress is a checked partial plan.
#[wasm_bindgen]
pub fn compare_cost(request: &str, on_progress: &js_sys::Function) -> Result<String, JsValue> {
    let input: beta_marker_plan::cost::CostRequest =
        serde_json::from_str(request).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let input = beta_marker_api::prepare_cost_comparison(&input)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let result = beta_marker_plan::cost::compare_with_engine(
        &input,
        |partial| {
            let json = serde_json::to_string(partial)?;
            let answer = on_progress
                .call1(&JsValue::NULL, &JsValue::from_str(&json))
                .map_err(|_| anyhow::anyhow!("Comparison progress callback failed"))?;
            anyhow::ensure!(answer.as_bool() != Some(false), "Comparison stopped");
            Ok(())
        },
        &mut browser_nest,
    )
    .map_err(|e| JsValue::from_str(&format!("{e:#}")))?;
    serde_json::to_string(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Plan a cut order (the `/api/plan` body). Runs synchronously: call it in a
/// Worker. `on_progress` receives each checked partial plan as JSON; returning
/// `false` stops the search between markers. Returns the final plan as JSON.
#[wasm_bindgen]
pub fn plan(request: &str, on_progress: &js_sys::Function) -> Result<String, JsValue> {
    let input: beta_marker_api::PlanRequest = serde_json::from_str(request)
        .map_err(|e| JsValue::from_str(&format!("Invalid JSON: {e}")))?;
    let mut callback_error = None;
    let progress = |partial: &beta_marker_plan::CutPlan| -> anyhow::Result<()> {
        let json = serde_json::to_string(partial)?;
        match on_progress.call1(&JsValue::NULL, &JsValue::from_str(&json)) {
            Ok(keep) if keep.as_bool() == Some(false) => anyhow::bail!("Cut plan stopped."),
            Ok(_) => Ok(()),
            Err(error) => {
                callback_error = Some(error);
                anyhow::bail!("Progress callback failed.")
            }
        }
    };
    let plan = beta_marker_api::plan(&input, &mut browser_nest, progress);
    if let Some(error) = callback_error {
        return Err(error);
    }
    let plan = plan.map_err(|e| JsValue::from_str(&format!("{e:#}")))?;
    serde_json::to_string(&plan).map_err(|e| JsValue::from_str(&e.to_string()))
}
