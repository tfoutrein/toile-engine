//! Input Actions — abstraction layer mapping physical inputs to named actions.
//!
//! Game code queries actions ("jump", "move") instead of raw keys/buttons.
//! Each action can have multiple bindings (keyboard + gamepad + mouse).
//! Supports Button (digital), Axis (1D analog), and Vec2 (2D composite/stick).

use std::collections::HashMap;

use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::input::{Input, GamepadAxis, GamepadButton, MouseButton};
use winit::keyboard::KeyCode;

// ── Types ───────────────────────────────────────────────────────────────────

/// What kind of value an action produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    /// Digital on/off (pressed/released).
    Button,
    /// 1D analog value (-1.0 to 1.0).
    Axis,
    /// 2D direction (stick or WASD composite).
    Vec2,
}

/// A physical input source.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputSource {
    Key { key: String },
    MouseButton { button: String },
    GamepadButton { button: String },
    GamepadAxis { axis: String },
}

/// Role in a Vec2 composite (WASD → direction).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompositeRole {
    Up,
    Down,
    Left,
    Right,
}

/// A single binding from a physical input to an action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputBinding {
    pub source: InputSource,
    #[serde(default = "default_dead_zone")]
    pub dead_zone: f32,
    #[serde(default)]
    pub composite: Option<CompositeRole>,
}

fn default_dead_zone() -> f32 { 0.2 }

/// Definition of a named action with its bindings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputAction {
    pub name: String,
    pub action_type: ActionType,
    pub bindings: Vec<InputBinding>,
}

/// Computed state of an action for the current frame.
#[derive(Debug, Clone, Default)]
pub struct ActionState {
    pub pressed: bool,
    pub just_pressed: bool,
    pub just_released: bool,
    pub value: f32,
    pub vec2: Vec2,
    prev_pressed: bool,
}

// ── InputActionMap ──────────────────────────────────────────────────────────

/// The complete set of actions + their computed states.
pub struct InputActionMap {
    pub actions: Vec<InputAction>,
    states: HashMap<String, ActionState>,
}

impl InputActionMap {
    pub fn new(actions: Vec<InputAction>) -> Self {
        let states = actions.iter().map(|a| (a.name.clone(), ActionState::default())).collect();
        Self { actions, states }
    }

    /// Create with the default platformer/topdown bindings.
    pub fn with_defaults() -> Self {
        Self::new(default_actions())
    }

    /// Update all action states from raw Input. Call once per frame.
    pub fn update(&mut self, input: &Input) {
        for action in &self.actions {
            let state = self.states.entry(action.name.clone()).or_default();
            let prev = state.pressed;
            state.prev_pressed = prev;

            match action.action_type {
                ActionType::Button => {
                    let pressed = action.bindings.iter().any(|b| is_source_active(input, &b.source, b.dead_zone));
                    state.pressed = pressed;
                    state.just_pressed = pressed && !prev;
                    state.just_released = !pressed && prev;
                    state.value = if pressed { 1.0 } else { 0.0 };
                }
                ActionType::Axis => {
                    let mut val = 0.0_f32;
                    for b in &action.bindings {
                        val += get_source_value(input, &b.source, b.dead_zone);
                    }
                    val = val.clamp(-1.0, 1.0);
                    state.value = val;
                    state.pressed = val.abs() > 0.1;
                    state.just_pressed = state.pressed && !prev;
                    state.just_released = !state.pressed && prev;
                }
                ActionType::Vec2 => {
                    let mut v = Vec2::ZERO;
                    for b in &action.bindings {
                        match b.composite {
                            Some(CompositeRole::Up) => {
                                if is_source_active(input, &b.source, b.dead_zone) { v.y += 1.0; }
                            }
                            Some(CompositeRole::Down) => {
                                if is_source_active(input, &b.source, b.dead_zone) { v.y -= 1.0; }
                            }
                            Some(CompositeRole::Left) => {
                                if is_source_active(input, &b.source, b.dead_zone) { v.x -= 1.0; }
                            }
                            Some(CompositeRole::Right) => {
                                if is_source_active(input, &b.source, b.dead_zone) { v.x += 1.0; }
                            }
                            None => {
                                // Direct axis (stick) — match axis name to x or y
                                let val = get_source_value(input, &b.source, b.dead_zone);
                                if let InputSource::GamepadAxis { ref axis } = b.source {
                                    if axis.contains("X") || axis.ends_with("x") { v.x += val; }
                                    else if axis.contains("Y") || axis.ends_with("y") { v.y += val; }
                                }
                            }
                        }
                    }
                    // Clamp magnitude to 1.0
                    if v.length() > 1.0 { v = v.normalize(); }
                    state.vec2 = v;
                    state.pressed = v.length() > 0.1;
                    state.just_pressed = state.pressed && !prev;
                    state.just_released = !state.pressed && prev;
                    state.value = v.length();
                }
            }
        }
    }

