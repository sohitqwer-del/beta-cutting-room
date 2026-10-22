//! The browser uses the same DXF and optional rule-table importer as the CLI.
use super::{read, write};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn import_dxf(bytes: &[u8], options: &str, rules: Option<String>) -> Result<String, String> {
    let options = read(options)?;
    let result = if let Some(rules) = rules {
        let table =
            beta_marker_dxf::GradeTable::parse(rules.as_bytes()).map_err(|e| format!("{e:#}"))?;
        beta_marker_dxf::import_dxf_with_rules(bytes, &options, &table)
    } else {
        beta_marker_dxf::import_dxf(bytes, &options)
    };
    write(&result.map_err(|e| format!("{e:#}"))?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn browser_import_preserves_native_report_and_grade_rules() {
        for bytes in [
            include_bytes!("../../../testdata/dxf/shirt-aama-mm.dxf").as_slice(),
            include_bytes!("../../../testdata/dxf/grade-base-mm.dxf").as_slice(),
        ] {
            let expected = beta_marker_dxf::import_dxf(bytes, &Default::default()).unwrap();
            let actual: serde_json::Value = read(&import_dxf(bytes, "{}", None).unwrap()).unwrap();
            assert_eq!(actual, serde_json::to_value(expected).unwrap());
        }
        let rules = include_str!("../../../testdata/dxf/grade-cumulative.rul");
        let actual: serde_json::Value = read(
            &import_dxf(
                include_bytes!("../../../testdata/dxf/grade-base-mm.dxf"),
                "{}",
                Some(rules.into()),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(actual["sizes"], serde_json::json!(["S", "M", "L"]));
        assert!(import_dxf(b"bad", "{}", None).is_err());
    }
}
