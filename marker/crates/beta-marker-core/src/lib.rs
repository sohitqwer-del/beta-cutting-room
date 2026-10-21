//! BETA Marker core: the job/result data contract, garment rules, metrics and the
//! independent validator. Pure Rust, no I/O, compiles to WebAssembly.

pub mod geometry;
pub mod metrics;
pub mod model;
pub mod rules;
pub mod style;
#[cfg(test)]
mod style_tests;
pub mod validate;

pub use metrics::{Metrics, compute_metrics};
pub use model::*;
pub use rules::{AllowedRotations, Demand, allowed_rotations, check_job, demands};
pub use validate::validate;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
