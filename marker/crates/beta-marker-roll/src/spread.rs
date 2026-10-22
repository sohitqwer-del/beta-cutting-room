//! Bounded exact search for small whole-ply problems and a heuristic portfolio.

use crate::RollRegister;
use crate::model::{nonnegative, positive};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const EPS: f64 = 1e-8;

type PlyPath = Vec<(usize, f64, f64)>;

/// Shade identities are scoped to a lot and dye lot, not just a colour label.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ShadeIdentity {
    /// Receiving lot.
    pub lot: String,
    /// Dye batch within the lot.
    pub dye_lot: String,
    /// Colour group within that batch.
    pub shade_group: String,
}

/// The lay must use one shade identity unless mixing is explicitly enabled.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "mode")]
pub enum ShadePolicy {
    /// Choose one compatible lot/dye/shade identity for the whole lay.
    #[default]
    SameGroup,
    /// Restrict to this exact lot/dye/shade identity.
    Required {
        /// Exact permitted stock identity.
        identity: ShadeIdentity,
    },
    /// The factory accepts shade mixing in this lay; output still warns about it.
    AllowMixing,
}

/// A safe overlap/cut interval along the marker, in metres.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SpliceZone {
    /// First safe overlap position along the marker, in metres.
    pub start_m: f64,
    /// Last safe overlap endpoint along the marker, in metres.
    pub end_m: f64,
}

/// Independent cut-plan adapter input; no dependency on the plan crate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayRequest {
    /// Unique cut-plan lay identity.
    pub id: String,
    /// Length of the final nested marker, in metres.
    pub marker_length_m: f64,
    /// Passes to lay: whole lengths taken from rolls. The JSON name is kept for
    /// file compatibility. Open fabric lays one ply per pass; folded and tubular
    /// fabric lay two (see `beta_marker_plan::plies_per_pass`).
    pub plies: u32,
    /// Width of the final marker, in millimetres.
    pub fabric_width_mm: f64,
    /// Stock colour compatibility rule.
    #[serde(default)]
    pub shade_rule: ShadePolicy,
    /// Optional stock restriction (for example, one shrinkage group).
    #[serde(default)]
    pub eligible_roll_ids: Vec<String>,
    /// Whether a ply can be assembled from several roll pieces.
    #[serde(default)]
    pub allow_splicing: bool,
    /// Safe intervals for the entire physical overlap.
    #[serde(default)]
    pub splice_zones: Vec<SpliceZone>,
}

/// Search limits and physical overlap allowance. All runs are deterministic.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AllocationSettings {
    /// Physical extra cloth at each splice, in metres.
    pub splice_overlap_m: f64,
    /// Largest total ply count eligible for whole-ply exact search (at most 64).
    pub exact_max_plies: usize,
    /// Largest register eligible for exact search (at most 64 rolls).
    pub exact_max_rolls: usize,
    /// Maximum exact-search nodes; zero disables the exact pass.
    pub exact_node_limit: usize,
    /// Includes descending-length first-fit and best-fit, then varied tie breaks.
    pub heuristic_trials: usize,
    /// Bound the backtracking search for one spliced ply.
    pub splice_node_limit: usize,
}

impl Default for AllocationSettings {
    fn default() -> Self {
        Self {
            splice_overlap_m: 0.1,
            exact_max_plies: 14,
            exact_max_rolls: 12,
            exact_node_limit: 100_000,
            heuristic_trials: 24,
            splice_node_limit: 4000,
        }
    }
}

/// A forward-facing piece of one ply, mapped from roll to marker coordinates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpreadSegment {
    /// Source stock identity.
    pub roll_id: String,
    /// One-based ply number within this lay.
    pub ply: u32,
    /// Measured distance from the roll start, in metres.
    pub roll_start_m: f64,
    /// Start along the marker, in metres.
    pub marker_start_m: f64,
    /// Includes overlap material, if any.
    pub length_m: f64,
}

