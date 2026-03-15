# ADR-026 : Ombres 2D

- **Statut :** Acceptee
- **Date :** 2026-03-15
- **Concerne :** v0.4

## Contexte

Les ombres completent l'eclairage en donnant du volume et de la lisibilite a la scene. Sans ombres, les lumieres eclairent a travers les murs et les objets, ce qui casse l'immersion. La technique standard pour les ombres 2D est la shadow map 1D.

## Decision

**Shadow maps 1D par source de lumiere, avec support des soft shadows.**

### Technique : 1D Shadow Map

Pour chaque lumiere qui projette des ombres :

1. **Occlusion pass** : rendre les occluders (sprites marques comme `cast_shadow = true`) dans une texture temporaire autour de la lumiere.
2. **Ray marching** : pour chaque angle (resolution angulaire configurable, ex: 360 ou 720 rayons), marcher depuis la lumiere vers l'exterieur et trouver la distance du premier occluder. Stocker dans une texture 1D (largeur = resolution angulaire, 1 pixel de haut).
3. **Shadow sampling** : dans le light pass, pour chaque fragment, calculer l'angle et la distance par rapport a la lumiere. Comparer avec la shadow map : si distance fragment > distance occluder, le fragment est dans l'ombre.

### Soft shadows

- **PCF (Percentage Closer Filtering)** : sampler plusieurs voisins dans la shadow map et moyenner. Cree des bords adoucis.
- **Penombre** : falloff progressif base sur la distance a l'occluder. Plus l'ombre est loin de l'objet qui la projette, plus elle est diffuse.

### Configuration

```rust
struct ShadowConfig {
    resolution: u32,       // rayons par lumiere (defaut: 360)
    soft_shadows: bool,    // PCF active (defaut: true)
    pcf_samples: u32,      // nombre d'echantillons PCF (defaut: 5)
    max_shadow_casters: u32, // limite par scene (defaut: 256)
}
```

### Entites occluders

Les entites sont marquees comme occluders dans leur definition :
```json
{ "cast_shadow": true }
```

Par defaut, seules les entites avec `cast_shadow: true` bloquent la lumiere. Les tilemaps peuvent aussi etre marquees comme occluders (par layer).

### Performance

- Shadow map 1D : 1 draw call par lumiere projetant des ombres
- Resolution 360 = 360 pixels de texture par lumiere
- Cible : 10 lumieres avec ombres simultanees a 60 FPS
- Lumieres sans ombres : cout zero (pas de shadow pass)

## Consequences

### Positives
- Les ombres ajoutent enormement de profondeur visuelle avec peu d'effort artistique
- La technique 1D shadow map est bien documentee et performante en 2D
- Les soft shadows evitent l'aspect brut des ombres binaires
- Compatible avec le systeme d'eclairage (ADR-025)

### Negatives
- Resolution angulaire limitee : artefacts visibles sur les ombres lointaines a faible resolution
- Les occluders doivent etre marques explicitement (pas automatique)
- Le ray marching est couteux si beaucoup de lumieres projettent des ombres
- Les ombres 1D ne supportent pas les occluders semi-transparents
