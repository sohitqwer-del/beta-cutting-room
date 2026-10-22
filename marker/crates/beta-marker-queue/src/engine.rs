//! Serial typed worker. No persisted command can execute arbitrary code.
use crate::*;
use beta_marker_core::{Job, MarkerResult, Order};
use beta_marker_plan::{CutOrder, CutPlan, PlanOptions};
use std::collections::BTreeMap;

fn invoke(
    job: &Job,
    options: &beta_marker_nest::NestOptions,
    progress: impl FnMut(beta_marker_nest::NestProgress),
) -> Result<MarkerResult> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if beta_marker_match::needs_rules(job) {
            beta_marker_match::nest_with_rules(job, options)
        } else {
            beta_marker_nest::nest_with_progress(job, options, progress)
        }
    }))
    .map_err(|payload| {
        let message = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| payload.downcast_ref::<&str>().copied())
            .unwrap_or("worker panic");
        anyhow::anyhow!("Engine/callback panicked; discarded slice: {message}")
    })?
}

pub(crate) fn construction_guard(
    queue: &Queue,
    id: &str,
    done: std::sync::mpsc::Receiver<()>,
    timeout: std::time::Duration,
) {
    if matches!(
        done.recv_timeout(timeout),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout)
    ) {
        let _=queue.update(id,|r|{
            r.construction_guard_triggered=true;
            if r.status==Status::Running {r.summary="First layout exceeded the construction guard; continuous fallback active (no restart).".into();}
            Ok(())
        });
    }
}

fn continuous_call(
    queue: &Queue,
    record: &JobRecord,
    job: &Job,
    options: &beta_marker_nest::NestOptions,
    progress: &mut impl FnMut(Progress),
) -> Result<MarkerResult> {
    let id = &record.submission.id;
    let cold = options.warm_start.is_none();
    queue.update(id, |r| {
        if r.status == Status::Running {
            r.summary = if cold {
                "Construct once, then continuous search; short slicing disabled."
            } else {
                "Continuous warm search; construction state is not restarted for checkpoints."
            }
            .into();
        }
        Ok(())
    })?;
    emit(progress,id,"Continuous engine call: checkpoint slicing is disabled until the engine can retain construction state.".into(),false,None);
    active(queue, id)?;
    let started = std::time::Instant::now();
    let mut first = None;
    let outcome = std::thread::scope(|scope| {
        let (send, receive) = std::sync::mpsc::channel();
        if cold {
            scope.spawn(|| {
                construction_guard(queue, id, receive, std::time::Duration::from_secs(60))
            });
        }
        let result = invoke(job, options, |p| {
            if first.is_none() {
                first = Some(p.seconds);
                let _ = send.send(());
            }
            emit(
                progress,
                id,
                "Feasible engine improvement; geometry saves at the continuous-call boundary."
                    .into(),
                false,
                Some(p.length),
            );
        });
        let _ = send.send(());
        result
    });
    let elapsed = started.elapsed().as_secs_f64();
    queue.update(id, |r| {
        r.worker_seconds += elapsed * options.workers.unwrap_or(2) as f64;
        if cold && let Some(observed) = first.or_else(|| outcome.as_ref().ok().map(|_| elapsed)) {
            r.first_layout_s = Some(r.first_layout_s.map_or(observed, |old| old.max(observed)));
            r.construction_guard_triggered |= observed > 60.0;
        }
        Ok(())
    })?;
    if queue.status(id)?.construction_guard_triggered {
        emit(progress,id,"Construction guard exceeded: retaining one continuous run; no construction retry or short slice.".into(),false,None);
    }
    outcome
}