/// The outgoing piece ends at position + overlap; the incoming starts at position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpliceInstruction {
    /// One-based ply number within the lay.
    pub ply: u32,
    /// Outgoing stock identity.
    pub from_roll: String,
    /// Incoming stock identity.
    pub to_roll: String,
    /// Incoming piece's marker start, in metres.
    pub marker_position_m: f64,
    /// Outgoing cloth extending past the join, in metres.
    pub overlap_m: f64,
}

/// One table row; partial plies count once for each contributing roll.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RollAllocation {
    /// Source stock identity.
    pub roll_id: String,
    /// Number of distinct passes this roll contributes to, including split passes.
    pub plies: u32,
    /// Number of uninterrupted whole passes from this roll in this lay.
    pub full_plies: u32,
    /// Cloth consumed in this lay, including overlaps, in metres.
    pub used_m: f64,
    /// Final residual, reported only in the last lay using this roll.
    pub end_bit_m: f64,
    /// Recorded colour group (lot and dye lot remain available in the register).
    pub shade_group: String,
}

/// Requested and fulfilled plies plus executable forward spread coordinates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LaySpread {
    /// Original request; marker geometry and constraints remain auditable.
    pub lay: LayRequest,
    /// Number of fully covered passes. No partial pass consumes stock.
    pub allocated_plies: u32,
    /// Table rows, one per contributing roll.
    pub allocations: Vec<RollAllocation>,
    /// Forward spread pieces in ply and physical consumption order.
    pub segments: Vec<SpreadSegment>,
    /// Joins between pieces of one ply.
    pub splices: Vec<SpliceInstruction>,
}

/// Whole-register material balance. Unopened stock is not called waste.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RollBalance {
    /// Stock identity.
    pub roll_id: String,
    /// Whether any segment consumes this roll.
    pub opened: bool,
    /// Total consumed length across all lays, in metres.
    pub used_m: f64,
    /// Unconsumed stock, in metres.
    pub remaining_m: f64,
}

/// Search disclosure: only a completed exhaustive whole-ply search proves optimality.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AllocationSearch {
    /// Exact branch-and-bound nodes visited.
    pub exact_nodes: usize,
    /// Complete exhaustive search proved minimum whole-ply end-bit waste.
    pub optimal_whole_ply: bool,
    /// Number of heuristic repacks, including the baseline and local swaps.
    pub heuristic_trials: usize,
}

/// JSON-ready result with explicit material and completeness accounting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpreadingPlan {
    /// Execution order; search may reorder the input lays to reduce end bits.
    pub lays: Vec<LaySpread>,
    /// Whether every requested ply has its entire marker length covered.
    pub complete: bool,
    /// Material balances for all rolls, including unopened stock.
    pub rolls: Vec<RollBalance>,
    /// Consumed cloth including splice overlaps, in metres.
    pub total_used_m: f64,
    /// Remaining stock on opened rolls; recoverable, not necessarily scrap.
    pub total_end_bits_m: f64,
    /// Sum of physical splice overlaps, in metres.
    pub splice_overlap_waste_m: f64,
    /// End-bit exposure plus physical splice overlaps, in linear metres.
    pub total_waste_m: f64,
    /// Measured length of rolls never opened, in metres.
    pub unopened_stock_m: f64,
    /// Roll transitions in output lay/ply/segment order (including lay boundaries).
    pub roll_changes: usize,
    /// Shortage, exclusion, mixing and defect-review messages.
    pub warnings: Vec<String>,
    /// Search effort and scope of any optimality claim.
    pub search: AllocationSearch,
}

#[derive(Clone)]
struct State {
    remaining: Vec<f64>,
    segments: Vec<Vec<SpreadSegment>>,
    allocated: Vec<u32>,
    order: Vec<usize>,
}

fn identity(r: &crate::Roll) -> ShadeIdentity {
    ShadeIdentity {
        lot: r.lot.clone(),
        dye_lot: r.dye_lot.clone(),
        shade_group: r.shade_group.clone(),
    }
}

fn eligible(lay: &LayRequest, r: &crate::Roll) -> bool {
    r.usable_width_mm + EPS >= lay.fabric_width_mm
        && (lay.eligible_roll_ids.is_empty() || lay.eligible_roll_ids.contains(&r.id))
        && match &lay.shade_rule {
            ShadePolicy::Required { identity: wanted } => *wanted == identity(r),
            _ => true,
        }
}

