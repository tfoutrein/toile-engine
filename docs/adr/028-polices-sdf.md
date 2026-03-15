# ADR-028 : Polices SDF (Signed Distance Field)

- **Statut :** Acceptee
- **Date :** 2026-03-15
- **Concerne :** v0.4

## Contexte

Le rendu de texte actuel (fontdue, rasterisation bitmap) produit du texte net a une taille fixe mais flou quand on zoom ou quand on change de taille. Les polices SDF/MSDF rendent le texte net a n'importe quelle echelle, avec un seul atlas de texture. C'est la technique standard des moteurs modernes (Unity TextMeshPro, Godot MSDF).

## Decision

**Rendu de texte MSDF (Multi-channel Signed Distance Field) avec generation d'atlas a l'import.**

### Technique MSDF

Contrairement au SDF classique (1 canal, bords arrondis sur les coins aigus), le MSDF utilise 3 canaux (RGB) pour encoder la distance au bord le plus proche. Le fragment shader reconstruit les bords nets en calculant `median(r, g, b)` et en appliquant un seuil.

### Pipeline

1. **Import** : a partir d'un fichier TTF/OTF, generer un atlas MSDF + fichier de metriques (glyph positions, advances, kerning). Outil : `msdf-atlas-gen` (execute au build) ou implementation Rust integree.
2. **Atlas** : texture RGB unique contenant tous les glyphes necessaires. Taille configurable (defaut 1024x1024). Regenere si de nouveaux caracteres sont demandes.
3. **Rendu** : quad par glyphe, fragment shader MSDF. Le seuil `screenPxRange` est calcule dynamiquement pour garantir des bords nets quelle que soit la taille.

### Effets texte

Les effets sont des parametres du shader MSDF, pas du code supplementaire :

| Effet | Parametre |
|-------|-----------|
| **Outline** | `outline_width`, `outline_color` — bord colore autour du texte |
| **Shadow** | `shadow_offset`, `shadow_color`, `shadow_softness` — ombre portee |
| **Glow** | `glow_radius`, `glow_color` — halo lumineux |
| **Gradient** | `gradient_top`, `gradient_bottom` — degradee vertical |

### API

```rust
// Charger une police MSDF
let font = ctx.load_msdf_font("assets/fonts/Roboto.ttf");

// Dessiner du texte avec effets
ctx.draw_text_msdf("Score: 1000", pos, font, TextStyle {
    size: 24.0,
    color: Color::WHITE,
    outline: Some(Outline { width: 2.0, color: Color::BLACK }),
    shadow: Some(Shadow { offset: Vec2::new(2.0, -2.0), color: Color::rgba(0, 0, 0, 0.5) }),
    ..default()
});
```

### Compatibilite

L'API `draw_text` existante (fontdue, bitmap) reste disponible pour la compatibilite et les cas ou la rasterisation bitmap est suffisante. `draw_text_msdf` est la nouvelle API recommandee.

## Consequences

### Positives
- Texte net de 8px a 200px avec un seul atlas
- Effets (outline, shadow, glow) sans passes supplementaires
- Performance identique au texte bitmap (1 draw call, meme pipeline sprite)
- Standard de l'industrie (compatible avec les generateurs MSDF existants)

### Negatives
- L'atlas MSDF doit etre genere au build ou au premier chargement (quelques secondes)
- Les polices tres decoratives (script, handwriting) peuvent mal encoder en MSDF
- Necessite un fragment shader dedie (pas le meme que le sprite shader)
- `msdf-atlas-gen` est un outil externe C++ (ou alternative Rust a developper)
