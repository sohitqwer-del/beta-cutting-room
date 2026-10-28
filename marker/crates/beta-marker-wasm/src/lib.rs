//! JSON-in/JSON-out browser bindings. These functions also run in native tests.
mod export;
mod nesting;
pub use export::*;
mod import;
use beta_marker_core::{Job, MarkerResult, Placement, Pt};
pub use import::*;
use serde::{Serialize, de::DeserializeOwned};
use wasm_bindgen::prelude::*;

fn read<T: DeserializeOwned>(json: &str) -> Result<T, String> {
    serde_json::from_str(json).map_err(|error| error.to_string())
}
fn write(value: &impl Serialize) -> Result<String, String> {
    serde_json::to_string(value).map_err(|error| error.to_string())
}
fn read_job(json: &str) -> Result<Job, String> {
    let job: Job = read(json)?;
    for piece in &job.pieces {
        let multiplier = beta_marker_core::rules::multiplier(&job, piece);
        if piece.quantity.checked_mul(multiplier).is_none()
            || piece.mirror_quantity.checked_mul(multiplier).is_none()
        {
            return Err(format!("Required copy count exceeds u32 for {}", piece.id));
        }
    }
    Ok(job)
}
/// Apply one atomic Rust editing command, including polygon contact and group transforms.
#[wasm_bindgen]
pub fn edit(job: &str, result: &str, request: &str) -> Result<String, String> {
    write(&beta_marker_edit::apply(
        &read_job(job)?,
        &read(result)?,
        read(request)?,
    )?)
}

