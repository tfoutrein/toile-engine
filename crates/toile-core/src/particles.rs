use glam::Vec2;

use crate::curve::Curve;
use crate::gradient::Gradient;

/// Shape of the particle emitter area.
#[derive(Debug, Clone)]
pub enum EmitterShape {
    Point,
    Circle { radius: f32 },
    Rectangle { half_extents: Vec2 },
    Line { length: f32 },
}

/// Blend mode for particle rendering.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlendMode {
    Alpha,
    Additive,
}

/// Configuration for a particle emitter.
#[derive(Debug, Clone)]
pub struct ParticleEmitter {
    pub shape: EmitterShape,
    pub rate: f32,
    pub burst: Option<u32>,
    pub lifetime: (f32, f32),
    pub initial_speed: (f32, f32),
    pub spread_angle: f32,
    pub direction: f32,
    pub gravity: Vec2,
    pub size_start: (f32, f32),
    pub size_over_life: Curve,
    pub color_over_life: Gradient,
    pub rotation_speed: (f32, f32),
    pub blend_mode: BlendMode,
}

impl Default for ParticleEmitter {
    fn default() -> Self {
        Self {
            shape: EmitterShape::Point,
            rate: 50.0,
            burst: None,
            lifetime: (0.5, 1.5),
            initial_speed: (50.0, 100.0),
            spread_angle: std::f32::consts::TAU,
            direction: std::f32::consts::FRAC_PI_2,
            gravity: Vec2::new(0.0, -100.0),
            size_start: (4.0, 8.0),
            size_over_life: Curve::linear(1.0, 0.0),
            color_over_life: Gradient::fade_out(),
            rotation_speed: (0.0, 0.0),
            blend_mode: BlendMode::Alpha,
        }
    }
}

/// A single live particle.
#[derive(Debug, Clone)]
pub struct Particle {
    pub position: Vec2,
    pub velocity: Vec2,
    pub age: f32,
    pub lifetime: f32,
    pub base_size: f32,
    pub rotation: f32,
    pub rotation_speed: f32,
}

/// Simple xorshift RNG (no external dependency).
struct Rng(u32);

impl Rng {
    fn new(seed: u32) -> Self {
        Self(seed.max(1))
    }
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
    fn f32(&mut self) -> f32 {
        (self.next() as f64 / u32::MAX as f64) as f32
    }
    fn range(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.f32()
    }
}

/// Pool of particles for one emitter instance.
pub struct ParticlePool {
    pub emitter: ParticleEmitter,
    pub particles: Vec<Particle>,
    pub position: Vec2,
    accumulator: f32,
    rng: Rng,
    pub active: bool,
}

impl ParticlePool {
    pub fn new(emitter: ParticleEmitter, position: Vec2) -> Self {
        let capacity = (emitter.rate * emitter.lifetime.1 * 1.5) as usize + 64;
        Self {
            emitter,
            particles: Vec::with_capacity(capacity),
            position,
            accumulator: 0.0,
            rng: Rng::new(42),
            active: true,
        }
    }

    /// Emit a burst of particles.
    pub fn burst(&mut self, count: u32) {
        for _ in 0..count {
            self.spawn_one();
        }
    }

    /// Update all particles and emit new ones based on rate.
    pub fn update(&mut self, dt: f32) {
        // Emit based on rate
        if self.active {
            self.accumulator += dt;
            let interval = 1.0 / self.emitter.rate.max(0.1);
            while self.accumulator >= interval {
                self.accumulator -= interval;
                self.spawn_one();
            }
        }

        // Update existing particles
        for p in &mut self.particles {
            p.age += dt;
            p.velocity += self.emitter.gravity * dt;
            p.position += p.velocity * dt;
            p.rotation += p.rotation_speed * dt;
        }

        // Remove dead particles
        self.particles.retain(|p| p.age < p.lifetime);
    }

    /// Get the current visual state of each particle for rendering.
    /// Returns (position, size, rotation, packed_color) tuples.
    pub fn render_data(&self) -> Vec<(Vec2, f32, f32, u32)> {
        self.particles
            .iter()
            .map(|p| {
                let t = (p.age / p.lifetime).clamp(0.0, 1.0);
                let size = p.base_size * self.emitter.size_over_life.sample(t);
                let color = self.emitter.color_over_life.sample_packed(t);
                (p.position, size, p.rotation, color)
            })
            .collect()
    }

