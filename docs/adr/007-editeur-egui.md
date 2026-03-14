# ADR-007 : egui comme framework UI de l'éditeur

- **Statut :** Acceptée
- **Date :** 2026-03-14
- **Concerne :** v0.1
- **Dépend de :** ADR-001 (Rust), ADR-002 (wgpu)

## Contexte

Le moteur embarque un éditeur visuel (scene editor, inspector, asset browser). Le framework UI de cet éditeur doit s'intégrer avec le pipeline de rendu wgpu, supporter l'itération rapide sur les features de l'éditeur, et idéalement tourner aussi en WASM pour une future version web de l'éditeur.

## Options considérées

### Dear ImGui (via cimgui/imgui-rs)
- **Pour :** le standard de l'industrie pour les outils de jeu. Utilisé en studio AAA (Blizzard, Epic). Écosystème d'extensions massif (node editors, color pickers, plots). Battle-tested. Documentation excellente.
- **Contre :** C++ natif, nécessite FFI depuis Rust (cimgui + imgui-rs). Build plus complexe. L'esthétique par défaut est "outil développeur" — nécessite un theming poussé pour les créatifs.

### egui
- **Pour :** Rust pur. S'intègre directement avec wgpu via `egui-wgpu`. Immediate-mode. API ergonomique et "Rustique". Support WASM natif (eframe). Développement actif, communauté croissante.
- **Contre :** écosystème de widgets plus petit que Dear ImGui. Moins de widgets spécialisés (node editor, courbe editor). Moins battle-tested en production. L'esthétique par défaut est aussi "développeur".

### Qt (via bindings Rust)
- **Pour :** le gold standard des applications desktop. Polished. Mature. Accessibilité. Tree views, tables, docking, rich text.
- **Contre :** dépendance massive. Licence complexe (LGPL ou commercial cher). L'intégration avec wgpu nécessite un widget de rendu custom. Crée une architecture éditeur séparé du runtime (mauvais pour la preview live). Overkill.

### Tauri
- **Pour :** UI web (HTML/CSS/JS) — l'écosystème UI le plus riche. Backend Rust. Binaires légers.
- **Contre :** l'éditeur est un process séparé du moteur. La communication éditeur↔moteur nécessite de l'IPC. La preview live est complexe (il faut streamer le rendu ou embarquer le moteur dans un canvas webview). Latence d'interaction.

## Décision

**egui.**

1. **L'éditeur vit dans le moteur.** C'est l'argument décisif. egui dessine directement dans notre pipeline wgpu via `egui-wgpu`. L'éditeur et le jeu partagent le même process, le même framebuffer, le même monde. Le bouton "Play" ne lance pas un process séparé — il active le game loop dans le même viewport. C'est ce qui permet la preview live instantanée et l'édition WYSIWYG.

2. **Rust pur.** Pas de build C++, pas de FFI, pas de wrappers unsafe. egui s'intègre naturellement dans l'architecture Rust du moteur. Les types sont partagés sans sérialisation.

3. **WASM natif.** egui tourne dans le navigateur (eframe/eframe-template). Quand on livrera l'éditeur web (post-v1.0), on n'aura pas à réécrire l'UI — elle compilera telle quelle en WASM.

4. **Vitesse d'itération.** Immediate-mode UI = ajouter un panneau ou un widget en quelques lignes. Pas de layout XML, pas de style sheets, pas de tree binding. On prototype des features d'éditeur en minutes.

**Compromis accepté :** l'esthétique egui par défaut est utilitaire. On investit dans le theming (fonts custom, couleurs, spacing) dès le MVP pour une apparence professionnelle. L'éditeur MVP sert les développeurs — un éditeur plus poli pour les créatifs (potentiellement Tauri-based) est une considération v0.3+.

## Conséquences

### Positives
- L'éditeur et le moteur sont un seul process = preview live instantanée
- Rust pur, intégration naturelle avec wgpu
- Itération très rapide sur les features d'éditeur
- Chemin vers un éditeur WASM (navigateur) sans réécriture

### Négatives
- Écosystème de widgets spécialisés plus petit (node editor, courbe editor devront être construits ou portés)
- Esthétique par défaut utilitaire (investissement theming nécessaire)
- Moins battle-tested que Dear ImGui pour les outils de production
- Les layouts complexes (docking avancé, multi-fenêtre) sont plus jeunes que dans ImGui
