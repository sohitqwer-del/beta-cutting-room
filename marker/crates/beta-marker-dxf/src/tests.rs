use super::*;
use beta_marker_core::{
    MarkKind, RotationRule,
    geometry::{area, bbox},
};

fn document(header: &str, blocks: &str, entities: &str) -> String {
    format!(
        "0\nSECTION\n2\nHEADER\n{header}0\nENDSEC\n0\nSECTION\n2\nBLOCKS\n{blocks}0\nENDSEC\n0\nSECTION\n2\nENTITIES\n{entities}0\nENDSEC\n0\nEOF\n"
    )
}
fn mm() -> &'static str {
    "9\n$INSUNITS\n70\n4\n"
}
fn block(name: &str, entities: &str) -> String {
    format!("0\nBLOCK\n2\n{name}\n10\n0\n20\n0\n{entities}0\nENDBLK\n")
}
fn insert(name: &str, params: &str) -> String {
    format!("0\nINSERT\n2\n{name}\n{params}")
}
fn square(layer: i32, size: f64, closed: bool) -> String {
    format!(
        "0\nLWPOLYLINE\n8\n{layer}\n90\n4\n70\n{}\n10\n0\n20\n0\n10\n{size}\n20\n0\n10\n{size}\n20\n{size}\n10\n0\n20\n{size}\n",
        u8::from(closed)
    )
}
fn text(value: &str) -> String {
    format!("0\nTEXT\n8\n15\n10\n5\n20\n5\n1\n{value}\n")
}
fn read(s: &str) -> ImportReport {
    import_dxf(s.as_bytes(), &ImportOptions::default()).expect("valid test DXF")
}

