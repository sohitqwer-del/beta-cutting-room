//! Reproduce the small, original garment examples: cargo run -p beta-marker-nest
//! --example generate_jobs -- examples/jobs
use anyhow::Result;
use beta_marker_core::*;
use std::collections::BTreeMap;
use std::path::PathBuf;

fn piece(id: &str, outline: Vec<Pt>) -> Piece {
    Piece {
        id: id.into(),
        name: id.into(),
        style: Some("BETA synthetic".into()),
        size: None,
        material: None,
        outline,
        grain: None,
        quantity: 1,
        mirror_quantity: 0,
        rotation: RotationRule::Inherit,
        buffer: 0.0,
        marks: vec![],
        ..Default::default()
    }
}

fn job(name: &str, width: f64, pieces: Vec<Piece>) -> Job {
    Job {
        match_rules: Vec::new(),
        format: JOB_FORMAT.into(),
        name: name.into(),
        fabric: Fabric {
            width,
            max_length: None,
            nap: Nap::None,
            allow_flip: false,
            tilt_deg: 0.0,
            ..Default::default()
        },
        spacing: Spacing::default(),
        pieces,
        order: None,
        meta: BTreeMap::new(),
    }
}

/// Four synthetic jobs, independent of any vendor patterns or file formats.
pub fn synthetic_jobs() -> Vec<(&'static str, Job)> {
    // x is length/grain. The front has a shoulder slope, armhole and neck step.
    let front = vec![
        [0., 0.],
        [620., 0.],
        [620., 235.],
        [290., 235.],
        [205., 205.],
        [100., 205.],
        [30., 155.],
        [30., 70.],
        [0., 55.],
    ];
    let back = vec![
        [0., 0.],
        [640., 0.],
        [640., 460.],
        [270., 460.],
        [190., 410.],
        [80., 410.],
        [0., 300.],
    ];
    let sleeve = vec![
        [0., 0.],
        [380., 0.],
        [440., 115.],
        [380., 230.],
        [0., 230.],
        [-45., 150.],
        [-45., 80.],
    ];
    let mut shirts = vec![];
    for (size, scale) in [("S", 0.94), ("M", 1.0), ("L", 1.06)] {
        for (name, shape, paired) in [
            ("front", &front, true),
            ("back", &back, false),
            ("sleeve", &sleeve, true),
        ] {
            let mut p = piece(
                &format!("{name}-{size}"),
                shape.iter().map(|p| [p[0] * scale, p[1] * scale]).collect(),
            );
            p.size = Some(size.into());
            p.mirror_quantity = u32::from(paired);
            shirts.push(p);
        }
    }
    let mut shirt_job = job("Synthetic shirt S/M/L", 1500., shirts);
    shirt_job.order = Some(Order {
        sizes: BTreeMap::from([("S".into(), 1), ("M".into(), 2), ("L".into(), 1)]),
    });
    let mut leg = piece(
        "trouser-front",
        vec![
            [0., 0.],
            [1030., 20.],
            [1030., 220.],
            [470., 260.],
            [350., 320.],
            [120., 300.],
            [0., 240.],
        ],
    );
    leg.mirror_quantity = 1;
    let mut rear = piece(
        "trouser-back",
        vec![
            [0., 0.],
            [1050., 10.],
            [1050., 240.],
            [450., 290.],
            [290., 370.],
            [80., 330.],
            [0., 260.],
        ],
    );
    rear.mirror_quantity = 1;
    let mut trousers = job(
        "Synthetic trousers with one-way nap",
        1500.,
        vec![leg, rear],
    );
    trousers.fabric.nap = Nap::OneWay;
    let mut buffered = job(
        "Synthetic buffered shirt parts",
        1200.,
        vec![
            piece("front", front),
            piece("back", back),
            piece("sleeve", sleeve),
        ],
    );
    buffered.spacing = Spacing {
        piece_gap: 4.,
        edge_gap: 10.,
    };
    for (i, p) in buffered.pieces.iter_mut().enumerate() {
        p.buffer = (i * 2) as f64;
        p.quantity = 2;
    }
    let mut tilted = job(
        "Synthetic narrow panel requiring tilt",
        30.1,
        vec![piece(
            "bias-panel",
            beta_marker_core::geometry::transform_points(
                &[[0., 0.], [200., 0.], [200., 30.], [0., 30.]],
                false,
                2.,
                [0., 0.],
            ),
        )],
    );
    tilted.fabric.tilt_deg = 2.;
    tilted.pieces[0].rotation = RotationRule::Fixed;
    vec![
        ("shirts-sml.json", shirt_job),
        ("trousers-one-way.json", trousers),
        ("gaps-buffers.json", buffered),
        ("tilt.json", tilted),
    ]
}

fn main() -> Result<()> {
    let directory = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("examples/jobs"));
    std::fs::create_dir_all(&directory)?;
    for (name, job) in synthetic_jobs() {
        let mut json = serde_json::to_string_pretty(&job)?;
        json.push('\n');
        std::fs::write(directory.join(name), json)?;
    }
    Ok(())
}
