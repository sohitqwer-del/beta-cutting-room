//! Reproducible synthetic photographs and symmetric metric accuracy measurements.
//! The renderer starts from independent known geometry, not detected outlines.
use crate::{
    calibration::{Radial, sample},
    geometry::{self, Homography, area, distance, fit_homography, outline_distance},
    target::Target,
};
use anyhow::{Result, ensure};
use beta_marker_core::{Mark, MarkKind, Piece, Pt};
use image::{GrayImage, Luma};
use serde::{Deserialize, Serialize};

/// Photographic degradations, with deterministic noise.
#[derive(Debug, Clone, Copy)]
pub struct PhotoOptions {
    /// Output pixel dimensions; 4000 by 3000 is 12 MP.
    pub dimensions: [u32; 2],
    /// Perspective camera rather than an almost frontal scan.
    pub perspective: bool,
    /// Gaussian blur standard deviation in photo pixels.
    pub blur_px: f32,
    /// Uniform additive noise half-range in 8-bit intensity levels.
    pub noise: f64,
    /// Smooth illumination loss across the field, 0..0.5.
    pub lighting: f64,
    /// Known radial distortion coefficient.
    pub k1: f64,
    /// Random seed.
    pub seed: u64,
}
impl Default for PhotoOptions {
    fn default() -> Self {
        Self {
            dimensions: [4000, 3000],
            perspective: true,
            blur_px: 0.9,
            noise: 4.,
            lighting: 0.28,
            k1: 0.035,
            seed: 42,
        }
    }
}
/// One synthetic photo with independent ground truth in the Piece coordinate system.
pub struct SyntheticPhoto {
    /// Rendered image.
    pub image: GrayImage,
    /// Physical outline and intentionally inserted marks; coordinates are mm, y up.
    pub truth: Piece,
    /// Known world-to-ideal-pixel map.
    pub homography: Homography,
    /// Known lens model.
    pub radial: Radial,
    /// Scale applied to the source garment piece to fit the A3 working area.
    pub source_scale: f64,
}
/// Quantitative recovery measurements. Distances sample both outlines every 0.5 mm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Accuracy {
    /// Symmetric mean nearest-segment distance, mm.
    pub mean_mm: f64,
    /// Largest sampled distance in either direction, mm.
    pub max_mm: f64,
    /// Absolute area difference relative to ground truth, percent.
    pub area_error_pct: f64,
    /// True positive notches within 2 mm of their known edge positions.
    pub notches_found: usize,
    /// Total intentional physical V cuts and drawn edge ticks.
    pub notches_expected: usize,
    /// Detected notches not matched to a known mark.
    pub notch_false_positives: usize,
    /// Drill detections within 2 mm of true centres.
    pub drills_found: usize,
    /// Acute grain-line angle error, if detected.
    pub grain_error_deg: Option<f64>,
}

