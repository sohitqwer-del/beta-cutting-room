//! Offline parametric pattern drafting. Millimetres, y up, angles in degrees.
#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
pub mod model;
pub use model::*;
pub mod catalog;
pub mod editor;
pub mod evaluate;
pub mod exchange;
pub mod formula;
pub mod geometry;
pub mod grading;
pub mod history;
pub mod measurements;
pub mod print;
pub mod production;
pub mod rule_exchange;
pub mod seamly;
pub mod starters;
pub mod symmetry;
pub mod tools;
