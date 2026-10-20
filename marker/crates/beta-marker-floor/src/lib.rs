//! Offline floor workflows. All quantities, validation and state changes run in Rust.
//! The caller persists a successful response atomically; errors never change its input.

pub mod api;
pub mod floor;
pub mod intake;
pub mod sheets;

// Reuse the exact lightweight cutroom sources without pulling the native nesting
// engine into WASM. Move these to a shared contracts crate during integration.
#[path = "../../beta-marker-cutroom/src/bundles.rs"]
pub mod bundles;
/// Existing QR/ticket writer with the documented wasm32 integer-width shim.
#[path = "../../beta-marker-cutroom/src/tickets.rs"]
pub mod tickets;
#[path = "../../beta-marker-cutroom/src/integration.rs"]
pub mod integration;
#[path = "../../beta-marker-cutroom/src/recuts.rs"]
pub mod recuts;

use wasm_bindgen::prelude::*;

/// JSON command boundary shared by the standalone CLI and browser.
pub fn dispatch(input: &str) -> anyhow::Result<String> {
    anyhow::ensure!(input.len() <= 50_000_000, "Floor request exceeds 50 MB.");
    Ok(serde_json::to_string(&api::execute(
        serde_json::from_str(input)?,
    )?)?)
}

/// Run a typed command locally in the browser or desktop WebView.
#[wasm_bindgen]
pub fn floor_request(input: &str) -> Result<String, JsValue> {
    dispatch(input).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Preview a bounded CSV or XLSX file without changing saved factory records.
#[wasm_bindgen]
pub fn floor_preview(
    bytes: &[u8],
    format: &str,
    kind: &str,
    mapping: &str,
) -> Result<String, JsValue> {
    (|| -> anyhow::Result<String> {
        let mapping = serde_json::from_str(mapping)?;
        Ok(serde_json::to_string(&intake::preview(
            bytes,
            format,
            kind.parse()?,
            &mapping,
        )?)?)
    })()
    .map_err(|e| JsValue::from_str(&e.to_string()))
}

#[cfg(test)]
mod tests;
