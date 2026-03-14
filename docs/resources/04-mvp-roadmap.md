# Roadmap MVP — Moteur 2D Haute Performance

## Table des matières
1. [Features MVP du core engine](#1-features-mvp-du-core-engine)
2. [Décisions d'architecture](#2-décisions-darchitecture)
3. [Roadmap phase par phase](#3-roadmap-phase-par-phase)
4. [Références open-source](#4-références-open-source)
5. [Benchmarks de performance cibles](#5-benchmarks-de-performance-cibles)
6. [Stratégie cross-plateforme](#6-stratégie-cross-plateforme)

---

## 1. Features MVP du core engine

Le MVP doit répondre à une question : **peut-on créer un jeu 2D simple et jouable avec ce moteur ?** Un platformer, un shoot, ou un puzzle doit être réalisable. Tout le reste est post-MVP.

### 1.1 Fenêtrage et gestion d'input

- Fenêtre native avec contexte de rendu GPU (OpenGL 3.3 / ES 3.0 comme baseline)
- Couche d'input abstraite : clavier (down/up/pressed), souris (position, boutons, scroll), gamepad (axes, boutons)
- Événements fenêtre : resize, focus, close, minimize, DPI scaling
- **Scope MVP** : fenêtre unique seulement, pas de multi-moniteur au-delà du DPI basique
- **Choix bibliothèque** : SDL3 ou GLFW pour le windowing ; `winit` en Rust

### 1.2 Rendu de sprites avec batching

- Support d'atlas de textures (pack de sprites en atlas, référence par région UV)
- **Sprite batching** : grouper les draw calls par texture et shader, soumettre la géométrie en bloc. **La feature de performance la plus importante pour la 2D.**
- Ordre de dessin : z-order explicite ou ordre de soumission
- Primitives basiques : rectangles, cercles, lignes (pour le debug rendering)
- Rendu de texte via bitmap fonts
- Modes de blend : alpha blending au minimum
- **Cible** : 10 000+ sprites par frame à 60 FPS sur GPU intégrés

### 1.3 Système de caméra

- Projection orthographique 2D
- Transform : position (pan), rotation, scale (zoom)
- Mapping viewport : coordonnées monde → écran et inversement (pour le mouse picking)
- **Scope MVP** : caméra unique. Split-screen post-MVP.

### 1.4 Détection de collision simple

- AABB vs AABB (bounding box aligné sur les axes)
- Cercle vs cercle, AABB vs cercle
- Point vs AABB, point vs cercle
- **Partitionnement spatial** : grille uniforme ou quadtree pour le broad-phase. Sans ça, la vérification de collision est O(n²).
- **Pas de simulation physique dans le MVP** — juste la détection d'overlap et la résolution basique (vecteur de push-out)

### 1.5 Playback audio basique

- Charger WAV et OGG
- Play, stop, pause, resume, loop
- Contrôle de volume par son et volume master
- Au moins 16 sons simultanés
- **Scope MVP** : pas d'audio positionnel 3D, pas d'effets, pas de streaming (tout en mémoire)
- **Bibliothèque** : miniaudio (C, single-header), ou `kira`/`rodio` en Rust

### 1.6 Game loop avec fixed timestep

```
accumulator = 0
previous_time = now()

while running:
    current_time = now()
    frame_time = current_time - previous_time
    previous_time = current_time
    frame_time = min(frame_time, MAX_FRAME_TIME)  // garde anti-spirale de mort

    accumulator += frame_time

    while accumulator >= FIXED_DT:
        update(FIXED_DT)       // physique/logique à taux fixe (ex: 60 Hz)
        accumulator -= FIXED_DT

    alpha = accumulator / FIXED_DT
    render(alpha)              // interpolation pour un rendu fluide
```

### 1.7 Chargement d'assets

- Chargement synchrone dans le MVP (async en Phase 2)
- Images : PNG, JPEG minimum (via `stb_image` ou crate `image`)
- Sons : WAV, OGG Vorbis
- Système de handles : charger une fois, retourner un handle léger, ref-counted ou arena-allocated
- Le hot-reloading est post-MVP mais le système de handles doit être conçu pour le rendre possible

---

## 2. Décisions d'architecture

### 2.1 Monolithique vs modulaire

**Recommandation : modulaire avec une façade.**

Modules séparés avec des frontières claires, mais un seul crate/bibliothèque de niveau supérieur qui ré-exporte tout :

```
engine/
  core/          -- types math, temps, logging, gestion d'erreurs
  platform/      -- fenêtre, input, abstraction plateforme
  graphics/      -- renderer, textures, shaders, batching
  audio/         -- chargement de sons, playback, mixing
  collision/     -- formes, tests d'overlap, partitionnement spatial
  assets/        -- chargement d'assets, gestion de handles
  app/           -- game loop, lifecycle de l'application, API de haut niveau
```

Ne pas sur-modulariser dans le MVP. Éviter des crates séparés pour "math" ou "logging".

### 2.2 Pipeline de rendu 2D

```
Code utilisateur
  |
  v
Commandes de dessin (sprites, formes, texte) → Buffer de commandes (trié par layer/z)
  |
  v
Batch builder : groupe les commandes consécutives partageant (texture, shader, blend mode)
  |
  v
Pour chaque batch :
  - Bind texture/shader/état
  - Upload vertex data (position, UV, couleur) dans un VBO dynamique
  - Émettre un seul appel glDrawElements
  |
  v
Swap buffers
```

**Décisions clés :**
- **VBO + IBO unique, mappé ou orphaned à chaque frame**
- **Format vertex** : position (vec2), UV (vec2), couleur (vec4 ou u32 packed), layer/z (float). 24-36 bytes par vertex. 4 vertices par sprite, 6 indices.
- **Un shader par défaut** pour les quads texturés. Shaders custom post-MVP.
- **Tri d'état** : trier les commandes par (layer, texture, shader) pour minimiser les changements d'état.

### 2.3 Système d'événements

**Recommandation : canaux d'événements typés, lus par frame.**

Éviter les systèmes basés sur des callbacks (callback hell, problèmes de lifetime). À la place :
- À chaque frame, la couche plateforme produit des événements dans des ring buffers typés
- Les systèmes lisent les événements qui les intéressent pendant leur phase d'update
- Les événements sont nettoyés à la fin de chaque frame

Types d'événements MVP :
- `WindowEvent` (resize, close, focus)
- `KeyEvent` (pressed, released, repeat)
- `MouseEvent` (moved, button pressed/released, scroll)
- `GamepadEvent` (connected, disconnected, button, axis)

### 2.4 Scene Graph vs ECS plat

**Recommandation : ECS plat avec relations parent-enfant optionnelles pour les transforms.**

Un scene graph complet (arbre de nœuds avec transforms hérités) crée des problèmes de performance. Un ECS plat est cache-friendly et composable.

Pour le MVP : un ECS simple ou un "struct of arrays" entity store. Support parent-enfant comme composant optionnel `Parent` / `Children`.

---

## 3. Roadmap phase par phase

### Phase 1 : Core Runtime (Semaines 1-6)

**Objectif** : Ouvrir une fenêtre, rendre des sprites, jouer des sons, gérer l'input.

| Semaine | Livrable |
|---------|----------|
| 1 | Création de fenêtre, contexte OpenGL, clear screen. Input polling (clavier, souris). Game loop fixed timestep. |
| 2 | Chargement de texture (PNG). Rendu d'un sprite unique (quad texturé). Caméra orthographique. |
| 3 | Sprite batching. 10 000 sprites à 60 FPS. Ordre de dessin basique (layers). |
| 4 | Audio : charger WAV/OGG, play/stop/volume. Mixer 16 canaux. |
| 5 | Utilitaires math (vec2, mat3/mat4, rect). Collision AABB et cercle. Broad-phase par grille spatiale. |
| 6 | Système de handles d'assets. Rendu de texte bitmap. Overlay debug (FPS, draw calls). |

**Critère de sortie** : On peut construire un clone de Breakout ou Flappy Bird.

### Phase 2 : Systèmes de jeu (Semaines 7-14)

**Objectif** : Assez d'infrastructure pour construire un vrai jeu indie.

| Semaine | Livrable |
|---------|----------|
| 7-8 | Système d'animation de sprites (sprite sheets, séquences de frames, contrôle de playback). Bibliothèque de tweening (lerp, fonctions d'easing). |
| 9-10 | Système de tilemap : charger Tiled (TMX/JSON), rendre les layers de tiles, collision basée sur les tiles. |
| 11-12 | Physique 2D simple : rigid bodies, gravité, intégration de vélocité, réponse de collision basique. |
| 13 | Système de particules (émetteur, propriétés : lifetime, vélocité, couleur, taille). |
| 14 | Gestion de scènes (load/unload, transitions). Chargement d'assets asynchrone. |

**Critère de sortie** : On peut construire un platformer avec plusieurs niveaux, personnages animés, effets de particules et monde en tilemap.

### Phase 3 : Éditeur / Outils de création (Semaines 15-22)

**Objectif** : Outils visuels pour ne plus éditer du JSON à la main.

| Semaine | Livrable |
|---------|----------|
| 15-16 | Intégration ImGui (ou egui en Rust) pour l'UI debug in-engine. Inspecteur d'entités. |
| 17-18 | Éditeur de niveaux : placer des entités, éditer des tilemaps, sauver/charger des scènes. |
| 19-20 | Éditeur d'animations : définir des animations de sprites visuellement, prévisualiser le playback. |
| 21-22 | Profiler overlay (breakdown du frame time, usage mémoire). Navigateur d'assets. |

**Critère de sortie** : Un non-programmeur peut ouvrir l'éditeur, placer des objets, définir des animations, et exporter un niveau jouable.

### Phase 4 : Couche de scripting (Semaines 23-28)

**Objectif** : Permettre la logique de gameplay dans un langage de scripting.

| Semaine | Livrable |
|---------|----------|
| 23-24 | Intégrer un langage de scripting. Candidats : Lua (via mlua/rlua), Wren, Rhai (Rust-natif). Lua est le choix le plus sûr. |
| 25-26 | Binder les API core du moteur aux scripts : création d'entités, requêtes d'input, playback audio, callbacks de collision. |
| 27-28 | Hot-reload des scripts au runtime. Comportements d'objets de jeu scriptés. |

**Critère de sortie** : La logique de gameplay (IA ennemis, pickups, triggers de dialogue) peut être écrite entièrement en scripts et rechargée sans redémarrer le moteur.

### Phase 5 : Polish et pipeline d'assets (Semaines 29-36)

| Semaine | Livrable |
|---------|----------|
| 29-30 | Pipeline d'assets : packing d'atlas offline, compression, bundles binaires. |
| 31-32 | Builds cross-plateforme : CI pour Windows, macOS, Linux. Export WASM (backend WebGL2). |
| 33-34 | Améliorations audio : streaming pour la musique, crossfade, DSP basique. |
| 35-36 | Documentation, jeux exemples (3-4 samples complets), série de tutoriels. |

---

## 4. Références open-source

| Projet | Langage | À étudier | À éviter |
|--------|---------|-----------|----------|
| **Bevy** | Rust | ECS archetype, système de plugins, `Handle<T>`, `Events<T>` | Complexité du renderer 3D, scheduling sophistiqué |
| **macroquad** | Rust | Ergonomie API, batching derrière une API immediate-mode | Manque de structure pour les grands jeux |
| **Raylib** | C | Nommage des fonctions, API minimale mais complète | Pas de batching intégré, pas d'ECS |
| **Love2D** | C++/Lua | Lifecycle `load/update/draw`, modules `love.*` | Performance ceiling plus bas (Lua) |
| **Godot** | C++ | Scènes comme prefabs réutilisables, signals | Scope 500k+ LOC, ne pas répliquer |
| **Comfy** | Rust | Petit moteur 2D opinioné, bon exemple minimal | — |
| **GGEZ** | Rust | API Love2D-inspired, bonne référence 2D | — |
| **Ebitengine** | Go | Simplicité, bonne performance avec API minimale | — |
| **Sokol** | C | `sokol_gfx.h` pour une abstraction graphique propre et minimale | — |

---

## 5. Benchmarks de performance cibles

### Rendu de sprites

| Métrique | Cible MVP | Cible stretch |
|----------|----------|---------------|
| Max sprites (60 FPS, GPU intégré) | 10 000 | 50 000 |
| Max sprites (60 FPS, GPU dédié) | 50 000 | 200 000 |
| Draw calls par frame (10k sprites) | < 20 | < 5 |
| Efficacité batch (sprites/draw call) | 500+ | 2 000+ |
| Upload vertex buffer (10k sprites) | < 0.5 ms | < 0.2 ms |

### Budget temps par frame (60 FPS = 16.67 ms)

| Phase | Budget |
|-------|--------|
| Input polling | < 0.1 ms |
| Logique de jeu (update) | < 4.0 ms |
| Détection de collision (1 000 entités) | < 1.0 ms |
| Détection de collision (10 000 entités) | < 3.0 ms |
| Construction et tri des batches | < 1.0 ms |
| Soumission GPU (draw calls) | < 2.0 ms |
| Mixage audio | < 0.5 ms |
| Buffer swap + vsync | reste |
| **Total CPU** | **< 8.0 ms** |

### Mémoire

| Métrique | Cible |
|----------|-------|
| Overhead moteur (pas de jeu chargé) | < 20 MB |
| Mémoire par entité (transform, sprite, collider) | < 256 bytes |
| 10 000 entités total | < 5 MB |

### Temps de démarrage

| Métrique | Cible |
|----------|-------|
| Fenêtre visible | < 500 ms |
| Premier frame rendu | < 1 seconde |
| Petit jeu entièrement chargé (50 textures, 20 sons) | < 3 secondes |

### Points de référence

- **macroquad** : 100 000+ sprites à 60 FPS sur matériel moderne
- **Raylib** : ~50 000 sprites confortablement avec son système de batch
- **Love2D** : SpriteBatch de 10 000+ sprites typique dans les jeux publiés
- **Ebitengine** : 50 000 sprites à 60 FPS atteignable

Viser la performance de macroquad/Raylib à la fin de la Phase 1.

---

## 6. Stratégie cross-plateforme

### Tier 1 : Desktop (dès la Phase 1)

**Windows, macOS, Linux** — supportés dès le jour 1.

- **Graphiques** : OpenGL 3.3 Core Profile comme baseline (compatible avec ~100% du matériel desktop des 12 dernières années)
- **Windowing** : SDL3 ou `winit` (Rust)
- **Audio** : miniaudio (C) ou `kira`/`rodio` (Rust)
- **macOS** : gérer le DPI Retina/HiDPI. Apple a déprécié OpenGL → prévoir un backend Metal post-MVP

### Tier 2 : Web / WASM (Phase 5)

- **Graphiques** : WebGL2 (mappe étroitement sur OpenGL ES 3.0). Si le renderer desktop utilise OpenGL 3.3 Core sans extensions, le portage vers WebGL2 est mécanique.
- **Audio** : WebAudio API (backend séparé nécessaire)
- **Build** : WASM via Emscripten (C/C++) ou `wasm-pack`/`wasm-bindgen` (Rust)
- **Limitations** : pas d'accès filesystem (VFS ou fetch HTTP), pas de threads
- **Performance** : WASM = 70-90% du natif. Le batching est encore plus critique car les draw calls WebGL2 ont un overhead plus élevé.

### Tier 3 : Mobile (Post-MVP)

- **iOS** : backend Metal (OpenGL ES déprécié). Touch input, safe areas, notches.
- **Android** : OpenGL ES 3.0 (compatibilité large) ou Vulkan. NDK build.
- **Recommandation** : Ne pas penser au mobile tant que desktop et web ne sont pas solides. Le mobile a une complexité plateforme disproportionnée.

### Design de la couche d'abstraction

```rust
trait Platform {
    fn create_window(config: WindowConfig) -> Window;
    fn poll_events() -> Vec<PlatformEvent>;
    fn swap_buffers();
    fn elapsed_time() -> Duration;
}

trait GraphicsBackend {
    fn create_texture(data: &[u8], width: u32, height: u32) -> TextureHandle;
    fn draw_batch(vertices: &[Vertex], indices: &[u16], texture: TextureHandle);
    fn set_viewport(x: i32, y: i32, w: u32, h: u32);
}

trait AudioBackend {
    fn load_sound(data: &[u8], format: AudioFormat) -> SoundHandle;
    fn play(sound: SoundHandle, params: PlayParams);
    fn stop(sound: SoundHandle);
}
```

Écrire le moteur contre ces traits. Implémenter `DesktopPlatform + OpenGLBackend + MiniaudioBackend` pour la Phase 1. Les backends web et mobile viendront plus tard. Le code moteur ne change jamais — seules les implémentations de backends changent.

---

## Résumé : quoi construire en premier

La chose la plus importante est d'obtenir un triangle à l'écran, puis un quad texturé, puis 10 000 quads texturés à 60 FPS. Tout le reste suit.

**Priorité pour le premier mois :**

1. Fenêtre + contexte GL + game loop
2. Rendu de quad texturé
3. Sprite batching (c'est là que la performance se joue)
4. Gestion d'input
5. Caméra (projection orthographique + transform)
6. Playback audio
7. Détection de collision

Ne pas construire d'ECS, de scene graph, de pipeline d'assets ou d'éditeur tant qu'on ne peut pas rendre 10 000 sprites avec de l'audio et de l'input fonctionnels.
