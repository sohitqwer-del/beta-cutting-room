//! Generic ISO 6983-style knife-table output. No vendor dialect is assumed.
use crate::{ExportOptions, scene};
use anyhow::ensure;
use beta_marker_core::{Job, MarkerResult, Pt, geometry::signed_area};
use serde::{Deserialize, Serialize};
use std::fmt::Write;

/// Factory post-processor profile. Load with `serde_json::from_reader`.
/// Defaults are illustrative: M05 raises the knife, M03 lowers it, M70 drills,
/// and M71 notches. These M-codes are NOT universal cutter commands. Verify and
/// adapt every command for the target controller before using a physical cutter.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct IsoProfile {
    /// One controller command per line, with the knife retracted.
    pub knife_up: String,
    /// Start a cutting stroke.
    pub knife_down: String,
    /// Perform a drill operation at the current XY coordinate.
    pub drill: String,
    /// Perform a notch operation at the current XY coordinate.
    pub notch: String,
    /// Optional cut speed, mm/min. Omitted uses the controller's current feed.
    pub cut_feed_mm_min: Option<f64>,
    /// Optional mark speed, mm/min.
    pub mark_feed_mm_min: Option<f64>,
    /// Commands before G21/G90, for controller setup.
    pub preamble: Vec<String>,
    /// End-of-program commands, after the final knife-up.
    pub postamble: Vec<String>,
}
impl Default for IsoProfile {
    fn default() -> Self {
        Self {
            knife_up: "M05".into(),
            knife_down: "M03".into(),
            drill: "M70".into(),
            notch: "M71".into(),
            cut_feed_mm_min: None,
            mark_feed_mm_min: None,
            preamble: vec![],
            postamble: vec!["M30".into()],
        }
    }
}
struct Operation {
    points: Vec<Pt>,
    closed: bool,
    code: Option<String>,
}
fn outline_operations(points: Vec<Pt>, fold: Option<[Pt; 2]>) -> Vec<Operation> {
    let Some([a, b]) = fold else {
        return vec![Operation {
            points,
            closed: true,
            code: None,
        }];
    };
    let len = distance(a, b);
    let on_fold = |p: Pt| {
        len > 0.0
            && ((b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])).abs() / len <= 0.01
    };
    let edges: Vec<bool> = (0..points.len())
        .map(|i| on_fold(points[i]) && on_fold(points[(i + 1) % points.len()]))
        .collect();
    let Some(index) = edges.iter().position(|&edge| edge) else {
        return vec![Operation {
            points,
            closed: true,
            code: None,
        }];
    };
    let start = (index + 1) % points.len();
    let mut stroke = vec![points[start]];
    let mut operations = Vec::new();
    for offset in 0..points.len() {
        let i = (start + offset) % points.len();
        let next = (i + 1) % points.len();
        if edges[i] {
            if stroke.len() > 1 {
                operations.push(Operation {
                    points: std::mem::take(&mut stroke),
                    closed: false,
                    code: None,
                });
            }
            stroke = vec![points[next]];
        } else {
            stroke.push(points[next]);
        }
    }
    operations
}
fn distance(a: Pt, b: Pt) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}
fn corners(points: &[Pt]) -> Vec<Pt> {
    let mut out = Vec::with_capacity(points.len());
    for &point in points {
        if out
            .last()
            .is_none_or(|&previous| distance(previous, point) > 1e-9)
        {
            out.push(point);
        }
    }
    if out.len() > 1 && distance(out[0], out[out.len() - 1]) < 1e-9 {
        out.pop();
    }
    let original = out.clone();
    if original.len() > 3 {
        out = original
            .iter()
            .enumerate()
            .filter_map(|(i, &p)| {
                let a = original[(i + original.len() - 1) % original.len()];
                let b = original[(i + 1) % original.len()];
                let cross = (p[0] - a[0]) * (b[1] - p[1]) - (p[1] - a[1]) * (b[0] - p[0]);
                (cross.abs() > 1e-9).then_some(p)
            })
            .collect();
    }
    if signed_area(&out) < 0.0 {
        out.reverse();
    }
    out
}
fn emit(
    ops: &mut Vec<Operation>,
    current: &mut Pt,
    out: &mut String,
    profile: &IsoProfile,
    feed: Option<f64>,
) -> anyhow::Result<()> {
    while !ops.is_empty() {
        let mut best = (f64::INFINITY, 0, 0);
        for (i, op) in ops.iter().enumerate() {
            for (j, &p) in
                op.points
                    .iter()
                    .enumerate()
                    .take(if op.closed { op.points.len() } else { 1 })
            {
                let d = distance(*current, p);
                if d < best.0 {
                    best = (d, i, j);
                }
            }
        }
        let mut op = ops.remove(best.1);
        if op.closed {
            op.points.rotate_left(best.2);
        }
        let first = op.points[0];
        writeln!(
            out,
            "{}\nG00 X{:.3} Y{:.3}",
            profile.knife_up, first[0], first[1]
        )?;
        if let Some(code) = op.code {
            writeln!(out, "{code}")?;
        } else {
            writeln!(out, "{}", profile.knife_down)?;
            if let Some(feed) = feed {
                writeln!(out, "G01 F{feed:.3}")?;
            }
            for p in op
                .points
                .iter()
                .skip(1)
                .chain(if op.closed { Some(&first) } else { None })
            {
                writeln!(out, "G01 X{:.3} Y{:.3}", p[0], p[1])?;
            }
        }
        writeln!(out, "{}", profile.knife_up)?;
        *current = if op.closed {
            first
        } else {
            *op.points.last().unwrap_or(&first)
        };
    }
    Ok(())
}
/// Make a generic CNC knife program in absolute millimetres. Internal marks and
/// drills precede all outlines. Each stage uses nearest-neighbour travel order;
/// closed outlines start at a corner and run counter-clockwise, even if mirrored.
pub fn to_iso(job: &Job, result: &MarkerResult, profile: &IsoProfile) -> anyhow::Result<String> {
    for command in [
        &profile.knife_up,
        &profile.knife_down,
        &profile.drill,
        &profile.notch,
    ]
    .into_iter()
    .chain(profile.preamble.iter())
    .chain(profile.postamble.iter())
    {
        ensure!(
            !command.is_empty() && command.bytes().all(|b| (32..=126).contains(&b)),
            "ISO commands must be non-empty, single-line ASCII."
        );
    }
    for feed in [profile.cut_feed_mm_min, profile.mark_feed_mm_min]
        .into_iter()
        .flatten()
    {
        ensure!(
            feed.is_finite() && feed > 0.0,
            "Feed rates must be finite and positive."
        );
    }
    let scene = scene::build_without_guides(
        job,
        result,
        &ExportOptions {
            labels: false,
            ..Default::default()
        },
    )?;
    let mut marks = Vec::new();
    let mut cuts = Vec::new();
    for path in scene.paths {
        match path.layer {
            1 => {
                let points = corners(&path.points);
                ensure!(
                    points.len() >= 3,
                    "Cut outline has fewer than three corners."
                );
                cuts.extend(outline_operations(points, path.fold_edge));
            }
            4 => {
                if let Some(&point) = path.points.first() {
                    marks.push(Operation {
                        points: vec![point],
                        closed: false,
                        code: Some(profile.notch.clone()),
                    });
                }
            }
            8 if !path.points.is_empty() => {
                marks.push(Operation {
                    points: path.points,
                    closed: path.closed,
                    code: None,
                });
            }
            _ => {}
        }
    }
    for point in scene.circles {
        marks.push(Operation {
            points: vec![point],
            closed: false,
            code: Some(profile.drill.clone()),
        });
    }
    let mut out = String::from("(BETA generic knife program - verify post-processor profile)\n");
    for command in &profile.preamble {
        writeln!(out, "{command}")?;
    }
    out.push_str("G21\nG90\n");
    let mut current = [0.0; 2];
    emit(
        &mut marks,
        &mut current,
        &mut out,
        profile,
        profile.mark_feed_mm_min,
    )?;
    out.push_str("(OUTLINES)\n");
    emit(
        &mut cuts,
        &mut current,
        &mut out,
        profile,
        profile.cut_feed_mm_min,
    )?;
    writeln!(out, "{}", profile.knife_up)?;
    for command in &profile.postamble {
        writeln!(out, "{command}")?;
    }
    Ok(out)
}
