//! Offline HPGL geometry recovery. Plotter coordinates remain in marker space.

use crate::{curves, pattern};
use anyhow::{Context, Result, bail, ensure};
use beta_marker_core::{
    Fabric, JOB_FORMAT, Job, Mark, MarkKind, MarkerResult, Piece, Placement, Pt, RESULT_FORMAT,
    RotationRule, compute_metrics,
    geometry::{area, bbox, transform_points},
};

/// Controls for recovering a plotted marker. All distances except `plu_per_mm` are mm.
#[derive(Debug, Clone)]
pub struct HpglOptions {
    /// Plotter units per millimetre (normally 40).
    pub plu_per_mm: f64,
    /// Known fabric width. Otherwise use an enclosing rectangle or the largest cut-line y.
    pub width: Option<f64>,
    /// Endpoint joining and duplicate-shape tolerance.
    pub tolerance_mm: f64,
    /// Name assigned to both outputs.
    pub name: String,
}

impl Default for HpglOptions {
    fn default() -> Self {
        Self {
            plu_per_mm: 40.0,
            width: None,
            tolerance_mm: 0.05,
            name: "Imported marker".into(),
        }
    }
}

/// A reusable job and the original plotted placements, measured with BETA's formula.
#[derive(Debug)]
pub struct HpglReport {
    /// Patterns with translations removed and counts inferred from the plot.
    pub job: Job,
    /// Original placement; no nesting is performed by the importer.
    pub result: MarkerResult,
    /// Geometry and garment-rule assumptions to review before cutting.
    pub warnings: Vec<String>,
}

#[derive(Clone)]
struct Path {
    points: Vec<Pt>,
    circle: Option<Pt>,
    pen: u32,
}

struct Plot {
    paths: Vec<Path>,
    labels: Vec<(Pt, String)>,
    current: Pt,
    known_position: bool,
    active: Vec<Pt>,
    down: bool,
    absolute: bool,
    pen: u32,
    delimiter: u8,
    warnings: Vec<String>,
    points: usize,
}

impl Default for Plot {
    fn default() -> Self {
        Self {
            paths: vec![],
            labels: vec![],
            current: [0.0, 0.0],
            known_position: true,
            active: vec![],
            down: false,
            absolute: true,
            pen: 1,
            delimiter: 3,
            warnings: vec![],
            points: 0,
        }
    }
}

impl Plot {
    fn flush(&mut self) {
        if self.active.len() > 1 {
            self.paths.push(Path {
                points: std::mem::take(&mut self.active),
                circle: None,
                pen: self.pen,
            });
        }
        self.active.clear();
    }
    fn move_to(&mut self, p: Pt) -> Result<()> {
        ensure!(
            p.iter().all(|n| n.is_finite() && n.abs() <= 1e10),
            "HPGL coordinates out of range"
        );
        if self.down && self.pen != 0 {
            if self.active.is_empty() {
                self.active.push(self.current);
            }
            self.active.push(p);
            self.points += 1;
            ensure!(
                self.points <= 2_000_000,
                "HPGL exceeds two million drawn points"
            );
        }
        self.current = p;
        Ok(())
    }
    fn coordinates(&mut self, n: &[f64]) -> Result<()> {
        ensure!(
            n.len().is_multiple_of(2),
            "HPGL coordinate list needs x,y pairs"
        );
        if !n.is_empty() {
            ensure!(
                self.absolute || self.known_position,
                "relative HPGL movement after LB needs an absolute reposition; label advance is font-dependent"
            );
            if self.absolute && !self.known_position {
                ensure!(
                    !self.down,
                    "drawing from the end of LB needs an absolute pen-up reposition"
                );
            }
            self.known_position = true;
        }
        for xy in n.as_chunks::<2>().0 {
            self.move_to(if self.absolute {
                [xy[0], xy[1]]
            } else {
                [self.current[0] + xy[0], self.current[1] + xy[1]]
            })?;
        }
        Ok(())
    }
}

fn numbers(bytes: &[u8]) -> Result<Vec<f64>> {
    let s = std::str::from_utf8(bytes).context("non-ASCII HPGL parameters")?;
    let mut result = vec![];
    let mut start = None;
    for (i, ch) in s.char_indices() {
        if ch.is_ascii_whitespace() || ch == ',' {
            if let Some(a) = start.take() {
                result.push(number(&s[a..i])?);
            }
        } else if (ch == '-' || ch == '+')
            && start.is_some()
            && !matches!(s.as_bytes().get(i.wrapping_sub(1)), Some(b'e' | b'E'))
        {
            let a = start.replace(i).context("number start")?;
            result.push(number(&s[a..i])?);
        } else if start.is_none() {
            start = Some(i);
        }
    }
    if let Some(a) = start {
        result.push(number(&s[a..])?);
    }
    Ok(result)
}

fn number(s: &str) -> Result<f64> {
    let n: f64 = s
        .parse()
        .with_context(|| format!("invalid HPGL number {s:?}"))?;
    ensure!(n.is_finite() && n.abs() <= 1e12, "HPGL number out of range");
    Ok(n)
}

