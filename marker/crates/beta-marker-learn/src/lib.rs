//! Offline estimates of Lectra's achieved efficiency, not packing bounds.
//!
//! Features and inference run entirely in Rust. The optional training tools use
//! Python only offline. No estimator can replace an independently checked layout.
mod features;
mod model;
pub use features::{
    Features, PieceSummary, Shape, TaskSummary, feature_names, features_from_job, summarize,
    summarize_rotations,
};
pub use model::{Estimate, Model};
