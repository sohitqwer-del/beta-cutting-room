//! End-to-end synthetic factory example. Does not change the main CLI.
use anyhow::{Context, Result};
use beta_marker_core::{Fabric, JOB_FORMAT, Job, Piece};
use beta_marker_cutroom::bundles::{BundleRules, CutPart, Lay, PlyRun, bundle_lay};
use beta_marker_cutroom::recuts::{Damage, schedule_recuts};
use beta_marker_cutroom::reports::{
    LayActuals, SpreadingRecord, lay_report, report_text, reports_csv,
};
use beta_marker_cutroom::tickets::{TicketPaper, sticker_pdf, ticket_pdf};
use std::{fs, path::PathBuf};
fn main() -> Result<()> {
    let output = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .context("usage: demo <output-directory>")?;
    let lay = Lay {
        id: "L1".into(),
        style: "TEE".into(),
        order: "O1".into(),
        colour: "Blue".into(),
        plies: vec![
            PlyRun {
                first: 1,
                last: 30,
                shade_group: "A".into(),
            },
            PlyRun {
                first: 31,
                last: 60,
                shade_group: "B".into(),
            },
        ],
        parts: [("front", 0), ("back", 0), ("front", 1), ("back", 1)]
            .map(|(part, garment)| CutPart {
                piece_id: part.into(),
                size: "M".into(),
                garment,
                slot: 0,
                mirrored: false,
            })
            .to_vec(),
    };
    let bundles = bundle_lay(&lay, BundleRules::default())?;
    let source = Job {
        format: JOB_FORMAT.into(),
        name: "synthetic tee".into(),
        fabric: Fabric {
            width: 1500.0,
            ..Default::default()
        },
        pieces: ["front", "back"]
            .map(|id| Piece {
                id: id.into(),
                size: Some("M".into()),
                quantity: 1,
                outline: vec![[0.0, 0.0], [500.0, 0.0], [500.0, 450.0], [0.0, 450.0]],
                ..Default::default()
            })
            .to_vec(),
        ..Default::default()
    };
    let mut ledger = vec![Damage::record(
        "D1".into(),
        &bundles[0],
        bundles[0].stickers[0].piece.clone(),
        "knife damage".into(),
    )?];
    let recuts = schedule_recuts(&source, &bundles, &mut ledger, "R1")?;
    let report = lay_report(&LayActuals {
        lay: lay.id.clone(),
        order: lay.order.clone(),
        width_mm: 1500.0,
        marker_length_mm: 1000.0,
        plies: 60,
        end_allowance_per_ply_mm: 20.0,
        piece_area_per_ply_mm2: 900_000.0,
        pieces_per_ply: 4,
        actual_piece_area_mm2: 54_000_000.0,
        actual_pieces: 240,
        recut_count: 1,
        spreading: vec![SpreadingRecord {
            issued_mm: 62500.0,
            end_bits_mm: 1000.0,
        }],
    })?;
    // Render all PDF bytes before writing any file, so validation errors leave no partial print job.
    let a4 = ticket_pdf(&bundles, TicketPaper::A4)?;
    let labels = ticket_pdf(&bundles, TicketPaper::Label100x50)?;
    let stickers = sticker_pdf(&bundles[..1], TicketPaper::Label100x50)?;
    fs::create_dir_all(&output)?;
    for (name, data) in [
        ("tickets-a4.pdf", a4),
        ("tickets-label.pdf", labels),
        ("stickers.pdf", stickers),
    ] {
        fs::write(output.join(name), data)?;
    }
    for (name, data) in [
        ("bundles.json", serde_json::to_vec_pretty(&bundles)?),
        ("damage-ledger.json", serde_json::to_vec_pretty(&ledger)?),
        ("recut-jobs.json", serde_json::to_vec_pretty(&recuts)?),
    ] {
        fs::write(output.join(name), data)?;
    }
    fs::write(
        output.join("report.csv"),
        reports_csv(std::slice::from_ref(&report)),
    )?;
    fs::write(output.join("report.txt"), report_text(&report))?;
    println!(
        "Created {} bundles ({} pieces); {} re-cut job.\n{}",
        bundles.len(),
        bundles.iter().map(|b| b.stickers.len()).sum::<usize>(),
        recuts.len(),
        report_text(&report)
    );
    Ok(())
}