// HP PCL 5 Technical Reference, vector group, PE: a signed integer's low
// bit carries its sign; remaining bits are base-64 or base-32 digits, low first.
fn pe_number(bytes: &[u8], i: &mut usize, seven: bool) -> Result<f64> {
    let mut value = 0_u64;
    let mut shift = 0;
    loop {
        let raw = *bytes.get(*i).context("truncated HPGL PE number")?;
        *i += 1;
        let b = if seven { raw & 127 } else { raw };
        if b <= 32 || b == 127 || (!seven && (128..=160).contains(&b)) || b == 255 {
            continue;
        }
        let end = if seven { 95 } else { 191 };
        let limit = if seven { 126 } else { 254 };
        let radix_bits = if seven { 5 } else { 6 };
        let digit = if (end..=limit).contains(&b) {
            b - end
        } else if (63..63 + (1 << radix_bits)).contains(&b) {
            b - 63
        } else {
            bail!("invalid HPGL PE digit {b}");
        };
        ensure!(shift < 48, "HPGL PE number exceeds precision limit");
        value |= u64::from(digit) << shift;
        if b >= end {
            return Ok((value / 2) as f64 * if value & 1 == 1 { -1.0 } else { 1.0 });
        }
        shift += radix_bits;
    }
}

fn encoded(plot: &mut Plot, bytes: &[u8]) -> Result<()> {
    let mut i = 0;
    let mut seven = false;
    let mut fractional = 0;
    let mut absolute = false;
    let mut up = false;
    while i < bytes.len() {
        let raw = bytes[i];
        let flag = raw & 127;
        if raw <= 32 || raw == 127 || raw == 255 || (128..=160).contains(&raw) {
            i += 1;
            continue;
        }
        match flag {
            b'7' => {
                seven = true;
                i += 1;
            }
            b'<' => {
                plot.flush();
                up = true;
                i += 1;
            }
            b'=' => {
                absolute = true;
                i += 1;
            }
            b':' | b'>' => {
                i += 1;
                let n = pe_number(bytes, &mut i, seven)?;
                if flag == b':' {
                    ensure!((0.0..=u32::MAX as f64).contains(&n), "invalid PE pen");
                    plot.flush();
                    plot.pen = n as u32;
                } else {
                    ensure!(
                        (-24.0..=24.0).contains(&n),
                        "PE fractional bits out of range"
                    );
                    fractional = n as i32;
                }
            }
            _ => {
                ensure!(
                    plot.known_position || (absolute && up),
                    "PE after LB needs an absolute pen-up reposition"
                );
                let x = pe_number(bytes, &mut i, seven)? / 2_f64.powi(fractional);
                let y = pe_number(bytes, &mut i, seven)? / 2_f64.powi(fractional);
                plot.down = !up;
                plot.move_to(if absolute {
                    [x, y]
                } else {
                    [plot.current[0] + x, plot.current[1] + y]
                })?;
                plot.known_position = true;
                absolute = false;
                up = false;
            }
        }
        ensure!(plot.paths.len() <= 100_000, "HPGL exceeds 100000 paths");
    }
    ensure!(!absolute && !up, "HPGL PE flag without a coordinate pair");
    Ok(())
}