/// Retain the shorter complete marker after fresh search, using current rules and exact locks.
#[wasm_bindgen]
pub fn edit_choose_search(
    job: &str,
    incumbent: &str,
    candidate: &str,
    fixed: &str,
) -> Result<String, String> {
    write(&beta_marker_edit::choose_search_result(
        &read_job(job)?,
        &read(incumbent)?,
        &read(candidate)?,
        &read::<Vec<Placement>>(fixed)?,
    )?)
}
/// Return exact overlap areas, gap violations, section checks and missing copies.
#[wasm_bindgen]
pub fn edit_inspect(job: &str, result: &str) -> Result<String, String> {
    write(&beta_marker_edit::inspect(&read_job(job)?, &read(result)?)?)
}
/// Validate reuse of an unchanged style, ratio, fabric and piece set.
#[wasm_bindgen]
pub fn edit_reuse(current: &str, saved_job: &str, saved_result: &str) -> Result<String, String> {
    write(&beta_marker_edit::reuse(
        &read_job(current)?,
        &read_job(saved_job)?,
        &read(saved_result)?,
    )?)
}
/// Validate a job and result; returns a serialized `ValidationReport`.
#[wasm_bindgen]
pub fn validate(job: &str, result: &str) -> Result<String, String> {
    write(&beta_marker_core::validate(
        &read_job(job)?,
        &read::<MarkerResult>(result)?,
    ))
}
/// Recompute marker length, area and efficiency from placements.
#[wasm_bindgen]
pub fn metrics(job: &str, placements: &str) -> Result<String, String> {
    let job = beta_marker_core::rules::effective_job(&read_job(job)?);
    let m = beta_marker_core::compute_metrics(&job, &read::<Vec<Placement>>(placements)?);
    write(
        &serde_json::json!({"length": m.length, "piece_area": m.piece_area, "efficiency": m.efficiency,
            "useful_piece_area": m.useful_piece_area, "useful_yield": m.useful_yield, "surplus_copies": m.surplus_copies}),
    )
}
/// Return grain-aligned angles, tilt and the free-rotation flag for a piece copy.
#[wasm_bindgen]
pub fn allowed_rotations(job: &str, piece_id: &str, mirrored: bool) -> Result<String, String> {
    let job = read_job(job)?;
    let piece = job
        .pieces
        .iter()
        .find(|piece| piece.id == piece_id)
        .ok_or_else(|| format!("Unknown piece: {piece_id}"))?;
    let r = beta_marker_core::allowed_rotations(&job, piece, mirrored);
    write(&serde_json::json!({"angles": r.angles, "tilt_deg": r.tilt_deg, "free": r.free}))
}
/// Return required counts, applying the size ratio and normal/mirrored quantities.
#[wasm_bindgen]
pub fn demands(job: &str) -> Result<String, String> {
    let job = read_job(job)?;
    let demands: Vec<_> = beta_marker_core::demands(&job).into_iter().map(|d| {
        serde_json::json!({"piece_index": d.piece_index, "piece_id": d.piece_id, "mirrored": d.mirrored, "count": d.count})
    }).collect();
    write(&demands)
}
/// Check the job before nesting; returns a serialized array of issues.
#[wasm_bindgen]
pub fn check_job(job: &str) -> Result<String, String> {
    write(&beta_marker_core::check_job(&read_job(job)?))
}
/// Return shrinkage-adjusted pattern geometry for drawing and editing results.
#[wasm_bindgen]
pub fn effective_job(job: &str) -> Result<String, String> {
    write(&beta_marker_core::rules::effective_job(&read_job(job)?))
}
/// Draw the same full or folded outline used by the Rust garment rules.
#[wasm_bindgen]
pub fn placed_outline(job: &str, placement: &str) -> Result<String, String> {
    let job = beta_marker_core::rules::effective_job(&read_job(job)?);
    let placement: Placement = read(placement)?;
    let piece = job
        .pieces
        .iter()
        .find(|p| p.id == placement.piece_id)
        .ok_or_else(|| format!("Unknown piece: {}", placement.piece_id))?;
    write(&beta_marker_core::rules::placed_outline(piece, &placement))
}
/// Return normal and mirrored cut-copy counts for this placement and fabric layout.
#[wasm_bindgen]
pub fn layer_copies(job: &str, placement: &str) -> Result<String, String> {
    let job = read_job(job)?;
    write(&beta_marker_core::rules::layer_copies(
        job.fabric.layout,
        &read::<Placement>(placement)?,
    ))
}
/// Batch drawing geometry and layer counts. Decode and scale the job only once.
#[wasm_bindgen]
pub fn drawing(job: &str, placements: &str) -> Result<String, String> {
    let job = beta_marker_core::rules::effective_job(&read_job(job)?);
    let placements: Vec<Placement> = read(placements)?;
    let pieces: std::collections::HashMap<_, _> =
        job.pieces.iter().map(|p| (p.id.as_str(), p)).collect();
    let mut cut_copies = 0_u64;
    let outlines: Vec<Vec<Pt>> = placements
        .iter()
        .map(|placement| {
            let Some(piece) = pieces.get(placement.piece_id.as_str()) else {
                return vec![];
            };
            let (a, b) = beta_marker_core::rules::layer_copies(job.fabric.layout, placement);
            cut_copies += u64::from(a) + u64::from(b);
            beta_marker_core::rules::placed_outline(piece, placement)
        })
        .collect();
    write(&serde_json::json!({"outlines":outlines,"cut_copies":cut_copies}))
}
/// Transform JSON points using `R(rotation_deg) * M(point) + translate`.
#[wasm_bindgen]
pub fn transform(
    points: &str,
    mirrored: bool,
    rot: f64,
    translate: &str,
) -> Result<String, String> {
    let points: Vec<Pt> = read(points)?;
    let translate: Pt = read(translate)?;
    if !rot.is_finite() {
        return Err("Rotation must be finite".into());
    }
    write(&beta_marker_core::geometry::transform_points(
        &points, mirrored, rot, translate,
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn editing_json_api_round_trips_and_rejects_unknown_commands() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../packages/marker-wasm/test/fixtures/contract.json"
        ))
        .unwrap();
        let job = fixture["job"].to_string();
        let result = fixture["result"].to_string();
        let output: serde_json::Value = read(
            &edit(
                &job,
                &result,
                r#"{"selected":[0],"op":"move","delta":[5,5]}"#,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            output["result"]["placements"][0]["translate"],
            serde_json::json!([6., 6.])
        );
        let check: serde_json::Value = read(&edit_inspect(&job, &result).unwrap()).unwrap();
        // Ratio M:2 needs two normal and two mirrored copies; one normal is placed.
        assert_eq!(check["unplaced"].as_array().unwrap().len(), 3);
        assert_eq!(check["geometry_ok"], true);
        assert!(edit(&job, &result, r#"{"op":"unknown"}"#).is_err());
        assert!(edit_reuse(&job, &job, &result).is_err()); // incomplete demand cannot be a finished marker
    }
    fn numeric_json(value: serde_json::Value) -> serde_json::Value {
        use serde_json::Value;
        match value {
            Value::Number(n) => serde_json::json!(n.as_f64().unwrap()),
            Value::Array(a) => Value::Array(a.into_iter().map(numeric_json).collect()),
            Value::Object(o) => {
                Value::Object(o.into_iter().map(|(k, v)| (k, numeric_json(v))).collect())
            }
            v => v,
        }
    }
    #[test]
    fn json_api_obeys_the_shared_model_contract() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../packages/marker-wasm/test/fixtures/contract.json"
        ))
        .unwrap();
        let job = fixture["job"].to_string();
        let result = fixture["result"].to_string();
        let decoded_job: Job = read(&job).unwrap();
        let decoded_result: MarkerResult = read(&result).unwrap();
        // Comparing with the original JSON catches silently ignored contract fields.
        let v02 = &fixture["v02"];
        use beta_marker_core::{Layout, MatchAxes, Nap, ShadeAxis};
        for (key, values) in [
            (
                "layouts",
                serde_json::to_value([Layout::Open, Layout::Folded, Layout::Tubular]).unwrap(),
            ),
            (
                "naps",
                serde_json::to_value([Nap::None, Nap::OneWay, Nap::OneWayPerBundle]).unwrap(),
            ),
            (
                "shade_axes",
                serde_json::to_value([ShadeAxis::Length, ShadeAxis::Width]).unwrap(),
            ),
            (
                "match_axes",
                serde_json::to_value([MatchAxes::X, MatchAxes::Y, MatchAxes::Both]).unwrap(),
            ),
        ] {
            assert_eq!(v02["enum_values"][key], values);
        }
        let modern_job: Job = serde_json::from_value(v02["job"].clone()).unwrap();
        let modern_result: MarkerResult = serde_json::from_value(v02["result"].clone()).unwrap();
        assert_eq!(
            numeric_json(serde_json::to_value(&modern_job).unwrap()),
            numeric_json(v02["job"].clone())
        );
        assert_eq!(
            numeric_json(serde_json::to_value(&modern_result).unwrap()),
            numeric_json(v02["result"].clone())
        );
        let effective: Job = read(&effective_job(&v02["job"].to_string()).unwrap()).unwrap();
        assert_eq!(effective.fabric.shrinkage.length_pct, 0.0);
        assert_eq!(effective.pieces[0].outline[1], [20.4, 0.0]);
        let outline: Vec<Pt> =
            read(&placed_outline(&job, &fixture["result"]["placements"][0].to_string()).unwrap())
                .unwrap();
        assert_eq!(outline[0], [1.0, 1.0]);
        let drawn: serde_json::Value =
            read(&drawing(&job, &fixture["result"]["placements"].to_string()).unwrap()).unwrap();
        assert_eq!(drawn["cut_copies"], 1);
        assert_eq!(
            read::<Vec<Pt>>(&drawn["outlines"][0].to_string()).unwrap(),
            outline
        );
        assert_eq!(
            read::<[u32; 2]>(
                &layer_copies(&job, &fixture["result"]["placements"][0].to_string()).unwrap()
            )
            .unwrap(),
            [1, 0]
        );
        assert_eq!(
            read::<[u32; 2]>(
                &layer_copies(
                    &v02["job"].to_string(),
                    &fixture["result"]["placements"][0].to_string()
                )
                .unwrap()
            )
            .unwrap(),
            [1, 1]
        );
        assert_eq!(
            read::<Job>(&write(&decoded_job).unwrap()).unwrap(),
            decoded_job
        );
        assert_eq!(
            read::<MarkerResult>(&write(&decoded_result).unwrap()).unwrap(),
            decoded_result
        );
        assert_eq!(
            read::<Vec<serde_json::Value>>(&check_job(&job).unwrap())
                .unwrap()
                .len(),
            0
        );
        let counts: serde_json::Value = read(&demands(&job).unwrap()).unwrap();
        assert_eq!(counts[0]["count"], 2);
        assert_eq!(counts[1]["mirrored"], true);
        let rotations: serde_json::Value =
            read(&allowed_rotations(&job, "rect", true).unwrap()).unwrap();
        assert_eq!(rotations["angles"], serde_json::json!([180.0, 0.0]));
        assert!(allowed_rotations(&job, "missing", false).is_err());
        let metrics: serde_json::Value =
            read(&metrics(&job, &fixture["result"]["placements"].to_string()).unwrap()).unwrap();
        assert_eq!(metrics["length"], 21.0);
        assert_eq!(metrics["piece_area"], 200.0);
        let report: beta_marker_core::ValidationReport =
            read(&validate(&job, &result).unwrap()).unwrap();
        assert_eq!(report.length, 21.0);
        let mut overflow = decoded_job;
        overflow.pieces[0].quantity = u32::MAX;
        assert!(demands(&write(&overflow).unwrap()).is_err());
    }
    #[test]
    fn shared_transform_fixture_matches_rust() {
        let fixtures: serde_json::Value = serde_json::from_str(include_str!(
            "../../../packages/marker-wasm/test/fixtures/transform.json"
        ))
        .unwrap();
        for fixture in fixtures.as_array().unwrap() {
            let actual: Vec<Pt> = read(
                &transform(
                    &fixture["points"].to_string(),
                    fixture["mirrored"].as_bool().unwrap(),
                    fixture["rotation_deg"].as_f64().unwrap(),
                    &fixture["translate"].to_string(),
                )
                .unwrap(),
            )
            .unwrap();
            let expected: Vec<Pt> = serde_json::from_value(fixture["expected"].clone()).unwrap();
            assert_eq!(actual.len(), expected.len());
            for (a, b) in actual.iter().zip(&expected) {
                assert!((a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9);
            }
        }
    }
    #[test]
    fn malformed_json_is_an_error_on_native_too() {
        assert!(check_job("{}").is_err());
        assert!(transform("bad", false, 0.0, "[0,0]").is_err());
        assert!(transform("[]", false, f64::NAN, "[0,0]").is_err());
    }
}

mod quick;
pub use quick::*;
