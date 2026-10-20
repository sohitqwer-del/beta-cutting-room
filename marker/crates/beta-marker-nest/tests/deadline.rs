//! Public deadline and cancellation regressions. Large public-job runs are opt-in.
use anyhow::{Context, Result};
use beta_marker_core::{Fabric, JOB_FORMAT, Job, MarkerResult, Piece, Placement};
use beta_marker_nest::{NestOptions, nest, nest_with_cancellation};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

fn dense_job() -> Job {
    Job {
        format: JOB_FORMAT.into(),
        name: "bounded obstacle regression".into(),
        fabric: Fabric {
            width: 100.0,
            ..Default::default()
        },
        pieces: vec![Piece {
            id: "disc".into(),
            quantity: 114,
            outline: (0..96)
                .map(|i| {
                    let a = f64::from(i) * std::f64::consts::TAU / 96.0;
                    [4.0 + 4.0 * a.cos(), 4.0 + 4.0 * a.sin()]
                })
                .collect(),
            ..Default::default()
        }],
        ..Default::default()
    }
}

#[test]
fn fixed_construction_returns_complete_marker_on_tiny_budget() -> Result<()> {
    let job = dense_job();
    let fixed = Placement {
        piece_id: "disc".into(),
        locked: true,
        bundle: Some(0),
        ..Default::default()
    };
    let start = Instant::now();
    let result = nest(
        &job,
        &NestOptions {
            time_limit_s: 0.02,
            seed: Some(42),
            workers: Some(2),
            fixed: vec![fixed.clone()],
            ..Default::default()
        },
    )?;
    assert!(
        start.elapsed().as_secs_f64() < 3.0,
        "deadline overrun: {:?}",
        start.elapsed()
    );
    assert_eq!(result.placements.len(), 114);
    assert!(result.placements.contains(&fixed));
    assert!(beta_marker_core::validate(&job, &result).ok);
    Ok(())
}

#[test]
fn cancellation_retains_checked_incumbent_and_locked_identity() -> Result<()> {
    let mut job = dense_job();
    job.pieces[0].quantity = 16;
    let fixed = Placement {
        piece_id: "disc".into(),
        locked: true,
        bundle: Some(0),
        ..Default::default()
    };
    let cancel = AtomicBool::new(false);
    let start = Instant::now();
    let result = nest_with_cancellation(
        &job,
        &NestOptions {
            time_limit_s: 10.0,
            seed: Some(42),
            workers: Some(2),
            fixed: vec![fixed.clone()],
            ..Default::default()
        },
        &cancel,
        |_| {
            cancel.store(true, Ordering::Relaxed);
        },
    )?;
    assert!(start.elapsed().as_secs_f64() < 3.0);
    assert!(result.placements.contains(&fixed));
    assert!(result.warnings.iter().any(|w| w.contains("cancelled")));
    assert!(beta_marker_core::validate(&job, &result).ok);
    Ok(())
}

#[test]
fn cancellation_during_search_keeps_a_complete_incumbent() -> Result<()> {
    let job = dense_job();
    let cancel = AtomicBool::new(false);
    let searching = AtomicBool::new(false);
    let started = Instant::now();
    let result = std::thread::scope(|scope| {
        scope.spawn(|| {
            while !searching.load(Ordering::Acquire) && started.elapsed().as_secs_f64() < 2.0 {
                std::thread::yield_now();
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
            cancel.store(true, Ordering::Release);
        });
        nest_with_cancellation(
            &job,
            &NestOptions {
                time_limit_s: 10.0,
                seed: Some(42),
                workers: Some(2),
                ..Default::default()
            },
            &cancel,
            |_| searching.store(true, Ordering::Release),
        )
    })?;
    assert!(started.elapsed().as_secs_f64() < 3.0);
    assert_eq!(result.placements.len(), 114);
    assert!(beta_marker_core::validate(&job, &result).ok);
    let retained = nest_with_cancellation(
        &job,
        &NestOptions {
            time_limit_s: 10.0,
            seed: Some(42),
            workers: Some(2),
            warm_start: Some(result.clone()),
            ..Default::default()
        },
        &cancel,
        |_| {},
    )?;
    assert!(retained.length <= result.length);
    assert!(retained.warnings.iter().any(|w| w.contains("cancelled")));
    Ok(())
}

#[test]
#[ignore = "public QA-006 fixture; 1, 5, 15 and 60 second budgets"]
fn simon_qa006_fixed_budget_matrix() -> Result<()> {
    let root = std::path::PathBuf::from(std::env::var("BETA_DATA_DIR")?)
        .join("marker-bench/astra-fix/qa006");
    let job: Job = serde_json::from_reader(std::fs::File::open(root.join("factory-job.json"))?)?;
    let previous: MarkerResult =
        serde_json::from_reader(std::fs::File::open(root.join("locked-result.json"))?)?;
    assert_eq!(previous.placements.len(), 114);
    let fixed: Vec<_> = previous
        .placements
        .iter()
        .filter(|p| p.locked)
        .cloned()
        .collect();
    ensure_fixed(&fixed)?;
    for seconds in [1.0, 5.0, 15.0, 60.0] {
        let started = Instant::now();
        let result = nest(
            &job,
            &NestOptions {
                time_limit_s: seconds,
                seed: Some(42),
                workers: Some(2),
                fixed: fixed.clone(),
                ..Default::default()
            },
        )?;
        assert!(
            started.elapsed().as_secs_f64() <= seconds + 10.0,
            "budget {seconds}, elapsed {:?}",
            started.elapsed()
        );
        assert!(beta_marker_core::validate(&job, &result).ok);
        assert!(fixed.iter().all(|f| result.placements.contains(f)));
        eprintln!(
            "QA006 budget={seconds} wall={:.3} length={:.3} efficiency={:.4}",
            started.elapsed().as_secs_f64(),
            result.length,
            result.efficiency
        );
    }
    Ok(())
}

fn ensure_fixed(fixed: &[Placement]) -> Result<()> {
    fixed
        .first()
        .context("QA fixture must contain a locked placement")?;
    Ok(())
}
