//! Quick Marker's JSON boundary. All computation and validation stays in Rust.
use super::{read, read_job, write};
use beta_marker_core::{Layout, Nap, Pt, Spacing};
use beta_pattern_digitize::{assist, quick};
use serde::{Deserialize, Serialize};
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
    let pieces: Vec<quick::QuickPiece> = read(pieces)?;
    let setup: PaperFabric = read(fabric)?;
    setup.validate(&pieces)?;
    let mut job = quick::pieces_to_job(
        &pieces,
        &quick::QuickFabric {
            width_mm: setup.width_mm,
            nap: setup.nap,
            layout: setup.layout,
        },
        read(garments)?,
    )
    .map_err(message)?;
    if let Some(name) = &setup.set_name {
        job.name = name.clone();
    }
    if let Some(material) = &setup.material {
        job.name = format!("{} · {}", job.name, material);
    }
    job.spacing = Spacing {
        piece_gap: setup.piece_gap_mm,
        edge_gap: setup.edge_gap_mm,
    };
    job.meta.insert(
        "beta_paper_fabric".into(),
        serde_json::json!({
            "gsm": setup.gsm, "material": setup.material, "construction": setup.construction,
            "set_name": setup.set_name,
        }),
    );
    write(&job)
}

/// Capture metadata never implies that GSM determines fit, stretch or shrinkage.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PaperFabric {
    width_mm: f64,
    #[serde(default)]
    nap: Nap,
    #[serde(default)]
    layout: Layout,
    set_name: Option<String>,
    material: Option<String>,
    gsm: Option<f64>,
    construction: Option<String>,
    #[serde(default)]
    piece_gap_mm: f64,
    #[serde(default)]
    edge_gap_mm: f64,
}
impl PaperFabric {
    fn validate(&self, pieces: &[quick::QuickPiece]) -> Result<(), String> {
        for (name, value) in [
            ("Pattern set name", &self.set_name),
            ("Fabric group", &self.material),
            ("Construction", &self.construction),
        ] {
            if value
                .as_ref()
                .is_some_and(|v| v.trim().is_empty() || v.chars().count() > 160)
            {
                return Err(format!("{name} must contain 1..160 characters"));
            }
        }
        if self
            .gsm
            .is_some_and(|v| !v.is_finite() || v <= 0. || v > 5000.)
        {
            return Err("GSM must be greater than zero and at most 5000".into());
        }
        if [self.piece_gap_mm, self.edge_gap_mm]
            .iter()
            .any(|v| !v.is_finite() || *v < 0. || *v > 100.)
        {
            return Err("Piece and edge gaps must be between 0 and 100 mm".into());
        }
        let group = pieces
            .first()
            .map(|p| p.piece.material.as_deref().unwrap_or("main"));
        if pieces
            .iter()
            .any(|p| Some(p.piece.material.as_deref().unwrap_or("main")) != group)
            || self.material.as_deref().is_some_and(|m| Some(m) != group)
        {
            return Err("Make a separate marker for each fabric group".into());
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TapeRequest {
    endpoints: [Pt; 2],
    measured_mm: f64,
    tolerance_mm: f64,
}
#[derive(Serialize)]
pub struct TapeReport {
    endpoints: [Pt; 2],
    digital_mm: f64,
    measured_mm: f64,
    error_mm: f64,
    tolerance_mm: f64,
    pass: bool,
}

/// Snap operator taps to the outline and compare a straight distance with a tape.
/// This is one local check, not a certificate of the entire outline's accuracy.
#[wasm_bindgen]
pub fn quick_check_tape(piece: &str, request: &str) -> Result<String, String> {
    let piece: quick::QuickPiece = read(piece)?;
    assist::validate_pieces(std::slice::from_ref(&piece)).map_err(message)?;
    let r: TapeRequest = read(request)?;
    if !r.measured_mm.is_finite()
        || r.measured_mm <= 0.
        || r.measured_mm > 10000.
        || !r.tolerance_mm.is_finite()
        || !(0.1..=10.).contains(&r.tolerance_mm)
        || r.endpoints
            .iter()
            .flatten()
            .any(|v| !v.is_finite() || v.abs() > 100000.)
    {
        return Err(
            "Enter a positive tape distance up to 10000 mm and a tolerance of 0.1..10 mm".into(),
        );
    }
    let outline = &piece.piece.outline;
    let snap = |p: Pt| -> Pt {
        outline
            .iter()
            .zip(outline.iter().cycle().skip(1))
            .take(outline.len())
            .map(|(a, b)| {
                let dx = b[0] - a[0];
                let dy = b[1] - a[1];
                let d = dx * dx + dy * dy;
                let t = if d > 0. {
                    (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / d).clamp(0., 1.)
                } else {
                    0.
                };
                [a[0] + t * dx, a[1] + t * dy]
            })
            .min_by(|a, b| {
                ((a[0] - p[0]).hypot(a[1] - p[1])).total_cmp(&((b[0] - p[0]).hypot(b[1] - p[1])))
            })
            .unwrap()
    };
    let endpoints = r.endpoints.map(snap);
    let digital_mm = (endpoints[0][0] - endpoints[1][0]).hypot(endpoints[0][1] - endpoints[1][1]);
    if digital_mm < 1. {
        return Err("Choose two different points at least 1 mm apart".into());
    }
    let error_mm = digital_mm - r.measured_mm;
    write(&TapeReport {
        endpoints,
        digital_mm,
        measured_mm: r.measured_mm,
        error_mm,
        tolerance_mm: r.tolerance_mm,
        pass: error_mm.abs() <= r.tolerance_mm,
    })
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
    fn panel() -> quick::QuickPiece {
        quick::QuickPiece::new(
            Piece {
                id: "p".into(),
                name: "Panel".into(),
                size: Some("M".into()),
                material: Some("main".into()),
                outline: vec![[0., 0.], [100., 0.], [100., 200.], [0., 200.]],
                grain: Some([[0., 0.], [0., 200.]]),
                quantity: 1,
                ..Default::default()
            },
            "photo-1",
            1.,
        )
    }
    #[test]
    fn tape_check_snaps_and_compares_without_resizing_the_pattern() {
        let p = serde_json::to_string(&panel()).unwrap();
        let checked: serde_json::Value = serde_json::from_str(
            &quick_check_tape(
                &p,
                r#"{"endpoints":[[0.2,0.1],[99.8,0.1]],"measured_mm":100,"tolerance_mm":1}"#,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(checked["pass"], true);
        assert!((checked["digital_mm"].as_f64().unwrap() - 100.).abs() < 0.5);
        let failed: serde_json::Value = serde_json::from_str(
            &quick_check_tape(
                &p,
                r#"{"endpoints":[[0,0],[100,0]],"measured_mm":95,"tolerance_mm":3}"#,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(failed["error_mm"], 5.);
        assert_eq!(failed["pass"], false);
        for r in [
            r#"{"endpoints":[[0,0],[0,0]],"measured_mm":100,"tolerance_mm":3}"#,
            r#"{"endpoints":[[0,0],[100,0]],"measured_mm":0,"tolerance_mm":3}"#,
            r#"{"endpoints":[[0,0],[100,0]],"measured_mm":100,"tolerance_mm":100}"#,
        ] {
            assert!(quick_check_tape(&p, r).is_err());
        }
    }
    #[test]
    fn paper_groups_are_separate_and_gsm_does_not_change_geometry() {
        let p = panel();
        let input = serde_json::to_string(&vec![p.clone()]).unwrap();
        let job: beta_marker_core::Job = serde_json::from_str(&quick_pieces_to_job(&input,
            r#"{"width_mm":1500,"set_name":"School shirt","material":"main","gsm":180,"construction":"woven","piece_gap_mm":3,"edge_gap_mm":8}"#,r#"{"M":2}"#).unwrap()).unwrap();
        assert_eq!(job.pieces[0].outline, p.piece.outline);
        assert_eq!(job.spacing.piece_gap, 3.);
        assert_eq!(job.spacing.edge_gap, 8.);
        assert_eq!(job.meta["beta_paper_fabric"]["gsm"], 180.);
        assert_eq!(job.name, "School shirt · main");
        let mut lining = p.clone();
        lining.piece.id = "lining".into();
        lining.piece.material = Some("lining".into());
        let mixed = serde_json::to_string(&vec![p, lining]).unwrap();
        assert!(
            quick_pieces_to_job(&mixed, r#"{"width_mm":1500}"#, r#"{"M":1}"#)
                .unwrap_err()
                .contains("separate marker")
        );
        for fabric in [
            r#"{"width_mm":1500,"gsm":-1}"#,
            r#"{"width_mm":1500,"edge_gap_mm":101}"#,
            r#"{"width_mm":1500,"material":"lining"}"#,
        ] {
            assert!(quick_pieces_to_job(&input, fabric, r#"{"M":1}"#).is_err());
        }
    }
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
