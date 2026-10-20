//! Offline, folder-backed production queue. A single OS-locked runner bounds CPU
//! across callers. Per-job JSON plus atomic checkpoints survive process restart.
#![warn(missing_docs)]
mod batch;
mod engine;
mod model;
use anyhow::{Context, Result, ensure};
pub use model::*;
use serde::{Serialize, de::DeserializeOwned};
use std::{
    fs::{self, File, OpenOptions},
    io::{BufReader, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

/// Durable local queue. Clones share the folder and OS locks, not in-memory state.
#[derive(Debug, Clone)]
pub struct Queue {
    root: PathBuf,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}
fn id_ok(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 96
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        && ![
            "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
            "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
        ]
        .contains(&id.to_ascii_uppercase().as_str())
}
fn lock(path: &Path) -> Result<File> {
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)?;
    file.lock()?;
    Ok(file)
}
fn read<T: DeserializeOwned>(path: &Path) -> Result<T> {
    serde_json::from_reader(BufReader::new(File::open(path)?))
        .with_context(|| format!("Invalid JSON in {}", path.display()))
}
pub(crate) fn atomic_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let data = serde_json::to_vec_pretty(value)?;
    atomic_bytes(path, &data)
}
pub(crate) fn atomic_bytes(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().context("Output needs a parent directory")?;
    fs::create_dir_all(parent)?;
    let tmp = path.with_extension("json.tmp");
    let mut file = File::create(&tmp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(&tmp, path).with_context(|| format!("Atomic save {}", path.display()))?;
    // Sync directory entries on Unix. Windows persists flushed file contents;
    // OS file locks, not leftover lock-file existence, determine ownership.
    #[cfg(unix)]
    File::open(parent)?.sync_all()?;
    Ok(())
}

impl Queue {
    /// Open/create a queue folder. Does not start or recover jobs until run().
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(root.join("jobs"))?;
        fs::create_dir_all(root.join("results"))?;
        Ok(Self { root })
    }
    /// Queue root path.
    pub fn root(&self) -> &Path {
        &self.root
    }
    fn state_lock(&self) -> Result<File> {
        lock(&self.root.join("state.lock"))
    }
    fn path(&self, id: &str) -> Result<PathBuf> {
        ensure!(
            id_ok(id),
            "Job id must contain 1-96 ASCII letters, digits, '-' or '_', and cannot be a Windows device name."
        );
        Ok(self.root.join("jobs").join(format!("{id}.json")))
    }
    /// Add a complete immutable request. Duplicate ids never overwrite jobs.
    pub fn add(&self, input: Submission) -> Result<JobRecord> {
        let path = self.path(&input.id)?;
        ensure!(
            input.time.seconds().is_finite()
                && input.time.seconds() > 0.0
                && input.time.seconds() <= 86_400.0,
            "Time tier must be in (0, 86400] seconds."
        );
        ensure!(!input.name.trim().is_empty(), "Job name is required.");
        let check_pattern = |job: &beta_marker_core::Job, ignore_ratio: bool| -> Result<()> {
            let mut pattern = job.clone();
            let optional = [
                pattern.fabric.max_length,
                pattern.fabric.repeat.as_ref().and_then(|r| r.period_x),
                pattern.fabric.repeat.as_ref().and_then(|r| r.period_y),
            ]
            .into_iter()
            .chain(
                pattern
                    .pieces
                    .iter()
                    .flat_map(|p| [p.block_margin, p.tilt_deg]),
            );
            ensure!(
                optional.flatten().all(f64::is_finite),
                "Optional fabric/piece numbers must be finite."
            );
            if ignore_ratio {
                pattern.order = None;
            }
            let issues = beta_marker_core::check_job(&pattern);
            ensure!(
                issues.is_empty(),
                "Invalid source job: {}",
                issues
                    .iter()
                    .map(|i| i.message.as_str())
                    .collect::<Vec<_>>()
                    .join("; ")
            );
            Ok(())
        };
        match &input.task {
            Task::Nest { job, .. } => check_pattern(job, false)?,
            Task::Batch { jobs, .. } => {
                for job in jobs {
                    check_pattern(job, false)?;
                }
            }
            Task::Plan { order, .. } | Task::Pipeline { order, .. } => {
                check_pattern(&order.job, true)?
            }
            Task::WidthTrials { request } => check_pattern(&request.order.job, true)?,
        }
        // Native callers can construct non-finite floats that JSON would write
        // as null. Reject requests that could not be read after restart.
        let _: Submission = serde_json::from_value(serde_json::to_value(&input)?)
            .context("Submission contains an unsupported/non-finite number")?;
        let _lock = self.state_lock()?;
        ensure!(!path.exists(), "Job id already exists: {}", input.id);
        let record = JobRecord {
            format: QUEUE_FORMAT.into(),
            submission: input,
            status: Status::Queued,
            created_ms: now(),
            updated_ms: now(),
            attempts: 0,
            search_seconds: 0.0,
            runtime_s: 0.0,
            worker_seconds: 0.0,
            first_layout_s: None,
            construction_guard_triggered: false,
            checkpoint: None,
            summary: "Waiting".into(),
            error: None,
        };
        atomic_json(&path, &record)?;
        Ok(record)
    }
    /// Read one version-checked job.
    pub fn status(&self, id: &str) -> Result<JobRecord> {
        let path = self.path(id)?;
        let record: JobRecord = read(&path)?;
        ensure!(
            record.format == QUEUE_FORMAT && record.submission.id == id,
            "Unsupported or mismatched queue record: {id}"
        );
        Ok(record)
    }
    /// Read all valid jobs plus isolated corrupt-file diagnostics.
    pub fn list(&self) -> Result<(Vec<JobRecord>, Vec<String>)> {
        let mut jobs = Vec::new();
        let mut failures = Vec::new();
        for entry in fs::read_dir(self.root.join("jobs"))? {
            let path = entry?.path();
            if path.extension().is_none_or(|x| x != "json") {
                continue;
            }
            let id = path.file_stem().and_then(|x| x.to_str()).unwrap_or("");
            match self.status(id) {
                Ok(record) => jobs.push(record),
                Err(e) => failures.push(format!("{}: {e:#}", path.display())),
            }
        }
        jobs.sort_by(|a, b| {
            b.submission
                .priority
                .cmp(&a.submission.priority)
                .then(a.created_ms.cmp(&b.created_ms))
                .then(a.submission.id.cmp(&b.submission.id))
        });
        Ok((jobs, failures))
    }
    fn update(&self, id: &str, f: impl FnOnce(&mut JobRecord) -> Result<()>) -> Result<JobRecord> {
        let _lock = self.state_lock()?;
        let mut record = self.status(id)?;
        f(&mut record)?;
        record.updated_ms = now();
        atomic_json(&self.path(id)?, &record)?;
        Ok(record)
    }
    /// Pause queued/running work. An in-flight engine call finishes its bounded slice.
    pub fn pause(&self, id: &str) -> Result<JobRecord> {
        self.update(id, |r| {
            ensure!(
                matches!(r.status, Status::Queued | Status::Running | Status::Paused),
                "Only queued/running jobs can pause."
            );
            r.status = Status::Paused;
            r.summary = "Paused; valid checkpoint retained".into();
            Ok(())
        })
    }
    /// Cancel queued, running or paused work; keep valid output for inspection.
    pub fn cancel(&self, id: &str) -> Result<JobRecord> {
        self.update(id, |r| {
            ensure!(
                !matches!(r.status, Status::Completed | Status::Failed),
                "Completed/failed jobs cannot be cancelled."
            );
            r.status = Status::Cancelled;
            r.summary = "Cancelled; valid checkpoint retained".into();
            Ok(())
        })
    }
    /// Resume paused/cancelled/failed work, keeping the best checkpoint and budget.
    pub fn resume(&self, id: &str) -> Result<JobRecord> {
        self.update(id, |r| {
            ensure!(
                matches!(
                    r.status,
                    Status::Paused | Status::Cancelled | Status::Failed
                ),
                "Only paused, cancelled or failed jobs can resume."
            );
            r.status = Status::Queued;
            r.error = None;
            r.summary = "Resumed; waiting".into();
            Ok(())
        })
    }
    /// Load the durable checked result, if one exists.
    pub fn checkpoint(&self, id: &str) -> Result<Option<Checkpoint>> {
        let record = self.status(id)?;
        // The geometry rename can precede the record-pointer rename at crash
        // time. Recover the checked orphan by its fixed, safe path as well.
        let path = self.root.join("results").join(id).join("checkpoint.json");
        if !path.exists() {
            return Ok(None);
        }
        let value = read(&path)?;
        engine::validate_checkpoint(&record.submission.task, &value)?;
        Ok(Some(value))
    }
    /// Read checked ratio-marker files, including a paused plan's markers before
    /// its first whole-plan checkpoint. Corrupt geometry returns an error.
    pub fn marker_checkpoints(&self, id: &str) -> Result<Vec<RatioCheckpoint>> {
        self.status(id)?;
        let path = self.root.join("results").join(id).join("markers");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let mut entries = fs::read_dir(path)?
            .map(|e| e.map(|e| e.path()))
            .collect::<std::io::Result<Vec<_>>>()?;
        entries.sort();
        let mut results = Vec::new();
        for path in entries {
            if path.extension().is_some_and(|x| x == "json") {
                let entry: RatioCheckpoint = read(&path)?;
                let report = beta_marker_core::validate(&entry.job, &entry.result);
                ensure!(report.ok, "Invalid ratio checkpoint in {}", path.display());
                results.push(entry);
            }
        }
        Ok(results)
    }
    /// Save and announce independently checked geometry. The runner owns this
    /// method; a checkpoint is never replaced by a worse marker/partial plan.
    fn save_checkpoint(&self, id: &str, checkpoint: &Checkpoint) -> Result<bool> {
        let _lock = self.state_lock()?;
        let mut record = self.status(id)?;
        engine::validate_checkpoint(&record.submission.task, checkpoint)?;
        let path = self.root.join("results").join(id).join("checkpoint.json");
        if path.exists() {
            let old: Checkpoint = read(&path)?;
            if !engine::better(checkpoint, &old) {
                if record.checkpoint.is_none() {
                    record.checkpoint = Some(format!("results/{id}/checkpoint.json"));
                    atomic_json(&self.path(id)?, &record)?;
                }
                return Ok(false);
            }
        }
        atomic_json(&path, checkpoint)?;
        record.checkpoint = Some(format!("results/{id}/checkpoint.json"));
        record.updated_ms = now();
        atomic_json(&self.path(id)?, &record)?;
        Ok(true)
    }
    /// Drain queued jobs in priority order. A second runner fails immediately;
    /// an OS-released stale runner lock permits safe recovery after a crash.
    pub fn run(
        &self,
        options: RunOptions,
        mut progress: impl FnMut(Progress),
    ) -> Result<RunSummary> {
        ensure!(
            options.max_workers > 0 && options.max_workers <= 256,
            "max_workers must be in [1, 256]."
        );
        ensure!(
            options.slice_seconds.is_finite()
                && options.slice_seconds > 0.0
                && options.slice_seconds <= 3600.0,
            "slice_seconds must be in (0, 3600]."
        );
        let runner = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.root.join("runner.lock"))?;
        runner
            .try_lock()
            .context("Another runner owns this queue")?;
        let mut summary = RunSummary::default();
        let (jobs, unreadable) = self.list()?;
        summary.unreadable = unreadable;
        for job in jobs.into_iter().filter(|r| r.status == Status::Running) {
            self.update(&job.submission.id, |r| {
                if r.status == Status::Running {
                    r.status = Status::Queued;
                    r.summary = "Recovered interrupted run; checkpoint retained".into();
                }
                Ok(())
            })?;
        }
        loop {
            let (jobs, _) = self.list()?;
            let Some(job) = jobs.into_iter().find(|r| r.status == Status::Queued) else {
                break;
            };
            let id = job.submission.id.clone();
            // Claim only if a concurrent operator has not paused/cancelled it.
            let claimed = self.update(&id, |r| {
                if r.status == Status::Queued {
                    r.status = Status::Running;
                    r.attempts = r.attempts.saturating_add(1);
                    r.error = None;
                }
                Ok(())
            })?;
            if claimed.status != Status::Running {
                continue;
            }
            progress(Progress {
                job_id: id.clone(),
                status: Status::Running,
                message: "Started".into(),
                best_length_mm: None,
                checkpoint_saved: false,
            });
            let started = std::time::Instant::now();
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                engine::execute(self, &claimed, options, &mut progress)
            }));
            let result = match outcome {
                Ok(r) => r,
                Err(_) => Err(anyhow::anyhow!(
                    "Worker panicked; retained any checked checkpoint."
                )),
            };
            let current = self.update(&id, |r| {
                r.runtime_s += started.elapsed().as_secs_f64();
                if r.status == Status::Running {
                    match &result {
                        Ok(message) => {
                            r.status = Status::Completed;
                            r.summary = message.clone();
                        }
                        Err(e) => {
                            r.status = Status::Failed;
                            r.error = Some(format!("{e:#}"));
                            r.summary = "Failed; other jobs continue".into();
                        }
                    }
                }
                Ok(())
            })?;
            match current.status {
                Status::Completed => summary.completed.push(id.clone()),
                Status::Failed => summary.failed.push(id.clone()),
                _ => summary.interrupted.push(id.clone()),
            }
            progress(Progress {
                job_id: id,
                status: current.status,
                message: current.summary,
                best_length_mm: None,
                checkpoint_saved: current.checkpoint.is_some(),
            });
        }
        atomic_json(&self.root.join("summary.json"), &summary)?;
        Ok(summary)
    }
}

#[cfg(test)]
mod tests;
