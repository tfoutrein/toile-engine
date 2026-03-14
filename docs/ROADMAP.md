# Toile Engine — Roadmap Complète

**Document vivant** | Dernière mise à jour : 2026-03-14

---

## Table des matières

1. [Vision & Thèmes directeurs](#vision--thèmes-directeurs)
2. [Timeline visuelle](#timeline-visuelle)
3. [v0.1 — "First Light" (MVP)](#v01--first-light-mvp-12-semaines)
4. [v0.2 — "Game Systems"](#v02--game-systems-8-semaines)
5. [v0.3 — "Creator Tools"](#v03--creator-tools-8-semaines)
6. [v0.4 — "Visual Polish"](#v04--visual-polish-8-semaines)
7. [v0.5 — "Web & Share"](#v05--web--share-6-semaines)
8. [v1.0 — "Production Ready"](#v10--production-ready-12-semaines)
9. [v1.5 — "Connected"](#v15--connected-12-semaines)
10. [v2.0 — "Ecosystem"](#v20--ecosystem-continu)
11. [Ce qu'on reporte délibérément et pourquoi](#ce-quon-reporte-délibérément-et-pourquoi)
12. [Jalons communautaires](#jalons-communautaires)

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
 |           |         |         |"Visual  |=v0.5=|            |            |
 |           |         |         | Polish" |6 sem |            |            |
 |           |         |         |         |"Web" |===v1.0=====|            |
 |           |         |         |         |      | 12 sem     |            |
 |           |         |         |         |      |"Production |====v1.5====|
 |           |         |         |         |      | Ready"     | 12 sem     |
 |           |         |         |         |      |            |"Connected" |=>v2.0=>
```

| Version | Codename | Durée | Semaines cumulées |
|---------|----------|-------|-------------------|
| v0.1 | First Light | 12 semaines | 12 |
| v0.2 | Game Systems | 8 semaines | 20 |
| v0.3 | Creator Tools | 8 semaines | 28 |
| v0.4 | Visual Polish | 8 semaines | 36 |
| v0.5 | Web & Share | 6 semaines | 42 |
| v1.0 | Production Ready | 12 semaines | 54 |
| v1.5 | Connected | 12 semaines | 66 |
| v2.0 | Ecosystem | Continu | — |

**Total jusqu'à la release stable v1.0 : ~54 semaines (~12.5 mois)**

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

**Philosophie :** Assez d'infrastructure pour construire un vrai jeu indie. Personnages animés, mondes tile-based, effets de particules, et scripting intégré transforment le moteur d'un jouet de rendu en outil de création de jeux.

### Nouvelles features

| Catégorie | Features |
|-----------|----------|
| **Animation** | Système d'animation complet : séquences de frames, tags Aseprite, state machine pour transitions. Tweening (linear, ease-in/out, bezier). |
| **Tilemap** | Tiled JSON complet : tile layers (CSV + base64+zlib), object layers, image layers. GID flip-bits. Collision par tile. Rendu efficace par batches pré-construits. |
| **Particules** | Simulation CPU. Émetteurs : point, cercle, rectangle, ligne. Lifetime, vélocité, gravité, taille/couleur over life (courbe/gradient). 10 presets intégrés (feu, fumée, étincelles, pluie, neige, explosion...). |
| **Physique simple** | Rigid bodies (dynamic, static, kinematic). Gravité. Intégration de vélocité. Réponse de collision basique (bounce, slide). |
| **Scènes** | Pile de scènes (push/pop). Transitions (fade, slide, wipe). Chargement async. Arena allocator par scène. |
| **Assets** | Chargement async en background thread. Handles avec futures. Support écran de chargement. |

### Éditeur

- Peinture de tilemap : palette, brush/fill/eraser, auto-tiling bitmask, multi-layers
- Preview d'animation dans l'inspecteur, scrub de frames
- Gestion des layers : nommés, visibilité/lock, réordonnement, opacité
- Preview de particules en temps réel dans le viewport
- Hiérarchie d'objets avec recherche/filtre

### IA / MCP

- CRUD entités complet : `create_entity`, `delete_entity`, `get_entity`, `list_entities`, `update_entity`
- Manipulation de composants : `add_component`, `remove_component`, `set_component_property`
- Contrôle d'exécution : `play_scene`, `stop_scene`, `pause_scene`, `step_frame`
- `get_game_state` retourne l'état sérialisé complet. Mode headless complet.

### Nouveaux formats

- **QOI** (format runtime interne pour chargement rapide)
- **Aseprite JSON** (frame tags, durée par frame) complet

### Critère de sortie
Un platformer avec personnages animés, niveaux tilés, effets de particules. Tilemap 200×200 à 60 FPS. Hot-reload Lua < 500ms. Un agent IA crée une scène complète via MCP sans intervention humaine. 5 000 particules à 60 FPS.

---

## v0.3 — "Creator Tools" (8 semaines)

**Philosophie :** Combler le fossé pour les non-programmeurs. Le système d'event sheets (inspiré de Construct) et les behaviors pré-construits permettent à un designer ou artiste de construire un jeu jouable sans écrire une seule ligne de code.

### Event Sheets

| Feature | Détails |
|---------|---------|
| **Modèle** | Paires condition-action dans un UI tableur. Quand les conditions (gauche) sont remplies, les actions (droite) s'exécutent. |
| **Conditions** | `On created`, `Every tick`, `On key pressed/released/held`, `On mouse click`, `On collision with [tag]`, `If variable [op] value`, `Every N seconds`, `On animation finished`, `Compare distance`... |
| **Actions** | `Set position`, `Move at angle`, `Move toward`, `Set variable`, `Destroy`, `Spawn object`, `Play sound`, `Play animation`, `Set visibility`, `Apply force`, `Go to scene`, `Wait N seconds`... |
| **Expressions** | Parser inline : `player.x + 10`, `random(1, 100)`, `distance(self, player)`. Type-checked à l'édition. |
| **Organisation** | Groupes (sections collapsibles), sous-événements (conditions imbriquées), fonctions (blocs nommés appelables). |

### Behaviors (Comportements pré-construits)

| Behavior | Paramètres |
|----------|-----------|
| **Platform** | gravity, jump_force, max_speed, acceleration, deceleration, coyote_time, input_buffering, max_jumps |
| **TopDown** | max_speed, acceleration, deceleration, diagonal_correction |
| **Bullet** | speed, acceleration, gravity, bounce_off_solids |
| **Physics** | body_type, mass, friction, bounciness, linear_damping, angular_damping |
| **Solid** | (aucun) — marque une entité comme impassable |
| **Sine** | property (x/y/angle/opacity/size), magnitude, period, wave_type |
| **Fade** | fade_in_time, fade_out_time, destroy_on_fade_out |
| **Wrap** | (aucun) — wrap aux bords de l'écran (style Asteroids) |

### Éditeur

- Éditeur d'event sheets : picker condition/action par dropdown cherchable, drag to reorder, blocs color-codés, validation temps réel
- Inspecteur de behaviors : bouton "Add Behavior", widgets par paramètre, tooltips documentation
- Système de prefabs : sauvegarder une entité configurée comme template réutilisable, instances héritent les changements
- Templates de projet : "Vide", "Platformer", "Top-Down", "Shoot-em-up" avec assets de démarrage

### Nouveaux formats

- **LDtk** (.ldtk JSON) — world layout, IntGrid, auto-layers, entités typées
- **Aseprite direct** (.ase/.aseprite binaire) — parsing sans export CLI

### IA / MCP

- CRUD event sheets : `create_event_sheet`, `add_event`, `remove_event`
- Gestion behaviors : `add_behavior`, `remove_behavior`, `set_behavior_property`
- Gestion prefabs : `create_prefab`, `instantiate_prefab`
- `create_project_from_template` — l'IA scaffolde un projet complet
- `llms.txt` v2 avec syntaxe event sheets et paramètres behaviors

### Critère de sortie
Un non-programmeur construit un platformer jouable (mouvement, ennemis, collectibles, condition win/lose) en event sheets et behaviors en < 30 minutes. Le behavior Platform "feels good" out-of-the-box. L'IA scaffolde un projet complet via MCP.

---

## v0.4 — "Visual Polish" (8 semaines)

**Philosophie :** Élever le plafond de qualité visuelle. Éclairage 2D avec normal maps (la feature qui a rendu l'art de Hollow Knight sublime mais nécessitait des workarounds douloureux), éditeur de shaders visuel, polices SDF, et pipeline de post-processing.

### Nouvelles features

| Feature | Détails |
|---------|---------|
| **Éclairage 2D** | Point lights, directional, spot. Normal maps par sprite (convention `hero.png` + `hero_n.png`, auto-détecté). Shadow casting. Lumière ambiante. Light culling par grille spatiale. Cible : **50+ lumières dynamiques à 60 FPS**. |
| **Ombres** | Shadow maps 1D projetées depuis les sources de lumière. Soft shadows avec falloff. Pénombre optionnelle. |
| **Éditeur de shaders visuel** | Graphe de nœuds pour shaders 2D. Nœuds : texture sample, color, manipulation UV, math ops, time, bruit (perlin, simplex, voronoi), formes SDF, screen UV, distortion. Compile en WGSL. Preview live. |
| **Polices SDF** | MSDF (multi-channel signed distance field). Atlas généré à l'import via `msdf-atlas-gen`. Shader MSDF : texte net à n'importe quel zoom. Effets outline, shadow, glow comme paramètres shader. |
| **Post-processing** | Rendu offscreen. Chaîne configurable : **CRT** (scanlines, courbure, aberration chromatique), **Bloom** (threshold + gaussian blur + composite), **Color grading** (texture LUT), **Vignette**, **Screen shake** (basé trauma). Toggleable par scène. |
| **Éditeur de particules** | Panneau dédié. Widget éditeur de courbe (handles bezier, points de contrôle draggables). Widget éditeur de gradient. Gizmo de forme d'émetteur. Sub-émetteurs. |

### IA / MCP

- `create_light`, `set_light_property`, `list_lights`
- `create_shader_from_description` — l'IA décrit l'effet visuel, le moteur suggère un graphe ou génère du WGSL
- `set_post_processing_stack` — liste ordonnée d'effets avec paramètres
- `take_screenshot` supporte `with_lighting=true/false`, `with_post_processing=true/false` pour comparaison A/B

### Nouveaux formats

- **TTF/OTF** avec génération d'atlas SDF
- Format `.shader` custom (graphe sérialisé en JSON)
- Textures LUT (PNG, strip standard 256×16)

### Critère de sortie
50 lumières dynamiques avec ombres à 60 FPS. Sprites avec normal maps réagissent correctement aux lumières. Texte SDF net de 8px à 200px. CRT + Bloom < 0.5ms overhead à 1080p. Un non-programmeur crée un shader custom avec l'éditeur visuel.

---

## v0.5 — "Web & Share" (6 semaines)

**Philosophie :** Le chemin le plus rapide de "j'ai fait un jeu" à "joue à mon jeu" est une URL. L'export web supprime la barrière du téléchargement et débloque itch.io, Newgrounds, et le partage social. Cible : **< 3 MB** pour un jeu simple.

### Nouvelles features

| Feature | Détails |
|---------|---------|
| **Export WASM/WebGL2** | Compilation via `wasm-pack`/`wasm-bindgen`. wgpu cible WebGPU nativement ; fallback WebGL2 via backend OpenGL de wgpu. |
| **Bundling d'assets web** | Pack tous les assets dans un `.bin` unique avec manifeste. Fetch via HTTP. Chargement streaming avec barre de progression. |
| **Audio web** | Backend WebAudio API. Auto-unlock du contexte audio au premier input utilisateur. |
| **Optimisation taille** | Tree-shaking, LTO, `wasm-opt`, compression zstd, WebP pour textures. Cible : **< 3 MB** (hello world platformer), **< 8 MB** (jeu riche). |
| **Deploy itch.io** | `toile deploy itch <user/game>` via butler CLI. Upload automatique. |
| **Preview navigateur** | `toile serve` — serveur HTTP local, ouvre le jeu dans le navigateur avec hot-reload WebSocket. |
| **Input tactile** | Mapping touch → pointer events. Gamepad virtuel on-screen optionnel pour navigateurs mobiles. |

### IA / MCP

- `build_project { platform: "web" }` retourne le chemin et la taille du build
- `deploy_itch { user: "...", game: "..." }` retourne l'URL
- `start_web_preview` retourne une URL localhost utilisable par l'IA avec un outil navigateur

### Nouvelles plateformes

| Plateforme | Statut |
|-----------|--------|
| **Web (WASM/WebGL2)** | **Nouveau** |
| **Web (WASM/WebGPU)** | **Expérimental** (Chrome 121+) |

### Critère de sortie
Platformer (5 niveaux, 20 sprites, 10 sons) exporte en web < 3 MB. Chargement < 2s sur connexion 50 Mbps. 60 FPS sur Chrome/Firefox/Safari. Deploy itch.io en zéro étapes manuelles. Le jeu joue identiquement desktop et web.

---

## v1.0 — "Production Ready" (12 semaines)

**Philosophie :** Tout ce dont une équipe a besoin pour livrer un jeu commercial. Pipeline d'assets, accessibilité, localisation, simulation déterministe, profiling, et documentation complète avec jeux exemples.

**Cette version inclut un gel de l'API publique.** Après v1.0, l'API suit le semantic versioning.

### Nouvelles features

| Feature | Détails |
|---------|---------|
| **Pipeline d'assets** | Import → Process (premultiply alpha, atlas packing MaxRects, atlas SDF, compression) → Pack (bundle binaire .pak, zstd, avec manifeste). Runtime : asset manager charge depuis .pak ; chemins de fichiers marchent encore en dev. |
| **Framework d'accessibilité** | Screen reader (MSAA Windows, NSAccessibility macOS, AT-SPI Linux). Modes daltonien (deutéranopie, protanopie, tritanopie — simulation + correction). Remapping d'input global. Scaling de texte. Réduction de mouvement (désactive shake, particules, animations rapides). |
| **Localisation** | Tables de strings (CSV/JSON). Lookup par clé : `t("ui.play_button")`. Pluralisation par langue. Support RTL (arabe, hébreu). Fallbacks de polices par langue (CJK, arabe, thaï). Pseudolocalisation (mode debug). |
| **Mode déterministe** | Opt-in. Maths fixed-point pour la physique. RNG seedé. Enregistrement d'inputs frame-locked. Fichier replay : `{ seed, inputs_per_frame[] }`. Playback : feed les inputs enregistrés, vérifie les checksums d'état par frame. |
| **Profiling** | Breakdown frame time (bar chart subsystèmes). Visualisation de batches (overlay color-codé). Heatmap d'overdraw. Inspecteur mémoire. Inspecteur d'entités live. |
| **Documentation** | Référence API auto-générée. Guides conceptuels (architecture, getting started, scripting, behaviors, shaders, accessibilité, localisation). Tutoriels vidéo. |
| **Jeux exemples** | **Platformer** (3 niveaux, controller, ennemis, UI). **Top-down shooter** (arène procédurale, vagues). **Puzzle** (grille, undo/redo, 20 niveaux). **Visual novel** (dialogues, branching, portraits). Chaque jeu est complet, poli, open-source, avec code commenté. |

### IA / MCP

- Replay : `start_recording`, `stop_recording`, `play_replay`, `get_replay_checksum`
- Profiling : `get_frame_profile` retourne des données structurées
- Localisation : `set_language`, `get_string`, `list_languages`, `add_translation`
- Accessibilité : `set_accessible_name`, `enable_colorblind_mode`
- Test runner headless : `toile test --headless --replay replay.bin --checksum expected.sha256`
- `llms.txt` v3 (API complète + behaviors + event sheets + shader nodes)

### Critère de sortie
Les 4 jeux exemples compilent et tournent sur Windows/macOS/Linux/Web. Pipeline d'assets réduit un projet 100 MB dev en < 15 MB distribution. Screen reader lit les éléments UI (NVDA/VoiceOver). Un replay joue identiquement cross-plateforme. Documentation couvre 100% de l'API publique. Zéro crash bug connu.

---

## v1.5 — "Connected" (12 semaines)

**Philosophie :** Les jeux sont sociaux. Le multiplayer avec rollback netcode adresse la feature la plus demandée et absente de tous les moteurs 2D. Le modding étend la vie des jeux. Les analytics aident les développeurs à comprendre leurs joueurs.

### Nouvelles features

| Feature | Détails |
|---------|---------|
| **Rollback netcode** | Style GGPO. Exploite le mode déterministe v1.0. Save/restore de snapshots d'état. Input delay configurable (1-6 frames). Budget rollback : resimulation de 15 frames en < 1.1ms à 60 FPS. Prédiction et correction d'input. Smoothing visuel. |
| **Système de lobby** | Host/join via code de lobby. Découverte de pairs via relay server. NAT traversal (STUN/TURN). |
| **Matchmaking** | Matchmaking basé sur le skill. Préférences régionales. Système de file d'attente. Composant serveur léger déployable. |
| **Mode spectateur** | Stream d'état de jeu en lecture seule. Délayé pour prévenir la triche. |
| **Framework de modding** | Lua sandboxé (pas d'accès filesystem/réseau/OS). Overrides d'assets par chemin. Manifeste de mod (JSON). Manager de mods (UI enable/disable, load order, détection de conflits). |
| **Analytics** | Heatmaps (position joueur par frame, agrégée). Enregistrement de sessions (réutilise le système replay). Événements custom. Backend self-hosted (REST API + SQLite). |
| **Intégration Steam** | Steamworks SDK : achievements, leaderboards, cloud saves, overlay, rich presence. Workshop pour mods. `toile deploy steam`. |

### Critère de sortie
Deux joueurs jouent un fighting game avec rollback à 200ms de latence simulée. Resimulation 15 frames < 1.1ms. Un mod remplace tous les sprites joueur et ajoute un ennemi sans toucher au code moteur. Lua sandboxé ne peut pas accéder au filesystem. Heatmap de 1000 sessions se rend correctement. Build Steam s'upload et se lance.

---

## v2.0 — "Ecosystem" (continu)

**Philosophie :** Scaler au-delà d'un développeur sur une plateforme. Mobile, consoles, édition collaborative, marketplace, agents de playtesting IA, et SDK de plugins transforment le moteur d'un outil en plateforme.

### Features planifiées

| Feature | Effort estimé |
|---------|--------------|
| **Export iOS** (Metal, touch, App Store packaging) | 8-10 semaines |
| **Export Android** (OpenGL ES 3.0/Vulkan, NDK, APK/AAB) | 8-10 semaines |
| **Export console** (Switch, PlayStation, Xbox — partenariats NDA) | Continu, dépend des partenaires |
| **Édition collaborative** (CRDT, multi-utilisateurs temps réel) | 12-16 semaines |
| **Marketplace / asset store** (assets communautaires, partage de revenus) | 12+ semaines |
| **Agents de playtesting IA** (exploration autonome, rapport de bugs, heatmaps) | 12+ semaines |
| **SDK de plugins/extensions** (API stable, hooks lifecycle, points de contribution UI) | 8-10 semaines |
| **Toolkit de génération procédurale** (bruit, WFC, graph-based, donjons) | 6-8 semaines |
| **Système de dialogue/narration** (branching, conditions, localisation, éditeur visuel) | 6-8 semaines |
| **Audio avancé** (positionnel 2D, bus DSP, musique adaptative, intégration FMOD/Wwise) | 6-8 semaines |

---

## Ce qu'on reporte délibérément et pourquoi

| Feature | Reportée à | Raison |
|---------|-----------|--------|
| **Physique complète** (joints, contraintes) | v0.2-v0.3 | La physique cassée de Godot est le reproche #1. On livre d'abord une collision simple et correcte. La physique complète vient quand on peut la faire bien (intégration Rapier). |
| **3D de quelque forme que ce soit** | Jamais | C'est un moteur 2D. Pas 2.5D, pas "2D avec éléments 3D". Les moteurs qui essaient les deux ne font bien ni l'un ni l'autre. |
| **Scripting visuel node-based** | Post-v1.0 (si demande) | Les event sheets (style Construct) ont une barrière d'entrée plus basse que les graphes de nœuds. Le node-based crée du spaghetti plus vite. |
| **DSL custom (style GDScript)** | Post-v1.0 (si jamais) | Designer un bon DSL est extrêmement difficile. Lua est prouvé. Un mauvais DSL est pire que pas de DSL. |
| **Mobile** | v2.0 | Complexité plateforme disproportionnée. Desktop et web doivent être rock-solid d'abord. |
| **Consoles** | v2.0+ | Nécessite des partenariats NDA, du matériel dédié, et des couches plateforme closed-source. Initiative business, pas juste engineering. |
| **Multiplayer** | v1.5 | Le rollback netcode nécessite la simulation déterministe. Les contraintes architecturales sont conçues à v1.0 (mode déterministe), mais le networking peut attendre. |
| **Marketplace** | v2.0 | Un marketplace prématuré sans communauté est une vitrine vide. Construire la communauté d'abord. |
| **Spine 2D** | Post-v0.3 | Spine est premium ($70-$350). L'animation frame-by-frame et l'import Aseprite couvrent 80%+ des besoins indie. |
| **SVG runtime** | Probablement jamais | SVG est énormément complexe. Rasteriser à l'import via NanoSVG. Le rendu SVG runtime est un gouffre. |
| **Édition collaborative** | v2.0 | L'édition CRDT est un investissement infra majeur. Nécessite un éditeur mature et stable comme fondation. |

---

## Jalons communautaires

| Jalon | Timing cible | Description |
|-------|-------------|-------------|
| **Lancement open-source** | Release v0.1 | Repo public sous licence MIT. Guide de contribution, code de conduite. |
| **Discord / forum** | Release v0.1 | Hub communautaire central. |
| **Première game jam** | Release v0.2 | Hoster ou sponsoriser une game jam utilisant le moteur. Dogfood les outils. |
| **Série de tutoriels (texte)** | v0.2-v0.3 | "Getting Started" jusqu'à "Publishing Your Game". |
| **Série de tutoriels (vidéo)** | v0.3-v1.0 | YouTube, visant les débutants. |
| **Premier contributeur externe** | ~v0.2 | Quelqu'un hors de l'équipe core soumet un PR mergé. |
| **10 jeux communautaires** | v0.3-v0.4 | Jeux faits par des gens hors de l'équipe core. Preuve d'utilisabilité. |
| **Talk conférence / devlog** | v0.4+ | Blog/vidéo documentant le dev. Soumission conférence (GDC, Nordic Game, Handmade Seattle). |
| **100 GitHub stars** | v0.2-v0.3 | Signal de traction précoce. |
| **1 000 GitHub stars** | v0.5-v1.0 | Intérêt communautaire significatif. |
| **Premier jeu commercial** | v1.0+ | Un jeu vendu sur Steam ou itch.io construit avec le moteur. Validation ultime. |
| **Écosystème d'asset packs** | v1.0+ | La communauté crée et partage des starter packs, tilesets, sons. |
| **Écosystème de plugins** | v2.0+ | Extensions tierces : nouveaux behaviors, outils éditeur, importeurs. |
| **Adoption éducative** | v1.0+ | Une école ou cours en ligne utilise le moteur pour l'enseignement. |

---

## Annexes

### Stack technique

| Couche | Technologie | Verrouillée à |
|--------|-----------|--------------|
| Langage | Rust | v0.1 |
| Build | Cargo | v0.1 |
| Windowing/Input | SDL3 (rust bindings) | v0.1 |
| Rendu | wgpu | v0.1 |
| Shaders | WGSL | v0.1 |
| ECS | hecs | v0.1 |
| Math | glam | v0.1 |
| Audio | kira | v0.1 |
| Physique | Custom simple (v0.1-v0.2), Rapier (v0.3+) | v0.2 |
| Scripting | Lua 5.4 / LuaJIT via mlua | v0.2 |
| UI éditeur | egui | v0.1 |
| Format de scène | JSON + JSON Schema | v0.1 |
| Assets (dev) | Fichiers source (PNG, ASE, JSON, WAV, OGG, TTF) | v0.1 |
| Assets (dist) | .pak binaire (zstd) | v1.0 |
| Web | wasm-pack + wasm-bindgen | v0.5 |
| Networking | UDP custom + rollback style GGPO | v1.5 |
| IA | Serveur MCP (transport stdio) | v0.1 |
| Documentation | llms.txt + mdBook + API docs auto-générées | v0.1 |

### Cibles de performance

| Métrique | Cible v0.1 | Cible v1.0 |
|----------|-----------|-----------|
| Max sprites à 60 FPS (GPU intégré) | 10 000 | 50 000 |
| Max sprites à 60 FPS (GPU dédié) | 50 000 | 200 000 |
| Draw calls (10k sprites) | < 20 | < 5 |
| Overhead mémoire moteur | < 20 MB | < 15 MB |
| Mémoire par entité | < 256 bytes | < 192 bytes |
| Fenêtre visible | < 500 ms | < 200 ms |
| Premier frame | < 1 s | < 500 ms |
| Petit jeu chargé | < 3 s | < 1.5 s |
| Export web (hello world) | N/A | < 3 MB |
| Budget CPU total (60 FPS) | < 10 ms | < 8 ms |
| Collision (10k entités) | < 3 ms | < 1.5 ms |
| Hot-reload Lua | N/A | < 500 ms |
| Resimulation rollback (15 frames) | N/A | < 1.1 ms |

---

*Ce document est vivant. Les cibles et timelines seront révisées selon la vélocité réelle, le feedback communautaire, et l'évolution du paysage concurrentiel. L'ordre des features reflète des priorités basées sur la recherche, pas un séquencement arbitraire — chaque version construit sur la fondation de la précédente, et rien n'est planifié avant que ses dépendances soient stables.*
