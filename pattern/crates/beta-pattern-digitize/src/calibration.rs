//! Original square-code detection and robust planar/lens calibration.
use crate::{
    geometry::{self, Homography, distance, fit_homography, segment_distance},
    target::{Target, black_cell, checksum},
};
use anyhow::{Result, ensure};
use beta_marker_core::Pt;
use image::{GrayImage, imageops::FilterType};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A decoded square, ordered in the marker's own orientation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Detection {
    /// Identifier verified with CRC and orientation cells.
    pub id: u8,
    /// Outside corners in original image pixels, clockwise from code top-left.
    pub corners: [Pt; 4],
}
/// Radial camera model about the image centre; no tangential terms.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Radial {
    /// Distortion coefficient in `r_distorted = r * (1 + k1*r^2)`.
    pub k1: f64,
    /// Centre in pixels.
    pub centre: Pt,
    /// Half the larger image dimension; makes radii dimensionless.
    pub scale: f64,
}
impl Radial {
    /// Apply distortion to an ideal pixel point.
    pub fn distort(&self, p: Pt) -> Pt {
        let x = p[0] - self.centre[0];
        let y = p[1] - self.centre[1];
        let f = 1. + self.k1 * (x * x + y * y) / (self.scale * self.scale);
        [self.centre[0] + x * f, self.centre[1] + y * f]
    }
    /// Invert the radial model by Newton iteration.
    pub fn undistort(&self, p: Pt) -> Pt {
        let x = p[0] - self.centre[0];
        let y = p[1] - self.centre[1];
        let rd = x.hypot(y) / self.scale;
        if rd < 1e-12 {
            return p;
        }
        let mut r = rd;
        for _ in 0..12 {
            let derivative = 1. + 3. * self.k1 * r * r;
            if derivative < 0.1 {
                break;
            }
            r -= (r * (1. + self.k1 * r * r) - rd) / derivative;
        }
        [self.centre[0] + x * r / rd, self.centre[1] + y * r / rd]
    }
}
/// Calibration and quality measurements. Board coordinates run down in mm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Calibration {
    /// Board mm to undistorted image pixels.
    pub homography: Homography,
    /// Fitted or zero radial term.
    pub radial: Radial,
    /// Number of corner correspondences retained by RANSAC.
    pub inlier_corners: usize,
    /// Root-mean-square reprojection error in image pixels.
    pub rms_px: f64,
}
impl Calibration {
    /// Map board mm to the photographed pixel.
    pub fn project(&self, p: Pt) -> Option<Pt> {
        self.homography.map(p).map(|q| self.radial.distort(q))
    }
    /// Map photographed pixels to board mm.
    pub fn unproject(&self, p: Pt) -> Option<Pt> {
        self.homography
            .inverse()
            .ok()?
            .map(self.radial.undistort(p))
    }
}

/// Bilinear sampling. Pixel centres have coordinates `(x+0.5,y+0.5)`.
pub(crate) fn sample(image: &GrayImage, p: Pt) -> Option<f64> {
    let x = p[0] - 0.5;
    let y = p[1] - 0.5;
    if !x.is_finite()
        || !y.is_finite()
        || x < 0.
        || y < 0.
        || x > image.width() as f64 - 1.
        || y > image.height() as f64 - 1.
    {
        return None;
    }
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(image.width() - 1);
    let y1 = (y0 + 1).min(image.height() - 1);
    let dx = x - x0 as f64;
    let dy = y - y0 as f64;
    Some(
        (1. - dy) * ((1. - dx) * image[(x0, y0)][0] as f64 + dx * image[(x1, y0)][0] as f64)
            + dy * ((1. - dx) * image[(x0, y1)][0] as f64 + dx * image[(x1, y1)][0] as f64),
    )
}