/// Render a FreeSewing or other Piece on the original target, adding one V cut,
/// one edge tick, a drill dot, and a grain line for scored feature tests.
pub fn photograph(
    target: &Target,
    source: &Piece,
    options: &PhotoOptions,
) -> Result<SyntheticPhoto> {
    target.validate()?;
    ensure!(
        source.outline.len() >= 3 && source.outline.iter().flatten().all(|x| x.is_finite()),
        "source has no finite outline"
    );
    let [width, height] = options.dimensions;
    ensure!(
        (200..=8000).contains(&width) && (200..=8000).contains(&height),
        "synthetic dimensions out of range"
    );
    ensure!(
        options.blur_px.is_finite()
            && (0. ..=5.).contains(&options.blur_px)
            && options.noise.is_finite()
            && (0. ..=30.).contains(&options.noise)
            && options.lighting.is_finite()
            && (0. ..=0.5).contains(&options.lighting)
            && options.k1.is_finite()
            && options.k1.abs() <= 0.10,
        "invalid photo degradations"
    );
    let mut min = [f64::INFINITY; 2];
    let mut max = [f64::NEG_INFINITY; 2];
    for p in &source.outline {
        for k in 0..2 {
            min[k] = min[k].min(p[k]);
            max[k] = max[k].max(p[k]);
        }
    }
    ensure!(
        max[0] - min[0] > 1. && max[1] - min[1] > 1.,
        "source has zero extent"
    );
    let scale = ((target.width_mm - 100.) / (max[0] - min[0]))
        .min((target.height_mm - 100.) / (max[1] - min[1]))
        .min(1.);
    let offset = [
        (target.width_mm - (max[0] - min[0]) * scale) / 2.,
        (target.height_mm - (max[1] - min[1]) * scale) / 2.,
    ];
    let mut poly: Vec<Pt> = source
        .outline
        .iter()
        .map(|p| {
            [
                offset[0] + (p[0] - min[0]) * scale,
                offset[1] + (max[1] - p[1]) * scale,
            ]
        })
        .collect();
    if area(&poly) < 0. {
        poly.reverse();
    }
    let longest = (0..poly.len())
        .max_by(|&i, &j| {
            distance(poly[i], poly[(i + 1) % poly.len()])
                .total_cmp(&distance(poly[j], poly[(j + 1) % poly.len()]))
        })
        .unwrap_or(0);
    let a = poly[longest];
    let b = poly[(longest + 1) % poly.len()];
    let len = distance(a, b);
    ensure!(len > 25., "source has no edge long enough for test notches");
    let d = [(b[0] - a[0]) / len, (b[1] - a[1]) / len];
    let inside = [-d[1], d[0]];
    let notch = [(a[0] + b[0]) / 2., (a[1] + b[1]) / 2.];
    let tip = [notch[0] + inside[0] * 3., notch[1] + inside[1] * 3.];
    poly.splice(
        longest + 1..longest + 1,
        [
            [notch[0] - d[0] * 2., notch[1] - d[1] * 2.],
            tip,
            [notch[0] + d[0] * 2., notch[1] + d[1] * 2.],
        ],
    );
    let tick = [a[0] + d[0] * len * 0.25, a[1] + d[1] * len * 0.25];
    let tick_end = [tick[0] + inside[0] * 4., tick[1] + inside[1] * 4.];
    let centre = [target.width_mm / 2., target.height_mm / 2.];
    // Choose a real interior location rather than assuming the bounding-box centre
    // is inside a concave garment outline.
    let mut anchor = None;
    let mut clearance = 0.;
    for y in 0..20 {
        for x in 0..20 {
            let p = [
                offset[0] + (max[0] - min[0]) * scale * (x as f64 + 0.5) / 20.,
                offset[1] + (max[1] - min[1]) * scale * (y as f64 + 0.5) / 20.,
            ];
            if geometry::contains(&poly, p) {
                let c = outline_distance(p, &poly);
                if c > clearance {
                    clearance = c;
                    anchor = Some(p);
                }
            }
        }
    }
    let anchor = anchor.unwrap_or(centre);
    let grain_len = (clearance * 1.4).clamp(26., 80.);
    let grain = [
        [anchor[0], anchor[1] - grain_len / 2.],
        [anchor[0], anchor[1] + grain_len / 2.],
    ];
    let drill = [anchor[0] + (clearance * 0.5).min(15.), anchor[1]];
    ensure!(
        grain.iter().all(|&p| geometry::contains(&poly, p)) && geometry::contains(&poly, drill),
        "source too narrow for synthetic feature placement"
    );
    // Render at 8 px/mm; vector scale and photo resolution are independent.
    let ppm = 8.;
    let bw = (target.width_mm * ppm).ceil() as u32;
    let bh = (target.height_mm * ppm).ceil() as u32;
    ensure!(
        (bw as u64) * (bh as u64) <= 24_000_000,
        "synthetic board too large"
    );
    let mut board = GrayImage::from_pixel(bw, bh, Luma([255]));
    for r in target.rectangles(true) {
        let x0 = (r.x * ppm).round().max(0.) as u32;
        let y0 = (r.y * ppm).round().max(0.) as u32;
        let x1 = ((r.x + r.w) * ppm).round().min(bw as f64) as u32;
        let y1 = ((r.y + r.h) * ppm).round().min(bh as f64) as u32;
        for y in y0..y1 {
            for x in x0..x1 {
                board[(x, y)] = Luma([r.grey]);
            }
        }
    }
    // Scanline vector fill, without reusing the scanner's segmentation/tracer.
    for y in 0..bh {
        let py = (y as f64 + 0.5) / ppm;
        let mut intersections = Vec::new();
        for i in 0..poly.len() {
            let a = poly[i];
            let b = poly[(i + 1) % poly.len()];
            if (a[1] > py) != (b[1] > py) {
                intersections.push(a[0] + (b[0] - a[0]) * (py - a[1]) / (b[1] - a[1]));
            }
        }
        intersections.sort_by(f64::total_cmp);
        for pair in intersections.as_chunks::<2>().0 {
            let low = (pair[0] * ppm - 0.5).ceil().max(0.) as u32;
            let high = (pair[1] * ppm - 0.5).ceil().min(bw as f64) as u32;
            for x in low..high {
                board[(x, y)] = Luma([245]);
            }
        }
    }
    fn line(board: &mut GrayImage, a: Pt, b: Pt, ppm: f64, width: f64) {
        let lo = [
            ((a[0].min(b[0]) - width) * ppm).max(0.) as u32,
            ((a[1].min(b[1]) - width) * ppm).max(0.) as u32,
        ];
        let hi = [
            ((a[0].max(b[0]) + width) * ppm).min(board.width() as f64) as u32,
            ((a[1].max(b[1]) + width) * ppm).min(board.height() as f64) as u32,
        ];
        for y in lo[1]..hi[1] {
            for x in lo[0]..hi[0] {
                if geometry::segment_distance(
                    [(x as f64 + 0.5) / ppm, (y as f64 + 0.5) / ppm],
                    a,
                    b,
                ) <= width / 2.
                {
                    board[(x, y)] = Luma([15]);
                }
            }
        }
    }
    line(&mut board, grain[0], grain[1], ppm, 0.65);
    line(&mut board, tick, tick_end, ppm, 0.7);
    line(&mut board, drill, drill, ppm, 2.6);
    let dst = if options.perspective {
        [
            [0.080, 0.130],
            [0.915, 0.060],
            [0.955, 0.880],
            [0.105, 0.940],
        ]
    } else {
        [
            [0.065, 0.065],
            [0.935, 0.065],
            [0.935, 0.935],
            [0.065, 0.935],
        ]
    };
    let corners = [
        [0., 0.],
        [target.width_mm, 0.],
        [target.width_mm, target.height_mm],
        [0., target.height_mm],
    ];
    let h = fit_homography(
        &corners
            .into_iter()
            .zip(dst.map(|p| [p[0] * width as f64, p[1] * height as f64]))
            .collect::<Vec<_>>(),
    )?;
    let inverse = h.inverse()?;
    let radial = Radial {
        k1: options.k1,
        centre: [width as f64 / 2., height as f64 / 2.],
        scale: width.max(height) as f64 / 2.,
    };
    let mut photo = GrayImage::new(width, height);
    let mut rng = options.seed.max(1);
    for y in 0..height {
        for x in 0..width {
            let q = radial.undistort([x as f64 + 0.5, y as f64 + 0.5]);
            let v = inverse
                .map(q)
                .and_then(|p| sample(&board, [p[0] * ppm, p[1] * ppm]))
                .unwrap_or(145.);
            let nx = x as f64 / width as f64;
            let ny = y as f64 / height as f64;
            let lighting = 1.
                - options.lighting
                    * (0.6 * nx + 0.4 * ny + 0.18 * (nx * 6.).sin() * (ny * 3.).cos());
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let noise = ((rng & 65535) as f64 / 32767.5 - 1.) * options.noise;
            photo[(x, y)] = Luma([(v * lighting + noise).round().clamp(0., 255.) as u8]);
        }
    }
    if options.blur_px > 0. {
        photo = image::imageops::blur(&photo, options.blur_px);
    }
    let flip = |p: Pt| [p[0], target.height_mm - p[1]];
    let truth = Piece {
        id: source.id.clone(),
        name: source.name.clone(),
        outline: poly.into_iter().map(flip).collect(),
        grain: Some(grain.map(flip)),
        quantity: 1,
        marks: vec![
            Mark {
                kind: MarkKind::Notch,
                points: vec![flip(notch), flip(tip)],
                text: None,
            },
            Mark {
                kind: MarkKind::Notch,
                points: vec![flip(tick), flip(tick_end)],
                text: None,
            },
            Mark {
                kind: MarkKind::Drill,
                points: vec![flip(drill)],
                text: None,
            },
        ],
        ..Default::default()
    };
    Ok(SyntheticPhoto {
        image: photo,
        truth,
        homography: h,
        radial,
        source_scale: scale,
    })
}