pub(crate) fn marker(job: &Job, result: &MarkerResult) -> Result<()> {
    let checked = beta_marker_core::validate(job, result);
    ensure!(
        checked.ok,
        "Checkpoint geometry failed validation: {:?}",
        checked.errors
    );
    ensure!(
        (checked.length - result.length).abs() <= 0.01 && result.width == job.fabric.width,
        "Checkpoint metrics do not match geometry."
    );
    Ok(())
}
fn checked_plan(order: &CutOrder, plan: &CutPlan) -> Result<()> {
    ensure!(
        plan.ordered == order.quantities && plan.order_name == order.name,
        "Checkpoint belongs to a different order."
    );
    let mut totals = BTreeMap::<String, u64>::new();
    let mut fabric = 0.0;
    for lay in &plan.lays {
        ensure!(
            lay.passes > 0
                && lay.passes <= order.max_passes()
                && lay.ratio.values().map(|n| u64::from(*n)).sum::<u64>()
                    <= u64::from(order.max_garments_per_marker),
            "Checkpoint lay exceeds order limits."
        );
        let mut job = order.job.clone();
        job.order = Some(Order {
            sizes: lay.ratio.clone(),
        });
        let result = lay
            .result
            .as_ref()
            .context("Checkpoint lay has no geometry")?;
        marker(&job, result)?;
        ensure!(
            (lay.marker_length_mm - result.length).abs() <= 0.01,
            "Checkpoint lay length mismatch."
        );
        for (size, n) in &lay.ratio {
            *totals.entry(size.clone()).or_default() += u64::from(*n) * u64::from(lay.passes);
        }
        fabric +=
            (result.length + 2.0 * plan.options.end_allowance_mm) * f64::from(lay.passes) / 1000.0;
    }
    ensure!(
        (fabric - plan.total_fabric_m).abs() <= 1e-6 * (1.0 + fabric),
        "Checkpoint fabric does not reconcile."
    );
    for (size, &n) in &order.quantities {
        let made = totals.remove(size).unwrap_or(0);
        ensure!(
            made == u64::from(plan.planned.get(size).copied().unwrap_or(0)),
            "Checkpoint quantities do not reconcile."
        );
        ensure!(
            made as f64 <= (f64::from(n) * (1.0 + order.overcut_pct / 100.0)).floor(),
            "Checkpoint exceeds allowed over-cut."
        );
    }
    ensure!(
        totals.values().all(|n| *n == 0),
        "Checkpoint has unordered sizes."
    );
    ensure!(
        plan.complete
            == order
                .quantities
                .iter()
                .all(|(s, n)| plan.planned.get(s).copied().unwrap_or(0) >= *n),
        "Checkpoint completeness mismatch."
    );
    Ok(())
}
pub(crate) fn validate_checkpoint(task: &Task, value: &Checkpoint) -> Result<()> {
    match (task, value) {
        (Task::Nest { job, .. }, Checkpoint::Marker(result)) => marker(job, result),
        (
            Task::Batch {
                jobs, widths_mm, ..
            },
            Checkpoint::Batch(result),
        ) => crate::batch::validate(jobs, widths_mm, result),
        (Task::Plan { order, .. } | Task::Pipeline { order, .. }, Checkpoint::Plan(plan)) => {
            checked_plan(order, plan)
        }
        (Task::WidthTrials { request }, Checkpoint::Plan(plan)) => {
            let mut order = request.order.clone();
            if let Some(result) = plan.lays.first().and_then(|l| l.result.as_ref()) {
                ensure!(
                    request.offers.iter().any(|o| o.width_mm == result.width),
                    "Checkpoint width is not an offer."
                );
                order.job.fabric.width = result.width;
            }
            checked_plan(&order, plan)
        }
        (Task::WidthTrials { request }, Checkpoint::WidthTrials(comparison)) => {
            for trial in &comparison.trials {
                if let Some(plan) = &trial.plan {
                    let mut order = request.order.clone();
                    order.job.fabric.width = trial.width_mm;
                    checked_plan(&order, plan)?;
                }
                if let Some(spread) = &trial.spread {
                    let register = request
                        .offers
                        .get(trial.offer_index)
                        .and_then(|o| o.rolls.as_ref())
                        .context("No stock for spread")?;
                    beta_marker_roll::validate_plan(register, spread)?;
                }
            }
            if let Some(i) = comparison.chosen {
                let selected = comparison
                    .trials
                    .get(i)
                    .context("Chosen trial index is out of range")?;
                ensure!(
                    selected.feasible && selected.final_evaluated,
                    "Chosen width has no feasible final evaluation."
                );
            }
            Ok(())
        }
        _ => anyhow::bail!("Checkpoint kind does not match the queue task."),
    }
}
pub(crate) fn better(new: &Checkpoint, old: &Checkpoint) -> bool {
    match (new, old) {
        (Checkpoint::Marker(n), Checkpoint::Marker(o)) => n.length < o.length,
        (Checkpoint::Batch(n), Checkpoint::Batch(o)) => {
            let valid = |b: &BatchResult| b.trials.iter().filter(|t| t.result.is_some()).count();
            valid(n) > valid(o) || (valid(n) == valid(o) && n.trials.len() >= o.trials.len())
        }
        (Checkpoint::Plan(n), Checkpoint::Plan(o)) => {
            let missing = |p: &CutPlan| p.shortfall.values().map(|n| u64::from(*n)).sum::<u64>();
            missing(n) < missing(o) || (missing(n) == missing(o) && n.objective_m < o.objective_m)
        }
        (Checkpoint::WidthTrials(_), _) => true,
        (_, Checkpoint::WidthTrials(_)) => false,
        _ => true,
    }
}
pub(crate) fn active(queue: &Queue, id: &str) -> Result<()> {
    ensure!(
        queue.status(id)?.status == Status::Running,
        "Job paused or cancelled; checkpoint retained."
    );
    Ok(())
}
pub(crate) fn emit(
    progress: &mut impl FnMut(Progress),
    id: &str,
    message: String,
    saved: bool,
    length: Option<f64>,
) {
    progress(Progress {
        job_id: id.into(),
        status: Status::Running,
        message,
        best_length_mm: length,
        checkpoint_saved: saved,
    });
}
fn plan_options(record: &JobRecord, opts: &PlanOptions, run: RunOptions) -> PlanOptions {
    let mut options = opts.clone();
    // Tiers apply per marker for a plan, matching time-per-marker in the CLI.
    // The injected executor divides these budgets into cancellable slices.
    let _ = run;
    options.time_per_marker_s = options
        .time_per_marker_s
        .min(record.submission.time.seconds());
    options.final_time_s = record.submission.time.seconds();
    options
}
fn plan_job(
    queue: &Queue,
    record: &JobRecord,
    order: &CutOrder,
    opts: &PlanOptions,
    run: RunOptions,
    progress: &mut impl FnMut(Progress),
) -> Result<CutPlan> {
    ensure!(
        run.max_workers >= 2,
        "Planning needs two workers; raise max_workers to at least 2."
    );
    let id = &record.submission.id;
    let options = plan_options(record, opts, run);
    let progress = std::cell::RefCell::new(progress);
    let mut callback = |plan: &CutPlan| {
        let saved = queue.save_checkpoint(id, &Checkpoint::Plan(plan.clone()))?;
        emit(
            &mut |e| (progress.borrow_mut())(e),
            id,
            format!(
                "{} lays, {:.3} m, complete={}",
                plan.lays.len(),
                plan.total_fabric_m,
                plan.complete
            ),
            saved,
            None,
        );
        active(queue, id)
    };
    let mut engine = |job: &Job, opts: &beta_marker_nest::NestOptions| {
        sliced_marker(queue, record, job, opts, run, &mut |e| {
            (progress.borrow_mut())(e)
        })
    };
    let plan = match queue.checkpoint(id)? {
        Some(Checkpoint::Plan(plan)) if plan.complete => beta_marker_plan::refine_plan_with_engine(
            order,
            &options,
            &plan,
            &mut callback,
            &mut engine,
        )?,
        _ => beta_marker_plan::plan_ratios_with_engine(
            order,
            &options,
            &[],
            &mut callback,
            &mut engine,
        )?,
    };
    ensure!(
        plan.complete,
        "Order has a quantity shortfall; checked partial plan retained."
    );
    // Return the best durable plan, including an earlier resumed incumbent.
    match queue.checkpoint(id)? {
        Some(Checkpoint::Plan(best)) => Ok(best),
        _ => Ok(plan),
    }
}

