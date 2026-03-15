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
    /// Play an animation clip.
    PlayAnimation { anim: String },
    /// Switch to another scene.
    GoToScene { scene: String },
    /// Log a message (for debugging).
    Log { message: String },
}
