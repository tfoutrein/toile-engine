use std::collections::{HashMap, HashSet};

use glam::Vec2;
use winit::event::{ElementState, KeyEvent, MouseButton as WinitMouseButton, MouseScrollDelta};
use winit::keyboard::{KeyCode, PhysicalKey};

pub use winit::keyboard::KeyCode as Key;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

// ── Gamepad types ───────────────────────────────────────────────────────────

/// Standardized gamepad button names (Xbox-style layout, mapped by gilrs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadButton {
    South,         // A / Cross
    East,          // B / Circle
    West,          // X / Square
    North,         // Y / Triangle
    LeftShoulder,  // LB / L1
    RightShoulder, // RB / R1
    LeftTrigger,   // LT / L2 (digital)
    RightTrigger,  // RT / R2 (digital)
    Select,        // Back / Share
    Start,         // Start / Options
    Guide,         // Xbox / PS button
    LeftStick,     // L3
    RightStick,    // R3
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
}

/// Standardized gamepad axis names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadAxis {
    LeftStickX,
    LeftStickY,
    RightStickX,
    RightStickY,
    LeftTrigger,
    RightTrigger,
}

/// Type of gamepad for display purposes (glyph selection).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GamepadType {
    Xbox,
    PlayStation,
    SwitchPro,
    Generic,
}

/// Per-gamepad state.
#[derive(Debug, Clone)]
pub struct GamepadState {
    pub name: String,
    pub gamepad_type: GamepadType,
    pub buttons_down: HashSet<GamepadButton>,
    pub buttons_pressed: HashSet<GamepadButton>,
    pub buttons_released: HashSet<GamepadButton>,
    pub axes: HashMap<GamepadAxis, f32>,
}

impl GamepadState {
    fn new(name: String, gamepad_type: GamepadType) -> Self {
        Self {
            name,
            gamepad_type,
            buttons_down: HashSet::new(),
            buttons_pressed: HashSet::new(),
            buttons_released: HashSet::new(),
            axes: HashMap::new(),
        }
    }
}

/// Default dead zone for analog sticks.
const DEFAULT_DEAD_ZONE: f32 = 0.2;

/// Apply scaled radial dead zone to a single axis value.
fn apply_dead_zone_1d(value: f32, dead_zone: f32) -> f32 {
    let abs = value.abs();
    if abs < dead_zone {
        0.0
    } else {
        let sign = value.signum();
        sign * ((abs - dead_zone) / (1.0 - dead_zone)).clamp(0.0, 1.0)
    }
}

// ── Conversion helpers ──────────────────────────────────────────────────────

fn gilrs_button_to_gamepad(btn: gilrs::Button) -> Option<GamepadButton> {
    match btn {
        gilrs::Button::South => Some(GamepadButton::South),
        gilrs::Button::East => Some(GamepadButton::East),
        gilrs::Button::West => Some(GamepadButton::West),
        gilrs::Button::North => Some(GamepadButton::North),
        gilrs::Button::LeftTrigger => Some(GamepadButton::LeftShoulder),
        gilrs::Button::RightTrigger => Some(GamepadButton::RightShoulder),
        gilrs::Button::LeftTrigger2 => Some(GamepadButton::LeftTrigger),
        gilrs::Button::RightTrigger2 => Some(GamepadButton::RightTrigger),
        gilrs::Button::Select => Some(GamepadButton::Select),
        gilrs::Button::Start => Some(GamepadButton::Start),
        gilrs::Button::Mode => Some(GamepadButton::Guide),
        gilrs::Button::LeftThumb => Some(GamepadButton::LeftStick),
        gilrs::Button::RightThumb => Some(GamepadButton::RightStick),
        gilrs::Button::DPadUp => Some(GamepadButton::DPadUp),
        gilrs::Button::DPadDown => Some(GamepadButton::DPadDown),
        gilrs::Button::DPadLeft => Some(GamepadButton::DPadLeft),
        gilrs::Button::DPadRight => Some(GamepadButton::DPadRight),
        _ => None,
    }
}

