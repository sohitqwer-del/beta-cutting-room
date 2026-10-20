//! Name-based material inference for files without explicit material metadata.

use beta_marker_core::{Job, Piece};
use std::collections::{BTreeMap, BTreeSet};

/// Material name to case-insensitive name fragments. Earlier sorted keys win.
/// An explicit DXF Material/Category always takes precedence over this dictionary.
pub type MaterialKeywords = BTreeMap<String, Vec<String>>;

fn defaults() -> MaterialKeywords {
    BTreeMap::from([
        (
            "interlining".into(),
            vec![
                "interlining".into(),
                "interfacing".into(),
                "fusing".into(),
                "fusible".into(),
            ],
        ),
        ("lining".into(), vec!["lining".into()]),
        (
            "pocket bag".into(),
            vec!["pocket bag".into(), "pocker bag".into()],
        ),
        (
            "reflective".into(),
            vec!["reflective".into(), "refective".into()],
        ),
    ])
}

/// Fill missing material names from piece names. Unmatched names become `shell`.
/// Custom entries override entries of the same material in the built-in dictionary.
/// Returns a warning for every inferred material, so assumptions remain visible.
pub fn classify_materials(pieces: &mut [Piece], custom: &MaterialKeywords) -> Vec<String> {
    let mut dictionary = defaults();
    dictionary.extend(custom.clone());
    let mut warnings = vec![];
    for p in pieces {
        if p.material.as_ref().is_some_and(|s| !s.trim().is_empty()) {
            continue;
        }
        let name = p.name.to_lowercase();
        let material = dictionary
            .iter()
            .find(|(_, words)| {
                words
                    .iter()
                    .any(|w| !w.trim().is_empty() && name.contains(&w.to_lowercase()))
            })
            .map(|(m, _)| m.as_str())
            .unwrap_or("shell");
        p.material = Some(material.into());
        warnings.push(format!(
            "{}: material inferred as {material:?} from name; verify against the bill of materials",
            p.name
        ));
    }
    warnings
}

/// Keep one material (case-insensitive exact name) and its relevant match rules.
pub fn filter_material(job: &Job, material: &str) -> Job {
    let mut job = job.clone();
    job.pieces.retain(|p| {
        p.material
            .as_deref()
            .unwrap_or("shell")
            .eq_ignore_ascii_case(material)
    });
    retain_rules(&mut job);
    job
}

/// Make a separate job per material. Width, spacing, order and source metadata survive.
pub fn split_materials(job: &Job) -> BTreeMap<String, Job> {
    let names: BTreeSet<_> = job
        .pieces
        .iter()
        .map(|p| p.material.as_deref().unwrap_or("shell").to_lowercase())
        .collect();
    names
        .into_iter()
        .map(|name| {
            let mut part = filter_material(job, &name);
            part.name = format!("{} / {name}", job.name);
            (name, part)
        })
        .collect()
}

fn retain_rules(job: &mut Job) {
    let ids: BTreeSet<_> = job.pieces.iter().map(|p| p.id.as_str()).collect();
    job.match_rules.retain(|r| {
        ids.contains(r.a.piece_id.as_str())
            && r.b
                .as_ref()
                .is_none_or(|b| ids.contains(b.piece_id.as_str()))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precedence_custom_words_and_filter() {
        let mut pieces = [
            "Interlining cuff",
            "Lining sleeve",
            "Pocket Bag",
            "Reflective tape",
            "Body",
            "Mesh insert",
        ]
        .into_iter()
        .map(|name| Piece {
            name: name.into(),
            ..Default::default()
        })
        .collect::<Vec<_>>();
        pieces[4].material = Some("Cotton".into());
        let warnings = classify_materials(
            &mut pieces,
            &BTreeMap::from([("mesh".into(), vec!["mesh".into()])]),
        );
        assert_eq!(warnings.len(), 5);
        assert_eq!(
            pieces
                .iter()
                .map(|p| p.material.as_deref().expect("material"))
                .collect::<Vec<_>>(),
            [
                "interlining",
                "lining",
                "pocket bag",
                "reflective",
                "Cotton",
                "mesh"
            ]
        );
        let job = Job {
            pieces,
            ..Default::default()
        };
        assert_eq!(split_materials(&job).len(), 6);
        assert_eq!(filter_material(&job, "LINING").pieces.len(), 1);
    }

    #[test]
    fn openpattern_materials_keep_all_35_pieces() {
        // Open+Pattern, CC BY 4.0. Attribution: testdata/open/dxf/README.md.
        let mut report = crate::import_dxf(
            include_bytes!("../../../testdata/open/dxf/openpattern-pt-windbreaker-M.dxf"),
            &crate::ImportOptions::default(),
        )
        .expect("open pattern");
        classify_materials(&mut report.pieces, &MaterialKeywords::new());
        let parts = split_materials(&Job {
            pieces: report.pieces,
            ..Default::default()
        });
        assert_eq!(parts.values().map(|j| j.pieces.len()).sum::<usize>(), 35);
        for material in ["lining", "reflective", "pocket bag", "shell"] {
            assert!(parts.contains_key(material), "{material}");
        }
        assert_eq!(parts["lining"].pieces.len(), 5);
    }
}
