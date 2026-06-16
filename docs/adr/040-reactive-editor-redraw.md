# ADR-040 : Redraw réactif de l'éditeur (anti-chauffe CPU)

- **Statut :** Acceptée — livrée
- **Date :** 2026-06-16
- **Concerne :** v0.5+ (toile-app, toile-editor)

## Contexte

L'`AppHandler` (boucle winit partagée par l'éditeur **et** les 24 examples)
appelait `window.request_redraw()` **inconditionnellement** à la fin de chaque
`RedrawRequested`. Couplé au présent vsync (Fifo), l'application rendait donc en
**continu** à la fréquence de l'écran (~120 Hz sur M2 Pro), même quand rien ne
changeait.

C'est correct pour un **jeu** (il simule chaque frame), mais l'**éditeur** a une
UI majoritairement statique : mesuré à **~27 % de CPU au repos** sur le simple
écran d'accueil → chauffe et ventilation « au bout d'un moment » (rapportée par
l'utilisateur, reproduite).

## Decision

Rendre la boucle **réactive pour l'éditeur, inchangée pour les jeux**, via un
unique point d'extension sur le trait `Game` :

```rust
fn redraw_after(&self) -> std::time::Duration { Duration::ZERO } // défaut
```

- `Duration::ZERO` (défaut) → `request_redraw()` chaque frame = rendu continu.
  **Tous les examples gardent ce comportement à l'octet près.**
- Sinon → `ControlFlow::WaitUntil(now + d)` : la boucle dort. `new_events`
  redessine quand le timer expire ; tout **input** (clavier/souris/molette/
  modificateurs) et un **resize** forcent un redraw immédiat.

L'éditeur (`EditorApp::redraw_after`) retourne :
- `ZERO` tant que **quelque chose anime** (splash, preview de sprite, analyse IA,
  réponse Copilot en vol) ;
- sinon le `repaint_delay` qu'**egui** réclame lui-même (récupéré depuis
  `FullOutput.viewport_output[ROOT]` dans `EguiOverlay`), **plafonné à 100 ms**.

Le plafond 100 ms garantit que les résultats asynchrones (logs de jeu, chargement
d'assets, réponses IA) remontent en ≤100 ms même sans input, sans avoir à câbler
un signal de repaint inter-thread.

## Consequences

### Positives
- Idle éditeur **~27 % → ~7 % CPU** (−73 %), donc bien moins de chauffe. Plein
  régime conservé pendant l'interaction (egui demande alors `repaint_delay`≈0) et
  l'animation. Responsivité input intacte (redraw immédiat sur événement).
- Jeux/examples **inchangés** (défaut `ZERO`).

### Negatives / honnêteté
- Au repos, egui peut réclamer ~30 FPS sur certains écrans (hover, curseur sur la
  fenêtre) → le plancher réel observé est ~7 %, pas ~2 %. Suffisant pour la
  chauffe ; affiner l'accueil serait du polish à rendement décroissant.
- Résultats async plafonnés à 100 ms de latence (imperceptible ; les chemins
  vraiment temps réel — stream IA — passent en continu via le flag dédié).

## Validation
- `cargo build --workspace --all-targets` + clippy clean ; 23 tests éditeur.
- Mesure empirique `top` avant/après (27 % → 7 % au repos).
- Revue adversariale du diff (boucle de fondation) : verdict **SOUND** ; le seul
  risque relevé (stream Copilot bridé à 10 FPS) corrigé en ajoutant `ai_loading`
  à la condition d'animation.