fn validate(register: &RollRegister, lays: &[LayRequest], s: &AllocationSettings) -> Result<()> {
    register.validate()?;
    nonnegative(s.splice_overlap_m, "splice overlap")?;
    ensure!(
        s.heuristic_trials > 0 && s.heuristic_trials <= 256,
        "heuristic trials must be between 1 and 256"
    );
    ensure!(
        s.exact_max_plies <= 64 && s.exact_max_rolls <= 64,
        "exact search dimensions must be at most 64"
    );
    ensure!(
        s.exact_node_limit <= 10_000_000
            && s.splice_node_limit > 0
            && s.splice_node_limit <= 1_000_000,
        "search node limits are out of range"
    );
    let mut ids = BTreeSet::new();
    let mut count = 0_u64;
    for l in lays {
        ensure!(
            !l.id.trim().is_empty() && ids.insert(&l.id),
            "lay ids must be nonempty and unique"
        );
        positive(l.marker_length_m, "marker length")?;
        positive(l.fabric_width_mm, "marker width")?;
        ensure!(l.plies > 0, "lay {} needs at least one ply", l.id);
        count += u64::from(l.plies);
        ensure!(count <= 100_000, "at most 100000 total plies per request");
        positive(l.marker_length_m * f64::from(l.plies), "lay fabric demand")?;
        for id in &l.eligible_roll_ids {
            ensure!(
                register.rolls.iter().any(|r| &r.id == id),
                "lay {} refers to unknown roll {id}",
                l.id
            );
        }
        for z in &l.splice_zones {
            nonnegative(z.start_m, "splice zone start")?;
            ensure!(
                z.end_m.is_finite() && z.start_m <= z.end_m && z.end_m < l.marker_length_m,
                "splice zone must lie before the marker end"
            );
        }
    }
    ensure!(
        register
            .rolls
            .iter()
            .map(|r| r.measured_length_m)
            .sum::<f64>()
            .is_finite(),
        "stock total overflow"
    );
    ensure!(
        lays.iter()
            .map(|l| l.marker_length_m * f64::from(l.plies))
            .sum::<f64>()
            .is_finite(),
        "demand total overflow"
    );
    Ok(())
}

fn initial(register: &RollRegister, lays: &[LayRequest]) -> State {
    State {
        remaining: register.rolls.iter().map(|r| r.measured_length_m).collect(),
        segments: vec![Vec::new(); lays.len()],
        allocated: vec![0; lays.len()],
        order: Vec::new(),
    }
}

fn append_piece(
    state: &mut State,
    register: &RollRegister,
    li: usize,
    ri: usize,
    ply: u32,
    start: f64,
    length: f64,
) {
    let r = &register.rolls[ri];
    if state.segments[li].is_empty() {
        state.order.push(li);
    }
    state.segments[li].push(SpreadSegment {
        roll_id: r.id.clone(),
        ply,
        roll_start_m: r.measured_length_m - state.remaining[ri],
        marker_start_m: start,
        length_m: length,
    });
    state.remaining[ri] = (state.remaining[ri] - length).max(0.0);
}

fn objective(state: &State, register: &RollRegister, lays: &[LayRequest]) -> (f64, f64, usize) {
    let unmet: f64 = lays
        .iter()
        .enumerate()
        .map(|(i, l)| f64::from(l.plies - state.allocated[i]) * l.marker_length_m)
        .sum();
    let mut end = 0.0;
    let mut consumed = 0.0;
    for (r, &rem) in register.rolls.iter().zip(&state.remaining) {
        if rem < r.measured_length_m - EPS {
            end += rem;
        }
        consumed += r.measured_length_m - rem;
    }
    let demand: f64 = lays
        .iter()
        .enumerate()
        .map(|(i, l)| f64::from(state.allocated[i]) * l.marker_length_m)
        .sum();
    let mut prev = None;
    let mut changes = 0;
    for &li in &state.order {
        let segments = &state.segments[li];
        for seg in segments {
            if prev.is_some_and(|p| p != &seg.roll_id) {
                changes += 1;
            }
            prev = Some(&seg.roll_id);
        }
    }
    (unmet, end + (consumed - demand).max(0.0), changes)
}

