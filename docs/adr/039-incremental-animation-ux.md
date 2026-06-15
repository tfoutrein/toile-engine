# ADR-039 : Ajout incrémental d'animations à une entité (flux entité-d'abord, asset browser désambiguïsé, helper additif unique)

- **Statut :** Acceptée — Phases 0 & 1 livrées (dialogue modal Add Animation reporté en Phase 2)
- **Date :** 2026-06-15
- **Concerne :** v0.5+ (editeur)

## Contexte

ADR-038 a livré la state machine état→animation et le panneau « Animation
States ». Mais le **flux d'association d'animations à une entité** reste
contre-intuitif. Une analyse multi-agents (vérifiée sur le code) a montré que le
**modèle mental de l'éditeur est inversé** : il traite chaque animation comme un
effet de bord de l'import d'un asset, au lieu de partir de l'entité.

| # | Incohérence (vérifiée) | Ou |
|---|------------------------|----|
| 1 | « Add to Scene » **crée toujours une entité** | `browser_panel.rs`, drain `overlay.rs` |
| 2 | « Set as sprite of selection » fait un **clear inconditionnel** des animations | `overlay.rs` (`e.animations.clear()`) |
| 3 | **3 importers, 3 sémantiques** de collision de nom (Aseprite remplace, Strip ignore, IA garde) | `sprite_editor.rs`, `chat_panel.rs` |
| 4 | **Aucune action humaine « ajouter à la sélection »** — l'additif n'existe que côté IA | `chat_panel.rs:add_entity_animation` |
| 5 | Animations **invisibles dans la hiérarchie** (`has_children` les ignore) | `overlay.rs` |
| 6 | Conditions de déclenchement **codées en dur et invisibles** (`select_states` privée, ×24 et 5.0 magiques) | `game_runner.rs` |

Conséquence : importer `idle` puis `marche` crée **2 entités** au lieu d'une
entité à 2 animations ; donner un sprite à une entité **détruit** ses anims ; et
on ne voit nulle part l'arbre « entité → animations → états → conditions ».

## Decision

**Inverser le modèle en ENTITÉ-D'ABORD et ADDITIF, sans changer le format de
scène** (refonte purement UX/éditeur, 100% rétrocompatible).

### 1. Asset browser désambiguïsé

Quand une entité est sélectionnée, l'action par défaut sur un asset devient
**« Add as animation to <sélection> »** (additif). Les autres :
- **« Create new entity from asset »** (ex-« Add to Scene », logique inchangée).
- **« Replace sprite of <sélection>… »** (ex-« Set as sprite ») — le clear
  inconditionnel est remplacé par un dialogue **Keep animations / Replace all /
  Abort** (Phase 3), partagé avec le bouton « Browse » de l'inspecteur.

### 2. Helper additif unique

Un seul `add_animation_to_entity(entity, spec, sourcing, on_conflict)` partagé par
l'UI humaine **et** l'outil IA `add_entity_animation`. Il **PUSH** une
`AnimationData` (ne `clear` jamais), applique une **règle de collision unique**
`on_conflict { KeepBoth (défaut, suffixe `_2`) | Replace | Abort }` — unifiant les
3 importers divergents — puis appelle `auto_populate_missing_bindings`
(ex-`auto_bind_animation_states`, **contrat durci : ne surcharge jamais un binding
existant**, testé). Il gère les **deux modèles de sourcing** (grille partagée vs
strip autonome) avec un avertissement non-bloquant en cas de mismatch (jamais
d'auto-magic : l'utilisateur choisit).

### 3. Lisibilité — l'entité devient un arbre

- **Hiérarchie** (affichée d'office) : `has_children` inclut les animations ;
  sous l'entité, un groupe **« Animations (n) »** (nom + frames + fps + loop + tag
  `[grid]`/`[strip]`) et un groupe **« Animation States »** (Idle→idle… avec
  **badge condition lisible** et `!` sur binding cassé).
- **Inspecteur** : section « Animation States » au rang de Behaviors/Collision
  (Phase 2) + bouton **« + Add Animation »**.

### 4. Conditions rendues lisibles

Chaque état canonique affiche sa **condition de déclenchement** en clair (ex.
« Run : au sol, |vx| > 120 »), dérivée de la logique de `select_states`. En v0.5
les badges sont **lisibles mais statiques** (pas de sliders move_threshold/facing :
ce serait un trompe-l'œil tant que l'édition réelle des règles n'existe pas —
renvoyée à ADR-038 Phase 5 / event sheets). À terme, `select_states` est
refactorisée dans `toile-scene` (enum `ConditionDescription`) pour que l'éditeur en
dérive le texte sans duplication (Phase 0.5).

### 5. États custom (attack/hurt/dash)

**Lecture seule** en v0.5 : affichés avec un badge « script » ; leur déclenchement
passe par les event sheets (`PlayAnimation`, ADR-038 Phase 5). On ne sur-promet pas
une UI de règles qui n'existe pas encore.

## Plan d'implementation