fn read(bytes: &[u8], scale: f64) -> Result<Plot> {
    ensure!(bytes.len() <= 64 * 1024 * 1024, "HPGL exceeds 64 MiB");
    let mut p = Plot::default();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_whitespace() || bytes[i] == b';' {
            i += 1;
            continue;
        }
        ensure!(
            i + 1 < bytes.len() && bytes[i..i + 2].iter().all(u8::is_ascii_alphabetic),
            "expected HPGL command at byte {i}; supply a plain HPGL stream without a printer wrapper"
        );
        let cmd = bytes[i..i + 2].to_ascii_uppercase();
        i += 2;
        if cmd == b"LB" {
            p.flush();
            let end = bytes[i..]
                .iter()
                .position(|&b| b == p.delimiter)
                .context("unterminated HPGL LB label")?
                + i;
            p.labels.push((
                p.current,
                String::from_utf8_lossy(&bytes[i..end]).into_owned(),
            ));
            ensure!(p.labels.len() <= 100_000, "HPGL exceeds 100000 labels");
            p.known_position = false;
            i = end + 1;
            continue;
        }
        if cmd == b"PE" {
            let end = bytes[i..]
                .iter()
                .position(|&b| b == b';')
                .context("PE needs a semicolon terminator")?
                + i;
            encoded(&mut p, &bytes[i..end])?;
            i = end + 1;
            continue;
        }
        let start = i;
        if cmd == b"DT" {
            // The delimiter itself may be a letter, including the first of two letters.
            if i < bytes.len() && bytes[i] != b';' {
                i += 1;
            }
        }
        while i < bytes.len() && bytes[i] != b';' {
            if i + 1 < bytes.len()
                && bytes[i].is_ascii_alphabetic()
                && bytes[i + 1].is_ascii_alphabetic()
            {
                break;
            }
            i += 1;
        }
        let raw = &bytes[start..i];
        if cmd == b"DT" {
            p.delimiter = raw.first().copied().unwrap_or(3);
            continue;
        }
        let n = match cmd.as_slice() {
            b"IN" | b"DF" | b"PU" | b"PD" | b"PA" | b"PR" | b"SP" | b"CI" | b"AA" | b"AR"
            | b"EA" | b"ER" | b"SI" => numbers(raw)?,
            _ => vec![],
        };
        match cmd.as_slice() {
            b"IN" | b"DF" => {
                p.flush();
                p.current = [0.0, 0.0];
                p.known_position = true;
                p.absolute = true;
                p.down = false;
                p.pen = 1;
                p.delimiter = 3;
            }
            b"SP" => {
                p.flush();
                let pen = n.first().copied().unwrap_or(0.0);
                ensure!(
                    (0.0..=u32::MAX as f64).contains(&pen) && pen.fract() == 0.0,
                    "invalid HPGL pen"
                );
                p.pen = pen as u32;
            }
            b"PU" => {
                p.flush();
                p.down = false;
                p.coordinates(&n)?;
            }
            b"PD" => {
                p.down = true;
                p.coordinates(&n)?;
            }
            b"PA" | b"PR" => {
                p.absolute = cmd == b"PA";
                p.coordinates(&n)?;
            }
            b"SI" => {
                ensure!(n.is_empty() || n.len() == 2, "SI needs width,height");
            }
            b"CI" => {
                ensure!(
                    p.known_position,
                    "CI after LB needs an absolute pen-up reposition"
                );
                ensure!(
                    !n.is_empty() && n.len() <= 2 && n[0] > 0.0,
                    "CI needs a positive radius"
                );
                p.flush();
                let pts = curves::circle(p.current, n[0], curves::TOLERANCE * scale)?;
                p.points += pts.len();
                if p.pen != 0 {
                    p.paths.push(Path {
                        points: pts,
                        circle: Some(p.current),
                        pen: p.pen,
                    });
                }
            }
            b"AA" | b"AR" => {
                ensure!(
                    p.known_position,
                    "AA/AR after LB needs an absolute pen-up reposition"
                );
                ensure!((3..=4).contains(&n.len()), "AA/AR needs centre x,y,sweep");
                let c = if cmd == b"AR" {
                    [p.current[0] + n[0], p.current[1] + n[1]]
                } else {
                    [n[0], n[1]]
                };
                let pts = curves::arc(
                    c,
                    curves::distance(c, p.current),
                    (p.current[1] - c[1]).atan2(p.current[0] - c[0]),
                    n[2].to_radians(),
                    curves::TOLERANCE * scale,
                )?;
                for q in pts.into_iter().skip(1) {
                    p.move_to(q)?;
                }
            }
            b"EA" | b"ER" => {
                ensure!(
                    p.known_position,
                    "EA/ER after LB needs an absolute pen-up reposition"
                );
                ensure!(n.len() == 2, "EA/ER needs opposite corner x,y");
                let a = p.current;
                let b = if cmd == b"ER" {
                    [a[0] + n[0], a[1] + n[1]]
                } else {
                    [n[0], n[1]]
                };
                p.flush();
                if p.pen != 0 {
                    p.paths.push(Path {
                        points: vec![a, [b[0], a[1]], b, [a[0], b[1]], a],
                        circle: None,
                        pen: p.pen,
                    });
                }
                p.points += 5;
            }
            // Text appearance and line styles do not change recovered geometry.
            b"DI" | b"DR" | b"SR" | b"SL" | b"LO" | b"ES" | b"SS" | b"SA" | b"CS" | b"CA"
            | b"SD" | b"AD" | b"LT" | b"PW" | b"VS" | b"WU" | b"NP" => {}
            // Ignoring coordinate transforms would silently produce a false comparison.
            b"SC" | b"IP" | b"IR" | b"RO" | b"IW" | b"PM" | b"FP" | b"EP" | b"BZ" | b"BR"
            | b"WG" | b"RA" | b"RR" | b"PG" => bail!(
                "HPGL {} changes geometry or pages and is unsupported; export unscaled plain polylines",
                String::from_utf8_lossy(&cmd)
            ),
            _ => {
                let warning = format!(
                    "Unknown HPGL command {} ignored; check the plot",
                    String::from_utf8_lossy(&cmd)
                );
                if !p.warnings.contains(&warning) {
                    p.warnings.push(warning);
                }
            }
        }
        ensure!(
            p.points <= 2_000_000 && p.paths.len() <= 100_000 && p.labels.len() <= 100_000,
            "HPGL exceeds geometry limits"
        );
    }
    p.flush();
    for path in &mut p.paths {
        for q in &mut path.points {
            q[0] /= scale;
            q[1] /= scale;
        }
        if let Some(c) = &mut path.circle {
            c[0] /= scale;
            c[1] /= scale;
        }
        ensure!(
            path.points
                .iter()
                .flatten()
                .all(|n| n.is_finite() && n.abs() <= 1e12),
            "scaled HPGL coordinates out of range; check plu-per-mm"
        );
    }
    for (q, _) in &mut p.labels {
        q[0] /= scale;
        q[1] /= scale;
    }
    Ok(p)
}

