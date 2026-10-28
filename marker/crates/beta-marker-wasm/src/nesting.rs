//! Worker-facing nesting session. The JS host yields between `step` calls.
use beta_marker_match::browser::MatchingSession;
use beta_marker_nest::browser::{BrowserOptions, BrowserProgress, BrowserSession};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct NestSession {
    inner: Session,
}
enum Session {
    Plain(Box<BrowserSession>),
    Matching(Box<MatchingSession>),
}
impl Session {
    fn step(&mut self) -> Result<BrowserProgress, String> {
        match self {
            Self::Plain(s) => s.step(),
            Self::Matching(s) => s.step(),
        }
        .map_err(|e| e.to_string())
    }
    fn cancel(&mut self) {
        match self {
            Self::Plain(s) => s.cancel(),
            Self::Matching(s) => s.cancel(),
        }
    }
}

#[wasm_bindgen]
impl NestSession {
    #[wasm_bindgen(constructor)]
    pub fn new(job: &str, options: &str) -> Result<NestSession, String> {
        let job = super::read_job(job)?;
        let options: BrowserOptions = super::read(options)?;
        Ok(Self {
            inner: if beta_marker_match::needs_rules(&job) {
                Session::Matching(Box::new(
                    MatchingSession::new(job, options).map_err(|e| e.to_string())?,
                ))
            } else {
                Session::Plain(Box::new(
                    BrowserSession::new(job, options).map_err(|e| e.to_string())?,
                ))
            },
        })
    }

    /// Run one synchronous round; returns a JSON progress record with a valid marker.
    pub fn step(&mut self) -> Result<String, String> {
        super::write(&self.inner.step().map_err(|e| e.to_string())?)
    }

    /// Within-round callbacks are sent directly from the Worker while Rust runs.
    pub fn step_with_progress(&mut self, callback: &js_sys::Function) -> Result<String, JsValue> {
        let mut callback_error = None;
        let mut emit = |progress: BrowserProgress| {
            if callback_error.is_some() {
                return;
            }
            match super::write(&progress) {
                Ok(json) => {
                    if let Err(error) = callback.call1(&JsValue::NULL, &JsValue::from_str(&json)) {
                        callback_error = Some(error);
                    }
                }
                Err(error) => callback_error = Some(JsValue::from_str(&error)),
            }
        };
        let progress = match &mut self.inner {
            Session::Plain(session) => session.step_with_progress(&mut emit),
            Session::Matching(session) => session.step_with_progress(&mut emit),
        }
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
        if let Some(error) = callback_error {
            return Err(error);
        }
        super::write(&progress).map_err(|e| JsValue::from_str(&e))
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
