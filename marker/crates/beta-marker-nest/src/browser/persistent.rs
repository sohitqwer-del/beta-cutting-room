//! A resumable controller over the unmodified Sparrow separator API.
//! Imported geometry, worker RNGs, collision weights and unfinished separation
//! attempts live across host yields. Only independently valid layouts escape.
use crate::{
    Instant, NestOptions, NestProgress, build_instance, map_solution, safety, warm_solution,
};
use anyhow::{Result, ensure};
use beta_marker_core::{Demand, Job, Layout, MarkerResult, Nap, RESULT_FORMAT};
use jagua_rs::probs::spp::entities::{SPProblem, SPSolution};
use rand::{Rng, RngExt, SeedableRng, rngs::Xoshiro256PlusPlus};
use sparrow::{
    config::DEFAULT_SPARROW_CONFIG,
    consts::LBF_SAMPLE_CONFIG,
    optimizer::{lbf::LBFBuilder, separator::Separator},
    util::{
        listener::DummySolListener,
        terminator::{BasicTerminator, Terminator},
    },
};
use std::time::Duration;

pub(super) struct Session {
    seed: u64,
    source: Job,
    job: Job,
    mapping: Vec<Demand>,
    shift: f64,
    separator: Separator,
    feasible: SPSolution,
    native_best: SPSolution,
    pool: Vec<SPSolution>,
    explored: bool,
    best: MarkerResult,
    active: bool,
    step: f32,
    finished: bool,
}

