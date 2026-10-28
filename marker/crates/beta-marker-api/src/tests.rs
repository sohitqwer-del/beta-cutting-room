use super::*;
use serde_json::json;

fn rectangle_job() -> Job {
    serde_json::from_value(json!({
        "format": "beta-marker.job/v1", "name": "Rectangles",
        "fabric": {"width": 100.0},
        "pieces": [{"id": "rect", "name": "Rectangle", "size": "M", "quantity": 1, "mirror_quantity": 0,
            "outline": [[0.0, 0.0], [20.0, 0.0], [20.0, 10.0], [0.0, 10.0]]}],
        "order": {"sizes": {"M": 1}}
    }))
    .expect("job")
}

#[test]
fn measured_cost_route_keeps_failures_and_only_selects_final_valid_plans() {
    let request = json!({
        "order": order(4, 4), "currency":"INR",
        "offers":[
            {"id":"fits","width_mm":100.0,"price":{"unit":"metre","price":20.0},"lay_cost":1.0},
            {"id":"too-narrow","width_mm":1.0,"price":{"unit":"metre","price":1.0},"lay_cost":0.0},
            {"id":"no-stock","width_mm":100.0,"price":{"unit":"metre","price":0.0},"lay_cost":0.0,"rolls":{"rolls":[]}}
        ],
        "options":{"time_per_marker_s":0.05,"max_candidates":2,"max_refinements":1,"search_nodes":100},
        "finalists":1,"final_time_s":0.05
    });
    let output = cutroom("/api/plan/cost", &serde_json::to_vec(&request).unwrap())
        .unwrap()
        .unwrap();
    let comparison: beta_marker_plan::cost::CostComparison =
        serde_json::from_slice(output.body()).unwrap();
    assert_eq!(comparison.trials.len(), 3);
    assert!(!comparison.trials[1].feasible);
    assert!(!comparison.trials[2].feasible);
    assert!(comparison.trials[2].error.is_some());
    let winner = &comparison.trials[comparison.chosen.expect("a winner")];
    assert!(winner.feasible && winner.final_evaluated);
    assert!(winner.plan.as_ref().unwrap().complete);
    assert!(winner.cost.as_ref().unwrap().total > 0.);
}

#[test]
fn cost_route_rejects_unbounded_interactive_requests() {
    let request =
        json!({"order":order(4,4),"currency":"INR","offers":[],"finalists":1,"final_time_s":8.0});
    let input: beta_marker_plan::cost::CostRequest = serde_json::from_value(request).unwrap();
    assert!(check_cost_comparison(&input).is_err());
}

#[test]
fn cost_comparison_preserves_table_end_and_rejects_unhandled_cut_sections() {
    let mut request: beta_marker_plan::cost::CostRequest = serde_json::from_value(json!({
        "order":order(4,4),"currency":"INR","offers":[{"id":"a","width_mm":100,"price":{"unit":"metre","price":20},"lay_cost":0}],
        "finalists":1,"final_time_s":30
    })).unwrap();
    request
        .order
        .job
        .meta
        .insert("editor_v1".into(), json!({"table_length":25}));
    assert_eq!(
        prepare_cost_comparison(&request)
            .unwrap()
            .order
            .job
            .fabric
            .max_length,
        Some(25.)
    );
    request.order.job.fabric.max_length = Some(20.);
    assert_eq!(
        prepare_cost_comparison(&request)
            .unwrap()
            .order
            .job
            .fabric
            .max_length,
        Some(20.)
    );
    request
        .order
        .job
        .meta
        .insert("editor_v1".into(), json!({"sections":[10.]}));
    assert!(
        prepare_cost_comparison(&request)
            .unwrap_err()
            .to_string()
            .contains("cut sections")
    );
}
fn rectangle_marker() -> MarkerResult {
    serde_json::from_value(json!({
        "format": "beta-marker.result/v1", "job_name": "Rectangles", "width": 100.0,
        "length": 20.0, "piece_area": 200.0, "efficiency": 0.1, "engine": "fixture",
        "seed": 1, "runtime_s": 0.0, "warnings": [],
        "placements": [{"piece_id": "rect", "copy": 0, "mirrored": false, "rotation_deg": 0.0,
            "translate": [0.0, 0.0], "bundle": 0}]
    }))
    .expect("marker")
}
fn order(quantity: u32, max_plies: u32) -> CutOrder {
    CutOrder {
        format: beta_marker_plan::CUT_ORDER_FORMAT.into(),
        name: "Order".into(),
        job: rectangle_job(),
        quantities: [("M".to_string(), quantity)].into(),
        max_plies,
        max_garments_per_marker: 1,
        overcut_pct: 0.0,
    }
}