#[test]
fn real_seamly_aama_exports_metadata_visibility_units_and_pairs() {
    for (name, explicit_units, material, grain, pair_note) in [
        ("shirt-front-back-50", true, true, true, true),
        ("baseline-shirt-50", false, true, true, true),
        ("shirt-50-r27", true, true, true, true),
        ("default-labels-shirt-front-back", true, false, true, false),
        ("hidden-labels-shirt-front-back", true, false, true, false),
        ("hidden-grain-shirt-front-back", true, true, false, true),
    ] {
        let bytes = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../testdata/dxf/seamly")
                .join(format!("{name}.dxf")),
        )
        .unwrap();
        for pairs in [false, true] {
            let report = import_dxf(
                &bytes,
                &ImportOptions {
                    pairs,
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(report.units, Units::Mm, "{name}");
            assert_eq!(report.dialect, "aama");
            assert_eq!(report.sizes, ["50"]);
            assert_eq!(report.pieces.len(), 2);
            assert_eq!(
                report.warnings.iter().any(|w| w.contains("size heuristic")),
                !explicit_units,
                "{name}"
            );
            assert_eq!(
                report
                    .warnings
                    .iter()
                    .any(|w| w.contains("mirrored-pair note")),
                pair_note && !pairs,
                "{name}"
            );
            assert_eq!(
                report.warnings.iter().any(|w| w.contains("no grain line")),
                !grain,
                "{name}"
            );
            for expected in ["Shirt-front", "Shirt-back"] {
                let p = report.pieces.iter().find(|p| p.name == expected).unwrap();
                assert_eq!(
                    (p.quantity, p.mirror_quantity),
                    if pairs { (1, 1) } else { (2, 0) }
                );
                assert_eq!(p.material.as_deref(), material.then_some("cotton"));
                assert_eq!(p.grain.is_some(), grain);
                if let Some([a, b]) = p.grain {
                    assert!((a[0] - b[0]).abs() < 1e-6);
                    assert!(((a[1] - b[1]).abs() - 250.).abs() < 1e-6);
                }
                let bounds = bbox(&p.outline).unwrap();
                assert!(
                    (bounds[3] - bounds[1] - 673.).abs() < 1.,
                    "{name}: {bounds:?}"
                );
                assert_eq!(
                    p.marks.iter().filter(|m| m.kind == MarkKind::Line).count(),
                    2
                );
                // One semantic source notch is drawn on both stitch and cut
                // paths. Keep both DXF segments rather than merging them.
                assert_eq!(
                    p.marks.iter().filter(|m| m.kind == MarkKind::Notch).count(),
                    2
                );
                assert_eq!(
                    p.marks.iter().any(|m| m.kind == MarkKind::Text
                        && m.text
                            .as_deref()
                            .is_some_and(|t| t.to_ascii_lowercase().contains("mirrored pair"))),
                    pair_note
                );
            }
        }
    }
}

#[test]
fn empty_label_fields_do_not_hide_later_metadata() {
    let labels = text("Piece Name: ")
        + &text("Piece Name: Front")
        + &text("Size: ")
        + &text("Size: 50")
        + &text("Quantity: ")
        + &text("Quantity: 2");
    let report = read(&document(
        mm(),
        &block("P", &(square(1, 100., true) + &labels)),
        &insert("P", ""),
    ));
    let p = &report.pieces[0];
    assert_eq!(p.name, "Front");
    assert_eq!(p.size.as_deref(), Some("50"));
    assert_eq!(p.quantity, 2);
}

#[test]
fn bulges_include_closed_last_edge_and_signed_arcs() {
    let curve = "0\nLWPOLYLINE\n8\n1\n70\n1\n90\n2\n10\n0\n20\n0\n42\n1\n10\n100\n20\n0\n42\n1\n";
    let s = document(mm(), &block("Circle", curve), &insert("Circle", ""));
    let p = &read(&s).pieces[0];
    assert!((area(&p.outline) - std::f64::consts::PI * 2500.0).abs() / area(&p.outline) < 0.0005);
    assert!(p.outline.len() > 100);
    assert_ne!(p.outline.first(), p.outline.last());
    let negative = curve.replace("42\n1\n", "42\n-1\n");
    let r = read(&document(
        mm(),
        &block("Circle", &negative),
        &insert("Circle", ""),
    ));
    assert!((area(&r.pieces[0].outline) - area(&p.outline)).abs() < 1e-6);
}

#[test]
fn closed_open_and_segment_joining() {
    let open = read(&document(
        mm(),
        &block("Open", &square(1, 100.0, false)),
        &insert("Open", ""),
    ));
    assert!(open.warnings.iter().any(|w| w.contains("not closed")));
    assert_eq!(area(&open.pieces[0].outline), 10_000.0);
    let closed = read(&document(
        mm(),
        &block("Closed", &square(1, 100.0, true)),
        &insert("Closed", ""),
    ));
    assert!(!closed.warnings.iter().any(|w| w.contains("not closed")));
    let lines = "0\nLINE\n8\n1\n10\n0\n20\n0\n11\n100\n21\n0\n0\nLINE\n8\n1\n10\n100\n20\n100\n11\n100.02\n21\n0\n0\nLINE\n8\n1\n10\n0\n20\n100\n11\n100\n21\n100\n0\nLINE\n8\n1\n10\n0\n20\n0\n11\n0\n21\n100\n";
    let joined = read(&document(
        mm(),
        &block("Joined", lines),
        &insert("Joined", ""),
    ));
    assert!(!joined.warnings.iter().any(|w| w.contains("not closed")));
    assert_eq!(joined.pieces[0].outline.len(), 4);
    assert!((area(&joined.pieces[0].outline) - 10_000.0).abs() < 2.0);
}

#[test]
fn nested_insert_transform_base_point_layer_inheritance_and_grain_direction() {
    let grain = "0\nLINE\n8\n7\n10\n0\n20\n0\n11\n10\n21\n0\n";
    let inner = block("Inner", &(square(0, 10.0, true) + grain)).replacen(
        "10\n0\n20\n0\n",
        "10\n5\n20\n5\n",
        1,
    );
    let outer = block(
        "Outer",
        &insert("Inner", "8\n1\n10\n10\n20\n0\n41\n2\n42\n3\n50\n90\n"),
    );
    let s = document(
        mm(),
        &(inner + &outer),
        &insert("Outer", "10\n100\n20\n200\n50\n90\n"),
    );
    let report = read(&s);
    let p = &report.pieces[0];
    assert!((area(&p.outline) - 600.0).abs() < 1e-6);
    let bounds = bbox(&p.outline).expect("bounds");
    for (a, b) in bounds.into_iter().zip([90.0, 195.0, 110.0, 225.0]) {
        assert!((a - b).abs() < 1e-6, "{a} != {b}");
    }
    let g = p.grain.expect("grain");
    assert!(g[1][0] < g[0][0] && (g[1][1] - g[0][1]).abs() < 1e-6);
}

#[test]
fn unit_precedence_text_measurement_override_and_heuristic() {
    let blocks = block("P", &square(1, 20.0, true));
    for (header, extra, opts, expected) in [
        (
            "9\n$INSUNITS\n70\n1\n",
            text("UNITS: METRIC"),
            ImportOptions::default(),
            508.0,
        ),
        ("", text("UNITS: ENGLISH"), ImportOptions::default(), 508.0),
        (
            "9\n$MEASUREMENT\n70\n1\n",
            String::new(),
            ImportOptions::default(),
            20.0,
        ),
        ("", String::new(), ImportOptions::default(), 508.0),
        (
            mm(),
            String::new(),
            ImportOptions {
                units: Units::Cm,
                pairs: false,
            },
            200.0,
        ),
    ] {
        let report = import_dxf(
            document(header, &blocks, &(insert("P", "") + &extra)).as_bytes(),
            &opts,
        )
        .expect("units");
        assert!((bbox(&report.pieces[0].outline).expect("bounds")[2] - expected).abs() < 1e-6);
    }
}

#[test]
fn crlf_spaces_unknown_sections_and_r12_vertices() {
    let poly = "0\nPOLYLINE\n8\n1\n70\n1\n0\nVERTEX\n10\n0\n20\n0\n0\nVERTEX\n10\n100\n20\n0\n0\nVERTEX\n10\n0\n20\n100\n0\nSEQEND\n";
    let s = document(mm(), &block("P", poly), &insert("P", "")).replacen(
        "0\nSECTION\n",
        "0\nSECTION\n2\nUNKNOWN\n97\nAnything here\n0\nENDSEC\n0\nSECTION\n",
        1,
    );
    let spaced = s.lines().map(|s| format!("  {s}\r\n")).collect::<String>();
    assert_eq!(area(&read(&spaced).pieces[0].outline), 5000.0);
}

#[test]
fn arc_circle_spline_mtext_and_all_garment_layers() {
    let mut entities = square(1, 100.0, true);
    entities += "0\nARC\n8\n8\n10\n50\n20\n50\n40\n20\n50\n0\n51\n180\n";
    entities += "0\nCIRCLE\n8\n13\n10\n20\n20\n20\n40\n1\n";
    entities += "0\nPOINT\n8\n13\n10\n30\n20\n30\n";
    entities += "0\nSPLINE\n8\n11\n70\n0\n71\n2\n40\n0\n40\n0\n40\n0\n40\n1\n40\n1\n40\n1\n10\n0\n20\n0\n10\n50\n20\n100\n10\n100\n20\n0\n";
    entities += "0\nMTEXT\n8\n15\n10\n2\n20\n3\n3\nPiece Name: Spl\n1\nine\\PSize: M\n";
    entities += "0\nLINE\n8\n7\n10\n10\n20\n90\n11\n10\n21\n10\n0\nLINE\n8\n7\n10\n10\n20\n10\n11\n10\n21\n20\n";
    for layer in [2, 3, 4] {
        entities += &format!("0\nPOINT\n8\n{layer}\n10\n0\n20\n30\n");
    }
    entities += "0\nLINE\n8\n14\n10\n10\n20\n10\n11\n90\n21\n10\n";
    entities += &text("Rule Table: GRADES");
    entities += &text("Annotation: stitch here");
    let report = read(&document(mm(), &block("P", &entities), &insert("P", "")));
    let p = &report.pieces[0];
    assert_eq!(p.name, "Spline");
    assert_eq!(p.size.as_deref(), Some("M"));
    assert_eq!(p.grain, Some([[10.0, 90.0], [10.0, 10.0]]));
    assert_eq!(
        p.marks.iter().filter(|m| m.kind == MarkKind::Drill).count(),
        2
    );
    assert_eq!(
        p.marks.iter().filter(|m| m.kind == MarkKind::Notch).count(),
        1
    );
    assert!(
        p.marks
            .iter()
            .any(|m| m.kind == MarkKind::Line && m.points.len() > 30)
    );
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.contains("grading is not applied"))
    );
    let s = p
        .marks
        .iter()
        .find(|m| {
            m.kind == MarkKind::Line && m.points.len() > 30 && m.points.first() == Some(&[0.0, 0.0])
        })
        .expect("spline");
    assert!(
        s.points
            .iter()
            .any(|p| (p[0] - 50.0).abs() < 1e-6 && (p[1] - 50.0).abs() < 1e-6)
    );
}

