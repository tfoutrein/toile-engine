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
/// Call `handle_*` during event dispatch, query during update/draw,
/// then `end_frame()` at end of frame.
pub struct Input {
    keys_down: HashSet<KeyCode>,
    keys_pressed_this_frame: HashSet<KeyCode>,
    keys_released_this_frame: HashSet<KeyCode>,
    mouse_down: HashSet<MouseButton>,
    mouse_pressed_this_frame: HashSet<MouseButton>,
    mouse_position: Vec2,
    scroll_delta: Vec2,
}

impl Input {
    pub fn new() -> Self {
        Self {
            keys_down: HashSet::new(),
            keys_pressed_this_frame: HashSet::new(),
            keys_released_this_frame: HashSet::new(),
            mouse_down: HashSet::new(),
            mouse_pressed_this_frame: HashSet::new(),
            mouse_position: Vec2::ZERO,
            scroll_delta: Vec2::ZERO,
        }
    }

    // --- Event handlers (called from AppHandler) ---

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

    /// Call at the end of each frame.
    pub fn end_frame(&mut self) {
        self.keys_pressed_this_frame.clear();
        self.keys_released_this_frame.clear();
        self.mouse_pressed_this_frame.clear();
        self.scroll_delta = Vec2::ZERO;
    }

    // --- Query API ---

    pub fn is_key_down(&self, key: KeyCode) -> bool {
        self.keys_down.contains(&key)
    }

    /// True for the entire frame the key was first pressed.
    /// Safe to call multiple times per frame — always returns the same value.
    pub fn is_key_just_pressed(&self, key: KeyCode) -> bool {
        self.keys_pressed_this_frame.contains(&key)
    }

    /// True for the entire frame the key was released.
    pub fn is_key_just_released(&self, key: KeyCode) -> bool {
        self.keys_released_this_frame.contains(&key)
    }

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
}
