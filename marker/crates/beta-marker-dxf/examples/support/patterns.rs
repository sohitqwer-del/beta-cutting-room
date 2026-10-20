//! Original parametric drafting exercises, not production fit patterns.

use beta_marker_core::{Mark, MarkKind, Piece, Pt, RotationRule};

fn curve(out: &mut Vec<Pt>, a: Pt, b: Pt, c: Pt) {
    let start = out[out.len() - 1];
    for i in 1..=32 {
        let t = i as f64 / 32.0;
        let u = 1.0 - t;
        out.push([
            u * u * u * start[0]
                + 3.0 * u * u * t * a[0]
                + 3.0 * u * t * t * b[0]
                + t * t * t * c[0],
            u * u * u * start[1]
                + 3.0 * u * u * t * a[1]
                + 3.0 * u * t * t * b[1]
                + t * t * t * c[1],
        ]);
    }
}

fn rectangle(w: f64, h: f64) -> Vec<Pt> {
    vec![[0.0, 0.0], [w, 0.0], [w, h], [0.0, h]]
}

fn bodice(w: f64, h: f64, neck: f64) -> Vec<Pt> {
    let mut p = vec![[0.0, 0.0], [w, 0.0], [w * 0.94, h * 0.38], [w, h * 0.68]];
    curve(
        &mut p,
        [w * 0.72, h * 0.72],
        [w * 0.70, h * 0.87],
        [w * 0.79, h * 0.94],
    );
    p.push([w * 0.32, h]);
    curve(
        &mut p,
        [w * 0.32, h * (1.0 - neck)],
        [w * 0.08, h * (1.0 - neck)],
        [0.0, h * (1.0 - neck)],
    );
    p
}

fn sleeve(w: f64, h: f64) -> Vec<Pt> {
    let mut p = vec![[w * 0.18, 0.0], [w * 0.82, 0.0], [w, h * 0.73]];
    curve(&mut p, [w, h * 0.90], [w * 0.74, h], [w * 0.50, h]);
    curve(&mut p, [w * 0.26, h], [0.0, h * 0.90], [0.0, h * 0.73]);
    p
}

fn leg(w: f64, h: f64, back: bool) -> Vec<Pt> {
    let extra = if back { 0.11 } else { 0.0 };
    let mut p = vec![
        [w * 0.18, 0.0],
        [w * 0.78, 0.0],
        [w * 0.72, h * 0.49],
        [w, h * 0.74],
    ];
    curve(
        &mut p,
        [w * 0.69, h * 0.75],
        [w * (0.67 + extra), h * 0.90],
        [w * (0.65 + extra), h],
    );
    p.push([w * 0.10, h * 0.98]);
    curve(
        &mut p,
        [w * 0.02, h * 0.80],
        [w * 0.12, h * 0.57],
        [w * 0.16, h * 0.45],
    );
    p
}

fn piece(style: &str, name: &str, size: &str, outline: Vec<Pt>, quantity: u32) -> Piece {
    let b = beta_marker_core::geometry::bbox(&outline).expect("draft outline");
    let w = b[2] - b[0];
    let h = b[3] - b[1];
    let marks = vec![
        Mark {
            kind: MarkKind::Notch,
            points: vec![outline[0], [outline[0][0], outline[0][1] + 5.0]],
            text: None,
        },
        Mark {
            kind: MarkKind::Notch,
            points: vec![outline[1]],
            text: None,
        },
    ];
    Piece {
        id: format!("{style}-{name}-{size}"),
        name: name.into(),
        style: Some(style.into()),
        size: Some(size.into()),
        material: Some("Cotton".into()),
        outline,
        grain: Some([
            [b[0] + w * 0.45, b[1] + h * 0.2],
            [b[0] + w * 0.45, b[1] + h * 0.7],
        ]),
        quantity,
        mirror_quantity: 0,
        rotation: RotationRule::Inherit,
        buffer: 0.0,
        marks,
        ..Default::default()
    }
}

