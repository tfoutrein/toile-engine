# ADR-037 : Menus contextuels (clic droit) intelligents et adaptatifs

- **Statut :** Acceptee — implementee (Phases 0-5, PR #6 a #10)
- **Date :** 2026-06-15
- **Concerne :** v0.5+ (editeur)

## Contexte

L'editeur Toile expose ses actions presque exclusivement par la barre de menus
(File/Edit/View), des boutons de toolbar et quelques raccourcis clavier
(`input.rs` gere C/V/D/S/Z/Suppr). Il n'existe **aucun menu contextuel au clic
droit** dans tout le projet (verifie : aucune occurrence de `context_menu`).

Resultat : les actions frequentes et **localisees sur un objet** (dupliquer une
entite, supprimer une tuile, definir un asset comme sprite de la selection,
reveler un fichier dans le Finder, reinitialiser une transformation…) obligent a
faire des aller-retours vers une toolbar ou a connaitre un raccourci. C'est
l'ecart le plus visible avec un editeur moderne (Godot, Unity, Tiled).

Le besoin exprime : un **menu contextuel intelligent**, c'est-a-dire dont le
contenu **s'adapte a la situation** — l'objet sous le curseur, l'etat de la
selection, les capacites de l'objet, le presse-papier, le mode d'edition.

## Decision

**Adopter un systeme UNIFIE de menus contextuels pilote par le contexte : un seul
vocabulaire d'items, un seul point d'application des mutations, et deux chemins de
rendu selon que la surface est un widget egui ou le viewport wgpu.**

### Constat technique fondateur (verifie sur le code reel + egui 0.33.3)

| Fait verifie | Ou | Consequence |
|---|---|---|
| `Response::context_menu()` existe en egui 0.33.3 | `response.rs:954` | Les **panneaux egui** branchent le clic droit **directement**, sans machine a etats |
| `Ui::menu_button` (SubMenuButton natif) | `ui.rs:3126` | Les sous-menus (« Add Behavior ▸ ») sont **gratuits** |
| Aucun `CentralPanel` quand un projet est ouvert | `overlay.rs:54` (pose le CentralPanel **uniquement** si `project_dir.is_none()`) | Le **viewport est du wgpu brut**, sans `Response` egui : le clic droit y est capte cote **winit** |
| Tri du renderer = `layer` PRIMAIRE puis texture | `sprite_renderer.rs:387` | « Bring to Front / Send to Back » agissent sur `entity.layer`, **pas** sur l'ordre du `Vec` |

### Architecture : deux rendus, un vocabulaire, un point de mutation

**(A) Surfaces qui SONT des widgets egui** — hierarchie, inspecteur, asset
browser, palette de tilemap, listes de presets/animations. On appelle
directement :

```rust
response.context_menu(|ui| self.entity_menu_items(ui, &mut actions));
```

egui gere l'ancrage au curseur, l'auto-fermeture (clic dehors / Echap) et les
sous-menus natifs. Aucun etat ad hoc.

**(B) Le viewport central (wgpu brut)** — il n'a aucune `Response`. On capte le
clic droit dans `input.rs::handle_update` (`is_mouse_just_pressed(MouseButton::Right)`),
on fait un hit-test monde *rotation-aware* (factorise depuis `input.rs:303-313`),
et on stocke `pending_context_menu: Option<(egui::Pos2, ContextMenuKind)>`. Le
rendu se fait en fin de `show_overlay_panels` via `egui::Area` (Order::Foreground)
+ `egui::Frame::menu` — **meme apparence** que les menus deroulants — en appelant
**les memes fonctions d'items** que la branche (A).

**Un seul vocabulaire** : les corps de menu (`entity_menu_items`,
`viewport_menu_items`, `scene_menu_items`, `asset_menu_items`…) sont des fonctions
partagees, appelees indifferemment depuis l'`Area` du viewport ou depuis
`response.context_menu` des panneaux. Libelle et look uniques garantis.

**Mutations DIFFEREES** (contrainte d'emprunt) : pendant qu'on tient le
`&egui::Context` (clone en `editor_app.rs:590`), on ne peut pas muter `self`. On
suit l'idiome **deja en place** pour `pending_add_to_scene` (`overlay.rs:638`) et
la barre de menus : la closure **pose des flags** dans `ContextMenuActions` (et
lit des donnees immuables pre-clonees avant le bloc egui : presence du
presse-papier, behaviors deja presents, min/max layer, capacites de l'entite),
puis `apply_context_actions(&mut self)` applique **tout apres** le bloc egui, avec
`push_undo()` avant mutation et `status_msg` apres.

### Intelligence : regles d'adaptation

1. **Objet sous le curseur** → determine le `ContextMenuKind` (entite / vide /
   cadre-guide / tuile / asset / frame…).
2. **Selection** : 0 → menu reduit (Paste, Add Entity, Fit All, toggles View) ;
   1 → menu entite complet. Le clic droit sur une entite non selectionnee **la
   selectionne d'abord**.
3. **Capacites de l'objet** : « Edit Sprite » si l'entite a un sprite, « Edit
   Particles » si elle a un emetteur, « Remove Collider/Light » selon presence,
   « Add Behavior ▸ » ne liste **que** les behaviors absents.
4. **Presse-papier** : « Paste » grise via `ui.add_enabled(clipboard.is_some(), …)`.
5. **Mode editeur** : Entity / Tilemap / Particle / SpriteAnim / AssetBrowser
   selectionnent la famille de menus. Le menu viewport-entite ne s'ouvre **jamais**
   hors mode Entity.

### Conflits de gestes & robustesse

- **Shift+clic-droit reste l'erase de tuile** (`viewport.rs:212`). Le menu
  viewport s'ouvre sur clic droit **sans Shift**, avec garde explicite
  (`if shift_held { return; }`). L'ordre de frame
  `update → draw → render_overlay` (`editor_app.rs:544-593`) garantit que la
  capture du menu (update) precede l'erase (draw).
- **Flag anti-clic-traversant** : `egui_consumed_pointer = ctx.is_pointer_over_area()
  || ctx.wants_pointer_input()` (calcule en fin de `show_overlay_panels`, lu a la
  frame suivante — lag d'1 frame, deja le comportement de `overlay.rs:48`) empeche
  le menu viewport de s'ouvrir sous un panneau egui.
- **Actions destructrices** (Delete Entity/Scene/Project, Clear Layer) passent par
  un **dialog de confirmation modal**, pose apres `apply_context_actions` (pattern
  des file-pickers existants).

### Decisions de design retenues (questions tranchees)

- **Tilemap** : le clic droit fait un **Eyedropper direct** (pioche immediate de
  la tuile sous le curseur), pas un menu — plus rapide. Erase/Fill restent
  accessibles autrement (Shift+clic-droit pour l'erase, outils).
- **Inspecteur** : pas de menu contextuel sur chaque champ. On privilegie de
  **petits boutons « Reset »** a cote des champs + Cmd/Ctrl+C/V natif sur les
  TextEdit. Le clic droit est **reserve aux sections** (behaviors, tags,
  variables, collider, light : Remove/Duplicate/Reset).

## Consequences

### Positives
- Couverture rapide et homogene de tout l'editeur pour un cout faible : la majeure
  partie **reutilise** des fonctions deja cablees (`input.rs` Copy/Paste/Dup/Delete,
  `inspector.rs` behaviors/tags/collider, `tilemap_tool` erase/flood_fill,
  `overlay.rs` add-to-scene).
- Un seul module a maintenir (`context_menu.rs`), un seul point de mutation.
- L'architecture en deux chemins est **forcee** par la structure existante (viewport
  wgpu vs panneaux egui), pas un choix arbitraire.

### Negatives / risques
- Le menu viewport (`Area`) est **invisible a `egui_kittest`** (il bypasse le
  routage winit) → validation par **macpilot** (capture PNG du clic droit reel).
- `ContextMenuActions` (sac de flags) peut grossir : a contenir via des
  `Option<payload>` groupes et un `apply_context_actions` en un seul `match`.
- Lag d'1 frame du flag `egui_consumed_pointer` (cas limite negligeable).
- Les raccourcis affiches dans les items NEW (Cmd+X/0/R, Shift+]/[) doivent etre
  cables comme **vrais** raccourcis (Phase 5), sinon le menu « mentirait ».

## Alternatives rejetees

1. **`CentralPanel` transparent (Sense::click) sur le viewport** pour obtenir une
   `Response` partout → rejete : capterait/consommerait les clics du viewport
   (selection, drag, resize, pan). Probleme de toute facon evite : le viewport n'a
   jamais de CentralPanel quand un projet est ouvert.
2. **Un menu ad hoc par surface** → rejete : duplication des items, looks
   divergents, multiples points de fermeture.
3. **Tout deporter en barre de menus / raccourcis** → rejete : ne repond pas au
   besoin d'actions contextuelles localisees.
4. **Machine a etats `pending_widget_menu` pour les panneaux** (suggeree par une
   critique) → rejete car fondee sur une premisse fausse : `response.context_menu`
   existe en egui 0.33.3 ; l'etat n'est requis que pour le viewport.

## Plan d'implementation (par phases, quick wins d'abord)

| Phase | Contenu | Fichiers | Effort |
|---|---|---|---|
| **0 — Fondations** | `context_menu.rs` (`ContextMenuKind`, `ContextMenuActions`, helper de rendu) ; champs `pending_context_menu`, `egui_consumed_pointer` ; `hit_test_entity` extrait (+ test) ; `apply_context_actions` ; helpers `reveal_in_finder`, `m_label` | `context_menu.rs` (NEW), `editor_app.rs`, `input.rs`, `helpers.rs` | M |
| **1 — Quick wins** | Menu viewport entite (Copy/Cut/Paste Here/Duplicate/Delete/Rename/Focus Camera) + Reveal in Finder/Copy path sur le browser ; **validation macpilot** | `input.rs`, `panels/overlay.rs`, `context_menu.rs`, `toile-asset-library/.../browser_panel.rs` | M |
| **2 — Panneaux** | `response.context_menu` sur hierarchie (entite/scene) + sections de l'inspecteur ; « Add Behavior ▸ » natif filtre | `panels/overlay.rs`, `panels/inspector.rs`, `context_menu.rs` | L |
| **3 — Browser/logs/projets** | Set-as-sprite-of-selection, Set background, menus logs (Copy/Save/Clear), menus projets (accueil) | `toile-asset-library/.../browser_panel.rs`, `panels/overlay.rs` | L |
| **4 — Tilemap/Particle/SpriteAnim** | Eyedropper direct (Tilemap) ; menus presets/animations/frames | `input.rs`, `tilemap_tool.rs`, `particle_editor.rs`, `panels/sprite_editor.rs` | M |
| **5 — Polish** | Z-order (sur `layer`), Reset Transform/Rotation, confirmations modales, raccourcis NEW | `context_menu.rs`, `inspector.rs`, `editor_app.rs`, `input.rs` | M |

## Questions ouvertes (hors scope immediat)

- **Lock/Unlock entite** : tres utile mais exige un champ `locked: bool` sur
  `EntityData` → **changement de format de scene** (relevant d'ADR-031). A traiter
  dans un avenant ou un ADR dedie.
- Menus sur les **calques de tilemap** (Duplicate/Rename/Merge Down) : suppose de
  clarifier d'abord comment l'utilisateur choisit le « calque actif ».
- Modes **AICopilot** et editeur d'**event-sheets** : menus specifiques a evaluer
  ulterieurement.

## Validation

- Logique pure (`hit_test_entity`, `apply_context_actions`) : tests unitaires.
- Surfaces-widgets : `egui_kittest` (`secondary_clicked`, pattern
  `particle_editor.rs:79`).
- Menu viewport + flux reels : **macpilot** (clic droit reel, confirmations,
  Z-order visuel) — invisibles a `egui_kittest`.
