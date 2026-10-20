//! Versioned disk and desktop contracts. Input geometry is embedded for restart.
use beta_marker_core::{Job, MarkerResult};
use beta_marker_nest::NestOptions;
use beta_marker_plan::{
    CutOrder, CutPlan, PlanOptions,
    cost::{CostComparison, CostRequest},
};
use beta_marker_roll::RollRegister;
use serde::{Deserialize, Serialize};

/// Job state schema. Readers reject unknown versions instead of overwriting them.
pub const QUEUE_FORMAT: &str = "beta-marker.queue/v1";

/// Search budget tiers; construction, validation and exports add overhead.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(tag = "tier", rename_all = "snake_case")]
pub enum TimeTier {
    /// Twenty seconds of search.
    Draft,
    /// Sixty seconds of search.
    Costing,
    /// Three hundred seconds of search.
    Production,
    /// Caller-selected overnight search budget.
    Overnight {
        /// Search seconds.
        seconds: f64,
    },
}
impl TimeTier {
    /// Search seconds in this tier.
    pub fn seconds(self) -> f64 {
        match self {
            Self::Draft => 20.0,
            Self::Costing => 60.0,
            Self::Production => 300.0,
            Self::Overnight { seconds } => seconds,
        }
    }
}

/// Typed operations. No shell commands or arbitrary executables are persisted.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Task {
    /// Nest one complete job continuously; resume from a checked checkpoint.
    Nest {
        /// Embedded immutable job.
        job: Job,
        /// Search rules; tier replaces total time, runner bounds workers.
        options: NestOptions,
    },
    /// Nest several source jobs at optional trial widths, isolating each failure.
    Batch {
        /// Embedded jobs in stable trial order.
        jobs: Vec<Job>,
        /// Trial widths, millimetres. Empty keeps each job's original width.
        #[serde(default)]
        widths_mm: Vec<f64>,
        /// Search rules; tier applies per job/width trial.
        options: NestOptions,
    },
    /// Choose marker ratios and plies for an order.
    Plan {
        /// Order and patterns.
        order: CutOrder,
        /// Planner controls.
        options: PlanOptions,
    },
    /// Plan, allocate stock, check defects, and write production files.
    Pipeline {
        /// Order with stock-compatible shrinkage already applied.
        order: CutOrder,
        /// Planner controls.
        options: PlanOptions,
        /// Approved measured stock.
        rolls: RollRegister,
        /// Restrict stock to these identities.
        #[serde(default)]
        eligible_roll_ids: Vec<String>,
        /// Printed fabric colour.
        colour: String,
        /// Optional merchandiser report settings. No GSM or fabric price is guessed.
        #[serde(default)]
        report: Option<ReportSettings>,
    },
    /// Batch width/ratio trial priced against the same order.
    WidthTrials {
        /// Commercial alternatives and final-nest settings.
        request: CostRequest,
    },
}

/// Commercial inputs for pipeline XLSX/CSV/PDF reports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSettings {
    /// Buyer/style/PO/date and approval fields.
    pub meta: beta_marker_cutroom::production::ReportMeta,
    /// Grams per square metre.
    pub gsm: f64,
    /// Currency label, for example INR.
    pub currency: String,
    /// Purchase price per metre.
    pub price_per_m: f64,
    /// Cost per lay in the named currency.
    pub lay_cost: f64,
    /// Purchase wastage allowance, percent.
    pub purchase_wastage_pct: f64,
}

/// New job request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Submission {
    /// Filesystem-safe stable id. Reusing an id is rejected.
    pub id: String,
    /// Operator-facing name.
    pub name: String,
    /// Higher numbers run first. Creation time then id breaks ties.
    #[serde(default)]
    pub priority: i32,
    /// Search time tier.
    pub time: TimeTier,
    /// Operation input.
    pub task: Task,
}

/// Durable lifecycle, also used by the desktop application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Ready for the single queue runner.
    Queued,
    /// Claimed by the queue runner.
    Running,
    /// Paused by the operator, including between slices.
    Paused,
    /// Cancelled; any valid checkpoint remains available.
    Cancelled,
    /// Finished successfully.
    Completed,
    /// Failed independently; subsequent jobs still run.
    Failed,
}

