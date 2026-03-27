# ADR-035 : Support Gamepad + Systeme Input Actions

- **Statut :** Proposee
- **Date :** 2026-03-28
- **Concerne :** v0.5 / v1.0

## Contexte

Le moteur Toile gere actuellement le clavier et la souris via **winit**, avec des bindings en dur dans le code :

```rust
// game_runner.rs — bindings hardcodes
BehaviorInput {
    left: ctx.input.is_key_down(Key::ArrowLeft) || ctx.input.is_key_down(Key::KeyA),
    jump_pressed: ctx.input.is_key_just_pressed(Key::Space),
    ...
}
```

Problemes :
- **Zero support gamepad** : pas de manette Xbox, PlayStation, Switch, ni generique Bluetooth
- **Pas de remapping** : les joueurs ne peuvent pas changer les touches
- **Pas d'input analogique** : les sticks analogiques ne sont pas exploitables
- **Bindings dans le code** : changer les touches necessite de modifier le code source

Les moteurs de reference (Godot 4.x, Unity Input System) ont converge vers un modele **Input Actions** : une couche d'abstraction ou le code ne reference jamais de boutons physiques, seulement des actions nommees ("jump", "move_left").

## Decision

**Adopter une architecture Input Actions en 3 couches, avec support gamepad via gilrs (ou SDL3 quand migre), et un panneau de configuration dans l'editeur.**

### Architecture 3 couches

```
┌──────────────────────────────────────────────────────┐
│ Couche 1 : Hardware                                   │
│ winit (clavier/souris) + gilrs (gamepads)              │
│ Boutons bruts, axes bruts, connect/disconnect          │
└──────────────────┬───────────────────────────────────┘
                   │
┌──────────────────▼───────────────────────────────────┐
│ Couche 2 : Input Action Map                           │
│ Bindings : touche/bouton/axe → action nommee          │
│ Dead zones, composites (WASD→Vec2), sensibilite       │
│ Contextes : "gameplay", "ui", "menu"                  │
└──────────────────┬───────────────────────────────────┘
                   │
┌──────────────────▼───────────────────────────────────┐
│ Couche 3 : Game Code                                  │
│ Behaviors : input_actions.get_vec2("move")            │
│ Event sheets : OnActionPressed { action: "jump" }     │
│ Jamais de reference a Key::Space ou GamepadButton::A  │
└──────────────────────────────────────────────────────┘
```

### 1. Support Gamepad — gilrs (immediat) ou SDL3 (post-migration)

**Option retenue pour la Phase 1 : gilrs** (crate Rust pure, pas de dep native)

gilrs fournit :
- Detection automatique des manettes connectees
- Hotplug (connect/disconnect a chaud)
- Mapping unifie via la SDL gamepad database (Xbox, PlayStation, Switch, generique)
- Axes normalises (-1.0 a 1.0), boutons uniformises
- Force feedback / rumble
- Cross-platform (Windows, macOS, Linux)

Quand la migration SDL3 (ADR-003) sera faite, gilrs pourra etre remplace par le gamepad subsystem SDL3, mais l'architecture reste identique.

**Etat gamepad dans Input :**

```rust
pub struct GamepadState {
    pub id: gilrs::GamepadId,
    pub name: String,
    pub gamepad_type: GamepadType, // Xbox, PlayStation, Switch, Generic
    pub buttons_down: HashSet<GamepadButton>,
    pub buttons_pressed: HashSet<GamepadButton>,
    pub buttons_released: HashSet<GamepadButton>,
    pub axes: HashMap<GamepadAxis, f32>, // -1.0 a 1.0, dead zone deja appliquee
}

// Ajoute a Input
pub struct Input {
    // ... existant (keys, mouse) ...
    pub gamepads: HashMap<u32, GamepadState>,
    pub gilrs: Option<gilrs::Gilrs>,
}
```

**Boutons virtualises (standard gamepad) :**
- `South` (A/Cross), `East` (B/Circle), `West` (X/Square), `North` (Y/Triangle)
- `LeftShoulder`, `RightShoulder` (bumpers)
- `LeftTrigger`, `RightTrigger`
- `DPadUp/Down/Left/Right`
- `Start`, `Select`, `Guide`
- `LeftStick`, `RightStick` (click)

**Axes :**
- `LeftStickX`, `LeftStickY`, `RightStickX`, `RightStickY`
- `LeftTrigger`, `RightTrigger` (0.0 a 1.0)

### 2. Systeme Input Actions

