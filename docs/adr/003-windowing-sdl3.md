# ADR-003 : SDL3 pour le windowing et l'input

- **Statut :** Acceptée
- **Date :** 2026-03-14
- **Concerne :** v0.1
- **Dépend de :** ADR-001 (Rust)

## Contexte

Le moteur a besoin d'une couche d'abstraction plateforme pour la création de fenêtres, la gestion d'input (clavier, souris, gamepad), et les événements système (resize, focus, DPI). Cette couche doit être cross-plateforme (Windows, macOS, Linux) et extensible au mobile et au web à terme.

## Options considérées

### winit
- **Pour :** Rust natif pur. Le standard de l'écosystème Rust (utilisé par Bevy). Pas de dépendance C. Bonne intégration avec `raw-window-handle` pour wgpu.
- **Contre :** pas de support gamepad (nécessite `gilrs` séparément, avec ses propres edge cases). Pas d'audio intégré. Mobile plus jeune que SDL. Certains quirks spécifiques à la plateforme (IME, multi-fenêtre) moins matures.

### SDL3 (via rust-sdl3)
- **Pour :** 25+ ans de maturité cross-plateforme. Base de données gamepad exhaustive (des centaines de contrôleurs supportés out-of-the-box). Haptics intégrés. Support mobile éprouvé (iOS/Android). Connaissance encyclopédique des quirks de chaque OS.
- **Contre :** dépendance C nécessitant FFI. Les bindings Rust pour SDL3 sont plus récents que ceux de SDL2. Installation de SDL3 en dépendance système (ou bundling).

### GLFW
- **Pour :** léger, focalisé sur le windowing + input. API C propre.
- **Contre :** pas de gamepad database aussi complète que SDL. Pas d'audio. Pas de support mobile. FFI depuis Rust. Peu d'avantages sur SDL3.

## Décision

**SDL3 via des bindings Rust.**

1. **Input gamepad.** Le support gamepad de SDL est le meilleur de l'industrie. Des centaines de contrôleurs reconnus automatiquement avec les bons mappings. winit nécessite `gilrs` qui a des edge cases sur certains OS. Pour un moteur de jeu, le gamepad doit "juste marcher".

2. **Maturité plateforme.** SDL a 25+ ans d'accumulation de workarounds pour les quirks de Windows, macOS, et Linux. DPI handling, IME, fullscreen, multi-moniteur — SDL les gère mieux que winit grâce à cette expérience accumulée.

3. **Chemin mobile.** SDL3 est éprouvé sur iOS et Android. Quand Toile ciblera le mobile (v2.0), SDL sera prêt. winit sur mobile est possible mais moins mature.

4. **SDL3 n'est PAS le renderer.** On utilise SDL3 uniquement pour le windowing, l'input et l'abstraction plateforme. Tout le rendu passe par wgpu. Il n'y a pas de couplage entre SDL3 et le pipeline graphique.

**Fallback :** si l'intégration SDL3 + wgpu via `raw-window-handle` pose des problèmes insolubles en semaine 1, on bascule sur winit + gilrs. La couche `Platform` est un trait — le swap est localisé.

## Conséquences

### Positives
- Support gamepad best-in-class dès le MVP
- Maturité cross-plateforme inégalée
- Chemin mobile éprouvé pour le futur
- SDL3 modernise l'API (par rapport à SDL2) avec de meilleurs patterns

### Négatives
- Dépendance C (FFI) dans un projet sinon pur Rust
- Les bindings Rust SDL3 sont plus récents/moins stabilisés que SDL2
- Installation de SDL3 comme dépendance système (ou bundling via cmake)
- winit aurait été "plus Rustic" et aurait évité le FFI
