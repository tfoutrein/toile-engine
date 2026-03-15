# ADR-030 : Editeur de particules

- **Statut :** Acceptee
- **Date :** 2026-03-15
- **Concerne :** v0.4

## Contexte

Le systeme de particules (v0.2) fournit 8 presets et un EmitterConfig programmatique. Mais creer des effets custom necessite de modifier du code Rust et de recompiler. Un editeur visuel avec des widgets de courbe et de gradient permet de tweaker les parametres en temps reel, ce qui est essentiel pour les artistes VFX.

## Decision

**Panneau editeur de particules dans l'editeur egui, avec courbes bezier, gradients, et preview live.**

### Widgets

| Widget | Usage |
|--------|-------|
| **Courbe bezier** | Editer la variation temporelle d'un parametre (taille, vitesse, opacite sur la duree de vie). Points de controle draggables avec handles bezier. |
| **Gradient** | Editer la couleur au fil du temps (start color -> end color avec stops intermediaires). Bar coloree cliquable pour ajouter/deplacer les stops. |
| **Gizmo de forme** | Visualiser et editer la forme d'emission (point, cercle, rectangle, cone) directement dans la viewport. Handles draggables pour rayon, largeur, angle. |
| **Slider range** | Min/max pour les parametres avec variation aleatoire (ex: taille initiale entre 2 et 5). |

### Parametres editables

| Categorie | Parametres |
|-----------|-----------|
| **Emission** | Rate, burst count, burst interval, max particles |
| **Forme** | Point, Circle (rayon), Rectangle (w, h), Cone (angle, distance) |
| **Lifetime** | Min/max duree de vie |
| **Velocite** | Direction, spread angle, vitesse min/max |
| **Taille** | Taille initiale (range), courbe taille/temps |
| **Couleur** | Gradient couleur/temps, opacite/temps |
| **Rotation** | Rotation initiale (range), vitesse de rotation (range) |
| **Gravite** | Vecteur gravite, facteur par emetteur |
| **Physics** | Friction, bounce (experimental) |
| **Sub-emitters** | Emetteur declenche a la creation, mort, ou collision d'une particule |

### Sub-emitters

Un emetteur peut declencher un autre emetteur quand une particule :
- **Nait** (on_create) — trainee, etincelles a l'emission
- **Meurt** (on_death) — explosion secondaire, fumee post-impact
- **Collide** (on_collision) — rebond avec particules d'impact

Les sub-emitters referencent un EmitterConfig par nom. Profondeur maximale : 2 niveaux.

### Format de sauvegarde

Les configurations de particules sont sauvegardees en JSON (`.particles.json`) :
```json
{
  "name": "explosion",
  "emitter": {
    "rate": 0,
    "burst_count": 50,
    "shape": { "type": "Circle", "radius": 5.0 },
    "lifetime": [0.3, 0.8],
    "speed": [100, 300],
    "size_curve": [[0, 5], [0.3, 8], [1, 0]],
    "color_gradient": [
      { "t": 0, "color": [255, 200, 50, 255] },
      { "t": 0.5, "color": [255, 80, 20, 255] },
      { "t": 1, "color": [60, 20, 10, 0] }
    ],
    "gravity": [0, -200],
    "sub_emitters": {
      "on_death": "smoke_puff"
    }
  }
}
```

### Integration

- **Editeur** : panneau dedie accessible depuis la barre de menu ou clic droit sur un emetteur dans la scene
- **Preview live** : les particules s'emettent en temps reel dans le panneau preview pendant l'edition
- **Scene** : les emetteurs sont des entites comme les autres, positionnables dans l'editeur de scene
- **MCP** : `create_particle_emitter`, `update_particle_config` pour creation IA

## Consequences

### Positives
- Les artistes VFX peuvent tweaker les effets sans recompiler
- La preview live accelere drastiquement l'iteration
- Les courbes et gradients offrent un controle precis que les presets ne permettent pas
- Les sub-emitters permettent des effets complexes (explosion -> fumee -> etincelles)
- Le format JSON est manipulable par l'IA via MCP

### Negatives
- Implementer des courbes bezier editables dans egui est non trivial
- Les sub-emitters ajoutent de la complexite au systeme de particules CPU
- Le nombre de parametres peut etre intimidant pour les debutants
- La performance des sub-emitters doit etre surveillee (cascade de particules)
