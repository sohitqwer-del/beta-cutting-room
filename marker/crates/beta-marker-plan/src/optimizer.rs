//! Bounded branch-and-bound over integer ratio/ply choices. No external solver.
use crate::{Candidate, PlanOptions};
use std::collections::HashMap;

pub(crate) struct Solution {
    pub actions: Vec<(usize, u32)>,
    pub exhausted: bool,
}

struct Search<'a> {
    candidates: &'a [Candidate],
    required: &'a [u32],
    upper: &'a [u32],
    max_passes: u32,
    opts: &'a PlanOptions,
    best: Vec<(usize, u32)>,
    best_cost: f64,
    partial: Vec<(usize, u32)>,
    partial_covered: u64,
    partial_cost: f64,
    seen: HashMap<Vec<u32>, f64>,
    remaining: usize,
    exhausted: bool,
    unit_bound: f64,
    weights: Vec<f64>,
    capacity: Vec<u64>,
    total_capacity: u64,
}

impl Search<'_> {
    // A feasible dual price per size bounds fabric use. Capacity independently
    // bounds the number of lays. Neither bound assumes exact quantity equality.
    fn bound(&self, made: &[u32], action: Option<(usize, u32)>) -> f64 {
        let mut missing = 0u64;
        let mut weighted = 0.0;
        let mut lays = 0u64;
        for (i, (&n, &q)) in made.iter().zip(self.required).enumerate() {
            let n = n + action.map_or(0, |(c, p)| self.candidates[c].ratio[i] * p);
            let need = u64::from(q.saturating_sub(n));
            if need > 0 && self.capacity[i] == 0 {
                return f64::INFINITY;
            }
            missing += need;
            weighted += need as f64 * self.weights[i];
            if need > 0 {
                lays = lays.max(need.div_ceil(self.capacity[i]));
            }
        }
        if missing == 0 {
            return 0.0;
        }
        if self.total_capacity == 0 {
            return f64::INFINITY;
        }
        lays = lays.max(missing.div_ceil(self.total_capacity));
        weighted.max(missing as f64 * self.unit_bound) + lays as f64 * self.opts.lay_cost_m
    }

    fn cost(&self, i: usize, p: u32) -> f64 {
        (self.candidates[i].length + 2.0 * self.opts.end_allowance_mm) * f64::from(p) / 1000.0
            + self.opts.lay_cost_m
    }

    fn max_p(&self, i: usize, made: &[u32]) -> u32 {
        self.candidates[i]
            .ratio
            .iter()
            .zip(made)
            .zip(self.upper)
            .filter(|((r, _), _)| **r > 0)
            .map(|((r, n), upper)| (upper - n) / r)
            .min()
            .unwrap_or(0)
            .min(self.max_passes)
    }

    fn add(&self, made: &[u32], i: usize, p: u32) -> Vec<u32> {
        made.iter()
            .zip(&self.candidates[i].ratio)
            .map(|(n, r)| n + r * p)
            .collect()
    }

    fn covered(&self, made: &[u32]) -> u64 {
        made.iter()
            .zip(self.required)
            .map(|(n, q)| u64::from((*n).min(*q)))
            .sum()
    }

    fn record(&mut self, made: &[u32], cost: f64, path: &[(usize, u32)]) -> bool {
        let covered = self.covered(made);
        if covered > self.partial_covered
            || (covered == self.partial_covered && cost < self.partial_cost)
        {
            self.partial_covered = covered;
            self.partial_cost = cost;
            self.partial = path.to_vec();
        }
        let complete = made.iter().zip(self.required).all(|(n, q)| n >= q);
        if complete && cost < self.best_cost {
            self.best_cost = cost;
            self.best = path.to_vec();
        }
        complete
    }

    fn greedy(&mut self, first: Option<usize>) {
        let mut made = vec![0; self.required.len()];
        let mut path = Vec::new();
        let mut cost = 0.0;
        for step in 0..10_000 {
            if self.record(&made, cost, &path) {
                break;
            }
            let covered = self.covered(&made);
            let mut best = None;
            for i in 0..self.candidates.len() {
                if step == 0 && first.is_some_and(|v| v != i) {
                    continue;
                }
                if !self.candidates[i].length.is_finite() {
                    continue;
                }
                let max_p = self.max_p(i, &made);
                // Stop at the first quantity target reached; later steps repair the rest.
                let p = self.candidates[i]
                    .ratio
                    .iter()
                    .zip(&made)
                    .zip(self.required)
                    .filter(|((r, n), q)| **r > 0 && n < q)
                    .map(|((r, n), q)| (q - n).div_ceil(*r))
                    .min()
                    .unwrap_or(0)
                    .min(max_p);
                if p == 0 {
                    continue;
                }
                let next = self.add(&made, i, p);
                let gain = self.covered(&next) - covered;
                if gain == 0 {
                    continue;
                }
                let score = self.cost(i, p) / gain as f64;
                if best.as_ref().is_none_or(|(_, _, _, s)| score < *s) {
                    best = Some((i, p, next, score));
                }
            }
            let Some((i, p, next, _)) = best else {
                break;
            };
            cost += self.cost(i, p);
            path.push((i, p));
            made = next;
        }
        self.record(&made, cost, &path);
    }

    fn repeated_ratio(&mut self, i: usize) {
        let c = &self.candidates[i];
        if !c.length.is_finite() {
            return;
        }
        let mut plies = 0;
        for (&q, &r) in self.required.iter().zip(&c.ratio) {
            if q > 0 && r == 0 {
                return;
            }
            if r > 0 {
                plies = plies.max(q.div_ceil(r));
            }
        }
        if c.ratio
            .iter()
            .zip(self.upper)
            .any(|(r, u)| u64::from(*r) * u64::from(plies) > u64::from(*u))
        {
            return;
        }
        let mut made = vec![0; self.required.len()];
        let mut cost = 0.0;
        let mut path = Vec::new();
        while plies > 0 && path.len() < 10_000 {
            let p = plies.min(self.max_passes);
            if p == 0 {
                return;
            }
            made = self.add(&made, i, p);
            cost += self.cost(i, p);
            path.push((i, p));
            plies -= p;
        }
        self.record(&made, cost, &path);
    }

    fn visit(&mut self, made: Vec<u32>, cost: f64, path: &mut Vec<(usize, u32)>) {
        if self.remaining == 0 {
            self.exhausted = false;
            return;
        }
        self.remaining -= 1;
        if self.record(&made, cost, path) {
            return;
        }
        if path.len() >= 256 {
            self.exhausted = false;
            return;
        }
        if cost + self.bound(&made, None) >= self.best_cost - 1e-10 {
            return;
        }
        if self.seen.get(&made).is_some_and(|v| *v <= cost) {
            return;
        }
        self.seen.insert(made.clone(), cost);
        // Any complete continuation must include a marker for this unmet size.
        // Lays commute, so exploring it first loses no feasible combination.
        let pivot = made
            .iter()
            .zip(self.required)
            .enumerate()
            .filter(|(_, (n, q))| n < q)
            .max_by_key(|(_, (n, q))| *q - *n)
            .map(|(i, _)| i);
        let Some(pivot) = pivot else {
            return;
        };
        let covered = self.covered(&made);
        let mut moves = Vec::new();
        for (i, c) in self.candidates.iter().enumerate() {
            if c.ratio[pivot] == 0 || !c.length.is_finite() {
                continue;
            }
            // Additional plies after satisfying every size in this ratio cannot help.
            let useful = c
                .ratio
                .iter()
                .zip(&made)
                .zip(self.required)
                .filter(|((r, _), _)| **r > 0)
                .map(|((r, n), q)| q.saturating_sub(*n).div_ceil(*r))
                .max()
                .unwrap_or(0);
            let max_p = self.max_p(i, &made).min(useful);
            let mut ply_choices: Vec<u32> = if max_p <= 4096 {
                (1..=max_p).collect()
            } else {
                // Avoid allocating billions of moves for an unrealistic height limit.
                // Keep target breakpoints and report that this search is approximate.
                self.exhausted = false;
                let mut choices = vec![1, max_p];
                for ((r, n), q) in c.ratio.iter().zip(&made).zip(self.required) {
                    if *r == 0 {
                        continue;
                    }
                    let target = q.saturating_sub(*n).div_ceil(*r);
                    choices.extend([target.saturating_sub(1), target, target.saturating_add(1)]);
                }
                choices.retain(|p| *p > 0 && *p <= max_p);
                choices
            };
            ply_choices.sort_unstable();
            ply_choices.dedup();
            for p in ply_choices {
                let next_cost = self.cost(i, p);
                if cost + next_cost + self.bound(&made, Some((i, p))) >= self.best_cost - 1e-10 {
                    continue;
                }
                let next_covered: u64 = made
                    .iter()
                    .zip(&c.ratio)
                    .zip(self.required)
                    .map(|((n, r), q)| u64::from((n + r * p).min(*q)))
                    .sum();
                let gain = next_covered - covered;
                moves.push((i, p, next_cost / gain as f64));
            }
        }
        moves.sort_by(|a, b| {
            a.2.total_cmp(&b.2)
                .then_with(|| b.1.cmp(&a.1))
                .then_with(|| a.0.cmp(&b.0))
        });
        for (i, p, _) in moves {
            if self.remaining == 0 {
                self.exhausted = false;
                break;
            }
            let next_cost = cost + self.cost(i, p);
            if next_cost >= self.best_cost {
                continue;
            }
            path.push((i, p));
            self.visit(self.add(&made, i, p), next_cost, path);
            path.pop();
        }
    }
}

