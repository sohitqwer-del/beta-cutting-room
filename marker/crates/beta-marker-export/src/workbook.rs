//! Excel cutting reports. No planner dependency: optional lay data is a table.
use crate::{ExportOptions, scene};
use beta_marker_core::{Job, MarkerResult, MatchAxes, Nap, ShadeAxis};
use rust_xlsxwriter::{Color, Format, Workbook};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Optional cut-plan worksheet, independent of the planner's evolving types.
/// Cells are literal text, never formulas. Callers choose the columns and units.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CutPlanTable {
    /// Column labels, for example lay, size ratio, plies, length (mm), fabric (m).
    pub headers: Vec<String>,
    /// Rows must have exactly the same number of cells as the headers.
    pub rows: Vec<Vec<String>>,
}
pub(super) fn garments(job: &Job) -> u64 {
    match &job.order {
        Some(order) => order.sizes.values().map(|&n| u64::from(n)).sum(),
        None => job
            .pieces
            .iter()
            .filter(|p| p.quantity > 0 || p.mirror_quantity > 0)
            .map(|p| p.size.as_deref())
            .collect::<BTreeSet<_>>()
            .len() as u64,
    }
}
pub(super) fn summary(job: &Job, result: &MarkerResult) -> Vec<(String, String)> {
    let effective = beta_marker_core::rules::effective_job(job);
    let m = beta_marker_core::compute_metrics(&effective, &result.placements);
    let pieces: BTreeMap<_, _> = effective
        .pieces
        .iter()
        .map(|p| (p.id.as_str(), p))
        .collect();
    let count = garments(job);
    vec![
        ("Job".into(), scene::single_line(&job.name)),
        ("Width (mm)".into(), format!("{:.3}", job.fabric.width)),
        ("Length (mm)".into(), format!("{:.3}", m.length)),
        (
            "Efficiency (%)".into(),
            format!("{:.2}", m.efficiency * 100.),
        ),
        ("Garments".into(), count.to_string()),
        (
            "Consumption per garment (m)".into(),
            if count > 0 {
                format!("{:.6}", m.length / 1000. / count as f64)
            } else {
                "Unavailable".into()
            },
        ),
        ("Layout".into(), format!("{:?}", job.fabric.layout)),
        (
            "Nap".into(),
            match job.fabric.nap {
                Nap::None => "Either direction",
                Nap::OneWay => "One way",
                Nap::OneWayPerBundle => "One way per bundle",
            }
            .into(),
        ),
        (
            "Allow flip".into(),
            job.pieces
                .iter()
                .map(|p| {
                    format!(
                        "{}: {}",
                        p.id,
                        if beta_marker_core::rules::flip_allowed(job, p) {
                            "Yes"
                        } else {
                            "No"
                        }
                    )
                })
                .collect::<Vec<_>>()
                .join("; "),
        ),
        ("Piece gap (mm)".into(), job.spacing.piece_gap.to_string()),
        ("Edge gap (mm)".into(), job.spacing.edge_gap.to_string()),
        ("Tilt (deg)".into(), job.fabric.tilt_deg.to_string()),
        (
            "Shade bands".into(),
            job.fabric
                .shade
                .map(|s| {
                    format!(
                        "{}: {} mm bands",
                        if s.axis == ShadeAxis::Length {
                            "Along length"
                        } else {
                            "Across width"
                        },
                        s.band_mm
                    )
                })
                .unwrap_or_else(|| "None".into()),
        ),
        (
            "Repeat".into(),
            job.fabric
                .repeat
                .map(|r| {
                    format!(
                        "X period: {}; Y period: {}; origin: ({}, {}) mm; tolerance: {} mm",
                        r.period_x
                            .map(|v| format!("{v} mm"))
                            .unwrap_or_else(|| "none".into()),
                        r.period_y
                            .map(|v| format!("{v} mm"))
                            .unwrap_or_else(|| "none".into()),
                        r.offset_x,
                        r.offset_y,
                        if r.tolerance_mm == 0.0 {
                            1.0
                        } else {
                            r.tolerance_mm
                        }
                    )
                })
                .unwrap_or_else(|| "None".into()),
        ),
        (
            "Shrinkage (%)".into(),
            format!(
                "Length {}, width {}",
                job.fabric.shrinkage.length_pct, job.fabric.shrinkage.width_pct
            ),
        ),
        (
            "Match rules".into(),
            if job.match_rules.is_empty() {
                "None".into()
            } else {
                job.match_rules
                    .iter()
                    .map(|r| {
                        format!(
                            "{}/{} matches {} on {}; offset ({}, {}) mm",
                            r.a.piece_id,
                            r.a.point_id,
                            r.b.as_ref()
                                .map(|b| format!("{}/{}", b.piece_id, b.point_id))
                                .unwrap_or_else(|| "fabric repeat".into()),
                            match r.axes {
                                MatchAxes::X => "X",
                                MatchAxes::Y => "Y",
                                MatchAxes::Both => "X and Y",
                            },
                            r.offset[0],
                            r.offset[1]
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("; ")
            },
        ),
        ("Defects".into(), job.fabric.defects.len().to_string()),
        (
            "Block placements".into(),
            result
                .placements
                .iter()
                .filter(|pl| {
                    pieces
                        .get(pl.piece_id.as_str())
                        .is_some_and(|p| p.block_margin.is_some())
                })
                .count()
                .to_string(),
        ),
    ]
}
/// Build a workbook: marker summary, pieces by size/bundle, and an optional
/// generic cut-plan table. Metrics are recomputed from effective cut geometry.
pub fn to_xlsx(
    job: &Job,
    result: &MarkerResult,
    cut_plan: Option<&CutPlanTable>,
) -> anyhow::Result<Vec<u8>> {
    // Apply the same finite/known-piece checks as vector output before writing.
    scene::build_without_guides(job, result, &ExportOptions::default())?;
    let mut book = Workbook::new();
    let heading = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0x16324f))
        .set_font_color(Color::White);
    let wrap = Format::new().set_text_wrap();
    let sheet = book.add_worksheet();
    sheet.set_name("Marker summary")?;
    sheet.set_column_width(0, 32)?;
    sheet.set_column_width(1, 80)?;
    sheet.write_string_with_format(0, 0, "Marker summary", &heading)?;
    for (i, (key, value)) in summary(job, result).iter().enumerate() {
        let row = i as u32 + 1;
        sheet.write_string(row, 0, key)?;
        if (1..=5).contains(&i)
            && let Ok(number) = value.parse::<f64>()
        {
            sheet.write_number(row, 1, number)?;
        } else {
            sheet.write_string_with_format(row, 1, value, &wrap)?;
        }
    }
    let sheet = book.add_worksheet();
    sheet.set_name("Pieces")?;
    let headers = [
        "Size",
        "Bundle",
        "Piece",
        "Piece ID",
        "Placements",
        "Normal copies",
        "Mirrored copies",
        "Block margin (mm)",
    ];
    for (col, h) in headers.iter().enumerate() {
        sheet.write_string_with_format(0, col as u16, *h, &heading)?;
        sheet.set_column_width(col as u16, 22)?;
    }
    sheet.set_freeze_panes(1, 0)?;
    let pieces: BTreeMap<_, _> = job.pieces.iter().map(|p| (p.id.as_str(), p)).collect();
    let mut counts = BTreeMap::<(Option<&str>, Option<u32>, &str), (u64, u64, u64)>::new();
    for pl in &result.placements {
        if let Some(p) = pieces.get(pl.piece_id.as_str()) {
            let entry = counts
                .entry((p.size.as_deref(), pl.bundle, p.id.as_str()))
                .or_default();
            let (normal, mirror) = beta_marker_core::rules::layer_copies(job.fabric.layout, pl);
            entry.0 += 1;
            entry.1 += u64::from(normal);
            entry.2 += u64::from(mirror);
        }
    }
    for (i, ((size, bundle, id), (placements, normal, mirror))) in counts.iter().enumerate() {
        let row = u32::try_from(i + 1)?;
        let p = pieces[id];
        sheet.write_string(row, 0, size.unwrap_or("unsized"))?;
        if let Some(b) = bundle {
            sheet.write_number(row, 1, *b)?;
        } else {
            sheet.write_string(row, 1, "Unassigned")?;
        }
        sheet.write_string(row, 2, &p.name)?;
        sheet.write_string(row, 3, *id)?;
        sheet.write_number(row, 4, *placements as f64)?;
        sheet.write_number(row, 5, *normal as f64)?;
        sheet.write_number(row, 6, *mirror as f64)?;
        if let Some(m) = p.block_margin {
            sheet.write_number(row, 7, m)?;
        }
    }
    if let Some(table) = cut_plan {
        anyhow::ensure!(
            !table.headers.is_empty()
                && table.headers.len() <= 16_384
                && table.rows.len() < 1_048_576,
            "Cut plan table exceeds worksheet limits or has no columns."
        );
        let sheet = book.add_worksheet();
        sheet.set_name("Cut plan")?;
        for (col, h) in table.headers.iter().enumerate() {
            sheet.write_string_with_format(0, col as u16, h, &heading)?;
            sheet.set_column_width(col as u16, 24)?;
        }
        for (i, row) in table.rows.iter().enumerate() {
            anyhow::ensure!(
                row.len() == table.headers.len(),
                "Cut plan row {} has the wrong number of cells.",
                i + 1
            );
            for (col, cell) in row.iter().enumerate() {
                sheet.write_string(i as u32 + 1, col as u16, cell)?;
            }
        }
        sheet.set_freeze_panes(1, 0)?;
    }
    Ok(book.save_to_buffer()?)
}

/// Write literal, unit-labelled tables without implying that one marker represents
/// an entire order. Worksheet names follow Excel's ordinary naming rules.
pub fn to_tables_xlsx(tables: &[(&str, CutPlanTable)]) -> anyhow::Result<Vec<u8>> {
    anyhow::ensure!(!tables.is_empty(), "At least one report table is required.");
    let mut book = Workbook::new();
    let heading = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0x16324f))
        .set_font_color(Color::White);
    for (name, table) in tables {
        anyhow::ensure!(
            !table.headers.is_empty()
                && table.headers.len() <= 16_384
                && table.rows.len() < 1_048_576,
            "Report table exceeds worksheet limits."
        );
        let sheet = book.add_worksheet();
        sheet.set_name(*name)?;
        for (c, h) in table.headers.iter().enumerate() {
            sheet.set_column_width(c as u16, 26)?;
            sheet.write_string_with_format(0, c as u16, h, &heading)?;
        }
        for (r, row) in table.rows.iter().enumerate() {
            anyhow::ensure!(
                row.len() == table.headers.len(),
                "Report row has the wrong number of cells."
            );
            for (c, cell) in row.iter().enumerate() {
                sheet.write_string(r as u32 + 1, c as u16, cell)?;
            }
        }
        sheet.set_freeze_panes(1, 0)?;
    }
    Ok(book.save_to_buffer()?)
}