impl Session {
    pub(super) fn eligible(job: &Job, opts: &NestOptions) -> bool {
        opts.fixed.is_empty()
            && job.fabric.layout == Layout::Open
            && job.fabric.defects.is_empty()
            && job.fabric.shade.is_none()
            && job.fabric.repeat.is_none()
            && job.match_rules.is_empty()
            && job.fabric.nap != Nap::OneWayPerBundle
            && job
                .pieces
                .iter()
                .all(|p| p.block_margin.is_none() && !beta_marker_core::rules::flip_allowed(job, p))
    }
    pub(super) fn new(source: &Job, opts: &NestOptions) -> Result<Self> {
        ensure!(
            Self::eligible(source, opts),
            "persistent Sparrow session does not support these constraints"
        );
        let job = beta_marker_core::rules::effective_job(source);
        let mapping = beta_marker_core::demands(&job);
        let (instance, shift) = build_instance(&job, &mapping)?;
        let seed = opts.seed.unwrap_or(0);
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(seed);
        let mut normalized = opts.warm_start.clone();
        if let Some(warm) = &mut normalized
            && warm.placements.iter().any(|p| p.bundle.is_none())
        {
            crate::v02::assign_bundles(&job, &mut warm.placements);
        }
        let prob = if let Some(warm) = &normalized {
            let solution = warm_solution(&job, warm, &mapping, &instance, shift)?;
            let mut prob = SPProblem::new(instance)?;
            prob.restore(&solution);
            prob
        } else {
            LBFBuilder::new(
                instance,
                Xoshiro256PlusPlus::seed_from_u64(rng.next_u64()),
                LBF_SAMPLE_CONFIG,
            )?
            .construct()?
            .prob
        };
        let feasible = prob.save();
        let mut config = DEFAULT_SPARROW_CONFIG.cmpr_cfg.separator_config;
        config.n_workers = 1;
        let separator = Separator::new(prob, rng, config);
        let placements = map_solution(&feasible, &mapping, &job, shift)?;
        let metrics = beta_marker_core::compute_metrics(&job, &placements);
        let mut best = MarkerResult {
            format: RESULT_FORMAT.into(),
            job_name: job.name.clone(),
            width: job.fabric.width,
            length: metrics.length,
            piece_area: metrics.piece_area,
            efficiency: metrics.efficiency,
            useful_piece_area: metrics.useful_piece_area,
            useful_yield: metrics.useful_yield,
            surplus_copies: metrics.surplus_copies,
            engine: "sparrow 0.3.0 persistent browser".into(),
            seed: Some(seed),
            runtime_s: 0.,
            placements,
            warnings: vec![],
        };
        safety::repair(&job, &mut best)?;
        ensure!(
            beta_marker_core::validate(source, &best).ok,
            "persistent initial layout failed independent validation"
        );
        let active = separator.ct.get_total_loss() > 0.;
        Ok(Self {
            seed,
            source: source.clone(),
            job,
            mapping,
            shift,
            separator,
            native_best: feasible.clone(),
            pool: vec![feasible.clone()],
            explored: false,
            feasible,
            best,
            active,
            step: 0.002,
            finished: false,
        })
    }
    pub(super) fn step(
        &mut self,
        seconds: f64,
        progress: &mut dyn FnMut(NestProgress),
    ) -> Result<MarkerResult> {
        let started = Instant::now();
        let mut term = BasicTerminator::new();
        term.new_timeout(Duration::try_from_secs_f64(seconds)?);
        progress(NestProgress {
            seconds: 0.,
            length: self.best.length,
            result: self.best.clone(),
        });
        if !self.explored {
            let config = DEFAULT_SPARROW_CONFIG.expl_cfg;
            let mut listener = crate::ProgressListener {
                job: &self.job,
                mapping: &self.mapping,
                shift: self.shift,
                started,
                best: self.best.length,
                candidate: None,
                callback: progress,
            };
            let solutions = sparrow::optimizer::explore::exploration_phase(
                &mut self.separator,
                &mut listener,
                &term,
                &config,
            );
            if let Some(mut candidate) = listener.candidate.take()
                && candidate.length < self.best.length
            {
                candidate.seed = Some(self.seed);
                candidate.engine = "sparrow 0.3.0 persistent browser".into();
                self.best = candidate;
            }
            if let Some(solution) = solutions.last() {
                self.feasible = solution.clone();
                self.native_best = solution.clone();
            }
            self.pool = solutions.into_iter().rev().take(8).collect();
            self.active = self.separator.ct.get_total_loss() > 0.;
            self.explored = true;
        }
        while !self.finished && !term.kill() {
            if !self.active {
                let width = self.feasible.strip_width();
                let next = self.native_best.strip_width() * (1. - self.step);
                if next >= width || self.step < 1e-7 {
                    self.finished = true;
                    break;
                }
                self.separator.change_strip_width(width, None)?;
                self.separator.rollback(&self.feasible, None);
                let split = self.separator.rng.random_range(0.0..width);
                if self
                    .separator
                    .change_strip_width(next, Some(split))
                    .is_err()
                {
                    self.step *= 0.7;
                    continue;
                }
                self.active = true;
            }
            let (solution, snapshot) = self.separator.separate(&term, &mut DummySolListener);
            self.separator.rollback(&solution, Some(&snapshot));
            if snapshot.get_total_loss() == 0. {
                self.feasible = solution;
                self.active = false;
                if self.feasible.strip_width() < self.native_best.strip_width() {
                    self.native_best = self.feasible.clone();
                }
                self.pool.insert(0, self.feasible.clone());
                self.pool.truncate(8);
                if let Ok(placements) =
                    map_solution(&self.feasible, &self.mapping, &self.job, self.shift)
                {
                    let mut candidate = self.best.clone();
                    candidate.placements = placements;
                    if safety::repair(&self.job, &mut candidate).is_ok()
                        && beta_marker_core::validate(&self.source, &candidate).ok
                        && candidate.length < self.best.length
                    {
                        candidate.runtime_s = started.elapsed().as_secs_f64();
                        self.best = candidate;
                        progress(NestProgress {
                            seconds: started.elapsed().as_secs_f64(),
                            length: self.best.length,
                            result: self.best.clone(),
                        });
                    }
                }
            } else if !term.kill() {
                // A full failed strike decays the next compression step. A
                // timeout keeps the unfinished geometry and GLS weights alive.
                self.step *= 0.7;
                self.active = false;
                if !self.pool.is_empty() {
                    let r = self.separator.rng.random::<f32>() * self.separator.rng.random::<f32>();
                    let i = ((r * self.pool.len() as f32) as usize).min(self.pool.len() - 1);
                    self.feasible = self.pool[i].clone();
                }
            }
        }
        let mut result = self.best.clone();
        result.seed = Some(self.seed);
        result.runtime_s = started.elapsed().as_secs_f64();
        result.warnings.push(format!("Persistent browser separator: one model import, {} cumulative evaluations; unfinished attempts and RNG retained.",self.separator.total_evals));
        Ok(result)
    }
    pub(super) fn finished(&self) -> bool {
        self.finished
    }
    #[cfg(test)]
    pub(super) fn evaluations(&self) -> usize {
        self.separator.total_evals
    }
}
