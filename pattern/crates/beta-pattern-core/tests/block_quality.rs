use beta_pattern_core::{
    Value, block_checks,
    blocks::{self, BlockKind, Create},
    spec_export,
    style::{self, Operation},
};

fn draft(k: BlockKind) -> beta_pattern_core::Document {
    blocks::create(&Create {
        block: Some(k),
        ..Default::default()
    })
    .unwrap()
}

#[test]
fn every_block_size_passes_measured_quality_and_specs_in_both_examples() {
    let mut sizes = 0;
    let mut assertions = 0;
    for preset in ["indian_adult", "iso_example"] {
        for info in blocks::catalog() {
            let doc = blocks::create(&Create {
                block: Some(info.id),
                chart: Some(blocks::size_chart(preset).unwrap()),
                ..Default::default()
            })
            .unwrap();
            let report = block_checks::report(&doc).unwrap();
            for size in &report.sizes {
                sizes += 1;
                assertions += size.checks.len();
                assert!(
                    size.pass,
                    "{preset} {:?} {}: {:?} {:?}",
                    info.id,
                    size.size,
                    size.errors,
                    size.checks.iter().filter(|c| !c.pass).collect::<Vec<_>>()
                );
                for category in [
                    "seam",
                    "tangent",
                    "grain",
                    "notch",
                    "allowance",
                    "geometry",
                    "finished_measurement",
                    "chart",
                ] {
                    assert!(
                        size.checks.iter().any(|c| c.category == category),
                        "missing {category}"
                    );
                }
            }
            let spec = block_checks::spec_sheet(&doc).unwrap();
            assert_eq!(spec.sizes, doc.sizes);
            for row in &spec.rows {
                assert_eq!(row.values.len(), 5);
                assert!(row.values[0].grade_step_mm.is_none());
                for pair in row.values.windows(2) {
                    assert!(
                        (pair[1].grade_step_mm.unwrap()
                            - (pair[1].measured_mm - pair[0].measured_mm))
                            .abs()
                            < 1e-9
                    );
                }
                assert!(row.values.iter().all(|v| v.measured_mm.is_finite()
                    && v.measured_mm > 0.
                    && v.pass != Some(false)));
            }
        }
    }
    assert_eq!(sizes, 70);
    assert!(assertions > 5000);
    eprintln!("70 block/size rows; {assertions} measured assertions");
}

#[test]
fn edits_are_measured_instead_of_hiding_behind_chart_values() {
    let mut doc = draft(BlockKind::TShirt);
    // A valid symmetric geometry edit changes the hem without changing requested body length.
    for id in ["front.A", "front.B"] {
        if let beta_pattern_core::Object::Point {
            operation: beta_pattern_core::PointOp::Absolute { y, .. },
        } = doc.drawing.get_mut(id).unwrap()
        {
            let old = match y {
                Value::Formula(s) => s.clone(),
                _ => panic!(),
            };
            // 15 mm exceeds the proposed 10 mm body-length QC tolerance.
            *y = Value::Formula(format!("({old})+15"));
        }
    }
    let r = block_checks::report(&doc).unwrap();
    assert!(!r.pass);
    assert!(r.sizes.iter().all(|s| {
        s.checks
            .iter()
            .any(|c| c.id == "pom_body_length_hps" && !c.pass)
    }));
    let spec = block_checks::spec_sheet(&doc).unwrap();
    assert!(
        spec.rows
            .iter()
            .find(|r| r.id == "body_length_hps")
            .unwrap()
            .values
            .iter()
            .all(|v| v.pass == Some(false))
    );
}

#[test]
fn changed_allowances_missing_notches_and_excess_cap_ease_are_reported() {
    let mut doc = draft(BlockKind::TShirt);
    doc.pieces
        .iter_mut()
        .find(|p| p.id == "neck_rib")
        .unwrap()
        .annotations
        .clear();
    doc.pieces
        .iter_mut()
        .find(|p| p.id == "sleeve")
        .unwrap()
        .edges[0]
        .allowance = 7.0.into();
    let r = block_checks::report(&doc).unwrap();
    for row in r.sizes {
        assert!(
            row.checks
                .iter()
                .any(|c| c.id == "neck_rib.assembly_notches" && !c.pass)
        );
        assert!(
            row.checks
                .iter()
                .any(|c| c.id == "sleeve.allowance_0" && !c.pass)
        );
    }
    let edited = style::edit(
        &draft(BlockKind::TShirt),
        &[Operation::SetVariable {
            name: "cap_ease".into(),
            value: 30.0.into(),
        }],
    )
    .unwrap();
    assert!(
        blocks::seam_reports(&edited)
            .unwrap()
            .iter()
            .all(|r| r.rows.iter().all(|r| r.pass))
    );
    assert!(
        block_checks::report(&edited)
            .unwrap()
            .sizes
            .iter()
            .all(|s| s
                .checks
                .iter()
                .any(|c| c.id == "cap_ease_percent" && !c.pass))
    );
}

#[test]
fn a_broken_size_remains_visible_and_never_passes() {
    let mut doc = draft(BlockKind::Trouser);
    doc.measurements
        .get_mut("XL")
        .unwrap()
        .insert("inseam".into(), 100.);
    let r = block_checks::report(&doc).unwrap();
    assert_eq!(r.sizes.len(), 5);
    assert!(!r.pass);
    let bad = r.sizes.iter().find(|s| s.size == "XL").unwrap();
    assert!(!bad.pass);
    assert!(!bad.errors.is_empty());
}

