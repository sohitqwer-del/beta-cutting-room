//! Assembly checks and repeated production marks, independent of browser UI.
use crate::{Annotation, Document, Object, PointOp, Value, evaluate, geometry::*};
use anyhow::{Context, Result, ensure};
use beta_marker_core::{MarkKind, Pt};
use serde::{Deserialize, Serialize};

/// A connected run of directed stitch edges. Indices are zero based.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeamRun {
    /// Pattern piece ID.
    pub piece: String,
    /// Edge indices in connected order, with no repetitions.
    pub edges: Vec<usize>,
    /// Reverse the whole run for sewing alignment.
    #[serde(default)]
    pub reversed: bool,
}
/// Check a pair of seams in every size. Positive ease means A should exceed B.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeamCheck {
    /// Display name for the assembly.
    pub name: String,
    /// First seam run.
    pub a: SeamRun,
    /// Second seam run.
    pub b: SeamRun,
    /// Required A minus B length in mm.
    pub ease_mm: f64,
    /// Permitted absolute error around the required ease.
    pub tolerance_mm: f64,
}
/// One size's assembly measurements, before seam allowances.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeamRow {
    /// Size label.
    pub size: String,
    /// First run length.
    pub a_mm: f64,
    /// Second run length.
    pub b_mm: f64,
    /// Actual first minus second length.
    pub difference_mm: f64,
    /// Requested ease for this size (can differ in a formula-driven block report).
    #[serde(default)]
    pub required_ease_mm: f64,
    /// Actual difference minus requested ease.
    pub error_mm: f64,
    /// Whether error is within the supplied tolerance.
    pub pass: bool,
    /// First seam, translated so its start is at the origin.
    pub a_path: Vec<Pt>,
    /// Second seam, translated and rotated to match the first start tangent.
    /// Neither seam is stretched. This is an alignment preview, not a fit simulation.
    pub b_path: Vec<Pt>,
}
/// Full all-size seam check with explicit inputs and sampling tolerance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeamReport {
    /// Input settings, so a saved report is interpretable.
    pub check: SeamCheck,
    /// Curve flattening tolerance in mm.
    pub sampling_mm: f64,
    /// Rows in document size order.
    pub rows: Vec<SeamRow>,
}
fn run_path(doc: &Document, graded: &[crate::GradedPiece], run: &SeamRun) -> Result<Vec<Pt>> {
    let i = doc
        .pieces
        .iter()
        .position(|p| p.id == run.piece)
        .context("unknown seam piece")?;
    ensure!(!run.edges.is_empty(), "choose at least one seam edge");
    let p = &graded[i];
    let n = p.seam_edges.len();
    ensure!(
        run.edges.iter().all(|e| *e < n),
        "seam edge index out of range"
    );
    let unique: std::collections::BTreeSet<_> = run.edges.iter().collect();
    ensure!(unique.len() == run.edges.len(), "repeated seam edge");
    ensure!(
        run.edges.windows(2).all(|w| (w[0] + 1) % n == w[1]),
        "seam edges must follow connected piece order"
    );
    let mut path = vec![];
    for edge in &run.edges {
        let segment = &p.seam_edges[*edge];
        if let Some(previous) = path.last() {
            ensure!(
                distance(*previous, segment[0]) < 1e-6,
                "disconnected graded seam"
            );
        }
        path.extend(segment.iter().skip(usize::from(!path.is_empty())).copied());
    }
    if run.reversed {
        path.reverse();
    }
    ensure!(length(&path) > 1e-8, "seam has zero length");
    Ok(path)
}
/// Measure actual graded stitch paths, including factory point rules.
pub fn check_seams(doc: &Document, check: &SeamCheck, sampling_mm: f64) -> Result<SeamReport> {
    ensure!(check.ease_mm.is_finite(), "ease must be finite");
    ensure!(
        check.tolerance_mm.is_finite() && check.tolerance_mm >= 0.,
        "tolerance must be finite and non-negative"
    );
    ensure!(
        !doc.sizes.is_empty() && doc.sizes.len() <= 100,
        "use 1 to 100 sizes"
    );
    let mut rows = vec![];
    for size in &doc.sizes {
        let graded = evaluate::grade(doc, size, sampling_mm)?;
        let a = run_path(doc, &graded, &check.a)?;
        let b = run_path(doc, &graded, &check.b)?;
        let a_mm = length(&a);
        let b_mm = length(&b);
        let error_mm = a_mm - b_mm - check.ease_mm;
        let ua = unit(sub(a[1], a[0]))?;
        let ub = unit(sub(b[1], b[0]))?;
        let c = dot(ua, ub);
        let s = cross(ub, ua);
        rows.push(SeamRow {
            size: size.clone(),
            a_mm,
            b_mm,
            difference_mm: a_mm - b_mm,
            required_ease_mm: check.ease_mm,
            error_mm,
            pass: error_mm.abs() <= check.tolerance_mm + 1e-9,
            a_path: a.iter().map(|p| sub(*p, a[0])).collect(),
            b_path: b
                .iter()
                .map(|p| {
                    let q = sub(*p, b[0]);
                    [c * q[0] - s * q[1], s * q[0] + c * q[1]]
                })
                .collect(),
        });
    }
    Ok(SeamReport {
        check: check.clone(),
        sampling_mm,
        rows,
    })
}
/// Export measurements as a CSV with units and signed ease/error columns.
pub fn seam_csv(report: &SeamReport) -> String {
    let quote = |s: &str| format!("\"{}\"", s.replace('"', "\"\""));
    let mut csv =
        "check,size,a_mm,b_mm,difference_mm,ease_mm,error_mm,tolerance_mm,result\r\n".to_string();
    for r in &report.rows {
        csv.push_str(&format!(
            "{},{},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{}\r\n",
            quote(&report.check.name),
            quote(&r.size),
            r.a_mm,
            r.b_mm,
            r.difference_mm,
            r.required_ease_mm,
            r.error_mm,
            report.check.tolerance_mm,
            if r.pass { "PASS" } else { "CHECK" }
        ));
    }
    csv
}
/// Repeated mark type. Buttonholes are internal lines, never cut boundaries.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RowMark {
    /// Drill centres with button-diameter labels.
    Button,
    /// A straight slit with endpoint bars, labelled as a buttonhole.
    Buttonhole,
    /// Plain drill centres.
    Drill,
}
/// A parametric row of placket marks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkRow {
    /// Target piece ID.
    pub piece: String,
    /// Unique prefix for generated drawing points.
    pub id: String,
    /// First centre point.
    pub origin: String,
    /// Number of marks, 1..100.
    pub count: usize,
    /// Centre spacing in mm.
    pub spacing: Value,
    /// Row direction in degrees.
    pub direction: Value,
    /// Mark kind.
    pub kind: RowMark,
    /// Button diameter or buttonhole length.
    pub length: Value,
    /// Buttonhole angle, independent of row direction.
    pub angle: Value,
}
/// Add a reusable row using native point constructions and ordinary marker marks.
/// Formula and geometry failures in any size reject the entire edit.
pub fn add_mark_row(doc: &Document, row: &MarkRow) -> Result<Document> {
    ensure!(crate::editor::valid_id(&row.id), "use a valid mark row ID");
    ensure!((1..=100).contains(&row.count), "use 1 to 100 marks");
    let index = doc
        .pieces
        .iter()
        .position(|p| p.id == row.piece)
        .context("unknown mark piece")?;
    for size in &doc.sizes {
        let mut e = evaluate::Evaluator::new(doc, size, 0.1)?;
        e.point(&row.origin)?;
        ensure!(e.value(&row.spacing)? > 0., "mark spacing must be positive");
        ensure!(e.value(&row.length)? > 0., "mark length must be positive");
        e.value(&row.direction)?;
        e.value(&row.angle)?;
    }
    let expr = |v: &Value| match v {
        Value::Number(n) => n.to_string(),
        Value::Formula(s) => s.clone(),
    };
    let mut out = doc.clone();
    let mut add = |id: String, operation: PointOp| -> Result<String> {
        ensure!(
            !out.drawing.contains_key(&id),
            "mark point ID already exists: {id}"
        );
        out.drawing.insert(id.clone(), Object::Point { operation });
        Ok(id)
    };
    let mut annotations = vec![];
    for i in 0..row.count {
        let center = add(
            format!("{}.c{i}", row.id),
            PointOp::Polar {
                from: row.origin.clone(),
                distance: Value::Formula(format!("({})*{i}", expr(&row.spacing))),
                angle: row.direction.clone(),
            },
        )?;
        match row.kind {
            RowMark::Button => annotations.push(Annotation::Button {
                point: center,
                diameter: row.length.clone(),
            }),
            RowMark::Drill => annotations.push(Annotation::Points {
                mark: MarkKind::Drill,
                points: vec![center],
                text: None,
            }),
            RowMark::Buttonhole => {
                let mut ends = vec![];
                for sign in [-1, 1] {
                    ends.push(add(
                        format!("{}.h{i}e{}", row.id, ends.len()),
                        PointOp::Polar {
                            from: center.clone(),
                            distance: Value::Formula(format!("({})*{sign}/2", expr(&row.length))),
                            angle: row.angle.clone(),
                        },
                    )?);
                }
                annotations.push(Annotation::Points {
                    mark: MarkKind::Line,
                    points: ends.clone(),
                    text: Some("buttonhole".into()),
                });
                for (j, end) in ends.into_iter().enumerate() {
                    let mut bar = vec![];
                    for sign in [-1, 1] {
                        bar.push(add(
                            format!("{}.h{i}b{j}p{}", row.id, bar.len()),
                            PointOp::Polar {
                                from: end.clone(),
                                distance: (sign as f64 * 1.5).into(),
                                angle: Value::Formula(format!("({})+90", expr(&row.angle))),
                            },
                        )?);
                    }
                    annotations.push(Annotation::Points {
                        mark: MarkKind::Line,
                        points: bar,
                        text: Some("buttonhole end".into()),
                    });
                }
            }
        }
    }
    out.pieces[index].annotations.extend(annotations);
    for size in &out.sizes {
        evaluate::grade(&out, size, 0.1)?;
    }
    Ok(out)
}