/// Whole-plan or marker checkpoint, always independently validated before saving.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Checkpoint {
    /// Complete feasible geometry; job snapshot is in the submission.
    Marker(MarkerResult),
    /// Checked results and explicit failures for a batch of independent markers.
    Batch(BatchResult),
    /// Checked ratio geometries; quantity completeness is explicit.
    Plan(CutPlan),
    /// All width alternatives and a finally nested winner.
    WidthTrials(CostComparison),
}

/// One independently nested batch job/width trial.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchTrial {
    /// Index in Task::Batch jobs.
    pub job_index: usize,
    /// Exact trial width, millimetres.
    pub width_mm: f64,
    /// Best checked geometry; None on an isolated failure.
    pub result: Option<MarkerResult>,
    /// Isolated failure reason.
    pub error: Option<String>,
}

/// Durable partial or complete batch result. Successful trials survive resume.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BatchResult {
    /// Trials completed so far, including failed trials.
    pub trials: Vec<BatchTrial>,
    /// True only when every requested trial succeeded.
    pub complete: bool,
}

/// A checked marker for one exact ratio/width job, available before a complete
/// cut plan exists. Geometry/rules are embedded to prevent stale reuse.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatioCheckpoint {
    /// Exact width, ratio, geometry and rules used for this marker.
    pub job: Job,
    /// Best independently validated result for the exact job.
    pub result: MarkerResult,
}

/// JSON per job. Control and worker updates use one short OS file lock.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRecord {
    /// [`QUEUE_FORMAT`].
    pub format: String,
    /// Immutable input and scheduling fields.
    pub submission: Submission,
    /// Lifecycle status.
    pub status: Status,
    /// Unix milliseconds at creation.
    pub created_ms: u64,
    /// Unix milliseconds at last change.
    pub updated_ms: u64,
    /// Number of starts, including restart recovery and resume.
    pub attempts: u32,
    /// Completed/dispatched search-budget seconds, not measured CPU time.
    /// Plan tasks restart from checked markers when resumed.
    pub search_seconds: f64,
    /// Total wall seconds across attempts, including construction and validation.
    pub runtime_s: f64,
    /// Engine wall seconds times declared workers, summed across bounded calls.
    /// This is occupied worker time, not measured hardware CPU utilisation.
    #[serde(default)]
    pub worker_seconds: f64,
    /// Time to the first reported feasible cold layout. Includes construction
    /// and any search before that callback; not a pure constructor profiler.
    #[serde(default)]
    pub first_layout_s: Option<f64>,
    /// No cold feasible-layout callback arrived within the 60-second guard.
    /// The engine remains in one continuous call; no construction is restarted.
    #[serde(default)]
    pub construction_guard_triggered: bool,
    /// Relative best-valid result path, when available.
    pub checkpoint: Option<String>,
    /// Operator-facing completion or recovery summary.
    pub summary: String,
    /// Last independent failure, when any.
    pub error: Option<String>,
}

/// Queue CPU and responsiveness controls. Jobs execute serially, so total engine
/// workers never exceed this limit. The coordinator thread is not an engine worker.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(default)]
pub struct RunOptions {
    /// Maximum total engine workers; planning currently requires at least two.
    pub max_workers: usize,
    /// Reserved checkpoint-interval request. The current engine cannot retain
    /// construction state, so execution is continuous even when this is smaller.
    pub slice_seconds: f64,
}
impl Default for RunOptions {
    fn default() -> Self {
        Self {
            max_workers: 2,
            slice_seconds: 60.0,
        }
    }
}

/// Synchronous progress events. Long callbacks count as job wall time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Progress {
    /// Stable queue job id.
    pub job_id: String,
    /// Current durable lifecycle.
    pub status: Status,
    /// Phase or improvement summary.
    pub message: String,
    /// Best feasible length when nesting a single marker, millimetres.
    pub best_length_mm: Option<f64>,
    /// True only after valid geometry has reached durable disk storage.
    pub checkpoint_saved: bool,
}

/// Summary of one draining run. Failed or malformed jobs do not stop other jobs.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RunSummary {
    /// Successful jobs this run.
    pub completed: Vec<String>,
    /// Failed jobs this run.
    pub failed: Vec<String>,
    /// Paused or cancelled jobs this run.
    pub interrupted: Vec<String>,
    /// Corrupt JSON files skipped without modification.
    pub unreadable: Vec<String>,
}
