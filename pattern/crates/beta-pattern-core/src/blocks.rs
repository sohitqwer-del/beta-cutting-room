//! Original BETA measurement-driven starting drafts. No vendor drafting code or data.
use crate::{
    model::*,
    production::{SeamCheck, SeamReport, SeamRun},
    style::{self, SizeChart},
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Supported original basic blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum BlockKind {
    TShirt,
    Shirt,
    Trouser,
    ALineSkirt,
    Kurta,
    Polo,
    TrackPant,
}
/// User-adjustable ease and allowances, in mm.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[allow(missing_docs)]
pub struct Ease {
    pub chest: Option<f64>,
    pub waist: Option<f64>,
    pub hip: Option<f64>,
    pub sleeve_cap: Option<f64>,
    pub seam_allowance: Option<f64>,
    pub hem_allowance: Option<f64>,
}
/// A blank document or a parametric block request.
#[derive(Debug, Clone, Default, Deserialize)]
#[allow(missing_docs)]
pub struct Create {
    pub name: Option<String>,
    pub block: Option<BlockKind>,
    pub chart: Option<SizeChart>,
    #[serde(default)]
    pub ease: Ease,
}
/// Display information for the block picker.
#[derive(Debug, Clone, Serialize)]
#[allow(missing_docs)]
pub struct BlockInfo {
    pub id: BlockKind,
    pub name: &'static str,
    pub pieces: Vec<&'static str>,
    pub note: &'static str,
}
/// Available blocks; every listed piece is generated, with editable formulas.
pub fn catalog() -> Vec<BlockInfo> {
    use BlockKind::*;
    [
        (TShirt,"T-shirt",vec!["front","back","sleeve","neck_rib"]),
        (Shirt,"Shirt",vec!["front","back","yoke","sleeve","cuff","collar","stand","placket"]),
        (Trouser,"Trouser",vec!["front","back","waistband","pocket_bag"]),
        (ALineSkirt,"A-line skirt",vec!["front","back","waistband"]),
        (Kurta,"Kurta / tunic",vec!["front","back","sleeve","neck_facing","placket"]),
        (Polo,"Polo",vec!["front","back","sleeve","collar","placket"]),
        (TrackPant,"Track pant",vec!["front","back","waistband","pocket_bag"]),
    ].into_iter().map(|(id,name,pieces)| BlockInfo {id,name,pieces,note:"Original BETA starting draft. Review fit, fabric stretch and sewing details before production."}).collect()
}
/// Original editable example charts. Neither preset claims certified ISO sizing.
pub fn size_chart(preset: &str) -> Result<SizeChart> {
    ensure!(
        matches!(preset, "indian_adult" | "iso_example"),
        "Choose indian_adult or iso_example as the chart preset."
    );
    let mut measurements = BTreeMap::new();
    for (i, size) in ["S", "M", "L", "XL", "XXL"].into_iter().enumerate() {
        let n = i as f64;
        let chest = if preset == "indian_adult" {
            920. + 60. * n
        } else {
            880. + 80. * n
        };
        measurements.insert(
            size.into(),
            [
                ("chest", chest),
                ("waist", chest - 140.),
                ("hip", chest + 40.),
                ("neck", 360. + 15. * n),
                ("shoulder", 125. + 5. * n),
                ("arm_depth", 205. + 10. * n),
                ("body_length", 650. + 15. * n),
                ("sleeve_length", 580. + 10. * n),
                ("short_sleeve", 215. + 5. * n),
                ("wrist", 170. + 5. * n),
                ("bicep", 300. + 15. * n),
                ("trouser_length", 990. + 15. * n),
                ("inseam", 735. + 10. * n),
                ("ankle", 340. + 10. * n),
                ("skirt_length", 600. + 10. * n),
                ("tunic_length", 930. + 20. * n),
            ]
            .into_iter()
            .map(|(k, v)| (k.into(), v))
            .collect(),
        );
    }
    Ok(SizeChart {
        name: if preset == "indian_adult" {
            "BETA Indian adult example"
        } else {
            "BETA ISO-style measurement example"
        }
        .into(),
        sizes: ["S", "M", "L", "XL", "XXL"].map(String::from).to_vec(),
        base_size: "M".into(),
        measurements,
    })
}
fn point(doc: &mut Document, id: &str, x: &str, y: &str) {
    doc.drawing.insert(
        id.into(),
        Object::Point {
            operation: PointOp::Absolute {
                x: x.into(),
                y: y.into(),
            },
        },
    );
}
fn line(doc: &mut Document, id: &str, a: &str, b: &str) {
    doc.drawing.insert(
        id.into(),
        Object::Line {
            from: a.into(),
            to: b.into(),
        },
    );
}
fn cubic(doc: &mut Document, id: &str, points: [&str; 4]) {
    doc.drawing.insert(
        id.into(),
        Object::Cubic {
            points: points.map(String::from),
        },
    );
}
fn edge(id: &str, allowance: &str, reversed: bool) -> Edge {
    Edge {
        object: id.into(),
        allowance: allowance.into(),
        reversed,
        corner: Corner::Miter,
    }
}
fn piece(id: &str, name: &str, edges: Vec<Edge>) -> PatternPiece {
    PatternPiece {
        id: id.into(),
        name: name.into(),
        edges,
        quantity: 1,
        mirror_quantity: 0,
        material: Some("shell".into()),
        grain: None,
        fold_line: None,
        rotation: Default::default(),
        buffer: 0.0.into(),
        annotations: vec![],
        grade_points: BTreeMap::new(),
        internal_paths: vec![],
        anchors: BTreeMap::new(),
        grain_angle: None,
        label_point: None,
    }
}
fn reflect(doc: &mut Document, id: &str, source: &str, origin: &str) {
    doc.drawing.insert(
        id.into(),
        Object::MirrorAxis {
            source: source.into(),
            origin: origin.into(),
            axis: Axis::Vertical,
        },
    );
}
fn variable(doc: &mut Document, name: &str, formula: &str) {
    doc.variables.insert(name.into(), formula.into());
}
fn check(name: &str, a: &str, ae: &[usize], b: &str, be: &[usize], ease: f64) -> SeamCheck {
    SeamCheck {
        name: name.into(),
        a: SeamRun {
            piece: a.into(),
            edges: ae.to_vec(),
            reversed: false,
        },
        b: SeamRun {
            piece: b.into(),
            edges: be.to_vec(),
            reversed: true,
        },
        ease_mm: ease,
        tolerance_mm: 1.,
    }
}
fn rectangle(doc: &mut Document, id: &str, name: &str, width: &str, height: &str, quantity: u32) {
    for (suffix, x, y) in [
        ("A", "0", "0"),
        ("B", width, "0"),
        ("C", width, height),
        ("D", "0", height),
    ] {
        point(doc, &format!("{id}.{suffix}"), x, y);
    }
    let mut edges = vec![];
    for (i, a, b) in [(0, "A", "B"), (1, "B", "C"), (2, "C", "D"), (3, "D", "A")] {
        let key = format!("{id}.e{i}");
        line(doc, &key, &format!("{id}.{a}"), &format!("{id}.{b}"));
        edges.push(edge(&key, "v.seam_allowance", false));
    }
    point(
        doc,
        &format!("{id}.grain0"),
        &format!("({width})/2"),
        &format!("({height})*0.2"),
    );
    point(
        doc,
        &format!("{id}.grain1"),
        &format!("({width})/2"),
        &format!("({height})*0.8"),
    );
    let mut p = piece(id, name, edges);
    p.quantity = quantity;
    p.grain = Some([format!("{id}.grain0"), format!("{id}.grain1")]);
    p.label_point = Some(format!("{id}.grain0"));
    doc.pieces.push(p);
}
fn body(doc: &mut Document, id: &str, front: bool, shirt: bool) {
    let n = |s: &str| format!("{id}.{s}");
    let neck_depth = if front { "v.neck_width+10" } else { "20" };
    for (suffix, x, y) in [
        ("A", "0", "0"),
        ("B", "v.body_hem", "0"),
        ("C", "v.body_width", "v.body_length-m.arm_depth"),
        (
            "D",
            "v.neck_width+sqrt(m.shoulder^2-v.shoulder_drop^2)",
            "v.body_length-v.shoulder_drop",
        ),
        ("E", "v.neck_width", "v.body_length"),
        ("F", "0", &format!("v.body_length-({neck_depth})")),
        ("arm1", "v.body_width-20", "v.body_length-m.arm_depth+30"),
        (
            "arm2",
            "v.neck_width+sqrt(m.shoulder^2-v.shoulder_drop^2)-15",
            "v.body_length-v.shoulder_drop-40",
        ),
        (
            "neck1",
            "v.neck_width",
            &format!("v.body_length-({neck_depth})*0.9"),
        ),
        (
            "neck2",
            "v.neck_width*0.45",
            &format!("v.body_length-({neck_depth})"),
        ),
        ("grain0", "v.body_width/2", "v.body_length*0.2"),
        ("grain1", "v.body_width/2", "v.body_length*0.65"),
    ] {
        point(doc, &n(suffix), x, y);
    }
    for (s, a, b) in [
        ("hem", "A", "B"),
        ("side", "B", "C"),
        ("shoulder", "D", "E"),
        ("center", "F", "A"),
    ] {
        line(doc, &n(s), &n(a), &n(b));
    }
    cubic(doc, &n("arm"), [&n("C"), &n("arm1"), &n("arm2"), &n("D")]);
    cubic(
        doc,
        &n("neck"),
        [&n("E"), &n("neck1"), &n("neck2"), &n("F")],
    );
    let mut edges = vec![
        edge(&n("hem"), "v.hem_allowance", false),
        edge(&n("side"), "v.seam_allowance", false),
        edge(&n("arm"), "v.seam_allowance", false),
        edge(&n("shoulder"), "v.seam_allowance", false),
        edge(&n("neck"), "v.seam_allowance", false),
    ];
    if shirt && front {
        edges.push(edge(&n("center"), "v.seam_allowance", false));
    } else {
        for s in ["neck", "shoulder", "arm", "side", "hem"] {
            reflect(doc, &n(&format!("mirror_{s}")), &n(s), &n("A"));
            edges.push(edge(
                &n(&format!("mirror_{s}")),
                if s == "hem" {
                    "v.hem_allowance"
                } else {
                    "v.seam_allowance"
                },
                true,
            ));
        }
    }
    let mut p = piece(id, if front { "Front" } else { "Back" }, edges);
    p.grain = Some([n("grain0"), n("grain1")]);
    p.label_point = Some(n("grain0"));
    p.anchors = BTreeMap::from([
        ("side_hem".into(), n("B")),
        ("underarm".into(), n("C")),
        ("shoulder".into(), n("D")),
    ]);
    if shirt && front {
        p.mirror_quantity = 1;
    } else {
        p.fold_line = Some([n("A"), n("F")]);
    }
    p.annotations.push(Annotation::Notch {
        edge: 1,
        distance: "length(front.side)/2".into(),
        depth: 3.0.into(),
    });
    if p.fold_line.is_some() {
        p.annotations.push(Annotation::Notch {
            edge: 8,
            distance: "length(front.side)/2".into(),
            depth: 3.0.into(),
        });
    }
    doc.pieces.push(p);
}
fn shirt_yoke(doc: &mut Document) -> Result<()> {
    // A straight yoke seam meets the shoulder/armhole junction. Both parts stay parametric.
    reflect(doc, "back.mirror_D", "back.D", "back.A");
    line(doc, "back.yoke_join", "back.D", "back.mirror_D");
    let p = doc
        .pieces
        .iter_mut()
        .find(|p| p.id == "back")
        .context("Back block missing")?;
    let all = p.edges.clone();
    p.edges = vec![
        all[0].clone(),
        all[1].clone(),
        all[2].clone(),
        edge("back.yoke_join", "v.seam_allowance", false),
        all[7].clone(),
        all[8].clone(),
        all[9].clone(),
    ];
    for annotation in &mut p.annotations {
        if let Annotation::Notch { edge, .. } = annotation
            && *edge == 8
        {
            *edge = 5;
        }
    }
    p.fold_line = Some(["back.A".into(), "back.F".into()]);
    point(doc, "yoke.grain0", "0", "v.body_length-v.shoulder_drop+5");
    point(doc, "yoke.grain1", "0", "v.body_length-21");
    let mut y = piece(
        "yoke",
        "Yoke",
        vec![
            all[3].clone(),
            all[4].clone(),
            all[5].clone(),
            all[6].clone(),
            edge("back.yoke_join", "v.seam_allowance", true),
        ],
    );
    y.quantity = 2;
    y.grain = Some(["yoke.grain0".into(), "yoke.grain1".into()]);
    y.label_point = Some("yoke.grain0".into());
    y.fold_line = Some(["back.A".into(), "back.F".into()]);
    doc.pieces.push(y);
    Ok(())
}
fn sleeve(doc: &mut Document, long: bool, cuffed: bool) {
    // Normalize a two-control cap profile to the measured body armhole length.
    for (s, x, y) in [
        ("T0", "-m.bicep/2", "0"),
        ("T1", "-m.bicep*0.45", "m.arm_depth*0.2"),
        ("T2", "-m.bicep*0.2", "m.arm_depth*0.4"),
        ("T3", "0", "m.arm_depth*0.4"),
    ] {
        point(doc, &format!("sleeve.{s}"), x, y);
    }
    cubic(
        doc,
        "sleeve.template",
        ["sleeve.T0", "sleeve.T1", "sleeve.T2", "sleeve.T3"],
    );
    variable(
        doc,
        "sleeve_scale",
        "(length(front.arm)+length(back.arm)+v.cap_ease)/(2*length(sleeve.template))",
    );
    variable(
        doc,
        "sleeve_length",
        if long {
            "m.sleeve_length"
        } else {
            "m.short_sleeve"
        },
    );
    variable(
        doc,
        "cuff_width",
        if long { "m.wrist+60" } else { "m.bicep+40" },
    );
    for (s, x, y) in [
        ("A", "-v.cuff_width/2", "0"),
        ("B", "v.cuff_width/2", "0"),
        (
            "C",
            "m.bicep*v.sleeve_scale/2",
            "v.sleeve_length-m.arm_depth*0.4*v.sleeve_scale",
        ),
        (
            "cap1",
            "m.bicep*0.45*v.sleeve_scale",
            "v.sleeve_length-m.arm_depth*0.2*v.sleeve_scale",
        ),
        ("cap2", "m.bicep*0.2*v.sleeve_scale", "v.sleeve_length"),
        ("D", "0", "v.sleeve_length"),
        ("grain0", "0", "20"),
        ("grain1", "0", "v.sleeve_length-20"),
    ] {
        point(doc, &format!("sleeve.{s}"), x, y);
    }
    line(doc, "sleeve.hem", "sleeve.A", "sleeve.B");
    line(doc, "sleeve.side", "sleeve.B", "sleeve.C");
    cubic(
        doc,
        "sleeve.cap",
        ["sleeve.C", "sleeve.cap1", "sleeve.cap2", "sleeve.D"],
    );
    reflect(doc, "sleeve.mirror_cap", "sleeve.cap", "sleeve.D");
    reflect(doc, "sleeve.mirror_side", "sleeve.side", "sleeve.D");
    let mut p = piece(
        "sleeve",
        "Sleeve",
        vec![
            edge(
                "sleeve.hem",
                if cuffed {
                    "v.seam_allowance"
                } else {
                    "v.hem_allowance"
                },
                false,
            ),
            edge("sleeve.side", "v.seam_allowance", false),
            edge("sleeve.cap", "v.seam_allowance", false),
            edge("sleeve.mirror_cap", "v.seam_allowance", true),
            edge("sleeve.mirror_side", "v.seam_allowance", true),
        ],
    );
    p.mirror_quantity = 1;
    p.grain = Some(["sleeve.grain0".into(), "sleeve.grain1".into()]);
    p.label_point = Some("sleeve.grain0".into());
    p.annotations.push(Annotation::Notch {
        edge: 2,
        distance: "length(sleeve.cap)".into(),
        depth: 3.0.into(),
    });
    doc.pieces.push(p);
}
fn lower(doc: &mut Document, track: bool, skirt: bool) {
    if skirt {
        variable(doc, "lower_length", "m.skirt_length");
        for id in ["front", "back"] {
            for (s, x, y) in [
                ("A", "-v.waist_quarter", "v.lower_length"),
                ("B", "v.waist_quarter", "v.lower_length"),
                ("C", "v.hip_quarter", "v.lower_length-200"),
                ("D", "v.hip_quarter+60", "0"),
                ("E", "-v.hip_quarter-60", "0"),
                ("F", "-v.hip_quarter", "v.lower_length-200"),
            ] {
                point(doc, &format!("{id}.{s}"), x, y);
            }
            let mut edges = vec![];
            for (i, a, b) in [
                (0, "A", "B"),
                (1, "B", "C"),
                (2, "C", "D"),
                (3, "D", "E"),
                (4, "E", "F"),
                (5, "F", "A"),
            ] {
                let key = format!("{id}.e{i}");
                line(doc, &key, &format!("{id}.{a}"), &format!("{id}.{b}"));
                edges.push(edge(
                    &key,
                    if i == 3 {
                        "v.hem_allowance"
                    } else {
                        "v.seam_allowance"
                    },
                    false,
                ));
            }
            point(doc, &format!("{id}.grain0"), "0", "20");
            point(doc, &format!("{id}.grain1"), "0", "v.lower_length-20");
            let mut p = piece(id, if id == "front" { "Front" } else { "Back" }, edges);
            p.grain = Some([format!("{id}.grain0"), format!("{id}.grain1")]);
            p.fold_line = p.grain.clone();
            p.label_point = Some(format!("{id}.grain0"));
            doc.pieces.push(p);
        }
    } else {
        variable(doc, "lower_length", "m.trouser_length");
        variable(doc, "rise", "m.trouser_length-m.inseam");
        variable(doc, "leg_width", "(m.ankle+80)/2");
        if track {
            variable(doc, "waist_quarter", "v.hip_quarter");
        }
        for id in ["front", "back"] {
            for (s, x, y) in [
                ("A", "(v.hip_quarter-v.leg_width)/2", "0"),
                ("B", "(v.hip_quarter+v.leg_width)/2", "0"),
                ("C", "v.hip_quarter", "m.inseam"),
                ("D", "v.hip_quarter", "v.lower_length"),
                ("E", "v.hip_quarter-v.waist_quarter", "v.lower_length"),
                ("F", "0", "m.inseam"),
                (
                    "crotch1",
                    if id == "front" {
                        "v.hip_quarter-v.waist_quarter"
                    } else {
                        "v.hip_quarter-v.waist_quarter-15"
                    },
                    "m.inseam+v.rise*0.3",
                ),
                (
                    "crotch2",
                    if id == "front" { "-25" } else { "-40" },
                    "m.inseam+v.rise*0.05",
                ),
                ("grain0", "v.hip_quarter/2", "20"),
                ("grain1", "v.hip_quarter/2", "m.inseam"),
            ] {
                point(doc, &format!("{id}.{s}"), x, y);
            }
            let mut edges = vec![];
            for (i, a, b) in [
                (0, "A", "B"),
                (1, "B", "C"),
                (2, "C", "D"),
                (3, "D", "E"),
                (5, "F", "A"),
            ] {
                let key = format!("{id}.e{i}");
                line(doc, &key, &format!("{id}.{a}"), &format!("{id}.{b}"));
            }
            cubic(
                doc,
                &format!("{id}.e4"),
                [
                    &format!("{id}.E"),
                    &format!("{id}.crotch1"),
                    &format!("{id}.crotch2"),
                    &format!("{id}.F"),
                ],
            );
            for i in 0..6 {
                edges.push(edge(
                    &format!("{id}.e{i}"),
                    if i == 0 {
                        "v.hem_allowance"
                    } else {
                        "v.seam_allowance"
                    },
                    false,
                ));
            }
            let mut p = piece(
                id,
                if id == "front" {
                    "Front leg"
                } else {
                    "Back leg"
                },
                edges,
            );
            p.mirror_quantity = 1;
            p.grain = Some([format!("{id}.grain0"), format!("{id}.grain1")]);
            p.label_point = Some(format!("{id}.grain0"));
            p.anchors.insert("hip".into(), format!("{id}.C"));
            doc.pieces.push(p);
        }
        rectangle(doc, "pocket_bag", "Pocket bag", "m.hip/6", "m.hip/4", 2);
        if let Some(p) = doc.pieces.last_mut() {
            p.material = Some("lining".into());
            p.mirror_quantity = 2;
        }
    }
    rectangle(
        doc,
        "waistband",
        if track { "Elastic casing" } else { "Waistband" },
        "4*v.waist_quarter",
        "80",
        1,
    );
}
/// Create a complete editable draft, then validate every size.
pub fn create(request: &Create) -> Result<Document> {
    let chart = request.chart.clone().unwrap_or(size_chart("indian_adult")?);
    chart.validate()?;
    let mut doc = Document {
        name: request
            .name
            .clone()
            .unwrap_or_else(|| "Untitled style".into()),
        sizes: chart.sizes,
        base_size: chart.base_size,
        measurements: chart.measurements,
        ..Document::default()
    };
    let Some(kind) = request.block else {
        style::validate(&doc)?;
        return Ok(doc);
    };
    let info = catalog()
        .into_iter()
        .find(|b| b.id == kind)
        .context("Unknown block")?;
    if request.name.is_none() {
        doc.name = format!("BETA {}", info.name);
    }
    let e = &request.ease;
    for (name, value) in [
        ("ease_chest", e.chest.unwrap_or(80.)),
        ("ease_waist", e.waist.unwrap_or(40.)),
        ("ease_hip", e.hip.unwrap_or(60.)),
        ("cap_ease", e.sleeve_cap.unwrap_or(0.)),
        ("seam_allowance", e.seam_allowance.unwrap_or(10.)),
        ("hem_allowance", e.hem_allowance.unwrap_or(20.)),
    ] {
        ensure!(
            value.is_finite() && (0. ..=500.).contains(&value),
            "Ease and allowances must be finite, from 0 to 500 mm ({name})."
        );
        doc.variables.insert(name.into(), value.into());
    }
    let mut checks = vec![];
    if matches!(
        kind,
        BlockKind::Trouser | BlockKind::TrackPant | BlockKind::ALineSkirt
    ) {
        variable(&mut doc, "hip_quarter", "(m.hip+v.ease_hip)/4");
        variable(&mut doc, "waist_quarter", "(m.waist+v.ease_waist)/4");
        let skirt = kind == BlockKind::ALineSkirt;
        lower(&mut doc, kind == BlockKind::TrackPant, skirt);
        checks.push(check("Side seams", "front", &[1, 2], "back", &[1, 2], 0.));
        if !skirt {
            checks.push(check("Inseams", "front", &[5], "back", &[5], 0.));
        }
    } else {
        for (name, value) in [
            ("body_width", "(m.chest+v.ease_chest)/4"),
            ("body_hem", "v.body_width"),
            ("body_length", "m.body_length"),
            ("neck_width", "m.neck/6"),
            ("shoulder_drop", "30"),
        ] {
            variable(&mut doc, name, value);
        }
        let shirt = kind == BlockKind::Shirt;
        if kind == BlockKind::Kurta {
            variable(&mut doc, "body_length", "m.tunic_length");
            variable(
                &mut doc,
                "body_hem",
                "max(v.body_width,(m.hip+v.ease_hip)/4)+20",
            );
        }
        if shirt {
            variable(&mut doc, "shoulder_drop", "45");
        }
        body(&mut doc, "front", true, shirt);
        body(&mut doc, "back", false, shirt);
        if shirt {
            shirt_yoke(&mut doc)?;
        }
        sleeve(&mut doc, shirt || kind == BlockKind::Kurta, shirt);
        variable(
            &mut doc,
            "neck_length",
            "2*(length(front.neck)+length(back.neck))",
        );
        match kind {
            BlockKind::TShirt => rectangle(
                &mut doc,
                "neck_rib",
                "Neck rib (15% stretch)",
                "v.neck_length*0.85",
                "40",
                1,
            ),
            BlockKind::Shirt => {
                rectangle(&mut doc, "cuff", "Cuff", "v.cuff_width", "100", 2);
                rectangle(&mut doc, "collar", "Collar", "v.neck_length", "65", 2);
                rectangle(&mut doc, "stand", "Collar stand", "v.neck_length", "35", 2);
                rectangle(
                    &mut doc,
                    "placket",
                    "Front placket",
                    "30",
                    "length(front.center)",
                    2,
                );
                checks.push(check("Back to yoke", "back", &[3], "yoke", &[4], 0.));
                checks.push(check("Cuff to sleeve", "cuff", &[0], "sleeve", &[0], 0.));
                checks.push(check("Collar to stand", "collar", &[0], "stand", &[0], 0.));
            }
            BlockKind::Polo => {
                rectangle(&mut doc, "collar", "Polo collar", "v.neck_length", "70", 2);
                rectangle(&mut doc, "placket", "Polo placket", "30", "180", 2);
            }
            BlockKind::Kurta => {
                rectangle(
                    &mut doc,
                    "neck_facing",
                    "Neck binding",
                    "v.neck_length",
                    "35",
                    1,
                );
                rectangle(&mut doc, "placket", "Kurta placket", "30", "250", 2);
            }
            _ => {}
        }
        checks.push(check("Side seams", "front", &[1], "back", &[1], 0.));
        checks.push(check(
            "Shoulders",
            "front",
            &[3],
            if shirt { "yoke" } else { "back" },
            if shirt { &[0] } else { &[3] },
            0.,
        ));
        checks.push(check(
            "Front sleeve cap",
            "sleeve",
            &[2],
            "front",
            &[2],
            e.sleeve_cap.unwrap_or(0.) / 2.,
        ));
        checks.push(check(
            "Back sleeve cap",
            "sleeve",
            &[3],
            "back",
            &[2],
            e.sleeve_cap.unwrap_or(0.) / 2.,
        ));
        checks.push(check("Sleeve underarm", "sleeve", &[1], "sleeve", &[4], 0.));
    }
    doc.meta.insert("block".into(), serde_json::to_value(kind)?);
    let mut checks = serde_json::to_value(checks)?;
    if let Some(checks) = checks.as_array_mut() {
        for check in checks {
            if check["name"]
                .as_str()
                .is_some_and(|s| s.ends_with("sleeve cap"))
            {
                check["ease_formula"] = serde_json::json!("v.cap_ease/2");
            }
        }
    }
    doc.meta.insert("block_seams".into(), checks);
    doc.meta
        .insert("block_note".into(), serde_json::json!(info.note));
    arrange(&mut doc)?;
    style::validate(&doc)?;
    Ok(doc)
}
/// Recompute the block's sewing checks after edits; mismatches are returned, not hidden.
pub fn seam_reports(doc: &Document) -> Result<Vec<SeamReport>> {
    let checks: Vec<serde_json::Value> = serde_json::from_value(
        doc.meta
            .get("block_seams")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
    )?;
    checks
        .iter()
        .map(|raw| {
            let mut check: SeamCheck = serde_json::from_value(raw.clone())?;
            let formula = raw
                .get("ease_formula")
                .map(|v| serde_json::from_value::<Value>(v.clone()))
                .transpose()?;
            if let Some(formula) = &formula {
                check.ease_mm =
                    crate::evaluate::Evaluator::new(doc, &doc.base_size, 0.05)?.value(formula)?;
            }
            let mut report = crate::production::check_seams(doc, &check, 0.05)?;
            if let Some(formula) = formula {
                for row in &mut report.rows {
                    row.required_ease_mm =
                        crate::evaluate::Evaluator::new(doc, &row.size, 0.05)?.value(&formula)?;
                    row.error_mm = row.difference_mm - row.required_ease_mm;
                    row.pass = row.error_mm.abs() <= check.tolerance_mm + 1e-9;
                }
            }
            Ok(report)
        })
        .collect()
}

/// Lay out generated blocks with 50 mm clear space across the whole size chart.
/// Coordinates remain formulas; the size nest is still a separate aligned view.
fn arrange(doc: &mut Document) -> Result<()> {
    // The shirt yoke originally shares back paths. Give it its own editable drawing.
    if let Some(index) = doc.pieces.iter().position(|p| p.id == "yoke") {
        let drawing = doc.drawing.clone();
        for (id, object) in drawing.iter().filter(|(id, _)| id.starts_with("back.")) {
            let encoded = serde_json::to_string(object)?.replace("back.", "yoke.source.");
            doc.drawing.insert(
                id.replacen("back.", "yoke.source.", 1),
                serde_json::from_str(&encoded)?,
            );
        }
        let encoded = serde_json::to_string(&doc.pieces[index])?.replace("back.", "yoke.source.");
        doc.pieces[index] = serde_json::from_str(&encoded)?;
    }
    for piece in &doc.pieces {
        let prefix = format!("{}.", piece.id);
        for (id, object) in &mut doc.drawing {
            if id.starts_with(&prefix)
                && let Object::Point {
                    operation: PointOp::Absolute { x, y },
                } = object
            {
                let text = |v: &Value| match v {
                    Value::Number(n) => n.to_string(),
                    Value::Formula(s) => s.clone(),
                };
                *x = Value::Formula(format!("({})+v.layout_{}_x", text(x), piece.id));
                *y = Value::Formula(format!("({})+v.layout_{}_y", text(y), piece.id));
            }
        }
    }
    doc.meta
        .insert("block_layout".into(), serde_json::json!(true));
    refresh_layout(doc)
}
/// Recompute layout offsets after a chart change, keeping all sizes in separate columns.
pub fn refresh_layout(doc: &mut Document) -> Result<()> {
    if doc.meta.get("block_layout") != Some(&serde_json::json!(true)) {
        return Ok(());
    }
    // Only original block pieces own layout variables; derivatives have their own placement.
    let ids: Vec<_> = catalog()
        .into_iter()
        .find(|b| serde_json::to_value(b.id).ok().as_ref() == doc.meta.get("block"))
        .map(|b| b.pieces.into_iter().map(str::to_string).collect())
        .unwrap_or_default();
    for id in &ids {
        for axis in ["x", "y"] {
            doc.variables
                .insert(format!("layout_{id}_{axis}"), 0.0.into());
        }
    }
    let mut bounds = std::collections::BTreeMap::<String, [f64; 4]>::new();
    for size in &doc.sizes {
        let mut evaluator = crate::evaluate::Evaluator::new(doc, size, style::TOLERANCE)?;
        for piece in &doc.pieces {
            if !ids.contains(&piece.id) {
                continue;
            }
            let g = evaluator.derived_source(&piece.id)?;
            let b = crate::geometry::bbox(&g.piece.outline).context("Empty block")?;
            bounds
                .entry(piece.id.clone())
                .and_modify(|a| {
                    a[0] = a[0].min(b[0]);
                    a[1] = a[1].min(b[1]);
                    a[2] = a[2].max(b[2]);
                    a[3] = a[3].max(b[3]);
                })
                .or_insert(b);
        }
    }
    let mut cursor = 0.;
    for piece in &doc.pieces {
        if let Some(b) = bounds.get(&piece.id) {
            doc.variables
                .insert(format!("layout_{}_x", piece.id), (cursor - b[0]).into());
            doc.variables
                .insert(format!("layout_{}_y", piece.id), (-b[1]).into());
            cursor += b[2] - b[0] + 50.;
        }
    }
    Ok(())
}
