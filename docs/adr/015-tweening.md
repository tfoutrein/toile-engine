# ADR-015 : Bibliothèque de tweening intégrée

- **Statut :** Acceptée
- **Date :** 2026-03-14
- **Concerne :** v0.2

## Contexte

Le tweening (interpolation de propriétés sur le temps) est omniprésent dans les jeux 2D : animations UI (fade in/out, slide), mouvement de caméra lissé, effets visuels (pulse, shake), transitions de scènes. La v0.1 n'a aucun système de tweening — tout est fait manuellement avec lerp.

## Décision

**Bibliothèque de tweening intégrée dans toile-core, sans dépendance externe.**

Le tweening est assez simple pour ne pas nécessiter de crate externe. L'implémentation couvre :

### Fonctions d'easing
- Linear
- Ease-in (quadratic, cubic, sine)
- Ease-out (quadratic, cubic, sine)
- Ease-in-out (quadratic, cubic, sine)
- Bezier custom (4 points de contrôle)

### API

```rust
pub struct Tween {
    pub from: f32,
    pub to: f32,
    pub duration: f32,
    pub elapsed: f32,
    pub easing: EasingFunction,
    pub repeat: RepeatMode,  // Once, Loop, PingPong
}

impl Tween {
    pub fn new(from: f32, to: f32, duration: f32) -> Self;
    pub fn with_easing(self, easing: EasingFunction) -> Self;
    pub fn advance(&mut self, dt: f32) -> f32;  // returns current value
    pub fn is_done(&self) -> bool;
}
```

### Tween manager
Un `TweenManager` stocke les tweens actifs et les avance chaque frame. Les tweens peuvent être attachés à des entités ECS via un composant `TweenComponent`.

## Conséquences

### Positives
- API simple et chainable
- Pas de dépendance externe
- Réutilisable pour les animations UI, la caméra, les transitions de scènes
- Les fonctions d'easing sont aussi utilisées par le système de particules (size/color over life)

### Négatives
- Scope limité aux valeurs scalaires f32 (les tweens de Vec2 nécessitent deux tweens séparés, ou un wrapper)