/// Compare outlines in a shared coordinate frame; do not align away calibration error.
pub fn measure(truth: &Piece, recovered: &Piece) -> Result<Accuracy> {
    ensure!(
        truth.outline.len() >= 3 && recovered.outline.len() >= 3 && area(&truth.outline).abs() > 0.,
        "invalid outlines for accuracy measurement"
    );
    let mut errors = Vec::new();
    for (a, b) in [
        (&truth.outline, &recovered.outline),
        (&recovered.outline, &truth.outline),
    ] {
        for i in 0..a.len() {
            let p = a[i];
            let q = a[(i + 1) % a.len()];
            let n = (distance(p, q) / 0.5).ceil() as usize;
            for j in 0..n.max(1) {
                let t = j as f64 / n.max(1) as f64;
                errors.push(outline_distance(
                    [p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])],
                    b,
                ));
            }
        }
    }
    let actual: Vec<_> = recovered
        .marks
        .iter()
        .filter(|m| m.kind == MarkKind::Notch)
        .filter_map(|m| m.points.first().copied())
        .collect();
    let mut used = vec![false; actual.len()];
    let mut found = 0;
    let mut expected = 0;
    for mark in truth.marks.iter().filter(|m| m.kind == MarkKind::Notch) {
        expected += 1;
        if let Some(p) = mark.points.first()
            && let Some((i, _)) = actual
                .iter()
                .enumerate()
                .filter(|(i, q)| !used[*i] && distance(**q, *p) < 2.)
                .min_by(|(_, a), (_, b)| distance(**a, *p).total_cmp(&distance(**b, *p)))
        {
            used[i] = true;
            found += 1;
        }
    }
    let drills_found = truth
        .marks
        .iter()
        .filter(|m| m.kind == MarkKind::Drill)
        .filter(|m| {
            recovered.marks.iter().any(|a| {
                a.kind == MarkKind::Drill
                    && a.points
                        .first()
                        .zip(m.points.first())
                        .is_some_and(|(a, b)| distance(*a, *b) < 2.)
            })
        })
        .count();
    let grain_error_deg = truth.grain.zip(recovered.grain).map(|(a, b)| {
        let u = [a[1][0] - a[0][0], a[1][1] - a[0][1]];
        let v = [b[1][0] - b[0][0], b[1][1] - b[0][1]];
        ((u[0] * v[0] + u[1] * v[1]).abs() / (distance(a[0], a[1]) * distance(b[0], b[1])))
            .clamp(0., 1.)
            .acos()
            .to_degrees()
    });
    Ok(Accuracy {
        mean_mm: errors.iter().sum::<f64>() / errors.len() as f64,
        max_mm: errors.iter().copied().fold(0., f64::max),
        area_error_pct: (area(&truth.outline).abs() - area(&recovered.outline).abs()).abs()
            / area(&truth.outline).abs()
            * 100.,
        notches_found: found,
        notches_expected: expected,
        notch_false_positives: actual.len() - found,
        drills_found,
        grain_error_deg,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        target::SheetSize,
        vision::{ScanOptions, scan},
    };
    #[test]
    fn degraded_photo_recovers_metric_piece() {
        let target = Target::tiled(SheetSize::A3, 1, 1, false).unwrap();
        let source = Piece {
            outline: vec![[0., 0.], [230., 0.], [210., 160.], [80., 185.], [10., 130.]],
            ..Default::default()
        };
        let photo = photograph(
            &target,
            &source,
            &PhotoOptions {
                dimensions: [1800, 1350],
                ..Default::default()
            },
        )
        .unwrap();
        let result = scan(
            &photo.image,
            &target,
            &ScanOptions {
                fit_radial: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(result.pieces.len(), 1);
        let error = measure(&photo.truth, &result.pieces[0]).unwrap();
        eprintln!("synthetic accuracy: {error:?}");
        assert!(error.mean_mm < 1.);
        assert!(error.max_mm < 2.);
        assert_eq!(error.drills_found, 1);
        assert_eq!(error.notches_found, 2);
        assert!(error.grain_error_deg.unwrap() < 1.);
    }
}
