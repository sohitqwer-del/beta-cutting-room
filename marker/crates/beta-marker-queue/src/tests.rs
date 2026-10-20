use super::*;
use beta_marker_core::{Fabric, JOB_FORMAT, Job, Order, Piece};
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct TestDir(PathBuf);
impl TestDir {
    fn new() -> Self {
        Self(
            PathBuf::from(std::env::var("CARGO_TARGET_DIR").expect("test build/data directory"))
                .join("queue-tests")
                .join(format!(
                    "{}-{}",
                    std::process::id(),
                    NEXT.fetch_add(1, Ordering::Relaxed)
                )),
        )
    }
}
impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn job() -> Job {
    Job {
        format: JOB_FORMAT.into(),
        name: "tiny".into(),
        fabric: Fabric {
            width: 100.0,
            ..Default::default()
        },
        pieces: vec![Piece {
            id: "front".into(),
            name: "Front".into(),
            quantity: 1,
            size: Some("M".into()),
            outline: vec![[0.0, 0.0], [40.0, 0.0], [40.0, 20.0], [0.0, 20.0]],
            ..Default::default()
        }],
        order: Some(Order {
            sizes: std::collections::BTreeMap::from([("M".into(), 1)]),
        }),
        ..Default::default()
    }
}
fn request(id: &str, priority: i32) -> Submission {
    Submission {
        id: id.into(),
        name: id.into(),
        priority,
        time: TimeTier::Overnight { seconds: 0.04 },
        task: Task::Nest {
            job: job(),
            options: Default::default(),
        },
    }
}

fn impossible_job() -> Job {
    let mut j = job();
    j.fabric.max_length = Some(50.0);
    j.pieces[0].quantity = 2;
    j.pieces[0].outline = vec![[0.0, 0.0], [40.0, 0.0], [40.0, 61.0], [0.0, 61.0]];
    j
}

#[test]
fn disk_priority_failure_isolation_and_restart() -> Result<()> {
    let dir = TestDir::new();
    let queue = Queue::open(&dir.0)?;
    queue.add(request("low", 0))?;
    queue.add(request("high", 5))?;
    let mut bad = request("broken", 10);
    if let Task::Nest { job, .. } = &mut bad.task {
        *job = impossible_job();
    }
    queue.add(bad)?;
    fs::write(queue.root.join("jobs/corrupt.json"), b"{")?;
    let summary = queue.run(
        RunOptions {
            slice_seconds: 0.02,
            ..Default::default()
        },
        |_| {},
    )?;
    assert_eq!(summary.failed, ["broken"]);
    assert_eq!(summary.completed, ["high", "low"]);
    assert_eq!(summary.unreadable.len(), 1);
    let reopened = Queue::open(&dir.0)?;
    assert_eq!(reopened.status("high")?.status, Status::Completed);
    let Some(Checkpoint::Marker(result)) = reopened.checkpoint("high")? else {
        panic!("marker checkpoint")
    };
    assert!(beta_marker_core::validate(&job(), &result).ok);
    Ok(())
}

#[test]
fn pause_cancel_resume_and_crash_recovery_keep_best() -> Result<()> {
    let dir = TestDir::new();
    let queue = Queue::open(&dir.0)?;
    queue.add(request("one", 0))?;
    let control = queue.clone();
    let mut paused = false;
    let result = queue.run(
        RunOptions {
            slice_seconds: 0.02,
            ..Default::default()
        },
        |event| {
            if event.checkpoint_saved && !paused {
                control.pause("one").expect("pause while worker runs");
                paused = true;
            }
        },
    )?;
    assert_eq!(result.interrupted, ["one"]);
    let first = queue.checkpoint("one")?.expect("saved before pause");
    // Simulate a crash between the geometry rename and pointer rename.
    queue.update("one", |r| {
        r.checkpoint = None;
        Ok(())
    })?;
    assert!(queue.checkpoint("one")?.is_some());
    queue.cancel("one")?;
    queue.resume("one")?;
    // A crashed process leaves Running on disk; OS locks are automatically released.
    queue.update("one", |r| {
        r.status = Status::Running;
        Ok(())
    })?;
    let result = Queue::open(&dir.0)?.run(
        RunOptions {
            slice_seconds: 0.02,
            ..Default::default()
        },
        |_| {},
    )?;
    assert_eq!(result.completed, ["one"]);
    assert_eq!(queue.status("one")?.attempts, 2);
    let Some(Checkpoint::Marker(last)) = queue.checkpoint("one")? else {
        panic!("marker")
    };
    let Checkpoint::Marker(first) = first else {
        panic!("marker")
    };
    assert!(last.length <= first.length);
    Ok(())
}

