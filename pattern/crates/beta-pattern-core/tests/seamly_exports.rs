//! Comparisons with independently exported original Apache-2.0 data.
use beta_marker_core::{MarkKind, Pt};
use beta_pattern_core::{
    evaluate,
    geometry::{area, bbox, distance},
    seamly,
};
use std::path::Path;

fn fixture(name: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../testdata/seamly")
            .join(name),
    )
    .unwrap()
}
fn segment_distance(p: Pt, a: Pt, b: Pt) -> f64 {
    let d = [b[0] - a[0], b[1] - a[1]];
    let n = d[0] * d[0] + d[1] * d[1];
    let t = if n == 0. {
        0.
    } else {
        ((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / n
    }
    .clamp(0., 1.);
    distance(p, [a[0] + t * d[0], a[1] + t * d[1]])
}
// Bidirectional boundary distance, including edge interiors every <= 0.1 mm.
// Distance to a boundary is 1-Lipschitz, so max + 0.05 mm bounds unsampled points.
fn directed(a: &[Pt], b: &[Pt]) -> f64 {
    a.iter()
        .zip(a.iter().cycle().skip(1))
        .take(a.len())
        .map(|(p, q)| {
            let steps = (distance(*p, *q) / 0.1).ceil().max(1.) as usize;
            (0..=steps)
                .map(|i| {
                    let t = i as f64 / steps as f64;
                    let r = [p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])];
                    b.iter()
                        .zip(b.iter().cycle().skip(1))
                        .take(b.len())
                        .map(|(s, u)| segment_distance(r, *s, *u))
                        .fold(f64::INFINITY, f64::min)
                })
                .fold(0., f64::max)
        })
        .fold(0., f64::max)
}
fn aligned(source: &[Pt], reference: &[Pt]) -> Vec<Pt> {
    let a = bbox(source).unwrap();
    let b = bbox(reference).unwrap();
    source
        .iter()
        .map(|p| [p[0] + b[0] - a[0], p[1] + b[1] - a[1]])
        .collect()
}

#[test]
fn boundary_distance_checks_edge_interiors() {
    let square = [[0., 0.], [2., 0.], [2., 2.], [0., 2.]];
    // Same vertices, but diagonals cross the square interior. A vertex-only
    // comparison would incorrectly report zero displacement.
    let crossed = [square[0], square[2], square[1], square[3]];
    assert!((directed(&crossed, &square) - 1.).abs() < 0.05);
    assert!(directed(&square, &square) < 1e-12);
}

#[test]
fn explicit_copy_rules_preserve_totals_and_reject_bad_mappings() {
    let ms = fixture("original-multisize.smms");
    let expected: serde_json::Value = serde_json::from_str(&fixture("expected.json")).unwrap();
    for case in expected.as_array().unwrap() {
        let xml = fixture(case["file"].as_str().unwrap());
        let default = seamly::import_multisize(&xml, &ms).unwrap();
        let mut options = seamly::ImportOptions::default();
        for piece in &default.document.pieces {
            let want = case["pieces"]
                .as_array()
                .unwrap()
                .iter()
                .find(|p| p["name"] == piece.name)
                .unwrap();
            let mirrored = want["intended_mirror_quantity"].as_u64().unwrap() as u32;
            options.copy_rules.insert(
                piece.id.clone(),
                seamly::CopyRule {
                    normal: piece.quantity - mirrored,
                    mirrored,
                },
            );
        }
        let json = serde_json::to_string(&options).unwrap();
        let options: seamly::ImportOptions = serde_json::from_str(&json).unwrap();
        let paired = seamly::import_multisize_with_options(&xml, &ms, &options).unwrap();
        assert!(
            !paired
                .warnings
                .iter()
                .any(|w| w.contains("mirrored-pair annotation"))
        );
        assert!(paired.warnings.iter().any(|w| w.contains("BP-004")));
        for size in &paired.document.sizes {
            for p in evaluate::grade(&paired.document, size, 0.1).unwrap() {
                assert_eq!((p.piece.quantity, p.piece.mirror_quantity), (1, 1));
            }
        }
        let first = default.document.pieces[0].id.clone();
        for (normal, mirrored) in [(0, 0), (1, 0), (2, 1), (u32::MAX, 1)] {
            let mut invalid = seamly::ImportOptions::default();
            invalid
                .copy_rules
                .insert(first.clone(), seamly::CopyRule { normal, mirrored });
            assert!(seamly::import_with_options(&xml, Some(&ms), "50", &invalid).is_err());
        }
        let mut unknown = seamly::ImportOptions::default();
        unknown.copy_rules.insert(
            "typo".into(),
            seamly::CopyRule {
                normal: 1,
                mirrored: 1,
            },
        );
        assert!(seamly::import_with_options(&xml, Some(&ms), "50", &unknown).is_err());
        let mut partial = seamly::ImportOptions::default();
        partial.copy_rules.insert(
            first,
            seamly::CopyRule {
                normal: 1,
                mirrored: 1,
            },
        );
        let report = seamly::import_with_options(&xml, Some(&ms), "50", &partial).unwrap();
        let warnings = report
            .warnings
            .iter()
            .filter(|w| w.contains("mirrored-pair annotation"))
            .count();
        assert_eq!(warnings, default.document.pieces.len() - 1);
        let forbid = xml.replace("forbidFlipping=\"false\"", "forbidFlipping=\"true\"");
        let report = seamly::import(&forbid, Some(&ms), "50").unwrap();
        assert!(
            report
                .document
                .pieces
                .iter()
                .all(|p| p.quantity == 2 && p.mirror_quantity == 0)
        );
    }
}

