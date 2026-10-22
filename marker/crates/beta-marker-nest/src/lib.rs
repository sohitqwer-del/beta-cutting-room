//! Garment rules and coordinate mapping for the unmodified sparrow optimizer.
//!
//! Jagua's fixed strip height is marker y; its variable width is marker x.
//! Jagua's export undoes its internal centring, so exported translations apply
//! to the grain-aligned original outlines.
//! Fixed pieces, defects and fold edges use the adapter's f64 obstacle packer.

pub mod browser;
mod profile;
mod safety;
mod tuning;
pub use tuning::{ConstructionOrder, PortfolioStrategy, SearchTuning, nest_with_tuning};
#[cfg(test)]
mod tests;
mod v02;

use anyhow::{Context, Result, ensure};
use beta_marker_core::geometry::{grain_alignment_deg, norm_deg, transform_points};
use beta_marker_core::{Demand, Job, MarkerResult, Placement, RESULT_FORMAT};
pub(crate) use jagua_rs::Instant;
use jagua_rs::geometry::shape_modification::ShapeModifyConfig;
use jagua_rs::io::ext_repr::{ExtItem, ExtOrientation, ExtRotation, ExtSPolygon, ExtShape};
use jagua_rs::io::import::Importer;
use jagua_rs::probs::spp::entities::{SPInstance, SPSolution, Strip};
use rand::{SeedableRng, rngs::Xoshiro256PlusPlus};
use serde::{Deserialize, Serialize};
use sparrow::config::{DEFAULT_SPARROW_CONFIG, ShrinkDecayStrategy};
use sparrow::consts::{DEFAULT_FAIL_DECAY_RATIO_CMPR, DEFAULT_MAX_CONSEQ_FAILS_EXPL};
use sparrow::util::listener::{ReportType, SolutionListener};
use sparrow::util::terminator::BasicTerminator;
use std::collections::BTreeMap;
use std::time::Duration;

/// Search controls. Sparrow import, initial construction and final checks add
/// overhead to its exploration/compression budget. The constrained path includes
/// construction and compaction in its allowance, with preparation, conservative
/// seed completion and independent validation still adding overhead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NestOptions {
    /// Total search budget in seconds. Sparrow runs split it 80/20 between
    /// exploration and compression; constrained runs construct and compact.
    pub time_limit_s: f64,
    /// Random seed. The result records the seed, including an internally chosen one.
    #[serde(default)]
    pub seed: Option<u64>,
    /// Maximum separator workers per run; `None` uses three. Portfolio runs
    /// may use fewer workers to fit the available CPU budget.
    #[serde(default)]
    pub workers: Option<usize>,
    /// Enable failure-based early stopping.
    #[serde(default)]
    pub early_termination: bool,
    /// Placements to keep exactly as given (locked pieces, matched pieces). The engine
    /// nests the remaining demand around them. Their copies count towards the demand.
    #[serde(default)]
    pub fixed: Vec<beta_marker_core::Placement>,
    /// Independent runs with different seeds (portfolio); the best valid result wins.
    /// 0 or 1 = a single run. Waves share the total time when CPUs are limited.
    #[serde(default)]
    pub runs: u32,
    /// Start from an earlier result of the same job (for example after a small edit).
    #[serde(default)]
    pub warm_start: Option<MarkerResult>,
}

impl Default for NestOptions {
    fn default() -> Self {
        Self {
            time_limit_s: 60.0,
            seed: None,
            workers: None,
            early_termination: false,
            fixed: Vec::new(),
            runs: 1,
            warm_start: None,
        }
    }
}

/// One feasible improvement, measured on the original cut outlines.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NestProgress {
    /// Wall time since entry, including instance construction.
    pub seconds: f64,
    /// Best feasible marker length, millimetres.
    pub length: f64,
    /// Complete independently checked layout, in the caller's piece identities.
    pub result: MarkerResult,
}

/// Nest and check a job, without installing a logger, signal handler or exporter.
pub fn nest(job: &Job, opts: &NestOptions) -> Result<MarkerResult> {
    nest_with_progress(job, opts, |_| {})
}