pub(crate) fn solve(
    candidates: &[Candidate],
    required: &[u32],
    upper: &[u32],
    max_passes: u32,
    opts: &PlanOptions,
) -> Solution {
    let unit_bound = candidates
        .iter()
        .filter(|c| c.length.is_finite())
        .map(|c| {
            (c.length + 2.0 * opts.end_allowance_mm)
                / 1000.0
                / c.ratio.iter().map(|r| f64::from(*r)).sum::<f64>()
        })
        .fold(f64::INFINITY, f64::min);
    let measured: Vec<_> = candidates.iter().filter(|c| c.length.is_finite()).collect();
    let mut weights: Vec<_> = (0..required.len())
        .map(|i| {
            measured
                .iter()
                .filter(|c| {
                    c.ratio[i] > 0 && c.ratio.iter().enumerate().all(|(j, r)| j == i || *r == 0)
                })
                .map(|c| (c.length + 2.0 * opts.end_allowance_mm) / 1000.0 / f64::from(c.ratio[i]))
                .reduce(f64::min)
                .unwrap_or(if unit_bound.is_finite() {
                    unit_bound
                } else {
                    0.0
                })
        })
        .collect();
    // Scale all prices until every marker costs at least the garments' prices.
    // This is a feasible dual solution even when mixed markers pack much better.
    let scale = measured
        .iter()
        .map(|c| {
            let value: f64 = c
                .ratio
                .iter()
                .zip(&weights)
                .map(|(r, w)| f64::from(*r) * w)
                .sum();
            if value > 0.0 {
                ((c.length + 2.0 * opts.end_allowance_mm) / 1000.0) / value
            } else {
                1.0
            }
        })
        .fold(1.0, f64::min);
    weights.iter_mut().for_each(|w| *w *= scale);
    let capacity: Vec<_> = (0..required.len())
        .map(|i| {
            measured
                .iter()
                .map(|c| u64::from(c.ratio[i]) * u64::from(max_passes))
                .max()
                .unwrap_or(0)
        })
        .collect();
    let total_capacity = measured
        .iter()
        .map(|c| c.ratio.iter().map(|r| u64::from(*r)).sum::<u64>() * u64::from(max_passes))
        .max()
        .unwrap_or(0);
    let mut search = Search {
        candidates,
        required,
        upper,
        max_passes,
        opts,
        best: Vec::new(),
        best_cost: f64::INFINITY,
        partial: Vec::new(),
        partial_covered: 0,
        partial_cost: f64::INFINITY,
        seen: HashMap::new(),
        remaining: opts.search_nodes,
        exhausted: true,
        unit_bound,
        weights,
        capacity,
        total_capacity,
    };
    search.greedy(None);
    for i in 0..candidates.len() {
        search.repeated_ratio(i);
        search.greedy(Some(i));
    }
    search.visit(vec![0; required.len()], 0.0, &mut Vec::new());
    Solution {
        actions: if search.best_cost.is_finite() {
            search.best
        } else {
            search.partial
        },
        exhausted: search.exhausted,
    }
}
