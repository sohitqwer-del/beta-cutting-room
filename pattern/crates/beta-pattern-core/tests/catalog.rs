//! Original known-coordinate cases for every catalog construction.
use beta_pattern_core::{
    evaluate::{Evaluator, Geometry},
    geometry::*,
    tools, *,
};
use serde_json::{Value as Json, json};
use std::collections::BTreeSet;
fn apply(d: &Document, tool: &str, id: &str, p: Json) -> Document {
    tools::apply(d, tool, id, p.as_object().unwrap().clone()).unwrap()
}
fn fixture() -> Document {
    let mut d = Document::default();
    for (id, x, y) in [
        ("A", 0., 0.),
        ("B", 6., 0.),
        ("C", 0., 8.),
        ("D", 6., 8.),
        ("E", 10., 0.),
        ("F", 3., -4.),
        ("G", 3., 4.),
        ("H", 2., 0.),
        ("I", 4., 0.),
    ] {
        d = apply(&d, "absolute", id, json!({"x":x,"y":y}));
    }
    d = apply(&d, "line", "horizontal", json!({"from":"A","to":"B"}));
    d = apply(&d, "line", "vertical", json!({"from":"F","to":"G"}));
    for (id, c) in [("circleA", "A"), ("circleB", "B")] {
        d = apply(
            &d,
            "arc",
            id,
            json!({"center":c,"radius":5,"start":0,"sweep":360}),
        );
    }
    apply(
        &d,
        "arc",
        "quarter",
        json!({"center":"A","radius":10,"start":0,"sweep":90}),
    )
}
#[test]
fn every_catalog_tool_has_known_coordinates() {
    let d = fixture();
    let tangent_y = 18.75_f64.sqrt();
    let points = vec![
        ("absolute", json!({"x":1,"y":2}), [1., 2.]),
        (
            "polar",
            json!({"from":"A","distance":4,"angle":90}),
            [0., 4.],
        ),
        (
            "along_line",
            json!({"from":"A","to":"B","distance":3}),
            [3., 0.],
        ),
        ("midpoint", json!({"a":"A","b":"B"}), [3., 0.]),
        ("foot", json!({"point":"G","a":"A","b":"B"}), [3., 0.]),
        ("normal", json!({"a":"A","b":"B","distance":4}), [0., 4.]),
        (
            "bisector",
            json!({"vertex":"A","a":"B","b":"C","distance":2_f64.sqrt()}),
            [1., 1.],
        ),
        (
            "line_intersection",
            json!({"a":["A","B"],"b":["F","G"]}),
            [3., 0.],
        ),
        (
            "intersection",
            json!({"a":"horizontal","b":"vertical"}),
            [3., 0.],
        ),
        (
            "intersect_xy",
            json!({"x_point":"B","y_point":"C"}),
            [6., 8.],
        ),
        (
            "shoulder",
            json!({"center":"A","length":5,"a":"A","b":"B"}),
            [-5., 0.],
        ),
        (
            "triangle",
            json!({"a":"B","b":"C","axis_a":"F","axis_b":"G"}),
            [3., -1.],
        ),
        (
            "line_axis",
            json!({"a":"A","b":"B","origin":"G","angle":90}),
            [3., 0.],
        ),
        (
            "curve_axis",
            json!({"curve":"horizontal","origin":"G","angle":270}),
            [3., 0.],
        ),
        (
            "circle_intersection",
            json!({"a":"A","radius_a":5,"b":"B","radius_b":5}),
            [3., 4.],
        ),
        (
            "arc_intersection",
            json!({"a":"circleA","b":"circleB"}),
            [3., 4.],
        ),
        (
            "circle_line",
            json!({"center":"A","radius":5,"a":"A","b":"B"}),
            [-5., 0.],
        ),
        (
            "arc_line",
            json!({"arc":"circleA","a":"A","b":"B"}),
            [-5., 0.],
        ),
        (
            "circle_tangent",
            json!({"point":"E","center":"A","radius":5}),
            [2.5, tangent_y],
        ),
        (
            "arc_tangent",
            json!({"point":"E","arc":"circleA"}),
            [2.5, tangent_y],
        ),
        (
            "curve_point",
            json!({"curve":"quarter","distance":5.*std::f64::consts::PI}),
            [0., 10.],
        ),
        (
            "true_dart",
            json!({"apex":"A","leg":"B","other":"C"}),
            [8., 0.],
        ),
        (
            "path_endpoint",
            json!({"curve":"quarter","index":1}),
            [0., 10.],
        ),
        ("move", json!({"source":"B","dx":1,"dy":2}), [7., 2.]),
        (
            "rotate",
            json!({"source":"B","pivot":"A","angle":90}),
            [0., 6.],
        ),
        (
            "mirror_line",
            json!({"source":"B","a":"A","b":"C"}),
            [-6., 0.],
        ),
        (
            "mirror_axis",
            json!({"source":"C","origin":"A","axis":"horizontal"}),
            [0., -8.],
        ),
    ];
    let paths = vec![
        ("line", json!({"from":"A","to":"B"}), [0., 0.], [6., 0.]),
        (
            "cubic",
            json!({"points":["A","H","I","B"]}),
            [0., 0.],
            [6., 0.],
        ),
        (
            "spline",
            json!({"points":["A","B","E"]}),
            [0., 0.],
            [10., 0.],
        ),
        (
            "arc",
            json!({"center":"A","radius":10,"start":0,"sweep":90}),
            [10., 0.],
            [0., 10.],
        ),
        (
            "arc_length",
            json!({"center":"A","radius":10,"start":0,"length":5.*std::f64::consts::PI}),
            [10., 0.],
            [0., 10.],
        ),
        (
            "elliptical_arc",
            json!({"center":"A","radius_x":6,"radius_y":8,"rotation":90,"start":0,"sweep":90}),
            [0., 6.],
            [-8., 0.],
        ),
        (
            "cubic_path",
            json!({"segments":[["A","H","I","B"],["B","B","E","E"]]}),
            [0., 0.],
            [10., 0.],
        ),
        (
            "cut_path",
            json!({"curve":"horizontal","start":1,"length":3}),
            [1., 0.],
            [4., 0.],
        ),
    ];
    let mut tested = BTreeSet::new();
    for (tool, params, expected) in points {
        let d = apply(&d, tool, "result", params);
        let q = Evaluator::new(&d, "M", 0.001)
            .unwrap()
            .point("result")
            .unwrap();
        assert!(
            distance(q, expected) < 1e-8,
            "{tool}: {q:?} expected {expected:?}"
        );
        tested.insert(tool.to_string());
    }
    for (tool, params, start, end) in paths {
        let d = apply(&d, tool, "result", params);
        let q = Evaluator::new(&d, "M", 0.001)
            .unwrap()
            .path("result")
            .unwrap();
        assert!(
            distance(q[0], start) < 1e-8 && distance(*q.last().unwrap(), end) < 1e-8,
            "{tool}"
        );
        tested.insert(tool.to_string());
    }
    assert_eq!(tested, tools::catalog().into_iter().map(|s| s.id).collect());
}
#[test]
fn exact_geometry_edge_cases_and_arc_filtering() {
    assert_eq!(
        circle_intersections([0., 0.], 5., [10., 0.], 5.).unwrap(),
        vec![[5., 0.]]
    );
    assert!(
        circle_intersections([0., 0.], 5., [11., 0.], 5.)
            .unwrap()
            .is_empty()
    );
    assert!(circle_intersections([0., 0.], 5., [0., 0.], 5.).is_err());
    assert_eq!(
        circle_line([0., 0.], 5., [-10., 5.], [10., 5.]).unwrap(),
        vec![[0., 5.]]
    );
    assert!(circle_tangents([0., 0.], [0., 0.], 5.).is_err());
    assert!(flatten_ellipse([0., 0.], [1., 0.], 0., 0., 90., 0.1).is_err());
    assert!(angle_in_arc(0., 350., 20.));
    assert!(angle_in_arc(350., 10., -20.));
    assert!(!angle_in_arc(180., 350., 20.));
    let d = apply(
        &fixture(),
        "arc_line",
        "contact",
        json!({"arc":"quarter","a":"A","b":"B","index":0}),
    );
    assert_eq!(
        Evaluator::new(&d, "M", 0.1)
            .unwrap()
            .point("contact")
            .unwrap(),
        [10., 0.]
    );
    let d = apply(
        &d,
        "rotate",
        "rotated_line",
        json!({"source":"horizontal","pivot":"A","angle":90}),
    );
    assert!(
        matches!(Evaluator::new(&d,"M",0.1).unwrap().object("rotated_line").unwrap(),Geometry::Path(p) if distance(p[1],[0.,6.])<1e-8)
    );
}
#[test]
fn group_transforms_and_piece_union_metadata() {
    let d = fixture();
    for (tool, p, expected) in [
        ("move", json!({"dx":2,"dy":3}), [8., 3.]),
        ("rotate", json!({"pivot":"A","angle":90}), [0., 6.]),
        ("mirror_line", json!({"a":"A","b":"C"}), [-6., 0.]),
        (
            "mirror_axis",
            json!({"origin":"A","axis":"vertical"}),
            [-6., 0.],
        ),
    ] {
        let g = tools::transform_group(
            &d,
            &["B".into(), "horizontal".into()],
            "_copy",
            tool,
            p.as_object().unwrap().clone(),
        )
        .unwrap();
        assert!(
            distance(
                Evaluator::new(&g, "M", 0.1)
                    .unwrap()
                    .point("B_copy")
                    .unwrap(),
                expected
            ) < 1e-8
        );
        assert!(
            !tools::set_group_visibility(&g, "_copy", false)
                .unwrap()
                .groups["_copy"]
                .visible
        );
        assert!(tools::delete(&g, "B_copy").is_err());
        assert!(
            tools::transform_group(
                &g,
                &["B".into()],
                "_copy",
                tool,
                p.as_object().unwrap().clone()
            )
            .is_err()
        );
    }
    let mut d = Document::default();
    tools::insert_polygon(
        &mut d,
        "left",
        "Left",
        &[[0., 0.], [10., 0.], [10., 10.], [0., 10.]],
    )
    .unwrap();
    tools::insert_polygon(
        &mut d,
        "right",
        "Right",
        &[[10., 0.], [20., 0.], [20., 10.], [10., 10.]],
    )
    .unwrap();
    d.pieces[0].anchors.insert("hem".into(), "left.p0".into());
    d = tools::set_allowance(&d, "left", 2.0.into(), Corner::Miter).unwrap();
    d = tools::set_allowance(&d, "right", 2.0.into(), Corner::Miter).unwrap();
    d.pieces[0].annotations = vec![
        Annotation::Notch {
            edge: 0,
            distance: 5.0.into(),
            depth: 1.0.into(),
        },
        Annotation::Notch {
            edge: 1,
            distance: 5.0.into(),
            depth: 1.0.into(),
        },
    ];
    d.pieces[1].annotations = vec![Annotation::Button {
        point: "right.p1".into(),
        diameter: 2.0.into(),
    }];
    let u = tools::union_pieces(&d, "left", "right", "joined", "M", 0.01).unwrap();
    let g = beta_pattern_core::evaluate::grade(&u, "M", 0.01).unwrap();
    assert_eq!(area(&g[2].seam), 200.);
    assert_eq!(g[2].piece.marks.len(), 2);
    assert_eq!(g[2].piece.marks[0].points[0], [5., 0.]);
    assert_eq!(area(&g[2].piece.outline), 336.);
    assert_eq!(g[2].piece.match_points[0].point, [0., 0.]);
    assert!(
        tools::union_shared_seam(
            &[[0., 0.], [10., 0.], [10., 10.], [0., 10.]],
            &[[1., 1.], [9., 1.], [9., 9.], [1., 9.]]
        )
        .is_err()
    );
}
#[test]
fn internal_paths_labels_grain_and_four_notch_shapes() {
    let mut d = Document {
        label: Some("Style 10".into()),
        ..Document::default()
    };
    tools::insert_polygon(
        &mut d,
        "piece",
        "Panel",
        &[[0., 0.], [100., 0.], [100., 100.], [0., 100.]],
    )
    .unwrap();
    d = apply(&d, "absolute", "bottom", json!({"x":50,"y":-10}));
    d = apply(&d, "absolute", "top", json!({"x":50,"y":110}));
    d = apply(&d, "line", "guide", json!({"from":"bottom","to":"top"}));
    let p = &mut d.pieces[0];
    p.label_point = Some("piece.p0".into());
    p.material = Some("cotton".into());
    p.grain_angle = Some(GrainAngle {
        origin: "piece.p0".into(),
        angle: 90.0.into(),
        length: 80.0.into(),
    });
    p.internal_paths.push(InternalPath {
        name: "hem fold".into(),
        edges: vec![p.edges[0].clone()],
        closed: false,
    });
    for (i, shape) in [
        NotchType::Slit,
        NotchType::V,
        NotchType::T,
        NotchType::Castle,
    ]
    .into_iter()
    .enumerate()
    {
        p.annotations.push(Annotation::ShapedNotch {
            edge: 0,
            distance: ((i + 1) as f64 * 10.).into(),
            depth: 3.0.into(),
            width: 4.0.into(),
            shape,
        });
    }
    p.annotations.push(Annotation::SeamIntersections {
        path: "guide".into(),
        depth: 3.0.into(),
        width: 4.0.into(),
        shape: NotchType::V,
    });
    let g = beta_pattern_core::evaluate::grade(&d, "M", 0.01).unwrap();
    let p = &g[0].piece;
    assert_eq!(p.marks[0].points, vec![[10., 0.], [10., 3.]]);
    assert_eq!(p.marks[1].points, vec![[18., 0.], [20., 3.], [22., 0.]]);
    assert_eq!(
        p.marks[2].points,
        vec![[30., 0.], [30., 3.], [28., 3.], [32., 3.]]
    );
    assert_eq!(
        p.marks[3].points,
        vec![[38., 0.], [38., 3.], [42., 3.], [42., 0.]]
    );
    assert_eq!(p.marks.len(), 8);
    assert!(
        p.marks
            .last()
            .unwrap()
            .text
            .as_ref()
            .unwrap()
            .contains("Style 10")
    );
    assert!(distance(p.grain.unwrap()[1], [0., 80.]) < 1e-8);
}
