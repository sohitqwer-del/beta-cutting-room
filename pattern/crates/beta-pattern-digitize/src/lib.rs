//! Offline paper-pattern digitizing. Images run down; exported pieces run up, in mm.
pub mod calibration;
pub mod geometry;
pub mod synthetic;
pub mod target;
pub mod vision;

pub use vision::{ScanOptions, ScanResult, scan, scan_file};