fn join(mut paths: Vec<Path>, tol: f64) -> Vec<Path> {
    let mut result = vec![];
    while let Some(mut p) = paths.pop() {
        p.points.dedup_by(|a, b| curves::distance(*a, *b) < 1e-9);
        if p.points.len() < 2 {
            continue;
        }
        while curves::distance(p.points[0], p.points[p.points.len() - 1]) > tol {
            let a = p.points[0];
            let b = p.points[p.points.len() - 1];
            let next = paths.iter().enumerate().find_map(|(i, q)| {
                if q.pen != p.pen
                    || q.points.len() < 2
                    || q.circle.is_some()
                    || curves::distance(q.points[0], q.points[q.points.len() - 1]) <= tol
                {
                    return None;
                }
                let x = q.points[0];
                let y = q.points[q.points.len() - 1];
                if curves::distance(b, x) <= tol {
                    Some((i, false, false))
                } else if curves::distance(b, y) <= tol {
                    Some((i, false, true))
                } else if curves::distance(a, y) <= tol {
                    Some((i, true, false))
                } else if curves::distance(a, x) <= tol {
                    Some((i, true, true))
                } else {
                    None
                }
            });
            let Some((i, prepend, reverse)) = next else {
                break;
            };
            let mut q = paths.swap_remove(i).points;
            if reverse {
                q.reverse();
            }
            if prepend {
                q.pop();
                q.extend(p.points);
                p.points = q;
            } else {
                p.points.extend(q.into_iter().skip(1));
            }
        }
        result.push(p);
    }
    result
}

fn closed(p: &Path, tol: f64) -> bool {
    p.points.len() > 3 && curves::distance(p.points[0], p.points[p.points.len() - 1]) <= tol
}

fn split_subloops(paths: Vec<Path>, tol: f64) -> (Vec<Path>, usize) {
    let mut output = vec![];
    let mut recovered = 0;
    for path in paths {
        if closed(&path, tol) || path.points.len() < 5 {
            output.push(path);
            continue;
        }
        // A branch can make endpoint joining produce B..B,C rather than B..B.
        // Exact repeated plotter coordinates delimit the cut cycle. Do not use
        // the joining tolerance here: nearby samples on an arc are not cycles.
        let key = |p: Pt| p.map(|v| if v == 0.0 { 0 } else { v.to_bits() });
        let mut seen = std::collections::HashMap::<[u64; 2], usize>::new();
        let mut stack: Vec<Pt> = vec![];
        for point in path.points {
            if let Some(&start) = seen.get(&key(point))
                && stack.len() - start >= 3
                && area(&stack[start..]) > 1e-8
            {
                let mut points = stack[start..].to_vec();
                points.push(point);
                output.push(Path {
                    points,
                    circle: None,
                    pen: path.pen,
                });
                recovered += 1;
                for (i, &old) in stack.iter().enumerate().skip(start + 1) {
                    if seen.get(&key(old)) == Some(&i) {
                        seen.remove(&key(old));
                    }
                }
                stack.truncate(start + 1);
                continue;
            }
            seen.insert(key(point), stack.len());
            stack.push(point);
        }
        if stack.len() > 1 {
            output.push(Path {
                points: stack,
                circle: None,
                pen: path.pen,
            });
        }
    }
    (output, recovered)
}

fn is_rectangle(p: &[Pt], tol: f64) -> bool {
    let Some(b) = bbox(p) else {
        return false;
    };
    (area(p) - (b[2] - b[0]) * (b[3] - b[1])).abs() <= tol * tol
        && p.iter().all(|q| {
            (q[0] - b[0]).abs() <= tol
                || (q[0] - b[2]).abs() <= tol
                || (q[1] - b[1]).abs() <= tol
                || (q[1] - b[3]).abs() <= tol
        })
}

