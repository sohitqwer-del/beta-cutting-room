use crate::{floor::*, intake::*, sheets, tickets::ScanPayload};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../packages/marker-wasm/test/fixtures/cutting-room.json"
    ))
    .expect("original shared fixture")
}
fn state() -> FloorState {
    let mut s = FloorState {
        register: serde_json::from_value(fixture()["register"].clone()).expect("rolls"),
        ..Default::default()
    };
    s.register.rolls[0].defects.clear();
    s.register.rolls[0].usable_width_mm = 100.0;
    s
}
fn prepare() -> Prepare {
    let f = fixture();
    Prepare {
        id: "LAY-01".into(),
        order: "PO1".into(),
        colour: "Blue".into(),
        date: "2026-10-19".into(),
        table: "T1".into(),
        job: serde_json::from_value(f["job"].clone()).expect("job"),
        marker: serde_json::from_value(f["result"].clone()).expect("marker"),
        plies: 2,
        end_allowance_mm: 10.0,
        roll_ids: vec!["R1".into()],
        splice_zones: vec![],
        splice_overlap_m: 0.1,
        pieces_per_bundle: 20,
    }
}
fn actuals() -> Actuals {
    Actuals {
        plies: 2,
        rolls: vec![ActualRoll {
            roll_id: "R1".into(),
            issued_m: 0.1,
            returned_m: 0.018,
        }],
        note: "Measured".into(),
    }
}
fn cut_state() -> FloorState {
    let mut s = state();
    s.prepare(prepare()).expect("prepare");
    s.advance(
        "LAY-01",
        LayStatus::Spread,
        Some(actuals()),
        "2026-10-19",
        "Asha",
        "",
    )
    .expect("spread");
    s.advance("LAY-01", LayStatus::Cut, None, "2026-10-19", "Asha", "")
        .expect("cut");
    s
}
fn csv_orders() -> Vec<u8> {
    template(Kind::Orders, "csv").expect("template")
}

