//! Byte-preserving adapters to the native factory writers.
use super::{read, read_job, write};
use beta_marker_export::{self as output, ExportOptions};
use wasm_bindgen::prelude::*;

/// Factory formats only: avoid linking the workbook/ZIP writer into this module.
#[wasm_bindgen]
pub fn export_marker(
    job: &str,
    result: &str,
    format: &str,
    options: &str,
) -> Result<Vec<u8>, String> {
    let job = read_job(job)?;
    let result = read(result)?;
    let options: ExportOptions = read(options)?;
    let bytes = match format.to_ascii_lowercase().as_str() {
        "svg" => output::to_svg(&job, &result, &options).map(String::into_bytes),
        "plt" | "hpgl" => output::to_hpgl(&job, &result, &options).map(String::into_bytes),
        "pdf" => output::to_pdf(&job, &result, &options),
        "dxf" => output::to_dxf(&job, &result, &options).map(String::into_bytes),
        "iso" => {
            output::to_iso(&job, &result, &output::IsoProfile::default()).map(String::into_bytes)
        }
        _ => return Err("Choose svg, plt, hpgl, pdf, dxf or iso.".into()),
    };
    bytes.map_err(|e| format!("{e:#}"))
}

#[wasm_bindgen]
pub fn marker_labels(job: &str, placements: &str) -> Result<String, String> {
    write(
        &output::marker_labels(&read_job(job)?, &read::<Vec<_>>(placements)?)
            .map_err(|e| format!("{e:#}"))?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_writer_and_json_binding_are_byte_identical() {
        let f: serde_json::Value = serde_json::from_str(include_str!(
            "../../../packages/marker-wasm/test/fixtures/contract.json"
        ))
        .unwrap();
        let job = f["job"].to_string();
        let result = f["result"].to_string();
        for format in ["svg", "plt", "hpgl", "pdf", "dxf", "iso"] {
            for options in [
                "{}",
                r#"{"labels":false,"marks":false,"rotate":true,"pdf_tile_mm":210}"#,
            ] {
                let native = output::export(
                    format,
                    &read_job(&job).unwrap(),
                    &read(&result).unwrap(),
                    &read(options).unwrap(),
                )
                .unwrap()
                .0;
                assert_eq!(
                    export_marker(&job, &result, format, options).unwrap(),
                    native,
                    "{format} {options}"
                );
            }
        }
        assert!(export_marker(&job, &result, "unknown", "{}").is_err());
        assert!(export_marker("bad", &result, "pdf", "{}").is_err());
        assert!(marker_labels(&job, r#"[{"piece_id":"missing"}]"#).is_err());
    }
}
