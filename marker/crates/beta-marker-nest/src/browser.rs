//! Single-thread browser search, yielding to the host between warm-start rounds.
//! All geometry and acceptance decisions stay in Rust. No shared memory is needed.
use crate::{Instant, NestOptions};
use anyhow::{Result, ensure};
use beta_marker_core::{Job, MarkerResult};
use serde::{Deserialize, Serialize};

/// Browser controls. One worker and one run per round are enforced.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BrowserOptions {
    pub time_limit_s: f64,
    /// JSON-safe integer. Omission chooses a fresh, recorded seed.
    pub seed: Option<u64>,
    /// Search seconds per round, excluding preparation and validation.
    pub round_time_s: f64,
    pub early_termination: bool,
    pub fixed: Vec<beta_marker_core::Placement>,
    pub warm_start: Option<MarkerResult>,
}

impl Default for BrowserOptions {
    fn default() -> Self {
        Self {
            time_limit_s: 30.0,
            seed: None,
            round_time_s: 2.0,
            early_termination: false,
            fixed: vec![],
            warm_start: None,
        }
    }
}

/// A complete, independently checked incumbent, never a partial layout.
#[derive(Clone, Debug, Serialize)]
pub struct BrowserProgress {
    /// True for a within-round notification. It needs no host acknowledgement.
    pub checkpoint: bool,
    pub round: u32,
    pub seconds: f64,
    pub round_seconds: f64,
    pub improved: bool,
    pub done: bool,
    pub cancelled: bool,
    pub result: MarkerResult,
}

/// A Worker calls `step`, yields to its message loop, then calls `step` again.
/// Cancellation is cooperative at round boundaries. Construction, an individual
/// Sparrow separator strike, repair and validation can overrun the round budget.
pub struct BrowserSession {
    job: Job,
    options: BrowserOptions,
    seed: u64,
    started: Option<Instant>,
    best: Option<MarkerResult>,
    seeded: bool,
    round: u32,
    cancelled: bool,
    done: bool,
    last_checkpoint: Option<Instant>,
}

impl BrowserSession {
    pub fn new(job: Job, options: BrowserOptions) -> Result<Self> {
        ensure!(
            options.time_limit_s.is_finite() && (0.001..=3600.0).contains(&options.time_limit_s),
            "time_limit_s must be between 0.001 and 3600"
        );
        ensure!(
            options.round_time_s.is_finite() && (0.05..=30.0).contains(&options.round_time_s),
            "round_time_s must be between 0.05 and 30"
        );
        const MAX_SEED: u64 = (1 << 53) - 1;
        ensure!(
            options.seed.is_none_or(|seed| seed <= MAX_SEED),
            "seed must be a non-negative JavaScript safe integer"
        );
        crate::safety::check_input(&job)?;
        let issues = beta_marker_core::check_job(&job);
        ensure!(issues.is_empty(), "invalid job: {issues:?}");
        if let Some(warm) = &options.warm_start {
            ensure!(
                warm.job_name == job.name && warm.width == job.fabric.width,
                "warm start belongs to a different job or fabric width"
            );
            let report = beta_marker_core::validate(&job, warm);
            ensure!(report.ok, "invalid warm start: {:?}", report.errors);
            // v02 normalizes bundle IDs on fixed pieces. Let it check identity
            // and demand before accepting a warm start with fixed constraints.
        }
        Ok(Self {
            job,
            seed: options
                .seed
                .unwrap_or_else(|| rand::random::<u64>() & MAX_SEED),
            options,
            best: None,
            seeded: false,
            started: None,
            round: 0,
            cancelled: false,
            done: false,
            last_checkpoint: None,
        })
    }

    pub fn cancel(&mut self) {
        self.cancelled = true;
    }

    pub fn step(&mut self) -> Result<BrowserProgress> {
        self.step_with_progress(&mut |_| {})
    }

