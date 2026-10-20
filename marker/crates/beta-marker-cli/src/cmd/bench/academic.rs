//! The pinned sparrow v0.3.0 jagua input format, without engine internals.
use anyhow::{Context, Result, bail, ensure};
use beta_marker_core::{Fabric, JOB_FORMAT, Job, Nap, Piece, RotationRule, Spacing, check_job};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

const REFERENCES: &str = include_str!("../../../../../bench/academic-references.csv");

pub fn reference(id: &str) -> Option<f64> {
    REFERENCES.lines().skip(1).find_map(|line| {
        let columns: Vec<_> = line.split(',').collect();
        (columns.first() == Some(&id))
            .then(|| columns[2].parse().ok())
            .flatten()
    })
}

pub fn load(id: &str) -> Result<Job> {
    ensure!(
        [
            "shirts", "trousers", "swim", "albano", "dagli", "mao", "marques", "gardeyn0",
            "gardeyn1"
        ]
        .contains(&id),
        "unknown instance {id}"
    );
    let folder = PathBuf::from(
        std::env::var_os("BETA_DATA_DIR")
            .context("set BETA_DATA_DIR to the local public benchmark data folder")?,
    )
    .join("datasets/esicup-sparrow");
    let path = folder.join(format!("{id}.json"));
    load_cached(&path).with_context(|| format!("load academic instance {id}"))
}

fn load_cached(path: &Path) -> Result<Job> {
    let bytes = fs::read(path).with_context(|| format!(
        "Cannot read local benchmark fixture {}. Prepare the public Sparrow v0.3.0 data before running benchmarks; runtime downloads are disabled.",
        path.display()
    ))?;
    convert(&bytes)
}

#[derive(Deserialize)]
struct Instance {
    name: String,
    strip_height: f64,
    items: Vec<Item>,
}
#[derive(Deserialize)]
struct Item {
    id: u64,
    demand: u32,
    orientation: Orientation,
    shape: Shape,
}
#[derive(Deserialize)]
struct Orientation {
    rotation: Rotation,
    #[serde(default)]
    mirror: Option<serde_json::Value>,
    #[serde(default)]
    reflection_axes: Vec<serde_json::Value>,
}
#[derive(Deserialize)]
struct Rotation {
    mode: String,
    #[serde(default)]
    angles: Vec<f64>,
}
#[derive(Deserialize)]
struct Shape {
    r#type: String,
    data: serde_json::Value,
}

pub fn convert(bytes: &[u8]) -> Result<Job> {
    let source: Instance = serde_json::from_slice(bytes)?;
    let mut pieces = vec![];
    for item in source.items {
        ensure!(
            item.orientation
                .mirror
                .as_ref()
                .is_none_or(|v| v.is_null() || v == false),
            "mirror orientation needs an exact mapping"
        );
        ensure!(
            item.orientation.reflection_axes.is_empty(),
            "reflection axes cannot be mapped exactly"
        );
        let r = item.orientation.rotation;
        let mut angles = r.angles;
        ensure!(angles.iter().all(|a| a.is_finite()), "non-finite rotation");
        for a in &mut angles {
            *a = a.rem_euclid(360.0);
        }
        angles.sort_by(f64::total_cmp);
        angles.dedup();
        let rotation = match (r.mode.as_str(), angles.as_slice()) {
            ("continuous", _) => RotationRule::Free,
            ("discrete", [0.0]) => RotationRule::Fixed,
            ("discrete", [0.0, 180.0]) => RotationRule::Flip180,
            ("discrete", [0.0, 90.0, 180.0, 270.0]) => RotationRule::Quarter,
            _ => bail!(
                "rotation set cannot be mapped exactly: {} {angles:?}",
                r.mode
            ),
        };
        ensure!(
            item.shape.r#type == "simple_polygon",
            "only simple polygons are comparable"
        );
        let mut outline: Vec<[f64; 2]> = serde_json::from_value(item.shape.data)?;
        if outline.first() == outline.last() {
            outline.pop();
        }
        pieces.push(Piece {
            id: item.id.to_string(),
            name: item.id.to_string(),
            style: None,
            size: None,
            material: None,
            outline,
            grain: None,
            quantity: item.demand,
            mirror_quantity: 0,
            rotation,
            buffer: 0.0,
            marks: vec![],
            ..Default::default()
        });
    }
    let job = Job {
        match_rules: Vec::new(),
        format: JOB_FORMAT.into(),
        name: source.name,
        fabric: Fabric {
            width: source.strip_height,
            max_length: None,
            nap: Nap::None,
            allow_flip: false,
            tilt_deg: 0.0,
            ..Default::default()
        },
        spacing: Spacing::default(),
        pieces,
        order: None,
        meta: BTreeMap::from([(
            "source".into(),
            "sparrow v0.3.0; source units retained numerically".into(),
        )]),
    };
    let issues = check_job(&job);
    ensure!(issues.is_empty(), "invalid academic job: {issues:?}");
    Ok(job)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maps_demand_width_and_rotations_without_scaling() {
        for (rotation, expected) in [
            (
                r#"{"mode":"discrete","angles":[180,0]}"#,
                RotationRule::Flip180,
            ),
            (
                r#"{"mode":"discrete","angles":[0,90,180,270]}"#,
                RotationRule::Quarter,
            ),
            (r#"{"mode":"continuous"}"#, RotationRule::Free),
        ] {
            let json = format!(
                r#"{{"name":"test","strip_height":10,"items":[{{"id":4,"demand":3,"orientation":{{"rotation":{rotation}}},"shape":{{"type":"simple_polygon","data":[[0,0],[2,0],[0,2],[0,0]]}}}}]}}"#
            );
            let job = convert(json.as_bytes()).unwrap();
            assert_eq!(job.fabric.width, 10.0);
            assert_eq!(job.pieces[0].quantity, 3);
            assert_eq!(job.pieces[0].outline.len(), 3);
            assert_eq!(job.pieces[0].rotation, expected);
            assert!(job.pieces[0].grain.is_none());
        }
        assert_eq!(reference("shirts"), Some(89.66));
        assert_eq!(reference("gardeyn0"), None);
    }
    #[test]
    fn rejects_unrepresentable_angles() {
        let json = br#"{"name":"bad","strip_height":10,"items":[{"id":0,"demand":1,"orientation":{"rotation":{"mode":"discrete","angles":[45]}},"shape":{"type":"simple_polygon","data":[[0,0],[1,0],[0,1]]}}]}"#;
        assert!(convert(json).unwrap_err().to_string().contains("exactly"));
    }

    #[test]
    fn missing_fixture_fails_offline_without_creating_a_download_folder() {
        let root = std::env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join(format!("missing-academic-qa-{}", std::process::id()));
        assert!(!root.exists());
        let error = load_cached(&root.join("shirts.json")).unwrap_err();
        assert!(error.to_string().contains("runtime downloads are disabled"));
        assert!(!root.exists());
    }
}
