//! Metric rectification, paper segmentation, contours, and simple mark recognition.
use crate::{
    calibration::{self, Calibration, Detection, components, sample},
    geometry::{self, area, contains, distance, outline_distance, simplify},
    target::Target,
};
use anyhow::{Context, Result, ensure};
use beta_marker_core::{Fabric, JOB_FORMAT, Job, Mark, MarkKind, Piece, Pt};
use image::{GrayImage, ImageReader, Luma, Rgb, RgbImage};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path, time::Instant};

/// Controls for a scan. Defaults suit light paper on the printed grey board.
#[derive(Debug, Clone)]
pub struct ScanOptions {
    /// Rectified sampling pitch (0.1 to 2 mm per pixel).
    pub mm_per_pixel: f64,
    /// Maximum Douglas-Peucker chord error in mm.
    pub simplify_mm: f64,
    /// Smallest accepted piece area in square mm.
    pub min_area_mm2: f64,
    /// Fit one radial coefficient from at least eight markers.
    pub fit_radial: bool,
    /// Optional grain endpoints, in board mm with y DOWN. Applied to the containing piece.
    pub grain: Option<[Pt; 2]>,
    /// Optional normalized brightness threshold (1..254); default is Otsu.
    pub threshold: Option<u8>,
    /// Conservatively smooth only shallow contour turns before simplification.
    pub smooth: bool,
}
impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            mm_per_pixel: 0.5,
            simplify_mm: 0.25,
            min_area_mm2: 100.,
            fit_radial: false,
            grain: None,
            threshold: None,
            smooth: true,
        }
    }
}
/// Measurements and warnings, separate from the BETA Marker piece contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanReport {
    /// Number of verified codes seen.
    pub detected_markers: usize,
    /// Calibration quality, including fitted lens term.
    pub calibration: Calibration,
    /// Threshold after background illumination normalization.
    pub threshold: u8,
    /// Total processing time, excluding image decoding and file writing.
    pub runtime_s: f64,
    /// Conditions that require operator review.
    pub warnings: Vec<String>,
}
/// A scan's pieces, report, and metric debug images.
pub struct ScanResult {
    /// Pieces in mm, y up. Board origin is the lower-left corner.
    pub pieces: Vec<Piece>,
    /// Calibration and runtime information.
    pub report: ScanReport,
    /// Rectified image, before thresholding.
    pub rectified: GrayImage,
    /// Rectified RGB overlay: green outlines, orange notches, red drills, blue grain.
    pub overlay: RgbImage,
    /// Detected code corners in the source photograph.
    pub detections: Vec<Detection>,
}
impl ScanResult {
    /// Wrap the recovered pieces in the existing BETA Marker job contract.
    pub fn job(&self, width: f64) -> Result<Job> {
        ensure!(
            width.is_finite() && width > 0.,
            "fabric width must be positive"
        );
        let mut meta = std::collections::BTreeMap::new();
        meta.insert("digitize".into(), serde_json::to_value(&self.report)?);
        Ok(Job {
            format: JOB_FORMAT.into(),
            name: "Digitized paper patterns".into(),
            fabric: Fabric {
                width,
                ..Default::default()
            },
            pieces: self.pieces.clone(),
            meta,
            ..Default::default()
        })
    }
}
/// Load an offline PNG/JPEG with bounded decoder allocations, then scan it.
pub fn scan_file(path: &Path, target: &Target, options: &ScanOptions) -> Result<ScanResult> {
    let mut reader = ImageReader::open(path)?.with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16000);
    limits.max_image_height = Some(16000);
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    let image = reader.decode().context("decode PNG/JPEG")?.to_luma8();
    scan(&image, target, options)
}