fn gilrs_axis_to_gamepad(axis: gilrs::Axis) -> Option<GamepadAxis> {
    match axis {
        gilrs::Axis::LeftStickX => Some(GamepadAxis::LeftStickX),
        gilrs::Axis::LeftStickY => Some(GamepadAxis::LeftStickY),
        gilrs::Axis::RightStickX => Some(GamepadAxis::RightStickX),
        gilrs::Axis::RightStickY => Some(GamepadAxis::RightStickY),
        gilrs::Axis::LeftZ => Some(GamepadAxis::LeftTrigger),
        gilrs::Axis::RightZ => Some(GamepadAxis::RightTrigger),
        _ => None,
    }
}

fn detect_gamepad_type(name: &str) -> GamepadType {
    let lower = name.to_lowercase();
    if lower.contains("xbox") || lower.contains("xinput") || lower.contains("microsoft") {
        GamepadType::Xbox
    } else if lower.contains("playstation") || lower.contains("dualshock") || lower.contains("dualsense") || lower.contains("sony") {
        GamepadType::PlayStation
    } else if lower.contains("switch") || lower.contains("nintendo") || lower.contains("pro controller") {
        GamepadType::SwitchPro
    } else {
        GamepadType::Generic
    }
}

// ── Input struct ────────────────────────────────────────────────────────────

/// Per-frame input state. Collects winit events and gilrs gamepad events,
/// exposes a polling API.
///
/// Call `handle_*` during event dispatch, `poll_gamepads()` once per frame,
/// query during update/draw, then `end_frame()` at end of frame.
pub struct Input {
    // Keyboard
    keys_down: HashSet<KeyCode>,
    keys_pressed_this_frame: HashSet<KeyCode>,
    keys_released_this_frame: HashSet<KeyCode>,
    // Mouse
    mouse_down: HashSet<MouseButton>,
    mouse_pressed_this_frame: HashSet<MouseButton>,
    mouse_position: Vec2,
    scroll_delta: Vec2,
    scale_factor: f32,
    // Gamepads
    pub(crate) gilrs: Option<gilrs::Gilrs>,
    gamepads: HashMap<gilrs::GamepadId, GamepadState>,
    /// Ordered list of connected gamepad IDs (for player indexing).
    gamepad_order: Vec<gilrs::GamepadId>,
}

impl Input {
    pub fn new() -> Self {
        let gilrs = match gilrs::Gilrs::new() {
            Ok(g) => {
                log::info!("Gamepad subsystem initialized");
                Some(g)
            }
            Err(e) => {
                log::warn!("Failed to initialize gamepad subsystem: {e}");
                None
            }
        };

        let mut input = Self {
            keys_down: HashSet::new(),
            keys_pressed_this_frame: HashSet::new(),
            keys_released_this_frame: HashSet::new(),
            mouse_down: HashSet::new(),
            mouse_pressed_this_frame: HashSet::new(),
            mouse_position: Vec2::ZERO,
            scroll_delta: Vec2::ZERO,
            scale_factor: 1.0,
            gilrs,
            gamepads: HashMap::new(),
            gamepad_order: Vec::new(),
        };

        // Register already-connected gamepads
        if let Some(ref g) = input.gilrs {
            let connected: Vec<(gilrs::GamepadId, String)> = g.gamepads()
                .filter(|(_, gp)| gp.is_connected())
                .map(|(id, gp)| (id, gp.name().to_string()))
                .collect();
            for (id, name) in connected {
                let gp_type = detect_gamepad_type(&name);
                log::info!("Gamepad connected: {} ({:?})", name, gp_type);
                input.gamepads.insert(id, GamepadState::new(name, gp_type));
                input.gamepad_order.push(id);
            }
        }

        input
    }