#[test]
fn second_runner_and_bad_ids_cannot_mutate_jobs() -> Result<()> {
    let dir = TestDir::new();
    let queue = Queue::open(&dir.0)?;
    for id in ["../escape", "CON", "NUL", "bad:name", ""] {
        assert!(queue.add(request(id, 0)).is_err());
    }
    queue.add(request("valid", 0))?;
    assert!(queue.add(request("valid", 2)).is_err());
    let runner = lock(&queue.root.join("runner.lock"))?;
    assert!(queue.run(Default::default(), |_| {}).is_err());
    drop(runner);
    queue.pause("valid")?;
    assert_eq!(queue.run(Default::default(), |_| {})?.completed.len(), 0);
    queue.resume("valid")?;
    queue.cancel("valid")?;
    assert!(queue.checkpoint("valid")?.is_none());
    Ok(())
}

#[test]
fn tiers_and_cpu_limit_are_explicit() -> Result<()> {
    assert_eq!(TimeTier::Draft.seconds(), 20.0);
    assert_eq!(TimeTier::Costing.seconds(), 60.0);
    assert_eq!(TimeTier::Production.seconds(), 300.0);
    let dir = TestDir::new();
    let queue = Queue::open(&dir.0)?;
    assert!(
        queue
            .run(
                RunOptions {
                    max_workers: 0,
                    ..Default::default()
                },
                |_| {}
            )
            .is_err()
    );
    let mut input = request("invalid-time", 0);
    input.time = TimeTier::Overnight { seconds: f64::NAN };
    assert!(queue.add(input).is_err());
    let mut numeric = request("invalid-optional-number", 0);
    if let Task::Nest { job, .. } = &mut numeric.task {
        job.fabric.max_length = Some(f64::NAN);
    }
    assert!(queue.add(numeric).is_err());
    Ok(())
}

#[test]
fn typed_plan_pipeline_and_width_trials_finish_with_checked_files() -> Result<()> {
    use beta_marker_plan::{CUT_ORDER_FORMAT, CutOrder, PlanOptions, cost::*};
    let dir = TestDir::new();
    let queue = Queue::open(&dir.0)?;
    let order = CutOrder {
        format: CUT_ORDER_FORMAT.into(),
        name: "tiny order".into(),
        job: job(),
        quantities: std::collections::BTreeMap::from([("M".into(), 4)]),
        max_plies: 4,
        max_garments_per_marker: 1,
        overcut_pct: 0.0,
    };
    let options = PlanOptions {
        time_per_marker_s: 0.01,
        final_time_s: 0.02,
        max_candidates: 1,
        max_refinements: 1,
        search_nodes: 100,
        ..Default::default()
    };
    let register = beta_marker_roll::RollRegister {
        rolls: vec![beta_marker_roll::Roll {
            id: "r1".into(),
            lot: "L".into(),
            dye_lot: "D".into(),
            shade_group: "A".into(),
            ticket_length_m: 10.0,
            measured_length_m: 10.0,
            usable_width_mm: 100.0,
            shrinkage: Default::default(),
            defects: vec![],
        }],
    };
    let mut plan = request("plan", 0);
    plan.time = TimeTier::Overnight { seconds: 0.02 };
    plan.task = Task::Plan {
        order: order.clone(),
        options: options.clone(),
    };
    queue.add(plan)?;
    let mut pipeline = request("pipeline", 0);
    pipeline.time = TimeTier::Overnight { seconds: 0.02 };
    pipeline.task = Task::Pipeline {
        order: order.clone(),
        options: options.clone(),
        rolls: register,
        eligible_roll_ids: vec![],
        colour: "Blue".into(),
        report: Some(ReportSettings {
            meta: Default::default(),
            gsm: 200.0,
            currency: "INR".into(),
            price_per_m: 50.0,
            lay_cost: 10.0,
            purchase_wastage_pct: 2.0,
        }),
    };
    queue.add(pipeline)?;
    let mut widths = request("widths", 0);
    widths.time = TimeTier::Overnight { seconds: 0.02 };
    widths.task = Task::WidthTrials {
        request: CostRequest {
            order,
            currency: "INR".into(),
            offers: vec![WidthOffer {
                id: "100mm".into(),
                width_mm: 100.0,
                price: FabricPrice::Metre { price: 50.0 },
                lay_cost: 10.0,
                purchase_wastage_pct: 2.0,
                end_bit_charge_fraction: 0.0,
                rolls: None,
                eligible_roll_ids: vec![],
            }],
            ratio_sets: vec![],
            options,
            finalists: 1,
            final_time_s: 0.02,
            allocation: Default::default(),
        },
    };
    queue.add(widths)?;
    let result = queue.run(
        RunOptions {
            slice_seconds: 0.02,
            ..Default::default()
        },
        |_| {},
    )?;
    assert!(result.failed.is_empty(), "{:?}", queue.list()?);
    assert_eq!(result.completed.len(), 3);
    assert!(queue.root.join("results/pipeline/tickets.pdf").exists());
    assert!(queue.root.join("results/pipeline/report.xlsx").exists());
    assert!(matches!(
        queue.checkpoint("widths")?,
        Some(Checkpoint::WidthTrials(_))
    ));
    assert!(!queue.marker_checkpoints("plan")?.is_empty());
    Ok(())
}