#[test]
fn csv_and_xlsx_templates_roundtrip() {
    for kind in [Kind::Orders, Kind::Rolls] {
        for format in ["csv", "xlsx"] {
            let bytes = template(kind, format).expect("template");
            let p = preview(&bytes, format, kind, &Mapping::new()).expect("preview");
            assert!(p.errors.is_empty(), "{:?}", p.errors);
            assert_eq!(p.rows.len(), 1);
        }
    }
}
#[test]
fn importer_preserves_unicode_quotes_and_colour_separation() {
    let bytes = "schema_version,external_id,purchase_order,buyer,style,colour,size,quantity,due_date\r\n1,01,PO1,\"Buyer, एक\",TEE,Blue,M,20,2024-02-29\r\n1,02,PO1,Buyer,TEE,Red,M,7,\r\n";
    let p = preview(bytes.as_bytes(), "csv", Kind::Orders, &Mapping::new()).expect("preview");
    assert!(p.errors.is_empty());
    let mut s = state();
    assert_eq!(s.import(&p).expect("import"), 2);
    let q = s
        .quantities("PO1", "TEE", "Blue", &prepare().job)
        .expect("q");
    assert_eq!(q["M"], 20);
    assert_eq!(s.orders[0].external_id, "01");
    assert_eq!(s.orders[0].buyer, "Buyer, एक");
}
#[test]
fn preview_retains_row_errors_and_blocks_commit() {
    let text = String::from_utf8(csv_orders())
        .expect("utf8")
        .replace(",20,", ",-1,");
    let p = preview(text.as_bytes(), "csv", Kind::Orders, &Mapping::new()).expect("preview");
    assert_eq!(p.errors[0].row, 2);
    assert!(p.errors[0].message.contains("quantity"));
    let mut s = state();
    assert!(s.import(&p).is_err());
    assert!(s.orders.is_empty());
}
#[test]
fn external_id_replay_is_idempotent_and_conflicts_are_atomic() {
    let mut s = state();
    let p = preview(&csv_orders(), "csv", Kind::Orders, &Mapping::new()).expect("p");
    assert_eq!(s.import(&p).expect("first"), 1);
    assert_eq!(s.import(&p).expect("repeat"), 0);
    let before = serde_json::to_value(&s).expect("json");
    let mut changed = p.clone();
    let mut new = changed.orders[0].clone();
    new.external_id = "NEW".into();
    changed.orders.insert(0, new);
    changed.orders[1].quantity = 30;
    assert!(s.import(&changed).is_err());
    assert_eq!(serde_json::to_value(&s).expect("json"), before);
}
#[test]
fn column_mapping_rejects_ambiguous_headers() {
    let text = String::from_utf8(csv_orders())
        .expect("utf8")
        .replace("quantity", "Garments");
    let p = preview(text.as_bytes(), "csv", Kind::Orders, &Mapping::new()).expect("preview");
    assert!(p.errors.iter().any(|e| e.row == 1));
    let map = Mapping::from([("quantity".into(), "Garments".into())]);
    assert!(
        preview(text.as_bytes(), "csv", Kind::Orders, &map)
            .expect("mapped")
            .errors
            .is_empty()
    );
    let bad = text.replace("size,Garments", "size,size");
    assert!(
        !preview(bad.as_bytes(), "csv", Kind::Orders, &map)
            .expect("bad headers")
            .errors
            .is_empty()
    );
}
#[test]
fn nonfinite_rolls_duplicate_ids_versions_and_dates_fail() {
    let t = String::from_utf8(template(Kind::Rolls, "csv").expect("csv")).expect("utf8");
    assert!(
        !preview(
            t.replace(",100,100,", ",100,NaN,").as_bytes(),
            "csv",
            Kind::Rolls,
            &Mapping::new()
        )
        .expect("preview")
        .errors
        .is_empty()
    );
    assert!(date("2023-02-29").is_err());
    assert!(date("2024-02-29").is_ok());
    assert!(date("2026-13-01").is_err());
    let text = String::from_utf8(csv_orders()).expect("utf8");
    let line = text.lines().nth(1).expect("row");
    let duplicate = format!("{text}{line}\r\n");
    assert!(
        !preview(duplicate.as_bytes(), "csv", Kind::Orders, &Mapping::new())
            .expect("preview")
            .errors
            .is_empty()
    );
    assert!(
        !preview(
            text.replacen("\r\n1,", "\r\n2,", 1).as_bytes(),
            "csv",
            Kind::Orders,
            &Mapping::new()
        )
        .expect("preview")
        .errors
        .is_empty()
    );
}
#[test]
fn xlsx_rejects_formulas_and_wrong_sheet() {
    let mut w = rust_xlsxwriter::Workbook::new();
    let s = w.add_worksheet();
    s.set_name("Orders").expect("name");
    for (i, h) in Kind::Orders.columns().iter().enumerate() {
        s.write_string(0, i as u16, *h).expect("header");
    }
    s.write_formula(1, 7, "=1+1").expect("formula");
    assert!(
        preview(
            &w.save_to_buffer().expect("xlsx"),
            "xlsx",
            Kind::Orders,
            &Mapping::new()
        )
        .is_err()
    );
    assert!(
        preview(
            &template(Kind::Rolls, "xlsx").expect("xlsx"),
            "xlsx",
            Kind::Orders,
            &Mapping::new()
        )
        .is_err()
    );
}
#[test]
fn bounded_file_and_csv_injection_defence() {
    assert!(preview(&vec![b'a'; 5_000_001], "csv", Kind::Orders, &Mapping::new()).is_err());
    assert_eq!(csv_text("=SUM(A1)"), "'=SUM(A1)");
    assert_eq!(csv_text("  @X"), "'  @X");
    let csv =
        String::from_utf8(csv_table(&["text"], &[vec!["=HYPERLINK(\"x\")".into()]]).expect("csv"))
            .expect("utf8");
    assert!(csv.contains("'=HYPERLINK"));
}
#[test]
fn spread_accounts_for_each_end_and_stickers() {
    let mut s = state();
    s.prepare(prepare()).expect("lay");
    s.validate().expect("valid");
    let l = &s.lays[0];
    assert!((l.spread.total_used_m - 0.082).abs() < 1e-9);
    let m = sheets::machine(l).expect("machine");
    assert_eq!(m.rows.len(), 2);
    assert_eq!(m.rows[1].ply, 2);
    assert_eq!(m.rows[0].head_allowance_mm, 10.0);
    assert!((m.rows[1].roll_start_m - 0.041).abs() < 1e-9);
    let stickers = String::from_utf8(sheets::stickers_csv(l).expect("stickers")).expect("utf8");
    assert!(!stickers.contains("first_ply"));
    assert_eq!(stickers.lines().count(), 3);
    assert!(
        sheets::lay_html(l)
            .expect("html")
            .starts_with(b"<!doctype html>")
    );
}
#[test]
fn reserved_roll_cannot_be_allocated_twice() {
    let mut s = state();
    s.prepare(prepare()).expect("lay");
    let mut second = prepare();
    second.id = "LAY-02".into();
    assert!(s.prepare(second).is_err());
    assert_eq!(s.lays.len(), 1);
}
#[test]
fn invalid_marker_and_short_stock_never_release() {
    let mut s = state();
    let mut p = prepare();
    p.marker.placements[0].translate = [-50.0, 0.0];
    assert!(s.prepare(p).is_err());
    s.register.rolls[0].measured_length_m = 0.01;
    assert!(s.prepare(prepare()).is_err());
    assert!(s.lays.is_empty());
}
#[test]
fn shrinkage_and_defects_block_release() {
    let mut s = state();
    s.register.rolls[0].shrinkage.length_pct = 3.0;
    assert!(s.prepare(prepare()).is_err());
    s = state();
    s.register.rolls[0].defects =
        serde_json::from_value(fixture()["register"]["rolls"][0]["defects"].clone())
            .expect("defect");
    s.register.rolls[0].defects[0].along_m = 0.01;
    assert!(s.prepare(prepare()).is_err());
}
#[test]
fn splice_zone_cannot_cross_a_piece() {
    let mut s = state();
    let mut p = prepare();
    p.splice_zones = vec![beta_marker_roll::SpliceZone {
        start_m: 0.010,
        end_m: 0.030,
    }];
    assert!(s.prepare(p).is_err());
}
#[test]
fn safe_splice_is_reported_in_real_execution_order() {
    let mut s = state();
    s.register.rolls[0].measured_length_m = 0.035;
    let mut r = s.register.rolls[0].clone();
    r.id = "R2".into();
    r.measured_length_m = 0.006;
    s.register.rolls.push(r);
    let mut p = prepare();
    p.plies = 1;
    p.roll_ids.push("R2".into());
    p.splice_overlap_m = 0.0;
    p.splice_zones = vec![beta_marker_roll::SpliceZone {
        start_m: 0.031,
        end_m: 0.040,
    }];
    s.prepare(p).expect("safe splice");
    assert_eq!(s.lays[0].spread.lays[0].splices.len(), 1);
    let m = sheets::machine(&s.lays[0]).expect("machine");
    assert_eq!(m.rows.len(), 2);
    assert!((m.rows[1].table_start_m - 0.035).abs() < 1e-8);
}
#[test]
fn forward_transitions_and_actuals_cannot_skip_or_fabricate() {
    let mut s = state();
    s.prepare(prepare()).expect("prepare");
    assert!(
        s.advance("LAY-01", LayStatus::Cut, None, "2026-10-19", "Asha", "")
            .is_err()
    );
    let mut a = actuals();
    a.plies = 3;
    assert!(
        s.advance(
            "LAY-01",
            LayStatus::Spread,
            Some(a),
            "2026-10-19",
            "Asha",
            ""
        )
        .is_err()
    );
    let mut a = actuals();
    a.rolls[0].returned_m = 1.0;
    assert!(
        s.advance(
            "LAY-01",
            LayStatus::Spread,
            Some(a),
            "2026-10-19",
            "Asha",
            ""
        )
        .is_err()
    );
    assert_eq!(s.lays[0].status, LayStatus::Planned);
    s.advance(
        "LAY-01",
        LayStatus::Spread,
        Some(actuals()),
        "2026-10-19",
        "Asha",
        "",
    )
    .expect("spread");
    assert!(
        s.advance(
            "LAY-01",
            LayStatus::Cut,
            Some(actuals()),
            "2026-10-19",
            "Asha",
            ""
        )
        .is_err()
    );
}
#[test]
fn dashboard_excludes_unmeasured_lays_from_variance() {
    let mut s = cut_state();
    let mut r = s.register.rolls[0].clone();
    r.id = "R2".into();
    s.register.rolls.push(r);
    let mut p = prepare();
    p.id = "LAY-02".into();
    p.roll_ids = vec!["R2".into()];
    s.prepare(p).expect("second");
    let d = s.dashboard("2026-10-19").expect("dash");
    assert_eq!(d.lays, 2);
    assert_eq!(d.measured_lays, 1);
    assert!((d.planned_m - 0.164).abs() < 1e-8);
    assert!((d.variance_m).abs() < 1e-8);
    assert!((d.marker_efficiency_pct.expect("eff") - 19.047619).abs() < 1e-5);
    assert!((d.cutting_efficiency_pct.expect("eff") - 9.75609756).abs() < 1e-6);
}
#[test]
fn sewing_requires_bundled_lay_and_exact_registered_payload() {
    let mut s = cut_state();
    let p = ScanPayload::bundle(&s.lays[0].bundles[0]).expect("payload");
    let qr = p.encode().expect("qr");
    assert!(s.issue(&qr, "I1", "2026-10-19", "LINE1", "Asha").is_err());
    s.advance("LAY-01", LayStatus::Bundled, None, "2026-10-19", "Asha", "")
        .expect("bundle");
    let mut fake = p;
    fake.q = 99;
    assert!(
        s.issue(
            &fake.encode().expect("fake"),
            "I1",
            "2026-10-19",
            "LINE1",
            "Asha"
        )
        .is_err()
    );
    assert!(
        s.issue(&qr, "I1", "2026-10-19", "LINE1", "Asha")
            .expect("issue")
    );
    assert!(
        !s.issue(&qr, "I2", "2026-10-19", "LINE1", "Asha")
            .expect("repeat")
    );
    assert!(s.issue(&qr, "I3", "2026-10-19", "LINE2", "Asha").is_err());
    s.validate().expect("saved");
    let csv = String::from_utf8(sheets::wip_csv(&s).expect("csv")).expect("utf8");
    assert_eq!(csv.lines().count(), 2);
    assert!(csv.contains("issued_parts"));
    assert_eq!(
        s.dashboard("2026-10-19").expect("dashboard").issued_parts,
        2
    );
}
#[test]
fn damage_recuts_keep_identity_and_only_damaged_quantity() {
    let mut s = cut_state();
    let b = &s.lays[0].bundles[0];
    let key = b.stickers[0].piece.clone();
    let number = b.number;
    s.damage("LAY-01", "D1", number, key.clone(), "Knife")
        .expect("damage");
    assert!(s.damage("LAY-01", "D2", number, key, "Knife").is_err());
    s.schedule("LAY-01", "RC1").expect("schedule");
    assert_eq!(s.lays[0].recuts[0].job.pieces[0].quantity, 1);
    assert_eq!(s.lays[0].recuts[0].shade_group, "[\"LOT1\",\"DYE1\",\"A\"]");
    assert_eq!(
        s.lays[0].damage[0].status,
        crate::recuts::RecutStatus::Scheduled
    );
    s.lays[0].damage[0]
        .advance(crate::recuts::RecutStatus::Cut)
        .expect("cut");
    s.lays[0].damage[0]
        .advance(crate::recuts::RecutStatus::Replaced)
        .expect("replace");
    s.validate().expect("valid");
    assert_eq!(s.dashboard("2026-10-19").expect("dash").open_recuts, 0);
}
#[test]
fn backup_rejects_forged_bundle_and_actuals() {
    let mut s = cut_state();
    s.lays[0].bundles[0].stickers[0].number = 2;
    assert!(s.validate().is_err());
    let mut s = cut_state();
    s.lays[0].actuals.as_mut().expect("actual").plies = 1;
    assert!(s.validate().is_err());
    let mut s = cut_state();
    s.lays[0].marker.piece_area = 9999.0;
    assert!(s.validate().is_err());
}
#[test]
fn printed_identifiers_are_html_escaped() {
    let mut s = state();
    let mut p = prepare();
    p.id = "<script>alert(1)</script>".into();
    s.prepare(p).expect("prepare");
    let html = String::from_utf8(sheets::lay_html(&s.lays[0]).expect("html")).expect("utf8");
    assert!(!html.contains("<script>"));
    assert!(html.contains("&lt;script&gt;"));
}
#[test]
fn json_api_complete_import_to_sewing_and_exports() {
    let mut s = state();
    fn call(s: &mut FloorState, op: &str, data: Value) -> Value {
        let raw =
            crate::dispatch(&json!({"op":op,"state":s,"data":data}).to_string()).expect("API");
        let v: Value = serde_json::from_str(&raw).expect("reply");
        *s = serde_json::from_value(v["state"].clone()).expect("state");
        v
    }
    call(
        &mut s,
        "import",
        json!({"kind":"orders","format":"csv","bytes":csv_orders()}),
    );
    call(
        &mut s,
        "prepare",
        serde_json::to_value(prepare()).expect("prepare"),
    );
    for status in ["spread", "cut", "bundled"] {
        call(
            &mut s,
            "advance",
            json!({"id":"LAY-01","status":status,"actuals":if status=="spread"{json!(actuals())}else{Value::Null},"date":"2026-10-19","operator":"Asha"}),
        );
    }
    let qr = ScanPayload::bundle(&s.lays[0].bundles[0])
        .expect("payload")
        .encode()
        .expect("encode");
    call(
        &mut s,
        "issue",
        json!({"payload":qr,"event_id":"I1","date":"2026-10-19","line":"LINE1","operator":"Asha"}),
    );
    for kind in [
        "lay_sheet",
        "machine_csv",
        "machine_json",
        "sticker_order",
        "tickets",
        "stickers",
        "wip",
        "consumption",
        "backup",
    ] {
        let v = call(&mut s, "export", json!({"kind":kind,"lay":"LAY-01"}));
        assert!(!v["download"]["bytes"].as_array().expect("bytes").is_empty());
    }
    assert_eq!(s.issues.len(), 1);
}

