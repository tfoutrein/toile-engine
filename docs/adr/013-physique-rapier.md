# ADR-013 : Rapier2D pour la simulation physique

- **Statut :** Acceptée
- **Date :** 2026-03-14
- **Concerne :** v0.2
- **Remplace partiellement :** La collision simple de la v0.1 (AABB/Circle + MTV) reste disponible pour les cas légers.

## Contexte

La v0.1 offre une détection de collision basique (AABB, Circle, overlap test + MTV push-out). C'est suffisant pour un Breakout ou un platformer simple, mais pas pour les jeux qui reposent sur la physique : rigid bodies, gravité, rebonds réalistes, joints/contraintes, plateformes mobiles.

## Options considérées

### Box2D (via bindings FFI)
- **Pour :** Le standard de l'industrie depuis 15 ans. Utilisé par Unity, GameMaker, Love2D. Documentation massive. Comportement très bien connu.
- **Contre :** C++ natif → FFI depuis Rust. Les bindings Rust (box2d-rs) sont souvent en retard sur les versions upstream. Pas de parallélisme natif. L'API C++ est moins ergonomique que du Rust natif.

### Rapier2D
- **Pour :** Rust pur. Conçu par les créateurs de nalgebra. Performance excellente (SIMD, parallélisme optionnel). API ergonomique et bien typée. Intégration naturelle avec glam. Utilisé par Bevy. Déterminisme optionnel (pour le netcode). Support des joints, CCD (continuous collision detection), et capteurs.
- **Contre :** API plus complexe que notre système simple actuel. Dépendance conséquente en taille de compilation. Peut être overkill pour les jeux très simples.

### Custom (étendre le système v0.1)
- **Pour :** Contrôle total. Pas de dépendance. Léger.
- **Contre :** Réinventer la roue. Les edge cases de la physique (empilement, tunneling, joints) sont un gouffre d'ingénierie. On n'arrivera jamais à la qualité de Rapier.

## Décision

**Rapier2D comme système physique optionnel, en complément de la collision simple existante.**

1. **Rust natif.** Rapier s'intègre naturellement dans l'architecture Rust de Toile. Pas de FFI, pas de wrappers unsafe, types partagés avec glam.

2. **Ne pas imposer.** Rapier est une dépendance **optionnelle**. Les jeux qui n'ont besoin que de collision basique (Breakout, top-down) continuent à utiliser `toile-collision` directement. Rapier est pour les jeux qui ont besoin de vraie physique (rigid bodies, joints, plateformes mobiles).

3. **Déterminisme.** Rapier supporte la simulation déterministe, ce qui est un prérequis pour le rollback netcode (v1.5). En intégrant Rapier maintenant, on prépare le terrain.

4. **Bevy l'utilise.** La plus grande validation dans l'écosystème Rust game dev. Les problèmes sont identifiés et corrigés par une large communauté.

## Intégration

- `toile-collision` reste le module de collision léger (overlap tests, spatial grid)
- Un nouveau module ou une feature flag dans `toile-ecs` fournit `RigidBodyComponent`, `PhysicsWorld`
- La physique Rapier tourne dans un système ECS qui met à jour les `Transform` chaque tick
- L'éditeur expose les propriétés physiques dans l'Inspector (body type, mass, friction, bounciness)

## Conséquences

### Positives
- Simulation physique de qualité professionnelle sans effort d'implémentation
- Déterminisme disponible pour le futur netcode
- API Rust ergonomique, intégration naturelle
- Les jeux simples ne sont pas impactés (Rapier est optionnel)

### Négatives
- Dépendance conséquente (temps de compilation accru)
- Courbe d'apprentissage pour les concepts Rapier (ColliderBuilder, RigidBodyBuilder, etc.)
- Deux systèmes de collision coexistent (simple + Rapier), ce qui peut être confus
