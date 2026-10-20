//! Versioned, bounded CSV/XLSX intake. Preview is separate from committing records.
use anyhow::{Context, Result, bail, ensure};
use beta_marker_roll::{Roll, RollRegister};
use calamine::{Data, Reader, Xlsx};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Cursor, Read},
    str::FromStr,
};

/// Supported intake tables. Orders use one row per PO/style/colour/size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// Purchase-order garment quantities, separated by style, colour and size.
    Orders,
    /// Measured receiving roll packing list.
    Rolls,
}
impl FromStr for Kind {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "orders" => Ok(Self::Orders),
            "rolls" => Ok(Self::Rolls),
            _ => bail!("Choose orders or rolls."),
        }
    }
}
impl Kind {
    /// Stable version-1 column names and explicit units.
    pub fn columns(self) -> &'static [&'static str] {
        match self {
            Self::Orders => &[
                "schema_version",
                "external_id",
                "purchase_order",
                "buyer",
                "style",
                "colour",
                "size",
                "quantity",
                "due_date",
            ],
            Self::Rolls => &[
                "schema_version",
                "id",
                "lot",
                "dye_lot",
                "shade_group",
                "ticket_length_m",
                "measured_length_m",
                "usable_width_mm",
                "length_pct",
                "width_pct",
            ],
        }
    }
    fn sheet(self) -> &'static str {
        match self {
            Self::Orders => "Orders",
            Self::Rolls => "Rolls",
        }
    }
}

/// One immutable external purchase-order line. Quantity counts garments, not parts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderLine {
    /// Stable external purchase-order line identity.
    pub external_id: String,
    /// Factory or buyer purchase-order reference.
    pub purchase_order: String,
    /// Buyer name or reference.
    pub buyer: String,
    /// Garment style reference.
    pub style: String,
    /// Fabric colour; never combined across colours.
    pub colour: String,
    /// Garment size label, preserved as text.
    pub size: String,
    /// Whole garment count for this order line.
    pub quantity: u32,
    /// ISO calendar date, YYYY-MM-DD; empty means unscheduled.
    pub due_date: String,
}
/// Cell-specific preview error. Row 1 contains column names.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowError {
    /// One-based source row; header is row 1.
    pub row: usize,
    /// Canonical source column, or row for a record-level error.
    pub column: String,
    /// Plain-English validation message.
    pub message: String,
}
/// A reviewable import result. Any errors block the entire commit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preview {
    /// Selected version-1 intake table.
    pub kind: Kind,
    /// Original source header names.
    pub headers: Vec<String>,
    /// Source data rows or emitted instruction sequence.
    pub rows: Vec<Vec<String>>,
    /// Blocking row/header errors; no partial commit is allowed.
    pub errors: Vec<RowError>,
    /// Received purchase-order lines with stable external IDs.
    pub orders: Vec<OrderLine>,
    /// Measured receiving stock and inspection records.
    pub register: RollRegister,
}
/// Canonical column name -> source header. Omitted names match literally.
pub type Mapping = BTreeMap<String, String>;

pub(crate) fn label(value: &str, name: &str) -> Result<()> {
    ensure!(
        !value.trim().is_empty() && value.len() <= 256 && !value.chars().any(char::is_control),
        "{name} must contain 1 to 256 bytes without control characters."
    );
    Ok(())
}
/// Validate an actual calendar date, including leap years.
pub fn date(value: &str) -> Result<()> {
    let bytes = value.as_bytes();
    ensure!(
        bytes.len() == 10
            && bytes[4] == b'-'
            && bytes[7] == b'-'
            && bytes
                .iter()
                .enumerate()
                .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit()),
        "Use a date in YYYY-MM-DD form."
    );
    let y: u32 = value[..4].parse()?;
    let m: usize = value[5..7].parse()?;
    let d: u32 = value[8..].parse()?;
    let days = [
        31,
        if y.is_multiple_of(400) || (y.is_multiple_of(4) && !y.is_multiple_of(100)) {
            29
        } else {
            28
        },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    ensure!(
        y >= 1900 && (1..=12).contains(&m) && d > 0 && d <= days[m - 1],
        "Date is outside the calendar."
    );
    Ok(())
}
impl OrderLine {
    /// Check identifiers, quantity limits and an optional delivery date.
    pub fn validate(&self) -> Result<()> {
        for (name, v) in [
            ("external id", &self.external_id),
            ("purchase order", &self.purchase_order),
            ("buyer", &self.buyer),
            ("style", &self.style),
            ("colour", &self.colour),
            ("size", &self.size),
        ] {
            label(v, name)?;
        }
        ensure!(
            self.quantity > 0 && self.quantity <= 1_000_000,
            "Quantity must be a whole garment count from 1 to 1000000."
        );
        if !self.due_date.is_empty() {
            date(&self.due_date)?;
        }
        Ok(())
    }
}