fn better(a: &State, b: &State, register: &RollRegister, lays: &[LayRequest]) -> bool {
    let a = objective(a, register, lays);
    let b = objective(b, register, lays);
    if (a.0 - b.0).abs() > EPS {
        a.0 < b.0
    } else if (a.1 - b.1).abs() > EPS {
        a.1 < b.1
    } else {
        a.2 < b.2
    }
}

fn hash(n: usize, seed: usize) -> u64 {
    let mut x = (n as u64).wrapping_add((seed as u64).wrapping_mul(0x9e3779b97f4a7c15));
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    x ^ (x >> 31)
}

fn candidates(
    state: &State,
    register: &RollRegister,
    lay: &LayRequest,
    shade: Option<&ShadeIdentity>,
    trial: usize,
    need: f64,
) -> Vec<usize> {
    let mut ids: Vec<_> = register
        .rolls
        .iter()
        .enumerate()
        .filter(|(i, r)| {
            eligible(lay, r) && state.remaining[*i] > EPS && shade.is_none_or(|s| *s == identity(r))
        })
        .map(|(i, _)| i)
        .collect();
    ids.sort_by(|&a, &b| {
        let rank = |i: usize| {
            let rem = state.remaining[i];
            let opened = rem < register.rolls[i].measured_length_m - EPS;
            match trial % 4 {
                0 => (0, i as f64), // descending-lay first-fit
                1 => (
                    i32::from(!opened),
                    if rem + EPS >= need {
                        rem - need
                    } else {
                        1e12 + rem
                    },
                ),
                2 => (i32::from(!opened), rem % need), // finish near a whole-ply boundary
                _ => (i32::from(!opened), hash(i, trial) as f64),
            }
        };
        let a = rank(a);
        let b = rank(b);
        a.0.cmp(&b.0).then(a.1.total_cmp(&b.1))
    });
    ids
}

