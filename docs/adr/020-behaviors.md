# ADR-020 : Système de Behaviors (comportements pré-construits)

- **Statut :** Acceptée
- **Date :** 2026-03-15
- **Concerne :** v0.3

## Contexte

Les behaviors sont des packages de logique pré-construits qu'on attache à une entité via l'inspecteur, sans écrire de code. Attacher un behavior "Platform" donne instantanément la gravité, le saut, et la collision avec les solides. C'est le mécanisme clé d'accessibilité de Construct et GameMaker.

## Décision

**Système de behaviors attachables aux entités, avec paramètres configurables dans l'inspecteur.**

### Behaviors v0.3

| Behavior | Paramètres | Description |
|----------|-----------|-------------|
| **Platform** | gravity, jump_force, max_speed, acceleration, deceleration, coyote_time, input_buffering, max_jumps | Platformer character controller complet. S'appuie sur la collision simple (toile-collision) pour la détection de sol/mur. |
| **TopDown** | max_speed, acceleration, deceleration, diagonal_correction | Mouvement 4/8 directions. Gère l'input WASD/flèches. |
| **Bullet** | speed, acceleration, gravity, bounce_off_solids | Se déplace en ligne droite. Pour projectiles, ennemis patrouilleurs. |
| **Solid** | (aucun) | Marque l'entité comme obstacle. Les behaviors Platform et TopDown collisionnent automatiquement avec les Solids. |
| **Sine** | property, magnitude, period, wave_type | Oscille une propriété (x, y, angle, opacity, size). Pour plateformes flottantes, effets de pulse. |
| **Fade** | fade_in_time, fade_out_time, destroy_on_fade_out | Anime l'opacité. Auto-destruction optionnelle après fade-out. |
| **Wrap** | (aucun) | Wrap aux bords de la vue (quand l'entité sort d'un côté, elle réapparaît de l'autre). |
| **Physics** | body_type, mass, friction, restitution, damping | Wrapper autour de Rapier (toile-physics) avec des paramètres accessibles dans l'inspecteur. |

### Architecture

```rust
pub trait Behavior: Send + Sync {
    fn name(&self) -> &str;
    fn update(&mut self, entity: &mut EntityState, dt: f32, world: &BehaviorWorld);
    fn params(&self) -> &[BehaviorParam];
    fn set_param(&mut self, name: &str, value: ParamValue);
}

pub struct BehaviorParam {
    pub name: String,
    pub kind: ParamKind,  // Float, Int, Bool, Enum
    pub value: ParamValue,
    pub description: String,
}
```

Les behaviors sont stockés comme composants ECS (`BehaviorList`) sur chaque entité. Un système parcourt toutes les entités avec des behaviors et les met à jour chaque tick.

### Interaction avec les Event Sheets
Les event sheets peuvent tester les états des behaviors comme conditions (ex: "Platform is on ground") et déclencher des actions de behaviors (ex: "Platform simulate jump").

## Conséquences

### Positives
- Un jeu jouable en 5 minutes : placer un sprite, attacher "Platform" + "Solid" au sol, play
- Les paramètres sont éditables dans l'inspecteur sans code
- Les behaviors "Platform" et "TopDown" offrent un game feel professionnel out-of-the-box (coyote time, input buffering)
- Extensible : les utilisateurs avancés peuvent créer des behaviors custom en Lua

### Négatives
- Chaque behavior est un module à implémenter et tester
- L'interaction entre behaviors peut créer des conflits (Platform + Physics sur la même entité)
- Le "game feel" du behavior Platform doit être excellent dès la v0.3 — les valeurs par défaut sont critiques
