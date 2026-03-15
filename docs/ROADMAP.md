# Toile Engine — Roadmap Complète

**Document vivant** | Dernière mise à jour : 2026-03-15

---

## Table des matières

1. [Vision & Thèmes directeurs](#vision--thèmes-directeurs)
2. [Timeline visuelle](#timeline-visuelle)
3. [v0.1 — "First Light" (MVP)](#v01--first-light-mvp-12-semaines)
4. [v0.2 — "Game Systems"](#v02--game-systems-8-semaines)
5. [v0.3 — "Creator Tools"](#v03--creator-tools-8-semaines)
6. [v0.4 — "Visual Polish"](#v04--visual-polish-8-semaines)
7. [v0.5 — "Complete Editor"](#v05--complete-editor-10-semaines)
8. [v1.0 — "Production Ready"](#v10--production-ready-12-semaines)
9. [v1.5 — "Web & Share"](#v15--web--share-8-semaines)
10. [v2.0 — "Connected"](#v20--connected-12-semaines)
11. [v3.0 — "Ecosystem"](#v30--ecosystem-continu)
12. [Ce qu'on reporte délibérément et pourquoi](#ce-quon-reporte-délibérément-et-pourquoi)
13. [Jalons communautaires](#jalons-communautaires)

---

## Vision & Thèmes directeurs

Ce moteur existe pour combler le "missing middle" : le fossé entre les outils **simples mais limités** (Construct, RPG Maker) et les moteurs **puissants mais écrasants** (Unity, Godot). Notre étoile polaire est la **divulgation progressive de complexité** — les choses simples sont simples, les choses complexes sont possibles, et rien n'est une boîte noire.

### Thème 1 : La 2D n'est pas de la 3D simplifiée

Chaque moteur concurrent traite la 2D comme un sous-ensemble de la 3D. Nous construisons **2D-first** : rendu pixel-perfect, gestion de monde tile-based, animation frame-by-frame, effets screen-space, et physique de platformer sont des primitives core, pas des ajouts. Résultat : exports plus petits, démarrage plus rapide, meilleure performance.

### Thème 2 : Contrôle avec commodité

Défauts opinionés qui marchent out-of-the-box, combinés avec la possibilité de remplacer n'importe quel système. Philosophie "bibliothèque, pas framework" au niveau architecture : le moteur est une composition de modules remplaçables derrière une façade de haut niveau. Chaque couche est inspectable. Pas de boîtes noires.

### Thème 3 : L'IA comme citoyen de première classe

Les MCP servers sont greffés sur Unity et Godot après coup. Nous concevons pour le contrôle IA dès le jour 1 : format de scène JSON déclaratif validé par JSON Schema, serveur MCP natif, réponses d'erreur structurées, mode headless, et documentation `llms.txt`.

### Thème 4 : Confiance et durabilité

Post-crise Unity, les développeurs ont besoin de confiance. Open-source (MIT), APIs stables versionnées avec politiques de dépréciation claires, fichiers de scène conçus pour le version control, et aucune entité corporate qui puisse changer les règles unilatéralement.

### Thème 5 : L'accessibilité est une feature, pas un ajout

Aucun moteur 2D majeur ne fournit nativement le support lecteur d'écran, les modes daltonien, ou le remapping d'input. Nous construisons le framework d'accessibilité dans le moteur lui-même.

---

## Timeline visuelle

```
2026       Q2           Q3           Q4        2027 Q1         Q2           Q3
 |           |            |            |            |            |            |
 |===v0.1====|            |            |            |            |            |
 |  12 sem   |==v0.2===|  |            |            |            |            |
 |  "First   | 8 sem   |  |            |            |            |            |
 |   Light"  | "Game   |==v0.3===|    |            |            |            |
 |           | Systems"| 8 sem   |    |            |            |            |
 |           |         |"Creator |==v0.4===|       |            |            |
 |           |         | Tools"  | 8 sem   |       |            |            |
 |           |         |         |"Visual  |==v0.5====|         |            |
 |           |         |         | Polish" | 10 sem   |         |            |
 |           |         |         |         |"Complete |===v1.0=====|         |
 |           |         |         |         | Editor"  | 12 sem     |         |
 |           |         |         |         |          |"Production |==v1.5===|
 |           |         |         |         |          | Ready"     | 8 sem   |
 |           |         |         |         |          |            |"Web &   |=>v2.0=>
 |           |         |         |         |          |            | Share"  |
```

| Version | Codename | Durée | Semaines cumulées |
|---------|----------|-------|-------------------|
| v0.1 | First Light | 12 semaines | 12 |
| v0.2 | Game Systems | 8 semaines | 20 |
| v0.3 | Creator Tools | 8 semaines | 28 |
| v0.4 | Visual Polish | 8 semaines | 36 |
| v0.5 | **Complete Editor** | 10 semaines | 46 |
| v1.0 | Production Ready | 12 semaines | 58 |
| v1.5 | **Web & Share** | 8 semaines | 66 |
| v2.0 | Connected | 12 semaines | 78 |
| v3.0 | Ecosystem | Continu | — |

**Total jusqu'à la release stable v1.0 : ~58 semaines (~13.5 mois)**

---

## v0.1 — "First Light" (MVP, 12 semaines)

> Voir le document détaillé : [MVP-PROPOSAL.md](./MVP-PROPOSAL.md)

**Philosophie :** Prouver que la boucle fondamentale fonctionne. Un développeur peut ouvrir une fenêtre, rendre des sprites, gérer l'input, jouer de l'audio, détecter des collisions, et construire un jeu simple (Breakout, Flappy Bird).

### Features

| Catégorie | Features |
|-----------|----------|
| **Runtime** | Fenêtre SDL3, game loop fixed timestep, input (clavier/souris/gamepad), sprite batching (10k+ à 60fps), animation sprites, caméra 2D, collision AABB/cercle, audio WAV/OGG, texte BMFont/TTF, tilemap Tiled JSON, overlay debug |
| **Éditeur** | Canvas pan/zoom, placement d'entités, gizmos transform, inspecteur, hiérarchie, asset browser, undo/redo, save/load JSON, bouton Play |
| **Scripting** | Lua (mlua/LuaJIT), callbacks lifecycle, bindings API moteur, hot-reload |
| **IA** | Serveur MCP skeleton, JSON Schema, llms.txt, CLI, erreurs structurées, mode headless |
| **Formats** | PNG, WAV, OGG, TexturePacker JSON, Aseprite JSON, Tiled JSON, BMFont, TTF |
| **Plateformes** | Windows, macOS, Linux |

### Critère de sortie
Un platformer avec joueur animé, 3 niveaux Tiled, collision, audio, caméra suivant le joueur, logique en Lua avec hot-reload, éditable dans l'éditeur, pilotable par IA via MCP.

---

## v0.2 — "Game Systems" (8 semaines)

**Philosophie :** Assez d'infrastructure pour construire un vrai jeu indie.

### Nouvelles features

| Catégorie | Features |
|-----------|----------|
| **Animation** | Système d'animation complet : séquences de frames, tags Aseprite, state machine pour transitions. Tweening (linear, ease-in/out, bezier). |
| **Tilemap** | Tiled JSON complet : tile layers (CSV + base64+zlib), object layers, image layers. GID flip-bits. Collision par tile. |
| **Particules** | Simulation CPU. Émetteurs : point, cercle, rectangle, ligne. Lifetime, vélocité, gravité, taille/couleur over life (courbe/gradient). 8 presets. |
| **Physique** | Rapier2D : rigid bodies (dynamic, static, kinematic). Gravité. Réponse de collision. |
| **Scènes** | Pile de scènes (push/pop). Transitions (fade, slide). Chargement async. |
| **Assets** | Chargement async en background thread. Handles avec futures. |

### Critère de sortie
Un platformer avec personnages animés, niveaux tilés, effets de particules. 5 000 particules à 60 FPS.

---

## v0.3 — "Creator Tools" (8 semaines)

**Philosophie :** Combler le fossé pour les non-programmeurs. Event sheets + behaviors + prefabs.

### Features

| Feature | Détails |
|---------|---------|
| **Event Sheets** | Paires condition-action. 8 conditions, 11 actions. Exécuteur avec état par entité. |
| **Behaviors** | 7 pre-built : Platform, TopDown, Bullet, Sine, Fade, Wrap, Solid. Tous sérialisables. |
| **Prefabs** | Template réutilisable avec behaviors + event sheet reference. MCP create/list/instantiate. |
| **Templates** | 4 templates de projet : empty, platformer, topdown, shmup. `toile new`. |
| **LDtk** | Import .ldtk JSON : IntGrid, entités, multi-niveaux. |
| **Aseprite** | Parser binaire .ase/.aseprite direct. |

### Critère de sortie
Un non-programmeur construit un platformer jouable en event sheets et behaviors en < 30 minutes.

---

## v0.4 — "Visual Polish" (8 semaines) ✅

**Philosophie :** Élever le plafond de qualité visuelle.

### Features livrées

| Feature | Détails |
|---------|---------|
| **Post-Processing** | Pipeline chainable : Bloom, CRT, Vignette, Pixelate, Screen Shake, Color Grading |
| **Éclairage 2D** | Point lights avec falloff, radius, intensité, couleur configurable |
| **Ombres 2D** | Shadow maps 1D, PCF soft shadows, ray marching |
| **SDF Fonts** | Texte net à toute échelle depuis un atlas 32px. Outline, drop shadow, animated glow |
| **Shader Graph** | Graphe de nœuds → compilateur WGSL. PostEffect::Custom. 4 effets démo |
| **Particle Editor** | Intégré dans l'éditeur (mode Particles). Curve editor, gradient editor, sub-emitters, JSON save/load |
| **MCP** | 5 nouveaux outils particles (20 total) |
| **CLI** | `toile editor` |

---

## v0.5 — "Complete Editor" (10 semaines)

> ADR : [031-v05-editor-mvp-complet.md](adr/031-v05-editor-mvp-complet.md)

**Philosophie :** Connecter toutes les briques. Un utilisateur crée un jeu complet (menus → gameplay → game over) depuis l'éditeur et le lance avec `toile run`. Toutes les features existantes (behaviors, event sheets, particules, transitions) deviennent accessibles depuis l'UI.

### Le problème

Toutes les briques existent mais sont **orphelines** :

| Système | Sérialisable | Éditeur UI | Runtime | Verdict |
|---------|---|---|---|---|
| Behaviors (7 types) | ✅ | ❌ | ❌ | Orphelin |
| Event Sheets (8 cond, 11 actions) | ✅ | ❌ | ❌ | Orphelin |
| Prefabs | ✅ | ❌ | ⚠️ MCP only | Orphelin |
| Particules | ✅ | ✅ panel | ⚠️ Demo | Partiel |
| Scene transitions | ✅ code | ❌ | ❌ | Orphelin |
| Éclairage/Ombres | ✅ code | ❌ | ⚠️ Demo | Orphelin |
| Post-processing | ✅ code | ❌ | ⚠️ Demo | Orphelin |

### Phase 1 : Format + Runtime (fondations)

**Format de scène étendu :**

```rust
// EntityData v2
pub behaviors: Vec<BehaviorConfig>,
pub event_sheet: Option<String>,
pub particle_emitter: Option<String>,
pub tags: Vec<String>,
pub variables: HashMap<String, f64>,
pub collision_shape: Option<CollisionShape>,
pub visible: bool,

// SceneData v2
pub background_color: [f32; 4],
pub camera_zoom: f32,
pub camera_position: [f32; 2],
pub gravity: [f32; 2],
```

**Game Runner :**
- BehaviorExecutor — exécute Platform/TopDown/Bullet/Sine/Fade/Wrap/Solid chaque frame
- EventSheetExecutor — évalue les event sheets, exécute les commandes
- CollisionSystem — détection AABB/Circle entre entités taggées
- ParticleManager — charge et update les émetteurs attachés
- SceneLoader — charge JSON, instancie textures, résout prefabs

**CLI :**
```bash
toile run                       # depuis Toile.toml
toile run --scene level2.json   # scène spécifique
```

### Phase 2 : Éditeur Inspector enrichi

| Section Inspector | Fonctionnalité |
|-------------------|----------------|
| **Behaviors** | +Add Behavior, dropdown, éditeur de paramètres par type |
| **Tags & Variables** | Chips éditables, variables initiales clé/valeur |
| **Collision Shape** | AABB/Circle, gizmo dans le viewport |
| **Particle Emitter** | Sélecteur .particles.json ou preset |
| **Sprite Selector** | Choisir une texture parmi les assets |
| **Scene Settings** | Background color, gravity, camera defaults |

### Phase 3 : Play Mode + Event Sheet Editor

- **Play/Stop** dans la toolbar de l'éditeur
- **Event Sheet Editor** : mode dédié, créer conditions + actions avec pickers visuels
- **Scene transitions** : GoToScene preview

### Phase 4 : MCP + Polish

| Nouvel outil MCP | Description |
|------------------|-------------|
| `add_behavior` | Ajouter un behavior à une entité |
| `remove_behavior` | Retirer un behavior |
| `set_entity_tags` | Définir les tags |
| `set_entity_variables` | Variables initiales |
| `create_event_sheet` | Créer un event sheet |
| `get_event_sheet` | Lire un event sheet |
| `update_event_sheet` | Modifier un event sheet |

### Critère de sortie
Un jeu de platformer complet (menu → 3 niveaux → game over) créé **entièrement depuis l'éditeur**, sans écrire une ligne de Rust. Lancé via `toile run`. Les behaviors Platform + Solid fonctionnent. Les event sheets gèrent les collectibles et les transitions de scène. Les particules sont attachées aux entités. Le tout sauvegarde/charge en JSON.

---

## v1.0 — "Production Ready" (12 semaines)

**Philosophie :** Tout ce dont une équipe a besoin pour livrer un jeu commercial.

### Nouvelles features

| Feature | Détails |
|---------|---------|
| **Pipeline d'assets** | Import → Process (atlas packing, compression) → Pack (.pak binaire zstd). |
| **Framework d'accessibilité** | Screen reader, modes daltonien, remapping input, scaling texte, réduction de mouvement. |
| **Localisation** | Tables de strings, pluralisation, support RTL, fallbacks polices CJK. |
| **Mode déterministe** | Maths fixed-point, RNG seedé, enregistrement/playback de replays. |
| **Profiling** | Breakdown frame time, heatmap overdraw, inspecteur mémoire. |
| **Documentation** | Référence API, guides, tutoriels, 4 jeux exemples complets. |
| **Undo/Redo avancé** | Historique complet dans l'éditeur avec groupes d'opérations. |

### Critère de sortie
Les 4 jeux exemples compilent et tournent sur Windows/macOS/Linux. Pipeline d'assets réduit 100 MB → < 15 MB. Screen reader lit les éléments UI. Documentation 100% de l'API publique. Zéro crash bug connu.

---

## v1.5 — "Web & Share" (8 semaines)

> Note : anciennement prévu comme v0.5. Reporté pour prioriser l'éditeur complet (ADR-031).
> Voir aussi : [ADR-018 (web export)](adr/018-web-export-wasm.md)

**Philosophie :** Le chemin le plus rapide de "j'ai fait un jeu" à "joue à mon jeu" est une URL.

### Nouvelles features

| Feature | Détails |
|---------|---------|
| **Export WASM/WebGPU** | Compilation via wasm-pack. Fallback WebGL2. |
| **Bundling web** | Assets en .bin unique, fetch HTTP, chargement streaming. |
| **Audio web** | Backend WebAudio API. Auto-unlock au premier input. |
| **Optimisation taille** | Tree-shaking, LTO, wasm-opt. Cible : < 3 MB hello world. |
| **Deploy itch.io** | `toile deploy itch <user/game>` via butler CLI. |
| **Preview navigateur** | `toile serve` — serveur HTTP local + hot-reload WebSocket. |
| **Input tactile** | Touch → pointer events. Gamepad virtuel on-screen optionnel. |

### Critère de sortie
Platformer 5 niveaux exporte en web < 3 MB. 60 FPS Chrome/Firefox/Safari. Deploy itch.io en zéro étapes manuelles.

---

## v2.0 — "Connected" (12 semaines)

**Philosophie :** Les jeux sont sociaux. Multiplayer rollback netcode + modding + analytics.

### Features

| Feature | Détails |
|---------|---------|
| **Rollback netcode** | Style GGPO. Exploite le mode déterministe v1.0. |
| **Lobby / Matchmaking** | Host/join via code, NAT traversal, matchmaking skill-based. |
| **Mode spectateur** | Stream d'état en lecture seule. |
| **Framework de modding** | Lua sandboxé, overrides d'assets, manifeste mod JSON. |
| **Analytics** | Heatmaps, enregistrement sessions, événements custom, backend self-hosted. |
| **Intégration Steam** | Achievements, leaderboards, cloud saves, Workshop. |

### Critère de sortie
Deux joueurs jouent un fighting game avec rollback à 200ms de latence. Un mod remplace tous les sprites sans toucher au code. Build Steam s'upload et se lance.

---

## v3.0 — "Ecosystem" (continu)

| Feature | Effort estimé |
|---------|--------------|
| Export iOS (Metal, touch, App Store) | 8-10 semaines |
| Export Android (OpenGL ES 3.0/Vulkan) | 8-10 semaines |
| Export console (Switch, PlayStation, Xbox) | Continu |
| Édition collaborative (CRDT) | 12-16 semaines |
| Marketplace / asset store | 12+ semaines |
| Agents de playtesting IA | 12+ semaines |
| SDK de plugins/extensions | 8-10 semaines |
| Génération procédurale (WFC, donjons) | 6-8 semaines |
| Système de dialogue/narration | 6-8 semaines |
| Audio avancé (positionnel, DSP, FMOD/Wwise) | 6-8 semaines |

---

## Ce qu'on reporte délibérément et pourquoi

| Feature | Reportée à | Raison |
|---------|-----------|--------|
| **3D** | Jamais | Moteur 2D-pure. Les moteurs qui essaient les deux ne font bien ni l'un ni l'autre. |
| **Export web** | v1.5 | L'éditeur complet (v0.5) et la stabilité production (v1.0) passent avant la distribution. |
| **Scripting visuel node-based** | Post-v1.0 | Event sheets ont une barrière d'entrée plus basse. |
| **DSL custom** | Post-v1.0 | Lua est prouvé. Un mauvais DSL est pire que pas de DSL. |
| **Mobile** | v3.0 | Desktop et web d'abord. |
| **Consoles** | v3.0+ | NDA, matériel dédié, initiative business. |
| **Multiplayer** | v2.0 | Nécessite mode déterministe (v1.0). |
| **Marketplace** | v3.0 | Communauté d'abord, marketplace ensuite. |

---

## Jalons communautaires

| Jalon | Timing cible |
|-------|-------------|
| Lancement open-source | v0.1 |
| Discord / forum | v0.1 |
| Première game jam | v0.2 |
| Série de tutoriels (texte) | v0.2-v0.3 |
| Série de tutoriels (vidéo) | v0.3-v1.0 |
| Premier contributeur externe | ~v0.2 |
| 10 jeux communautaires | v0.3-v0.4 |
| Talk conférence / devlog | v0.4+ |
| 100 GitHub stars | v0.2-v0.3 |
| 1 000 GitHub stars | v0.5-v1.0 |
| Premier jeu commercial | v1.0+ |
| Écosystème de plugins | v2.0+ |
| Adoption éducative | v1.0+ |

---

*Ce document est vivant. Les cibles et timelines seront révisées selon la vélocité réelle, le feedback communautaire, et l'évolution du paysage concurrentiel.*