fn xlsx_preflight(bytes: &[u8]) -> Result<()> {
    let mut zip =
        zip::ZipArchive::new(Cursor::new(bytes)).context("This is not an XLSX workbook.")?;
    ensure!(zip.len() <= 200, "Workbook has too many archive entries.");
    let mut total = 0u64;
    for i in 0..zip.len() {
        let mut f = zip.by_index(i)?;
        total = total
            .checked_add(f.size())
            .context("Workbook size overflow.")?;
        ensure!(total <= 20_000_000, "Expanded workbook exceeds 20 MB.");
        if !f.name().starts_with("xl/worksheets/") || !f.name().ends_with(".xml") {
            continue;
        }
        let mut xml = Vec::new();
        (&mut f).take(20_000_001).read_to_end(&mut xml)?;
        ensure!(xml.len() <= 20_000_000, "Expanded worksheet exceeds 20 MB.");
        let mut reader = quick_xml::Reader::from_reader(xml.as_slice());
        loop {
            match reader.read_event()? {
                quick_xml::events::Event::Start(e) | quick_xml::events::Event::Empty(e) => {
                    for a in e.attributes() {
                        let a = a?;
                        if e.name().as_ref() == b"row" && a.key.as_ref() == b"r" {
                            let row: u32 = std::str::from_utf8(&a.value)?.parse()?;
                            ensure!(
                                (1..=10001).contains(&row),
                                "Workbook row exceeds the import limit."
                            );
                        }
                        if (e.name().as_ref() == b"c" && a.key.as_ref() == b"r")
                            || (e.name().as_ref() == b"dimension" && a.key.as_ref() == b"ref")
                        {
                            let reference = std::str::from_utf8(&a.value)?;
                            for cell in reference.split(':') {
                                let split = cell
                                    .find(|c: char| c.is_ascii_digit())
                                    .context("Invalid spreadsheet cell address.")?;
                                let mut col = 0u32;
                                for c in cell[..split].bytes() {
                                    ensure!(c.is_ascii_uppercase(), "Invalid spreadsheet column.");
                                    col = col
                                        .checked_mul(26)
                                        .and_then(|v| v.checked_add(u32::from(c - b'A' + 1)))
                                        .context("Column overflow.")?;
                                }
                                let row: u32 = cell[split..].parse()?;
                                ensure!(
                                    (1..=64).contains(&col) && (1..=10001).contains(&row),
                                    "Workbook cells must stay within 10001 rows and 64 columns."
                                );
                            }
                        }
                    }
                }
                quick_xml::events::Event::Eof => break,
                _ => {}
            }
        }
    }
    Ok(())
}

fn read_table(bytes: &[u8], format: &str, kind: Kind) -> Result<Vec<Vec<String>>> {
    ensure!(bytes.len() <= 5_000_000, "Import exceeds 5 MB.");
    let rows: Vec<Vec<String>> = match format {
        "csv" => {
            let text = std::str::from_utf8(bytes)
                .context("CSV must use UTF-8. In Excel choose CSV UTF-8.")?
                .trim_start_matches('\u{feff}');
            let mut reader = csv::ReaderBuilder::new()
                .has_headers(false)
                .flexible(true)
                .from_reader(text.as_bytes());
            reader
                .records()
                .take(10002)
                .map(|r| Ok(r?.iter().map(|s| s.trim().to_owned()).collect()))
                .collect::<Result<Vec<_>>>()?
        }
        "xlsx" => {
            xlsx_preflight(bytes)?;
            let mut book: Xlsx<_> = Xlsx::new(Cursor::new(bytes))?;
            ensure!(
                book.sheet_names().iter().any(|s| s == kind.sheet()),
                "Workbook needs a sheet named {}.",
                kind.sheet()
            );
            let formulas = book.worksheet_formula(kind.sheet())?;
            ensure!(
                !formulas.cells().any(|(_, _, s)| !s.is_empty()),
                "Formulas are not accepted. Paste values into the template."
            );
            let range = book.worksheet_range(kind.sheet())?;
            ensure!(
                range.start() == Some((0, 0)),
                "Place the template headers in row 1, starting at cell A1."
            );
            ensure!(
                range.height() <= 10001 && range.width() <= 64,
                "Table exceeds 10000 data rows or 64 columns."
            );
            range
                .rows()
                .map(|r| {
                    r.iter()
                        .map(|c| match c {
                            Data::Empty => Ok(String::new()),
                            Data::String(s) => Ok(s.trim().to_owned()),
                            Data::Int(n) => Ok(n.to_string()),
                            Data::Float(n) if n.is_finite() => Ok(n.to_string()),
                            _ => bail!(
                                "Use text, numbers and YYYY-MM-DD text dates in the template."
                            ),
                        })
                        .collect()
                })
                .collect::<Result<Vec<_>>>()?
        }
        _ => bail!("Choose a UTF-8 CSV or XLSX template. Legacy XLS is not supported."),
    };
    ensure!(
        (2..=10001).contains(&rows.len())
            && rows
                .iter()
                .all(|r| r.len() <= 64 && r.iter().all(|v| v.len() <= 4096)),
        "Import needs 1 to 10000 data rows; at most 64 columns and 4096 bytes per cell."
    );
    Ok(rows)
}

