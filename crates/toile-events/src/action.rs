use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ActionKind {
    /// Set entity position.
    SetPosition { x: f64, y: f64 },
    /// Move at an angle (degrees) at a given speed (pixels/sec).
    MoveAtAngle { angle: f64, speed: f64 },
    /// Move toward a named entity at a given speed.
    MoveToward { target: String, speed: f64 },
    /// Set a user variable.
    SetVariable { name: String, value: f64 },
    /// Add to a user variable.
    AddToVariable { name: String, amount: f64 },
    /// Destroy this entity.
    Destroy,
    /// Spawn an entity from a prefab.
    SpawnObject { prefab: String, x: f64, y: f64 },
    /// Play a sound effect.
    PlaySound { sound: String },
    /// Play an animation clip. Takes priority over the auto state machine until the
    /// clip finishes (or ResumeAutoAnimation is called). ADR-038.
    PlayAnimation { anim: String },
    /// Release a scripted PlayAnimation lock so the auto state machine resumes
    /// driving idle/walk/jump. ADR-038 Phase 5.
    ResumeAutoAnimation,
    /// Switch to another scene.
    GoToScene { scene: String },
    /// Log a message (for debugging).
    Log { message: String },
}
