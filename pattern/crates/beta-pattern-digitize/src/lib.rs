//! Offline paper-pattern digitizing. Images run down; exported pieces run up, in mm.
pub mod calibration;
pub mod geometry;
pub mod synthetic;
pub mod target;
pub mod vision;

pub use vision::{ScanOptions, ScanResult, scan, scan_file};

pub mod assist;
pub mod quick;

/// Native timings; browsers measure the synchronous call with performance.now().
/// std::time::Instant is unsupported on wasm32-unknown-unknown.
pub(crate) struct Timer {
    #[cfg(not(target_arch = "wasm32"))]
    start: std::time::Instant,
}
impl Timer {
    fn start() -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            start: std::time::Instant::now(),
        }
    }
    fn seconds(&self) -> f64 {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.start.elapsed().as_secs_f64()
        }
        #[cfg(target_arch = "wasm32")]
        {
            0.
        }
    }
}
pub mod quick_synthetic;
