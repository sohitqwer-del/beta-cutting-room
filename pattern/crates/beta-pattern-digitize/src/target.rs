//! Original BETA square codes and scale-true SVG/PDF calibration boards.
use anyhow::{Result, ensure};
use beta_marker_core::Pt;
use serde::{Deserialize, Serialize};
use std::{fmt::Write as _, path::Path};

/// The supported printer sheet sizes, in landscape orientation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SheetSize {
    /// 420 by 297 mm.
    A3,
    /// 594 by 420 mm.
    A2,
}
impl SheetSize {
    /// Width and height in millimetres.
    pub fn dimensions(self) -> Pt {
        match self {
            Self::A3 => [420., 297.],
            Self::A2 => [594., 420.],
        }
    }
}
/// One original 8 by 8 square code, with an outer black ring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetMarker {
    /// Unique unsigned identifier (not an ArUco or AprilTag family).
    pub id: u8,
    /// Top-left corner on the board; board coordinates run down.
    pub origin: Pt,
    /// Outside edge length, in mm.
    pub size_mm: f64,
}
impl TargetMarker {
    /// Four outside corners, clockwise from top-left in image coordinates.
    pub fn corners(&self) -> [Pt; 4] {
        let [x, y] = self.origin;
        let s = self.size_mm;
        [[x, y], [x + s, y], [x + s, y + s], [x, y + s]]
    }
}
/// A measurable, serializable target manifest. Keep it with the printed sheets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    /// Format identifier, checked before scanning.
    pub format: String,
    /// Physical board extent, in mm.
    pub width_mm: f64,
    /// Physical board extent, in mm.
    pub height_mm: f64,
    /// Unique coded squares at known positions.
    pub markers: Vec<TargetMarker>,
    /// Printer page format.
    pub sheet: SheetSize,
    /// Page columns; adjacent pages overlap by 10 mm.
    pub columns: u32,
    /// Page rows; adjacent pages overlap by 10 mm.
    pub rows: u32,
    /// Export only perimeter pages, leaving a table's centre unprinted.
    pub table_border: bool,
}
#[derive(Clone, Copy)]
pub(crate) struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub grey: u8,
}

