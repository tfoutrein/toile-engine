# ADR-027 : Editeur de shaders visuel

- **Statut :** Acceptee
- **Date :** 2026-03-15
- **Concerne :** v0.4

## Contexte

Les effets visuels custom (distorsion, dissolve, outline, water, hologram) necessitent des shaders. Ecrire du WGSL a la main est une barriere pour les non-programmeurs et meme pour les developpeurs habitues aux outils visuels. Un editeur de shaders par graphe de noeuds (comme Shader Graph de Unity ou le Visual Shader de Godot) permet de creer des effets sans code.

## Decision

**Editeur de shaders visuel par graphe de noeuds, integre a l'editeur egui, compilant en WGSL.**

### Types de noeuds

| Categorie | Noeuds |
|-----------|--------|
| **Input** | UV, Screen UV, Time, Sprite Color, Sprite Alpha, Custom Uniform |
| **Texture** | Sample Texture, Sample Normal Map |
| **Math** | Add, Subtract, Multiply, Divide, Power, Abs, Fract, Mod, Clamp, Lerp, Step, Smoothstep |
| **Trigonometrie** | Sin, Cos, Atan2 |
| **Vecteur** | Split (RGBA), Combine, Dot, Cross, Normalize, Length, Distance |
| **Bruit** | Perlin, Simplex, Voronoi, Value Noise |
| **Forme SDF** | Circle, Box, Line, Union, Subtraction, Smooth Union |
| **Couleur** | HSV to RGB, RGB to HSV, Brightness, Contrast, Saturation, Tint |
| **Effet** | Distortion (UV offset), Dissolve (noise threshold), Outline (edge detect), Pixelate |
| **Output** | Fragment Color (RGBA) |

### Format de serialisation

Les graphes sont sauvegardes en JSON (`.shader.json`) :
```json
{
  "name": "dissolve",
  "nodes": [
    { "id": 1, "type": "Time", "position": [100, 200] },
    { "id": 2, "type": "Perlin", "position": [300, 200], "params": { "scale": 10.0 } },
    { "id": 3, "type": "Step", "position": [500, 200] },
    { "id": 4, "type": "FragmentColor", "position": [700, 200] }
  ],
  "edges": [
    { "from": [1, "out"], "to": [2, "offset"] },
    { "from": [2, "value"], "to": [3, "edge"] },
    { "from": [3, "out"], "to": [4, "alpha"] }
  ]
}
```

### Compilation

Le graphe est compile en WGSL par traversee topologique :
1. Trier les noeuds par dependances
2. Generer une variable WGSL par sortie de noeud
3. Injecter les fonctions de bruit/SDF comme fonctions utilitaires
4. Produire un fragment shader compatible avec le pipeline sprite existant

### Integration editeur

- Panneau egui avec canvas de graphe (zoom, pan, selection)
- Connexion par drag & drop entre ports
- Preview live du shader sur un sprite dans la scene
- Bouton "Export WGSL" pour utilisation avancee

### MCP

- `create_shader_from_description` — l'IA decrit l'effet, le moteur genere un graphe
- `list_shaders` — lister les shaders du projet
- `apply_shader` — appliquer un shader a un sprite/entite

## Consequences

### Positives
- Les artistes et game designers creent des effets sans ecrire de code
- Le format JSON est AI-friendly (l'IA peut generer des graphes)
- La compilation en WGSL garantit les performances natives
- La preview live accelere l'iteration
- La bibliotheque de noeuds de bruit/SDF couvre la majorite des effets 2D courants

### Negatives
- Implementer un editeur de graphe dans egui est complexe (layout, connections, zoom)
- Le compilateur graphe-vers-WGSL doit gerer les cas d'erreur (cycles, types incompatibles)
- Les shaders generes ne sont pas aussi optimises que du WGSL ecrit a la main
- Maintenance de la bibliotheque de noeuds au fil des versions
