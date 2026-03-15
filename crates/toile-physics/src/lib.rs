use std::collections::HashMap;

use glam::Vec2;
use rapier2d::prelude::*;

/// Body type for physics entities.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BodyType {
    Static,
    Dynamic,
    Kinematic,
}

/// Definition for creating a physics body.
#[derive(Debug, Clone)]
pub struct BodyDef {
    pub body_type: BodyType,
    pub position: Vec2,
    pub rotation: f32,
    pub mass: f32,
    pub friction: f32,
    pub restitution: f32,
    pub linear_damping: f32,
    pub angular_damping: f32,
    pub gravity_scale: f32,
    pub fixed_rotation: bool,
}

impl Default for BodyDef {
    fn default() -> Self {
        Self {
            body_type: BodyType::Dynamic,
            position: Vec2::ZERO,
            rotation: 0.0,
            mass: 1.0,
            friction: 0.5,
            restitution: 0.3,
            linear_damping: 0.0,
            angular_damping: 0.0,
            gravity_scale: 1.0,
            fixed_rotation: false,
        }
    }
}

/// Collider shape for physics.
#[derive(Debug, Clone)]
pub enum PhysicsShape {
    Box { half_w: f32, half_h: f32 },
    Circle { radius: f32 },
    Capsule { half_h: f32, radius: f32 },
}

/// Handle referencing a physics body (opaque to the user).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PhysicsBodyHandle(pub u64);

/// The physics world — wraps Rapier2D pipeline and body/collider sets.
pub struct PhysicsWorld {
    gravity: Vec2,
    integration_params: IntegrationParameters,
    physics_pipeline: PhysicsPipeline,
    island_manager: IslandManager,
    broad_phase: BroadPhaseBvh,
    narrow_phase: NarrowPhase,
    rigid_body_set: RigidBodySet,
    collider_set: ColliderSet,
    impulse_joint_set: ImpulseJointSet,
    multibody_joint_set: MultibodyJointSet,
    ccd_solver: CCDSolver,

    // Mapping: user handle <-> rapier handle
    handle_to_rapier: HashMap<PhysicsBodyHandle, RigidBodyHandle>,
    rapier_to_handle: HashMap<RigidBodyHandle, PhysicsBodyHandle>,
    next_handle: u64,
}

impl PhysicsWorld {
    pub fn new(gravity: Vec2) -> Self {
        Self {
            gravity,
            integration_params: IntegrationParameters::default(),
            physics_pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(),
            broad_phase: BroadPhaseBvh::new(),
            narrow_phase: NarrowPhase::new(),
            rigid_body_set: RigidBodySet::new(),
            collider_set: ColliderSet::new(),
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            handle_to_rapier: HashMap::new(),
            rapier_to_handle: HashMap::new(),
            next_handle: 1,
        }
    }

    /// Add a physics body with a collider. Returns a handle.
    pub fn add_body(&mut self, def: &BodyDef, shape: &PhysicsShape) -> PhysicsBodyHandle {
        let rb = match def.body_type {
            BodyType::Static => RigidBodyBuilder::fixed(),
            BodyType::Dynamic => RigidBodyBuilder::dynamic(),
            BodyType::Kinematic => RigidBodyBuilder::kinematic_position_based(),
        }
        .translation(vector![def.position.x, def.position.y].into())
        .rotation(def.rotation)
        .linear_damping(def.linear_damping)
        .angular_damping(def.angular_damping)
        .gravity_scale(def.gravity_scale)
        .locked_axes(if def.fixed_rotation {
            LockedAxes::ROTATION_LOCKED
        } else {
            LockedAxes::empty()
        })
        .build();

        let rb_handle = self.rigid_body_set.insert(rb);

        let collider = match shape {
            PhysicsShape::Box { half_w, half_h } => {
                ColliderBuilder::cuboid(*half_w, *half_h)
            }
            PhysicsShape::Circle { radius } => {
                ColliderBuilder::ball(*radius)
            }
            PhysicsShape::Capsule { half_h, radius } => {
                ColliderBuilder::capsule_y(*half_h, *radius)
            }
        }
        .friction(def.friction)
        .restitution(def.restitution)
        .build();

        self.collider_set
            .insert_with_parent(collider, rb_handle, &mut self.rigid_body_set);

        let handle = PhysicsBodyHandle(self.next_handle);
        self.next_handle += 1;
        self.handle_to_rapier.insert(handle, rb_handle);
        self.rapier_to_handle.insert(rb_handle, handle);

        handle
    }

