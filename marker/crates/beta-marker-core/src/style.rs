//! Spreading instructions and bounded repeat/difference geometry for the style UI.
use crate::{Job, Layout, MarkerResult, Piece, Pt, geometry, rules};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// First physical ply face. Does not authorize mirrored pattern placement.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Face {
    #[default]
    Up,
    Down,
    FaceToFace,
}
/// Extra spreading instructions, saved at `Job.meta.beta_style_lay`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaySettings {
    pub face: Face,
    pub plies: u32,
    pub end_allowance_mm: f64,
}
impl Default for LaySettings {
    fn default() -> Self {
        Self {
            face: Face::Up,
            plies: 1,
            end_allowance_mm: 0.,
        }
    }
}
/// Colour stripe, repeated along the named axis. X stripes run across the width.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepeatBand {
    pub axis: Axis,
    pub offset_mm: f64,
    pub width_mm: f64,
    pub colour: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Axis {
    X,
    Y,
}
impl Axis {
    fn index(self) -> usize {
        usize::from(self == Self::Y)
    }
}
/// One centre line, clipped to a placed cut outline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayLine {
    pub placement: usize,
    pub piece_id: String,
    pub axis: Axis,
    pub points: [Pt; 2],
    pub colour: String,
    pub width_mm: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayOutline {
    pub piece_id: String,
    pub size: Option<String>,
    pub points: Vec<Pt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placement: Option<usize>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Difference {
    pub kind: String,
    pub piece_id: String,
    pub before: Vec<Pt>,
    pub after: Vec<Pt>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviousMarker {
    pub job: Job,
    pub result: MarkerResult,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OverlayOptions {
    pub bands: Option<Vec<RepeatBand>>,
    #[serde(default)]
    pub graded: Vec<Piece>,
    pub previous: Option<PreviousMarker>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkerOverlay {
    pub matches: Vec<crate::model::MatchReport>,
    pub stripes: Vec<OverlayLine>,
    pub outlines: Vec<OverlayOutline>,
    pub graded: Vec<OverlayOutline>,
    pub previous: Vec<Difference>,
    pub spread_length_mm: f64,
}
/// Read and validate physical spreading instructions, with layout-aware defaults.
pub fn lay_settings(job: &Job) -> Result<LaySettings, String> {
    let lay: LaySettings = match job.meta.get("beta_style_lay") {
        Some(value) => serde_json::from_value(value.clone())
            .map_err(|e| format!("Invalid lay settings: {e}"))?,
        None => LaySettings {
            plies: if job.fabric.layout == Layout::Open {
                1
            } else {
                2
            },
            ..Default::default()
        },
    };
    if lay.plies == 0 || lay.plies > 10000 {
        return Err("Choose 1 to 10000 physical plies.".into());
    }
    if (job.fabric.layout != Layout::Open || lay.face == Face::FaceToFace)
        && !lay.plies.is_multiple_of(2)
    {
        return Err(
            "Folded, tubular and face-to-face lays need an even number of physical plies.".into(),
        );
    }
    if !lay.end_allowance_mm.is_finite() || !(0. ..=10000.).contains(&lay.end_allowance_mm) {
        return Err("End allowance must be finite, from 0 to 10000 mm at each end.".into());
    }
    Ok(lay)
}
/// Stored drawing bands, independent of repeat phase constraints.
pub fn repeat_bands(job: &Job) -> Result<Vec<RepeatBand>, String> {
    let bands = match job.meta.get("beta_repeat_bands") {
        Some(v) => {
            serde_json::from_value(v.clone()).map_err(|e| format!("Invalid repeat bands: {e}"))?
        }
        None => vec![],
    };
    check_bands(job, &bands)?;
    Ok(bands)
}
fn check_bands(job: &Job, bands: &[RepeatBand]) -> Result<(), String> {
    if bands.len() > 128 {
        return Err("Use at most 128 colour bands.".into());
    }
    for band in bands {
        let period = job
            .fabric
            .repeat
            .and_then(|r| {
                if band.axis == Axis::X {
                    r.period_x
                } else {
                    r.period_y
                }
            })
            .ok_or("Each colour band needs a repeat on its axis.")?;
        if !period.is_finite()
            || period <= 0.
            || !band.offset_mm.is_finite()
            || !band.width_mm.is_finite()
            || band.width_mm <= 0.
            || band.width_mm > period
        {
            return Err("Band widths must be positive and no wider than their repeat; offsets must be finite.".into());
        }
        if band.colour.len() != 7
            || !band.colour.starts_with('#')
            || !band.colour.as_bytes()[1..]
                .iter()
                .all(u8::is_ascii_hexdigit)
        {
            return Err("Use a #RRGGBB colour for each band.".into());
        }
    }
    Ok(())
}
/// Safe point-in-polygon for hit tests and overlay tests; boundary counts as inside.
pub fn contains(point: Pt, polygon: &[Pt]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let mut inside = false;
    for i in 0..polygon.len() {
        let a = polygon[i];
        let b = polygon[(i + 1) % polygon.len()];
        let cross = (b[0] - a[0]) * (point[1] - a[1]) - (b[1] - a[1]) * (point[0] - a[0]);
        if cross.abs() < 1e-8
            && point[0] >= a[0].min(b[0]) - 1e-8
            && point[0] <= a[0].max(b[0]) + 1e-8
            && point[1] >= a[1].min(b[1]) - 1e-8
            && point[1] <= a[1].max(b[1]) + 1e-8
        {
            return true;
        }
        if (a[1] > point[1]) != (b[1] > point[1])
            && point[0] < (b[0] - a[0]) * (point[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            inside = !inside;
        }
    }
    inside
}
/// Clip an infinite axis-aligned line using even/odd polygon intersections.
pub fn clip_line(polygon: &[Pt], axis: Axis, coordinate: f64) -> Vec<[Pt; 2]> {
    let d = axis.index();
    let other = 1 - d;
    let mut crossings = vec![];
    for i in 0..polygon.len() {
        let a = polygon[i];
        let b = polygon[(i + 1) % polygon.len()];
        // Half-open vertex rule avoids duplicate tangent intersections.
        if (a[d] <= coordinate && coordinate < b[d]) || (b[d] <= coordinate && coordinate < a[d]) {
            crossings.push(a[other] + (coordinate - a[d]) * (b[other] - a[other]) / (b[d] - a[d]));
        }
    }
    crossings.sort_by(f64::total_cmp);
    crossings
        .as_chunks::<2>()
        .0
        .iter()
        .filter_map(|v| {
            if v[1] - v[0] < 1e-8 {
                return None;
            }
            let mut a = [0.; 2];
            let mut b = a;
            a[d] = coordinate;
            b[d] = coordinate;
            a[other] = v[0];
            b[other] = v[1];
            Some([a, b])
        })
        .collect()
}
fn outlines(job: &Job, result: &MarkerResult) -> Result<Vec<OverlayOutline>, String> {
    if result.placements.len() > 20000 {
        return Err("Overlay has too many placements.".into());
    }
    let effective = rules::effective_job(job);
    let mut out = vec![];
    for (i, pl) in result.placements.iter().enumerate() {
        let piece = effective
            .pieces
            .iter()
            .find(|p| p.id == pl.piece_id)
            .ok_or_else(|| format!("Overlay piece '{}' was not found.", pl.piece_id))?;
        let points = rules::cut_outline(piece, pl);
        if points.len() < 3
            || points.len() > 100000
            || points.iter().flatten().any(|n| !n.is_finite())
        {
            return Err("Overlay contains an invalid outline.".into());
        }
        out.push(OverlayOutline {
            piece_id: piece.id.clone(),
            size: piece.size.clone(),
            points,
            placement: Some(i),
        });
    }
    Ok(out)
}
/// Build clipped repeats, size outlines and a previous-version difference without I/O.
pub fn overlay(
    job: &Job,
    result: &MarkerResult,
    options: &OverlayOptions,
) -> Result<MarkerOverlay, String> {
    let errors = crate::check_job(job);
    if !errors.is_empty() {
        return Err(errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("; "));
    }
    let lay = lay_settings(job)?;
    let outlines = outlines(job, result)?;
    let mut bands = options.bands.clone().unwrap_or(repeat_bands(job)?);
    check_bands(job, &bands)?;
    if bands.is_empty()
        && let Some(r) = job.fabric.repeat
    {
        for (axis, period) in [(Axis::X, r.period_x), (Axis::Y, r.period_y)] {
            if let Some(period) = period {
                bands.push(RepeatBand {
                    axis,
                    offset_mm: 0.,
                    width_mm: 0.3_f64.min(period),
                    colour: "#64748b".into(),
                });
            }
        }
    }
    let mut stripes = vec![];
    let mut candidates = 0usize;
    for o in &outlines {
        let bounds = geometry::bbox(&o.points).ok_or("Empty overlay outline.")?;
        for band in &bands {
            let r = job.fabric.repeat.ok_or("Stripe bands need a repeat.")?;
            let d = band.axis.index();
            let period = if d == 0 { r.period_x } else { r.period_y }
                .ok_or("Stripe band axis has no repeat.")?;
            let offset = if d == 0 { r.offset_x } else { r.offset_y } + band.offset_mm;
            let first = offset + ((bounds[d] - offset) / period).ceil() * period;
            let count = ((bounds[d + 2] - first) / period).floor() + 1.;
            if !count.is_finite() || count > 20000. {
                return Err("Repeat is too fine for this overlay. Increase the spacing.".into());
            }
            let count = count.max(0.) as usize;
            candidates = candidates.saturating_add(count);
            if candidates > 50000 {
                return Err(
                    "Overlay exceeds 50000 stripe candidates. Show fewer pieces or a wider repeat."
                        .into(),
                );
            }
            for k in 0..count {
                for points in clip_line(&o.points, band.axis, first + k as f64 * period) {
                    if stripes.len() >= 50000 {
                        return Err("Overlay exceeds 50000 clipped segments.".into());
                    }
                    stripes.push(OverlayLine {
                        placement: o.placement.unwrap_or(0),
                        piece_id: o.piece_id.clone(),
                        axis: band.axis,
                        points,
                        colour: band.colour.clone(),
                        width_mm: band.width_mm,
                    });
                }
            }
        }
    }
    if options.graded.len() > 10000 {
        return Err("Too many graded outlines.".into());
    }
    let mut graded = vec![];
    for p in &options.graded {
        if p.outline.len() < 3 || p.outline.iter().flatten().any(|n| !n.is_finite()) {
            return Err("A graded overlay outline is invalid.".into());
        }
        graded.push(OverlayOutline {
            piece_id: p.id.clone(),
            size: p.size.clone(),
            points: p.outline.clone(),
            placement: None,
        });
    }
    let mut previous = vec![];
    if let Some(old) = &options.previous {
        let before = self::outlines(&old.job, &old.result)?;
        let key = |p: &crate::Placement| (p.piece_id.clone(), p.copy, p.mirrored);
        let mut old_map: BTreeMap<_, _> = old
            .result
            .placements
            .iter()
            .zip(&before)
            .map(|(p, o)| (key(p), o))
            .collect();
        if old_map.len() != old.result.placements.len() {
            return Err("Previous marker repeats a placement identity.".into());
        }
        let mut seen = std::collections::BTreeSet::new();
        for (p, o) in result.placements.iter().zip(&outlines) {
            if !seen.insert(key(p)) {
                return Err("Current marker repeats a placement identity.".into());
            }
            let old = old_map.remove(&key(p));
            let kind = if let Some(old) = old {
                if old.points.len() == o.points.len()
                    && old
                        .points
                        .iter()
                        .zip(&o.points)
                        .all(|(a, b)| (a[0] - b[0]).hypot(a[1] - b[1]) <= 0.01)
                {
                    "unchanged"
                } else {
                    "moved"
                }
            } else {
                "added"
            };
            previous.push(Difference {
                kind: kind.into(),
                piece_id: o.piece_id.clone(),
                before: old.map(|o| o.points.clone()).unwrap_or_default(),
                after: o.points.clone(),
            });
        }
        for o in old_map.values() {
            previous.push(Difference {
                kind: "removed".into(),
                piece_id: o.piece_id.clone(),
                before: o.points.clone(),
                after: vec![],
            });
        }
    }
    let length = outlines
        .iter()
        .flat_map(|o| &o.points)
        .map(|p| p[0])
        .fold(0., f64::max);
    Ok(MarkerOverlay {
        matches: crate::validate::match_reports(job, result)?,
        stripes,
        outlines,
        graded,
        previous,
        spread_length_mm: length + 2. * lay.end_allowance_mm,
    })
}
