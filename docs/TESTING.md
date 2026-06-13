# Testing & Visual Verification

Toile has three complementary layers for checking that a change actually works:

1. **Unit tests** — fast, pure logic (math, collision, events, behaviors, parsers).
2. **`toile-harness`** — a *headless* GPU renderer that draws scenes/sprites off-screen and writes a PNG. This is the engine equivalent of a Playwright screenshot: deterministic, no window required, scriptable, CI-friendly.
3. **`tools/screenshot-app.sh`** — screenshots a *real* running window (editor / examples), for the egui UI and the full post-processing/lighting pipeline that only run windowed.

---

## 1. Unit tests

```bash
cargo test --workspace            # everything (libs + examples compile + unit tests)
cargo test -p toile-behaviors     # one crate
cargo test -p toile-behaviors single_jump   # one test by name substring
```

All tests are in-module `#[cfg(test)]` blocks; there is no `tests/` integration dir except `toile-harness`.

> Note: `cargo test --workspace` compiles the example binaries too, so a stale example breaks the whole test run — keep `crates/toile-app/examples/*` compiling.

---

## 2. `toile-harness` — headless render → PNG

Boots a real GPU device with **no window/surface**, renders with the engine's real `SpriteRenderer`, reads the framebuffer back, and saves a PNG you can open or diff.

```bash
# GPU smoke test: draws a known R/G/B + rotated-quad pattern.
cargo run -p toile-harness -- smoke --out harness-out/smoke.png

# Render any scene JSON (entities tinted by role, or their real sprite if found).
cargo run -p toile-harness -- scene examples/run-demo/scenes/main.json \
  --out harness-out/level.png --width 800 --height 600

# Options: --zoom <f32> (omit = auto-fit all entities), --camera "x,y",
#          --assets <dir> (root for relative sprite_path; default = scene dir)
```

Entities are coloured by role when they have no resolvable sprite: player = blue, coin = gold, enemy = red, solid/platform/wall = grey.

### As a library (write your own visual assertions)

`toile-harness` is also a crate. Use it to render and assert on pixels:

```rust
use toile_harness::Harness;
use toile_graphics::sprite_renderer::{DrawSprite, pack_color};
use toile_graphics::camera::Camera2D;
use toile_core::color::Color;
use glam::Vec2;

let mut h = Harness::new(256, 256)?;             // Err if no GPU -> skip in CI
let mut cam = Camera2D::new(256.0, 256.0);
h.render(&cam, &[/* DrawSprite { .. } */], Color::BLACK);
let px = h.pixels()?;                              // tightly-packed RGBA8
// assert on px[..], or:
h.save_png("out.png")?;
```

See `crates/toile-harness/tests/smoke.rs` for the pattern (it skips gracefully when no GPU is present).

### Why headless instead of just screenshotting the window?

It is deterministic and repeatable (fixed size, fixed pixels), needs no display, and can run unattended — so it can verify scene/level/sprite work the way Playwright verifies a web page, and can become a CI visual-regression gate.

---

## 3. `tools/screenshot-app.sh` — real windowed screenshot (macOS)

For things the headless renderer doesn't cover — the **egui editor UI**, and the full pipeline (lighting, post-processing, scene-stack transitions) that the windowed `App` runs:

```bash
tools/screenshot-app.sh cargo run --example breakout -p toile-app
OUT=shots/editor.png WAIT=6 tools/screenshot-app.sh cargo run -p toile-cli -- editor
```

It launches the command, waits `WAIT` seconds, captures the frontmost window (full display if window bounds can't be read), then kills the app. macOS needs Screen Recording permission (and Accessibility for the per-window crop) for the terminal app.

---

## Roadmap: headless gameplay simulation (next step)

Today `toile-harness` renders *static* scene state. The natural next increment is a headless driver that runs the real `Game`/`GameRunner` loop for N fixed ticks with a **scripted input timeline** (press Right at frame 5, Jump at frame 20…), capturing PNGs and dumping entity state for assertions — letting us verify *gameplay* (movement, collisions, coins collected) visually and deterministically. That requires a small `GpuContext` headless mode (offscreen target instead of a surface) so the full `GameContext` can be built without a window; it is intentionally not done yet to keep the windowed engine path untouched.
