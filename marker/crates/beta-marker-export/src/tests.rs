use super::*;
use beta_marker_core::*;
use std::collections::BTreeMap;

fn fixture() -> (Job, MarkerResult) {
    let p = Piece {
        id: "square".into(),
        name: "Front <&> (test)".into(),
        style: None,
        size: Some("M".into()),
        material: None,
        outline: vec![[0., 0.], [100., 0.], [100., 100.], [0., 100.]],
        grain: None,
        quantity: 1,
        mirror_quantity: 0,
        rotation: RotationRule::Flip180,
        buffer: 0.,
        marks: vec![
            Mark {
                kind: MarkKind::Notch,
                points: vec![[50., 0.], [50., 1.]],
                text: None,
            },
            Mark {
                kind: MarkKind::Drill,
                points: vec![[50., 50.]],
                text: None,
            },
            Mark {
                kind: MarkKind::Line,
                points: vec![[10., 10.], [20., 20.]],
                text: None,
            },
            Mark {
                kind: MarkKind::Text,
                points: vec![[20., 30.]],
                text: Some("Fold; do not cut\u{3}".into()),
            },
        ],
        ..Default::default()
    };
    let j = Job {
        match_rules: Vec::new(),
        format: JOB_FORMAT.into(),
        name: "Calibration & test".into(),
        fabric: Fabric {
            width: 100.,
            max_length: None,
            nap: Nap::None,
            allow_flip: false,
            tilt_deg: 0.,
            ..Default::default()
        },
        spacing: Spacing::default(),
        pieces: vec![p],
        order: Some(Order {
            sizes: BTreeMap::from([("M".into(), 2)]),
        }),
        meta: BTreeMap::new(),
    };
    let r = MarkerResult {
        format: RESULT_FORMAT.into(),
        job_name: j.name.clone(),
        width: 999.,
        length: 999.,
        piece_area: 1.,
        efficiency: 0.,
        useful_piece_area: 0.,
        useful_yield: 0.,
        surplus_copies: 0,
        engine: "handmade".into(),
        seed: None,
        runtime_s: 0.,
        warnings: vec![],
        placements: vec![Placement {
            piece_id: "square".into(),
            copy: 0,
            mirrored: false,
            rotation_deg: 0.,
            translate: [0., 0.],
            locked: false,
            ..Default::default()
        }],
    };
    (j, r)
}

#[test]
fn shaped_notches_keep_all_vertices_and_explicit_slit_depth() {
    let (mut job, result) = fixture();
    job.pieces[0].marks = vec![
        Mark {
            kind: MarkKind::Notch,
            points: vec![[49., 0.], [50., 5.], [51., 0.]],
            text: Some("v".into()),
        },
        Mark {
            kind: MarkKind::Notch,
            points: vec![[60., 0.], [60., 5.]],
            text: Some("slit".into()),
        },
    ];
    let s = scene::build(
        &job,
        &result,
        &ExportOptions {
            labels: false,
            ..Default::default()
        },
    )
    .unwrap();
    let marks: Vec<_> = s.paths.iter().filter(|p| p.layer == 4).collect();
    assert_eq!(marks.len(), 2);
    assert_eq!(marks[0].points, job.pieces[0].marks[0].points);
    assert_eq!(marks[1].points, job.pieces[0].marks[1].points);
}

