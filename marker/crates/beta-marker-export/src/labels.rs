//! Interior label anchors on the exact displayed cut boundary.
use anyhow::{Context, Result, ensure};
use beta_marker_core::{
    Job, Placement, Pt,
    geometry::{bbox, transform_point},
    rules,
};
use serde::Serialize;
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap},
};

#[derive(Debug, Clone, Serialize)]
pub struct MarkerLabel {
    pub placement: usize,
    pub piece_id: String,
    pub name: String,
    pub size: Option<String>,
    pub anchor: Pt,
    pub rotation_deg: f64,
    pub clearance_mm: f64,
}

/// Labels follow shrinkage, folded halves, mirror transforms and fusing blocks.
pub fn marker_labels(job: &Job, placements: &[Placement]) -> Result<Vec<MarkerLabel>> {
    let job = rules::effective_job(job);
    let pieces: HashMap<_, _> = job.pieces.iter().map(|p| (p.id.as_str(), p)).collect();
    let mut anchors = HashMap::new();
    placements
        .iter()
        .enumerate()
        .map(|(index, placement)| {
            let piece = pieces
                .get(placement.piece_id.as_str())
                .context("Unknown label piece")?;
            ensure!(
                placement.rotation_deg.is_finite()
                    && placement.translate.iter().all(|v| v.is_finite()),
                "Invalid label transform"
            );
            let (local_anchor, clearance_mm, baseline) =
                match anchors.get(&(piece.id.as_str(), placement.on_fold)) {
                    Some(value) => *value,
                    None => {
                        // A fusing block may surround a concave pattern. Keep the label
                        // inside the pattern itself, which is also inside that block.
                        let outline = rules::placed_outline(
                            piece,
                            &Placement {
                                on_fold: placement.on_fold,
                                ..Default::default()
                            },
                        );
                        let (anchor, clearance) = interior(&outline)?;
                        let source = bbox(&outline).context("Empty label outline")?;
                        let angle = if source[3] - source[1] > source[2] - source[0] {
                            90.0
                        } else {
                            0.0
                        };
                        anchors.insert(
                            (piece.id.as_str(), placement.on_fold),
                            (anchor, clearance, angle),
                        );
                        (anchor, clearance, angle)
                    }
                };
            let anchor = transform_point(
                local_anchor,
                placement.mirrored,
                placement.rotation_deg,
                placement.translate,
            );
            let rotation_deg = (placement.rotation_deg
                + if placement.mirrored {
                    -baseline
                } else {
                    baseline
                }
                + 90.0)
                .rem_euclid(180.0)
                - 90.0;
            Ok(MarkerLabel {
                placement: index,
                piece_id: piece.id.clone(),
                name: piece.name.clone(),
                size: piece.size.clone(),
                anchor,
                rotation_deg,
                clearance_mm,
            })
        })
        .collect()
}

/// Signed Euclidean distance to the polygon; positive means inside.
fn distance(point: Pt, polygon: &[Pt]) -> f64 {
    let mut inside = false;
    let mut nearest = f64::INFINITY;
    for (a, b) in polygon
        .iter()
        .zip(polygon.iter().cycle().skip(1))
        .take(polygon.len())
    {
        if (a[1] > point[1]) != (b[1] > point[1])
            && point[0] < (b[0] - a[0]) * (point[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            inside = !inside;
        }
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let norm = dx * dx + dy * dy;
        let t = if norm > 0.0 {
            ((point[0] - a[0]) * dx + (point[1] - a[1]) * dy) / norm
        } else {
            0.0
        }
        .clamp(0.0, 1.0);
        nearest = nearest.min((point[0] - a[0] - t * dx).hypot(point[1] - a[1] - t * dy));
    }
    if inside { nearest } else { -nearest }
}

#[derive(Clone, Copy)]
struct Cell {
    center: Pt,
    half: Pt,
    distance: f64,
    upper: f64,
}
impl Cell {
    fn new(center: Pt, half: Pt, polygon: &[Pt]) -> Self {
        let distance = distance(center, polygon);
        Self {
            center,
            half,
            distance,
            upper: distance + half[0].hypot(half[1]),
        }
    }
}
impl PartialEq for Cell {
    fn eq(&self, rhs: &Self) -> bool {
        self.upper == rhs.upper
    }
}
impl Eq for Cell {}
impl PartialOrd for Cell {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        Some(self.cmp(rhs))
    }
}
impl Ord for Cell {
    fn cmp(&self, rhs: &Self) -> Ordering {
        self.upper.total_cmp(&rhs.upper)
    }
}