/// Four-connected components. Tiny components are discarded before returning.
pub(crate) fn components(mask: &[u8], w: usize, h: usize, min: usize) -> Vec<Vec<u32>> {
    let mut todo = mask.to_vec();
    let mut result = Vec::new();
    let mut queue = Vec::new();
    for start in 0..todo.len() {
        if todo[start] == 0 {
            continue;
        }
        queue.clear();
        queue.push(start as u32);
        todo[start] = 0;
        let mut head = 0;
        while head < queue.len() {
            let i = queue[head] as usize;
            head += 1;
            let x = i % w;
            let y = i / w;
            for next in [
                if x > 0 { Some(i - 1) } else { None },
                if x + 1 < w { Some(i + 1) } else { None },
                if y > 0 { Some(i - w) } else { None },
                if y + 1 < h { Some(i + w) } else { None },
            ]
            .into_iter()
            .flatten()
            {
                if todo[next] != 0 {
                    todo[next] = 0;
                    queue.push(next as u32);
                }
            }
        }
        if queue.len() >= min {
            result.push(queue.clone());
        }
    }
    result
}

pub(crate) fn quad(points: &[Pt]) -> Option<[Pt; 4]> {
    let mut h = geometry::hull(points.to_vec());
    if h.len() < 4 {
        return None;
    }
    while h.len() > 4 {
        let k = (0..h.len()).min_by(|&i, &j| {
            let loss = |k: usize| {
                geometry::area(&[h[(k + h.len() - 1) % h.len()], h[k], h[(k + 1) % h.len()]]).abs()
            };
            loss(i).total_cmp(&loss(j))
        })?;
        h.remove(k);
    }
    if geometry::area(&h).abs() < 50. {
        return None;
    }
    // Fit each outer edge with total least squares using nearby boundary points.
    let mut lines = Vec::new();
    for i in 0..4 {
        let a = h[i];
        let b = h[(i + 1) % 4];
        let tolerance = distance(a, b) * 0.025 + 0.5;
        let near: Vec<_> = points
            .iter()
            .copied()
            .filter(|&p| {
                segment_distance(p, a, b) < tolerance
                    && distance(p, a) > tolerance * 2.
                    && distance(p, b) > tolerance * 2.
            })
            .collect();
        if near.len() < 4 {
            return None;
        }
        let n = near.len() as f64;
        let c = [
            near.iter().map(|p| p[0]).sum::<f64>() / n,
            near.iter().map(|p| p[1]).sum::<f64>() / n,
        ];
        let xx = near.iter().map(|p| (p[0] - c[0]).powi(2)).sum::<f64>();
        let yy = near.iter().map(|p| (p[1] - c[1]).powi(2)).sum::<f64>();
        let xy = near
            .iter()
            .map(|p| (p[0] - c[0]) * (p[1] - c[1]))
            .sum::<f64>();
        let angle = 0.5 * (2. * xy).atan2(xx - yy);
        let normal = [-angle.sin(), angle.cos()];
        lines.push((normal, normal[0] * c[0] + normal[1] * c[1]));
    }
    let mut corners = [[0.; 2]; 4];
    for i in 0..4 {
        let (a, u) = lines[(i + 3) % 4];
        let (b, v) = lines[i];
        let det = a[0] * b[1] - a[1] * b[0];
        if det.abs() < 0.1 {
            return None;
        }
        corners[i] = [(u * b[1] - a[1] * v) / det, (a[0] * v - u * b[0]) / det];
    }
    Some(corners)
}