#[test]
fn hidden_native_grain_is_omitted_with_actionable_warning() {
    let xml = fixture("shirt-front-back.sm2d")
        .replace("grainline visible=\"true\"", "grainline visible=\"false\"");
    let report = seamly::import_multisize(&xml, &fixture("original-multisize.smms")).unwrap();
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.contains("grain is hidden"))
    );
    assert!(!report.warnings.iter().any(|w| w.contains("BP-004")));
    for p in evaluate::grade(&report.document, "50", 0.1).unwrap() {
        assert!(p.piece.grain.is_none());
    }
}
#[test]
fn native_against_real_seamly_exports() {
    let expected: serde_json::Value = serde_json::from_str(&fixture("expected.json")).unwrap();
    let ms = fixture("original-multisize.smms");
    let mut max_grain_delta: f64 = 0.;
    for case in expected.as_array().unwrap() {
        let file = case["file"].as_str().unwrap();
        let xml = fixture(file);
        let report = seamly::import_multisize(&xml, &ms).unwrap();
        for size in ["48", "50", "52"] {
            for tolerance in [0.1, 0.01] {
                let individual = seamly::import(
                    &xml,
                    Some(&fixture(&format!("original-size-{size}.smis"))),
                    size,
                )
                .unwrap();
                let native = evaluate::grade(&report.document, size, tolerance).unwrap();
                let single = evaluate::grade(&individual.document, size, tolerance).unwrap();
                let dxf = fixture(&format!(
                    "exports/{}-{size}.dxf",
                    file.trim_end_matches(".sm2d")
                ));
                let reference =
                    beta_marker_dxf::import_dxf(dxf.as_bytes(), &Default::default()).unwrap();
                assert_eq!(reference.units, beta_marker_dxf::Units::Mm);
                assert_eq!(native.len(), case["pieces"].as_array().unwrap().len());
                assert_eq!(reference.pieces.len(), native.len());
                for g in &native {
                    let p = &g.piece;
                    let want = case["pieces"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|e| e["name"] == p.name)
                        .unwrap();
                    let d = reference.pieces.iter().find(|d| d.name == p.name).unwrap();
                    assert_eq!(p.quantity, want["quantity"].as_u64().unwrap() as u32);
                    assert_eq!(p.mirror_quantity, 0);
                    assert_eq!(p.material.as_deref(), want["material"].as_str());
                    assert_eq!(d.quantity, p.quantity);
                    assert_eq!(d.size.as_deref(), Some(size));
                    assert_eq!(d.material, p.material);
                    assert!(d.grain.is_some());
                    let s = single.iter().find(|s| s.piece.name == p.name).unwrap();
                    assert_eq!(g.seam, s.seam);
                    assert_eq!(p.outline, s.piece.outline);
                    let stitch = d
                        .marks
                        .iter()
                        .filter(|m| m.kind == MarkKind::Line)
                        .max_by(|a, b| area(&a.points).total_cmp(&area(&b.points)))
                        .unwrap();
                    let seam = aligned(&g.seam, &stitch.points);
                    // One rigid translation for both contours also checks their
                    // relative allowance placement. Never fit them independently.
                    let shift = [seam[0][0] - g.seam[0][0], seam[0][1] - g.seam[0][1]];
                    let cut: Vec<_> = p
                        .outline
                        .iter()
                        .map(|p| [p[0] + shift[0], p[1] + shift[1]])
                        .collect();
                    let seam_distance =
                        directed(&seam, &stitch.points).max(directed(&stitch.points, &seam));
                    let cut_distance = directed(&cut, &d.outline).max(directed(&d.outline, &cut));
                    let budget = if tolerance == 0.01 { 0.15 } else { 0.5 };
                    assert!(
                        seam_distance + 0.05 <= budget,
                        "{} {size}: seam distance {seam_distance}",
                        p.name
                    );
                    assert!(
                        cut_distance + 0.05 <= budget,
                        "{} {size}: cut distance {cut_distance}",
                        p.name
                    );
                    let area_budget = if tolerance == 0.01 { 2. } else { 10. };
                    assert!((area(&seam) - area(&stitch.points)).abs() < area_budget);
                    assert!((area(&cut) - area(&d.outline)).abs() < area_budget);
                    for (a, b) in [(&g.seam, &stitch.points), (&p.outline, &d.outline)] {
                        let a = bbox(a).unwrap();
                        let b = bbox(b).unwrap();
                        assert!(((a[2] - a[0]) - (b[2] - b[0])).abs() < budget);
                        assert!(((a[3] - a[1]) - (b[3] - b[1])).abs() < budget);
                    }
                    let grain = p.grain.unwrap();
                    let reference_grain = d.grain.unwrap();
                    assert!(
                        (distance(grain[0], grain[1])
                            - distance(reference_grain[0], reference_grain[1]))
                        .abs()
                            < 1e-6
                    );
                    let direction = |g: [Pt; 2]| {
                        [
                            (g[1][0] - g[0][0]) / distance(g[0], g[1]),
                            (g[1][1] - g[0][1]) / distance(g[0], g[1]),
                        ]
                    };
                    assert!(distance(direction(grain), direction(reference_grain)) < 1e-6);
                    let a = bbox(&p.outline).unwrap();
                    let b = bbox(&d.outline).unwrap();
                    let delta = distance(
                        [grain[0][0] + b[0] - a[0], grain[0][1] + b[1] - a[1]],
                        reference_grain[0],
                    );
                    max_grain_delta = max_grain_delta.max(delta);
                    assert!(
                        report
                            .warnings
                            .iter()
                            .any(|w| w.contains("BP-004") && w.contains(&p.name))
                    );
                    assert_eq!(
                        report.document.meta["seamly_import_warnings"],
                        serde_json::to_value(&report.warnings).unwrap()
                    );
                    eprintln!(
                        "{} {size} tolerance {tolerance}: seam {seam_distance:.6}, cut {cut_distance:.6}, seam area {:.6}, cut area {:.6}",
                        p.name,
                        (area(&seam) - area(&stitch.points)).abs(),
                        (area(&cut) - area(&d.outline)).abs()
                    );
                }
            }
        }
    }
    assert!(
        (24.9..25.1).contains(&max_grain_delta),
        "grain warning must retain the original placement regression: {max_grain_delta}"
    );
}

#[test]
fn current_simple_splines_reject_unvalidated_shape_coefficients() {
    let xml =
        fixture("shirt-front-back.sm2d").replace("type=\"simple\"", "type=\"simple\" kAsm1=\"2\"");
    let err = seamly::import_multisize(&xml, &fixture("original-multisize.smms")).unwrap_err();
    assert!(
        err.to_string()
            .contains("non-default simple spline coefficient")
    );
    assert!(
        serde_json::from_str::<seamly::CopyRule>(r#"{"normal":1,"mirrored":1,"typo":2}"#).is_err()
    );
}