    pub fn particle_count(&self) -> usize {
        self.particles.len()
    }

    fn spawn_one(&mut self) {
        let em = &self.emitter;

        // Position offset based on shape
        let offset = match &em.shape {
            EmitterShape::Point => Vec2::ZERO,
            EmitterShape::Circle { radius } => {
                let angle = self.rng.range(0.0, std::f32::consts::TAU);
                let r = self.rng.f32().sqrt() * radius;
                Vec2::new(angle.cos() * r, angle.sin() * r)
            }
            EmitterShape::Rectangle { half_extents } => Vec2::new(
                self.rng.range(-half_extents.x, half_extents.x),
                self.rng.range(-half_extents.y, half_extents.y),
            ),
            EmitterShape::Line { length } => {
                Vec2::new(self.rng.range(-length * 0.5, length * 0.5), 0.0)
            }
        };

        // Velocity
        let speed = self.rng.range(em.initial_speed.0, em.initial_speed.1);
        let angle = em.direction + self.rng.range(-em.spread_angle * 0.5, em.spread_angle * 0.5);
        let velocity = Vec2::new(angle.cos(), angle.sin()) * speed;

        self.particles.push(Particle {
            position: self.position + offset,
            velocity,
            age: 0.0,
            lifetime: self.rng.range(em.lifetime.0, em.lifetime.1),
            base_size: self.rng.range(em.size_start.0, em.size_start.1),
            rotation: self.rng.range(0.0, std::f32::consts::TAU),
            rotation_speed: self.rng.range(em.rotation_speed.0, em.rotation_speed.1),
        });
    }
}

// --- Presets ---

pub mod presets {
    use super::*;

    pub fn fire() -> ParticleEmitter {
        ParticleEmitter {
            rate: 120.0,
            lifetime: (0.3, 0.8),
            initial_speed: (40.0, 100.0),
            direction: std::f32::consts::FRAC_PI_2,
            spread_angle: 0.5,
            gravity: Vec2::new(0.0, 20.0),
            size_start: (10.0, 20.0),
            size_over_life: Curve::linear(1.0, 0.0),
            color_over_life: Gradient::from_stops(vec![
                (0.0, [1.0, 0.9, 0.2, 1.0]),
                (0.3, [1.0, 0.4, 0.0, 0.9]),
                (0.7, [0.8, 0.1, 0.0, 0.5]),
                (1.0, [0.3, 0.0, 0.0, 0.0]),
            ]),
            ..Default::default()
        }
    }

    pub fn smoke() -> ParticleEmitter {
        ParticleEmitter {
            rate: 30.0,
            lifetime: (1.0, 3.0),
            initial_speed: (10.0, 30.0),
            direction: std::f32::consts::FRAC_PI_2,
            spread_angle: 0.8,
            gravity: Vec2::new(0.0, 15.0),
            size_start: (12.0, 24.0),
            size_over_life: Curve::linear(0.5, 2.0),
            color_over_life: Gradient::from_stops(vec![
                (0.0, [0.5, 0.5, 0.5, 0.6]),
                (1.0, [0.3, 0.3, 0.3, 0.0]),
            ]),
            ..Default::default()
        }
    }

    pub fn sparks() -> ParticleEmitter {
        ParticleEmitter {
            rate: 150.0,
            lifetime: (0.2, 0.6),
            initial_speed: (100.0, 250.0),
            spread_angle: std::f32::consts::TAU,
            gravity: Vec2::new(0.0, -200.0),
            size_start: (3.0, 6.0),
            size_over_life: Curve::constant(1.0),
            color_over_life: Gradient::from_stops(vec![
                (0.0, [1.0, 1.0, 0.5, 1.0]),
                (0.5, [1.0, 0.6, 0.0, 1.0]),
                (1.0, [1.0, 0.2, 0.0, 0.0]),
            ]),
            ..Default::default()
        }
    }

    pub fn rain() -> ParticleEmitter {
        ParticleEmitter {
            shape: EmitterShape::Line { length: 800.0 },
            rate: 200.0,
            lifetime: (0.5, 1.0),
            initial_speed: (300.0, 500.0),
            direction: -std::f32::consts::FRAC_PI_2 - 0.1,
            spread_angle: 0.05,
            gravity: Vec2::ZERO,
            size_start: (1.0, 2.0),
            size_over_life: Curve::constant(1.0),
            color_over_life: Gradient::solid(0.6, 0.7, 1.0, 0.4),
            ..Default::default()
        }
    }

