//! Experimental grade-table execution using an explicit, documented ASCII subset.

use crate::{ImportOptions, ImportReport, curves, parser, pattern};
use anyhow::{Context, Result, bail, ensure};
use beta_marker_core::{MarkKind, Piece, Pt};
use geo::{Polygon, Validation};
use std::collections::BTreeMap;

/// How table offsets relate to the base size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradeMode {
    /// Each size's x/y offset is measured from the base.
    Cumulative,
    /// Entry i is the step from size i-1 to i; entry 0 is unused.
    Incremental,
}

/// Experimental grade rules in millimetres, one x/y pair per named size.
/// See the README for accepted keywords and the incremental convention.
#[derive(Debug, Clone)]
pub struct GradeTable {
    /// Sizes in grading order (never alphabetically sorted).
    pub sizes: Vec<String>,
    /// Size used by the source geometry.
    pub base: String,
    /// Source table interpretation, before conversion to base-relative offsets.
    pub mode: GradeMode,
    /// Rule ID -> base-relative displacement per size, in millimetres.
    pub rules: BTreeMap<String, Vec<Pt>>,
    /// Assumptions and detected ambiguities.
    pub warnings: Vec<String>,
}

fn words(s: &str) -> Vec<&str> {
    s.split(|c: char| c.is_whitespace() || c == ',')
        .filter(|s| !s.is_empty())
        .collect()
}

impl GradeTable {
    /// Parse a UTF-8 ASCII-style `.rul` file. Does not access the filesystem.
    /// Size, base and rules are required. Unknown metadata lines are warned.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        ensure!(bytes.len() <= 8 * 1024 * 1024, "grade table exceeds 8 MiB");
        let text = std::str::from_utf8(bytes).context("grade table must be UTF-8")?;
        let mut sizes = vec![];
        let mut base = None;
        let mut mode = None;
        let mut scale = 1.0;
        let mut rules = BTreeMap::new();
        let mut warnings = vec![];
        let mut rule_lines: Vec<String> = vec![];
        for line in text.lines() {
            let line = line.split_once("//").map_or(line, |(a, _)| a).trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            // AAMA rule tables end with a bare END line.
            if line.eq_ignore_ascii_case("END") {
                break;
            }
            if let Some((key, value)) = line.split_once(':') {
                let value = value.trim();
                match key.trim().to_ascii_uppercase().as_str() {
                    "SIZES" | "SIZE LIST" => {
                        sizes = words(value).iter().map(|s| s.to_string()).collect()
                    }
                    "BASE" | "SAMPLE SIZE" | "BASE SIZE" => base = Some(value.to_string()),
                    "MODE" => {
                        mode = Some(match value.to_ascii_uppercase().as_str() {
                            "CUMULATIVE" | "DELTA" => GradeMode::Cumulative,
                            "INCREMENTAL" => GradeMode::Incremental,
                            _ => bail!("unknown grade MODE {value:?}"),
                        })
                    }
                    "UNITS" => {
                        scale = match value.to_ascii_uppercase().as_str() {
                            "MM" | "METRIC" => 1.0,
                            "CM" => 10.0,
                            "INCH" | "ENGLISH" => 25.4,
                            _ => bail!("unknown grade units {value:?}"),
                        }
                    }
                    "RULE" => rule_lines.push(value.into()),
                    _ => warnings.push(format!("Grade table metadata {key:?} ignored")),
                }
            } else if let Some(last) = rule_lines.last_mut() {
                last.push(' ');
                last.push_str(line);
            } else {
                bail!("grade table line requires a keyword and colon: {line:?}");
            }
        }
        ensure!(
            !sizes.is_empty() && sizes.len() <= 100,
            "grade table requires 1..100 SIZES"
        );
        let unique: std::collections::BTreeSet<_> = sizes.iter().collect();
        ensure!(unique.len() == sizes.len(), "duplicate grade size");
        let base = base.context("grade table requires BASE or SAMPLE SIZE")?;
        let pivot = sizes
            .iter()
            .position(|s| s == &base)
            .context("grade base is absent from SIZES")?;
        let mut delta_keyword = false;
        for line in rule_lines {
            let mut tokens = words(&line);
            if tokens
                .first()
                .is_some_and(|s| s.eq_ignore_ascii_case("DELTA"))
            {
                tokens.remove(0);
                delta_keyword = true;
            }
            ensure!(
                tokens.len() == 1 + 2 * sizes.len(),
                "rule needs an ID and one x,y pair per size: {line:?}"
            );
            let id = tokens[0].trim_start_matches('#').to_string();
            ensure!(!id.is_empty(), "empty grade rule ID");
            let mut offsets = vec![];
            for pair in tokens[1..].as_chunks::<2>().0 {
                let mut p = [0.0; 2];
                for i in 0..2 {
                    p[i] = pair[i].parse::<f64>().context("invalid grade offset")? * scale;
                    ensure!(
                        p[i].is_finite() && p[i].abs() <= 1e6,
                        "grade offset out of range"
                    );
                }
                offsets.push(p);
            }
            ensure!(
                rules.insert(id, offsets).is_none(),
                "duplicate grade rule ID"
            );
        }
        ensure!(
            !rules.is_empty() && rules.len() <= 100_000,
            "grade table needs rules (at most 100000)"
        );
        let mode = mode.unwrap_or_else(|| {
            if delta_keyword || rules.values().all(|v| curves::distance(v[pivot],[0.0,0.0]) < 1e-9) {
                warnings.push("Grade MODE absent: base-size zero offsets or DELTA suggest cumulative values; verify this assumption.".into());
                GradeMode::Cumulative
            } else {
                warnings.push("Grade MODE absent and base offsets nonzero: assuming incremental steps ending at each size; this is ambiguous. Set MODE explicitly.".into());
                GradeMode::Incremental
            }
        });
        if delta_keyword {
            ensure!(
                mode == GradeMode::Cumulative,
                "RULE DELTA conflicts with incremental MODE"
            );
        }
        for offsets in rules.values_mut() {
            if mode == GradeMode::Incremental {
                offsets[0] = [0.0, 0.0];
                for i in 1..offsets.len() {
                    let previous = offsets[i - 1];
                    offsets[i][0] += previous[0];
                    offsets[i][1] += previous[1];
                }
            }
            let origin = offsets[pivot];
            if mode == GradeMode::Cumulative {
                ensure!(
                    curves::distance(origin, [0.0, 0.0]) < 1e-9,
                    "cumulative rule must have zero displacement at base size"
                );
            }
            for p in offsets {
                p[0] -= origin[0];
                p[1] -= origin[1];
            }
        }
        warnings.push("Experimental grading: verify sizes against measurements. Unnumbered contour points use arc-length interpolation; other geometry follows the nearest graded contour displacement. Rule axes must match the expanded drawing axes.".into());
        Ok(Self {
            sizes,
            base,
            mode,
            rules,
            warnings,
        })
    }
}

