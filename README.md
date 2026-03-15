<p align="center">
  <img src="assets/toile-logo-transparent.png" alt="Toile Engine" width="200">
</p>

<h1 align="center">Toile Engine</h1>

<p align="center">
  <strong>A 2D-pure, AI-native, open-source game engine written in Rust.</strong>
</p>

<p align="center">
  <a href="#features">Features</a> &bull;
  <a href="#quick-start">Quick Start</a> &bull;
  <a href="#examples">Examples</a> &bull;
  <a href="#ai-native">AI-Native</a> &bull;
  <a href="#architecture">Architecture</a>
</p>

---

## What is Toile?

**Toile** (French for "canvas" and "web") is a 2D game engine that occupies the missing middle between simple tools like Construct and powerful but overwhelming engines like Godot/Unity. It does one thing -- 2D games -- and does it well.

- **2D-pure**: No 3D baggage. Pixel-perfect rendering, sprite batching, tilemaps, and frame-by-frame animation are first-class primitives.
- **AI-native**: Built-in MCP server lets AI assistants (Claude, Cursor, Copilot) create and manipulate game scenes directly. JSON scene format with schema validation.
- **Open-source**: MIT licensed. No runtime fees. No surprises.

## Features

| Feature | Description |
|---------|-------------|
| **Sprite Rendering** | Batched rendering, 10k+ sprites at 60 FPS on integrated GPUs |
| **Audio** | WAV/OGG playback via kira (play, stop, pause, loop, volume) |
| **Collision** | AABB/Circle detection with MTV resolution, spatial grid broad-phase |
| **Physics** | Rapier2D integration (rigid bodies, joints, forces, impulses) |
| **ECS** | hecs-based entity component system |
| **Particles** | CPU particle system with 8 presets (fire, smoke, sparks, rain, snow, explosion, dust, confetti) |
| **Tweening** | 15 easing functions, Curve/Gradient interpolation, RepeatMode (Once/Loop/PingPong) |
| **Scene Stack** | Push/pop/replace scenes with fade transitions (menu, gameplay, pause overlay) |
| **Animation** | Aseprite JSON import, frame-based animation with playback modes |
| **Tilemap** | Tiled JSON import + in-editor tilemap painting (brush, eraser, fill) |
| **Lua Scripting** | Embedded Lua 5.4 with hot-reload for game logic |
| **Text Rendering** | TTF rasterization via fontdue, draw_text API |
| **Async Loading** | Background asset loading with progress tracking |
| **Visual Editor** | egui-based editor with hierarchy, inspector, drag & drop, resize handles, rotation, tilemap painting, save/load |
| **MCP Server** | 12 tools for AI-driven scene + tilemap manipulation |
| **CLI** | `toile new`, `toile list-entities`, `toile add-entity` |

## Quick Start

```bash
# Clone and build
git clone https://github.com/toile-engine/toile.git
cd toile
cargo build

# Run the Breakout demo
cargo run --example breakout

# Run the Platformer demo
cargo run --example platformer

# Open the visual editor
cargo run --example editor -p toile-editor

# Create a new project
cargo run --bin toile -- new my-game
```

## Examples

### Breakout
Full game with audio, collision, text, ECS, and sprite batching.
```bash
cargo run --example breakout
```

### Platformer
Tiled tilemap, Aseprite animation, Lua enemy AI, camera scrolling, coyote time.
```bash
cargo run --example platformer
```

### 10K Sprites Benchmark
Stress test: 10,000 moving sprites across 4 textures and 4 layers.
```bash
cargo run --release --example bench_10k_sprites
```

### Particles
Interactive particle demo with 6 switchable presets + explosion burst.
```bash
cargo run --example particles_demo
```

### Physics
Rapier2D rigid body simulation — boxes fall, bounce, and stack. Click to spawn.
```bash
cargo run --example physics_demo
```

### Scene Stack
Menu → Gameplay → Pause overlay with fade transitions.
```bash
cargo run --example scene_demo
```

### Async Loading
Background asset loading with progress bar.
```bash
cargo run --example loading_demo
```

### Visual Editor
Scene editor with hierarchy, inspector, drag & drop, resize handles, rotation, tilemap painting.
```bash
cargo run --example editor -p toile-editor
```

## AI-Native

Toile is designed from the ground up to be controlled by AI assistants.

### MCP Server
The built-in MCP server exposes 8 tools for scene manipulation:

| Tool | Description |
|------|-------------|
| `get_project_info` | Project directory, engine version, available scenes |
| `list_scenes` | List all scene JSON files |
| `create_scene` | Create a new empty scene |
| `load_scene` | Load and return scene data |
| `list_entities` | List entities with positions |
| `create_entity` | Add an entity to a scene |
| `delete_entity` | Remove an entity by ID |
| `update_entity` | Modify entity properties |
| `create_tilemap` | Create a tilemap grid in a scene |
| `set_tile` | Set a tile at a position |
| `fill_rect` | Fill a rectangle of tiles |
| `get_tile` | Read a tile at a position |

Configure in `.mcp.json`:
```json
{
  "mcpServers": {
    "toile": {
      "type": "stdio",
      "command": "cargo",
      "args": ["run", "--bin", "toile-mcp-server", "--", "."]
    }
  }
}
```

### CLI
```bash
toile new my-game              # Scaffold a project
toile list-entities scene.json  # List entities
toile add-entity scene.json Player 100 200  # Add entity
```

### JSON Scene Format
Scenes are human-readable, diff-friendly, and LLM-friendly:
```json
{
  "name": "level1",
  "entities": [
    {
      "id": 1,
      "name": "Hero",
      "x": 100.0,
      "y": 200.0,
      "width": 32.0,
      "height": 32.0
    }
  ]
}
```

## Architecture

```
toile/
  crates/
    toile-core/        Math, time, color, handles
    toile-platform/    Windowing + input (winit)
    toile-graphics/    wgpu 2D renderer, sprite batching, camera
    toile-audio/       Audio playback (kira)
    toile-collision/   AABB/Circle detection, spatial grid
    toile-ecs/         Entity Component System (hecs)
    toile-assets/      Asset loading, fonts, animation, tilemap
    toile-scripting/   Lua VM + hot-reload (mlua)
    toile-scene/       Scene serialization (JSON)
    toile-editor/      Visual editor (egui)
    toile-physics/     Rapier2D physics (optional)
    toile-mcp/         MCP server for AI control (rmcp)
    toile-cli/         CLI binary
    toile-app/         Application framework, game loop
```

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Language | Rust |
| Rendering | wgpu (Vulkan/Metal/DX12/OpenGL) |
| Windowing | winit |
| Audio | kira |
| ECS | hecs |
| Math | glam |
| Scripting | Lua 5.4 (mlua) |
| Physics | Rapier2D (optional) |
| Editor UI | egui |
| MCP | rmcp |

## License

MIT License. See [LICENSE](LICENSE) for details.

---

<p align="center">
  <sub>Built with Rust. Designed for creators. Powered by AI.</sub>
</p>