/// Find one ply path. Splices may use any number of rolls, bounded by stock and nodes.
fn ply_path(
    state: &State,
    register: &RollRegister,
    lay: &LayRequest,
    shade: Option<&ShadeIdentity>,
    trial: usize,
    settings: &AllocationSettings,
) -> Option<PlyPath> {
    let ids = candidates(state, register, lay, shade, trial, lay.marker_length_m);
    // A two-roll repair can consume opened tails even when a whole roll is available.
    // New opened capacity is the marginal end-bit-plus-overlap objective.
    if trial % 4 == 2 && lay.allow_splicing {
        let opening = |i: usize| {
            if state.remaining[i] < register.rolls[i].measured_length_m - EPS {
                0.0
            } else {
                register.rolls[i].measured_length_m
            }
        };
        let mut best: Option<(f64, PlyPath)> = ids
            .iter()
            .filter(|&&i| state.remaining[i] + EPS >= lay.marker_length_m)
            .map(|&i| (opening(i), vec![(i, 0.0, lay.marker_length_m)]))
            .min_by(|a, b| a.0.total_cmp(&b.0));
        for &a in &ids {
            for &b in &ids {
                if a == b {
                    continue;
                }
                let cost = opening(a) + opening(b);
                if best.as_ref().is_some_and(|v| cost >= v.0 - EPS) {
                    continue;
                }
                for z in &lay.splice_zones {
                    let low = z
                        .start_m
                        .max(lay.marker_length_m - state.remaining[b])
                        .max(EPS * 2.0);
                    let high = (z.end_m - settings.splice_overlap_m)
                        .min(state.remaining[a] - settings.splice_overlap_m);
                    if low <= high + EPS && high > EPS {
                        let cut = high;
                        best = Some((
                            cost,
                            vec![
                                (a, 0.0, cut + settings.splice_overlap_m),
                                (b, cut, lay.marker_length_m - cut),
                            ],
                        ));
                    }
                }
            }
        }
        if let Some((_, path)) = best {
            return Some(path);
        }
    }
    struct PathSearch<'a> {
        state: &'a State,
        lay: &'a LayRequest,
        ids: Vec<usize>,
        settings: &'a AllocationSettings,
        nodes: usize,
    }
    impl PathSearch<'_> {
        fn visit(&mut self, start: f64, path: &mut PlyPath) -> bool {
            self.nodes += 1;
            if self.nodes > self.settings.splice_node_limit {
                return false;
            }
            let need = self.lay.marker_length_m - start;
            for &ri in &self.ids {
                if !path.iter().any(|(i, _, _)| *i == ri) && self.state.remaining[ri] + EPS >= need
                {
                    path.push((ri, start, need));
                    return true;
                }
            }
            if !self.lay.allow_splicing || path.len() >= 63 {
                return false;
            }
            for index in 0..self.ids.len() {
                let ri = self.ids[index];
                if path.iter().any(|(i, _, _)| *i == ri) {
                    continue;
                }
                let max_cut = start + self.state.remaining[ri] - self.settings.splice_overlap_m;
                let mut positions = Vec::new();
                for z in &self.lay.splice_zones {
                    let last = max_cut.min(z.end_m - self.settings.splice_overlap_m);
                    if last + EPS >= z.start_m && last > start + EPS {
                        positions.push(last);
                    }
                    if z.start_m > start + EPS && z.start_m <= last + EPS {
                        positions.push(z.start_m);
                    }
                }
                positions.sort_by(|a, b| b.total_cmp(a));
                positions.dedup_by(|a, b| (*a - *b).abs() < EPS);
                for cut in positions {
                    path.push((ri, start, cut + self.settings.splice_overlap_m - start));
                    if self.visit(cut, path) {
                        return true;
                    }
                    path.pop();
                }
            }
            false
        }
    }
    let mut search = PathSearch {
        state,
        lay,
        ids,
        settings,
        nodes: 0,
    };
    let mut path = Vec::new();
    search.visit(0.0, &mut path).then_some(path)
}

fn fill_lay(
    state: &State,
    register: &RollRegister,
    lays: &[LayRequest],
    li: usize,
    shade: Option<&ShadeIdentity>,
    trial: usize,
    settings: &AllocationSettings,
) -> State {
    let mut state = state.clone();
    let lay = &lays[li];
    for ply in 1..=lay.plies {
        let Some(path) = ply_path(&state, register, lay, shade, trial, settings) else {
            break;
        };
        for (ri, start, length) in path {
            append_piece(&mut state, register, li, ri, ply, start, length);
        }
        state.allocated[li] += 1;
    }
    state
}

fn greedy(
    register: &RollRegister,
    lays: &[LayRequest],
    order: &[usize],
    trial: usize,
    settings: &AllocationSettings,
    first_come: bool,
) -> State {
    let mut state = initial(register, lays);
    for &li in order {
        let lay = &lays[li];
        if lay.shade_rule == ShadePolicy::AllowMixing {
            state = fill_lay(&state, register, lays, li, None, trial, settings);
        } else {
            let mut shades = Vec::new();
            for r in &register.rolls {
                if eligible(lay, r) && !shades.contains(&identity(r)) {
                    shades.push(identity(r));
                }
            }
            let mut best: Option<State> = None;
            for shade in shades {
                let next = fill_lay(&state, register, lays, li, Some(&shade), trial, settings);
                if first_come && next.allocated[li] == lay.plies {
                    best = Some(next);
                    break;
                }
                if best
                    .as_ref()
                    .is_none_or(|b| better(&next, b, register, lays))
                {
                    best = Some(next);
                }
            }
            if let Some(best) = best {
                state = best;
            }
        }
    }
    state
}

struct ExactSearch<'a> {
    register: &'a RollRegister,
    lays: &'a [LayRequest],
    tasks: Vec<usize>,
    limit: usize,
    nodes: usize,
    exhausted: bool,
    best: State,
}

