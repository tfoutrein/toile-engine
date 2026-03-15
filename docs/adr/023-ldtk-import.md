# ADR-023 : Import LDtk

- **Statut :** Acceptée
- **Date :** 2026-03-15
- **Concerne :** v0.3

## Contexte

LDtk (Level Designer Toolkit) est un éditeur de niveaux 2D moderne créé par Sébastien Bénard (créateur de Dead Cells). Il gagne en popularité comme alternative à Tiled grâce à son approche plus structurée des entités et son support d'auto-layers. La v0.1 supporte Tiled JSON. Ajouter LDtk élargit l'écosystème d'outils compatible.

## Décision

**Importer le format LDtk (.ldtk JSON) en complément de Tiled.**

### Données LDtk à parser

| Concept LDtk | Mapping Toile |
|---------------|---------------|
| **World** | Multi-scènes (chaque Level = une scène) |
| **Levels** | SceneData avec position dans le monde |
| **IntGrid layers** | Collision tiles (valeur entière = type de collision) |
| **Tile layers** | Tilemap visuelle (identique à Tiled) |
| **Entity layers** | Entités Toile (position, taille, propriétés custom) |
| **Auto-layers** | Tiles générées par règles (importées comme tiles statiques) |
| **Enums** | Mapping vers des tags ou propriétés d'entités |
| **Field definitions** | Propriétés custom sur les entités |

### Avantages de LDtk sur Tiled
- Entités typées avec des champs définis (pas juste des propriétés key-value)
- World view (disposition spatiale de plusieurs niveaux)
- Auto-layers (règles d'auto-tiling intégrées au fichier)
- Schéma auto-descriptif (le .ldtk contient les définitions de types)

## Conséquences

### Positives
- Les utilisateurs de LDtk peuvent utiliser Toile sans changer d'outil de level design
- Les entités LDtk sont plus structurées que les objets Tiled
- Le world layout permet des métroidvanias multi-niveaux

### Négatives
- Format complexe (le fichier .ldtk est large et contient beaucoup de métadonnées)
- Deux importeurs de tilemap à maintenir (Tiled + LDtk)
- Les auto-layers sont importées comme tiles statiques (pas de re-génération au runtime)
