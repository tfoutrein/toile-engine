# ADR-009 : Architecture modulaire avec façade

- **Statut :** Acceptée
- **Date :** 2026-03-14
- **Concerne :** v0.1

## Contexte

L'architecture du moteur doit être structurée pour : la maintenabilité (chaque module a une responsabilité claire), la testabilité (les modules sont testables indépendamment), la remplaçabilité (changer un backend sans toucher le reste), et la simplicité d'usage (l'utilisateur final a une API unifiée simple).

## Options considérées

### Monolithique
- **Pour :** simple à démarrer. Pas de gestion de dépendances inter-crates. Un seul `cargo build`.
- **Contre :** au-delà de quelques milliers de lignes, le code devient un enchevêtrement. Les temps de compilation souffrent (tout recompile à chaque changement). Impossible de swapper un backend. Difficile pour les contributions (tout le monde travaille dans les mêmes fichiers).

### Micro-crates (sur-modularisé)
- **Pour :** isolation maximale. Chaque micro-module (math, logging, color, rect...) est un crate séparé.
- **Contre :** explosion de la gestion de dépendances. Les changements transversaux touchent 15 fichiers `Cargo.toml`. La surcharge cognitive de naviguer entre des dizaines de crates minuscules. Prématuré pour un MVP.

### Modulaire avec façade (notre choix)
- **Pour :** modules séparés avec frontières claires, mais une façade de haut niveau (`toile-app`) qui ré-exporte tout. L'utilisateur a une API simple (`use toile::*`), les internes sont proprement découpés.
- **Contre :** nécessite une discipline pour maintenir les frontières propres.

## Décision

**Architecture modulaire en workspace Cargo avec façade `toile-app`.**

### Structure des crates

```
toile/
├── Cargo.toml              (workspace)
├── crates/
│   ├── toile-core/          Math, temps, logging, erreurs, handles générationnels, events
│   ├── toile-platform/      Abstraction SDL3 : fenêtre, input, events plateforme
│   ├── toile-graphics/      Renderer wgpu : textures, sprite batcher, caméra, texte, debug draw
│   ├── toile-audio/         Intégration kira : chargement sons, playback, mixing
│   ├── toile-collision/     Formes, tests d'overlap, grille spatiale
│   ├── toile-ecs/           Re-export hecs + composants Toile (Transform, Sprite, etc.)
│   ├── toile-assets/        Asset manager, loaders de formats, handles
│   ├── toile-scripting/     VM Lua (mlua), bindings API moteur, hot-reload
│   ├── toile-scene/         Sérialisation JSON, validation JSON Schema, diffing
│   ├── toile-editor/        Éditeur egui : panneaux, gizmos, undo/redo
│   ├── toile-mcp/           Serveur MCP : outils, handlers, screenshots
│   ├── toile-cli/           Binaire CLI : scaffolding, run, build
│   └── toile-app/           Façade : App builder, game loop, lifecycle, re-exports
```

### Règles d'architecture

1. **Les dépendances vont vers le bas.** `toile-app` dépend de tout. `toile-editor` dépend de `toile-graphics`, `toile-ecs`, `toile-scene`. `toile-core` ne dépend de rien.

2. **Les backends sont derrière des traits.** `toile-platform` expose `trait Platform`. `toile-graphics` expose `trait GraphicsBackend`. `toile-audio` expose `trait AudioBackend`. Le moteur est écrit contre ces traits, pas contre des implémentations concrètes.

3. **Pas de dépendances circulaires.** Si deux crates ont besoin de se connaître, l'interface commune monte dans `toile-core`.

4. **Un crate = une responsabilité.** `toile-graphics` ne connaît pas l'audio. `toile-audio` ne connaît pas le rendu. Ils communiquent via le monde ECS et les événements.

## Conséquences

### Positives
- Compilation incrémentale efficace (changer `toile-scripting` ne recompile pas `toile-graphics`)
- Les backends sont swappables (changer SDL3 pour winit = changer l'implémentation de `trait Platform`)
- Les contributions sont localisées (un contributeur sur l'audio ne touche pas le rendu)
- L'API utilisateur est simple grâce à la façade `toile-app`

### Négatives
- Plus de fichiers `Cargo.toml` à maintenir
- La discipline des frontières de crates demande de la rigueur
- Certains types partagés (Handle, EntityId) doivent vivre dans `toile-core` ce qui crée un "gravity well"