    // ── Query API ──

    pub fn is_pressed(&self, action: &str) -> bool {
        self.states.get(action).is_some_and(|s| s.pressed)
    }

    pub fn is_just_pressed(&self, action: &str) -> bool {
        self.states.get(action).is_some_and(|s| s.just_pressed)
    }

    pub fn is_just_released(&self, action: &str) -> bool {
        self.states.get(action).is_some_and(|s| s.just_released)
    }

    pub fn get_value(&self, action: &str) -> f32 {
        self.states.get(action).map(|s| s.value).unwrap_or(0.0)
    }

    pub fn get_vec2(&self, action: &str) -> Vec2 {
        self.states.get(action).map(|s| s.vec2).unwrap_or(Vec2::ZERO)
    }

    pub fn state(&self, action: &str) -> Option<&ActionState> {
        self.states.get(action)
    }

    /// List all action names.
    pub fn action_names(&self) -> Vec<&str> {
        self.actions.iter().map(|a| a.name.as_str()).collect()
    }

    // ── Mutation API ──

    /// Add a new action.
    pub fn add_action(&mut self, action: InputAction) {
        self.states.insert(action.name.clone(), ActionState::default());
        self.actions.push(action);
    }

    /// Remove an action by name.
    pub fn remove_action(&mut self, name: &str) {
        self.actions.retain(|a| a.name != name);
        self.states.remove(name);
    }

    /// Add a binding to an existing action. Returns false if action not found.
    pub fn add_binding(&mut self, action_name: &str, binding: InputBinding) -> bool {
        if let Some(action) = self.actions.iter_mut().find(|a| a.name == action_name) {
            action.bindings.push(binding);
            true
        } else {
            false
        }
    }

    /// Remove a binding from an action by index.
    pub fn remove_binding(&mut self, action_name: &str, binding_index: usize) -> bool {
        if let Some(action) = self.actions.iter_mut().find(|a| a.name == action_name) {
            if binding_index < action.bindings.len() {
                action.bindings.remove(binding_index);
                return true;
            }
        }
        false
    }

    // ── Serialization ──

    /// Save actions to a JSON file.
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<(), String> {
        let json = serde_json::to_string_pretty(&self.actions)
            .map_err(|e| format!("Serialize error: {e}"))?;
        std::fs::write(path, json)
            .map_err(|e| format!("Write error: {e}"))?;
        Ok(())
    }

    /// Load actions from a JSON file. Falls back to defaults if file doesn't exist.
    pub fn load_from_file(path: &std::path::Path) -> Self {
        if path.exists() {
            if let Ok(json) = std::fs::read_to_string(path) {
                if let Ok(actions) = serde_json::from_str::<Vec<InputAction>>(&json) {
                    return Self::new(actions);
                }
            }
        }
        Self::with_defaults()
    }
}

// ── Source evaluation ───────────────────────────────────────────────────────

