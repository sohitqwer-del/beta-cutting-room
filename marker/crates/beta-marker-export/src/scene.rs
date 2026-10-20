use crate::ExportOptions;
use anyhow::{Context, ensure};
use beta_marker_core::rules::cut_outline;
use beta_marker_core::{
    Job, MarkKind, MarkerResult, Pt,
    geometry::{bbox, place, signed_area, transform_point},
    rules::effective_job,
};
use std::collections::{BTreeSet, HashMap};

pub(super) struct Path {
    pub points: Vec<Pt>,
    pub closed: bool,
    pub pen: u8,
    pub colour: &'static str,
    pub layer: u8,
    pub fold_edge: Option<[Pt; 2]>,
}
pub(super) struct Text {
    pub point: Pt,
    pub value: String,
    pub footer: bool,
}
pub(super) struct Scene {
    pub length: f64,
    pub canvas_length: f64,
    pub width: f64,
    pub height: f64,
    pub paths: Vec<Path>,
    pub circles: Vec<Pt>,
    pub texts: Vec<Text>,
}
pub(super) fn single_line(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}
pub(super) fn ascii(s: &str) -> String {
    single_line(s)
        .chars()
        .map(|c| if c.is_ascii() { c } else { '?' })
        .collect()
}

pub(super) fn build(
    job: &Job,
    result: &MarkerResult,
    opts: &ExportOptions,
) -> anyhow::Result<Scene> {
    build_inner(job, result, opts, true)
}

pub(super) fn build_without_guides(
    job: &Job,
    result: &MarkerResult,
    opts: &ExportOptions,
) -> anyhow::Result<Scene> {
    build_inner(job, result, opts, false)
}