/// Run calibration, metric resampling, segmentation, tracing, and mark recognition.
pub fn scan(image: &GrayImage, target: &Target, options: &ScanOptions) -> Result<ScanResult> {
    let start = Instant::now();
    target.validate()?;
    ensure!(
        options.mm_per_pixel.is_finite() && (0.1..=2.).contains(&options.mm_per_pixel),
        "sampling pitch must be 0.1..2 mm/pixel"
    );
    ensure!(
        options.simplify_mm.is_finite() && (0. ..=5.).contains(&options.simplify_mm),
        "invalid simplification tolerance"
    );
    ensure!(
        options.min_area_mm2.is_finite() && options.min_area_mm2 > 0.,
        "minimum piece area must be positive"
    );
    ensure!(
        options.threshold.is_none_or(|t| t > 0 && t < 255),
        "threshold must be 1..254"
    );
    if let Some(grain) = options.grain {
        ensure!(
            grain.iter().flatten().all(|v| v.is_finite()) && distance(grain[0], grain[1]) > 1.,
            "invalid grain endpoints"
        );
    }
    ensure!(
        (image.width() as u64) * (image.height() as u64) <= 64_000_000,
        "photograph exceeds 64 megapixels"
    );
    let detections = calibration::detect_markers(image)?;
    let cal = calibration::calibrate(
        target,
        &detections,
        [image.width(), image.height()],
        options.fit_radial,
    )?;
    let pitch = options.mm_per_pixel;
    let w = (target.width_mm / pitch).ceil() as u32;
    let h = (target.height_mm / pitch).ceil() as u32;
    ensure!(
        (w as u64) * (h as u64) <= 24_000_000,
        "rectified image exceeds 24 megapixels; use a larger mm/pixel value"
    );
    let mut rectified = GrayImage::new(w, h);
    let mut valid = vec![0; w as usize * h as usize];
    for y in 0..h {
        for x in 0..w {
            if let Some(v) = cal
                .project([(x as f64 + 0.5) * pitch, (y as f64 + 0.5) * pitch])
                .and_then(|p| sample(image, p))
            {
                rectified[(x, y)] = Luma([v.round() as u8]);
                valid[(y * w + x) as usize] = 1;
            }
        }
    }
    // The factory setup reserves a 36 mm empty border. Sample its grey surface
    // on all four sides, then interpolate an illumination field across the board.
    validate_border(&rectified, pitch)?;
    let normalized = normalize_background(&rectified, pitch);
    let border = (36. / pitch).ceil() as u32;
    ensure!(
        w > 2 * border + 2 && h > 2 * border + 2,
        "target has no usable interior"
    );
    let threshold = options
        .threshold
        .unwrap_or_else(|| otsu(&normalized, border));
    ensure!(
        (90..=235).contains(&threshold),
        "paper/background contrast is unclear; use a dark matte backing or set --threshold"
    );
    let mut mask = vec![0; w as usize * h as usize];
    for y in border..h - border {
        for x in border..w - border {
            let i = (y * w + x) as usize;
            mask[i] = u8::from(valid[i] != 0 && normalized[(x, y)][0] > threshold);
        }
    }
    // Bridge thin ink strokes (including ticks that meet the edge). They are
    // marks, not missing paper. Wider physical V cuts remain in the contour.
    mask = close_mask(
        &mask,
        w as usize,
        h as usize,
        (0.5 / pitch).ceil().max(1.) as usize,
    );
    let min = (options.min_area_mm2 / (pitch * pitch)).ceil() as usize;
    let mut pieces = Vec::new();
    let mut warnings = vec![
        "Review every outline, notch, drill, and grain before cutting. Text can resemble marks."
            .into(),
    ];
    if !options.fit_radial {
        warnings.push("Lens correction disabled. Use a normal/tele camera lens and keep the board near the image centre.".into());
    }
    let mut grain_assigned = options.grain.is_none();
    for component in components(&mask, w as usize, h as usize, min) {
        let touches = component.iter().any(|&i| {
            let x = i % w;
            let y = i / w;
            x <= border
                || x + 1 >= w - border
                || y <= border
                || y + 1 >= h - border
                || valid[i as usize] == 0
                || valid[i.saturating_sub(1) as usize] == 0
                || valid[(i + 1).min(w * h - 1) as usize] == 0
                || valid[i.saturating_sub(w) as usize] == 0
                || valid[(i + w).min(w * h - 1) as usize] == 0
        });
        ensure!(
            !touches,
            "a piece touches the usable target boundary or photo edge; use a larger target and keep a 36 mm empty border"
        );
        let mut local = vec![0; mask.len()];
        for &i in &component {
            local[i as usize] = 1;
        }
        let mut contour = trace(&local, &normalized, threshold)
            .into_iter()
            .max_by(|a, b| area(a).abs().total_cmp(&area(b).abs()))
            .unwrap_or_default();
        for p in &mut contour {
            p[0] *= pitch;
            p[1] *= pitch;
        }
        if contour.len() < 4 || area(&contour).abs() < options.min_area_mm2 {
            continue;
        }
        // One constrained local quadratic step suppresses pixel stair steps, while
        // retaining sharp turns (including physical V cuts) and limiting motion.
        if options.smooth {
            contour = smooth_contour(&contour, pitch * 0.20);
        }
        let mut outline = simplify(&contour, options.simplify_mm);
        if outline.len() < 3 {
            continue;
        }
        let (mut marks, mut grain) = features(&normalized, &outline, pitch);
        marks.extend(cut_notches(&contour));
        if let Some(g) = options.grain
            && contains(&outline, g[0])
            && contains(&outline, g[1])
        {
            ensure!(
                !grain_assigned,
                "grain override belongs to more than one piece"
            );
            grain = Some(g);
            grain_assigned = true;
        }
        if grain.is_none() {
            warnings.push(format!("Piece {}: no reliable grain line; supply --grain x1,y1,x2,y2 in board mm (y down).",pieces.len()+1));
        }
        // The whole board uses a common Cartesian origin; do not silently change scale.
        let flip = |p: Pt| [p[0], target.height_mm - p[1]];
        for p in &mut outline {
            *p = flip(*p);
        }
        if area(&outline) < 0. {
            outline.reverse();
        }
        for m in &mut marks {
            for p in &mut m.points {
                *p = flip(*p);
            }
        }
        pieces.push(Piece {
            id: format!("SCAN-{}", pieces.len() + 1),
            name: format!("Scanned piece {}", pieces.len() + 1),
            outline,
            grain: grain.map(|g| g.map(flip)),
            quantity: 1,
            marks,
            ..Default::default()
        });
    }
    ensure!(
        !pieces.is_empty(),
        "no complete paper pieces found on the grey background"
    );
    ensure!(
        grain_assigned,
        "grain endpoints do not both lie inside a recovered piece"
    );
    let mut overlay = RgbImage::from_fn(w, h, |x, y| {
        let v = rectified[(x, y)][0];
        Rgb([v, v, v])
    });
    let px = |p: Pt| [p[0] / pitch, (target.height_mm - p[1]) / pitch];
    for p in &pieces {
        for i in 0..p.outline.len() {
            draw_line(
                &mut overlay,
                px(p.outline[i]),
                px(p.outline[(i + 1) % p.outline.len()]),
                [30, 220, 70],
            );
        }
        if let Some(g) = p.grain {
            draw_line(&mut overlay, px(g[0]), px(g[1]), [40, 120, 255]);
        }
        for m in &p.marks {
            if let Some(p) = m.points.first() {
                let q = px(*p);
                let color = if m.kind == MarkKind::Notch {
                    [255, 150, 0]
                } else {
                    [255, 40, 40]
                };
                draw_line(
                    &mut overlay,
                    [q[0] - 3., q[1] - 3.],
                    [q[0] + 3., q[1] + 3.],
                    color,
                );
                draw_line(
                    &mut overlay,
                    [q[0] - 3., q[1] + 3.],
                    [q[0] + 3., q[1] - 3.],
                    color,
                );
            }
        }
    }
    Ok(ScanResult {
        pieces,
        report: ScanReport {
            detected_markers: detections.len(),
            calibration: cal,
            threshold,
            runtime_s: start.elapsed().as_secs_f64(),
            warnings,
        },
        rectified,
        overlay,
        detections,
    })
}

