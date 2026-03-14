# ADR-012 : Système de particules CPU

- **Statut :** Acceptée
- **Date :** 2026-03-14
- **Concerne :** v0.2

## Contexte

Les effets de particules (feu, fumée, étincelles, pluie, explosions) sont indispensables pour un moteur 2D. Le MVP v0.1 n'en a pas. La v0.2 doit livrer un système de particules performant et accessible.

## Options considérées

### Particules GPU (compute shaders)
- **Pour :** Support de centaines de milliers de particules. Offloade le travail au GPU. Idéal pour les effets massifs.
- **Contre :** Nécessite compute shaders (WGSL compute). Plus complexe à implémenter. Pas compatible WebGL2 (seulement WebGPU). La lecture de l'état des particules côté CPU est coûteuse (pour la collision). Overkill pour la plupart des jeux 2D.

### Particules CPU
- **Pour :** Simple à implémenter. L'état des particules est accessible côté CPU (collision, scripting). Compatible avec tous les backends (y compris WebGL2). 5 000-10 000 particules à 60 FPS est largement suffisant pour les jeux 2D. Les particules sont rendues comme des sprites — réutilise le sprite batcher existant.
- **Contre :** Plafond de performance plus bas (~10k particules). Utilise le CPU pour le calcul (mais les jeux 2D ont rarement un CPU saturé).

### Bibliothèque tierce
- **Pour :** Pas de code à maintenir.
- **Contre :** Aucune bibliothèque Rust de particules 2D n'est assez mature ou intégrée pour notre pipeline.

## Décision

**Particules CPU, rendues via le sprite batcher existant.**

1. **Suffisant pour la 2D.** Les jeux 2D indie utilisent rarement plus de quelques milliers de particules simultanées. Les cas typiques (feu, pluie, explosion) nécessitent 100-2000 particules.

2. **Réutilise l'existant.** Chaque particule est un `DrawSprite` avec position, taille, couleur et UV. Le sprite batcher gère déjà le tri par texture et le batching — les particules bénéficient de toute l'optimisation existante sans code GPU additionnel.

3. **Compatible web.** Les particules CPU fonctionneront identiquement sur desktop et web (v0.5), sans dépendance aux compute shaders.

4. **Accessible au scripting.** L'état des particules est lisible en Lua pour des interactions gameplay (ex: collision avec des particules de feu).

**Les particules GPU sont planifiées pour la v0.4 (Visual Polish) comme optimisation pour les effets massifs.**

## Architecture

```rust
pub struct ParticleEmitter {
    pub shape: EmitterShape,       // Point, Circle, Rectangle, Line
    pub rate: f32,                 // particles per second
    pub burst: Option<u32>,        // one-shot burst count
    pub particle_lifetime: (f32, f32),  // min, max seconds
    pub initial_velocity: (Vec2, Vec2), // min, max
    pub gravity: Vec2,
    pub size_over_life: Curve,     // 0..1 → size multiplier
    pub color_over_life: Gradient, // 0..1 → RGBA
    pub rotation_speed: (f32, f32),
    pub texture: TextureHandle,
    pub blend_mode: BlendMode,     // Alpha, Additive
}

pub struct Particle {
    pub position: Vec2,
    pub velocity: Vec2,
    pub age: f32,
    pub lifetime: f32,
    pub size: f32,
    pub rotation: f32,
    pub color: [f32; 4],
}
```

L'émetteur spawne des particules chaque frame selon `rate`. Chaque particule est mise à jour (position += velocity * dt, age += dt) et supprimée quand `age >= lifetime`. Les propriétés visuelles (taille, couleur) sont interpolées sur la durée de vie via des courbes.

## Conséquences

### Positives
- Simple à implémenter et à debugger
- Réutilise le sprite batcher pour le rendu
- Compatible avec tous les backends (desktop, web)
- État lisible côté CPU pour le scripting et la collision

### Négatives
- Plafond à ~10k particules (suffisant pour la 2D)
- Consomme du CPU (négligeable pour les jeux 2D typiques)
- Les particules GPU devront être ajoutées séparément en v0.4 si nécessaire
