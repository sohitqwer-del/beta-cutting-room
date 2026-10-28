//! Contact events for horizontal translation of polygon outlines.
//!
//! A concave pair can have several separate collision intervals. Binary search
//! over its entire bounding box can skip a usable pocket. Segment/vertex
//! contacts enumerate the interval boundaries, including positive clearance.
use beta_marker_core::Pt;

pub(super) fn convex(points: &[Pt]) -> bool {
    let (mut positive, mut negative) = (false, false);
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        let c = points[(i + 2) % points.len()];
        let cross = (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0]);
        positive |= cross > 0.0;
        negative |= cross < 0.0;
        if positive && negative {
            return false;
        }
    }
    true
}

/// Candidate x translations, in increasing order. Every candidate must still
/// pass the exact polygon/distance predicate; these events alone prove no fit.
pub(super) fn events(
    moving: &[Pt],
    obstacle: &[Pt],
    y: f64,
    clearance: f64,
    low: f64,
    high: f64,
    mut stopped: impl FnMut() -> bool,
) -> Option<Vec<f64>> {
    let mut xs = vec![high];
    let mut add = |x: f64| {
        if x.is_finite() && x > low + 1e-9 && x < high {
            xs.push(x);
        }
    };
    // Moving vertex against an obstacle edge, with the projected contact on
    // that edge. The two signs cover both sides and both polygon windings.
    for j in 0..obstacle.len() {
        let a = obstacle[j];
        let b = obstacle[(j + 1) % obstacle.len()];
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let length = dx.hypot(dy);
        if dy == 0.0 || length == 0.0 {
            continue;
        }
        for (i, p) in moving.iter().enumerate() {
            if i % 64 == 0 && stopped() {
                return None;
            }
            let cross = dx * (p[1] + y - a[1]) - dy * (p[0] - a[0]);
            for sign in [-1.0, 1.0] {
                let x = (cross + sign * clearance * length) / dy;
                let projection =
                    ((p[0] + x - a[0]) * dx + (p[1] + y - a[1]) * dy) / (length * length);
                if (-1e-9..=1.0 + 1e-9).contains(&projection) {
                    add(x);
                }
            }
        }
    }
    // Obstacle vertex against a moving edge.
    for j in 0..moving.len() {
        let a = moving[j];
        let b = moving[(j + 1) % moving.len()];
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let length = dx.hypot(dy);
        if dy == 0.0 || length == 0.0 {
            continue;
        }
        for (i, p) in obstacle.iter().enumerate() {
            if i % 64 == 0 && stopped() {
                return None;
            }
            let cross = dx * (p[1] - y - a[1]) - dy * (p[0] - a[0]);
            for sign in [-1.0, 1.0] {
                let x = (-cross + sign * clearance * length) / dy;
                let projection =
                    ((p[0] - x - a[0]) * dx + (p[1] - y - a[1]) * dy) / (length * length);
                if (-1e-9..=1.0 + 1e-9).contains(&projection) {
                    add(x);
                }
            }
        }
    }
    // Round clearance around edge endpoints: segment contacts do not cover
    // a closest pair of vertices outside either segment's projection.
    for p in moving {
        for (i, q) in obstacle.iter().enumerate() {
            if i % 64 == 0 && stopped() {
                return None;
            }
            let dy = p[1] + y - q[1];
            if dy.abs() <= clearance {
                let dx = ((clearance - dy.abs()) * (clearance + dy.abs()))
                    .max(0.0)
                    .sqrt();
                add(q[0] - p[0] - dx);
                add(q[0] - p[0] + dx);
            }
        }
    }
    if stopped() {
        return None;
    }
    xs.sort_by(f64::total_cmp);
    xs.dedup_by(|a, b| (*a - *b).abs() < 1e-8);
    Some(xs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contact_events_include_both_windings_and_positive_clearance() {
        let small = vec![[0., 0.], [10., 0.], [10., 20.], [0., 20.]];
        let pocket = vec![
            [0., 0.],
            [100., 0.],
            [100., 100.],
            [40., 100.],
            [40., 20.],
            [20., 20.],
            [20., 100.],
            [0., 100.],
        ];
        assert!(convex(&small));
        assert!(!convex(&pocket));
        for mut a in [small.clone(), small.iter().copied().rev().collect()] {
            for b in [&pocket, &pocket.iter().copied().rev().collect()] {
                for clearance in [0.002, 2.002] {
                    let xs =
                        events(&a, b, 40.0, clearance, 0.0, 100.0 + clearance, || false).unwrap();
                    assert!(xs.iter().any(|&x| (x - (20.0 + clearance)).abs() < 1e-7));
                }
            }
            // The reverse vertex/edge calculation is also needed when the
            // concave outline moves and the small obstacle is stationary.
            a.reverse();
            let xs = events(&pocket, &a, -40.0, 2.002, -100.0, 100.0, || false).unwrap();
            assert!(xs.iter().all(|x| x.is_finite()));
        }
    }

    #[test]
    fn stopped_contact_enumeration_yields_without_a_candidate() {
        let square = [[0., 0.], [10., 0.], [10., 10.], [0., 10.]];
        assert!(events(&square, &square, 0.0, 1.0, 0.0, 20.0, || true).is_none());
    }
}