    // --- Keyboard event handlers ---

    pub fn handle_key_event(&mut self, event: &KeyEvent) {
        if event.repeat {
            return;
        }
        if let PhysicalKey::Code(code) = event.physical_key {
            match event.state {
                ElementState::Pressed => {
                    if self.keys_down.insert(code) {
                        self.keys_pressed_this_frame.insert(code);
                    }
                }
                ElementState::Released => {
                    self.keys_down.remove(&code);
                    self.keys_released_this_frame.insert(code);
                }
            }
        }
    }

    // --- Mouse event handlers ---

    pub fn handle_mouse_button(&mut self, button: WinitMouseButton, state: ElementState) {
        let btn = match button {
            WinitMouseButton::Left => MouseButton::Left,
            WinitMouseButton::Right => MouseButton::Right,
            WinitMouseButton::Middle => MouseButton::Middle,
            _ => return,
        };
        match state {
            ElementState::Pressed => {
                if self.mouse_down.insert(btn) {
                    self.mouse_pressed_this_frame.insert(btn);
                }
            }
            ElementState::Released => {
                self.mouse_down.remove(&btn);
            }
        }
    }

    pub fn handle_cursor_moved(&mut self, x: f64, y: f64) {
        self.mouse_position = Vec2::new(x as f32 / self.scale_factor, y as f32 / self.scale_factor);
    }

    pub fn set_scale_factor(&mut self, scale: f64) {
        self.scale_factor = scale as f32;
    }

    pub fn handle_mouse_wheel(&mut self, delta: &MouseScrollDelta) {
        match delta {
            MouseScrollDelta::LineDelta(x, y) => {
                self.scroll_delta += Vec2::new(*x, *y);
            }
            MouseScrollDelta::PixelDelta(pos) => {
                self.scroll_delta += Vec2::new(pos.x as f32, pos.y as f32);
            }
        }
    }

    // --- Gamepad polling ---

