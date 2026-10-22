//! JSON style dispatcher, callable from WASM or a native server.
use beta_pattern_core::{Document, blocks, style};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

fn field<T: DeserializeOwned>(request: &Value, name: &str) -> Result<T, String> {
    serde_json::from_value(
        request
            .get(name)
            .cloned()
            .ok_or_else(|| format!("Supply {name}."))?,
    )
    .map_err(|e| format!("Check {name}: {e}"))
}
fn output(value: impl serde::Serialize) -> Result<Value, String> {
    serde_json::to_value(value).map_err(|e| e.to_string())
}
fn plain(e: anyhow::Error) -> String {
    format!("{e:#}")
}
/// Pure dispatcher. Errors never mutate caller-owned documents.
pub fn dispatch(request: &Value) -> Result<Value, String> {
    let action = request["action"].as_str().ok_or("Choose a style action.")?;
    match action {
        "blocks" => output(blocks::catalog()),
        "size_chart" => output(
            blocks::size_chart(request["preset"].as_str().unwrap_or("indian_adult"))
                .map_err(plain)?,
        ),
        "create" => output(blocks::create(&field(request, "request")?).map_err(plain)?),
        "load" => output(style::load(&field::<String>(request, "source")?).map_err(plain)?),
        "overlay" => output(beta_marker_core::style::overlay(
            &field(request, "job")?,
            &field(request, "result")?,
            &field(request, "options")?,
        )?),
        _ => {
            let doc: Document = field(request, "document")?;
            match action {
                "edit" => {
                    let ops: Vec<Value> = field(request, "operations")?;
                    if ops.len() > 1000 {
                        return Err("Use at most 1000 edits in one transaction.".into());
                    }
                    let mut next = doc.clone();
                    for op in &ops {
                        if op["op"] == "piece_action" {
                            let result = super::pattern_operations::apply(&next, &op["request"])?;
                            next = field(&result, "document")?;
                            let action = op["request"]["action"].as_str().unwrap_or("");
                            if matches!(
                                action,
                                "split"
                                    | "dart"
                                    | "dart_transfer"
                                    | "dart_close"
                                    | "pleat"
                                    | "slash_spread"
                            ) {
                                let mode = op["request"]["production_mode"]
                                    .as_str()
                                    .unwrap_or("replace");
                                if !matches!(mode, "replace" | "additional") {
                                    return Err(
                                        "Choose replace or additional as production_mode.".into()
                                    );
                                }
                                if mode == "replace" {
                                    let source =
                                        op["request"]["piece"].as_str().ok_or("Choose piece")?;
                                    style::archive_replaced_sources(&mut next, &[source])
                                        .map_err(plain)?;
                                }
                            }
                        } else {
                            let operation = serde_json::from_value(op.clone())
                                .map_err(|e| format!("Check the edit: {e}"))?;
                            next = style::prepare(&next, &operation).map_err(plain)?;
                        }
                    }
                    style::validate(&next).map_err(plain)?;
                    output(next)
                }
                "save" => output(style::save(&doc).map_err(plain)?),
                "grade" => {
                    style::validate(&doc).map_err(plain)?;
                    let mut sizes = serde_json::Map::new();
                    for size in &doc.sizes {
                        sizes.insert(
                            size.clone(),
                            output(
                                beta_pattern_core::evaluate::grade(&doc, size, style::TOLERANCE)
                                    .map_err(plain)?,
                            )?,
                        );
                    }
                    Ok(json!({"sizes":sizes}))
                }
                "hit_test" => output(
                    style::hit_test(
                        &doc,
                        &field::<String>(request, "size")?,
                        field(request, "at")?,
                        field(request, "radius")?,
                    )
                    .map_err(plain)?,
                ),
                "measure" => output(
                    style::measure(
                        &doc,
                        &field::<String>(request, "size")?,
                        &field(request, "request")?,
                    )
                    .map_err(plain)?,
                ),
                "seam_report" => output(
                    beta_pattern_core::production::check_seams(
                        &doc,
                        &field(request, "check")?,
                        0.05,
                    )
                    .map_err(plain)?,
                ),
                "block_seams" => output(blocks::seam_reports(&doc).map_err(plain)?),
                "block_checks" => {
                    output(beta_pattern_core::block_checks::report(&doc).map_err(plain)?)
                }
                "spec_sheet" => {
                    output(beta_pattern_core::block_checks::spec_sheet(&doc).map_err(plain)?)
                }
                "export_spec_sheet" => output(
                    beta_pattern_core::spec_export::export(
                        &doc,
                        &field::<String>(request, "format")?,
                    )
                    .map_err(plain)?,
                ),
                "to_job" => {
                    output(style::to_job(&doc, &field(request, "settings")?).map_err(plain)?)
                }
                _ => Err(format!("Unknown style action '{action}'.")),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dispatch_creates_edits_grades_and_reopens() {
        let doc = dispatch(&json!({"action":"create","request":{"block":"t_shirt"}})).unwrap();
        let edited=dispatch(&json!({"action":"edit","document":doc,"operations":[{"op":"set_allowance","piece":"front","edges":[0,9],"width":25,"corner":"miter"}]})).unwrap();
        assert_ne!(edited, doc);
        let saved = dispatch(&json!({"action":"save","document":edited})).unwrap();
        assert_eq!(
            dispatch(&json!({"action":"load","source":saved})).unwrap(),
            edited
        );
        let graded = dispatch(&json!({"action":"grade","document":edited})).unwrap();
        assert_eq!(graded["sizes"].as_object().unwrap().len(), 5);
        assert!(dispatch(&json!({"action":"edit","document":doc,"operations":[{"op":"delete_object","id":"front.A"}]})).is_err());
    }
    #[test]
    fn dispatcher_rejects_malformed_requests_without_panics() {
        for r in [
            json!({}),
            json!({"action":"nope"}),
            json!({"action":"create","request":{"block":"missing"}}),
            json!({"action":"load","source":"{}"}),
            json!({"action":"overlay","job":{}}),
        ] {
            assert!(dispatch(&r).is_err());
        }
    }

    #[test]
    fn spec_dispatch_and_dart_replacement_never_approve_the_archived_block() {
        let doc = blocks::create(&blocks::Create {
            block: Some(blocks::BlockKind::Shirt),
            ..Default::default()
        })
        .unwrap();
        let spec = dispatch(&json!({"action":"spec_sheet","document":doc})).unwrap();
        assert_eq!(spec["schema"], "beta-pattern.spec/v1");
        let report = dispatch(&json!({"action":"block_checks","document":doc})).unwrap();
        assert_eq!(report["pass"], true);
        for format in ["pdf", "csv", "xlsx"] {
            let file =
                dispatch(&json!({"action":"export_spec_sheet","document":doc,"format":format}))
                    .unwrap();
            assert!(!file["bytes"].as_array().unwrap().is_empty());
        }
        let mut e = beta_pattern_core::evaluate::Evaluator::new(&doc, "M", 0.05).unwrap();
        let side = e.path("front.side").unwrap();
        let center = beta_pattern_core::geometry::length(&side) / 2.;
        let (p, _) = beta_pattern_core::geometry::at_distance(&side, center).unwrap();
        let point = json!({"op":"put_object","id":"dart_apex","object":{"kind":"point","operation":{"op":"absolute","x":p[0]-55.,"y":p[1]}}});
        let mut action = json!({"op":"piece_action","request":{"action":"dart","piece":"front","id":"front_dart","size":"M","seam_edge":1,"center":center,"width":20.,"apex":"dart_apex"}});
        let edited =
            dispatch(&json!({"action":"edit","document":doc,"operations":[point,action]})).unwrap();
        let source = edited["pieces"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["id"] == "front")
            .unwrap();
        assert_eq!(source["quantity"], 0);
        assert_eq!(source["mirror_quantity"], 0);
        let replacement = edited["pieces"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["id"] == "front_dart")
            .unwrap();
        assert_eq!(replacement["quantity"], 1);
        assert_eq!(replacement["mirror_quantity"], 1);
        assert_eq!(
            dispatch(&json!({"action":"block_seams","document":edited})).unwrap(),
            json!([])
        );
        assert_eq!(
            dispatch(&json!({"action":"block_checks","document":edited})).unwrap()["pass"],
            false
        );
        assert!(dispatch(&json!({"action":"spec_sheet","document":edited})).is_err());
        let job=dispatch(&json!({"action":"to_job","document":edited,"settings":{"fabric":{"width":1500},"spacing":{"piece_gap":3,"edge_gap":5},"ratio":{"M":1}}})).unwrap();
        assert!(
            !job["pieces"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["id"] == "front-M")
        );
        assert!(
            job["pieces"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["id"] == "front_dart-M")
        );
        action["request"]["production_mode"] = json!("additional");
        let extra =
            dispatch(&json!({"action":"edit","document":doc,"operations":[point,action]})).unwrap();
        assert_eq!(
            extra["pieces"]
                .as_array()
                .unwrap()
                .iter()
                .find(|p| p["id"] == "front")
                .unwrap()["quantity"],
            1
        );
        assert!(dispatch(&json!({"action":"spec_sheet","document":extra})).is_err());
    }
}