impl ExactSearch<'_> {
    fn visit(&mut self, depth: usize, state: &mut State, shades: &mut [Option<ShadeIdentity>]) {
        if self.nodes >= self.limit {
            self.exhausted = false;
            return;
        }
        self.nodes += 1;
        if depth == self.tasks.len() {
            if better(state, &self.best, self.register, self.lays) {
                self.best = state.clone();
            }
            return;
        }
        // Once opened, a roll's entire length enters the final end-bit objective.
        let demand: f64 = self
            .lays
            .iter()
            .map(|l| l.marker_length_m * f64::from(l.plies))
            .sum();
        let opened: f64 = self
            .register
            .rolls
            .iter()
            .enumerate()
            .filter(|(i, r)| state.remaining[*i] < r.measured_length_m - EPS)
            .map(|(_, r)| r.measured_length_m)
            .sum();
        let incumbent = objective(&self.best, self.register, self.lays);
        if incumbent.0 < EPS && opened - demand > incumbent.1 + EPS {
            return;
        }
        let li = self.tasks[depth];
        let lay = &self.lays[li];
        let ids = candidates(
            state,
            self.register,
            lay,
            shades[li].as_ref(),
            1,
            lay.marker_length_m,
        );
        for ri in ids {
            if state.remaining[ri] + EPS < lay.marker_length_m {
                continue;
            }
            let old_shade = shades[li].clone();
            if lay.shade_rule != ShadePolicy::AllowMixing {
                shades[li] = Some(identity(&self.register.rolls[ri]));
            }
            let old_remaining = state.remaining[ri];
            let ply = state.allocated[li] + 1;
            append_piece(state, self.register, li, ri, ply, 0.0, lay.marker_length_m);
            state.allocated[li] += 1;
            self.visit(depth + 1, state, shades);
            state.allocated[li] -= 1;
            state.segments[li].pop();
            if state.segments[li].is_empty() {
                state.order.pop();
            }
            state.remaining[ri] = old_remaining;
            shades[li] = old_shade;
            if !self.exhausted {
                return;
            }
        }
    }
}

