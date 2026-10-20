use super::*;
use beta_marker_core::{Fabric, JOB_FORMAT, Piece, RotationRule, Spacing};

fn job() -> Job {
    Job {
        format: JOB_FORMAT.into(),
        name: "gpu test".into(),
        fabric: Fabric {
            width: 110.0,
            ..Default::default()
        },
        spacing: Spacing {
            piece_gap: 1.5,
            edge_gap: 2.0,
        },
        pieces: vec![Piece {
            id: "concave".into(),
            name: "concave".into(),
            quantity: 2,
            mirror_quantity: 1,
            outline: vec![
                [0.0, 0.0],
                [43.0, 0.0],
                [43.0, 12.0],
                [20.0, 12.0],
                [20.0, 31.0],
                [0.0, 31.0],
            ],
            grain: Some([[0.0, 0.0], [1.0, 0.0]]),
            rotation: RotationRule::Quarter,
            buffer: 0.7,
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn cpu() -> Result<Proposer> {
    Proposer::new(GpuOptions {
        backend: BackendChoice::Cpu,
        ..Default::default()
    })
}

#[test]
fn cpu_constructs_exact_valid_mirrored_concave_copies() -> Result<()> {
    let j = job();
    let proposal = cpu()?.construct(&j)?;
    assert!(beta_marker_core::validate(&j, &proposal.result).ok);
    assert_eq!(proposal.stats.backend, "CPU");
    assert_eq!(proposal.stats.dispatches, 0);
    assert_eq!(proposal.stats.exact_rejections, 0);
    assert_eq!(proposal.result.placements.len(), 3);
    assert!(
        proposal
            .result
            .placements
            .iter()
            .all(|p| p.bundle == Some(0))
    );
    Ok(())
}

#[test]
fn shrinkage_blocks_and_size_ratios_are_validated() -> Result<()> {
    let mut j = job();
    j.pieces[0].size = Some("M".into());
    j.pieces[0].block_margin = Some(1.0);
    j.order = Some(beta_marker_core::Order {
        sizes: [("M".into(), 2)].into(),
    });
    j.fabric.shrinkage.length_pct = 2.0;
    j.fabric.shrinkage.width_pct = 1.0;
    let result = cpu()?.construct(&j)?.result;
    assert_eq!(result.placements.len(), 6);
    assert!(result.placements.iter().any(|p| p.bundle == Some(1)));
    assert!(beta_marker_core::validate(&j, &result).ok);
    Ok(())
}

#[test]
fn rejects_invalid_and_unsupported_inputs_without_device() -> Result<()> {
    assert!(
        Proposer::new(GpuOptions {
            cell_mm: f64::NAN,
            ..Default::default()
        })
        .is_err()
    );
    let mut p = cpu()?;
    let mut j = job();
    j.fabric.layout = Layout::Folded;
    assert!(p.construct(&j).is_err());
    j = job();
    j.fabric.width = f64::INFINITY;
    assert!(p.construct(&j).is_err());
    j = job();
    j.pieces[0].outline[0][0] = f64::NAN;
    assert!(p.construct(&j).is_err());
    j = job();
    j.pieces[0].outline = vec![[0.0, 0.0], [40.0, 30.0], [0.0, 30.0], [20.0, 0.0]];
    assert!(p.construct(&j).is_err());
    j = job();
    j.pieces[0].quantity = u32::MAX;
    j.pieces[0].size = Some("M".into());
    j.order = Some(beta_marker_core::Order {
        sizes: [("M".into(), 2)].into(),
    });
    assert!(p.construct(&j).is_err());
    j = job();
    j.order = Some(beta_marker_core::Order {
        sizes: [("unused".into(), u32::MAX)].into(),
    });
    assert!(p.construct(&j).is_err());
    assert!(
        Proposer::new(GpuOptions {
            backend: BackendChoice::Cpu,
            max_grid_words: 1,
            ..Default::default()
        })?
        .construct(&job())
        .is_err()
    );
    Ok(())
}

#[test]
fn absent_adapter_automatically_falls_back_without_gpu() -> Result<()> {
    let mut p = cpu()?;
    p.options.backend = BackendChoice::Auto;
    p.gpu = select_backend(
        BackendChoice::Auto,
        Err(anyhow::anyhow!("no adapter (injected)")),
        &mut p.initial,
    )?;
    assert!(p.gpu.is_none());
    let proposal = p.construct(&job())?;
    assert!(
        proposal
            .stats
            .fallback_reason
            .as_deref()
            .is_some_and(|s| s.contains("no adapter"))
    );
    assert!(beta_marker_core::validate(&job(), &proposal.result).ok);
    let result = nest(
        &job(),
        &NestOptions {
            time_limit_s: 0.02,
            workers: Some(2),
            seed: Some(1),
            ..Default::default()
        },
        &mut p,
    )?;
    assert!(
        result
            .warnings
            .iter()
            .any(|w| w.contains("GPU unavailable; plain nesting"))
    );
    assert!(beta_marker_core::validate(&job(), &result).ok);
    assert!(
        select_backend(
            BackendChoice::GpuRequired,
            Err(anyhow::anyhow!("no adapter")),
            &mut p.initial
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn word_shifts_preserve_collision_across_32_bit_boundaries() -> Result<()> {
    let mut strip = Raster::empty(8, 100, 1000)?;
    let mut shape = Raster::empty(3, 35, 1000)?;
    shape.bits[0] = u32::MAX;
    shape.bits[1] = 7;
    shape.bits[2] = 3;
    for y in 0..=65 {
        strip.bits.fill(0);
        assert!(strip.fits(&shape, 2, y));
        strip.stamp(&shape, 2, y);
        assert!(!strip.fits(&shape, 2, y));
        assert!(strip.fits(&shape, 5, y));
    }
    Ok(())
}

#[test]
fn rejected_raster_warm_start_uses_plain_nest() -> Result<()> {
    let mut j = job();
    j.fabric.defects.push(beta_marker_core::Zone {
        polygon: vec![[0.0, 90.0], [1.0, 90.0], [1.0, 91.0], [0.0, 91.0]],
        ..Default::default()
    });
    let options = NestOptions {
        time_limit_s: 0.02,
        workers: Some(2),
        seed: Some(1),
        ..Default::default()
    };
    let result = nest(&j, &options, &mut cpu()?)?;
    assert!(beta_marker_core::validate(&j, &result).ok);
    assert!(result.warnings.iter().any(|w| w.contains("plain nesting")));
    Ok(())
}

#[test]
fn cpu_warm_start_refines_through_native_adapter() -> Result<()> {
    let j = job();
    let mut proposer = cpu()?;
    let proposal = proposer.construct(&j)?;
    let options = NestOptions {
        time_limit_s: 0.05,
        workers: Some(2),
        seed: Some(1),
        warm_start: Some(proposal.result.clone()),
        ..Default::default()
    };
    let refined = beta_marker_nest::nest(&j, &options)?;
    assert!(refined.length <= proposal.result.length + 1e-6);
    assert!(beta_marker_core::validate(&j, &refined).ok);
    Ok(())
}

#[test]
fn wrapper_preserves_explicit_fixed_placements() -> Result<()> {
    let j = job();
    let mut proposer = cpu()?;
    let proposal = proposer.construct(&j)?;
    let mut fixed = proposal.result.placements[0].clone();
    fixed.locked = true;
    let options = NestOptions {
        time_limit_s: 0.02,
        workers: Some(2),
        seed: Some(1),
        fixed: vec![fixed.clone()],
        ..Default::default()
    };
    let result = nest(&j, &options, &mut proposer)?;
    assert!(result.placements.contains(&fixed));
    assert!(beta_marker_core::validate(&j, &result).ok);
    Ok(())
}

#[test]
fn gap_fill_keeps_locks_and_cannot_regress() -> Result<()> {
    let mut j = job();
    j.pieces[0].rotation = RotationRule::Fixed;
    let mut proposer = cpu()?;
    let mut incumbent = proposer.construct(&j)?.result;
    incumbent.placements[0].locked = true;
    let locked = incumbent.placements[0].clone();
    incumbent.placements[2].translate[0] += 500.0;
    let m = beta_marker_core::compute_metrics(&j, &incumbent.placements);
    incumbent.length = m.length;
    incumbent.efficiency = m.efficiency;
    assert!(beta_marker_core::validate(&j, &incumbent).ok);
    let fill = proposer.fill_gaps(&j, &incumbent, 3)?;
    assert!(fill.result.length < incumbent.length);
    assert!(fill.result.placements.contains(&locked));
    assert!(beta_marker_core::validate(&j, &fill.result).ok);
    let again = proposer.fill_gaps(&j, &fill.result, 3)?;
    assert!(again.result.length <= fill.result.length);
    assert!(again.result.placements.contains(&locked));
    let mut stale = again.result.clone();
    stale.length = 1.0;
    stale.efficiency = 0.99;
    let normalized = proposer.fill_gaps(&j, &stale, 0)?;
    assert!(
        (normalized.result.length - beta_marker_core::validate(&j, &normalized.result).length)
            .abs()
            < 1e-6
    );
    assert!(proposer.fill_gaps(&j, &incumbent, 33).is_err());
    Ok(())
}

#[test]
#[ignore = "explicit hardware parity test; ordinary tests do not need a GPU"]
fn gpu_matches_cpu_for_shifted_masks_and_layout() -> Result<()> {
    let gpu = backend::Gpu::new()?;
    let mut strip = Raster::empty(12, 100, 1000)?;
    let mut shape = Raster::empty(3, 35, 1000)?;
    for row in 0..shape.rows {
        shape.bits[(row * 2) as usize] = 0x55;
        shape.bits[(row * 2 + 1) as usize] = 7;
    }
    for y in [0, 1, 17, 31, 32, 63, 65] {
        strip.bits.fill(0);
        strip.stamp(&shape, 2, y);
        let count = 9 * (strip.height - shape.height + 1);
        for first in [0, 131, 200, count - 1, count] {
            let expected = (first..count).find(|&i| strip.fits(&shape, i / 66, i % 66));
            let actual = gpu.scan(&strip, &shape, 8, first, &mut ScanStats::default())?;
            assert_eq!(actual, expected, "shift={y}, first={first}");
        }
    }
    // Cross the 65,535-workgroup boundary used by real garment scans.
    let mut large = Raster::empty(5001, 1000, 200_000)?;
    large.bits.fill(u32::MAX);
    large.bits[(4999 * large.stride + 999 / 32) as usize] &= !(1 << (999 % 32));
    let mut dot = Raster::empty(1, 1, 10)?;
    dot.bits[0] = 1;
    assert_eq!(
        gpu.scan(&large, &dot, 4999, 4_194_230, &mut ScanStats::default())?,
        Some(4_999_999)
    );
    let j = job();
    let cpu = cpu()?.construct(&j)?;
    let gpu = Proposer::new(GpuOptions {
        backend: BackendChoice::GpuRequired,
        ..Default::default()
    })?
    .construct(&j)?;
    assert_eq!(cpu.result.placements, gpu.result.placements);
    assert!(beta_marker_core::validate(&j, &gpu.result).ok);
    Ok(())
}
