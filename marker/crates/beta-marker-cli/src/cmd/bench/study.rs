//! Load-only audits and deterministic replacement of overloaded trials.
//! The score is never used to choose a retry or a replacement.
use super::{csv, runner::Replay};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

type Key = (String, u64);
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct LoadRow {
    pub id: String,
    pub seed: u64,
    pub arm: String,
    pub mean_cpu: Option<f64>,
    pub peak_cpu: Option<f64>,
    pub samples: usize,
    #[serde(default)]
    pub max_sampling_gap_s: f64,
    #[serde(default)]
    pub sampling_interrupted: bool,
    pub high_load: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct Audit {
    pub median_cpu: Option<f64>,
    pub median_absolute_deviation: Option<f64>,
    pub threshold_cpu: Option<f64>,
    pub outlier_tasks: Vec<Key>,
    pub rows: Vec<LoadRow>,
    pub rule: String,
}
struct Table {
    header: Vec<String>,
    rows: Vec<Vec<String>>,
}
impl Table {
    fn load(path: &Path) -> Result<Self> {
        let mut data = csv::records(&fs::read_to_string(path)?)?.into_iter();
        let header = data.next().context("empty CSV")?;
        let rows: Vec<_> = data.collect();
        ensure!(
            rows.iter().all(|r| r.len() == header.len()),
            "CSV column mismatch in {}",
            path.display()
        );
        Ok(Self { header, rows })
    }
    fn column(&self, name: &str) -> Result<usize> {
        self.header
            .iter()
            .position(|s| s == name)
            .with_context(|| format!("missing CSV column {name}"))
    }
    fn key(&self, row: &[String]) -> Result<Key> {
        Ok((
            row[self.column("id")?].clone(),
            row[self.column("seed")?].parse()?,
        ))
    }
    fn write(&self, path: &Path) -> Result<()> {
        let text = std::iter::once(&self.header)
            .chain(&self.rows)
            .map(|r| csv::encode(r))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        fs::write(path, text)?;
        Ok(())
    }
}
fn json(path: &Path) -> Result<Value> {
    let bytes = fs::read(path)?;
    Ok(serde_json::from_slice(
        bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes),
    )?)
}
fn median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut values = values.to_vec();
    values.sort_by(f64::total_cmp);
    Some((values[(values.len() - 1) / 2] + values[values.len() / 2]) / 2.0)
}
fn arms(root: &Path) -> Vec<(&'static str, PathBuf)> {
    if root.join("baseline/results.csv").exists() {
        vec![
            ("baseline", root.join("baseline")),
            ("variant", root.join("variant")),
        ]
    } else {
        vec![("run", root.to_owned())]
    }
}
fn cpu_samples(root: &Path) -> Result<Vec<(u64, f64)>> {
    let conditions = json(&root.join("conditions.json"))?;
    let samples = conditions
        .get("cpu_samples")
        .and_then(Value::as_array)
        .context("conditions.json has no cpu_samples")?;
    let mut out = Vec::new();
    for sample in samples {
        if let Some((time, cpu)) = sample
            .get("unix_ms")
            .and_then(Value::as_u64)
            .zip(sample.get("cpu_percent").and_then(Value::as_f64))
        {
            ensure!(
                cpu.is_finite() && (0.0..=100.0).contains(&cpu),
                "invalid CPU sample"
            );
            out.push((time, cpu));
        }
    }
    ensure!(!out.is_empty(), "no usable CPU samples");
    Ok(out)
}
fn window(samples: &[(u64, f64)], start: u64, end: u64) -> (Option<f64>, Option<f64>, usize) {
    let mut values: Vec<_> = samples
        .iter()
        .filter(|(t, _)| *t >= start && *t <= end)
        .map(|(_, v)| *v)
        .collect();
    if values.is_empty()
        && let Some((_, value)) = samples.iter().min_by_key(|(time, _)| time.abs_diff(start))
    {
        values.push(*value);
    }
    let mean = (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64);
    let peak = values.iter().copied().reduce(f64::max);
    (mean, peak, values.len())
}

fn sampling_gap_s(samples: &[(u64, f64)], start: u64, end: u64) -> f64 {
    let mut times: Vec<u64> = samples
        .iter()
        .map(|(t, _)| *t)
        .filter(|t| *t > start && *t < end)
        .collect();
    times.extend([start, end]);
    times.sort_unstable();
    times
        .windows(2)
        .map(|pair| pair[1].saturating_sub(pair[0]))
        .max()
        .unwrap_or(0) as f64
        / 1000.0
}