/// Import a base-size DXF and generate sizes using the supplied experimental rules.
pub fn import_dxf_with_rules(
    bytes: &[u8],
    opts: &ImportOptions,
    table: &GradeTable,
) -> Result<ImportReport> {
    let mut report = pattern::import(parser::read(bytes)?, opts, Some(table))?;
    report.warnings.extend(table.warnings.clone());
    if opts.pairs {
        report
            .warnings
            .push("Pairs: even quantities split into equal normal and mirrored copies.".into());
    }
    Ok(report)
}

/// Recognize explicit rule references, avoiding ordinary size/quantity labels.
fn rule_id(text: &str) -> Option<&str> {
    let text = text.trim();
    if let Some(rest) = text.strip_prefix('#') {
        return rest
            .split([',', ' ', '\t'])
            .next()
            .filter(|s| !s.is_empty());
    }
    let (key, value) = text.split_once(':')?;
    if ["RULE", "GRADE", "GRADE RULE", "RULE ID"]
        .iter()
        .any(|k| key.trim().eq_ignore_ascii_case(k))
    {
        return value
            .trim()
            .split([',', ' ', '\t'])
            .next()
            .filter(|s| !s.is_empty());
    }
    None
}

fn projection(poly: &[Pt], p: Pt) -> (usize, f64, f64) {
    (0..poly.len())
        .map(|i| {
            let a = poly[i];
            let b = poly[(i + 1) % poly.len()];
            let dx = b[0] - a[0];
            let dy = b[1] - a[1];
            let t = if dx * dx + dy * dy > 0.0 {
                (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (i, t, curves::distance(p, [a[0] + t * dx, a[1] + t * dy]))
        })
        .min_by(|a, b| a.2.total_cmp(&b.2))
        .unwrap_or((0, 0.0, f64::INFINITY))
}

// A contour owns only rules that lie on it. Cut and stitch rules stay separate.
struct GradePath {
    outline: Vec<Pt>,
    indices: BTreeMap<usize, String>,
}
impl GradePath {
    fn new(source: &[Pt], references: &[(Pt, String)]) -> Result<Option<Self>> {
        let anchors: Vec<_> = references
            .iter()
            .filter_map(|(point, id)| {
                let (edge, t, d) = projection(source, *point);
                (d <= 0.1).then(|| (edge, t, id.clone()))
            })
            .collect();
        if anchors.is_empty() {
            return Ok(None);
        }
        // Insert projected grade anchors into the sampled boundary, so a rule can
        // attach to a straight segment without requiring an existing DXF vertex.
        let mut outline = vec![];
        let mut indices = BTreeMap::new();
        for i in 0..source.len() {
            let a = source[i];
            let b = source[(i + 1) % source.len()];
            let mut on_edge: Vec<_> = anchors.iter().filter(|(edge, _, _)| *edge == i).collect();
            on_edge.sort_by(|a, b| a.1.total_cmp(&b.1));
            outline.push(a);
            for (_, t, id) in on_edge {
                let q = [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])];
                if curves::distance(*outline.last().context("outline vertex")?, q) > 1e-9 {
                    outline.push(q);
                }
                let idx = outline.len() - 1;
                if let Some(old) = indices.insert(idx, id.clone()) {
                    ensure!(old == *id, "conflicting grade rules at one boundary point");
                }
            }
        }
        // Remove duplicate endpoint anchors introduced by projection onto a preceding edge.
        let mut cleaned = vec![];
        let mut mapped = BTreeMap::new();
        for (i, q) in outline.into_iter().enumerate() {
            if cleaned
                .last()
                .is_none_or(|p| curves::distance(*p, q) > 1e-9)
            {
                cleaned.push(q);
            }
            if let Some(id) = indices.get(&i)
                && let Some(old) = mapped.insert(cleaned.len() - 1, id.clone())
            {
                ensure!(old == *id, "conflicting grade rules at one boundary point");
            }
        }
        if cleaned.len() > 1 && curves::distance(cleaned[0], cleaned[cleaned.len() - 1]) < 1e-9 {
            let last = cleaned.len() - 1;
            cleaned.pop();
            if let Some(id) = mapped.remove(&last)
                && let Some(old) = mapped.insert(0, id.clone())
            {
                ensure!(
                    old == id,
                    "conflicting grade rules at closing boundary point"
                );
            }
        }
        let outline = cleaned;
        let indices = mapped;
        Ok(Some(Self { outline, indices }))
    }
    fn deltas(&self, table: &GradeTable, size: usize) -> Vec<Pt> {
        let n = self.outline.len();
        let mut deltas = vec![[0.0, 0.0]; n];
        let keys: Vec<_> = self.indices.keys().copied().collect();
        for k in 0..keys.len() {
            let start = keys[k];
            let end = keys[(k + 1) % keys.len()];
            let da = table.rules[&self.indices[&start]][size];
            let db = table.rules[&self.indices[&end]][size];
            let steps = if end > start {
                end - start
            } else {
                n - start + end
            };
            let total: f64 = (0..steps)
                .map(|j| {
                    curves::distance(
                        self.outline[(start + j) % n],
                        self.outline[(start + j + 1) % n],
                    )
                })
                .sum();
            let mut distance = 0.0;
            for j in 0..steps {
                let i = (start + j) % n;
                let t = if total > 0.0 { distance / total } else { 0.0 };
                deltas[i] = [da[0] + t * (db[0] - da[0]), da[1] + t * (db[1] - da[1])];
                distance += curves::distance(self.outline[i], self.outline[(i + 1) % n]);
            }
        }
        deltas
    }
    fn displacement(&self, deltas: &[Pt], point: Pt) -> Pt {
        let (i, t, _) = projection(&self.outline, point);
        let a = deltas[i];
        let b = deltas[(i + 1) % deltas.len()];
        [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]
    }
}

