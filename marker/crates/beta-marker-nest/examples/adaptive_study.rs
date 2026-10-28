//! Frozen public input, paired current baseline versus adaptive orchestration.
use anyhow::{Context, Result, ensure};
use beta_marker_core::{Job, MarkerResult, Placement};
use beta_marker_nest::{NestOptions, PortfolioStrategy, SearchTuning, nest_with_tuning};
use serde::Deserialize;
use std::{fs, io::Write, time::Instant};
#[derive(Deserialize)]
struct Case {
    id: String,
    job: Job,
    #[serde(default)]
    fixed: Vec<Placement>,
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).is_some_and(|s| s == "--audit") {
        ensure!(
            args.len() == 5,
            "adaptive_study --audit CASES JSONL SUMMARY"
        );
        return audit(&args[2], &args[3], &args[4]);
    }
    ensure!(args.len() == 4, "adaptive_study CASES OUTPUT SECONDS");
    let cases: Vec<Case> = serde_json::from_slice(&fs::read(&args[1])?)?;
    let seconds: f64 = args[3].parse()?;
    ensure!(
        seconds >= 12. && seconds.is_finite(),
        "use at least twelve seconds"
    );
    let mut output = fs::File::create(&args[2])?;
    for seed in [1, 7] {
        for case in cases.iter().filter(|c| {
            [
                "hugo-Folded",
                "teagan-Folded",
                "simon-front-locked",
                "simon-interior",
                "teagan-browser",
            ]
            .contains(&c.id.as_str())
        }) {
            let mut lengths = [0.; 2];
            for arm in if seed == 1 { [0, 1] } else { [1, 0] } {
                let options = NestOptions {
                    time_limit_s: seconds,
                    seed: Some(seed),
                    workers: Some(1),
                    fixed: case.fixed.clone(),
                    ..Default::default()
                };
                let tuning = SearchTuning {
                    portfolio: if arm == 0 {
                        PortfolioStrategy::Parallel
                    } else {
                        PortfolioStrategy::Adaptive
                    },
                    ..Default::default()
                };
                let started = Instant::now();
                let result: MarkerResult = nest_with_tuning(&case.job, &options, &tuning)
                    .with_context(|| case.id.clone())?;
                let wall = started.elapsed().as_secs_f64();
                let validation = beta_marker_core::validate(&case.job, &result);
                ensure!(
                    validation.ok,
                    "invalid {}: {:?}",
                    case.id,
                    validation.errors
                );
                ensure!(
                    case.fixed.iter().all(|p| result.placements.contains(p)),
                    "fixed record changed"
                );
                lengths[arm] = result.length;
                writeln!(
                    output,
                    "{}",
                    serde_json::json!({"case":case.id,"seed":seed,"variant":if arm==0 {"baseline"} else {"adaptive"},"requested_s":seconds,"wall_s":wall,"length_mm":result.length,"yield":result.useful_yield,"valid":true,"fixed_exact":true,"result":result})
                )?;
                output.flush()?;
            }
            println!(
                "{} seed {seed}: {:.3}% fabric saved",
                case.id,
                100. * (1. - lengths[1] / lengths[0])
            );
        }
    }
    Ok(())
}

fn audit(cases_path: &str, input: &str, output: &str) -> Result<()> {
    use std::collections::BTreeMap;
    let cases: Vec<Case> = serde_json::from_slice(&fs::read(cases_path)?)?;
    let mut pairs: BTreeMap<(String, u64), [Option<serde_json::Value>; 2]> = BTreeMap::new();
    for line in fs::read_to_string(input)?.lines() {
        let row: serde_json::Value = serde_json::from_str(line)?;
        let id = row["case"].as_str().context("case")?;
        let case = cases.iter().find(|c| c.id == id).context("unknown case")?;
        let result: MarkerResult = serde_json::from_value(row["result"].clone())?;
        ensure!(
            beta_marker_core::validate(&case.job, &result).ok,
            "invalid result"
        );
        ensure!(
            case.fixed.iter().all(|p| result.placements.contains(p)),
            "changed fixed record"
        );
        let index = match row["variant"].as_str() {
            Some("baseline") => 0,
            Some("adaptive") => 1,
            _ => anyhow::bail!("unknown variant"),
        };
        let key = (id.to_owned(), row["seed"].as_u64().context("seed")?);
        ensure!(
            pairs.entry(key.clone()).or_insert_with(|| [None, None])[index].is_none(),
            "duplicate result"
        );
        pairs.get_mut(&key).expect("inserted")[index] = Some(row);
    }
    ensure!(pairs.len() == 10, "expected ten paired results");
    let mut rows = vec![];
    let mut savings = vec![];
    for ((id, seed), pair) in pairs {
        let old = pair[0].as_ref().context("missing baseline")?;
        let new = pair[1].as_ref().context("missing candidate")?;
        let saving = 100.
            * (1.
                - new["length_mm"].as_f64().context("length")?
                    / old["length_mm"].as_f64().context("length")?);
        savings.push(saving);
        rows.push(serde_json::json!({"case":id,"seed":seed,"fabric_saved_pct":saving,"baseline_wall_s":old["wall_s"],"adaptive_wall_s":new["wall_s"],"baseline_mm":old["length_mm"],"adaptive_mm":new["length_mm"]}));
    }
    savings.sort_by(f64::total_cmp);
    let summary = serde_json::json!({"layouts_validated":20,"pairs":rows,"mean_fabric_saved_pct":savings.iter().sum::<f64>()/savings.len() as f64,"median_fabric_saved_pct":(savings[4]+savings[5])/2.,"minimum_fabric_saved_pct":savings[0],"maximum_fabric_saved_pct":savings[9],"note":"Positive means less fabric. Same requested 12-second budget; actual wall time is reported. Small public pilot, not a general result."});
    fs::write(output, serde_json::to_vec_pretty(&summary)?)?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}
