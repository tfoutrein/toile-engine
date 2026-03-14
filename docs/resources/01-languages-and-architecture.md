# Langages & Architecture pour un Moteur 2D Haute Performance

## Table des matières
1. [Comparaison des langages](#1-comparaison-des-langages)
2. [Backends de rendu](#2-backends-de-rendu)
3. [Architecture ECS](#3-architecture-ecs)
4. [Gestion mémoire](#4-gestion-mémoire)
5. [Moteurs 2D existants à étudier](#5-moteurs-2d-existants-à-étudier)
6. [Recommandations MVP](#6-recommandations-mvp)

---

## 1. Comparaison des langages

### C

| Aspect | Évaluation |
|---|---|
| **Performance** | Contrôle maximal ; zéro abstractions. GCC/Clang produisent un code excellent. |
| **Écosystème** | Massif. Chaque SDK plateforme, chaque API graphique a une interface C. SDL2, Raylib, stb sont tous en C. |
| **Sécurité** | Aucune. Gestion mémoire manuelle, pas de bounds checking, UB partout. |
| **Build System** | CMake (standard de facto mais mal aimé). Pas de gestionnaire de paquets standardisé. |
| **Pour un moteur 2D** | Excellent pour la simplicité et la portabilité. Raylib prouve qu'on peut construire un framework 2D génial en C pur. |

**Verdict** : Idéal pour les moteurs minimalistes ou comme base à wrapper dans un langage de scripting.

### C++

| Aspect | Évaluation |
|---|---|
| **Performance** | À parité avec C. Templates = abstractions zero-cost. Move semantics, SIMD bien supporté. |
| **Écosystème** | Le plus mature en développement de jeux. Unreal, Godot (core), tous les moteurs AAA. Bibliothèques : EnTT, GLM, Dear ImGui, Box2D, etc. |
| **Sécurité** | Meilleure que C avec RAII, smart pointers, `std::span`, mais UB toujours possible. C++20/23 avec sanitizers atténue beaucoup de problèmes. |
| **Build System** | CMake domine. vcpkg et Conan pour les packages. Les modules C++20 améliorent lentement les temps de compilation. |
| **Pour un moteur 2D** | Le choix éprouvé. Tout le stack 2D existe et est battle-tested. |

**Verdict** : Le choix pragmatique et sûr. Plus grand écosystème, plus de documentation, plus de potentiel de recrutement.

### Rust

| Aspect | Évaluation |
|---|---|
| **Performance** | Comparable à C/C++. Backend LLVM. Garantie d'absence de data races à la compilation. |
| **Écosystème** | En croissance rapide. Bevy est le moteur phare. `wgpu` (Rust-native) est la bibliothèque graphique de référence. crates.io offre un excellent gestionnaire de paquets. Bibliothèques : `glam` (math), `rapier` (physique), `winit` (fenêtrage), `gilrs` (gamepad). |
| **Sécurité** | L'avantage majeur. Ownership/borrowing empêche use-after-free, double-free et data races à la compilation. |
| **Build System** | `cargo` est best-in-class. Dépendances, build, tests, benchmarks, docs — tout unifié. |
| **Pour un moteur 2D** | Excellent. Le borrow checker peut sembler restrictif pour certains patterns (parent-enfant, structures auto-référentielles), mais l'ECS fonctionne naturellement avec le modèle d'ownership de Rust. |

**Verdict** : Le choix le plus tourné vers l'avenir. Les garanties de sécurité éliminent toute une classe de bugs. L'écosystème est désormais assez mature. Les temps de compilation et la courbe d'apprentissage sont les inconvénients principaux.

### Zig

| Aspect | Évaluation |
|---|---|
| **Performance** | Comparable à C. Comptime (exécution compile-time) permet des génériques zero-cost sans templates ni macros. |
| **Écosystème** | Encore petit. Le langage approchait la stabilité 1.0 en 2025 mais n'y est pas encore. Bibliothèques clés : `zig-gamedev`, moteur `mach`. |
| **Sécurité** | Meilleure que C : pas de flux de contrôle caché, types optionnels au lieu de pointeurs null, vérifications runtime en debug. Mais pas de borrow checker. |
| **Build System** | Système de build intégré excellent. La cross-compilation est de première classe. Peut aussi compiler du C/C++. |
| **Pour un moteur 2D** | Prometteur mais risqué. L'interop C est transparente (`@cImport`), donc toutes les bibliothèques C fonctionnent directement. |

**Verdict** : L'option "nouvelle" la plus excitante. La cross-compilation est imbattable. Cependant, le statut pré-1.0 implique des breaking changes et un écosystème mince.

### Matrice de synthèse

| Critère | C | C++ | Rust | Zig |
|---|---|---|---|---|
| Performance brute | A+ | A+ | A+ | A+ |
| Maturité écosystème | A | A+ | B+ | C+ |
| Sécurité mémoire | D | C+ | A+ | B |
| Système de build | C | C+ | A+ | A |
| Bibliothèques 2D | A | A+ | B+ | C |
| Courbe d'apprentissage | B | C | C- | B+ |
| Cross-compilation | C | C | B | A+ |
| Viabilité long terme | B | A | A+ | B+ |

---

## 2. Backends de rendu

### OpenGL (3.3+ / ES 3.0)

- **Avantages** : API simple pour la 2D. Fonctionne partout (Windows, Linux, macOS via profil compatibilité, mobile via ES, web via WebGL). Immense base de tutoriels.
- **Inconvénients** : Déprécié sur macOS (Apple s'est arrêté à OpenGL 4.1). API à états, sujette aux erreurs. Pas de compute shaders sous GL 4.3.
- **Pour la 2D** : Toujours le chemin le plus rapide vers un renderer 2D fonctionnel.

### Vulkan

- **Avantages** : Performance et contrôle maximaux. Multi-threading explicite. Disponible sur Windows, Linux, Android, et macOS/iOS via MoltenVK.
- **Inconvénients** : Extrêmement verbeux (un "hello triangle" = ~1000 lignes). Overkill pour la 2D.
- **Pour la 2D** : Non recommandé comme backend direct. La complexité n'est pas justifiée pour du rendu 2D.

### Metal

- **Avantages** : First-class sur les plateformes Apple. API moderne et propre. Excellents outils (Xcode GPU debugger).
- **Inconvénients** : Apple uniquement.
- **Pour la 2D** : Nécessaire pour la performance native Apple, mais seulement comme un backend derrière une couche d'abstraction.

### SDL2 / SDL3 Renderer

- **Avantages** : Le renderer 2D intégré de SDL gère le rendu de sprites basique et utilise automatiquement le meilleur backend. SDL3 (2024-2025) modernise l'API avec un accès GPU explicite.
- **Inconvénients** : Limité — pas de shaders custom (dans SDL2 ; SDL3 ajoute une API GPU). On en atteint vite les limites.
- **Pour la 2D** : Excellent pour un MVP ou prototype. SDL pour le windowing/input même avec un autre renderer.

### wgpu / WebGPU

- **Avantages** : Abstraction moderne, safe, cross-plateforme sur Vulkan, Metal, D3D12 et OpenGL. Conçu pour le web (WebGPU) mais fonctionne en natif. Rust-native (`wgpu` crate) avec bindings C (`wgpu-native`). API propre et bien conçue. Compute shaders intégrés.
- **Inconvénients** : Léger overhead de la couche d'abstraction (négligeable pour la 2D). Le langage de shaders est WGSL (ou SPIR-V via `naga`).
- **Pour la 2D** : Le meilleur choix moderne pour un renderer 2D cross-plateforme.

### Approche recommandée

**Pour un MVP** : **SDL3 pour le windowing, input et audio** + **wgpu (ou SDL3 GPU API) pour le rendu**.

```
[Code du jeu]
    |
[API Renderer 2D]  (votre abstraction : sprite batches, caméras, layers)
    |
[wgpu / SDL3 GPU]  (abstraction GPU cross-plateforme)
    |
[Vulkan | Metal | D3D12 | OpenGL]  (backends plateforme, gérés par wgpu)
```

Alternative en C/C++ : **sokol_gfx** — bibliothèque single-header cross-plateforme par Andre Weissflog. Cible GL3.3, Metal, D3D11 et WebGPU avec une API C propre.

---

## 3. Architecture ECS (Entity Component System)

### Pourquoi l'ECS est le standard

L'architecture OOP traditionnelle (hiérarchies d'héritage profondes : `Entity > Actor > Character > Player`) souffre de :
- **Le problème du diamant** : conflits d'héritage multiple
- **Hiérarchies rigides** : ajouter un nouveau comportement nécessite de restructurer l'arbre
- **Mauvaise performance cache** : objets dispersés en mémoire heap, chacun avec des pointeurs vtable

L'ECS résout les trois problèmes :

1. **Composition plutôt qu'héritage** : une entité est juste un ID. Le comportement vient de la combinaison de composants (données) traités par des systèmes (logique).
2. **Cache-friendly** : les composants du même type sont stockés de manière contiguë en mémoire (Structure of Arrays).
3. **Parallélisme** : les systèmes déclarent quels composants ils lisent/écrivent. Les systèmes non-conflictuels peuvent tourner en parallèle.
4. **Hot-reload friendly** : ajouter/retirer des composants au runtime est trivial.

### Concepts fondamentaux

```
Entity:     Juste un ID (u32 ou u64). Pas de données, pas de comportement.
Component:  Struct de données pure. Pas de méthodes (idéalement).
            Exemples : Position { x, y }, Sprite { texture_id, rect }, Velocity { dx, dy }
System:     Fonction qui itère sur les entités avec des combinaisons de composants spécifiques.
            Exemple : movement_system requête tous les (Position, Velocity) et met à jour Position.
```

### Archetype vs Sparse-Set ECS

**Archetype** (Bevy, Unity DOTS, flecs) :
- Entités avec le même set de composants groupées en "archetypes"
- Itération extrêmement rapide (accès mémoire linéaire)
- Ajouter/retirer des composants déplace l'entité entre archetypes (a un coût)

**Sparse-set** (EnTT) :
- Chaque type de composant a son propre sparse set
- Ajout/retrait de composants en O(1)
- Itération nécessite l'intersection de sets

### Meilleures implémentations ECS

| Bibliothèque | Langage | Architecture | Notes |
|---|---|---|---|
| **EnTT** | C++ | Sparse set | Standard de l'industrie C++. Utilisé dans Minecraft. Single-header. |
| **flecs** | C | Archetype | ECS complet avec langage de requêtes, réflexion, API REST pour le debug. |
| **Bevy ECS** | Rust | Archetype | Partie de Bevy mais utilisable standalone. Scheduling compile-time, parallélisme auto. |
| **hecs** | Rust | Archetype | Minimal, sans fioritures. Excellent pour l'apprentissage. |

---

## 4. Gestion mémoire

### Stratégie 1 : Arena / Bump Allocator

```
struct Arena {
    buffer: [u8; CAPACITY],
    offset: usize,
}

fn alloc(arena, size, align) -> *void {
    aligned_offset = align_up(arena.offset, align);
    arena.offset = aligned_offset + size;
    return &arena.buffer[aligned_offset];
}

fn reset(arena) {
    arena.offset = 0;  // "libère" tout d'un coup
}
```

- **Usage** : allocations temporaires par frame, scratch space, listes de commandes de rendu
- **Avantages** : allocation O(1), zéro fragmentation, désallocation de masse
- **C'est l'allocateur le plus important pour un moteur de jeu.**

### Stratégie 2 : Pool Allocator (Free List)

- **Usage** : objets de même taille avec durée de vie dynamique — entités, particules, bullets
- **Avantages** : alloc et free O(1), pas de fragmentation, itération cache-friendly

### Stratégie 3 : Index générationnels

Pattern critique pour les moteurs de jeu. Au lieu de pointeurs bruts (qui peuvent devenir dangling), utilisez :

```
struct GenIndex {
    index: u32,       // slot dans le pool
    generation: u32,  // incrémenté à chaque réutilisation du slot
}
```

Empêche les use-after-free pour les handles d'entités. Le type `Entity` de Bevy est exactement ce pattern.

### Stratégie 4 : SoA (Structure of Arrays)

```
// AoS (Array of Structures) — mauvaise utilisation du cache
struct Entity { position: Vec2, velocity: Vec2, health: f32, sprite_id: u32 }
entities: [Entity; 10000]

// SoA (Structure of Arrays) — excellente utilisation du cache
positions:  [Vec2; 10000]
velocities: [Vec2; 10000]
healths:    [f32; 10000]
sprite_ids: [u32; 10000]
```

### Architecture mémoire pratique

```
Layout mémoire du jeu :
|-- Arena permanente (vit indéfiniment : métadonnées assets, config)
|-- Arena de niveau (libérée à la transition de niveau)
|-- Arena frame A (ping-pong : commandes de rendu, strings temporaires)
|-- Arena frame B (ping-pong)
|-- Pool d'entités (générationnel, pour l'ECS)
|-- Pools de composants (un par type de composant)
|-- Heap d'assets (textures, sons — gestion séparée, possiblement ref-counted)
```

---

## 5. Moteurs 2D existants à étudier

### Godot (C++, GDScript, C#)
- **Architecture** : Arbre de scènes avec composition par nœuds.
- **Points forts** : Éditeur exceptionnel (construit avec Godot lui-même). GDScript conçu pour la logique de jeu. La 2D est un citoyen de première classe. Tilemap, physique 2D, éclairage 2D, particules 2D intégrés.
- **À retenir** : Le modèle de composition scène/nœuds est intuitif. Étudier la séparation `_process()` / `_physics_process()`.

### Bevy (Rust)
- **Architecture** : ECS pur. Tout est un composant. Le scheduler parallélise automatiquement.
- **Points forts** : Le design ECS le plus propre. Architecture de plugins. Hot reloading d'assets. Renderer basé sur `wgpu`.
- **À retenir** : Comment structurer un moteur basé ECS. Pattern `App` builder, trait `Plugin`, distinction `Resource` vs `Component`.

### Raylib (C)
- **Architecture** : Immediate-mode, style single-header. Pas d'ECS, pas de scene graph. Juste des fonctions.
- **Points forts** : Simplicité inégalée. Un sprite en 5 lignes. Multi-plateforme (desktop, mobile, web, Raspberry Pi).
- **À retenir** : La simplicité d'API. Étudier `raylib.h` — un masterclass en design d'API.

### Love2D (C++/Lua)
- **Architecture** : Core C++ avec couche de scripting Lua. Callbacks : `love.load()`, `love.update(dt)`, `love.draw()`.
- **Points forts** : Le chemin le plus rapide de l'idée au jeu jouable. Batching automatique. Celeste a été prototypé dans Love2D.
- **À retenir** : Le modèle de callbacks (`load`, `update`, `draw`) est l'abstraction de game loop la plus simple viable.

### macroquad (Rust)
- **Architecture** : Immediate-mode, inspiré de Raylib. Utilise `miniquad` comme backend de rendu.
- **Points forts** : Simplicité rivalisant Raylib mais en Rust. Support WASM/web first-class. Temps de compilation très rapides.
- **À retenir** : Comment construire une abstraction de rendu minimale. L'architecture de `miniquad`.

---

## 6. Recommandations MVP

### Stack recommandé

#### Option A : Le chemin pragmatique (Rust + wgpu)

```
Langage :    Rust
Windowing :  winit ou SDL3
Rendu :      wgpu
ECS :        hecs ou custom
Math :       glam
Physique :   rapier (plus tard)
Audio :      kira ou rodio
Build :      Cargo
```

#### Option B : Le chemin éprouvé (C++ + SDL3)

```
Langage :    C++20
Windowing :  SDL3
Rendu :      SDL3 GPU API ou sokol_gfx
ECS :        EnTT
Math :       GLM
Physique :   Box2D (plus tard)
Audio :      SDL3_mixer ou miniaudio
Build :      CMake + vcpkg
```

#### Option C : Le chemin minimaliste (C + SDL3)

```
Langage :    C11
Windowing :  SDL3
Rendu :      sokol_gfx ou SDL3 GPU API
ECS :        flecs
Math :       HandmadeMath.h
Audio :      miniaudio
Build :      CMake ou système de build Zig
```

### Principes d'architecture

1. **Data-oriented, pas object-oriented.** Préférer les tableaux plats aux arbres de pointeurs.
2. **Séparer update et rendu.** La simulation produit un "snapshot de rendu" que le renderer consomme.
3. **Références par handles, pas par pointeurs bruts.** Utiliser des index générationnels.
4. **Couches d'abstraction** : Code jeu → Services moteur → Renderer 2D → Abstraction plateforme → OS/GPU
5. **Concevoir pour le hot-reload dès le jour 1.**
6. **Profiler avant d'optimiser.** Un sprite batcher naïf est assez rapide pour la plupart des jeux 2D.
