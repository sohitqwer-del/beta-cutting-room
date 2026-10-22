//! Reproducible block evidence. All output paths are supplied by the caller.
//!
//! `capture OUT [BASELINE]` writes, for both example charts and every block: the document,
//! graded geometry, block checks, spec sheet, PDF/CSV/XLSX spec files and an SVG review
//! sheet with every size stacked. With BASELINE (a folder of saved `<chart>-<block>.json`
//! documents) the saved documents are measured instead of new drafts.
//!
//! `report BEFORE AFTER MARKDOWN` tabulates every check of two capture folders.
use anyhow::{Context, Result, bail};
use beta_pattern_core::{
    Document,
    blocks::{self, Create},
    evaluate,
    model::GradedPiece,
};
use serde_json::Value;
use std::{collections::BTreeMap, fmt::Write as _, path::Path};

const CHARTS: [&str; 2] = ["indian_adult", "iso_example"];
const COLOURS: [&str; 5] = ["#1b7837", "#2166ac", "#b2182b", "#762a83", "#8c510a"];

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["capture", out] => capture(Path::new(out), None),
        ["capture", out, base] => capture(Path::new(out), Some(Path::new(base))),
        ["report", before, after, md] => report(Path::new(before), Path::new(after), Path::new(md)),
        _ => bail!("Usage: capture OUT [BASELINE] | report BEFORE AFTER MARKDOWN"),
    }
}

