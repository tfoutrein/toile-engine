# ADR-038 : State machine état→animation pilotée par les behaviors + UX « Animation States »

- **Statut :** Proposee — Phases 0-4 implementees (Phase 5 optionnelle restante)
- **Date :** 2026-06-15
- **Concerne :** v0.5+ (editeur + runner)

## Contexte

Toile sait deja **afficher et faire avancer** des animations : `AnimationData`
(frames, fps, looping, sprite_file/strip) est complet, le runner avance les frames
et flippe l'entite. Le rendu n'est pas le probleme. Une analyse multi-agents du
pipeline a identifie trois ruptures qui rendent l'association « sprite anime ↔
entite » penible :

| # | Probleme | Ou |
|---|----------|----|
| **R5 (central)** | Le mapping **etat→animation est code en dur** : reserve au tag `player`, **3 etats** (idle/walk/jump), noms **exacts et sensibles a la casse**, seuil de vitesse fige | `game_runner.rs:741-763` |
| **R6** | **Aucun pont behavior→etat** generique : `on_ground`/`velocity` existent mais ne pilotent rien hors du bloc player ; les **non-joueurs ne s'animent jamais** automatiquement | runner |
| **R4** | **Aucun apercu anime** dans l'editeur (il faut lancer le jeu) ; le champ `sprite_editor_preview_anim` etait **du code mort** | `editor_app.rs:107` |
| **R1-R3** | UX d'assignation **eclatee en 3 chemins divergents** (seul l'import drag/IA remplit `animations`), config de grille inversee, frame picking **un clic par frame** | inspector / overlay / sprite_editor |

Consequence concrete : pour qu'une animation joue aujourd'hui, l'entite doit etre
taguee `player` **et** ses anims s'appeler **exactement** `idle`/`walk`/`jump`.
Sinon, rien. Et on ne voit l'animation qu'en lançant le jeu.

## Decision

**Faire de l'ÉTAT un concept de premier ordre** : on associe des sprites a un
etat (Idle/Walk/Run/Jump/Fall), pas a un nom magique. Le binding est declaratif,
additif et retrocompatible ; une state machine runtime generique remplace le bloc
code en dur ; l'editeur expose des « slots » d'etats avec apercu anime.

### Decisions de design retenues

- **Auto-animation « magique »** : des qu'une entite possede un behavior de
  mouvement (Platform/TopDown) **et** des animations correspondantes, l'auto-anim
  s'active — sans configuration. (Pas de tag `player` requis, pas de flag a cocher
  pour le cas courant.)
- **États = enum ferme + Custom** : `AnimState = Idle | Walk | Run | Jump | Fall |
  Custom(String)`. Les 5 etats courants donnent des slots clairs dans l'editeur ;
  `Custom` couvre attack/hurt/dash sans recompiler.

### Modele de donnees (additif, retrocompatible)

Nouveau champ sur `EntityData` (`toile-scene`), sur le modele de `locked`/`visible`
(ADR-037) :

```rust
#[serde(default)]
pub animation_states: Option<AnimationStateMap>

struct AnimationStateMap {
    bindings: Vec<StateBinding>,   // StateBinding { state: AnimState, anim: String }
    facing: FacingMode,            // None | VelocityX | InputX
    auto: bool,                    // l'auto-anim vit ici (pas de champ primitif separe)
    move_threshold: f32,           // defaut 5.0 (reproduit l'actuel)
}
```

`animation_states` **absent ⇒ comportement legacy strictement inchange**. Les
bindings **pointent** vers des noms d'`AnimationData` existants (pas de duplication
des frames). Aucune migration : les scenes v0.5 chargent et tournent a l'identique.

### Runtime : state machine generique

- **Pont** : une struct `MotionSnapshot { on_ground, was_on_ground, vx, vy, facing }`
  remplie par le runner **apres** la phase behaviors, a partir de `EntityState`.
  Definie dans `toile-behaviors`/`toile-scene` (pas `toile-runner`) pour eviter une
  dependance circulaire et permettre sa consommation par les events (Phase 5).
