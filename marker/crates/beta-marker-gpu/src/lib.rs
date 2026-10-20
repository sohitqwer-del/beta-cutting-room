//! Experimental, headless raster placement proposals. Raster cells never decide
//! validity: selected positions use f64 polygon tests and full layouts pass the
//! independent core validator before they can become a Sparrow warm start.
mod backend;

use anyhow::{Context, Result, ensure};
use beta_marker_core::geometry::{area, bbox};
use beta_marker_core::rules::{
    allowed_rotations, bundle_sizes, cut_outline, demands, effective_job,
};
use beta_marker_core::{Job, Layout, MarkerResult, Nap, Placement, Pt, RESULT_FORMAT};
use beta_marker_nest::NestOptions;
use geo::{Distance, Euclidean, Relate, Validation};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Choose hardware automatically, require a GPU, or run identical raster logic on CPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendChoice {
    /// Use a hardware adapter if available; otherwise use CPU raster scans.
    Auto,
    /// Return an error if a hardware adapter or scan is unavailable.
    GpuRequired,
    /// Do not initialize graphics APIs.
    Cpu,
}

/// Bounded experiment settings, separate from the production nesting contract.
#[derive(Debug, Clone)]
pub struct GpuOptions {
    /// Cell size in millimetres. Small cells cost more and lose less space.
    pub cell_mm: f64,
    /// Device policy. Automatic mode falls back to CPU after a device error.
    pub backend: BackendChoice,
    /// Maximum strip allocation, in u32 words (4 bytes each).
    pub max_grid_words: usize,
}

impl Default for GpuOptions {
    fn default() -> Self {
        Self {
            cell_mm: 2.0,
            backend: BackendChoice::Auto,
            max_grid_words: 4_000_000,
        }
    }
}

/// Host timings include transfers and synchronization; they are not GPU timestamps.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ScanStats {
    /// Hardware name/backend, or CPU.
    pub backend: String,
    /// Reason for CPU fallback, when applicable.
    pub fallback_reason: Option<String>,
    /// Adapter/device/shader setup, once per reusable proposer.
    pub initialization_s: f64,
    /// Total proposal time, excluding reusable adapter setup.
    pub proposal_s: f64,
    /// Raster scanning wall time, including allocation, uploads and readback.
    pub scan_s: f64,
    /// GPU compute-pass time from timestamp queries, when supported. Excludes transfers.
    pub gpu_kernel_s: Option<f64>,
    /// Number of compute dispatches; zero on CPU.
    pub dispatches: u64,
    /// Candidate positions in dispatched domains (or actually examined on CPU).
    pub candidates: u64,
    /// Exact polygon checks of selected raster candidates.
    pub exact_checks: u64,
    /// Raster candidates rejected by exact checks.
    pub exact_rejections: u64,
}

/// A complete independently validated proposed layout and its measured cost.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    /// Full exact-validated layout, suitable for `NestOptions::warm_start`.
    pub result: MarkerResult,
    /// Device selection and measured proposal cost.
    pub stats: ScanStats,
}

/// Reusable device and pipeline. No window, surface, network or CUDA is used.
pub struct Proposer {
    gpu: Option<backend::Gpu>,
    options: GpuOptions,
    initial: ScanStats,
}

impl Proposer {
    /// Initialize hardware once. CPU mode does not enumerate graphics devices.
    pub fn new(options: GpuOptions) -> Result<Self> {
        ensure!(
            options.cell_mm.is_finite() && options.cell_mm >= 0.25,
            "cell size must be finite and at least 0.25 mm"
        );
        ensure!(
            options.max_grid_words > 0 && options.max_grid_words <= 16_000_000,
            "grid limit must be 1..=16000000 words"
        );
        let started = Instant::now();
        let mut initial = ScanStats {
            backend: "CPU".into(),
            ..Default::default()
        };
        let gpu = if options.backend == BackendChoice::Cpu {
            None
        } else {
            select_backend(options.backend, backend::Gpu::new(), &mut initial)?
        };
        initial.initialization_s = started.elapsed().as_secs_f64();
        Ok(Self {
            gpu,
            options,
            initial,
        })
    }

