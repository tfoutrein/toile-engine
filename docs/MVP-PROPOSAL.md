# Toile Engine — Proposition MVP

**Un moteur 2D pur, AI-native, open-source pour le "missing middle".**

Version 0.1 — Mars 2026

---

## 1. Vision & Positionnement

### Le problème

Le marché des moteurs 2D est cassé en deux. D'un côté, des outils accessibles mais limités — Construct 3 (abonnement, HTML5 uniquement, plafond de complexité bas), RPG Maker (genre-locked), GameMaker (scaling difficile, GML propriétaire). De l'autre, des moteurs puissants mais écrasants — Unity (2D greffée sur de la 3D, confiance détruite par le runtime fee 2023), Godot (le plus proche concurrent mais traînant du poids 3D, physique cassée, tilemap editor qui lag), Bevy (pas d'éditeur, temps de compilation Rust, artefacts 2D venant des internals 3D).

Entre ces deux camps se trouve le **missing middle** : un moteur 2D aussi facile à démarrer que Construct, aussi puissant que Godot pour les projets ambitieux, et construit exclusivement pour la 2D. Pas de baggage 3D. Pas de piège d'abonnement. Pas de risque corporate.

### L'opportunité

Trois tendances convergentes rendent ce moment idéal :

1. **Engine fatigue.** Le fiasco du pricing Unity a déclenché un exode massif. Les développeurs cherchent activement des alternatives. La renaissance du "sans moteur" (Noel Berry, Celeste) montre l'appétit pour des outils légers et transparents — mais la plupart des développeurs ont toujours besoin d'un éditeur et d'un pipeline d'assets.

2. **Développement AI-native.** Le "vibe coding" est mainstream — 25% du batch Y Combinator W25 avait des codebases à 95%+ générées par IA. Mais aucun moteur de jeu n'a été conçu pour le contrôle par IA. Les serveurs MCP pour Unity et Godot sont des ajouts après coup. Un moteur conçu dès le jour 1 pour le pilotage par IA — format de scène déclaratif, validation JSON Schema, serveur MCP natif, feedback d'erreurs structuré — est une position marché unique.

3. **La 2D n'est pas de la 3D simplifiée.** Le rendu pixel-perfect, l'animation frame-by-frame, les mondes tile-based, la physique de platformer avec coyote time et input buffering — ce sont des préoccupations 2D de premier rang qui méritent une ingénierie de premier rang, pas des adaptations de pipelines 3D.

### Elevator Pitch

> **Toile** est un moteur 2D pur, AI-native, open-source écrit en Rust. Il occupe le missing middle entre l'accessibilité de Construct et la puissance de Godot. Les scènes sont en JSON. Le serveur MCP est intégré. L'éditeur est livré avec le moteur. La taille d'export commence à 2 MB. Il fait une seule chose — des jeux 2D — et la fait mieux que tout le reste.

### Nom

**Toile** — mot français signifiant à la fois "canvas" (la surface de création) et "web" (la toile). Ce double sens est parfait pour un moteur 2D : le canvas sur lequel les créatifs peignent leurs jeux, et le web vers lequel ils les exportent. Le nom est court (5 lettres), mémorable, français, et immédiatement évocateur du monde de la création artistique. `toile new mon-jeu`, `toile run`, `toile build --web` — ça sonne bien en CLI aussi.

---

## 2. Stack Technique

Chaque choix ci-dessous est définitif. Pas de compromis.

### Langage : Rust

- **Sécurité mémoire à la compilation.** Le borrow checker élimine les use-after-free, double-free et data races — des classes entières de bugs qui affligent les moteurs C/C++. Pour un moteur open-source acceptant des contributions communautaires, ce n'est pas optionnel.
- **Cargo est best-in-class.** Dépendances, builds, tests, benchmarks, documentation — unifiés. Pas de CMake hell. Un nouveau contributeur lance `cargo build` et ça marche.
- **L'écosystème est prêt.** `wgpu`, `winit`, `glam`, `hecs`, `kira`, `gilrs` — chaque bibliothèque nécessaire existe, est maintenue, et est conçue pour le game dev. Ce n'était pas vrai en 2022. C'est vrai en 2026.
- **Pérennité.** L'adoption de Rust en programmation système accélère. C++ stagne en mindshare. Zig est excitant mais pré-1.0. Rust est le bon pari pour un projet qui doit durer une décennie.
- **AI-friendly.** Le système de types de Rust sert de documentation machine-readable. Les LLMs entraînés sur du Rust produisent du code plus correct que du C++.

### Rendu : wgpu

- **Rust natif.** Zéro overhead FFI, zéro wrappers unsafe.
- **Cross-plateforme par design.** Abstrait Vulkan, Metal, DX12 et OpenGL — et cible WebGPU pour le déploiement web. Un renderer, toutes les plateformes.
- **API moderne.** Compute shaders, binding de ressources explicite. Place pour les features 2D avancées (éclairage, particules, post-processing).
- **Battle-tested.** Bevy l'utilise. Firefox l'utilise. Maintenu par l'équipe `gfx-rs` avec le backing Mozilla.

### Windowing/Plateforme : SDL3 (via rust-sdl3)

- **Gestion d'input battle-tested.** La base de données gamepad SDL3, les haptics et l'abstraction d'input sont vastement plus matures que winit.
- **Maturité cross-plateforme.** SDL a 25+ ans de connaissance des quirks de plateformes.
- **Chemin mobile.** Le support mobile SDL3 est éprouvé pour iOS/Android.

On utilise SDL3 pour le windowing, l'input et l'abstraction plateforme. On n'utilise PAS le renderer SDL3 — wgpu gère tout le rendu.

### Audio : kira

- **Conçu pour les jeux.** Contrôle de paramètres par tweening, audio synchronisé par horloge, bases d'audio spatial — des features dont les jeux ont besoin.
- **Rust-native.** Pas de FFI, pas de wrappers unsafe.
- **Streaming intégré.** Le streaming musical fonctionne sans code custom.

### ECS : hecs

- **Minimal et focalisé.** Un ECS archetype standalone sans scheduler, sans app framework, sans système de plugins. Il fait une chose : stocker et requêter les entités/composants efficacement.
- **On contrôle le scheduler.** On veut notre propre game loop et ordonnancement de systèmes.
- **Petit footprint de compilation.** hecs compile vite (contrairement à Bevy ECS).

### Scripting : Lua (via mlua)

- **Livrer le MVP.** Designer un bon DSL est un projet de plusieurs mois. Lua existe, fonctionne, et a un track record de 30 ans en jeux (WoW, Roblox, Love2D, Defold, Factorio).
- **Performance LuaJIT.** Vitesse quasi-native pour la logique de jeu.
- **Hot-reload natif.** Les modules Lua peuvent être rechargés au runtime par design.
- **AI-friendly.** Les LLMs produisent d'excellent code Lua. Le langage est assez petit pour qu'une IA contienne toute l'API en contexte.

### UI Éditeur : egui

- **Rust-native.** Pas de FFI C++. egui est du pur Rust et s'intègre directement avec wgpu via `egui-wgpu`.
- **L'éditeur se rend dans le moteur.** egui dessine directement dans notre pipeline wgpu. L'éditeur EST le moteur — pas de process séparé, pas d'IPC. C'est critique pour l'édition WYSIWYG et la preview live.
- **Support WASM.** egui tourne dans le navigateur. L'éditeur pourra éventuellement être livré comme web app — zéro installation.

### Format de scène : JSON + JSON Schema

Non-négociable pour le positionnement AI-native :
- Les LLMs peuvent générer des scènes valides en contraignant la sortie au schéma
- Les scènes sont lisibles, diff-friendly et merge-friendly en version control
- Le schéma EST la documentation — chaque champ, type, défaut et contrainte est machine-readable
- La validation attrape les erreurs avant qu'elles n'atteignent le moteur

### Table récapitulative

| Couche | Choix | Crate/Bibliothèque |
|--------|-------|-------------------|
| Langage | Rust | stable toolchain |
| Rendu | wgpu | `wgpu` |
| Windowing/Input | SDL3 | `sdl3` (rust bindings) |
| Audio | kira | `kira` |
| ECS | hecs | `hecs` |
| Math | glam | `glam` |
| Scripting | Lua | `mlua` (LuaJIT) |
| UI éditeur | egui | `egui` + `egui-wgpu` |
| Format de scène | JSON | `serde_json` + `jsonschema` |
| Physique (post-MVP) | Rapier | `rapier2d` |

---

## 3. Feature Set MVP

### DANS le MVP (v0.1)

#### Core Engine

| Feature | Spécification |
|---------|--------------|
| **Game loop** | Fixed timestep (60 Hz physique) avec rendu interpolé. Pattern accumulator avec garde anti-spirale de mort. |
| **Windowing** | Fenêtre unique, resize, DPI/HiDPI, toggle fullscreen. |
| **Input** | Clavier (down/up/pressed/released), souris (position, boutons, scroll, delta), gamepad (axes, boutons, connexion/déconnexion). Couche de mapping d'input abstraite. |
| **Rendu de sprites** | Support atlas de textures, sprite batching (cible : 10 000+ sprites à 60 FPS sur GPU intégré). Z-order par layer + ordre explicite. Alpha blending. |
| **Animation de sprites** | Animation frame-based depuis sprite sheets. Frame tags, durée par frame, modes loop/once/ping-pong. Import JSON Aseprite. |
| **Caméra** | Projection orthographique 2D. Pan, zoom, rotation. Conversion coordonnées monde↔écran. |
| **Détection de collision** | AABB vs AABB, cercle vs cercle, AABB vs cercle, point vs forme. Grille uniforme broad-phase. Détection d'overlap + vecteur push-out. Pas de simulation physique complète. |
| **Audio** | Charger WAV (SFX) et OGG Vorbis (musique). Play, stop, pause, resume, loop. Volume par son et master. 16+ sons simultanés. |
| **Rendu de texte** | BMFont (bitmap font). Rasterisation TTF en atlas de glyphes au chargement via `fontdue`. |
| **Tilemap** | Charger le format Tiled JSON. Rendre les tile layers. Collision tile-based depuis les object layers. Gestion des bit-flips GID. |
| **Overlay debug** | FPS, draw calls, entity count, breakdown frame time. Toggle par hotkey. |

#### Formats d'assets supportés (MVP)

| Catégorie | Formats |
|-----------|---------|
| Images | PNG |
| Audio | WAV (SFX), OGG Vorbis (musique) |
| Atlas de sprites | TexturePacker JSON (hash), export JSON Aseprite |
| Tilemap | Tiled JSON |
| Polices | BMFont (.fnt + .png), TTF |
| Données | JSON (scènes, config, définitions d'entités) |

#### Éditeur (v0.1)

| Feature | Spécification |
|---------|--------------|
| **Layout** | Panneaux dockables : viewport scène, inspecteur, asset browser, hiérarchie d'entités, console. |
| **Viewport de scène** | Canvas infini avec pan/zoom. Grille overlay (toggle, taille configurable). |
| **Placement d'entités** | Drag depuis l'asset browser vers le viewport. Sélectionner, déplacer, tourner, scaler avec gizmos. Multi-sélection box. |
| **Inspecteur** | Propriétés de l'entité sélectionnée : nom, transform, référence sprite, layer, propriétés custom. |
| **Hiérarchie d'entités** | Vue arbre de toutes les entités. Réordonnement par drag. Nesting parent/enfant. |
| **Asset browser** | Basé filesystem. Thumbnails pour les images. Recherche/filtre. Drag-to-viewport. |
| **Undo/redo** | Pattern Command couvrant toutes les opérations. Ctrl+Z / Ctrl+Shift+Z. |
| **Save/load** | Sérialisation/désérialisation des scènes en JSON. |
| **Bouton Play** | Exécuter la scène courante dans le viewport. Stop retourne en mode édition. |
| **Snap à la grille** | Grid snapping toggle avec taille de cellule configurable. |

#### Intégration IA (MVP)

| Feature | Spécification |
|---------|--------------|
| **Serveur MCP** | Intégré, expose : CRUD scènes, CRUD entités, manipulation de composants, listing d'assets, config projet, run/stop jeu, capture de screenshot, lecture console. |
| **llms.txt** | Livré avec chaque release. Markdown structuré documentant l'API complète, le format de scène et les types de composants. Optimisé pour la consommation par LLM. |
| **JSON Schema** | Schémas publiés pour : format de scène, config projet, définitions de composants, définitions d'animations. |
| **Erreurs structurées** | Toutes les erreurs retournent du JSON avec code, message humain, contexte, alternatives disponibles et suggestions de fix. |
| **CLI** | `toile new <project>`, `toile run`, `toile build`, `toile add-entity`, `toile list-entities`, `toile export`. Chaque opération éditeur a un équivalent CLI. |
| **Mode headless** | Le moteur tourne sans affichage pour le testing automatisé et les boucles de validation IA. |

#### Scripting (MVP)

| Feature | Spécification |
|---------|--------------|
| **Intégration Lua** | mlua avec backend LuaJIT. |
| **Bindings API moteur** | Création/requête d'entités, polling d'input, playback audio, callbacks de collision, contrôle caméra, gestion de scènes. |
| **Lifecycle de script** | Callbacks `on_create()`, `on_update(dt)`, `on_destroy()` par entité. |
| **Hot-reload** | Les scripts Lua se rechargent à la sauvegarde du fichier sans redémarrer le jeu. |

### EXPLICITEMENT EXCLU du MVP

| Feature | Pourquoi exclue | Quand |
|---------|----------------|-------|
| Simulation physique (rigid bodies, joints, forces) | La détection de collision + push-out suffit pour les jeux MVP. Rapier arrive en v0.2. | v0.2 |
| Système de particules | Nice-to-have mais ne bloque pas le jeu test MVP. | v0.2 |
| Éclairage et ombres 2D | Différenciateur majeur mais scope d'ingénierie significatif. | v0.4 |
| Éditeur de tilemap (paint/fill/erase in-engine) | Le MVP utilise Tiled comme éditeur externe. | v0.2 |
| Éditeur de timeline d'animation | Le MVP importe les animations Aseprite. | v0.3 |
| Event sheets / scripting visuel | Lua d'abord. Le scripting visuel est v0.3+. | v0.3 |
| Behaviors pré-construits (platformer, top-down, etc.) | Nécessite que la couche scripting soit stable. | v0.2 |
| Export Web/WASM | wgpu cible WebGPU mais le pipeline de build nécessite du travail. Desktop d'abord. | v0.2 |
| Export mobile (iOS/Android) | Complexité plateforme disproportionnée. | v1.5+ |
| Multiplayer/netcode | Nécessite une architecture de simulation déterministe. | v1.5 |
| Shaders custom (WGSL user) | Le shader sprite par défaut couvre les besoins MVP. | v0.2 |
| Chargement d'assets async | Le chargement synchrone est suffisant pour les petits jeux MVP. | v0.2 |
| Hot-reload d'assets (textures, audio) | Le hot-reload Lua est inclus. Le hot-reload d'assets suit. | v0.2 |
| Animation squelettique Spine 2D | L'animation frame-based couvre le MVP. | v0.4 |
| Format tilemap LDtk | Tiled couvre le MVP. LDtk arrive ensuite. | v0.2 |
| Rendu de polices SDF | BMFont + TTF rasterisé suffisent pour le MVP. | v0.4 |
| Système de localisation | Ne bloque pas la validation MVP. | v0.4 |
| Framework d'accessibilité | Différenciateur important mais pas bloquant pour le MVP. | v1.0 |

---

## 4. Architecture

### Structure des modules

```
toile/
  toile-core/          -- Math (glam re-export), temps, logging, types d'erreur,
                          handles générationnels, canaux d'événements
  toile-platform/      -- Abstraction SDL3 : fenêtre, input, événements plateforme
                          trait Platform { ... }
  toile-graphics/      -- Renderer wgpu : gestion de textures, sprite batcher,
                          caméra, rendu de texte, debug draw
                          trait GraphicsBackend { ... }
  toile-audio/         -- Intégration kira : chargement de sons, playback, mixing
                          trait AudioBackend { ... }
  toile-collision/     -- Formes (AABB, cercle), tests d'overlap, grille spatiale
  toile-ecs/           -- hecs re-export + composants Toile :
                          Transform, Sprite, Animator, Collider, ScriptRef
  toile-assets/        -- Asset manager : load, cache, système de handles
                          Loaders de formats : PNG, WAV, OGG, JSON atlas, Tiled, BMFont, TTF
  toile-scripting/     -- VM Lua (mlua), bindings API moteur, hot-reload
  toile-scene/         -- Sérialisation/désérialisation de scènes (JSON + JSON Schema)
                          Diffing de scènes pour le hot-reload
  toile-editor/        -- Éditeur basé egui : panneaux, gizmos, pile undo/redo
  toile-mcp/           -- Serveur MCP : définitions d'outils, handlers de requêtes,
                          capture de screenshots, réponses structurées
  toile-cli/           -- Binaire CLI : scaffolding de projet, run, build, export
  toile-app/           -- Façade de haut niveau : App builder, game loop, lifecycle
                          Re-exporte tout pour l'API simple `use toile::*`
```

### Flux de données

```
                    ┌──────────────────────────────────┐
                    │           toile-app               │
                    │  App::new() -> .add_system() ->   │
                    │  .run()                           │
                    └──────────┬───────────────────────┘
                               │
          ┌────────────────────┼────────────────────┐
          │                    │                    │
          v                    v                    v
   ┌─────────────┐   ┌──────────────┐   ┌──────────────────┐
   │  Platform    │   │   ECS World  │   │   Asset Manager  │
   │  (SDL3)      │   │   (hecs)     │   │                  │
   │              │   │              │   │  Textures, Sons   │
   │  Fenêtre     │   │  Entités +   │   │  Atlas, Polices   │
   │  Input       │   │  Composants  │   │  Tilemaps, Scènes │
   │  Événements  │   │              │   │                  │
   └──────┬───────┘   └──────┬───────┘   └────────┬─────────┘
          │                  │                     │
          v                  v                     │
   ┌─────────────────────────────────────────┐     │
   │            Game Loop (par frame)         │     │
   │                                         │     │
   │  1. Platform.poll_events()              │     │
   │  2. Input.update(events)                │     │
   │  3. Scripting.update(dt)  [Lua]         │     │
   │  4. User systems.update(dt)             │     │
   │  5. Collision.detect(world)             │     │
   │  6. Animation.update(dt)               │     │
   │  7. Renderer.collect(world) → cmds     │     │
   │  8. Renderer.sort(cmds) par layer/tex  │     │
   │  9. Renderer.batch_and_draw(cmds) ──────────→ wgpu
   │ 10. Editor.draw() [egui]               │     │
   │ 11. Platform.swap_buffers()            │     │
   └─────────────────────────────────────────┘     │
          │                                        │
          v                                        v
   ┌─────────────┐                         ┌──────────────┐
   │  MCP Server  │ ← commandes JSON ←     │  CLI / IA    │
   │  (async)     │ → feedback structuré →  │  Assistant   │
   └─────────────┘                         └──────────────┘
```

### Abstractions clés

**Couche plateforme par traits :**

```rust
pub trait Platform {
    fn create_window(&self, config: &WindowConfig) -> Result<Window>;
    fn poll_events(&mut self) -> &[PlatformEvent];
    fn swap_buffers(&self);
    fn elapsed_time(&self) -> Duration;
}

pub trait GraphicsBackend {
    fn create_texture(&mut self, data: &[u8], w: u32, h: u32) -> TextureHandle;
    fn destroy_texture(&mut self, handle: TextureHandle);
    fn submit_batch(&mut self, batch: &SpriteBatch);
    fn set_camera(&mut self, camera: &Camera2D);
}

pub trait AudioBackend {
    fn load_sound(&mut self, data: &[u8], format: AudioFormat) -> SoundHandle;
    fn play(&mut self, sound: SoundHandle, params: &PlayParams) -> PlaybackHandle;
    fn stop(&mut self, handle: PlaybackHandle);
    fn set_master_volume(&mut self, volume: f32);
}
```

Le build desktop utilise `Sdl3Platform + WgpuBackend + KiraAudio`. Un futur build web utilisera `WebPlatform + WgpuBackend + WebAudioBackend`. Le code moteur est identique — seules les implémentations de backends changent.

**Handles générationnels partout :** Pas de pointeurs bruts vers les assets ou entités. Chaque référence est un `Handle<T>` contenant un index et un compteur de génération, empêchant les use-after-free.

**Undo/redo basé sur les commandes :** Chaque opération éditeur produit une `Command` avec `execute()` et `undo()`. La pile de commandes est exposée via le serveur MCP — les assistants IA peuvent expérimenter et rollback en sécurité.

**Canaux d'événements, pas de callbacks :** Les événements plateforme, collision et script passent par des ring buffers typés lus par frame. Pas de callbacks, pas de problèmes de lifetime.

---

## 5. Critères de succès

### Cibles de performance

| Métrique | Cible |
|----------|-------|
| Sprites à 60 FPS (GPU intégré, ex: Intel Iris) | 10 000 |
| Sprites à 60 FPS (GPU dédié) | 50 000 |
| Draw calls par frame (10k sprites, même atlas) | < 20 |
| Temps CPU par frame (1 000 entités avec collision) | < 8 ms |
| Overhead mémoire moteur (pas de jeu chargé) | < 20 MB |
| Mémoire par entité (transform + sprite + collider) | < 256 bytes |
| Fenêtre visible au lancement | < 500 ms |
| Premier frame rendu | < 1 seconde |
| Petit jeu chargé (50 textures, 20 sons) | < 3 secondes |

### Test "Can Build X Game"

Le MVP n'est pas terminé tant que le jeu suivant ne peut pas être construit entièrement dans Toile :

**Un platformer avec :**
- Un personnage joueur avec des sprites animés idle, run et jump (importés d'Aseprite)
- Input clavier et gamepad
- Niveau tile-based chargé depuis Tiled
- Collision avec les tiles solides et les entités ennemies
- 3 niveaux avec transitions de scènes
- Effets sonores (saut, atterrissage, collecte) et musique de fond
- Une caméra qui suit le joueur avec scrolling fluide
- Un éditeur in-engine qui peut placer des entités, ajuster les propriétés, et sauvegarder/charger le niveau
- Logique de jeu écrite en scripts Lua avec hot-reload

Si ce jeu tourne à 60 FPS sur un MacBook Air 2020 (M1, GPU intégré) et que le niveau peut être édité par quelqu'un qui n'a jamais écrit de code, le MVP est complet.

### Plateformes cibles (MVP)

| Plateforme | Statut |
|-----------|--------|
| macOS (Apple Silicon + Intel) | Cible de développement primaire |
| Linux (x86_64) | Supporté, testé en CI |
| Windows (x86_64) | Supporté, testé en CI |
| Web/WASM | Pas dans le MVP (v0.2) |
| iOS / Android | Pas dans le MVP (v1.5+) |

---

## 6. Timeline estimée

Développeur solo, temps plein. 12 semaines jusqu'au MVP.

### Semaine 1-2 : Fondation

- Scaffolding projet : workspace, structure de crates, CI (GitHub Actions macOS/Linux/Windows)
- Création de fenêtre SDL3 + initialisation contexte wgpu
- Clear screen à une couleur. Vérifier cross-plateforme.
- Game loop avec fixed timestep (60 Hz update, rendu interpolé)
- Input polling : clavier, souris. Mapping d'input abstrait.
- Types math de base via glam. Rect, Color utilitaires.

**Milestone :** Fenêtre s'ouvre, clear en cornflower blue, répond au clavier/souris, tourne à 60 FPS verrouillés sur les trois plateformes desktop.

### Semaine 3-4 : Rendu

- Chargement de textures PNG via crate `image`
- Rendu d'un quad texturé unique via wgpu
- Caméra orthographique (pan, zoom, conversion monde→écran)
- Sprite batcher : tri par (layer, texture), batch des sprites consécutifs partageant l'état, un seul draw call par batch
- Benchmark : 10 000 sprites à 60 FPS sur GPU intégré
- Debug draw : rectangles, cercles, lignes (wireframe)
- Rendu de texte basique : loader + renderer BMFont

**Milestone :** 10 000 sprites animés à 60 FPS. La caméra pan fluidement. Le FPS counter s'affiche en BMFont.

### Semaine 5-6 : Audio + Collision + ECS

- Intégration kira : charger WAV et OGG, play/stop/pause/loop, contrôle volume
- Intégration hecs : entités avec composants Transform, Sprite, Collider, Animator
- Détection de collision : AABB, cercle, AABB-cercle, point-forme
- Grille spatiale uniforme pour le broad-phase
- Détection d'overlap + résolution par vecteur push-out
- Système de handles d'assets avec index générationnels

**Milestone :** Clone de Breakout. La balle rebondit sur la raquette et les briques avec collision. Les effets sonores jouent à l'impact. Tous les game objects sont des entités ECS.

### Semaine 7-8 : Animation + Tilemap + Scripting

- Système d'animation de sprites : séquences de frames, durée par frame, modes de boucle, import JSON Aseprite avec frame tags
- Loader de tilemap Tiled JSON : tile layers, object layers, gestion des bits GID, collision tile-based
- Renderer de tilemap avec batching efficace
- Intégration Lua via mlua : initialisation VM, bindings API moteur (entité, input, audio, collision, caméra)
- Callbacks lifecycle de script : `on_create`, `on_update`, `on_draw_debug`
- Hot-reload Lua : file watcher déclenche le rechargement de module

**Milestone :** Prototype de platformer. Un joueur animé court et saute à travers un niveau Tiled. Le comportement ennemi est écrit en Lua. Éditer le script Lua met à jour le comportement sans redémarrer.

### Semaine 9-10 : Éditeur (Phase 1)

- Intégration egui dans le pipeline wgpu
- Layout à panneaux dockables : viewport, inspecteur, hiérarchie, asset browser, console
- Viewport de scène : canvas infini, pan/zoom, grille, snapping
- Placement d'entités : drag depuis l'asset browser vers le viewport
- Gizmos de sélection et transformation : move, rotate, scale. Multi-sélection par box.
- Panneau inspecteur : éditer transform, référence sprite, layer, propriétés custom
- Arbre de hiérarchie d'entités avec drag-to-reorder
- Pile undo/redo (pattern Command) couvrant toutes les opérations
- Sérialisation de scènes : sauvegarder/charger des fichiers JSON
- Bouton Play/Stop : exécuter la scène dans le viewport, stop retourne en mode édition

**Milestone :** Ouvrir l'éditeur, glisser des sprites sur le canvas, arranger un niveau, configurer les propriétés dans l'inspecteur, sauvegarder, appuyer sur play, voir le jeu tourner. L'undo fonctionne pour chaque opération.

### Semaine 11-12 : Intégration IA + Polish

- Serveur MCP : exposer CRUD scènes, CRUD entités, manipulation de composants, listing d'assets, run/stop, screenshot, lecture console
- Authoring JSON Schema pour le format de scène, config projet, définitions de composants
- Génération llms.txt depuis la documentation API du moteur
- Système d'erreurs structurées : toutes les erreurs retournent du JSON avec code, message, contexte, suggestions
- CLI : `toile new`, `toile run`, `toile build`, `toile add-entity`, `toile list-entities`
- Mode headless : moteur tourne sans affichage
- Chargement TTF via fontdue
- Thumbnails dans l'asset browser
- Tests d'intégration : construire le jeu test platformer de bout en bout
- Passe de profiling et optimisation
- Écrire 3 projets exemples : Breakout, platformer, top-down shooter

**Milestone :** Le jeu test MVP (platformer) est jouable, éditable, et peut être modifié par un assistant IA via le serveur MCP. Tous les critères de succès sont remplis.

---

## 7. Risques & Mitigations

| Risque | Impact | Mitigation |
|--------|--------|-----------|
| **Intégration wgpu + SDL3** | L'interaction fenêtrage SDL3 ↔ surface wgpu peut impliquer des douleurs spécifiques par plateforme | Problème résolu connu (`raw-window-handle`). Spike en semaine 1 jour 1. Si > 2 jours, fallback sur winit pour le MVP. |
| **Qualité visuelle egui** | L'esthétique immediate-mode peut paraître trop "outil dev" pour les créatifs | Investir dans le theming tôt. L'éditeur MVP sert les développeurs ; un éditeur Tauri poli pour non-programmeurs est une considération v0.3. |
| **Surface API Lua trop large** | Binder toute l'API moteur à Lua est fastidieux et sujet aux erreurs | Utiliser les macros `UserData` de mlua. Commencer avec une API minimale, étendre selon les besoins du jeu test. |
| **Scope creep features IA** | MCP, llms.txt, JSON Schema, erreurs structurées, CLI, mode headless = scope significatif | Le core moteur (S1-10) passe en premier. Les features IA sont en S11-12 et sont additives. |
| **Burnout développeur solo** | 12 semaines intensives | Timeline conçue avec le travail de rendu lourd en début (S3-4) quand la motivation est haute. Clone de Breakout à S6 = dopamine précoce. |
| **Temps de compilation Rust** | L'overhead de compilation pourrait ralentir la boucle edit-compile-test | Compilation incrémentale workspace, `cargo-watch`, profiler les temps de compilation hebdomadairement. La couche Lua fournit une échappatoire pour la logique gameplay. |
| **Échec de positionnement** | Le moteur lance mais ne trouve pas d'audience | L'angle AI-native est le coin d'entrée. Le premier adopteur cible est un dev qui utilise Claude/Cursor quotidiennement et veut un moteur qui travaille AVEC son workflow IA. |

---

## Position concurrentielle

| Critère | Toile (MVP) | Godot 4 | Construct 3 | Love2D | Bevy | GameMaker |
|---------|-------------|---------|-------------|--------|------|-----------|
| 2D pur (pas de baggage 3D) | Oui | Non | Oui | Oui | Non | Plutôt |
| Open source | Oui (MIT) | Oui (MIT) | Non | Oui (zlib) | Oui (MIT) | Non |
| Éditeur intégré | Oui | Oui | Oui | Non | Non | Oui |
| AI-native (MCP, llms.txt) | Oui | Après coup | Non | Non | Non | Non |
| Scripting Lua | Oui | Non | Non | Oui | Non | Non (GML) |
| Format de scène JSON | Oui | Non (.tscn) | Non | N/A | Non | Non |
| Sprite batcher | Oui | Oui | Oui | Oui | Oui | Oui |
| Export web (MVP) | Non (v0.2) | Oui | Oui | Non | Non | Oui |
| Taille d'export (hello world) | ~2 MB cible | ~20 MB | ~2 MB | ~3 MB | ~10 MB | ~15 MB |
| Sécurité mémoire | Oui (Rust) | Non (C++) | N/A (JS) | Non (C++) | Oui (Rust) | N/A |

---

*Ce document définit le scope, le stack et les critères de succès de Toile v0.1. Il est délibérément opinioné. Chaque choix technologique est un engagement, pas une suggestion. Le moteur est livré quand le jeu test platformer est jouable, éditable et pilotable par IA.*
