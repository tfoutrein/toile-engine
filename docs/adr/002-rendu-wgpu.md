# ADR-002 : wgpu comme backend de rendu

- **Statut :** Acceptée
- **Date :** 2026-03-14
- **Concerne :** v0.1
- **Dépend de :** ADR-001 (Rust)

## Contexte

Le moteur 2D a besoin d'un backend de rendu cross-plateforme performant. Le rendu 2D est plus simple que la 3D (sprites texturés, batching, blending) mais doit supporter des features avancées à terme (éclairage 2D, compute shaders pour particules, post-processing). Le backend doit aussi cibler le web (WASM) pour la v0.5.

## Options considérées

### OpenGL 3.3 (direct)
- **Pour :** API simple et bien connue pour la 2D. Tutoriels massifs. Fonctionne sur quasiment tout le matériel. WebGL2 est un subset direct.
- **Contre :** déprécié sur macOS (Apple arrêté à 4.1). API stateful et error-prone. Pas de compute shaders sous GL 4.3. Pas d'avenir sur Apple. Bindings Rust (gl/glow) moins ergonomiques que wgpu.

### SDL3 GPU API
- **Pour :** API C propre et cross-plateforme. Abstrait Vulkan/Metal/D3D12. Livré avec SDL3 (déjà utilisé pour le windowing). Simple pour de la 2D basique.
- **Contre :** API C nécessitant des FFI wrappers en Rust. Pas de compute shaders (en 2026). Moins flexible que wgpu pour les features avancées (éclairage, post-processing). Pas de ciblage WebGPU natif.

### Vulkan (direct)
- **Pour :** performance et contrôle maximaux. Multi-threading explicite. MoltenVK pour macOS.
- **Contre :** extrêmement verbeux (~1000 lignes pour un triangle). Pipeline state objects, descriptor sets, synchronisation manuels. Overkill total pour un moteur 2D. Le ratio effort/bénéfice est désastreux pour la 2D.

### wgpu
- **Pour :** Rust natif (zéro FFI). Abstrait Vulkan, Metal, DX12, OpenGL. Cible WebGPU nativement pour le web. API moderne (compute shaders, binding explicite). Battle-tested (Bevy, Firefox). Maintenu par l'équipe gfx-rs (backing Mozilla).
- **Contre :** léger overhead de la couche d'abstraction (négligeable pour la 2D). Shaders en WGSL (moins connu que GLSL, mais convertible via naga). Dépendance significative.

### sokol_gfx
- **Pour :** single-header C, léger, cible GL3.3/Metal/D3D11/WebGPU. API propre. Excellent pour du C.
- **Contre :** C-natif, nécessite des FFI wrappers Rust. Pas d'écosystème Rust. Moins de features que wgpu. Communauté plus petite.

## Décision

**wgpu.**

1. **Rust natif.** wgpu est la bibliothèque graphique de référence en Rust. Zéro overhead FFI, intégration naturelle avec le reste du stack (egui-wgpu, etc.).

2. **Un renderer, toutes les plateformes.** Vulkan (Linux/Windows/Android), Metal (macOS/iOS), DX12 (Windows), OpenGL (fallback), et WebGPU (web). On écrit le renderer une fois, il tourne partout.

3. **Chemin web natif.** wgpu cible WebGPU nativement. Notre export WASM (v0.5) ne sera pas un hack de compatibilité mais une cible de première classe. C'est stratégique pour le positionnement "Toile" (canvas + web).

4. **Marge pour les features avancées.** Compute shaders (particules GPU v0.4), render-to-texture (post-processing v0.4), éclairage 2D (v0.4). Le plafond technique est très haut.

5. **Bevy l'utilise.** La plus grande validation possible dans l'écosystème Rust game dev. Les problèmes sont identifiés et corrigés par une large communauté.

## Conséquences

### Positives
- Cross-plateforme sans effort (desktop + web + mobile futur)
- Les features avancées (compute, render-to-texture) sont accessibles sans changer de backend
- L'écosystème Rust (egui-wgpu, etc.) s'intègre naturellement
- Le chemin vers WebGPU est natif, pas un hack

### Négatives
- Le langage de shaders est WGSL (pas GLSL). Moins connu. Mitigé par naga (transpileur GLSL→WGSL).
- wgpu est une dépendance lourde en taille de compilation. Mitigé par la compilation incrémentale.
- L'overhead d'abstraction, bien que négligeable pour la 2D, existe. Si on constate des bottlenecks, on peut toujours descendre vers le backend natif (wgpu-hal).
