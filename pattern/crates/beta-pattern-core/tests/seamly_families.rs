//! Own XML fixtures test independent format exploration, not general compatibility claims.
use beta_pattern_core::{evaluate::Evaluator, geometry::*, seamly};
#[test]
fn imported_families_have_known_coordinates_and_native_roundtrip() {
    let imported = seamly::import(include_str!("fixtures/families.sm2d"), None, "M").unwrap();
    let doc: beta_pattern_core::Document =
        serde_json::from_str(&serde_json::to_string(&imported.document).unwrap()).unwrap();
    let mut e = Evaluator::new(&doc, "M", 0.001).unwrap();
    for (id, p) in [
        ("shoulder", [-5., 0.]),
        ("height", [3., 0.]),
        ("triangle", [3., -1.]),
        ("lineAxis", [3., 0.]),
        ("curveAxis", [3., 0.]),
        ("arcIntersection", [3., 4.]),
        ("circleIntersection", [3., 4.]),
        ("circleTangent", [2.5, 18.75_f64.sqrt()]),
        ("arcTangent", [2.5, 18.75_f64.sqrt()]),
        ("arcLine", [-5., 0.]),
        ("circleLine", [-5., 0.]),
        ("xy", [6., 8.]),
        ("curveIntersection", [3., 0.]),
        ("cutArc", [0., 10.]),
        ("cutSpline", [3., 0.]),
        ("cutSplinePath", [8., 0.]),
        ("trueDart", [8., 0.]),
        ("rotated", [0., 6.]),
        ("moved", [7., 2.]),
        ("mirroredLine", [-6., 0.]),
        ("mirroredAxis", [0., -8.]),
    ] {
        let q = e.point(id).unwrap();
        assert!(distance(q, p) < 1e-8, "{id}: {q:?} expected {p:?}");
    }
    for (id, p) in [
        ("lengthArc", [0., 10.]),
        ("ellipse", [-8., 0.]),
        ("cubic", [6., 0.]),
        ("interactive", [10., 0.]),
        ("bezierPath", [6., 0.]),
        ("splinePath", [10., 0.]),
    ] {
        assert!(
            distance(*e.path(id).unwrap().last().unwrap(), p) < 1e-8,
            "{id}"
        );
    }
    assert!(!doc.groups["g1"].visible);
}
#[test]
fn unsupported_geometry_fails_with_specific_type() {
    let xml = include_str!("fixtures/families.sm2d");
    for (old, new, expected) in [
        ("type=\"shoulder\"", "type=\"newShoulder\"", "newShoulder"),
        (
            "type=\"rotation\"",
            "type=\"unknownOperation\"",
            "unknownOperation",
        ),
        (
            "</calculation>",
            "<unknownCurve/></calculation>",
            "unknownCurve",
        ),
    ] {
        let error = format!(
            "{:#}",
            seamly::import(&xml.replace(old, new), None, "M").unwrap_err()
        );
        assert!(error.contains(expected), "{error}");
    }
}
#[test]
fn circular_detail_and_internal_paths_remain_parametric() {
    let xml = "<pattern><unit>mm</unit><draw><calculation><point id='1' name='O' type='single' x='0' y='0'/><arc id='2' name='circle' center='1' radius='50' angle1='0' angle2='360'/><point id='3' name='Q' type='single' x='10' y='0'/><line id='4' name='guide' firstPoint='1' secondPoint='3'/></calculation><details><detail id='1' name='Circle' labelPoint='1' fabric='cotton' quantity='2'><nodes><node idObject='2' type='NodeArc'/></nodes><internalPath name='guide'><node idObject='4'/></internalPath><anchor name='centre' idObject='1'/><grainline center='1' angle='90' length='20'/></detail></details></draw></pattern>";
    let report = seamly::import(xml, None, "M").unwrap();
    let d = report.document;
    let g = beta_pattern_core::evaluate::grade(&d, "M", 0.01).unwrap();
    assert!((area(&g[0].seam) - 2500. * std::f64::consts::PI).abs() < 3.);
    assert_eq!(g[0].piece.quantity, 2);
    assert_eq!(g[0].piece.match_points[0].point, [0., 0.]);
    assert_eq!(g[0].piece.marks.len(), 2);
}