pub(crate) fn sliced_marker(
    queue: &Queue,
    record: &JobRecord,
    job: &Job,
    options: &beta_marker_nest::NestOptions,
    run: RunOptions,
    progress: &mut impl FnMut(Progress),
) -> Result<MarkerResult> {
    let id = &record.submission.id;
    // Stable FNV-1a name; the complete embedded job is checked on cache hits so
    // a hash collision cannot cause an incompatible geometry reuse.
    let bytes = serde_json::to_vec(job)?;
    let hash = bytes.iter().fold(0xcbf29ce484222325u64, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
    });
    let path = queue
        .root
        .join("results")
        .join(id)
        .join("markers")
        .join(format!("{hash:016x}.json"));
    let mut best = options.warm_start.clone();
    if path.exists() {
        let stored: RatioCheckpoint = read(&path)?;
        if stored.job == *job {
            marker(job, &stored.result)?;
            if best
                .as_ref()
                .is_none_or(|m| stored.result.length < m.length)
            {
                best = Some(stored.result);
            }
        }
    }
    let total = options.time_limit_s.min(record.submission.time.seconds());
    let mut used = 0.0;
    while used + 1e-9 < total {
        active(queue, id)?;
        let mut opts = options.clone();
        // Warm starts still rebuild engine state. Until a persistent session or
        // full-geometry callback exists, never restart phases just to checkpoint.
        opts.time_limit_s = total - used;
        opts.workers = Some(run.max_workers.min(2));
        opts.runs = 1;
        opts.warm_start = best.clone();
        opts.seed = Some(options.seed.unwrap_or(42));
        let outcome = continuous_call(queue, record, job, &opts, progress);
        queue.update(id, |r| {
            r.search_seconds += opts.time_limit_s;
            Ok(())
        })?;
        let result = outcome?;
        marker(job, &result)?;
        let improved = best.as_ref().is_none_or(|m| result.length < m.length);
        if improved {
            best = Some(result);
        }
        let retained = best.as_ref().context("No feasible marker")?;
        atomic_json(
            &path,
            &RatioCheckpoint {
                job: job.clone(),
                result: retained.clone(),
            },
        )?;
        emit(
            progress,
            id,
            format!(
                "Ratio checkpoint {} after {:.2} search s",
                path.file_name().unwrap_or_default().to_string_lossy(),
                used + opts.time_limit_s
            ),
            true,
            Some(retained.length),
        );
        used += opts.time_limit_s;
    }
    active(queue, id)?;
    best.context("No valid marker produced")
}