/// Nest with a synchronous callback for each feasible length improvement.
///
/// Callback time counts towards the budget. Random streams follow the seed, but
/// wall-clock cutoffs and worker scheduling preclude byte-identical replay.
pub fn nest_with_progress(
    job: &Job,
    opts: &NestOptions,
    mut callback: impl FnMut(NestProgress),
) -> Result<MarkerResult> {
    tuning::nest_with_tuning_and_progress(job, opts, &SearchTuning::default(), &mut callback)
}

/// Nest with cooperative cancellation and a checked complete incumbent.
///
/// Set `cancel` from another thread or from the progress callback. This entry
/// point uses the bounded obstacle packer, including for an unconstrained job.
/// A cold start first constructs a cheap complete marker; preparation and final
/// validation still run. Cancellation never returns an incomplete marker.
pub fn nest_with_cancellation(
    job: &Job,
    opts: &NestOptions,
    cancel: &std::sync::atomic::AtomicBool,
    mut callback: impl FnMut(NestProgress),
) -> Result<MarkerResult> {
    v02::nest_controlled(
        job,
        opts,
        &mut callback,
        &SearchTuning::default(),
        Some(cancel),
    )
}

/// Engine id suffix of a fold half joined to its pinning bar (see `v02::pinned`).
const PINNED: &str = "_pin";

fn nest_single(
    job: &Job,
    opts: &NestOptions,
    callback: &mut dyn FnMut(NestProgress),
    tuning: &SearchTuning,
) -> Result<MarkerResult> {
    let started = Instant::now();
    safety::check_input(job)?;
    let issues = beta_marker_core::check_job(job);
    ensure!(
        issues.is_empty(),
        "invalid job: {}",
        issues
            .iter()
            .map(|i| format!("{}: {}", i.code, i.message))
            .collect::<Vec<_>>()
            .join("; ")
    );
    ensure!(
        opts.time_limit_s.is_finite() && opts.time_limit_s > 0.0,
        "time limit must be finite and positive"
    );
    let duration =
        Duration::try_from_secs_f64(opts.time_limit_s).context("time limit is too large")?;
    ensure!(opts.workers != Some(0), "workers must be greater than zero");
    let mut config = DEFAULT_SPARROW_CONFIG;
    config.expl_cfg.time_limit = duration.mul_f64(1.0 - tuning.compression_share);
    config.cmpr_cfg.time_limit = duration.mul_f64(tuning.compression_share);
    config.expl_cfg.shrink_step = tuning.exploration_shrink as f32;
    config.expl_cfg.solution_pool_distribution_stddev = tuning.pool_stddev as f32;
    for separator in [
        &mut config.expl_cfg.separator_config,
        &mut config.cmpr_cfg.separator_config,
    ] {
        separator.sample_config.n_container_samples = tuning.container_samples;
        separator.sample_config.n_focussed_samples = tuning.focused_samples;
        separator.sample_config.n_coord_descents = tuning.coordinate_descents;
    }
    if let Some(workers) = opts.workers {
        config.expl_cfg.separator_config.n_workers = workers;
        config.cmpr_cfg.separator_config.n_workers = workers;
    }
    if opts.early_termination {
        config.expl_cfg.max_conseq_failed_attempts = Some(DEFAULT_MAX_CONSEQ_FAILS_EXPL);
        config.cmpr_cfg.shrink_decay =
            ShrinkDecayStrategy::FailureBased(DEFAULT_FAIL_DECAY_RATIO_CMPR);
    }
    let seed = opts.seed.unwrap_or_else(rand::random);
    let mut mapping = beta_marker_core::demands(job);
    match tuning.construction_order {
        ConstructionOrder::Input => {}
        ConstructionOrder::Reverse => mapping.reverse(),
        ConstructionOrder::Shuffle => {
            use rand::seq::SliceRandom;
            mapping.shuffle(&mut Xoshiro256PlusPlus::seed_from_u64(seed));
        }
    }
    let (instance, shift) = build_instance(job, &mapping)?;
    let initial = opts
        .warm_start
        .as_ref()
        .map(|result| warm_solution(job, result, &mapping, &instance, shift))
        .transpose()?;
    let mut listener = ProgressListener {
        job,
        mapping: &mapping,
        shift,
        started,
        best: f64::INFINITY,
        candidate: None,
        callback,
    };
    let solution = sparrow::optimizer::optimize(
        instance,
        Xoshiro256PlusPlus::seed_from_u64(seed),
        &mut listener,
        &mut BasicTerminator::new(),
        &config.expl_cfg,
        &config.cmpr_cfg,
        initial.as_ref(),
    )
    .context("sparrow could not construct or optimise a marker")?;
    let retained = listener.candidate.take();
    let placements = map_solution(&solution, &mapping, job, shift)?;
    let metrics = beta_marker_core::compute_metrics(job, &placements);
    let mut result = MarkerResult {
        format: RESULT_FORMAT.into(),
        job_name: job.name.clone(),
        width: job.fabric.width,
        length: metrics.length,
        piece_area: metrics.piece_area,
        efficiency: metrics.efficiency,
        useful_piece_area: metrics.useful_piece_area,
        useful_yield: metrics.useful_yield,
        surplus_copies: metrics.surplus_copies,
        engine: "sparrow 0.3.0".into(),
        seed: Some(seed),
        runtime_s: 0.0,
        placements,
        warnings: vec![],
    };
    let repaired = safety::repair(job, &mut result);
    if let Some(mut candidate) = retained
        && (repaired.is_err() || candidate.length < result.length)
    {
        candidate.seed = Some(seed);
        result = candidate;
    } else {
        repaired?;
    }
    result.runtime_s = started.elapsed().as_secs_f64();
    Ok(result)
}

