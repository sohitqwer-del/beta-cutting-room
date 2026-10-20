//! Public-only folded-fabric study. Three frozen seeds, adjacent before/after
//! calls, alternating arm order, and independently checked results.
use super::*;

#[derive(Clone)]
struct Case {
    name: String,
    job: Job,
    seed: u64,
    paired: bool,
}

fn inputs(fixtures: &Path) -> Result<Vec<(String, Job, bool)>> {
    let mut cases = Vec::new();
    for name in ["teagan", "simon", "titan"] {
        let job: Job =
            serde_json::from_reader(File::open(fixtures.join(format!("{name}-ratio.job.json")))?)?;
        // Open comparators use the new executable once. Ordinary-code regression
        // is measured separately with paired development tasks.
        cases.push((format!("{name}-open"), job.clone(), false));
        cases.push((
            format!("{name}-folded"),
            folded(&job, Layout::Folded)?,
            true,
        ));
        if name == "teagan" {
            cases.push((
                format!("{name}-tubular"),
                folded(&job, Layout::Tubular)?,
                true,
            ));
        }
    }
    for (name, width, sleeves, garments) in [
        ("synthetic-tee450", 450.0, true, 4),
        ("synthetic-tank-body525", 525.0, false, 6),
        ("synthetic-polo-body600", 600.0, true, 6),
    ] {
        let mut job: Job =
            serde_json::from_reader(File::open(fixtures.join("teagan-M.job.json"))?)?;
        job.name = format!("{name}: hypothetical Teagan knit body");
        job.fabric.width = width * 2.0;
        job.order = Some(beta_marker_core::Order {
            sizes: std::collections::BTreeMap::from([("M".into(), garments)]),
        });
        job.pieces.retain(|p| {
            matches!(p.name.as_str(), "front" | "back") || (sleeves && p.name == "sleeve")
        });
        job.meta.insert("study".into(), serde_json::json!({
            "source": "FreeSewing Teagan M, MIT; original outlines unchanged",
            "synthetic": true, "flat_tube_width_mm": width,
            "note": "Body packing scenario only; tank omits sleeves, polo omits collar/placket. Not a complete new sewing pattern."
        }));
        let mut tube = folded(&job, Layout::Tubular)?;
        // `folded` recognizes Teagan in the synthetic name above.
        tube.name.clone_from(&job.name);
        cases.push((format!("{name}-open"), job, false));
        cases.push((format!("{name}-tubular"), tube, true));
    }
    Ok(cases)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn matrix(
    before: &Path,
    after: &Path,
    fixtures: &Path,
    dev: &Path,
    out: &Path,
    seconds: f64,
    concurrency: usize,
) -> Result<()> {
    ensure!(!out.exists(), "use a fresh output directory");
    ensure!(
        [60.0, 300.0].contains(&seconds),
        "study budgets are 60 or 300 seconds"
    );
    ensure!(
        (1..=2).contains(&concurrency),
        "at most two concurrent trials (one separator worker each)"
    );
    fs::create_dir_all(out)?;
    let mut definitions = inputs(fixtures)?;
    if seconds == 60.0 {
        for (i, line) in BufReader::new(File::open(dev)?)
            .lines()
            .take(10)
            .enumerate()
        {
            let value: serde_json::Value = serde_json::from_str(&line?)?;
            let id = value["id"]
                .as_str()
                .map_or_else(|| value["id"].to_string(), str::to_owned);
            ensure!(
                id.chars().all(|c| c.is_ascii_digit()),
                "development task IDs must be numeric"
            );
            definitions.push((
                format!("lectra-dev-{i:02}-{id}"),
                serde_json::from_value(value["job"].clone())?,
                true,
            ));
        }
    }
    let mut cases = Vec::new();
    for seed in [1, 42, 2026] {
        for (name, job, paired) in &definitions {
            cases.push(Case {
                name: name.clone(),
                job: job.clone(),
                seed,
                paired: *paired,
            });
        }
    }
    save(
        out.join("protocol.json"),
        &serde_json::json!({
            "seconds": seconds, "seeds": [1,42,2026], "concurrency": concurrency,
            "workers_per_trial": 1, "before": before, "after": after,
            "jobs": definitions.iter().map(|(name, _, paired)| serde_json::json!({"name":name,"paired":paired})).collect::<Vec<_>>(),
            "notes": "All data public. Each before/after pair runs adjacently; order alternates. Failures remain in denominator. Open garment comparators run once; ten development tasks are paired."
        }),
    )?;
    let next = AtomicUsize::new(0);
    std::thread::scope(|scope| -> Result<()> {
        let mut workers = Vec::new();
        for _ in 0..concurrency {
            let next = &next;
            let cases = &cases;
            workers.push(scope.spawn(move || -> Result<()> {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(case) = cases.get(index) else { break };
                    let dir = out.join(format!("{}-seed{}", case.name, case.seed));
                    fs::create_dir_all(&dir)?;
                    save(dir.join("job.json"), &case.job)?;
                    save(
                        dir.join("options.json"),
                        &NestOptions {
                            time_limit_s: seconds,
                            seed: Some(case.seed),
                            workers: Some(1),
                            ..Default::default()
                        },
                    )?;
                    let mut arms = if case.paired {
                        vec![("before", before), ("after", after)]
                    } else {
                        vec![("after", after)]
                    };
                    if index % 2 == 1 {
                        arms.reverse();
                    }
                    for (arm, exe) in arms {
                        isolated(
                            exe,
                            &dir.join("job.json"),
                            &dir.join("options.json"),
                            &dir.join(arm),
                            seconds + 30.0,
                        )?;
                    }
                    eprintln!(
                        "finished {}/{} {} seed {}",
                        index + 1,
                        cases.len(),
                        case.name,
                        case.seed
                    );
                }
                Ok(())
            }));
        }
        for worker in workers {
            worker
                .join()
                .map_err(|_| anyhow::anyhow!("study worker panicked"))??;
        }
        Ok(())
    })?;
    let mut rows = Vec::new();
    for case in &cases {
        let dir = out.join(format!("{}-seed{}", case.name, case.seed));
        let after: Trial = serde_json::from_reader(File::open(dir.join("after/supervised.json"))?)?;
        let before: Option<Trial> = if case.paired {
            Some(serde_json::from_reader(File::open(
                dir.join("before/supervised.json"),
            )?)?)
        } else {
            None
        };
        rows.push(serde_json::json!({
            "case": case.name, "seed": case.seed, "seconds": seconds, "before": before, "after": after
        }));
    }
    save(out.join("rows.json"), &rows)?;
    Ok(())
}

