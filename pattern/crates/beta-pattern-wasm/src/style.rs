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
}
