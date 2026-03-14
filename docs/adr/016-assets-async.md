# ADR-016 : Chargement d'assets asynchrone

- **Statut :** Acceptée
- **Date :** 2026-03-14
- **Concerne :** v0.2

## Contexte

La v0.1 charge tous les assets de manière synchrone dans `init()`. C'est simple mais bloquant — le jeu freeze pendant le chargement. Pour des jeux avec beaucoup d'assets, il faut un chargement asynchrone avec barre de progression et écran de chargement.

## Options considérées

### Chargement synchrone (statu quo)
- **Pour :** Simple. Pas de complexité de threading.
- **Contre :** Freeze visible au chargement. Pas d'écran de chargement possible. Inacceptable pour des jeux avec 50+ textures et sons.

### Thread dédié (std::thread)
- **Pour :** Simple à implémenter. Un thread charge les fichiers en arrière-plan, envoie les données via un channel.
- **Contre :** L'upload GPU (texture creation) doit se faire sur le thread principal. Nécessite un système de "finalization" sur le main thread.

### Tokio async runtime
- **Pour :** Async/await natif. Déjà dans le workspace (utilisé par le MCP server).
- **Contre :** Tokio est overkill pour du file I/O simple dans un game loop. Le game loop est synchrone (pas async). Mélanger async et sync est source de bugs.

## Décision

**Thread dédié avec channel pour le file I/O, finalization GPU sur le main thread.**

1. **Thread de chargement** : lit les fichiers disque (PNG, WAV, OGG, JSON) et décode les formats en mémoire (pixels RGBA, samples audio). Envoie les données brutes via un `mpsc::channel`.

2. **Main thread** : à chaque frame, poll le channel. Pour chaque asset prêt, effectue l'upload GPU (`create_texture`, etc.) et stocke le handle. Met à jour la progression.

3. **API utilisateur** :
```rust
// Queue an asset for loading
let handle = ctx.load_async("assets/big_tileset.png");

// Check if ready (non-blocking)
if ctx.is_loaded(handle) {
    // Use the asset
}

// Or wait with a loading screen
ctx.load_all_async(vec!["a.png", "b.wav", "c.json"]);
while !ctx.all_loaded() {
    let progress = ctx.loading_progress(); // 0.0..1.0
    // Draw loading bar
}
```

## Conséquences

### Positives
- Pas de freeze au chargement
- Écran de chargement avec barre de progression
- Compatible avec le game loop synchrone existant
- Simple à implémenter (un thread, un channel)

### Négatives
- Complexité additionnelle par rapport au chargement synchrone
- L'upload GPU reste sur le main thread (inévitable avec wgpu)
- Les assets ne sont pas immédiatement disponibles après `load_async`