pub(crate) fn execute(
    queue: &Queue,
    record: &JobRecord,
    run: RunOptions,
    progress: &mut impl FnMut(Progress),
) -> Result<String> {
    let id = &record.submission.id;
    let out = queue.root.join("results").join(id);
    fs::create_dir_all(&out)?;
    active(queue, id)?;
    match &record.submission.task {
        Task::Batch {
            jobs,
            widths_mm,
            options,
        } => crate::batch::execute(queue, record, jobs, widths_mm, options, run, progress),
        Task::Nest { job, options } => {
            let mut best = match queue.checkpoint(id)? {
                Some(Checkpoint::Marker(m)) => {
                    marker(job, &m)?;
                    Some(m)
                }
                _ => None,
            };
            let mut used = record.search_seconds;
            let total = record.submission.time.seconds();
            while used + 1e-9 < total {
                active(queue, id)?;
                let mut opts = options.clone();
                opts.time_limit_s = total - used;
                opts.workers = Some(run.max_workers.min(2));
                opts.runs = 1;
                opts.seed = Some(options.seed.unwrap_or(42));
                opts.warm_start = best.clone().or_else(|| options.warm_start.clone());
                let outcome = continuous_call(queue, record, job, &opts, progress);
                match outcome {
                    Ok(result) => {
                        marker(job, &result)?;
                        let saved =
                            queue.save_checkpoint(id, &Checkpoint::Marker(result.clone()))?;
                        if best.as_ref().is_none_or(|b| result.length < b.length) {
                            best = Some(result);
                        }
                        used += opts.time_limit_s;
                        queue.update(id, |r| {
                            r.search_seconds = used;
                            Ok(())
                        })?;
                        emit(
                            progress,
                            id,
                            format!("Saved best valid marker; {:.2}/{total:.2} search s", used),
                            saved,
                            best.as_ref().map(|b| b.length),
                        );
                    }
                    Err(e) => {
                        return Err(e)
                            .context("Nest slice failed; retained earlier valid checkpoint");
                    }
                }
            }
            active(queue, id)?;
            let best = best.context("No valid marker produced")?;
            atomic_json(&out.join("result.json"), &best)?;
            Ok(format!(
                "Complete: {:.3} m, {:.2}% efficiency; {:.2} search s",
                best.length / 1000.0,
                best.efficiency * 100.0,
                used
            ))
        }
        Task::Plan { order, options } => {
            let plan = plan_job(queue, record, order, options, run, progress)?;
            atomic_json(&out.join("result.json"), &plan)?;
            Ok(format!(
                "Complete order: {} lays, {:.3} m",
                plan.lays.len(),
                plan.total_fabric_m
            ))
        }
        Task::WidthTrials { request } => {
            ensure!(
                run.max_workers >= 2,
                "Width planning requires at least two workers."
            );
            let mut request = request.clone();
            request.options = plan_options(record, &request.options, run);
            request.final_time_s = request
                .final_time_s
                .min(record.submission.time.seconds())
                .max(request.options.time_per_marker_s);
            let progress = std::cell::RefCell::new(progress);
            let mut engine = |job: &Job, options: &beta_marker_nest::NestOptions| {
                sliced_marker(queue, record, job, options, run, &mut |e| {
                    (progress.borrow_mut())(e)
                })
            };
            let comparison = beta_marker_plan::cost::compare_with_engine(
                &request,
                |plan| {
                    let saved = queue.save_checkpoint(id, &Checkpoint::Plan(plan.clone()))?;
                    emit(
                        &mut |e| (progress.borrow_mut())(e),
                        id,
                        "Checked width-trial plan".into(),
                        saved,
                        None,
                    );
                    active(queue, id)
                },
                &mut engine,
            )?;
            queue.save_checkpoint(id, &Checkpoint::WidthTrials(comparison.clone()))?;
            atomic_json(&out.join("result.json"), &comparison)?;
            ensure!(
                comparison.chosen.is_some(),
                "No feasible finally evaluated width; see result.json."
            );
            Ok(comparison.reason)
        }
        Task::Pipeline {
            order,
            options,
            rolls,
            eligible_roll_ids,
            colour,
            report,
        } => {
            rolls.validate()?;
            let eligible: std::collections::BTreeSet<_> = rolls
                .rolls
                .iter()
                .filter(|r| {
                    r.usable_width_mm >= order.job.fabric.width
                        && (eligible_roll_ids.is_empty() || eligible_roll_ids.contains(&r.id))
                })
                .map(|r| r.id.as_str())
                .collect();
            ensure!(
                !eligible.is_empty(),
                "No approved roll fits the marker width."
            );
            let bands: std::collections::BTreeSet<_> =
                beta_marker_roll::group_rolls(rolls, &Default::default())?
                    .into_iter()
                    .filter(|g| g.roll_ids.iter().any(|id| eligible.contains(id.as_str())))
                    .map(|g| (g.length_band, g.width_band))
                    .collect();
            ensure!(
                bands.len() <= 1,
                "Pipeline stock spans incompatible shrinkage bands; supply approved roll IDs from one band pair."
            );
            for roll in rolls.rolls.iter().filter(|r| {
                r.usable_width_mm >= order.job.fabric.width
                    && (eligible_roll_ids.is_empty() || eligible_roll_ids.contains(&r.id))
            }) {
                ensure!(
                    roll.shrinkage.length_pct <= order.job.fabric.shrinkage.length_pct + 1e-9
                        && roll.shrinkage.width_pct <= order.job.fabric.shrinkage.width_pct + 1e-9,
                    "Selected stock requires more shrinkage compensation than the embedded order."
                );
            }
            let plan = plan_job(queue, record, order, options, run, progress)?;
            active(queue, id)?;
            let requests = beta_marker_plan::spreading_requests(
                &plan,
                order.job.fabric.width,
                eligible_roll_ids,
            )?;
            let spread = beta_marker_roll::allocate(rolls, &requests, &Default::default())?;
            beta_marker_roll::validate_plan(rolls, &spread)?;
            atomic_json(&out.join("cutplan.json"), &plan)?;
            atomic_json(&out.join("spreading-plan.json"), &spread)?;
            ensure!(
                spread.complete,
                "Stock shortage; checked plan and partial allocation saved."
            );
            // Validate all defects before producing any production output.
            for lay in &spread.lays {
                let index = requests
                    .iter()
                    .position(|r| r.id == lay.lay.id)
                    .context("Unknown lay")?;
                let mut job = order.job.clone();
                job.order = Some(Order {
                    sizes: plan.lays[index].ratio.clone(),
                });
                let result = plan.lays[index].result.as_ref().context("Missing marker")?;
                let map = beta_marker_roll::map_defects(rolls, lay, &Default::default())?;
                atomic_json(&out.join(format!("{}-defects.json", lay.lay.id)), &map)?;
                ensure!(
                    !map.requires_replanning,
                    "Allocated defects require replacement/replanning."
                );
                for mut zone in map.zones {
                    for point in &mut zone.polygon {
                        point[0] -= plan.options.end_allowance_mm;
                    }
                    job.fabric.defects.push(zone);
                }
                marker(&job, result)?;
            }
            let mut bundles = Vec::new();
            let mut first_bundle = 1;
            for lay in &spread.lays {
                active(queue, id)?;
                let index = requests
                    .iter()
                    .position(|r| r.id == lay.lay.id)
                    .context("Unknown lay")?;
                let mut job = order.job.clone();
                job.order = Some(Order {
                    sizes: plan.lays[index].ratio.clone(),
                });
                let result = plan.lays[index].result.as_ref().context("Missing marker")?;
                let physical = beta_marker_cutroom::integration::from_spread(
                    &job,
                    result,
                    rolls,
                    lay,
                    &order.name,
                    colour,
                )?;
                let parts = beta_marker_cutroom::bundles::bundle_lay(
                    &physical,
                    beta_marker_cutroom::bundles::BundleRules {
                        first_bundle,
                        ..Default::default()
                    },
                )?;
                first_bundle = first_bundle
                    .checked_add(u32::try_from(parts.len())?)
                    .context("Bundle id overflow")?;
                bundles.extend(parts);
                atomic_json(&out.join(format!("{}-job.json", lay.lay.id)), &job)?;
                atomic_json(&out.join(format!("{}-result.json", lay.lay.id)), result)?;
                for format in ["plt", "iso", "mini"] {
                    let (bytes, _, extension) =
                        beta_marker_export::export(format, &job, result, &Default::default())?;
                    atomic_bytes(
                        &out.join(format!("{}-{}.{}", lay.lay.id, format, extension)),
                        &bytes,
                    )?;
                }
            }
            atomic_json(&out.join("bundles.json"), &bundles)?;
            atomic_bytes(
                &out.join("tickets.pdf"),
                &beta_marker_cutroom::tickets::ticket_pdf(
                    &bundles,
                    beta_marker_cutroom::tickets::TicketPaper::A4,
                )?,
            )?;
            active(queue, id)?;
            atomic_bytes(
                &out.join("stickers.pdf"),
                &beta_marker_cutroom::tickets::sticker_pdf(
                    &bundles,
                    beta_marker_cutroom::tickets::TicketPaper::A4,
                )?,
            )?;
            atomic_json(&out.join("result.json"), &plan)?;
            if let Some(settings) = report {
                active(queue, id)?;
                let pack = beta_marker_cutroom::production::build(
                    &beta_marker_cutroom::production::ReportInput {
                        meta: settings.meta.clone(),
                        job: order.job.clone(),
                        plan: plan.clone(),
                        gsm: settings.gsm,
                        currency: settings.currency.clone(),
                        price_per_m: settings.price_per_m,
                        lay_cost: settings.lay_cost,
                        purchase_wastage_pct: settings.purchase_wastage_pct,
                        register: Some(rolls.clone()),
                        spread: Some(spread.clone()),
                        comparison: None,
                        actuals: None,
                    },
                )?;
                atomic_json(&out.join("report.json"), &pack)?;
                atomic_bytes(
                    &out.join("report.xlsx"),
                    &beta_marker_cutroom::production::xlsx(&pack)?,
                )?;
                atomic_bytes(
                    &out.join("report.pdf"),
                    &beta_marker_cutroom::production::pdf(&pack)?,
                )?;
                for (name, csv) in beta_marker_cutroom::production::csv_files(&pack) {
                    atomic_bytes(&out.join(name), csv.as_bytes())?;
                }
            } else {
                let table = beta_marker_export::CutPlanTable {
                    headers: [
                        "Lay",
                        "Ratio",
                        "Plies",
                        "Passes",
                        "Marker m",
                        "Efficiency %",
                        "Planned fabric m",
                    ]
                    .map(String::from)
                    .to_vec(),
                    rows: plan
                        .lays
                        .iter()
                        .enumerate()
                        .map(|(i, l)| {
                            vec![
                                format!("L{:03}", i + 1),
                                l.ratio
                                    .iter()
                                    .map(|(s, n)| format!("{s}:{n}"))
                                    .collect::<Vec<_>>()
                                    .join(" "),
                                l.plies.to_string(),
                                l.passes.to_string(),
                                (l.marker_length_mm / 1000.0).to_string(),
                                (l.efficiency * 100.0).to_string(),
                                l.fabric_m.to_string(),
                            ]
                        })
                        .collect(),
                };
                atomic_bytes(
                    &out.join("report.xlsx"),
                    &beta_marker_export::to_tables_xlsx(&[("Planned production", table)])?,
                )?;
            }
            Ok(format!(
                "Pipeline complete: {} lays, {} bundles, {:.3} m",
                plan.lays.len(),
                bundles.len(),
                spread.total_used_m
            ))
        }
    }
}
