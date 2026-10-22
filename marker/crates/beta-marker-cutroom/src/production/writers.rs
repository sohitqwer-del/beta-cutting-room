//! Spreadsheet, exchange and printable views of the same computed report pack.
use super::*;
use rust_xlsxwriter::{Color, Format, Formula, Workbook};
use std::fmt::Write;

#[derive(Clone)]
enum Cell {
    Text(String),
    Number(f64),
    Empty,
}
impl Cell {
    fn text(&self) -> String {
        match self {
            Self::Text(s) => s.clone(),
            Self::Number(n) => format!("{n:.6}")
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string(),
            Self::Empty => String::new(),
        }
    }
}
fn t(s: impl Into<String>) -> Cell {
    Cell::Text(s.into())
}
fn n(v: f64) -> Cell {
    Cell::Number(v)
}
fn optional(v: Option<f64>) -> Cell {
    v.map_or(Cell::Empty, n)
}
struct Table {
    name: String,
    headers: Vec<String>,
    rows: Vec<Vec<Cell>>,
}
fn table(name: &str, headers: &[&str], rows: Vec<Vec<Cell>>) -> Table {
    Table {
        name: name.into(),
        headers: headers.iter().map(|s| s.to_string()).collect(),
        rows,
    }
}
fn tables(p: &ReportPack) -> Vec<Table> {
    let c = &p.costing;
    let mut cost = vec![
        vec![t("Order"), t(&p.order), t("")],
        vec![t("PO"), t(&p.meta.po), t("")],
        vec![t("Buyer"), t(&p.meta.buyer), t("")],
        vec![t("Style / revision"), t(&p.meta.style), t("")],
        vec![t("Fabric"), t(&p.meta.fabric), t("")],
        vec![t("Colour"), t(&p.meta.colour), t("")],
        vec![t("Date"), t(&p.meta.date), t("")],
        vec![t("Prepared by"), t(&p.meta.prepared_by), t("")],
        vec![t("Approved by"), t(&p.meta.approved_by), t("")],
        vec![t("Usable laid-flat width"), n(c.usable_width_mm), t("mm")],
        vec![t("Width / laid-flat dia"), n(c.width_inches), t("inch")],
        vec![t("Physical width for mass"), n(c.mass_width_mm), t("mm")],
        vec![t("GSM"), n(c.gsm), t("g/m2")],
        vec![
            t("Ordered garments"),
            n(c.ordered_garments as f64),
            t("pieces"),
        ],
        vec![
            t("Planned garments"),
            n(c.planned_garments as f64),
            t("pieces"),
        ],
        vec![t("Net marker fabric"), n(c.net_marker_m), t("m")],
        vec![t("Head/tail end loss"), n(c.end_loss_m), t("m")],
        vec![t("Splice overlap loss"), n(c.splice_loss_m), t("m")],
        vec![t("Consumed fabric"), n(c.consumed_m), t("m")],
        vec![
            t("Purchase wastage"),
            n(c.purchase_wastage_pct),
            t("% of consumption"),
        ],
        vec![t("Purchase wastage cloth"), n(c.purchase_wastage_m), t("m")],
        vec![t("Gross purchase fabric"), n(c.gross_purchase_m), t("m")],
        vec![
            t("Total extra over net"),
            n(c.total_extra_pct_of_net),
            t("% of net marker"),
        ],
        vec![t("Total order cost"), n(c.total_cost), t(&p.currency)],
        vec![
            t("Cost per ordered garment"),
            n(c.cost_per_ordered_piece),
            t(format!("{}/piece", p.currency)),
        ],
    ];
    for (label, consumption) in [
        ("Net / planned garment", Some(c.net)),
        ("Gross / ordered garment", Some(c.gross)),
        ("Actual / good garment", c.actual),
    ] {
        for (unit, value) in [
            ("m/piece", consumption.map(|x| x.m_per_piece)),
            ("m/dozen", consumption.map(|x| x.m_per_dozen)),
            ("kg/piece", consumption.map(|x| x.kg_per_piece)),
            ("kg/dozen", consumption.map(|x| x.kg_per_dozen)),
            ("yd/piece", consumption.map(|x| x.yards_per_piece)),
        ] {
            cost.push(vec![t(label), optional(value), t(unit)]);
        }
    }
    cost.push(vec![
        t("Fabric purchase price"),
        n(c.price_per_m),
        t(format!("{}/m", p.currency)),
    ]);
    cost.push(vec![
        t("Lay starting price"),
        n(c.price_per_lay),
        t(format!("{}/lay", p.currency)),
    ]);
    cost.push(vec![t("Number of lays"), n(p.lays.len() as f64), t("lays")]);
    cost.push(vec![
        t("Physical width layers"),
        n(f64::from(c.fabric_layers)),
        t("1 open; 2 folded/tubular"),
    ]);
    cost.push(vec![
        t("Purchase laid-flat width"),
        n(c.mass_width_mm / f64::from(c.fabric_layers)),
        t("mm; stock-use weighted"),
    ]);
    let lays = p
        .lays
        .iter()
        .map(|l| {
            vec![
                t(&l.lay),
                t(&l.marker),
                n(f64::from(l.plies)),
                n(l.marker_m),
                n(l.head_m),
                n(l.tail_m),
                t(format_ratio(&l.ratio)),
                t(format_ratio(&l.quantities)),
                t(l.rolls.join(", ")),
                t(shades_text(&l.shades)),
                n(l.recoverable_ends_m),
            ]
        })
        .collect();
    let mut widths = Vec::new();
    if let Some(comparison) = &p.widths {
        for (i, w) in comparison.trials.iter().enumerate() {
            widths.push(vec![
                t(&w.offer_id),
                n(w.width_mm / 25.4),
                t(&w.ratio_set),
                t(if comparison.chosen == Some(i) {
                    "SELECTED"
                } else if w.final_evaluated {
                    "final"
                } else {
                    "short only"
                }),
                t(w.feasible.to_string()),
                optional(w.cost.as_ref().map(|c| c.consumed_m)),
                optional(w.cost.as_ref().map(|c| c.recoverable_end_bits_m)),
                optional(w.cost.as_ref().map(|c| c.total)),
                t(w.error.as_deref().unwrap_or("")),
            ]);
        }
    }
    let rolls = p
        .rolls
        .iter()
        .map(|r| {
            vec![
                t(&r.roll),
                t(&r.lot),
                t(&r.dye_lot),
                t(&r.shade),
                n(r.width_inches),
                n(r.initial_m),
                optional(r.planned_used_m),
                optional(r.planned_end_bit_m),
                optional(r.issued_m),
                optional(r.returned_m),
                optional(r.actual_used_m),
                optional(r.scrap_m),
                n(r.balance_m),
            ]
        })
        .collect();
    let reconcile = p
        .reconciliation
        .iter()
        .map(|r| {
            vec![
                t(&r.size),
                n(f64::from(r.ordered)),
                n(f64::from(r.planned)),
                optional(r.cut.map(f64::from)),
                n(r.planned_balance as f64),
                optional(r.cut_balance.map(|n| n as f64)),
            ]
        })
        .collect();
    let mut notes = p.notes.iter().map(|s| vec![t(s)]).collect::<Vec<_>>();
    if let Some(w) = &p.widths {
        notes.push(vec![t(&w.reason)]);
    }
    vec![
        table("Costing", &["Field", "Value", "Unit / denominator"], cost),
        table(
            "Lays",
            &[
                "Lay",
                "Marker",
                "Plies",
                "Marker m",
                "Head m/ply",
                "Tail m/ply",
                "Ratio",
                "Planned size output",
                "Rolls",
                "Lot / dye / shade",
                "Recoverable ends m",
            ],
            lays,
        ),
        table(
            "Widths",
            &[
                "Offer",
                "Width inch",
                "Ratio set",
                "Stage",
                "Feasible",
                "Consumed m",
                "Recoverable ends m",
                &format!("Cost {}", p.currency),
                "Failure",
            ],
            widths,
        ),
        table(
            "Roll register",
            &[
                "Roll",
                "Lot",
                "Dye lot",
                "Shade",
                "Width inch",
                "Measured m",
                "Planned used m",
                "Planned ends m",
                "Actual issued m",
                "Actual returned m",
                "Actual used m",
                "Actual scrap m",
                "Balance m",
            ],
            rolls,
        ),
        table(
            "Reconciliation",
            &[
                "Size",
                "Ordered",
                "Planned",
                "Good cut",
                "Planned - ordered",
                "Cut - ordered",
            ],
            reconcile,
        ),
        table("Notes", &["Scope / assumptions"], notes),
    ]
}
fn format_ratio(r: &BTreeMap<String, u32>) -> String {
    r.iter()
        .map(|(s, n)| format!("{s}:{n}"))
        .collect::<Vec<_>>()
        .join(" ")
}
fn shades_text(shades: &[ShadeIdentity]) -> String {
    shades
        .iter()
        .map(|s| format!("{:?}/{:?}/{:?}", s.lot, s.dye_lot, s.shade_group))
        .collect::<Vec<_>>()
        .join(", ")
}

