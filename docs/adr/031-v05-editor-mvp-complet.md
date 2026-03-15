# ADR-031 : v0.5 "Complete Editor MVP" — Editeur complet et jeu jouable de bout en bout

- **Statut :** Acceptee
- **Date :** 2026-03-15
- **Concerne :** v0.5
- **Remplace :** Le scope initial de la v0.5 ("Web & Share") est repoussé post-v1.0

## Contexte

Après la v0.4, toutes les briques visuelles et gameplay existent :
- 7 behaviors (Platform, TopDown, Bullet, Sine, Fade, Wrap, Solid)
- 8 presets de particules + éditeur visuel
- Event sheets (8 conditions, 11 actions, exécuteur)
- Prefabs (sérialisation, MCP tools)
- Éclairage 2D, ombres, post-processing, SDF fonts, shader graph
- Scene stack + transitions (fade, slide)
- Tilemap painting

**Problème :** ces systèmes sont orphelins. Impossible de construire un jeu complet depuis l'éditeur :
- Les entities n'ont ni behaviors, ni event sheets, ni collision shapes
- Pas de mode Play pour tester le jeu dans l'éditeur
- Pas de runtime qui exécute les behaviors et les event sheets
- Pas de `toile run` pour lancer un projet comme un jeu

## Décision

**La v0.5 connecte tout : format de scène étendu, runtime intégré, éditeur complet, `toile run`.**

L'objectif : un utilisateur peut créer un jeu complet (menus, gameplay, game over) depuis l'éditeur et le lancer avec `toile run`.

### 1. Format de scène étendu (EntityData v2)

```rust
pub struct EntityData {
    // Existant
    pub id: u64,
    pub name: String,
    pub x: f32, pub y: f32,
    pub rotation: f32,
    pub scale_x: f32, pub scale_y: f32,
    pub width: f32, pub height: f32,
    pub layer: i32,
    pub sprite_path: String,

    // Nouveau v0.5
    pub behaviors: Vec<BehaviorConfig>,         // Platform, TopDown, etc.
    pub event_sheet: Option<String>,            // chemin vers .event.json
    pub particle_emitter: Option<String>,       // chemin vers .particles.json
    pub tags: Vec<String>,                      // pour collisions et event sheets
    pub variables: HashMap<String, f64>,        // variables par entité
    pub collision_shape: Option<CollisionShape>, // AABB ou Circle
    pub visible: bool,                          // toggle visibilité
}
```

```rust
pub struct SceneData {
    // Existant
    pub name: String,
    pub entities: Vec<EntityData>,
    pub tilemap: Option<TilemapData>,
    pub next_id: u64,

    // Nouveau v0.5
    pub background_color: [f32; 4],
    pub camera_zoom: f32,
    pub camera_position: [f32; 2],
    pub gravity: [f32; 2],                    // pour behaviors Platform
}
```

### 2. Runtime : Game Runner

Un "Game Runner" qui prend un `SceneData` et le fait tourner :

- **BehaviorExecutor** : chaque frame, itère sur les entités avec behaviors et exécute Platform/TopDown/Bullet/Sine/Fade/Wrap/Solid
- **EventSheetExecutor** : chaque frame, évalue les event sheets de chaque entité, exécute les commandes (SetPosition, Destroy, SpawnObject, GoToScene, PlaySound, etc.)
- **CollisionSystem** : détection AABB/Circle entre entités taggées, fournit les résultats aux event sheets (OnCollisionWith)
- **ParticleManager** : charge et update les émetteurs attachés aux entités
- **SceneLoader** : charge les scènes depuis JSON, instancie les textures, résout les prefabs

### 3. Éditeur : panneaux manquants

| Panneau | Description |
|---------|-------------|
| **Behavior Inspector** | Section dans l'Inspector : +Add Behavior, dropdown (Platform/TopDown/Bullet/Sine/Fade/Wrap/Solid), éditeur de paramètres pour chaque type |
| **Tags & Variables** | Section dans l'Inspector : tags (chips éditables), variables initiales (clé/valeur) |
| **Event Sheet Editor** | Panneau modal ou mode dédié : créer/éditer des events (conditions + actions) avec pickers visuels |
| **Collision Shape** | Section dans l'Inspector : choix AABB/Circle, affichage du gizmo dans le viewport |
| **Particle Emitter** | Section dans l'Inspector : sélectionner un .particles.json ou un preset |
| **Sprite Selector** | Section dans l'Inspector : choisir une texture parmi les assets du projet |
| **Scene Settings** | Panneau accessible via le menu : background color, gravity, camera defaults |
| **Play/Stop** | Bouton Play dans la toolbar : lance le Game Runner sur la scène courante, bouton Stop pour revenir à l'éditeur |

### 4. CLI : `toile run`

```bash
toile run                    # lance le jeu depuis Toile.toml (scène d'entrée)
toile run --scene level2.json  # lance une scène spécifique
```

Le manifest `Toile.toml` est étendu :

```toml
[project]
name = "My Game"
version = "0.1.0"
engine = "toile"

[game]
entry_scene = "scenes/main.json"
window_width = 1280
window_height = 720
window_title = "My Game"
background_color = [0.1, 0.1, 0.15, 1.0]
```

### 5. MCP : outils manquants

| Outil | Description |
|-------|-------------|
| `add_behavior` | Ajouter un behavior à une entité |
| `remove_behavior` | Retirer un behavior d'une entité |
| `set_entity_tags` | Définir les tags d'une entité |
| `set_entity_variables` | Définir les variables initiales d'une entité |
| `create_event_sheet` | Créer un event sheet avec conditions/actions |
| `get_event_sheet` | Lire un event sheet |
| `update_event_sheet` | Modifier un event sheet |

## Phases de livraison

### Phase 1 : Format + Runtime (fondations)
1. Étendre EntityData et SceneData (champs v0.5)
2. Implémenter le Game Runner (behavior executor + event sheet executor + collision)
3. `toile run` — charger un Toile.toml et lancer le jeu
4. Valider avec un Breakout jouable créé uniquement en JSON

### Phase 2 : Éditeur Inspector enrichi
5. Inspector : behaviors (add/remove/edit)
6. Inspector : tags, variables, collision shape
7. Inspector : particle emitter selector + sprite selector
8. Scene Settings panel

### Phase 3 : Play Mode + Event Sheet Editor
9. Play/Stop dans l'éditeur
10. Event Sheet Editor (mode ou panneau dédié)
11. Scene transitions visuelles (GoToScene preview)

### Phase 4 : MCP + Polish
12. MCP tools (add_behavior, event sheets, etc.)
13. Undo/Redo basique dans l'éditeur
14. Tests E2E : créer un jeu complet depuis l'éditeur, le jouer

## Conséquences

### Positives
- Un jeu complet (menu → gameplay → game over) peut être créé depuis l'éditeur
- Toutes les briques existantes sont enfin connectées et utilisables
- Les MCP tools permettent à un LLM de créer un jeu complet sans toucher au code Rust
- Le format de scène est la source de vérité pour tout le jeu

### Négatives
- Le format de scène v0.5 n'est pas rétrocompatible (migration nécessaire)
- Le Game Runner est un composant complexe avec beaucoup d'interactions
- L'Event Sheet Editor est le panneau le plus complexe de l'éditeur

### Risques
- Scope ambitieux — les 4 phases doivent rester découpées et livrables indépendamment
- Le runtime behavior execution nécessite un système de collision fonctionnel
- L'Event Sheet Editor peut devenir un projet en soi — commencer minimal (liste + formulaires)
