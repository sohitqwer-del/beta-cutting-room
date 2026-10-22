//! Factory QC spec exports. All formats consume the same measured Rust report.
use crate::xlsx::{Cell, Sheet, workbook};
use crate::{
    Document,
    block_checks::{self, SpecSheet},
};
use anyhow::{Result, bail};
use serde::Serialize;

/// Offline binary download. Byte arrays work in native JSON and WASM alike.
#[derive(Debug, Clone, Serialize)]
#[allow(missing_docs)]
pub struct SpecFile {
    pub filename: String,
    pub mime: String,
    pub bytes: Vec<u8>,
}
fn number(n: Option<f64>) -> String {
    n.map(|n| format!("{n:.2}")).unwrap_or_default()
}
/// Header and rows of the one-row-per-POM-and-size table (CSV and the XLSX Data sheet).
struct Table {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}
fn tidy(s: &SpecSheet) -> Table {
    let headers = [
        "Style",
        "Size",
        "POM ID",
        "Point of measure",
        "Method",
        "Measured mm",
        "Body/design mm",
        "Ease mm",
        "Target mm",
        "Grade step mm",
        "Tolerance +/- mm",
        "Target check",
        "Unit",
    ]
    .map(String::from)
    .to_vec();
    let rows = s
        .rows
        .iter()
        .flat_map(|r| {
            r.values.iter().map(|v| {
                vec![
                    s.style.clone(),
                    v.size.clone(),
                    r.id.clone(),
                    r.name.clone(),
                    r.method.clone(),
                    number(Some(v.measured_mm)),
                    number(v.body_mm),
                    number(v.ease_mm),
                    number(v.target_mm),
                    number(v.grade_step_mm),
                    number(Some(r.tolerance_mm)),
                    v.pass
                        .map(|p| if p { "pass" } else { "fail" }.into())
                        .unwrap_or_else(|| "design only".into()),
                    "mm".into(),
                ]
            })
        })
        .collect();
    Table { headers, rows }
}
fn csv(s: &SpecSheet) -> Vec<u8> {
    // A leading apostrophe protects user names from spreadsheet formula interpretation.
    let quote = |v: &str| {
        let safe = if v.trim_start().starts_with(['=', '+', '-', '@', '\t', '\r'])
            && v.parse::<f64>().is_err()
        {
            format!("'{v}")
        } else {
            v.into()
        };
        format!("\"{}\"", safe.replace('"', "\"\""))
    };
    let t = tidy(s);
    let mut out = String::from("\u{feff}");
    for row in std::iter::once(t.headers).chain(t.rows) {
        out.push_str(&row.iter().map(|v| quote(v)).collect::<Vec<_>>().join(","));
        out.push_str("\r\n");
    }
    out.into_bytes()
}
fn wrap(s: &str, width: usize) -> Vec<String> {
    let mut lines = vec![];
    let mut line = String::new();
    for word in s.split_whitespace() {
        if !line.is_empty() && line.chars().count() + word.chars().count() + 1 > width {
            lines.push(std::mem::take(&mut line));
        }
        for ch in word.chars() {
            if line.chars().count() == width {
                lines.push(std::mem::take(&mut line));
            }
            line.push(ch);
        }
        line.push(' ');
    }
    if !line.is_empty() {
        lines.push(line.trim_end().into());
    }
    lines
}
fn pdf(s: &SpecSheet) -> Result<Vec<u8>> {
    let mut pages = vec![];
    for size in &s.sizes {
        let mut lines = vec!["BETA / MEASUREMENT SPECIFICATION".into()];
        lines.extend(wrap(
            &format!("Style: {} | Size: {size} | Base: {}", s.style, s.base_size),
            88,
        ));
        lines.push(
            "Stitch-line design dimensions. All values in mm; QC tolerances are proposed.".into(),
        );
        lines.push("".into());
        lines.push(format!(
            "{:<30} {:>9} {:>9} {:>9} {:>7} {:>7}",
            "POINT OF MEASURE", "ACTUAL", "TARGET", "GRADE", "+/-", "CHECK"
        ));
        lines.push("-".repeat(77));
        for row in &s.rows {
            let v = row.values.iter().find(|v| &v.size == size).unwrap();
            let name: String = row.name.chars().take(30).collect();
            lines.push(format!(
                "{name:<30} {:>9.2} {:>9} {:>9} {:>7.2} {:>7}",
                v.measured_mm,
                number(v.target_mm),
                number(v.grade_step_mm),
                row.tolerance_mm,
                v.pass
                    .map(|p| if p { "PASS" } else { "FAIL" })
                    .unwrap_or("DESIGN")
            ));
        }
        lines.push("".into());
        lines.extend(wrap("GRADE is the change from the previous ordered size; blank for the first size. DESIGN means no body/ease target. It does not mean a failed measurement.",88));
        lines.push("".into());
        for note in &s.notes {
            lines.extend(wrap(note, 88));
            lines.push("".into());
        }
        for page in lines.chunks(46) {
            pages.push(page.to_vec());
        }
    }
    let mut methods = vec!["BETA / POINTS OF MEASURE - METHODS".into(), "".into()];
    for row in &s.rows {
        methods.extend(wrap(&format!("{} [{}]", row.name, row.id), 88));
        methods.extend(wrap(&row.method, 88));
        methods.push("".into());
    }
    for page in methods.chunks(46) {
        pages.push(page.to_vec());
    }
    crate::print::report_pdf(&pages)
}
fn xlsx(s: &SpecSheet) -> Result<Vec<u8>> {
    // Measurements are numeric cells (two decimals); names and methods are literal text.
    let text = |t: &str| Cell::Text(t.into());
    let num = |n: Option<f64>| n.map_or(Cell::Empty, Cell::Number);
    let mut headers = vec!["Point of measure".into(), "Tolerance +/- mm".into()];
    headers.extend(s.sizes.iter().map(|size| format!("{size} mm")));
    let by_size = |value: &dyn Fn(&crate::block_checks::SpecValue) -> Option<f64>, tol: bool| {
        s.rows
            .iter()
            .map(|r| {
                let mut row = vec![
                    text(&r.name),
                    if tol {
                        Cell::Number(r.tolerance_mm)
                    } else {
                        Cell::Empty
                    },
                ];
                row.extend(r.values.iter().map(|v| num(value(v))));
                row
            })
            .collect::<Vec<_>>()
    };
    let data = tidy(s);
    let numeric = [5usize, 6, 7, 8, 9, 10];
    workbook(&[
        Sheet {
            name: "Measurements",
            headers: headers.clone(),
            rows: by_size(&|v| Some(v.measured_mm), true),
        },
        Sheet {
            name: "Targets",
            headers: headers.clone(),
            rows: by_size(&|v| v.target_mm, true),
        },
        Sheet {
            name: "Grade steps",
            headers,
            rows: by_size(&|v| v.grade_step_mm, false),
        },
        Sheet {
            name: "Methods",
            headers: vec![
                "POM ID".into(),
                "Point of measure".into(),
                "Method / notes".into(),
            ],
            rows: s
                .rows
                .iter()
                .map(|r| vec![text(&r.id), text(&r.name), text(&r.method)])
                .chain(
                    s.notes
                        .iter()
                        .map(|n| vec![text("Note"), Cell::Empty, text(n)]),
                )
                .collect(),
        },
        Sheet {
            name: "Data",
            headers: data.headers,
            rows: data
                .rows
                .into_iter()
                .map(|row| {
                    row.into_iter()
                        .enumerate()
                        .map(|(i, v)| match v.parse::<f64>() {
                            Ok(n) if numeric.contains(&i) => Cell::Number(n),
                            _ if v.is_empty() => Cell::Empty,
                            _ => Cell::Text(v),
                        })
                        .collect()
                })
                .collect(),
        },
    ])
}
/// Export a measured style as PDF, quoted CSV or XLSX. All writers are offline Rust.
pub fn export(doc: &Document, format: &str) -> Result<SpecFile> {
    let s = block_checks::spec_sheet(doc)?;
    let (mime, bytes) = match format {
        "pdf" => ("application/pdf", pdf(&s)?),
        "csv" => ("text/csv;charset=utf-8", csv(&s)),
        "xlsx" => (
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            xlsx(&s)?,
        ),
        _ => bail!("Choose pdf, csv or xlsx for a spec sheet."),
    };
    Ok(SpecFile {
        filename: format!("measurement-spec.{format}"),
        mime: mime.into(),
        bytes,
    })
}
