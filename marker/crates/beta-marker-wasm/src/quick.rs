//! Quick Marker's JSON boundary. All computation and validation stays in Rust.
use super::{read, read_job, write};
use beta_pattern_digitize::{assist, quick};
use wasm_bindgen::prelude::*;
fn message(e: impl std::fmt::Display) -> String {
    e.to_string()
}
#[wasm_bindgen]
pub fn quick_scan(photo: &[u8], options: &str) -> Result<String, String> {
    write(&quick::scan_bytes(photo, &read(options)?).map_err(message)?)
}
#[wasm_bindgen]
pub fn quick_assist(pieces: &str) -> Result<String, String> {
    let mut pieces: Vec<quick::QuickPiece> = read(pieces)?;
    assist::suggest(&mut pieces).map_err(message)?;
    write(&pieces)
}
#[wasm_bindgen]
pub fn quick_merge(scans: &str) -> Result<String, String> {
    write(&quick::merge(&read::<Vec<quick::SessionPhoto>>(scans)?).map_err(message)?)
}
#[wasm_bindgen]
pub fn quick_pieces_to_job(pieces: &str, fabric: &str, garments: &str) -> Result<String, String> {
    write(
        &quick::pieces_to_job(
            &read::<Vec<quick::QuickPiece>>(pieces)?,
            &read(fabric)?,
            read(garments)?,
        )
        .map_err(message)?,
    )
}
#[wasm_bindgen]
pub fn tiled_marker_pdf(job: &str, result: &str, options: &str) -> Result<Vec<u8>, String> {
    beta_marker_export::tiled_marker_pdf(&read_job(job)?, &read(result)?, &read(options)?)
        .map_err(message)
}
#[wasm_bindgen]
pub fn chalk_guide_pdf(job: &str, result: &str, options: &str) -> Result<Vec<u8>, String> {
    beta_marker_export::chalk_guide_pdf(&read_job(job)?, &read(result)?, &read(options)?)
        .map_err(message)
}
#[wasm_bindgen]
pub fn projector_layout(job: &str, result: &str) -> Result<String, String> {
    write(&beta_marker_export::projector_layout(&read_job(job)?, &read(result)?).map_err(message)?)
}
#[cfg(test)]
mod tests {
    use super::*;
    use beta_marker_core::Piece;
    #[test]
    fn quick_json_contract_retains_review_fields_and_rejects_missing_size() {
        let mut p = quick::QuickPiece::new(
            Piece {
                id: "p".into(),
                name: "Panel".into(),
                outline: vec![[0., 0.], [100., 0.], [100., 200.], [0., 200.]],
                quantity: 1,
                ..Default::default()
            },
            "photo-1",
            1.,
        );
        let input = serde_json::to_string(&vec![p.clone()]).unwrap();
        let suggested: serde_json::Value =
            serde_json::from_str(&quick_assist(&input).unwrap()).unwrap();
        assert_eq!(suggested[0]["quantity"], 1);
        assert_eq!(suggested[0]["mirror_quantity"], 0);
        assert_eq!(suggested[0]["photoId"], "photo-1");
        assert!(suggested[0]["suggestions"]["grain"].is_object());
        assert!(quick_pieces_to_job(&input, r#"{"width_mm":1500}"#, r#"{"M":1}"#).is_err());
        p.piece.size = Some("M".into());
        p.piece.grain = Some([[0., 0.], [0., 200.]]);
        let input = serde_json::to_string(&vec![p]).unwrap();
        let job: beta_marker_core::Job = serde_json::from_str(
            &quick_pieces_to_job(&input, r#"{"width_mm":1500}"#, r#"{"M":1}"#).unwrap(),
        )
        .unwrap();
        assert_eq!(job.pieces.len(), 1);
        assert!(quick_scan(b"bad", r#"{"reference":"a4-sheet"}"#).is_err());
        assert!(quick_assist("[]").is_err());
        assert!(quick_merge("[]").is_err());
    }
}
