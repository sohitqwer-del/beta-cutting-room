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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImportOptions {
    /// Source unit override.
    #[serde(default)]
    pub units: Units,
    /// Split each positive even Quantity N into N/2 normal and N/2 mirrored
    /// copies. False preserves Quantity N because AAMA has no standard pair flag.
    #[serde(default)]
    pub pairs: bool,
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
}

/// Import pattern pieces from DXF bytes. All output coordinates are millimetres.
pub fn import_dxf(bytes: &[u8], opts: &ImportOptions) -> anyhow::Result<ImportReport> {
    let mut report = pattern::import(parser::read(bytes)?, opts, None)?;
    if opts.pairs {
        let split = report
            .pieces
            .iter()
            .filter(|p| p.mirror_quantity > 0)
            .count();
        if split > 0 {
            report.warnings.push(format!(
                "Pairs: {split} pieces with an even Quantity N were split into N/2 normal and N/2 mirrored copies. Check left/right pieces before cutting."
            ));
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests;
