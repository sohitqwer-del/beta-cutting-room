//! Offline marker output: SVG previews, HPGL plots, vector PDFs and cutting reports.
//! All output uses recomputed geometry and metrics, rather than result metadata.

use beta_marker_core::{Job, MarkerResult};
use serde::{Deserialize, Serialize};

mod dxf;
mod hpgl;
mod iso;
mod labels;
mod pdf;
mod scene;
mod sheets;
mod svg;
#[cfg(test)]
mod tests;
mod workbook;

pub use dxf::to_dxf;
pub use iso::{IsoProfile, to_iso};
pub use labels::{MarkerLabel, marker_labels};
pub use sheets::{LabelOptions, to_labels_pdf, to_mini_marker_pdf};
pub use workbook::{CutPlanTable, to_tables_xlsx, to_xlsx};

/// Drawing controls shared by all output formats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportOptions {
    /// Draw notches, drills, internal lines and pattern text.
    #[serde(default = "yes")]
    pub marks: bool,
    /// Write piece labels (name, size, zero-based copy, mirror state).
    #[serde(default = "yes")]
    pub labels: bool,
    /// PDF tile side in mm, including margins. `None` uses one page when possible.
    /// Both axes tile automatically if a page would exceed 5080 mm (200 inches).
    #[serde(default)]
    pub pdf_tile_mm: Option<f64>,
    /// HPGL only: turn the marker 90 degrees so length feeds along plotter y.
    #[serde(default)]
    pub rotate: bool,
}
fn yes() -> bool {
    true
}
impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            marks: true,
            labels: true,
            pdf_tile_mm: None,
            rotate: false,
        }
    }
}

/// Make an SVG preview with mm dimensions, upright text and size colours.
pub fn to_svg(job: &Job, result: &MarkerResult, opts: &ExportOptions) -> anyhow::Result<String> {
    svg::render(&scene::build(job, result, opts)?)
}
/// Make an absolute-coordinate HPGL / PLT file at 40 plotter units per mm.
pub fn to_hpgl(job: &Job, result: &MarkerResult, opts: &ExportOptions) -> anyhow::Result<String> {
    hpgl::render(&scene::build(job, result, opts)?, opts.rotate)
}
/// Make a vector PDF at 1:1 scale (72/25.4 points/mm). Print at actual size.
/// Pages tile in both axes above 5080 mm, with page numbers and alignment marks.
/// The base Helvetica font replaces characters outside printable ASCII with `?`.
pub fn to_pdf(job: &Job, result: &MarkerResult, opts: &ExportOptions) -> anyhow::Result<Vec<u8>> {
    pdf::render(&scene::build(job, result, opts)?, opts.pdf_tile_mm)
}
/// Plot/print export with clipped stripe centre guides. Cutter output is not accepted here.
/// The existing options contract remains unchanged for CLI callers.
pub fn export_with_stripe_guides(
    format: &str,
    job: &Job,
    result: &MarkerResult,
    opts: &ExportOptions,
) -> anyhow::Result<(Vec<u8>, &'static str, &'static str)> {
    anyhow::ensure!(
        matches!(
            format.to_ascii_lowercase().as_str(),
            "pdf" | "plt" | "hpgl" | "svg"
        ),
        "Stripe guide export supports PDF, PLT and SVG."
    );
    let mut guided = job.clone();
    guided
        .meta
        .insert("beta_stripe_guides".into(), serde_json::json!(true));
    export(format, &guided, result, opts)
}
/// Make a plain-text cutting report. Consumption is marker metres divided by
/// the total garments in the size ratio (one per distinct size without an order).
pub fn to_report(job: &Job, result: &MarkerResult) -> String {
    use std::collections::{BTreeMap, BTreeSet};
    use std::fmt::Write;
    let details = workbook::summary(job, result);
    let effective = beta_marker_core::rules::effective_job(job);
    let job = &effective;
    let m = beta_marker_core::compute_metrics(job, &result.placements);
    let mut out = format!(
        "Job: {}\nWidth: {:.3} mm\nLength: {:.3} mm\nEfficiency: {:.2}%\n",
        scene::single_line(&job.name),
        job.fabric.width,
        m.length,
        m.efficiency * 100.0
    );
    let pieces: BTreeMap<_, _> = job.pieces.iter().map(|p| (p.id.as_str(), p)).collect();
    let mut sizes = BTreeMap::<&str, usize>::new();
    let mut unknown = 0;
    for pl in &result.placements {
        if let Some(p) = pieces.get(pl.piece_id.as_str()) {
            *sizes
                .entry(p.size.as_deref().unwrap_or("(unsized)"))
                .or_default() += 1;
        } else {
            unknown += 1;
        }
    }
    for (size, count) in sizes {
        let _ = writeln!(out, "Size {}: {count} pieces", scene::single_line(size));
    }
    if unknown > 0 {
        let _ = writeln!(out, "Unknown piece placements: {unknown}");
    }
    let garments: u64 = match &job.order {
        Some(order) => order.sizes.values().map(|&n| u64::from(n)).sum(),
        None => job
            .pieces
            .iter()
            .filter(|p| p.quantity > 0 || p.mirror_quantity > 0)
            .map(|p| p.size.as_deref())
            .collect::<BTreeSet<_>>()
            .len() as u64,
    };
    let _ = writeln!(out, "Garments: {garments}");
    if garments > 0 {
        let _ = writeln!(
            out,
            "Consumption per garment: {:.6} m (size-ratio average)",
            m.length / 1000.0 / garments as f64
        );
    } else {
        out.push_str("Consumption per garment: unavailable (no garments ordered)\n");
    }
    for (key, value) in details.iter().skip(6) {
        let _ = writeln!(out, "{key}: {value}");
    }
    for pl in &result.placements {
        if let Some(p) = pieces.get(pl.piece_id.as_str()) {
            let _ = writeln!(
                out,
                "Piece {} | size {} | bundle {} | copy {}{}",
                scene::single_line(&p.name),
                p.size.as_deref().unwrap_or("unsized"),
                pl.bundle
                    .map(|b| b.to_string())
                    .unwrap_or_else(|| "unassigned".into()),
                pl.copy,
                p.block_margin
                    .map(|v| format!(" | block margin {v} mm"))
                    .unwrap_or_default()
            );
        }
    }
    out
}