fn finish(
    state: State,
    register: &RollRegister,
    lays: &[LayRequest],
    s: &AllocationSettings,
    search: AllocationSearch,
) -> SpreadingPlan {
    let obj = objective(&state, register, lays);
    let mut warnings = Vec::new();
    let mut output = Vec::new();
    let mut order = state.order.clone();
    for li in 0..lays.len() {
        if !order.contains(&li) {
            order.push(li);
        }
    }
    for (oi, &li) in order.iter().enumerate() {
        let lay = &lays[li];
        if state.allocated[li] < lay.plies {
            warnings.push(format!("Lay {}: allocated {} of {} plies; check compatible stock, shade, width and splice zones (bounded search may miss a solution).", lay.id, state.allocated[li], lay.plies));
        }
        let narrow: Vec<_> = register
            .rolls
            .iter()
            .filter(|r| r.usable_width_mm + EPS < lay.fabric_width_mm)
            .map(|r| r.id.as_str())
            .collect();
        if !narrow.is_empty() {
            warnings.push(format!(
                "Lay {}: rolls too narrow and excluded: {}.",
                lay.id,
                narrow.join(", ")
            ));
        }
        let segments = state.segments[li].clone();
        let needs_defect_review = segments.iter().any(|seg| {
            register.rolls.iter().any(|r| {
                r.id == seg.roll_id
                    && r.defects.iter().any(|d| {
                        d.action != crate::DefectAction::Ignore
                            && d.along_m < seg.roll_start_m + seg.length_m
                            && d.along_m + d.length_mm / 1000.0 > seg.roll_start_m
                            && d.across_mm < lay.fabric_width_mm
                    })
            })
        });
        if needs_defect_review {
            warnings.push(format!("Lay {}: the allocated cloth contains defects. Run map_defects, revise the marker or spread as needed, and recheck lengths before production.", lay.id));
        }
        let mut splices = Vec::new();
        for pair in segments.windows(2) {
            if pair[0].ply == pair[1].ply {
                splices.push(SpliceInstruction {
                    ply: pair[1].ply,
                    from_roll: pair[0].roll_id.clone(),
                    to_roll: pair[1].roll_id.clone(),
                    marker_position_m: pair[1].marker_start_m,
                    overlap_m: s.splice_overlap_m,
                });
            }
        }
        let mut allocations = Vec::new();
        let mut shades = BTreeSet::new();
        for (ri, r) in register.rolls.iter().enumerate() {
            let selected: Vec<_> = segments.iter().filter(|p| p.roll_id == r.id).collect();
            if selected.is_empty() {
                continue;
            }
            shades.insert(identity(r));
            let last_use = !order
                .iter()
                .skip(oi + 1)
                .any(|&next| state.segments[next].iter().any(|seg| seg.roll_id == r.id));
            allocations.push(RollAllocation {
                roll_id: r.id.clone(),
                plies: selected
                    .iter()
                    .map(|p| p.ply)
                    .collect::<BTreeSet<_>>()
                    .len() as u32,
                full_plies: selected
                    .iter()
                    .filter(|p| {
                        p.marker_start_m.abs() < EPS
                            && (p.length_m - lay.marker_length_m).abs() < EPS
                    })
                    .count() as u32,
                used_m: selected.iter().map(|p| p.length_m).sum(),
                end_bit_m: if last_use { state.remaining[ri] } else { 0.0 },
                shade_group: r.shade_group.clone(),
            });
        }
        if shades.len() > 1 {
            warnings.push(format!(
                "Lay {}: shade mixing is enabled and this lay uses {} shade identities.",
                lay.id,
                shades.len()
            ));
        }
        output.push(LaySpread {
            lay: lay.clone(),
            allocated_plies: state.allocated[li],
            allocations,
            segments,
            splices,
        });
    }
    let rolls: Vec<_> = register
        .rolls
        .iter()
        .zip(state.remaining)
        .map(|(r, rem)| RollBalance {
            roll_id: r.id.clone(),
            opened: rem < r.measured_length_m - EPS,
            used_m: r.measured_length_m - rem,
            remaining_m: rem,
        })
        .collect();
    let total_used_m: f64 = rolls.iter().map(|r| r.used_m).sum();
    let total_end_bits_m: f64 = rolls
        .iter()
        .filter(|r| r.opened)
        .map(|r| r.remaining_m)
        .sum();
    let unopened_stock_m = rolls
        .iter()
        .filter(|r| !r.opened)
        .map(|r| r.remaining_m)
        .sum();
    let splice_overlap_waste_m = output
        .iter()
        .flat_map(|l| &l.splices)
        .map(|s| s.overlap_m)
        .sum::<f64>()
        .max(0.0);
    SpreadingPlan {
        complete: output.iter().all(|l| l.allocated_plies == l.lay.plies),
        lays: output,
        rolls,
        total_used_m,
        total_end_bits_m,
        splice_overlap_waste_m,
        total_waste_m: total_end_bits_m + splice_overlap_waste_m,
        unopened_stock_m,
        roll_changes: obj.2,
        warnings,
        search,
    }
}

/// Input-order, first-compatible-roll baseline, with the same physical constraints.
pub fn allocate_first_come(
    register: &RollRegister,
    lays: &[LayRequest],
    settings: &AllocationSettings,
) -> Result<SpreadingPlan> {
    validate(register, lays, settings)?;
    let order: Vec<_> = (0..lays.len()).collect();
    let state = greedy(register, lays, &order, 0, settings, true);
    let plan = finish(
        state,
        register,
        lays,
        settings,
        AllocationSearch {
            exact_nodes: 0,
            optimal_whole_ply: false,
            heuristic_trials: 1,
        },
    );
    crate::validate_plan(register, &plan)?;
    Ok(plan)
}