#[test]
fn shorter_actual_lay_regenerates_bundles_and_invalidates_old_machine_release() {
    let mut s = state();
    s.prepare(prepare()).expect("prepare");
    let old = ScanPayload::bundle(&s.lays[0].bundles[0]).expect("old");
    let mut a = actuals();
    a.plies = 1;
    a.note = "Roll ended; stop after one complete ply".into();
    s.advance(
        "LAY-01",
        LayStatus::Spread,
        Some(a),
        "2026-10-19",
        "Asha",
        "",
    )
    .expect("short lay");
    s.advance("LAY-01", LayStatus::Cut, None, "2026-10-19", "Asha", "")
        .expect("cut");
    s.advance("LAY-01", LayStatus::Bundled, None, "2026-10-19", "Asha", "")
        .expect("bundled");
    let current = ScanPayload::bundle(&s.lays[0].bundles[0]).expect("current");
    assert_eq!(current.q, 1);
    assert!(
        s.issue(
            &old.encode().expect("qr"),
            "I1",
            "2026-10-19",
            "LINE1",
            "Asha"
        )
        .is_err()
    );
    assert!(
        s.issue(
            &current.encode().expect("qr"),
            "I1",
            "2026-10-19",
            "LINE1",
            "Asha"
        )
        .expect("issue")
    );
    assert!(sheets::machine(&s.lays[0]).is_err());
    assert!(sheets::stickers_csv(&s.lays[0]).is_ok());
    s.validate().expect("saved");
}

