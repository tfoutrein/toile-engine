# ADR-017 : Peinture de tilemap dans l'éditeur

- **Statut :** Acceptée
- **Date :** 2026-03-14
- **Concerne :** v0.2

## Contexte

La v0.1 charge les tilemaps depuis des fichiers Tiled JSON, mais tout l'édition de niveaux se fait dans l'éditeur externe Tiled. Pour le "missing middle" (accessibilité de Construct + puissance de Godot), l'éditeur Toile doit offrir ses propres outils de peinture de tilemap.

## Décision

**Intégrer un éditeur de tilemap dans le viewport de l'éditeur egui.**

### Outils de peinture

| Outil | Description |
|-------|-------------|
| **Brush** | Peindre un tile à la position de la souris. Clic + drag pour peindre en continu. |
| **Eraser** | Supprimer un tile (mettre le GID à 0). |
| **Fill** | Flood fill — remplir une zone contiguë avec le tile sélectionné. |
| **Rectangle** | Dessiner un rectangle rempli de tiles. |
| **Picker** | Clic sur un tile dans le viewport pour sélectionner son type. |

### Palette de tiles
- Panneau affichant le tileset sous forme de grille
- Clic pour sélectionner le tile à peindre
- Preview du tile sélectionné sur le curseur

### Multi-layers
- Chaque couche de tiles est éditable indépendamment
- Sélecteur de couche active dans le panneau
- Visibilité/verrouillage par couche

### Auto-tiling (v0.2 stretch, v0.3 si pas le temps)
- Règles bitmask pour auto-sélection des variantes de tiles (bords, coins)
- Compatible avec les conventions Wang tiles et Tiled auto-tile

### Format
Les tilemaps éditées dans l'éditeur sont sauvegardées en **JSON Toile** (extension du format SceneData), pas en format Tiled. L'import Tiled reste pour les assets créés dans Tiled.

## Conséquences

### Positives
- Les level designers peuvent créer des niveaux sans quitter l'éditeur Toile
- Workflow intégré : peindre des tiles + placer des entités dans le même outil
- Réduction de la dépendance à des outils externes

### Négatives
- Scope significatif (brush, fill, layers, palette)
- L'auto-tiling est complexe et peut glisser vers la v0.3
- Deux formats de tilemap coexistent (Tiled import + Toile natif)
