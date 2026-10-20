use super::*;
use beta_marker_core::geometry::{place, transform_point};
use beta_marker_core::*;
use jagua_rs::io::ext_repr::{ExtLayout, ExtPlacedItem, ExtTransformation};
use jagua_rs::probs::spp::io::ext_repr::ExtSPSolution;

#[path = "../../../examples/generate_jobs.rs"]
#[allow(dead_code)]
mod fixtures;

fn small_job() -> Job {
    let mut job = fixtures::synthetic_jobs().remove(2).1;
    job.fabric.width = 300.;
    job.pieces.truncate(1);
    let p = &mut job.pieces[0];
    p.outline = vec![
        [10., 20.],
        [90., 20.],
        [90., 60.],
        [45., 60.],
        [45., 100.],
        [10., 100.],
    ];
    p.grain = Some([[10., 20.], [10., 100.]]);
    p.quantity = 2;
    p.mirror_quantity = 2;
    p.buffer = 3.;
    job
}

#[test]
fn mirror_grain_and_centring_round_trip() -> Result<()> {
    let mut job = small_job();
    job.pieces[0].grain = Some([[10., 20.], [90., 100.]]);
    let mapping = beta_marker_core::demands(&job);
    let (instance, shift) = build_instance(&job, &mapping)?;
    assert!((f64::from(instance.base_strip.fixed_height) - (300. - 2. * shift)).abs() < 1e-5);
    let ext = ExtSPSolution {
        strip_width: 500.,
        density: 0.,
        run_time_sec: 0,
        layout: ExtLayout {
            container_id: 0,
            density: 0.,
            placed_items: mapping
                .iter()
                .enumerate()
                .map(|(i, _)| ExtPlacedItem {
                    item_id: i as u64,
                    transformation: ExtTransformation {
                        reflected: false,
                        rotation: 180.,
                        translation: (200., 150.),
                    },
                })
                .collect(),
        },
    };
    let sol = jagua_rs::probs::spp::io::import_solution(&instance, &ext)?;
    let placements = map_solution(&sol, &mapping, &job, shift)?;
    for pl in &placements {
        assert!(allowed_rotations(&job, &job.pieces[0], pl.mirrored).allows(pl.rotation_deg));
        let base = grain_alignment_deg(job.pieces[0].grain, pl.mirrored);
        for (point, actual) in job.pieces[0]
            .outline
            .iter()
            .zip(place(&job.pieces[0].outline, pl))
        {
            let aligned = transform_point(*point, pl.mirrored, base, [0., 0.]);
            let expected = transform_point(aligned, false, 180., [200. + shift, 150. + shift]);
            assert!((actual[0] - expected[0]).abs() < 0.001);
            assert!((actual[1] - expected[1]).abs() < 0.001);
        }
    }
    Ok(())
}

#[test]
fn tilt_is_discrete_bounded_and_relative_to_grain() {
    let mut job = small_job();
    job.fabric.tilt_deg = 2.;
    let d = &demands(&job)[0];
    let base = grain_alignment_deg(job.pieces[0].grain, d.mirrored);
    let ExtRotation::Discrete { angles } = engine_rotation(&job, d, base) else {
        panic!("expected discrete");
    };
    assert_eq!(angles.len(), 10);
    for a in angles {
        assert!(allowed_rotations(&job, &job.pieces[0], d.mirrored).allows(base + f64::from(a)));
    }
    job.pieces[0].rotation = RotationRule::Free;
    assert!(matches!(
        engine_rotation(&job, d, base),
        ExtRotation::Continuous {}
    ));
}

#[test]
fn buffered_pairs_fit_and_report_progress() -> Result<()> {
    let job = small_job();
    let mut progress = vec![];
    let result = nest_with_progress(
        &job,
        &NestOptions {
            time_limit_s: 1.,
            seed: Some(12),
            workers: Some(1),
            early_termination: true,
            ..Default::default()
        },
        |p| progress.push(p),
    )?;
    check_placements(&job, &result.placements)?;
    assert_eq!(result.placements.len(), 4);
    assert_eq!(result.seed, Some(12));
    assert!(validate(&job, &result).ok);
    assert!(!progress.is_empty());
    assert!(result.length <= progress.last().expect("reported a valid result").length + 1e-7);
    assert!(
        progress
            .windows(2)
            .all(|w| w[0].length > w[1].length && w[0].seconds <= w[1].seconds)
    );
    Ok(())
}

