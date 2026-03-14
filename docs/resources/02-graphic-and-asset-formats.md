# Formats Graphiques & Assets pour Moteur 2D

## Table des matières
1. [Formats d'images](#1-formats-dimages)
2. [Formats de sprite sheets / atlas](#2-formats-de-sprite-sheets--atlas)
3. [Formats de tile maps](#3-formats-de-tile-maps)
4. [Formats d'animation](#4-formats-danimation)
5. [Formats vectoriels](#5-formats-vectoriels)
6. [Formats de polices](#6-formats-de-polices)
7. [Formats audio](#7-formats-audio)
8. [Formats de scènes / niveaux](#8-formats-de-scènes--niveaux)
9. [Pipeline d'assets](#9-pipeline-dassets)
10. [Priorités MVP](#10-priorités-mvp)

---

## 1. Formats d'images

| Format | Alpha | Lossless | Complexité parsing | Taille fichier | Priorité MVP |
|--------|-------|----------|-------------------|---------------|-------------|
| **PNG** | Oui | Oui | Moyenne | Moyenne | **Indispensable** |
| **QOI** | Oui | Oui | Triviale | Moyenne | Haute |
| **BMP** | Partiel | Oui | Triviale | Énorme | Basse |
| **TGA** | Oui | Oui/RLE | Simple | Grande | Basse |
| **WebP** | Oui | Les deux | Complexe | Petite | Moyenne |
| **JPEG** | Non | Non | Complexe | Petite | Basse |
| **PSD** | Oui | Oui | Extrême | Grande | Ignorer |
| **AVIF** | Oui | Les deux | Extrême | Minuscule | Ignorer |

### PNG — Le standard incontournable
- 32-bit RGBA, compression lossless (DEFLATE)
- Support universel dans tous les outils et bibliothèques
- Bibliothèques : stb_image, SDL_image, lodepng

### QOI — Le format idéal pour le runtime
- Spécification tenant sur une page (~300 lignes de C pour un codec complet)
- **20-50x plus rapide** à encoder et **3-4x plus rapide** à décoder que PNG
- Idéal comme format interne/runtime : convertir les PNG en QOI dans le pipeline d'assets

### Formats à ignorer pour le MVP
- **PSD** : trop complexe, les artistes doivent exporter en PNG
- **AVIF** : codec trop lourd, ratio complexité/bénéfice mauvais
- **BMP** : trop volumineux, pas de valeur ajoutée

---

## 2. Formats de sprite sheets / atlas

### TexturePacker JSON (format hash) — **Indispensable**

Le format le plus universel. Structure :

```json
{
  "frames": {
    "sprite_name.png": {
      "frame": {"x": 0, "y": 0, "w": 32, "h": 32},
      "rotated": false,
      "trimmed": true,
      "spriteSourceSize": {"x": 2, "y": 1, "w": 28, "h": 30},
      "sourceSize": {"w": 32, "h": 32}
    }
  },
  "meta": {
    "image": "spritesheet.png",
    "size": {"w": 512, "h": 256},
    "scale": "1"
  }
}
```

Champs clés :
- `frame` : rectangle dans l'atlas
- `rotated` : rotation 90° pour un packing plus serré
- `trimmed` : suppression des bordures transparentes
- `spriteSourceSize` : offset pour restaurer le positionnement original

### Aseprite (.ase / .aseprite)

L'outil de pixel art dominant dans l'indie gamedev. Deux approches :

1. **Export CLI** (recommandé pour MVP) : `aseprite -b input.ase --sheet output.png --data output.json` → produit un JSON compatible TexturePacker
2. **Parsing binaire direct** : format bien documenté (header 128 bytes + frames + chunks). Contient layers, frames, animation tags, palette, slices.

### Sparrow/Starling XML

Couvre les assets Kenney et beaucoup d'autres outils :
```xml
<TextureAtlas imagePath="sheet.png">
  <SubTexture name="sprite" x="0" y="0" width="32" height="32"/>
</TextureAtlas>
```

### Spine Atlas
Format texte propre à Spine 2D. Seulement si vous supportez les animations Spine.

---

## 3. Formats de tile maps

### Tiled (.tmx / .json) — **Indispensable**

L'éditeur de tile maps standard. **Utiliser l'export JSON**, plus simple à parser que le XML.

Structure :
- **Map** : dimensions en tiles, taille des tiles en pixels, orientation (orthogonal, isométrique, hexagonal)
- **Tilesets** (.tsx) : image source, dimensions des tiles, propriétés par tile, formes de collision par tile, animations par tile
- **Layers** :
  - **Tile layers** : grille 2D de GIDs (IDs globaux). Stockage CSV, base64, ou base64+zlib/gzip/zstd
  - **Object layers** : objets freeform (spawn points, triggers, formes de collision)
  - **Image layers** : images de fond avec offset/parallax
  - **Group layers** : dossiers de layers
- **Properties** : propriétés custom clé-valeur sur tout élément

**Attention aux GIDs** : les bits 31/30/29 encodent le flip horizontal, vertical et diagonal. Il faut les masquer pour obtenir l'index réel.

### LDtk (.ldtk) — **Haute priorité**

"Level Designer Toolkit" par le créateur de Dead Cells. JSON moderne et propre.

- **World** : contient plusieurs niveaux arrangés spatialement (idéal pour les metroidvanias)
- **Levels** : dimensions, position dans le monde, layers
- **Layer types** :
  - **IntGrid** : grille de valeurs entières (collision/type de terrain)
  - **Tiles** : placement visuel de tiles
  - **Entities** : objets typés avec champs (très structuré, définitions d'entités avec types, défauts, contraintes)
  - **AutoLayer** : auto-tiling basé sur des règles
- **Enums** : définis dans le fichier, utilisables comme types de champs d'entités

Avantage clé sur Tiled : les définitions d'entités sont first-class. Le schéma est auto-descriptif.

### Ogmo — Priorité basse
Éditeur plus simple, JSON. Usage en déclin comparé à Tiled et LDtk.

---

## 4. Formats d'animation

### Animations sprite sheet (frame-by-frame) — **Indispensable**

L'approche la plus simple. Données nécessaires :
- Rectangles de frames (depuis l'atlas)
- Ordre des frames, durée par frame
- Mode de boucle (loop/once/ping-pong)

### Données d'animation Aseprite

L'export JSON inclut les `frameTags` :
```json
"frameTags": [
  {"name": "idle", "from": 0, "to": 3, "direction": "forward"},
  {"name": "run", "from": 4, "to": 9, "direction": "forward"},
  {"name": "jump", "from": 10, "to": 14, "direction": "pingpong"}
]
```
Chaque frame a une `"duration"` en millisecondes pour le timing variable.

### Spine 2D — Priorité moyenne

Animation squelettique premium (utilisé dans Hollow Knight, Slay the Spire).
- Fichiers : `.json` ou `.skel` (binaire) + `.atlas`
- Concepts : bones, slots, attachments, skins, contraintes IK
- **Licensing** : runtime open source mais nécessite une licence Spine ($70-$350)
- Recommandation : intégrer le runtime officiel spine-c plutôt que parser soi-même

### Autres (priorité basse ou à ignorer)
- **DragonBones** : gratuit/open-source, format JSON. Moins populaire que Spine, communauté en déclin.
- **Spriter** (.scml/.scon) : essentiellement abandonné.
- **Lottie** : format After Effects, surtout pour les animations UI. Complexe. Ignorer pour MVP.

---

## 5. Formats vectoriels

### SVG
- Usage en jeux : limité. Utile pour les éléments UI résolution-indépendante, les backgrounds procéduraux, les jeux au style vectoriel.
- SVG est un format énorme (XML avec paths, transforms, gradients, filtres, texte, CSS, animations...). Un renderer SVG complet est un projet majeur.
- **Approche pratique** : utiliser NanoSVG (single-header C, ~1500 lignes) pour rasteriser les SVGs en bitmaps au chargement.
- **Priorité MVP** : Basse. Rasteriser dans le pipeline d'assets si nécessaire.

---

## 6. Formats de polices

### BMFont (.fnt + .png) — **Haute priorité**

Police bitmap pré-rasterisée. Extrêmement simple à parser et à rendre.

```
info face="Arial" size=32 bold=0 italic=0
common lineHeight=36 base=28 scaleW=256 scaleH=256 pages=1
page id=0 file="arial_0.png"
chars count=95
char id=65 x=0 y=0 width=20 height=28 xoffset=1 yoffset=2 xadvance=22 page=0
kerning first=65 second=86 amount=-2
```

Parfait pour les jeux pixel art avec des polices custom.

### TTF / OTF — **Indispensable**

Formats de polices vectorielles standard. Rasterisés en atlas de glyphes au chargement.
- Bibliothèques : FreeType (complet, C), stb_truetype (single-header C, plus simple), fontdue (Rust)
- Approche : au démarrage, rendre les glyphes nécessaires aux tailles voulues dans un atlas de texture.

### SDF Fonts (Signed Distance Field) — Priorité moyenne

Technique où les bitmaps de glyphes stockent des valeurs de distance signée. Permet un scaling haute qualité, des outlines, ombres portées et effets de glow dans le shader.
- **MSDF** (Multi-channel SDF) : utilise les canaux RGB pour des coins plus nets
- Outils : msdf-atlas-gen
- Shader GLSL : ~10-30 lignes

---

## 7. Formats audio

### WAV — **Indispensable** (effets sonores)
- Audio PCM non compressé. Zéro overhead de décodage.
- Parsing trivial (header RIFF + samples bruts)
- Usage : effets sonores courts (sauts, impacts, pickups)
- Inconvénient : fichiers énormes (~10 MB/minute pour 16-bit stereo 44.1 kHz)

### OGG Vorbis — **Indispensable** (musique)
- Codec audio lossy open-source, sans brevets
- Bonne compression (~10:1 vs WAV), excellente qualité, streamable
- `stb_vorbis` : décodeur single-header C
- Usage : musique, audio ambiant, effets sonores longs

### MP3 — Priorité moyenne
- Format lossy universel. Brevets expirés (depuis 2017).
- `minimp3` : décodeur single-header C
- Le looping gapless est délicat (délai/padding de l'encodeur)

### FLAC — Priorité basse
- Audio lossless compressé (50-60% de la taille WAV). Overkill pour les jeux.

### MIDI — Priorité basse
- Pas de l'audio mais des événements musicaux. Nécessite un synthétiseur logiciel.
- Intéressant pour les jeux rétro ou la musique dynamique/adaptative.

---

## 8. Formats de scènes / niveaux

### JSON — **Indispensable**
- Lisible, supporté universellement, facile à parser
- Bibliothèques : cJSON, nlohmann/json, rapidjson, yyjson (C/C++), serde_json (Rust)
- Usage : fichiers de niveaux, définitions d'entités, configuration, arbres de dialogues

### TOML — Priorité basse-moyenne
- Simple, conçu pour la configuration. Syntaxe claire et non ambiguë.
- Usage : fichiers de configuration moteur/projet

### Formats binaires — Post-MVP
- **FlatBuffers** : désérialisation zero-copy. Temps de chargement quasi instantanés.
- **MessagePack** : JSON binaire compact et rapide
- Convertir JSON en binaire dans le pipeline d'assets pour les builds de distribution

### YAML — Ignorer
JSON couvre les mêmes usages avec un outillage plus simple.

---

## 9. Pipeline d'assets

### Pipeline MVP (simple)

Pour le MVP, pas de step de build — chargement direct :

1. **Charger les PNG** au runtime avec stb_image
2. **Charger les JSON** (atlas/map/animation) avec un parser JSON
3. **Charger les TTF** avec stb_truetype
4. **Charger WAV/OGG** avec dr_wav et stb_vorbis
5. **Chemins de fichiers comme IDs** : `assets/sprites/hero.png`
6. **Ajouter le hot-reload** : surveiller le dossier assets, recharger les fichiers modifiés

### Pipeline mature (production)

#### Import
- Accepter les formats source : `.png`, `.ase`, `.tmx`/`.ldtk`, `.ttf`, `.wav`, `.ogg`
- Valider les fichiers, reporter les erreurs tôt

#### Process / Convert
- **Images** : resize, premultiply alpha, conversion d'espace colorimétrique
- **Sprite sheets** : packing de sprites individuels en atlas (algorithmes : MaxRects, Skyline, Guillotine)
- **Aseprite** : export en spritesheet + JSON
- **Polices** : rasteriser TTF en atlas de glyphes (optionnellement SDF)

#### Pack / Optimize
- Combiner les petites textures en atlas plus grands
- Compression zlib/zstd des assets binaires
- Packing dans un fichier archive unique (PAK, ZIP, ou VFS)
- Manifeste d'assets avec types, tailles, offsets, dépendances

#### Runtime Loading
- **Asset manager** central : charge, cache et fournit l'accès aux assets par ID
- **Chargement paresseux** : charger à la demande
- **Ref counting ou ownership** : décharger quand plus référencé
- **Hot reloading** (mode dev) : watchers filesystem (inotify, FSEvents, ReadDirectoryChangesW)

---

## 10. Priorités MVP

### Tier 1 — Indispensable (implémenter en premier)

| Catégorie | Format | Raison |
|-----------|--------|--------|
| Image | **PNG** | Standard universel |
| Atlas | **TexturePacker JSON** (hash) | Couvre Aseprite export, TexturePacker, et beaucoup d'outils |
| Tile Map | **Tiled JSON** | Éditeur de maps le plus utilisé |
| Animation | **Sprite sheet + frame tags** | Système d'animation le plus simple, compatible export Aseprite |
| Police | **BMFont** (.fnt + .png) | Trivial à parser et rendre |
| Audio | **WAV** (SFX) + **OGG Vorbis** (musique) | Couvre tous les besoins audio |
| Données | **JSON** | Format de données universel |

### Tier 2 — Haute valeur (implémenter ensuite)

| Catégorie | Format | Raison |
|-----------|--------|--------|
| Image | **QOI** | Trivial à implémenter, idéal comme format runtime |
| Tile Map | **LDtk** | Moderne, JSON propre, popularité croissante |
| Police | **TTF** via stb_truetype | Texte scalable sans pré-baking |
| Police | **Rendu SDF** | Amélioration visuelle majeure pour le texte scalé |
| Image | **Aseprite binaire** (.ase) | Chargement direct sans export CLI |

### Tier 3 — Nice to have

| Catégorie | Format | Raison |
|-----------|--------|--------|
| Image | WebP, TGA | Compatibilité plus large |
| Atlas | Sparrow/Starling XML | Assets Kenney |
| Animation | Spine 2D | Animation squelettique premium (utiliser le runtime officiel) |
| Audio | MP3 | Compatibilité assets legacy |
| Données | Binaire/FlatBuffers | Optimisation temps de chargement |

### Bibliothèques recommandées (C/C++)

| Bibliothèque | Usage | Type |
|---|---|---|
| **stb_image** | PNG, JPEG, BMP, TGA, GIF | Single header |
| **stb_image_write** | Écriture PNG, BMP, TGA, JPEG | Single header |
| **stb_truetype** | Chargement et rasterisation TTF | Single header |
| **stb_vorbis** | Décodage OGG Vorbis | Single header |
| **dr_wav** | Chargement WAV | Single header |
| **dr_mp3** | Décodage MP3 | Single header |
| **qoi.h** | Encode/décode QOI | Single header (~300 lignes) |
| **yyjson** | Parsing JSON (le plus rapide) | Single header |
| **miniaudio** | Playback audio complet | Single header |
| **NanoSVG** | Parsing et rasterisation SVG | Deux single headers |
| **lodepng** | PNG uniquement avec support encoding | Single header |
