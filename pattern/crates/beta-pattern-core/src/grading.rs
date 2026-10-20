//! Factory grade rules and outline-distance interpolation.
use crate::{Document, GradeMode, GradeRule, geometry::*};
use anyhow::{Context, Result, ensure};
use beta_marker_core::Pt;
use std::collections::BTreeMap;

/// Resolve a rule into movement from the base size.
pub fn rule_delta(doc: &Document, rule: &GradeRule, size: &str) -> Result<Pt> {
    let base = doc
        .sizes
        .iter()
        .position(|s| s == &doc.base_size)
        .context("base size missing from size order")?;
    let target = doc
        .sizes
        .iter()
        .position(|s| s == size)
        .context("unknown target size")?;
    if base == target {
        return Ok([0., 0.]);
    }
    let entry = |i: usize| -> Result<Pt> {
        let d = *rule
            .deltas
            .get(&doc.sizes[i])
            .with_context(|| format!("missing delta for size {}", doc.sizes[i]))?;
        ensure!(d.iter().all(|v| v.is_finite()), "non-finite grade delta");
        Ok(d)
    };
    if rule.mode == GradeMode::Relative {
        return entry(target);
    }
    let mut out = [0., 0.];
    for i in base.min(target) + 1..=base.max(target) {
        out = add(out, mul(entry(i)?, if target > base { 1. } else { -1. }));
    }
    Ok(out)
}
/// Interpolate displacement around a closed outline by arc length between graded vertices.
pub fn interpolate(outline: &[Pt], anchors: &BTreeMap<usize, Pt>) -> Result<Vec<Pt>> {
    ensure!(
        !anchors.is_empty() && anchors.keys().all(|i| *i < outline.len()),
        "grade anchors must refer to outline vertices"
    );
    let n = outline.len();
    let mut shifts = vec![[0., 0.]; n];
    let ids: Vec<_> = anchors.keys().copied().collect();
    for k in 0..ids.len() {
        let a = ids[k];
        let b = ids[(k + 1) % ids.len()];
        let steps = if b > a { b - a } else { n - a + b };
        let total: f64 = (0..steps)
            .map(|j| distance(outline[(a + j) % n], outline[(a + j + 1) % n]))
            .sum();
        ensure!(total > 1e-9, "zero-length grading span");
        let mut traveled = 0.;
        for j in 0..steps {
            let i = (a + j) % n;
            let t = traveled / total;
            shifts[i] = add(mul(anchors[&a], 1. - t), mul(anchors[&b], t));
            traveled += distance(outline[i], outline[(i + 1) % n]);
        }
    }
    Ok(shifts)
}
/// Transfer outline deformation to a mark using the nearest stitch segment.
pub(crate) fn move_point(p: Pt, outline: &[Pt], shifts: &[Pt]) -> Pt {
    let mut best = (f64::INFINITY, [0., 0.]);
    for i in 0..outline.len() {
        let j = (i + 1) % outline.len();
        let v = sub(outline[j], outline[i]);
        let n = dot(v, v);
        if n <= 1e-18 {
            continue;
        }
        let t = (dot(sub(p, outline[i]), v) / n).clamp(0., 1.);
        let d = distance(p, add(outline[i], mul(v, t)));
        if d < best.0 {
            best = (d, add(mul(shifts[i], 1. - t), mul(shifts[j], t)));
        }
    }
    add(p, best.1)
}
/// Reusable factory rule library, independent of drawing objects and piece attachments.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RuleLibrary {
    /// Always `beta-pattern.rule-library/v1`.
    pub format: String,
    /// Human-readable library name.
    pub name: String,
    /// Ordered size labels used by incremental rules.
    pub sizes: Vec<String>,
    /// Origin for cumulative movement.
    pub base_size: String,
    /// Named rules with x/y movement in millimetres.
    pub rules: BTreeMap<String, GradeRule>,
}
/// Extract a reusable library. No evaluated geometry or private body measurements are included.
pub fn rule_library(doc: &Document, name: &str) -> RuleLibrary {
    RuleLibrary {
        format: "beta-pattern.rule-library/v1".into(),
        name: name.into(),
        sizes: doc.sizes.clone(),
        base_size: doc.base_size.clone(),
        rules: doc.rules.clone(),
    }
}
/// Install a size-compatible library atomically, replacing its rule IDs in the document.
/// Every rule must define finite movement for every non-base size.
pub fn apply_library(doc: &Document, library: &RuleLibrary) -> Result<Document> {
    ensure!(
        library.format == "beta-pattern.rule-library/v1",
        "unsupported rule library format"
    );
    ensure!(
        library.sizes == doc.sizes && library.base_size == doc.base_size,
        "rule library size order or base size differs"
    );
    ensure!(!library.rules.is_empty(), "rule library is empty");
    for (id, rule) in &library.rules {
        ensure!(
            !id.is_empty()
                && rule.deltas.keys().all(|s| doc.sizes.contains(s))
                && rule.deltas.values().flatten().all(|v| v.is_finite()),
            "invalid rule '{id}'"
        );
        if rule.mode == GradeMode::Relative
            && let Some(base) = rule.deltas.get(&doc.base_size)
        {
            ensure!(
                distance(*base, [0., 0.]) < 1e-9,
                "relative base movement must be zero"
            );
        }
        for size in &doc.sizes {
            rule_delta(doc, rule, size)?;
        }
    }
    let mut out = doc.clone();
    out.rules.extend(library.rules.clone());
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn relative_and_incremental() {
        let d = Document {
            sizes: vec!["S".into(), "M".into(), "L".into(), "XL".into()],
            ..Document::default()
        };
        let mut r = GradeRule {
            mode: GradeMode::Incremental,
            deltas: BTreeMap::from([
                ("M".into(), [2., 3.]),
                ("L".into(), [4., 5.]),
                ("XL".into(), [6., 7.]),
            ]),
        };
        assert_eq!(rule_delta(&d, &r, "S").unwrap(), [-2., -3.]);
        assert_eq!(rule_delta(&d, &r, "XL").unwrap(), [10., 12.]);
        r.mode = GradeMode::Relative;
        assert_eq!(rule_delta(&d, &r, "XL").unwrap(), [6., 7.]);
        assert!(rule_delta(&d, &r, "S").is_err());
    }
    #[test]
    fn proportional_outline() {
        let p = [[0., 0.], [5., 0.], [10., 0.], [10., 10.], [0., 10.]];
        let a = BTreeMap::from([(0, [0., 0.]), (2, [4., 2.])]);
        let s = interpolate(&p, &a).unwrap();
        assert_eq!(s[1], [2., 1.]);
        assert!(distance(s[3], [8. / 3., 4. / 3.]) < 1e-12);
        assert_eq!(move_point([5., 0.], &p, &s), [7., 1.]);
    }
}