#[test]
fn mirror_unfold_and_explicit_pairs() {
    let entities = square(1, 100.0, true)
        + "0\nLINE\n8\n6\n10\n0\n20\n0\n11\n0\n21\n100\n0\nPOINT\n8\n4\n10\n100\n20\n50\n"
        + &text("Quantity: 4");
    let s = document(mm(), &block("Half", &entities), &insert("Half", ""));
    let report = read(&s);
    let p = &report.pieces[0];
    assert_eq!(area(&p.outline), 20_000.0);
    assert_eq!(p.fold_line, Some([[0.0, 0.0], [0.0, 100.0]]));
    assert_eq!((p.quantity, p.mirror_quantity), (4, 0));
    assert_eq!(
        p.marks.iter().filter(|m| m.kind == MarkKind::Notch).count(),
        2
    );
    let paired = import_dxf(
        s.as_bytes(),
        &ImportOptions {
            pairs: true,
            ..Default::default()
        },
    )
    .expect("pairs");
    assert_eq!(
        (paired.pieces[0].quantity, paired.pieces[0].mirror_quantity),
        (2, 2)
    );
    assert!(report.warnings.iter().any(|w| w.contains("unfolded")));
}

#[test]
fn named_match_layers_annotations_and_insert_attributes() {
    let entities = square(1, 100.0, true)
        + "0\nPOINT\n8\nMATCH_SIDE\n10\n0\n20\n25\n"
        + "0\nTEXT\n8\n15\n10\n100\n20\n25\n1\nSTRIPE: chest\n"
        + "0\nPOINT\n8\n4\n10\n0\n20\n75\n";
    let s = document(
        mm(),
        &block("P", &entities),
        &(insert("P", "") + "0\nATTRIB\n2\nMaterial\n1\nSilk\n10\n0\n20\n0\n0\nSEQEND\n"),
    );
    let report = read(&s);
    assert_eq!(report.pieces[0].material.as_deref(), Some("Silk"));
    assert_eq!(report.pieces[0].match_points.len(), 2);
    assert_eq!(report.pieces[0].match_points[0].id, "MATCH_SIDE");
    assert_eq!(report.pieces[0].match_points[1].point, [100.0, 25.0]);
}