fn capture(root: &Path, baseline: Option<&Path>) -> Result<()> {
    std::fs::create_dir_all(root)?;
    for preset in CHARTS {
        for info in blocks::catalog() {
            let id = serde_json::to_value(info.id)?
                .as_str()
                .context("block id")?
                .to_owned();
            let key = format!("{preset}-{id}");
            let doc: Document = if let Some(base) = baseline {
                serde_json::from_str(&std::fs::read_to_string(base.join(format!("{key}.json")))?)?
            } else {
                blocks::create(&Create {
                    block: Some(info.id),
                    chart: Some(blocks::size_chart(preset)?),
                    ..Default::default()
                })?
            };
            let prefix = root.join(&key);
            let report = beta_pattern_core::block_checks::report(&doc)?;
            for row in &report.sizes {
                for c in row.checks.iter().filter(|c| !c.pass) {
                    eprintln!("{key}/{} {} = {:.3} {}", row.size, c.id, c.value, c.unit);
                }
                for error in &row.errors {
                    eprintln!("{key}/{} ERROR {error}", row.size);
                }
            }
            std::fs::write(
                prefix.with_extension("checks.json"),
                serde_json::to_string_pretty(&report)?,
            )?;
            if let Ok(spec) = beta_pattern_core::block_checks::spec_sheet(&doc) {
                std::fs::write(
                    prefix.with_extension("spec.json"),
                    serde_json::to_string_pretty(&spec)?,
                )?;
            }
            if baseline.is_none() {
                for format in ["pdf", "csv", "xlsx"] {
                    std::fs::write(
                        prefix.with_extension(format),
                        beta_pattern_core::spec_export::export(&doc, format)?.bytes,
                    )?;
                }
                std::fs::write(
                    prefix.with_extension("json"),
                    serde_json::to_string_pretty(&doc)?,
                )?;
            }
            let graded = doc
                .sizes
                .iter()
                .map(|s| Ok((s.clone(), evaluate::grade(&doc, s, 0.05)?)))
                .collect::<Result<Vec<_>>>()?;
            std::fs::write(
                prefix.with_extension("graded.json"),
                serde_json::to_string(&graded.iter().cloned().collect::<BTreeMap<_, _>>())?,
            )?;
            let stage = if baseline.is_some() {
                "before (bd2a230)"
            } else {
                "after"
            };
            std::fs::write(
                prefix.with_extension("svg"),
                svg(
                    &format!("{} - {preset} example chart - {stage}", info.name),
                    &doc,
                    &graded,
                )?,
            )?;
        }
    }
    Ok(())
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// True-scale review sheet: every size's stitch line stacked per piece, base-size cut line,
/// grain and notches. Pieces flow into rows; one scale for the whole sheet.
fn svg(title: &str, doc: &Document, graded: &[(String, Vec<GradedPiece>)]) -> Result<String> {
    const SCALE: f64 = 0.5; // px per mm
    const ROW_MM: f64 = 2400.;
    const GAP: f64 = 60.;
    let base = graded
        .iter()
        .position(|(s, _)| *s == doc.base_size)
        .unwrap_or(0);
    let pieces = graded[0].1.len();
    // Bounds per piece over all sizes, including the cut line.
    let mut boxes = vec![];
    for i in 0..pieces {
        let mut b = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for (_, g) in graded {
            for p in g[i].seam.iter().chain(&g[i].piece.outline) {
                b = [
                    b[0].min(p[0]),
                    b[1].min(p[1]),
                    b[2].max(p[0]),
                    b[3].max(p[1]),
                ];
            }
        }
        boxes.push(b);
    }
    let names: Vec<String> = graded[base]
        .1
        .iter()
        .map(|g| {
            format!(
                "{} (cut {}{})",
                g.piece.name,
                g.piece.quantity,
                if g.piece.mirror_quantity > 0 {
                    format!(" + {} mirrored", g.piece.mirror_quantity)
                } else {
                    String::new()
                }
            )
        })
        .collect();
    // Flow layout in mm; y grows downward on the sheet. A 12 px label needs about
    // 7 px (14 mm) per character, so short pieces reserve their label width.
    let (mut x, mut y, mut row_h) = (0., 0., 0f64);
    let mut origin = vec![];
    for (b, name) in boxes.iter().zip(&names) {
        let w = (b[2] - b[0]).max(name.chars().count() as f64 * 14.);
        let h = b[3] - b[1] + 40.;
        if x > 0. && x + w > ROW_MM {
            x = 0.;
            y += row_h + GAP;
            row_h = 0.;
        }
        origin.push((x, y));
        x += w + GAP;
        row_h = row_h.max(h);
    }
    let sheet_w = ROW_MM * SCALE + 40.;
    let top = 96.;
    let sheet_h = top + (y + row_h) * SCALE + 70.;
    let mut out = String::new();
    writeln!(
        out,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{sheet_w:.0}" height="{sheet_h:.0}" viewBox="0 0 {sheet_w:.0} {sheet_h:.0}" font-family="Arial, Helvetica, sans-serif" role="img" aria-label="{}">"##,
        escape(title)
    )?;
    writeln!(
        out,
        r##"<title>{}</title><rect width="100%" height="100%" fill="#ffffff"/>"##,
        escape(title)
    )?;
    writeln!(
        out,
        r##"<text x="20" y="30" font-size="20" fill="#14212b">{}</text>"##,
        escape(title)
    )?;
    writeln!(
        out,
        r##"<text x="20" y="52" font-size="12" fill="#3b4a56">True scale, 1 px = 2 mm. Coloured: stitch line of every size. Grey dashed: {} cut line. Black: {} grain and notches. Example block, not a cutting file.</text>"##,
        escape(&doc.base_size),
        escape(&doc.base_size)
    )?;
    for (k, (size, _)) in graded.iter().enumerate() {
        let lx = 20. + k as f64 * 80.;
        let c = COLOURS[k % COLOURS.len()];
        writeln!(
            out,
            r##"<path d="M{lx} 74h26" stroke="{c}" stroke-width="3"/><text x="{:.0}" y="79" font-size="13" fill="{c}">{}</text>"##,
            lx + 32.,
            escape(size)
        )?;
    }
    let point = |i: usize, p: [f64; 2]| {
        let (ox, oy) = origin[i];
        let b = boxes[i];
        (
            20. + (ox + p[0] - b[0]) * SCALE,
            top + 24. + (oy + b[3] - p[1]) * SCALE,
        )
    };
    let poly = |i: usize, pts: &[[f64; 2]]| {
        pts.iter()
            .map(|p| {
                let (x, y) = point(i, *p);
                format!("{x:.2},{y:.2}")
            })
            .collect::<Vec<_>>()
            .join(" ")
    };
    for i in 0..pieces {
        let g = &graded[base].1[i];
        let (lx, ly) = (20. + origin[i].0 * SCALE, top + 14. + origin[i].1 * SCALE);
        let name = &names[i];
        writeln!(
            out,
            r##"<text x="{lx:.1}" y="{ly:.1}" font-size="12" fill="#14212b">{}</text>"##,
            escape(name)
        )?;
        writeln!(
            out,
            r##"<polygon points="{}" fill="none" stroke="#8a949c" stroke-width="1" stroke-dasharray="5 3"/>"##,
            poly(i, &g.piece.outline)
        )?;
        for (k, (_, sized)) in graded.iter().enumerate() {
            writeln!(
                out,
                r##"<polygon points="{}" fill="none" stroke="{}" stroke-width="1.2"/>"##,
                poly(i, &sized[i].seam),
                COLOURS[k % COLOURS.len()]
            )?;
        }
        if let Some(grain) = g.piece.grain {
            writeln!(
                out,
                r##"<polyline points="{}" fill="none" stroke="#14212b" stroke-width="1.2" stroke-dasharray="8 4"/>"##,
                poly(i, &grain)
            )?;
        }
        for mark in &g.piece.marks {
            if mark.kind == beta_marker_core::MarkKind::Notch && !mark.points.is_empty() {
                writeln!(
                    out,
                    r##"<polyline points="{}" fill="none" stroke="#000000" stroke-width="2"/>"##,
                    poly(i, &mark.points)
                )?;
            }
        }
    }
    let sy = sheet_h - 30.;
    writeln!(
        out,
        r##"<path d="M20 {sy:.0}h{:.0}" stroke="#14212b" stroke-width="2"/><text x="{:.0}" y="{:.0}" font-size="12" fill="#14212b">100 mm</text></svg>"##,
        100. * SCALE,
        28. + 100. * SCALE,
        sy + 4.
    )?;
    Ok(out)
}

fn load(dir: &Path) -> Result<BTreeMap<String, Value>> {
    let mut out = BTreeMap::new();
    for preset in CHARTS {
        for info in blocks::catalog() {
            let id = serde_json::to_value(info.id)?
                .as_str()
                .context("block id")?
                .to_owned();
            let key = format!("{preset}-{id}");
            let text = std::fs::read_to_string(dir.join(format!("{key}.checks.json")))
                .with_context(|| format!("{key}.checks.json"))?;
            out.insert(key, serde_json::from_str(&text)?);
        }
    }
    Ok(out)
}

fn counts(reports: &BTreeMap<String, Value>) -> [usize; 5] {
    let rows: Vec<&Value> = reports
        .values()
        .flat_map(|r| r["sizes"].as_array().into_iter().flatten())
        .collect();
    let checks: Vec<&Value> = rows
        .iter()
        .flat_map(|r| r["checks"].as_array().into_iter().flatten())
        .collect();
    [
        rows.len(),
        rows.iter().filter(|r| r["pass"] == true).count(),
        checks.len(),
        checks.iter().filter(|c| c["pass"] != true).count(),
        rows.iter()
            .filter(|r| r["errors"].as_array().is_some_and(|e| !e.is_empty()))
            .count(),
    ]
}

fn report(before_dir: &Path, after_dir: &Path, md: &Path) -> Result<()> {
    let before = load(before_dir)?;
    let after = load(after_dir)?;
    let mut s = String::new();
    let line = |s: &mut String, t: &str| {
        s.push_str(t);
        s.push('\n');
    };
    line(&mut s, "# Block checks: before and after round 0.7");
    line(&mut s, "");
    line(
        &mut s,
        "Generated by `cargo run -p beta-pattern-core --example block_evidence`. Do not edit by hand.",
    );
    line(&mut s, "");
    line(
        &mut s,
        "- **Before**: the documents that `blocks::create` made at `bd2a230` (release 0.6.0), default ease, saved as JSON and measured with the current checks.",
    );
    line(&mut s, "- **After**: the current drafts.");
    line(
        &mut s,
        "- Both bundled charts are synthetic examples (see `BLOCKS.md`). Sizes S to XXL.",
    );
    line(
        &mut s,
        "- These are geometry and consistency checks. Passing them is not wearer-fit approval; a sewn sample is still needed.",
    );
    line(&mut s, "");
    line(
        &mut s,
        "| Snapshot | Size rows | Passing rows | Checks | Failed checks | Rows with errors |",
    );
    line(&mut s, "|---|---:|---:|---:|---:|---:|");
    for (label, r) in [("Before", &before), ("After", &after)] {
        let c = counts(r);
        line(
            &mut s,
            &format!(
                "| {label} | {} | {} | {} | {} | {} |",
                c[0], c[1], c[2], c[3], c[4]
            ),
        );
    }
    line(&mut s, "");
    line(
        &mut s,
        "Check counts differ because the new drafts add marks and pieces that are checked. `n/a` means the check did not exist for that draft; it is not counted as a pass.",
    );
    line(
        &mut s,
        "Seam and measurement values are signed residuals (actual minus expected). Tangent values are the kink in degrees after opening the joined seam flat. Bounds are inclusive. Values are rounded to 0.01; pass/fail uses full precision.",
    );
    line(&mut s, "");
    line(&mut s, "## What the before failures mean");
    line(&mut s, "");
    line(
        &mut s,
        "- **Trouser and track pant**: `crotch_extension_total` was 0.01-0.02 of hip. The 0.6 legs had almost no crotch extension, so the panels could not pass between the legs. `thigh_hip` 0.51 is a tight thigh. Front and back were nearly the same shape (`back_front_rise` 1.01-1.02), and the crotch kinked 48-54 degrees at the inseam.",
    );
    line(
        &mut s,
        "- **Shirt**: `yoke_depth_cb` 25 mm. The yoke seam went straight through the shoulder points, so the yoke had no armhole part.",
    );
    line(
        &mut s,
        "- **Knit and kurta bodies**: `back.concave_radius_4/5` 1.1-1.5. The 20 mm deep back neck curved so tightly that the 10 mm allowance nearly folded over itself.",
    );
    line(
        &mut s,
        "- **Tangents**: joined corners kinked badly, for example 113 degrees where the armhole meets the side seam, 138-148 degrees at the T-shirt sleeve underarm, 24-42 degrees at the neck/shoulder join.",
    );
    line(
        &mut s,
        "- **Marks and grain**: assembly notches, component grain and ease figures that the 0.6 drafts did not have.",
    );
    line(&mut s, "");
    line(&mut s, "## Every block and size");
    line(&mut s, "");
    line(
        &mut s,
        "| Chart / block | Size | Before failed / checks | After failed / checks |",
    );
    line(&mut s, "|---|---|---:|---:|");
    let rows = |r: &Value| r["sizes"].as_array().cloned().unwrap_or_default();
    let failed = |row: &Value| {
        let checks = row["checks"].as_array().cloned().unwrap_or_default();
        let errors = row["errors"].as_array().map_or(0, Vec::len);
        let text = format!(
            "{} / {}",
            checks.iter().filter(|c| c["pass"] != true).count(),
            checks.len()
        );
        if errors > 0 {
            format!("{text} + error")
        } else {
            text
        }
    };
    for (key, a) in &after {
        for (b, a) in rows(&before[key]).iter().zip(rows(a)) {
            line(
                &mut s,
                &format!(
                    "| {key} | {} | {} | {} |",
                    a["size"].as_str().unwrap_or(""),
                    failed(b),
                    failed(&a)
                ),
            );
        }
    }
    for (key, a) in &after {
        let sizes: Vec<String> = rows(a)
            .iter()
            .map(|r| r["size"].as_str().unwrap_or("").to_owned())
            .collect();
        let index = |r: &Value| -> BTreeMap<String, BTreeMap<String, Value>> {
            rows(r)
                .iter()
                .map(|row| {
                    (
                        row["size"].as_str().unwrap_or("").to_owned(),
                        row["checks"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .map(|c| (c["id"].as_str().unwrap_or("").to_owned(), c.clone()))
                            .collect(),
                    )
                })
                .collect()
        };
        let (bm, am) = (index(&before[key]), index(a));
        // Order: current checks first (in report order), then checks only in the old draft.
        let mut ids: Vec<(String, Value)> = vec![];
        for map in [&am, &bm] {
            for size in &sizes {
                for (id, c) in map.get(size).into_iter().flatten() {
                    if !ids.iter().any(|(i, _)| i == id) {
                        ids.push((id.clone(), c.clone()));
                    }
                }
            }
        }
        let order: Vec<String> = rows(a)
            .first()
            .and_then(|r| r["checks"].as_array().cloned())
            .unwrap_or_default()
            .iter()
            .filter_map(|c| c["id"].as_str().map(str::to_owned))
            .collect();
        ids.sort_by_key(|(id, _)| order.iter().position(|o| o == id).unwrap_or(usize::MAX));
        line(&mut s, "");
        line(&mut s, &format!("## {key}"));
        line(&mut s, "");
        for error in rows(&before[key])
            .iter()
            .chain(rows(a).iter())
            .flat_map(|r| r["errors"].as_array().cloned().unwrap_or_default())
        {
            line(
                &mut s,
                &format!("- Evaluation error: {}", error.as_str().unwrap_or("")),
            );
        }
        let mut header = "| Check | Unit | Bounds |".to_owned();
        let mut rule = "|---|---|---|".to_owned();
        for stage in ["Before", "After"] {
            for size in &sizes {
                header.push_str(&format!(" {stage} {size} |"));
                rule.push_str("---:|");
            }
        }
        line(&mut s, &header);
        line(&mut s, &rule);
        for (id, c) in ids {
            let mut text = format!(
                "| {id} | {} | {:.2} to {:.2} |",
                c["unit"].as_str().unwrap_or(""),
                c["min"].as_f64().unwrap_or(f64::NAN),
                c["max"].as_f64().unwrap_or(f64::NAN)
            );
            for map in [&bm, &am] {
                for size in &sizes {
                    match map.get(size).and_then(|m| m.get(&id)) {
                        Some(v) => {
                            let value = v["value"].as_f64().unwrap_or(f64::NAN);
                            let value = if value.abs() < 0.005 { 0. } else { value };
                            text.push_str(&format!(
                                " {value:.2}{} |",
                                if v["pass"] == true { "" } else { " **FAIL**" }
                            ));
                        }
                        None => text.push_str(" n/a |"),
                    }
                }
            }
            line(&mut s, &text);
        }
    }
    std::fs::write(md, s)?;
    let (b, a) = (counts(&before), counts(&after));
    println!("before {b:?} after {a:?}");
    Ok(())
}
