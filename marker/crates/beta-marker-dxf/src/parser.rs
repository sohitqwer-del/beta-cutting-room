//! Small group-code reader. Unknown sections are deliberately opaque.

use anyhow::{Context, Result, bail, ensure};
use beta_marker_core::Pt;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub(crate) struct Pair {
    pub code: i32,
    pub value: String,
}

#[derive(Clone, Debug)]
pub(crate) struct Record {
    pub kind: String,
    pub fields: Vec<Pair>,
    pub vertices: Vec<Record>,
}

impl Record {
    pub fn value(&self, code: i32) -> Option<&str> {
        self.fields
            .iter()
            .find(|p| p.code == code)
            .map(|p| p.value.as_str())
    }
    pub fn num(&self, code: i32, default: f64) -> Result<f64> {
        let Some(s) = self.value(code) else {
            return Ok(default);
        };
        let n: f64 = s
            .parse()
            .with_context(|| format!("{}: invalid group {code}: {s:?}", self.kind))?;
        ensure!(
            n.is_finite() && n.abs() <= 1e12,
            "{}: group {code} out of range",
            self.kind
        );
        Ok(n)
    }
    pub fn required(&self, code: i32) -> Result<f64> {
        ensure!(
            self.value(code).is_some(),
            "{} missing group {code}",
            self.kind
        );
        self.num(code, 0.0)
    }
    pub fn int(&self, code: i32, default: i32) -> Result<i32> {
        self.value(code).map_or(Ok(default), |s| {
            s.parse()
                .with_context(|| format!("{}: invalid integer group {code}", self.kind))
        })
    }
    pub fn point(&self, code: i32) -> Result<Pt> {
        Ok([self.required(code)?, self.required(code + 10)?])
    }
    pub fn layer(&self) -> &str {
        self.value(8).unwrap_or("0")
    }
    pub fn text(&self) -> String {
        let text = self
            .fields
            .iter()
            .filter(|p| matches!(p.code, 1 | 3))
            .map(|p| p.value.as_str())
            .collect::<String>();
        if self.kind == "MTEXT" {
            text.replace("\\P", "\n")
        } else {
            text
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Block {
    pub base: Pt,
    pub entities: Vec<Record>,
}

pub(crate) struct Drawing {
    pub header: BTreeMap<String, Vec<Pair>>,
    pub blocks: BTreeMap<String, Block>,
    pub entities: Vec<Record>,
    pub comments: Vec<String>,
}

pub(crate) fn read(bytes: &[u8]) -> Result<Drawing> {
    ensure!(
        bytes.len() <= 64 * 1024 * 1024,
        "DXF exceeds the 64 MiB input limit"
    );
    if bytes.starts_with(b"AutoCAD Binary DXF") || bytes.contains(&0) {
        bail!("Binary DXF is not supported; export an ASCII DXF file");
    }
    let source = std::str::from_utf8(bytes).context("DXF must be ASCII or UTF-8 text")?;
    let mut lines: Vec<&str> = source
        .trim_start_matches('\u{feff}')
        .trim_end_matches(|c: char| c.is_whitespace() || c == '\u{1a}')
        .lines()
        .collect();
    while lines.last().is_some_and(|s| s.trim().is_empty()) {
        lines.pop();
    }
    ensure!(
        lines.len().is_multiple_of(2),
        "DXF has an unmatched group-code line"
    );
    ensure!(
        lines.len() <= 4_000_000,
        "DXF has too many group-code pairs"
    );
    let mut pairs = Vec::with_capacity(lines.len() / 2);
    let mut comments = Vec::new();
    for (i, chunk) in lines.as_chunks::<2>().0.iter().enumerate() {
        let code: i32 = chunk[0]
            .trim()
            .parse()
            .with_context(|| format!("invalid group code on line {}", i * 2 + 1))?;
        ensure!((0..=1071).contains(&code), "invalid DXF group code {code}");
        let value = if matches!(code, 1 | 3 | 1000) {
            chunk[1].to_string()
        } else {
            chunk[1].trim().to_string()
        };
        if code == 999 {
            comments.push(value);
        } else {
            pairs.push(Pair { code, value });
        }
    }
    let mut drawing = Drawing {
        header: BTreeMap::new(),
        blocks: BTreeMap::new(),
        entities: vec![],
        comments,
    };
    let mut i = 0;
    let mut sections = 0;
    let mut eof = false;
    while i < pairs.len() {
        ensure!(
            pairs[i].code == 0,
            "expected SECTION or EOF at group pair {}",
            i + 1
        );
        if pairs[i].value.eq_ignore_ascii_case("EOF") {
            eof = true;
            break;
        }
        ensure!(
            pairs[i].value.eq_ignore_ascii_case("SECTION"),
            "expected SECTION, found {}",
            pairs[i].value
        );
        i += 1;
        ensure!(
            pairs.get(i).is_some_and(|p| p.code == 2),
            "SECTION missing its name"
        );
        let name = pairs[i].value.to_ascii_uppercase();
        i += 1;
        let start = i;
        while i < pairs.len()
            && !(pairs[i].code == 0 && pairs[i].value.eq_ignore_ascii_case("ENDSEC"))
        {
            i += 1;
        }
        ensure!(i < pairs.len(), "section {name} has no ENDSEC");
        match name.as_str() {
            "HEADER" => {
                let mut variable = None;
                for p in &pairs[start..i] {
                    if p.code == 9 {
                        variable = Some(p.value.to_ascii_uppercase());
                    } else if let Some(key) = &variable {
                        drawing
                            .header
                            .entry(key.clone())
                            .or_default()
                            .push(p.clone());
                    }
                }
            }
            "ENTITIES" => drawing.entities.extend(records(&pairs[start..i])?),
            "BLOCKS" => {
                let raw = records(&pairs[start..i])?;
                let mut iter = raw.into_iter();
                while let Some(record) = iter.next() {
                    ensure!(
                        record.kind == "BLOCK",
                        "BLOCKS expected BLOCK, found {}",
                        record.kind
                    );
                    let key = record
                        .value(2)
                        .or(record.value(3))
                        .context("BLOCK missing name")?
                        .to_string();
                    let base = [record.num(10, 0.0)?, record.num(20, 0.0)?];
                    let mut entities = vec![];
                    let mut ended = false;
                    for entity in iter.by_ref() {
                        if entity.kind == "ENDBLK" {
                            ended = true;
                            break;
                        }
                        entities.push(entity);
                    }
                    ensure!(ended, "BLOCK {key} has no ENDBLK");
                    ensure!(
                        drawing
                            .blocks
                            .insert(key.to_ascii_uppercase(), Block { base, entities })
                            .is_none(),
                        "duplicate BLOCK {key}"
                    );
                }
            }
            _ => {}
        }
        sections += 1;
        i += 1;
    }
    ensure!(sections > 0 && eof, "ASCII DXF requires a SECTION and EOF");
    Ok(drawing)
}

fn records(pairs: &[Pair]) -> Result<Vec<Record>> {
    let mut result: Vec<Record> = vec![];
    let mut i = 0;
    let mut polyline: Option<Record> = None;
    while i < pairs.len() {
        ensure!(pairs[i].code == 0, "entity missing group 0");
        let kind = pairs[i].value.to_ascii_uppercase();
        i += 1;
        let start = i;
        while i < pairs.len() && pairs[i].code != 0 {
            i += 1;
        }
        let record = Record {
            kind,
            fields: pairs[start..i].to_vec(),
            vertices: vec![],
        };
        match record.kind.as_str() {
            "POLYLINE" => {
                ensure!(polyline.is_none(), "POLYLINE missing SEQEND");
                polyline = Some(record);
            }
            "VERTEX" => polyline
                .as_mut()
                .context("VERTEX outside POLYLINE")?
                .vertices
                .push(record),
            "SEQEND" => {
                if let Some(mut poly) = polyline.take() {
                    if record.value(1001) == Some("BETA_MARK") {
                        poly.fields.extend(record.fields);
                    }
                    result.push(poly);
                }
            }
            "ATTRIB" if result.last().is_some_and(|r| r.kind == "INSERT") => {
                if let Some(insert) = result.last_mut() {
                    insert.vertices.push(record);
                }
            }
            _ => {
                ensure!(polyline.is_none(), "POLYLINE missing SEQEND");
                result.push(record);
            }
        }
    }
    ensure!(polyline.is_none(), "POLYLINE missing SEQEND");
    Ok(result)
}