/// A named CSV per report table. Identifier cells are quoted and formula-leading
/// text is prefixed with an apostrophe for spreadsheet-safe import. Numeric
/// columns remain numeric; JSON keeps exact identifiers without this escape.
pub fn csv_files(pack: &ReportPack) -> BTreeMap<String, String> {
    let field = |s: &str| {
        let dangerous = s
            .trim_start_matches(char::is_whitespace)
            .starts_with(['=', '+', '-', '@'])
            || s.starts_with(['\t', '\r', '\n']);
        let safe = if dangerous { format!("'{s}") } else { s.into() };
        format!("\"{}\"", safe.replace('"', "\"\""))
    };
    tables(pack)
        .into_iter()
        .map(|tab| {
            let mut out = tab
                .headers
                .iter()
                .map(|s| field(s))
                .collect::<Vec<_>>()
                .join(",");
            out.push_str("\r\n");
            for row in tab.rows {
                out.push_str(
                    &row.iter()
                        .map(|cell| match cell {
                            Cell::Text(s) => field(s),
                            Cell::Number(v) => v.to_string(),
                            Cell::Empty => String::new(),
                        })
                        .collect::<Vec<_>>()
                        .join(","),
                );
                out.push_str("\r\n");
            }
            (
                format!("{}.csv", tab.name.to_lowercase().replace(' ', "-")),
                out,
            )
        })
        .collect()
}