```rust
/// Type d'action
pub enum ActionType {
    Button,      // pressed / released
    Axis,        // -1.0 a 1.0
    Vec2,        // direction 2D (sticks, WASD composite)
}

/// Source d'un binding
pub enum InputSource {
    Key(KeyCode),
    MouseButton(MouseButton),
    GamepadButton(GamepadButton),
    GamepadAxis { axis: GamepadAxis, direction: f32 }, // +1 ou -1
}

/// Un binding mappe une source physique a une action
pub struct InputBinding {
    pub source: InputSource,
    pub dead_zone: f32,          // pour les axes (defaut 0.2)
    pub sensitivity: f32,        // multiplicateur (defaut 1.0)
    pub composite_role: Option<CompositeRole>, // Up/Down/Left/Right pour Vec2
}

pub enum CompositeRole { Up, Down, Left, Right }

/// Definition d'une action
pub struct InputAction {
    pub name: String,
    pub action_type: ActionType,
    pub bindings: Vec<InputBinding>,
}

/// Etat calcule d'une action pour un frame
pub struct ActionState {
    pub pressed: bool,        // actuellement enfonce
    pub just_pressed: bool,   // premier frame
    pub just_released: bool,  // frame de relachement
    pub value: f32,           // pour Axis (-1..1)
    pub vec2: Vec2,           // pour Vec2
}

/// Map complete des actions
pub struct InputActionMap {
    pub actions: HashMap<String, InputAction>,
    pub states: HashMap<String, ActionState>,
}

impl InputActionMap {
    pub fn is_pressed(&self, action: &str) -> bool;
    pub fn is_just_pressed(&self, action: &str) -> bool;
    pub fn is_just_released(&self, action: &str) -> bool;
    pub fn get_value(&self, action: &str) -> f32;
    pub fn get_vec2(&self, action: &str) -> Vec2;

    /// Evalue tous les bindings contre l'etat Input brut
    pub fn update(&mut self, input: &Input);
}
```

### 3. Bindings par defaut

```json
{
  "input_actions": [
    {
      "name": "move",
      "type": "vec2",
      "bindings": [
        {"source": {"key": "KeyA"}, "composite": "left"},
        {"source": {"key": "KeyD"}, "composite": "right"},
        {"source": {"key": "KeyW"}, "composite": "up"},
        {"source": {"key": "KeyS"}, "composite": "down"},
        {"source": {"key": "ArrowLeft"}, "composite": "left"},
        {"source": {"key": "ArrowRight"}, "composite": "right"},
        {"source": {"key": "ArrowUp"}, "composite": "up"},
        {"source": {"key": "ArrowDown"}, "composite": "down"},
        {"source": {"gamepad_axis": "LeftStickX"}, "composite": null},
        {"source": {"gamepad_axis": "LeftStickY"}, "composite": null}
      ]
    },
    {
      "name": "jump",
      "type": "button",
      "bindings": [
        {"source": {"key": "Space"}},
        {"source": {"gamepad_button": "South"}}
      ]
    },
    {
      "name": "fire",
      "type": "button",
      "bindings": [
        {"source": {"mouse_button": "Left"}},
        {"source": {"gamepad_button": "RightTrigger"}}
      ]
    },
    {
      "name": "ui_accept",
      "type": "button",
      "bindings": [
        {"source": {"key": "Enter"}},
        {"source": {"gamepad_button": "South"}}
      ]
    },
    {
      "name": "ui_cancel",
      "type": "button",
      "bindings": [
        {"source": {"key": "Escape"}},
        {"source": {"gamepad_button": "East"}}
      ]
    }
  ]
}
```

### 4. Refactoring BehaviorInput

```rust
// Avant (hardcode)
fn build_behavior_input(ctx: &GameContext) -> BehaviorInput {
    BehaviorInput {
        left: ctx.input.is_key_down(Key::ArrowLeft) || ctx.input.is_key_down(Key::KeyA),
        ...
    }
}

// Apres (via actions)
fn build_behavior_input(actions: &InputActionMap) -> BehaviorInput {
    let move_vec = actions.get_vec2("move");
    BehaviorInput {
        left: move_vec.x < -0.3,
        right: move_vec.x > 0.3,
        up: move_vec.y > 0.3,
        down: move_vec.y < -0.3,
        jump_pressed: actions.is_just_pressed("jump"),
        jump_down: actions.is_pressed("jump"),
        move_analog: move_vec, // nouveau champ pour le mouvement analogique
    }
}
```

### 5. Event Sheets — nouvelles conditions

```rust
pub enum ConditionKind {
    // Existants
    OnKeyPressed { key: String },
    OnKeyDown { key: String },
    OnKeyReleased { key: String },
    OnMouseClick { button: String },

    // Nouveaux — basees sur les actions
    OnActionPressed { action: String },
    OnActionDown { action: String },
    OnActionReleased { action: String },
    IfActionValue { action: String, op: CompareOp, value: f64 },
}
```

Les anciennes conditions `OnKeyPressed` restent pour la retrocompatibilite, mais les nouvelles `OnActionPressed` sont recommandees car elles fonctionnent avec clavier ET gamepad.

### 6. Dead Zones

**Scaled radial dead zone** (meilleure methode) :

```rust
fn apply_dead_zone(input: Vec2, inner: f32, outer: f32) -> Vec2 {
    let magnitude = input.length();
    if magnitude < inner {
        return Vec2::ZERO;
    }
    let normalized = input / magnitude;
    let remapped = (magnitude - inner) / (outer - inner);
    normalized * remapped.clamp(0.0, 1.0)
}
```

