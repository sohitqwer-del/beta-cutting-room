//! Opt-in phase timing for local diagnosis; no geometry or job data is logged.
use crate::Instant;

pub(crate) struct Phase {
    name: &'static str,
    started: Instant,
    enabled: bool,
}

impl Phase {
    pub(crate) fn new(name: &'static str) -> Self {
        let enabled = std::env::var_os("BETA_NEST_PROFILE").is_some();
        if enabled {
            eprintln!("nest_phase {name} begin");
        }
        Self {
            name,
            started: Instant::now(),
            enabled,
        }
    }
}

impl Drop for Phase {
    fn drop(&mut self) {
        if self.enabled {
            eprintln!(
                "nest_phase {} {:.6}s",
                self.name,
                self.started.elapsed().as_secs_f64()
            );
        }
    }
}