    /// Poll gilrs for gamepad events. Call once per frame before update().
    pub fn poll_gamepads(&mut self) {
        let gilrs = match self.gilrs.as_mut() {
            Some(g) => g,
            None => return,
        };

        // Clear per-frame states
        for state in self.gamepads.values_mut() {
            state.buttons_pressed.clear();
            state.buttons_released.clear();
        }

        while let Some(event) = gilrs.next_event() {
            match event.event {
                gilrs::EventType::Connected => {
                    let gp = gilrs.gamepad(event.id);
                    let name = gp.name().to_string();
                    let gp_type = detect_gamepad_type(&name);
                    log::info!("Gamepad connected: {} ({:?})", name, gp_type);
                    self.gamepads.insert(event.id, GamepadState::new(name, gp_type));
                    if !self.gamepad_order.contains(&event.id) {
                        self.gamepad_order.push(event.id);
                    }
                }
                gilrs::EventType::Disconnected => {
                    if let Some(state) = self.gamepads.get(&event.id) {
                        log::info!("Gamepad disconnected: {}", state.name);
                    }
                    self.gamepads.remove(&event.id);
                    self.gamepad_order.retain(|id| *id != event.id);
                }
                gilrs::EventType::ButtonPressed(btn, _) => {
                    if let Some(gb) = gilrs_button_to_gamepad(btn) {
                        if let Some(state) = self.gamepads.get_mut(&event.id) {
                            state.buttons_down.insert(gb);
                            state.buttons_pressed.insert(gb);
                        }
                    }
                }
                gilrs::EventType::ButtonReleased(btn, _) => {
                    if let Some(gb) = gilrs_button_to_gamepad(btn) {
                        if let Some(state) = self.gamepads.get_mut(&event.id) {
                            state.buttons_down.remove(&gb);
                            state.buttons_released.insert(gb);
                        }
                    }
                }
                gilrs::EventType::AxisChanged(axis, value, _) => {
                    if let Some(ga) = gilrs_axis_to_gamepad(axis) {
                        if let Some(state) = self.gamepads.get_mut(&event.id) {
                            let processed = apply_dead_zone_1d(value, DEFAULT_DEAD_ZONE);
                            state.axes.insert(ga, processed);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // --- Frame end ---

    pub fn end_frame(&mut self, had_ticks: bool) {
        if had_ticks {
            self.keys_pressed_this_frame.clear();
            self.keys_released_this_frame.clear();
            self.mouse_pressed_this_frame.clear();
        }
        self.scroll_delta = Vec2::ZERO;
    }

    // --- Keyboard query API ---

    pub fn is_key_down(&self, key: KeyCode) -> bool {
        self.keys_down.contains(&key)
    }

    pub fn is_key_just_pressed(&self, key: KeyCode) -> bool {
        self.keys_pressed_this_frame.contains(&key)
    }

    pub fn is_key_just_released(&self, key: KeyCode) -> bool {
        self.keys_released_this_frame.contains(&key)
    }

    // --- Mouse query API ---

    pub fn is_mouse_down(&self, button: MouseButton) -> bool {
        self.mouse_down.contains(&button)
    }

    pub fn is_mouse_just_pressed(&self, button: MouseButton) -> bool {
        self.mouse_pressed_this_frame.contains(&button)
    }

    pub fn mouse_position(&self) -> Vec2 {
        self.mouse_position
    }

    pub fn scroll_delta(&self) -> Vec2 {
        self.scroll_delta
    }

    // --- Gamepad query API ---

    /// Number of connected gamepads.
    pub fn gamepad_count(&self) -> usize {
        self.gamepads.len()
    }

    /// Get the state of gamepad by player index (0-based).
    pub fn gamepad(&self, player_index: usize) -> Option<&GamepadState> {
        self.gamepad_order.get(player_index)
            .and_then(|id| self.gamepads.get(id))
    }

    /// Check if a gamepad button is currently held (by player index).
    pub fn is_gamepad_button_down(&self, player: usize, button: GamepadButton) -> bool {
        self.gamepad(player).is_some_and(|s| s.buttons_down.contains(&button))
    }

    /// Check if a gamepad button was just pressed this frame.
    pub fn is_gamepad_button_just_pressed(&self, player: usize, button: GamepadButton) -> bool {
        self.gamepad(player).is_some_and(|s| s.buttons_pressed.contains(&button))
    }

    /// Check if a gamepad button was just released this frame.
    pub fn is_gamepad_button_just_released(&self, player: usize, button: GamepadButton) -> bool {
        self.gamepad(player).is_some_and(|s| s.buttons_released.contains(&button))
    }

    /// Get a gamepad axis value (-1.0 to 1.0, dead zone already applied).
    pub fn gamepad_axis(&self, player: usize, axis: GamepadAxis) -> f32 {
        self.gamepad(player)
            .and_then(|s| s.axes.get(&axis).copied())
            .unwrap_or(0.0)
    }

    /// Get left stick as Vec2 (x: left/right, y: up/down).
    pub fn gamepad_left_stick(&self, player: usize) -> Vec2 {
        Vec2::new(
            self.gamepad_axis(player, GamepadAxis::LeftStickX),
            self.gamepad_axis(player, GamepadAxis::LeftStickY),
        )
    }

    /// Get right stick as Vec2.
    pub fn gamepad_right_stick(&self, player: usize) -> Vec2 {
        Vec2::new(
            self.gamepad_axis(player, GamepadAxis::RightStickX),
            self.gamepad_axis(player, GamepadAxis::RightStickY),
        )
    }

    /// Get all connected gamepads (for UI display).
    pub fn connected_gamepads(&self) -> Vec<(usize, &GamepadState)> {
        self.gamepad_order.iter().enumerate()
            .filter_map(|(i, id)| self.gamepads.get(id).map(|s| (i, s)))
            .collect()
    }
}