pub(super) fn audit(root: &Path, reference: Option<&Path>) -> Result<Audit> {
    let samples = cpu_samples(root)?;
    let mut rows = Vec::new();
    for (arm, path) in arms(root) {
        let table = Table::load(&path.join("results.csv"))?;
        let start = table.column("started_unix_ms")?;
        let end = table.column("finished_unix_ms")?;
        for row in &table.rows {
            let (id, seed) = table.key(row)?;
            let start_ms = row[start].parse()?;
            let end_ms = row[end].parse()?;
            ensure!(end_ms >= start_ms, "trial clock moved backwards");
            let (mean_cpu, peak_cpu, count) = window(&samples, start_ms, end_ms);
            let max_sampling_gap_s = sampling_gap_s(&samples, start_ms, end_ms);
            rows.push(LoadRow {
                id,
                seed,
                arm: arm.into(),
                mean_cpu,
                peak_cpu,
                samples: count,
                max_sampling_gap_s,
                sampling_interrupted: max_sampling_gap_s > 30.0,
                high_load: false,
            });
        }
    }
    let means: Vec<_> = rows.iter().filter_map(|r| r.mean_cpu).collect();
    let centre = median(&means);
    let deviations: Vec<_> = means
        .iter()
        .filter_map(|v| centre.map(|c| (v - c).abs()))
        .collect();
    let mad = median(&deviations);
    let threshold = if let Some(path) = reference {
        json(path)?.get("threshold_cpu").and_then(Value::as_f64)
    } else {
        centre
            .zip(mad)
            .map(|(centre, mad)| centre + (3.0 * mad).max(15.0))
    };
    let mut high = BTreeSet::new();
    for row in &mut rows {
        row.high_load = row.sampling_interrupted
            || row
                .mean_cpu
                .zip(threshold)
                .is_some_and(|(mean, threshold)| mean > threshold);
        if row.high_load {
            high.insert((row.id.clone(), row.seed));
        }
    }
    let audit=Audit {median_cpu:centre,median_absolute_deviation:mad,threshold_cpu:threshold,outlier_tasks:high.iter().cloned().collect(),rows,
        rule:"Repeat a task if mean CPU over a trial window > original run median + max(15 percentage points, 3 * median absolute deviation), or its telemetry window has a gap >30 seconds. Both arms of a flagged pair repeat. Original CPU cutoffs stay fixed. Scores never select retries.".into()};
    fs::write(
        root.join("load-audit.json"),
        serde_json::to_vec_pretty(&audit)?,
    )?;
    let mut replay: Replay = serde_json::from_value(json(&root.join("replay.json"))?)?;
    replay
        .tasks
        .retain(|t| high.contains(&(t.id.clone(), t.seed)));
    fs::write(root.join("load-rerun.json"), serde_json::to_vec(&replay)?)?;
    Ok(audit)
}