fn build_inner(
    job: &Job,
    result: &MarkerResult,
    opts: &ExportOptions,
    show_guides: bool,
) -> anyhow::Result<Scene> {
    let effective = effective_job(job);
    let job = &effective;
    let m = beta_marker_core::compute_metrics(job, &result.placements);
    ensure!(
        job.fabric.width.is_finite() && job.fabric.width > 0.0 && m.length.is_finite(),
        "Fabric dimensions must be finite and width must be positive."
    );
    let mut scene = Scene {
        length: m.length.max(1.0),
        canvas_length: m.length.max(24.0),
        width: job.fabric.width,
        height: job.fabric.width + 14.0,
        paths: vec![],
        circles: vec![],
        texts: vec![],
    };
    let by_id: HashMap<_, _> = job.pieces.iter().map(|p| (p.id.as_str(), p)).collect();
    let sizes: Vec<_> = job
        .pieces
        .iter()
        .map(|p| p.size.as_deref().unwrap_or(""))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    const PALETTE: [&str; 8] = [
        "#1769aa", "#a12635", "#267044", "#7541a0", "#a45a00", "#007b83", "#9a3e79", "#424242",
    ];
    for pl in &result.placements {
        let p = by_id
            .get(pl.piece_id.as_str())
            .with_context(|| format!("Unknown piece '{}'.", pl.piece_id))?;
        ensure!(
            p.outline.len() >= 3,
            "Piece '{}' has no usable outline.",
            p.id
        );
        ensure!(
            p.block_margin.is_none_or(|v| v.is_finite() && v >= 0.0),
            "Piece '{}' has an invalid block margin.",
            p.id
        );
        let points = cut_outline(p, pl);
        ensure!(
            points.iter().flatten().all(|n| n.is_finite()),
            "Piece '{}' has non-finite coordinates.",
            p.id
        );
        let bounds = bbox(&points).context("Empty outline")?;
        scene.length = scene.length.max(bounds[2]);
        scene.canvas_length = scene.canvas_length.max(scene.length);
        let colour = PALETTE[sizes
            .binary_search(&p.size.as_deref().unwrap_or(""))
            .unwrap_or(0)
            % PALETTE.len()];
        scene.paths.push(Path {
            points,
            closed: true,
            pen: 1,
            colour,
            layer: 1,
            fold_edge: if pl.on_fold {
                p.fold_line.map(|line| {
                    let q = place(&line, pl);
                    [q[0], q[1]]
                })
            } else {
                None
            },
        });
        if opts.labels {
            scene.texts.push(Text {
                footer: false,
                point: [bounds[0] + 1.0, (bounds[1] + bounds[3]) / 2.0],
                value: format!(
                    "{} | {} | copy {}{} | bundle {}",
                    single_line(&p.name),
                    single_line(p.size.as_deref().unwrap_or("unsized")),
                    pl.copy,
                    if pl.mirrored { " | mirrored" } else { "" },
                    pl.bundle
                        .map(|b| b.to_string())
                        .unwrap_or_else(|| "unassigned".into())
                ),
            });
        }
        if opts.marks {
            for mark in &p.marks {
                ensure!(
                    mark.points.iter().flatten().all(|v| v.is_finite()),
                    "Piece '{}' has non-finite marks.",
                    p.id
                );
                let Some(&anchor) = mark.points.first() else {
                    continue;
                };
                if !visible(p, pl, anchor) && mark.kind != MarkKind::Line {
                    continue;
                }
                match mark.kind {
                    MarkKind::Drill => scene.circles.push(transform_point(
                        anchor,
                        pl.mirrored,
                        pl.rotation_deg,
                        pl.translate,
                    )),
                    MarkKind::Text => {
                        if let Some(text) = &mark.text {
                            scene.texts.push(Text {
                                footer: false,
                                point: transform_point(
                                    anchor,
                                    pl.mirrored,
                                    pl.rotation_deg,
                                    pl.translate,
                                ),
                                value: single_line(text),
                            });
                        }
                    }
                    MarkKind::Line => {
                        for segment in clipped_lines(p, pl, &mark.points) {
                            scene.paths.push(Path {
                                points: place(&segment, pl),
                                closed: false,
                                pen: 2,
                                colour: "#555555",
                                layer: 8,
                                fold_edge: None,
                            });
                        }
                    }
                    MarkKind::Notch => {
                        let direction = mark.points.get(1).copied().unwrap_or_else(|| {
                            // The closest outline segment supplies an inward normal for
                            // notches that carry only an anchor.
                            let mut nearest = (f64::INFINITY, [0.0, 1.0]);
                            for i in 0..p.outline.len() {
                                let a = p.outline[i];
                                let b = p.outline[(i + 1) % p.outline.len()];
                                let dx = b[0] - a[0];
                                let dy = b[1] - a[1];
                                let l2 = dx * dx + dy * dy;
                                if l2 == 0.0 {
                                    continue;
                                }
                                let t = (((anchor[0] - a[0]) * dx + (anchor[1] - a[1]) * dy) / l2)
                                    .clamp(0.0, 1.0);
                                let d =
                                    (anchor[0] - a[0] - t * dx).hypot(anchor[1] - a[1] - t * dy);
                                if d < nearest.0 {
                                    let w = signed_area(&p.outline).signum();
                                    nearest = (d, [-dy * w, dx * w]);
                                }
                            }
                            [anchor[0] + nearest.1[0], anchor[1] + nearest.1[1]]
                        });
                        let dx = direction[0] - anchor[0];
                        let dy = direction[1] - anchor[1];
                        let len = dx.hypot(dy);
                        let tip = if len > 0.0 {
                            [anchor[0] + 3.0 * dx / len, anchor[1] + 3.0 * dy / len]
                        } else {
                            [anchor[0], anchor[1] + 3.0]
                        };
                        scene.paths.push(Path {
                            points: place(&[anchor, tip], pl),
                            closed: false,
                            pen: 2,
                            colour: "#555555",
                            layer: 4,
                            fold_edge: None,
                        });
                    }
                }
            }
        }
        if pl.on_fold
            && let Some(line) = p.fold_line
        {
            let fold_points = place(&line, pl);
            let fold_y = (fold_points[0][1] + fold_points[1][1]) / 2.0;
            let label_y = if (fold_y - bounds[1]).abs() <= (fold_y - bounds[3]).abs() {
                fold_y + 3.0
            } else {
                fold_y - 3.0
            };
            let label_x = (fold_points[0][0] + fold_points[1][0]) / 2.0 - 3.0;
            scene.paths.push(Path {
                points: fold_points,
                closed: false,
                pen: 3,
                colour: "#007b83",
                layer: 0,
                fold_edge: None,
            });
            scene.texts.push(Text {
                point: [label_x, label_y],
                value: "FOLD".into(),
                footer: false,
            });
        }
        if opts.marks
            && let Some(grain) = p.grain
        {
            for segment in clipped_lines(p, pl, &grain) {
                scene.paths.push(Path {
                    points: place(&segment, pl),
                    closed: false,
                    pen: 2,
                    colour: "#777777",
                    layer: 7,
                    fold_edge: None,
                });
            }
        }
    }
    if show_guides {
        guides(job, &mut scene)?;
    }
    let length = if result.placements.is_empty() {
        0.0
    } else {
        scene.length
    };
    let efficiency = if length > 0.0 {
        m.piece_area / (job.fabric.width * length)
    } else {
        0.0
    };
    let title = format!(
        "{} | Width {:.2} mm | Length {:.2} mm | Efficiency {:.2}%",
        single_line(&job.name),
        job.fabric.width,
        length,
        efficiency * 100.0
    );
    let columns = ((scene.canvas_length - 2.0) / 3.0).floor().max(1.0) as usize;
    let mut line = String::new();
    let mut row = 0;
    for word in title.split_whitespace() {
        for chunk in word.chars().collect::<Vec<_>>().chunks(columns) {
            if !line.is_empty() && line.chars().count() + 1 + chunk.len() > columns {
                scene.texts.push(Text {
                    footer: true,
                    point: [1.0, job.fabric.width + 5.0 + row as f64 * 4.0],
                    value: std::mem::take(&mut line),
                });
                row += 1;
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.extend(chunk);
        }
    }
    if !line.is_empty() {
        scene.texts.push(Text {
            footer: true,
            point: [1.0, job.fabric.width + 5.0 + row as f64 * 4.0],
            value: line,
        });
    }
    scene.height = scene.height.max(job.fabric.width + 10.0 + row as f64 * 4.0);
    // Model and plotter y point up. Put the first title line at the top.
    for text in scene.texts.iter_mut().filter(|text| text.footer) {
        text.point[1] = scene.width + scene.height - text.point[1];
    }
    ensure!(
        scene.height.is_finite()
            && scene
                .paths
                .iter()
                .all(|p| p.points.iter().flatten().all(|v| v.is_finite()))
            && scene.circles.iter().flatten().all(|v| v.is_finite())
            && scene
                .texts
                .iter()
                .all(|t| t.point.iter().all(|v| v.is_finite())),
        "Transformed drawing contains non-finite coordinates."
    );
    Ok(scene)
}

fn guides(job: &Job, scene: &mut Scene) -> anyhow::Result<()> {
    use beta_marker_core::ShadeAxis;
    let mut line = |points, colour| {
        scene.paths.push(Path {
            points,
            closed: false,
            pen: 3,
            colour,
            layer: 0,
            fold_edge: None,
        })
    };
    for defect in &job.fabric.defects {
        ensure!(
            defect.polygon.iter().flatten().all(|v| v.is_finite()),
            "Defect coordinates must be finite."
        );
        if defect.polygon.len() < 3 {
            continue;
        }
        let mut border = defect.polygon.clone();
        border.push(border[0]);
        line(border, "#b3261e");
        if let Some([x, y, max_x, max_y]) = bbox(&defect.polygon) {
            // Clip diagonal hatching to an arbitrary simple polygon using edge intersections.
            let count = ((max_x - x + max_y - y) / 5.0).ceil();
            ensure!(
                count <= 100_000.0,
                "Defect hatching would exceed 100,000 lines."
            );
            for i in 0..=count as usize {
                let c = x - max_y + i as f64 * 5.0;
                let mut hits = Vec::new();
                for (&a, &b) in defect
                    .polygon
                    .iter()
                    .zip(defect.polygon.iter().cycle().skip(1))
                    .take(defect.polygon.len())
                {
                    let (sa, sb) = (a[0] - a[1] - c, b[0] - b[1] - c);
                    if (sa >= 0.0) != (sb >= 0.0) {
                        let t = sa / (sa - sb);
                        hits.push([a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]);
                    }
                }
                hits.sort_by(|a, b| a[0].total_cmp(&b[0]));
                for pair in hits.as_chunks::<2>().0 {
                    line(pair.to_vec(), "#b3261e");
                }
            }
        }
    }
    let mut grid = |axis: usize, period: f64, offset: f64, colour| -> anyhow::Result<()> {
        ensure!(
            period.is_finite() && period > 0.0 && offset.is_finite(),
            "Guide spacing must be finite and positive."
        );
        let extent = if axis == 0 { scene.length } else { scene.width };
        let start = offset.rem_euclid(period);
        if start > extent {
            return Ok(());
        }
        let count = ((extent - start) / period).floor().max(0.0);
        ensure!(
            count <= 100_000.0,
            "Repeat/shade grid would exceed 100,000 lines."
        );
        for i in 0..=count as usize {
            let v = start + i as f64 * period;
            line(
                if axis == 0 {
                    vec![[v, 0.0], [v, scene.width]]
                } else {
                    vec![[0.0, v], [scene.length, v]]
                },
                colour,
            );
        }
        Ok(())
    };
    if let Some(repeat) = job.fabric.repeat {
        if let Some(p) = repeat.period_x {
            grid(0, p, repeat.offset_x, "#dddddd")?;
        }
        if let Some(p) = repeat.period_y {
            grid(1, p, repeat.offset_y, "#dddddd")?;
        }
    }
    if let Some(shade) = job.fabric.shade {
        grid(
            if shade.axis == ShadeAxis::Length {
                0
            } else {
                1
            },
            shade.band_mm,
            0.0,
            "#b08738",
        )?;
    }
    Ok(())
}

fn side(piece: &beta_marker_core::Piece, point: Pt) -> f64 {
    piece
        .fold_line
        .map(|[a, b]| (b[0] - a[0]) * (point[1] - a[1]) - (b[1] - a[1]) * (point[0] - a[0]))
        .unwrap_or(0.)
}
fn visible(piece: &beta_marker_core::Piece, pl: &beta_marker_core::Placement, point: Pt) -> bool {
    !pl.on_fold || side(piece, point) >= -1e-9
}
fn clipped_lines(
    piece: &beta_marker_core::Piece,
    pl: &beta_marker_core::Placement,
    points: &[Pt],
) -> Vec<Vec<Pt>> {
    if !pl.on_fold {
        return vec![points.to_vec()];
    }
    let mut segments = Vec::new();
    for pair in points.windows(2) {
        let (mut a, mut b) = (pair[0], pair[1]);
        let (sa, sb) = (side(piece, a), side(piece, b));
        if sa < 0. && sb < 0. {
            continue;
        }
        if (sa >= 0.) != (sb >= 0.) {
            let t = sa / (sa - sb);
            let hit = [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])];
            if sa < 0. {
                a = hit;
            } else {
                b = hit;
            }
        }
        segments.push(vec![a, b]);
    }
    segments
}
