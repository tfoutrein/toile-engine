# ADR-029 : Post-processing

- **Statut :** Acceptee
- **Date :** 2026-03-15
- **Concerne :** v0.4

## Contexte

Les effets de post-processing (bloom, CRT, vignette, screen shake) donnent une identite visuelle forte aux jeux 2D. Ils sont appliques apres le rendu de la scene, sur le framebuffer final. Un pipeline configurable permet aux createurs de combiner des effets sans code.

## Decision

**Pipeline de post-processing configurable avec rendu offscreen et chaine d'effets.**

### Architecture

1. **Rendu offscreen** : la scene est rendue dans une texture intermediaire (au lieu du swapchain directement).
2. **Chaine d'effets** : chaque effet est un full-screen pass qui lit la texture precedente et ecrit dans la suivante. Ping-pong entre deux framebuffers.
3. **Composite final** : le dernier effet ecrit dans le swapchain.

### Effets inclus

| Effet | Description | Parametres |
|-------|-------------|-----------|
| **Bloom** | Halo lumineux autour des zones brillantes | `threshold`, `intensity`, `radius`, `iterations` |
| **CRT** | Simulation ecran cathodique | `scanline_intensity`, `curvature`, `chromatic_aberration`, `vignette` |
| **Color Grading** | Application d'une LUT de couleur | `lut_texture` (PNG 256x16 strip), `intensity` |
| **Vignette** | Assombrissement des bords de l'ecran | `intensity`, `smoothness`, `color` |
| **Screen Shake** | Tremblement de camera base sur un systeme de trauma | `trauma` (0-1), `decay`, `frequency`, `amplitude` |
| **Pixelate** | Reduction de resolution (retro) | `pixel_size` |
| **Aberration chromatique** | Decalage RGB sur les bords | `intensity`, `offset` |

### Bloom (detail)

1. **Threshold** : extraire les pixels au-dessus du seuil de luminosite.
2. **Downsample** : reduire la resolution par 2 sur N iterations (defaut 5).
3. **Upsample + blur** : remonter en resolution avec un blur gaussien a chaque etape.
4. **Composite** : additionner le bloom au rendu original.

### Screen Shake (detail)

Systeme base sur le concept de "trauma" (0 = calme, 1 = tremblement maximum) :
- Le trauma decroit automatiquement avec le temps (`trauma -= decay * dt`)
- L'offset de camera est `amplitude * trauma^2 * noise(time * frequency)`
- Les axes X, Y et la rotation sont perturbes independamment avec des frequences de bruit differentes

### Configuration par scene

```rust
scene.post_processing = PostProcessingStack {
    effects: vec![
        PostEffect::Bloom { threshold: 0.8, intensity: 0.5, radius: 4, iterations: 5 },
        PostEffect::Vignette { intensity: 0.3, smoothness: 0.5, color: Color::BLACK },
    ],
    enabled: true,
};
```

### Performance

Cible : **< 0.5 ms overhead a 1080p** pour CRT + Bloom.

Optimisations :
- Bloom downsample a des resolutions tres petites (derniere iteration = ~32px)
- Pas de ping-pong si un seul effet
- Effets desactivables individuellement et par scene

### MCP

- `set_post_processing_stack` — configurer la chaine d'effets avec parametres
- `add_post_effect` — ajouter un effet a la chaine
- `remove_post_effect` — retirer un effet
- `take_screenshot` avec `with_post_processing=true/false` pour comparaison A/B

## Consequences

### Positives
- Identite visuelle forte avec zero effort de code (configuration JSON/API)
- Le CRT effect donne instantanement un look retro professionnel
- Le bloom ajoute de la richesse visuelle (combine avec l'eclairage 2D)
- Screen shake ameliore le game feel
- Pipeline extensible pour ajouter de nouveaux effets plus tard

### Negatives
- Le rendu offscreen double la memoire de framebuffer
- Chaque effet est un full-screen pass (cout proportionnel a la resolution)
- Le bloom avec beaucoup d'iterations est couteux sur les GPU bas de gamme
- Les effets de post-processing peuvent masquer des problemes artistiques sous-jacents
