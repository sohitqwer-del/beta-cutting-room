//! Minimal dependency-free XLSX writer for small reports: stored (uncompressed) ZIP,
//! inline strings, numbers with two decimals, bold frozen header row.
//! Strings are never formulas, so user text cannot execute in a spreadsheet.
use anyhow::{Result, ensure};

/// One worksheet cell.
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    /// Literal text.
    Text(String),
    /// Number shown with two decimals.
    Number(f64),
    /// Empty cell.
    Empty,
}

/// Worksheet: name, header row and body rows.
pub struct Sheet<'a> {
    /// Tab name; at most 31 characters and no `[]:*?/\`.
    pub name: &'a str,
    /// Header labels.
    pub headers: Vec<String>,
    /// Body rows, each as long as the header.
    pub rows: Vec<Vec<Cell>>,
}

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            // XML 1.0 forbids most control characters.
            '\t' | '\n' | '\r' => out.push(c),
            c if (c as u32) < 0x20 => {}
            c => out.push(c),
        }
    }
    out
}

fn column(mut i: usize) -> String {
    let mut s = String::new();
    loop {
        s.insert(0, (b'A' + (i % 26) as u8) as char);
        if i < 26 {
            return s;
        }
        i = i / 26 - 1;
    }
}

fn sheet_xml(sheet: &Sheet<'_>) -> Result<String> {
    let width = sheet.headers.len();
    ensure!(
        width > 0 && width <= 16_384,
        "Report table exceeds worksheet limits."
    );
    let mut x = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetViews><sheetView workbookViewId="0"><pane ySplit="1" topLeftCell="A2" activePane="bottomLeft" state="frozen"/></sheetView></sheetViews><cols>"#,
    );
    x.push_str(&format!(
        r#"<col min="1" max="{width}" width="24" customWidth="1"/></cols><sheetData>"#
    ));
    let header: Vec<Cell> = sheet.headers.iter().cloned().map(Cell::Text).collect();
    for (r, row) in std::iter::once(&header).chain(&sheet.rows).enumerate() {
        ensure!(
            row.len() == width,
            "Report row has the wrong number of cells."
        );
        x.push_str(&format!(r#"<row r="{}">"#, r + 1));
        for (c, cell) in row.iter().enumerate() {
            let at = format!("{}{}", column(c), r + 1);
            let style = if r == 0 { 1 } else { 0 };
            match cell {
                Cell::Text(t) => x.push_str(&format!(
                    r#"<c r="{at}" s="{style}" t="inlineStr"><is><t xml:space="preserve">{}</t></is></c>"#,
                    escape(t)
                )),
                Cell::Number(n) if n.is_finite() => {
                    x.push_str(&format!(r#"<c r="{at}" s="2"><v>{n}</v></c>"#))
                }
                Cell::Number(_) | Cell::Empty => {}
            }
        }
        x.push_str("</row>");
    }
    x.push_str("</sheetData></worksheet>");
    Ok(x)
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & 0u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}

/// Stored ZIP archive. Fixed 1980-01-01 timestamps keep output reproducible.
fn zip(files: &[(String, Vec<u8>)]) -> Result<Vec<u8>> {
    let mut out = vec![];
    let mut central = vec![];
    for (name, data) in files {
        ensure!(
            out.len() < u32::MAX as usize / 2 && data.len() < u32::MAX as usize / 2,
            "Report is too large."
        );
        let (crc, size, offset) = (crc32(data), data.len() as u32, out.len() as u32);
        let header = |sig: u32, central: bool| {
            let mut h = sig.to_le_bytes().to_vec();
            if central {
                h.extend(20u16.to_le_bytes()); // version made by
            }
            for v in [20u16, 0x0800, 0, 0, 0x21] {
                h.extend(v.to_le_bytes()); // version, UTF-8 names, stored, time, date
            }
            h.extend(crc.to_le_bytes());
            h.extend(size.to_le_bytes());
            h.extend(size.to_le_bytes());
            h.extend((name.len() as u16).to_le_bytes());
            h.extend(0u16.to_le_bytes());
            if central {
                h.extend([0u8; 6]); // comment length, disk, internal attributes
                h.extend(0u32.to_le_bytes()); // external attributes
                h.extend(offset.to_le_bytes());
            }
            h.extend(name.as_bytes());
            h
        };
        out.extend(header(0x0403_4b50, false));
        out.extend(data);
        central.extend(header(0x0201_4b50, true));
    }
    let (start, len) = (out.len() as u32, central.len() as u32);
    out.extend(central);
    out.extend(0x0605_4b50u32.to_le_bytes());
    out.extend([0u8; 4]);
    out.extend((files.len() as u16).to_le_bytes());
    out.extend((files.len() as u16).to_le_bytes());
    out.extend(len.to_le_bytes());
    out.extend(start.to_le_bytes());
    out.extend(0u16.to_le_bytes());
    Ok(out)
}

/// Write a workbook with one worksheet per table.
pub fn workbook(sheets: &[Sheet<'_>]) -> Result<Vec<u8>> {
    ensure!(
        !sheets.is_empty() && sheets.len() <= 255,
        "At least one report table is required."
    );
    for s in sheets {
        ensure!(
            !s.name.is_empty()
                && s.name.chars().count() <= 31
                && !s.name.contains(['[', ']', ':', '*', '?', '/', '\\']),
            "Invalid worksheet name."
        );
    }
    let n = sheets.len();
    let mut types = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>"#,
    );
    let mut book = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets>"#,
    );
    let mut rels = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    );
    for (i, s) in sheets.iter().enumerate() {
        let k = i + 1;
        types.push_str(&format!(r#"<Override PartName="/xl/worksheets/sheet{k}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>"#));
        book.push_str(&format!(
            r#"<sheet name="{}" sheetId="{k}" r:id="rId{k}"/>"#,
            escape(s.name)
        ));
        rels.push_str(&format!(r#"<Relationship Id="rId{k}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet{k}.xml"/>"#));
    }
    types.push_str("</Types>");
    book.push_str("</sheets></workbook>");
    rels.push_str(&format!(r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/></Relationships>"#, n + 1));
    let styles = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><numFmts count="1"><numFmt numFmtId="164" formatCode="0.00"/></numFmts><fonts count="2"><font><sz val="11"/><name val="Calibri"/></font><font><b/><sz val="11"/><name val="Calibri"/></font></fonts><fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills><borders count="1"><border/></borders><cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs><cellXfs count="3"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/><xf numFmtId="0" fontId="1" fillId="0" borderId="0" xfId="0" applyFont="1"/><xf numFmtId="164" fontId="0" fillId="0" borderId="0" xfId="0" applyNumberFormat="1"/></cellXfs><cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles></styleSheet>"#;
    let root = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#;
    let mut files = vec![
        ("[Content_Types].xml".to_string(), types.into_bytes()),
        ("_rels/.rels".to_string(), root.as_bytes().to_vec()),
        ("xl/workbook.xml".to_string(), book.into_bytes()),
        ("xl/_rels/workbook.xml.rels".to_string(), rels.into_bytes()),
        ("xl/styles.xml".to_string(), styles.as_bytes().to_vec()),
    ];
    for (i, s) in sheets.iter().enumerate() {
        files.push((
            format!("xl/worksheets/sheet{}.xml", i + 1),
            sheet_xml(s)?.into_bytes(),
        ));
    }
    zip(&files)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn columns_crc_and_archive_shape() {
        assert_eq!(column(0), "A");
        assert_eq!(column(25), "Z");
        assert_eq!(column(26), "AA");
        assert_eq!(column(701), "ZZ");
        assert_eq!(column(702), "AAA");
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
        let bytes = workbook(&[Sheet {
            name: "Data",
            headers: vec!["A&B".into(), "mm".into()],
            rows: vec![vec![Cell::Text("=1+1 <x>".into()), Cell::Number(12.5)]],
        }])
        .unwrap();
        assert!(bytes.starts_with(b"PK\x03\x04"));
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("A&amp;B") && text.contains("=1+1 &lt;x&gt;"));
        assert!(text.contains(r#"<c r="B2" s="2"><v>12.5</v></c>"#));
        assert!(!text.contains("<f>"));
        assert!(
            workbook(&[Sheet {
                name: "a/b",
                headers: vec!["x".into()],
                rows: vec![]
            }])
            .is_err()
        );
    }
}