/// Original shirt: eight pieces in each of S, M, L and XL.
pub fn shirt() -> Vec<Piece> {
    let mut out = vec![];
    for (i, size) in ["S", "M", "L", "XL"].iter().enumerate() {
        let g = i as f64;
        let mut front = piece(
            "BETA-SHIRT",
            "Front",
            size,
            bodice(270.0 + 15.0 * g, 690.0 + 15.0 * g, 0.13),
            2,
        );
        for p in [[90.0, 390.0], [180.0, 390.0]] {
            front.marks.push(Mark {
                kind: MarkKind::Drill,
                points: vec![p],
                text: None,
            });
        }
        out.push(front);
        out.push(piece(
            "BETA-SHIRT",
            "Back",
            size,
            bodice(510.0 + 28.0 * g, 690.0 + 15.0 * g, 0.035),
            1,
        ));
        out.push(piece(
            "BETA-SHIRT",
            "Yoke",
            size,
            bodice(490.0 + 25.0 * g, 150.0 + 4.0 * g, 0.05),
            2,
        ));
        out.push(piece(
            "BETA-SHIRT",
            "Sleeve",
            size,
            sleeve(410.0 + 12.0 * g, 590.0 + 8.0 * g),
            2,
        ));
        out.push(piece(
            "BETA-SHIRT",
            "Collar",
            size,
            vec![
                [0.0, 0.0],
                [390.0 + 10.0 * g, 0.0],
                [420.0 + 10.0 * g, 75.0],
                [15.0, 60.0],
            ],
            2,
        ));
        out.push(piece(
            "BETA-SHIRT",
            "Collar stand",
            size,
            bodice(400.0 + 10.0 * g, 36.0, 0.04),
            2,
        ));
        out.push(piece(
            "BETA-SHIRT",
            "Cuff",
            size,
            rectangle(235.0 + 6.0 * g, 110.0),
            4,
        ));
        let mut pocket = piece(
            "BETA-SHIRT",
            "Pocket",
            size,
            vec![
                [0.0, 20.0],
                [65.0, 0.0],
                [130.0, 20.0],
                [130.0, 150.0],
                [0.0, 150.0],
            ],
            1,
        );
        pocket.marks.push(Mark {
            kind: MarkKind::Drill,
            points: vec![[65.0, 125.0]],
            text: None,
        });
        out.push(pocket);
    }
    out
}

/// Original trouser: five pieces in each of waist sizes 30, 32, 34 and 36.
pub fn trouser() -> Vec<Piece> {
    let mut out = vec![];
    for (i, size) in ["30", "32", "34", "36"].iter().enumerate() {
        let g = i as f64;
        out.push(piece(
            "BETA-TROUSER",
            "Front",
            size,
            leg(310.0 + 13.0 * g, 1040.0 + 8.0 * g, false),
            2,
        ));
        out.push(piece(
            "BETA-TROUSER",
            "Back",
            size,
            leg(350.0 + 13.0 * g, 1060.0 + 8.0 * g, true),
            2,
        ));
        out.push(piece(
            "BETA-TROUSER",
            "Waistband",
            size,
            rectangle(790.0 + 50.8 * g, 85.0),
            1,
        ));
        out.push(piece(
            "BETA-TROUSER",
            "Pocket bag",
            size,
            bodice(230.0 + 5.0 * g, 300.0, 0.3),
            2,
        ));
        out.push(piece(
            "BETA-TROUSER",
            "Fly",
            size,
            bodice(65.0, 220.0, 0.1),
            1,
        ));
    }
    out
}

/// Original T-shirt: front, back, sleeve and neck band in each of S, M and L.
pub fn tee() -> Vec<Piece> {
    let mut out = vec![];
    for (i, size) in ["S", "M", "L"].iter().enumerate() {
        let g = i as f64;
        out.push(piece(
            "BETA-TEE",
            "Front",
            size,
            bodice(480.0 + 30.0 * g, 650.0 + 20.0 * g, 0.14),
            1,
        ));
        out.push(piece(
            "BETA-TEE",
            "Back",
            size,
            bodice(480.0 + 30.0 * g, 650.0 + 20.0 * g, 0.04),
            1,
        ));
        out.push(piece(
            "BETA-TEE",
            "Sleeve",
            size,
            sleeve(360.0 + 15.0 * g, 240.0 + 10.0 * g),
            2,
        ));
        out.push(piece(
            "BETA-TEE",
            "Neck band",
            size,
            rectangle(410.0 + 15.0 * g, 40.0),
            1,
        ));
    }
    out
}
