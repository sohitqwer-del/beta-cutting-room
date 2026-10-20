//! Small geometry helpers shared by all crates. Heavy geometry (overlap tests, offsets)
//! lives in the validator and the nesting adapter.

use crate::model::{Placement, Pt};

/// Signed area (shoelace). Positive for counter-clockwise polygons.
pub fn signed_area(poly: &[Pt]) -> f64 {
    let n = poly.len();
    if n < 3 {
        return 0.0;
    }
    let mut s = 0.0;
    for i in 0..n {
        let [x1, y1] = poly[i];
        let [x2, y2] = poly[(i + 1) % n];
        s += x1 * y2 - x2 * y1;
    }
    s / 2.0
}

/// Absolute polygon area.
pub fn area(poly: &[Pt]) -> f64 {
    signed_area(poly).abs()
}

/// Axis-aligned bounding box `[min_x, min_y, max_x, max_y]`. `None` for an empty slice.
pub fn bbox(points: &[Pt]) -> Option<[f64; 4]> {
    let first = points.first()?;
    let mut b = [first[0], first[1], first[0], first[1]];
    for p in &points[1..] {
        b[0] = b[0].min(p[0]);
        b[1] = b[1].min(p[1]);
        b[2] = b[2].max(p[0]);
        b[3] = b[3].max(p[1]);
    }
    Some(b)
}

/// Normalise an angle to `[0, 360)`.
pub fn norm_deg(a: f64) -> f64 {
    let r = a.rem_euclid(360.0);
    if (r - 360.0).abs() < 1e-9 { 0.0 } else { r }
}

/// Smallest absolute difference between two angles, degrees, in `[0, 180]`.
pub fn angle_diff_deg(a: f64, b: f64) -> f64 {
    let d = norm_deg(a - b);
    d.min(360.0 - d)
}

/// Apply a placement transform to one piece point: `R(rot) * M(p) + t`.
pub fn transform_point(p: Pt, mirrored: bool, rotation_deg: f64, translate: Pt) -> Pt {
    let x = if mirrored { -p[0] } else { p[0] };
    let y = p[1];
    let (s, c) = rotation_deg.to_radians().sin_cos();
    [c * x - s * y + translate[0], s * x + c * y + translate[1]]
}

/// Apply a placement transform to a polygon or polyline.
pub fn transform_points(points: &[Pt], mirrored: bool, rotation_deg: f64, translate: Pt) -> Vec<Pt> {
    points.iter().map(|&p| transform_point(p, mirrored, rotation_deg, translate)).collect()
}

/// Transform piece points with a [`Placement`].
pub fn place(points: &[Pt], pl: &Placement) -> Vec<Pt> {
    transform_points(points, pl.mirrored, pl.rotation_deg, pl.translate)
}

/// Rotation (degrees, CCW) that turns the grain line of a piece to point along `+x`.
///
/// `grain` is `[from, to]` in piece coordinates. For a mirrored copy the grain is mirrored
/// first. `None` or a zero-length grain line gives 0.
pub fn grain_alignment_deg(grain: Option<[Pt; 2]>, mirrored: bool) -> f64 {
    let Some([a, b]) = grain else { return 0.0 };
    let mut dx = b[0] - a[0];
    let dy = b[1] - a[1];
    if mirrored {
        dx = -dx;
    }
    if dx.hypot(dy) < 1e-12 {
        return 0.0;
    }
    norm_deg(-dy.atan2(dx).to_degrees())
}

/// Clip a polygon to the half-plane on the left of the directed line `a -> b`
/// (Sutherland-Hodgman). Used for halves of symmetric pieces placed on a fold.
pub fn clip_left_of_line(poly: &[Pt], a: Pt, b: Pt) -> Vec<Pt> {
    let side = |p: Pt| (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
    let mut out = Vec::with_capacity(poly.len() + 2);
    let n = poly.len();
    for i in 0..n {
        let (p, q) = (poly[i], poly[(i + 1) % n]);
        let (sp, sq) = (side(p), side(q));
        if sp >= 0.0 {
            out.push(p);
        }
        if (sp >= 0.0) != (sq >= 0.0) {
            let t = sp / (sp - sq);
            out.push([p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn clip_half_square() {
        let sq = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        // Line x = 5 going up: left side is x <= 5.
        let half = clip_left_of_line(&sq, [5.0, 0.0], [5.0, 10.0]);
        assert!((area(&half) - 50.0).abs() < 1e-9);
        assert!(half.iter().all(|p| p[0] <= 5.0 + 1e-9));
    }

    use super::*;

    #[test]
    fn square_area() {
        let sq = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        assert!((signed_area(&sq) - 100.0).abs() < 1e-9);
        let cw: Vec<Pt> = sq.iter().rev().copied().collect();
        assert!((signed_area(&cw) + 100.0).abs() < 1e-9);
        assert!((area(&cw) - 100.0).abs() < 1e-9);
    }

    #[test]
    fn transform_mirror_then_rotate() {
        // (1, 0) mirrored -> (-1, 0), rotated 90 CCW -> (0, -1), translated -> (5, 4)
        let p = transform_point([1.0, 0.0], true, 90.0, [5.0, 5.0]);
        assert!((p[0] - 5.0).abs() < 1e-9 && (p[1] - 4.0).abs() < 1e-9);
    }

    #[test]
    fn grain_alignment_points_grain_to_plus_x() {
        // Grain along +y: rotate -90 (= 270) to point along +x.
        let g = Some([[0.0, 0.0], [0.0, 10.0]]);
        let a = grain_alignment_deg(g, false);
        assert!((a - 270.0).abs() < 1e-9);
        let p = transform_point([0.0, 10.0], false, a, [0.0, 0.0]);
        assert!((p[0] - 10.0).abs() < 1e-9 && p[1].abs() < 1e-9);
        // Mirrored grain along (+1, +1) becomes (-1, +1).
        let g = Some([[0.0, 0.0], [1.0, 1.0]]);
        let a = grain_alignment_deg(g, true);
        let p = transform_point([1.0, 1.0], true, a, [0.0, 0.0]);
        assert!(p[0] > 0.0 && p[1].abs() < 1e-9);
    }

    #[test]
    fn angle_helpers() {
        assert_eq!(norm_deg(-90.0), 270.0);
        assert!((angle_diff_deg(359.0, 1.0) - 2.0).abs() < 1e-9);
    }
}
