//! Paired release measurement. Geometry and evidence stay in the external output.
//! prepare MARKER INPUT_JSONL OUT; run ROOT; summarize ROOT SITE_TEMPLATE_DIR
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    process::{Command, Stdio},
    time::Instant,
};
#[path = "../../crates/beta-marker-cli/src/cmd/bench/csv.rs"]
#[allow(dead_code)]
mod csv;
fn read(p: impl AsRef<Path>) -> Result<Value> {
    Ok(serde_json::from_slice(&fs::read(p)?)?)
}
fn write(p: impl AsRef<Path>, v: &Value) -> Result<()> {
    fs::write(p, serde_json::to_vec_pretty(v)?)?;
    Ok(())
}
fn hash(p: impl AsRef<Path>) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(fs::read(p)?)))
}
fn task(id: Value, job: Value, reference: Value, stratum: Value) -> Value {
    json!({"id":id,"job":job,"seed":1,"budget_s":60.0,"reference":reference,"stratum":stratum,"area_ratio":1.0})
}
fn prepare(marker: &Path, input: &Path, out: &Path) -> Result<()> {
    fs::create_dir_all(out.join("inputs"))?;
    ensure!(
        hash(input)? == "66edbac7fcb1baa57945147a99113b7449f262d928012d43385847d039849c8c",
        "public input hash mismatch"
    );
    let search = json!({"tuning":read(marker.join("bench/experiments/v03/selected.json"))?,"runs":1,"shade_attempts":1,"ignore_rules":false});
    let mut tasks = Vec::new();
    for line in fs::read_to_string(input)?.lines() {
        let mut v: Value = serde_json::from_str(line)?;
        ensure!(
            v["comparable"] == true && v["sheet_type"] == 0,
            "unsupported task"
        );
        v["job"]["meta"]["benchmark_tier"] = v["tier"].clone();
        tasks.push(task(
            v["id"].clone(),
            v["job"].clone(),
            v["lectra_efficiency"].clone(),
            v["stratum"].clone(),
        ));
    }
    ensure!(tasks.len() == 300, "expected 300 tasks");
    let mut stages = Vec::new();
    for (i, chunk) in tasks.chunks(20).enumerate() {
        let name = format!("lectra-{i:02}");
        write(
            out.join(format!("inputs/{name}.json")),
            &json!({"dataset":"lectra","tasks":chunk,"search":search,"paired_baseline":null}),
        )?;
        stages
            .push(json!({"name":name,"order":if i%2==0 {vec!["07","08"]} else {vec!["08","07"]}}));
    }
    let mut garments = Vec::new();
    for name in ["simon", "teagan", "titan"] {
        let original =
            read(marker.join(format!("testdata/open/freesewing/{name}-ratio.job.json")))?;
        for layout in ["open", "folded", "tubular"] {
            for defect in [false, true] {
                let mut job = original.clone();
                job["fabric"]["layout"] = json!(layout);
                if layout != "open" {
                    job["fabric"]["width"] =
                        json!(original["fabric"]["width"].as_f64().context("width")? / 2.0);
                    if name == "teagan" {
                        for p in job["pieces"].as_array_mut().context("pieces")? {
                            if ["front", "back"].contains(&p["name"].as_str().unwrap_or("")) {
                                let ys: Vec<_> = p["outline"]
                                    .as_array()
                                    .context("outline")?
                                    .iter()
                                    .map(|x| x[1].as_f64().unwrap())
                                    .collect();
                                let lo = ys.iter().copied().fold(f64::INFINITY, f64::min);
                                let hi = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                                p["fold_line"] = json!([[0.0, lo], [0.0, hi]]);
                            }
                        }
                    }
                }
                if defect {
                    job["fabric"]["defects"] = json!([{"id":"launch8-start","polygon":[[0,0],[20,0],[20,20],[0,20]],"note":"Original synthetic 20 mm start defect"}]);
                }
                let id = format!(
                    "{name}-{layout}-{}",
                    if defect { "defect" } else { "clean" }
                );
                garments.push(task(
                    json!(id),
                    job,
                    Value::Null,
                    json!(format!(
                        "{layout};{}",
                        if defect { "defect" } else { "clean" }
                    )),
                ));
            }
        }
    }
    write(
        out.join("inputs/garments.json"),
        &json!({"dataset":"garments","tasks":garments,"search":search,"paired_baseline":null}),
    )?;
    stages.push(json!({"name":"garments","order":["08","07"]}));
    write(
        out.join("plan.json"),
        &json!({"stages":stages,"input_sha256":hash(input)?,"tasks":318,"seed":1,"budget_s":60,"jobs":4,"workers":3}),
    )
}
fn run(root: &Path) -> Result<()> {
    let out = root.join("out");
    let plan = read(out.join("plan.json"))?;
    let mut timing = Vec::new();
    for stage in plan["stages"].as_array().context("stages")? {
        let name = stage["name"].as_str().unwrap();
        for ver in stage["order"].as_array().unwrap() {
            let ver = ver.as_str().unwrap();
            let dest = out.join(format!("{name}-{ver}"));
            ensure!(!dest.exists(), "fresh run only: {}", dest.display());
            let log = fs::File::create(out.join(format!("{name}-{ver}.log")))?;
            let start = Instant::now();
            let status = Command::new("nice")
                .args(["-n", "10"])
                .arg(root.join(format!("bin/beta-marker{ver}")))
                .args(["bench", "replay", "--input"])
                .arg(out.join(format!("inputs/{name}.json")))
                .args(["--jobs", "4", "--threads-per-task", "3", "--out"])
                .arg(&dest)
                .current_dir(root.join(format!("source{ver}/marker")))
                .stdout(Stdio::from(log.try_clone()?))
                .stderr(Stdio::from(log))
                .status()?;
            timing.push(json!({"stage":name,"version":ver,"wall_s":start.elapsed().as_secs_f64(),"ok":status.success()}));
            write(out.join("timing.json"), &json!(timing))?;
            println!(
                "completed {name}-{ver}: {:.1} s",
                start.elapsed().as_secs_f64()
            );
            ensure!(status.success(), "stage failed; inspect log");
        }
    }
    fs::write(out.join("DONE"), "complete\n")?;
    Ok(())
}
fn stats(label: &str, diffs: &[f64], attempts: usize) -> Value {
    let mut d = diffs.to_vec();
    d.sort_by(f64::total_cmp);
    let n = d.len();
    json!({"label":label,"headline":true,"attempts":attempts,"valid":n,
        "mean_diff_pp":if n>0 {Some(d.iter().sum::<f64>()/n as f64)}else{None},
        "median_diff_pp":if n>0 {Some((d[(n-1)/2]+d[n/2])/2.0)}else{None},
        "win":d.iter().filter(|&&v|v>0.1).count(),"tie":d.iter().filter(|&&v|v.abs()<=0.1).count(),"loss":d.iter().filter(|&&v|v< -0.1).count(),"other_failures":attempts-n})
}
fn collect(out: &Path, name: &str, ver: &str) -> Result<Vec<Value>> {
    let path = out.join(format!("{name}-{ver}"));
    let config = read(path.join("run.json"))?;
    let records = csv::records(&fs::read_to_string(path.join("results.csv"))?)?;
    let header = &records[0];
    let mut rows = Vec::new();
    for record in &records[1..] {
        ensure!(record.len() == header.len(), "incomplete CSV");
        let get = |k: &str| -> Result<&str> {
            Ok(&record[header.iter().position(|h| h == k).context("CSV field")?])
        };
        let id = get("id")?;
        let valid = get("valid")? == "true";
        let index = config["tasks"]
            .as_array()
            .context("tasks")?
            .iter()
            .position(|t| t["id"] == id)
            .context("task ID")?;
        let dir = path.join(format!("trials/{index:05}"));
        let mut row = json!({"id":id,"valid":valid,"error":get("errors")?,"tier":get("tier")?,"stratum":get("stratum")?,"runtime_s":get("runtime_s")?.parse::<f64>()?,"validation_s":get("validation_s")?.parse::<f64>()?});
        if valid {
            let validation = read(dir.join("validation.json"))?;
            ensure!(
                validation["ok"] == true
                    && validation["errors"]
                        .as_array()
                        .is_some_and(|a| a.is_empty()),
                "validator failure"
            );
            let result = read(dir.join("result.json"))?;
            for (k, c) in [("efficiency", "our_efficiency"), ("length", "our_length")] {
                row[k] = json!(get(c)?.parse::<f64>()?);
            }
            for (k, c) in [("reference", "reference_efficiency")] {
                if !get(c)?.is_empty() {
                    row[k] = json!(get(c)?.parse::<f64>()?);
                }
            }
            for metric in ["length", "efficiency", "useful_yield"] {
                ensure!(
                    (result[metric].as_f64().context("result metric")?
                        - validation[metric].as_f64().context("validator metric")?)
                    .abs()
                        < 1e-6,
                    "result and validator metric disagree: {id} {metric}"
                );
            }
            ensure!(
                (row["efficiency"].as_f64().unwrap()
                    - 100.0 * validation["efficiency"].as_f64().unwrap())
                .abs()
                    < 1e-6,
                "CSV and validator efficiency disagree"
            );
            row["useful_yield_pct"] = validation["useful_yield"]
                .as_f64()
                .map(|v| json!(v * 100.0))
                .unwrap_or(Value::Null);
            row["surplus_copies"] = validation["surplus_copies"].clone();
            row["validation_sha256"] = json!(hash(dir.join("validation.json"))?);
        }
        rows.push(row);
    }
    ensure!(
        rows.len() == config["tasks"].as_array().unwrap().len(),
        "missing trials"
    );
    Ok(rows)
}
fn summarize(root: &Path, templates: &Path) -> Result<()> {
    let out = root.join("out");
    ensure!(out.join("DONE").exists(), "run incomplete");
    let mut a = Vec::new();
    let mut b = Vec::new();
    for i in 0..15 {
        let n = format!("lectra-{i:02}");
        a.extend(collect(&out, &n, "07")?);
        b.extend(collect(&out, &n, "08")?);
    }
    let ga = collect(&out, "garments", "07")?;
    let gb = collect(&out, "garments", "08")?;
    let mut all_pairs = Vec::new();
    let mut groups = Vec::new();
    for (label, old, new) in [("Lectra 300", &a, &b), ("FreeSewing 18", &ga, &gb)] {
        let old: BTreeMap<_, _> = old.iter().map(|r| (r["id"].as_str().unwrap(), r)).collect();
        let mut diffs = Vec::new();
        for n in new {
            let o = old
                .get(n["id"].as_str().unwrap())
                .context("unpaired task")?;
            let delta = if o["valid"] == true && n["valid"] == true {
                Some(n["efficiency"].as_f64().unwrap() - o["efficiency"].as_f64().unwrap())
            } else {
                None
            };
            if let Some(d) = delta {
                diffs.push(d);
            }
            all_pairs
                .push(json!({"suite":label,"id":n["id"],"before":o,"after":n,"change_pp":delta}));
        }
        groups.push(stats(
            if label == "FreeSewing 18" {
                "All 18 garment jobs (occupied efficiency)"
            } else {
                "0.8 minus 0.7 (300 Lectra tasks)"
            },
            &diffs,
            new.len(),
        ));
    }
    let lectra_diffs: Vec<_> = b
        .iter()
        .filter(|r| r["valid"] == true)
        .map(|r| r["efficiency"].as_f64().unwrap() - r["reference"].as_f64().unwrap())
        .collect();
    let old_diffs: Vec<_> = a
        .iter()
        .filter(|r| r["valid"] == true)
        .map(|r| r["efficiency"].as_f64().unwrap() - r["reference"].as_f64().unwrap())
        .collect();
    let timing = read(out.join("timing.json"))?;
    let revisions = json!({"07":"9a852d27ae5471eea1f7dc58e4ccb277f4e4fd88","08":"345f42ee0ed906620e9a11a8e135045f034df553"});
    let mut garment_groups = Vec::new();
    for layout in ["open", "folded", "tubular"] {
        for defect in ["clean", "defect"] {
            let label = format!("{layout}-{defect}");
            let pairs: Vec<_> = all_pairs
                .iter()
                .filter(|p| {
                    p["suite"] == "FreeSewing 18" && p["id"].as_str().unwrap().ends_with(&label)
                })
                .collect();
            for metric in ["efficiency", "useful_yield_pct"] {
                let diffs: Vec<_> = pairs
                    .iter()
                    .filter_map(|p| {
                        Some(p["after"][metric].as_f64()? - p["before"][metric].as_f64()?)
                    })
                    .collect();
                garment_groups.push(stats(&format!("{label}: {metric}"), &diffs, pairs.len()));
            }
        }
    }
    let garment_useful_diffs: Vec<_> = all_pairs
        .iter()
        .filter(|p| p["suite"] == "FreeSewing 18")
        .filter_map(|p| {
            Some(
                p["after"]["useful_yield_pct"].as_f64()?
                    - p["before"]["useful_yield_pct"].as_f64()?,
            )
        })
        .collect();
    let garment_overall_useful = stats(
        "All 18 garment jobs (useful yield)",
        &garment_useful_diffs,
        18,
    );
    let strata: std::collections::BTreeSet<_> =
        b.iter().map(|r| r["stratum"].as_str().unwrap()).collect();
    let mut stratum_groups = Vec::new();
    for stratum in strata {
        let pairs: Vec<_> = all_pairs
            .iter()
            .filter(|p| p["suite"] == "Lectra 300" && p["after"]["stratum"] == stratum)
            .collect();
        let d: Vec<_> = pairs
            .iter()
            .filter_map(|p| p["change_pp"].as_f64())
            .collect();
        let l: Vec<_> = pairs
            .iter()
            .filter_map(|p| {
                Some(p["after"]["efficiency"].as_f64()? - p["after"]["reference"].as_f64()?)
            })
            .collect();
        stratum_groups.push(
            json!({"paired":stats(stratum,&d,pairs.len()),"lectra":stats(stratum,&l,pairs.len())}),
        );
    }
    let mut runtimes = Vec::new();
    for (suite, old, new) in [("Lectra", &a, &b), ("FreeSewing", &ga, &gb)] {
        for (ver, rows) in [("07", old), ("08", new)] {
            let mut times: Vec<_> = rows
                .iter()
                .map(|r| r["runtime_s"].as_f64().unwrap())
                .collect();
            times.sort_by(f64::total_cmp);
            let stage_wall: f64 = timing
                .as_array()
                .unwrap()
                .iter()
                .filter(|s| {
                    s["version"] == ver
                        && (s["stage"].as_str().unwrap().starts_with("lectra")
                            == (suite == "Lectra"))
                })
                .map(|s| s["wall_s"].as_f64().unwrap())
                .sum();
            runtimes.push(json!({"suite":suite,"version":ver,"sum_s":times.iter().sum::<f64>(),"median_s":(times[(times.len()-1)/2]+times[times.len()/2])/2.0,"max_s":times.last(),"runner_wall_s":stage_wall}));
        }
    }
    let summary = json!({"schema":"beta-launch8-paired/1","total_runner_wall_s":timing.as_array().unwrap().iter().map(|t|t["wall_s"].as_f64().unwrap()).sum::<f64>(),"revisions":revisions,"paired_groups":groups,"garment_overall_useful":garment_overall_useful,"garment_groups":garment_groups,"strata":stratum_groups,"runtime":runtimes,"new_vs_lectra":stats("0.8 vs Lectra",&lectra_diffs,300),"old_vs_lectra":stats("0.7 vs Lectra",&old_diffs,300),"pairs":all_pairs,"timing":timing,"executable_sha256":{"07":hash(root.join("bin/beta-marker07"))?,"08":hash(root.join("bin/beta-marker08"))?}});
    write(out.join("summary.json"), &summary)?;
    fs::write(out.join("report-tables.md"), report_tables(&summary))?;
    fs::create_dir_all(out.join("site"))?;
    let template = read(templates.join("lectra-60s.json"))?;
    for (id, title, group, kind, reference) in [
        (
            "lectra-60s-08",
            "0.8 on Lectra tasks, 60 seconds",
            summary["new_vs_lectra"].clone(),
            "lectra",
            template["reference"].clone(),
        ),
        (
            "lectra-08-vs-07",
            "0.8 vs 0.7, 300 public tasks",
            groups[0].clone(),
            "other",
            json!({"name":"BETA Cutting Room 0.7.0","type":"engine-baseline"}),
        ),
        (
            "freesewing-08-vs-07",
            "0.8 vs 0.7, garment lay matrix",
            groups[1].clone(),
            "other",
            json!({"name":"BETA Cutting Room 0.7.0","type":"engine-baseline"}),
        ),
    ] {
        let mut v = template.clone();
        for k in [
            "strata",
            "items",
            "correlation",
            "quantiles_pp",
            "task_ids",
            "x_validation",
            "x_runtime",
            "x_provenance",
        ] {
            v.as_object_mut().unwrap().remove(k);
        }
        v["id"] = json!(id);
        v["title"] = json!(title);
        v["kind"] = json!(kind);
        v["date"] = json!("2026-10-28");
        v["order"] = json!(5);
        v["groups"] = json!([group]);
        v["reference"] = reference;
        v["software"] = json!({"beta_version":"0.8.0 candidate","engine_revision":revisions["08"]});
        v["summary_text"] = json!(
            "One fresh search per engine and task. Engine order alternates in fixed 20-task Lectra blocks. Negative differences mean 0.8 used more fabric. This does not establish general commercial parity."
        );
        v["summary_text"] = json!(match id {
            "lectra-60s-08" =>
                "0.8 is behind Lectra on this sample. Dense markers with 151 or more pieces remain a weakness. No general commercial parity claim is supported.",
            "lectra-08-vs-07" =>
                "0.8 is slightly worse on this ordinary-open sample. Most pairs tie within 0.1 point. This one-seed run does not establish a durable regression or a broad improvement.",
            _ =>
                "Gains concentrate in open lays with a start defect. Clean folded cases are flat; Simon tubular cases and its folded defect case regress. Useful yield excludes surplus copies.",
        });
        v["conditions"]["background_load"] = json!(
            "Shared machine; archived conditions.json CPU samples and per-task timestamps document load. No claim of exclusive CPU use."
        );
        v["conditions"]["notes"] = json!(
            "Same 60-second requested budget, seed 1, four concurrent tasks and three search workers per task in both engines. Effective nice 19 for both (nested nice -n 10), versus historical nice 10. Whole-call wall time can exceed the search budget."
        );
        v["reproduce"] = json!({"commands":["See the paired release protocol in marker/bench/results/2026-10-28-public-0.8.md"],"doc":"marker/bench/results/2026-10-28-public-0.8.md"});
        v["x_release_pair"] = json!({"revisions":revisions,"tie_threshold_pp":0.1,"timing":timing});
        v["x_runtime"] = json!(runtimes);
        if id == "lectra-60s-08" {
            v["strata"] = json!(
                stratum_groups
                    .iter()
                    .map(|g| g["lectra"].clone())
                    .collect::<Vec<_>>()
            );
        }
        v["sources"] = json!([{"label":"Paired release report","url":"https://github.com/sohitqwer-del/beta-cutting-room/blob/main/marker/bench/results/2026-10-28-public-0.8.md"}]);
        if kind == "other" {
            v["assumptions"] = json!([
                "This compares two BETA engine revisions, not a commercial engine.",
                "One seed per task does not estimate search variability.",
                "Folded and tubular garment comparisons need useful yield as well as occupied footprint efficiency; see the report."
            ]);
        }
        if id == "freesewing-08-vs-07" {
            v["groups"] = json!(
                summary["garment_groups"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|g| g["label"].as_str().unwrap().ends_with(": useful_yield_pct"))
                    .map(|g| {
                        let mut g = g.clone();
                        g["headline"] = json!(false);
                        g["label"] = json!(
                            g["label"]
                                .as_str()
                                .unwrap()
                                .replace(": useful_yield_pct", " (useful yield)")
                        );
                        g
                    })
                    .collect::<Vec<_>>()
            );
            v["groups"]
                .as_array_mut()
                .unwrap()
                .insert(0, groups[1].clone());
            v["groups"]
                .as_array_mut()
                .unwrap()
                .insert(0, garment_overall_useful.clone());
            v["groups"][1]["headline"] = json!(false);
        }
        write(out.join(format!("site/{id}.json")), &v)?;
    }
    Ok(())
}

