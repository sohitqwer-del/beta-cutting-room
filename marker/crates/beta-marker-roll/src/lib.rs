//! Fabric inspection, shade/shrinkage grouping and auditable spreading plans.
//! Roll distances use metres; widths and marker defect polygons use millimetres.

#![warn(missing_docs)]

mod audit;
mod demo;
pub use demo::demo_register;
mod defects;
mod inspection;
mod model;
mod spread;
mod tables;

pub use audit::*;
pub use defects::*;
pub use inspection::*;
pub use model::*;
pub use spread::*;

#[cfg(test)]
mod tests;