fn median(mut values: Vec<u8>) -> f64 {
    values.sort_unstable();
    values.get(values.len() / 2).copied().unwrap_or(100).max(20) as f64
}
fn validate_border(image: &GrayImage, pitch: f64) -> Result<()> {
    let pad = (34. / pitch).round() as u32;
    let border = (36. / pitch).ceil() as u32;
    if image.width() <= 2 * border + 2 || image.height() <= 2 * border + 2 {
        return Ok(());
    }
    for y in border..image.height() - border {
        let a = median((pad..pad + 3).map(|x| image[(x, y)][0]).collect());
        let b = median(
            (image.width() - pad - 3..image.width() - pad)
                .map(|x| image[(x, y)][0])
                .collect(),
        );
        ensure!(
            a.max(b) <= a.min(b) * 1.8 || a.max(b) < 130.,
            "usable target boundary is covered, or a hard shadow crosses the empty border"
        );
    }
    Ok(())
}
fn close_mask(mask: &[u8], w: usize, h: usize, r: usize) -> Vec<u8> {
    fn pass(input: &[u8], w: usize, h: usize, r: usize, dilate: bool) -> Vec<u8> {
        let mut integral = vec![0_u32; (w + 1) * (h + 1)];
        for y in 0..h {
            let mut row = 0;
            for x in 0..w {
                row += input[y * w + x] as u32;
                integral[(y + 1) * (w + 1) + x + 1] = integral[y * (w + 1) + x + 1] + row;
            }
        }
        let mut out = vec![0; w * h];
        for y in 0..h {
            for x in 0..w {
                let a = x.saturating_sub(r);
                let b = y.saturating_sub(r);
                let c = (x + r + 1).min(w);
                let d = (y + r + 1).min(h);
                let n = integral[d * (w + 1) + c] + integral[b * (w + 1) + a]
                    - integral[d * (w + 1) + a]
                    - integral[b * (w + 1) + c];
                out[y * w + x] = u8::from(if dilate {
                    n > 0
                } else {
                    n as usize == (c - a) * (d - b)
                });
            }
        }
        out
    }
    pass(&pass(mask, w, h, r, true), w, h, r, false)
}
fn normalize_background(image: &GrayImage, pitch: f64) -> GrayImage {
    let w = image.width();
    let h = image.height();
    let pad = (34. / pitch).round() as u32;
    if w < 2 * pad + 3 || h < 2 * pad + 3 {
        return image.clone();
    }
    let mut left = vec![100.; h as usize];
    let mut right = left.clone();
    for y in 0..h {
        let a = y.saturating_sub(3);
        let b = (y + 4).min(h);
        left[y as usize] = median(
            (a..b)
                .flat_map(|yy| (pad..pad + 3).map(move |x| image[(x, yy)][0]))
                .collect(),
        );
        right[y as usize] = median(
            (a..b)
                .flat_map(|yy| (w - pad - 3..w - pad).map(move |x| image[(x, yy)][0]))
                .collect(),
        );
    }
    GrayImage::from_fn(w, h, |x, y| {
        let t = (x as f64 - pad as f64) / (w - 2 * pad) as f64;
        let bg = left[y as usize] * (1. - t) + right[y as usize] * t;
        Luma([(image[(x, y)][0] as f64 * 100. / bg.max(20.))
            .round()
            .clamp(0., 255.) as u8])
    })
}
fn otsu(image: &GrayImage, border: u32) -> u8 {
    let mut hist = [0_u64; 256];
    for y in border..image.height() - border {
        for x in border..image.width() - border {
            hist[image[(x, y)][0] as usize] += 1;
        }
    }
    let n = hist.iter().sum::<u64>() as f64;
    let sum = hist
        .iter()
        .enumerate()
        .map(|(i, &n)| i as f64 * n as f64)
        .sum::<f64>();
    let mut count = 0.;
    let mut partial = 0.;
    let mut best = 0.;
    let mut first = 0;
    let mut last = 0;
    for (i, &v) in hist.iter().enumerate() {
        count += v as f64;
        partial += v as f64 * i as f64;
        if count == 0. || count == n {
            continue;
        }
        let score = count * (n - count) * (partial / count - (sum - partial) / (n - count)).powi(2);
        if score > best + 1e-6 {
            best = score;
            first = i;
            last = i;
        } else if (score - best).abs() < 1e-6 {
            last = i;
        }
    }
    ((first + last) / 2) as u8
}

