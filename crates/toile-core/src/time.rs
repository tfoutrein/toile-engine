use std::time::{Duration, Instant};

/// Fixed-timestep game clock (Glenn Fiedler's "Fix Your Timestep" pattern).
///
/// Decouples the physics/logic update rate from the render rate.
/// Also tracks FPS and frame time for debug overlay.
pub struct GameClock {
    fixed_dt: Duration,
    max_frame_time: Duration,
    accumulator: Duration,
    previous_time: Instant,
    total_time: Duration,
    tick_count: u64,
    frame_time: Duration,
    fps_accumulator: Duration,
    fps_frame_count: u32,
    fps: f64,
}

impl GameClock {
    pub fn new(update_hz: u32) -> Self {
        Self {
            fixed_dt: Duration::from_secs_f64(1.0 / update_hz as f64),
            max_frame_time: Duration::from_millis(250),
            accumulator: Duration::ZERO,
            previous_time: Instant::now(),
            total_time: Duration::ZERO,
            tick_count: 0,
            frame_time: Duration::ZERO,
            fps_accumulator: Duration::ZERO,
            fps_frame_count: 0,
            fps: 0.0,
        }
    }

    /// Advance the clock. Returns `(ticks, alpha)`.
    pub fn advance(&mut self) -> (u32, f64) {
        let now = Instant::now();
        let mut frame_time = now - self.previous_time;
        self.previous_time = now;

        if frame_time > self.max_frame_time {
            frame_time = self.max_frame_time;
        }

        self.frame_time = frame_time;
        self.accumulator += frame_time;

        // FPS sampling (updated once per second)
        self.fps_frame_count += 1;
        self.fps_accumulator += frame_time;
        if self.fps_accumulator >= Duration::from_secs(1) {
            self.fps = self.fps_frame_count as f64 / self.fps_accumulator.as_secs_f64();
            self.fps_frame_count = 0;
            self.fps_accumulator = Duration::ZERO;
        }

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

    pub fn fps(&self) -> f64 {
        self.fps
    }

    pub fn frame_time_ms(&self) -> f64 {
        self.frame_time.as_secs_f64() * 1000.0
    }
}
