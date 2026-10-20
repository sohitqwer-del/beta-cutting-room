//! Minimal PDF 1.4 writer. Coordinates are explicit points; no rasterisation or
//! external fonts are needed. Page content clips a shared vector Form XObject.
use crate::scene::{Scene, ascii};
use anyhow::{Context, ensure};
use std::fmt::Write;

const PT: f64 = 72.0 / 25.4;
const MARGIN: f64 = 5.0;
const MAX_PAGE: f64 = 5080.0;
pub(super) fn escaped(s: &str) -> String {
    ascii(s)
        .replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
}
pub(super) fn stream(content: &str, extra: &str) -> Vec<u8> {
    format!(
        "<< /Length {} {extra} >>\nstream\n{content}\nendstream",
        content.len()
    )
    .into_bytes()
}
pub(super) fn scene_content(s: &Scene) -> anyhow::Result<String> {
    let mut out = format!(
        "0.5 w\n0 0 0 RG\n0 0 {} {} re S\n",
        s.length * PT,
        s.width * PT
    );
    for path in &s.paths {
        let Some(first) = path.points.first() else {
            continue;
        };
        let colour = path.colour.trim_start_matches('#');
        if let Ok(rgb) = u32::from_str_radix(colour, 16) {
            writeln!(
                out,
                "{} {} {} RG",
                ((rgb >> 16) & 255) as f64 / 255.0,
                ((rgb >> 8) & 255) as f64 / 255.0,
                (rgb & 255) as f64 / 255.0
            )?;
        }
        writeln!(out, "{} w", if path.pen == 3 { 0.25 } else { 0.5 })?;
        writeln!(out, "{} {} m", first[0] * PT, first[1] * PT)?;
        for p in &path.points[1..] {
            writeln!(out, "{} {} l", p[0] * PT, p[1] * PT)?;
        }
        out.push_str(if path.closed { "h S\n" } else { "S\n" });
    }
    for p in &s.circles {
        let x = p[0] * PT;
        let y = p[1] * PT;
        let r = 0.75 * PT;
        let k = r * 0.5522847498307936;
        writeln!(out, "{} {y} m", x + r)?;
        for [a, b, c, d, e, f] in [
            [x + r, y + k, x + k, y + r, x, y + r],
            [x - k, y + r, x - r, y + k, x - r, y],
            [x - r, y - k, x - k, y - r, x, y - r],
            [x + k, y - r, x + r, y - k, x + r, y],
        ] {
            writeln!(out, "{a} {b} {c} {d} {e} {f} c")?;
        }
        out.push_str("h S\n");
    }
    for text in &s.texts {
        writeln!(
            out,
            "BT /F1 {} Tf 1 0 0 1 {} {} Tm ({}) Tj ET",
            3.0 * PT,
            text.point[0] * PT,
            text.point[1] * PT,
            escaped(&text.value)
        )?;
    }
    Ok(out)
}

pub(super) fn render(s: &Scene, tile: Option<f64>) -> anyhow::Result<Vec<u8>> {
    let side = tile.unwrap_or(MAX_PAGE).min(MAX_PAGE);
    ensure!(
        side.is_finite() && side > 2.0 * MARGIN,
        "PDF tile size must be finite and greater than 10 mm."
    );
    if let Some(requested) = tile {
        ensure!(
            requested.is_finite() && requested > 2.0 * MARGIN,
            "PDF tile size must be finite and greater than 10 mm."
        );
    }
    let extent = side - 2.0 * MARGIN;
    let nx = (s.canvas_length / extent).ceil().max(1.0);
    let ny = (s.height / extent).ceil().max(1.0);
    ensure!(
        nx * ny <= 100_000.0,
        "PDF would need more than 100,000 pages; use larger tiles."
    );
    let nx = nx as usize;
    let ny = ny as usize;
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        vec![],
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
        stream(
            &scene_content(s)?,
            &format!(
                "/Type /XObject /Subtype /Form /BBox [0 0 {} {}] /Resources << /Font << /F1 3 0 R >> >>",
                s.canvas_length * PT,
                s.height * PT
            ),
        ),
    ];
    let mut kids = String::new();
    for row in 0..ny {
        for col in 0..nx {
            let x = col as f64 * extent;
            let y = row as f64 * extent;
            let w = (s.canvas_length - x).min(extent);
            let h = (s.height - y).min(extent);
            let page = objects.len() + 1;
            write!(kids, "{page} 0 R ")?;
            objects.push(format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << /XObject << /Marker 4 0 R >> /Font << /F1 3 0 R >> >> /Contents {} 0 R >>", (w + 2.0*MARGIN)*PT, (h+2.0*MARGIN)*PT, page+1).into_bytes());
            let mut content = format!(
                "q\n{} {} {} {} re W n\n1 0 0 1 {} {} cm\n/Marker Do\nQ\n0 0 0 RG 0.4 w\n",
                MARGIN * PT,
                MARGIN * PT,
                w * PT,
                h * PT,
                (MARGIN - x) * PT,
                (MARGIN - y) * PT
            );
            // Crosses sit at matching tile boundary corners, in the margin.
            for cx in [MARGIN, MARGIN + w] {
                for cy in [MARGIN, MARGIN + h] {
                    writeln!(
                        content,
                        "{} {} m {} {} l S\n{} {} m {} {} l S",
                        (cx - 2.0) * PT,
                        cy * PT,
                        (cx + 2.0) * PT,
                        cy * PT,
                        cx * PT,
                        (cy - 2.0) * PT,
                        cx * PT,
                        (cy + 2.0) * PT
                    )?;
                }
            }
            let page_label = format!(
                "Page {}/{} - column {}, row {}",
                row * nx + col + 1,
                nx * ny,
                col + 1,
                row + 1
            );
            let font_size = 7.0_f64.min(w * PT / (page_label.len() as f64 * 0.6));
            writeln!(
                content,
                "BT /F1 {font_size} Tf {} {} Td ({page_label}) Tj ET",
                MARGIN * PT,
                1.5 * PT,
            )?;
            objects.push(stream(&content, ""));
        }
    }
    objects[1] = format!("<< /Type /Pages /Count {} /Kids [{kids}] >>", nx * ny).into_bytes();
    finish(objects)
}

/// A set of same-size pages with Helvetica and explicit point coordinates.
pub(super) fn paper_pages(
    contents: &[String],
    width_mm: f64,
    height_mm: f64,
) -> anyhow::Result<Vec<u8>> {
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        vec![],
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
    ];
    let mut kids = String::new();
    for content in contents {
        let page = objects.len() + 1;
        write!(kids, "{page} 0 R ")?;
        objects.push(format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << /Font << /F1 3 0 R >> >> /Contents {} 0 R >>",width_mm*PT,height_mm*PT,page+1).into_bytes());
        objects.push(stream(content, ""));
    }
    objects[1] = format!(
        "<< /Type /Pages /Count {} /Kids [{kids}] >>",
        contents.len()
    )
    .into_bytes();
    finish(objects)
}

pub(super) fn finish(objects: Vec<Vec<u8>>) -> anyhow::Result<Vec<u8>> {
    let mut out = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = vec![0];
    for (i, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(object);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    let mut tail = format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len());
    for offset in &offsets[1..] {
        ensure!(
            (*offset as u64) <= 9_999_999_999_u64,
            "PDF is too large for a classic xref table."
        );
        writeln!(tail, "{offset:010} 00000 n ")?;
    }
    write!(
        tail,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        offsets.len()
    )
    .context("Writing PDF trailer")?;
    out.extend_from_slice(tail.as_bytes());
    Ok(out)
}