/// Detect the original BETA codes. No commercial fiducial family is used.
/// Detection uses a reduced image, but returned coordinates use the original size.
pub fn detect_markers(image: &GrayImage) -> Result<Vec<Detection>> {
    ensure!(
        image.width() >= 64 && image.height() >= 64,
        "image is too small for calibration"
    );
    let scale = (2400. / image.width().max(image.height()) as f64).min(1.);
    let reduced = image::imageops::resize(
        image,
        (image.width() as f64 * scale).round() as u32,
        (image.height() as f64 * scale).round() as u32,
        FilterType::Triangle,
    );
    let w = reduced.width() as usize;
    let h = reduced.height() as usize;
    let mut integral = vec![0_u64; (w + 1) * (h + 1)];
    for y in 0..h {
        let mut row = 0;
        for x in 0..w {
            row += reduced[(x as u32, y as u32)][0] as u64;
            integral[(y + 1) * (w + 1) + x + 1] = integral[y * (w + 1) + x + 1] + row;
        }
    }
    let mut mask = vec![0; w * h];
    let radius = (w.min(h) / 50).max(12);
    for y in 0..h {
        for x in 0..w {
            let x0 = x.saturating_sub(radius);
            let y0 = y.saturating_sub(radius);
            let x1 = (x + radius + 1).min(w);
            let y1 = (y + radius + 1).min(h);
            let sum = integral[y1 * (w + 1) + x1] + integral[y0 * (w + 1) + x0]
                - integral[y1 * (w + 1) + x0]
                - integral[y0 * (w + 1) + x1];
            let mean = sum as f64 / ((x1 - x0) * (y1 - y0)) as f64;
            mask[y * w + x] =
                u8::from((reduced[(x as u32, y as u32)][0] as f64) < (mean * 0.60).min(110.));
        }
    }
    drop(integral);
    let mut detections = BTreeMap::<u8, Option<Detection>>::new();
    for component in components(&mask, w, h, 60) {
        if component.len() > w * h / 10 {
            continue;
        }
        let mut min = [w, h];
        let mut max = [0, 0];
        for &i in &component {
            let x = i as usize % w;
            let y = i as usize / w;
            min[0] = min[0].min(x);
            min[1] = min[1].min(y);
            max[0] = max[0].max(x);
            max[1] = max[1].max(y);
        }
        let bw = max[0] - min[0] + 1;
        let bh = max[1] - min[1] + 1;
        if bw < 18 || bh < 18 || bw > bh * 4 || bh > bw * 4 || component.len() * 5 < bw * bh {
            continue;
        }
        let mut boundary = Vec::new();
        for &i in &component {
            let x = i as usize % w;
            let y = i as usize / w;
            // Pixel-edge centres avoid a half-pixel expansion of the code.
            if x > 0 && mask[i as usize - 1] == 0 {
                boundary.push([x as f64, y as f64 + 0.5]);
            }
            if x + 1 < w && mask[i as usize + 1] == 0 {
                boundary.push([x as f64 + 1., y as f64 + 0.5]);
            }
            if y > 0 && mask[i as usize - w] == 0 {
                boundary.push([x as f64 + 0.5, y as f64]);
            }
            if y + 1 < h && mask[i as usize + w] == 0 {
                boundary.push([x as f64 + 0.5, y as f64 + 1.]);
            }
        }
        let Some(corners) = quad(&boundary) else {
            continue;
        };
        let ideal = [[0., 0.], [8., 0.], [8., 8.], [0., 8.]];
        let Ok(map) = fit_homography(&ideal.into_iter().zip(corners).collect::<Vec<_>>()) else {
            continue;
        };
        let mut levels = [[0.; 8]; 8];
        let mut good = true;
        for (y, row) in levels.iter_mut().enumerate() {
            for (x, v) in row.iter_mut().enumerate() {
                if let Some(value) = map
                    .map([x as f64 + 0.5, y as f64 + 0.5])
                    .and_then(|p| sample(&reduced, p))
                {
                    *v = value;
                } else {
                    good = false;
                }
            }
        }
        if !good {
            continue;
        }
        let lo = levels.iter().flatten().copied().fold(255., f64::min);
        let hi = levels.iter().flatten().copied().fold(0., f64::max);
        if hi - lo < 60. {
            continue;
        }
        let threshold = (lo + hi) * 0.5;
        for rot in 0..4 {
            let bit = |x: usize, y: usize| {
                let (x, y) = match rot {
                    0 => (x, y),
                    1 => (7 - y, x),
                    2 => (7 - x, 7 - y),
                    _ => (y, 7 - x),
                };
                levels[y][x] < threshold
            };
            let mut word = 0_u16;
            for y in 2..6 {
                for x in 2..6 {
                    if bit(x, y) {
                        word |= 1 << ((y - 2) * 4 + x - 2);
                    }
                }
            }
            let id = word as u8;
            if (word >> 8) as u8 != checksum(id) {
                continue;
            }
            if (0..8).any(|y| (0..8).any(|x| bit(x, y) != black_cell(id, x, y))) {
                continue;
            }
            let detection = Detection {
                id,
                corners: std::array::from_fn(|i| {
                    let p = corners[(i + rot) % 4];
                    [
                        p[0] * image.width() as f64 / w as f64,
                        p[1] * image.height() as f64 / h as f64,
                    ]
                }),
            };
            // Duplicate printed IDs are ambiguous. Refuse both, rather than pick one.
            detections
                .entry(id)
                .and_modify(|d| *d = None)
                .or_insert(Some(detection));
            break;
        }
    }
    Ok(detections.into_values().flatten().collect())
}

