//! Offline ASCII DXF-AAMA / ASTM pattern import and R12 export.
//!
//! Output coordinates are millimetres. See the crate README for sampling,
//! garment conventions, limits, and the opt-in quantity pairing rule.

mod curves;
mod export;
mod grading;
mod hpgl;
mod materials;
mod parser;
mod pattern;

use beta_marker_core::Piece;
use serde::{Deserialize, Serialize};

pub use export::export_aama;
pub use grading::{GradeMode, GradeTable, import_dxf_with_rules};
pub use hpgl::{HpglOptions, HpglReport, import_hpgl};
pub use materials::{MaterialKeywords, classify_materials, filter_material, split_materials};

/// Source drawing units. Output geometry is always in millimetres.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Units {
    /// Read $INSUNITS / AAMA "UNITS" text; fall back to a size heuristic and warn.
    #[default]
    Auto,
    /// Millimetres.
    Mm,
    /// Centimetres.
    Cm,
    /// Inches (25.4 mm exactly).
    Inch,
}

impl Units {
    fn scale(self) -> f64 {
        match self {
            Self::Inch => 25.4,
            Self::Cm => 10.0,
            Self::Auto | Self::Mm => 1.0,
        }
    }
}

/// Options for importing a drawing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportOptions {
    /// Source unit override.
    #[serde(default)]
    pub units: Units,
    /// Split each positive even Quantity N of an asymmetric piece into N/2
    /// normal and N/2 mirrored (left/right) copies. This is the default, since
    /// AAMA has no standard pair flag; the report records the assumption.
    /// Symmetric pieces keep same-hand copies. False keeps every Quantity N
    /// as same-hand copies.
    #[serde(default = "default_pairs")]
    pub pairs: bool,
}

fn default_pairs() -> bool {
    true
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            units: Units::Auto,
            pairs: true,
        }
    }
}

/// Imported pieces and the assumptions made while reading them.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImportReport {
    /// Pieces with usable cut boundaries.
    pub pieces: Vec<Piece>,
    /// Sizes found, in file order.
    pub sizes: Vec<String>,
    /// "aama", "astm" or "generic".
    pub dialect: String,
    /// Units the file was read in.
    pub units: Units,
    /// Recoverable omissions and assumptions.
    pub warnings: Vec<String>,
    /// True when at least one even Quantity was split into left/right copies.
    #[serde(default)]
    pub pairs_assumed: bool,
}

/// Import pattern pieces from DXF bytes. All output coordinates are millimetres.
pub fn import_dxf(bytes: &[u8], opts: &ImportOptions) -> anyhow::Result<ImportReport> {
    pattern::import(parser::read(bytes)?, opts, None)
}

#[cfg(test)]
mod tests;
