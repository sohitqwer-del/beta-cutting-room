//! JSON bindings for offline pattern editing. Geometry is evaluated by Rust.
use beta_pattern_core::{Document, evaluate, exchange};
use wasm_bindgen::prelude::*;

fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn grade_document(doc: &Document) -> Result<String, String> {
    if doc.sizes.is_empty() || doc.sizes.len() > 100 {
        return Err("Choose 1 to 100 sizes".into());
    }
    let mut sizes = std::collections::BTreeMap::new();
    for size in &doc.sizes {
        let mut evaluator =
            evaluate::Evaluator::new(doc, size, 0.1).map_err(|e| format!("{e:#}"))?;
        for key in doc.variables.keys() {
            evaluator
                .value(&beta_pattern_core::Value::Formula(format!("v.{key}")))
                .map_err(|e| format!("{e:#}"))?;
        }
        for id in doc.drawing.keys() {
            evaluator.object(id).map_err(|e| format!("{e:#}"))?;
        }
        sizes.insert(
            size,
            evaluate::grade(doc, size, 0.1).map_err(|e| format!("{e:#}"))?,
        );
    }
    serde_json::to_string(&serde_json::json!({"sizes":sizes})).map_err(|e| e.to_string())
}
/// Evaluate every size with its measurements and formulas.
#[wasm_bindgen]
pub fn grade(document: &str) -> Result<String, String> {
    let doc: Document = serde_json::from_str(document).map_err(error)?;
    grade_document(&doc).map_err(error)
}
/// Original BETA parametric bodice, with four sizes and 10 mm seam allowance.
#[wasm_bindgen]
pub fn starter() -> Result<String, String> {
    serde_json::to_string(&beta_pattern_core::starters::bodice()).map_err(error)
}
/// Convert selected size ratios and fabric settings to a marker job.
#[wasm_bindgen]
pub fn to_job(document: &str, fabric: &str, ratio: &str) -> Result<String, String> {
    let doc = serde_json::from_str(document).map_err(error)?;
    let fabric = serde_json::from_str(fabric).map_err(error)?;
    let ratio = serde_json::from_str(ratio).map_err(error)?;
    let job = exchange::to_job(&doc, fabric, ratio, 0.1).map_err(|e| error(format!("{e:#}")))?;
    serde_json::to_string(&job).map_err(error)
}
/// Export all document sizes using the public AAMA-style DXF conventions.
#[wasm_bindgen]
pub fn export_dxf(document: &str) -> Result<String, String> {
    let doc: Document = serde_json::from_str(document).map_err(error)?;
    exchange::export_dxf(&doc, &doc.sizes, 0.1).map_err(|e| error(format!("{e:#}")))
}
/// Turn accepted digitized pieces into an editable fixed-geometry document.
#[wasm_bindgen]
pub fn from_job(job: &str) -> Result<String, String> {
    let job = serde_json::from_str(job).map_err(error)?;
    let doc =
        beta_pattern_core::starters::from_marker_job(&job).map_err(|e| error(format!("{e:#}")))?;
    serde_json::to_string(&doc).map_err(error)
}

mod editor;

/// Inspect all draft objects and return piece checks for the chosen size.
#[wasm_bindgen]
pub fn inspect(document: &str, size: &str) -> Result<String, String> {
    let doc = serde_json::from_str(document).map_err(error)?;
    serde_json::to_string(&editor::inspect(&doc, size)?).map_err(error)
}
/// Evaluate a formula without committing it.
#[wasm_bindgen]
pub fn check_formula(document: &str, size: &str, expression: &str) -> Result<f64, String> {
    editor::check_formula(
        &serde_json::from_str(document).map_err(error)?,
        size,
        expression,
    )
}
/// Find the nearest draft point, path projection or grid coordinate.
#[wasm_bindgen]
pub fn snap(document: &str, size: &str, request: &str) -> Result<String, String> {
    let doc = serde_json::from_str(document).map_err(error)?;
    let r: serde_json::Value = serde_json::from_str(request).map_err(error)?;
    let at = serde_json::from_value(r["at"].clone()).map_err(error)?;
    serde_json::to_string(&editor::snap(
        &doc,
        size,
        editor::SnapSettings {
            at,
            radius: r["radius"].as_f64().unwrap_or(8.),
            grid: r["grid"].as_f64().unwrap_or(10.),
            points: r["points"].as_bool().unwrap_or(true),
            paths: r["paths"].as_bool().unwrap_or(true),
            ignore: r["ignore"].as_str(),
        },
    )?)
    .map_err(error)
}
/// Derive millimetres per pixel for a traced photo.
#[wasm_bindgen]
pub fn image_scale(a: &str, b: &str, distance_mm: f64) -> Result<f64, String> {
    editor::image_scale(
        serde_json::from_str(a).map_err(error)?,
        serde_json::from_str(b).map_err(error)?,
        distance_mm,
    )
}
/// Import the supported Seamly2D subset using the same offline native parser.
#[wasm_bindgen]
pub fn import_seamly(
    pattern: &str,
    measurements: Option<String>,
    size: &str,
) -> Result<String, String> {
    let report = beta_pattern_core::seamly::import(pattern, measurements.as_deref(), size)
        .map_err(|e| format!("{e:#}"))?;
    serde_json::to_string(&report).map_err(error)
}

