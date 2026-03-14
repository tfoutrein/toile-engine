# ADR-005 : hecs comme bibliothèque ECS

- **Statut :** Acceptée
- **Date :** 2026-03-14
- **Concerne :** v0.1
- **Dépend de :** ADR-001 (Rust)

## Contexte

L'architecture ECS (Entity Component System) est le standard des moteurs de jeu modernes pour ses avantages en performance (cache-friendly), en flexibilité (composition plutôt qu'héritage), et en parallélisme. Le moteur a besoin d'un ECS pour gérer les entités de jeu (sprites, ennemis, tiles, particules) et leurs composants (transform, sprite, collider, animator, script).

## Options considérées

### Bevy ECS
- **Pour :** l'ECS le plus avancé de l'écosystème Rust. Archetype-based. Scheduling automatique avec parallélisme. Système de `Events<T>`. Pattern `Resource` vs `Component`. Large communauté, bien documenté.
- **Contre :** tire l'écosystème Bevy (lourde dépendance). Le scheduling automatique impose des contraintes sur l'ordonnancement des systèmes — or nous voulons un game loop explicite. Temps de compilation significatif. Couplage fort avec les conventions Bevy.

### EnTT (via FFI)
- **Pour :** le standard de l'industrie C++ (utilisé dans Minecraft). Sparse-set, extrêmement rapide. Single-header.
- **Contre :** C++ natif. FFI depuis Rust complexe et unsafe. Perd tous les avantages du système de types Rust. Pas idiomatique.

### hecs
- **Pour :** archetype-based, minimal, standalone. Pas de scheduler, pas de framework, pas de plugins. Compile vite. API claire et petite. Utilisé en production.
- **Contre :** pas de scheduling automatique (on le gère nous-mêmes). Pas de système d'events intégré (on le construit). Communauté plus petite que Bevy ECS.

### flecs (via FFI)
- **Pour :** ECS C le plus complet. Langage de requêtes, réflexion, REST API debug, hiérarchies. Production-ready.
- **Contre :** C-natif (FFI). La richesse de features est un avantage en C mais un overhead en Rust où le système de types fournit déjà beaucoup de garanties.

### Custom (from scratch)
- **Pour :** contrôle total. Ajusté exactement à nos besoins. Pas de dépendance.
- **Contre :** du temps d'ingénierie significatif pour un résultat probablement inférieur à hecs dans les premiers mois. Vanité d'ingénieur au détriment du MVP.

## Décision

**hecs.**

1. **Minimal = on contrôle.** hecs fournit le stockage (archetype-based, cache-friendly) et les requêtes. Point. Pas de scheduler magique, pas d'opinions sur le game loop, pas de système de plugins. C'est exactement ce qu'on veut : une couche de stockage performante sur laquelle on construit NOTRE game loop et NOTRE ordonnancement de systèmes.

2. **Compilation rapide.** Bevy ECS ajoute des minutes de compilation. hecs compile en secondes. Pour un MVP où l'itération rapide est critique, c'est un facteur déterminant.

3. **Pas de vanité.** Construire un ECS custom est un piège classique du développement de moteur. hecs est prouvé, performant, et minimal. On peut toujours le remplacer plus tard si nos besoins divergent — la frontière est un trait `World`.

4. **Chemin d'évolution.** Si on atteint les limites de hecs (scheduling parallèle, requêtes complexes), on peut migrer vers Bevy ECS ou construire notre propre solution. L'abstraction `World` dans toile-ecs isole cette dépendance.

## Conséquences

### Positives
- Compilation rapide
- Game loop et scheduling explicites sous notre contrôle
- Stockage archetype cache-friendly (itération rapide sur 10k+ entités)
- API petite = facile à apprendre pour les contributeurs

### Négatives
- Pas de scheduling automatique (on code l'ordre des systèmes manuellement — acceptable pour le MVP)
- Pas de système d'events intégré (on construit le nôtre avec des ring buffers typés)
- Communauté plus petite que Bevy ECS (mais le code est simple et auditable)
