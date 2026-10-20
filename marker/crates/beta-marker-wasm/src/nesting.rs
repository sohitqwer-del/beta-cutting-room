//! Worker-facing nesting session. The JS host yields between `step` calls.
use beta_marker_nest::browser::{BrowserOptions, BrowserSession};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct NestSession {
    inner: BrowserSession,
}

#[wasm_bindgen]
impl NestSession {
    #[wasm_bindgen(constructor)]
    pub fn new(job: &str, options: &str) -> Result<NestSession, String> {
        let job = super::read_job(job)?;
        let options: BrowserOptions = super::read(options)?;
        Ok(Self {
            inner: BrowserSession::new(job, options).map_err(|e| e.to_string())?,
        })
    }

    /// Run one synchronous round; returns a JSON progress record with a valid marker.
    pub fn step(&mut self) -> Result<String, String> {
        super::write(&self.inner.step().map_err(|e| e.to_string())?)
    }

    /// Set the stop flag. The next step returns the retained incumbent.
    pub fn cancel(&mut self) {
        self.inner.cancel();
    }
}

/// Synchronous low-level API. Use the TS Worker helper on a UI thread.
/// The callback receives a JSON progress record. Returning false stops the search.
#[wasm_bindgen]
pub fn nest(job: &str, options: &str, on_progress: &js_sys::Function) -> Result<String, JsValue> {
    let mut session = NestSession::new(job, options).map_err(|e| JsValue::from_str(&e))?;
    loop {
        let progress = session
            .inner
            .step()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let payload = super::write(&progress).map_err(|e| JsValue::from_str(&e))?;
        let keep_going = on_progress.call1(&JsValue::NULL, &JsValue::from_str(&payload))?;
        if progress.done {
            return super::write(&progress.result).map_err(|e| JsValue::from_str(&e));
        }
        if keep_going.as_bool() == Some(false) {
            session.cancel();
        }
    }
}