    /// Device selection and one-time initialization cost.
    pub fn device_stats(&self) -> &ScanStats {
        &self.initial
    }

    /// Construct a complete large-area-first, lowest-x layout on an open strip.
    /// Unsupported coupled garment rules return an error, rather than relaxing them.
    pub fn construct(&mut self, source: &Job) -> Result<Proposal> {
        self.construct_with_kept(source, &[])
    }

    /// Reinsert up to `count` small tail pieces around the remaining placements.
    /// Locked pieces stay exact. The independently valid incumbent is returned
    /// unchanged if reconstruction fails or does not shorten the marker.
    pub fn fill_gaps(
        &mut self,
        source: &Job,
        incumbent: &MarkerResult,
        count: usize,
    ) -> Result<Proposal> {
        ensure!(count <= 32, "fill-gaps count must be at most 32");
        ensure!(
            incumbent.job_name == source.name && incumbent.width == source.fabric.width,
            "incumbent belongs to another job or width"
        );
        let report = beta_marker_core::validate(source, incumbent);
        ensure!(report.ok, "invalid fill-gaps incumbent");
        // Editor geometry may be valid while its cached metrics are stale.
        // Compare physical lengths recomputed by the independent validator.
        let mut normalized = incumbent.clone();
        normalized.length = report.length;
        normalized.piece_area = report.piece_area;
        normalized.efficiency = report.efficiency;
        let incumbent = &normalized;
        let job = effective_job(source);
        check_supported(&job)?;
        let started = Instant::now();
        let mut candidates: Vec<_> = incumbent
            .placements
            .iter()
            .enumerate()
            .filter_map(|(i, pl)| {
                if pl.locked {
                    return None;
                }
                let piece = job.pieces.iter().find(|p| p.id == pl.piece_id)?;
                let b = bbox(&cut_outline(piece, pl))?;
                (b[2] > incumbent.length * 0.65).then_some((i, area(&piece.outline)))
            })
            .collect();
        candidates.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
        let removed: std::collections::BTreeSet<_> =
            candidates.into_iter().take(count).map(|c| c.0).collect();
        let kept: Vec<_> = incumbent
            .placements
            .iter()
            .enumerate()
            .filter(|(i, _)| !removed.contains(i))
            .map(|(_, p)| p.clone())
            .collect();
        let attempt = self.construct_with_kept(source, &kept);
        let mut proposal = match attempt {
            Ok(p) if p.result.length < incumbent.length - 1e-6 => p,
            Ok(mut p) => {
                p.result = incumbent.clone();
                p
            }
            Err(error) => Proposal {
                result: incumbent.clone(),
                stats: ScanStats {
                    fallback_reason: Some(format!("fill gaps retained incumbent: {error:#}")),
                    ..self.initial.clone()
                },
            },
        };
        proposal.stats.proposal_s = started.elapsed().as_secs_f64();
        Ok(proposal)
    }

