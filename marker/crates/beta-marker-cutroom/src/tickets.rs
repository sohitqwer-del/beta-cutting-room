//! Versioned QR scan payloads and compressed vector PDF labels.
use crate::bundles::{Bundle, PieceKey, Sticker};
use anyhow::{Result, ensure};
use flate2::{Compression, write::ZlibEncoder};
use qrcode::{Color, EcLevel, QrCode};
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use std::io::Write as IoWrite;

/// Public WIP scan contract. Short JSON keys keep labels compact. See README.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScanPayload {
    /// Schema version, currently 1.
    pub v: u8,
    /// Style.
    pub s: String,
    /// Order.
    pub o: String,
    /// Size.
    pub z: String,
    /// Colour.
    pub c: String,
    /// Lay identity (bundle numbers alone are not globally unique).
    pub l: String,
    /// Shade group.
    pub h: String,
    /// Part id.
    pub part: String,
    /// Physical handedness shared by this bundle.
    pub m: bool,
    /// Floor bundle number.
    pub b: u32,
    /// Inclusive physical ply range.
    pub p: [u32; 2],
    /// Quantity in the bundle; one for a piece sticker.
    pub q: u32,
    /// Optional physical piece reference on a sticker.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub piece: Option<PieceKey>,
    /// Printed garment number on a piece sticker.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub n: Option<u32>,
}
impl ScanPayload {
    /// Construct a bundle-ticket payload, validating all physical references.
    pub fn bundle(bundle: &Bundle) -> Result<Self> {
        validate_bundle(bundle)?;
        let payload = Self {
            v: 1,
            s: bundle.style.clone(),
            o: bundle.order.clone(),
            z: bundle.size.clone(),
            c: bundle.colour.clone(),
            l: bundle.lay.clone(),
            h: bundle.shade_group.clone(),
            part: bundle.piece_id.clone(),
            m: bundle.stickers[0].piece.mirrored,
            b: bundle.number,
            p: bundle.plies,
            q: bundle.stickers.len() as u32,
            piece: None,
            n: None,
        };
        payload.check()?;
        Ok(payload)
    }
    /// Encode compact UTF-8 JSON after validation.
    pub fn encode(&self) -> Result<String> {
        self.check()?;
        Ok(serde_json::to_string(self)?)
    }
    /// Decode and validate a scanned payload, including the version.
    pub fn decode(text: &str) -> Result<Self> {
        ensure!(text.len() <= 4096, "scan payload exceeds 4096 bytes");
        let value: Self = serde_json::from_str(text)?;
        value.check()?;
        Ok(value)
    }
    fn check(&self) -> Result<()> {
        ensure!(self.v == 1, "unsupported scan payload version");
        ensure!(
            [
                &self.s, &self.o, &self.z, &self.c, &self.l, &self.h, &self.part
            ]
            .iter()
            .all(|s| !s.trim().is_empty() && s.len() <= 256),
            "scan identifiers must contain 1 to 256 bytes"
        );
        ensure!(
            self.b > 0 && self.q > 0 && self.p[0] > 0 && self.p[1] >= self.p[0],
            "invalid bundle quantity or ply range"
        );
        ensure!(
            self.piece.is_some() == self.n.is_some(),
            "piece reference and sticker number must occur together"
        );
        if let Some(piece) = &self.piece {
            ensure!(
                self.q == 1
                    && self.n.is_some_and(|n| n > 0)
                    && piece.piece_id == self.part
                    && piece.mirrored == self.m
                    && self.p == [piece.ply, piece.ply],
                "invalid piece scan reference"
            );
        }
        Ok(())
    }
}

