//! A4 overview and sticker sheets. All page sizes are explicit millimetres.
use crate::{ExportOptions, pdf, scene, workbook};
use anyhow::ensure;
use beta_marker_core::{Job, MarkerResult};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Write};
const PT: f64 = 72.0 / 25.4;
/// A4 sticker grid. Defaults fill a 3 x 8 sheet with 10 mm outer margins and
/// 2 mm gutters. Match these dimensions to the stock used by the factory.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LabelOptions {
    /// Labels across an A4 portrait sheet.
    pub columns: u32,
    /// Labels down the sheet.
    pub rows: u32,
    /// Outer paper margin in mm.
    pub margin_mm: f64,
    /// Horizontal and vertical gutter in mm.
    pub gap_mm: f64,
}
impl Default for LabelOptions {
    fn default() -> Self {
        Self {
            columns: 3,
            rows: 8,
            margin_mm: 10.,
            gap_mm: 2.,
        }
    }
}
fn text(out: &mut String, x: f64, y: f64, width: f64, value: &str, font: f64) -> std::fmt::Result {
    let value = scene::ascii(value);
    let size = font.min(width * PT / (value.len().max(1) as f64 * 0.6));
    writeln!(
        out,
        "BT /F1 {size} Tf 1 0 0 1 {} {} Tm ({}) Tj ET",
        x * PT,
        y * PT,
        pdf::escaped(&value)
    )
}
/// Make one sticker per placement (copy and bundle numbers are zero-based).
/// This labels physical placements, not every layer cut under that placement.
pub fn to_labels_pdf(
    job: &Job,
    result: &MarkerResult,
    opts: &LabelOptions,
) -> anyhow::Result<Vec<u8>> {
    scene::build_without_guides(job, result, &ExportOptions::default())?;
    ensure!(
        opts.rows > 0 && opts.columns > 0 && opts.rows <= 100 && opts.columns <= 100,
        "Label rows and columns must be between 1 and 100."
    );
    ensure!(
        opts.margin_mm.is_finite()
            && opts.margin_mm >= 0.
            && opts.gap_mm.is_finite()
            && opts.gap_mm >= 0.,
        "Label margins and gutters must be finite and non-negative."
    );
    let w = (210. - 2. * opts.margin_mm - f64::from(opts.columns - 1) * opts.gap_mm)
        / f64::from(opts.columns);
    let h = (297. - 2. * opts.margin_mm - f64::from(opts.rows - 1) * opts.gap_mm)
        / f64::from(opts.rows);
    ensure!(w >= 30. && h >= 20., "Labels must be at least 30 x 20 mm.");
    let per_page = opts.rows as usize * opts.columns as usize;
    let pieces: HashMap<_, _> = job.pieces.iter().map(|p| (p.id.as_str(), p)).collect();
    let mut contents = Vec::new();
    for chunk in result.placements.chunks(per_page) {
        let mut out = String::from("0 0 0 RG 0.25 w\n");
        for (i, pl) in chunk.iter().enumerate() {
            let p = pieces[pl.piece_id.as_str()];
            let x = opts.margin_mm + (i % opts.columns as usize) as f64 * (w + opts.gap_mm);
            let y =
                297. - opts.margin_mm - (i / opts.columns as usize + 1) as f64 * (h + opts.gap_mm)
                    + opts.gap_mm;
            writeln!(out, "{} {} {} {} re S", x * PT, y * PT, w * PT, h * PT)?;
            let lines = [
                format!("Style: {}", p.style.as_deref().unwrap_or(&job.name)),
                format!("Size: {}", p.size.as_deref().unwrap_or("unsized")),
                format!(
                    "Bundle: {}",
                    pl.bundle
                        .map(|b| b.to_string())
                        .unwrap_or_else(|| "unassigned".into())
                ),
                format!("Piece: {}", p.name),
                format!(
                    "Copy: {}{}",
                    pl.copy,
                    if pl.mirrored { " (mirrored)" } else { "" }
                ),
            ];
            let capacity = ((w - 4.) * PT / (7.5 * 0.6)).floor() as usize;
            let wrapped: Vec<String> = lines
                .iter()
                .flat_map(|line| {
                    let ascii = scene::ascii(line);
                    ascii
                        .as_bytes()
                        .chunks(capacity.max(1))
                        .map(|chunk| String::from_utf8_lossy(chunk).into_owned())
                        .collect::<Vec<_>>()
                })
                .collect();
            ensure!(
                wrapped.len() as f64 * 3.2 <= h - 4.,
                "Label text does not fit at a readable size. Use fewer rows or columns for larger labels."
            );
            for (row, line) in wrapped.iter().enumerate() {
                text(
                    &mut out,
                    x + 2.,
                    y + h - 4. - row as f64 * 3.2,
                    w - 4.,
                    line,
                    7.5,
                )?;
            }
        }
        contents.push(out);
    }
    if contents.is_empty() {
        let mut out = String::new();
        text(&mut out, 10., 280., 190., "No placements to label.", 12.)?;
        contents.push(out);
    }
    pdf::paper_pages(&contents, 210., 297.)
}
/// Make a single A4 overview. The drawing is scaled to fit; this PDF is not a
/// cutting pattern. A summary table states recomputed dimensions and consumption.
pub fn to_mini_marker_pdf(job: &Job, result: &MarkerResult) -> anyhow::Result<Vec<u8>> {
    let mut scene = scene::build(
        job,
        result,
        &ExportOptions {
            labels: false,
            ..Default::default()
        },
    )?;
    scene.texts.retain(|t| !t.footer);
    let mut out = String::from("0 0 0 RG 0.4 w\n");
    text(
        &mut out,
        10.,
        282.,
        190.,
        "MINI MARKER - overview only",
        16.,
    )?;
    let summary = workbook::summary(job, result);
    for (row, (key, value)) in summary.iter().take(12).enumerate() {
        let y = 272. - row as f64 * 6.;
        text(&mut out, 10., y, 67., key, 9.)?;
        text(&mut out, 80., y, 120., value, 9.)?;
        writeln!(
            out,
            "{} {} m {} {} l S",
            10. * PT,
            (y - 2.) * PT,
            200. * PT,
            (y - 2.) * PT
        )?;
    }
    let scale = (190. / scene.length).min(165. / scene.width);
    let x = 10.;
    let y = 22. + (165. - scene.width * scale) / 2.;
    writeln!(
        out,
        "q\n{} {} {} {} re W n\n{scale} 0 0 {scale} {} {} cm\n{}\nQ",
        10. * PT,
        22. * PT,
        190. * PT,
        165. * PT,
        x * PT,
        y * PT,
        pdf::scene_content(&scene)?
    )?;
    text(
        &mut out,
        10.,
        14.,
        190.,
        &format!(
            "Scale 1:{:.2}. Print labels and full-size patterns separately.",
            1. / scale
        ),
        9.,
    )?;
    text(
        &mut out,
        10.,
        9.,
        190.,
        "Hatched: defects | Light grid: repeat | Band lines: shade | FOLD: do not cut fold",
        8.,
    )?;
    pdf::paper_pages(&[out], 210., 297.)
}