    /// Publish checked improvements while the synchronous optimizer is running.
    pub fn step_with_progress(
        &mut self,
        callback: &mut dyn FnMut(BrowserProgress),
    ) -> Result<BrowserProgress> {
        ensure!(
            !(self.cancelled && self.best.is_none()),
            "search cancelled before a complete marker was available"
        );
        let started = *self.started.get_or_insert_with(Instant::now);
        let before = Instant::now();
        let mut improved = false;
        if !self.done && !self.cancelled {
            let remaining = self.options.time_limit_s - started.elapsed().as_secs_f64();
            if !self.seeded || remaining > 0.0 {
                let mut options = NestOptions {
                    time_limit_s: self.options.round_time_s.min(remaining.max(0.001)),
                    seed: Some(self.seed.wrapping_add(u64::from(self.round)) & ((1 << 53) - 1)),
                    workers: Some(1),
                    runs: 1,
                    early_termination: self.options.early_termination,
                    fixed: self.options.fixed.clone(),
                    // Keep the cheap seed as recovery, but let Sparrow use its
                    // own constructor on its first attempt. A box-based seed
                    // can trap its search in a much worse layout. A warm start
                    // also fixes the two-layer allocation, so every third round
                    // of a folded or tubular job starts fresh to try others.
                    warm_start: if self.round == 0
                        || (self.job.fabric.layout != beta_marker_core::Layout::Open
                            && self.options.fixed.is_empty()
                            && self.round.is_multiple_of(3))
                    {
                        self.options.warm_start.clone()
                    } else {
                        self.best
                            .clone()
                            .or_else(|| self.options.warm_start.clone())
                    },
                };
                // Publish a checked BETA-owned incumbent before entering Sparrow.
                // Some upstream separator work can exceed its timeout. The host
                // can then terminate a stalled Worker without losing this marker.
                let round = self.round + u32::from(self.seeded);
                let best = &mut self.best;
                let last_checkpoint = &mut self.last_checkpoint;
                let mut on_progress = |event: crate::NestProgress| {
                    let mut candidate = event.result;
                    if best
                        .as_ref()
                        .is_some_and(|old| candidate.length >= old.length)
                    {
                        return;
                    }
                    if !beta_marker_core::validate(&self.job, &candidate).ok {
                        return;
                    }
                    candidate.seed = options.seed;
                    candidate.runtime_s = started.elapsed().as_secs_f64();
                    *best = Some(candidate.clone());
                    improved = true;
                    if last_checkpoint.is_none_or(|last| last.elapsed().as_secs_f64() >= 0.5) {
                        *last_checkpoint = Some(Instant::now());
                        callback(BrowserProgress {
                            checkpoint: true,
                            round,
                            seconds: candidate.runtime_s,
                            round_seconds: before.elapsed().as_secs_f64(),
                            improved: true,
                            done: false,
                            cancelled: false,
                            result: candidate,
                        });
                    }
                };
                let outcome = if !self.seeded {
                    options.time_limit_s = options.time_limit_s.min(0.1);
                    let cancel = std::sync::atomic::AtomicBool::new(false);
                    crate::nest_with_cancellation(&self.job, &options, &cancel, &mut on_progress)
                } else {
                    self.round += 1;
                    crate::nest_with_progress(&self.job, &options, &mut on_progress)
                };
                let candidate = match outcome {
                    Ok(candidate) => candidate,
                    Err(error) if self.best.is_some() => {
                        self.done = true;
                        let mut retained = self.best.clone().expect("checked above");
                        retained.warnings.push(format!(
                            "Browser improvement failed; checked incumbent retained: {error}"
                        ));
                        retained
                    }
                    Err(error) => return Err(error),
                };
                let report = beta_marker_core::validate(&self.job, &candidate);
                ensure!(
                    report.ok,
                    "independent validation failed: {:?}",
                    report.errors
                );
                if self
                    .best
                    .as_ref()
                    .is_none_or(|best| candidate.length <= best.length)
                {
                    improved |= self
                        .best
                        .as_ref()
                        .is_none_or(|best| candidate.length < best.length);
                    self.best = Some(candidate);
                } else if self.done {
                    // Preserve the diagnostic on the retained valid incumbent.
                    self.best = Some(candidate);
                }
                self.seeded = true;
            }
        }
        let seconds = started.elapsed().as_secs_f64();
        self.done |= self.cancelled || seconds >= self.options.time_limit_s;
        let result = self
            .best
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("no complete marker"))?;
        result.runtime_s = seconds;
        if self.cancelled
            && !result
                .warnings
                .iter()
                .any(|w| w.starts_with("Browser search stopped"))
        {
            result
                .warnings
                .push("Browser search stopped between rounds; best valid marker retained.".into());
        }
        Ok(BrowserProgress {
            checkpoint: false,
            round: self.round,
            seconds,
            round_seconds: before.elapsed().as_secs_f64(),
            improved,
            done: self.done,
            cancelled: self.cancelled,
            result: result.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn job() -> Job {
        serde_json::from_str(include_str!(
            "../../../packages/marker-wasm/test/fixtures/contract.json"
        ))
        .map(|v: serde_json::Value| serde_json::from_value(v["job"].clone()).unwrap())
        .unwrap()
    }
    fn options() -> BrowserOptions {
        BrowserOptions {
            time_limit_s: 0.15,
            round_time_s: 0.05,
            seed: Some(7),
            ..Default::default()
        }
    }
    #[test]
    fn within_round_checkpoints_are_valid_monotone_and_throttled() -> Result<()> {
        let job = job();
        let mut session = BrowserSession::new(job.clone(), options())?;
        let mut events = vec![];
        let boundary = session.step_with_progress(&mut |p| events.push(p))?;
        assert!(
            !events.is_empty(),
            "seed must be visible before step returns"
        );
        let mut length = f64::INFINITY;
        let mut seconds = None;
        for event in events {
            assert!(event.checkpoint && !event.done);
            assert!(beta_marker_core::validate(&job, &event.result).ok);
            assert!(event.result.length < length);
            if let Some(last) = seconds {
                assert!(event.seconds - last >= 0.49);
            }
            seconds = Some(event.seconds);
            length = event.result.length;
        }
        assert!(!boundary.checkpoint);
        assert!(boundary.result.length <= length);
        Ok(())
    }
    #[test]
    fn valid_monotone_rounds_and_cooperative_stop() -> Result<()> {
        let job = job();
        let mut session = BrowserSession::new(job.clone(), options())?;
        let first = session.step()?;
        assert!(beta_marker_core::validate(&job, &first.result).ok);
        session.cancel();
        let stopped = session.step()?;
        assert!(stopped.done && stopped.cancelled);
        assert_eq!(stopped.result.placements, first.result.placements);
        assert_eq!(stopped.round, first.round);
        Ok(())
    }
    #[test]
    fn deadline_returns_complete_result_and_warm_start_never_regresses() -> Result<()> {
        let job = job();
        let mut session = BrowserSession::new(job.clone(), options())?;
        let mut last_length = f64::INFINITY;
        loop {
            let progress = session.step()?;
            assert!(progress.result.length <= last_length);
            assert!(beta_marker_core::validate(&job, &progress.result).ok);
            last_length = progress.result.length;
            if progress.done {
                break;
            }
        }
        let final_result = session.step()?;
        assert!(final_result.done);
        Ok(())
    }
    #[test]
    fn rejects_bad_controls_invalid_warm_start_and_early_cancel() -> Result<()> {
        for options in [
            BrowserOptions {
                time_limit_s: f64::NAN,
                ..options()
            },
            BrowserOptions {
                round_time_s: 0.0,
                ..options()
            },
            BrowserOptions {
                seed: Some(u64::MAX),
                ..options()
            },
        ] {
            assert!(BrowserSession::new(job(), options).is_err());
        }
        assert!(serde_json::from_str::<BrowserOptions>(r#"{"workers":3}"#).is_err());
        let mut session = BrowserSession::new(job(), options())?;
        session.cancel();
        assert!(session.step().is_err());
        let mut session = BrowserSession::new(job(), options())?;
        let mut warm = session.step()?.result;
        warm.placements[0].translate = [-100.0, -100.0];
        assert!(
            BrowserSession::new(
                job(),
                BrowserOptions {
                    warm_start: Some(warm),
                    ..options()
                }
            )
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn locked_piece_and_defect_survive_browser_rounds() -> Result<()> {
        let mut job = job();
        job.fabric.defects.push(beta_marker_core::Zone {
            id: "hole".into(),
            polygon: vec![[0., 0.], [30., 0.], [30., 100.], [0., 100.]],
            note: None,
        });
        let mut session = BrowserSession::new(job.clone(), options())?;
        let first = session.step()?.result;
        let mut fixed = first.placements[0].clone();
        fixed.locked = true;
        let mut session = BrowserSession::new(
            job.clone(),
            BrowserOptions {
                fixed: vec![fixed.clone()],
                ..options()
            },
        )?;
        let p = session.step()?;
        assert!(p.result.placements.contains(&fixed));
        assert!(beta_marker_core::validate(&job, &p.result).ok);
        session.cancel();
        assert!(session.step()?.result.placements.contains(&fixed));
        Ok(())
    }

    #[test]
    fn folded_browser_result_obeys_layer_and_fold_rules() -> Result<()> {
        let job = Job {
            format: beta_marker_core::JOB_FORMAT.into(),
            fabric: beta_marker_core::Fabric {
                width: 60.0,
                layout: beta_marker_core::Layout::Folded,
                ..Default::default()
            },
            pieces: vec![beta_marker_core::Piece {
                id: "fold-panel".into(),
                quantity: 1,
                outline: vec![[0., 0.], [60., 0.], [60., 40.], [0., 40.]],
                fold_line: Some([[0., 20.], [60., 20.]]),
                ..Default::default()
            }],
            ..Default::default()
        };
        let mut session = BrowserSession::new(job.clone(), options())?;
        let p = session.step()?;
        assert!(beta_marker_core::validate(&job, &p.result).ok);
        assert!(p.result.engine.contains("bounded"));
        Ok(())
    }
}