/// Export bytes plus their HTTP content type and filename extension.
/// Format names are case-insensitive. `plt` also accepts `hpgl`.
/// ISO, label grids and workbook tables use defaults; use the typed `to_*`
/// functions when a factory profile, label grid or cut-plan table is needed.
/// This formats geometry; callers must validate before sending cutter output.
pub fn export(
    format: &str,
    job: &Job,
    result: &MarkerResult,
    options: &ExportOptions,
) -> anyhow::Result<(Vec<u8>, &'static str, &'static str)> {
    let (bytes, mime, extension) = match format.to_ascii_lowercase().as_str() {
        "svg" => (
            to_svg(job, result, options)?.into_bytes(),
            "image/svg+xml",
            "svg",
        ),
        "plt" | "hpgl" => (
            to_hpgl(job, result, options)?.into_bytes(),
            "application/vnd.hp-hpgl",
            "plt",
        ),
        "pdf" => (to_pdf(job, result, options)?, "application/pdf", "pdf"),
        "report" => (
            to_report(job, result).into_bytes(),
            "text/plain; charset=utf-8",
            "txt",
        ),
        "iso" => (
            to_iso(job, result, &IsoProfile::default())?.into_bytes(),
            "text/plain; charset=utf-8",
            "iso",
        ),
        "dxf" => (
            to_dxf(job, result, options)?.into_bytes(),
            "image/vnd.dxf",
            "dxf",
        ),
        "xlsx" => (
            to_xlsx(job, result, None)?,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            "xlsx",
        ),
        "labels" => (
            to_labels_pdf(job, result, &LabelOptions::default())?,
            "application/pdf",
            "pdf",
        ),
        "mini" => (to_mini_marker_pdf(job, result)?, "application/pdf", "pdf"),
        _ => anyhow::bail!(
            "Unknown export format '{format}'. Use svg, plt, pdf, report, iso, dxf, xlsx, labels or mini."
        ),
    };
    Ok((bytes, mime, extension))
}

mod quick;
pub use quick::{
    ChalkGuideOptions, Language, Paper, ProjectedPiece, ProjectorLayout, TiledMarkerOptions,
    chalk_guide_pdf, projector_layout, tiled_marker_pdf,
};
