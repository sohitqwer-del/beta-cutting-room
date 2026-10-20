//! Independent job/width trials, retaining completed successes across resume.
use crate::*;
use beta_marker_core::Job;
use beta_marker_nest::NestOptions;

fn descriptors(jobs: &[Job], widths: &[f64]) -> Result<Vec<(usize, f64)>> {
    ensure!(!jobs.is_empty(), "Batch needs at least one job.");
    let mut unique = Vec::new();
    for &width in widths {
        ensure!(
            width.is_finite() && width > 0.0,
            "Batch widths must be finite and positive."
        );
        if !unique.contains(&width) {
            unique.push(width);
        }
    }
    let mut trials = Vec::new();
    for (i, job) in jobs.iter().enumerate() {
        if unique.is_empty() {
            ensure!(
                job.fabric.width.is_finite() && job.fabric.width > 0.0,
                "Source width must be positive."
            );
            trials.push((i, job.fabric.width));
        } else {
            trials.extend(unique.iter().map(|width| (i, *width)));
        }
    }
    Ok(trials)
}

pub(crate) fn validate(jobs: &[Job], widths: &[f64], result: &BatchResult) -> Result<()> {
    let requested = descriptors(jobs, widths)?;
    let mut seen = std::collections::BTreeSet::new();
    for trial in &result.trials {
        ensure!(
            requested.contains(&(trial.job_index, trial.width_mm))
                && seen.insert((trial.job_index, trial.width_mm.to_bits())),
            "Unexpected/duplicate batch trial."
        );
        if let Some(marker) = &trial.result {
            let mut job = jobs[trial.job_index].clone();
            job.fabric.width = trial.width_mm;
            crate::engine::marker(&job, marker)?;
            ensure!(
                trial.error.is_none(),
                "Successful batch trial cannot contain a failure."
            );
        } else {
            ensure!(trial.error.is_some(), "Failed batch trial needs a reason.");
        }
    }
    ensure!(
        result.complete
            == (result.trials.len() == requested.len()
                && result.trials.iter().all(|t| t.result.is_some())),
        "Batch completeness does not reconcile."
    );
    Ok(())
}

pub(crate) fn execute(
    queue: &Queue,
    record: &JobRecord,
    jobs: &[Job],
    widths: &[f64],
    options: &NestOptions,
    run: RunOptions,
    progress: &mut impl FnMut(Progress),
) -> Result<String> {
    let requested = descriptors(jobs, widths)?;
    let id = &record.submission.id;
    let previous = match queue.checkpoint(id)? {
        Some(Checkpoint::Batch(b)) => b,
        _ => BatchResult::default(),
    };
    let mut result = previous.clone();
    let out = queue.root().join("results").join(id);
    for (index, &(job_index, width_mm)) in requested.iter().enumerate() {
        crate::engine::active(queue, id)?;
        if previous
            .trials
            .iter()
            .any(|t| t.job_index == job_index && t.width_mm == width_mm && t.result.is_some())
        {
            continue;
        }
        let mut job = jobs[job_index].clone();
        job.fabric.width = width_mm;
        let mut options = options.clone();
        options.time_limit_s = record.submission.time.seconds();
        let outcome = crate::engine::sliced_marker(queue, record, &job, &options, run, progress);
        crate::engine::active(queue, id)?;
        let trial = match outcome {
            Ok(marker) => BatchTrial {
                job_index,
                width_mm,
                result: Some(marker),
                error: None,
            },
            Err(e) => BatchTrial {
                job_index,
                width_mm,
                result: None,
                error: Some(format!("{e:#}")),
            },
        };
        atomic_json(&out.join(format!("trial-{:04}-job.json", index + 1)), &job)?;
        atomic_json(&out.join(format!("trial-{:04}.json", index + 1)), &trial)?;
        if let Some(i) = result
            .trials
            .iter()
            .position(|t| t.job_index == job_index && t.width_mm == width_mm)
        {
            result.trials[i] = trial;
        } else {
            result.trials.push(trial);
        }
        result.complete = result.trials.len() == requested.len()
            && result.trials.iter().all(|t| t.result.is_some());
        let saved = queue.save_checkpoint(id, &Checkpoint::Batch(result.clone()))?;
        crate::engine::emit(
            progress,
            id,
            format!("Batch trial {}/{} complete", index + 1, requested.len()),
            saved,
            None,
        );
    }
    result.complete =
        result.trials.len() == requested.len() && result.trials.iter().all(|t| t.result.is_some());
    queue.save_checkpoint(id, &Checkpoint::Batch(result.clone()))?;
    atomic_json(&out.join("result.json"), &result)?;
    let mut csv = "job_index,width_mm,length_mm,efficiency_pct,error\r\n".to_string();
    for t in &result.trials {
        csv.push_str(&format!(
            "{},{},{},{},\"{}\"\r\n",
            t.job_index,
            t.width_mm,
            t.result
                .as_ref()
                .map_or(String::new(), |m| m.length.to_string()),
            t.result
                .as_ref()
                .map_or(String::new(), |m| (m.efficiency * 100.0).to_string()),
            t.error.as_deref().unwrap_or("").replace('"', "\"\"")
        ));
    }
    atomic_bytes(&out.join("summary.csv"), csv.as_bytes())?;
    ensure!(
        result.complete,
        "Batch has isolated failures; checked successes and summary are saved."
    );
    Ok(format!(
        "Batch complete: {} valid job/width trials",
        result.trials.len()
    ))
}