#[test]
fn grade_attributes_move_notches_and_match_points_and_reject_bad_anchors() {
    let mut entities = square(1, 100.0, true);
    for (x, y, id) in [(0, 0, 1), (100, 0, 2), (100, 100, 2), (0, 100, 1)] {
        entities += &format!("0\nATTRIB\n2\nRULE\n1\n{id}\n10\n{x}\n20\n{y}\n");
    }
    entities += "0\nPOINT\n8\n4\n10\n100\n20\n50\n0\nTEXT\n1\nMATCH: side\n10\n100\n20\n50\n";
    let table = crate::GradeTable::parse(
        b"SIZES: M L\nBASE: M\nMODE: CUMULATIVE\nRULE: 1 0,0 -2,0\nRULE: 2 0,0 2,0",
    )
    .expect("table");
    let s = document(mm(), &block("P", &entities), &insert("P", ""));
    let report = crate::import_dxf_with_rules(s.as_bytes(), &ImportOptions::default(), &table)
        .expect("grade");
    let large = &report.pieces[1];
    assert_eq!(large.match_points[0].point, [102.0, 50.0]);
    assert!(
        large
            .marks
            .iter()
            .any(|m| m.kind == MarkKind::Notch && m.points == vec![[102.0, 50.0]])
    );
    let rotated = document(mm(), &block("P", &entities), &insert("P", "50\n90\n"));
    assert!(
        crate::import_dxf_with_rules(rotated.as_bytes(), &ImportOptions::default(), &table)
            .is_err()
    );
    let unknown = s.replace("1\n1\n10\n0", "1\n99\n10\n0");
    assert!(
        crate::import_dxf_with_rules(unknown.as_bytes(), &ImportOptions::default(), &table)
            .is_err()
    );
}

