# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

**Toile** is a 2D-pure, AI-native game engine in Rust, structured as an 18-crate Cargo workspace (`resolver = "2"`, edition **2024**, version `0.5.0-dev`). It targets the "missing middle" between simple tools (Construct) and heavy engines (Godot/Unity). Code is English; design docs (`docs/ROADMAP.md`, `docs/adr/*`) are French.

## Commands

```bash
# Build (edition 2024 requires a recent stable toolchain; rust-toolchain.toml pins channel = stable)
cargo build                      # debug, whole workspace
cargo build --release            # needed for perf-sensitive demos

# Test — ALL tests are in-module #[cfg(test)] units; there is NO tests/ integration dir
cargo test                       # whole workspace (~63 tests across logic/data crates)
cargo test -p toile-events       # one crate (tests live in toile-core, -collision, -events,
                                 #   -assets, -asset-library, -behaviors, -physics, -runner, -scene)
cargo test -p toile-events <name>  # single test by fn-name substring (cargo matches the FQ path)

# Lint — none wired into CI/pre-commit; standard tooling works if you want it
cargo clippy --workspace
cargo fmt
```

### Running things

Examples (24 of them) live in **`crates/toile-app/examples/`**, NOT at the workspace root, so they need `-p toile-app`:

```bash
cargo run --example breakout -p toile-app
cargo run --example platformer -p toile-app
cargo run --release --example bench_10k_sprites -p toile-app   # use --release for benches
# others: lighting_demo, shadows_demo, post_processing_demo, shader_editor_demo, particles_demo,
#   particle_editor_demo, physics_demo, scene_demo, tween_demo, behaviors_demo, event_sheet_demo,
#   prefab_demo, loading_demo, audio_demo, template_demo, ldtk_demo, aseprite_demo, gamepad_demo,
#   input_actions_demo, msdf_font_demo, sprite_input, hello_window
cargo run --example editor -p toile-editor                     # the lone editor-crate example

# The CLI binary is named `toile` (crate toile-cli), not `toile-cli`:
cargo run -p toile-cli -- editor                               # launch the visual editor
cargo run -p toile-cli -- new my-game --template platformer    # empty|platformer|topdown|shmup
cargo run -p toile-cli -- run examples/run-demo                # run a data-driven project
cargo run -p toile-cli -- templates | list-entities <s.json> | add-entity <s.json> <name> <x> <y>

# MCP server (binary toile-mcp-server, crate toile-mcp) — takes the project dir as argv[1]:
cargo run --bin toile-mcp-server -- .

# Asset Library standalone browser:
cargo run -p toile-asset-library --bin toile-asset-browser

# Headless visual verification (no window): render a scene/sprite to a PNG you can inspect.
cargo run -p toile-harness -- smoke
cargo run -p toile-harness -- scene examples/run-demo/scenes/main.json --out level.png
```

For visual/UI verification see **`docs/TESTING.md`**: `toile-harness` is a headless GPU renderer (Playwright-style off-screen screenshots, CI-friendly) and `tools/screenshot-app.sh` captures the real windowed editor/examples. A full code/security/perf audit lives in `docs/AUDIT-2026-06-13.md`.

## Architecture

### Dependency flow

Strictly layered, no cycles (ADR-009). `toile-core` is the dependency-free foundation; `toile-app` is the top-level façade that re-exports every subsystem. Dependencies always flow downward; shared types live in `toile-core`.