    fn construct_with_kept(&mut self, source: &Job, kept: &[Placement]) -> Result<Proposal> {
        let started = Instant::now();
        let job = effective_job(source);
        check_supported(&job)?;
        let mut stats = self.initial.clone();
        let cell = self.options.cell_mm;
        let shift = job.spacing.edge_gap - job.spacing.piece_gap / 2.0;
        let height = dimension((job.fabric.width - 2.0 * shift) / cell)?;
        let mut strip = Raster::empty(1, height, self.options.max_grid_words)?;
        let mut slots = Vec::new();
        let mut mask_words = 0_usize;
        let bundles = bundle_sizes(&job);
        for demand in demands(&job) {
            let piece = &job.pieces[demand.piece_index];
            let ids: Vec<_> = bundles
                .iter()
                .enumerate()
                .filter(|(_, size)| **size == piece.size)
                .map(|(i, _)| i as u32)
                .collect();
            let per_garment = if demand.mirrored {
                piece.mirror_quantity
            } else {
                piece.quantity
            }
            .max(1);
            let angles = allowed_rotations(&job, piece, demand.mirrored);
            // Continuous rotation is sampled at quarter turns. These are legal
            // proposals, not an assertion that all angles have been searched.
            let angles = if angles.free {
                vec![0.0, 90.0, 180.0, 270.0]
            } else {
                angles.angles
            };
            let radius = job.spacing.piece_gap / 2.0 + piece.buffer + 0.02;
            let mut variants = Vec::new();
            for angle in angles {
                let pl = Placement {
                    piece_id: piece.id.clone(),
                    mirrored: demand.mirrored,
                    rotation_deg: angle,
                    ..Default::default()
                };
                let points = cut_outline(piece, &pl);
                let bounds = bbox(&points).context("empty cut outline")?;
                let normalized: Vec<_> = points
                    .iter()
                    .map(|p| [p[0] - bounds[0] + radius, p[1] - bounds[1] + radius])
                    .collect();
                let mask = Raster::polygon(&normalized, radius, cell, self.options.max_grid_words)?;
                if mask.height <= height {
                    mask_words = mask_words
                        .checked_add(mask.bits.len())
                        .context("mask cache size overflow")?;
                    ensure!(mask_words <= 32_000_000, "piece mask cache exceeds 128 MB");
                    variants.push(Variant {
                        placement: pl,
                        mask,
                        offset: [radius - bounds[0] + shift, radius - bounds[1] + shift],
                        end: bounds[2] - bounds[0] + radius + shift,
                    });
                }
            }
            ensure!(
                !variants.is_empty(),
                "piece '{}' does not fit the raster width",
                piece.id
            );
            let variants = std::sync::Arc::new(variants);
            for copy in 0..demand.count {
                slots.push(Slot {
                    piece: demand.piece_index,
                    copy,
                    bundle: ids.get((copy / per_garment) as usize).copied(),
                    variants: variants.clone(),
                    weight: area(&piece.outline),
                });
            }
        }
        let mut order: Vec<_> = (0..slots.len()).collect();
        order.sort_by(|&a, &b| slots[b].weight.total_cmp(&slots[a].weight).then(a.cmp(&b)));
        let mut placements = kept.to_vec();
        let mut occupied: Vec<ExactShape> = Vec::new();
        let mut used_rows = 0;
        for pl in kept {
            let piece = job
                .pieces
                .iter()
                .find(|p| p.id == pl.piece_id)
                .context("unknown kept piece")?;
            let points = cut_outline(piece, pl);
            let world: Vec<_> = points
                .iter()
                .map(|p| [p[0] - shift, p[1] - shift])
                .collect();
            let radius = job.spacing.piece_gap / 2.0 + piece.buffer + 0.02;
            let mut world_mask =
                Raster::polygon(&world, radius, cell, self.options.max_grid_words)?;
            // The valid incumbent's clearance halo can cross the strip edge.
            // Clip that halo to the scan domain; exact checks still use its
            // full polygons and buffers.
            world_mask.clip_height(height);
            strip.grow(world_mask.rows + 1, self.options.max_grid_words)?;
            strip.stamp(&world_mask, 0, 0);
            used_rows = used_rows.max(world_mask.rows);
            occupied.push(ExactShape::new(points, piece.buffer)?);
        }
        for index in order {
            let slot = &slots[index];
            let piece = &job.pieces[slot.piece];
            if kept.iter().any(|p| {
                p.piece_id == piece.id
                    && p.mirrored == slot.variants[0].placement.mirrored
                    && p.copy == slot.copy
            }) {
                continue;
            }
            let mut best: Option<(f64, u32, usize, Placement, ExactShape)> = None;
            for (orientation, variant) in slot.variants.iter().enumerate() {
                let max_x = used_rows;
                strip.grow(max_x + variant.mask.rows + 1, self.options.max_grid_words)?;
                let y_count = height - variant.mask.height + 1;
                let mut first = 0;
                // Conservative rasterization normally needs one check. Keep a
                // bounded retry path; a rejected position never reaches output.
                for _ in 0..64 {
                    let candidate = self.scan(&strip, &variant.mask, max_x, first, &mut stats)?;
                    let Some(candidate) = candidate else { break };
                    let (x, y) = (candidate / y_count, candidate % y_count);
                    let mut pl = variant.placement.clone();
                    pl.copy = slot.copy;
                    pl.bundle = slot.bundle;
                    pl.translate = [
                        f64::from(x) * cell + variant.offset[0],
                        f64::from(y) * cell + variant.offset[1],
                    ];
                    let shape = ExactShape::new(cut_outline(piece, &pl), piece.buffer)?;
                    stats.exact_checks += 1;
                    if exact_fits(&job, &shape, &occupied) {
                        let end = f64::from(x) * cell + variant.end;
                        if best
                            .as_ref()
                            .is_none_or(|b| end < b.0 || (end == b.0 && y < b.1))
                        {
                            best = Some((end, y, orientation, pl, shape));
                        }
                        break;
                    }
                    stats.exact_rejections += 1;
                    first = candidate + 1;
                }
            }
            let (_, _, orientation, pl, shape) =
                best.with_context(|| format!("no raster position for '{}'", piece.id))?;
            let variant = &slot.variants[orientation];
            let x = ((pl.translate[0] - variant.offset[0]) / cell).round() as u32;
            let y = ((pl.translate[1] - variant.offset[1]) / cell).round() as u32;
            strip.stamp(&variant.mask, x, y);
            used_rows = used_rows.max(x + variant.mask.rows);
            placements.push(pl);
            occupied.push(shape);
        }
        let metrics = beta_marker_core::compute_metrics(&job, &placements);
        let result = MarkerResult {
            format: RESULT_FORMAT.into(),
            job_name: source.name.clone(),
            width: job.fabric.width,
            length: metrics.length,
            piece_area: metrics.piece_area,
            efficiency: metrics.efficiency,
            engine: format!("experimental raster / {}", stats.backend),
            seed: None,
            runtime_s: started.elapsed().as_secs_f64(),
            placements,
            warnings: vec![format!(
                "Raster proposals at {cell} mm; exact polygon and independent validation gates. Tilt is not sampled; free rotation uses quarter turns."
            )],
        };
        let report = beta_marker_core::validate(source, &result);
        ensure!(
            report.ok,
            "proposal failed independent validation: {:?}",
            report.errors
        );
        stats.proposal_s = started.elapsed().as_secs_f64();
        Ok(Proposal { result, stats })
    }

