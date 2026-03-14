use std::time::{Duration, Instant};

/// Fixed-timestep game clock (Glenn Fiedler's "Fix Your Timestep" pattern).
///
/// Decouples the physics/logic update rate from the render rate.
/// The accumulator stores leftover time between frames, and the
/// `advance()` method returns how many fixed ticks to execute
/// plus an interpolation alpha for smooth rendering.
pub struct GameClock {
    fixed_dt: Duration,
    max_frame_time: Duration,
    accumulator: Duration,
    previous_time: Instant,
    total_time: Duration,
    tick_count: u64,
}

impl GameClock {
    pub fn new(update_hz: u32) -> Self {
        Self {
            fixed_dt: Duration::from_secs_f64(1.0 / update_hz as f64),
            max_frame_time: Duration::from_millis(250), // death spiral guard
            accumulator: Duration::ZERO,
            previous_time: Instant::now(),
            total_time: Duration::ZERO,
            tick_count: 0,
        }
    }

    /// Advance the clock by the elapsed wall time since the last call.
    ///
    /// Returns `(ticks, alpha)`:
    /// - `ticks`: number of fixed-step updates to run this frame
    /// - `alpha`: interpolation factor (0.0..1.0) for rendering between states
    pub fn advance(&mut self) -> (u32, f64) {
        let now = Instant::now();
        let mut frame_time = now - self.previous_time;
        self.previous_time = now;

        if frame_time > self.max_frame_time {
            frame_time = self.max_frame_time;
        }

        self.accumulator += frame_time;

        let mut ticks = 0u32;
        while self.accumulator >= self.fixed_dt {
            self.accumulator -= self.fixed_dt;
            self.total_time += self.fixed_dt;
            self.tick_count += 1;
            ticks += 1;
        }

        let alpha = self.accumulator.as_secs_f64() / self.fixed_dt.as_secs_f64();
        (ticks, alpha)
    }

    pub fn fixed_dt(&self) -> Duration {
        self.fixed_dt
    }

    pub fn fixed_dt_secs(&self) -> f64 {
        self.fixed_dt.as_secs_f64()
    }

    pub fn total_time(&self) -> Duration {
        self.total_time
    }

    pub fn tick_count(&self) -> u64 {
        self.tick_count
    }
}
