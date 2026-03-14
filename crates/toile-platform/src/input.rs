use std::collections::HashSet;

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

/// Per-frame input state. Collects winit events and exposes a polling API.
///
/// Usage pattern:
/// 1. During winit event dispatch: call `handle_*` methods
/// 2. During update/draw: call `is_key_down`, `mouse_position`, etc.
/// 3. At end of frame: call `end_frame` to snapshot state for next-frame diffing
pub struct Input {
    keys_down: HashSet<KeyCode>,
    keys_down_prev: HashSet<KeyCode>,
    keys_just_pressed: HashSet<KeyCode>,
    keys_just_released: HashSet<KeyCode>,
    mouse_down: HashSet<MouseButton>,
    mouse_down_prev: HashSet<MouseButton>,
    mouse_position: Vec2,
    scroll_delta: Vec2,
}

impl Input {
    pub fn new() -> Self {
        Self {
            keys_down: HashSet::new(),
            keys_down_prev: HashSet::new(),
            keys_just_pressed: HashSet::new(),
            keys_just_released: HashSet::new(),
            mouse_down: HashSet::new(),
            mouse_down_prev: HashSet::new(),
            mouse_position: Vec2::ZERO,
            scroll_delta: Vec2::ZERO,
        }
    }

    // --- Event handlers (called from AppHandler) ---

    pub fn handle_key_event(&mut self, event: &KeyEvent) {
        // Ignore key repeats (auto-repeat when held) — they would
        // cause is_key_just_pressed to mis-fire on held keys.
        if event.repeat {
            return;
        }
        if let PhysicalKey::Code(code) = event.physical_key {
            match event.state {
                ElementState::Pressed => {
                    if self.keys_down.insert(code) {
                        // insert returns true if the key was NOT already present
                        self.keys_just_pressed.insert(code);
                    }
                }
                ElementState::Released => {
                    self.keys_down.remove(&code);
                    self.keys_just_released.insert(code);
                }
            }
        }
    }

    pub fn handle_mouse_button(&mut self, button: WinitMouseButton, state: ElementState) {
        let btn = match button {
            WinitMouseButton::Left => MouseButton::Left,
            WinitMouseButton::Right => MouseButton::Right,
            WinitMouseButton::Middle => MouseButton::Middle,
            _ => return,
        };
        match state {
            ElementState::Pressed => {
                self.mouse_down.insert(btn);
            }
            ElementState::Released => {
                self.mouse_down.remove(&btn);
            }
        }
    }

    pub fn handle_cursor_moved(&mut self, x: f64, y: f64) {
        self.mouse_position = Vec2::new(x as f32, y as f32);
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

    /// Call at the end of each frame to clear per-frame state.
    pub fn end_frame(&mut self) {
        self.keys_down_prev.clone_from(&self.keys_down);
        self.keys_just_pressed.clear();
        self.keys_just_released.clear();
        self.mouse_down_prev.clone_from(&self.mouse_down);
        self.scroll_delta = Vec2::ZERO;
    }

    // --- Query API ---

    pub fn is_key_down(&self, key: KeyCode) -> bool {
        self.keys_down.contains(&key)
    }

    /// True only once per press (safe to call in multi-tick update loops).
    pub fn is_key_just_pressed(&mut self, key: KeyCode) -> bool {
        self.keys_just_pressed.remove(&key)
    }

    /// True only once per release (safe to call in multi-tick update loops).
    pub fn is_key_just_released(&mut self, key: KeyCode) -> bool {
        self.keys_just_released.remove(&key)
    }

    pub fn is_mouse_down(&self, button: MouseButton) -> bool {
        self.mouse_down.contains(&button)
    }

    pub fn is_mouse_just_pressed(&self, button: MouseButton) -> bool {
        self.mouse_down.contains(&button) && !self.mouse_down_prev.contains(&button)
    }

    pub fn mouse_position(&self) -> Vec2 {
        self.mouse_position
    }

    pub fn scroll_delta(&self) -> Vec2 {
        self.scroll_delta
    }
}