#[test]
fn missing_boundary_skips_piece_and_ids_are_sanitized_unique() {
    let bad = block("Empty", &text("Piece Name: Empty"));
    let good = block(
        "Good",
        &(square(1, 100.0, true) + &text("Piece Name: fr/ont")),
    );
    let report = read(&document(
        mm(),
        &(bad + &good),
        &(insert("Empty", "") + &insert("Good", "") + &insert("Good", "")),
    ));
    assert_eq!(report.pieces.len(), 2);
    assert_eq!(report.pieces[0].id, "STYLE-FR-ONT-BASE");
    assert_eq!(report.pieces[1].id, "STYLE-FR-ONT-BASE-2");
    assert!(report.warnings.iter().any(|w| w.contains("no layer-1")));
}

#[test]
fn round_trip_preserves_geometry_rules_quantities_and_marks() {
    let fixture = include_bytes!("../../../testdata/dxf/shirt-aama-mm.dxf");
    let mut expected = import_dxf(
        fixture,
        &ImportOptions {
            pairs: true,
            ..Default::default()
        },
    )
    .expect("fixture")
    .pieces;
    expected[0].rotation = RotationRule::Quarter;
    expected[0].buffer = 2.0;
    expected[0].marks.push(beta_marker_core::Mark {
        kind: MarkKind::Text,
        points: vec![[12.0, 13.0]],
        text: Some("Piece Name: keep this annotation\nliteral \\P".into()),
    });
    expected[0].marks.push(beta_marker_core::Mark {
        kind: MarkKind::Text,
        points: vec![[12.0, 13.0]],
        text: None,
    });
    expected[0].marks.push(beta_marker_core::Mark {
        kind: MarkKind::Line,
        points: vec![[20.0, 30.0]],
        text: Some("reference point".into()),
    });
    expected[0].marks.push(beta_marker_core::Mark {
        kind: MarkKind::Line,
        points: vec![[20.0, 30.0], [25.0, 33.0], [40.0, 30.0]],
        text: Some("internal curve".into()),
    });
    let actual = read(&export_aama(&expected).expect("export")).pieces;
    assert_eq!(actual.len(), expected.len());
    for (mut a, mut b) in actual.into_iter().zip(expected) {
        assert_eq!(a.outline.len(), b.outline.len());
        for (p, q) in a.outline.iter().zip(&b.outline) {
            assert!(curves::distance(*p, *q) < 0.01);
        }
        a.outline.clear();
        b.outline.clear();
        for (x, y) in a.marks.iter_mut().zip(b.marks.iter_mut()) {
            assert_eq!(x.points.len(), y.points.len());
            for (p, q) in x.points.iter().zip(&y.points) {
                assert!(curves::distance(*p, *q) < 0.01);
            }
            x.points.clear();
            y.points.clear();
        }
        assert_eq!(a, b);
    }
}

#[test]
fn original_fixtures_counts_areas_grain_notches_and_drills() {
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("../../../testdata/dxf/expected.json"))
            .expect("manifest");
    for (name, bytes, count, dialect) in [
        (
            "shirt-aama-mm.dxf",
            include_bytes!("../../../testdata/dxf/shirt-aama-mm.dxf").as_slice(),
            32,
            "aama",
        ),
        (
            "trouser-aama-inch.dxf",
            include_bytes!("../../../testdata/dxf/trouser-aama-inch.dxf").as_slice(),
            20,
            "aama",
        ),
        (
            "tee-astm-mm.dxf",
            include_bytes!("../../../testdata/dxf/tee-astm-mm.dxf").as_slice(),
            12,
            "astm",
        ),
    ] {
        let report = import_dxf(bytes, &ImportOptions::default()).expect("fixture");
        assert_eq!(report.pieces.len(), count);
        assert_eq!(report.dialect, dialect);
        for (piece, reference) in report
            .pieces
            .iter()
            .zip(expected[name].as_array().expect("manifest pieces"))
        {
            let expected_area = reference["area_mm2"].as_f64().expect("area");
            assert!(
                (area(&piece.outline) - expected_area).abs() / expected_area < 0.005,
                "{name}: {}",
                piece.id
            );
            assert!(piece.grain.is_some());
            assert_eq!(piece.name, reference["name"].as_str().expect("name"));
            assert_eq!(piece.size.as_deref(), reference["size"].as_str());
            assert_eq!(
                piece.quantity,
                u32::try_from(reference["quantity"].as_u64().expect("quantity"))
                    .expect("quantity u32")
            );
            for (kind, key) in [(MarkKind::Notch, "notches"), (MarkKind::Drill, "drills")] {
                assert_eq!(
                    piece.marks.iter().filter(|m| m.kind == kind).count() as u64,
                    reference[key].as_u64().expect("count")
                );
            }
        }
    }
    let generic = import_dxf(
        include_bytes!("../../../testdata/dxf/generic-polylines.dxf"),
        &ImportOptions::default(),
    )
    .expect("generic");
    assert_eq!(generic.dialect, "generic");
    assert_eq!(generic.pieces.len(), 2);
    assert_eq!(area(&generic.pieces[0].outline), 150_000.0);
    assert_eq!(area(&generic.pieces[1].outline), 85_000.0);
    assert!(generic.pieces.iter().all(|p| p.grain.is_some()));
    assert_eq!(generic.pieces[0].marks.len(), 1);
    assert_eq!(generic.pieces[0].marks[0].kind, MarkKind::Line);
    assert_eq!(area(&generic.pieces[0].marks[0].points), 1600.0);
}