pub(crate) fn generate(
    base: &Piece,
    table: &GradeTable,
    warnings: &mut Vec<String>,
) -> Result<Vec<Piece>> {
    ensure!(
        base.size.as_ref().is_none_or(|s| s == &table.base),
        "{}: source size {:?} differs from rule base {}; supply only base-size blocks",
        base.name,
        base.size,
        table.base
    );
    ensure!(
        base.fold_line.is_none(),
        "experimental grading of unfolded layer-6 half patterns is unsupported; supply full base outlines"
    );
    let mut references: Vec<(Pt, String)> = vec![];
    for mark in &base.marks {
        if mark.kind == MarkKind::Text
            && let Some(id) = mark.text.as_deref().and_then(rule_id)
            && let Some(&point) = mark.points.first()
        {
            ensure!(
                table.rules.contains_key(id),
                "{}: unknown grade rule {id}",
                base.name
            );
            for (other, old) in &references {
                ensure!(
                    curves::distance(*other, point) > 1e-6 || table.rules[old] == table.rules[id],
                    "{}: conflicting grade rules at one reference point",
                    base.name
                );
            }
            if references
                .iter()
                .any(|(other, _)| curves::distance(*other, point) <= 1e-6)
            {
                continue;
            }
            references.push((point, id.into()));
        }
    }
    ensure!(
        !references.is_empty(),
        "{}: no grade references; use #ID or RULE: ID",
        base.name
    );
    let points_per_size = base.outline.len()
        + references.len()
        + base.marks.iter().map(|m| m.points.len()).sum::<usize>();
    ensure!(
        points_per_size.saturating_mul(table.sizes.len()) <= 2_000_000,
        "graded piece exceeds two million output points; split the size range or simplify the source"
    );
    let cut = GradePath::new(&base.outline, &references)?;
    let mut internals = vec![];
    // Single POINT references are not contours. Closed internal lines may have
    // independent rules, even when they share the nearest cut segment.
    for (idx, mark) in base.marks.iter().enumerate() {
        if mark.kind == MarkKind::Line
            && mark.points.len() >= 4
            && curves::distance(mark.points[0], mark.points[mark.points.len() - 1]) <= 0.05
            && let Some(path) = GradePath::new(&mark.points[..mark.points.len() - 1], &references)?
        {
            internals.push((idx, path));
        }
    }
    ensure!(
        cut.is_some() || !internals.is_empty(),
        "{}: no grade rules on the cut boundary or a closed internal contour",
        base.name
    );
    if cut.is_none() {
        warnings.push(format!("{}: no cut-line rules; cut outline follows the nearest graded internal contour displacement. This approximates seam allowance; verify graded cut measurements.",base.name));
    }
    let off_cut = references
        .iter()
        .filter(|(q, _)| projection(&base.outline, *q).2 > 0.1)
        .count();
    if off_cut > 0 {
        warnings.push(format!("{}: {off_cut} internal grade references kept separate from cut-line rules; {} closed internal contours graded independently. Other unnumbered marks follow the nearest graded contour.",base.name,internals.len()));
    }
    let mut output = vec![];
    for (size_idx, size) in table.sizes.iter().enumerate() {
        let paths: Vec<_> = cut.iter().chain(internals.iter().map(|(_, p)| p)).collect();
        let deltas: Vec<_> = paths.iter().map(|p| p.deltas(table, size_idx)).collect();
        let move_by = |q: Pt, index: usize| {
            let d = paths[index].displacement(&deltas[index], q);
            [q[0] + d[0], q[1] + d[1]]
        };
        let nearest = |q: Pt| {
            paths
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    projection(&a.outline, q)
                        .2
                        .total_cmp(&projection(&b.outline, q).2)
                })
                .map_or(0, |(i, _)| i)
        };
        let displace = |q: Pt| {
            if let Some((_, id)) = references
                .iter()
                .find(|(a, _)| curves::distance(*a, q) <= 1e-6)
            {
                let d = table.rules[id][size_idx];
                [q[0] + d[0], q[1] + d[1]]
            } else {
                move_by(q, nearest(q))
            }
        };
        let mut p = base.clone();
        p.id = format!("{}-GRADE-{size}", base.id);
        p.size = Some(size.clone());
        // Explicit cut rules win. Without them transfer displacement while
        // retaining each cut vertex's vector from the source internal contour.
        p.outline = if let Some(path) = &cut {
            path.outline.iter().map(|&q| move_by(q, 0)).collect()
        } else {
            base.outline
                .iter()
                .map(|&q| move_by(q, nearest(q)))
                .collect()
        };
        if let Some(g) = &mut p.grain {
            // An unnumbered grain line is a direction constraint, not a fabric
            // contour. Nearest-edge displacement at each end could invent tilt.
            let numbered = |q: Pt| {
                references
                    .iter()
                    .any(|(a, _)| curves::distance(*a, q) <= 1e-6)
            };
            if g.iter().copied().all(numbered) {
                *g = g.map(displace);
            } else {
                let source = g
                    .iter()
                    .copied()
                    .find(|&q| numbered(q))
                    .unwrap_or([(g[0][0] + g[1][0]) / 2.0, (g[0][1] + g[1][1]) / 2.0]);
                let target = displace(source);
                *g = g.map(|q| [q[0] + target[0] - source[0], q[1] + target[1] - source[1]]);
            }
        }
        for (idx, mark) in p.marks.iter_mut().enumerate() {
            if let Some((position, (_, path))) =
                internals.iter().enumerate().find(|(_, (i, _))| *i == idx)
            {
                let index = usize::from(cut.is_some()) + position;
                mark.points = path.outline.iter().map(|&q| move_by(q, index)).collect();
                if let Some(&q) = mark.points.first() {
                    mark.points.push(q);
                }
            } else {
                mark.points = mark.points.iter().copied().map(displace).collect();
            }
        }
        for point in &mut p.match_points {
            point.point = displace(point.point);
        }
        ensure!(
            p.outline.iter().flatten().all(|v| v.is_finite()),
            "graded coordinates out of range"
        );
        ensure!(
            beta_marker_core::geometry::area(&p.outline) > 1e-8,
            "{}: grading collapsed size {size}",
            base.name
        );
        let polygon = Polygon::new(p.outline.iter().map(|q| (q[0], q[1])).collect(), vec![]);
        ensure!(
            polygon.is_valid(),
            "{}: grading produced an invalid outline for size {size}: {:?}",
            base.name,
            polygon.validation_errors()
        );
        output.push(p);
    }
    warnings.push(format!(
        "{}: generated {} sizes from {} grade references (experimental)",
        base.name,
        output.len(),
        references.len()
    ));
    Ok(output)
}
#[cfg(test)]
mod tests {
    use super::*;
    use beta_marker_core::geometry::bbox;

