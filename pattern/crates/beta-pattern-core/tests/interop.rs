//! Original independently authored interoperability data; Apache-2.0 notice accompanies fixtures.
use beta_pattern_core::{evaluate, exchange, geometry::*, measurements, seamly};
const MS: &str = include_str!("fixtures/interop/original-multisize.smms");
#[test]
fn original_multisize_values_match_original_individual_tables() {
    let set = measurements::import_seamly_sizes(MS, &[48., 50., 52.], &[176.]).unwrap();
    for (size, xml) in [
        ("48", include_str!("fixtures/interop/original-size-48.smis")),
        ("50", include_str!("fixtures/interop/original-size-50.smis")),
        ("52", include_str!("fixtures/interop/original-size-52.smis")),
    ] {
        assert_eq!(
            set.values[size],
            measurements::import_seamly(xml, size).unwrap().values[size]
        );
    }
    assert_eq!(set.values["48"]["waist"], 740.);
    assert_eq!(set.values["52"]["waist"], 820.);
}
#[test]
fn original_garment_files_import_grade_and_dxf_roundtrip() {
    for (name, xml, count, material) in [
        (
            "skirt",
            include_str!("fixtures/interop/skirt-block.sm2d"),
            2,
            "cotton",
        ),
        (
            "shirt",
            include_str!("fixtures/interop/shirt-front-back.sm2d"),
            2,
            "cotton",
        ),
        (
            "trouser",
            include_str!("fixtures/interop/trouser-front.sm2d"),
            1,
            "twill",
        ),
    ] {
        let report = seamly::import_multisize(xml, MS).unwrap_or_else(|e| panic!("{name}: {e:#}"));
        let doc = report.document;
        assert_eq!(doc.sizes, ["48", "50", "52"]);
        assert_eq!(doc.base_size, "50");
        assert_eq!(doc.pieces.len(), count);
        let mut previous = 0.;
        for size in &doc.sizes {
            let g = evaluate::grade(&doc, size, 0.1).unwrap();
            let a: f64 = g.iter().map(|g| area(&g.seam)).sum();
            assert!(a > previous);
            previous = a;
            for p in &g {
                assert_eq!(p.piece.quantity, 2);
                assert_eq!(p.piece.mirror_quantity, 0);
                assert_eq!(p.piece.material.as_deref(), Some(material));
                assert!(p.piece.grain.is_some());
                assert!(
                    p.piece
                        .marks
                        .iter()
                        .any(|m| m.kind == beta_marker_core::MarkKind::Line)
                );
                if xml.contains("notch=\"true\"") {
                    assert!(
                        p.piece
                            .marks
                            .iter()
                            .any(|m| m.kind == beta_marker_core::MarkKind::Notch)
                    );
                }
            }
        }
        let dxf = exchange::export_dxf(&doc, &doc.sizes, 0.1).unwrap();
        let round = beta_marker_dxf::import_dxf(dxf.as_bytes(), &Default::default()).unwrap();
        assert_eq!(round.pieces.len(), count * 3);
        for size in &doc.sizes {
            for g in evaluate::grade(&doc, size, 0.1).unwrap() {
                let p = round.pieces.iter().find(|p| p.id == g.piece.id).unwrap();
                assert!((area(&p.outline) - area(&g.piece.outline)).abs() < 0.01);
                assert_eq!(p.marks.len(), g.piece.marks.len());
            }
        }
        assert!(report.warnings.iter().any(|w| w.contains("mirrored-pair")));
    }
}
