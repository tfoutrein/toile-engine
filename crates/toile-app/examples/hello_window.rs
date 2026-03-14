//! Toile Engine — Hello Window
//!
//! Opens a 1280x720 window and clears it to cornflower blue at 60 FPS.
//! This is the Week 1 milestone: proof that the core loop works.
//!
//! Run with: `cargo run --example hello_window`

use toile_app::App;

fn main() {
    App::new()
        .with_title("Toile — Hello Window")
        .with_size(1280, 720)
        .run();
}