pub(crate) fn validate_bundle(b: &Bundle) -> Result<()> {
    ensure!(
        !b.stickers.is_empty() && b.stickers.len() <= 1_000_000,
        "bundle needs 1 to one million pieces"
    );
    ensure!(
        b.plies[0] > 0 && b.plies[1] >= b.plies[0],
        "invalid bundle ply range"
    );
    let mut keys = std::collections::BTreeSet::new();
    let mut plies = std::collections::BTreeSet::new();
    let mirrored = b.stickers[0].piece.mirrored;
    for s in &b.stickers {
        ensure!(
            s.piece.piece_id == b.piece_id
                && s.piece.mirrored == mirrored
                && (b.plies[0]..=b.plies[1]).contains(&s.piece.ply)
                && s.number > 0
                && s.number <= s.last_number
                && keys.insert(&s.piece),
            "invalid or duplicate bundle piece"
        );
        plies.insert(s.piece.ply);
    }
    ensure!(
        plies.len() as u64 == u64::from(b.plies[1]) - u64::from(b.plies[0]) + 1,
        "bundle must contain one contiguous ply range"
    );
    Ok(())
}

/// Sheet geometry. Print at 100% scale; disable printer fit-to-page.
#[derive(Debug, Clone, Copy)]
pub enum TicketPaper {
    /// A4 portrait, ten 100 × 50 mm labels, 5 mm left/top margin.
    A4,
    /// One 100 × 50 mm page for a label printer.
    Label100x50,
}

/// Make a PDF with one QR ticket per bundle. Empty input is an error.
pub fn ticket_pdf(bundles: &[Bundle], paper: TicketPaper) -> Result<Vec<u8>> {
    let labels = bundles
        .iter()
        .map(|b| {
            let payload = ScanPayload::bundle(b)?;
            Ok((payload, lines(b, None)))
        })
        .collect::<Result<Vec<_>>>()?;
    render(labels, paper)
}

