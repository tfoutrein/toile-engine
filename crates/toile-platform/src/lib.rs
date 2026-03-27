pub mod input;

pub use input::{Input, Key, MouseButton, GamepadButton, GamepadAxis, GamepadType, GamepadState};

/// Window configuration.
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub resizable: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Toile Engine".to_string(),
            width: 1280,
            height: 720,
            resizable: true,
        }
    }
}

pub use winit;
