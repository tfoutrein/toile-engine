use glam::Vec2;
use serde::{Deserialize, Serialize};
use crate::types::{BehaviorInput, EntityState, SolidCheck};

/// Platformer character controller with game-feel tuning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformConfig {
    pub gravity: f32,
    pub jump_force: f32,
    pub max_speed: f32,
    pub acceleration: f32,
    pub deceleration: f32,
    pub coyote_time: f32,
    pub jump_buffer: f32,
    pub max_jumps: u32,
}

impl Default for PlatformConfig {
    fn default() -> Self {
        Self {
            gravity: 800.0,
            jump_force: 400.0,
            max_speed: 200.0,
            acceleration: 1500.0,
            deceleration: 1200.0,
            coyote_time: 0.08,
            jump_buffer: 0.12,
            max_jumps: 1,
        }
    }
}

/// Runtime state for the Platform behavior.
#[derive(Debug, Default)]
pub struct PlatformState {
    pub coyote_timer: f32,
    pub jump_buffer_timer: f32,
    pub jumps_remaining: u32,
    jump_consumed: bool, // prevents re-filling buffer on same press
}

/// Update a platformer entity. Returns true if the entity is on the ground.
pub fn update(
    config: &PlatformConfig,
    state: &mut PlatformState,
    entity: &mut EntityState,
    input: &BehaviorInput,
    solid_check: &SolidCheck,
    dt: f32,
) {
    // Horizontal movement
    let target_vx = if input.right {
        config.max_speed
    } else if input.left {
        -config.max_speed
    } else {
        0.0
    };

    if target_vx.abs() > 0.01 {
        // Accelerate
        let diff = target_vx - entity.velocity.x;
        let accel = config.acceleration * dt * diff.signum();
        if accel.abs() > diff.abs() {
            entity.velocity.x = target_vx;
        } else {
            entity.velocity.x += accel;
        }
    } else {
        // Decelerate
        let decel = config.deceleration * dt;
        if entity.velocity.x.abs() < decel {
            entity.velocity.x = 0.0;
        } else {
            entity.velocity.x -= decel * entity.velocity.x.signum();
        }
    }

    // Coyote time
    if entity.on_ground {
        state.coyote_timer = config.coyote_time;
        state.jumps_remaining = config.max_jumps;
    } else {
        state.coyote_timer -= dt;
    }

    // Jump buffer — only fill once per press
    if input.jump_pressed && !state.jump_consumed {
        state.jump_buffer_timer = config.jump_buffer;
        state.jump_consumed = true;
        log::trace!("JUMP INPUT: pressed, buffer={:.3}", state.jump_buffer_timer);
    }
    if !input.jump_pressed && !input.jump_down {
        state.jump_consumed = false; // reset when key is fully released
    }
    state.jump_buffer_timer -= dt;

    // Jump — check conditions
    let can_jump = state.jump_buffer_timer > 0.0
        && (state.coyote_timer > 0.0 || state.jumps_remaining > 0);

    if input.jump_pressed && !can_jump {
        log::debug!(
            "JUMP BLOCKED: buffer={:.3}, coyote={:.3}, jumps_left={}, on_ground={}",
            state.jump_buffer_timer, state.coyote_timer,
            state.jumps_remaining, entity.on_ground
        );
    }

    if can_jump {
        log::debug!(
            "JUMP! coyote={:.3}, buffer={:.3}, on_ground={}, vel_y={:.1}",
            state.coyote_timer, state.jump_buffer_timer,
            entity.on_ground, entity.velocity.y
        );
        entity.velocity.y = config.jump_force;
        state.coyote_timer = 0.0;
        state.jump_buffer_timer = 0.0;
        if !entity.on_ground {
            state.jumps_remaining = state.jumps_remaining.saturating_sub(1);
        }
        entity.on_ground = false;
    }

    // Variable jump height: cut velocity when releasing jump
    if !input.jump_down && entity.velocity.y > 0.0 {
        entity.velocity.y *= 0.5;
    }

    // Gravity
    entity.velocity.y -= config.gravity * dt;

    // Move X — use a slightly shorter hitbox (shrink bottom by 2px)
    // to avoid detecting the surface the player is standing on
    let half = entity.size * 0.5;
    entity.position.x += entity.velocity.x * dt;
    let shrunk_half = Vec2::new(half.x, half.y - 2.0);
    let check_pos = Vec2::new(entity.position.x, entity.position.y + 1.0);
    if solid_check(check_pos, shrunk_half) {
        entity.position.x -= entity.velocity.x * dt;
        entity.velocity.x = 0.0;
    }

    // Move Y — use full hitbox
    let was_on_ground = entity.on_ground;
    entity.position.y += entity.velocity.y * dt;
    entity.on_ground = false;
    if solid_check(entity.position, half) {
        if entity.velocity.y < 0.0 {
            entity.on_ground = true;
        }
        entity.position.y -= entity.velocity.y * dt;
        entity.velocity.y = 0.0;
    }

    if was_on_ground != entity.on_ground {
        log::trace!("GROUND STATE: {} -> {}", was_on_ground, entity.on_ground);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_solid(_pos: Vec2, _half: Vec2) -> bool { false }

    #[test]
    fn gravity_pulls_down() {
        let config = PlatformConfig::default();
        let mut state = PlatformState::default();
        let mut entity = EntityState {
            position: Vec2::new(0.0, 100.0),
            velocity: Vec2::ZERO,
            rotation: 0.0,
            on_ground: false,
            size: Vec2::new(32.0, 32.0),
            opacity: 1.0,
            alive: true,
        };
        let input = BehaviorInput {
            left: false, right: false, up: false, down: false,
            jump_pressed: false, jump_down: false,
        };

        let start_y = entity.position.y;
        for _ in 0..60 {
            update(&config, &mut state, &mut entity, &input, &no_solid, 1.0 / 60.0);
        }
        assert!(entity.position.y < start_y, "Entity should fall");
    }

    #[test]
    fn horizontal_movement() {
        let config = PlatformConfig::default();
        let mut state = PlatformState::default();
        let mut entity = EntityState {
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            rotation: 0.0,
            on_ground: true,
            size: Vec2::new(32.0, 32.0),
            opacity: 1.0,
            alive: true,
        };
        let input = BehaviorInput {
            left: false, right: true, up: false, down: false,
            jump_pressed: false, jump_down: false,
        };

        for _ in 0..30 {
            update(&config, &mut state, &mut entity, &input, &no_solid, 1.0 / 60.0);
        }
        assert!(entity.position.x > 0.0, "Entity should move right");
        assert!(entity.velocity.x > 0.0, "Velocity should be positive");
    }
}
