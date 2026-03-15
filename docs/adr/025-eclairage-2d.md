# ADR-025 : Eclairage 2D

- **Statut :** Acceptee
- **Date :** 2026-03-15
- **Concerne :** v0.4

## Contexte

L'eclairage 2D est le differenciateur visuel le plus impactant pour les jeux 2D modernes (Hollow Knight, Dead Cells, Celeste). Les sprites a plat manquent de profondeur; des lumieres dynamiques avec normal maps creent un rendu professionnel. Aucun moteur 2D mid-range n'offre un pipeline eclairage + normal maps simple d'utilisation.

## Decision

**Systeme d'eclairage 2D avec normal maps et ombres, integre au pipeline de rendu sprite.**

### Types de lumiere

| Type | Parametres |
|------|-----------|
| **Point light** | position, couleur, rayon, intensite, falloff |
| **Directional** | direction, couleur, intensite (soleil/lune) |
| **Spot light** | position, direction, angle, rayon, couleur, intensite |

### Normal maps

Convention de nommage : `hero.png` + `hero_n.png` (auto-detecte). Si pas de normal map, le sprite est eclaire de maniere uniforme (flat normal = (0, 0, 1)).

### Pipeline de rendu

1. **G-Buffer pass** : rendre les sprites dans un framebuffer avec albedo (RGBA) + normal (RG, reconstruit B). Pas de MRT si non supporte : deux passes separees.
2. **Light pass** : pour chaque lumiere, rendre un quad couvrant son rayon. Le fragment shader lit albedo + normal et calcule la contribution lumineuse (diffuse Lambert + optionnel specular Blinn-Phong).
3. **Shadow pass** (optionnel) : shadow map 1D par point/spot light. Cast des rayons depuis la lumiere vers les occluders. Resultat utilise pour masquer la lumiere.
4. **Composite** : ambiant + somme des contributions lumineuses.

### Performance

Cible : **50+ lumieres dynamiques a 60 FPS** sur GPU integre.

Optimisations :
- Light culling par grille spatiale (ne rendre que les lumieres visibles)
- Lumieres sans ombres : pas de shadow pass
- LOD lumiere : lumieres lointaines/petites ignorees
- Batch les lumieres dans un compute pass si WebGPU disponible

### Configuration par scene

```rust
scene.lighting = LightingConfig {
    ambient_color: Color::rgb(0.1, 0.1, 0.15),
    ambient_intensity: 0.3,
    enabled: true,
};
```

### API

```rust
// Creer une lumiere
let light = ctx.add_light(PointLight {
    position: Vec2::new(100.0, 50.0),
    color: Color::rgb(1.0, 0.9, 0.7),
    radius: 200.0,
    intensity: 1.5,
    cast_shadows: true,
});

// Charger un sprite avec normal map (auto-detect)
let tex = ctx.load_texture("hero.png"); // charge aussi hero_n.png si present
```

## Consequences

### Positives
- Qualite visuelle pro avec peu d'effort pour l'utilisateur (convention de nommage)
- Normal maps standard compatibles avec les assets existants (Aseprite, Laigter, SpriteIlluminator)
- Les lumieres sans ombres sont peu couteuses en perf
- Differenciateur fort par rapport a Construct/RPG Maker

### Negatives
- Necessite un framebuffer offscreen (render-to-texture) : complexite GPU accrue
- Les shadow maps 1D sont limitees (pas de penombre realiste sur grandes surfaces)
- Les normal maps sont un asset supplementaire a creer ou generer
- Le deferred lighting ne fonctionne pas bien avec la transparence (sprites semi-transparents)
