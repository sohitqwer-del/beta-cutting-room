//! Rule-library interchange using BETA's existing bounded ASCII RUL subset.
//! No vendor-native compatibility is implied. Geometry stays in DXF or pattern JSON.
use crate::{
    Document, GradeMode, GradeRule,
    grading::{self, RuleLibrary},
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

/// Parsed library plus assumptions from the existing marker RUL reader.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryImport {
    /// Rules in document size order.
    pub library: RuleLibrary,
    /// Warnings to show before committing imported rules.
    pub warnings: Vec<String>,
}
/// Parse a JSON library or the marker reader's documented RUL subset.
/// This function only parses; applying checks size compatibility and collisions.
pub fn read(source: &str, format: &str) -> Result<LibraryImport> {
    ensure!(
        source.len() <= 8 * 1024 * 1024,
        "rule library exceeds 8 MiB"
    );
    if format == "json" {
        return Ok(LibraryImport {
            library: serde_json::from_str(source)?,
            warnings: vec![],
        });
    }
    ensure!(format == "rul", "choose JSON or RUL");
    let table = beta_marker_dxf::GradeTable::parse(source.as_bytes())?;
    let rules = table
        .rules
        .iter()
        .map(|(id, deltas)| {
            (
                id.clone(),
                GradeRule {
                    mode: GradeMode::Relative,
                    deltas: table
                        .sizes
                        .iter()
                        .cloned()
                        .zip(deltas.iter().copied())
                        .collect(),
                },
            )
        })
        .collect();
    Ok(LibraryImport {
        library: RuleLibrary {
            format: "beta-pattern.rule-library/v1".into(),
            name: "Imported RUL".into(),
            sizes: table.sizes,
            base_size: table.base,
            rules,
        },
        warnings: table.warnings,
    })
}
/// Install rules atomically. Existing IDs require explicit replacement.
/// Validate every affected size so an import cannot quietly create invalid pieces.
pub fn install(doc: &Document, library: &RuleLibrary, replace: bool) -> Result<Document> {
    ensure!(
        replace || library.rules.keys().all(|id| !doc.rules.contains_key(id)),
        "rule IDs already exist; enable Replace matching rule IDs after reviewing the table"
    );
    let out = grading::apply_library(doc, library)?;
    for size in &out.sizes {
        crate::evaluate::grade(&out, size, 0.1)?;
    }
    Ok(out)
}
/// Export a reusable library as JSON or the existing ASCII RUL subset.
/// Mixed relative/incremental rules become cumulative displacements in RUL.
pub fn write(doc: &Document, name: &str, format: &str) -> Result<String> {
    let library = grading::rule_library(doc, name);
    grading::apply_library(doc, &library)?;
    if format == "json" {
        return Ok(serde_json::to_string_pretty(&library)?);
    }
    ensure!(format == "rul", "choose JSON or RUL");
    let safe = |s: &str| {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
    };
    ensure!(
        doc.sizes.iter().all(|s| safe(s)) && doc.rules.keys().all(|s| safe(s)),
        "RUL size and rule names must use ASCII letters, numbers, underscores, dots or hyphens; use JSON for other names"
    );
    let mut out = format!(
        "# BETA ASCII RUL subset; cumulative millimetres\nUNITS: MM\nMODE: CUMULATIVE\nSIZES: {}\nBASE: {}\n",
        doc.sizes.join(" "),
        doc.base_size
    );
    for (id, rule) in &doc.rules {
        out.push_str(&format!("RULE: {id}"));
        for size in &doc.sizes {
            let d = grading::rule_delta(doc, rule, size)?;
            ensure!(
                d.iter().all(|n| n.abs() <= 1e6),
                "RUL delta exceeds supported range"
            );
            out.push_str(&format!(" {} {}", d[0], d[1]));
        }
        out.push('\n');
    }
    out.push_str("END\n");
    // Keep writer and existing reader in agreement rather than guessing dialects.
    beta_marker_dxf::GradeTable::parse(out.as_bytes())?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    fn document() -> Document {
        Document {
            sizes: vec!["S".into(), "M".into(), "L".into(), "XL".into()],
            rules: BTreeMap::from([(
                "width".into(),
                GradeRule {
                    mode: GradeMode::Incremental,
                    deltas: BTreeMap::from([
                        ("M".into(), [2., 3.]),
                        ("L".into(), [4., 5.]),
                        ("XL".into(), [6., 7.]),
                    ]),
                },
            )]),
            ..Document::default()
        }
    }
    #[test]
    fn json_keeps_modes_and_rul_keeps_movements_both_sides_of_base() {
        let d = document();
        for format in ["json", "rul"] {
            let source = write(&d, "Factory", format).unwrap();
            let parsed = read(&source, format).unwrap();
            if format == "json" {
                assert_eq!(parsed.library.rules, d.rules);
            }
            let next = install(&d, &parsed.library, true).unwrap();
            for s in &d.sizes {
                assert_eq!(
                    grading::rule_delta(&next, &next.rules["width"], s).unwrap(),
                    grading::rule_delta(&d, &d.rules["width"], s).unwrap()
                );
            }
            assert!(install(&d, &parsed.library, false).is_err());
        }
    }
    #[test]
    fn incompatible_ambiguous_invalid_and_unsafe_inputs() {
        let d = document();
        let mut lib = grading::rule_library(&d, "Test");
        lib.sizes.reverse();
        assert!(install(&d, &lib, true).is_err());
        assert!(
            read(
                "SIZES: M L\nBASE: M\nMODE: CUMULATIVE\nRULE: a 1 2 3 4\nEND\n",
                "rul"
            )
            .is_err()
        );
        assert!(
            read("SIZES: M L\nBASE: M\nRULE: a 0 0 3 4\nEND\n", "rul")
                .unwrap()
                .warnings
                .iter()
                .any(|w| w.contains("MODE absent"))
        );
        let mut bad = d.clone();
        bad.rules
            .insert("line\nRULE: injected".into(), d.rules["width"].clone());
        assert!(write(&bad, "Test", "rul").is_err());
        assert!(write(&bad, "Test", "json").is_ok());
        let mut lib = grading::rule_library(&d, "Test");
        lib.rules.get_mut("width").unwrap().deltas.remove("L");
        assert!(install(&d, &lib, true).is_err());
    }
}
