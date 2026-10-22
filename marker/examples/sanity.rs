//! Convert an academic jagua instance, then compare adapter and raw CLI runs.
//! Usage: sanity <input.json> <raw-sparrow-executable> <output-dir> [seconds=60]
//! Check saved raw output: sanity --check-raw <job.json> <raw.json> <output-dir>
use anyhow::{Context, Result, ensure};
use beta_marker_core::*;
use beta_marker_nest::{NestOptions, nest_with_progress};
use jagua_rs::io::ext_repr::ExtShape;
use jagua_rs::probs::spp::io::ext_repr::ExtSPInstance;
use jagua_rs::probs::spp::io::ext_repr::ExtSPSolution;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.first().is_some_and(|s| s == "--check-raw") {
        ensure!(
            args.len() == 4,
            "usage: sanity --check-raw <job.json> <raw.json> <output-dir>"
        );
        let job: Job = serde_json::from_reader(std::fs::File::open(&args[1])?)?;
        let raw: serde_json::Value = serde_json::from_reader(std::fs::File::open(&args[2])?)?;
        let report = check_raw(&job, &raw, &PathBuf::from(&args[3]))?;
        println!(
            "{}",
            serde_json::to_string_pretty(
                &serde_json::json!({"raw_valid":report.ok,"errors":report.errors.len(),"warnings":report.warnings.len()})
            )?
        );
        return Ok(());
    }
    ensure!(
        (3..=4).contains(&args.len()),
        "usage: sanity <input.json> <raw-sparrow-executable> <output-dir> [seconds=60]"
    );
    let input = std::fs::canonicalize(&args[0])?;
    let raw_exe = std::fs::canonicalize(&args[1])?;
    let output = PathBuf::from(&args[2]);
    std::fs::create_dir_all(&output)?;
    let output = std::fs::canonicalize(output)?;
    let seconds: u64 = args
        .get(3)
        .map(|s| s.to_string_lossy().parse())
        .transpose()?
        .unwrap_or(60);
    let instance: ExtSPInstance = serde_json::from_reader(std::fs::File::open(&input)?)?;
    ensure!(
        instance.min_item_separation == 0.,
        "this comparison expects zero separation"
    );
    let pieces = instance
        .items
        .iter()
        .map(|item| {
            let outline = match &item.base.shape {
                ExtShape::SimplePolygon(p) => &p.0,
                ExtShape::Polygon(p) if p.inner.is_empty() => &p.outer.0,
                _ => anyhow::bail!("academic conversion needs a polygon without holes"),
            };
            Ok(Piece {
                id: format!("item-{}", item.base.id),
                name: format!("item-{}", item.base.id),
                style: None,
                size: None,
                material: None,
                outline: outline
                    .iter()
                    .map(|p| [f64::from(p.0), f64::from(p.1)])
                    .collect(),
                grain: None,
                quantity: u32::try_from(item.demand)?,
                mirror_quantity: 0,
                rotation: RotationRule::Flip180,
                buffer: 0.,
                marks: vec![],
                ..Default::default()
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let job = Job {
        match_rules: Vec::new(),
        format: JOB_FORMAT.into(),
        name: instance.name.clone(),
        fabric: Fabric {
            width: f64::from(instance.strip_height),
            max_length: None,
            nap: Nap::None,
            allow_flip: false,
            tilt_deg: 0.,
            ..Default::default()
        },
        spacing: Spacing::default(),
        pieces,
        order: None,
        meta: BTreeMap::new(),
    };
    std::fs::write(output.join("job.json"), serde_json::to_vec_pretty(&job)?)?;
    let mut improvements = vec![];
    let result = nest_with_progress(
        &job,
        &NestOptions {
            time_limit_s: seconds as f64,
            seed: Some(1),
            workers: Some(3),
            early_termination: false,
            ..Default::default()
        },
        |p| improvements.push(p),
    )?;
    std::fs::write(
        output.join("adapter.json"),
        serde_json::to_vec_pretty(&result)?,
    )?;
    let validation = validate(&job, &result);
    std::fs::write(
        output.join("validation.json"),
        serde_json::to_vec_pretty(&validation)?,
    )?;
    ensure!(validation.ok, "invalid academic adapter result");
    std::fs::write(
        output.join("progress.json"),
        serde_json::to_vec_pretty(&improvements)?,
    )?;
    let log = std::fs::File::create(output.join("raw.log"))?;
    let status = Command::new(raw_exe)
        .current_dir(&output)
        .arg("--input")
        .arg(&input)
        .arg("--global-time")
        .arg(seconds.to_string())
        .args(["--rng-seed", "1", "--workers", "3"])
        .stdout(Stdio::from(log))
        .stderr(Stdio::inherit())
        .status()
        .context("launch raw sparrow CLI")?;
    ensure!(status.success(), "raw sparrow CLI failed: {status}");
    let raw: serde_json::Value = serde_json::from_reader(std::fs::File::open(
        output
            .join("output")
            .join(format!("final_{}.json", instance.name)),
    )?)?;
    let raw_length = raw["solution"]["strip_width"]
        .as_f64()
        .context("raw strip_width missing")?;
    let raw_efficiency = result.piece_area / (job.fabric.width * raw_length);
    let raw_validation = check_raw(&job, &raw, &output)?;
    let summary = serde_json::json!({"instance":instance.name,"seconds_per_phase_budget_total":seconds,"seed":1,"workers":3,
        "adapter_efficiency":result.efficiency,"raw_efficiency":raw_efficiency,"difference_percentage_points":100.*(result.efficiency-raw_efficiency),
        "adapter_length":result.length,"raw_length":raw_length,"adapter_runtime_s":result.runtime_s,"improvements":improvements.len(),"raw_valid":raw_validation.ok});
    std::fs::write(
        output.join("summary.json"),
        serde_json::to_vec_pretty(&summary)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    ensure!(
        result.efficiency + 0.005 >= raw_efficiency,
        "adapter lost more than 0.5 percentage points"
    );
    Ok(())
}

fn check_raw(
    job: &Job,
    raw: &serde_json::Value,
    output: &std::path::Path,
) -> Result<ValidationReport> {
    let solution: ExtSPSolution =
        serde_json::from_value(raw["solution"].clone()).context("invalid raw solution")?;
    let mut copies = BTreeMap::<(String, bool), u32>::new();
    let placements = solution
        .layout
        .placed_items
        .into_iter()
        .map(|item| {
            let id = format!("item-{}", item.item_id);
            let transform = item.transformation;
            let copy = copies.entry((id.clone(), transform.reflected)).or_default();
            let placement = Placement {
                piece_id: id,
                copy: *copy,
                mirrored: transform.reflected,
                rotation_deg: f64::from(transform.rotation),
                translate: [
                    f64::from(transform.translation.0),
                    f64::from(transform.translation.1),
                ],
                ..Default::default()
            };
            *copy += 1;
            placement
        })
        .collect::<Vec<_>>();
    let metrics = compute_metrics(job, &placements);
    let result = MarkerResult {
        format: RESULT_FORMAT.into(),
        job_name: job.name.clone(),
        width: job.fabric.width,
        length: f64::from(solution.strip_width),
        piece_area: metrics.piece_area,
        efficiency: metrics.piece_area / (job.fabric.width * f64::from(solution.strip_width)),
        useful_piece_area: metrics.useful_piece_area,
        useful_yield: metrics.useful_yield,
        surplus_copies: metrics.surplus_copies,
        engine: "raw sparrow 0.3.0".into(),
        seed: Some(1),
        runtime_s: 0.0,
        placements,
        warnings: Vec::new(),
    };
    let report = validate(job, &result);
    std::fs::create_dir_all(output)?;
    std::fs::write(
        output.join("raw-beta-result.json"),
        serde_json::to_vec_pretty(&result)?,
    )?;
    std::fs::write(
        output.join("raw-validation.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(report)
}
