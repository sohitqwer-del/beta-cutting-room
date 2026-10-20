//! Deserialize generated data with the real product contract, then check rules.
use beta_marker_core::{Job, check_job, demands, geometry::signed_area};
use std::{env, error::Error, fs, path::Path};

fn round_trip_matches(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    use serde_json::Value;
    match (a, b) {
        (Value::Object(a), Value::Object(b)) => a
            .iter()
            .all(|(key, value)| b.get(key).is_some_and(|v| round_trip_matches(value, v))),
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| round_trip_matches(a, b))
        }
        (Value::Number(a), Value::Number(b)) => a.as_f64() == b.as_f64(),
        _ => a == b,
    }
}

fn visit(dir: &Path, count: &mut usize) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            continue;
        }
        if !path.to_string_lossy().ends_with(".job.json") {
            continue;
        }
        let bytes = fs::read(&path)?;
        let job: Job = serde_json::from_slice(&bytes)?;
        let issues = check_job(&job);
        if !issues.is_empty() {
            return Err(format!("{}: {issues:?}", path.display()).into());
        }
        // Reject unknown fields too: serde's default parser would ignore them.
        let parsed: serde_json::Value = serde_json::from_slice(&bytes)?;
        let serialized = serde_json::to_value(&job)?;
        if !round_trip_matches(&parsed, &serialized) {
            return Err(format!("{}: contract round trip differs", path.display()).into());
        }
        for piece in &job.pieces {
            if signed_area(&piece.outline) <= 0.0 || piece.outline.first() == piece.outline.last() {
                return Err(format!("{}: invalid outline {}", path.display(), piece.id).into());
            }
            let [a, b] = piece.grain.ok_or("missing grain")?;
            if (b[0] - a[0]).hypot(b[1] - a[1]) < 1.0 {
                return Err("degenerate grain".into());
            }
        }
        if demands(&job).is_empty() {
            return Err("empty demand".into());
        }
        *count += 1;
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut count = 0;
    for dir in env::args().skip(1) {
        visit(Path::new(&dir), &mut count)?;
    }
    if count == 0 {
        return Err("pass at least one directory containing .job.json files".into());
    }
    println!("{count} jobs deserialized, round-tripped and checked against beta-marker-core");
    Ok(())
}
