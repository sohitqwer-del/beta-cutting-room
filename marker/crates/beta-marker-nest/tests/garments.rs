//! Short end-to-end searches on original synthetic jobs.
use anyhow::Result;
use beta_marker_core::{allowed_rotations, demands};
use beta_marker_nest::{NestOptions, nest};

#[path = "../../../examples/generate_jobs.rs"]
#[allow(dead_code)]
mod fixtures;

#[test]
fn synthetic_garments_and_required_tilt() -> Result<()> {
    for (name, job) in fixtures::synthetic_jobs() {
        let result = nest(
            &job,
            &NestOptions {
                time_limit_s: 1.,
                seed: Some(1),
                workers: Some(1),
                early_termination: true,
                ..Default::default()
            },
        )?;
        assert_eq!(
            result.placements.len(),
            demands(&job)
                .iter()
                .map(|d| d.count as usize)
                .sum::<usize>(),
            "{name}"
        );
        assert!(
            result.efficiency > 0. && result.efficiency <= 1.0001,
            "{name}"
        );
        for pl in &result.placements {
            let p = job
                .pieces
                .iter()
                .find(|p| p.id == pl.piece_id)
                .expect("known piece");
            assert!(allowed_rotations(&job, p, pl.mirrored).allows(pl.rotation_deg));
        }
        if name == "tilt.json" {
            assert!((result.placements[0].rotation_deg - 358.).abs() < 0.001);
        }
    }
    Ok(())
}

#[test]
#[ignore = "long quality check; short searches run in synthetic_garments_and_required_tilt"]
fn shirt_sixty_seconds() -> Result<()> {
    let job = fixtures::synthetic_jobs().remove(0).1;
    let result = nest(
        &job,
        &NestOptions {
            time_limit_s: 60.,
            seed: Some(1),
            ..NestOptions::default()
        },
    )?;
    assert!(result.efficiency > 0.7);
    Ok(())
}
