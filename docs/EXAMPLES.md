# Toile Engine — Examples & Demos

All examples are runnable from the repository root. Use `--release` for performance-critical demos.

---

## Table of Contents

- [Tools](#tools)
- [Games](#games)
- [Rendering](#rendering)
- [Visual Effects](#visual-effects)
- [Game Systems](#game-systems)
- [Assets & Formats](#assets--formats)
- [Editor & Tools](#editor--tools)

---

## Tools

### Visual Editor
The full editor with hierarchy, inspector, sprite animations, particle editor, asset browser.
```bash
toile editor
```
Or without installing:
```bash
cargo run -p toile-cli -- editor
```

### Run a Game Project
Launch a game project from its `Toile.toml` manifest.
```bash
toile run workspace/projects/my-game
```
Or the built-in demo:
```bash
cargo run -p toile-cli -- run examples/run-demo
```
**Controls in game:** Arrow keys / WASD to move, Space to jump, **R** to reset scene.

### Asset Browser (standalone)
Browse, import, and classify asset packs without the editor.
```bash
cargo run -p toile-asset-library --bin toile-asset-browser
```

### Create a New Project
Scaffold a project from a template (empty, platformer, topdown, shmup).
```bash
toile new my-game --template platformer
```

---

## Games

### Breakout
Full game with audio, collision, text rendering, ECS, and sprite batching.
```bash
cargo run --example breakout -p toile-app
```

### Platformer
Tiled tilemap, Aseprite animation, Lua enemy AI, camera scrolling, coyote time.
```bash
cargo run --example platformer -p toile-app
```

### Coin Collector (run-demo)
Data-driven platformer created entirely in JSON — validates the Game Runner.
```bash
cargo run -p toile-cli -- run examples/run-demo
```

---

## Rendering

### Hello Window
Minimal window — verifies wgpu + winit setup.
```bash
cargo run --example hello_window -p toile-app
```

### Sprite & Input
Basic sprite rendering with keyboard/mouse input.
```bash
cargo run --example sprite_input -p toile-app
```

### 10K Sprites Benchmark
Stress test: 10,000 moving sprites across 4 textures and 4 layers. **Use `--release` for accurate results.**
```bash
cargo run --release --example bench_10k_sprites -p toile-app
```

### SDF Font Rendering
Crisp text at any scale from a single 32px atlas — outline, drop shadow, animated glow.
```bash
cargo run --example msdf_font_demo -p toile-app
```

---

## Visual Effects

### Lighting
2D point lights with configurable radius, intensity, color, and falloff.
```bash
cargo run --example lighting_demo -p toile-app
```

### Shadows
2D shadow casting with 1D shadow maps, PCF soft shadows, ray marching.
```bash
cargo run --example shadows_demo -p toile-app
```

### Post-Processing
Bloom, CRT scanlines, Vignette, Pixelate, Screen Shake, Color Grading — all toggleable live.
```bash
cargo run --example post_processing_demo -p toile-app
```

### Shader Graph
Node-based shader graph with 4 demo effects (wave, glitch, pixelate, chromatic aberration).
```bash
cargo run --example shader_editor_demo -p toile-app
```

### Particles
Interactive particle demo with 8 switchable presets + explosion burst.
```bash
cargo run --example particles_demo -p toile-app
```

### Particle Editor
Live particle editor with curve/gradient widgets, sub-emitters, JSON save/load.
```bash
cargo run --example particle_editor_demo -p toile-app
```

---

## Game Systems

### Physics
Rapier2D rigid body simulation — boxes fall, bounce, and stack. Click to spawn.
```bash
cargo run --example physics_demo -p toile-app
```

### Scene Stack
Menu → Gameplay → Pause overlay with fade transitions.
```bash
cargo run --example scene_demo -p toile-app
```

### Tweening
All 15 easing curves animated side by side (Linear, EaseIn, EaseOut, Bounce, Elastic...).
```bash
cargo run --example tween_demo -p toile-app
```

### Behaviors
All 7 pre-built behaviors in action: Platform, TopDown, Bullet, Sine, Fade, Wrap, Solid.
```bash
cargo run --example behaviors_demo -p toile-app
```

### Event Sheets
Data-driven game rules with conditions (key press, collision, timer) and actions (move, destroy, spawn).
```bash
cargo run --example event_sheet_demo -p toile-app
```

### Prefabs
Place prefab instances in Edit mode, then play as a platformer character with shooting.
```bash
cargo run --example prefab_demo -p toile-app
```

### Async Loading
Background asset loading with progress bar.
```bash
cargo run --example loading_demo -p toile-app
```

### Audio
WAV/OGG playback — play, stop, pause, loop, volume.
```bash
cargo run --example audio_demo -p toile-app
```

---

## Assets & Formats

### Project Templates
Load and play each of the 4 project templates live (Empty, Platformer, TopDown, Shmup).
```bash
cargo run --example template_demo -p toile-app
```

### LDtk Import
Import LDtk levels with IntGrid collision, entities, and multi-level navigation.
```bash
cargo run --example ldtk_demo -p toile-app
```

### Aseprite Binary Import
Parse .ase/.aseprite files directly — animated sprite with tags, filmstrip view.
```bash
cargo run --example aseprite_demo -p toile-app
```

---

## Editor & Tools

### Visual Editor (legacy example)
The original editor example (before the standalone `toile editor` command).
```bash
cargo run --example editor -p toile-editor
```

---

## Summary Table

| Example | Category | Command |
|---------|----------|---------|
| hello_window | Rendering | `cargo run --example hello_window -p toile-app` |
| sprite_input | Rendering | `cargo run --example sprite_input -p toile-app` |
| bench_10k_sprites | Rendering | `cargo run --release --example bench_10k_sprites -p toile-app` |
| msdf_font_demo | Rendering | `cargo run --example msdf_font_demo -p toile-app` |
| breakout | Game | `cargo run --example breakout -p toile-app` |
| platformer | Game | `cargo run --example platformer -p toile-app` |
| lighting_demo | VFX | `cargo run --example lighting_demo -p toile-app` |
| shadows_demo | VFX | `cargo run --example shadows_demo -p toile-app` |
| post_processing_demo | VFX | `cargo run --example post_processing_demo -p toile-app` |
| shader_editor_demo | VFX | `cargo run --example shader_editor_demo -p toile-app` |
| particles_demo | VFX | `cargo run --example particles_demo -p toile-app` |
| particle_editor_demo | VFX | `cargo run --example particle_editor_demo -p toile-app` |
| physics_demo | Systems | `cargo run --example physics_demo -p toile-app` |
| scene_demo | Systems | `cargo run --example scene_demo -p toile-app` |
| tween_demo | Systems | `cargo run --example tween_demo -p toile-app` |
| behaviors_demo | Systems | `cargo run --example behaviors_demo -p toile-app` |
| event_sheet_demo | Systems | `cargo run --example event_sheet_demo -p toile-app` |
| prefab_demo | Systems | `cargo run --example prefab_demo -p toile-app` |
| loading_demo | Systems | `cargo run --example loading_demo -p toile-app` |
| audio_demo | Systems | `cargo run --example audio_demo -p toile-app` |
| template_demo | Assets | `cargo run --example template_demo -p toile-app` |
| ldtk_demo | Assets | `cargo run --example ldtk_demo -p toile-app` |
| aseprite_demo | Assets | `cargo run --example aseprite_demo -p toile-app` |
| editor | Editor | `cargo run --example editor -p toile-editor` |