/// CRC-8 (polynomial 0x1d, initial 0xa7) of the one-byte identifier.
pub(crate) fn checksum(id: u8) -> u8 {
    let mut c = id ^ 0xa7;
    for _ in 0..8 {
        c = if c & 128 != 0 {
            (c << 1) ^ 0x1d
        } else {
            c << 1
        };
    }
    c
}
/// True for a black code cell. The inner corner at (1,1) sets orientation.
pub fn black_cell(id: u8, x: usize, y: usize) -> bool {
    if x >= 8 || y >= 8 {
        return false;
    }
    if x == 0 || y == 0 || x == 7 || y == 7 {
        return true;
    }
    if x == 1 && y == 1 {
        return true;
    }
    if (2..6).contains(&x) && (2..6).contains(&y) {
        let i = (y - 2) * 4 + x - 2;
        let word = (id as u16) | ((checksum(id) as u16) << 8);
        return word & (1 << i) != 0;
    }
    false
}
impl Target {
    /// Make a target of one or more A3/A2 sheets. Trim/overlap 10 mm joins.
    pub fn tiled(sheet: SheetSize, columns: u32, rows: u32, table_border: bool) -> Result<Self> {
        ensure!(
            (1..=16).contains(&columns) && (1..=16).contains(&rows),
            "tile counts must be 1..16"
        );
        let [w, h] = sheet.dimensions();
        let width = columns as f64 * (w - 10.) + 10.;
        let height = rows as f64 * (h - 10.) + 10.;
        let mut positions = Vec::new();
        let nx = ((width - 40.) / 95.).ceil() as usize;
        let ny = ((height - 40.) / 95.).ceil() as usize;
        for i in 0..=nx {
            let x = 20. + i as f64 * (width - 40.) / nx as f64;
            positions.push([x, 20.]);
            positions.push([x, height - 20.]);
        }
        for i in 1..ny {
            let y = 20. + i as f64 * (height - 40.) / ny as f64;
            positions.push([20., y]);
            positions.push([width - 20., y]);
        }
        ensure!(
            positions.len() <= 256,
            "board needs more than 256 marker codes; use fewer tiles"
        );
        let target = Self {
            format: "beta-digitize.target/v1".into(),
            width_mm: width,
            height_mm: height,
            markers: positions
                .into_iter()
                .enumerate()
                .map(|(id, c)| TargetMarker {
                    id: id as u8,
                    origin: [c[0] - 12., c[1] - 12.],
                    size_mm: 24.,
                })
                .collect(),
            sheet,
            columns,
            rows,
            table_border,
        };
        target.validate()?;
        Ok(target)
    }
    /// Validate a manifest received from an untrusted file.
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.format == "beta-digitize.target/v1",
            "unsupported target format"
        );
        ensure!(
            self.width_mm.is_finite()
                && self.height_mm.is_finite()
                && (80. ..=20000.).contains(&self.width_mm)
                && (80. ..=20000.).contains(&self.height_mm),
            "invalid target dimensions"
        );
        ensure!(
            (1..=16).contains(&self.columns) && (1..=16).contains(&self.rows),
            "invalid tile counts"
        );
        ensure!(
            (4..=256).contains(&self.markers.len()),
            "target needs 4..256 markers"
        );
        let mut used = [false; 256];
        for m in &self.markers {
            ensure!(!used[m.id as usize], "duplicate marker id");
            used[m.id as usize] = true;
            ensure!(
                m.size_mm.is_finite()
                    && (8. ..=100.).contains(&m.size_mm)
                    && m.origin.iter().all(|v| v.is_finite()),
                "invalid marker"
            );
            ensure!(
                m.origin[0] >= 0.
                    && m.origin[1] >= 0.
                    && m.origin[0] + m.size_mm <= self.width_mm
                    && m.origin[1] + m.size_mm <= self.height_mm,
                "marker outside target"
            );
        }
        Ok(())
    }
    /// Drawing rectangles, shared by both printable formats and synthetic tests.
    pub(crate) fn rectangles(&self, background: bool) -> Vec<Rect> {
        let mut r = Vec::new();
        if background {
            r.push(Rect {
                x: 5.,
                y: 5.,
                w: self.width_mm - 10.,
                h: self.height_mm - 10.,
                grey: 100,
            });
        }
        // Known-size 10 mm checker border; alternating black/white 5 mm squares.
        for i in 0..((self.width_mm - 10.) / 5.).floor() as usize {
            for j in 0..2 {
                for y in [5. + j as f64 * 5., self.height_mm - 15. + j as f64 * 5.] {
                    r.push(Rect {
                        x: 5. + i as f64 * 5.,
                        y,
                        w: 5.,
                        h: 5.,
                        grey: if (i + j) % 2 == 0 { 0 } else { 255 },
                    });
                }
            }
        }
        for i in 0..((self.height_mm - 30.) / 5.).floor() as usize {
            for j in 0..2 {
                for x in [5. + j as f64 * 5., self.width_mm - 15. + j as f64 * 5.] {
                    r.push(Rect {
                        x,
                        y: 15. + i as f64 * 5.,
                        w: 5.,
                        h: 5.,
                        grey: if (i + j) % 2 == 0 { 0 } else { 255 },
                    });
                }
            }
        }
        // Quiet zones must cover the checkerboard underneath each code.
        for m in &self.markers {
            r.push(Rect {
                x: m.origin[0] - 2.,
                y: m.origin[1] - 2.,
                w: m.size_mm + 4.,
                h: m.size_mm + 4.,
                grey: 255,
            });
            for y in 0..8 {
                for x in 0..8 {
                    if black_cell(m.id, x, y) {
                        r.push(Rect {
                            x: m.origin[0] + x as f64 * m.size_mm / 8.,
                            y: m.origin[1] + y as f64 * m.size_mm / 8.,
                            w: m.size_mm / 8.,
                            h: m.size_mm / 8.,
                            grey: 0,
                        });
                    }
                }
            }
        }
        r
    }
    /// Save a complete, metric SVG board (large boards require tiled printing).
    pub fn svg(&self) -> Result<String> {
        self.validate()?;
        let mut s = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}mm\" height=\"{}mm\" viewBox=\"0 0 {} {}\"><title>BETA calibration target: print at 100 percent; checker cells 5 mm; marker 24 mm</title><rect width=\"100%\" height=\"100%\" fill=\"white\"/>",
            self.width_mm, self.height_mm, self.width_mm, self.height_mm
        );
        for r in self.rectangles(!self.table_border) {
            write!(s,"<rect x=\"{:.5}\" y=\"{:.5}\" width=\"{:.5}\" height=\"{:.5}\" fill=\"rgb({grey},{grey},{grey})\"/>",r.x,r.y,r.w,r.h,grey=r.grey).ok();
        }
        s.push_str("</svg>");
        Ok(s)
    }
    /// Generate an uncompressed vector PDF. Each page is a physical A3/A2 sheet.
    /// Table mode emits perimeter pages only. Page positions appear in the margin.
    pub fn pdf(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let [w, h] = self.sheet.dimensions();
        let scale = 72. / 25.4;
        let pages: Vec<_> = (0..self.rows)
            .flat_map(|row| (0..self.columns).map(move |col| (col, row)))
            .filter(|&(c, r)| {
                !self.table_border
                    || c == 0
                    || r == 0
                    || c + 1 == self.columns
                    || r + 1 == self.rows
            })
            .collect();
        let mut objects = vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            String::new(),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
        ];
        let rects = self.rectangles(!self.table_border);
        let mut kids = Vec::new();
        for (col, row) in pages {
            let page_id = objects.len() + 1;
            let content_id = page_id + 1;
            kids.push(format!("{page_id} 0 R"));
            objects.push(format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.5} {:.5}] /Resources << /Font << /F1 3 0 R >> >> /Contents {content_id} 0 R >>",w*scale,h*scale));
            let ox = col as f64 * (w - 10.);
            let oy = row as f64 * (h - 10.);
            let mut content = format!(
                "q {scale:.8} 0 0 -{scale:.8} 0 {:.8} cm 5 5 {:.5} {:.5} re W n\n",
                h * scale,
                w - 10.,
                h - 10.
            );
            for r in &rects {
                let x = r.x - ox;
                let y = r.y - oy;
                if x + r.w >= 5. && y + r.h >= 5. && x <= w - 5. && y <= h - 5. {
                    writeln!(
                        content,
                        "{:.5} g {:.5} {:.5} {:.5} {:.5} re f",
                        r.grey as f64 / 255.,
                        x,
                        y,
                        r.w,
                        r.h
                    )
                    .ok();
                }
            }
            // This label stays inside the printer's 5 mm margin and between
            // bottom-row codes on a single sheet. It never hides a marker.
            writeln!(content,"Q q 1 g {:.5} {:.5} {:.5} {:.5} re f Q BT /F1 5 Tf {:.5} {:.5} Td 7 TL (BETA 100% - tile {},{}) Tj T* (Origin {:.0},{:.0} mm) Tj T* (10 checker cells = 50 mm; code = 24 mm) Tj ET",38.*scale,17.*scale,55.*scale,9.*scale,39.*scale,23.*scale,col+1,row+1,ox,oy).ok();
            objects.push(format!(
                "<< /Length {} >>\nstream\n{}endstream",
                content.len(),
                content
            ));
        }
        objects[1] = format!(
            "<< /Type /Pages /Kids [{}] /Count {} >>",
            kids.join(" "),
            kids.len()
        );
        let mut out = b"%PDF-1.4\n".to_vec();
        let mut offsets = vec![0];
        for (i, obj) in objects.iter().enumerate() {
            offsets.push(out.len());
            out.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", i + 1, obj).as_bytes());
        }
        let xref = out.len();
        out.extend_from_slice(
            format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len()).as_bytes(),
        );
        for offset in &offsets[1..] {
            out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        out.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
                offsets.len()
            )
            .as_bytes(),
        );
        Ok(out)
    }
    /// Save a PDF or SVG by extension, and a matching `.target.json` manifest.
    pub fn save(&self, path: &Path) -> Result<()> {
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        match ext.to_ascii_lowercase().as_str() {
            "pdf" => std::fs::write(path, self.pdf()?)?,
            "svg" => std::fs::write(path, self.svg()?)?,
            _ => anyhow::bail!("target output must end in .pdf or .svg"),
        }
        std::fs::write(
            path.with_extension("target.json"),
            serde_json::to_vec_pretty(self)?,
        )?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metric_pages_and_unique_codes() {
        let t = Target::tiled(SheetSize::A3, 2, 2, true).unwrap();
        assert_eq!(t.width_mm, 830.);
        assert_eq!(t.height_mm, 584.);
        assert!(t.pdf().unwrap().starts_with(b"%PDF-1.4"));
        assert!(t.svg().unwrap().contains("830mm"));
        let mut bad = t.clone();
        bad.markers[1].id = bad.markers[0].id;
        assert!(bad.validate().is_err());
    }
}