#[test]
fn running_cancel_keeps_geometry_and_other_jobs_continue() -> Result<()> {
    let dir = TestDir::new();
    let queue = Queue::open(&dir.0)?;
    queue.add(request("cancel-me", 10))?;
    queue.add(request("continue", 0))?;
    let control = queue.clone();
    let mut cancelled = false;
    let summary = queue.run(
        RunOptions {
            slice_seconds: 0.02,
            ..Default::default()
        },
        |e| {
            if e.job_id == "cancel-me" && e.checkpoint_saved && !cancelled {
                control.cancel("cancel-me").expect("cancel worker");
                cancelled = true;
            }
        },
    )?;
    assert_eq!(summary.interrupted, ["cancel-me"]);
    assert_eq!(summary.completed, ["continue"]);
    assert!(matches!(
        queue.checkpoint("cancel-me")?,
        Some(Checkpoint::Marker(_))
    ));
    Ok(())
}

#[test]
fn batch_width_trials_isolate_errors_and_resume_only_failures() -> Result<()> {
    let dir = TestDir::new();
    let queue = Queue::open(&dir.0)?;
    let bad = impossible_job();
    let mut batch = request("batch", 10);
    batch.time = TimeTier::Overnight { seconds: 0.02 };
    batch.task = Task::Batch {
        jobs: vec![job(), bad],
        widths_mm: vec![100.0, 120.0, 100.0],
        options: Default::default(),
    };
    queue.add(batch)?;
    queue.add(request("after-batch", 0))?;
    let run = RunOptions {
        slice_seconds: 0.02,
        ..Default::default()
    };
    let summary = queue.run(run, |_| {})?;
    assert_eq!(summary.failed, ["batch"]);
    assert_eq!(summary.completed, ["after-batch"]);
    let Some(Checkpoint::Batch(result)) = queue.checkpoint("batch")? else {
        panic!("batch checkpoint")
    };
    assert_eq!(result.trials.len(), 4);
    assert!(!result.complete);
    assert_eq!(
        result.trials.iter().filter(|t| t.result.is_some()).count(),
        2
    );
    let before = queue.status("batch")?.search_seconds;
    queue.resume("batch")?;
    queue.run(run, |_| {})?;
    assert!((queue.status("batch")?.search_seconds - before - 0.04).abs() < 1e-6);
    assert!(queue.root.join("results/batch/summary.csv").exists());
    Ok(())
}

#[test]
fn continuous_mode_never_reconstructs_for_short_intervals_and_guard_persists() -> Result<()> {
    let dir = TestDir::new();
    let queue = Queue::open(&dir.0)?;
    queue.add(request("continuous", 0))?;
    let mut calls = 0;
    queue.run(
        RunOptions {
            slice_seconds: 0.005,
            ..Default::default()
        },
        |e| {
            if e.message.starts_with("Continuous engine call:") {
                calls += 1;
            }
        },
    )?;
    assert_eq!(calls, 1);
    assert!(queue.status("continuous")?.first_layout_s.is_some());
    queue.add(request("guard", 0))?;
    queue.update("guard", |r| {
        r.status = Status::Running;
        Ok(())
    })?;
    let (_send, receive) = std::sync::mpsc::channel();
    crate::engine::construction_guard(&queue, "guard", receive, std::time::Duration::ZERO);
    let record = Queue::open(&dir.0)?.status("guard")?;
    assert!(record.construction_guard_triggered);
    assert!(record.summary.contains("continuous fallback"));
    Ok(())
}