// Marching squares: graph vertices are horizontal/vertical sample-grid edges.
// Grey-level interpolation gives subpixel crossings instead of pixel stair steps.
fn trace(mask: &[u8], grey: &GrayImage, threshold: u8) -> Vec<Vec<Pt>> {
    let w = grey.width() as usize;
    let h = grey.height() as usize;
    let mut graph = HashMap::<usize, (Pt, Vec<usize>)>::new();
    for y in 0..h - 1 {
        for x in 0..w - 1 {
            let indices = [
                y * w + x,
                y * w + x + 1,
                (y + 1) * w + x + 1,
                (y + 1) * w + x,
            ];
            let bits = indices.map(|i| mask[i] != 0);
            let code = (bits[0] as u8)
                | ((bits[1] as u8) << 1)
                | ((bits[2] as u8) << 2)
                | ((bits[3] as u8) << 3);
            if code == 0 || code == 15 {
                continue;
            }
            let coords = [
                [x as f64 + 0.5, y as f64 + 0.5],
                [x as f64 + 1.5, y as f64 + 0.5],
                [x as f64 + 1.5, y as f64 + 1.5],
                [x as f64 + 0.5, y as f64 + 1.5],
            ];
            let keys = [
                2 * (y * w + x),
                2 * (y * w + x + 1) + 1,
                2 * ((y + 1) * w + x),
                2 * (y * w + x) + 1,
            ];
            let mut crossings = Vec::new();
            for e in 0..4 {
                let j = (e + 1) % 4;
                if bits[e] == bits[j] {
                    continue;
                }
                let a = grey.as_raw()[indices[e]] as f64;
                let b = grey.as_raw()[indices[j]] as f64;
                let t = if (b - a).abs() > 1. {
                    ((threshold as f64 - a) / (b - a)).clamp(0.05, 0.95)
                } else {
                    0.5
                };
                let p = [
                    coords[e][0] + t * (coords[j][0] - coords[e][0]),
                    coords[e][1] + t * (coords[j][1] - coords[e][1]),
                ];
                graph.entry(keys[e]).or_insert((p, Vec::new()));
                crossings.push(e);
            }
            let pairs = if crossings.len() == 2 {
                vec![(crossings[0], crossings[1])]
            } else if code == 5 {
                vec![(0, 3), (1, 2)]
            } else {
                vec![(0, 1), (2, 3)]
            };
            for (a, b) in pairs {
                if let Some(v) = graph.get_mut(&keys[a]) {
                    v.1.push(keys[b]);
                }
                if let Some(v) = graph.get_mut(&keys[b]) {
                    v.1.push(keys[a]);
                }
            }
        }
    }
    let mut contours = Vec::new();
    while let Some(&start) = graph.keys().min() {
        let mut current = start;
        let mut previous = usize::MAX;
        let mut path = Vec::new();
        while let Some((p, next)) = graph.remove(&current) {
            path.push(p);
            let Some(&n) = next.iter().find(|&&n| n != previous) else {
                break;
            };
            previous = current;
            current = n;
            if current == start {
                break;
            }
        }
        if current == start && path.len() >= 4 {
            contours.push(path);
        }
    }
    contours
}
fn smooth_contour(p: &[Pt], limit: f64) -> Vec<Pt> {
    (0..p.len())
        .map(|i| {
            let a = p[(i + p.len() - 1) % p.len()];
            let b = p[i];
            let c = p[(i + 1) % p.len()];
            let u = [b[0] - a[0], b[1] - a[1]];
            let v = [c[0] - b[0], c[1] - b[1]];
            let dot = (u[0] * v[0] + u[1] * v[1]) / (distance(a, b) * distance(b, c)).max(1e-12);
            if dot < 0.70 {
                return b;
            }
            let q = [
                (a[0] + 2. * b[0] + c[0]) / 4.,
                (a[1] + 2. * b[1] + c[1]) / 4.,
            ];
            let d = distance(q, b);
            let f = if d > limit { limit / d } else { 1. };
            [b[0] + (q[0] - b[0]) * f, b[1] + (q[1] - b[1]) * f]
        })
        .collect()
}

