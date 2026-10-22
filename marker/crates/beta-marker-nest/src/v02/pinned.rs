//! Fold halves inside Sparrow. Sparrow cannot fix an item's y position, so a
//! fold half is joined into an engine outline that spans the usable width to
//! within a few micrometres, which leaves it free only along the marker:
//!
//! - folded fabric: the half plus a thin bar that reaches the selvedge;
//! - a tube: a half on the top fold joined by a short bar to a half on the
//!   bottom fold, so both folds are used at the same place along the marker
//!   (a half without a partner gets a bar to the other fold).
//!
//! After the search the bars are dropped and every half is snapped exactly
//! onto its fold edge. The snap is smaller than the engine's clearance guard.
use super::{Slot, bounded};
use anyhow::{Context, Result, bail, ensure};
use beta_marker_core::geometry::{
    angle_diff_deg, bbox, norm_deg, transform_point, transform_points,
};
use beta_marker_core::{Job, Layout, Placement, Pt, RotationRule};
use geo::{BooleanOps, Coord, Distance, Euclidean, LineString, MultiPolygon, Polygon};
use std::collections::BTreeMap;

/// Vertical play left to a pinned outline inside the engine strip, mm. It must
/// stay below the engine's own clearance guard, so snapping never creates contact.
const SLACK: f64 = 0.004;
/// Bar thickness along the marker, mm. Piece gaps are added by the engine.
const BAR: f64 = 1.0;
/// Largest snap accepted when restoring a half, mm.
pub(super) const SNAP_LIMIT: f64 = 0.05;

/// One fold half in the frame of its engine outline.
#[derive(Clone, Debug)]
struct Half {
    mirrored: bool,
    rotation: f64,
    origin: Pt,
    points: Vec<Pt>,
}

impl Half {
    fn shift(&mut self, dx: f64, dy: f64) {
        self.origin = [self.origin[0] + dx, self.origin[1] + dy];
        for p in &mut self.points {
            *p = [p[0] + dx, p[1] + dy];
        }
    }

    fn bounds(&self) -> Result<[f64; 4]> {
        bbox(&self.points).context("empty fold half")
    }

    fn placement(&self, template: &Placement, turn: f64, translate: Pt) -> Placement {
        let origin = transform_point(self.origin, false, turn, [0.0, 0.0]);
        let mut pl = template.clone();
        pl.on_fold = true;
        pl.mirrored = self.mirrored;
        pl.rotation_deg = norm_deg(self.rotation + turn);
        pl.translate = [origin[0] + translate[0], origin[1] + translate[1]];
        pl
    }
}

/// The engine representation of one or two fold halves.
#[derive(Clone, Debug)]
pub(super) struct Pin {
    /// Half with its fold on the top edge (y = width).
    top: Half,
    /// A tube's half on the bottom fold (y = 0) and its slot.
    bottom: Option<(usize, Half)>,
    /// The outline may turn by 180 degrees (a tube), swapping the folds.
    half_turn: bool,
    /// Lowest y of the bar of a single half.
    bar_bottom: f64,
    bar_left: bool,
}

fn polygon(points: &[Pt]) -> Polygon<f64> {
    Polygon::new(
        LineString::from(
            points
                .iter()
                .map(|p| Coord { x: p[0], y: p[1] })
                .collect::<Vec<_>>(),
        ),
        vec![],
    )
}

impl Pin {
    /// Slot of the bottom half, when this outline carries two halves.
    pub fn partner(&self) -> Option<usize> {
        self.bottom.as_ref().map(|(slot, _)| *slot)
    }