/// Read, map and validate a file. Row errors remain visible and do not import anything.
pub fn preview(bytes: &[u8], format: &str, kind: Kind, mapping: &Mapping) -> Result<Preview> {
    let mut rows = read_table(bytes, format, kind)?;
    let headers = rows.remove(0);
    let mut out = Preview {
        kind,
        headers,
        rows,
        errors: vec![],
        orders: vec![],
        register: RollRegister::default(),
    };
    let mut indices = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for col in kind.columns() {
        let name = mapping.get(*col).map_or(*col, String::as_str);
        let matches: Vec<_> = out
            .headers
            .iter()
            .enumerate()
            .filter(|(_, v)| v.as_str() == name)
            .map(|(i, _)| i)
            .collect();
        if matches.len() != 1 || !seen.insert(matches.first().copied()) {
            out.errors.push(RowError {
                row: 1,
                column: (*col).into(),
                message: format!("Map {col} to one unique source column (currently {name})."),
            });
        } else {
            indices.insert(*col, matches[0]);
        }
    }
    if !out.errors.is_empty() {
        return Ok(out);
    }
    let mut identities = BTreeSet::new();
    for (i, row) in out.rows.iter().enumerate() {
        let mut errors = Vec::new();
        if row.len() != out.headers.len() {
            errors.push(RowError {
                row: i + 2,
                column: "row".into(),
                message: "Wrong number of cells.".into(),
            });
        }
        let value = |k: &str| -> String { row.get(indices[k]).cloned().unwrap_or_default() };
        if value("schema_version") != "1" {
            errors.push(RowError {
                row: i + 2,
                column: "schema_version".into(),
                message: "Schema version must be 1.".into(),
            });
        }
        let id_key = if kind == Kind::Orders {
            "external_id"
        } else {
            "id"
        };
        if !identities.insert(value(id_key)) {
            errors.push(RowError {
                row: i + 2,
                column: id_key.into(),
                message: "Duplicate identity in this file.".into(),
            });
        }
        let parsed = (|| -> Result<()> {
            match kind {
                Kind::Orders => {
                    let raw = value("quantity");
                    let quantity = raw
                        .parse::<u32>()
                        .context("quantity: enter a positive whole garment count.")?;
                    let order = OrderLine {
                        external_id: value("external_id"),
                        purchase_order: value("purchase_order"),
                        buyer: value("buyer"),
                        style: value("style"),
                        colour: value("colour"),
                        size: value("size"),
                        quantity,
                        due_date: value("due_date"),
                    };
                    order.validate()?;
                    out.orders.push(order);
                }
                Kind::Rolls => {
                    let n = |k: &str| -> Result<f64> {
                        let v: f64 = value(k)
                            .parse()
                            .with_context(|| format!("{k}: enter a number in the stated unit."))?;
                        ensure!(v.is_finite(), "{k}: enter a finite number.");
                        Ok(v)
                    };
                    let roll = Roll {
                        id: value("id"),
                        lot: value("lot"),
                        dye_lot: value("dye_lot"),
                        shade_group: value("shade_group"),
                        ticket_length_m: n("ticket_length_m")?,
                        measured_length_m: n("measured_length_m")?,
                        usable_width_mm: n("usable_width_mm")?,
                        shrinkage: beta_marker_core::Shrinkage {
                            length_pct: n("length_pct")?,
                            width_pct: n("width_pct")?,
                        },
                        defects: vec![],
                    };
                    for s in [&roll.id, &roll.lot, &roll.dye_lot, &roll.shade_group] {
                        label(s, "roll identity")?;
                    }
                    RollRegister {
                        rolls: vec![roll.clone()],
                    }
                    .validate()?;
                    out.register.rolls.push(roll);
                }
            }
            Ok(())
        })();
        if let Err(e) = parsed {
            errors.push(RowError {
                row: i + 2,
                column: "row".into(),
                message: e.to_string(),
            });
        }
        out.errors.extend(errors);
    }
    Ok(out)
}

