use crate::bundles::*;
use crate::costing::*;
use crate::recuts::*;
use crate::reports::*;
use crate::tickets::*;
use beta_marker_core::{Fabric, JOB_FORMAT, Job, Order, Piece};
use std::collections::{BTreeMap, BTreeSet};

fn lay() -> Lay {
    Lay {
        id: "L1".into(),
        style: "TEE".into(),
        order: "O1".into(),
        colour: "Blue".into(),
        plies: vec![
            PlyRun {
                first: 1,
                last: 30,
                shade_group: "A".into(),
            },
            PlyRun {
                first: 31,
                last: 60,
                shade_group: "B".into(),
            },
        ],
        parts: [("front", 0), ("back", 0), ("front", 1), ("back", 1)]
            .map(|(p, g)| CutPart {
                piece_id: p.into(),
                size: "M".into(),
                garment: g,
                slot: 0,
                mirrored: false,
            })
            .to_vec(),
    }
}
fn job() -> Job {
    Job {
        format: JOB_FORMAT.into(),
        name: "tee".into(),
        fabric: Fabric {
            width: 100.0,
            ..Default::default()
        },
        pieces: ["front", "back"]
            .map(|id| Piece {
                id: id.into(),
                size: Some("M".into()),
                quantity: 1,
                outline: vec![[0.0, 0.0], [40.0, 0.0], [40.0, 30.0], [0.0, 30.0]],
                ..Default::default()
            })
            .to_vec(),
        ..Default::default()
    }
}
#[test]
fn numbering_keeps_garment_parts_together_and_never_mixes_shades() {
    let bundles = bundle_lay(&lay(), BundleRules::default()).expect("valid lay");
    assert_eq!(bundles.len(), 12);
    let mut numbers = BTreeMap::new();
    let mut keys = BTreeSet::new();
    for (i, b) in bundles.iter().enumerate() {
        assert_eq!(b.number, i as u32 + 1);
        assert_eq!(b.stickers.len(), 20);
        assert_eq!(b.size, "M");
        for s in &b.stickers {
            assert!(keys.insert(s.piece.clone()));
            assert_eq!(s.last_number, 120);
            assert_eq!(s.number, (s.piece.ply - 1) * 2 + s.piece.garment + 1);
            assert_eq!(b.shade_group, if s.piece.ply <= 30 { "A" } else { "B" });
            if let Some(n) = numbers.insert((s.piece.ply, s.piece.garment), s.number) {
                assert_eq!(n, s.number);
            }
        }
    }
    assert_eq!(keys.len(), 240);
    assert_eq!(numbers.len(), 120);
    assert_eq!(bundles[0].stickers[0].label(), "1-120");
}
#[test]
fn size_handedness_partial_bundles_and_offsets() {
    let mut l = lay();
    l.plies = vec![PlyRun {
        first: 1,
        last: 3,
        shade_group: "A".into(),
    }];
    l.parts[2].size = "L".into();
    l.parts[3].size = "L".into();
    l.parts[2].mirrored = true;
    let b = bundle_lay(
        &l,
        BundleRules {
            pieces_per_bundle: 2,
            first_bundle: 40,
            sticker_offset: 60,
        },
    )
    .expect("valid lay");
    assert_eq!(b.len(), 8);
    assert_eq!(b[0].number, 40);
    assert_eq!(b[7].number, 47);
    assert!(b.iter().any(|b| b.stickers.len() == 1));
    for b in &b {
        assert!(
            b.stickers
                .iter()
                .all(|s| s.number >= 61 && s.last_number == 63)
        );
    }
}
#[test]
fn invalid_lay_and_overflow_fail_cleanly() {
    let mut l = lay();
    l.plies[1].first = 32;
    assert!(bundle_lay(&l, BundleRules::default()).is_err());
    let mut l = lay();
    l.parts.push(l.parts[0].clone());
    assert!(bundle_lay(&l, BundleRules::default()).is_err());
    let mut l = lay();
    l.parts[2].size = "L".into();
    assert!(bundle_lay(&l, BundleRules::default()).is_err());
    assert!(
        bundle_lay(
            &lay(),
            BundleRules {
                pieces_per_bundle: 0,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        bundle_lay(
            &lay(),
            BundleRules {
                sticker_offset: u32::MAX,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        bundle_lay(
            &lay(),
            BundleRules {
                first_bundle: u32::MAX,
                ..Default::default()
            }
        )
        .is_err()
    );
}
#[test]
fn scan_roundtrip_preserves_unicode_and_rejects_bad_schema() {
    let mut l = lay();
    l.colour = "\u{928}\u{940}\u{932}\u{93e}".into();
    l.order = "a\"b\\c".into();
    let b = bundle_lay(&l, BundleRules::default()).expect("bundles");
    let p = ScanPayload::bundle(&b[0]).expect("payload");
    let encoded = p.encode().expect("encode");
    assert_eq!(ScanPayload::decode(&encoded).expect("decode"), p);
    let mut p = p;
    p.v = 2;
    assert!(p.encode().is_err());
    assert!(ScanPayload::decode("{}").is_err());
    assert!(ScanPayload::decode(&encoded.replace("\"q\":20", "\"q\":0")).is_err());
}
#[test]
fn pdf_pages_dimensions_and_escaping() {
    let mut l = lay();
    l.style = "S(1)\\style".into();
    let b = bundle_lay(&l, BundleRules::default()).expect("bundles");
    let pdf = ticket_pdf(&b, TicketPaper::A4).expect("A4 PDF");
    let text = String::from_utf8_lossy(&pdf);
    assert!(text.starts_with("%PDF-1.4"));
    assert!(text.contains("/Count 2"));
    assert!(text.contains("/Filter /FlateDecode"));
    let start = pdf
        .windows(7)
        .position(|w| w == b"stream\n")
        .expect("stream")
        + 7;
    let prefix = std::str::from_utf8(&pdf[15..start]).expect("ASCII object headers");
    let length: usize = prefix
        .rsplit("/Length ")
        .next()
        .expect("length")
        .split_whitespace()
        .next()
        .expect("length value")
        .parse()
        .expect("stream length");
    let mut decoded = String::new();
    std::io::Read::read_to_string(
        &mut flate2::read::ZlibDecoder::new(&pdf[start..start + length]),
        &mut decoded,
    )
    .expect("inflate PDF page");
    assert!(decoded.contains("S\\(1\\)\\\\style"));
    assert!(
        length * 3 < decoded.len(),
        "QR page streams should compress well"
    );
    assert!(text.ends_with("%%EOF\n"));
    let pdf = sticker_pdf(&b[..1], TicketPaper::Label100x50).expect("sticker PDF");
    let text = String::from_utf8_lossy(&pdf);
    assert!(text.contains("/Count 20"));
    assert!(text.contains("283.46456692913387"));
    assert!(ticket_pdf(&[], TicketPaper::A4).is_err());
}
#[test]
fn recuts_preserve_exact_demands_and_split_shades() {
    let bundles = bundle_lay(&lay(), BundleRules::default()).expect("bundles");
    let a = &bundles[0];
    let b = bundles
        .iter()
        .find(|b| b.shade_group == "B")
        .expect("shade B");
    let mut ledger = vec![
        Damage::record("D1".into(), a, a.stickers[0].piece.clone(), "hole".into()).expect("damage"),
        Damage::record("D2".into(), b, b.stickers[0].piece.clone(), "knife".into())
            .expect("damage"),
    ];
    let jobs = schedule_recuts(&job(), &bundles, &mut ledger, "R1").expect("schedule");
    assert_eq!(jobs.len(), 2);
    assert_ne!(jobs[0].shade_group, jobs[1].shade_group);
    for r in &jobs {
        assert!(beta_marker_core::check_job(&r.job).is_empty());
        assert_eq!(
            beta_marker_core::demands(&r.job)
                .iter()
                .map(|d| d.count)
                .sum::<u32>(),
            1
        );
        assert_eq!(r.job.meta["shade_group"], r.shade_group);
    }
    assert_eq!(ledger[0].status, RecutStatus::Scheduled);
    assert!(ledger[0].advance(RecutStatus::Replaced).is_err());
    ledger[0].advance(RecutStatus::Cut).expect("cut");
    ledger[0].advance(RecutStatus::Replaced).expect("return");
    assert!(
        schedule_recuts(&job(), &bundles, &mut ledger, "R2")
            .expect("empty")
            .is_empty()
    );
}
#[test]
fn recut_scheduling_is_atomic_and_rejects_duplicates() {
    let b = bundle_lay(&lay(), BundleRules::default()).expect("bundle");
    let d = Damage::record(
        "D1".into(),
        &b[0],
        b[0].stickers[0].piece.clone(),
        "hole".into(),
    )
    .expect("damage");
    let mut ledger = vec![
        d.clone(),
        Damage {
            id: "D2".into(),
            ..d.clone()
        },
    ];
    assert!(schedule_recuts(&job(), &b, &mut ledger, "R1").is_err());
    assert_eq!(ledger[0], d);
    let mut ledger = vec![Damage {
        size: "XL".into(),
        ..d.clone()
    }];
    assert!(schedule_recuts(&job(), &b, &mut ledger, "R1").is_err());
    assert_eq!(ledger[0].status, RecutStatus::Open);
    assert!(
        Damage::record(
            "D2".into(),
            &b[0],
            PieceKey {
                ply: 999,
                ..d.piece
            },
            "hole".into()
        )
        .is_err()
    );
}
fn actuals() -> LayActuals {
    LayActuals {
        lay: "L1".into(),
        order: "O1".into(),
        width_mm: 1000.0,
        marker_length_mm: 2000.0,
        plies: 10,
        end_allowance_per_ply_mm: 20.0,
        piece_area_per_ply_mm2: 1_500_000.0,
        pieces_per_ply: 4,
        actual_piece_area_mm2: 15_000_000.0,
        actual_pieces: 40,
        recut_count: 2,
        spreading: vec![SpreadingRecord {
            issued_mm: 21_000.0,
            end_bits_mm: 500.0,
        }],
    }
}
#[test]
fn report_reconciles_allowances_end_bits_and_efficiency() {
    let r = lay_report(&actuals()).expect("report");
    assert_eq!(r.planned_mm, 20200.0);
    assert_eq!(r.actual_mm, 20500.0);
    assert_eq!(r.variance_mm, 300.0);
    assert_eq!(r.marker_efficiency_pct, 75.0);
    assert!((r.cutting_efficiency_pct.expect("actual") - 73.1707317073).abs() < 1e-8);
    assert_eq!(r.planned_pieces, 40);
    assert_eq!(r.recut_count, 2);
    assert!(report_text(&r).contains("20.200 m"));
    let mut r = r;
    r.id = "L,\"1".into();
    assert!(reports_csv(&[r]).contains("\"L,\"\"1\""));
}
#[test]
fn order_efficiency_is_area_weighted() {
    let a = actuals();
    let mut b = a.clone();
    b.lay = "L2".into();
    b.width_mm = 2000.0;
    let r = order_report(&[a.clone(), b]).expect("order");
    assert_eq!(r.marker_efficiency_pct, 50.0);
    assert!(
        (r.cutting_efficiency_pct.expect("actual") - 100.0 * 30_000_000.0 / 61_500_000.0).abs()
            < 1e-8
    );
    assert!(order_report(&[a.clone(), a]).is_err());
}
#[test]
fn reports_reject_invalid_numbers_and_handle_unstarted_lays() {
    let mut a = actuals();
    a.spreading[0].end_bits_mm = 22000.0;
    assert!(lay_report(&a).is_err());
    let mut a = actuals();
    a.width_mm = f64::INFINITY;
    assert!(lay_report(&a).is_err());
    let mut a = actuals();
    a.spreading.clear();
    a.actual_pieces = 0;
    a.actual_piece_area_mm2 = 0.0;
    assert_eq!(
        lay_report(&a).expect("unstarted").cutting_efficiency_pct,
        None
    );
}
#[test]
fn estimator_respects_area_ratios_and_shrinkage() {
    let mut j = job();
    j.order = Some(Order {
        sizes: BTreeMap::from([("M".into(), 3)]),
    });
    let model = EfficiencyModel {
        coefficients: [0.8, 0.0, 0.0, 0.0, 0.0, 0.0],
        training_samples: 8,
        provenance: "test".into(),
    };
    let e = estimate(&j, 100.0, 3, &model).expect("estimate");
    assert_eq!(e.total_piece_area_mm2, 7200.0);
    assert!((e.fabric_per_garment_mm - 30.0).abs() < 1e-10);
    assert_eq!(e.features.convexity_ratio, 1.0);
    assert_eq!(e.features.largest_width_ratio, 0.3);
    assert!(estimate(&j, 100.0, 2, &model).is_err());
    j.fabric.shrinkage.length_pct = 10.0;
    assert!(
        (estimate(&j, 100.0, 3, &model)
            .expect("shrinkage")
            .total_piece_area_mm2
            - 7920.0)
            .abs()
            < 1e-8
    );
}
#[test]
fn estimator_uses_convex_hull_and_guards_bad_inputs() {
    let mut j = job();
    j.pieces[0].outline = vec![
        [0.0, 0.0],
        [40.0, 0.0],
        [40.0, 10.0],
        [10.0, 10.0],
        [10.0, 30.0],
        [0.0, 30.0],
    ];
    let (f, _) = features(&j, 100.0).expect("concave");
    assert!(f.convexity_ratio < 1.0);
    assert!(features(&j, f64::NAN).is_err());
    assert!(features(&j, 1.0).is_err());
    assert!(estimate(&j, 100.0, 0, &EfficiencyModel::default()).is_err());
    j.order = Some(Order {
        sizes: BTreeMap::from([("M".into(), u32::MAX)]),
    });
    j.pieces[0].quantity = 2;
    assert!(features(&j, 100.0).is_err());
}
fn observation(i: usize) -> Observation {
    let t = i as f64 / 12.0;
    let features = Features {
        piece_count: t,
        mean_piece_area: t * t,
        largest_width_ratio: 0.3,
        convexity_ratio: 0.9,
        rotation_freedom: 0.25,
    };
    let efficiency = 0.6 + 0.1 * t;
    Observation {
        source: format!("synthetic-{i}"),
        features,
        efficiency,
        length_mm: 100.0 / efficiency,
        width_mm: 100.0,
        garments: 1,
    }
}
#[test]
fn fit_and_mape_use_consumption_error_not_efficiency_error() {
    let observations = (0..12).map(observation).collect::<Vec<_>>();
    let m = fit(&observations, 0.00001).expect("fit");
    assert!(consumption_mape(&m, &[observation(12)]).expect("holdout") < 0.01);
    let m = EfficiencyModel {
        coefficients: [0.5, 0.0, 0.0, 0.0, 0.0, 0.0],
        ..Default::default()
    };
    assert!((consumption_mape(&m, &[observation(0)]).expect("mape") - 20.0).abs() < 1e-10);
    assert!(fit(&[], 0.1).is_err());
    assert!(consumption_mape(&m, &[]).is_err());
}
#[test]
fn accurate_mode_returns_a_checked_real_nest() {
    let j = job();
    let a = accurate_estimate(&j, 100.0, 1, 0.05, 42).expect("nest");
    assert!(beta_marker_core::validate(&j, &a.marker).ok);
    assert!(a.fabric_per_garment_mm > 0.0);
    assert!(accurate_estimate(&j, 100.0, 1, 21.0, 42).is_err());
}

#[test]
fn recut_mirror_count_is_not_multiplied_by_original_order() {
    let mut l = lay();
    l.parts[0].mirrored = true;
    let b = bundle_lay(&l, BundleRules::default()).expect("bundles");
    let bundle = b
        .iter()
        .find(|b| b.stickers[0].piece.mirrored)
        .expect("mirrored bundle");
    let mut ledger = vec![
        Damage::record(
            "D1".into(),
            bundle,
            bundle.stickers[0].piece.clone(),
            "hole".into(),
        )
        .expect("damage"),
    ];
    let mut source = job();
    source.order = Some(Order {
        sizes: BTreeMap::from([("M".into(), 10)]),
    });
    let r = schedule_recuts(&source, &b, &mut ledger, "R1").expect("recut");
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].job.order, None);
    assert_eq!(r[0].job.pieces.len(), 1);
    assert_eq!(r[0].job.pieces[0].quantity, 0);
    assert_eq!(r[0].job.pieces[0].mirror_quantity, 1);
    assert!(beta_marker_core::check_job(&r[0].job).is_empty());
}

#[test]
fn scan_and_pdf_reject_inconsistent_physical_references() {
    let mut b = bundle_lay(&lay(), BundleRules::default()).expect("bundles");
    b[0].stickers[0].piece.ply = 999;
    assert!(ScanPayload::bundle(&b[0]).is_err());
    assert!(sticker_pdf(&b[..1], TicketPaper::A4).is_err());
    let mut b = bundle_lay(&lay(), BundleRules::default()).expect("bundles");
    b[0].style.clear();
    assert!(ScanPayload::bundle(&b[0]).is_err());
}

#[test]
fn recuts_distinguish_orders_with_the_same_lay_and_bundle_numbers() {
    let mut a = bundle_lay(&lay(), BundleRules::default())
        .expect("bundle")
        .remove(0);
    let mut b = a.clone();
    a.order = "order-a".into();
    b.order = "order-b".into();
    let mut ledger = vec![
        Damage::record("D1".into(), &a, a.stickers[0].piece.clone(), "hole".into())
            .expect("damage"),
        Damage::record("D2".into(), &b, b.stickers[0].piece.clone(), "hole".into())
            .expect("damage"),
    ];
    let jobs = schedule_recuts(&job(), &[a, b], &mut ledger, "R1").expect("separate orders");
    assert_eq!(jobs.len(), 2);
    assert_ne!(jobs[0].job.meta["order"], jobs[1].job.meta["order"]);
}

#[test]
fn floor_bundles_share_canonical_garment_size_order() -> anyhow::Result<()> {
    let mut lay = lay();
    lay.parts = ["XL", "S", "M"]
        .into_iter()
        .enumerate()
        .map(|(garment, size)| CutPart {
            piece_id: "front".into(),
            size: size.into(),
            garment: garment as u32,
            slot: 0,
            mirrored: false,
        })
        .collect();
    lay.plies = vec![PlyRun {
        first: 1,
        last: 1,
        shade_group: "A".into(),
    }];
    let bundles = bundle_lay(&lay, BundleRules::default())?;
    assert_eq!(
        bundles.iter().map(|b| b.size.as_str()).collect::<Vec<_>>(),
        ["S", "M", "XL"]
    );
    Ok(())
}

#[test]
fn accurate_cost_applies_matching_rules() -> anyhow::Result<()> {
    let mut j = job();
    j.fabric.width = 100.0;
    j.fabric.repeat = Some(beta_marker_core::Repeat {
        period_x: Some(20.0),
        tolerance_mm: 0.1,
        ..Default::default()
    });
    j.pieces[0].match_points.push(beta_marker_core::MatchPoint {
        id: "anchor".into(),
        point: [0.0, 0.0],
    });
    j.match_rules.push(beta_marker_core::MatchRule {
        a: beta_marker_core::PointRef {
            piece_id: "front".into(),
            point_id: "anchor".into(),
        },
        b: None,
        axes: beta_marker_core::MatchAxes::X,
        offset: [0.0, 0.0],
    });
    let estimate = accurate_estimate(&j, 100.0, 1, 0.02, 42)?;
    assert!(beta_marker_core::validate(&j, &estimate.marker).ok);
    Ok(())
}