/// Make a numbering sticker with a QR reference for every physical piece.
pub fn sticker_pdf(bundles: &[Bundle], paper: TicketPaper) -> Result<Vec<u8>> {
    let mut labels = Vec::new();
    for b in bundles {
        let payload = ScanPayload::bundle(b)?;
        for s in &b.stickers {
            let mut p = payload.clone();
            p.q = 1;
            p.p = [s.piece.ply, s.piece.ply];
            p.piece = Some(s.piece.clone());
            p.n = Some(s.number);
            labels.push((p, lines(b, Some(s))));
            ensure!(
                labels.len() <= 100_000,
                "PDF exceeds 100,000 stickers; split the print job"
            );
        }
    }
    render(labels, paper)
}
fn lines(b: &Bundle, s: Option<&Sticker>) -> Vec<String> {
    vec![
        s.map_or_else(
            || format!("BUNDLE {}  Qty {}", b.number, b.stickers.len()),
            |s| format!("{}  Bundle {}", s.label(), b.number),
        ),
        format!("Style: {}", b.style),
        format!("Order: {}", b.order),
        format!("Size: {}  Colour: {}", b.size, b.colour),
        format!(
            "Part: {} ({})",
            b.piece_id,
            if b.stickers[0].piece.mirrored {
                "mirror"
            } else {
                "normal"
            }
        ),
        format!("Lay: {}", b.lay),
        format!(
            "Shade: {}",
            serde_json::from_str::<[String; 3]>(&b.shade_group)
                .map(|[lot, dye, shade]| format!("{lot} / {dye} / {shade}"))
                .unwrap_or_else(|_| b.shade_group.clone())
        ),
        s.map_or_else(
            || format!("Plies: {}-{}", b.plies[0], b.plies[1]),
            |s| {
                format!(
                    "Ply {}  Garment {}  Slot {} {}",
                    s.piece.ply,
                    s.piece.garment,
                    s.piece.slot,
                    if s.piece.mirrored { "mirror" } else { "normal" }
                )
            },
        ),
    ]
}
fn escape(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_graphic() || c == ' ' {
                c
            } else {
                '?'
            }
        })
        .collect::<String>()
        .replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
}
const PT: f64 = 72.0 / 25.4;
// Conservative upper bounds in Helvetica ems. Wide identifiers must not overflow
// the label just because average Latin character widths are much smaller.
fn text_width_ems(text: &str) -> f64 {
    text.chars()
        .map(|c| match c {
            'W' | '@' | '%' => 1.0,
            'M' | 'm' => 0.85,
            'w' => 0.80,
            'i' | 'l' | 'I' | '.' | ',' | ':' | ';' | '!' | '\'' | '|' => 0.30,
            ' ' => 0.28,
            '(' | ')' | '[' | ']' | '{' | '}' => 0.35,
            '0'..='9' => 0.56,
            'A'..='Z' => 0.78,
            'a'..='z' => 0.60,
            _ => 0.70,
        })
        .sum::<f64>()
        .max(0.70)
}
fn render(labels: Vec<(ScanPayload, Vec<String>)>, paper: TicketPaper) -> Result<Vec<u8>> {
    ensure!(
        !labels.is_empty() && labels.len() <= 100_000,
        "print job needs 1 to 100,000 labels"
    );
    let (width, height, per_page, cols, margin) = match paper {
        TicketPaper::A4 => (210.0, 297.0, 10, 2, 5.0),
        TicketPaper::Label100x50 => (100.0, 50.0, 1, 1, 0.0),
    };
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        vec![],
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
    ];
    let mut kids = String::new();
    for page_labels in labels.chunks(per_page) {
        let mut content = String::from("0 g 0 G 0.3 w\n");
        for (index, (payload, lines)) in page_labels.iter().enumerate() {
            let x = margin + (index % cols) as f64 * 100.0;
            let y = height - margin - (index / cols + 1) as f64 * 50.0;
            writeln!(
                content,
                "{} {} {} {} re S",
                x * PT,
                y * PT,
                100.0 * PT,
                50.0 * PT
            )?;
            let encoded = payload.encode()?;
            let qr = QrCode::with_error_correction_level(encoded.as_bytes(), EcLevel::M)?;
            let side = qr.width();
            let module = 36.0 / (side + 8) as f64;
            ensure!(
                module >= 0.30,
                "QR modules below 0.30 mm: shorten identifiers before printing"
            );
            // Four white modules on each side are the QR quiet zone.
            for row in 0..side {
                for col in 0..side {
                    if qr[(col, row)] == Color::Dark {
                        writeln!(
                            content,
                            "{:.5} {:.5} {:.5} {:.5} re f",
                            (x + 3.0 + (col + 4) as f64 * module) * PT,
                            (y + 7.0 + (side - row + 3) as f64 * module) * PT,
                            module * PT,
                            module * PT
                        )?;
                    }
                }
            }
            for (line, text) in lines.iter().enumerate() {
                let display = escape(text);
                let font =
                    (49.0 * PT / text_width_ems(text)).min(if line == 0 { 11.0 } else { 9.0 });
                ensure!(
                    font >= 4.5,
                    "label text too long: shorten identifiers before printing"
                );
                writeln!(
                    content,
                    "BT /F1 {font:.3} Tf {} {} Td ({display}) Tj ET",
                    (x + 43.0) * PT,
                    (y + 43.0 - line as f64 * 4.8) * PT
                )?;
            }
        }
        let page = objects.len() + 1;
        write!(kids, "{page} 0 R ")?;
        objects.push(format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << /Font << /F1 3 0 R >> >> /Contents {} 0 R >>",width*PT,height*PT,page+1).into_bytes());
        // Compress each independent page stream. Geometry and scan payloads
        // stay vector/exact; large piece-sticker batches remain practical to open.
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(content.as_bytes())?;
        let compressed = encoder.finish()?;
        let mut stream = format!(
            "<< /Length {} /Filter /FlateDecode >>\nstream\n",
            compressed.len()
        )
        .into_bytes();
        stream.extend_from_slice(&compressed);
        stream.extend_from_slice(b"\nendstream");
        objects.push(stream);
    }
    objects[1] = format!(
        "<< /Type /Pages /Count {} /Kids [{kids}] >>",
        labels.len().div_ceil(per_page)
    )
    .into_bytes();
    let mut out = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = vec![0];
    for (i, obj) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(obj);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    let mut tail = format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len());
    for offset in offsets.iter().skip(1) {
        ensure!((*offset as u64) <= 9_999_999_999u64, "PDF exceeds xref limit");
        writeln!(tail, "{offset:010} 00000 n ")?;
    }
    write!(
        tail,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        offsets.len()
    )?;
    out.extend_from_slice(tail.as_bytes());
    Ok(out)
}