// Small absolute-coordinate HPGL reader. LB consumes its declared terminator,
// so semicolons inside labels cannot be mistaken for plotter commands.
#[derive(Default)]
struct Plot {
    outlines: Vec<Vec<(i32, i32)>>,
    marks: Vec<Vec<(i32, i32)>>,
    labels: Vec<String>,
}
fn read_hpgl(source: &str) -> Plot {
    let mut result = Plot::default();
    let mut rest = source;
    let mut term = '\x03';
    let mut pen = 0;
    let mut current = (0, 0);
    let mut stroke = Vec::new();
    while !rest.trim_start().is_empty() {
        rest = rest.trim_start();
        let command = &rest[..2];
        rest = &rest[2..];
        if command == "LB" {
            let end = rest.find(term).expect("label terminator");
            result.labels.push(rest[..end].into());
            rest = &rest[end + term.len_utf8()..];
            rest = rest.strip_prefix(';').unwrap_or(rest);
            continue;
        }
        let end = rest.find(';').expect("command terminator");
        let args = &rest[..end];
        rest = &rest[end + 1..];
        match command {
            "DT" => term = args.chars().next().expect("DT character"),
            "SP" => pen = args.parse().expect("pen number"),
            "PU" | "PD" => {
                if command == "PU" && !stroke.is_empty() {
                    if pen == 1 {
                        result.outlines.push(std::mem::take(&mut stroke));
                    } else {
                        result.marks.push(std::mem::take(&mut stroke));
                    }
                }
                let values: Vec<i32> = if args.is_empty() {
                    vec![]
                } else {
                    args.split(',')
                        .map(|v| v.parse().expect("integer coordinate"))
                        .collect()
                };
                assert_eq!(values.len() % 2, 0);
                if command == "PD" && stroke.is_empty() {
                    stroke.push(current);
                }
                for xy in values.as_chunks::<2>().0 {
                    current = (xy[0], xy[1]);
                    if command == "PD" {
                        stroke.push(current);
                    }
                }
            }
            _ => {}
        }
    }
    result
}
#[test]
fn hpgl_calibration_closed_outlines_and_marks() {
    let (j, r) = fixture();
    let out = to_hpgl(&j, &r, &ExportOptions::default()).expect("HPGL");
    assert!(out.starts_with("IN;") && out.ends_with("SP0;\n"));
    assert!(out.contains("SI0.20,0.30;") && out.contains("SP2;"));
    let plot = read_hpgl(&out);
    assert_eq!(plot.outlines.len(), 1);
    assert_eq!(
        plot.outlines[0],
        vec![(0, 0), (4000, 0), (4000, 4000), (0, 4000), (0, 0)]
    );
    for outline in plot.outlines {
        assert_eq!(outline.first(), outline.last());
    }
    assert!(
        plot.marks
            .iter()
            .any(|p| p == &vec![(2000, 0), (2000, 120)])
    );
    assert!(plot.labels.iter().any(|s| s.contains("Fold; do not cut")));
    assert!(plot.labels.iter().any(|s| s.contains("copy 0")));
}
#[test]
fn hpgl_rotation_and_mirrored_geometry() {
    let (j, mut r) = fixture();
    let opts = ExportOptions {
        rotate: true,
        ..ExportOptions::default()
    };
    let a = read_hpgl(&to_hpgl(&j, &r, &opts).expect("HPGL"));
    let xs: Vec<_> = a.outlines[0].iter().map(|p| p.0).collect();
    let ys: Vec<_> = a.outlines[0].iter().map(|p| p.1).collect();
    assert_eq!(
        xs.iter().max().expect("x") - xs.iter().min().expect("x"),
        4000
    );
    assert_eq!(
        ys.iter().max().expect("y") - ys.iter().min().expect("y"),
        4000
    );
    assert!(to_hpgl(&j, &r, &opts).expect("HPGL").contains("DI0,1;"));
    r.placements[0].mirrored = true;
    r.placements[0].translate = [100., 0.];
    let a = read_hpgl(&to_hpgl(&j, &r, &ExportOptions::default()).expect("HPGL"));
    assert_eq!(
        a.outlines[0],
        vec![(4000, 0), (0, 0), (0, 4000), (4000, 4000), (4000, 0)]
    );
    assert!(a.labels.iter().any(|s| s.contains("mirrored")));
}
#[test]
fn svg_units_flip_marks_labels_and_escape() {
    let (j, r) = fixture();
    let out = to_svg(&j, &r, &ExportOptions::default()).expect("SVG");
    assert!(out.contains("width=\"100mm\"") && out.contains("scale(1 -1)"));
    assert!(out.contains("<rect") && out.contains("<polygon") && out.contains("<circle"));
    assert!(out.contains("Front &lt;&amp;&gt;") && out.contains("Efficiency 100.00%"));
    let opts = ExportOptions {
        marks: false,
        labels: false,
        ..ExportOptions::default()
    };
    let out = to_svg(&j, &r, &opts).expect("SVG");
    assert!(!out.contains("<circle") && !out.contains("Front"));
    let out = read_hpgl(&to_hpgl(&j, &r, &opts).expect("HPGL"));
    assert!(out.marks.is_empty());
    assert!(!out.labels.iter().any(|s| s.contains("Front")));
}
fn find(bytes: &[u8], needle: &[u8]) -> usize {
    bytes
        .windows(needle.len())
        .position(|p| p == needle)
        .expect("PDF token")
}
fn check_pdf(bytes: &[u8], pages: usize) {
    assert!(bytes.starts_with(b"%PDF-1.4\n") && bytes.ends_with(b"%%EOF\n"));
    let start = find(bytes, b"startxref\n") + 10;
    let end = bytes[start..]
        .iter()
        .position(|&b| b == b'\n')
        .expect("xref newline")
        + start;
    let offset: usize = std::str::from_utf8(&bytes[start..end])
        .expect("xref offset")
        .parse()
        .expect("integer");
    assert_eq!(&bytes[offset..offset + 5], b"xref\n");
    let xref = std::str::from_utf8(&bytes[offset..]).expect("ASCII xref");
    let mut lines = xref.lines();
    assert_eq!(lines.next(), Some("xref"));
    let count: usize = lines
        .next()
        .expect("xref count")
        .split_whitespace()
        .nth(1)
        .expect("count")
        .parse()
        .expect("integer");
    assert_eq!(lines.next(), Some("0000000000 65535 f "));
    for id in 1..count {
        let line = lines.next().expect("xref entry");
        let pos: usize = line[..10].parse().expect("offset");
        assert!(bytes[pos..].starts_with(format!("{id} 0 obj\n").as_bytes()));
    }
    let text = String::from_utf8_lossy(bytes);
    assert_eq!(text.matches("/Type /Page ").count(), pages);
    assert!(text.contains(&format!("/Count {pages} /Kids")));
    // Check every stream's byte count.
    let mut remaining = bytes;
    while let Some(pos) = remaining.windows(8).position(|w| w == b"/Length ") {
        remaining = &remaining[pos + 8..];
        let n = remaining
            .iter()
            .position(|b| !b.is_ascii_digit())
            .expect("length end");
        let length: usize = std::str::from_utf8(&remaining[..n])
            .expect("length")
            .parse()
            .expect("integer");
        let start = find(remaining, b"stream\n") + 7;
        assert!(remaining[start + length..].starts_with(b"\nendstream"));
        remaining = &remaining[start + length + 10..];
    }
}
#[test]
fn pdf_structure_calibration_and_tiles() {
    let (j, r) = fixture();
    let opts = ExportOptions::default();
    let pdf = to_pdf(&j, &r, &opts).expect("PDF");
    check_pdf(&pdf, 1);
    let text = String::from_utf8_lossy(&pdf);
    let points = 100.0 * 72.0 / 25.4;
    assert!(text.contains(&format!("{points} 0 l")));
    assert!(text.contains(&format!("{points} {points} l")));
    assert!(text.contains("Front <&> \\(test\\)"));
    let opts = ExportOptions {
        pdf_tile_mm: Some(70.),
        ..opts
    };
    let pdf = to_pdf(&j, &r, &opts).expect("tiled PDF");
    check_pdf(&pdf, 4);
    assert!(String::from_utf8_lossy(&pdf).contains("Page 4/4 - column 2, row 2"));
}
#[test]
fn pdf_auto_tiles_both_axes_and_rejects_bad_sizes() {
    let (mut j, mut r) = fixture();
    j.fabric.width = 6000.;
    r.placements[0].translate = [6000., 0.];
    check_pdf(
        &to_pdf(&j, &r, &ExportOptions::default()).expect("large PDF"),
        4,
    );
    for size in [0., -10., 10., f64::NAN, f64::INFINITY] {
        assert!(
            to_pdf(
                &j,
                &r,
                &ExportOptions {
                    pdf_tile_mm: Some(size),
                    ..ExportOptions::default()
                }
            )
            .is_err()
        );
    }
}
#[test]
fn report_recomputes_metrics_and_consumption() {
    let (j, r) = fixture();
    let out = to_report(&j, &r);
    assert!(out.contains("Length: 100.000 mm") && out.contains("Efficiency: 100.00%"));
    assert!(
        out.contains("Size M: 1 pieces") && out.contains("Consumption per garment: 0.050000 m")
    );
}
#[test]
fn unknown_and_non_finite_geometry_are_errors() {
    let (j, mut r) = fixture();
    r.placements[0].piece_id = "unknown".into();
    assert!(to_svg(&j, &r, &ExportOptions::default()).is_err());
    r.placements[0].piece_id = "square".into();
    r.placements[0].translate[0] = f64::NAN;
    assert!(to_pdf(&j, &r, &ExportOptions::default()).is_err());
}