fn is_source_active(input: &Input, source: &InputSource, dead_zone: f32) -> bool {
    match source {
        InputSource::Key { key } => {
            if let Some(kc) = key_name_to_keycode(key) {
                input.is_key_down(kc)
            } else { false }
        }
        InputSource::MouseButton { button } => {
            let mb = match button.as_str() {
                "Left" | "left" => MouseButton::Left,
                "Right" | "right" => MouseButton::Right,
                "Middle" | "middle" => MouseButton::Middle,
                _ => return false,
            };
            input.is_mouse_down(mb)
        }
        InputSource::GamepadButton { button } => {
            if let Some(gb) = gamepad_button_from_name(button) {
                input.is_gamepad_button_down(0, gb)
            } else { false }
        }
        InputSource::GamepadAxis { axis } => {
            if let Some(ga) = gamepad_axis_from_name(axis) {
                input.gamepad_axis(0, ga).abs() > dead_zone
            } else { false }
        }
    }
}

fn get_source_value(input: &Input, source: &InputSource, dead_zone: f32) -> f32 {
    match source {
        InputSource::Key { key } => {
            if let Some(kc) = key_name_to_keycode(key) {
                if input.is_key_down(kc) { 1.0 } else { 0.0 }
            } else { 0.0 }
        }
        InputSource::MouseButton { .. } => {
            if is_source_active(input, source, dead_zone) { 1.0 } else { 0.0 }
        }
        InputSource::GamepadButton { button } => {
            if let Some(gb) = gamepad_button_from_name(button) {
                if input.is_gamepad_button_down(0, gb) { 1.0 } else { 0.0 }
            } else { 0.0 }
        }
        InputSource::GamepadAxis { axis } => {
            if let Some(ga) = gamepad_axis_from_name(axis) {
                let val = input.gamepad_axis(0, ga);
                if val.abs() < dead_zone { 0.0 } else { val }
            } else { 0.0 }
        }
    }
}

// ── Name → enum conversion ─────────────────────────────────────────────────

fn gamepad_button_from_name(name: &str) -> Option<GamepadButton> {
    match name {
        "South" | "A" | "Cross" => Some(GamepadButton::South),
        "East" | "B" | "Circle" => Some(GamepadButton::East),
        "West" | "X" | "Square" => Some(GamepadButton::West),
        "North" | "Y" | "Triangle" => Some(GamepadButton::North),
        "LeftShoulder" | "LB" | "L1" => Some(GamepadButton::LeftShoulder),
        "RightShoulder" | "RB" | "R1" => Some(GamepadButton::RightShoulder),
        "LeftTrigger" | "LT" | "L2" => Some(GamepadButton::LeftTrigger),
        "RightTrigger" | "RT" | "R2" => Some(GamepadButton::RightTrigger),
        "Select" | "Back" | "Share" => Some(GamepadButton::Select),
        "Start" | "Options" | "Menu" => Some(GamepadButton::Start),
        "Guide" | "Home" | "PS" => Some(GamepadButton::Guide),
        "LeftStick" | "L3" => Some(GamepadButton::LeftStick),
        "RightStick" | "R3" => Some(GamepadButton::RightStick),
        "DPadUp" => Some(GamepadButton::DPadUp),
        "DPadDown" => Some(GamepadButton::DPadDown),
        "DPadLeft" => Some(GamepadButton::DPadLeft),
        "DPadRight" => Some(GamepadButton::DPadRight),
        _ => None,
    }
}

fn gamepad_axis_from_name(name: &str) -> Option<GamepadAxis> {
    match name {
        "LeftStickX" | "LeftX" => Some(GamepadAxis::LeftStickX),
        "LeftStickY" | "LeftY" => Some(GamepadAxis::LeftStickY),
        "RightStickX" | "RightX" => Some(GamepadAxis::RightStickX),
        "RightStickY" | "RightY" => Some(GamepadAxis::RightStickY),
        "LeftTrigger" | "LT" => Some(GamepadAxis::LeftTrigger),
        "RightTrigger" | "RT" => Some(GamepadAxis::RightTrigger),
        _ => None,
    }
}