#[derive(Serialize, Deserialize)]
struct Row {
    case: String,
    seed: u64,
    seconds: f64,
    before: Option<Trial>,
    after: Trial,
}

fn average(values: impl Iterator<Item = f64>) -> Option<f64> {
    let (sum, count) = values.fold((0.0, 0), |(sum, count), x| (sum + x, count + 1));
    (count > 0).then(|| sum / f64::from(count))
}

fn field(rows: &[&Row], before: bool, metric: fn(&Trial) -> Option<f64>) -> Option<f64> {
    average(rows.iter().filter_map(|row| {
        let trial = if before {
            row.before.as_ref()?
        } else {
            &row.after
        };
        trial.valid.then(|| metric(trial)).flatten()
    }))
}

fn number(value: Option<f64>, scale: f64) -> String {
    value.map_or_else(|| "—".into(), |x| format!("{:.4}", x * scale))
}

/// Produce tables from all completed predeclared trials, retaining failed pairs.
pub(super) fn summarize(root: &Path, out: &Path) -> Result<()> {
    use std::collections::BTreeMap;
    use std::fmt::Write;
    fs::create_dir_all(out)?;
    let mut grouped = BTreeMap::<(u32, String), Vec<Row>>::new();
    let mut completed_budgets = Vec::new();
    for seconds in [60, 300] {
        let path = root.join(format!("{seconds}/rows.json"));
        if !path.exists() {
            continue;
        }
        let rows: Vec<Row> = serde_json::from_reader(File::open(path)?)?;
        ensure!(!rows.is_empty(), "budget {seconds} has no trial rows");
        completed_budgets.push(seconds);
        for row in rows {
            grouped
                .entry((seconds, row.case.clone()))
                .or_default()
                .push(row);
        }
    }
    ensure!(!completed_budgets.is_empty(), "no completed study budgets");
    let mut text = String::new();
    let mut summary = Vec::new();
    for seconds in [60, 300] {
        if !completed_budgets.contains(&seconds) {
            continue;
        }
        writeln!(text, "### Layered jobs, {seconds} s\n")?;
        writeln!(
            text,
            "Three-seed arithmetic means. Surplus is flat spread area in m². Quality means use valid pairs; wall-time means include all attempts.\n"
        )?;
        writeln!(
            text,
            "| Case | Valid pairs | Useful % before → after | Gain pp | Occupied % before → after | Surplus m² before → after | Length m before → after | Wall s before → after |"
        )?;
        writeln!(text, "|---|---:|---:|---:|---:|---:|---:|---:|")?;
        for ((budget, name), rows) in &grouped {
            if *budget != seconds || name.starts_with("lectra-") || rows[0].before.is_none() {
                continue;
            }
            let valid: Vec<_> = rows
                .iter()
                .filter(|r| r.before.as_ref().is_some_and(|b| b.valid) && r.after.valid)
                .collect();
            let gain = average(
                valid
                    .iter()
                    .filter_map(|r| Some(r.after.useful_pct? - r.before.as_ref()?.useful_pct?)),
            );
            writeln!(
                text,
                "| {name} | {}/{} | {} → {} | {} | {} → {} | {} → {} | {} → {} | {} → {} |",
                valid.len(),
                rows.len(),
                number(field(&valid, true, |t| t.useful_pct), 1.),
                number(field(&valid, false, |t| t.useful_pct), 1.),
                number(gain, 1.),
                number(field(&valid, true, |t| t.occupied_pct), 1.),
                number(field(&valid, false, |t| t.occupied_pct), 1.),
                number(field(&valid, true, |t| t.surplus_area), 1e-6),
                number(field(&valid, false, |t| t.surplus_area), 1e-6),
                number(field(&valid, true, |t| t.length_m), 1.),
                number(field(&valid, false, |t| t.length_m), 1.),
                number(
                    average(
                        rows.iter()
                            .filter_map(|r| r.before.as_ref().map(|t| t.wall_s))
                    ),
                    1.
                ),
                number(average(rows.iter().map(|r| r.after.wall_s)), 1.)
            )?;
            summary.push(serde_json::json!({
                "case": name, "seconds": seconds, "valid_pairs": valid.len(), "attempted": rows.len(),
                "mean_gain_pp": gain, "mean_saved_m": average(valid.iter().filter_map(|r| Some(r.before.as_ref()?.length_m? - r.after.length_m?))),
                "before_wall_s": average(rows.iter().filter_map(|r| r.before.as_ref().map(|b| b.wall_s))),
                "after_wall_s": average(rows.iter().map(|r| r.after.wall_s)),
                "trials": rows
            }));
        }
        writeln!(text, "\n### Open comparators, {seconds} s\n")?;
        writeln!(
            text,
            "| Case | Valid / attempted | Useful and occupied % | Surplus m² | Length m | Wall s |"
        )?;
        writeln!(text, "|---|---:|---:|---:|---:|---:|")?;
        for ((budget, name), rows) in &grouped {
            if *budget != seconds || rows[0].before.is_some() {
                continue;
            }
            let valid: Vec<_> = rows.iter().filter(|r| r.after.valid).collect();
            writeln!(
                text,
                "| {name} | {}/{} | {} | {} | {} | {} |",
                valid.len(),
                rows.len(),
                number(field(&valid, false, |t| t.useful_pct), 1.),
                number(field(&valid, false, |t| t.surplus_area), 1e-6),
                number(field(&valid, false, |t| t.length_m), 1.),
                number(field(&valid, false, |t| Some(t.wall_s)), 1.)
            )?;
        }
    }
    writeln!(text, "\n### Open development regression, 60 s\n")?;
    writeln!(
        text,
        "| Task | Valid pairs | Useful % before → after | Mean gain pp | Win / tie / loss | Failed before / after |"
    )?;
    writeln!(text, "|---|---:|---:|---:|---:|---:|")?;
    let mut task_means = Vec::new();
    let mut all_gains = Vec::new();
    for ((_, name), rows) in grouped
        .iter()
        .filter(|((_, name), _)| name.starts_with("lectra-"))
    {
        let valid: Vec<_> = rows
            .iter()
            .filter(|r| r.before.as_ref().is_some_and(|b| b.valid) && r.after.valid)
            .collect();
        let gains: Vec<_> = valid
            .iter()
            .filter_map(|r| Some(r.after.useful_pct? - r.before.as_ref()?.useful_pct?))
            .collect();
        let mean = average(gains.iter().copied());
        if let Some(mean) = mean {
            task_means.push(mean);
        }
        all_gains.extend(gains.iter().copied());
        let (mut wins, mut ties, mut losses) = (0, 0, 0);
        for row in rows {
            let Some(before) = &row.before else { continue };
            match (before.valid, row.after.valid) {
                (true, true) => {
                    let gain = row.after.useful_pct.context("missing score")?
                        - before.useful_pct.context("missing score")?;
                    if gain > 0.1 {
                        wins += 1;
                    } else if gain < -0.1 {
                        losses += 1;
                    } else {
                        ties += 1;
                    }
                }
                (false, true) => wins += 1,
                (true, false) => losses += 1,
                _ => {}
            }
        }
        writeln!(
            text,
            "| {name} | {}/{} | {} → {} | {} | {wins}/{ties}/{losses} | {}/{} |",
            valid.len(),
            rows.len(),
            number(field(&valid, true, |t| t.useful_pct), 1.),
            number(field(&valid, false, |t| t.useful_pct), 1.),
            number(mean, 1.),
            rows.iter()
                .filter(|r| r.before.as_ref().is_some_and(|b| !b.valid))
                .count(),
            rows.iter().filter(|r| !r.after.valid).count()
        )?;
    }
    let mean = average(task_means.iter().copied());
    let open_valid: Vec<_> = grouped
        .iter()
        .filter(|((_, name), _)| name.starts_with("lectra-"))
        .flat_map(|(_, rows)| rows)
        .filter(|r| r.before.as_ref().is_some_and(|b| b.valid) && r.after.valid)
        .collect();
    let before_wall = field(&open_valid, true, |t| Some(t.wall_s));
    let after_wall = field(&open_valid, false, |t| Some(t.wall_s));
    let interval = mean.and_then(|mean| {
        let n = task_means.len();
        if n < 2 {
            return None;
        }
        let variance = task_means.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1) as f64;
        let critical = if n == 10 {
            2.262
        } else if n == 9 {
            2.306
        } else {
            2.776
        };
        let margin = critical * (variance / n as f64).sqrt();
        Some([mean - margin, mean + margin])
    });
    writeln!(
        text,
        "\nMean gain across valid development pairs: {} pp ({} pairs).",
        number(average(all_gains.iter().copied()), 1.),
        all_gains.len()
    )?;
    if let Some([low, high]) = interval {
        writeln!(
            text,
            "Mean of {} task means: {} pp; approximate 95% t interval [{low:.4}, {high:.4}] pp. This describes this selected subset, not the full benchmark population.",
            task_means.len(),
            number(mean, 1.)
        )?;
    }
    writeln!(
        text,
        "Mean wall time across valid development pairs: {} s before, {} s after.",
        number(before_wall, 1.),
        number(after_wall, 1.)
    )?;
    let failed: Vec<_> = grouped
        .values()
        .flatten()
        .filter(|r| !r.after.valid || r.before.as_ref().is_some_and(|b| !b.valid))
        .collect();
    save(out.join("failures.json"), &failed)?;
    save(
        out.join("summary.json"),
        &serde_json::json!({
            "completed_budgets": completed_budgets,
            "layered": summary, "open_task_means": task_means, "open_pair_gains": all_gains,
            "open_mean_wall_s": {"before": before_wall, "after": after_wall},
            "open_task_mean_interval": interval, "failures": failed.len()
        }),
    )?;
    fs::write(out.join("tables.md"), text)?;
    Ok(())
}