/// Minimise unfilled metres, then end bits plus overlaps, then roll transitions.
/// A partial result is explicit: callers must check `complete` before spreading.
pub fn allocate(
    register: &RollRegister,
    lays: &[LayRequest],
    settings: &AllocationSettings,
) -> Result<SpreadingPlan> {
    validate(register, lays, settings)?;
    let mut order: Vec<_> = (0..lays.len()).collect();
    let mut best = greedy(register, lays, &order, 0, settings, true);
    for trial in 0..settings.heuristic_trials {
        order.sort_by(|&a, &b| match trial / 4 {
            0 => lays[b].marker_length_m.total_cmp(&lays[a].marker_length_m),
            1 => lays[b]
                .fabric_width_mm
                .total_cmp(&lays[a].fabric_width_mm)
                .then(lays[b].marker_length_m.total_cmp(&lays[a].marker_length_m)),
            2 => a.cmp(&b),
            _ => hash(a, trial / 4).cmp(&hash(b, trial / 4)),
        });
        let next = greedy(register, lays, &order, trial, settings, false);
        if better(&next, &best, register, lays) {
            best = next;
        }
    }
    // Local improvement: re-pack after each adjacent lay swap in descending order.
    order.sort_by(|&a, &b| lays[b].marker_length_m.total_cmp(&lays[a].marker_length_m));
    for i in 1..order.len().min(33) {
        order.swap(i - 1, i);
        let next = greedy(register, lays, &order, 2, settings, false);
        if better(&next, &best, register, lays) {
            best = next;
        }
        order.swap(i - 1, i);
    }
    let mut report = AllocationSearch {
        exact_nodes: 0,
        optimal_whole_ply: false,
        heuristic_trials: 1 + settings.heuristic_trials + order.len().min(33).saturating_sub(1),
    };
    let count: usize = lays.iter().map(|l| l.plies as usize).sum();
    if count <= settings.exact_max_plies
        && register.rolls.len() <= settings.exact_max_rolls
        && !lays
            .iter()
            .any(|l| l.allow_splicing && !l.splice_zones.is_empty())
        && settings.exact_node_limit > 0
    {
        let mut tasks = Vec::new();
        for (li, l) in lays.iter().enumerate() {
            tasks.extend(std::iter::repeat_n(li, l.plies as usize));
        }
        tasks.sort_by(|&a, &b| lays[b].marker_length_m.total_cmp(&lays[a].marker_length_m));
        let mut search = ExactSearch {
            register,
            lays,
            tasks,
            limit: settings.exact_node_limit,
            nodes: 0,
            exhausted: true,
            best,
        };
        search.visit(0, &mut initial(register, lays), &mut vec![None; lays.len()]);
        report.exact_nodes = search.nodes;
        report.optimal_whole_ply = search.exhausted
            && search
                .best
                .allocated
                .iter()
                .zip(lays)
                .all(|(n, l)| *n == l.plies);
        best = search.best;
    }
    let plan = finish(best, register, lays, settings, report);
    crate::validate_plan(register, &plan)?;
    Ok(plan)
}

/// A like-for-like comparison. Savings are absent if either plan is incomplete.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AllocationComparison {
    /// Receiving-order baseline under the same physical restrictions.
    pub first_come: SpreadingPlan,
    /// Best portfolio/exact result, including the baseline as a candidate.
    pub optimized: SpreadingPlan,
    /// Baseline minus optimized waste exposure, only for two complete plans.
    pub waste_saved_m: Option<f64>,
    /// Absent if the first-come plan has zero waste exposure.
    pub waste_saved_pct: Option<f64>,
}

/// Compare the heuristic/exact portfolio with the input-order baseline.
pub fn compare_allocations(
    register: &RollRegister,
    lays: &[LayRequest],
    settings: &AllocationSettings,
) -> Result<AllocationComparison> {
    let first_come = allocate_first_come(register, lays, settings)?;
    let optimized = allocate(register, lays, settings)?;
    let waste_saved_m = (first_come.complete && optimized.complete)
        .then_some(first_come.total_waste_m - optimized.total_waste_m);
    let waste_saved_pct = waste_saved_m
        .filter(|_| first_come.total_waste_m > EPS)
        .map(|v| v / first_come.total_waste_m * 100.0);
    Ok(AllocationComparison {
        first_come,
        optimized,
        waste_saved_m,
        waste_saved_pct,
    })
}
