//! Thin adapters to the core's shared print/plot writers.
use beta_pattern_core::{
    Document,
    print::{self, Paper, PrintOptions},
};

/// Export all document sizes as HPGL at 40 units per millimetre.
pub fn plt(doc: &Document) -> Result<String, String> {
    print::plotter_plt(doc, &doc.sizes, 0.1).map_err(|e| format!("{e:#}"))
}
/// Export physical A4/A3 tiles, with 10 mm overlap and a separate calibration page.
pub fn pdf(doc: &Document, paper: &str) -> Result<Vec<u8>, String> {
    let paper = match paper {
        "A4" => Paper::A4,
        "A3" => Paper::A3,
        _ => return Err("Choose A4 or A3".into()),
    };
    print::tiled_pdf(
        doc,
        &doc.sizes,
        &PrintOptions {
            paper,
            ..Default::default()
        },
    )
    .map_err(|e| format!("{e:#}"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn physical_paper_and_plotter_units() {
        let doc = beta_pattern_core::starters::bodice();
        let bytes = pdf(&doc, "A4").unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.starts_with("%PDF-1.4"));
        assert!(text.contains("595.275591 841.889764"));
        assert!(text.contains("283.464567 283.464567 re S"));
        assert!(plt(&doc).unwrap().starts_with("IN;PA;"));
        assert!(pdf(&doc, "invalid").is_err());
    }
}