    #[test]
    fn cut_and_sew_rules_grade_independently_and_internal_points_keep_rules() {
        let table = GradeTable::parse(include_bytes!("../../../testdata/dxf/grade-cut-sew.rul"))
            .expect("table");
        let report = import_dxf_with_rules(
            include_bytes!("../../../testdata/dxf/grade-cut-sew-mm.dxf"),
            &ImportOptions::default(),
            &table,
        )
        .expect("independent cut and sew rules");
        let areas: Vec<_> = report
            .pieces
            .iter()
            .map(|p| beta_marker_core::geometry::area(&p.outline))
            .collect();
        assert!(areas[0] < areas[1] && areas[1] < areas[2]);
        let large = &report.pieces[2];
        assert_eq!(bbox(&large.outline), Some([-3.0, -3.0, 123.0, 123.0]));
        let sew = large
            .marks
            .iter()
            .find(|m| m.kind == MarkKind::Line && m.points.len() > 4)
            .expect("sew line");
        assert_eq!(bbox(&sew.points), Some([10.0, 10.0, 110.0, 110.0]));
        let drill = large
            .marks
            .iter()
            .find(|m| m.kind == MarkKind::Drill)
            .expect("drill");
        assert_eq!(drill.points, vec![[60.0, 67.0]]);
        let grain = large.grain.expect("grain");
        assert_eq!(grain[0][0], grain[1][0]);
        assert!(report.warnings.iter().any(|w| w.contains("kept separate")));
        assert!(
            !report
                .warnings
                .iter()
                .any(|w| w.contains("approximates seam"))
        );
    }