fn ring_center(p: &[Pt], tol: f64) -> Option<Pt> {
    let b = bbox(p)?;
    let r = (b[2] - b[0] + b[3] - b[1]) / 4.0;
    let c = [(b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0];
    (p.len() >= 8
        && r <= 3.0
        && p.iter()
            .all(|&q| (curves::distance(q, c) - r).abs() < tol.max(0.05)))
    .then_some(c)
}

fn identity(label: &str) -> (String, Option<String>) {
    // BETA labels, also a useful public convention for handwritten imports.
    if let Some((name, rest)) = label.split_once('|') {
        let size = rest.split('|').next().unwrap_or("").trim();
        return (
            name.trim().into(),
            (!size.is_empty() && size != "unsized").then(|| size.into()),
        );
    }
    let mut name = None;
    let mut size = None;
    for line in label.lines() {
        if let Some((key, value)) = line.split_once(':') {
            if key.trim().eq_ignore_ascii_case("piece name") {
                name = Some(value.trim().into());
            }
            if key.trim().eq_ignore_ascii_case("size") {
                size = Some(value.trim().into());
            }
        }
    }
    (name.unwrap_or_else(|| label.trim().into()), size)
}

/// Recover closed cut lines, internal marks, names, quantities and plotted placements.
///
/// Grain cannot be recovered from pen geometry. Each distinct plotted orientation
/// is assumed grain-aligned with +x and may be flipped by 180 degrees when re-nested.
pub fn import_hpgl(bytes: &[u8], opts: &HpglOptions) -> Result<HpglReport> {
    ensure!(
        opts.plu_per_mm.is_finite() && opts.plu_per_mm > 0.0,
        "plu-per-mm must be finite and positive"
    );
    ensure!(
        opts.tolerance_mm.is_finite() && opts.tolerance_mm > 0.0 && opts.tolerance_mm <= 1.0,
        "HPGL tolerance must be positive and at most 1 mm"
    );
    if let Some(w) = opts.width {
        ensure!(
            w.is_finite() && w > 0.0,
            "width must be finite and positive"
        );
    }
    let mut plot = read(bytes, opts.plu_per_mm)?;
    let mut warnings = std::mem::take(&mut plot.warnings);
    warnings.push("HPGL grain is unknown: plotted orientations assumed grain-aligned along +x; rotation is flip180. Check grain and nap before cutting.".into());
    let tol = opts.tolerance_mm;
    let (paths, recovered) = split_subloops(join(plot.paths, tol), tol);
    ensure!(
        paths.len() <= 100_000,
        "HPGL exceeds 100000 paths after recovering cycles"
    );
    if recovered > 0 {
        warnings.push(format!("Recovered {recovered} closed outlines from branched pen paths; remaining tails are marks."));
    }
    let mut loops: Vec<Path> = paths
        .iter()
        .filter(|p| closed(p, tol) && area(&p.points) > 1e-8)
        .cloned()
        .collect();
    for p in &mut loops {
        p.points.pop();
    }
    loops.sort_by(|a, b| area(&b.points).total_cmp(&area(&a.points)));
    ensure!(!loops.is_empty(), "HPGL contains no closed cut outlines");
    let mut rectangle_width = None;
    // A rectangular frame is ambiguous with a garment. Require at least two
    // substantial contained loops, or a supplied matching width plus one loop.
    if let Some(frame) = loops.first() {
        let b = bbox(&frame.points).context("frame bounds")?;
        let contained = loops
            .iter()
            .skip(1)
            .filter(|p| area(&p.points) > 25.0 && pattern::nested(&frame.points, &p.points))
            .count();
        if is_rectangle(&frame.points, tol)
            && (contained >= 2
                || (contained >= 1 && opts.width.is_some_and(|w| (w - (b[3] - b[1])).abs() <= tol)))
        {
            rectangle_width = Some(b[3]);
            if b[1].abs() > tol {
                warnings.push("Marker frame starts away from y=0: inferred width includes its plot offset. Supply --width and review the original plot origin.".into());
            }
            warnings.push("Enclosing rectangle treated as marker boundary, not a cut piece; plotted origin retained.".into());
            loops.remove(0);
        }
    }
    let mut outer: Vec<Path> = vec![];
    let mut internal = vec![];
    for path in loops {
        if outer
            .iter()
            .any(|p| pattern::nested(&p.points, &path.points))
        {
            internal.push(path);
        } else {
            outer.push(path);
        }
    }
    ensure!(
        !outer.is_empty(),
        "HPGL has no pieces after removing marker frame"
    );
    let max_y = outer
        .iter()
        .flat_map(|p| &p.points)
        .map(|q| q[1])
        .fold(f64::NEG_INFINITY, f64::max);
    let width = opts.width.or(rectangle_width).unwrap_or_else(|| {
        warnings.push("No fabric width or marker boundary: width inferred from maximum cut-line y; unused fabric cannot be inferred. Supply --width for a fair comparison.".into());
        max_y
    });
    ensure!(
        width.is_finite() && width > 0.0,
        "cannot infer a positive width; supply --width"
    );
    let mut pieces: Vec<Piece> = vec![];
    let mut placements = vec![];
    let open: Vec<_> = paths.iter().filter(|p| !closed(p, tol)).collect();
    let mut assigned = vec![false; open.len()];
    for path in outer {
        let bounds = bbox(&path.points).context("piece bounds")?;
        let origin = [bounds[0], bounds[1]];
        let local = |q: Pt| [q[0] - origin[0], q[1] - origin[1]];
        let outline: Vec<_> = path.points.iter().copied().map(local).collect();
        let labels: Vec<_> = plot
            .labels
            .iter()
            .filter(|(q, _)| pattern::contains(&path.points, *q))
            .collect();
        let (name, mut size) = labels
            .iter()
            .find(|(_, s)| s.contains('|') || s.to_ascii_lowercase().contains("piece name:"))
            .or_else(|| labels.first())
            .map(|(_, s)| identity(s))
            .unwrap_or_else(|| (format!("Plotted piece {}", placements.len() + 1), None));
        if size.is_none() {
            size = labels.iter().flat_map(|(_, s)| s.lines()).find_map(|line| {
                let (key, value) = line.split_once(':')?;
                (key.trim().eq_ignore_ascii_case("size") && !value.trim().is_empty())
                    .then(|| value.trim().to_string())
            });
        }
        let mut marks = vec![];
        for p in internal
            .iter()
            .filter(|p| pattern::nested(&path.points, &p.points))
        {
            let center = p.circle.or_else(|| ring_center(&p.points, tol));
            let (kind, points) = if let Some(c) = center
                .filter(|_| bbox(&p.points).is_some_and(|b| (b[2] - b[0]).max(b[3] - b[1]) <= 6.0))
            {
                (MarkKind::Drill, vec![local(c)])
            } else {
                let mut pts: Vec<_> = p.points.iter().copied().map(local).collect();
                pts.push(pts[0]);
                (MarkKind::Line, pts)
            };
            marks.push(Mark {
                kind,
                points,
                text: None,
            });
        }
        for (idx, p) in open.iter().enumerate() {
            if p.points.iter().all(|&q| {
                pattern::contains(&path.points, q)
                    || pattern::boundary_distance(&path.points, q) <= 5.0
            }) {
                assigned[idx] = true;
                let length: f64 = p
                    .points
                    .windows(2)
                    .map(|w| curves::distance(w[0], w[1]))
                    .sum();
                let notch = length <= 10.0
                    && p.points
                        .iter()
                        .any(|&q| pattern::boundary_distance(&path.points, q) <= tol);
                let mut points = p.points.clone();
                if notch
                    && pattern::boundary_distance(&path.points, points[points.len() - 1])
                        < pattern::boundary_distance(&path.points, points[0])
                {
                    points.reverse();
                }
                marks.push(Mark {
                    kind: if notch {
                        MarkKind::Notch
                    } else {
                        MarkKind::Line
                    },
                    points: points.into_iter().map(local).collect(),
                    text: None,
                });
            }
        }
        for (q, s) in labels {
            marks.push(Mark {
                kind: MarkKind::Text,
                points: vec![local(*q)],
                text: Some(s.clone()),
            });
        }
        let found = pieces.iter().enumerate().find_map(|(idx, p)| {
            if p.size != size {
                return None;
            }
            equivalent(&p.outline, &path.points, tol)
                .map(|(mirrored, rotation_deg, translate)| (idx, mirrored, rotation_deg, translate))
        });
        let (idx, mirrored, rotation_deg, translate) = if let Some(found) = found {
            found
        } else {
            let idx = pieces.len();
            pieces.push(Piece {
                id: format!("PLT-{}", idx + 1),
                name,
                size,
                outline,
                marks,
                grain: Some([[0.0, 0.0], [1.0, 0.0]]),
                rotation: RotationRule::Flip180,
                quantity: 0,
                mirror_quantity: 0,
                ..Default::default()
            });
            (idx, false, 0.0, origin)
        };
        let piece = &mut pieces[idx];
        let count = if mirrored {
            &mut piece.mirror_quantity
        } else {
            &mut piece.quantity
        };
        let copy = *count;
        *count = count.checked_add(1).context("too many plotted copies")?;
        placements.push(Placement {
            piece_id: piece.id.clone(),
            copy,
            mirrored,
            rotation_deg,
            translate,
            ..Default::default()
        });
    }
    let unassigned = assigned.iter().filter(|a| !**a).count();
    if unassigned > 0 {
        warnings.push(format!("{unassigned} open pen paths outside pieces ignored; inspect these for unclosed cut lines."));
    }
    warnings.push("Nested loops are internal lines; near-edge paths up to 10 mm are notches; circular rings up to 6 mm diameter are drills. Grouping compares shape and size; per-copy labels and marks use the first copy.".into());
    let job = Job {
        format: JOB_FORMAT.into(),
        name: opts.name.clone(),
        fabric: Fabric {
            width,
            ..Default::default()
        },
        pieces,
        meta: std::collections::BTreeMap::from([
            ("import_warnings".into(), serde_json::json!(warnings)),
            ("source_format".into(), serde_json::json!("hpgl")),
            ("plu_per_mm".into(), serde_json::json!(opts.plu_per_mm)),
        ]),
        ..Default::default()
    };
    let metrics = compute_metrics(&job, &placements);
    let result = MarkerResult {
        format: RESULT_FORMAT.into(),
        job_name: job.name.clone(),
        width,
        length: metrics.length,
        piece_area: metrics.piece_area,
        efficiency: metrics.efficiency,
        engine: "imported".into(),
        seed: None,
        runtime_s: 0.0,
        placements,
        warnings: warnings.clone(),
    };
    Ok(HpglReport {
        job,
        result,
        warnings,
    })
}

fn equivalent(a: &[Pt], b: &[Pt], tol: f64) -> Option<(bool, f64, Pt)> {
    // Translation, half-turn and both reflections preserve the assumed grain axis.
    // Vertex count can differ when one plot subdivides a straight segment.
    for mirrored in [false, true] {
        for angle in [0.0, 180.0] {
            let transformed = transform_points(a, mirrored, angle, [0.0, 0.0]);
            let ab = bbox(&transformed)?;
            let bb = bbox(b)?;
            if ((ab[2] - ab[0]) - (bb[2] - bb[0])).abs() > tol
                || ((ab[3] - ab[1]) - (bb[3] - bb[1])).abs() > tol
            {
                continue;
            }
            let t = [bb[0] - ab[0], bb[1] - ab[1]];
            let placed: Vec<_> = transformed
                .iter()
                .map(|p| [p[0] + t[0], p[1] + t[1]])
                .collect();
            if (area(&placed) - area(b)).abs() > tol * tol {
                continue;
            }
            // Compare ordered corners, not an unordered vertex set: polygons
            // through the same vertices can have very different edges.
            let left = corners(&placed);
            let right = corners(b);
            if left.len() != right.len() {
                continue;
            }
            let n = left.len();
            for start in 0..n {
                if curves::distance(left[0], right[start]) > 1e-7 {
                    continue;
                }
                if [false, true].into_iter().any(|reverse| {
                    (0..n).all(|i| {
                        let j = if reverse {
                            (start + n - i) % n
                        } else {
                            (start + i) % n
                        };
                        curves::distance(left[i], right[j]) <= 1e-7
                    })
                }) {
                    return Some((mirrored, angle, t));
                }
            }
        }
    }
    None
}

fn corners(points: &[Pt]) -> Vec<Pt> {
    let mut points = points.to_vec();
    points.dedup_by(|a, b| curves::distance(*a, *b) < 1e-9);
    if points.len() > 1 && curves::distance(points[0], points[points.len() - 1]) < 1e-9 {
        points.pop();
    }
    let n = points.len();
    (0..n)
        .filter_map(|i| {
            let a = points[(i + n - 1) % n];
            let b = points[i];
            let c = points[(i + 1) % n];
            let u = [b[0] - a[0], b[1] - a[1]];
            let v = [c[0] - b[0], c[1] - b[1]];
            let cross = (u[0] * v[1] - u[1] * v[0]).abs();
            (!(cross <= 1e-9 * (u[0].hypot(u[1]) + v[0].hypot(v[1]))
                && u[0] * v[0] + u[1] * v[1] >= 0.0))
                .then_some(b)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use beta_marker_core::{geometry::place, validate};

    fn read(s: &[u8]) -> HpglReport {
        import_hpgl(
            s,
            &HpglOptions {
                width: Some(100.0),
                ..Default::default()
            },
        )
        .expect("HPGL")
    }

    #[test]
    fn relative_signed_coordinates_missing_terminators_labels_and_pen_zero() {
        let r = read(b"IN\r\nSP1PU400,400PD4400,400PR0,2000,-4000,0,0,-2000PU\r\nDT@;SI.2,.3;PU800,800LBFront | M | copy 0@;PA;SP0;PU0,0;EA40000,40000;");
        assert_eq!(r.job.pieces.len(), 1);
        assert_eq!(r.job.pieces[0].name, "Front");
        assert_eq!(r.job.pieces[0].size.as_deref(), Some("M"));
        assert_eq!(r.result.length, 110.0);
        assert_eq!(r.result.piece_area, 5000.0);
        assert!(validate(&r.job, &r.result).ok);
        let compact = read(b"IN;PU0,0;PR;PD4000+0+0+2000-4000+0+0-2000;");
        assert_eq!(compact.result.piece_area, 5000.0);
    }

    #[test]
    fn join_reversed_segments_classify_marks_and_keep_internal_rectangles() {
        let r = read(b"IN;PU0,0;PD4000,0;PU4000,2000;PD4000,0;PU0,2000;PD4000,2000;PU0,0;PD0,2000;SP2;PU800,800;EA1600,1200;PU2000,1000;CI30;PU0,1000;PD100,1000;");
        assert_eq!(r.job.pieces.len(), 1);
        let p = &r.job.pieces[0];
        assert_eq!(
            p.marks.iter().filter(|m| m.kind == MarkKind::Drill).count(),
            1
        );
        assert_eq!(
            p.marks.iter().filter(|m| m.kind == MarkKind::Notch).count(),
            1
        );
        assert_eq!(
            p.marks.iter().filter(|m| m.kind == MarkKind::Line).count(),
            1
        );
        assert!(validate(&r.job, &r.result).ok);
    }

    #[test]
    fn notch_sharing_an_outline_endpoint_does_not_hide_the_cut_cycle() {
        let r = read(b"IN;SP1;PU0,0;PD4000,0;PU4000,0;PD4000,2000;PU4000,2000;PD0,2000;PU0,2000;PD0,0;PU0,0;PD100,100;");
        assert_eq!(r.job.pieces.len(), 1);
        assert_eq!(r.result.piece_area, 5000.0);
        assert_eq!(
            r.job.pieces[0]
                .marks
                .iter()
                .filter(|m| m.kind == MarkKind::Notch)
                .count(),
            1
        );
        assert!(validate(&r.job, &r.result).ok);
        assert!(r.warnings.iter().any(|w| w.contains("branched pen paths")));
    }

    #[test]
    fn arc_absolute_relative_and_rectangles() {
        let r = read(b"IN;PU400,0;PD;AA0,0,180;AR400,0,180;PU;PU1000,1000;ER400,800;");
        assert_eq!(r.result.placements.len(), 2);
        assert!((r.result.piece_area - (std::f64::consts::PI * 100.0 + 200.0)).abs() < 1.0);
    }

    #[test]
    fn width_from_frame_or_cut_extent_and_override_units() {
        let s = b"IN;PU0,0;EA8000,6000;PU400,400;EA2400,2400;PU3200,400;EA5200,2400;";
        let r = import_hpgl(s, &HpglOptions::default()).expect("frame");
        assert_eq!(r.job.fabric.width, 150.0);
        assert_eq!(r.result.placements.len(), 2);
        assert!(validate(&r.job, &r.result).ok);
        let r = import_hpgl(
            b"IN;PU0,0;EA4000,2000;",
            &HpglOptions {
                plu_per_mm: 20.0,
                ..Default::default()
            },
        )
        .expect("scale");
        assert_eq!(r.job.fabric.width, 100.0);
        assert_eq!(r.result.length, 200.0);
        assert!(r.warnings.iter().any(|w| w.contains("unused fabric")));
    }

    #[test]
    fn original_comparison_fixture_has_known_consumption() {
        let r = import_hpgl(
            include_bytes!("../../../testdata/dxf/comparison-marker.plt"),
            &HpglOptions::default(),
        )
        .expect("original plot");
        assert_eq!(r.job.pieces.len(), 1);
        assert_eq!(r.job.pieces[0].quantity, 2);
        assert_eq!(r.result.placements.len(), 2);
        assert_eq!(r.result.width, 150.0);
        assert_eq!(r.result.length, 230.0);
        assert_eq!(r.result.piece_area, 9000.0);
        assert!((r.result.efficiency - 9000.0 / (150.0 * 230.0)).abs() < 1e-10);
        assert!(validate(&r.job, &r.result).ok);
    }

    #[test]
    fn grouping_reflections_and_half_turns_reproduces_shapes() {
        let s = b"IN;PU0,0;PD1600,0,1200,800,0,400,0,0;PU2400,0;PD4000,0,4000,400,2800,800,2400,0;PU6400,800;PD4800,800,5200,0,6400,400,6400,800;";
        let r = read(s);
        assert_eq!(r.job.pieces.len(), 1);
        assert_eq!(
            r.job.pieces[0].quantity + r.job.pieces[0].mirror_quantity,
            3
        );
        assert_eq!(r.job.pieces[0].mirror_quantity, 1);
        assert!(validate(&r.job, &r.result).ok);
        assert_eq!(r.result.length, 160.0);
        for pl in &r.result.placements {
            assert!((area(&place(&r.job.pieces[0].outline, pl)) - 550.0).abs() < 1e-7);
        }
    }

    #[test]
    fn nested_export_import_round_trip_uses_independent_metrics() {
        let p = Piece {
            id: "front".into(),
            name: "Front".into(),
            size: Some("M".into()),
            outline: vec![[0.0, 0.0], [60.0, 0.0], [50.0, 35.0], [0.0, 40.0]],
            quantity: 2,
            mirror_quantity: 1,
            marks: vec![Mark {
                kind: MarkKind::Drill,
                points: vec![[20.0, 20.0]],
                text: None,
            }],
            ..Default::default()
        };
        let job = Job {
            format: JOB_FORMAT.into(),
            name: "round trip".into(),
            pieces: vec![p],
            fabric: Fabric {
                width: 150.0,
                ..Default::default()
            },
            spacing: beta_marker_core::Spacing {
                piece_gap: 1.0,
                edge_gap: 1.0,
            },
            ..Default::default()
        };
        let nested = beta_marker_nest::nest(
            &job,
            &beta_marker_nest::NestOptions {
                time_limit_s: 0.05,
                workers: Some(2),
                seed: Some(42),
                ..Default::default()
            },
        )
        .expect("short nest");
        let hpgl = beta_marker_export::to_hpgl(&job, &nested, &Default::default()).expect("export");
        let r = import_hpgl(
            hpgl.as_bytes(),
            &HpglOptions {
                width: Some(150.0),
                ..Default::default()
            },
        )
        .expect("import");
        assert_eq!(r.result.placements.len(), nested.placements.len());
        assert!((r.result.length - nested.length).abs() <= 0.025);
        assert!((r.result.efficiency - nested.efficiency).abs() < 0.001);
        let v = validate(&r.job, &r.result);
        assert!(v.ok, "{:?}", v.errors);
        assert_eq!(r.result.length, v.length);
        assert_eq!(r.result.efficiency, v.efficiency);
    }

    #[test]
    fn malformed_and_unsupported_streams_fail_without_panics() {
        for s in [
            "",
            "PD1;",
            "PU0,0;PD1,0,1,1;",
            "LBmissing",
            "SC0,1,0,1;",
            "PEabcdef;",
            "PU1e999,0;",
            "CI-4;",
            "AA0,0;",
            "EA1;",
            "IN;PU0,0;EA4000,4000;LBname\x03;PR1,1;",
        ] {
            assert!(
                import_hpgl(s.as_bytes(), &HpglOptions::default()).is_err(),
                "{s}"
            );
        }
        assert!(
            import_hpgl(
                b"IN;",
                &HpglOptions {
                    plu_per_mm: 0.0,
                    ..Default::default()
                }
            )
            .is_err()
        );
        let r = read(b"IN;PU0,0;EA8000,8000;PU400,400;EA1600,1600;");
        // Width doesn't match the big rectangle; a single internal loop stays a mark.
        assert_eq!(r.job.pieces.len(), 1);
        assert_eq!(r.result.piece_area, 40000.0);
    }

    #[test]
    fn pe_base64_base32_fractional_absolute_and_negative() {
        fn encode(n: i64, seven: bool) -> Vec<u8> {
            let mut v = n.unsigned_abs() * 2 + u64::from(n < 0);
            let base = if seven { 32 } else { 64 };
            let mut bytes = vec![];
            while v >= base {
                bytes.push(63 + (v % base) as u8);
                v /= base;
            }
            bytes.push(if seven { 95 } else { 191 } + v as u8);
            bytes
        }
        for seven in [false, true] {
            let mut s = b"IN;SP1;PE".to_vec();
            if seven {
                s.push(b'7');
            }
            s.push(b'>');
            s.extend(encode(1, seven)); // one fractional binary bit
            s.extend(b"<=");
            for (x, y) in [(800, 800), (8000, 0), (0, 4000), (-8000, 0), (0, -4000)] {
                s.extend(encode(x, seven));
                s.extend(encode(y, seven));
            }
            s.extend(b";PU;");
            let r = read(&s);
            assert_eq!(r.result.piece_area, 5000.0);
            assert_eq!(r.result.length, 110.0);
            assert!(validate(&r.job, &r.result).ok);
        }
    }
}
