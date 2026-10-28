//! Bounded local contact search. Outside and crossing pieces never move.
use super::*;

fn enclosed(b: [f64; 4], region: [f64; 4]) -> bool {
    b[0] >= region[0] - EPS
        && b[1] >= region[1] - EPS
        && b[2] <= region[2] + EPS
        && b[3] <= region[3] + EPS
}
fn score(job: &Job, result: &MarkerResult, selected: &[usize]) -> Result<(f64, f64), String> {
    let sh = shapes(job, result)?;
    Ok((
        sh.iter().map(|s| s.bounds[2]).fold(0., f64::max),
        selected.iter().map(|&i| sh[i].bounds[2]).fold(0., f64::max),
    ))
}
fn better(a: (f64, f64), b: (f64, f64)) -> bool {
    a.0 < b.0 - EPS || ((a.0 - b.0).abs() <= EPS && a.1 < b.1 - EPS)
}

pub(super) fn improve(
    original: &Job,
    job: &Job,
    result: &mut MarkerResult,
    settings: &Settings,
    from: Pt,
    to: Pt,
) -> Result<(Vec<usize>, String), String> {
    if from.iter().chain(to.iter()).any(|v| !v.is_finite()) {
        return Err("Region corners must be finite.".into());
    }
    let region = [
        from[0].min(to[0]),
        from[1].min(to[1]),
        from[0].max(to[0]),
        from[1].max(to[1]),
    ];
    if region[2] - region[0] < EPS || region[3] - region[1] < EPS {
        return Err("Draw a region with positive length and width.".into());
    }
    if !beta_marker_core::validate(original, result).ok || !geometry_ok(job, result, settings)? {
        return Err("Start from a complete valid marker before improving an area.".into());
    }
    let sh = shapes(job, result)?;
    let selected: Vec<_> = sh
        .iter()
        .enumerate()
        .filter(|(i, s)| !result.placements[*i].locked && enclosed(s.bounds, region))
        .map(|(i, _)| i)
        .collect();
    if selected.is_empty() {
        return Err(
            "No unlocked pieces are fully inside this region. Crossing pieces stay fixed.".into(),
        );
    }
    if selected.len() > 32 || result.placements.len() > 1000 {
        return Err("Improve area supports up to 32 enclosed movable pieces in a marker of at most 1000 placements. Choose a smaller region.".into());
    }
    let original_result = result.clone();
    let before = score(job, result, &selected)?;
    let mut best_score = before;
    let patterns = [
        [Direction::Left, Direction::Left, Direction::Left],
        [Direction::Left, Direction::Up, Direction::Left],
        [Direction::Left, Direction::Down, Direction::Left],
        [Direction::Up, Direction::Left, Direction::Left],
        [Direction::Down, Direction::Left, Direction::Left],
    ];
    for (pass, pattern) in patterns.iter().enumerate() {
        let mut trial = result.clone();
        let mut order = selected.clone();
        if pass % 2 != 0 {
            order.reverse();
        }
        for &direction in pattern {
            for &i in &order {
                let before_move = trial.placements[i].translate;
                let b = shapes(job, &trial)?[i].bounds;
                let limit = match direction {
                    Direction::Left => b[0] - region[0],
                    Direction::Right => region[2] - b[2],
                    Direction::Down => b[1] - region[1],
                    Direction::Up => region[3] - b[3],
                }
                .max(0.);
                let distance = bump(job, &mut trial, &[i], direction, settings, false)?;
                if distance > limit {
                    let v = direction.vector();
                    trial.placements[i].translate =
                        [before_move[0] + v[0] * limit, before_move[1] + v[1] * limit];
                }
            }
        }
        metrics(job, &mut trial);
        let candidate_score = score(job, &trial, &selected)?;
        let trial_shapes = shapes(job, &trial)?;
        if better(candidate_score, best_score)
            && selected
                .iter()
                .all(|&i| enclosed(trial_shapes[i].bounds, region))
            && geometry_ok(job, &trial, settings)?
            && beta_marker_core::validate(original, &trial).ok
        {
            *result = trial;
            best_score = candidate_score;
        }
    }
    for (i, placement) in original_result.placements.iter().enumerate() {
        if !selected.contains(&i) && result.placements[i] != *placement {
            return Err("Region search changed an outside placement.".into());
        }
    }
    Ok((
        selected,
        format!(
            "Area preview: marker end reduced by {:.3} mm; selected area end reduced by {:.3} mm. Outside, crossing and locked pieces stayed fixed.",
            before.0 - best_score.0,
            before.1 - best_score.1
        ),
    ))
}
