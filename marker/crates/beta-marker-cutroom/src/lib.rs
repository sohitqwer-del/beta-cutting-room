//! Offline cutting-room operations. All lengths are millimetres and areas mm².
//! See the crate README for the numbering convention and scan payload contract.
#![warn(missing_docs)]

pub mod bundles;
pub mod costing;
pub mod integration;
pub mod recuts;
pub mod reports;
/// Costing, lay tickets, stock registers and reconciliation in CSV/XLSX/PDF.
pub mod production;
pub mod tickets;

#[cfg(test)]
mod tests;
