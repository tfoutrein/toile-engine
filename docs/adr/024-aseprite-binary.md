# ADR-024 : Import direct Aseprite binaire (.ase/.aseprite)

- **Statut :** Acceptée
- **Date :** 2026-03-15
- **Concerne :** v0.3

## Contexte

La v0.1 importe les animations Aseprite via le format JSON export (l'utilisateur doit exporter manuellement depuis Aseprite ou via CLI). L'import direct du format binaire `.ase`/`.aseprite` élimine cette étape et permet un workflow plus fluide : modifier dans Aseprite, hot-reload dans Toile.

## Décision

**Parser le format binaire Aseprite directement dans toile-assets.**

### Format .ase
Le format est bien documenté (spec officielle sur le repo Aseprite) :
- Header (128 bytes) : taille, nombre de frames, dimensions, profondeur de couleur, palette
- Frames : chaque frame contient des chunks
- Chunks : layer, cel (pixel data), tags, palette, slices, user data
- Pixel data : DEFLATE-compressé dans les cels

### Données extraites

| Donnée | Usage |
|--------|-------|
| Frames + durées | Animation frame-by-frame |
| Tags (frameTags) | Clips d'animation nommés (idle, run, jump) |
| Layers | Composition de sprites (optionnel) |
| Palette | Palette de couleurs pour palette-swapping |
| Slices | Régions nommées (hitbox, pivot) |
| Cels | Pixels bruts par frame par layer |

### Implémentation
- Parser le header et les frames séquentiellement
- Décompresser les cels DEFLATE avec `flate2`
- Composer les frames en un atlas de sprite sheet (comme l'export JSON le fait)
- Retourner un `SpriteSheet` identique à celui produit par `load_aseprite_json`

### Alternative : utiliser un crate existant
- `asefile` crate existe mais peut ne pas être à jour avec le format le plus récent
- Parser nous-mêmes donne un contrôle total et évite une dépendance

## Conséquences

### Positives
- Workflow artiste simplifié : sauvegarder dans Aseprite → hot-reload dans Toile
- Accès aux données supplémentaires (slices, palette) non disponibles dans l'export JSON
- Pas besoin de l'exécutable Aseprite CLI pour l'export

### Négatives
- Le format binaire est plus complexe à parser que le JSON
- Doit gérer les modes de couleur (indexed, RGBA, grayscale)
- Les cels compressés nécessitent `flate2` comme dépendance
- Maintenance du parser si Aseprite change le format (rare — le format est stable)