fn warm_solution(
    job: &Job,
    result: &MarkerResult,
    mapping: &[Demand],
    instance: &SPInstance,
    shift: f64,
) -> Result<SPSolution> {
    use jagua_rs::io::ext_repr::{ExtLayout, ExtPlacedItem, ExtTransformation};
    use jagua_rs::probs::spp::io::ext_repr::ExtSPSolution;
    ensure!(
        result.job_name == job.name && result.width == job.fabric.width,
        "warm start belongs to a different job or fabric width"
    );
    let report = beta_marker_core::validate(job, result);
    ensure!(report.ok, "invalid warm start: {:?}", report.errors);
    let placed_items = result
        .placements
        .iter()
        .map(|pl| {
            let idx = mapping
                .iter()
                .position(|d| d.piece_id == pl.piece_id && d.mirrored == pl.mirrored)
                .context("warm start has an unknown piece or mirror state")?;
            let base = grain_alignment_deg(job.pieces[mapping[idx].piece_index].grain, pl.mirrored);
            Ok(ExtPlacedItem {
                item_id: idx as u64,
                transformation: ExtTransformation {
                    reflected: false,
                    rotation: norm_deg(pl.rotation_deg - base) as f32,
                    translation: (
                        (pl.translate[0] - shift) as f32,
                        (pl.translate[1] - shift) as f32,
                    ),
                },
            })
        })
        .collect::<Result<Vec<_>>>()?;
    // The exported outlines omit the inflated collision margin. Reserve it at
    // the length end too; the separator resolves any conservative contacts.
    let slack = job
        .pieces
        .iter()
        .map(|p| p.buffer + job.spacing.piece_gap + safety::precision_guard(&p.outline))
        .fold(1.0_f64, f64::max);
    let width = (result.length - shift + slack) as f32;
    ensure!(
        width.is_finite() && width > 0.0,
        "warm-start length exceeds engine precision"
    );
    ensure!(
        placed_items
            .iter()
            .all(|p| p.transformation.translation.0.is_finite()
                && p.transformation.translation.1.is_finite()),
        "warm-start translation exceeds engine precision"
    );
    jagua_rs::probs::spp::io::import_solution(
        instance,
        &ExtSPSolution {
            strip_width: width,
            density: 0.0,
            run_time_sec: 0,
            layout: ExtLayout {
                container_id: 0,
                density: 0.0,
                placed_items,
            },
        },
    )
}

