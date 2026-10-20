//! Standalone offline driver until the lead wires `beta-marker roll`.

use anyhow::{Context, Result, bail, ensure};
use beta_marker_roll::*;
use serde::{Serialize, de::DeserializeOwned};
use std::{env, fs};

fn read<T: DeserializeOwned>(path: &str) -> Result<T> {
    let bytes = fs::read(path).with_context(|| format!("read {path}"))?;
    serde_json::from_slice(&bytes).with_context(|| format!("parse JSON in {path}"))
}

fn json(value: &impl Serialize) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn settings<T: DeserializeOwned + Default>(args: &[String], index: usize) -> Result<T> {
    args.get(index)
        .map_or_else(|| Ok(T::default()), |p| read(p))
}

fn demo() -> (RollRegister, Vec<LayRequest>) {
    let register = demo_register();
    let lays = (0..18)
        .map(|i| LayRequest {
            id: format!("L{i:02}"),
            marker_length_m: [5.7, 7.3, 9.4, 6.1, 8.6, 10.2][i % 6],
            plies: 14,
            fabric_width_mm: 1500.0,
            shade_rule: ShadePolicy::Required {
                identity: ShadeIdentity {
                    lot: "lot".into(),
                    dye_lot: "dye".into(),
                    shade_group: format!("S{}", i % 3),
                },
            },
            eligible_roll_ids: Vec::new(),
            allow_splicing: false,
            splice_zones: Vec::new(),
        })
        .collect();
    (register, lays)
}

fn main() -> Result<()> {
    let mut args: Vec<_> = env::args().skip(1).collect();
    let format = if args
        .last()
        .is_some_and(|a| ["--csv", "--text", "--json"].contains(&a.as_str()))
    {
        args.pop().unwrap_or_default()
    } else {
        "--json".into()
    };
    let usage = "usage: roll inspect|group REGISTER [SETTINGS] [--csv|--text]; roll allocate|compare REGISTER LAYS [SETTINGS] [--csv|--text]; roll defects REGISTER PLAN [SETTINGS]; roll demo [--text|--json]";
    let Some(command) = args.first() else {
        bail!("{usage}");
    };
    if command == "demo" {
        ensure!(args.len() == 1, "{usage}");
        let (register, lays) = demo();
        let c = compare_allocations(&register, &lays, &AllocationSettings::default())?;
        validate_plan(&register, &c.first_come)?;
        validate_plan(&register, &c.optimized)?;
        if format == "--json" {
            return json(&c);
        }
        if format == "--csv" {
            print!("{}", c.optimized.to_csv());
            return Ok(());
        }
        println!("40 rolls, seed 20261019, lengths 40-120 m, 3 shade groups; 18 lays x 14 plies.");
        println!(
            "First come: complete {}, waste {:.2} m, roll changes {}",
            c.first_come.complete, c.first_come.total_waste_m, c.first_come.roll_changes
        );
        println!(
            "Optimized: complete {}, waste {:.2} m, roll changes {}",
            c.optimized.complete, c.optimized.total_waste_m, c.optimized.roll_changes
        );
        if let (Some(m), Some(pct)) = (c.waste_saved_m, c.waste_saved_pct) {
            println!("Saved: {m:.2} m ({pct:.2}%) of opened-roll end-bit exposure.");
        }
        print!("{}", c.optimized.to_text());
        return Ok(());
    }
    let path = args.get(1).context(usage)?;
    let register: RollRegister = read(path)?;
    match command.as_str() {
        "inspect" => {
            ensure!((2..=3).contains(&args.len()), "{usage}");
            let report = inspect(&register, &settings(&args, 2)?)?;
            match format.as_str() {
                "--csv" => print!("{}", report.to_csv()),
                "--text" => print!("{}", report.to_text()),
                _ => return json(&report),
            }
        }
        "group" => {
            ensure!(
                (2..=3).contains(&args.len()) && format == "--json",
                "group accepts JSON output only"
            );
            return json(&group_rolls(&register, &settings(&args, 2)?)?);
        }
        "allocate" | "compare" => {
            ensure!((3..=4).contains(&args.len()), "{usage}");
            let lays: Vec<LayRequest> = read(args.get(2).context(usage)?)?;
            let settings: AllocationSettings = settings(&args, 3)?;
            if command == "compare" {
                ensure!(format == "--json", "compare accepts JSON output only");
                let c = compare_allocations(&register, &lays, &settings)?;
                validate_plan(&register, &c.first_come)?;
                validate_plan(&register, &c.optimized)?;
                return json(&c);
            }
            let plan = allocate(&register, &lays, &settings)?;
            validate_plan(&register, &plan)?;
            match format.as_str() {
                "--csv" => print!("{}", plan.to_csv()),
                "--text" => print!("{}", plan.to_text()),
                _ => return json(&plan),
            }
        }
        "defects" => {
            ensure!(
                (3..=4).contains(&args.len()) && format == "--json",
                "{usage}"
            );
            let plan: SpreadingPlan = read(args.get(2).context(usage)?)?;
            validate_plan(&register, &plan)?;
            let settings = settings(&args, 3)?;
            let mapped: Result<Vec<_>> = plan
                .lays
                .iter()
                .map(|l| map_defects(&register, l, &settings))
                .collect();
            return json(&mapped?);
        }
        _ => bail!("{usage}"),
    }
    Ok(())
}
