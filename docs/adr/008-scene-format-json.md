# ADR-008 : JSON + JSON Schema comme format de scène

- **Statut :** Acceptée
- **Date :** 2026-03-14
- **Concerne :** v0.1

## Contexte

Le format de fichier des scènes (niveaux, layouts) est un choix structurant qui impacte : la compatibilité avec le version control (Git), la génération par IA (LLMs), l'ergonomie d'édition manuelle, la performance de chargement, et l'extensibilité.

## Options considérées

### Format custom binaire
- **Pour :** chargement le plus rapide possible (mmap + cast). Taille minimale. Pas de parsing.
- **Contre :** illisible par un humain. Impossible à diff/merger dans Git. Impossible à générer par un LLM. Nécessite un outil dédié pour chaque opération. Le pire choix pour l'ouverture et la transparence.

### Godot .tscn (INI-like custom)
- **Pour :** lisible par un humain. Prouvé par Godot. Relativement compact.
- **Contre :** format custom non-standard. Pas de validation par schéma. Pas de parser standard (il faut écrire le sien). Pas trivial à générer par un LLM (les LLMs sont meilleurs en JSON/YAML).

### YAML
- **Pour :** lisible, supporte les commentaires, anchors/aliases pour la déduplication. Plus concis que JSON (moins de bruit syntaxique).
- **Contre :** la spécification YAML est notoirement large et piégeuse (problèmes de whitespace, types implicites). Parsers complexes. Les LLMs font plus d'erreurs de syntaxe YAML que JSON. Pas de JSON Schema natif.

### JSON + JSON Schema
- **Pour :** format universel, parsers dans chaque langage. JSON Schema permet la validation stricte et sert de documentation machine-readable. Les LLMs génèrent du JSON avec un taux d'erreur très faible (surtout avec la contrainte de schéma). Diff-friendly dans Git (une propriété par ligne). Outillage massif (jq, VS Code, etc.).
- **Contre :** verbeux (accolades, guillemets). Pas de commentaires natifs. Plus volumineux qu'un format binaire.

### FlatBuffers / MessagePack
- **Pour :** compact, rapide, schema-based. FlatBuffers est zero-copy.
- **Contre :** binaire = même problèmes que le format custom pour Git, LLMs, et édition manuelle. Utile pour la distribution (v1.0 .pak) mais pas pour le format de développement.

## Décision

**JSON avec validation JSON Schema.**

C'est **non-négociable** pour le positionnement AI-native de Toile.

1. **Les LLMs génèrent du JSON avec fiabilité.** Les modèles de langage sont entraînés massivement sur du JSON. Avec une contrainte de schéma (output-constrained generation), le taux de validité est quasi 100%. Aucun autre format texte n'offre cette fiabilité pour la génération automatique.

2. **Le schéma EST la documentation.** Le JSON Schema décrit chaque champ, son type, sa valeur par défaut, ses contraintes, et sa description. Un LLM qui lit le schéma comprend exactement comment générer une scène valide. C'est de la documentation machine-readable et humaine-readable en un seul artifact.

3. **Git-friendly.** JSON formaté (une propriété par ligne) produit des diffs propres et des merges résolvables. Les fichiers .meta (Unity) et les formats binaires (GameMaker) sont le cauchemar du version control — la recherche identifie ceci comme un pain point transversal à tous les moteurs.

4. **Universel.** Chaque langage a un parser JSON. Les outils de manipulation (jq, VS Code, éditeurs web) sont omniprésents. Les scènes Toile sont exploitables par n'importe quel outil tiers.

**Pour la distribution (v1.0)**, les scènes JSON sont converties en format binaire compact (.pak) par le pipeline d'assets. Le JSON est le format de développement. Le binaire est le format de shipping. Les deux coexistent.

## Format de scène proposé

```json
{
  "$schema": "https://toile-engine.dev/schemas/scene-v1.json",
  "scene": {
    "name": "forest_level",
    "size": { "width": 800, "height": 600 },
    "background": "#1a3a2a",
    "entities": [
      {
        "name": "player",
        "layer": 10,
        "components": {
          "transform": { "x": 100, "y": 300, "rotation": 0, "scale_x": 1, "scale_y": 1 },
          "sprite": { "asset": "sprites/hero.png", "width": 32, "height": 32 },
          "animator": { "default_animation": "idle", "animations_asset": "sprites/hero.json" },
          "collider": { "type": "aabb", "width": 28, "height": 30, "offset_x": 2, "offset_y": 2 },
          "script": { "path": "scripts/player.lua" }
        }
      }
    ],
    "tilemaps": [
      {
        "name": "terrain",
        "layer": 0,
        "source": "maps/forest.json",
        "format": "tiled"
      }
    ]
  }
}
```

Principes :
- **Références nommées** (`"player"`) plutôt que des IDs numériques
- **Structure plate** : entités comme liste, composants comme objet plat
- **Chemins d'assets relatifs** au projet
- **`$schema`** référence le schéma pour validation et autocomplétion IDE

## Conséquences

### Positives
- Génération par LLM fiable avec contrainte de schéma
- Le schéma sert de documentation machine et humaine
- Diffs Git propres et merges résolvables
- Outillage universel (jq, VS Code, parsers dans chaque langage)
- Validation automatique avant chargement dans le moteur

### Négatives
- Plus verbeux qu'un format binaire ou custom (mitigé : le format dev n'a pas besoin d'être compact)
- Pas de commentaires dans JSON standard (mitigé : le champ `"_comment"` est une convention courante, ou utilisation de JSON5 en phase d'édition)
- Performance de parsing plus lente qu'un format binaire (mitigé : acceptable pour le chargement de scènes, et le .pak binaire est utilisé pour la distribution)
