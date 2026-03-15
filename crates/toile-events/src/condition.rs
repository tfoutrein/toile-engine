use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ConditionKind {
    /// Fires once when the entity is first evaluated.
    OnCreate,
    /// Fires every frame.
    EveryTick,
    /// Fires on the frame the key is first pressed.
    OnKeyPressed { key: String },
    /// Fires on the frame the key is released.
    OnKeyReleased { key: String },
    /// True while the key is held down.
    OnKeyDown { key: String },
    /// Fires on mouse button click.
    OnMouseClick { button: String },
    /// True when colliding with an entity matching the tag (entity name).
    OnCollisionWith { tag: String },
    /// Compare a user variable against a value.
    IfVariable { name: String, op: CompareOp, value: f64 },
    /// Fires periodically at the given interval (seconds).
    EveryNSeconds { interval: f64 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompareOp {
    #[serde(rename = "==")]
    Equal,
    #[serde(rename = "!=")]
    NotEqual,
    #[serde(rename = "<")]
    Less,
    #[serde(rename = "<=")]
    LessOrEqual,
    #[serde(rename = ">")]
    Greater,
    #[serde(rename = ">=")]
    GreaterOrEqual,
}

impl CompareOp {
    pub fn test(&self, a: f64, b: f64) -> bool {
        match self {
            CompareOp::Equal => (a - b).abs() < 0.0001,
            CompareOp::NotEqual => (a - b).abs() >= 0.0001,
            CompareOp::Less => a < b,
            CompareOp::LessOrEqual => a <= b,
            CompareOp::Greater => a > b,
            CompareOp::GreaterOrEqual => a >= b,
        }
    }
}