/// Estimate a homography and, optionally, one radial term from many markers.
/// At least four distinct markers, spanning a broad part of the board, are required.
pub fn calibrate(
    target: &Target,
    detections: &[Detection],
    dimensions: [u32; 2],
    fit_radial: bool,
) -> Result<Calibration> {
    target.validate()?;
    let known: BTreeMap<_, _> = target.markers.iter().map(|m| (m.id, m)).collect();
    let matched: Vec<_> = detections
        .iter()
        .filter_map(|d| known.get(&d.id).map(|m| (*m, d)))
        .collect();
    let unique: std::collections::BTreeSet<_> = matched.iter().map(|(_, d)| d.id).collect();
    ensure!(
        unique.len() == matched.len(),
        "duplicate marker IDs in calibration input"
    );
    ensure!(
        matched.len() >= 4,
        "need at least four distinct visible BETA markers; found {}",
        matched.len()
    );
    let centres: Vec<_> = matched
        .iter()
        .map(|(m, _)| [m.origin[0] + m.size_mm / 2., m.origin[1] + m.size_mm / 2.])
        .collect();
    ensure!(
        geometry::area(&geometry::hull(centres)).abs() > target.width_mm * target.height_mm * 0.15,
        "visible markers cover too little of the board"
    );
    let raw: Vec<_> = matched
        .iter()
        .flat_map(|(m, d)| m.corners().into_iter().zip(d.corners))
        .collect();
    ensure!(
        raw.iter().all(|(_, p)| p.iter().all(|v| v.is_finite())),
        "invalid detected corners"
    );
    let mut radial = Radial {
        k1: 0.,
        centre: [dimensions[0] as f64 / 2., dimensions[1] as f64 / 2.],
        scale: dimensions[0].max(dimensions[1]) as f64 / 2.,
    };
    ensure!(radial.scale > 0., "empty photograph");
    if fit_radial {
        ensure!(
            matched.len() >= 8,
            "radial fitting needs at least eight well-spread markers"
        );
        let score = |k: f64| -> f64 {
            let lens = Radial { k1: k, ..radial };
            let pairs: Vec<_> = raw.iter().map(|&(a, b)| (a, lens.undistort(b))).collect();
            let Ok(h) = fit_homography(&pairs) else {
                return f64::INFINITY;
            };
            let mut residuals: Vec<_> = pairs
                .iter()
                .filter_map(|(a, b)| h.map(*a).map(|q| distance(q, *b).powi(2)))
                .collect();
            residuals.sort_by(f64::total_cmp);
            residuals.iter().take(residuals.len() * 9 / 10).sum::<f64>()
        };
        let mut best = (0., score(0.));
        for i in -20..=20 {
            let k = i as f64 * 0.01;
            let s = score(k);
            if s < best.1 {
                best = (k, s);
            }
        }
        let coarse = best.0;
        for i in -10..=10 {
            let k = coarse + i as f64 * 0.001;
            if k.abs() <= 0.20 {
                let s = score(k);
                if s < best.1 {
                    best = (k, s);
                }
            }
        }
        radial.k1 = best.0;
    }
    let pairs: Vec<_> = raw.iter().map(|&(a, b)| (a, radial.undistort(b))).collect();
    let (h, ids) = geometry::ransac(&pairs, 3.5)?;
    ensure!(
        ids.len() >= 12 && ids.len() * 4 >= raw.len() * 3,
        "too many inconsistent marker corners"
    );
    let mm_hull = geometry::hull(ids.iter().map(|&i| pairs[i].0).collect());
    ensure!(
        geometry::area(&mm_hull).abs() > target.width_mm * target.height_mm * 0.15,
        "calibration inliers cover too little area"
    );
    let rms = (ids
        .iter()
        .filter_map(|&i| {
            h.map(raw[i].0)
                .map(|p| distance(radial.distort(p), raw[i].1).powi(2))
        })
        .sum::<f64>()
        / ids.len() as f64)
        .sqrt();
    ensure!(rms < 3., "calibration reprojection error exceeds 3 pixels");
    Ok(Calibration {
        homography: h,
        radial,
        inlier_corners: ids.len(),
        rms_px: rms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target::SheetSize;
    use image::Luma;

    fn board(target: &Target) -> GrayImage {
        let ppm = 3.;
        let mut image = GrayImage::from_pixel(
            (target.width_mm * ppm) as u32,
            (target.height_mm * ppm) as u32,
            Luma([255]),
        );
        for r in target.rectangles(true) {
            for y in (r.y * ppm) as u32..((r.y + r.h) * ppm) as u32 {
                for x in (r.x * ppm) as u32..((r.x + r.w) * ppm) as u32 {
                    image[(x, y)] = Luma([r.grey]);
                }
            }
        }
        image
    }
    #[test]
    fn original_codes_decode_after_rotation_and_reject_damage() {
        let target = Target::tiled(SheetSize::A3, 1, 1, false).unwrap();
        let mut im = board(&target);
        let rotated = image::imageops::rotate90(&im);
        let found = detect_markers(&rotated).unwrap();
        assert_eq!(found.len(), target.markers.len());
        let m = &target.markers[0];
        let x0 = ((m.origin[0] + 6.) * 3.) as u32;
        let y0 = ((m.origin[1] + 6.) * 3.) as u32;
        // Flip a whole payload cell without changing the outside quadrilateral.
        for y in y0..y0 + 9 {
            for x in x0..x0 + 9 {
                im[(x, y)][0] = 255 - im[(x, y)][0];
            }
        }
        let damaged = detect_markers(&im).unwrap();
        assert!(!damaged.iter().any(|d| d.id == m.id));
        assert_eq!(damaged.len(), target.markers.len() - 1);
    }
    #[test]
    fn radial_fit_and_bad_coverage() {
        let target = Target::tiled(SheetSize::A3, 1, 1, false).unwrap();
        let h = Homography([7.3, 0.2, 200., -0.3, 7.6, 350., 0.0001, -0.0002, 1.]);
        let lens = Radial {
            k1: 0.035,
            centre: [2000., 1500.],
            scale: 2000.,
        };
        let detections: Vec<_> = target
            .markers
            .iter()
            .map(|m| Detection {
                id: m.id,
                corners: m.corners().map(|p| lens.distort(h.map(p).unwrap())),
            })
            .collect();
        let cal = calibrate(&target, &detections, [4000, 3000], true).unwrap();
        assert!((cal.radial.k1 - lens.k1).abs() < 0.0011);
        assert!(cal.rms_px < 0.1);
        assert!(calibrate(&target, &detections[..3], [4000, 3000], false).is_err());
        let bad = vec![detections[0].clone(); 4];
        assert!(calibrate(&target, &bad, [4000, 3000], false).is_err());
    }
    #[test]
    fn lens_round_trip() {
        let lens = Radial {
            k1: 0.05,
            centre: [2000., 1500.],
            scale: 2000.,
        };
        for p in [[200., 100.], [3900., 2600.], [2000., 1500.]] {
            assert!(distance(p, lens.undistort(lens.distort(p))) < 1e-7);
        }
    }
}
