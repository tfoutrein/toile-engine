# ADR-001 : Rust comme langage du moteur

- **Statut :** Acceptée
- **Date :** 2026-03-14
- **Concerne :** v0.1

## Contexte

Le choix du langage de programmation est la décision la plus structurante du projet. Il détermine l'écosystème de bibliothèques, les profils de contributeurs, la sécurité du code, les performances, et la viabilité à long terme. Toile est un moteur 2D open-source destiné à durer une décennie minimum.

## Options considérées

### C
- **Pour :** performance maximale, simplicité, portabilité universelle, interop avec toutes les bibliothèques (SDL, Raylib, stb). Raylib prouve qu'un framework 2D excellent se construit en C.
- **Contre :** aucune sécurité mémoire (UB, use-after-free, double-free), pas de gestion de paquets standard (CMake hell), pas de generics, code verbeux pour l'ECS. La contribution communautaire sur un projet C open-source est risquée sans discipline stricte.

### C++ (C++20)
- **Pour :** écosystème le plus mature en game dev (EnTT, GLM, Dear ImGui, Box2D). Templates pour abstractions zero-cost. Le stack entier du moteur 2D existe et est battle-tested. Plus de documentation et de recrutement potentiel.
- **Contre :** UB toujours possible malgré les smart pointers. CMake + vcpkg/Conan = friction de build. Temps de compilation longs. Les contributions communautaires sur du C++ complexe sont souvent de qualité variable. Stagnation en mindshare face à Rust.

### Rust
- **Pour :** sécurité mémoire à la compilation (borrow checker). Cargo est le meilleur système de build/paquets de l'industrie. L'écosystème game dev est mature en 2026 (wgpu, glam, hecs, kira, winit, mlua). Performances comparables à C/C++. Le système de types sert de documentation machine-readable pour les LLMs. Pas de data races. Adoption en accélération.
- **Contre :** courbe d'apprentissage (borrow checker). Temps de compilation incrémentale plus longs que C. Certains patterns game dev (shared mutable state, parent-enfant) demandent des contournements (ECS résout la plupart). Pool de développeurs plus petit que C++.

### Zig
- **Pour :** performance comparable à C, comptime pour des generics zero-cost, cross-compilation de première classe, interop C transparente. Build system intégré excellent.
- **Contre :** pré-1.0 — breaking changes attendus. Écosystème minuscule pour le game dev. Pas de gestionnaire de paquets aussi mature que Cargo. Risqué pour un projet destiné à durer 10+ ans.

## Décision

**Rust.**

Les raisons déterminantes :

1. **Sécurité pour l'open-source.** Un moteur open-source acceptant des contributions doit minimiser les classes de bugs catastrophiques. Le borrow checker élimine use-after-free, double-free et data races à la compilation. C'est un avantage structurel, pas juste une commodité.

2. **Cargo.** `cargo build` fonctionne du premier coup. Pas de configuration de build, pas de dépendances système à installer manuellement. C'est critique pour l'adoption communautaire. Un nouveau contributeur clone le repo et compile en une commande.

3. **L'écosystème est prêt.** wgpu (rendu), glam (math), hecs (ECS), kira (audio), mlua (Lua scripting), egui (UI éditeur) — chaque brique existe, est maintenue, et est conçue pour le game dev. Ce n'était pas vrai en 2022, mais c'est vrai en 2026.

4. **AI-friendly.** Les types Rust servent de documentation que les LLMs consomment. Le compilateur attrape les erreurs avant le runtime, réduisant les cycles de debug. Les LLMs produisent du Rust plus fiable que du C++ grâce au filet de sécurité du compilateur.

5. **Pérennité.** L'adoption de Rust en programmation système accélère (Linux kernel, Android, Windows). C++ est stable mais stagne en mindshare. Zig est excitant mais pré-1.0. Rust est le pari le plus sûr pour un projet qui doit durer une décennie.

## Conséquences

### Positives
- Les classes de bugs mémoire sont éliminées à la compilation
- Le build system est trivial pour les contributeurs
- L'écosystème de crates fournit toutes les briques nécessaires
- Le typage fort améliore la génération de code par IA

### Négatives
- La courbe d'apprentissage du borrow checker réduit le pool de contributeurs potentiels
- Les temps de compilation incrémentale sont plus longs que C (mitigé par workspace + cargo-watch)
- Certains patterns game dev classiques (shared mutable state) nécessitent des adaptations (ECS, interior mutability)
- Le hot-reload de code natif est plus complexe qu'en C (mitigé par la couche Lua pour la logique de jeu)