#[test]
fn known_markers_replace_the_engine_and_stale_ones_are_renested() {
    let marker = rectangle_marker();
    assert!(beta_marker_core::validate(&rectangle_job(), &marker).ok);
    let input = PlanRequest {
        order: order(7, 5),
        options: PlanOptions {
            time_per_marker_s: 0.1,
            ..Default::default()
        },
        markers: vec![KnownMarker {
            ratio: [("M".to_string(), 1)].into(),
            result: marker.clone(),
        }],
    };
    let mut never = |_: &Job, _: &beta_marker_nest::NestOptions| -> Result<MarkerResult> {
        panic!("a known marker must not be nested again")
    };
    let mut checkpoints = 0;
    let made = plan(&input, &mut never, |_| {
        checkpoints += 1;
        Ok(())
    })
    .expect("plan");
    assert!(made.complete && checkpoints > 0);
    assert_eq!(made.planned["M"], 7);
    assert!(
        made.lays
            .iter()
            .all(|l| l.passes <= 5 && l.plies == l.passes)
    );
    assert_eq!(known_markers(&made)[0].result.placements, marker.placements);
    // A marker that fails validation for this job is nested again.
    let mut stale = input;
    stale.markers[0].result.placements[0].translate = [0.0, 95.0]; // off the fabric width
    let mut calls = 0;
    let mut engine = |_: &Job, _: &beta_marker_nest::NestOptions| -> Result<MarkerResult> {
        calls += 1;
        Ok(rectangle_marker())
    };
    plan(&stale, &mut engine, |_| Ok(())).expect("plan");
    assert!(calls > 0);
}

#[test]
fn plan_input_errors_match_every_transport() {
    let input = PlanRequest {
        order: order(0, 5),
        options: PlanOptions::default(),
        markers: vec![],
    };
    assert!(
        check_plan(&input)
            .unwrap_err()
            .starts_with("Plan needs positive quantities")
    );
    let mut overflow = order(3, 5);
    overflow.job.pieces[0].quantity = u32::MAX;
    overflow.job.order = Some(beta_marker_core::Order {
        sizes: [("M".to_string(), 2)].into(),
    });
    let input = PlanRequest {
        order: overflow,
        options: PlanOptions::default(),
        markers: vec![],
    };
    assert_eq!(
        check_plan(&input).unwrap_err(),
        "Required copy count exceeds u32"
    );
}

#[test]
fn cutroom_routes_answer_json_and_pdf() {
    assert!(cutroom("/api/plan", b"{}").is_none());
    let register = beta_marker_roll::demo_register();
    let inspected = cutroom(
        "/api/roll/inspect",
        json!({"register": register}).to_string().as_bytes(),
    )
    .expect("route")
    .expect("inspect");
    let value: serde_json::Value = serde_json::from_slice(inspected.body()).unwrap();
    assert!(value["inspection"].is_object() || value["inspection"].is_array());
    let lay = physical_lay(PhysicalLayRequest {
        job: rectangle_job(),
        result: rectangle_marker(),
        id: "L1".into(),
        order: "PO-1".into(),
        colour: "Blue".into(),
        plies: vec![PlyRun {
            first: 1,
            last: 4,
            shade_group: "A".into(),
        }],
    })
    .expect("lay");
    let Output::Json(bundles) = cutroom(
        "/api/cutroom/bundles",
        json!({"lay": lay}).to_string().as_bytes(),
    )
    .unwrap()
    .unwrap() else {
        panic!("bundles are JSON")
    };
    let bundles: serde_json::Value = serde_json::from_slice(&bundles).unwrap();
    let tickets = cutroom(
        "/api/cutroom/tickets",
        json!({"bundles": bundles}).to_string().as_bytes(),
    )
    .unwrap()
    .unwrap();
    let Output::File { mime, body, .. } = tickets else {
        panic!("tickets are a file")
    };
    assert_eq!(mime, "application/pdf");
    assert!(body.starts_with(b"%PDF"));
    let error = cutroom("/api/cutroom/tickets", br#"{"bundles":[],"paper":"a3"}"#)
        .unwrap()
        .unwrap_err();
    assert!(error.to_string().contains("Paper must be"));
}

#[test]
fn spread_markers_take_passes_and_the_older_plies_name() {
    let register = beta_marker_roll::demo_register();
    for key in ["passes", "plies"] {
        let body = json!({"register": register, "markers": [{"id": "L1", "job": rectangle_job(),
            "result": rectangle_marker(), key: 3, "end_allowance_mm": 10.0}]});
        let out = cutroom("/api/roll/spread", body.to_string().as_bytes())
            .unwrap()
            .expect("spread");
        let plan: SpreadingPlan = serde_json::from_slice(out.body()).unwrap();
        assert_eq!(plan.lays[0].lay.plies, 3);
        assert!((plan.lays[0].lay.marker_length_m - 0.04).abs() < 1e-12);
    }
}

#[test]
fn plt_import_needs_a_width() {
    assert!(
        import_plt(b"IN;", None)
            .unwrap_err()
            .contains("positive fabric width")
    );
    assert!(import_plt(b"IN;", Some("-1")).is_err());
}
