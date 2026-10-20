//! Original synthetic stock used by the standalone demo and pipeline tests.
use crate::{Roll, RollRegister};
use beta_marker_core::Shrinkage;
/// Forty rolls with deterministic 40-120 m lengths and three inventory shades.
/// This is test stock, not measured factory data. Seed: 20261019.
pub fn demo_register() -> RollRegister {
    let mut seed = 20261019_u64;
    let rolls = (0..40)
        .map(|i| {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let length = 40.0 + ((seed >> 32) % 8001) as f64 / 100.0;
            Roll {
                id: format!("R{i:02}"),
                lot: "lot".into(),
                dye_lot: "dye".into(),
                shade_group: format!("S{}", i % 3),
                ticket_length_m: length,
                measured_length_m: length,
                usable_width_mm: 1500.0,
                shrinkage: Shrinkage {
                    length_pct: 2.0,
                    width_pct: 1.0,
                },
                defects: Vec::new(),
            }
        })
        .collect();
    RollRegister { rolls }
}