#[test]
fn folded_area_and_physical_handedness_expand_once() {
    let mut p = prepare();
    p.job.fabric.layout = beta_marker_core::Layout::Folded;
    p.job.pieces[0].mirror_quantity = 1;
    let mut s = state();
    s.prepare(p).expect("folded");
    assert_eq!(s.lays[0].bundles.len(), 2);
    assert_eq!(
        s.lays[0]
            .bundles
            .iter()
            .map(|b| b.stickers.len())
            .sum::<usize>(),
        4
    );
    s.advance(
        "LAY-01",
        LayStatus::Spread,
        Some(actuals()),
        "2026-10-19",
        "Asha",
        "",
    )
    .expect("spread");
    let d = s.dashboard("2026-10-19").expect("dash");
    assert!((d.cutting_efficiency_pct.expect("yield") - 9.75609756).abs() < 1e-6);
}

#[test]
fn negative_variance_remains_numeric_while_identifier_formulas_are_escaped() {
    let mut s = state();
    let mut p = prepare();
    p.order = "=PO1".into();
    s.prepare(p).expect("prepare");
    let mut a = actuals();
    a.rolls[0].issued_m = 0.08;
    a.rolls[0].returned_m = 0.01;
    s.advance(
        "LAY-01",
        LayStatus::Spread,
        Some(a),
        "2026-10-19",
        "Asha",
        "",
    )
    .expect("actuals");
    let csv = sheets::consumption_csv(&s).expect("csv");
    let mut reader = csv::Reader::from_reader(csv.as_slice());
    let row = reader.records().next().expect("row").expect("record");
    assert_eq!(&row[1], "'=PO1");
    assert!((row[8].parse::<f64>().expect("numeric variance") + 0.012).abs() < 1e-8);
}