/// Editable XLSX with numeric cells, cached formula results, filters and frozen
/// headers. Identifiers are explicit strings, so formula injection is impossible.
pub fn xlsx(pack: &ReportPack) -> Result<Vec<u8>> {
    let mut book = Workbook::new();
    let heading = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0x17324d))
        .set_font_color(Color::White)
        .set_text_wrap();
    let text = Format::new()
        .set_text_wrap()
        .set_align(rust_xlsxwriter::FormatAlign::Top);
    let number = Format::new().set_num_format("0.######");
    for tab in tables(pack) {
        ensure!(
            tab.rows.len() < 1_048_576,
            "Report exceeds Excel's row limit."
        );
        let sheet = book.add_worksheet();
        sheet.set_name(&tab.name)?;
        sheet.set_freeze_panes(1, 0)?;
        sheet.set_landscape();
        sheet.set_paper_size(9);
        sheet.set_print_fit_to_pages(1, 0);
        for (col, h) in tab.headers.iter().enumerate() {
            sheet.write_string_with_format(0, col as u16, h, &heading)?;
            sheet.set_column_width(
                col as u16,
                if tab.name == "Notes" {
                    100.0
                } else if tab.name == "Costing" && col == 0 {
                    31.0
                } else {
                    22.0
                },
            )?;
        }
        sheet.set_row_height(0, 32.0)?;
        for (r, row) in tab.rows.iter().enumerate() {
            for (c, cell) in row.iter().enumerate() {
                match cell {
                    Cell::Text(s) => {
                        sheet.write_string_with_format(r as u32 + 1, c as u16, s, &text)?;
                    }
                    Cell::Number(n) => {
                        sheet.write_number_with_format(r as u32 + 1, c as u16, *n, &number)?;
                    }
                    Cell::Empty => {}
                }
            }
            let lines = row
                .iter()
                .enumerate()
                .map(|(c, cell)| match cell {
                    Cell::Text(s) => s.chars().count().div_ceil(if tab.name == "Notes" {
                        96
                    } else if tab.name == "Costing" && c == 0 {
                        29
                    } else {
                        20
                    }),
                    _ => 1,
                })
                .max()
                .unwrap_or(1);
            sheet.set_row_height(r as u32 + 1, (lines.max(1) as f64 * 14.0 + 4.0).min(409.0))?;
        }
        if !tab.rows.is_empty() {
            sheet.autofilter(0, 0, tab.rows.len() as u32, tab.headers.len() as u16 - 1)?;
        }
        if tab.name == "Costing" {
            // Formula cells refer to visible labelled inputs in this sheet.
            // Values are cached for readers that do not recalculate on open.
            let mut formula = |excel_row: u32, expression: &str, value: f64| -> Result<()> {
                sheet.write_formula_with_format(
                    excel_row - 1,
                    1,
                    Formula::new(expression).set_result(value.to_string()),
                    &number,
                )?;
                Ok(())
            };
            let c = &pack.costing;
            formula(12, "=B11/25.4", c.width_inches)?;
            formula(13, "=B46*B45", c.mass_width_mm)?;
            formula(20, "=SUM(B17:B19)", c.consumed_m)?;
            formula(22, "=B20*B21/100", c.purchase_wastage_m)?;
            formula(23, "=SUM(B20,B22)", c.gross_purchase_m)?;
            formula(24, "=(B23-B17)/B17*100", c.total_extra_pct_of_net)?;
            formula(25, "=B23*B42+B43*B44", c.total_cost)?;
            formula(26, "=B25/B15", c.cost_per_ordered_piece)?;
            for (start, metres, denominator, values) in
                [(27, "B17", "B16", c.net), (32, "B23", "B15", c.gross)]
            {
                formula(
                    start,
                    &format!("={metres}/{denominator}"),
                    values.m_per_piece,
                )?;
                formula(start + 1, &format!("=B{start}*12"), values.m_per_dozen)?;
                formula(
                    start + 2,
                    &format!("=B{start}*B13*B14/1000000"),
                    values.kg_per_piece,
                )?;
                formula(
                    start + 3,
                    &format!("=B{}*12", start + 2),
                    values.kg_per_dozen,
                )?;
                formula(
                    start + 4,
                    &format!("=B{start}/0.9144"),
                    values.yards_per_piece,
                )?;
            }
        }
    }
    Ok(book.save_to_buffer()?)
}

