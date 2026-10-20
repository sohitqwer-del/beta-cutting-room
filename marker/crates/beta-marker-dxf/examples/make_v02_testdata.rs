//! Reproduce original HPGL and experimental grade-table fixtures offline.

use anyhow::Result;
use beta_marker_core::{Mark, MarkKind, Piece};

fn main() -> Result<()> {
    let destination = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testdata/dxf");
    let mut marks = vec![];
    for (i, p) in [[0.0, 0.0], [100.0, 0.0], [100.0, 50.0], [0.0, 50.0]]
        .into_iter()
        .enumerate()
    {
        marks.push(Mark {
            kind: MarkKind::Text,
            points: vec![p],
            text: Some(format!("#{}", i + 1)),
        });
    }
    let piece = Piece {
        id: "GRADE-BASE-M".into(),
        name: "Original graded panel".into(),
        size: Some("M".into()),
        outline: vec![
            [0.0, 0.0],
            [100.0, 0.0],
            [100.0, 50.0],
            [50.0, 50.0],
            [0.0, 50.0],
        ],
        quantity: 1,
        grain: Some([[10.0, 20.0], [90.0, 20.0]]),
        marks,
        ..Default::default()
    };
    std::fs::write(
        destination.join("grade-base-mm.dxf"),
        beta_marker_dxf::export_aama(&[piece])?,
    )?;
    std::fs::write(
        destination.join("grade-cumulative.rul"),
        "// Original Project BETA grading exercise, CC BY 4.0\nSIZES: S M L\nBASE: M\nUNITS: MM\nMODE: CUMULATIVE\nRULE: 1 2,1 0,0 -2,-1\nRULE: 2 -2,1 0,0 2,-1\nRULE: 3 -2,-1 0,0 2,1\nRULE: 4 2,-1 0,0 -2,1\n",
    )?;
    std::fs::write(
        destination.join("grade-incremental.rul"),
        "// Same original exercise, adjacent steps ending at each size; entry 0 unused.\nSIZES: S M L\nBASE: M\nUNITS: MM\nMODE: INCREMENTAL\nRULE: 1 0,0 -2,-1 -2,-1\nRULE: 2 0,0 2,-1 2,-1\nRULE: 3 0,0 2,1 2,1\nRULE: 4 0,0 -2,1 -2,1\n",
    )?;
    std::fs::write(
        destination.join("comparison-marker.plt"),
        "IN;PA;SP1;PU0,0;EA12000,6000;\nPU400,400;PD4400,400,4400,2400,2400,2000,400,2400,400,400;PU;\nPU5200,400;PD9200,400,9200,2400,7200,2000,5200,2400,5200,400;PU;\nSP2;PU2000,1200;CI30;PU6800,1200;CI30;PU800,800;LBFront | M | copy 0\x03;PU5600,800;LBFront | M | copy 1\x03;PU;SP0;\n",
    )?;
    println!(
        "Wrote original base, cumulative/incremental grade rules and comparison marker fixtures"
    );
    Ok(())
}