- **Selection** : une fonction **pure** `select_state(snapshot, behaviors) -> AnimState`
  remplace `game_runner.rs:741-763`.
  - Platform : `!on_ground && vy >= 0` → **Jump** (montee) ; `!on_ground && vy < 0`
    → **Fall** (chute, fallback Jump) ; `on_ground && |vx| > seuil_haut` → **Run**
    (fallback Walk) ; `on_ground && |vx| > move_threshold` → **Walk** ; sinon **Idle**.
    *(Convention monde +y vers le haut : `velocity.y > 0` = montee — verifie sur
    `platform.rs`, `jump_force` positif.)*
  - TopDown : `(vx,vy) != 0` → **Walk** ; sinon **Idle**.
- **Resolution état→anim** : binding explicite d'abord, sinon **fallback sur la
  convention de noms insensible a la casse + table de synonymes**
  (`idle|repos|stand`, `walk|marche`, `run|course|sprint`, `jump|saut`, `fall|chute`).
  Si l'anim cible n'existe pas, on conserve l'anim courante (pas de crash).
- **Flip** : `FacingMode::VelocityX` generalise le calcul player-only ; le flip UV
  existe deja (`game_runner.rs:938-940`).
- **PlaybackMode** : `RuntimeEntity.anim_finished` ; un etat `Once` (Jump
  non-looping) ne re-switche pas tant que l'anim n'est pas finie. Prepare
  `OnAnimationFinished`.
- **Priorite** : `ActionKind::PlayAnimation` garde la priorite (un flag
  `current_anim_locked` suspend la selection auto jusqu'a fin d'anim ou nouvel ordre).

### Éditeur : UX « Animation States »

- Bouton **« Make Player »** (1 clic : tag player + behavior Platform), avec un
  libelle reliant le role a l'activation idle/walk/jump.
- Panneau **« Animation States »** : ligne de **slots** d'etats deduits du behavior
  (Platform → Idle/Walk/Run/Jump/Fall ; TopDown → Idle/Walk), assignation par
  drag-drop / Frame Picker **multi-selection** (refondu), **apercu anime live** sur
  la slot selectionnee.
- Pre-remplissage auto des bindings depuis les tags Aseprite / noms de fichiers
  (table de synonymes).

## Plan d'implementation