#[test]
fn safety_rejects_overlap_gap_counts_rotation_and_edges() -> Result<()> {
    let mut job = small_job();
    job.pieces[0].quantity = 2;
    job.pieces[0].mirror_quantity = 0;
    job.pieces[0].grain = None;
    let a = Placement {
        piece_id: job.pieces[0].id.clone(),
        copy: 0,
        mirrored: false,
        rotation_deg: 0.,
        translate: [10., 0.],
        locked: false,
        ..Default::default()
    };
    let mut b = a.clone();
    b.copy = 1;
    b.translate[0] = 100.;
    check_placements(&job, &[a.clone(), b.clone()])?;
    assert!(check_placements(&job, std::slice::from_ref(&a)).is_err());
    b.translate = a.translate;
    assert!(check_placements(&job, &[a.clone(), b.clone()]).is_err());
    b.translate = [94., 0.]; // six mm separation, but ten required
    assert!(check_placements(&job, &[a.clone(), b.clone()]).is_err());
    b.translate = [100., 0.];
    b.rotation_deg = 90.;
    assert!(check_placements(&job, &[a.clone(), b.clone()]).is_err());
    b.rotation_deg = 0.;
    b.translate[1] = 300.;
    assert!(check_placements(&job, &[a, b]).is_err());
    Ok(())
}

#[test]
fn invalid_input_returns_errors() {
    let mut job = small_job();
    assert!(
        nest(
            &job,
            &NestOptions {
                workers: Some(0),
                ..NestOptions::default()
            }
        )
        .is_err()
    );
    assert!(
        nest(
            &job,
            &NestOptions {
                time_limit_s: f64::NAN,
                ..NestOptions::default()
            }
        )
        .is_err()
    );
    job.fabric.width = f64::NAN;
    assert!(nest(&job, &NestOptions::default()).is_err());
    job.fabric.width = 300.;
    job.pieces[0].buffer = f64::MAX;
    assert!(
        nest(&job, &NestOptions::default())
            .unwrap_err()
            .to_string()
            .contains("clearance exceeds engine precision")
    );
    job.pieces[0].buffer = 3.;
    job.pieces[0].quantity = u32::MAX;
    job.pieces[0].size = Some("M".into());
    job.order = Some(Order {
        sizes: BTreeMap::from([("M".into(), 2)]),
    });
    assert!(nest(&job, &NestOptions::default()).is_err());
}

#[test]
fn examples_are_reproducible() -> Result<()> {
    for (name, job) in fixtures::synthetic_jobs() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/jobs")
            .join(name);
        let text = std::fs::read_to_string(path)?;
        let stored: Job = serde_json::from_str(&text)?;
        assert_eq!(format!("{}\n", serde_json::to_string_pretty(&job)?), text);
        assert!(check_job(&stored).is_empty());
        assert!(check_job(&job).is_empty());
    }
    Ok(())
}

#[test]
fn detailed_collision_shape_stays_outside_the_cut_line() -> Result<()> {
    use geo::{Contains, Distance, Euclidean, Point, Polygon};
    let mut job = small_job();
    job.pieces.truncate(1);
    let piece = &mut job.pieces[0];
    piece.outline = (0..360)
        .map(|i| {
            let a = (i as f64).to_radians();
            [50. * a.cos(), 40. * a.sin()]
        })
        .collect();
    piece.grain = None;
    piece.mirror_quantity = 0;
    let mapping = demands(&job);
    let (instance, _) = build_instance(&job, &mapping)?;
    let item = instance.item(0);
    let collision = item.shape_cd();
    assert!(collision.n_vertices() < 360);
    let collision_poly = Polygon::new(
        collision
            .vertices()
            .iter()
            .map(|p| (f64::from(p.0), f64::from(p.1)))
            .collect(),
        vec![],
    );
    let pre = item.shape_orig().pre_transform.translation();
    for p in &job.pieces[0].outline {
        let p = Point::new(p[0] + f64::from(pre.0), p[1] + f64::from(pre.1));
        assert!(collision_poly.contains(&p));
        assert!(Euclidean.distance(collision_poly.exterior(), &p) >= 4.99);
    }
    Ok(())
}

#[test]
fn zero_edge_gap_gives_back_the_half_piece_gap() -> Result<()> {
    let mut job = small_job();
    job.spacing.edge_gap = 0.;
    let mapping = demands(&job);
    let (instance, shift) = build_instance(&job, &mapping)?;
    assert_eq!(shift, -2.);
    assert_eq!(instance.base_strip.fixed_height, 304.);
    Ok(())
}

#[test]
fn fixed_seed_repeats_initial_construction() -> Result<()> {
    let job = small_job();
    let mapping = demands(&job);
    let (instance, shift) = build_instance(&job, &mapping)?;
    let construct = || -> Result<Vec<Placement>> {
        let builder = sparrow::optimizer::lbf::LBFBuilder::new(
            instance.clone(),
            Xoshiro256PlusPlus::seed_from_u64(42),
            sparrow::consts::LBF_SAMPLE_CONFIG,
        )?
        .construct()?;
        map_solution(&builder.prob.save(), &mapping, &job, shift)
    };
    assert_eq!(construct()?, construct()?);
    Ok(())
}