#[test]
fn chart_review_catches_decreasing_and_unrealistic_grades() {
    let mut chart = blocks::size_chart("indian_adult").unwrap();
    chart
        .measurements
        .get_mut("L")
        .unwrap()
        .insert("chest".into(), 800.);
    let r = block_checks::chart_checks(&chart).unwrap();
    assert!(r["L"].iter().any(|c| !c.pass && c.id == "grade_chest"));
    assert!(r["XL"].iter().any(|c| !c.pass && c.id == "grade_chest"));
}

#[test]
fn spec_exports_share_values_and_defend_literal_user_text() {
    let mut doc = draft(BlockKind::Shirt);
    doc.name = "=1+1,\"Factory\"".into();
    let spec = block_checks::spec_sheet(&doc).unwrap();
    let csv = spec_export::export(&doc, "csv").unwrap();
    let text = String::from_utf8(csv.bytes).unwrap();
    assert!(text.contains("'="));
    assert!(text.contains("\"\"Factory\"\""));
    for row in &spec.rows {
        for v in &row.values {
            assert!(text.contains(&format!("\"{:.2}\"", v.measured_mm)));
        }
    }
    let pdf = spec_export::export(&doc, "pdf").unwrap();
    let text = String::from_utf8_lossy(&pdf.bytes);
    assert!(text.starts_with("%PDF-1.4"));
    assert!(text.contains("/Count 6 /Kids"));
    assert!(text.contains("POINTS OF MEASURE"));
    assert!(text.ends_with("%%EOF\n"));
    let xlsx = spec_export::export(&doc, "xlsx").unwrap();
    assert!(xlsx.bytes.starts_with(b"PK\x03\x04"));
    assert!(spec_export::export(&doc, "exe").is_err());
}

fn set(
    doc: &beta_pattern_core::Document,
    name: &str,
    formula: &str,
) -> beta_pattern_core::Document {
    style::edit(
        doc,
        &[Operation::SetVariable {
            name: name.into(),
            value: Value::Formula(formula.into()),
        }],
    )
    .unwrap()
}
fn failing(doc: &beta_pattern_core::Document, id: &str) -> bool {
    block_checks::report(doc)
        .unwrap()
        .sizes
        .iter()
        .all(|s| s.checks.iter().any(|c| c.id == id && !c.pass))
}

#[test]
fn a_leg_without_crotch_extension_cannot_pass() {
    // The 0.6 trouser had no extension: panels that cannot pass between the legs.
    let doc = draft(BlockKind::Trouser);
    let flat = set(&set(&doc, "front_ext", "0.001"), "back_ext", "0.002");
    assert!(failing(&flat, "crotch_extension_total"));
    assert!(failing(&flat, "thigh_hip"));
    // Same front and back panels: no longer back rise than front.
    let same = set(&set(&doc, "back_ext", "v.front_ext"), "back_raise", "0");
    assert!(failing(&same, "crotch_extension_back_front"));
}

#[test]
fn trouser_leg_edits_break_matching_seams_and_crease() {
    let doc = draft(BlockKind::Trouser);
    // A wider back hem alone changes the back leg seams; front/back no longer match.
    let wide = set(&doc, "back_hem_half", "(v.leg_width+v.hem_delta)/2+15");
    assert!(failing(&wide, "outseam"));
    assert!(failing(&wide, "inseam"));
    // A grain line moved off the crease twists the leg.
    let mut twisted = doc.clone();
    for id in ["front.grain0", "front.grain1"] {
        if let beta_pattern_core::Object::Point {
            operation: beta_pattern_core::PointOp::Absolute { x, .. },
        } = twisted.drawing.get_mut(id).unwrap()
        {
            let old = match x {
                Value::Formula(s) => s.clone(),
                _ => panic!(),
            };
            *x = Value::Formula(format!("({old})+12"));
        }
    }
    assert!(failing(&twisted, "front.crease_offset"));
}

#[test]
fn a_seam_allowance_wider_than_a_concave_curve_is_reported() {
    // Size S has a 21 mm knit underarm radius: an 18 mm allowance nearly folds there.
    // (A 20 mm allowance folds in some size, so the edit itself is rejected.)
    let doc = set(&draft(BlockKind::TShirt), "seam_allowance", "18");
    let r = block_checks::report(&doc).unwrap();
    let small = r.sizes.iter().find(|s| s.size == "S").unwrap();
    assert!(!small.pass);
    assert!(
        small
            .checks
            .iter()
            .any(|c| c.id.contains("concave_radius") && !c.pass && c.value > 1.)
    );
    assert!(
        style::edit(
            &draft(BlockKind::TShirt),
            &[Operation::SetVariable {
                name: "seam_allowance".into(),
                value: 20.0.into(),
            }],
        )
        .is_err()
    );
}

#[test]
fn shirt_yoke_has_depth_and_continues_the_armhole() {
    let r = block_checks::report(&draft(BlockKind::Shirt)).unwrap();
    for s in &r.sizes {
        let depth = s.checks.iter().find(|c| c.id == "yoke_depth_cb").unwrap();
        assert!(depth.pass && depth.value > 60.);
        assert!(s.checks.iter().any(|c| c.id == "armhole_at_yoke" && c.pass));
    }
}