    #[test]
    fn sew_only_rules_transfer_to_cut_with_warning() {
        let table = GradeTable::parse(include_bytes!("../../../testdata/dxf/grade-cut-sew.rul"))
            .expect("table");
        let report = import_dxf_with_rules(
            include_bytes!("../../../testdata/dxf/grade-sew-only-mm.dxf"),
            &ImportOptions::default(),
            &table,
        )
        .expect("sew-only rules");
        assert_eq!(
            bbox(&report.pieces[2].outline),
            Some([-2.0, -2.0, 122.0, 122.0])
        );
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.contains("approximates seam"))
        );
        let mut bad = report.pieces[1].clone();
        bad.marks.push(beta_marker_core::Mark {
            kind: MarkKind::Text,
            points: vec![[12.0, 12.0]],
            text: Some("#7".into()),
        });
        assert!(generate(&bad, &table, &mut vec![]).is_err());
    }

    #[test]
    fn grading_rejects_crossed_cut_outline() {
        let mut table =
            GradeTable::parse(include_bytes!("../../../testdata/dxf/grade-cut-sew.rul"))
                .expect("table");
        table.rules.get_mut("3").expect("corner rule")[2] = [-150.0, 0.0];
        let error = import_dxf_with_rules(
            include_bytes!("../../../testdata/dxf/grade-cut-sew-mm.dxf"),
            &ImportOptions::default(),
            &table,
        )
        .expect_err("crossed cut line");
        assert!(error.to_string().contains("invalid outline"));
    }

    #[test]
    fn unnumbered_grain_does_not_inherit_contour_tilt() {
        let table = GradeTable::parse(include_bytes!("../../../testdata/dxf/grade-cut-sew.rul"))
            .expect("table");
        let report = import_dxf_with_rules(
            include_bytes!("../../../testdata/dxf/grade-cut-sew-mm.dxf"),
            &ImportOptions::default(),
            &table,
        )
        .expect("graded");
        let mut base = report.pieces[1].clone();
        base.grain = Some([[5.0, 60.0], [60.0, 115.0]]);
        let result = generate(&base, &table, &mut vec![]).expect("grain translation");
        let grain = result[2].grain.expect("grain");
        assert!((grain[1][0] - grain[0][0] - 55.0).abs() < 1e-9);
        assert!((grain[1][1] - grain[0][1] - 55.0).abs() < 1e-9);
    }

    #[test]
    fn original_cumulative_and_incremental_generate_same_sizes() {
        let dxf = include_bytes!("../../../testdata/dxf/grade-base-mm.dxf");
        let cumulative =
            GradeTable::parse(include_bytes!("../../../testdata/dxf/grade-cumulative.rul"))
                .expect("table");
        let incremental = GradeTable::parse(include_bytes!(
            "../../../testdata/dxf/grade-incremental.rul"
        ))
        .expect("table");
        assert_eq!(cumulative.rules, incremental.rules);
        let report =
            import_dxf_with_rules(dxf, &ImportOptions::default(), &cumulative).expect("graded");
        assert_eq!(report.sizes, ["S", "M", "L"]);
        assert_eq!(report.pieces.len(), 3);
        let p = &report.pieces[2];
        assert_eq!(bbox(&p.outline), Some([-2.0, -1.0, 102.0, 51.0]));
        assert!(
            p.outline.contains(&[50.0, 51.0]),
            "unnumbered top point uses interpolation"
        );
        assert_eq!(
            bbox(&report.pieces[0].outline),
            Some([2.0, 1.0, 98.0, 49.0])
        );
        assert!(report.warnings.iter().any(|w| w.contains("Experimental")));
    }

    #[test]
    fn mode_detection_units_public_keywords_and_bad_tables() {
        let t = GradeTable::parse(
            b"SIZE LIST: S,M,L\r\nSAMPLE SIZE: M\r\nUNITS: CM\r\nRULE: DELTA 1 -1,-2 0,0 1,2\r\n",
        )
        .expect("delta");
        assert_eq!(t.mode, GradeMode::Cumulative);
        assert_eq!(t.rules["1"][2], [10.0, 20.0]);
        assert!(!t.warnings.is_empty());
        let t = GradeTable::parse(b"SIZES: S M L\nBASE: M\nRULE: 1 0,0 2,1 2,1\n")
            .expect("incremental detected");
        assert_eq!(t.mode, GradeMode::Incremental);
        assert_eq!(t.rules["1"][0], [-2.0, -1.0]);
        for s in [
            "",
            "SIZES: M M\nBASE: M\nRULE: 1 0,0 0,0",
            "SIZES: M\nBASE: L\nRULE: 1 0,0",
            "SIZES: M\nBASE: M\nRULE: 1 NaN,0",
            "SIZES: M\nBASE: M\nMODE: CUMULATIVE\nRULE: 1 1,0",
        ] {
            assert!(GradeTable::parse(s.as_bytes()).is_err(), "{s}");
        }
    }
}