/// Preserve all raw trials. Use the first retry below the original load cutoff;
/// if none qualifies, retain the last retry and disclose it as a persistent outlier.
pub(super) fn finalize(root: &Path) -> Result<()> {
    let initial: Audit = serde_json::from_value(json(&root.join("load-audit.json"))?)?;
    let replay: Replay = serde_json::from_value(json(&root.join("replay.json"))?)?;
    let mut pending: BTreeSet<Key> = initial.outlier_tasks.iter().cloned().collect();
    let mut sources = vec![root.to_owned()];
    let retries = root.join("reruns");
    if retries.exists() {
        let mut paths: Vec<_> = fs::read_dir(retries)?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|p| p.join("load-audit.json").exists())
            .collect();
        paths.sort();
        sources.extend(paths);
    }
    let arm_names: Vec<_> = arms(root).into_iter().map(|(name, _)| name).collect();
    let mut tables = BTreeMap::<String, Table>::new();
    for (arm, path) in arms(root) {
        tables.insert(arm.into(), Table::load(&path.join("results.csv"))?);
    }
    let task_order: BTreeMap<Key, usize> = replay
        .tasks
        .iter()
        .enumerate()
        .map(|(i, t)| ((t.id.clone(), t.seed), i))
        .collect();
    ensure!(
        task_order.len() == replay.tasks.len(),
        "duplicate task id/seed cannot be load-audited"
    );
    for table in tables.values_mut() {
        let rows = std::mem::take(&mut table.rows);
        let mut ranked = Vec::new();
        for row in rows {
            let key = table.key(&row)?;
            ranked.push((*task_order.get(&key).context("unknown CSV task")?, row));
        }
        ranked.sort_by_key(|(index, _)| *index);
        ensure!(
            ranked.len() == replay.tasks.len(),
            "incomplete original CSV"
        );
        table.rows = ranked.into_iter().map(|(_, row)| row).collect();
    }
    let mut selected: BTreeMap<Key, (usize, usize)> = replay
        .tasks
        .iter()
        .enumerate()
        .map(|(i, t)| ((t.id.clone(), t.seed), (0, i)))
        .collect();
    let mut all_samples = cpu_samples(root)?;
    for (source_index, source) in sources.iter().enumerate().skip(1) {
        let report: Audit = serde_json::from_value(json(&source.join("load-audit.json"))?)?;
        let still_high: BTreeSet<Key> = report.outlier_tasks.into_iter().collect();
        let retried: Replay = serde_json::from_value(json(&source.join("replay.json"))?)?;
        all_samples.extend(cpu_samples(source)?);
        let source_tables: BTreeMap<_, _> = arms(source)
            .into_iter()
            .map(|(arm, path)| Ok((arm, Table::load(&path.join("results.csv"))?)))
            .collect::<Result<_>>()?;
        for (local_index, task) in retried.tasks.iter().enumerate() {
            let key = (task.id.clone(), task.seed);
            if !pending.contains(&key) {
                continue;
            }
            for &arm in &arm_names {
                let source_table = source_tables.get(arm).context("retry is missing an arm")?;
                let source_row = source_table
                    .rows
                    .iter()
                    .find(|row| source_table.key(row).ok().as_ref() == Some(&key))
                    .context("retry is missing a task")?;
                let target = tables.get_mut(arm).context("missing original arm")?;
                ensure!(
                    target.header == source_table.header,
                    "retry CSV schema changed"
                );
                let index = target
                    .rows
                    .iter()
                    .position(|row| target.key(row).ok().as_ref() == Some(&key))
                    .context("unknown retry task")?;
                target.rows[index] = source_row.clone();
            }
            selected.insert(key.clone(), (source_index, local_index));
            if !still_high.contains(&key) {
                pending.remove(&key);
            }
        }
    }
    let effective = root.join("effective");
    fs::create_dir_all(&effective)?;
    for (arm, table) in &tables {
        let path = if arm == "run" {
            effective.clone()
        } else {
            effective.join(arm)
        };
        fs::create_dir_all(&path)?;
        table.write(&path.join("results.csv"))?;
        let mut manifest = json(&if arm == "run" {
            root.join("run.json")
        } else {
            root.join(arm).join("run.json")
        })?;
        manifest["effective_load_selection"] = Value::Bool(true);
        fs::write(path.join("run.json"), serde_json::to_vec_pretty(&manifest)?)?;
    }
    fs::write(effective.join("replay.json"), serde_json::to_vec(&replay)?)?;
    let samples: Vec<_> = all_samples
        .into_iter()
        .map(|(time, cpu)| serde_json::json!({"unix_ms":time,"cpu_percent":cpu}))
        .collect();
    fs::write(
        effective.join("conditions.json"),
        serde_json::to_vec(
            &serde_json::json!({"cpu_samples":samples,"shared_machine":true,"load_selection":"score-independent; see selection.json"}),
        )?,
    )?;
    let selection:Vec<_>=selected.iter().map(|((id,seed),(source,index))|serde_json::json!({"id":id,"seed":seed,"source":if *source==0 {".".into()} else {sources[*source].strip_prefix(root).unwrap_or(&sources[*source]).to_string_lossy().replace('\\', "/")},"trial_index":index})).collect();
    fs::write(
        effective.join("selection.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"rule":initial.rule,"original_threshold_cpu":initial.threshold_cpu,"persistent_outliers":pending,"selected":selection}),
        )?,
    )?;
    if tables.contains_key("baseline") {
        let b = &tables["baseline"];
        let v = &tables["variant"];
        let mut changes = Vec::new();
        for (br, vr) in b.rows.iter().zip(&v.rows) {
            ensure!(b.key(br)? == v.key(vr)?, "paired rows do not align");
            if br[b.column("valid")?] == "true"
                && vr[v.column("valid")?] == "true"
                && let Some(delta) = vr[v.column("our_efficiency")?]
                    .parse::<f64>()
                    .ok()
                    .zip(br[b.column("our_efficiency")?].parse::<f64>().ok())
                    .map(|(v, b)| v - b)
            {
                changes.push(delta);
            }
        }
        let mean =
            (!changes.is_empty()).then(|| changes.iter().sum::<f64>() / changes.len() as f64);
        fs::write(
            effective.join("paired-summary.json"),
            serde_json::to_vec_pretty(
                &serde_json::json!({"pairs":replay.tasks.len(),"valid_pairs":changes.len(),"mean_change_pp":mean,"median_change_pp":median(&changes)}),
            )?,
        )?;
    }
    audit(&effective, Some(&root.join("load-audit.json")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_telemetry_flags_an_interruption_even_when_cpu_is_saturated() {
        let samples = [(0, 100.0), (6000, 100.0), (504000, 100.0), (510000, 100.0)];
        assert_eq!(sampling_gap_s(&samples, 0, 510000), 498.0);
        assert_eq!(sampling_gap_s(&samples, 7000, 503000), 496.0);
        assert_eq!(sampling_gap_s(&samples, 0, 6000), 6.0);
        assert_eq!(sampling_gap_s(&[], 0, 1000), 1.0);
    }
    #[test]
    fn load_rule_uses_background_values_and_not_scores() {
        let means = [20.0, 21.0, 19.0, 20.0, 60.0];
        let centre = median(&means).unwrap();
        let mad = median(&means.map(|v| (v - centre).abs())).unwrap();
        let cutoff = centre + (3.0 * mad).max(15.0);
        assert_eq!(cutoff, 35.0);
        assert!(means[4] > cutoff && means[1] < cutoff);
        assert_eq!(
            window(&[(10, 20.0), (20, 40.0)], 10, 20),
            (Some(30.0), Some(40.0), 2)
        );
        assert_eq!(window(&[(10, 20.0)], 11, 12), (Some(20.0), Some(20.0), 1));
    }
    #[test]
    fn first_normal_load_retry_wins_even_when_a_later_score_is_better() -> Result<()> {
        use super::super::runner::{Search, Task};
        let root = std::env::var_os("BETA_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("tmp")
            .join(format!(
                "quality-load-test-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_nanos()
            ));
        let task = |id: &str| Task {
            id: id.into(),
            job: beta_marker_core::Job::default(),
            seed: 1,
            budget_s: 60.0,
            reference: Some(80.0),
            stratum: "test".into(),
            area_ratio: 1.0,
        };
        let search = Search {
            tuning: beta_marker_nest::SearchTuning::default(),
            runs: 1,
            shade_attempts: 1,
            ignore_rules: false,
        };
        let replay = Replay {
            dataset: "load-test".into(),
            tasks: vec![task("a"), task("b")],
            search: search.clone(),
            paired_baseline: Some(search),
        };
        fs::create_dir_all(&root)?;
        fs::write(root.join("replay.json"), serde_json::to_vec(&replay)?)?;
        let header =
            "id,seed,valid,our_efficiency,difference_pp,started_unix_ms,finished_unix_ms\n";
        for arm in ["baseline", "variant"] {
            fs::create_dir_all(root.join(arm))?;
            fs::write(root.join(arm).join("run.json"), b"{}")?;
        }
        // Completion order differs between arms; finalization must align by id/seed.
        fs::write(
            root.join("baseline/results.csv"),
            format!("{header}b,1,true,80,0,4000,5000\na,1,true,80,0,1000,2000\n"),
        )?;
        fs::write(
            root.join("variant/results.csv"),
            format!("{header}a,1,true,90,10,2100,3000\nb,1,true,81,1,6000,7000\n"),
        )?;
        fs::write(
            root.join("conditions.json"),
            serde_json::to_vec(&serde_json::json!({"cpu_samples":[
            {"unix_ms":1500,"cpu_percent":90},{"unix_ms":2500,"cpu_percent":20},
            {"unix_ms":4500,"cpu_percent":20},{"unix_ms":6500,"cpu_percent":20}]}))?,
        )?;
        let original = audit(&root, None)?;
        assert_eq!(original.outlier_tasks, vec![("a".into(), 1)]);
        for (attempt, score, time) in [("01", 81, 8000_u64), ("02", 99, 12000_u64)] {
            let retry = root.join("reruns").join(attempt);
            fs::create_dir_all(retry.join("baseline"))?;
            fs::create_dir_all(retry.join("variant"))?;
            let mut input = replay.clone();
            input.tasks = vec![task("a")];
            fs::write(retry.join("replay.json"), serde_json::to_vec(&input)?)?;
            fs::write(
                retry.join("baseline/results.csv"),
                format!("{header}a,1,true,80,0,{time},{}\n", time + 500),
            )?;
            fs::write(
                retry.join("variant/results.csv"),
                format!(
                    "{header}a,1,true,{score},{},{},{}\n",
                    score - 80,
                    time + 600,
                    time + 1000
                ),
            )?;
            fs::write(
                retry.join("conditions.json"),
                serde_json::to_vec(&serde_json::json!({"cpu_samples":[
                {"unix_ms":time+100,"cpu_percent":20},{"unix_ms":time+700,"cpu_percent":20}]}))?,
            )?;
            audit(&retry, Some(&root.join("load-audit.json")))?;
        }
        finalize(&root)?;
        let result = json(&root.join("effective/paired-summary.json"))?;
        assert_eq!(result["mean_change_pp"], 1.0);
        let selected = json(&root.join("effective/selection.json"))?;
        assert_eq!(selected["selected"][0]["source"], "reruns/01");
        assert_eq!(selected["persistent_outliers"], serde_json::json!([]));
        Ok(())
    }
}