fn check_placements(job: &Job, placements: &[Placement]) -> Result<()> {
    let metrics = compute_metrics(job, placements);
    let result = MarkerResult {
        format: RESULT_FORMAT.into(),
        job_name: job.name.clone(),
        width: job.fabric.width,
        length: metrics.length,
        piece_area: metrics.piece_area,
        efficiency: metrics.efficiency,
        engine: "test".into(),
        seed: None,
        runtime_s: 0.,
        placements: placements.to_vec(),
        warnings: vec![],
    };
    let report = validate(job, &result);
    ensure!(report.ok, "{:?}", report.errors);
    Ok(())
}

#[test]
fn zero_gap_f32_rounding_must_not_shrink_the_collision_outline() -> Result<()> {
    use geo::{Area, BooleanOps, Buffer, Covers};
    let mut job = small_job();
    job.spacing = Spacing::default();
    let p = &mut job.pieces[0];
    p.grain = None;
    p.buffer = 0.;
    p.mirror_quantity = 0;
    // Own four-vertex fixture: f64 right edge rounds inward to exactly 10 f32.
    // Two engine shapes at x=0 and x=10 touch, but their cut lines overlap.
    p.outline = vec![[0., 0.], [10.0000004, 0.], [10.0000004, 10.], [0., 10.]];
    let unguarded = Importer::new(DEFAULT_SPARROW_CONFIG.cde_config, None, None).import_item(
        &ExtItem {
            id: 0,
            orientation: ExtOrientation {
                rotation: ExtRotation::Discrete { angles: vec![0.] },
                reflection_axes: vec![],
            },
            shape: ExtShape::SimplePolygon(ExtSPolygon(
                p.outline
                    .iter()
                    .map(|p| (p[0] as f32, p[1] as f32))
                    .collect(),
            )),
            min_quality: None,
        },
        0,
    )?;
    assert!(!safety::collision_covers(&unguarded, &p.outline, 0.));
    let mapping = demands(&job);
    let (instance, _) = build_instance(&job, &mapping)?;
    assert!(safety::collision_covers(
        instance.item(0),
        &job.pieces[0].outline,
        0.
    ));
    let a = Placement {
        piece_id: job.pieces[0].id.clone(),
        copy: 0,
        mirrored: false,
        rotation_deg: 0.,
        translate: [0., 0.],
        locked: false,
        ..Default::default()
    };
    let mut b = a.clone();
    b.copy = 1;
    b.translate[0] = 10.;
    assert!(check_placements(&job, &[a.clone(), b.clone()]).is_err());
    let pa = safety::polygon(&place(&job.pieces[0].outline, &a));
    let pb = safety::polygon(&place(&job.pieces[0].outline, &b));
    // Analytic rectangle intersection: Boolean clipping can snap away this
    // sub-micron sliver, so it is not a reliable reference for its area.
    let overlap_width = job.pieces[0].outline[1][0] - b.translate[0];
    assert!(overlap_width * 10.0 > 0.0000039);
    // The old guard eroded both cut lines and therefore hid this real overlap.
    assert_eq!(
        pa.buffer(-0.005)
            .intersection(&pb.buffer(-0.005))
            .unsigned_area(),
        0.0
    );
    // Both vertices and all segments/interiors must fit the collision shape.
    let pre = instance.item(0).shape_orig().pre_transform.translation();
    let cut = safety::polygon(&transform_points(
        &job.pieces[0].outline,
        false,
        0.,
        [f64::from(pre.0), f64::from(pre.1)],
    ));
    let cd = geo::Polygon::new(
        instance
            .item(0)
            .shape_cd()
            .vertices()
            .iter()
            .map(|p| (f64::from(p.0), f64::from(p.1)))
            .collect(),
        vec![],
    );
    assert!(cd.covers(&cut));
    let m = compute_metrics(&job, &[a.clone(), b.clone()]);
    let mut result = MarkerResult {
        format: RESULT_FORMAT.into(),
        job_name: job.name.clone(),
        width: job.fabric.width,
        length: m.length,
        piece_area: m.piece_area,
        efficiency: m.efficiency,
        engine: "test".into(),
        seed: None,
        runtime_s: 0.,
        placements: vec![a, b],
        warnings: vec![],
    };
    safety::repair(&job, &mut result)?;
    assert!(validate(&job, &result).ok);
    assert!(
        result
            .warnings
            .iter()
            .any(|w| w.contains("validation_repair"))
    );
    Ok(())
}