fn report_tables(s: &Value) -> String {
    fn group(g: &Value) -> String {
        format!(
            "| {} | {} / {} | {:.4} | {:.4} | {} / {} / {} |\n",
            g["label"].as_str().unwrap(),
            g["valid"],
            g["attempts"],
            g["mean_diff_pp"].as_f64().unwrap_or(f64::NAN),
            g["median_diff_pp"].as_f64().unwrap_or(f64::NAN),
            g["win"],
            g["tie"],
            g["loss"]
        )
    }
    let header = "| Comparison | Valid / attempted | Mean pp | Median pp | Win / tie / loss |\n|---|---:|---:|---:|---:|\n";
    let mut md = String::from(header);
    for g in [
        &s["paired_groups"][0],
        &s["new_vs_lectra"],
        &s["old_vs_lectra"],
        &s["paired_groups"][1],
        &s["garment_overall_useful"],
    ] {
        md.push_str(&group(g));
    }
    md.push_str("\n### Garment groups: 0.8 minus 0.7\n\n");
    md.push_str(header);
    for g in s["garment_groups"].as_array().unwrap() {
        md.push_str(&group(g));
    }
    md.push_str("\n### All garment pairs\n\n| Job | Occupied % 0.7 / 0.8 | Useful % 0.7 / 0.8 | Length mm 0.7 / 0.8 | Surplus 0.7 / 0.8 | Wall s 0.7 / 0.8 |\n|---|---:|---:|---:|---:|---:|\n");
    for p in s["pairs"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["suite"] == "FreeSewing 18")
    {
        let o = &p["before"];
        let n = &p["after"];
        md.push_str(&format!(
            "| {} | {:.4} / {:.4} | {:.4} / {:.4} | {:.2} / {:.2} | {} / {} | {:.3} / {:.3} |\n",
            p["id"].as_str().unwrap(),
            o["efficiency"].as_f64().unwrap_or(f64::NAN),
            n["efficiency"].as_f64().unwrap_or(f64::NAN),
            o["useful_yield_pct"].as_f64().unwrap_or(f64::NAN),
            n["useful_yield_pct"].as_f64().unwrap_or(f64::NAN),
            o["length"].as_f64().unwrap_or(f64::NAN),
            n["length"].as_f64().unwrap_or(f64::NAN),
            o["surplus_copies"],
            n["surplus_copies"],
            o["runtime_s"].as_f64().unwrap(),
            n["runtime_s"].as_f64().unwrap()
        ));
    }
    for (key, title) in [
        ("paired", "Lectra strata: 0.8 minus 0.7"),
        ("lectra", "Lectra strata: 0.8 minus Lectra"),
    ] {
        md.push_str(&format!("\n### {title}\n\n"));
        md.push_str(header);
        for g in s["strata"].as_array().unwrap() {
            md.push_str(&group(&g[key]));
        }
    }
    md.push_str("\n### Actual times\n\n| Suite | Engine | Runner wall s | Sum call s | Median call s | Maximum call s |\n|---|---|---:|---:|---:|---:|\n");
    for r in s["runtime"].as_array().unwrap() {
        md.push_str(&format!(
            "| {} | {} | {:.3} | {:.3} | {:.3} | {:.3} |\n",
            r["suite"].as_str().unwrap(),
            r["version"].as_str().unwrap(),
            r["runner_wall_s"].as_f64().unwrap(),
            r["sum_s"].as_f64().unwrap(),
            r["median_s"].as_f64().unwrap(),
            r["max_s"].as_f64().unwrap()
        ));
    }
    md
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn threshold_and_failed_pairs_do_not_change_the_numeric_denominator() {
        let s = stats("paired", &[-0.2, -0.1, 0.0, 0.1, 0.4], 6);
        assert_eq!(s["valid"], 5);
        assert_eq!(s["other_failures"], 1);
        assert_eq!(s["win"], 1);
        assert_eq!(s["tie"], 3);
        assert_eq!(s["loss"], 1);
        assert_eq!(s["median_diff_pp"], 0.0);
        assert!((s["mean_diff_pp"].as_f64().unwrap() - 0.04).abs() < 1e-12);
    }
}
fn main() -> Result<()> {
    let a: Vec<_> = std::env::args().collect();
    match a.get(1).map(String::as_str) {
        Some("prepare") if a.len() == 5 => {
            prepare(Path::new(&a[2]), Path::new(&a[3]), Path::new(&a[4]))
        }
        Some("run") if a.len() == 3 => run(Path::new(&a[2])),
        Some("summarize") if a.len() == 4 => summarize(Path::new(&a[2]), Path::new(&a[3])),
        Some("check-stage") if a.len() == 5 => {
            let rows = collect(Path::new(&a[2]), &a[3], &a[4])?;
            println!(
                "Checked {} saved trials and their validator evidence",
                rows.len()
            );
            Ok(())
        }
        _ => {
            anyhow::bail!("prepare MARKER INPUT OUT | run ROOT | summarize ROOT SITE_TEMPLATE_DIR")
        }
    }
}
