//! Individual measurements, size/height increments, CSV and original XML interchange.
use crate::Document;
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Measurement rows in millimetres, keyed by size label.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MeasurementSet {
    /// Size order, smallest first.
    pub sizes: Vec<String>,
    /// Base size label.
    pub base_size: String,
    /// Size -> measurement -> millimetres.
    pub values: BTreeMap<String, BTreeMap<String, f64>>,
    /// Size -> stature in millimetres, if available.
    pub heights: BTreeMap<String, f64>,
}
/// A base measurement with adjacent-size increments and a height adjustment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeasurementRule {
    /// Value at base size and height, millimetres.
    pub base: f64,
    /// Size -> increment from previous size. Entries below base are subtracted.
    pub increments: BTreeMap<String, f64>,
    /// Measurement change per millimetre of stature difference.
    pub per_height_mm: f64,
}
/// Expand a base-size table across size and stature axes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultisizeTable {
    /// Ordered size labels.
    pub sizes: Vec<String>,
    /// Base size.
    pub base_size: String,
    /// Base stature in millimetres.
    pub base_height: f64,
    /// Statures to generate, millimetres.
    pub heights: Vec<f64>,
    /// Named base values and increments.
    pub rules: BTreeMap<String, MeasurementRule>,
}
impl MultisizeTable {
    /// Evaluate one size/height combination without rounding.
    pub fn row(&self, size: &str, height: f64) -> Result<BTreeMap<String, f64>> {
        ensure!(
            height.is_finite()
                && height > 0.
                && self.base_height.is_finite()
                && self.base_height > 0.,
            "invalid stature"
        );
        ensure!(
            self.sizes.iter().collect::<BTreeSet<_>>().len() == self.sizes.len(),
            "duplicate size"
        );
        let base = self
            .sizes
            .iter()
            .position(|s| s == &self.base_size)
            .context("base size missing")?;
        let target = self
            .sizes
            .iter()
            .position(|s| s == size)
            .context("unknown size")?;
        self.rules
            .iter()
            .map(|(name, r)| {
                let mut v = r.base + (height - self.base_height) * r.per_height_mm;
                for i in base.min(target) + 1..=base.max(target) {
                    v += r
                        .increments
                        .get(&self.sizes[i])
                        .context("missing adjacent-size increment")?
                        * if target >= base { 1. } else { -1. };
                }
                ensure!(v.is_finite(), "non-finite measurement '{name}'");
                Ok((name.clone(), v))
            })
            .collect()
    }
    /// Generate every combination. Labels use `size@height` when there are multiple heights.
    pub fn expand(&self) -> Result<MeasurementSet> {
        ensure!(
            !self.heights.is_empty() && self.heights.contains(&self.base_height),
            "height axis must include base height"
        );
        ensure!(
            self.heights
                .iter()
                .map(|h| h.to_bits())
                .collect::<BTreeSet<_>>()
                .len()
                == self.heights.len(),
            "duplicate height"
        );
        let mut out = MeasurementSet::default();
        for size in &self.sizes {
            for height in &self.heights {
                let label = if self.heights.len() == 1 {
                    size.clone()
                } else {
                    format!("{size}@{height}")
                };
                if size == &self.base_size && *height == self.base_height {
                    out.base_size = label.clone();
                }
                out.sizes.push(label.clone());
                out.heights.insert(label.clone(), *height);
                out.values.insert(label, self.row(size, *height)?);
            }
        }
        Ok(out)
    }
}
impl MeasurementSet {
    /// Replace document measurement rows and size order; constructions remain parametric.
    pub fn apply(&self, doc: &Document) -> Result<Document> {
        self.validate()?;
        let mut out = doc.clone();
        out.sizes = self.sizes.clone();
        out.base_size = self.base_size.clone();
        out.measurements = self.values.clone();
        Ok(out)
    }
    fn validate(&self) -> Result<()> {
        ensure!(
            !self.sizes.is_empty() && self.sizes.contains(&self.base_size),
            "measurement base size missing"
        );
        ensure!(
            self.sizes.iter().collect::<BTreeSet<_>>().len() == self.sizes.len(),
            "duplicate measurement size"
        );
        ensure!(
            self.values.len() == self.sizes.len(),
            "measurement size rows disagree"
        );
        for s in &self.sizes {
            let row = self.values.get(s).context("missing size row")?;
            ensure!(
                !row.is_empty() && row.iter().all(|(n, v)| valid_name(n) && v.is_finite()),
                "invalid measurement row"
            );
        }
        ensure!(
            self.heights
                .iter()
                .all(|(s, h)| self.sizes.contains(s) && h.is_finite() && *h > 0.),
            "invalid measurement height"
        );
        Ok(())
    }
}
fn valid_name(name: &str) -> bool {
    let mut c = name.chars();
    c.next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && c.all(|c| c.is_ascii_alphanumeric() || c == '_')
}
fn quote(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.into()
    }
}
/// CSV long format: `size,height_mm,name,value_mm,base`; rows preserve size order.
pub fn export_csv(set: &MeasurementSet) -> Result<String> {
    set.validate()?;
    let mut out = String::from("size,height_mm,name,value_mm,base\n");
    for size in &set.sizes {
        for (name, value) in &set.values[size] {
            out.push_str(&format!(
                "{},{},{},{},{}\n",
                quote(size),
                set.heights
                    .get(size)
                    .map(f64::to_string)
                    .unwrap_or_default(),
                quote(name),
                value,
                size == &set.base_size
            ));
        }
    }
    Ok(out)
}
fn csv_rows(source: &str) -> Result<Vec<Vec<String>>> {
    ensure!(source.len() <= 16 * 1024 * 1024, "CSV exceeds 16 MiB");
    let mut rows = vec![];
    let mut row = vec![];
    let mut field = String::new();
    let mut chars = source.chars().peekable();
    let mut quoted = false;
    let mut ended = false;
    while let Some(c) = chars.next() {
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    quoted = false;
                    ended = true;
                }
            } else {
                field.push(c);
            }
            continue;
        }
        match c {
            '"' => {
                ensure!(field.is_empty() && !ended, "quote in unquoted CSV field");
                quoted = true;
            }
            ',' => {
                row.push(std::mem::take(&mut field));
                ended = false;
            }
            '\n' => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
                ended = false;
            }
            '\r' if chars.peek() == Some(&'\n') => {}
            _ => {
                ensure!(!ended, "characters after CSV quote");
                field.push(c);
            }
        }
    }
    ensure!(!quoted, "unterminated CSV quote");
    if !field.is_empty() || !row.is_empty() || ended {
        row.push(field);
        rows.push(row);
    }
    Ok(rows)
}
/// Read the long CSV format. Duplicate values or inconsistent heights/base flags fail.
pub fn import_csv(source: &str) -> Result<MeasurementSet> {
    let rows = csv_rows(source)?;
    ensure!(
        rows.first()
            .is_some_and(|r| r == &["size", "height_mm", "name", "value_mm", "base"]),
        "expected measurement CSV header"
    );
    let mut out = MeasurementSet::default();
    for row in rows.into_iter().skip(1) {
        ensure!(row.len() == 5, "expected five CSV columns");
        let s = &row[0];
        ensure!(!s.is_empty(), "empty size");
        if !out.values.contains_key(s) {
            out.sizes.push(s.clone());
        }
        let value: f64 = row[3].parse().context("invalid measurement number")?;
        ensure!(
            out.values
                .entry(s.clone())
                .or_default()
                .insert(row[2].clone(), value)
                .is_none(),
            "duplicate measurement"
        );
        if !row[1].is_empty() {
            let h: f64 = row[1].parse()?;
            if let Some(old) = out.heights.insert(s.clone(), h) {
                ensure!(old == h, "inconsistent height");
            }
        }
        match row[4].as_str() {
            "true" => {
                ensure!(
                    out.base_size.is_empty() || out.base_size == *s,
                    "multiple base sizes"
                );
                out.base_size = s.clone();
            }
            "false" => {}
            _ => bail!("base must be true or false"),
        }
    }
    out.validate()?;
    Ok(out)
}
/// Read `.smis`/`.vit` individual or `.smms`/`.vst` size/height measurement XML.
/// Supports numeric `value`, or `base` with `size_increase`/`height_increase`.
/// Increases are per dimension step: explicit `step`, otherwise size 2 and stature 60 mm.
pub fn import_seamly(source: &str, size_label: &str) -> Result<MeasurementSet> {
    import_seamly_selection(source, size_label, None)
}
/// Evaluate selected numeric size/stature combinations from a multisize XML table.
/// Coordinates use source units. Include the table's base size and height.
pub fn import_seamly_sizes(source: &str, sizes: &[f64], heights: &[f64]) -> Result<MeasurementSet> {
    ensure!(
        !sizes.is_empty() && !heights.is_empty() && sizes.len() * heights.len() <= 10000,
        "invalid measurement selection"
    );
    import_seamly_selection(source, "M", Some((sizes, heights)))
}
fn import_seamly_selection(
    source: &str,
    size_label: &str,
    selection: Option<(&[f64], &[f64])>,
) -> Result<MeasurementSet> {
    ensure!(
        source.len() <= 16 * 1024 * 1024,
        "measurement XML exceeds 16 MiB"
    );
    let xml = roxmltree::Document::parse_with_options(
        source,
        roxmltree::ParsingOptions {
            nodes_limit: 200000,
            ..Default::default()
        },
    )?;
    let root = xml.root_element();
    ensure!(
        matches!(root.tag_name().name(), "vit" | "smis" | "vst" | "smms"),
        "unsupported measurement XML root"
    );
    let unit = xml
        .descendants()
        .find(|n| n.has_tag_name("unit"))
        .and_then(|n| n.text())
        .unwrap_or("mm");
    let scale = match unit.trim() {
        "mm" => 1.,
        "cm" => 10.,
        "in" | "inch" => 25.4,
        _ => bail!("unsupported measurement unit"),
    };
    let number = |s: &str| -> Result<f64> {
        let v: f64 = s.parse()?;
        ensure!(v.is_finite(), "non-finite measurement");
        Ok(v)
    };
    let multi = matches!(root.tag_name().name(), "vst" | "smms");
    let dimension = |tag: &str, fallback: f64| -> Result<(f64, Vec<f64>)> {
        if let Some(n) = xml.descendants().find(|n| n.has_tag_name(tag)) {
            let base = number(
                n.attribute("base")
                    .unwrap_or_else(|| n.text().unwrap_or("0")),
            )?;
            let min = number(n.attribute("min").unwrap_or("0"))?;
            let max = number(n.attribute("max").unwrap_or("0"))?;
            if min == 0. && max == 0. {
                return Ok((base, vec![base]));
            }
            let step = number(n.attribute("step").unwrap_or("1"))?;
            ensure!(
                step > 0. && max >= min && (max - min) / step <= 100.,
                "invalid measurement dimension"
            );
            let values = (0..=((max - min) / step).floor() as usize)
                .map(|i| min + i as f64 * step)
                .collect();
            Ok((base, values))
        } else {
            Ok((fallback, vec![fallback]))
        }
    };
    let (base_size, mut sizes) = dimension("size", 0.)?;
    let (base_height, mut heights) = dimension("height", 0.)?;
    if let Some((ss, hs)) = selection {
        ensure!(multi, "numeric selection requires a multisize table");
        sizes = ss.to_vec();
        heights = hs.to_vec();
    }
    ensure!(
        sizes.iter().chain(&heights).all(|n| n.is_finite()),
        "non-finite measurement dimension"
    );
    let step = |tag: &str, default: f64| -> Result<f64> {
        let step = xml
            .descendants()
            .find(|n| n.has_tag_name(tag))
            .and_then(|n| n.attribute("step"))
            .map(number)
            .transpose()?
            .unwrap_or(default);
        ensure!(step > 0., "invalid dimension step");
        Ok(step)
    };
    let size_step = step("size", 2.)?;
    let height_step = step("height", 60. / scale)?;
    let mut out = MeasurementSet::default();
    for sz in &sizes {
        for ht in &heights {
            let label = if !multi {
                size_label.into()
            } else if heights.len() == 1 {
                sz.to_string()
            } else {
                format!("{sz}@{ht}")
            };
            if *sz == base_size && *ht == base_height {
                out.base_size = label.clone();
            }
            let mut row = BTreeMap::new();
            for n in xml.descendants().filter(|n| n.has_tag_name("m")) {
                let name = n
                    .attribute("name")
                    .context("measurement needs name")?
                    .trim_start_matches('@');
                ensure!(valid_name(name), "unsupported measurement name '{name}'");
                let base = number(
                    n.attribute("value")
                        .or(n.attribute("base"))
                        .context("measurement needs value or base")?,
                )?;
                let si = number(
                    n.attribute("size_increase")
                        .or(n.attribute("sizeIncrease"))
                        .unwrap_or("0"),
                )?;
                let hi = number(
                    n.attribute("height_increase")
                        .or(n.attribute("heightIncrease"))
                        .unwrap_or("0"),
                )?;
                let value = (base
                    + (*sz - base_size) / size_step * si
                    + (*ht - base_height) / height_step * hi)
                    * scale;
                ensure!(
                    value.is_finite() && row.insert(name.into(), value).is_none(),
                    "invalid or duplicate measurement '{name}'"
                );
            }
            out.sizes.push(label.clone());
            if *ht > 0. {
                out.heights.insert(label.clone(), *ht * scale);
            }
            out.values.insert(label, row);
        }
    }
    out.validate()?;
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn csv_and_size_height_increments() {
        let t = MultisizeTable {
            sizes: vec!["S".into(), "M".into(), "L".into()],
            base_size: "M".into(),
            base_height: 1700.,
            heights: vec![1700., 1800.],
            rules: BTreeMap::from([(
                "chest".into(),
                MeasurementRule {
                    base: 900.,
                    increments: BTreeMap::from([("M".into(), 40.), ("L".into(), 50.)]),
                    per_height_mm: 0.1,
                },
            )]),
        };
        assert_eq!(t.row("S", 1800.).unwrap()["chest"], 870.);
        let set = t.expand().unwrap();
        assert_eq!(set.values["L@1800"]["chest"], 960.);
        assert_eq!(import_csv(&export_csv(&set).unwrap()).unwrap(), set);
        assert!(import_csv("size,height_mm,name,value_mm,base\nM,,x,NaN,true\n").is_err());
    }
    #[test]
    fn four_xml_roots() {
        for tag in ["vit", "smis"] {
            let s = format!("<{tag}><unit>cm</unit><m name='chest' value='90'/></{tag}>");
            assert_eq!(
                import_seamly(&s, "made to measure").unwrap().values["made to measure"]["chest"],
                900.
            );
        }
        for tag in ["vst", "smms"] {
            let s = format!(
                "<{tag}><unit>cm</unit><size base='50' min='48' max='52' step='2'/><height base='170' min='170' max='180' step='10'/><m name='chest' base='90' size_increase='2' height_increase='0.1'/></{tag}>"
            );
            let set = import_seamly(&s, "M").unwrap();
            assert_eq!(set.values["52@180"]["chest"], 921.);
            assert_eq!(set.base_size, "50@170");
        }
    }
}