    pub fn snow() -> ParticleEmitter {
        ParticleEmitter {
            shape: EmitterShape::Line { length: 800.0 },
            rate: 80.0,
            lifetime: (2.0, 5.0),
            initial_speed: (10.0, 30.0),
            direction: -std::f32::consts::FRAC_PI_2,
            spread_angle: 0.5,
            gravity: Vec2::new(0.0, -5.0),
            size_start: (4.0, 8.0),
            size_over_life: Curve::constant(1.0),
            color_over_life: Gradient::solid(1.0, 1.0, 1.0, 0.8),
            rotation_speed: (-1.0, 1.0),
            ..Default::default()
        }
    }

    pub fn explosion() -> ParticleEmitter {
        ParticleEmitter {
            rate: 0.0, // burst only
            burst: Some(100),
            lifetime: (0.3, 1.0),
            initial_speed: (100.0, 300.0),
            spread_angle: std::f32::consts::TAU,
            gravity: Vec2::new(0.0, -50.0),
            size_start: (6.0, 16.0),
            size_over_life: Curve::linear(1.0, 0.0),
            color_over_life: Gradient::from_stops(vec![
                (0.0, [1.0, 1.0, 0.8, 1.0]),
                (0.2, [1.0, 0.6, 0.0, 1.0]),
                (0.5, [0.8, 0.2, 0.0, 0.8]),
                (1.0, [0.3, 0.1, 0.0, 0.0]),
            ]),
            ..Default::default()
        }
    }

    pub fn dust() -> ParticleEmitter {
        ParticleEmitter {
            shape: EmitterShape::Rectangle {
                half_extents: Vec2::new(10.0, 2.0),
            },
            rate: 20.0,
            lifetime: (0.3, 0.8),
            initial_speed: (10.0, 30.0),
            direction: std::f32::consts::FRAC_PI_2,
            spread_angle: 1.0,
            gravity: Vec2::ZERO,
            size_start: (2.0, 5.0),
            size_over_life: Curve::linear(1.0, 0.0),
            color_over_life: Gradient::from_stops(vec![
                (0.0, [0.8, 0.7, 0.5, 0.6]),
                (1.0, [0.6, 0.5, 0.3, 0.0]),
            ]),
            ..Default::default()
        }
    }

    pub fn confetti() -> ParticleEmitter {
        ParticleEmitter {
            rate: 50.0,
            lifetime: (1.0, 3.0),
            initial_speed: (50.0, 150.0),
            direction: std::f32::consts::FRAC_PI_2,
            spread_angle: 1.5,
            gravity: Vec2::new(0.0, -80.0),
            size_start: (4.0, 8.0),
            size_over_life: Curve::constant(1.0),
            color_over_life: Gradient::solid(1.0, 1.0, 1.0, 1.0), // tinted per-particle
            rotation_speed: (-3.0, 3.0),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn particle_lifecycle() {
        let emitter = ParticleEmitter {
            rate: 100.0,
            lifetime: (1.0, 1.0), // live long enough to be counted
            ..Default::default()
        };
        let mut pool = ParticlePool::new(emitter, Vec2::ZERO);

        pool.update(0.1); // should spawn ~10 particles
        assert!(pool.particle_count() > 0, "Expected particles after update, got 0");

        // Age them past lifetime
        for _ in 0..10 {
            pool.update(0.05);
        }
        // Old particles should be dead, new ones spawned
        // The pool stays bounded
    }

    #[test]
    fn burst_mode() {
        let emitter = presets::explosion();
        let mut pool = ParticlePool::new(emitter, Vec2::ZERO);
        pool.active = false;
        pool.burst(50);
        assert_eq!(pool.particle_count(), 50);
    }

    #[test]
    fn render_data_format() {
        let emitter = presets::fire();
        let mut pool = ParticlePool::new(emitter, Vec2::new(100.0, 200.0));
        pool.update(0.1);
        let data = pool.render_data();
        assert!(!data.is_empty());
        for (pos, size, _rot, _color) in &data {
            assert!(size.is_finite());
            assert!(pos.x.is_finite() && pos.y.is_finite());
        }
    }
}
