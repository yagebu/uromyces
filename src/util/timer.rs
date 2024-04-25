use std::time::Instant;

/// A simple timer that can be used to log the time that certain steps took.
pub(crate) struct SimpleTimer {
    time: Instant,
}

impl SimpleTimer {
    pub fn new() -> Self {
        Self {
            time: Instant::now(),
        }
    }

    /// Reset the timer.
    fn reset(&mut self) {
        self.time = Instant::now();
    }

    /// Log the elapsed time since init or the last log and reset.
    pub fn log_elapsed(&mut self, step: &str) {
        let elapsed = self.time.elapsed();
        log::info!(
            "{:>4}.{:03}ms - {step}",
            elapsed.as_millis(),
            elapsed.as_micros() % 1000
        );
        self.reset();
    }
}