#[test]
fn marks_follow_piece_transform_and_anchor_only_notches_use_the_edge() {
    let (mut j, mut r) = fixture();
    j.pieces[0].marks[0].points.truncate(1);
    r.placements[0].mirrored = true;
    r.placements[0].rotation_deg = 90.;
    r.placements[0].translate = [100., 100.];
    let drawing = scene::build(&j, &r, &ExportOptions::default()).expect("drawing");
    let notch = &drawing.paths[1];
    assert!((notch.points[0][0] - 100.).abs() < 1e-9);
    assert!((notch.points[0][1] - 50.).abs() < 1e-9);
    assert!((notch.points[1][0] - 97.).abs() < 1e-9);
    assert!((notch.points[1][1] - 50.).abs() < 1e-9);
    assert!((drawing.circles[0][0] - 50.).abs() < 1e-9);
    assert!((drawing.circles[0][1] - 50.).abs() < 1e-9);
}

#[test]
fn multiple_sizes_get_distinct_colours_and_every_plot_outline_closes() {
    let (mut j, mut r) = fixture();
    let mut second = j.pieces[0].clone();
    second.id = "square-L".into();
    second.size = Some("L".into());
    j.pieces.push(second);
    let mut placement = r.placements[0].clone();
    placement.piece_id = "square-L".into();
    placement.translate = [100., 0.];
    r.placements.push(placement);
    let opts = ExportOptions::default();
    let drawing = scene::build(&j, &r, &opts).expect("drawing");
    let outlines: Vec<_> = drawing.paths.iter().filter(|p| p.pen == 1).collect();
    assert_ne!(outlines[0].colour, outlines[1].colour);
    let plot = read_hpgl(&to_hpgl(&j, &r, &opts).expect("HPGL"));
    assert_eq!(plot.outlines.len(), 2);
    for outline in plot.outlines {
        assert_eq!(outline.first(), outline.last());
    }
}