/// Bounded best-first subdivision, with a scanline seed that is inside even a
/// thin concave piece. A 0.5 mm target is enough for screen label placement.
fn interior(polygon: &[Pt]) -> Result<(Pt, f64)> {
    ensure!(
        polygon.len() >= 3
            && polygon
                .iter()
                .flatten()
                .all(|v| v.is_finite() && v.abs() < 1e12),
        "Invalid label outline"
    );
    let [x0, y0, x1, y1] = bbox(polygon).context("Empty label outline")?;
    ensure!(x1 > x0 && y1 > y0, "Label outline has no interior");
    let center = [(x0 + x1) / 2.0, (y0 + y1) / 2.0];
    // Pick an open Y interval, avoiding a line through vertices.
    let mut ys: Vec<_> = polygon.iter().map(|p| p[1]).collect();
    ys.sort_by(f64::total_cmp);
    ys.dedup();
    let pair = ys
        .windows(2)
        .min_by(|a, b| {
            ((a[0] + a[1]) / 2.0 - center[1])
                .abs()
                .total_cmp(&((b[0] + b[1]) / 2.0 - center[1]).abs())
        })
        .context("Label outline has no interior")?;
    let y = (pair[0] + pair[1]) / 2.0;
    let mut xs = vec![];
    for (a, b) in polygon
        .iter()
        .zip(polygon.iter().cycle().skip(1))
        .take(polygon.len())
    {
        if (a[1] > y) != (b[1] > y) {
            xs.push(a[0] + (y - a[1]) * (b[0] - a[0]) / (b[1] - a[1]));
        }
    }
    xs.sort_by(f64::total_cmp);
    #[allow(clippy::chunks_exact_to_as_chunks)] let pair = xs // 0.7.0: tested code kept; use as_chunks next round.
        .chunks_exact(2)
        .max_by(|a, b| (a[1] - a[0]).total_cmp(&(b[1] - b[0])))
        .context("Label outline has no interior")?;
    let mut best = Cell::new([(pair[0] + pair[1]) / 2.0, y], [0.0, 0.0], polygon);
    let root = Cell::new(center, [(x1 - x0) / 2.0, (y1 - y0) / 2.0], polygon);
    let mut queue = BinaryHeap::from([root]);
    for _ in 0..8192 {
        let Some(cell) = queue.pop() else {
            break;
        };
        if cell.distance > best.distance {
            best = cell;
        }
        if cell.upper - best.distance <= 0.5 {
            continue;
        }
        let axis = usize::from(cell.half[1] > cell.half[0]);
        let mut half = cell.half;
        half[axis] /= 2.0;
        for sign in [-1.0, 1.0] {
            let mut center = cell.center;
            center[axis] += sign * half[axis];
            queue.push(Cell::new(center, half, polygon));
        }
    }
    ensure!(best.distance > 0.0, "Label outline has no interior");
    Ok((best.center, best.distance))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn concave_and_thin_anchors_are_inside() {
        for polygon in [
            vec![
                [0., 0.],
                [100., 0.],
                [100., 10.],
                [10., 10.],
                [10., 100.],
                [0., 100.],
            ],
            vec![[0., 0.], [10000., 0.], [10000., 0.01], [0., 0.01]],
            vec![[0., 0.], [100., 0.], [50., 50.]],
        ] {
            let (anchor, clearance) = interior(&polygon).unwrap();
            assert!(distance(anchor, &polygon) > 0.0);
            assert_eq!(distance(anchor, &polygon), clearance);
        }
        assert!(interior(&[[0., 0.], [1., 0.], [2., 0.]]).is_err());
    }
    #[test]
    fn labels_follow_mirrors_folds_and_shrinkage() {
        let mut job = Job {
            pieces: vec![beta_marker_core::Piece {
                id: "panel".into(),
                name: "Panel".into(),
                size: Some("M".into()),
                outline: vec![[0., 0.], [100., 0.], [100., 40.], [0., 40.]],
                fold_line: Some([[0., 20.], [100., 20.]]),
                ..Default::default()
            }],
            ..Default::default()
        };
        job.fabric.shrinkage.length_pct = 5.0;
        for mirrored in [false, true] {
            for on_fold in [false, true] {
                let placement = Placement {
                    piece_id: "panel".into(),
                    mirrored,
                    on_fold,
                    rotation_deg: 180.0,
                    translate: [200., 100.],
                    ..Default::default()
                };
                let label = marker_labels(&job, std::slice::from_ref(&placement))
                    .unwrap()
                    .remove(0);
                let effective = rules::effective_job(&job);
                assert!(
                    distance(
                        label.anchor,
                        &rules::placed_outline(&effective.pieces[0], &placement)
                    ) > 0.0
                );
                assert!((-90.0..90.0).contains(&label.rotation_deg));
                assert_eq!(label.name, "Panel");
                assert_eq!(label.size.as_deref(), Some("M"));
            }
        }
    }

    #[test]
    fn fusing_block_does_not_put_label_outside_concave_pattern() {
        let job = Job {
            pieces: vec![beta_marker_core::Piece {
                id: "concave".into(),
                block_margin: Some(10.),
                outline: vec![
                    [0., 0.],
                    [100., 0.],
                    [100., 10.],
                    [10., 10.],
                    [10., 100.],
                    [0., 100.],
                ],
                ..Default::default()
            }],
            ..Default::default()
        };
        let placement = Placement {
            piece_id: "concave".into(),
            mirrored: true,
            rotation_deg: 37.,
            translate: [300., 300.],
            ..Default::default()
        };
        let label = marker_labels(&job, std::slice::from_ref(&placement))
            .unwrap()
            .remove(0);
        assert!(
            distance(
                label.anchor,
                &rules::placed_outline(&job.pieces[0], &placement)
            ) > 0.0
        );
        assert!(
            distance(
                label.anchor,
                &rules::cut_outline(&job.pieces[0], &placement)
            ) > 0.0
        );
    }
}
