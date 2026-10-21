//! Matching search rounds using the existing browser-worker progress contract.
use crate::{Instant, nest_with_rules};
use anyhow::{Context, Result, ensure};
use beta_marker_core::{Job, MarkerResult};
use beta_marker_nest::{
    NestOptions,
    browser::{BrowserOptions, BrowserProgress, BrowserSession},
};

/// Browser matching session. A worker yields between independently checked rounds.
pub struct MatchingSession {
    job: Job,
    options: BrowserOptions,
    started: Option<Instant>,
    best: Option<MarkerResult>,
    round: u32,
    cancelled: bool,
    done: bool,
}
impl MatchingSession {
    pub fn new(job: Job, options: BrowserOptions) -> Result<Self> {
        // Share option, job and warm-start checks with the established browser path.
        BrowserSession::new(job.clone(), options.clone())?;
        Ok(Self {
            job,
            best: options.warm_start.clone(),
            options,
            started: None,
            round: 0,
            cancelled: false,
            done: false,
        })
    }
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
    pub fn step(&mut self) -> Result<BrowserProgress> {
        ensure!(
            !(self.cancelled && self.best.is_none()),
            "Search cancelled before a checked marker was available."
        );
        let started = *self.started.get_or_insert_with(Instant::now);
        let before = Instant::now();
        let mut improved = false;
        if !self.done && !self.cancelled {
            let remaining = self.options.time_limit_s - started.elapsed().as_secs_f64();
            if self.best.is_none() || remaining > 0. {
                let options = NestOptions {
                    time_limit_s: self.options.round_time_s.min(remaining.max(0.01)),
                    seed: Some(
                        self.options
                            .seed
                            .unwrap_or(42)
                            .wrapping_add(u64::from(self.round))
                            & ((1 << 53) - 1),
                    ),
                    workers: Some(1),
                    runs: 1,
                    early_termination: self.options.early_termination,
                    fixed: self.options.fixed.clone(),
                    warm_start: self.best.clone(),
                };
                match nest_with_rules(&self.job, &options) {
                    Ok(candidate) => {
                        let checked = beta_marker_core::validate(&self.job, &candidate);
                        ensure!(
                            checked.ok,
                            "Matching round failed independent validation: {:?}",
                            checked.errors
                        );
                        if self
                            .best
                            .as_ref()
                            .is_none_or(|old| candidate.length < old.length)
                        {
                            self.best = Some(candidate);
                            improved = true;
                        }
                    }
                    Err(error) if self.best.is_some() => {
                        if let Some(best) = self.best.as_mut() {
                            best.warnings.push(format!(
                                "Matching round stopped; checked marker retained: {error}"
                            ));
                        }
                        self.done = true;
                    }
                    Err(error) => return Err(error),
                }
                self.round += 1;
            }
        }
        let seconds = started.elapsed().as_secs_f64();
        self.done |= self.cancelled || seconds >= self.options.time_limit_s;
        let mut result = self
            .best
            .clone()
            .context("No checked matching marker is available.")?;
        result.runtime_s = seconds;
        Ok(BrowserProgress {
            round: self.round,
            seconds,
            round_seconds: before.elapsed().as_secs_f64(),
            improved,
            done: self.done,
            cancelled: self.cancelled,
            result,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn browser_matching_round_is_valid_and_cancellation_keeps_it() {
        let job = crate::tests::job();
        let mut session = MatchingSession::new(
            job.clone(),
            BrowserOptions {
                time_limit_s: 1.,
                round_time_s: 0.2,
                seed: Some(42),
                ..Default::default()
            },
        )
        .unwrap();
        let first = session.step().unwrap();
        assert!(beta_marker_core::validate(&job, &first.result).ok);
        assert!(crate::match_errors(&job, &first.result).is_empty());
        session.cancel();
        let last = session.step().unwrap();
        assert!(last.done && last.cancelled);
        assert_eq!(first.result.placements, last.result.placements);
    }
}
