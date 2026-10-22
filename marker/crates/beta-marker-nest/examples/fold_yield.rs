//! Measure one folded, tubular or open marker and print one JSON line.
//! Useful yield is recomputed here from the layer assignments, independent of
//! the result's own metric fields.
//!
//! Usage: fold_yield JOB SECONDS SEED browser|native [open|folded|tubular] [WIDTH_MM] [RESULT_OUT]
//! The browser mode runs the single-thread round session used by the web app
//! (rounds of up to 5 s); native runs the threaded nester.
use anyhow::{Context, Result, bail};
use beta_marker_core::geometry::area;
use beta_marker_core::{Job, Layout, MarkerResult};
use beta_marker_nest::browser::{BrowserOptions, BrowserSession};
use beta_marker_nest::{NestOptions, nest};
use std::time::Instant;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 5 {
        bail!(
            "usage: fold_yield JOB SECONDS SEED browser|native [open|folded|tubular] [WIDTH_MM] [RESULT_OUT]"
        );
    }
    let mut job: Job = serde_json::from_slice(&std::fs::read(&args[1]).context("job")?)?;
    let seconds: f64 = args[2].parse()?;
    let seed: u64 = args[3].parse()?;
    let mode = args[4].as_str();
    if let Some(layout) = args.get(5) {
        job.fabric.layout = match layout.as_str() {
            "open" => Layout::Open,
            "folded" => Layout::Folded,
            "tubular" => Layout::Tubular,
            other => bail!("unknown layout {other}"),
        };
    }
    if let Some(width) = args.get(6) {
        job.fabric.width = width.parse()?;
    }
    let started = Instant::now();
    let result: MarkerResult = match mode {
        "native" => nest(
            &job,
            &NestOptions {
                time_limit_s: seconds,
                seed: Some(seed),
                ..Default::default()
            },
        )?,
        "browser" => {
            let mut session = BrowserSession::new(
                job.clone(),
                BrowserOptions {
                    time_limit_s: seconds,
                    seed: Some(seed),
                    round_time_s: seconds.min(5.0),
                    ..Default::default()
                },
            )?;
            loop {
                let step = session.step()?;
                if step.done {
                    break step.result;
                }
            }
        }
        other => bail!("unknown mode {other}"),
    };
    let wall_s = started.elapsed().as_secs_f64();
    if let Some(out) = args.get(7) {
        std::fs::write(out, serde_json::to_vec(&result)?)?;
    }
    let report = beta_marker_core::validate(&job, &result);
    let layers = if job.fabric.layout == Layout::Open {
        1.0
    } else {
        2.0
    };
    let (mut useful_area, mut useful, mut physical, mut on_fold) = (0.0, 0u64, 0u64, 0usize);
    for pl in &result.placements {
        let piece = job
            .pieces
            .iter()
            .find(|p| p.id == pl.piece_id)
            .context("unknown piece")?;
        let cut = if job.fabric.layout == Layout::Open || pl.on_fold {
            1
        } else {
            2
        };
        let served = if pl.layer_allocations.is_empty() {
            cut
        } else {
            pl.layer_allocations.iter().flatten().count() as u64
        };
        on_fold += usize::from(pl.on_fold);
        physical += cut;
        useful += served;
        useful_area += area(&piece.outline) * served as f64;
    }
    let garments: u32 = job.order.as_ref().map_or(1, |o| o.sizes.values().sum());
    let fabric = layers * job.fabric.width * result.length;
    println!(
        "{}",
        serde_json::json!({
            "job": job.name, "mode": mode, "layout": job.fabric.layout, "width_mm": job.fabric.width,
            "seconds": seconds, "seed": seed, "valid": report.ok, "wall_s": wall_s,
            "length_m": result.length / 1000.0,
            "m_per_garment": result.length / 1000.0 / f64::from(garments),
            "occupied_pct": 100.0 * result.efficiency,
            "useful_pct": 100.0 * useful_area / fabric,
            "surplus": physical - useful,
            "placements": result.placements.len(),
            "on_fold": on_fold,
        })
    );
    Ok(())
}