#[test]
fn cad_cleanup_preserves_boundary_and_rejects_crossings_and_spikes() -> Result<()> {
    let points = vec![
        [0., 0.],
        [0., 0.],
        [0., 5.],
        [0., 10.],
        [10., 10.],
        [10., 0.],
        [0., 0.],
    ];
    let clean = safety::clean_outline(&points)?;
    assert_eq!(clean.len(), 4);
    assert_eq!(geometry::area(&clean), 100.);
    assert!(geometry::signed_area(&clean) > 0.);
    for invalid in [
        vec![[0., 0.], [10., 10.], [0., 10.], [10., 0.]],
        vec![[0., 0.], [10., 0.], [5., 0.], [10., 10.], [0., 10.]],
    ] {
        assert!(
            safety::clean_outline(&invalid)
                .unwrap_err()
                .to_string()
                .contains("simple polygon")
        );
    }
    Ok(())
}

#[test]
fn repair_can_separate_coincident_copies_and_cannot_ignore_max_length() -> Result<()> {
    let mut job = small_job();
    job.pieces[0].mirror_quantity = 0;
    job.pieces[0].grain = None;
    let a = Placement {
        piece_id: job.pieces[0].id.clone(),
        copy: 0,
        mirrored: false,
        rotation_deg: 0.,
        translate: [0., 0.],
        locked: false,
        ..Default::default()
    };
    let mut b = a.clone();
    b.copy = 1;
    let mut result = MarkerResult {
        format: RESULT_FORMAT.into(),
        job_name: job.name.clone(),
        width: job.fabric.width,
        length: 0.,
        piece_area: 0.,
        efficiency: 0.,
        engine: "test".into(),
        seed: None,
        runtime_s: 0.,
        placements: vec![a, b],
        warnings: vec![],
    };
    safety::repair(&job, &mut result)?;
    assert!(validate(&job, &result).ok);
    job.fabric.max_length = Some(1.);
    assert!(
        safety::repair(&job, &mut result)
            .unwrap_err()
            .to_string()
            .contains("too_long")
    );
    Ok(())
}

#[test]
fn windbreaker_collision_outlines_cover_all_original_cad_segments() -> Result<()> {
    // Open+Pattern, CC BY 4.0. Attribution and source in testdata/open/dxf/README.md.
    let job: Job = serde_json::from_str(include_str!(
        "../tests/fixtures/windbreaker-sleeves.job.json"
    ))?;
    assert_eq!(job.pieces.len(), 2);
    safety::check_input(&job)?;
    let mapping = demands(&job);
    let (instance, _) = build_instance(&job, &mapping)?;
    // The actual reported under-panel needs only four vertices. Even with no
    // simplification its old unguarded f32 collision polygon loses coverage.
    let panel = &job.pieces[1];
    assert_eq!(panel.outline.len(), 4);
    let points = transform_points(
        &panel.outline,
        false,
        grain_alignment_deg(panel.grain, false),
        [0., 0.],
    );
    let old = Importer::new(DEFAULT_SPARROW_CONFIG.cde_config, None, None).import_item(
        &ExtItem {
            id: 0,
            orientation: ExtOrientation {
                rotation: ExtRotation::Discrete { angles: vec![0.] },
                reflection_axes: vec![],
            },
            shape: ExtShape::SimplePolygon(ExtSPolygon(
                points.iter().map(|p| (p[0] as f32, p[1] as f32)).collect(),
            )),
            min_quality: None,
        },
        0,
    )?;
    assert!(!safety::collision_covers(&old, &points, 0.));
    for (i, demand) in mapping.iter().enumerate() {
        let piece = &job.pieces[demand.piece_index];
        let points = transform_points(
            &piece.outline,
            demand.mirrored,
            grain_alignment_deg(piece.grain, demand.mirrored),
            [0., 0.],
        );
        assert!(
            safety::collision_covers(instance.item(i), &points, 0.),
            "{}",
            piece.id
        );
    }
    Ok(())
}

#[test]
fn each_piece_gets_its_own_tilt_samples() {
    let mut j = small_job();
    j.fabric.tilt_deg = 2.0;
    j.pieces[0].tilt_deg = Some(0.0);
    let demand = beta_marker_core::demands(&j).remove(0);
    let base = grain_alignment_deg(j.pieces[0].grain, demand.mirrored);
    let ExtRotation::Discrete { angles: exact } = engine_rotation(&j, &demand, base) else {
        panic!("discrete rule")
    };
    j.pieces[0].tilt_deg = Some(3.0);
    let ExtRotation::Discrete { angles: tilted } = engine_rotation(&j, &demand, base) else {
        panic!("discrete rule")
    };
    assert!(tilted.len() > exact.len());
    assert!(tilted.contains(&3.0) && tilted.contains(&357.0));
}