fn ascii(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii() && !c.is_control() {
                c
            } else if c.is_whitespace() {
                ' '
            } else {
                '?'
            }
        })
        .collect()
}
fn escaped(s: &str) -> String {
    ascii(s)
        .replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
}
fn wrap(s: &str, limit: usize) -> Vec<String> {
    let s = ascii(s);
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in s.split_whitespace() {
        for part in word.as_bytes().chunks(limit.max(1)) {
            let part = String::from_utf8_lossy(part);
            if line.len() + part.len() + 1 > limit && !line.is_empty() {
                lines.push(std::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(&part);
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}
fn text(out: &mut String, x: f64, y: f64, size: f64, s: &str) {
    let _ = writeln!(
        out,
        "BT /F1 {size:.1} Tf {x:.2} {y:.2} Td ({}) Tj ET",
        escaped(s)
    );
}
fn page_header(title: &str, pack: &ReportPack, part: &str) -> String {
    let mut out = "0.09 0.19 0.30 rg 0 771 595.28 71 re f\n1 1 1 rg\n".to_string();
    text(
        &mut out,
        36.0,
        815.0,
        16.0,
        "BETA Marker | production report",
    );
    text(&mut out, 36.0, 792.0, 11.0, title);
    out.push_str("0 0 0 rg\n");
    text(
        &mut out,
        36.0,
        752.0,
        9.0,
        &format!(
            "PO: {} | Style: {} | Date: {}",
            pack.meta.po, pack.meta.style, pack.meta.date
        ),
    );
    text(
        &mut out,
        36.0,
        738.0,
        9.0,
        &format!("Order: {} | Buyer: {}", pack.order, pack.meta.buyer),
    );
    text(&mut out, 36.0, 719.0, 9.0, part);
    out
}

/// A4 PDF with repeated page/table headers, wrapped identifiers and one lay ticket
/// per lay. Human text uses PDF Helvetica (non-ASCII replaced with '?'); CSV/XLSX
/// preserve Unicode. Production geometry is provided in separate plot/mini files.
pub fn pdf(pack: &ReportPack) -> Result<Vec<u8>> {
    let mut pages = Vec::new();
    for tab in tables(pack) {
        if tab.name == "Lays" || tab.rows.is_empty() {
            continue;
        }
        // Wide registers are split into labelled groups with the row identity
        // repeated in each. This keeps A4 text at a readable 9-point size.
        let groups: Vec<Vec<usize>> = if tab.headers.len() <= 6 {
            vec![(0..tab.headers.len()).collect()]
        } else {
            (1..tab.headers.len())
                .collect::<Vec<_>>()
                .chunks(4)
                .map(|chunk| std::iter::once(0).chain(chunk.iter().copied()).collect())
                .collect()
        };
        let mut out = page_header(
            &tab.name,
            pack,
            "Blanks mean unmeasured. Values round to six decimals for display.",
        );
        let mut y = 696.0;
        for (group_index, cols) in groups.iter().enumerate() {
            let part = format!(
                "{} | columns {}/{} | blanks = unmeasured",
                tab.name,
                group_index + 1,
                groups.len()
            );
            if group_index > 0 {
                y -= 26.0;
                if y < 160.0 {
                    pages.push(out);
                    out = page_header(&tab.name, pack, &part);
                    y = 696.0;
                }
                text(&mut out, 36.0, y, 9.0, &part);
                y -= 18.0;
            }
            let width = 523.0 / cols.len() as f64;
            let header = |out: &mut String, y: &mut f64| {
                out.push_str("0.90 0.94 0.97 rg\n");
                let _ = writeln!(out, "36 {} 523 44 re f", *y - 34.0);
                out.push_str("0 0 0 rg\n");
                for (j, &col) in cols.iter().enumerate() {
                    for (k, line) in wrap(&tab.headers[col], (width / 4.7) as usize - 2)
                        .iter()
                        .enumerate()
                    {
                        text(
                            out,
                            40.0 + j as f64 * width,
                            *y - 12.0 * k as f64,
                            9.0,
                            line,
                        );
                    }
                }
                *y -= 49.0;
            };
            header(&mut out, &mut y);
            for row in &tab.rows {
                let cells: Vec<_> = cols
                    .iter()
                    .map(|i| wrap(&row[*i].text(), (width / 4.7) as usize - 2))
                    .collect();
                let row_lines = cells.iter().map(Vec::len).max().unwrap_or(1);
                // Very long notes/identifiers continue over pages; no clipping.
                for line_index in 0..row_lines {
                    if y < 53.0 {
                        pages.push(out);
                        out = page_header(&tab.name, pack, &part);
                        y = 696.0;
                        header(&mut out, &mut y);
                    }
                    for (j, lines) in cells.iter().enumerate() {
                        if let Some(line) = lines.get(line_index) {
                            text(&mut out, 40.0 + j as f64 * width, y, 9.0, line);
                        }
                    }
                    y -= 12.0;
                }
                y -= 7.0;
                out.push_str("0.85 0.87 0.89 RG\n");
                let _ = writeln!(out, "36 {} m 559 {} l S", y + 10.0, y + 10.0);
                out.push_str("0 0 0 RG\n");
            }
        }
        pages.push(out);
    }
    for lay in &pack.lays {
        let mut out = page_header(
            "Lay / cutting ticket",
            pack,
            &format!("{} | marker {}", lay.lay, lay.marker),
        );
        let mut y = 687.0;
        let lines=vec![format!("Colour: {} | Fabric: {}",pack.meta.colour,pack.meta.fabric),
            format!("Width / laid-flat dia: {:.3} inch | GSM: {:.1}",pack.costing.width_inches,pack.costing.gsm),
            format!("Marker: {:.4} m | Plies: {}",lay.marker_m,lay.plies),
            format!("Head: {:.4} m/ply | Tail: {:.4} m/ply",lay.head_m,lay.tail_m),
            format!("Size ratio: {}",format_ratio(&lay.ratio)),format!("Planned output: {}",format_ratio(&lay.quantities)),
            format!("Rolls: {}",lay.rolls.join(", ")),format!("Lot / dye / shade: {}",shades_text(&lay.shades)),
            format!("Recoverable end bits on last use: {:.4} m",lay.recoverable_ends_m),
            "Planned ticket. Record actual issues, reusable returns and good cut output separately.".into(),
            "Table: __________________  Spreader: __________________  Cutter: __________________".into(),
            "Checked by: __________________  Date / time: __________________".into()];
        for line in lines {
            for line in wrap(&line, 99) {
                if y < 53.0 {
                    pages.push(out);
                    out = page_header("Lay ticket continued", pack, &lay.lay);
                    y = 687.0;
                }
                text(&mut out, 36.0, y, 10.0, &line);
                y -= 16.0;
            }
            y -= 12.0;
        }
        pages.push(out);
    }
    let count = pages.len();
    for (i, page) in pages.iter_mut().enumerate() {
        text(
            page,
            36.0,
            25.0,
            8.0,
            &format!(
                "{} | {} | page {}/{} | print A4",
                pack.meta.po,
                pack.currency,
                i + 1,
                count
            ),
        );
    }
    pdf_bytes(&pages)
}
fn pdf_bytes(pages: &[String]) -> Result<Vec<u8>> {
    let mut objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        Vec::new(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
    ];
    let mut kids = String::new();
    for page in pages {
        let id = objects.len() + 1;
        write!(kids, "{id} 0 R ")?;
        objects.push(format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595.28 841.89] /Resources << /Font << /F1 3 0 R >> >> /Contents {} 0 R >>",id+1).into_bytes());
        let mut stream = format!("<< /Length {} >>\nstream\n", page.len()).into_bytes();
        stream.extend_from_slice(page.as_bytes());
        stream.extend_from_slice(b"endstream");
        objects.push(stream);
    }
    objects[1] = format!("<< /Type /Pages /Count {} /Kids [{kids}] >>", pages.len()).into_bytes();
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
    for offset in offsets.iter().skip(1) {
        // u64: the 10-digit xref limit is larger than a 32-bit (WASM) usize.
        ensure!(*offset as u64 <= 9_999_999_999, "PDF exceeds xref limit");
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