    /// Remove a physics body.
    pub fn remove_body(&mut self, handle: PhysicsBodyHandle) {
        if let Some(rb_handle) = self.handle_to_rapier.remove(&handle) {
            self.rapier_to_handle.remove(&rb_handle);
            self.rigid_body_set.remove(
                rb_handle,
                &mut self.island_manager,
                &mut self.collider_set,
                &mut self.impulse_joint_set,
                &mut self.multibody_joint_set,
                true,
            );
        }
    }

    /// Step the physics simulation.
    pub fn step(&mut self, dt: f32) {
        self.integration_params.dt = dt;

        let gravity_vec = vector![self.gravity.x, self.gravity.y];
        self.physics_pipeline.step(
            gravity_vec.into(),
            &self.integration_params,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            &(),
            &(),
        );
    }

    /// Get the position and rotation of a body.
    pub fn get_transform(&self, handle: PhysicsBodyHandle) -> Option<(Vec2, f32)> {
        let rb_handle = self.handle_to_rapier.get(&handle)?;
        let rb = self.rigid_body_set.get(*rb_handle)?;
        let pos = rb.translation();
        let rot = rb.rotation().angle();
        Some((Vec2::new(pos.x, pos.y), rot))
    }

    /// Set the position of a kinematic body.
    pub fn set_position(&mut self, handle: PhysicsBodyHandle, pos: Vec2) {
        if let Some(rb_handle) = self.handle_to_rapier.get(&handle) {
            if let Some(rb) = self.rigid_body_set.get_mut(*rb_handle) {
                rb.set_translation(vector![pos.x, pos.y].into(), true);
            }
        }
    }

    /// Apply a force to a dynamic body.
    pub fn apply_force(&mut self, handle: PhysicsBodyHandle, force: Vec2) {
        if let Some(rb_handle) = self.handle_to_rapier.get(&handle) {
            if let Some(rb) = self.rigid_body_set.get_mut(*rb_handle) {
                rb.add_force(vector![force.x, force.y].into(), true);
            }
        }
    }

    /// Apply an impulse to a dynamic body.
    pub fn apply_impulse(&mut self, handle: PhysicsBodyHandle, impulse: Vec2) {
        if let Some(rb_handle) = self.handle_to_rapier.get(&handle) {
            if let Some(rb) = self.rigid_body_set.get_mut(*rb_handle) {
                rb.apply_impulse(vector![impulse.x, impulse.y].into(), true);
            }
        }
    }

    /// Set velocity of a body.
    pub fn set_velocity(&mut self, handle: PhysicsBodyHandle, vel: Vec2) {
        if let Some(rb_handle) = self.handle_to_rapier.get(&handle) {
            if let Some(rb) = self.rigid_body_set.get_mut(*rb_handle) {
                rb.set_linvel(vector![vel.x, vel.y].into(), true);
            }
        }
    }

    /// Get all body handles and their transforms (for syncing with ECS).
    pub fn all_transforms(&self) -> Vec<(PhysicsBodyHandle, Vec2, f32)> {
        self.handle_to_rapier
            .iter()
            .filter_map(|(handle, rb_handle)| {
                let rb = self.rigid_body_set.get(*rb_handle)?;
                let pos = rb.translation();
                let rot = rb.rotation().angle();
                Some((*handle, Vec2::new(pos.x, pos.y), rot))
            })
            .collect()
    }

    pub fn body_count(&self) -> usize {
        self.rigid_body_set.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_physics() {
        let mut world = PhysicsWorld::new(Vec2::new(0.0, -9.81));

        // Static floor
        world.add_body(
            &BodyDef {
                body_type: BodyType::Static,
                position: Vec2::new(0.0, -5.0),
                ..Default::default()
            },
            &PhysicsShape::Box {
                half_w: 50.0,
                half_h: 0.5,
            },
        );

        // Dynamic box above
        let box_h = world.add_body(
            &BodyDef {
                body_type: BodyType::Dynamic,
                position: Vec2::new(0.0, 5.0),
                ..Default::default()
            },
            &PhysicsShape::Box {
                half_w: 0.5,
                half_h: 0.5,
            },
        );

        // Step simulation
        for _ in 0..100 {
            world.step(1.0 / 60.0);
        }

        // Box should have fallen
        let (pos, _rot) = world.get_transform(box_h).unwrap();
        assert!(pos.y < 5.0, "Box should have fallen, y={}", pos.y);
        assert!(pos.y > -6.0, "Box should rest on floor, y={}", pos.y);
    }

    #[test]
    fn add_remove_body() {
        let mut world = PhysicsWorld::new(Vec2::new(0.0, -9.81));
        assert_eq!(world.body_count(), 0);

        let h = world.add_body(&BodyDef::default(), &PhysicsShape::Circle { radius: 1.0 });
        assert_eq!(world.body_count(), 1);

        world.remove_body(h);
        assert_eq!(world.body_count(), 0);
    }
}
