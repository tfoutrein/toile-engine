use serde::{Deserialize, Serialize};

use crate::action::ActionKind;
use crate::condition::ConditionKind;

/// A named collection of events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSheet {
    pub name: String,
    pub events: Vec<Event>,
}

impl EventSheet {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            events: Vec::new(),
        }
    }
}

/// A single event: when all conditions are met, execute actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub conditions: Vec<Condition>,
    pub actions: Vec<Action>,
    #[serde(default)]
    pub sub_events: Vec<Event>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

impl Event {
    pub fn new(conditions: Vec<Condition>, actions: Vec<Action>) -> Self {
        Self {
            conditions,
            actions,
            sub_events: Vec::new(),
            enabled: true,
        }
    }
}

/// A condition to check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub kind: ConditionKind,
    #[serde(default)]
    pub negated: bool,
}

impl Condition {
    pub fn new(kind: ConditionKind) -> Self {
        Self {
            kind,
            negated: false,
        }
    }

    pub fn negated(kind: ConditionKind) -> Self {
        Self {
            kind,
            negated: true,
        }
    }
}

/// An action to execute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub kind: ActionKind,
}

impl Action {
    pub fn new(kind: ActionKind) -> Self {
        Self { kind }
    }
}