```
toile-core ── foundation: Color, GameClock (fixed-timestep, Glenn Fiedler), Rect, Easing/Tween,
   │           Curve/Gradient (piecewise-linear keypoints), ParticleEmitter (recursive on_death),
   │           re-exports glam (Vec2/Mat3/…). Zero internal deps. `serde` feature.
   ├─ toile-platform   winit + gilrs input. Input + InputActionMap (named actions: move/jump/fire/
   │                   ui_*; Button/Axis/Vec2; multi-source bindings). Call end_frame() each frame.
   ├─ toile-graphics   wgpu renderer. GpuContext (device/queue/surface root), SpriteRenderer
   │                   (batched by layer+texture, color packed u32 RGBA8), Camera2D (ortho+zoom),
   │                   PostProcessingStack (ping-pong: Vignette/CRT/Pixelate/Bloom/Shake/ColorGrade),
   │                   ShaderGraph (DAG → WGSL compiler), SdfTextRenderer (SDF atlas).
   ├─ toile-audio      kira wrapper. Audio + Sound/Music/PlaybackId. Volume is dB (20·log10).
   ├─ toile-collision  Shape/Collider, narrow-phase returns MTV (Option<Vec2>), OBB-vs-OBB via SAT,
   │                   SpatialGrid broad-phase. `serde` feature.
   ├─ toile-assets     AsyncLoader (worker thread + mpsc, poll()/progress()), SpriteSheet, Font/TTF,
   │                   SdfFont, AnimationClip, Aseprite binary parser, LDtk + Tiled importers.
   ├─ toile-scene      JSON data model: SceneData / EntityData / SceneSettings / Prefab. The on-disk
   │                   source of truth. next_id is transient → call fix_next_id() after load.
   ├─ toile-behaviors  7 behaviors as (Config + update fn): Platform (slopes, coyote, jump-buffer),
   │                   TopDown, Bullet, Sine, Fade, Wrap, Solid. Take a SolidCheck callback. Zero deps.
   ├─ toile-events     Event sheets (visual scripting): EventSheet, ConditionKind/ActionKind,
   │                   evaluate_event_sheet(). EventContext takes closures for input queries.
   ├─ toile-ecs        Thin hecs wrapper: re-exports World/Entity + Toile components. No systems here.
   ├─ toile-scripting  Lua 5.4 (mlua), sandboxed (os/io/loadfile/dofile removed). ScriptWatcher
   │                   hot-reload is poll-driven by the caller, not a background thread.
   ├─ toile-physics    Rapier2D wrapper (PhysicsWorld). Bidirectional handle maps. ECS sync is caller's job.
   ├─ toile-runner     DATA-DRIVEN runtime (see below). Implements the Game trait.
   ├─ toile-asset-library  Asset-pack import: scanner + 3-pass classifier, thumbnails, manifest cache,
   │                   egui browser, AI ImportPlan overrides (ADR-036).
   ├─ toile-editor     egui visual editor (modes: Entity/Tilemap/Particle/SpriteAnim/AssetBrowser/
   │                   AICopilot). Holds AI copilot + bug reporter (see AI section).
   ├─ toile-mcp        MCP server for external AI control (see AI section).
   └─ toile-cli        `toile` binary; delegates to toile-editor / toile-runner / templates.
```

### Two ways a game runs

1. **Code-driven** — implement the `Game` trait from `toile-app` (`init` / `update` / `draw` / `render_overlay` / `handle_window_event` / `egui_ui`). The `AppHandler` owns the winit event loop and calls these with a `GameContext` that aggregates renderer, `Camera2D`, input + `InputActionMap`, audio, post-processing, and lighting. This is what every example in `crates/toile-app/examples/` does. `toile-app/src/scene.rs` adds a `SceneStack` (push/pop/replace with fade transitions) for menu→gameplay→pause flows. `F3` toggles the FPS/frame-time debug title overlay.

2. **Data-driven** — `crates/toile-runner` (`GameRunner`, also a `Game` impl) executes a project directory with **no code**. `toile run <dir>` reads a `Toile.toml` manifest (`[project]` + `[game] entry_scene/window_*`) and runs a 7-phase loop: camera modes → behaviors → spatial-grid collision → event-sheet evaluation → command apply (spawn/destroy/move/scene-transition) → animation frames → remove dead. See `examples/run-demo/` (`Toile.toml`, `scenes/main.json`, `scripts/*.event.json`). Gotcha: Platform/TopDown behaviors only run on entities tagged `"player"`; `collision_map` keys are the *tags collided with*, not entity IDs.