| Phase | Contenu | Effort |
|---|---|---|
| **0 — Quick wins** (rétrocompat pur) | Apercu anime live (reveil de `sprite_editor_preview_anim` via une frame **transitoire**, sans muter la scene) ; selection runtime **generalisee** a toute entite Platform/TopDown ; noms insensibles a la casse + synonymes ; **Fall** derive de `velocity.y` ; **Run** ; flip generalise ; bouton **Make Player** | S-M |
| **1 — Modele de donnees** | `AnimationStateMap`/`StateBinding`/`AnimState`/`FacingMode` + champ `animation_states` (#[serde(default)]) ; `MotionSnapshot` ; tests round-trip retrocompat ; preservation MCP/AI | M |
| **2 — State machine runtime** | `select_state` pur + `MotionSnapshot` + resolution bindings + `anim_finished`/Once + override `PlayAnimation` ; tests de non-regression legacy | M |
| **3 — Panneau « Animation States »** | slots par etat, dropdown anim, vignettes, grand apercu live, bandeau debug F3 « State → anim » | L |
| **4 — Import & frame picking** | **corriger l'incoherence des 3 chemins** (Browse / Set sprite / import remplissent `animations`) ; drag-drop asset→slot ; Frame Picker multi/range/reorder ; auto-detection strip vs grille | M-L |
| **5 (option) — Pont events** | `EventContext` + `on_ground/vx/vy` ; `OnGrounded`, `OnAnimationFinished` | M |

## Consequences

### Positives
- « Ça joue tout seul » : idle/walk/jump/fall automatiques des qu'il y a un behavior
  de mouvement + des anims, pour joueurs **et** ennemis/NPC.
- Noms d'anim libres (binding) et multilingues (synonymes) ; plus de piege casse.
- Apercu anime sans lancer le jeu.
- Additif : zero scene cassee.

### Negatives / honnetete sur l'effort
- La state machine, les types de donnees et l'UI « slots » sont du **NOUVEAU code**
  (pas de la reutilisation) : Phase 1 = M, Phase 3 = L, Frame Picker = refonte.
- Le champ `animation_states` doit etre preserve par MCP et l'AI copilot (round-trip).
- L'apercu anime par slot est O(N) : compromis « live sur la slot selectionnee
  seulement, thumbnails figes ailleurs ».

## Alternatives rejetees

1. **Tout via event sheets** (`PlayAnimation` + conditions) → rejete : regles a ecrire
   a la main par entite, contraire a l'objectif UX. Reste dispo pour les etats
   scriptes (attack/hurt).
2. **Convention de noms stricte** (anim DOIT s'appeler idle/walk/jump, sensible a la
   casse) → rejete : fragile (casse/langue), pas de noms perso, ne couvre ni les
   non-players ni Fall. Conservee seulement en **fallback** zero-config (desormais
   insensible a la casse + synonymes).
3. **AnimGraph generique** (transitions/conditions arbitraires, style Unity Animator)
   → rejete pour v0.5 (surdimensionne) ; reportable en v1.x.
4. **AnimatorComponent dans l'ECS** comme source de verite → rejete : le runtime
   data-driven n'utilise pas l'ECS pour les anims ; la scene JSON reste la source de
   verite.
5. **`auto_animate: bool` separe** en plus de `animation_states` → rejete : deux
   concepts pour la meme chose ; l'auto vit dans `AnimationStateMap.auto`.

## Questions ouvertes

- Unifier les deux modeles de sourcing (`sprite_sheet` grille vs `anim.sprite_file`
  strip) dans une ADR separee ? Impacte le calcul UV et les slots.
- Directions 4/8-way pour TopDown des le MVP ou plus tard ?
- Transitions (blend/duree) : v0.5 ou v1.x ? (MVP = switch instantane + garde Once.)
- `AnimationLibrary` reutilisable (copier un set idle/walk/jump entre entites) ?

## Phase 4 — decisions & report (implementee)

- **Cohérence des chemins (tranchée)** : « Add to Scene » et « Set as sprite of
  selection » importent desormais sprite_sheet + animations depuis les metadonnees
  de l'asset, puis pre-remplissent les bindings via `auto_bind_animation_states`.
  « Set sprite » fait un **reset propre** (sheet/anims/default/bindings) avant de
  repeupler, donc plus d'animations periemees desalignees. Le bouton **« Browse »**
  de l'inspecteur reste volontairement un **sprite simple** : c'est un selecteur de
  fichier brut (sans metadonnees), il pose le sprite et **reinitialise** les anims
  (pas de slot perime) ; pour des animations on passe par le navigateur d'assets ou
  « Setup Sprite & Animations ». Pas d'incoherence silencieuse.
- **Auto-bind** : a l'import, les bindings d'etats sont deduits des noms d'anims
  (table de synonymes, insensible a la casse) → les slots refletent ce qui jouera.
- **Frame Picker** : multi-selection par **Shift+clic** (ajoute la plage depuis la
  derniere frame ; l'ancre est reinitialisee a l'ouverture du picker).
- **Reporte** (acceptable, trace) : drag-drop asset→slot ; reorder des frames par
  glisser dans le picker ; auto-detection systematique strip vs grille au Browse.

## Validation

- Logique pure (`select_state`, resolution de noms) : tests unitaires reproduisant
  le comportement legacy (non-regression).
- Round-trip de scene avec/sans `animation_states`.
- Apercu + slots + auto-anim : **macpilot** (viewport invisible a `egui_kittest`).