fn features(image: &GrayImage, outline: &[Pt], pitch: f64) -> (Vec<Mark>, Option<[Pt; 2]>) {
    let w = image.width() as usize;
    let h = image.height() as usize;
    let mut mask = vec![0; w * h];
    let mut low = [w, h];
    let mut high = [0, 0];
    for p in outline {
        for k in 0..2 {
            let value = (p[k] / pitch).max(0.) as usize;
            low[k] = low[k].min(value);
            high[k] = high[k].max(value + 2);
        }
    }
    for y in low[1].min(h)..high[1].min(h) {
        for x in low[0].min(w)..high[0].min(w) {
            let p = [(x as f64 + 0.5) * pitch, (y as f64 + 0.5) * pitch];
            if image.as_raw()[y * w + x] < 135 && contains(outline, p) {
                mask[y * w + x] = 1;
            }
        }
    }
    let mut marks = Vec::new();
    let mut grain = None;
    let mut longest = 0.;
    for c in components(&mask, w, h, 2) {
        let pts: Vec<_> = c
            .iter()
            .map(|&i| {
                [
                    ((i as usize % w) as f64 + 0.5) * pitch,
                    ((i as usize / w) as f64 + 0.5) * pitch,
                ]
            })
            .collect();
        let n = pts.len() as f64;
        let centre = [
            pts.iter().map(|p| p[0]).sum::<f64>() / n,
            pts.iter().map(|p| p[1]).sum::<f64>() / n,
        ];
        let xx = pts.iter().map(|p| (p[0] - centre[0]).powi(2)).sum::<f64>() / n;
        let yy = pts.iter().map(|p| (p[1] - centre[1]).powi(2)).sum::<f64>() / n;
        let xy = pts
            .iter()
            .map(|p| (p[0] - centre[0]) * (p[1] - centre[1]))
            .sum::<f64>()
            / n;
        let angle = 0.5 * (2. * xy).atan2(xx - yy);
        let axis = [angle.cos(), angle.sin()];
        let variance = ((xx - yy).powi(2) + 4. * xy * xy).sqrt();
        let ratio = (xx + yy + variance) / (xx + yy - variance).max(0.01);
        let mut low = f64::INFINITY;
        let mut high = f64::NEG_INFINITY;
        for p in &pts {
            let t = (p[0] - centre[0]) * axis[0] + (p[1] - centre[1]) * axis[1];
            low = low.min(t);
            high = high.max(t);
        }
        let len = high - low + pitch;
        let width = n * pitch * pitch / len;
        let ends = [
            [centre[0] + low * axis[0], centre[1] + low * axis[1]],
            [centre[0] + high * axis[0], centre[1] + high * axis[1]],
        ];
        let edge = pts
            .iter()
            .map(|&p| outline_distance(p, outline))
            .fold(f64::INFINITY, f64::min);
        let penetration = pts
            .iter()
            .map(|&p| outline_distance(p, outline))
            .fold(0., f64::max);
        if ratio > 25. && len >= 25. && width <= 2.5 && edge > 2. && len > longest {
            longest = len;
            grain = Some(ends);
        } else if (0.8..=6.).contains(&len) && ratio < 2.8 && n * pitch * pitch >= 0.5 && edge > 3.
        {
            marks.push(Mark {
                kind: MarkKind::Drill,
                points: vec![centre],
                text: Some("Detected dark circular mark; verify drill".into()),
            });
        } else if ratio > 4.
            && (1.5..=10.).contains(&len)
            && width < 2.
            && edge < 1.2
            && penetration >= 1.5
        {
            let near = if outline_distance(ends[0], outline) < outline_distance(ends[1], outline) {
                ends
            } else {
                [ends[1], ends[0]]
            };
            marks.push(Mark {
                kind: MarkKind::Notch,
                points: near.to_vec(),
                text: Some("Detected edge tick/slit; verify notch".into()),
            });
        }
    }
    (marks, grain)
}
fn cut_notches(contour: &[Pt]) -> Vec<Mark> {
    // Group adjacent re-entrant turns: resampling can flatten a V's very tip
    // into a short segment. A broad armhole fails the mouth/depth/length checks.
    let p = simplify(contour, 0.65);
    let orientation = area(&p).signum();
    let turn: Vec<_> = (0..p.len())
        .map(|i| {
            let a = p[(i + p.len() - 1) % p.len()];
            let b = p[i];
            let c = p[(i + 1) % p.len()];
            let u = [b[0] - a[0], b[1] - a[1]];
            let v = [c[0] - b[0], c[1] - b[1]];
            ((u[0] * v[1] - u[1] * v[0]) * orientation).atan2(u[0] * v[0] + u[1] * v[1])
        })
        .collect();
    let mut result = Vec::new();
    for i in 0..p.len() {
        if turn[i] >= -0.08 || turn[(i + p.len() - 1) % p.len()] < -0.08 {
            continue;
        }
        let mut count = 1;
        let mut total = -turn[i];
        while count < 4 && turn[(i + count) % p.len()] < -0.08 {
            total -= turn[(i + count) % p.len()];
            count += 1;
        }
        if total < 70_f64.to_radians() {
            continue;
        }
        let a = p[(i + p.len() - 1) % p.len()];
        let c = p[(i + count) % p.len()];
        let l = distance(a, p[i]);
        let r = distance(p[(i + count - 1) % p.len()], c);
        if !(1. ..=8.).contains(&l) || !(1. ..=8.).contains(&r) {
            continue;
        }
        if (1. ..=8.).contains(&distance(a, c)) {
            let b = (0..count)
                .map(|j| p[(i + j) % p.len()])
                .max_by(|&u, &v| {
                    geometry::segment_distance(u, a, c)
                        .total_cmp(&geometry::segment_distance(v, a, c))
                })
                .unwrap_or(p[i]);
            let mouth = [(a[0] + c[0]) / 2., (a[1] + c[1]) / 2.];
            if geometry::segment_distance(b, a, c) >= 1. {
                result.push(Mark {
                    kind: MarkKind::Notch,
                    points: vec![mouth, b],
                    text: Some("Detected V cut; verify notch".into()),
                });
            }
        }
    }
    result
}
pub(crate) fn draw_line(image: &mut RgbImage, a: Pt, b: Pt, color: [u8; 3]) {
    let steps = (distance(a, b) * 2.).ceil().min(100000.) as usize;
    for i in 0..=steps {
        let t = i as f64 / steps.max(1) as f64;
        let x = (a[0] + t * (b[0] - a[0])).floor() as i32;
        let y = (a[1] + t * (b[1] - a[1])).floor() as i32;
        if x >= 0 && y >= 0 && x < image.width() as i32 && y < image.height() as i32 {
            image[(x as u32, y as u32)] = Rgb(color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn multiple_pieces_and_border_rejection() {
        let target = Target::tiled(crate::target::SheetSize::A3, 1, 1, false).unwrap();
        let ppm = 3.;
        let mut board = GrayImage::from_pixel(1260, 891, Luma([255]));
        for r in target.rectangles(true) {
            for y in (r.y * ppm) as u32..((r.y + r.h) * ppm) as u32 {
                for x in (r.x * ppm) as u32..((r.x + r.w) * ppm) as u32 {
                    board[(x, y)] = Luma([r.grey]);
                }
            }
        }
        for (a, b) in [([60., 60.], [175., 220.]), ([220., 80.], [355., 230.])] {
            for y in (a[1] * ppm) as u32..(b[1] * ppm) as u32 {
                for x in (a[0] * ppm) as u32..(b[0] * ppm) as u32 {
                    board[(x, y)] = Luma([245]);
                }
            }
        }
        let result = scan(
            &board,
            &target,
            &ScanOptions {
                grain: Some([[100., 100.], [100., 180.]]),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(result.pieces.len(), 2);
        assert_eq!(
            result.pieces.iter().filter(|p| p.grain.is_some()).count(),
            1
        );
        assert!(beta_marker_core::check_job(&result.job(1500.).unwrap()).is_empty());
        // Extend the first piece over the required empty border.
        for y in 180..300 {
            for x in 90..190 {
                board[(x, y)] = Luma([245]);
            }
        }
        let error = scan(&board, &target, &ScanOptions::default())
            .err()
            .unwrap()
            .to_string();
        assert!(error.contains("boundary"), "{error}");
    }
    #[test]
    fn marching_squares_keeps_concavity_and_holes() {
        let im = GrayImage::from_fn(30, 30, |x, y| {
            Luma([
                if (3..26).contains(&x) && (3..26).contains(&y) && !(10..15).contains(&x) {
                    240
                } else {
                    80
                },
            ])
        });
        let mask = im
            .as_raw()
            .iter()
            .map(|&v| u8::from(v > 160))
            .collect::<Vec<_>>();
        let loops = trace(&mask, &im, 160);
        assert_eq!(loops.len(), 2);
        assert!(loops.iter().all(|p| area(p).abs() > 100.));
    }
    #[test]
    fn invalid_options_return_errors() {
        let t = Target::tiled(crate::target::SheetSize::A3, 1, 1, false).unwrap();
        let o = ScanOptions {
            mm_per_pixel: f64::NAN,
            ..Default::default()
        };
        assert!(scan(&GrayImage::new(64, 64), &t, &o).is_err());
    }
}
