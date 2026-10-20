//! Small offline PDF and HPGL writers. No fonts, renderer or runtime dependency is required.
use crate::{
    Document, evaluate,
    geometry::{bbox, distance},
};
use anyhow::{Context, Result, ensure};
use beta_marker_core::{MarkKind, Piece, Pt};
use serde::{Deserialize, Serialize};
use std::fmt::Write;
const PT: f64 = 72. / 25.4;
/// Supported portrait paper sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Paper {
    /// 210 by 297 millimetres.
    #[default]
    A4,
    /// 297 by 420 millimetres.
    A3,
}
impl Paper {
    /// Width and height in millimetres.
    pub fn dimensions(self) -> Pt {
        match self {
            Self::A4 => [210., 297.],
            Self::A3 => [297., 420.],
        }
    }
}
/// Print at actual size (100 percent); page-fit scaling must be disabled by the user.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PrintOptions {
    /// Portrait page dimensions.
    pub paper: Paper,
    /// Printer-safe border, millimetres.
    pub margin_mm: f64,
    /// Shared content between adjacent pages, millimetres.
    pub overlap_mm: f64,
    /// Curve flattening tolerance, millimetres.
    pub tolerance_mm: f64,
}
impl Default for PrintOptions {
    fn default() -> Self {
        Self {
            paper: Paper::A4,
            margin_mm: 10.,
            overlap_mm: 10.,
            tolerance_mm: 0.1,
        }
    }
}
/// A single tile. Row zero is the bottom row; column zero is leftmost.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrintTile {
    /// Evaluated piece ID including size.
    pub piece_id: String,
    /// Zero-based piece index in export order.
    pub piece_index: usize,
    /// Zero-based column.
    pub column: usize,
    /// Zero-based row from bottom.
    pub row: usize,
    /// Number of columns for this piece.
    pub columns: usize,
    /// Number of rows for this piece.
    pub rows: usize,
    /// Lower-left of tile in original piece coordinates, millimetres.
    pub origin: Pt,
    /// Content width and height, millimetres.
    pub extent: Pt,
}
fn pieces(doc: &Document, sizes: &[String], tolerance: f64) -> Result<Vec<Piece>> {
    ensure!(!sizes.is_empty(), "select at least one print size");
    ensure!(
        sizes
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            == sizes.len(),
        "duplicate print size"
    );
    let pieces = sizes
        .iter()
        .map(|s| evaluate::grade(doc, s, tolerance))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .map(|g| g.piece)
        .collect::<Vec<_>>();
    ensure!(!pieces.is_empty(), "no pieces to print");
    Ok(pieces)
}
fn bounds(p: &Piece) -> Result<[f64; 4]> {
    let mut points = p.outline.clone();
    points.extend(p.grain.into_iter().flatten());
    points.extend(p.fold_line.into_iter().flatten());
    points.extend(p.marks.iter().flat_map(|m| m.points.iter().copied()));
    for mark in &p.marks {
        if mark.kind == MarkKind::Text
            && let (Some(origin), Some(text)) = (mark.points.first(), &mark.text)
        {
            points.push([origin[0] + text.chars().count() as f64 * 3., origin[1] + 4.]);
        }
    }
    let b = bbox(&points).context("empty piece")?;
    ensure!(b.iter().all(|v| v.is_finite()), "non-finite print bounds");
    Ok([b[0] - 2., b[1] - 2., b[2] + 2., b[3] + 2.])
}
fn plan(pieces: &[Piece], options: &PrintOptions) -> Result<Vec<PrintTile>> {
    let [w, h] = options.paper.dimensions();
    let m = options.margin_mm;
    let overlap = options.overlap_mm;
    ensure!(
        m.is_finite() && (5. ..=40.).contains(&m),
        "print margin must be 5 to 40 mm"
    );
    let extent = [w - 2. * m, h - 2. * m];
    ensure!(
        overlap.is_finite() && overlap >= 2. && overlap < extent[0].min(extent[1]) * 0.5,
        "invalid print overlap"
    );
    let step = [extent[0] - overlap, extent[1] - overlap];
    let mut out = vec![];
    for (i, p) in pieces.iter().enumerate() {
        let b = bounds(p)?;
        let cols = ((b[2] - b[0] - overlap) / step[0]).ceil().max(1.);
        let rows = ((b[3] - b[1] - overlap) / step[1]).ceil().max(1.);
        ensure!(
            cols * rows <= 10000. && (out.len() as f64 + cols * rows) <= 10000.,
            "print exceeds 10000 tiles"
        );
        let cols = cols as usize;
        let rows = rows as usize;
        for row in 0..rows {
            for col in 0..cols {
                out.push(PrintTile {
                    piece_id: p.id.clone(),
                    piece_index: i,
                    column: col,
                    row,
                    columns: cols,
                    rows,
                    origin: [b[0] + col as f64 * step[0], b[1] + row as f64 * step[1]],
                    extent,
                });
            }
        }
    }
    Ok(out)
}
/// Plan tile origins before export. The PDF adds a separate calibration page first.
pub fn tile_layout(
    doc: &Document,
    sizes: &[String],
    options: &PrintOptions,
) -> Result<Vec<PrintTile>> {
    plan(&pieces(doc, sizes, options.tolerance_mm)?, options)
}
fn ascii(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii() && !c.is_ascii_control() {
                c
            } else {
                '?'
            }
        })
        .collect()
}
fn escape(s: &str) -> String {
    ascii(s)
        .replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
}
fn text(out: &mut String, p: Pt, size: f64, s: &str) -> Result<()> {
    writeln!(
        out,
        "BT /F1 {size:.4} Tf 1 0 0 1 {:.4} {:.4} Tm ({}) Tj ET",
        p[0],
        p[1],
        escape(s)
    )?;
    Ok(())
}
fn path(out: &mut String, points: &[Pt], closed: bool) -> Result<()> {
    if let Some(first) = points.first() {
        writeln!(out, "{:.6} {:.6} m", first[0], first[1])?;
        for p in &points[1..] {
            writeln!(out, "{:.6} {:.6} l", p[0], p[1])?;
        }
        out.push_str(if closed { "h S\n" } else { "S\n" });
    }
    Ok(())
}
fn piece_content(p: &Piece) -> Result<String> {
    let mut out = String::from("0 0 0 RG 0.25 w\n");
    path(&mut out, &p.outline, true)?;
    if let Some(grain) = p.grain {
        out.push_str("0.15 w\n");
        path(&mut out, &grain, false)?;
        let v = [grain[1][0] - grain[0][0], grain[1][1] - grain[0][1]];
        let n = distance(grain[0], grain[1]);
        if n > 1e-9 {
            let u = [v[0] / n, v[1] / n];
            path(
                &mut out,
                &[
                    [
                        grain[1][0] - u[0] * 5. - u[1] * 2.,
                        grain[1][1] - u[1] * 5. + u[0] * 2.,
                    ],
                    grain[1],
                    [
                        grain[1][0] - u[0] * 5. + u[1] * 2.,
                        grain[1][1] - u[1] * 5. - u[0] * 2.,
                    ],
                ],
                false,
            )?;
        }
    }
    if let Some(fold) = p.fold_line {
        out.push_str("0.15 w [3 2] 0 d\n");
        path(&mut out, &fold, false)?;
        out.push_str("[] 0 d\n");
    }
    for m in &p.marks {
        if m.kind == MarkKind::Text {
            if let (Some(p), Some(s)) = (m.points.first(), &m.text) {
                text(&mut out, *p, 3., s)?;
            }
        } else if m.kind == MarkKind::Drill {
            for p in &m.points {
                path(&mut out, &[[p[0] - 1., p[1]], [p[0] + 1., p[1]]], false)?;
                path(&mut out, &[[p[0], p[1] - 1.], [p[0], p[1] + 1.]], false)?;
            }
        } else {
            path(&mut out, &m.points, false)?;
        }
    }
    Ok(out)
}
fn stream(content: &str, extra: &str) -> Vec<u8> {
    format!(
        "<< /Length {} {extra} >>\nstream\n{content}endstream",
        content.len()
    )
    .into_bytes()
}
fn document(objects: Vec<Vec<u8>>) -> Result<Vec<u8>> {
    let mut out = b"%PDF-1.4\n% BETA Pattern vector print\n".to_vec();
    let mut offsets = vec![0usize];
    for (i, o) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(o);
        out.extend_from_slice(b"\nendobj\n");
    }
    let start = out.len();
    let mut trailer = format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len());
    for offset in offsets.iter().skip(1) {
        ensure!((*offset as u64) < 10_000_000_000_u64, "PDF offset overflow");
        writeln!(trailer, "{offset:010} 00000 n ")?;
    }
    write!(
        trailer,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{start}\n%%EOF\n",
        offsets.len()
    )?;
    out.extend_from_slice(trailer.as_bytes());
    Ok(out)
}
fn page(
    objects: &mut Vec<Vec<u8>>,
    kids: &mut Vec<usize>,
    dimensions: Pt,
    content: String,
    xobjects: &str,
) {
    let id = objects.len() + 1;
    kids.push(id);
    objects.push(format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.6} {:.6}] /Resources << /Font << /F1 3 0 R >> /XObject << {xobjects} >> >> /Contents {} 0 R >>",dimensions[0]*PT,dimensions[1]*PT,id+1).into_bytes());
    objects.push(stream(&content, ""));
}
fn cross(out: &mut String, p: Pt) -> Result<()> {
    path(
        out,
        &[[p[0] - 2. * PT, p[1]], [p[0] + 2. * PT, p[1]]],
        false,
    )?;
    path(
        out,
        &[[p[0], p[1] - 2. * PT], [p[0], p[1] + 2. * PT]],
        false,
    )?;
    Ok(())
}
/// Dependency-free PDF 1.4: portrait A4/A3, vector pieces at 1:1, true overlap,
/// alignment crosses, tile coordinates and a separate 100 mm calibration-square page.
/// Each piece-size is printed once; labels retain its required copy quantities.
pub fn tiled_pdf(doc: &Document, sizes: &[String], options: &PrintOptions) -> Result<Vec<u8>> {
    let pieces = pieces(doc, sizes, options.tolerance_mm)?;
    let tiles = plan(&pieces, options)?;
    let dims = options.paper.dimensions();
    let m = options.margin_mm;
    let overlap = options.overlap_mm;
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        vec![],
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
    ];
    let mut forms = vec![];
    for p in &pieces {
        let b = bounds(p)?;
        let extra = format!(
            "/Type /XObject /Subtype /Form /BBox [{:.6} {:.6} {:.6} {:.6}] /Resources << /Font << /F1 3 0 R >> >>",
            b[0], b[1], b[2], b[3]
        );
        forms.push(objects.len() + 1);
        objects.push(stream(&piece_content(p)?, &extra));
    }
    let mut kids = vec![];
    let mut calibration = String::from("0 0 0 RG 0.5 w\n");
    text(
        &mut calibration,
        [20. * PT, (dims[1] - 25.) * PT],
        16.,
        "BETA Pattern - print calibration",
    )?;
    text(
        &mut calibration,
        [20. * PT, (dims[1] - 38.) * PT],
        10.,
        "Print at ACTUAL SIZE / 100%. Disable Fit to page.",
    )?;
    writeln!(
        calibration,
        "{:.6} {:.6} {:.6} {:.6} re S",
        20. * PT,
        (dims[1] - 160.) * PT,
        100. * PT,
        100. * PT
    )?;
    text(
        &mut calibration,
        [20. * PT, (dims[1] - 170.) * PT],
        10.,
        "This square must measure 100 mm by 100 mm (10 cm).",
    )?;
    text(
        &mut calibration,
        [20. * PT, (dims[1] - 185.) * PT],
        10.,
        &format!("Tiles overlap by {overlap} mm. Match crosses and grid coordinates."),
    )?;
    text(
        &mut calibration,
        [20. * PT, (dims[1] - 198.) * PT],
        10.,
        "Rows count from bottom; columns count from left. Trim one overlap edge.",
    )?;
    page(&mut objects, &mut kids, dims, calibration, "");
    for tile in &tiles {
        let mut out = String::new();
        let [w, h] = tile.extent;
        writeln!(
            out,
            "q\n{:.6} {:.6} {:.6} {:.6} re W n\n{PT:.10} 0 0 {PT:.10} {:.6} {:.6} cm\n/P Do\nQ",
            m * PT,
            m * PT,
            w * PT,
            h * PT,
            (m - tile.origin[0]) * PT,
            (m - tile.origin[1]) * PT
        )?;
        out.push_str("0.6 0.6 0.6 RG 0.25 w [2 2] 0 d\n");
        writeln!(
            out,
            "{:.6} {:.6} {:.6} {:.6} re S\n[] 0 d",
            m * PT,
            m * PT,
            w * PT,
            h * PT
        )?;
        // Crosses at the centre of each overlap. A shared global 50 mm grid gives
        // at least two matches even when adjacent rows or columns are shifted.
        out.push_str("0 0 0 RG 0.4 w\n");
        let b = bounds(&pieces[tile.piece_index])?;
        for x in [overlap * 0.5, w - overlap * 0.5] {
            let ymin = ((tile.origin[1] - b[1]) / 50.).ceil() as i64;
            let ymax = ((tile.origin[1] + h - b[1]) / 50.).floor() as i64;
            for i in ymin..=ymax {
                let y = b[1] + i as f64 * 50. - tile.origin[1];
                if y > 3. && y < h - 3. {
                    cross(&mut out, [(m + x) * PT, (m + y) * PT])?;
                }
            }
        }
        for y in [overlap * 0.5, h - overlap * 0.5] {
            let xmin = ((tile.origin[0] - b[0]) / 50.).ceil() as i64;
            let xmax = ((tile.origin[0] + w - b[0]) / 50.).floor() as i64;
            for i in xmin..=xmax {
                let x = b[0] + i as f64 * 50. - tile.origin[0];
                if x > 3. && x < w - 3. {
                    cross(&mut out, [(m + x) * PT, (m + y) * PT])?;
                }
            }
        }
        let label = format!(
            "{} | col {}/{} row {}/{} | x {:.1} y {:.1} mm",
            tile.piece_id,
            tile.column + 1,
            tile.columns,
            tile.row + 1,
            tile.rows,
            tile.origin[0],
            tile.origin[1]
        );
        let label: String = ascii(&label).chars().take(120).collect();
        text(&mut out, [m * PT, 4. * PT], 7., &label)?;
        text(
            &mut out,
            [m * PT, (dims[1] - 5.) * PT],
            8.,
            "1:1 - actual size 100%",
        )?;
        page(
            &mut objects,
            &mut kids,
            dims,
            out,
            &format!("/P {} 0 R", forms[tile.piece_index]),
        );
    }
    objects[1] = format!(
        "<< /Type /Pages /Count {} /Kids [{}] >>",
        kids.len(),
        kids.iter()
            .map(|id| format!("{id} 0 R"))
            .collect::<Vec<_>>()
            .join(" ")
    )
    .into_bytes();
    document(objects)
}
fn plot_point(p: Pt) -> Result<(i32, i32)> {
    ensure!(
        p.iter()
            .all(|v| v.is_finite() && (*v * 40.).abs() <= i32::MAX as f64),
        "plotter coordinate exceeds integer range"
    );
    Ok(((p[0] * 40.).round() as i32, (p[1] * 40.).round() as i32))
}
fn plot_path(out: &mut String, points: &[Pt], closed: bool, offset: Pt) -> Result<()> {
    if let Some(first) = points.first() {
        let (x, y) = plot_point([first[0] + offset[0], first[1] + offset[1]])?;
        write!(out, "PU{x},{y};")?;
        for p in points
            .iter()
            .skip(1)
            .chain(if closed { Some(first) } else { None })
        {
            let (x, y) = plot_point([p[0] + offset[0], p[1] + offset[1]])?;
            write!(out, "PD{x},{y};")?;
        }
        out.push_str("PU;\n");
    }
    Ok(())
}
/// HPGL/PLT at 40 units/mm. Piece-sizes are placed left-to-right with a 20 mm gap.
/// Pen 1 is the cut boundary; pen 2 is grain, marks and labels. Coordinates are rounded
/// to the nearest plotter unit (at most 0.0125 mm error per axis).
pub fn plotter_plt(doc: &Document, sizes: &[String], tolerance: f64) -> Result<String> {
    let pieces = pieces(doc, sizes, tolerance)?;
    let mut out = String::from("IN;PA;DT\x03;SI0.20,0.30;\n");
    let mut cursor = 10.;
    for p in &pieces {
        let b = bounds(p)?;
        let offset = [cursor - b[0], 10. - b[1]];
        out.push_str("SP1;\n");
        plot_path(&mut out, &p.outline, true, offset)?;
        out.push_str("SP2;\n");
        if let Some(g) = p.grain {
            plot_path(&mut out, &g, false, offset)?;
        }
        if let Some(fold) = p.fold_line {
            plot_path(&mut out, &fold, false, offset)?;
        }
        for m in &p.marks {
            if m.kind == MarkKind::Text {
                if let (Some(p), Some(s)) = (m.points.first(), &m.text) {
                    let (x, y) = plot_point([p[0] + offset[0], p[1] + offset[1]])?;
                    writeln!(out, "PU{x},{y};LB{}\x03;", ascii(s))?;
                }
            } else if m.kind == MarkKind::Drill {
                for p in &m.points {
                    plot_path(
                        &mut out,
                        &[[p[0] - 1., p[1]], [p[0] + 1., p[1]]],
                        false,
                        offset,
                    )?;
                    plot_path(
                        &mut out,
                        &[[p[0], p[1] - 1.], [p[0], p[1] + 1.]],
                        false,
                        offset,
                    )?;
                }
            } else {
                plot_path(&mut out, &m.points, false, offset)?;
            }
        }
        let (x, y) = plot_point([cursor, 6.])?;
        writeln!(out, "PU{x},{y};LB{}\x03;", ascii(&p.id))?;
        cursor += b[2] - b[0] + 20.;
    }
    out.push_str("PU;SP0;\n");
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a4_a3_scale_overlap_pdf_offsets_and_plotter_units() {
        let mut doc = Document::default();
        crate::tools::insert_polygon(
            &mut doc,
            "square",
            "Square",
            &[[0., 0.], [400., 0.], [400., 300.], [0., 300.]],
        )
        .unwrap();
        let sizes = doc.sizes.clone();
        let options = PrintOptions::default();
        let tiles = tile_layout(&doc, &sizes, &options).unwrap();
        assert_eq!(tiles.len(), 6);
        assert_eq!(tiles[1].origin[0] - tiles[0].origin[0], 180.);
        assert_eq!(tiles[0].extent[0] - 180., 10.);
        let pdf = tiled_pdf(&doc, &sizes, &options).unwrap();
        let text = std::str::from_utf8(&pdf).unwrap();
        assert!(text.contains("/Count 7 "));
        assert!(text.contains("283.464567 283.464567 re S"));
        assert!(text.contains("595.275591 841.889764"));
        let xref = text
            .split("startxref\n")
            .nth(1)
            .unwrap()
            .lines()
            .next()
            .unwrap()
            .parse::<usize>()
            .unwrap();
        assert!(text[xref..].starts_with("xref\n"));
        let xref_lines: Vec<_> = text[xref..].lines().collect();
        let count = xref_lines[1]
            .split_whitespace()
            .nth(1)
            .unwrap()
            .parse::<usize>()
            .unwrap();
        for id in 1..count {
            let offset = xref_lines[2 + id]
                .split_whitespace()
                .next()
                .unwrap()
                .parse::<usize>()
                .unwrap();
            assert!(text[offset..].starts_with(&format!("{id} 0 obj")));
        }
        let a3 = PrintOptions {
            paper: Paper::A3,
            ..options.clone()
        };
        assert_eq!(tile_layout(&doc, &sizes, &a3).unwrap().len(), 2);
        assert!(
            tiled_pdf(
                &doc,
                &sizes,
                &PrintOptions {
                    overlap_mm: f64::NAN,
                    ..options
                }
            )
            .is_err()
        );
        let plt = plotter_plt(&doc, &sizes, 0.1).unwrap();
        assert!(plt.contains("PU480,480;PD16480,480;PD16480,12480;PD480,12480;PD480,480;"));
        assert!(plt.ends_with("PU;SP0;\n"));
    }
}
