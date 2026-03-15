# ADR-018 : Export web WASM/WebGL2 (repoussé post-v1.0)

- **Statut :** Reportée (voir ADR-031)
- **Date :** 2026-03-14
- **Concerne :** v0.2 (préparation), ~~v0.5~~ post-v1.0 (livraison)
- **Note :** Le scope v0.5 a été réaffecté à "Complete Editor MVP" (ADR-031). L'export web reste pertinent mais la priorité est de connecter toutes les briques existantes en un éditeur complet.

## Contexte

L'export web est crucial pour la distribution (itch.io, Newgrounds, partage par URL). C'est prévu pour la v0.5 mais les décisions architecturales doivent être prises dès maintenant pour ne pas accumuler de dette technique incompatible avec le web.

## Décision

**Préparer le terrain pour WASM dès la v0.2 en respectant les contraintes web.**

### Contraintes à respecter dès maintenant

1. **Pas de threads bloquants** dans le game loop. Le web est single-threaded (sauf SharedArrayBuffer avec headers spécifiques). Le chargement async (ADR-016) doit avoir un fallback non-threadé.

2. **wgpu cible WebGPU nativement** (Chrome 121+). Fallback WebGL2 via le backend OpenGL de wgpu. Nos shaders WGSL sont déjà compatibles.

3. **Pas de filesystem**. Les assets doivent être loadables via HTTP fetch. Le système d'assets doit abstraire la source (fichier local OU URL).

4. **Audio** : WebAudio API nécessite un geste utilisateur pour démarrer. kira utilise cpal qui ne supporte pas le web — il faudra un backend audio alternatif pour le web.

5. **Taille d'export** : cible < 3 MB pour un hello world. Tree-shaking, LTO, `wasm-opt` seront nécessaires.

### Actions v0.2

- S'assurer que tous les modules compilent en `--target wasm32-unknown-unknown` (même sans runtime)
- Éviter les dépendances qui ne supportent pas WASM (vérifier chaque nouveau crate)
- Abstraire l'accès aux fichiers derrière un trait `AssetSource` (fichier local vs HTTP)

## Conséquences

### Positives
- Pas de surprise au moment de la v0.5
- Les contraintes web influencent positivement l'architecture (pas de blocking, abstraction I/O)

### Négatives
- Certaines features sont plus complexes à implémenter avec les contraintes web en tête
- L'audio web nécessitera un backend séparé (pas kira)