| Phase | Contenu | Effort |
|---|---|---|
| **0 — Lisibilité** ✅ | Renommages browser ; anims+états visibles dans la hiérarchie (groupes, tags grid/strip, badges conditions, `!` cassés) ; nettoyage des bindings à la suppression d'anim ; IA appelle auto_bind | S+M |
| **0.5 — Conditions drift-proof** (option) | Refactor `select_states` → enum partagée dans `toile-scene` + label dérivé testé | L |
| **1 — Ajout additif** (cœur) ✅ | `add_animation_to_entity` unifié (KeepBoth idempotent / Replace) ; action browser « Add as animation to «X» » (libellée + désactivée hors sélection) + drain ; bouton « + Add Animation » inspecteur (différé → push_undo) ; IA + importers Strip/Aseprite/quick-add routés sur le helper ; garde-fou grille `rows>1` (refus + message). Dialogue **modal** Add Animation (fps/loop/collision) reporté en Phase 2. | L |
| **2 — Inspecteur Animation States** | Section binding + conditions + Add Animation | M |
| **3 — Replace sprite + garde-fou grille/strip** | Dialogue Keep/Replace unique (browser + Browse) ; `detect_sourcing_model` | M |
| **4 — Arbre État>Anim>Frames** (option, reportée) | Jugé régressif par 2 revues → seulement si besoin confirmé ; nécessite d'extraire d'abord le frame picker en composant | L |

## Consequences

### Positives
- L'asset browser ne crée plus d'entité par accident ; « ajouter une anim » est
  l'action par défaut sur la sélection.
- Ajout incrémental (Idle présent → ajouter Walk sans rien écraser).
- Une **règle de collision unique** pour les 3 importers + l'IA (alignement
  humain ↔ IA).
- L'entité, ses animations, ses états et leurs conditions deviennent un arbre
  navigable.
- Additif : zéro scène cassée.

### Negatives / honnêteté
- Effort réel ~2× la première estimation : le badge condition cache un refactor de
  `select_states` ; le dialogue Add Animation est du **NEW** (struct d'état +
  cycle de vie + focus auto), pas une simple réutilisation ; afficher le groupe
  Animations dans la hiérarchie est du NEW UI.
- `animation_states` devient un opt-OUT progressif (créé par `auto_populate` quand
  une entité a des anims nommées + un behavior) — précision d'ADR-038, à refléter
  dans le commentaire de `lib.rs`.

## Alternatives rejetees

1. **Statu quo + doc des 3 chemins** → ne résout ni l'incohérence ni la perte de
   données.
2. **Graphe de transitions (Godot AnimationTree)** → hors scope « missing middle » ;
   une table état→condition→anim suffit et reste honnête sur le runtime (state
   machine pure).
3. **Forcer un seul modèle de sourcing** → casse la rétrocompat/flexibilité ; on
   garde les deux avec garde-fous visuels (tags + avertissement « mixed »).
4. **Arbre État>Anim>Frames en SpriteAnim dès v0.5** → jugé régressif (profondeur,
   perte de la comparaison côte-à-côte) → enrichissement non-destructif des listes
   plates, arbre reporté en Phase 4 optionnelle.
5. **Sliders move_threshold/facing dès v0.5** → trompe-l'œil (×24 et 5.0 restent
   figés) → reporté à ADR-038 Phase 5.
6. **Dupliquer `select_states` en label** → piège de maintenance → refactor/partage
   via enum.

## Questions ouvertes

- Placement de `select_states` refactorisée : `toile-scene` (à côté d'`AnimState`)
  vs module helper partagé — vérifier ADR-009 (flux de dépendances).
- États custom : garder lecture seule, ou ouvrir la création + condition basique
  plus tard ?
- Aligner l'outil **MCP** (`toile-mcp`, round-trips JSON) sur la même règle de
  collision — extraire la logique assez bas (`toile-scene`) pour les deux surfaces
  IA ?
- Dialogue Add Animation cas GRID : embarquer le frame picker, ou importer la
  séquence complète puis renvoyer vers « + pick » ?

## Suivi d'implémentation — Phase 1 (revue adversariale)

Une revue multi-agents du diff Phase 1 (25 findings, 21 confirmés) a conduit à durcir
l'implémentation **avant merge** :

- **Garde-fou grille** (finding HIGH) : l'ajout additif utilise le modèle *strip*, que le
  runtime découpe en **une seule ligne** ; un asset en grille `rows>1` serait silencieusement
  mal découpé. Le drain refuse désormais ces assets (métadonnée connue) avec un message
  orientant vers « Create new entity » / « Replace sprite » (chemins *grid*). Le bouton
  inspecteur reste un importeur de **bandes horizontales** (fichier brut, pas de métadonnée).
- **`KeepBoth` idempotent** : re-ajouter un clip identique (même nom + source + frames) est un
  no-op — fini l'accumulation de `walk_2`, `walk_3`… au ré-import.
- **`AnimConflict::Abort` supprimé** (code mort, aucun appelant) → règle réduite à
  `KeepBoth | Replace`.
- **Feedback sélection** : l'éditeur pousse le nom de l'entité sélectionnée dans le browser ;
  l'action est **libellée « Add as animation to «X» »** et **désactivée** hors sélection.
- **Undo inspecteur** : le bouton « + Add Animation » diffère la mutation (`pending_add_anim_file`)
  pour passer par `push_undo` hors de l'emprunt `&mut entity`. Ajouté aussi à l'état vide.

NITs acceptés (non bloquants, notés pour plus tard) : nommage opaque `walk`/`walk_2` ;
exact-name vs synonyme dans `auto_populate` ; parité d'iconographie detail panel ↔ menu contextuel.

## Validation

- Logique pure (collision, sourcing, `auto_populate` non-overwrite) : tests
  unitaires.
- Round-trip de scène (additif, rétrocompat).
- Flux d'ajout + arbre : **macpilot** (le viewport est invisible à `egui_kittest`).
- Revue adversariale par workflow sur chaque diff (ultracode).
