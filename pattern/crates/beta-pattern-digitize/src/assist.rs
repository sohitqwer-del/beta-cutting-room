//! Small nearest-neighbour outline model and geometric suggestions. No runtime I/O.
use crate::{
    geometry::{self, area, distance, outline_distance},
    quick::{Confidence, QuickPiece},
};
use anyhow::{Result, ensure};
use beta_marker_core::Pt;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion<T> {
    pub value: T,
    pub confidence: Confidence,
    pub reason: String,
}
impl<T> Suggestion<T> {
    pub fn new(value: T, confidence: Confidence, reason: &str) -> Self {
        Self {
            value,
            confidence,
            reason: reason.into(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quantity {
    pub normal: u32,
    pub mirrored: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Suggestions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<Suggestion<String>>,
    pub pair: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pair_piece_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<Suggestion<Quantity>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symmetry_axis: Option<Suggestion<[Pt; 2]>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cut_on_fold: Option<Suggestion<bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grain: Option<Suggestion<[Pt; 2]>>,
}
impl Default for Suggestions {
    fn default() -> Self {
        Self {
            name: None,
            pair: "single".into(),
            pair_piece_id: None,
            quantity: None,
            symmetry_axis: None,
            cut_on_fold: None,
            grain: None,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Example {
    pub label: String,
    pub source: String,
    pub features: Vec<f64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub format: String,
    pub attribution: String,
    pub examples: Vec<Example>,
}

pub fn validate_pieces(pieces: &[QuickPiece]) -> Result<()> {
    ensure!(
        !pieces.is_empty() && pieces.len() <= 300,
        "Choose 1..300 pieces"
    );
    let mut ids = BTreeSet::new();
    for p in pieces {
        ensure!(
            !p.piece.id.trim().is_empty() && ids.insert(&p.piece.id),
            "Piece IDs must be nonempty and unique"
        );
        ensure!(
            (3..=5000).contains(&p.piece.outline.len())
                && p.piece
                    .outline
                    .iter()
                    .flatten()
                    .all(|v| v.is_finite() && v.abs() < 100000.)
                && area(&p.piece.outline).abs() > 1.,
            "Invalid piece outline: {}",
            p.piece.id
        );
    }
    Ok(())
}
/// Equal arc-length samples keep PCA and descriptors independent of vertex density.
fn resample(poly: &[Pt], n: usize) -> Vec<Pt> {
    let lengths: Vec<_> = (0..poly.len())
        .map(|i| distance(poly[i], poly[(i + 1) % poly.len()]))
        .collect();
    let perimeter = lengths.iter().sum::<f64>();
    let mut edge = 0;
    let mut start = 0.;
    (0..n)
        .map(|i| {
            let t = i as f64 * perimeter / n as f64;
            while edge + 1 < poly.len() && start + lengths[edge] < t {
                start += lengths[edge];
                edge += 1;
            }
            let f = ((t - start) / lengths[edge].max(1e-12)).clamp(0., 1.);
            let a = poly[edge];
            let b = poly[(edge + 1) % poly.len()];
            [a[0] + f * (b[0] - a[0]), a[1] + f * (b[1] - a[1])]
        })
        .collect()
}
fn frame(poly: &[Pt]) -> (Pt, f64) {
    let p = resample(poly, 128);
    let n = p.len() as f64;
    let c = [
        p.iter().map(|p| p[0]).sum::<f64>() / n,
        p.iter().map(|p| p[1]).sum::<f64>() / n,
    ];
    let xx = p.iter().map(|p| (p[0] - c[0]).powi(2)).sum::<f64>();
    let yy = p.iter().map(|p| (p[1] - c[1]).powi(2)).sum::<f64>();
    let xy = p.iter().map(|p| (p[0] - c[0]) * (p[1] - c[1])).sum::<f64>();
    (c, 0.5 * (2. * xy).atan2(xx - yy))
}
fn align(poly: &[Pt], mirror: bool) -> Vec<Pt> {
    let (c, a) = frame(poly);
    let (s, co) = a.sin_cos();
    poly.iter()
        .map(|p| {
            let x = p[0] - c[0];
            let y = p[1] - c[1];
            [
                x * co + y * s,
                (-x * s + y * co) * if mirror { -1. } else { 1. },
            ]
        })
        .collect()
}
/// Shape-only descriptor: area-normalized dimensions, compactness, hull fill,
/// radial quantiles and widths in the principal-axis frame. Mirror invariant.
pub fn descriptor(poly: &[Pt]) -> Vec<f64> {
    let p = align(poly, false);
    let scale = area(poly).abs().sqrt().max(1e-9);
    let x0 = p.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
    let x1 = p.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max);
    let y0 = p.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min);
    let y1 = p.iter().map(|p| p[1]).fold(f64::NEG_INFINITY, f64::max);
    let perimeter = (0..p.len())
        .map(|i| distance(p[i], p[(i + 1) % p.len()]))
        .sum::<f64>();
    let mut result = vec![
        (x1 - x0).max(y1 - y0).ln() - scale.ln(),
        (x1 - x0).min(y1 - y0).ln() - scale.ln(),
        perimeter / scale / 4.,
        area(poly).abs() / area(&geometry::hull(poly.to_vec())).abs().max(1.),
    ];
    let mut radii: Vec<_> = resample(&p, 128)
        .iter()
        .map(|p| p[0].hypot(p[1]) / scale)
        .collect();
    radii.sort_by(f64::total_cmp);
    for i in [12, 32, 64, 96, 115] {
        result.push(radii[i]);
    }
    let mut widths = Vec::new();
    for i in 0..8 {
        let x = x0 + (x1 - x0) * (i as f64 + 0.5) / 8.;
        let mut ys = Vec::new();
        for j in 0..p.len() {
            let a = p[j];
            let b = p[(j + 1) % p.len()];
            if (a[0] <= x && b[0] > x) || (b[0] <= x && a[0] > x) {
                ys.push(a[1] + (x - a[0]) / (b[0] - a[0]) * (b[1] - a[1]));
            }
        }
        widths.push(
            (ys.iter().copied().fold(f64::NEG_INFINITY, f64::max)
                - ys.iter().copied().fold(f64::INFINITY, f64::min))
            .max(0.)
                / scale,
        );
    }
    let reversed: Vec<_> = widths.iter().rev().copied().collect();
    if reversed < widths {
        widths = reversed;
    }
    result.extend(widths);
    result
}
/// 1-NN, deliberately low-confidence. Front/back semantics often share an outline.
pub fn classify(model: &Model, features: &[f64]) -> Option<(String, f64)> {
    model
        .examples
        .iter()
        .filter(|e| e.features.len() == features.len())
        .map(|e| {
            let d = e
                .features
                .iter()
                .zip(features)
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f64>()
                .sqrt();
            (e.label.clone(), d)
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
}
/// Physical-size, rotation/translation-invariant similarity. Does not rescale.
/// Reflection is tested only when explicitly requested for pair suggestions.
pub fn same_shape(a: &[Pt], b: &[Pt], mirror: bool) -> f64 {
    if a.len() < 3 || b.len() < 3 {
        return f64::INFINITY;
    }
    let aa = area(a).abs();
    let ab = area(b).abs();
    if (aa - ab).abs() / aa.max(ab).max(1.) > 0.035 {
        return f64::INFINITY;
    }
    let a = align(&resample(a, 96), false);
    let b = align(&resample(b, 96), mirror);
    let norm = aa.sqrt().max(1.);
    let mut best = f64::INFINITY;
    for flip in [0., std::f64::consts::PI] {
        for correction in [-4_f64, -2., 0., 2., 4.] {
            let angle = flip + correction.to_radians();
            let (s, c) = angle.sin_cos();
            let q: Vec<_> = b
                .iter()
                .map(|p| [p[0] * c - p[1] * s, p[0] * s + p[1] * c])
                .collect();
            let sum = a.iter().map(|&p| outline_distance(p, &q)).sum::<f64>()
                + q.iter().map(|&p| outline_distance(p, &a)).sum::<f64>();
            best = best.min(sum / (a.len() + q.len()) as f64 / norm);
        }
    }
    best
}
fn symmetry(poly: &[Pt]) -> Option<[Pt; 2]> {
    let (center, principal) = frame(poly);
    let sample = resample(poly, 96);
    let scale = area(poly).abs().sqrt();
    let mut best = (f64::INFINITY, 0.);
    for base in [principal, principal + std::f64::consts::FRAC_PI_2] {
        for k in -8..=8 {
            let angle = base + (k as f64 * 0.5).to_radians();
            let (s, c) = angle.sin_cos();
            let mut max = 0_f64;
            let mut sum = 0.;
            for p in &sample {
                let x = p[0] - center[0];
                let y = p[1] - center[1];
                let dot = x * c + y * s;
                let q = [center[0] + 2. * dot * c - x, center[1] + 2. * dot * s - y];
                let d = outline_distance(q, poly);
                max = max.max(d);
                sum += d;
            }
            let score = sum / sample.len() as f64 / scale + max / scale * 0.25;
            if score < best.0 {
                best = (score, angle);
            }
        }
    }
    if best.0 > 0.015 {
        return None;
    }
    let (s, c) = best.1.sin_cos();
    let vals: Vec<_> = poly
        .iter()
        .map(|p| (p[0] - center[0]) * c + (p[1] - center[1]) * s)
        .collect();
    let low = vals.iter().copied().fold(f64::INFINITY, f64::min);
    let high = vals.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    Some([
        [center[0] + low * c, center[1] + low * s],
        [center[0] + high * c, center[1] + high * s],
    ])
}
pub fn suggest(pieces: &mut [QuickPiece]) -> Result<()> {
    validate_pieces(pieces)?;
    let model: Model = serde_json::from_str(include_str!("../model/shape-model.json"))?;
    for p in pieces.iter_mut() {
        p.notches = p
            .piece
            .marks
            .iter()
            .filter(|m| m.kind == beta_marker_core::MarkKind::Notch)
            .cloned()
            .collect();
        p.drills = p
            .piece
            .marks
            .iter()
            .filter(|m| m.kind == beta_marker_core::MarkKind::Drill)
            .cloned()
            .collect();
        let drawn = p.suggestions.grain.take().filter(|g| {
            g.reason.starts_with("Detected drawn line")
                && g.value
                    .iter()
                    .all(|&q| geometry::contains(&p.piece.outline, q))
        });
        let mut suggestions = Suggestions::default();
        if let Some((name, d)) = classify(&model, &descriptor(&p.piece.outline))
            && d < 1.2
        {
            suggestions.name = Some(Suggestion::new(
                name,
                Confidence::Low,
                "Nearest public outline; shape alone cannot reliably identify garment parts",
            ));
        }
        let axis = symmetry(&p.piece.outline);
        if let Some(axis) = axis {
            suggestions.pair = "symmetric".into();
            suggestions.symmetry_axis = Some(Suggestion::new(
                axis,
                Confidence::Medium,
                "Outline approximately matches its reflection",
            ));
            suggestions.cut_on_fold = Some(Suggestion::new(
                true,
                Confidence::Low,
                "Symmetry may permit cutting on fold; confirm construction and grain",
            ));
        }
        let (center, a) = frame(&p.piece.outline);
        let length = area(&p.piece.outline).abs().sqrt() * 0.5;
        suggestions.grain = drawn.or_else(|| {
            Some(Suggestion::new(
                p.piece.grain.or(axis).unwrap_or([
                    [center[0] - a.cos() * length, center[1] - a.sin() * length],
                    [center[0] + a.cos() * length, center[1] + a.sin() * length],
                ]),
                Confidence::Low,
                "Geometric direction only; confirm the actual fabric grain",
            ))
        });
        p.suggestions = suggestions;
    }
    let mut paired = BTreeSet::new();
    for i in 0..pieces.len() {
        if paired.contains(&i) || pieces[i].suggestions.symmetry_axis.is_some() {
            continue;
        }
        for j in i + 1..pieces.len() {
            if paired.contains(&j) || pieces[j].suggestions.symmetry_axis.is_some() {
                continue;
            }
            let mirrored = same_shape(&pieces[i].piece.outline, &pieces[j].piece.outline, true);
            let direct = same_shape(&pieces[i].piece.outline, &pieces[j].piece.outline, false);
            if mirrored < 0.012 && direct > mirrored + 0.004 {
                let left = pieces[i].piece.id.clone();
                let right = pieces[j].piece.id.clone();
                for (index, other) in [(i, right), (j, left)] {
                    let s = &mut pieces[index].suggestions;
                    s.pair = "mirror".into();
                    s.pair_piece_id = Some(other);
                    s.quantity = Some(Suggestion::new(
                        Quantity {
                            normal: 1,
                            mirrored: 1,
                        },
                        Confidence::Medium,
                        "If these are a pair, retain one representative with 1+1 copies; do not apply to both",
                    ));
                }
                paired.insert(i);
                paired.insert(j);
                break;
            }
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use beta_marker_core::Piece;
    fn piece(id: &str, outline: Vec<Pt>) -> QuickPiece {
        QuickPiece::new(
            Piece {
                id: id.into(),
                outline,
                quantity: 1,
                ..Default::default()
            },
            "a",
            1.,
        )
    }
    #[test]
    fn symmetry_offers_fold_but_never_changes_accepted_settings() {
        let p = vec![
            [0., 0.],
            [100., 0.],
            [120., 120.],
            [50., 180.],
            [-20., 120.],
        ];
        let mut pieces = vec![piece("a", p)];
        suggest(&mut pieces).unwrap();
        assert_eq!(pieces[0].suggestions.pair, "symmetric");
        assert!(pieces[0].suggestions.cut_on_fold.is_some());
        assert!(pieces[0].piece.fold_line.is_none());
        assert!(pieces[0].piece.grain.is_none());
    }
    #[test]
    fn mirror_pair_offers_one_plus_one_and_keeps_demand_unchanged() {
        let p = vec![[0., 0.], [160., 0.], [150., 220.], [90., 190.], [40., 120.]];
        let mirrored = p.iter().map(|p| [-p[0] + 600., p[1] + 100.]).collect();
        let mut pieces = vec![piece("a", p), piece("b", mirrored)];
        suggest(&mut pieces).unwrap();
        assert_eq!(pieces[0].suggestions.pair, "mirror");
        assert_eq!(
            pieces[0]
                .suggestions
                .quantity
                .as_ref()
                .unwrap()
                .value
                .mirrored,
            1
        );
        assert_eq!(pieces[0].piece.mirror_quantity, 0);
    }
    #[test]
    fn duplicate_similarity_uses_size_and_is_rotation_invariant() {
        let p = vec![[0., 0.], [160., 0.], [150., 220.], [90., 190.], [40., 120.]];
        let q: Vec<_> = p.iter().map(|p| [600. - p[1], 200. + p[0]]).collect();
        assert!(same_shape(&p, &q, false) < 0.001);
        let large: Vec<_> = p.iter().map(|p| [p[0] * 1.1, p[1] * 1.1]).collect();
        assert!(same_shape(&p, &large, false) > 0.012);
    }
    #[test]
    fn descriptor_does_not_depend_on_translation_rotation_or_scale() {
        let p = vec![[0., 0.], [160., 0.], [150., 220.], [90., 190.], [40., 120.]];
        let q: Vec<_> = p
            .iter()
            .map(|p| [600. - p[1] * 2., 200. + p[0] * 2.])
            .collect();
        let a = descriptor(&p);
        let b = descriptor(&q);
        assert!(a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-6));
    }
    #[test]
    fn model_is_small_and_names_always_low_confidence() {
        let model: Model = serde_json::from_str(include_str!("../model/shape-model.json")).unwrap();
        assert!(model.examples.len() > 20 && model.examples.len() < 300);
        assert_eq!(model.format, "beta-quick-shape/v1");
    }
}
