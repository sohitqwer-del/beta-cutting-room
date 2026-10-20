//! Exploded R12 marker DXF. All geometry is in marker millimetres.
use crate::{ExportOptions, scene};
use beta_marker_core::{Job, MarkerResult, Pt};
use std::fmt::Write;
fn line(out: &mut String, a: Pt, b: Pt, layer: u8) -> std::fmt::Result {
    writeln!(
        out,
        "0\nLINE\n8\n{layer}\n10\n{}\n20\n{}\n30\n0\n11\n{}\n21\n{}\n31\n0",
        a[0], a[1], b[0], b[1]
    )
}
/// Export the whole marker as exploded AAMA-style geometry: layer 1 cut,
/// 4 notches, 13 drills, 8 internal lines, 7 grain. Text carries name, size,
/// bundle and copy. Boundary and fabric guides use layer 0. The marker boundary
/// is four LINEs so generic import cannot mistake it for a garment piece.
pub fn to_dxf(job: &Job, result: &MarkerResult, opts: &ExportOptions) -> anyhow::Result<String> {
    let scene = scene::build(job, result, opts)?;
    let mut out = String::from(
        "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1009\n9\n$INSUNITS\n70\n4\n9\n$MEASUREMENT\n70\n1\n0\nENDSEC\n0\nSECTION\n2\nTABLES\n0\nTABLE\n2\nLAYER\n70\n7\n",
    );
    for layer in [0, 1, 4, 7, 8, 13, 15] {
        writeln!(out, "0\nLAYER\n2\n{layer}\n70\n0\n62\n7\n6\nCONTINUOUS")?;
    }
    out.push_str("0\nENDTAB\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n0\nTEXT\n8\n0\n10\n0\n20\n-5\n40\n2.5\n1\nUNITS: METRIC\n");
    let boundary = [
        [0., 0.],
        [scene.length, 0.],
        [scene.length, scene.width],
        [0., scene.width],
    ];
    for i in 0..4 {
        line(&mut out, boundary[i], boundary[(i + 1) % 4], 0)?;
    }
    for path in scene.paths {
        if path.points.is_empty() {
            continue;
        }
        if path.layer == 0 && path.closed {
            for i in 0..path.points.len() {
                line(
                    &mut out,
                    path.points[i],
                    path.points[(i + 1) % path.points.len()],
                    0,
                )?;
            }
        } else {
            writeln!(
                out,
                "0\nPOLYLINE\n8\n{}\n66\n1\n70\n{}\n10\n0\n20\n0\n30\n0",
                path.layer,
                if path.closed { 1 } else { 0 }
            )?;
            for p in path.points {
                writeln!(
                    out,
                    "0\nVERTEX\n8\n{}\n10\n{}\n20\n{}\n30\n0",
                    path.layer, p[0], p[1]
                )?;
            }
            out.push_str("0\nSEQEND\n");
        }
    }
    for p in scene.circles {
        writeln!(out, "0\nPOINT\n8\n13\n10\n{}\n20\n{}\n30\n0", p[0], p[1])?;
    }
    for text in scene.texts {
        writeln!(
            out,
            "0\nTEXT\n8\n15\n10\n{}\n20\n{}\n30\n0\n40\n2.5\n1\n{}",
            text.point[0],
            text.point[1],
            scene::ascii(&text.value)
        )?;
    }
    out.push_str("0\nENDSEC\n0\nEOF\n");
    Ok(out)
}