#[test]
fn malformed_and_binary_input_errors_without_panics() {
    assert!(
        import_dxf(b"AutoCAD Binary DXF\r\n\x1a\0", &ImportOptions::default())
            .expect_err("binary")
            .to_string()
            .contains("Binary DXF")
    );
    let cycle = document(mm(), &block("P", &insert("P", "")), &insert("P", ""));
    assert!(import_dxf(cycle.as_bytes(), &ImportOptions::default()).is_err());
    let mut state = 0x5eed_u64;
    for len in 0..1500 {
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                (state >> 32) as u8
            })
            .collect();
        assert!(import_dxf(&bytes, &ImportOptions::default()).is_err());
    }
    let valid = document(mm(), &block("P", &square(1, 100.0, true)), &insert("P", ""));
    for replacement in ["NaN", "inf", "-inf", "1e999", "garbage"] {
        let bad = valid.replace("100\n", &format!("{replacement}\n"));
        assert!(import_dxf(bad.as_bytes(), &ImportOptions::default()).is_err());
    }
    for i in (0..valid.len()).step_by(7) {
        let _ = import_dxf(&valid.as_bytes()[..i], &ImportOptions::default());
    }
}

#[test]
fn rational_spline_and_fit_point_fallback() {
    let points = curves::spline(
        &[[1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        &[1.0, 0.5_f64.sqrt(), 1.0],
        &[0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
        2,
        0.0001,
    )
    .expect("rational quarter circle");
    assert!(points.len() > 30);
    for p in points {
        assert!((p[0].hypot(p[1]) - 1.0).abs() < 1e-10);
    }
    let spline = "0\nSPLINE\n8\n8\n71\n3\n11\n10\n21\n10\n11\n20\n21\n30\n11\n30\n21\n10\n";
    let report = read(&document(
        mm(),
        &block("P", &(square(1, 100.0, true) + spline)),
        &insert("p", ""),
    ));
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.contains("fit points only"))
    );
    assert_eq!(
        report.pieces[0].marks[0].points,
        vec![[10.0, 10.0], [20.0, 30.0], [30.0, 10.0]]
    );
}

#[test]
fn heuristic_uses_piece_extent_and_notch_starts_at_boundary() {
    let mut segments = String::new();
    for y in [0, 50] {
        for x in (0..700).step_by(50) {
            segments += &format!("0\nLINE\n8\n1\n10\n{x}\n20\n{y}\n11\n{}\n21\n{y}\n", x + 50);
        }
    }
    for x in [0, 700] {
        segments += &format!("0\nLINE\n8\n1\n10\n{x}\n20\n0\n11\n{x}\n21\n50\n");
    }
    segments += "0\nLINE\n8\n4\n10\n12\n20\n25\n11\n0\n21\n25\n";
    let report = read(&document("", &block("P", &segments), &insert("P", "")));
    assert_eq!(report.units, Units::Mm);
    assert!((area(&report.pieces[0].outline) - 35_000.0).abs() < 1e-6);
    assert_eq!(
        report.pieces[0].marks[0].points,
        vec![[0.0, 25.0], [12.0, 25.0]]
    );
}