#[test]
fn iso_profile_order_winding_and_nearest_corner() {
    let (mut j, mut r) = fixture();
    r.placements[0].mirrored = true;
    r.placements[0].translate = [100., 0.];
    j.pieces[0].grain = Some([[0., 0.], [100., 0.]]);
    j.pieces[0].marks[1].points[0] = [80., 80.];
    let profile:IsoProfile=serde_json::from_str(r#"{"knife_up":"M50","knife_down":"M51","drill":"M52","notch":"M53","cut_feed_mm_min":800,"mark_feed_mm_min":200}"#).expect("profile");
    let out = to_iso(&j, &r, &profile).expect("ISO");
    assert!(out.contains("G21\nG90\n") && out.contains("G01 F800.000"));
    let (marks, cuts) = out.split_once("(OUTLINES)\n").expect("stages");
    assert!(marks.contains("M52") && marks.contains("M53"));
    assert!(!cuts.contains("M52") && !cuts.contains("M53"));
    assert_eq!(cuts.matches("M51").count(), 1); // grain is a guide, not a cut
    let points: Vec<[f64; 2]> = cuts
        .lines()
        .filter(|l| l.starts_with("G00 X") || l.starts_with("G01 X"))
        .map(|l| {
            let mut words = l.split_whitespace().skip(1);
            [
                words.next().expect("X")[1..].parse().expect("x"),
                words.next().expect("Y")[1..].parse().expect("y"),
            ]
        })
        .collect();
    assert_eq!(points.first(), points.last());
    assert!(beta_marker_core::geometry::signed_area(&points) > 0.);
    assert_eq!(points[0], [0., 100.]); // nearest corner after the final drill operation
    assert!(
        to_iso(
            &j,
            &r,
            &IsoProfile {
                cut_feed_mm_min: Some(-1.),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        to_iso(
            &j,
            &r,
            &IsoProfile {
                knife_down: "M03\nG00 X0".into(),
                ..Default::default()
            }
        )
        .is_err()
    );
}
#[test]
fn marker_dxf_round_trips_as_generic_pieces_in_mm() {
    let (mut j, mut r) = fixture();
    let mut p = j.pieces[0].clone();
    p.id = "back".into();
    p.name = "Back".into();
    j.pieces.push(p);
    let mut pl = r.placements[0].clone();
    pl.piece_id = "back".into();
    pl.translate = [120., 0.];
    pl.bundle = Some(2);
    r.placements.push(pl);
    let out = to_dxf(&j, &r, &ExportOptions::default()).expect("DXF");
    assert!(out.contains("AC1009") && out.contains("bundle 2"));
    for layer in [1, 4, 13, 8] {
        assert!(out.contains(&format!("8\n{layer}\n")));
    }
    let imported =
        beta_marker_dxf::import_dxf(out.as_bytes(), &Default::default()).expect("reimport");
    assert_eq!(imported.dialect, "generic");
    assert_eq!(imported.pieces.len(), 2);
    for piece in &imported.pieces {
        assert!((beta_marker_core::geometry::area(&piece.outline) - 10_000.).abs() < 0.001);
    }
    let max = imported
        .pieces
        .iter()
        .filter_map(|p| beta_marker_core::geometry::bbox(&p.outline))
        .map(|b| b[2])
        .fold(0., f64::max);
    assert_eq!(max, 220.); // boundary rectangle is not an extra piece
}
fn unzip(bytes: &[u8], name: &str) -> String {
    use std::io::Read;
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("zip");
    let mut out = String::new();
    zip.by_name(name)
        .expect("part")
        .read_to_string(&mut out)
        .expect("XML");
    out
}
#[test]
fn xlsx_assigns_layer_copies_to_each_garment_and_excludes_surplus() {
    let (mut j, mut r) = fixture();
    j.fabric.layout = Layout::Folded;
    j.order.as_mut().unwrap().sizes.insert("M".into(), 3);
    j.pieces[0].fold_line = Some([[0., 50.], [100., 50.]]);
    r.placements[0].bundle = Some(0);
    r.placements[0].layer_allocations = vec![
        Some(LayerAllocation {
            bundle: 0,
            mirrored: false,
        }),
        Some(LayerAllocation {
            bundle: 1,
            mirrored: false,
        }),
    ];
    let mut last = r.placements[0].clone();
    last.copy = 1;
    last.bundle = Some(2);
    last.translate = [100., 0.];
    last.layer_allocations = vec![
        Some(LayerAllocation {
            bundle: 2,
            mirrored: false,
        }),
        None,
    ];
    r.placements.push(last);
    let sheet = unzip(&to_xlsx(&j, &r, None).unwrap(), "xl/worksheets/sheet2.xml");
    for (row, bundle) in [(2, 0), (3, 1), (4, 2)] {
        for (column, expected) in [("B", bundle), ("E", 1), ("F", 1), ("G", 0)] {
            let cell = sheet
                .split(&format!("r=\"{column}{row}\""))
                .nth(1)
                .unwrap()
                .split("</c>")
                .next()
                .unwrap();
            assert!(
                cell.contains(&format!("<v>{expected}</v>")),
                "{column}{row}: {cell}"
            );
        }
    }
    assert!(!sheet.contains("r=\"B5\""));
}

#[test]
fn xlsx_has_recomputed_summary_literal_text_and_optional_plan() {
    let (mut j, mut r) = fixture();
    j.pieces[0].name = "=1+1".into();
    r.placements[0].bundle = Some(7);
    let table = CutPlanTable {
        headers: vec!["Lay".into(), "Plies".into()],
        rows: vec![vec!["1".into(), "20".into()]],
    };
    let bytes = to_xlsx(&j, &r, Some(&table)).expect("workbook");
    let workbook = unzip(&bytes, "xl/workbook.xml");
    assert!(
        workbook.contains("Marker summary")
            && workbook.contains("Cut plan")
            && workbook.contains("Pieces")
    );
    let strings = unzip(&bytes, "xl/sharedStrings.xml");
    assert!(strings.contains("Consumption per garment") && strings.contains("=1+1"));
    let summary = unzip(&bytes, "xl/worksheets/sheet1.xml");
    assert!(summary.contains("<v>100</v>") && !summary.contains("<v>999</v>"));
    assert!(summary.contains("<v>0.05</v>"));
    let pieces = unzip(&bytes, "xl/worksheets/sheet2.xml");
    assert!(pieces.contains("<v>7</v>") && !pieces.contains("<f>"));
    assert!(
        !unzip(
            &to_xlsx(&j, &r, None).expect("two sheets"),
            "xl/workbook.xml"
        )
        .contains("Cut plan")
    );
    assert!(
        to_xlsx(
            &j,
            &r,
            Some(&CutPlanTable {
                headers: vec!["one".into()],
                rows: vec![vec![]]
            })
        )
        .is_err()
    );
}
#[test]
fn a4_labels_paginate_and_mini_marker_is_one_page() {
    let (j, mut r) = fixture();
    r.placements[0].bundle = Some(3);
    r.placements = (0..25)
        .map(|i| {
            let mut pl = r.placements[0].clone();
            pl.copy = i;
            pl
        })
        .collect();
    let bytes = to_labels_pdf(&j, &r, &LabelOptions::default()).expect("labels");
    check_pdf(&bytes, 2);
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("Bundle: 3") && text.contains("Copy: 24"));
    let page = format!(
        "/MediaBox [0 0 {} {}]",
        210. * (72. / 25.4),
        297. * (72. / 25.4)
    );
    assert!(text.contains(&page));
    let mini = to_mini_marker_pdf(&j, &r).expect("mini");
    check_pdf(&mini, 1);
    let text = String::from_utf8_lossy(&mini);
    assert!(text.contains("overview only") && text.contains("Consumption per garment"));
    assert!(
        to_labels_pdf(
            &j,
            &r,
            &LabelOptions {
                columns: 0,
                ..Default::default()
            }
        )
        .is_err()
    );
}
#[test]
fn drawings_include_blocks_guides_and_fold_halves() {
    let (mut j, mut r) = fixture();
    j.fabric.shrinkage.length_pct = 10.;
    j.pieces[0].block_margin = Some(2.);
    r.placements[0].translate = [2., 2.];
    j.fabric.defects = vec![Zone {
        id: "hole".into(),
        polygon: vec![[150., 10.], [160., 10.], [160., 20.], [150., 20.]],
        note: None,
    }];
    j.fabric.repeat = Some(Repeat {
        period_x: Some(20.),
        ..Default::default()
    });
    j.fabric.shade = Some(ShadeRule {
        axis: ShadeAxis::Length,
        band_mm: 50.,
    });
    let scene = scene::build(&j, &r, &ExportOptions::default()).expect("scene");
    let bounds = beta_marker_core::geometry::bbox(&scene.paths[0].points).expect("block bounds");
    assert!(
        bounds
            .iter()
            .zip([0., 0., 114., 104.])
            .all(|(actual, expected)| (actual - expected).abs() < 1e-8)
    );
    assert!(scene.paths.iter().any(|p| p.colour == "#b3261e"));
    assert!(
        scene.paths.iter().any(|p| p.colour == "#dddddd")
            && scene.paths.iter().any(|p| p.colour == "#b08738")
    );
    assert!(
        to_hpgl(&j, &r, &ExportOptions::default())
            .expect("plot")
            .contains("SP3;")
    );
    j.pieces[0].block_margin = None;
    j.fabric.shrinkage = Default::default();
    j.fabric.layout = Layout::Folded;
    j.pieces[0].fold_line = Some([[100., 50.], [0., 50.]]);
    r.placements[0].on_fold = true;
    r.placements[0].translate = [0., 50.];
    let out = to_svg(&j, &r, &ExportOptions::default()).expect("fold svg");
    assert!(out.contains("FOLD"));
    let scene = scene::build(&j, &r, &ExportOptions::default()).expect("fold scene");
    assert_eq!(
        beta_marker_core::geometry::bbox(&scene.paths[0].points),
        Some([0., 50., 100., 100.])
    );
    let iso = to_iso(&j, &r, &IsoProfile::default()).expect("fold ISO");
    let cuts = iso.split_once("(OUTLINES)\n").expect("cuts").1;
    assert_eq!(cuts.lines().filter(|l| l.starts_with("G01 X")).count(), 3); // leave fold uncut
    j.fabric.layout = Layout::Tubular;
    j.pieces[0].fold_line = Some([[0., 50.], [100., 50.]]);
    r.placements[0].translate = [0., -50.];
    let scene = scene::build(&j, &r, &ExportOptions::default()).expect("bottom fold");
    let label = scene
        .texts
        .iter()
        .find(|t| t.value == "FOLD")
        .expect("fold label");
    assert_eq!(label.point[1], 3.0); // Label follows the bottom fold, not the top cut edge.
}

#[test]
fn iso_keeps_corners_next_to_duplicate_and_collinear_points() {
    let (mut j, r) = fixture();
    j.pieces[0].marks.clear();
    j.pieces[0].outline = vec![
        [0., 0.],
        [0., 0.],
        [50., 0.],
        [100., 0.],
        [100., 100.],
        [0., 100.],
        [0., 0.],
    ];
    let iso = to_iso(&j, &r, &IsoProfile::default()).expect("ISO");
    let cuts = iso.split_once("(OUTLINES)\n").expect("outlines").1;
    assert_eq!(cuts.lines().filter(|l| l.starts_with("G01 X")).count(), 4);
    assert!(cuts.contains("G00 X0.000 Y0.000") && cuts.contains("G01 X100.000 Y0.000"));
}

#[test]
fn cutter_skips_each_fold_segment_on_a_concave_half() {
    let (mut j, mut r) = fixture();
    j.fabric.layout = Layout::Tubular;
    j.pieces[0].marks.clear();
    // Exercise two fold segments on a concave outline. Symmetry of imported
    // patterns is outside the cutter writer; this checks only its stroke splitting.
    j.pieces[0].outline = vec![
        [0., -2.],
        [10., -2.],
        [10., 5.],
        [20., 5.],
        [20., -2.],
        [30., -2.],
        [30., 10.],
        [0., 10.],
    ];
    j.pieces[0].fold_line = Some([[0., 0.], [30., 0.]]);
    r.placements[0].on_fold = true;
    let iso = to_iso(&j, &r, &IsoProfile::default()).expect("concave fold");
    let cuts = iso.split_once("(OUTLINES)\n").expect("outlines").1;
    let mut previous: [f64; 2] = [0., 0.];
    for line in cuts
        .lines()
        .filter(|l| l.starts_with("G00 X") || l.starts_with("G01 X"))
    {
        let words: Vec<_> = line.split_whitespace().collect();
        let point = [
            words[1][1..].parse::<f64>().expect("x"),
            words[2][1..].parse::<f64>().expect("y"),
        ];
        if line.starts_with("G01") {
            assert!(
                !(previous[1].abs() < 0.01 && point[1].abs() < 0.01),
                "cut along fold: {line}"
            );
        }
        previous = point;
    }
}

#[test]
fn iso_visits_the_nearest_outline_before_a_farther_input_placement() {
    let (mut j, mut r) = fixture();
    j.pieces[0].marks.clear();
    let mut far = r.placements[0].clone();
    far.copy = 1;
    far.translate = [200., 0.];
    r.placements.insert(0, far);
    let out = to_iso(&j, &r, &IsoProfile::default()).expect("ISO");
    let cuts = out.split_once("(OUTLINES)\n").expect("outlines").1;
    assert!(
        cuts.find("G00 X0.000 Y0.000").expect("near") < cuts.find("G00 X200.000").expect("far")
    );
}

#[test]
fn unified_export_dispatches_every_format_and_rejects_unknown() {
    let (job, result) = fixture();
    for (format, extension) in [
        ("svg", "svg"),
        ("plt", "plt"),
        ("pdf", "pdf"),
        ("report", "txt"),
        ("iso", "iso"),
        ("dxf", "dxf"),
        ("xlsx", "xlsx"),
        ("labels", "pdf"),
        ("mini", "pdf"),
    ] {
        let (bytes, mime, ext) =
            export(format, &job, &result, &ExportOptions::default()).expect(format);
        assert!(!bytes.is_empty());
        assert!(mime.contains('/'));
        assert_eq!(ext, extension);
    }
    assert!(export("bad", &job, &result, &ExportOptions::default()).is_err());
}