/// Guard spreadsheet CSV exports against formula interpretation while preserving text.
pub fn csv_text(value: &str) -> String {
    if value.trim_start().starts_with(['=', '+', '-', '@']) || value.starts_with(['\t', '\r']) {
        format!("'{value}")
    } else {
        value.into()
    }
}
/// Encode system tables as quoted UTF-8 CSV with CRLF record separators.
pub fn csv_table(headers: &[&str], rows: &[Vec<String>]) -> Result<Vec<u8>> {
    csv_numeric_table(headers, rows, &[])
}
/// Keep explicitly typed finite numeric columns numeric, including negative variance.
/// Identifier columns still receive spreadsheet formula protection.
pub fn csv_numeric_table(
    headers: &[&str],
    rows: &[Vec<String>],
    numeric: &[usize],
) -> Result<Vec<u8>> {
    let mut w = csv::WriterBuilder::new()
        .terminator(csv::Terminator::CRLF)
        .from_writer(vec![]);
    w.write_record(headers)?;
    for row in rows {
        let mut cells = Vec::new();
        for (i, v) in row.iter().enumerate() {
            if numeric.contains(&i) {
                ensure!(
                    v.is_empty() || v.parse::<f64>().is_ok_and(f64::is_finite),
                    "CSV numeric column contains an invalid value."
                );
                cells.push(v.clone());
            } else {
                cells.push(csv_text(v));
            }
        }
        w.write_record(cells)?;
    }
    Ok(w.into_inner()?)
}
/// Export an editable, original template. XLSX cells are literal text, never formulas.
pub fn template(kind: Kind, format: &str) -> Result<Vec<u8>> {
    let example = match kind {
        Kind::Orders => vec![
            "1",
            "PO1-M-BLUE",
            "PO1",
            "Sample buyer",
            "TEE",
            "Blue",
            "M",
            "20",
            "2026-10-20",
        ],
        Kind::Rolls => vec![
            "1", "R1", "LOT1", "DYE1", "A", "100", "100", "1500", "0", "0",
        ],
    }
    .into_iter()
    .map(String::from)
    .collect::<Vec<_>>();
    match format {
        "csv" => csv_table(kind.columns(), &[example]),
        "xlsx" => {
            let mut w = rust_xlsxwriter::Workbook::new();
            let s = w.add_worksheet();
            s.set_name(kind.sheet())?;
            let heading = rust_xlsxwriter::Format::new()
                .set_bold()
                .set_background_color("DCEBE3");
            for (i, h) in kind.columns().iter().enumerate() {
                s.write_string_with_format(0, i as u16, *h, &heading)?;
                s.write_string(1, i as u16, &example[i])?;
                s.set_column_width(i as u16, 22)?;
            }
            s.set_freeze_panes(1, 0)?;
            let s = w.add_worksheet();
            s.set_name("Read me")?;
            s.set_column_width(0, 110)?;
            for (i,line) in ["BETA floor template v1. Replace the sample row. Keep column names and schema_version = 1.","One order row per PO, style, colour and size. external_id must be stable and unique.","Roll lengths are metres; usable width is millimetres in the laid-flat cutting frame.","Dates are YYYY-MM-DD text. Shrinkage is percentage points. Do not use formulas.","Preview every import. Identical IDs and contents are safe to replay; changed IDs are conflicts.","Packing lists do not certify inspection. Review measured length, defects, shade and shrinkage in Rolls."].iter().enumerate() { s.write_string(i as u32,0,*line)?; }
            Ok(w.save_to_buffer()?)
        }
        _ => bail!("Template format must be csv or xlsx."),
    }
}
