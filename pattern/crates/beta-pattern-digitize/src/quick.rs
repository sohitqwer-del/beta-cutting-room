//! Bounded offline photo scans and review-only multi-photo sessions.
use crate::{
    assist, calibration,
    geometry::{self, area, distance, fit_homography},
    target::{SheetSize, Target},
    vision,
};
use anyhow::{Context, Result, ensure};
use beta_marker_core::{Fabric, JOB_FORMAT, Job, Layout, Mark, MarkKind, Nap, Order, Piece, Pt};
use image::{GrayImage, ImageFormat, ImageReader, Luma, Rgb, RgbImage, imageops::FilterType};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Cursor,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    High,
    Medium,
    Low,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Warning {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub piece_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub photo_id: Option<String>,
}
impl Warning {
    pub(crate) fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            piece_id: None,
            photo_id: None,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickPiece {
    #[serde(flatten)]
    pub piece: Piece,
    pub photo_id: String,
    pub suggestions: assist::Suggestions,
    pub expected_error_mm: f64,
    pub notches: Vec<Mark>,
    pub drills: Vec<Mark>,
}
impl QuickPiece {
    pub fn new(piece: Piece, photo_id: &str, error: f64) -> Self {
        Self {
            notches: piece
                .marks
                .iter()
                .filter(|m| m.kind == MarkKind::Notch)
                .cloned()
                .collect(),
            drills: piece
                .marks
                .iter()
                .filter(|m| m.kind == MarkKind::Drill)
                .cloned()
                .collect(),
            piece,
            photo_id: photo_id.into(),
            suggestions: Default::default(),
            expected_error_mm: error,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Reference {
    A4Sheet,
    BetaTarget,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Options {
    pub reference: Reference,
    pub photo_id: String,
    pub target_manifest: Option<String>,
    pub max_pieces: usize,
    pub max_long_edge: u32,
    pub a4_corners: Option<[Pt; 4]>,
    pub use_second_sheet: bool,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            reference: Reference::A4Sheet,
            photo_id: "photo-1".into(),
            target_manifest: None,
            max_pieces: 40,
            max_long_edge: 2000,
            a4_corners: None,
            use_second_sheet: true,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scale {
    pub mm_per_px: f64,
    pub confidence: Confidence,
    pub method: Reference,
    pub expected_error_mm: f64,
    pub tilt_score: f64,
    pub reference_corners: Vec<Vec<Pt>>,
    pub image_to_mm: [f64; 9],
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scan {
    pub photo_id: String,
    pub pieces: Vec<QuickPiece>,
    pub overlay_png: Vec<u8>,
    pub scale: Scale,
    pub warnings: Vec<Warning>,
    pub source_size: [u32; 2],
    pub processed_size: [u32; 2],
    pub runtime_ms: f64,
}
/// Decode at most 12 MP, then release the decoded colour image before vision.
pub fn scan_bytes(bytes: &[u8], options: &Options) -> Result<Scan> {
    ensure!(
        !bytes.is_empty() && bytes.len() <= 20_000_000,
        "Choose a JPEG/PNG no larger than 20 MB"
    );
    let start = crate::Timer::start();
    let format = image::guess_format(bytes)?;
    ensure!(
        matches!(format, ImageFormat::Jpeg | ImageFormat::Png),
        "Only JPEG and PNG photos are supported"
    );
    let reader = ImageReader::with_format(Cursor::new(bytes), format);
    let (w, h) = reader.into_dimensions()?;
    ensure!(
        w >= 64 && h >= 64 && u64::from(w) * u64::from(h) <= 12_000_000,
        "Photo must be 64 pixels or larger and at most 12 megapixels"
    );
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(12000);
    limits.max_image_height = Some(12000);
    limits.max_alloc = Some(96 * 1024 * 1024);
    reader.limits(limits);
    let image = reader.decode().context("Decode JPEG/PNG")?.into_luma8();
    let mut result = scan_image(&image, options)?;
    result.runtime_ms = start.seconds() * 1000.;
    Ok(result)
}
/// Grayscale entry point used by deterministic synthetic tests.
pub fn scan_image(source: &GrayImage, options: &Options) -> Result<Scan> {
    let start = crate::Timer::start();
    ensure!(
        (640..=2400).contains(&options.max_long_edge),
        "maxLongEdge must be 640..2400"
    );
    ensure!(
        (1..=100).contains(&options.max_pieces),
        "maxPieces must be 1..100"
    );
    ensure!(
        !options.photo_id.trim().is_empty()
            && options.photo_id.len() <= 100
            && !options.photo_id.contains(':'),
        "Use a short unique photoId without a colon"
    );
    ensure!(
        source.width() >= 64
            && source.height() >= 64
            && u64::from(source.width()) * u64::from(source.height()) <= 12_000_000,
        "Photo must be 64 pixels or larger and at most 12 megapixels"
    );
    let ratio =
        (f64::from(options.max_long_edge) / f64::from(source.width().max(source.height()))).min(1.);
    let (w, h) = (
        (source.width() as f64 * ratio).round() as u32,
        (source.height() as f64 * ratio).round() as u32,
    );
    let image = image::imageops::resize(source, w, h, FilterType::Triangle);
    let sx = source.width() as f64 / w as f64;
    let sy = source.height() as f64 / h as f64;
    let mut result = if options.reference == Reference::BetaTarget {
        scan_target(&image, options, [sx, sy])?
    } else {
        scan_a4(&image, options, [sx, sy])?
    };
    result.source_size = [source.width(), source.height()];
    result.processed_size = [w, h];
    result.runtime_ms = start.seconds() * 1000.;
    for warning in &mut result.warnings {
        warning.photo_id = Some(options.photo_id.clone());
    }
    assist::suggest(&mut result.pieces)?;
    Ok(result)
}
fn png(image: &RgbImage) -> Result<Vec<u8>> {
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, ImageFormat::Png)?;
    Ok(bytes.into_inner())
}
fn overlay(
    image: &GrayImage,
    pieces: &[QuickPiece],
    project: impl Fn(Pt) -> Option<Pt>,
    references: &[Vec<Pt>],
) -> RgbImage {
    let mut out = RgbImage::from_fn(image.width(), image.height(), |x, y| {
        let v = image[(x, y)][0];
        Rgb([v, v, v])
    });
    for corners in references {
        for i in 0..corners.len() {
            vision::draw_line(
                &mut out,
                corners[i],
                corners[(i + 1) % corners.len()],
                [255, 190, 0],
            );
        }
    }
    for p in pieces {
        for i in 0..p.piece.outline.len() {
            if let (Some(a), Some(b)) = (
                project(p.piece.outline[i]),
                project(p.piece.outline[(i + 1) % p.piece.outline.len()]),
            ) {
                vision::draw_line(&mut out, a, b, [20, 230, 70]);
            }
        }
        if let Some([a, b]) = p
            .piece
            .grain
            .or_else(|| p.suggestions.grain.as_ref().map(|g| g.value))
            && let (Some(a), Some(b)) = (project(a), project(b))
        {
            vision::draw_line(&mut out, a, b, [30, 120, 255]);
        }
        for m in &p.piece.marks {
            if let Some(q) = m.points.first().and_then(|&p| project(p)) {
                let color = if m.kind == MarkKind::Notch {
                    [255, 140, 0]
                } else {
                    [255, 40, 40]
                };
                vision::draw_line(
                    &mut out,
                    [q[0] - 3., q[1] - 3.],
                    [q[0] + 3., q[1] + 3.],
                    color,
                );
                vision::draw_line(
                    &mut out,
                    [q[0] - 3., q[1] + 3.],
                    [q[0] + 3., q[1] - 3.],
                    color,
                );
            }
        }
    }
    out
}
fn scan_target(image: &GrayImage, options: &Options, source_scale: Pt) -> Result<Scan> {
    let target = match &options.target_manifest {
        Some(s) => serde_json::from_str(s)?,
        None => Target::tiled(SheetSize::A3, 1, 1, false)?,
    };
    // Bound the metric buffer too: a large manifest must not allocate 24 MP on a phone.
    let pitch = 0.5_f64.max((target.width_mm * target.height_mm / 3_000_000.).sqrt());
    ensure!(
        pitch <= 2.,
        "Target too large for a phone scan; split into smaller photos"
    );
    let found = vision::scan(
        image,
        &target,
        &vision::ScanOptions {
            mm_per_pixel: pitch,
            fit_radial: true,
            ..Default::default()
        },
    )?;
    let cal = &found.report.calibration;
    let mmpp = distance(
        cal.unproject([image.width() as f64 / 2., image.height() as f64 / 2.])
            .context("Invalid target scale")?,
        cal.unproject([image.width() as f64 / 2. + 1., image.height() as f64 / 2.])
            .context("Invalid target scale")?,
    );
    let estimate = (cal.rms_px * mmpp * 2.).max(pitch);
    let target_corners: Option<Vec<_>> = [
        [0., 0.],
        [target.width_mm, 0.],
        [target.width_mm, target.height_mm],
        [0., target.height_mm],
    ]
    .into_iter()
    .map(|p| cal.project(p))
    .collect();
    let tilt_score = target_corners
        .map(|p| tilt_for([p[0], p[1], p[2], p[3]], target.width_mm / target.height_mm))
        .unwrap_or(1.);
    let mut pieces: Vec<_> = found
        .pieces
        .into_iter()
        .map(|p| QuickPiece::new(p, &options.photo_id, estimate))
        .collect();
    for p in &mut pieces {
        if let Some(g) = p.piece.grain.take() {
            p.suggestions.grain = Some(assist::Suggestion::new(
                g,
                Confidence::Medium,
                "Detected drawn line; confirm it is the grain line",
            ));
        }
    }
    let mut warnings: Vec<_> = found
        .report
        .warnings
        .iter()
        .map(|m| Warning::new("review_required", m))
        .collect();
    if pieces.len() > options.max_pieces {
        pieces.truncate(options.max_pieces);
        warnings.push(Warning::new(
            "piece_limit",
            "Some pieces were omitted. Take separate photos with fewer pieces.",
        ));
    }
    warnings.extend(photo_warnings(image));
    if tilt_score > 0.18 {
        warnings.push(Warning::new(
            "tilt",
            "The target view is tilted. Flatten the paper and photograph more nearly overhead.",
        ));
    }
    for p in &pieces {
        let mut w = Warning::new("grain_unconfirmed", "Confirm grain before making a marker.");
        w.piece_id = Some(p.piece.id.clone());
        warnings.push(w);
    }
    let inv = cal.homography.inverse()?.0;
    let s = source_scale;
    let height = target.height_mm;
    let image_to_mm = [
        inv[0] / s[0],
        inv[1] / s[1],
        inv[2],
        (height * inv[6] - inv[3]) / s[0],
        (height * inv[7] - inv[4]) / s[1],
        height * inv[8] - inv[5],
        inv[6] / s[0],
        inv[7] / s[1],
        inv[8],
    ];
    let overlay = overlay(image, &pieces, |p| cal.project([p[0], height - p[1]]), &[]);
    Ok(Scan {
        photo_id: options.photo_id.clone(),
        pieces,
        overlay_png: png(&overlay)?,
        scale: Scale {
            mm_per_px: mmpp / source_scale[0],
            confidence: Confidence::High,
            method: Reference::BetaTarget,
            expected_error_mm: estimate,
            tilt_score,
            reference_corners: vec![],
            image_to_mm,
        },
        warnings,
        source_size: [0; 2],
        processed_size: [0; 2],
        runtime_ms: 0.,
    })
}

/// Lower-background samples on a coarse grid remove slow lighting gradients.
/// Bright occupied cells use the global dark-floor estimate, not the paper itself.
fn normalize(image: &GrayImage) -> GrayImage {
    let global = percentile(image.as_raw(), 0.20) as f64;
    let threshold = vision::otsu(image, 0) as f64;
    let cell = 64_u32;
    let nx = image.width().div_ceil(cell);
    let ny = image.height().div_ceil(cell);
    let mut bg = vec![global; (nx * ny) as usize];
    for gy in 0..ny {
        for gx in 0..nx {
            let mut values = Vec::new();
            for y in gy * cell..((gy + 1) * cell).min(image.height()) {
                for x in gx * cell..((gx + 1) * cell).min(image.width()) {
                    values.push(image[(x, y)][0]);
                }
            }
            let v = percentile(&values, 0.15) as f64;
            if v < threshold * 0.85 {
                bg[(gy * nx + gx) as usize] = v;
            }
        }
    }
    GrayImage::from_fn(image.width(), image.height(), |x, y| {
        let fx = (x as f64 / cell as f64 - 0.5).clamp(0., (nx - 1) as f64);
        let fy = (y as f64 / cell as f64 - 0.5).clamp(0., (ny - 1) as f64);
        let a = fx.floor() as u32;
        let b = fy.floor() as u32;
        let c = (a + 1).min(nx - 1);
        let d = (b + 1).min(ny - 1);
        let u = fx - a as f64;
        let v = fy - b as f64;
        let back = (1. - v)
            * ((1. - u) * bg[(b * nx + a) as usize] + u * bg[(b * nx + c) as usize])
            + v * ((1. - u) * bg[(d * nx + a) as usize] + u * bg[(d * nx + c) as usize]);
        Luma([(image[(x, y)][0] as f64 - back + global).clamp(0., 255.) as u8])
    })
}
fn percentile(values: &[u8], fraction: f64) -> u8 {
    let mut hist = [0_usize; 256];
    for &v in values {
        hist[v as usize] += 1;
    }
    let n = (values.len() as f64 * fraction) as usize;
    let mut total = 0;
    for (v, count) in hist.iter().enumerate() {
        total += count;
        if total > n {
            return v as u8;
        }
    }
    255
}
struct Region {
    contour: Vec<Pt>,
    quad: Option<[Pt; 4]>,
    mean: f64,
    edge: bool,
}
fn regions(image: &GrayImage, threshold: u8) -> Result<Vec<Region>> {
    let w = image.width() as usize;
    let h = image.height() as usize;
    let mask: Vec<_> = image
        .as_raw()
        .iter()
        .map(|&v| u8::from(v > threshold))
        .collect();
    let mask = vision::close_mask(&mask, w, h, 1);
    let mut out = Vec::new();
    for component in calibration::components(&mask, w, h, 100) {
        let mut low = [w, h];
        let mut high = [0, 0];
        let mut sum = 0_u64;
        for &i in &component {
            let x = i as usize % w;
            let y = i as usize / w;
            low[0] = low[0].min(x);
            low[1] = low[1].min(y);
            high[0] = high[0].max(x);
            high[1] = high[1].max(y);
            sum += image.as_raw()[i as usize] as u64;
        }
        let edge = low[0] <= 1 || low[1] <= 1 || high[0] + 2 >= w || high[1] + 2 >= h;
        let x0 = low[0].saturating_sub(2);
        let y0 = low[1].saturating_sub(2);
        let cw = (high[0] + 3).min(w) - x0;
        let ch = (high[1] + 3).min(h) - y0;
        let crop =
            image::imageops::crop_imm(image, x0 as u32, y0 as u32, cw as u32, ch as u32).to_image();
        let mut local = vec![0; cw * ch];
        for &i in &component {
            local[(i as usize / w - y0) * cw + (i as usize % w - x0)] = 1;
        }
        // Explicit zero rim lets edge-touching objects be traced and warned, never called complete.
        for x in 0..cw {
            local[x] = 0;
            local[(ch - 1) * cw + x] = 0;
        }
        for y in 0..ch {
            local[y * cw] = 0;
            local[y * cw + cw - 1] = 0;
        }
        let mut transitions = 0_usize;
        for y in 0..ch - 1 {
            for x in 0..cw - 1 {
                let i = y * cw + x;
                transitions +=
                    usize::from(local[i] != local[i + 1]) + usize::from(local[i] != local[i + cw]);
            }
        }
        ensure!(
            transitions <= 60000,
            "Background or paper texture is too complex. Use smooth dark backing and even lighting"
        );
        let mut contour = vision::trace(&local, &crop, threshold)
            .into_iter()
            .max_by(|a, b| area(a).abs().total_cmp(&area(b).abs()))
            .unwrap_or_default();
        if contour.len() < 4 {
            continue;
        }
        for p in &mut contour {
            p[0] += x0 as f64;
            p[1] += y0 as f64;
        }
        let quad = calibration::quad(&contour);
        out.push(Region {
            contour,
            quad,
            mean: sum as f64 / component.len() as f64,
            edge,
        });
        ensure!(
            out.len() <= 300,
            "Too many light objects or texture patches. Use smooth dark backing"
        );
    }
    Ok(out)
}
fn ordered(mut q: [Pt; 4]) -> [Pt; 4] {
    if area(&q) < 0. {
        q.reverse();
    }
    let lengths = [
        distance(q[0], q[1]) + distance(q[2], q[3]),
        distance(q[1], q[2]) + distance(q[3], q[0]),
    ];
    if lengths[0] > lengths[1] {
        q.rotate_left(1);
    }
    // Fix the remaining 180 degree ambiguity using the upper of the two short edges.
    if q[0][1] + q[1][1] > q[2][1] + q[3][1] {
        q.rotate_left(2);
    }
    q
}
fn rectangular(region: &Region) -> bool {
    let Some(q) = region.quad else {
        return false;
    };
    let q = ordered(q);
    let short = (distance(q[0], q[1]) + distance(q[2], q[3])) / 2.;
    let long = (distance(q[1], q[2]) + distance(q[3], q[0])) / 2.;
    let fill = area(&region.contour).abs() / area(&q).abs();
    let mut residuals: Vec<_> = region
        .contour
        .iter()
        .map(|&p| geometry::outline_distance(p, &q))
        .collect();
    residuals.sort_by(f64::total_cmp);
    let residual = residuals.iter().sum::<f64>() / residuals.len() as f64;
    !region.edge
        && region.mean > 150.
        && (0.96..1.025).contains(&fill)
        && residual < short * 0.006
        && residuals[residuals.len() * 95 / 100] < short * 0.02
        && (0.48..0.94).contains(&(short / long))
        && short > 70.
}
fn tilt(q: [Pt; 4]) -> f64 {
    tilt_for(q, 210. / 297.)
}
fn tilt_for(q: [Pt; 4], ratio: f64) -> f64 {
    let sides = [
        distance(q[0], q[1]),
        distance(q[1], q[2]),
        distance(q[2], q[3]),
        distance(q[3], q[0]),
    ];
    let edge = (sides[0] / sides[2])
        .max(sides[2] / sides[0])
        .max(sides[1] / sides[3])
        .max(sides[3] / sides[1])
        - 1.;
    let aspect = ((sides[0] + sides[2]) / (sides[1] + sides[3]) / ratio)
        .ln()
        .abs();
    edge.max(aspect)
}
fn scan_a4(image: &GrayImage, options: &Options, source_scale: Pt) -> Result<Scan> {
    let normalized = normalize(image);
    let threshold = vision::otsu(&normalized, 0).clamp(50, 220);
    let regions = regions(&normalized, threshold)?;
    let mut candidates: Vec<_> = regions
        .iter()
        .enumerate()
        .filter(|(_, r)| rectangular(r))
        .map(|(i, _)| i)
        .collect();
    candidates.sort_by(|&a, &b| {
        (regions[b].mean * area(&regions[b].contour).abs().sqrt())
            .total_cmp(&(regions[a].mean * area(&regions[a].contour).abs().sqrt()))
    });
    let mut warnings = photo_warnings(image);
    let corners = if let Some(q) = options.a4_corners {
        ensure!(
            q.iter().flatten().all(|v| v.is_finite()),
            "A4 corners must be finite"
        );
        let q = q.map(|p| [p[0] / source_scale[0], p[1] / source_scale[1]]);
        ensure!(
            area(&q) > 100.
                && q.iter().all(|p| p[0] >= 0.
                    && p[1] >= 0.
                    && p[0] < image.width() as f64
                    && p[1] < image.height() as f64),
            "A4 corners must be clockwise, inside the photo, with a 210 mm edge first"
        );
        q
    } else {
        let &first=candidates.first().context("No clear A4 sheet found. Separate a plain white sheet from the pieces on dark cloth, or select its four corners.")?;
        ordered(regions[first].quad.unwrap())
    };
    let mm = [[0., 0.], [210., 0.], [210., -297.], [0., -297.]];
    let map = fit_homography(&corners.into_iter().zip(mm).collect::<Vec<_>>())?;
    let inverse = map.inverse()?;
    let weights = [
        [0., 0.],
        [image.width() as f64, 0.],
        [image.width() as f64, image.height() as f64],
        [0., image.height() as f64],
    ]
    .map(|p| map.0[6] * p[0] + map.0[7] * p[1] + map.0[8]);
    ensure!(
        weights.iter().all(|w| w.is_finite() && *w > 0.05),
        "Photo crosses the reference's perspective horizon. Retake more nearly overhead"
    );
    let mut references = vec![corners.to_vec()];
    let center = [
        (corners[0][0] + corners[2][0]) / 2.,
        (corners[0][1] + corners[2][1]) / 2.,
    ];
    let mmpp = distance(
        map.map(center).context("Invalid scale")?,
        map.map([center[0] + 1., center[1]])
            .context("Invalid scale")?,
    );
    let tilt_score = tilt(corners);
    let mut exclude = BTreeSet::new();
    for (i, r) in regions.iter().enumerate() {
        if r.contour
            .iter()
            .filter(|&&p| geometry::contains(&corners, p))
            .count() as f64
            / r.contour.len() as f64
            > 0.4
            || r.quad.is_some_and(|q| distance(q[0], corners[0]) < 4.)
        {
            exclude.insert(i);
        }
    }
    let primary = *candidates.first().unwrap_or(&usize::MAX);
    if options.a4_corners.is_none() {
        exclude.insert(primary);
    }
    let mut second_ok = false;
    if options.use_second_sheet {
        for &i in &candidates {
            if exclude.contains(&i) {
                continue;
            }
            let q = ordered(regions[i].quad.unwrap());
            let projected: Option<Vec<_>> = q.iter().map(|&p| map.map(p)).collect();
            let Some(p) = projected else {
                continue;
            };
            let a = (distance(p[0], p[1]) + distance(p[2], p[3])) / 2.;
            let b = (distance(p[1], p[2]) + distance(p[3], p[0])) / 2.;
            let err = ((a.min(b) / 210. - 1.).abs()).max((a.max(b) / 297. - 1.).abs());
            if err < 0.04 {
                references.push(q.to_vec());
                exclude.insert(i);
                second_ok = true;
                break;
            }
        }
    }
    if second_ok {
        warnings.push(Warning::new("reference_ambiguous", "Two A4-sized rectangles are excluded as references. Confirm both are blank sheets; otherwise turn off the second-sheet option."));
    }
    if candidates.len() > 1 && !second_ok {
        warnings.push(Warning::new("reference_ambiguous","More than one rectangle could be the sheet. Check the yellow reference or select its corners."));
        if options.use_second_sheet {
            warnings.push(Warning::new("second_sheet_mismatch","A second rectangle does not agree with A4 scale. Check which objects are reference sheets."));
        }
    }
    if tilt_score > 0.18 {
        warnings.push(Warning::new(
            "tilt",
            "The view is tilted. Hold the phone more nearly overhead and keep paper flat.",
        ));
    }
    let mut pieces = Vec::new();
    let mut max_error = 0_f64;
    for (i, r) in regions.iter().enumerate() {
        if exclude.contains(&i) {
            continue;
        }
        let projected: Option<Vec<_>> = r.contour.iter().map(|&p| map.map(p)).collect();
        let Some(contour) = projected else {
            continue;
        };
        if area(&contour).abs() < 100. {
            continue;
        }
        ensure!(
            contour.iter().flatten().all(|v| v.abs() < 20000.),
            "Reference extrapolates too far; retake overhead with the sheet nearer the pieces"
        );
        let id = format!("SCAN-{}", pieces.len() + 1);
        if r.edge {
            let mut warning = Warning::new(
                "piece_outside_photo",
                "A light object touches the photo edge. Its outline is incomplete; retake the photo.",
            );
            warning.piece_id = Some(id.clone());
            warnings.push(warning);
        }
        let outline = geometry::simplify(&contour, 0.35);
        if outline.len() < 3 {
            continue;
        }
        let hull = geometry::hull(outline.clone());
        let solidity = area(&outline).abs() / area(&hull).abs().max(1.);
        if solidity < 0.67 {
            let mut warning = Warning::new(
                "touching_pieces",
                "This outline may join touching pieces, or be strongly concave. Separate pieces and check the outline.",
            );
            warning.piece_id = Some(id.clone());
            warnings.push(warning);
        }
        let far = outline
            .iter()
            .map(|&p| geometry::outline_distance(p, &mm))
            .fold(0_f64, f64::max);
        let error = (mmpp * 1.5 + 0.5) * (1. + far / 210.) * (1. + 2. * tilt_score);
        max_error = max_error.max(error);
        // Mark heuristics are run in photo space at the sheet's local pitch, then
        // transformed through the same homography as the recovered cut outline.
        let photo_outline: Vec<_> = geometry::simplify(&r.contour, 0.7)
            .into_iter()
            .map(|p| [p[0] * mmpp, p[1] * mmpp])
            .collect();
        let (mut marks, grain) = vision::features(&normalized, &photo_outline, mmpp);
        let mut valid_marks = Vec::new();
        for mut m in marks.drain(..) {
            let points: Option<Vec<_>> = m
                .points
                .iter()
                .map(|p| map.map([p[0] / mmpp, p[1] / mmpp]))
                .collect();
            if let Some(points) = points {
                m.points = points;
                valid_marks.push(m);
            }
        }
        valid_marks.extend(vision::cut_notches(&contour));
        let grain = grain.and_then(|g| {
            Some([
                map.map([g[0][0] / mmpp, g[0][1] / mmpp])?,
                map.map([g[1][0] / mmpp, g[1][1] / mmpp])?,
            ])
        });
        let piece = Piece {
            id: id.clone(),
            name: format!("Scanned piece {}", pieces.len() + 1),
            outline,
            grain,
            marks: valid_marks,
            quantity: 1,
            ..Default::default()
        };
        // Detected grain is a suggestion too. It is never silently accepted in Quick mode.
        let mut quick = QuickPiece::new(piece, &options.photo_id, error);
        if let Some(g) = quick.piece.grain.take() {
            quick.suggestions.grain = Some(assist::Suggestion::new(
                g,
                Confidence::Medium,
                "Detected drawn line; confirm it is the grain line",
            ));
        }
        let mut warning = Warning::new(
            "grain_unconfirmed",
            "Confirm the grain direction before making a marker.",
        );
        warning.piece_id = Some(id);
        warnings.push(warning);
        pieces.push(quick);
        if pieces.len() >= options.max_pieces {
            warnings.push(Warning::new(
                "piece_limit",
                "Piece limit reached. Check for omitted pieces and take separate photos.",
            ));
            break;
        }
    }
    ensure!(
        !pieces.is_empty(),
        "No separate paper pieces found. Use dark matte backing and leave gaps around every piece and sheet"
    );
    let low = tilt_score > 0.25
        || max_error > 3.
        || candidates.len() > 1 && !second_ok
        || distance(corners[0], corners[1]) < 180.;
    if low {
        warnings.push(Warning::new("scale_low_confidence","Scale is uncertain away from the A4 sheet. Move the sheet nearer the pieces, retake overhead, or use the BETA target."));
    }
    warnings.push(Warning::new("review_required","Check the yellow A4 reference, every outline and a ruler measurement. Wrinkles, curled paper and lens distortion are not corrected by one sheet."));
    let out = overlay(image, &pieces, |p| inverse.map(p), &references);
    let h = map.0;
    let [sx, sy] = source_scale;
    Ok(Scan {
        photo_id: options.photo_id.clone(),
        pieces,
        overlay_png: png(&out)?,
        scale: Scale {
            mm_per_px: mmpp / sx,
            confidence: if low {
                Confidence::Low
            } else {
                Confidence::Medium
            },
            method: Reference::A4Sheet,
            expected_error_mm: max_error,
            tilt_score,
            reference_corners: references
                .into_iter()
                .map(|r| r.into_iter().map(|p| [p[0] * sx, p[1] * sy]).collect())
                .collect(),
            image_to_mm: [
                h[0] / sx,
                h[1] / sy,
                h[2],
                h[3] / sx,
                h[4] / sy,
                h[5],
                h[6] / sx,
                h[7] / sy,
                h[8],
            ],
        },
        warnings,
        source_size: [0; 2],
        processed_size: [0; 2],
        runtime_ms: 0.,
    })
}
#[derive(Default)]
struct Quality {
    contrast: f64,
    laplacian: f64,
    saturated: f64,
    background_range: f64,
}
fn quality_warnings(q: Quality) -> Vec<Warning> {
    let mut out = Vec::new();
    if q.contrast < 55. {
        out.push(Warning::new(
            "low_contrast",
            "Paper and background are too similar. Use darker matte backing.",
        ));
    }
    if q.laplacian < 18. {
        out.push(Warning::new(
            "blur",
            "Edges may be blurred. Hold the phone still and tap to focus.",
        ));
    }
    if q.saturated > 0.12 {
        out.push(Warning::new(
            "glare",
            "Large bright areas are clipped. Check for glare and reduce direct light.",
        ));
    }
    if q.background_range > 35. {
        out.push(Warning::new(
            "shadow",
            "Lighting varies across the floor. Check outlines in shadows and flatten wrinkles.",
        ));
    }
    out
}
fn photo_warnings(image: &GrayImage) -> Vec<Warning> {
    let w = image.width();
    let h = image.height();
    let mut sum = 0.;
    let mut count = 0_f64;
    for y in (1..h - 1).step_by(2) {
        for x in (1..w - 1).step_by(2) {
            let l = 4. * image[(x, y)][0] as f64
                - image[(x - 1, y)][0] as f64
                - image[(x + 1, y)][0] as f64
                - image[(x, y - 1)][0] as f64
                - image[(x, y + 1)][0] as f64;
            sum += l * l;
            count += 1.;
        }
    }
    let mut back = Vec::new();
    for gy in 0..4 {
        for gx in 0..4 {
            let crop =
                image::imageops::crop_imm(image, gx * w / 4, gy * h / 4, w / 4, h / 4).to_image();
            back.push(percentile(crop.as_raw(), 0.15));
        }
    }
    quality_warnings(Quality {
        contrast: percentile(image.as_raw(), 0.95) as f64 - percentile(image.as_raw(), 0.15) as f64,
        laplacian: sum / count.max(1.),
        saturated: image.as_raw().iter().filter(|&&v| v >= 254).count() as f64
            / image.as_raw().len() as f64,
        background_range: percentile(&back, 0.8) as f64 - percentile(&back, 0.2) as f64,
    })
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionPhoto {
    pub photo_id: String,
    pub pieces: Vec<QuickPiece>,
    pub warnings: Vec<Warning>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Duplicate {
    pub piece_id: String,
    pub duplicate_of: String,
    pub confidence: Confidence,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub pieces: Vec<QuickPiece>,
    pub warnings: Vec<Warning>,
    pub duplicates: Vec<Duplicate>,
}
pub fn merge(photos: &[SessionPhoto]) -> Result<Session> {
    ensure!(
        !photos.is_empty() && photos.len() <= 30,
        "Choose 1..30 photos per session"
    );
    let mut ids = BTreeSet::new();
    let mut pieces = Vec::new();
    let mut warnings = Vec::new();
    for photo in photos {
        ensure!(
            !photo.photo_id.trim().is_empty()
                && !photo.photo_id.contains(':')
                && photo.photo_id.len() <= 100
                && ids.insert(&photo.photo_id),
            "Each photo needs a unique photoId without a colon"
        );
        for source in &photo.pieces {
            let mut p = source.clone();
            p.photo_id = photo.photo_id.clone();
            p.piece.id = format!("{}:{}", photo.photo_id, source.piece.id);
            pieces.push(p);
        }
        warnings.extend(photo.warnings.iter().cloned().map(|mut w| {
            w.photo_id = Some(photo.photo_id.clone());
            w.piece_id = w.piece_id.map(|id| format!("{}:{id}", photo.photo_id));
            w
        }));
    }
    ensure!(
        pieces.len() <= 300,
        "A phone session supports at most 300 pieces"
    );
    assist::validate_pieces(&pieces)?;
    let mut duplicates = Vec::new();
    for i in 0..pieces.len() {
        for j in 0..i {
            if pieces[i].photo_id != pieces[j].photo_id
                && assist::same_shape(&pieces[i].piece.outline, &pieces[j].piece.outline, false)
                    < 0.012
            {
                duplicates.push(Duplicate {
                    piece_id: pieces[i].piece.id.clone(),
                    duplicate_of: pieces[j].piece.id.clone(),
                    confidence: Confidence::Medium,
                });
                let mut w = Warning::new(
                    "duplicate_piece",
                    "This shape may already appear in another photo. Review both; no piece was removed.",
                );
                w.piece_id = Some(pieces[i].piece.id.clone());
                w.photo_id = Some(pieces[i].photo_id.clone());
                warnings.push(w);
                break;
            }
        }
    }
    assist::suggest(&mut pieces)?;
    Ok(Session {
        pieces,
        warnings,
        duplicates,
    })
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickFabric {
    pub width_mm: f64,
    #[serde(default)]
    pub nap: Nap,
    #[serde(default)]
    pub layout: Layout,
}
pub fn pieces_to_job(
    pieces: &[QuickPiece],
    fabric: &QuickFabric,
    garments: BTreeMap<String, u32>,
) -> Result<Job> {
    assist::validate_pieces(pieces)?;
    ensure!(
        !garments.is_empty()
            && garments.len() <= 100
            && garments
                .iter()
                .all(|(s, &n)| !s.trim().is_empty() && n > 0 && n <= 10000),
        "Choose positive garment counts (1..10000) for each size"
    );
    let mut sizes = BTreeSet::new();
    for p in pieces {
        let size = p
            .piece
            .size
            .as_ref()
            .context("Set a size on every accepted piece; scanning does not grade sizes")?;
        let count = garments
            .get(size)
            .context("Every piece size must be in the garment order")?;
        ensure!(
            p.piece
                .quantity
                .checked_add(p.piece.mirror_quantity)
                .and_then(|n| n.checked_mul(*count))
                .is_some_and(|n| n > 0 && n <= 100000),
            "Invalid or excessive piece quantity"
        );
        ensure!(
            p.piece.grain.is_some(),
            "Confirm grain on every piece before creating a marker job"
        );
        sizes.insert(size.clone());
    }
    ensure!(
        garments.keys().all(|s| sizes.contains(s)),
        "Supply real outlines for every ordered size; automatic grading is not available here"
    );
    let job = Job {
        format: JOB_FORMAT.into(),
        name: "Quick Marker".into(),
        fabric: Fabric {
            width: fabric.width_mm,
            nap: fabric.nap,
            layout: fabric.layout,
            ..Default::default()
        },
        pieces: pieces.iter().map(|p| p.piece.clone()).collect(),
        order: Some(Order { sizes: garments }),
        ..Default::default()
    };
    let errors = beta_marker_core::check_job(&job);
    ensure!(
        errors.is_empty(),
        "Invalid accepted job: {}",
        serde_json::to_string(&errors)?
    );
    Ok(job)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn scene() -> GrayImage {
        GrayImage::from_fn(1100, 800, |x, y| {
            Luma([if (40..250).contains(&x) && (40..337).contains(&y) {
                240
            } else if (400..580).contains(&x) && (100..340).contains(&y) && !(x > 520 && y < 170) {
                220
            } else {
                35
            }])
        })
    }
    fn codes(w: &[Warning]) -> Vec<&str> {
        w.iter().map(|w| w.code.as_str()).collect()
    }
    #[test]
    fn a4_recovers_metric_polygon_and_uses_subpixel_corners() {
        let result = scan_image(&scene(), &Options::default()).unwrap();
        assert_eq!(result.pieces.len(), 1);
        assert!((result.scale.mm_per_px - 1.).abs() < 0.02);
        let b = beta_marker_core::geometry::bbox(&result.pieces[0].piece.outline).unwrap();
        assert!((b[0] - 360.).abs() < 1. && (b[2] - 540.).abs() < 1. && (b[1] + 300.).abs() < 1.);
        assert!(result.pieces[0].piece.grain.is_none());
        assert!(codes(&result.warnings).contains(&"grain_unconfirmed"));
        assert_eq!(&result.overlay_png[..8], b"\x89PNG\r\n\x1a\n");
    }
    #[test]
    fn drawn_grain_is_visible_but_remains_unaccepted() {
        let mut image = scene();
        for y in 150..300 {
            for x in 440..442 {
                image[(x, y)] = Luma([20]);
            }
        }
        let result = scan_image(&image, &Options::default()).unwrap();
        let p = &result.pieces[0];
        assert!(p.piece.grain.is_none());
        assert!(
            p.suggestions
                .grain
                .as_ref()
                .unwrap()
                .reason
                .starts_with("Detected drawn line")
        );
        let overlay = image::load_from_memory(&result.overlay_png)
            .unwrap()
            .into_rgb8();
        assert!(overlay.pixels().filter(|p| p.0 == [30, 120, 255]).count() > 50);
    }
    #[test]
    fn warning_blur() {
        assert!(
            codes(&quality_warnings(Quality {
                laplacian: 1.,
                contrast: 150.,
                ..Default::default()
            }))
            .contains(&"blur")
        );
    }
    #[test]
    fn warning_glare() {
        assert!(
            codes(&quality_warnings(Quality {
                saturated: 0.3,
                laplacian: 100.,
                contrast: 150.,
                ..Default::default()
            }))
            .contains(&"glare")
        );
    }
    #[test]
    fn warning_low_contrast() {
        assert!(
            codes(&quality_warnings(Quality {
                contrast: 25.,
                laplacian: 100.,
                ..Default::default()
            }))
            .contains(&"low_contrast")
        );
    }
    #[test]
    fn warning_shadow() {
        assert!(
            codes(&quality_warnings(Quality {
                background_range: 60.,
                laplacian: 100.,
                contrast: 150.,
                ..Default::default()
            }))
            .contains(&"shadow")
        );
    }
    #[test]
    fn warning_outside_photo() {
        let mut image = scene();
        for y in 500..650 {
            for x in 950..1100 {
                image[(x, y)] = Luma([220]);
            }
        }
        let r = scan_image(&image, &Options::default()).unwrap();
        assert!(codes(&r.warnings).contains(&"piece_outside_photo"));
    }
    #[test]
    fn warning_touching_pieces() {
        let mut image = scene();
        for y in 450..720 {
            for x in 500..900 {
                if (x < 610 && y < 560)
                    || (x > 780 && y > 610)
                    || (600..800).contains(&x) && (540..630).contains(&y)
                {
                    image[(x, y)] = Luma([210]);
                }
            }
        }
        let r = scan_image(&image, &Options::default()).unwrap();
        assert!(codes(&r.warnings).contains(&"touching_pieces"));
    }
    #[test]
    fn warning_scale_low_confidence_far_from_sheet() {
        let r = scan_image(&scene(), &Options::default()).unwrap();
        assert!(codes(&r.warnings).contains(&"scale_low_confidence"));
        assert!(r.scale.expected_error_mm > 3.);
    }
    #[test]
    fn warning_tilt_and_selected_reference() {
        let q = [[40., 40.], [250., 40.], [220., 337.], [70., 337.]];
        let r = scan_image(
            &scene(),
            &Options {
                a4_corners: Some(q),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(codes(&r.warnings).contains(&"tilt"));
    }
    #[test]
    fn second_sheet_checks_scale_and_ambiguous_rectangle_warns() {
        let mut image = scene();
        for y in 450..747 {
            for x in 40..250 {
                image[(x, y)] = Luma([240]);
            }
        }
        let r = scan_image(&image, &Options::default()).unwrap();
        assert_eq!(r.scale.reference_corners.len(), 2);
        assert_eq!(r.pieces.len(), 1);
        for y in 450..747 {
            for x in 40..250 {
                image[(x, y)] = Luma([35]);
            }
        }
        for y in 450..650 {
            for x in 40..181 {
                image[(x, y)] = Luma([240]);
            }
        }
        let r = scan_image(&image, &Options::default()).unwrap();
        assert!(codes(&r.warnings).contains(&"reference_ambiguous"));
        assert!(codes(&r.warnings).contains(&"second_sheet_mismatch"));
    }
    #[test]
    fn warning_piece_limit() {
        let r = scan_image(
            &scene(),
            &Options {
                max_pieces: 1,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(codes(&r.warnings).contains(&"piece_limit"));
    }
    #[test]
    fn invalid_inputs_and_uniform_photo_fail_without_allocating_big_buffers() {
        assert!(scan_bytes(b"not a photo", &Options::default()).is_err());
        assert!(
            scan_image(
                &scene(),
                &Options {
                    max_long_edge: 100000,
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert!(
            scan_image(
                &GrayImage::from_pixel(640, 480, Luma([150])),
                &Options::default()
            )
            .is_err()
        );
        assert!(
            scan_image(
                &scene(),
                &Options {
                    a4_corners: Some([[0., 0.]; 4]),
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
    #[test]
    fn merge_warns_without_deleting_and_job_requires_real_sizes_and_grain() {
        let mut p = QuickPiece::new(
            Piece {
                id: "p".into(),
                name: "piece".into(),
                outline: vec![[0., 0.], [100., 0.], [90., 180.], [0., 200.]],
                quantity: 1,
                ..Default::default()
            },
            "a",
            1.,
        );
        let session = merge(&[
            SessionPhoto {
                photo_id: "a".into(),
                pieces: vec![p.clone()],
                warnings: vec![],
            },
            SessionPhoto {
                photo_id: "b".into(),
                pieces: vec![p.clone()],
                warnings: vec![],
            },
        ])
        .unwrap();
        assert_eq!(session.pieces.len(), 2);
        assert_eq!(session.duplicates.len(), 1);
        assert!(codes(&session.warnings).contains(&"duplicate_piece"));
        assert_eq!(session.pieces[0].piece.id, "a:p");
        let fabric = QuickFabric {
            width_mm: 1500.,
            nap: Nap::None,
            layout: Layout::Open,
        };
        let sizes = BTreeMap::from([("M".into(), 1)]);
        assert!(pieces_to_job(&[p.clone()], &fabric, sizes.clone()).is_err());
        p.piece.size = Some("M".into());
        assert!(pieces_to_job(&[p.clone()], &fabric, sizes.clone()).is_err());
        p.piece.grain = Some([[10., 10.], [10., 100.]]);
        assert!(pieces_to_job(&[p.clone()], &fabric, sizes).is_ok());
        assert!(
            pieces_to_job(
                &[p],
                &fabric,
                BTreeMap::from([("M".into(), 1), ("L".into(), 1)])
            )
            .is_err()
        );
    }
}