    /// Engine outline: the halves joined by their bar.
    pub fn outline(&self) -> Result<Vec<Pt>> {
        let t = self.top.bounds()?;
        let mut union = MultiPolygon::new(vec![polygon(&self.top.points)]);
        let bar = if let Some((_, bottom)) = &self.bottom {
            let b = bottom.bounds()?;
            let (lo, hi) = (t[0].max(b[0]), t[2].min(b[2]));
            ensure!(
                hi - lo > BAR,
                "paired fold halves do not overlap along the marker"
            );
            union = union.union(&MultiPolygon::new(vec![polygon(&bottom.points)]));
            let x = (lo + hi - BAR) / 2.0;
            [[x, b[1]], [x + BAR, b[1]], [x + BAR, t[3]], [x, t[3]]]
        } else {
            let x = if self.bar_left { t[0] } else { t[2] - BAR };
            [
                [x, self.bar_bottom],
                [x + BAR, self.bar_bottom],
                [x + BAR, t[3]],
                [x, t[3]],
            ]
        };
        let union = union.union(&MultiPolygon::new(vec![polygon(&bar)]));
        let largest = union
            .0
            .iter()
            .max_by(|a, b| geo::Area::unsigned_area(*a).total_cmp(&geo::Area::unsigned_area(*b)))
            .context("empty pinned outline")?;
        let mut points: Vec<Pt> = largest.exterior().coords().map(|c| [c.x, c.y]).collect();
        if points.len() > 1 && points.first() == points.last() {
            points.pop();
        }
        ensure!(points.len() >= 3, "degenerate pinned outline");
        Ok(points)
    }

    /// Make the outline taller by `extra` mm (negative makes it shorter).
    fn extend(&mut self, extra: f64) {
        match &mut self.bottom {
            Some((_, bottom)) => bottom.shift(0.0, -extra),
            None => self.bar_bottom -= extra,
        }
    }

    fn height(&self) -> Result<f64> {
        let t = self.top.bounds()?;
        Ok(match &self.bottom {
            Some((_, bottom)) => t[3] - bottom.bounds()?[1],
            None => t[3] - self.bar_bottom,
        })
    }

    /// Placements of the real halves for one engine placement of the outline,
    /// as `(slot, placement)`. `own` is the slot of the top half.
    pub fn placements(
        &self,
        slots: &[Slot],
        own: usize,
        engine_rotation: f64,
        translate: Pt,
    ) -> Vec<(usize, Placement)> {
        let turn = if angle_diff_deg(engine_rotation, 180.0) < 1.0 {
            180.0
        } else {
            0.0
        };
        let mut out = vec![(
            own,
            self.top.placement(&slots[own].placement, turn, translate),
        )];
        if let Some((slot, bottom)) = &self.bottom {
            out.push((
                *slot,
                bottom.placement(&slots[*slot].placement, turn, translate),
            ));
        }
        out
    }
}

/// Move a restored half by at most [`SNAP_LIMIT`] so its fold line lies
/// exactly on the nearest fold edge.
pub(super) fn snap(job: &Job, pl: &mut Placement) -> Result<()> {
    let piece = job
        .pieces
        .iter()
        .find(|p| p.id == pl.piece_id)
        .context("unknown pinned piece")?;
    let line = transform_points(
        &piece.fold_line.context("pinned piece has no fold line")?,
        pl.mirrored,
        pl.rotation_deg,
        pl.translate,
    );
    let y = (line[0][1] + line[1][1]) / 2.0;
    let target = if job.fabric.layout == Layout::Tubular && y < job.fabric.width / 2.0 {
        0.0
    } else {
        job.fabric.width
    };
    ensure!(
        (target - y).abs() <= SNAP_LIMIT,
        "pinned fold half drifted {:.4} mm from its fold",
        (target - y).abs()
    );
    pl.translate[1] += target - y;
    Ok(())
}

/// Size every outline so that, as inflated by the engine, it fills the strip
/// height but for [`SLACK`]. This measures the engine's own collision outlines,
/// so rounding and offsets are included. Engine piece `k` carries `pins[owner[k]]`.
pub(super) fn calibrate(engine: &mut Job, pins: &mut [Option<Pin>], owner: &[usize]) -> Result<()> {
    let shift = engine.spacing.edge_gap - engine.spacing.piece_gap / 2.0;
    let height = engine.fabric.width - 2.0 * shift;
    for (k, &slot) in owner.iter().enumerate() {
        if let Some(pin) = &mut pins[slot] {
            let radius = engine.spacing.piece_gap / 2.0 + engine.pieces[k].buffer;
            let target = height - SLACK - 2.0 * (radius * 1.002 + 0.01);
            let extra = target - pin.height()?;
            pin.extend(extra);
            ensure!(
                pin.bottom.is_some() || pin.top.bounds()?[1] - pin.bar_bottom > BAR,
                "fold half is too deep to pin inside the usable width"
            );
            engine.pieces[k].outline = pin.outline()?;
        }
    }
    for _ in 0..4 {
        let mapping = beta_marker_core::demands(engine);
        let (instance, _) = crate::build_instance(engine, &mapping)?;
        let mut worst = 0.0_f64;
        let mut corrections = Vec::new();
        for (item, _) in &instance.items {
            let k = mapping
                .get(usize::try_from(item.external_id())?)
                .context("unknown engine item")?
                .piece_index;
            if pins[owner[k]].is_some() {
                let b = item.shape_cd().bbox();
                let error = height - SLACK - f64::from(b.y_max - b.y_min);
                worst = worst.max(error.abs());
                corrections.push((k, error));
            }
        }
        if worst <= SLACK / 4.0 {
            return Ok(());
        }
        for (k, error) in corrections {
            let pin = pins[owner[k]].as_mut().context("missing pin")?;
            pin.extend(error);
            engine.pieces[k].outline = pin.outline()?;
        }
    }
    bail!("could not fit pinned fold halves to the engine strip")
}

