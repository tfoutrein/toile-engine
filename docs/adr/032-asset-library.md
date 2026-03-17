# ADR-032 : Asset Library — crate autonome pour import, classification et browsing d'assets

- **Statut :** Acceptee
- **Date :** 2026-03-17
- **Concerne :** v0.5 (MVP), v1.0 (enrichi), v1.5+ (ecosysteme)

## Contexte

Toile Engine a besoin d'une bibliotheque d'assets pour importer, classifier, indexer et parcourir les game packs 2D (sprites, tilesets, tilemaps, backgrounds, audio, fonts, UI, VFX, props). Les packs viennent de sources diverses (itch.io, Kenney, CraftPix) avec des structures et conventions tres differentes. Il n'existe actuellement aucun moyen d'importer un pack et d'utiliser son contenu dans l'editeur ou le runtime.

## Decision

**Creer `toile-asset-library` comme nouveau crate workspace** avec :
- Un **coeur logique** (types, scanner, classifier, manifest, thumbnails) sans dependance UI
- Un **widget egui** (`AssetBrowserPanel`) embeddable dans l'editeur
- Un **binaire standalone** pour les artistes et le tooling CI

Le crate depend de `toile-assets` (parsers Aseprite/LDtk existants) et `toile-scene` (conversion vers EntityData), mais pas l'inverse.

## Architecture

```
toile-asset-library/src/
  lib.rs              — API publique
  types.rs            — AssetType, ToileAsset, AssetManifest, metadonnees
  scanner.rs          — Scan recursif dossier/ZIP
  classifier.rs       — Classification auto (extension + path + heuristiques)
  manifest.rs         — Lire/ecrire toile-asset-manifest.json
  library.rs          — Index memoire, requetes, CRUD
  thumbnail.rs        — Generation thumbnails 128x128
  heuristics.rs       — Detection frame size, grille tileset, parallax
  importers/          — Parsers specifiques (Tiled JSON, audio headers, parallax)
  ui/                 — Widget egui AssetBrowserPanel (grille, detail, import)
  bin/main.rs         — App standalone
```

## Phasage

| Phase | Version | Contenu |
|-------|---------|---------|
| 1 | v0.5 | Core : types, scanner, classifier, manifest, thumbnails |
| 2 | v0.5 | UI : browser egui standalone, grille, filtres, import |
| 3 | v0.5 | Integration editeur : panneau embarque, drag & drop |
| 4 | v1.0 | Importeurs enrichis : TMX/TSX, parallax, audio complet |
| 5 | v1.0 | CLI + MCP : commandes assets |
| 6 | v1.5+ | Spine, Cocos2D, Starling, auto-tagging, pack registry |

## Consequences

### Positives
- Testable sans fenetre (coeur logique pur)
- CLI et MCP peuvent importer/requeter sans editeur
- Les artistes utilisent le browser standalone pour preparer les packs
- L'editeur recoit un panneau drop-in sans couplage architectural

### Negatives
- Un crate supplementaire a maintenir (acceptable, workspace a deja 17 crates)
- Les thumbnails doivent etre uploadees au GPU par l'hote (editeur ou standalone)
- Duplication possible de types Tiled entre toile-assets et toile-asset-library (mitigee par reutilisation)