fn engine_rotation(job: &Job, demand: &Demand, base: f64) -> ExtRotation {
    let allowed =
        beta_marker_core::allowed_rotations(job, &job.pieces[demand.piece_index], demand.mirrored);
    if allowed.free {
        return ExtRotation::Continuous {};
    }
    // At most four extra samples per base, including the tilt endpoints.
    let offsets = if allowed.tilt_deg > 0.0 {
        vec![
            -allowed.tilt_deg,
            -allowed.tilt_deg.min(0.5),
            0.0,
            allowed.tilt_deg.min(0.5),
            allowed.tilt_deg,
        ]
    } else {
        vec![0.0]
    };
    let mut angles: Vec<f32> = allowed
        .angles
        .iter()
        .flat_map(|a| offsets.iter().map(move |d| norm_deg(a - base + d) as f32))
        .collect();
    angles.sort_by(f32::total_cmp);
    angles.dedup();
    ExtRotation::Discrete { angles }
}

fn build_instance(job: &Job, mapping: &[Demand]) -> Result<(SPInstance, f64)> {
    let config = DEFAULT_SPARROW_CONFIG;
    let importer = Importer::new(
        config.cde_config,
        config.poly_simpl_tolerance,
        config.narrow_concavity_cutoff_ratio,
    );
    let half_gap = job.spacing.piece_gap / 2.0;
    // Cut-line clearance at engine y=0 is half_gap+buffer. Translation by
    // edge_gap-half_gap makes it edge_gap+buffer. A negative shift gives back
    // clearance that was reserved only for neighbouring pieces.
    let shift = job.spacing.edge_gap - half_gap;
    let height = (job.fabric.width - 2.0 * shift) as f32;
    ensure!(
        height.is_finite() && height > 0.0,
        "edge gaps leave no usable fabric width"
    );
    let mut items = Vec::with_capacity(mapping.len());
    for (idx, demand) in mapping.iter().enumerate() {
        let piece = &job.pieces[demand.piece_index];
        let base = grain_alignment_deg(piece.grain, demand.mirrored);
        let outline = safety::clean_outline(&piece.outline)?;
        let points = transform_points(&outline, demand.mirrored, base, [0.0, 0.0]);
        let true_points = transform_points(&piece.outline, demand.mirrored, base, [0.0, 0.0]);
        ensure!(
            true_points
                .iter()
                .flatten()
                .all(|v| (*v as f32).is_finite()),
            "grain-aligned outline exceeds engine precision for piece '{}'",
            piece.id
        );
        let ext = ExtItem {
            id: idx as u64,
            orientation: ExtOrientation {
                rotation: engine_rotation(job, demand, base),
                reflection_axes: vec![],
            },
            shape: ExtShape::SimplePolygon(ExtSPolygon(
                points.iter().map(|p| (p[0] as f32, p[1] as f32)).collect(),
            )),
            min_quality: None,
        };
        let mut item_importer = importer;
        if piece.id.ends_with(PINNED) {
            // Simplification may move a pinned outline's extreme edges outward.
            item_importer.shape_modify_config.simplify_tolerance = None;
        }
        let radius = half_gap + piece.buffer;
        // Reserve a numerical guard even when requested spacing is zero.
        // Round-join chords need slightly more than sec(0.05) inflation.
        let offset = (radius * 1.002 + safety::precision_guard(&true_points)) as f32;
        ensure!(
            offset.is_finite(),
            "clearance exceeds engine precision for piece '{}'",
            piece.id
        );
        item_importer.shape_modify_config.offset = Some(offset);
        let mut item = item_importer
            .import_item(&ext, idx)
            .with_context(|| format!("invalid collision outline for piece {}", piece.id))?;
        if !safety::collision_covers(&item, &true_points, radius) {
            // A simplifier must never reduce the actual cut outline. Retry the
            // inflated, unsimplified outer boundary if its coverage check fails.
            item_importer.shape_modify_config.simplify_tolerance = None;
            item_importer.shape_modify_config.narrow_concavity_cutoff = None;
            item = item_importer.import_item(&ext, idx).with_context(|| {
                format!("could not inflate collision outline for piece {}", piece.id)
            })?;
        }
        ensure!(
            safety::collision_covers(&item, &true_points, radius),
            "collision shape does not fully cover outline and clearance for piece '{}' at engine precision",
            piece.id
        );
        items.push((item, demand.count as usize));
    }
    let area: f32 = items
        .iter()
        .map(|(item, count)| item.area() * *count as f32)
        .sum();
    let strip = Strip::new(
        height,
        config.cde_config,
        ShapeModifyConfig::default(),
        area / height,
    )?;
    Ok((SPInstance::new(items, strip), shift))
}

