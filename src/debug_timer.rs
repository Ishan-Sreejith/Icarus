use std::time::{Duration, Instant};

pub struct DebugTimer {
    start_time: Instant,
    enabled: bool,
}

impl DebugTimer {
    pub fn new(enabled: bool) -> Self {
        Self {
            start_time: Instant::now(),
            enabled,
        }
    }

    pub fn phase(&self, phase_name: &str) -> PhaseTimer {
        PhaseTimer::new(phase_name, self.enabled)
    }

    pub fn total_time(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn print_total(&self, operation: &str) {
        if self.enabled {
            let elapsed = self.total_time();
            println!("[DEBUG] Total {} time: {:.2}ms", operation, elapsed.as_millis());
        }
    }
}

pub struct PhaseTimer {
    phase_name: String,
    start_time: Instant,
    enabled: bool,
}

impl PhaseTimer {
    fn new(phase_name: &str, enabled: bool) -> Self {
        if enabled {
            println!("[DEBUG] Starting phase: {}", phase_name);
        }
        Self {
            phase_name: phase_name.to_string(),
            start_time: Instant::now(),
            enabled,
        }
    }

    pub fn finish(self) {
        if self.enabled {
            let elapsed = self.start_time.elapsed();
            println!("[DEBUG] Phase '{}' completed in {:.2}ms",
                    self.phase_name, elapsed.as_millis());
        }
    }

    #[allow(dead_code)]
    pub fn finish_with_result<T>(self, result: &str) -> Self {
        if self.enabled {
            let elapsed = self.start_time.elapsed();
            println!("[DEBUG] Phase '{}' -> {} ({:.2}ms)",
                    self.phase_name, result, elapsed.as_millis());
        }
        self
    }
}

#[macro_export]
macro_rules! debug_time {
    ($timer:expr, $phase:expr, $code:block) => {{
        let phase_timer = $timer.phase($phase);
        let result = $code;
        phase_timer.finish();
        result
    }};
}