pub mod output;
/// Return a real tiled 1:1 PDF with physical A4/A3 pages and a calibration square.
#[wasm_bindgen]
pub fn export_pdf(document: &str, paper: &str) -> Result<Vec<u8>, String> {
    output::pdf(&serde_json::from_str(document).map_err(error)?, paper)
}
/// Return a plotter file with all sizes and a 100 mm calibration square.
#[wasm_bindgen]
pub fn export_plt(document: &str) -> Result<String, String> {
    output::plt(&serde_json::from_str(document).map_err(error)?)
}

/// Return typed form specifications for every engine construction tool.
#[wasm_bindgen]
pub fn catalog() -> Result<String, String> {
    serde_json::to_string(&beta_pattern_core::tools::catalog()).map_err(error)
}
/// Apply a catalog construction to an immutable document snapshot.
#[wasm_bindgen]
pub fn apply_tool(document: &str, request: &str) -> Result<String, String> {
    let doc = serde_json::from_str(document).map_err(error)?;
    let r: serde_json::Value = serde_json::from_str(request).map_err(error)?;
    let text = |name: &str| r[name].as_str().ok_or_else(|| format!("Missing {name}"));
    let params = r["parameters"]
        .as_object()
        .ok_or("Tool parameters must be an object")?
        .clone();
    let next = if let Some(selection) = r["selection"].as_array() {
        let selection: Vec<String> = selection
            .iter()
            .map(|s| {
                s.as_str()
                    .map(str::to_owned)
                    .ok_or("Choose valid group objects")
            })
            .collect::<Result<_, _>>()?;
        let mut params = params;
        params.remove("source");
        beta_pattern_core::tools::transform_group(
            &doc,
            &selection,
            text("id")?,
            text("tool")?,
            params,
        )
    } else {
        beta_pattern_core::tools::apply(&doc, text("tool")?, text("id")?, params)
    }
    .map_err(|e| format!("{e:#}"))?;
    serde_json::to_string(&serde_json::json!({"document":next})).map_err(error)
}

/// Piece operations for menu commands. The original piece is retained.
#[wasm_bindgen]
pub fn piece_action(document: &str, request: &str) -> Result<String, String> {
    serde_json::to_string(&operations::apply(
        &serde_json::from_str(document).map_err(error)?,
        &serde_json::from_str(request).map_err(error)?,
    )?)
    .map_err(error)
}
mod operations;

/// Import measurements with native CSV/XML validation.
#[wasm_bindgen]
pub fn import_measurements(
    document: &str,
    source: &str,
    format: &str,
    size: &str,
) -> Result<String, String> {
    serde_json::to_string(&editor::measurements(
        &serde_json::from_str(document).map_err(error)?,
        source,
        format,
        size,
    )?)
    .map_err(error)
}
/// Export a measurement table which the native engine can read back.
#[wasm_bindgen]
pub fn export_measurements(document: &str) -> Result<String, String> {
    editor::measurement_csv(&serde_json::from_str(document).map_err(error)?)
}
/// Align graded sizes at a named point, in Rust.
#[wasm_bindgen]
pub fn nest(document: &str, reference: &str) -> Result<String, String> {
    let doc: Document = serde_json::from_str(document).map_err(error)?;
    serde_json::to_string(
        &evaluate::nest(&doc, &doc.sizes, reference, 0.1).map_err(|e| format!("{e:#}"))?,
    )
    .map_err(error)
}
/// Move a draggable point using its construction constraints.
#[wasm_bindgen]
pub fn move_point(document: &str, id: &str, size: &str, target: &str) -> Result<String, String> {
    let doc = serde_json::from_str(document).map_err(error)?;
    let target = serde_json::from_str(target).map_err(error)?;
    serde_json::to_string(
        &beta_pattern_core::tools::move_point(&doc, id, size, target)
            .map_err(|e| format!("{e:#}"))?,
    )
    .map_err(error)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_starter_grades_and_measurement_edits_change_geometry() {
        let mut doc: Document = serde_json::from_str(include_str!(
            "../../../../marker/packages/marker-wasm/test/fixtures/pattern.doc.json"
        ))
        .unwrap();
        let before = grade_document(&doc).unwrap();
        doc.measurements
            .get_mut("M")
            .unwrap()
            .insert("chest".into(), 1000.0);
        let after = grade_document(&doc).unwrap();
        assert_ne!(before, after);
        let graded: serde_json::Value = serde_json::from_str(&after).unwrap();
        assert_eq!(graded["sizes"].as_object().unwrap().len(), 4);
        doc.variables.insert("ease".into(), "v.missing".into());
        assert!(grade_document(&doc).is_err());
    }
}

