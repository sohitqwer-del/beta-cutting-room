//! Rebuild the bundled 1-NN model and evaluate by whole-style held-out groups.
use anyhow::Result;
use beta_marker_core::Job;
use beta_pattern_digitize::assist::{Example, Model, classify, descriptor};
use std::{collections::BTreeMap, path::Path};
fn label(name: &str) -> Option<&'static str> {
    let n = name.to_lowercase();
    if n.contains("cuff") {
        Some("cuff")
    } else if n.contains("placket") {
        Some("placket")
    } else if n.contains("collar") {
        Some("collar")
    } else if n.contains("pocket") {
        Some("pocket")
    } else if n.contains("sleeve") {
        Some("sleeve")
    } else if n.contains("hood") {
        Some("hood")
    } else if n.contains("waistband") || n.starts_with("wb_") {
        Some("waistband")
    } else if n.contains("binding") {
        Some("binding")
    } else if n.contains("yoke") {
        Some("yoke")
    } else if n.contains("skirt") {
        Some("skirt")
    } else if n.contains("pant_") {
        Some("trouser")
    } else if n.contains("front") || n.contains("ftorso") {
        Some("front")
    } else if n.contains("back") || n.contains("btorso") {
        Some("back")
    } else {
        None
    }
}
fn main() -> Result<()> {
    let root = std::env::args()
        .nth(1)
        .unwrap_or("../marker/testdata/open".into());
    let output = std::env::args()
        .nth(2)
        .unwrap_or("crates/beta-pattern-digitize/model/shape-model.json".into());
    let mut examples = Vec::new();
    for source in ["freesewing", "garmentcode"] {
        let mut paths: Vec<_> = std::fs::read_dir(Path::new(&root).join(source))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.to_string_lossy().ends_with(if source == "freesewing" {
                    "-M.job.json"
                } else {
                    ".job.json"
                })
            })
            .collect();
        paths.sort();
        for path in paths {
            if path.to_string_lossy().contains("interfacing") {
                continue;
            } // same geometry as Simon main fabric
            let job: Job = serde_json::from_slice(&std::fs::read(&path)?)?;
            let name = path.file_name().unwrap().to_string_lossy();
            let group = if source == "freesewing" {
                name.split('-').next().unwrap().to_owned()
            } else {
                name.trim_end_matches(".job.json").to_owned()
            };
            for p in &job.pieces {
                if let Some(label) = label(&p.name) {
                    let example = Example {
                        label: label.into(),
                        source: format!("{source}/{group}/{}", p.name),
                        features: descriptor(&p.outline),
                    };
                    // Exclude exact/mirror duplicates within each group from training and scoring.
                    if !examples.iter().any(|e: &Example| {
                        e.source
                            .split('/')
                            .take(2)
                            .eq(example.source.split('/').take(2))
                            && e.label == example.label
                            && e.features
                                .iter()
                                .zip(&example.features)
                                .all(|(a, b)| (a - b).abs() < 0.005)
                    }) {
                        examples.push(example);
                    }
                }
            }
        }
    }
    let mut rows = Vec::new();
    let mut correct = 0;
    let mut classes: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for e in &examples {
        let group: Vec<_> = e.source.split('/').take(2).collect();
        let model = Model {
            format: "beta-quick-shape/v1".into(),
            attribution: String::new(),
            examples: examples
                .iter()
                .filter(|x| x.source.split('/').take(2).collect::<Vec<_>>() != group)
                .cloned()
                .collect(),
        };
        let (predicted, distance) = classify(&model, &e.features).unwrap();
        let ok = predicted == e.label;
        correct += usize::from(ok);
        let counts = classes.entry(e.label.clone()).or_default();
        counts.0 += usize::from(ok);
        counts.1 += 1;
        rows.push(serde_json::json!({"source":e.source,"actual":e.label,"predicted":predicted,"distance":distance,"correct":ok}));
    }
    let report = serde_json::json!({"split":"leave-one-whole-style-out; no ratio sizes or interfacing duplicates; near-identical descriptors deduplicated within style","correct":correct,"total":examples.len(),"accuracy":correct as f64/examples.len() as f64,"classes":classes,"predictions":rows});
    let model=Model{format:"beta-quick-shape/v1".into(),attribution:"FreeSewing MIT, Joost De Cock and contributors; GarmentCodeData CC BY 4.0, Maria Korosteleva et al., ECCV 2024, doi:10.3929/ethz-b-000690432. Derived shape descriptors, not endorsement. See MODEL.md.".into(),examples};
    std::fs::write(&output, serde_json::to_string_pretty(&model)?)?;
    std::fs::write(
        Path::new(&output).with_file_name("evaluation.json"),
        serde_json::to_string_pretty(&report)?,
    )?;
    println!(
        "Held-out whole styles: {correct}/{} = {:.2}%",
        model.examples.len(),
        100. * correct as f64 / model.examples.len() as f64
    );
    println!("{}", serde_json::to_string_pretty(&report["classes"])?);
    Ok(())
}