fn map_solution(
    solution: &SPSolution,
    mapping: &[Demand],
    job: &Job,
    shift: f64,
) -> Result<Vec<Placement>> {
    let layout = jagua_rs::io::export::export_layout_snapshot(solution.layout_snapshot(), 0);
    let mut copies = BTreeMap::<usize, u32>::new();
    let mut placements = Vec::with_capacity(layout.placed_items.len());
    for placed in layout.placed_items {
        let idx = usize::try_from(placed.item_id)?;
        let demand = mapping
            .get(idx)
            .context("engine returned an unknown item")?;
        let transform = placed.transformation;
        ensure!(
            !transform.reflected,
            "engine reflected an item without permission"
        );
        let copy = copies.entry(idx).or_default();
        placements.push(Placement {
            piece_id: demand.piece_id.clone(),
            copy: *copy,
            mirrored: demand.mirrored,
            rotation_deg: norm_deg(
                f64::from(transform.rotation)
                    + grain_alignment_deg(job.pieces[demand.piece_index].grain, demand.mirrored),
            ),
            translate: [
                f64::from(transform.translation.0) + shift,
                f64::from(transform.translation.1) + shift,
            ],
            locked: false,
            ..Default::default()
        });
        *copy += 1;
    }
    placements
        .sort_by(|a, b| (&a.piece_id, a.mirrored, a.copy).cmp(&(&b.piece_id, b.mirrored, b.copy)));
    v02::assign_bundles(job, &mut placements);
    Ok(placements)
}

struct ProgressListener<'a> {
    job: &'a Job,
    mapping: &'a [Demand],
    shift: f64,
    started: Instant,
    best: f64,
    candidate: Option<MarkerResult>,
    callback: &'a mut dyn FnMut(NestProgress),
}

impl SolutionListener for ProgressListener<'_> {
    fn report(&mut self, report: ReportType, solution: &SPSolution) {
        if matches!(
            report,
            ReportType::ExplFeas | ReportType::CmprFeas | ReportType::Final
        ) && let Ok(placements) = map_solution(solution, self.mapping, self.job, self.shift)
        {
            let metrics = beta_marker_core::compute_metrics(self.job, &placements);
            if metrics.length >= self.best {
                return;
            }
            let candidate = MarkerResult {
                format: RESULT_FORMAT.into(),
                job_name: self.job.name.clone(),
                width: self.job.fabric.width,
                length: metrics.length,
                piece_area: metrics.piece_area,
                efficiency: metrics.efficiency,
                useful_piece_area: metrics.useful_piece_area,
                useful_yield: metrics.useful_yield,
                surplus_copies: metrics.surplus_copies,
                engine: "sparrow 0.3.0".into(),
                seed: None,
                runtime_s: 0.0,
                placements,
                warnings: vec![],
            };
            if !beta_marker_core::validate(self.job, &candidate).ok {
                return;
            }
            let length = metrics.length;
            if length < self.best {
                self.best = length;
                self.candidate = Some(candidate.clone());
                (self.callback)(NestProgress {
                    seconds: self.started.elapsed().as_secs_f64(),
                    length,
                    result: candidate,
                });
            }
        }
    }
}
