//! BETA Marker core: the job/result data contract, garment rules, metrics and the
//! independent validator. Pure Rust, no I/O, compiles to WebAssembly.

pub mod geometry;
pub mod metrics;
pub mod model;
pub mod rules;
pub mod validate;

pub use metrics::{compute_metrics, Metrics};
pub use model::*;
pub use rules::{allowed_rotations, check_job, demands, AllowedRotations, Demand};
pub use validate::validate;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