/// Choose an engine representation for every fold-half slot. On a tube, halves
/// of similar length are paired across the two folds. `pins[i]` is `None` for
/// full cuts and for the bottom half of a pair.
pub(super) fn plan(
    job: &Job,
    slots: &[Slot],
    variant: usize,
    directions: &BTreeMap<u32, f64>,
) -> Result<Vec<Option<Pin>>> {
    let tube = job.fabric.layout == Layout::Tubular;
    // (slot, top half, bottom half if any, may turn)
    let mut halves = Vec::new();
    for (index, slot) in slots.iter().enumerate() {
        if !slot.placement.on_fold {
            continue;
        }
        let piece = &job.pieces[slot.piece];
        ensure!(
            piece.block_margin.is_none() && piece.fold_line.is_some(),
            "fold half '{}' cannot be pinned",
            piece.id
        );
        let choices = bounded::fold_orientations(job, slot, variant, directions)?;
        // Alternate hands between slots and runs for variety.
        let pick = |top: bool| {
            let list: Vec<_> = choices.iter().filter(|c| c.1 == top).collect();
            list.get((index + variant / 2) % list.len().max(1))
                .map(|(pl, _, points)| Half {
                    mirrored: pl.mirrored,
                    rotation: pl.rotation_deg,
                    origin: pl.translate,
                    points: points.clone(),
                })
        };
        let top =
            pick(true).with_context(|| format!("no pinned fold direction for '{}'", piece.id))?;
        let turn = tube
            && job.fabric.nap == beta_marker_core::Nap::None
            && beta_marker_core::allowed_rotations(job, piece, top.mirrored)
                .allows(top.rotation + 180.0);
        halves.push((index, top, if tube { pick(false) } else { None }, turn));
    }
    let length = |h: &Half| h.bounds().map_or(0.0, |b| b[2] - b[0]);
    if tube {
        halves.sort_by(|a, b| length(&b.1).total_cmp(&length(&a.1)).then(a.0.cmp(&b.0)));
    }
    let gap = job.spacing.piece_gap + 0.05;
    let mut pins = vec![None; slots.len()];
    let mut k = 0;
    while k < halves.len() {
        let (slot, top, _, turn) = halves[k].clone();
        let mut pin = Pin {
            top,
            bottom: None,
            half_turn: turn,
            bar_bottom: 0.0,
            bar_left: (slot + variant).is_multiple_of(2),
        };
        k += 1;
        if let Some((partner, _, Some(mut bottom), partner_turn)) = halves.get(k).cloned() {
            // Centre the bottom half under the top half.
            let (t, b) = (pin.top.bounds()?, bottom.bounds()?);
            bottom.shift((t[0] + t[2] - b[0] - b[2]) / 2.0, 0.0);
            if Euclidean.distance(&polygon(&pin.top.points), &polygon(&bottom.points)) >= gap {
                pin.bottom = Some((partner, bottom));
                pin.half_turn = turn && partner_turn;
                k += 1;
            }
        }
        pins[slot] = Some(pin);
    }
    Ok(pins)
}

/// Engine rotation rule for a joined outline.
pub(super) fn rotation_rule(pin: &Pin) -> RotationRule {
    if pin.half_turn {
        RotationRule::Flip180
    } else {
        RotationRule::Fixed
    }
}