- Inner dead zone : 0.2 (defaut, filtre le drift)
- Outer dead zone : 0.98 (defaut, garantit que le max est atteignable)
- Configurable par binding dans l'editeur

### 7. Panneau Editeur — Input Map

| Element UI | Description |
|------------|-------------|
| **Liste des actions** | Tableau avec nom, type (Button/Axis/Vec2), nombre de bindings |
| **Bindings par action** | Click pour deplier, liste des sources avec bouton "+" |
| **"Press any key/button"** | Modal qui capture le prochain evenement input (clavier, souris, ou gamepad) |
| **Dead zone slider** | Par binding analogique, 0.0 a 0.5 |
| **Periperiques connectes** | Section montrant les manettes detectees avec nom, type (Xbox/PS/Switch), et etat des boutons/axes en temps reel |
| **Test en direct** | Visualisation des axes et boutons de chaque manette (comme la page "Gamepad Tester" des navigateurs) |
| **Presets** | "Platformer", "TopDown", "Shmup" — pre-remplissent les actions standards |

### 8. Vibration / Rumble

```rust
impl GameContext {
    /// Declenche une vibration sur le gamepad du joueur.
    pub fn rumble(&self, player: u32, low: f32, high: f32, duration_ms: u32);
}
```

Expose dans les event sheets comme action :
```
Action: Rumble { intensity: 0.8, duration_ms: 200 }
```

Presets : `light_tap` (50ms, 30%), `hit` (200ms, 60%), `explosion` (400ms, 100%)

### 9. Detection du type de manette

```rust
pub enum GamepadType {
    Xbox,
    PlayStation,
    SwitchPro,
    Generic,
}
```

Utilise pour :
- Afficher les bons glyphes (A/B/X/Y vs Cross/Circle/Square/Triangle)
- Adapter les prompts in-game ("Appuyez sur A" vs "Appuyez sur X")
- Le type est disponible via `ctx.gamepad_type(player_index)`

## Phasage

### Phase 1 : Gamepad brut + Input Actions (immediat)
- Ajouter `gilrs` comme dependance
- Etendre `Input` avec `GamepadState`
- Creer `InputActionMap` avec bindings par defaut
- Refactorer `BehaviorInput` pour utiliser les actions
- Les event sheets continuent de fonctionner (retrocompat)
- La manette fonctionne out-of-the-box pour les behaviors Platform/TopDown

### Phase 2 : Editeur Input Map (v0.5/v1.0)
- Panneau de configuration des actions dans l'editeur
- "Press any key/button" pour capturer un binding
- Visualisation des manettes connectees avec test en direct
- Dead zone sliders
- Sauvegarde dans `input_map.json` du projet

### Phase 3 : Event sheets + Rumble (v1.0)
- Conditions `OnActionPressed`, `OnActionDown`, `OnActionReleased`
- Action `Rumble` dans les event sheets
- Expression `ActionValue("move_x")` pour les axes
- Presets de vibration

### Phase 4 : Multi-joueur local (v2.0)
- Jusqu'a 4 gamepads, chacun avec un `player_index`
- Actions par joueur (chaque joueur a sa propre `InputActionMap`)
- Split-screen ou shared-screen

## Options considerees

### Option A : winit gamepad (experimentale, rejetee)
- winit a un support gamepad experimental (`GamepadEvent`)
- Pas encore stable, API peut changer
- Base de donnees de controllers limitee
- Pas de rumble

### Option B : gilrs (retenue)
- Crate Rust pure, pas de dependance native
- Utilise la SDL gamepad database pour le mapping
- Support hotplug, force feedback, cross-platform
- Bien maintenue, largement utilisee dans l'ecosysteme Rust gamedev
- Peut coexister avec winit pour le clavier/souris

### Option C : SDL3 direct (retenue pour le futur)
- Meilleur support de manettes et de fonctionnalites avancees (gyro, touchpad)
- Necessite la migration windowing (ADR-003) — pas encore faite
- L'architecture Input Actions est identique, seule la couche 1 change

## Consequences

### Positives
- Les jeux Toile supportent les manettes out-of-the-box
- Les joueurs peuvent remapper toutes les touches
- L'input analogique permet des mouvements plus fluides
- Les event sheets fonctionnent avec clavier ET gamepad sans modification
- L'editeur visualise les peripheriques en temps reel

### Negatives
- Nouvelle dependance (gilrs) — ~200KB, pas de dep systeme
- Complexite supplementaire dans la couche input
- Les anciens projets doivent migrer vers les actions (retrocompat assuree)

### Risques
- gilrs pourrait ne pas supporter certaines manettes exotiques → fallback raw joystick
- La migration vers SDL3 (ADR-003) necessitera de remplacer gilrs → l'abstraction InputActions isole le changement
- Le multi-joueur local (Phase 4) necessite de repenser les event sheets par joueur
