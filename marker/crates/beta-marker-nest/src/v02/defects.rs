//! A portfolio for a short damaged start of an otherwise clean open lay.
//!
//! Preserve a checked obstacle layout, then let Sparrow optimize the clean
//! suffix. The discarded prefix is included in marker length and efficiency.
use super::{Slot, bounded, checks, directions, run_allocated};
use crate::{Instant, NestOptions, NestProgress, SearchTuning};
use anyhow::{Result, ensure};
use beta_marker_core::geometry::{area, bbox};
use beta_marker_core::{Job, Layout, MarkerResult};

pub(super) fn short_prefix(job: &Job, slots: &[Slot], opts: &NestOptions) -> Option<f64> {
    if job.fabric.layout != Layout::Open
        || job.fabric.defects.is_empty()
        || opts.time_limit_s < 1.0
        || job.fabric.shade.is_some()
        || job.fabric.repeat.is_some()
        || !job.match_rules.is_empty()
        || (!cfg!(target_arch = "wasm32")
            && std::thread::available_parallelism().map_or(true, |n| n.get() < 3))
    {
        return None;
    }
    // Avoid replacing the useful interior-defect search with a long blank
    // section. Only try this when all defects end within 5% of the area bound
    // and within a quarter of the fabric width from the start of the marker.
    let prefix = job
        .fabric
        .defects
        .iter()
        .map(|d| bbox(&d.polygon).map(|b| b[2]))
        .collect::<Option<Vec<_>>>()?
        .into_iter()
        .fold(0.0_f64, f64::max)
        + 0.01;
    let lower_bound = slots
        .iter()
        .map(|s| area(&job.pieces[s.piece].outline))
        .sum::<f64>()
        / job.fabric.width;
    (prefix <= (lower_bound * 0.05).min(job.fabric.width * 0.25)
        && job.fabric.max_length.is_none_or(|max| max > prefix))
    .then_some(prefix)
}

fn restore(job: &Job, mut result: MarkerResult, prefix: f64) -> Result<MarkerResult> {
    for pl in &mut result.placements {
        pl.translate[0] += prefix;
    }
    checks::refresh(job, &mut result)?;
    checks::independently_validate(job, &result)?;
    Ok(result)
}

pub(super) fn pack(
    job: &Job,
    slots: &[Slot],
    opts: &NestOptions,
    variant: usize,
    tuning: &SearchTuning,
    progress: &mut dyn FnMut(NestProgress),
    prefix: f64,
) -> Result<MarkerResult> {
    let started = Instant::now();
    let dirs = directions(job, slots, &[], variant)?;
    let mut seed_options = opts.clone();
    seed_options.time_limit_s *= 0.2;
    let mut best = bounded::pack(
        job,
        slots,
        &[],
        &seed_options,
        variant,
        &dirs,
        progress,
        None,
    )
    .ok()
    .or_else(|| opts.warm_start.clone());
    let remaining = opts.time_limit_s - started.elapsed().as_secs_f64();
    if remaining > 0.05 {
        let mut clean = job.clone();
        clean.fabric.defects.clear();
        clean.fabric.max_length = job.fabric.max_length.map(|max| max - prefix);
        let mut options = opts.clone();
        options.time_limit_s = remaining;
        options.warm_start = None;
        let mut accept = |candidate: MarkerResult| {
            if best
                .as_ref()
                .is_none_or(|old| candidate.length < old.length)
            {
                progress(NestProgress {
                    seconds: started.elapsed().as_secs_f64(),
                    length: candidate.length,
                    result: candidate.clone(),
                });
                best = Some(candidate);
            }
        };
        let result = run_allocated(
            &clean,
            slots,
            &[],
            &options,
            variant,
            tuning,
            &mut |event| {
                if let Ok(candidate) = restore(job, event.result, prefix) {
                    accept(candidate);
                }
            },
            None,
        );
        if let Ok(result) = result
            && let Ok(candidate) = restore(job, result, prefix)
        {
            accept(candidate);
        }
    }
    ensure!(
        best.is_some(),
        "neither obstacle packing nor the clean-suffix search found a complete marker"
    );
    let mut result = best.expect("checked above");
    result.runtime_s = started.elapsed().as_secs_f64();
    result.warnings.push(format!("Compared a clean-strip alternative after a {prefix:.2} mm damaged prefix; all prefix fabric is included in marker length and efficiency."));
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use beta_marker_core::{Fabric, JOB_FORMAT, Piece, Placement, RESULT_FORMAT, Zone};

    fn fixture() -> Job {
        Job {
            format: JOB_FORMAT.into(),
            name: "original prefix fixture".into(),
            fabric: Fabric {
                width: 100.0,
                defects: vec![Zone {
                    id: "start flaw".into(),
                    polygon: vec![[0., 0.], [1., 0.], [1., 100.], [0., 100.]],
                    note: None,
                }],
                ..Default::default()
            },
            pieces: vec![Piece {
                id: "panel".into(),
                quantity: 1,
                outline: vec![[0., 0.], [100., 0.], [100., 20.], [0., 20.]],
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[test]
    fn reserved_fabric_is_included_in_length_and_efficiency() -> Result<()> {
        let job = fixture();
        let result = MarkerResult {
            format: RESULT_FORMAT.into(),
            job_name: job.name.clone(),
            width: 100.,
            placements: vec![Placement {
                piece_id: "panel".into(),
                bundle: Some(0),
                ..Default::default()
            }],
            length: 0.,
            piece_area: 0.,
            efficiency: 0.,
            useful_piece_area: 0.,
            useful_yield: 0.,
            surplus_copies: 0,
            engine: "original test fixture".into(),
            seed: None,
            runtime_s: 0.,
            warnings: vec![],
        };
        let result = restore(&job, result, 1.01)?;
        assert!((result.length - 101.01).abs() < 1e-8);
        assert!((result.efficiency - 2000.0 / (100.0 * 101.01)).abs() < 1e-10);
        assert!(beta_marker_core::validate(&job, &result).ok);
        Ok(())
    }

    #[test]
    fn interior_defects_and_clean_jobs_do_not_select_a_blank_prefix() -> Result<()> {
        let mut job = fixture();
        let slots = super::super::slots(&job)?;
        let opts = NestOptions::default();
        job.fabric.defects[0]
            .polygon
            .iter_mut()
            .for_each(|p| p[0] += 30.0);
        assert!(short_prefix(&job, &slots, &opts).is_none());
        job.fabric.defects.clear();
        assert!(short_prefix(&job, &slots, &opts).is_none());
        Ok(())
    }
}