### Scene format = source of truth

Scenes/prefabs/event-sheets/particle configs are JSON (`*.json`, `*.event.json`, `*.particles.json`); project manifest is TOML (`Toile.toml`). These files — not code — are the canonical game data, edited by the editor, the MCP server, and AI directly. The v0.5 format (EntityData v2: `behaviors`, `tags`, `variables`, `collision_shape`, `visible`; scene-level `gravity`/`camera`/`background_color`) is **not** backward-compatible with pre-v0.5 scenes (ADR-031).

### AI-native architecture (three separate locations)

- **External MCP control** → crate `toile-mcp`. `ToileMcpServer` (rmcp 0.16, stdio). Tools are built manually via a `make_tool(name, desc, schema)` helper and dispatched in one big `handle_tool` match — **not** rmcp `#[tool]` macros. ~20 tools (scenes, entities, tilemaps, prefabs, particle emitters) that mutate scene JSON via load/save round-trips; no in-memory session state. Configured in `.mcp.json` (points at the prebuilt `target/debug/toile-mcp-server` — so `cargo build` it first).
- **In-editor copilot** → `crates/toile-editor/src/ai/` (`config.rs`, `client.rs`, `chat_panel.rs`, `tools.rs`). ADR-033. Multi-provider: Anthropic (default) + OpenAI-compatible (Scaleway/OpenAI/Groq/Ollama), dynamic model list via `GET /models`, tool-calling auto-continuation loop, markdown via `egui_commonmark`. Its ~two-dozen tools run **directly on the in-memory `SceneData`** (live viewport update), independent of the MCP process.
- **Bug reporter + asset import** → `toile-editor/src/ai/bug_reporter.rs` (ADR-034: `report_bug` shells out to `gh issue create --repo tfoutrein/toile-engine` with auto labels, dedup, per-session rate limit) and `toile-asset-library/src/ai_import.rs` (ADR-036: README+filetree → structured `ImportPlan` overriding the heuristic classifier).

## Conventions & gotchas

- **Examples need `-p toile-app`** — they are not workspace-root examples.
- **Binary names differ from crate names**: CLI = `toile` (crate `toile-cli`), MCP = `toile-mcp-server` (crate `toile-mcp`).
- **Physics is NOT a `--features` flag.** "Optional" in the docs means *isolated in its own crate* (`toile-physics`), per the modular philosophy. `rapier2d` is a hard dep of that crate, and `toile-app` depends on it directly. The only real feature flags are `serde` on `toile-core` and `toile-collision`.
- **macOS Retina: set `ctx.camera.zoom = 2.0` in new demos** or content renders too small (e.g. `examples/platformer.rs:462`). This applies to every demo on Retina displays.
- **AI features need a key** in `~/.toile/config.json` (`AiConfig`); editor workspace config lives in `~/.toile/`. No key → AI off, but heuristic/basic paths still work. The `gh` CLI must be installed + authed for the bug reporter.
- **`cargo build` before MCP use** — clients spawn the prebuilt binary at the path hardcoded in `.mcp.json`.

## Project status & decisions

`0.5.0-dev`. v0.5 "Complete Editor" (ADR-031) is delivered per `docs/ROADMAP.md`: it connected previously-orphaned systems so a full game can be built in the editor and launched with `toile run`. In progress: ADR-034 Log Watcher (Phase 2), ADR-033 vision/proactive suggestions, ADR-036 asset-import (status "Proposée" — design-of-record being progressively implemented). **Web/WASM export was moved from v0.5 to v1.5** "Web & Share"; 3D is never; next milestone is v1.0 "Production Ready".

**Every architectural decision is an ADR** in `docs/adr/` (process in `000-adr-process.md`) — consult the relevant ADR before changing a subsystem, and add a new ADR for significant decisions. `docs/EXAMPLES.md` is the annotated example index.