    fn scan(
        &mut self,
        strip: &Raster,
        mask: &Raster,
        max_x: u32,
        first: u32,
        stats: &mut ScanStats,
    ) -> Result<Option<u32>> {
        if let Some(gpu) = &self.gpu {
            match gpu.scan(strip, mask, max_x, first, stats) {
                Ok(result) => return Ok(result),
                Err(error) if self.options.backend == BackendChoice::Auto => {
                    stats.fallback_reason = Some(format!("GPU scan failed: {error:#}"));
                    stats.backend = "CPU after GPU error".into();
                    self.gpu = None;
                    self.initial.backend = stats.backend.clone();
                    self.initial.fallback_reason = stats.fallback_reason.clone();
                }
                Err(error) => return Err(error),
            }
        }
        let started = Instant::now();
        let y_count = strip.height - mask.height + 1;
        let count = (max_x + 1)
            .checked_mul(y_count)
            .context("candidate index overflow")?;
        let result = (first..count).find(|&i| {
            stats.candidates += 1;
            strip.fits(mask, i / y_count, i % y_count)
        });
        stats.scan_s += started.elapsed().as_secs_f64();
        Ok(result)
    }
}

fn select_backend(
    choice: BackendChoice,
    attempt: Result<backend::Gpu>,
    stats: &mut ScanStats,
) -> Result<Option<backend::Gpu>> {
    match attempt {
        Ok(gpu) => {
            stats.backend = gpu.name.clone();
            Ok(Some(gpu))
        }
        Err(error) if choice == BackendChoice::Auto => {
            stats.fallback_reason = Some(format!("{error:#}"));
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

/// Try a proposal, then refine it with the existing unmodified nesting adapter.
/// Existing user incumbents/fixed pieces keep their production path. A failed
/// proposal falls back to plain nesting and records the reason in the result.
/// Proposal time is charged against the search allowance; adapter import and
/// final validation still add overhead, just as they do in native nesting.
pub fn nest(job: &Job, options: &NestOptions, proposer: &mut Proposer) -> Result<MarkerResult> {
    ensure!(
        options.time_limit_s.is_finite() && options.time_limit_s > 0.0,
        "time limit must be finite and positive"
    );
    let started = Instant::now();
    let mut options = options.clone();
    let mut note = None;
    if options.fixed.is_empty() && options.warm_start.is_none() {
        if proposer.gpu.is_none() && proposer.options.backend == BackendChoice::Auto {
            note = Some(format!(
                "GPU unavailable; plain nesting: {}",
                proposer
                    .initial
                    .fallback_reason
                    .as_deref()
                    .unwrap_or("no hardware adapter")
            ));
        } else {
            match proposer.construct(job) {
                Ok(proposal) => {
                    note = Some(format!(
                        "GPU experiment: {} proposal took {:.3} s",
                        proposal.stats.backend, proposal.stats.proposal_s
                    ));
                    options.warm_start = Some(proposal.result);
                }
                Err(error) => {
                    note = Some(format!(
                        "GPU proposal unavailable; plain nesting: {error:#}"
                    ))
                }
            }
        }
        options.time_limit_s = (options.time_limit_s - started.elapsed().as_secs_f64()).max(0.001);
    }
    let mut result = beta_marker_nest::nest(job, &options)?;
    if let Some(note) = note {
        result.warnings.push(note);
    }
    result.runtime_s = started.elapsed().as_secs_f64();
    ensure!(
        beta_marker_core::validate(job, &result).ok,
        "refined result failed independent validation"
    );
    Ok(result)
}

fn check_supported(job: &Job) -> Result<()> {
    ensure!(
        job.order.as_ref().is_none_or(|order| order
            .sizes
            .values()
            .map(|&n| u64::from(n))
            .sum::<u64>()
            <= 10_000),
        "raster experiment supports at most 10000 garment bundles"
    );
    let mut copies = 0_u64;
    for piece in &job.pieces {
        let multiplier = beta_marker_core::rules::multiplier(job, piece);
        for quantity in [piece.quantity, piece.mirror_quantity] {
            let count = quantity
                .checked_mul(multiplier)
                .context("piece demand exceeds u32")?;
            copies = copies
                .checked_add(u64::from(count))
                .context("piece demand overflow")?;
        }
    }
    ensure!(
        copies <= 10_000,
        "raster experiment supports at most 10000 copies"
    );
    ensure!(beta_marker_core::check_job(job).is_empty(), "invalid job");
    ensure!(
        job.fabric.width.is_finite()
            && job.spacing.edge_gap.is_finite()
            && job.spacing.piece_gap.is_finite(),
        "fabric and gaps must be finite"
    );
    ensure!(
        job.fabric.layout == Layout::Open
            && job.fabric.nap != Nap::OneWayPerBundle
            && job.fabric.shade.is_none()
            && job.fabric.repeat.is_none()
            && job.match_rules.is_empty()
            && job.fabric.defects.is_empty(),
        "raster experiment supports open fabric without coupled bundle, shade, repeat or defect rules"
    );
    ensure!(
        job.pieces
            .iter()
            .all(|p| p.buffer.is_finite()
                && p.block_margin.is_none_or(|v| v.is_finite() && v >= 0.0)),
        "invalid buffer or block margin"
    );
    ensure!(
        job.fabric
            .max_length
            .is_none_or(|v| v.is_finite() && v > 0.0),
        "maximum length must be finite and positive"
    );
    for piece in &job.pieces {
        ExactShape::new(piece.outline.clone(), piece.buffer)?
            .polygon
            .check_validation()
            .with_context(|| format!("invalid polygon for '{}'", piece.id))?;
    }
    Ok(())
}

fn dimension(value: f64) -> Result<u32> {
    ensure!(
        value.is_finite() && value > 0.0 && value <= 1_000_000.0,
        "raster dimension outside 1..=1000000 cells"
    );
    Ok(value.ceil() as u32)
}

#[derive(Clone)]
struct Variant {
    placement: Placement,
    mask: Raster,
    offset: Pt,
    end: f64,
}
struct Slot {
    piece: usize,
    copy: u32,
    bundle: Option<u32>,
    variants: std::sync::Arc<Vec<Variant>>,
    weight: f64,
}

#[derive(Clone)]
struct Raster {
    rows: u32,
    height: u32,
    stride: u32,
    bits: Vec<u32>,
}
impl Raster {
    fn clip_height(&mut self, height: u32) {
        if height >= self.height {
            return;
        }
        let stride = height.div_ceil(32);
        let last = if height.is_multiple_of(32) {
            u32::MAX
        } else {
            (1_u32 << (height % 32)) - 1
        };
        let mut bits = Vec::with_capacity((self.rows * stride) as usize);
        for x in 0..self.rows {
            let base = (x * self.stride) as usize;
            bits.extend_from_slice(&self.bits[base..base + stride as usize]);
            if let Some(word) = bits.last_mut() {
                *word &= last;
            }
        }
        self.bits = bits;
        self.stride = stride;
        self.height = height;
    }
    fn empty(rows: u32, height: u32, limit: usize) -> Result<Self> {
        let stride = height.div_ceil(32);
        let size = u64::from(rows) * u64::from(stride);
        ensure!(
            size <= limit as u64,
            "raster exceeds configured memory limit"
        );
        Ok(Self {
            rows,
            height,
            stride,
            bits: vec![0; size as usize],
        })
    }
    fn grow(&mut self, rows: u32, limit: usize) -> Result<()> {
        if rows > self.rows {
            ensure!(
                u64::from(rows) * u64::from(self.stride) <= limit as u64,
                "raster exceeds configured memory limit"
            );
            self.bits.resize(rows as usize * self.stride as usize, 0);
            self.rows = rows;
        }
        Ok(())
    }
    // A vertical span hull is deliberately conservative for concave polygons.
    // Intersect edges with each column's expanded slab, then fill its y range.
    fn polygon(points: &[Pt], radius: f64, cell: f64, limit: usize) -> Result<Self> {
        let b = bbox(points).context("empty raster outline")?;
        ensure!(b.iter().all(|v| v.is_finite()), "non-finite raster bounds");
        let rows = dimension((b[2] + radius) / cell)?;
        let height = dimension((b[3] + radius) / cell)?;
        let mut out = Self::empty(rows, height, limit)?;
        let start = ((b[0] - radius).max(0.0) / cell).floor() as u32;
        for x in start..rows {
            let lo = f64::from(x) * cell - radius;
            let hi = f64::from(x + 1) * cell + radius;
            let (mut bottom, mut top) = (f64::INFINITY, f64::NEG_INFINITY);
            for i in 0..points.len() {
                let (a, b) = (points[i], points[(i + 1) % points.len()]);
                if a[0].max(b[0]) < lo || a[0].min(b[0]) > hi {
                    continue;
                }
                for q in [lo.max(a[0].min(b[0])), hi.min(a[0].max(b[0]))] {
                    let t = if (b[0] - a[0]).abs() < 1e-12 {
                        0.0
                    } else {
                        (q - a[0]) / (b[0] - a[0])
                    };
                    let y = a[1] + t * (b[1] - a[1]);
                    bottom = bottom.min(y);
                    top = top.max(y);
                }
                if (b[0] - a[0]).abs() < 1e-12 {
                    bottom = bottom.min(a[1].min(b[1]));
                    top = top.max(a[1].max(b[1]));
                }
            }
            if bottom.is_finite() {
                let low = ((bottom - radius).max(0.0) / cell).floor() as u32;
                let high = (((top + radius) / cell).floor() as u32).min(height - 1);
                for y in low..=high {
                    out.bits[(x * out.stride + y / 32) as usize] |= 1 << (y % 32);
                }
            }
        }
        Ok(out)
    }
    fn fits(&self, shape: &Self, x: u32, y: u32) -> bool {
        let shift = y % 32;
        for k in 0..shape.rows {
            let row = (k + shape.rows / 2) % shape.rows;
            let base = (x + row) * self.stride + y / 32;
            for w in 0..shape.stride {
                let bits = shape.bits[(row * shape.stride + w) as usize];
                if self.bits[(base + w) as usize] & (bits << shift) != 0 {
                    return false;
                }
                if shift != 0
                    && bits >> (32 - shift) != 0
                    && self.bits[(base + w + 1) as usize] & (bits >> (32 - shift)) != 0
                {
                    return false;
                }
            }
        }
        true
    }
    fn stamp(&mut self, shape: &Self, x: u32, y: u32) {
        let shift = y % 32;
        for row in 0..shape.rows {
            let base = (x + row) * self.stride + y / 32;
            for w in 0..shape.stride {
                let bits = shape.bits[(row * shape.stride + w) as usize];
                self.bits[(base + w) as usize] |= bits << shift;
                if shift != 0 && bits >> (32 - shift) != 0 {
                    self.bits[(base + w + 1) as usize] |= bits >> (32 - shift);
                }
            }
        }
    }
}

struct ExactShape {
    bounds: [f64; 4],
    polygon: geo::Polygon<f64>,
    buffer: f64,
}
impl ExactShape {
    fn new(points: Vec<Pt>, buffer: f64) -> Result<Self> {
        let bounds = bbox(&points).context("empty exact outline")?;
        ensure!(
            bounds.iter().all(|v| v.is_finite()) && area(&points) > 0.0,
            "unusable exact outline"
        );
        let mut line: Vec<_> = points.iter().map(|p| (p[0], p[1])).collect();
        line.dedup();
        if line.first() != line.last() {
            line.push(line[0]);
        }
        Ok(Self {
            bounds,
            polygon: geo::Polygon::new(line.into(), vec![]),
            buffer,
        })
    }
}
fn exact_fits(job: &Job, shape: &ExactShape, others: &[ExactShape]) -> bool {
    let b = shape.bounds;
    let edge = job.spacing.edge_gap + shape.buffer;
    if b[0] < edge
        || b[1] < edge
        || b[3] > job.fabric.width - edge
        || job.fabric.max_length.is_some_and(|v| b[2] > v)
    {
        return false;
    }
    others.iter().all(|other| {
        let a = other.bounds;
        let gap = job.spacing.piece_gap + shape.buffer + other.buffer;
        if a[0] > b[2] + gap || b[0] > a[2] + gap || a[1] > b[3] + gap || b[1] > a[3] + gap {
            return true;
        }
        !shape
            .polygon
            .relate(&other.polygon)
            .matches("T********")
            .unwrap_or(true)
            && (gap <= 0.0 || Euclidean.distance(&shape.polygon, &other.polygon) + 1e-8 >= gap)
    })
}

#[cfg(test)]
mod tests;