#[cfg(test)]
mod editor_round_trips {
    use super::*;
    use serde_json::{Value, json};
    #[test]
    fn json_bindings_draft_grade_export_and_reopen() {
        let doc = Document::default();
        let mut document = serde_json::to_string(&doc).unwrap();
        for (id, x, y) in [
            ("A", 0., 0.),
            ("B", 200., 0.),
            ("C", 200., 600.),
            ("D", 0., 600.),
        ] {
            let request = json!({"tool":"absolute","id":id,"parameters":{"x":x,"y":y}}).to_string();
            let response: Value =
                serde_json::from_str(&apply_tool(&document, &request).unwrap()).unwrap();
            document = response["document"].to_string();
        }
        for (id, a, b) in [
            ("waist", "A", "B"),
            ("side", "B", "C"),
            ("hem", "C", "D"),
            ("centre", "D", "A"),
        ] {
            let request = json!({"tool":"line","id":id,"parameters":{"from":a,"to":b}}).to_string();
            let response: Value =
                serde_json::from_str(&apply_tool(&document, &request).unwrap()).unwrap();
            document = response["document"].to_string();
        }
        let mut doc: Document = serde_json::from_str(&document).unwrap();
        let mut piece: Value = json!({"id":"skirt","name":"Skirt","edges":[{"object":"waist","allowance":12},{"object":"side","allowance":12},{"object":"hem","allowance":12},{"object":"centre","allowance":12}],"label_point":"A","internal_paths":[],"anchors":{"waist":"A"}});
        piece["quantity"] = json!(1);
        doc.pieces.push(serde_json::from_value(piece).unwrap());
        doc.sizes.push("L".into());
        document = serde_json::to_string(&doc).unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&grade(&document).unwrap()).unwrap()["sizes"]
                .as_object()
                .unwrap()
                .len(),
            2
        );
        assert!(
            export_dxf(&document)
                .unwrap()
                .contains("BETA_PATTERN_RULES_BEGIN")
        );
        assert!(export_pdf(&document, "A3").unwrap().starts_with(b"%PDF"));
        let job = to_job(
            &document,
            &json!({"width":1500}).to_string(),
            &json!({"M":1,"L":1}).to_string(),
        )
        .unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&job).unwrap()["pieces"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert!(from_job(&job).is_err()); // Digitizer acceptance converts a single size.
        let mut single_job: beta_marker_core::Job = serde_json::from_str(&job).unwrap();
        single_job.pieces.retain(|p| p.size.as_deref() == Some("M"));
        let reopened = from_job(&serde_json::to_string(&single_job).unwrap()).unwrap();
        assert!(grade(&reopened).is_ok());
        assert!(
            apply_tool(
                &document,
                &json!({"tool":"absolute","id":"A","parameters":{"x":0,"y":0}}).to_string()
            )
            .is_err()
        );
    }
    #[test]
    fn measurement_csv_round_trip_and_bad_formulas() {
        let doc = starter().unwrap();
        let csv = export_measurements(&doc).unwrap();
        let imported = import_measurements(&doc, &csv, "csv", "M").unwrap();
        assert_eq!(
            serde_json::from_str::<Document>(&doc).unwrap().measurements,
            serde_json::from_str::<Document>(&imported)
                .unwrap()
                .measurements
        );
        assert!(check_formula(&doc, "M", "m.chest/4").unwrap() > 0.);
        assert!(check_formula(&doc, "M", "m.missing").is_err());
        assert!(inspect(&doc, "missing").is_err());
        assert!(
            piece_action(
                &doc,
                &json!({"action":"unknown","piece":"front"}).to_string()
            )
            .is_err()
        );
    }
}

/// Photo digitizing uses the same bounded engine as Quick Marker.
#[wasm_bindgen]
pub fn quick_scan(photo: &[u8], options: &str) -> Result<String, String> {
    let options = serde_json::from_str(options).map_err(error)?;
    let scan = beta_pattern_digitize::quick::scan_bytes(photo, &options).map_err(error)?;
    serde_json::to_string(&scan).map_err(error)
}