fn key_name_to_keycode(name: &str) -> Option<KeyCode> {
    match name {
        "KeyA" | "A" | "a" => Some(KeyCode::KeyA),
        "KeyB" | "B" | "b" => Some(KeyCode::KeyB),
        "KeyC" | "C" | "c" => Some(KeyCode::KeyC),
        "KeyD" | "D" | "d" => Some(KeyCode::KeyD),
        "KeyE" | "E" | "e" => Some(KeyCode::KeyE),
        "KeyF" | "F" | "f" => Some(KeyCode::KeyF),
        "KeyG" | "G" | "g" => Some(KeyCode::KeyG),
        "KeyH" | "H" | "h" => Some(KeyCode::KeyH),
        "KeyI" | "I" | "i" => Some(KeyCode::KeyI),
        "KeyJ" | "J" | "j" => Some(KeyCode::KeyJ),
        "KeyK" | "K" | "k" => Some(KeyCode::KeyK),
        "KeyL" | "L" | "l" => Some(KeyCode::KeyL),
        "KeyM" | "M" | "m" => Some(KeyCode::KeyM),
        "KeyN" | "N" | "n" => Some(KeyCode::KeyN),
        "KeyO" | "O" | "o" => Some(KeyCode::KeyO),
        "KeyP" | "P" | "p" => Some(KeyCode::KeyP),
        "KeyQ" | "Q" | "q" => Some(KeyCode::KeyQ),
        "KeyR" | "R" | "r" => Some(KeyCode::KeyR),
        "KeyS" | "S" | "s" => Some(KeyCode::KeyS),
        "KeyT" | "T" | "t" => Some(KeyCode::KeyT),
        "KeyU" | "U" | "u" => Some(KeyCode::KeyU),
        "KeyV" | "V" | "v" => Some(KeyCode::KeyV),
        "KeyW" | "W" | "w" => Some(KeyCode::KeyW),
        "KeyX" | "X" | "x" => Some(KeyCode::KeyX),
        "KeyY" | "Y" | "y" => Some(KeyCode::KeyY),
        "KeyZ" | "Z" | "z" => Some(KeyCode::KeyZ),
        "Space" | "space" => Some(KeyCode::Space),
        "Enter" | "Return" => Some(KeyCode::Enter),
        "Escape" | "Esc" => Some(KeyCode::Escape),
        "Tab" => Some(KeyCode::Tab),
        "Backspace" => Some(KeyCode::Backspace),
        "ShiftLeft" | "Shift" => Some(KeyCode::ShiftLeft),
        "ShiftRight" => Some(KeyCode::ShiftRight),
        "ControlLeft" | "Ctrl" => Some(KeyCode::ControlLeft),
        "ArrowUp" | "Up" => Some(KeyCode::ArrowUp),
        "ArrowDown" | "Down" => Some(KeyCode::ArrowDown),
        "ArrowLeft" | "Left" => Some(KeyCode::ArrowLeft),
        "ArrowRight" | "Right" => Some(KeyCode::ArrowRight),
        "Digit1" | "1" => Some(KeyCode::Digit1),
        "Digit2" | "2" => Some(KeyCode::Digit2),
        "Digit3" | "3" => Some(KeyCode::Digit3),
        "Digit4" | "4" => Some(KeyCode::Digit4),
        "Digit5" | "5" => Some(KeyCode::Digit5),
        "Digit6" | "6" => Some(KeyCode::Digit6),
        "Digit7" | "7" => Some(KeyCode::Digit7),
        "Digit8" | "8" => Some(KeyCode::Digit8),
        "Digit9" | "9" => Some(KeyCode::Digit9),
        "Digit0" | "0" => Some(KeyCode::Digit0),
        _ => None,
    }
}

// ── Default bindings ────────────────────────────────────────────────────────

