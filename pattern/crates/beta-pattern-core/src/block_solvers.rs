//! Bounded scalar solves for original block curves; independent of document layout.
use crate::geometry::{flatten_cubic, length};
use anyhow::{Result, ensure};

fn solve(target: f64, low: f64, high: f64, length_at: impl Fn(f64) -> Result<f64>) -> Result<f64> {
    ensure!(
        target.is_finite() && high > low,
        "Invalid block curve dimensions"
    );
    ensure!(
        length_at(low)? <= target && length_at(high)? >= target,
        "The requested seam length cannot fit these dimensions. Review the chart or ease."
    );
    let (mut a, mut b) = (low, high);
    for _ in 0..40 {
        let mid = (a + b) / 2.;
        if length_at(mid)? < target {
            a = mid
        } else {
            b = mid
        }
    }
    Ok((a + b) / 2.)
}
/// Height of a symmetric S-shaped cap at fixed full bicep width.
pub fn cap_height(width: f64, half_length: f64, max_height: f64) -> Result<f64> {
    ensure!(
        width > 0. && width <= 3000. && max_height > 1. && max_height <= 3000.,
        "Check sleeve width and length"
    );
    solve(half_length, 0.01, max_height, |h| {
        Ok(length(&flatten_cubic(
            [
                [width / 2., 0.],
                [width * 0.27, 0.],
                [width * 0.22, h],
                [0., h],
            ],
            0.005,
        )?))
    })
}
/// Vertical span of a seam with vertical tangents at both ends and fixed lateral offset.
pub fn seam_height(dx: f64, target: f64) -> Result<f64> {
    ensure!(
        dx.is_finite() && target > 0. && target <= 5000.,
        "Check inseam dimensions"
    );
    solve(target, 0.01, target, |h| {
        Ok(length(&flatten_cubic(
            [[0., 0.], [0., h / 3.], [dx, 2. * h / 3.], [dx, h]],
            0.005,
        )?))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn solves_lengths_without_changing_width() {
        let h = cap_height(380., 240., 300.).unwrap();
        let p = flatten_cubic([[190., 0.], [102.6, 0.], [83.6, h], [0., h]], 0.001).unwrap();
        assert!((length(&p) - 240.).abs() < 0.02);
        let h = seam_height(25., 750.).unwrap();
        assert!(
            (length(
                &flatten_cubic(
                    [[0., 0.], [0., h / 3.], [25., h * 2. / 3.], [25., h]],
                    0.001
                )
                .unwrap()
            ) - 750.)
                .abs()
                < 0.02
        );
        assert!(cap_height(380., 100., 300.).is_err());
        assert!(seam_height(800., 750.).is_err());
    }
}
