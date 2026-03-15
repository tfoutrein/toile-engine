//! Toile Engine — Hello Window
//!
//! Opens a 1280x720 window and clears it to cornflower blue at 60 FPS.
//!
//! Run with: `cargo run --example hello_window`

use toile_app::{App, Game, GameContext};

struct EmptyGame;

impl Game for EmptyGame {
    fn init(&mut self, _ctx: &mut GameContext) {}
    fn update(&mut self, _ctx: &mut GameContext, _dt: f64) {}
    fn draw(&mut self, _ctx: &mut GameContext) {}
}

fn main() {
    App::new()
        .with_title("Toile — Hello Window")
        .with_size(1280, 720)
        .run(EmptyGame);
}