/// Standard set of actions for platformer/topdown games.
pub fn default_actions() -> Vec<InputAction> {
    vec![
        InputAction {
            name: "move".into(),
            action_type: ActionType::Vec2,
            bindings: vec![
                // WASD
                InputBinding { source: InputSource::Key { key: "KeyW".into() }, dead_zone: 0.0, composite: Some(CompositeRole::Up) },
                InputBinding { source: InputSource::Key { key: "KeyS".into() }, dead_zone: 0.0, composite: Some(CompositeRole::Down) },
                InputBinding { source: InputSource::Key { key: "KeyA".into() }, dead_zone: 0.0, composite: Some(CompositeRole::Left) },
                InputBinding { source: InputSource::Key { key: "KeyD".into() }, dead_zone: 0.0, composite: Some(CompositeRole::Right) },
                // Arrows
                InputBinding { source: InputSource::Key { key: "ArrowUp".into() }, dead_zone: 0.0, composite: Some(CompositeRole::Up) },
                InputBinding { source: InputSource::Key { key: "ArrowDown".into() }, dead_zone: 0.0, composite: Some(CompositeRole::Down) },
                InputBinding { source: InputSource::Key { key: "ArrowLeft".into() }, dead_zone: 0.0, composite: Some(CompositeRole::Left) },
                InputBinding { source: InputSource::Key { key: "ArrowRight".into() }, dead_zone: 0.0, composite: Some(CompositeRole::Right) },
                // Left stick
                InputBinding { source: InputSource::GamepadAxis { axis: "LeftStickX".into() }, dead_zone: 0.2, composite: None },
                InputBinding { source: InputSource::GamepadAxis { axis: "LeftStickY".into() }, dead_zone: 0.2, composite: None },
                // D-pad
                InputBinding { source: InputSource::GamepadButton { button: "DPadUp".into() }, dead_zone: 0.0, composite: Some(CompositeRole::Up) },
                InputBinding { source: InputSource::GamepadButton { button: "DPadDown".into() }, dead_zone: 0.0, composite: Some(CompositeRole::Down) },
                InputBinding { source: InputSource::GamepadButton { button: "DPadLeft".into() }, dead_zone: 0.0, composite: Some(CompositeRole::Left) },
                InputBinding { source: InputSource::GamepadButton { button: "DPadRight".into() }, dead_zone: 0.0, composite: Some(CompositeRole::Right) },
            ],
        },
        InputAction {
            name: "jump".into(),
            action_type: ActionType::Button,
            bindings: vec![
                InputBinding { source: InputSource::Key { key: "Space".into() }, dead_zone: 0.0, composite: None },
                InputBinding { source: InputSource::Key { key: "ArrowUp".into() }, dead_zone: 0.0, composite: None },
                InputBinding { source: InputSource::Key { key: "KeyW".into() }, dead_zone: 0.0, composite: None },
                InputBinding { source: InputSource::GamepadButton { button: "South".into() }, dead_zone: 0.0, composite: None },
            ],
        },
        InputAction {
            name: "fire".into(),
            action_type: ActionType::Button,
            bindings: vec![
                InputBinding { source: InputSource::MouseButton { button: "Left".into() }, dead_zone: 0.0, composite: None },
                InputBinding { source: InputSource::Key { key: "KeyX".into() }, dead_zone: 0.0, composite: None },
                InputBinding { source: InputSource::GamepadButton { button: "RightShoulder".into() }, dead_zone: 0.0, composite: None },
                InputBinding { source: InputSource::GamepadButton { button: "West".into() }, dead_zone: 0.0, composite: None },
            ],
        },
        InputAction {
            name: "ui_accept".into(),
            action_type: ActionType::Button,
            bindings: vec![
                InputBinding { source: InputSource::Key { key: "Enter".into() }, dead_zone: 0.0, composite: None },
                InputBinding { source: InputSource::GamepadButton { button: "South".into() }, dead_zone: 0.0, composite: None },
            ],
        },
        InputAction {
            name: "ui_cancel".into(),
            action_type: ActionType::Button,
            bindings: vec![
                InputBinding { source: InputSource::Key { key: "Escape".into() }, dead_zone: 0.0, composite: None },
                InputBinding { source: InputSource::GamepadButton { button: "East".into() }, dead_zone: 0.0, composite: None },
            ],
        },
    ]
}
