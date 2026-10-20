use crate::scene::{Scene, ascii};
use anyhow::ensure;
use beta_marker_core::Pt;
use std::fmt::Write;

pub(super) fn render(s: &Scene, rotate: bool) -> anyhow::Result<String> {
    let convert = |p: Pt| -> anyhow::Result<(i32, i32)> {
        let p = if rotate { [s.height - p[1], p[0]] } else { p };
        ensure!(
            p.iter()
                .all(|v| v.is_finite() && (v * 40.0).abs() <= i32::MAX as f64),
            "Coordinates exceed the HPGL integer range."
        );
        Ok(((p[0] * 40.0).round() as i32, (p[1] * 40.0).round() as i32))
    };
    let mut out = String::from("IN;PA;DT\x03;SI0.20,0.30;SP1;\n");
    for pen in [1, 2, 3] {
        writeln!(out, "SP{pen};")?;
        for path in s.paths.iter().filter(|p| p.pen == pen) {
            let Some(&first) = path.points.first() else {
                continue;
            };
            let (x, y) = convert(first)?;
            write!(out, "PU{x},{y};PD")?;
            for (i, &p) in path
                .points
                .iter()
                .skip(1)
                .chain(if path.closed { Some(&first) } else { None })
                .enumerate()
            {
                let (x, y) = convert(p)?;
                if i > 0 {
                    out.push(',');
                }
                write!(out, "{x},{y}")?;
            }
            out.push_str(";PU;\n");
        }
        if pen == 2 {
            for &centre in &s.circles {
                // A closed 32-sided drill ring stays accurate to <0.004 mm.
                for i in 0..=32 {
                    let angle = i as f64 * std::f64::consts::TAU / 32.0;
                    let (x, y) = convert([
                        centre[0] + 0.75 * angle.cos(),
                        centre[1] + 0.75 * angle.sin(),
                    ])?;
                    write!(out, "{}{x},{y};", if i == 0 { "PU" } else { "PD" })?;
                }
                out.push_str("PU;\n");
            }
            if rotate {
                out.push_str("DI0,1;\n");
            }
            for text in &s.texts {
                let (x, y) = convert(text.point)?;
                writeln!(out, "PU{x},{y};LB{}\x03;", ascii(&text.value))?;
            }
        }
    }
    out.push_str("PU;SP0;\n");
    Ok(out)
}
