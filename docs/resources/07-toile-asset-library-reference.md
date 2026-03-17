# Toile Engine — Référence des Game Packs, Asset Packs et Asset Library

> **Objectif** : Ce document spécifie la conception d'une **Asset Library** pour le moteur Toile. Il recense l'ensemble des types d'assets qu'un game pack 2D peut contenir, les formats courants pour chaque catégorie, l'organisation typique des packs sur les différentes plateformes, et l'architecture interne de la bibliothèque d'assets dans Toile.
>
> **Principe** : L'Asset Library doit permettre d'**importer un asset pack complet** (ZIP/dossier), d'en **indexer automatiquement** le contenu par catégorie, de **prévisualiser** chaque élément dans un browser intégré, et de **glisser-déposer** n'importe quel asset sur une entité Toile.

---

## Table des matières

1. [Anatomie d'un Game Pack 2D](#1-anatomie-dun-game-pack-2d)
2. [Catégories d'assets](#2-catégories-dassets)
   - 2.1 [Sprites & Characters](#21-sprites--characters)
   - 2.2 [Tilesets](#22-tilesets)
   - 2.3 [Tilemaps (Maps / Levels)](#23-tilemaps-maps--levels)
   - 2.4 [Backgrounds & Parallax](#24-backgrounds--parallax)
   - 2.5 [GUI / UI Elements](#25-gui--ui-elements)
   - 2.6 [Icons](#26-icons)
   - 2.7 [VFX & Particles](#27-vfx--particles)
   - 2.8 [Bitmap Fonts](#28-bitmap-fonts)
   - 2.9 [Audio (SFX & Music)](#29-audio-sfx--music)
   - 2.10 [Animations squelettiques](#210-animations-squelettiques)
   - 2.11 [Props & Objects](#211-props--objects)
   - 2.12 [Données de jeu (Metadata)](#212-données-de-jeu-metadata)
3. [Formats de Tilesets et Tilemaps — Spécifications détaillées](#3-formats-de-tilesets-et-tilemaps--spécifications-détaillées)
   - 3.1 [Tiled TMX / TSX (XML)](#31-tiled-tmx--tsx-xml)
   - 3.2 [Tiled JSON (.tmj / .tsj)](#32-tiled-json-tmj--tsj)
   - 3.3 [LDtk (.ldtk)](#33-ldtk-ldtk)
   - 3.4 [Tilesets en grille brute (convention)](#34-tilesets-en-grille-brute-convention)
   - 3.5 [Tilesets auto-tile / Wang tiles](#35-tilesets-auto-tile--wang-tiles)
4. [Formats de Backgrounds & Parallax](#4-formats-de-backgrounds--parallax)
5. [Formats GUI / UI](#5-formats-gui--ui)
6. [Formats de Bitmap Fonts](#6-formats-de-bitmap-fonts)
   - 6.1 [BMFont Text (.fnt)](#61-bmfont-text-fnt)
   - 6.2 [BMFont XML (.fnt / .xml)](#62-bmfont-xml-fnt--xml)
   - 6.3 [Fonts TTF/OTF embarquées](#63-fonts-ttfotf-embarquées)
7. [Formats Audio](#7-formats-audio)
8. [Formats VFX / Particles](#8-formats-vfx--particles)
9. [Organisation typique des Asset Packs](#9-organisation-typique-des-asset-packs)
   - 9.1 [Structure type itch.io / artiste indépendant](#91-structure-type-itchio--artiste-indépendant)
   - 9.2 [Structure type Kenney](#92-structure-type-kenney)
   - 9.3 [Structure type CraftPix](#93-structure-type-craftpix)
   - 9.4 [Structure type GameMaker Bundles](#94-structure-type-gamemaker-bundles)
10. [Architecture de l'Asset Library Toile](#10-architecture-de-lasset-library-toile)
    - 10.1 [Vue d'ensemble](#101-vue-densemble)
    - 10.2 [Le Manifest (toile-asset-manifest.json)](#102-le-manifest-toile-asset-manifestjson)
    - 10.3 [Détection et classification automatique](#103-détection-et-classification-automatique)
    - 10.4 [Asset Browser (UI)](#104-asset-browser-ui)
    - 10.5 [Intégration avec les entités Toile](#105-intégration-avec-les-entités-toile)
11. [Modèle de données interne](#11-modèle-de-données-interne)
12. [Tableau récapitulatif des formats](#12-tableau-récapitulatif-des-formats)
13. [Priorités d'implémentation](#13-priorités-dimplémentation)

---

## 1. Anatomie d'un Game Pack 2D

Un **game pack** (ou asset pack) 2D est une collection organisée de fichiers graphiques, audio et de données destinée à fournir tout ou partie des assets nécessaires à un jeu 2D. Les packs varient énormément en complétude — d'un simple jeu d'icônes à un kit complet contenant personnages, tilesets, UI, backgrounds, sons et musique.

### Les deux niveaux de granularité

| Niveau | Nom courant | Contenu | Exemples |
|--------|-------------|---------|----------|
| **Pack unitaire** | Asset Pack, Sprite Pack, Tileset Pack | Une seule catégorie d'assets dans un style cohérent | "Dungeon Tileset", "Medieval Icons", "Platformer Character" |
| **Game Kit** | Game Kit, Complete Pack, All-in-One | Plusieurs catégories couvrant les besoins d'un genre entier | "Platformer Game Kit" (perso + tiles + BG + UI + SFX), "RPG Complete Pack" |

### Ce que contient un game kit complet typique

Un game kit complet de type platformer ou RPG contient généralement :

- **Personnages** : sprites de joueur, ennemis, PNJ avec leurs animations (idle, walk, run, jump, attack, die, etc.)
- **Tilesets** : tuiles d'environnement (terrain, murs, plateformes, décoration) organisées en grilles régulières
- **Tilemaps** : niveaux pré-construits utilisant les tilesets (formats Tiled, LDtk, ou captures d'écran de référence)
- **Backgrounds** : arrière-plans, souvent en couches pour effet parallaxe
- **UI / GUI** : boutons, barres de vie, menus, panneaux, sliders, boîtes de dialogue
- **Icons** : icônes d'objets, compétences, équipements, statuts
- **Props / Objects** : objets interactifs (coffres, portes, leviers, items, projectiles)
- **VFX** : effets visuels (explosions, particules, trail effects, impacts)
- **Fonts** : polices bitmap ou TTF assorties au style du pack
- **Audio** : effets sonores (pas, coups, collecte, UI) et boucles musicales
- **Metadata** : licence, crédits, documentation, previews

---

## 2. Catégories d'assets

### 2.1 Sprites & Characters

Les sprites de personnages sont presque toujours fournis sous forme de **spritesheets en grille uniforme** (strip horizontal ou grid). Chaque ligne correspond généralement à une animation différente.

**Formats rencontrés :**
- PNG grille uniforme (le plus courant, ~80% des packs gratuits)
- Fichiers Aseprite natifs (.aseprite / .ase) (courant sur itch.io)
- JSON Array/Hash + PNG (export Aseprite ou TexturePacker)
- Sprites individuels dans des dossiers par animation
- Spine JSON + Atlas (animation squelettique, packs premium)

**Conventions de nommage courantes :**
```
player_idle_strip4.png      → strip horizontal, 4 frames
player-walk-spritesheet.png → grille, nb frames à déduire
Character/
  Idle/
    idle_0.png, idle_1.png, idle_2.png
  Run/
    run_0.png ... run_7.png
```

**Métadonnées typiquement absentes qu'il faut déduire ou demander :**
- Taille d'une frame (frame width × frame height)
- Nombre de frames par animation
- Framerate / durée par frame
- Mapping ligne → animation

**Animations standard attendues pour un personnage de platformer :**
idle, walk, run, jump, fall, attack, hurt, die, crouch, climb, slide, dash

**Animations standard attendues pour un personnage RPG top-down :**
idle_down, idle_up, idle_left, idle_right, walk_down, walk_up, walk_left, walk_right, attack_down, etc.

---

### 2.2 Tilesets

Un tileset est une **image unique contenant une grille de tuiles** (tiles) de taille régulière, destinée à construire des niveaux/cartes.

**Tailles de tuile courantes :** 8×8, 16×16, 32×32, 48×48, 64×64 pixels

**Types de tilesets :**

| Type | Description | Usage |
|------|-------------|-------|
| **Terrain basique** | Tuiles de sol, murs, plateformes | Construction de la structure du niveau |
| **Auto-tile / Wang tile** | Tuiles avec règles de connexion (bord, coin, etc.) | Placement automatique avec transition fluide |
| **Decorative** | Props décoratifs placés sur le terrain | Détails visuels, végétation, mobilier |
| **Animated tiles** | Séquences de tuiles animées (eau, lave, torches) | Éléments dynamiques de l'environnement |
| **Collection tileset** | Tuiles de tailles variées (pas de grille uniforme) | Objets de taille arbitraire via Tiled |

**Formats rencontrés :**
- PNG grille uniforme (dominante, ~90%)
- Tiled TSX (XML) référençant une image PNG
- Tiled JSON tileset (.tsj) 
- LDtk tileset (défini dans le fichier .ldtk)
- Fichier Aseprite avec calques de tilemap (Aseprite 1.3+)

**Convention de nommage des tilesets :**
```
tileset_terrain_16x16.png
dungeon-tiles-32.png
forest_autotile.png
tiles/
  ground.png
  walls.png
  decoration.png
```

---

### 2.3 Tilemaps (Maps / Levels)

Les tilemaps sont des **fichiers décrivant l'agencement des tuiles** pour former un niveau. Elles ne sont pas toujours incluses dans les asset packs (beaucoup ne fournissent que les tilesets), mais leur support est essentiel.

**Formats majeurs :**

| Format | Extension | Outil | Description |
|--------|-----------|-------|-------------|
| **Tiled XML** | `.tmx` | Tiled Map Editor | Standard de facto. XML avec encodage base64/CSV/XML des layers |
| **Tiled JSON** | `.tmj` | Tiled Map Editor | Équivalent JSON du TMX. Plus facile à parser |
| **LDtk** | `.ldtk` | LDtk | Format JSON moderne. Auto-layers, entités avancées, export simplifié |
| **Tiled TSX** | `.tsx` | Tiled | Tileset externe (XML), référencé par les TMX |
| **Tiled JSON tileset** | `.tsj` | Tiled | Tileset externe (JSON) |
| **LDtk externe** | `.ldtkl` | LDtk | Niveaux séparés en fichiers individuels |

**Concepts communs à Tiled et LDtk :**
- **Tile layers** : grille de tiles référençant un tileset
- **Object layers** : objets positionnés librement (spawn points, triggers, zones)
- **Image layers** : images de background non-tilées
- **Propriétés custom** : métadonnées (type de collision, nom, script à déclencher)
- **Orientations** : orthogonale, isométrique, staggered isometric, hexagonale

---

### 2.4 Backgrounds & Parallax

Les backgrounds sont des **images décoratives plein écran** ou quasi plein écran, souvent utilisées en arrière-plan des niveaux.

**Types :**
- **Static background** : image unique
- **Parallax layers** : plusieurs couches se déplaçant à des vitesses différentes pour créer un effet de profondeur. Typiquement 3 à 7 couches.
- **Tileable background** : image conçue pour se répéter horizontalement et/ou verticalement

**Organisation typique d'un parallax :**
```
backgrounds/
  forest/
    1_sky.png          → couche la plus éloignée (vitesse la plus lente)
    2_clouds.png
    3_mountains.png
    4_trees_back.png
    5_trees_front.png  → couche la plus proche (vitesse la plus rapide)
```

Les couches sont **nommées ou numérotées** pour indiquer l'ordre de profondeur. Il n'existe pas de format standardisé pour les parallax — c'est une convention de dossier et de nommage. L'importeur doit déduire l'ordre des couches à partir du nom ou du numéro.

**Format des images** : PNG (avec transparence pour les couches intermédiaires), parfois JPEG pour le ciel/couche de fond.

**Métadonnées absentes (à définir par l'utilisateur ou avec heuristiques) :**
- Vitesse de défilement par couche (ratio par rapport à la caméra)
- Axe(s) de répétition (horizontal, vertical, les deux)
- Si l'image est tileable ou non

---

### 2.5 GUI / UI Elements

Les éléments d'interface utilisateur sont fournis soit comme **sprites individuels**, soit comme **spritesheets**, soit comme des **9-slice/9-patch panels** pour le redimensionnement intelligent.

**Éléments typiques d'un GUI pack :**
- Boutons (normal, hover, pressed, disabled)
- Panneaux / fenêtres de dialogue
- Barres (vie, mana, XP, chargement) avec fill séparé
- Sliders, checkboxes, radio buttons
- Curseurs / pointeurs
- Inventaire slots
- Tooltips, bulles de dialogue
- Badges, cadres de portrait
- Tabs, scrollbars

**Formats rencontrés :**
- PNG sprites individuels (dominant)
- Spritesheet JSON Hash/Array avec données de 9-slice (`borders` dans TexturePacker)
- Fichier Aseprite avec slices pour le 9-patch
- PSD/AI sources (non utilisables directement par le moteur)

**Convention 9-slice :**
Le 9-slice (ou 9-patch) découpe une image en 9 zones. Les coins ne sont pas redimensionnés, les bords sont étirés dans une direction, et le centre est étiré dans les deux directions. Les données de 9-slice peuvent être :
- Définies dans le JSON du spritesheet (`borders: {left, top, right, bottom}`)
- Définies dans les Slices Aseprite (avec données 9-patch)
- Inférées par convention de nommage (ex: `panel_9slice.png`)
- Définies manuellement par l'utilisateur dans Toile

---

### 2.6 Icons

Les icônes sont des **petites images carrées** (16×16 à 128×128 pixels) regroupées par catégorie.

**Catégories courantes :** items, armes, armures, sorts, compétences, statuts, ressources, monnaies, éléments, classes

**Formats :**
- Spritesheet en grille uniforme (un fichier = une catégorie)
- Sprites individuels dans un dossier (un fichier = une icône)
- SVG (rare en jeu, courant pour le web — ex: game-icons.net)

---

### 2.7 VFX & Particles

Les effets visuels sont fournis sous différentes formes :

| Type | Format | Description |
|------|--------|-------------|
| **Spritesheet animée** | PNG grille ou strip | Séquence d'images (explosion, fumée, étincelles) |
| **Particle emitter** | JSON / XML | Description paramétrique d'un système de particules |
| **Shader / FX** | GLSL, HLSL | Code de shader (rare dans les packs gratuits) |

**Formats de particle emitters courants :**
- **Cocos2D particle (.plist)** : format XML Apple plist avec des paramètres de particules (gravity, speed, lifetime, color, etc.)
- **Particle Designer JSON** : format JSON exporté par des outils comme Particle Designer
- **Custom JSON** : beaucoup de moteurs définissent leur propre format
- **GDParticles (Godot)** : format .tres/.tscn propre à Godot

La plupart des VFX dans les packs gratuits sont simplement des **spritesheets animées** sans système de particules — ce sont des animations frame-by-frame classiques.

---

### 2.8 Bitmap Fonts

Les bitmap fonts sont des **polices pré-rendues** sous forme d'atlas d'images avec un fichier descripteur.

**Formats principaux :**
- **BMFont Text** (.fnt) : format texte AngelCode BMFont — le standard le plus courant
- **BMFont XML** (.fnt / .xml) : variante XML du même format
- **BMFont Binary** (.fnt) : variante binaire (rare)
- **TTF/OTF** : polices vectorielles embarquées (pas bitmap, mais souvent incluses dans les packs)

Le format BMFont (text ou XML) est composé de deux fichiers : un `.fnt` (descripteur) et un ou plusieurs `.png` (atlas de glyphes).

---

### 2.9 Audio (SFX & Music)

**Formats d'effets sonores (SFX) :**
- **WAV** (.wav) : PCM non compressé. Référence pour les SFX courts (qualité max, taille importante)
- **OGG Vorbis** (.ogg) : compressé avec perte, libre de droits. Standard pour les jeux indie
- **MP3** (.mp3) : compressé avec perte. Universel mais anciennement breveté
- **FLAC** (.flac) : compressé sans perte. Parfois pour les masters

**Formats de musique :**
- **OGG Vorbis** (.ogg) : dominant dans les packs de jeux
- **MP3** (.mp3) : très courant
- **WAV** (.wav) : parfois pour les boucles courtes
- **MIDI** (.mid) : rare, pour les jeux rétro
- **XM / MOD / IT** (.xm, .mod, .it) : formats tracker, niche mais fidèles au rétro

**Organisation typique :**
```
audio/
  sfx/
    player/
      jump.wav, land.wav, hurt.ogg, die.ogg
    ui/
      click.wav, hover.wav, confirm.ogg
    environment/
      wind.ogg, water.ogg
  music/
    level1_loop.ogg
    boss_theme.ogg
    menu.ogg
```

---

### 2.10 Animations squelettiques

Déjà couvertes en détail dans le document `toile-sprite-formats-reference.md`. Les formats Spine, DragonBones et Spriter sont parfois inclus dans des game packs premium.

---

### 2.11 Props & Objects

Les props sont des **éléments décoratifs ou interactifs** positionnés dans le monde mais qui ne font pas partie du tileset.

**Exemples :** coffres, portes, leviers, arbres, rochers, lampadaires, PNJ, projectiles, collectibles (pièces, cœurs), plateformes mobiles

**Formats :** identiques aux sprites (PNG individuels, spritesheets pour les animés)

---

### 2.12 Données de jeu (Metadata)

**Fichiers courants dans un pack :**
- `README.txt` / `README.md` : instructions, description du contenu
- `LICENSE.txt` / `LICENSE` : termes d'utilisation
- `CREDITS.txt` : attribution requise (CC-BY)
- `preview.png` / `thumbnail.png` : aperçu du pack
- `.itch.toml` : métadonnées itch.io (si distribué via butler)

---

## 3. Formats de Tilesets et Tilemaps — Spécifications détaillées

### 3.1 Tiled TMX / TSX (XML)

| Propriété | Valeur |
|-----------|--------|
| **Outil source** | Tiled Map Editor (gratuit, open-source, MIT) |
| **Extensions** | `.tmx` (map), `.tsx` (tileset externe) |
| **Fichiers associés** | `.png` (images tilesets), `.tsx` (tilesets externes), `.tx` (templates) |
| **Encodage** | UTF-8 XML |
| **Popularité** | ★★★★★ — Le standard de facto pour les tilemaps 2D |

#### Structure d'un fichier TMX

```xml
<?xml version="1.0" encoding="UTF-8"?>
<map version="1.10" tiledversion="1.11.0" orientation="orthogonal"
     renderorder="right-down" width="30" height="20"
     tilewidth="16" tileheight="16" infinite="0"
     nextlayerid="5" nextobjectid="10">

  <!-- Tileset intégré ou référence externe -->
  <tileset firstgid="1" source="terrain.tsx"/>

  <!-- Tileset intégré -->
  <tileset firstgid="257" name="objects" tilewidth="16" tileheight="16"
           tilecount="64" columns="8">
    <image source="objects.png" width="128" height="128"/>
    <tile id="0">
      <properties>
        <property name="type" value="collectible"/>
        <property name="points" type="int" value="100"/>
      </properties>
      <objectgroup>
        <object id="1" x="2" y="2" width="12" height="12"/>
      </objectgroup>
      <animation>
        <frame tileid="0" duration="200"/>
        <frame tileid="1" duration="200"/>
        <frame tileid="2" duration="200"/>
      </animation>
    </tile>
  </tileset>

  <!-- Tile Layer -->
  <layer id="1" name="Ground" width="30" height="20">
    <data encoding="csv">
1,1,1,2,2,2,3,3,...
0,0,0,1,1,0,0,0,...
    </data>
  </layer>

  <!-- Tile Layer avec Base64 + compression -->
  <layer id="2" name="Walls" width="30" height="20">
    <data encoding="base64" compression="zlib">
      eJztwTEBAAAAwqD1T20ND6AAAAAAAAB4NhAAAAE=
    </data>
  </layer>

  <!-- Object Layer -->
  <objectgroup id="3" name="Entities">
    <object id="1" name="player_spawn" type="spawn"
            x="48" y="160" width="16" height="16"/>
    <object id="2" name="enemy" type="goblin"
            x="200" y="100" width="16" height="16">
      <properties>
        <property name="patrol_distance" type="int" value="64"/>
      </properties>
    </object>
    <object id="3" name="trigger_zone" type="trigger"
            x="100" y="50">
      <polygon points="0,0 50,0 50,30 0,30"/>
    </object>
  </objectgroup>

  <!-- Image Layer (background) -->
  <imagelayer id="4" name="Background" offsetx="0" offsety="0"
              parallaxx="0.5" parallaxy="0.5">
    <image source="bg_mountains.png"/>
  </imagelayer>
</map>
```

#### Éléments clés du TMX

**`<map>` — racine :**

| Attribut | Description |
|----------|-------------|
| `orientation` | `orthogonal`, `isometric`, `staggered`, `hexagonal` |
| `renderorder` | `right-down`, `right-up`, `left-down`, `left-up` |
| `width`, `height` | Taille de la map en tuiles |
| `tilewidth`, `tileheight` | Taille d'une tuile en pixels |
| `infinite` | `0` = taille fixe, `1` = map infinie (chunks) |
| `parallaxx`, `parallaxy` | Facteur parallax par défaut (Tiled 1.6+) |

**`<tileset>` — définition d'un tileset :**

| Attribut | Description |
|----------|-------------|
| `firstgid` | Premier ID global de ce tileset dans la map |
| `source` | Chemin vers un fichier `.tsx` externe (exclusif avec définition inline) |
| `name` | Nom du tileset |
| `tilewidth`, `tileheight` | Taille des tuiles |
| `tilecount` | Nombre total de tuiles |
| `columns` | Nombre de colonnes dans l'image |
| `spacing` | Espacement entre tuiles dans l'image (pixels) |
| `margin` | Marge autour des tuiles dans l'image (pixels) |

**`<tile>` — propriétés par tuile :**
- `<properties>` : métadonnées custom (collision, type, etc.)
- `<objectgroup>` : formes de collision sur la tuile
- `<animation>` : séquence de frames animées (`<frame tileid="N" duration="ms"/>`)

**Encodage des tile layers :**
- `csv` : liste de GIDs séparés par des virgules (simple, lisible)
- `base64` : données binaires encodées en base64 (compact)
- Compression optionnelle : `zlib`, `gzip`, `zstd` (Tiled 1.3+)

**Global Tile IDs (GIDs) :**
- Un GID est un identifiant unique dans la map pour chaque tuile.
- GID `0` = tuile vide.
- Le GID d'une tuile = `firstgid` de son tileset + index local dans le tileset.
- Les 3 bits de poids fort encodent les transformations : bit 32 = flip horizontal, bit 31 = flip vertical, bit 30 = flip diagonal (rotation).

**Wang Sets (auto-tiling) :**
Depuis Tiled 1.5, les Wang Sets remplacent l'ancien système de terrain. Ils définissent des règles de connexion entre tuiles :
```xml
<wangsets>
  <wangset name="Grass to Dirt" type="corner" tile="-1">
    <wangcolor name="Grass" color="#00ff00" tile="0" probability="1"/>
    <wangcolor name="Dirt" color="#8b4513" tile="5" probability="1"/>
    <wangtile tileid="0" wangid="1,1,1,1,1,1,1,1"/>
    <wangtile tileid="1" wangid="1,1,1,2,2,2,1,1"/>
    <!-- ... -->
  </wangset>
</wangsets>
```

#### Structure d'un fichier TSX (tileset externe)

Identique au `<tileset>` inline dans le TMX, mais sans `firstgid` (qui est propre à chaque map) :

```xml
<?xml version="1.0" encoding="UTF-8"?>
<tileset version="1.10" tiledversion="1.11.0"
         name="terrain" tilewidth="16" tileheight="16"
         tilecount="256" columns="16">
  <image source="terrain.png" width="256" height="256"/>
  <tile id="10">
    <animation>
      <frame tileid="10" duration="250"/>
      <frame tileid="11" duration="250"/>
      <frame tileid="12" duration="250"/>
    </animation>
  </tile>
  <wangsets>
    <!-- ... -->
  </wangsets>
</tileset>
```

---

### 3.2 Tiled JSON (.tmj / .tsj)

Tiled supporte aussi l'export en JSON. Le format est l'équivalent exact du TMX en JSON.

**Extensions :** `.tmj` (map, anciennement `.json`), `.tsj` (tileset, anciennement `.json`)

**Structure simplifiée d'un .tmj :**

```json
{
  "type": "map",
  "version": "1.10",
  "tiledversion": "1.11.0",
  "orientation": "orthogonal",
  "renderorder": "right-down",
  "width": 30, "height": 20,
  "tilewidth": 16, "tileheight": 16,
  "infinite": false,
  "tilesets": [
    { "firstgid": 1, "source": "terrain.tsj" }
  ],
  "layers": [
    {
      "type": "tilelayer",
      "id": 1, "name": "Ground",
      "width": 30, "height": 20,
      "data": [1,1,1,2,2,3,0,0,...],
      "encoding": "csv"
    },
    {
      "type": "objectgroup",
      "id": 2, "name": "Entities",
      "objects": [
        {
          "id": 1, "name": "player_spawn", "type": "spawn",
          "x": 48, "y": 160, "width": 16, "height": 16,
          "properties": [
            { "name": "facing", "type": "string", "value": "right" }
          ]
        }
      ]
    },
    {
      "type": "imagelayer",
      "id": 3, "name": "Background",
      "image": "bg_mountains.png",
      "parallaxx": 0.5, "parallaxy": 0.5
    }
  ]
}
```

Le JSON est généralement préférable au XML pour le parsing (plus simple, plus rapide). Les deux formats sont strictement équivalents.

---

### 3.3 LDtk (.ldtk)

| Propriété | Valeur |
|-----------|--------|
| **Outil source** | LDtk (Level Designer Toolkit) — par Sébastien Bénard (créateur de Dead Cells) |
| **Licence** | Gratuit, MIT |
| **Extensions** | `.ldtk` (projet), `.ldtkl` (level externe) |
| **Fichiers associés** | `.png` (tilesets), dossier de PNGs par level (si Super Simple Export) |
| **Encodage** | UTF-8 JSON |
| **Popularité** | ★★★★ — En forte croissance, apprécié pour son UX et ses auto-layers |

#### Structure de haut niveau

```json
{
  "__header__": {
    "fileType": "LDtk Project JSON",
    "app": "LDtk",
    "appAuthor": "Sebastien 'deepnight' Benard"
  },
  "jsonVersion": "1.5.3",
  "worldLayout": "GridVania",
  "worldGridWidth": 256,
  "worldGridHeight": 256,
  "defs": {
    "layers": [ /* définitions de layers */ ],
    "entities": [ /* définitions d'entités */ ],
    "tilesets": [
      {
        "identifier": "Terrain",
        "uid": 1,
        "relPath": "tilesets/terrain.png",
        "pxWid": 256, "pxHei": 256,
        "tileGridSize": 16,
        "spacing": 0, "padding": 0,
        "tagsSourceEnumUid": null,
        "enumTags": []
      }
    ],
    "enums": [ /* énumérations custom */ ]
  },
  "levels": [
    {
      "identifier": "Level_0",
      "iid": "...",
      "worldX": 0, "worldY": 0,
      "pxWid": 480, "pxHei": 320,
      "bgColor": "#696A79",
      "layerInstances": [
        {
          "__identifier": "Entities",
          "__type": "Entities",
          "entityInstances": [
            {
              "__identifier": "Player",
              "__grid": [5, 10],
              "px": [88, 168],
              "fieldInstances": [
                { "__identifier": "health", "__value": 100, "__type": "Int" }
              ]
            }
          ]
        },
        {
          "__identifier": "Terrain",
          "__type": "Tiles",
          "__tilesetRelPath": "tilesets/terrain.png",
          "gridTiles": [
            { "px": [0, 304], "src": [0, 0], "f": 0, "t": 0 },
            { "px": [16, 304], "src": [16, 0], "f": 0, "t": 1 }
          ]
        },
        {
          "__identifier": "Collisions",
          "__type": "IntGrid",
          "intGridCsv": [0,0,0,0,1,1,1,0,0,0,...]
        }
      ]
    }
  ]
}
```

#### Spécificités LDtk

- **Auto-layers** : LDtk peut générer automatiquement des layers de tiles à partir d'un IntGrid (grille d'entiers représentant les types de terrain). Les règles d'auto-tiling sont définies dans l'éditeur et résolues avant l'export — le fichier JSON contient le résultat final.
- **Entités typées** : les entités ont des champs custom typés (Int, Float, Bool, String, Enum, Color, Point, Array, EntityRef, etc.)
- **Super Simple Export** : mode d'export simplifié qui génère un PNG composité par layer + un petit JSON d'entités, évitant le besoin de parser le gros fichier LDtk.
- **Worlds** : les niveaux sont organisés en "mondes" avec des dispositions Gridvania, linéaires ou libres.
- **Champs préfixés `__`** : les champs commençant par `__` (double underscore) sont des "helpers" dupliquant des données des définitions pour faciliter le parsing côté jeu.

#### Parsing recommandé

LDtk fournit un **JSON Schema** officiel et un générateur de types via QuickType. Les 2 sections essentielles à parser sont les `tilesets` (dans `defs`) et les `levels` (données de jeu). Le reste est surtout utile à l'éditeur.

---

### 3.4 Tilesets en grille brute (convention)

Beaucoup de tilesets sont distribués comme de **simples images PNG** sans aucun fichier descripteur. C'est le format le plus courant dans les packs gratuits.

**Paramètres nécessaires (à fournir par l'utilisateur ou par heuristiques) :**
- Taille de tuile (width × height)
- Espacement (spacing) entre tuiles
- Marge (margin) autour de la grille

**Heuristiques de détection automatique :**
1. Rechercher une taille de tuile parmi les standards (8, 16, 32, 48, 64) qui divise proprement les dimensions de l'image.
2. Vérifier si la première ligne/colonne de pixels est transparente ou d'une couleur de fond (indique un spacing).
3. Si le nom contient la taille (ex: `tileset_16x16.png`), l'utiliser.

---

### 3.5 Tilesets auto-tile / Wang tiles

Les auto-tiles permettent le **placement automatique** de transitions (herbe→terre, eau→terre, etc.) basé sur les voisins de chaque tuile.

**Systèmes courants :**

| Système | Nb tuiles | Description |
|---------|-----------|-------------|
| **2-corner** (simple) | 16 | Transitions basées sur 4 coins |
| **4-corner** (RPG Maker) | 47-48 | Standard RPG Maker (A2 autotile) |
| **8-edge** (blob) | 47 | Transitions basées sur 8 voisins (4 bords + 4 coins) |
| **Wang corner** | variable | Système Wang de Tiled (couleurs aux coins) |
| **Wang edge** | variable | Système Wang de Tiled (couleurs aux bords) |

Les données d'auto-tile sont définies soit dans le **fichier TSX/TMX** (via Wang Sets), soit dans le **fichier LDtk** (via les règles d'auto-layer), soit **implicitement** par la disposition des tuiles dans l'image (conventions RPG Maker, Godot, etc.).

---

## 4. Formats de Backgrounds & Parallax

Il n'existe pas de format standardisé pour les backgrounds parallax. Toile doit supporter ces cas :

| Cas | Détection | Import |
|-----|-----------|--------|
| **Images numérotées** | `bg_1.png`, `bg_2.png`, ... ou `layer_01.png`, `layer_02.png` | Trier par numéro, layer 1 = plus loin |
| **Noms descriptifs** | `sky.png`, `mountains.png`, `trees.png` | Présenter à l'utilisateur pour ordonnancement |
| **Dossier "parallax"** | Dossier nommé `parallax/`, `background/`, `bg/` | Traiter tout le contenu comme des couches |
| **Image unique** | Fichier isolé avec `background` ou `bg` dans le nom | Single background, pas de parallax |
| **Couche Tiled** | Image layer dans un TMX/LDtk | Utiliser les attributs `parallaxx`/`parallaxy` |

**Données de parallax à stocker dans Toile :**
```
ToileParallaxBackground
├── name: string
├── layers: ToileParallaxLayer[]
└── repeatX: bool, repeatY: bool

ToileParallaxLayer
├── imagePath: string
├── depth: number          // 0.0 (plus éloigné) à 1.0 (plus proche)
├── scrollFactorX: number  // vitesse relative à la caméra (0.0 à 1.0+)
├── scrollFactorY: number
├── offsetX: number, offsetY: number
├── repeatX: bool, repeatY: bool
└── autoScroll: { x, y } | null  // défilement automatique (nuages, etc.)
```

---

## 5. Formats GUI / UI

Les assets UI sont principalement des **PNG individuels** organisés par composant. Les formats de données accompagnant les UI sont :

| Format | Usage | Description |
|--------|-------|-------------|
| **PNG individuels** | Dominant | Un fichier par état de bouton, panel, etc. |
| **Spritesheet + JSON** | Courant (pro) | Atlas TexturePacker avec données de 9-slice |
| **Aseprite + Slices** | Courant (pixel art) | 9-patch défini dans les slices Aseprite |
| **SVG** | Rare (web) | Interface vectorielle scalable |

**Convention de nommage des états UI :**
```
button_normal.png
button_hover.png
button_pressed.png
button_disabled.png

bar_fill.png        → partie qui se remplit
bar_background.png  → fond de la barre
bar_border.png      → cadre de la barre
```

**Données 9-slice à stocker :**
```
ToileNineSlice
├── imagePath: string
├── borders: { left, top, right, bottom }  // en pixels
├── tileCenter: bool     // étirer ou répéter le centre
└── tileEdges: bool      // étirer ou répéter les bords
```

---

## 6. Formats de Bitmap Fonts

### 6.1 BMFont Text (.fnt)

Le format le plus courant. Fichier texte avec des lignes typées :

```
info face="Arial" size=32 bold=0 italic=0 charset="" unicode=1 stretchH=100 smooth=1 aa=1 padding=0,0,0,0 spacing=1,1
common lineHeight=32 base=26 scaleW=256 scaleH=256 pages=1 packed=0
page id=0 file="arial_0.png"
chars count=95
char id=32   x=0    y=0    width=0    height=0    xoffset=0    yoffset=0    xadvance=8    page=0  chnl=15
char id=33   x=120  y=68   width=6    height=22   xoffset=3    yoffset=4    xadvance=10   page=0  chnl=15
char id=34   x=82   y=90   width=10   height=10   xoffset=1    yoffset=4    xadvance=11   page=0  chnl=15
kernings count=91
kerning first=32  second=65  amount=-2
kerning first=49  second=49  amount=-2
```

**Sections :**

| Section | Description |
|---------|-------------|
| `info` | Métadonnées de la police (face, size, bold, italic, padding, spacing) |
| `common` | Données communes (lineHeight, base, dimensions atlas, nombre de pages) |
| `page` | Fichier(s) image de l'atlas (un par page) |
| `char` | Données par caractère (position dans atlas, offsets, avance) |
| `kerning` | Paires de kerning (espacement spécial entre certaines paires de lettres) |

**Champs `char` :**

| Champ | Description |
|-------|-------------|
| `id` | Code Unicode du caractère |
| `x`, `y` | Position dans l'atlas |
| `width`, `height` | Dimensions du glyphe dans l'atlas |
| `xoffset`, `yoffset` | Offset pour le positionnement du rendu |
| `xadvance` | Distance pour avancer le curseur après ce caractère |
| `page` | Index de la page atlas |
| `chnl` | Canal(aux) contenant les données (15 = tous) |

### 6.2 BMFont XML (.fnt / .xml)

Même structure, en XML :

```xml
<?xml version="1.0"?>
<font>
  <info face="Arial" size="32" bold="0" italic="0" charset=""
        unicode="1" stretchH="100" smooth="1" aa="1"
        padding="0,0,0,0" spacing="1,1"/>
  <common lineHeight="32" base="26" scaleW="256" scaleH="256"
          pages="1" packed="0"/>
  <pages>
    <page id="0" file="arial_0.png"/>
  </pages>
  <chars count="95">
    <char id="32" x="0" y="0" width="0" height="0"
          xoffset="0" yoffset="0" xadvance="8" page="0" chnl="15"/>
    <char id="33" x="120" y="68" width="6" height="22"
          xoffset="3" yoffset="4" xadvance="10" page="0" chnl="15"/>
  </chars>
  <kernings count="91">
    <kerning first="32" second="65" amount="-2"/>
  </kernings>
</font>
```

### 6.3 Fonts TTF/OTF embarquées

De nombreux packs incluent des polices vectorielles TTF/OTF. Toile doit pouvoir :
- Les enregistrer dans la bibliothèque d'assets
- Les rasteriser à la demande (via une bibliothèque comme `rusttype`, `fontdue`, ou les API système)
- Les utiliser pour le rendu de texte in-game

---

## 7. Formats Audio

| Format | Extension | Compression | Qualité | Usage recommandé |
|--------|-----------|-------------|---------|-----------------|
| **WAV** | `.wav` | Non compressé | Parfaite | SFX courts (< 5s) |
| **OGG Vorbis** | `.ogg` | Lossy | Très bonne | SFX + musique (standard indie) |
| **MP3** | `.mp3` | Lossy | Bonne | Musique (compatibilité universelle) |
| **FLAC** | `.flac` | Lossless | Parfaite | Masters, archivage |
| **OPUS** | `.opus` | Lossy | Excellente | Successeur d'OGG (meilleur ratio) |
| **XM/MOD/IT** | `.xm`, `.mod`, `.it` | Tracker | Variable | Musique rétro/chiptune |

**Données audio à indexer :**
```
ToileAudioAsset
├── path: string
├── format: "wav" | "ogg" | "mp3" | ...
├── duration: number        // en secondes
├── sampleRate: number
├── channels: number        // 1=mono, 2=stereo
├── category: "sfx" | "music" | "ambient" | "voice"
├── loop: bool              // inféré du nom ou métadonnées
└── tags: string[]          // ex: ["player", "jump", "action"]
```

---

## 8. Formats VFX / Particles

### Cocos2D Particle (.plist)

Format le plus courant pour les particules 2D. Structure Apple plist XML avec les paramètres :

```xml
<dict>
  <key>maxParticles</key> <real>250</real>
  <key>duration</key> <real>-1</real>
  <key>particleLifespan</key> <real>1.5</real>
  <key>particleLifespanVariance</key> <real>0.5</real>
  <key>speed</key> <real>100</real>
  <key>speedVariance</key> <real>30</real>
  <key>gravityx</key> <real>0</real>
  <key>gravityy</key> <real>-300</real>
  <key>emitterType</key> <real>0</real>
  <key>startColorRed</key> <real>1</real>
  <key>textureFileName</key> <string>particle.png</string>
  <!-- ... -->
</dict>
```

### Spritesheet VFX

La plupart des VFX dans les packs gratuits sont simplement des **spritesheets animées**. Elles se traitent exactement comme des sprites de personnages (voir section 2.1 et le document de référence des sprites).

---

## 9. Organisation typique des Asset Packs

### 9.1 Structure type itch.io / artiste indépendant

C'est la structure la plus variable. Chaque artiste a ses conventions. Exemples courants :

**Pack simple (tileset seul) :**
```
Forest_Tileset/
├── forest_tileset.png
├── forest_tileset.aseprite        (optionnel, source Aseprite)
├── preview.png
├── README.txt
└── LICENSE.txt
```

**Pack complet (artiste organisé) :**
```
Pixel_Adventure/
├── Main Characters/
│   ├── Ninja Frog/
│   │   ├── Idle (32x32).png       (strip horizontal)
│   │   ├── Run (32x32).png
│   │   ├── Jump (32x32).png
│   │   ├── Double Jump (32x32).png
│   │   ├── Wall Jump (32x32).png
│   │   ├── Fall (32x32).png
│   │   ├── Hit (32x32).png
│   │   └── Appearing (96x96).png
│   ├── Pink Man/
│   │   └── ...
│   └── Virtual Guy/
│       └── ...
├── Enemies/
│   ├── Angry Pig/
│   │   ├── Idle (36x30).png
│   │   ├── Walk (36x30).png
│   │   └── Hit (36x30).png
│   └── ...
├── Terrain/
│   └── Terrain (16x16).png
├── Background/
│   ├── Blue.png
│   ├── Brown.png
│   └── ...
├── Items/
│   ├── Fruits/
│   │   ├── Apple.png
│   │   └── ...
│   └── Boxes/
│       └── ...
├── Traps/
│   ├── Spikes/
│   │   └── ...
│   └── Fire/
│       └── ...
├── Other/
│   ├── Dust Particles/
│   │   └── ...
│   └── ...
└── Tileset.png
```

**Convention de nommage des tailles dans le nom de fichier :**
Un pattern très courant (surtout chez Pixel Frog et d'autres artistes itch.io) est d'inclure la taille des frames dans le nom : `Idle (32x32).png`, `Run (32x32).png`. Regex : `\((\d+)x(\d+)\)`.

### 9.2 Structure type Kenney

Kenney utilise une organisation très structurée et cohérente :

```
Kenney_Platformer_Pack/
├── Spritesheet/
│   ├── spritesheet_characters.png
│   ├── spritesheet_characters.xml     (Starling XML)
│   ├── spritesheet_tiles.png
│   └── spritesheet_tiles.xml
├── PNG/
│   ├── Characters/
│   │   ├── character_0000.png
│   │   ├── character_0001.png
│   │   └── ...
│   ├── Tiles/
│   │   ├── tile_0000.png
│   │   └── ...
│   └── Items/
│       └── ...
├── Vector/                             (fichiers SVG)
│   └── ...
├── Tilesheet/
│   └── tilesheet_complete.png          (grille uniforme)
├── Sample/
│   └── sample.png                      (preview/screenshot)
├── License.txt                         (CC0)
└── Kenney Fonts/
    ├── Kenney Pixel.ttf
    └── ...
```

**Points clés Kenney :**
- Spritesheets fournies au format **Starling XML** (.xml)
- Sprites individuels PNG toujours disponibles (pas besoin de parser la spritesheet)
- Fichiers vectoriels SVG inclus pour l'édition
- Style cohérent entre tous les packs (même palette, même esthétique)
- Licence CC0 universelle

### 9.3 Structure type CraftPix

CraftPix fournit des packs professionnels, souvent organisés par composant de jeu :

```
Medieval_Platformer_Game_Kit/
├── 1 Characters/
│   ├── 1 Heros/
│   │   ├── Hero1/
│   │   │   ├── Idle.png
│   │   │   ├── Run.png
│   │   │   ├── Attack.png
│   │   │   └── ...
│   │   └── Hero2/
│   │       └── ...
│   └── 2 Enemies/
│       └── ...
├── 2 Tileset/
│   ├── tileset.png
│   └── tileset_with_grid.png
├── 3 Background/
│   ├── 1.png
│   ├── 2.png
│   ├── 3.png
│   └── 4.png                          (couches parallax)
├── 4 GUI/
│   ├── Buttons/
│   │   └── ...
│   ├── Bars/
│   │   └── ...
│   └── Panels/
│       └── ...
├── 5 Props/
│   └── ...
├── 6 Icons/
│   └── ...
└── Preview/
    └── preview.png
```

### 9.4 Structure type GameMaker Bundles

GameMaker fournit des bundles intégrés avec parfois des animations Spine :

```
Maritime_Mayhem/
├── sprites/
│   ├── spr_ship_idle/
│   │   ├── 0.png, 1.png, 2.png, ...
│   │   └── sprite.yy                  (métadonnées GameMaker)
│   └── ...
├── sounds/
│   ├── snd_cannon.wav
│   └── ...
├── spine/
│   ├── ship.json
│   ├── ship.atlas
│   └── ship.png
├── tilesets/
│   └── ...
└── music/
    └── sea_shanty.ogg
```

---

## 10. Architecture de l'Asset Library Toile

### 10.1 Vue d'ensemble

```
┌─────────────────────────────────────────────────────────────────────┐
│                       ASSET LIBRARY TOILE                           │
│                                                                     │
│  ┌──────────────┐    ┌──────────────────┐    ┌──────────────────┐  │
│  │  IMPORT       │    │  ASSET DATABASE   │    │  ASSET BROWSER   │  │
│  │               │    │                   │    │  (UI)            │  │
│  │ ZIP/Dossier ──┤──▶│ Manifest JSON    ──┤──▶│ Thumbnails      │  │
│  │ Auto-detect   │    │ Index par type    │    │ Filtres/Search  │  │
│  │ Classify      │    │ Tags/Metadata     │    │ Preview         │  │
│  │ Thumbnail gen │    │ Paths/Refs        │    │ Drag & Drop     │  │
│  └──────────────┘    └──────────────────┘    └──────┬───────────┘  │
│                                                      │              │
│                                                      ▼              │
│                              ┌──────────────────────────────────┐  │
│                              │  ENTITÉ TOILE                     │  │
│                              │                                    │  │
│                              │  SpriteRenderer ← asset ref       │  │
│                              │  TileMap        ← tileset ref     │  │
│                              │  AudioSource    ← audio ref       │  │
│                              │  ParallaxBG     ← background ref  │  │
│                              │  UIElement      ← gui ref         │  │
│                              │  TextRenderer   ← font ref        │  │
│                              └──────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

### Pipeline d'import

```
1. ENTRÉE
   └── ZIP, dossier, ou fichiers individuels

2. SCAN & DÉTECTION
   ├── Parcourir récursivement tous les fichiers
   ├── Identifier le type de chaque fichier (extension + contenu)
   ├── Détecter les fichiers liés (JSON + PNG, .fnt + .png, .tmx + .tsx + .png)
   └── Identifier les patterns d'organisation (dossiers nommés, conventions)

3. CLASSIFICATION
   ├── Catégoriser chaque asset (sprite, tileset, bg, ui, audio, font, map, etc.)
   ├── Grouper les fichiers liés (spritesheet + descriptor, atlas + image, etc.)
   ├── Extraire les métadonnées (taille frames, nb frames, animations, tags)
   └── Détecter les paramètres (taille tuile, frame size) par heuristique ou nom

4. INDEXATION
   ├── Générer le manifest JSON (toile-asset-manifest.json)
   ├── Générer les thumbnails pour le browser
   ├── Parser les descripteurs (JSON, XML, .fnt, TMX, LDtk)
   └── Stocker les chemins relatifs

5. PRÉVISUALISATION
   ├── Sprites : afficher la première frame + lecture animation
   ├── Tilesets : afficher la grille avec les tuiles individuelles
   ├── Backgrounds : afficher le composite des couches
   ├── Audio : lecteur inline avec waveform
   ├── Fonts : preview "The quick brown fox..."
   └── Maps : rendu miniature du niveau
```

### 10.2 Le Manifest (toile-asset-manifest.json)

Chaque pack importé génère un manifest qui indexe tout son contenu :

```json
{
  "manifest_version": "1.0",
  "pack": {
    "name": "Medieval Platformer Kit",
    "author": "CraftPix",
    "license": "Royalty Free",
    "source": "craftpix.net",
    "importDate": "2026-03-17T10:30:00Z",
    "originalPath": "Medieval_Platformer_Game_Kit.zip",
    "tags": ["medieval", "platformer", "pixel-art", "16x16"]
  },
  "assets": [
    {
      "id": "hero1_idle",
      "type": "sprite",
      "subtype": "spritesheet_grid",
      "path": "Characters/Hero1/Idle.png",
      "thumbnailPath": ".toile/thumbs/hero1_idle.png",
      "metadata": {
        "frameWidth": 32,
        "frameHeight": 32,
        "frameCount": 6,
        "columns": 6,
        "rows": 1,
        "animations": [
          { "name": "idle", "frames": [0,1,2,3,4,5], "fps": 10, "loop": true }
        ]
      },
      "tags": ["character", "hero", "idle", "animated"]
    },
    {
      "id": "terrain_tileset",
      "type": "tileset",
      "subtype": "grid",
      "path": "Tileset/tileset.png",
      "thumbnailPath": ".toile/thumbs/terrain_tileset.png",
      "metadata": {
        "tileWidth": 16,
        "tileHeight": 16,
        "columns": 16,
        "rows": 16,
        "tileCount": 256,
        "spacing": 0,
        "margin": 0
      },
      "tags": ["terrain", "tileset", "ground", "walls"]
    },
    {
      "id": "forest_background",
      "type": "background",
      "subtype": "parallax",
      "path": "Background/",
      "thumbnailPath": ".toile/thumbs/forest_bg.png",
      "metadata": {
        "layers": [
          { "path": "Background/1.png", "depth": 0.0, "scrollFactor": 0.1 },
          { "path": "Background/2.png", "depth": 0.33, "scrollFactor": 0.3 },
          { "path": "Background/3.png", "depth": 0.66, "scrollFactor": 0.6 },
          { "path": "Background/4.png", "depth": 1.0, "scrollFactor": 1.0 }
        ]
      },
      "tags": ["background", "parallax", "forest"]
    },
    {
      "id": "level1_map",
      "type": "tilemap",
      "subtype": "tiled_tmx",
      "path": "Maps/level1.tmx",
      "metadata": {
        "width": 100, "height": 30,
        "tileWidth": 16, "tileHeight": 16,
        "orientation": "orthogonal",
        "tilesets": ["terrain_tileset"],
        "layerCount": 3
      },
      "tags": ["map", "level", "level1"]
    },
    {
      "id": "ui_button",
      "type": "gui",
      "subtype": "button_states",
      "path": "GUI/Buttons/",
      "thumbnailPath": ".toile/thumbs/ui_button.png",
      "metadata": {
        "states": {
          "normal": "GUI/Buttons/button_normal.png",
          "hover": "GUI/Buttons/button_hover.png",
          "pressed": "GUI/Buttons/button_pressed.png",
          "disabled": "GUI/Buttons/button_disabled.png"
        },
        "nineSlice": { "left": 8, "top": 8, "right": 8, "bottom": 8 }
      },
      "tags": ["ui", "button", "interactive"]
    },
    {
      "id": "pixel_font",
      "type": "font",
      "subtype": "bmfont_text",
      "path": "Fonts/pixel_font.fnt",
      "metadata": {
        "face": "Pixel",
        "size": 16,
        "pages": ["Fonts/pixel_font_0.png"],
        "charCount": 95
      },
      "tags": ["font", "pixel", "bitmap"]
    },
    {
      "id": "jump_sfx",
      "type": "audio",
      "subtype": "sfx",
      "path": "Audio/SFX/jump.wav",
      "metadata": {
        "format": "wav",
        "duration": 0.3,
        "sampleRate": 44100,
        "channels": 1
      },
      "tags": ["sfx", "player", "jump"]
    }
  ]
}
```

### 10.3 Détection et classification automatique

L'importeur doit être capable de classifier automatiquement les assets en se basant sur :

**1. Extension du fichier :**

| Extension | Type probable |
|-----------|---------------|
| `.png`, `.webp`, `.jpg` | Image (sprite, tileset, bg, ui, ou icon) |
| `.aseprite`, `.ase` | Sprite animé (Aseprite natif) |
| `.json` + `.png` | Spritesheet (JSON Hash/Array) ou skeleton (Spine/DragonBones) |
| `.tmx`, `.tmj` | Tilemap (Tiled) |
| `.tsx`, `.tsj` | Tileset (Tiled externe) |
| `.ldtk` | Projet LDtk (levels + tilesets) |
| `.atlas` + `.png` | Atlas Spine ou LibGDX |
| `.fnt` | Bitmap font (BMFont) |
| `.ttf`, `.otf`, `.woff2` | Police vectorielle |
| `.wav`, `.ogg`, `.mp3`, `.flac` | Audio |
| `.plist` | Spritesheet Cocos2D ou particules Cocos2D |
| `.xml` | Starling atlas ou Spriter SCML |
| `.skel` | Spine binary |
| `.scml` | Spriter XML |
| `.scon` | Spriter JSON |
| `.model3.json` | Live2D |

**2. Nom de fichier et chemin :**

| Pattern dans le chemin | Classification |
|-----------------------|----------------|
| `character/`, `player/`, `enemy/`, `npc/` | Sprite → personnage |
| `tile/`, `tileset/`, `terrain/` | Tileset |
| `map/`, `level/`, `world/` | Tilemap |
| `bg/`, `background/`, `parallax/` | Background |
| `ui/`, `gui/`, `hud/`, `menu/` | GUI element |
| `icon/`, `item/`, `inventory/` | Icon |
| `fx/`, `vfx/`, `effect/`, `particle/` | VFX |
| `font/` | Font |
| `sfx/`, `sound/`, `audio/`, `music/` | Audio |
| `prop/`, `object/`, `decoration/` | Prop |

**3. Contenu du fichier (pour les images PNG ambiguës) :**
- **Ratio d'aspect très large (>4:1)** → probable strip horizontal de sprite
- **Ratio d'aspect carré + grande taille divisible par 8/16/32** → probable tileset
- **Ratio d'aspect très large + très haute** → probable background
- **Image petite (< 128px de côté)** → probable icône ou sprite individuel
- **Image avec beaucoup de transparence en grille régulière** → probable spritesheet

**4. Taille de frame (pour les spritesheets en grille) :**
- Pattern dans le nom : `(32x32)`, `_32x32`, `-32` → taille de frame
- Si l'image est un strip (1 ligne), la hauteur = frame height
- Diviser par les tailles standard (8, 16, 32, 48, 64) pour trouver un bon diviseur

### 10.4 Asset Browser (UI)

L'Asset Browser est l'interface dans l'éditeur Toile permettant de parcourir et utiliser les assets importés.

**Fonctionnalités essentielles :**

| Feature | Description |
|---------|-------------|
| **Vue en grille** | Thumbnails de tous les assets avec nom |
| **Filtres par type** | Sprites, Tilesets, Backgrounds, UI, Icons, Audio, Fonts, Maps |
| **Filtres par pack** | Afficher les assets d'un seul pack ou tous |
| **Filtres par tag** | Recherche par tags (ex: "medieval", "enemy", "animated") |
| **Recherche texte** | Filtrer par nom de fichier ou tag |
| **Preview inline** | Clic → preview animée (sprites), lecture audio, preview de tileset |
| **Détail/Métadonnées** | Panneau latéral avec toutes les infos (taille, frames, format, etc.) |
| **Drag & Drop** | Glisser un asset sur la scène ou un composant d'entité |
| **Import rapide** | Bouton pour importer un nouveau pack (ZIP ou dossier) |
| **Re-scan** | Re-analyser un pack après modification externe |
| **Édition metadata** | Corriger la classification, la taille de frame, les tags |

**Interactions avec les entités Toile :**

| Action | Résultat |
|--------|----------|
| Drop sprite sur scène | Créer une entité avec SpriteRenderer configuré |
| Drop tileset sur TileMap | Assigner le tileset au composant TileMap |
| Drop background sur scène | Créer un ParallaxBackground configuré |
| Drop audio sur entité | Ajouter un AudioSource avec le clip |
| Drop font sur TextRenderer | Changer la police du texte |
| Drop tilemap sur scène | Importer le niveau complet (layers, objets, etc.) |
| Drop UI element sur Canvas | Créer un UIElement configuré (avec 9-slice si applicable) |

### 10.5 Intégration avec les entités Toile

Chaque type d'asset se connecte à un ou plusieurs **composants** de l'entité Toile :

```
Asset Type          →    Composant Toile
──────────────────────────────────────────────
Sprite/Spritesheet  →    SpriteRenderer (frame-based animation)
Skeleton animation  →    SkeletalRenderer (bone animation)
Tileset + Tilemap   →    TileMapRenderer
Background/Parallax →    ParallaxBackground
GUI Element         →    UISprite / UIButton / UIPanel / UIBar
Icon                →    SpriteRenderer (static)
Bitmap Font         →    TextRenderer.font
TTF/OTF Font        →    TextRenderer.font (rasterisé)
Audio SFX           →    AudioSource.clip
Audio Music         →    MusicPlayer.track
Particle System     →    ParticleEmitter.config
Prop                →    SpriteRenderer + optionnel Collider
```

---

## 11. Modèle de données interne

### Structures additionnelles (complément au document sprites)

```
ToileAssetLibrary
├── packs: Map<string, ToileAssetPack>
├── allAssets: Map<string, ToileAsset>     // index global par ID
├── byType: Map<AssetType, ToileAsset[]>   // index par type
└── searchIndex: SearchIndex               // index pour recherche texte/tags

ToileAssetPack
├── id: string                             // identifiant unique du pack
├── name: string
├── author: string
├── license: string
├── sourcePlatform: string                 // "itch.io", "kenney", "craftpix", etc.
├── importDate: Date
├── rootPath: string                       // chemin racine du pack sur disque
├── manifestPath: string                   // chemin du manifest JSON
├── assets: ToileAsset[]
└── tags: string[]

ToileAsset
├── id: string                             // unique dans la library
├── packId: string                         // référence au pack parent
├── type: AssetType                        // sprite, tileset, tilemap, bg, gui, icon, audio, font, vfx, prop
├── subtype: string                        // précision (spritesheet_grid, bmfont_text, parallax, etc.)
├── name: string                           // nom d'affichage
├── path: string                           // chemin relatif dans le pack
├── absolutePath: string                   // chemin absolu sur disque
├── thumbnailPath: string                  // chemin du thumbnail généré
├── metadata: AssetMetadata                // données spécifiques au type (union)
├── tags: string[]                         // tags de classification
├── relatedAssets: string[]                // IDs des assets liés (atlas→image, map→tileset)
└── userNotes: string                      // notes de l'utilisateur

AssetType = "sprite" | "tileset" | "tilemap" | "background" | "gui" | "icon"
          | "audio" | "font" | "vfx" | "prop" | "skeleton" | "data"

AssetMetadata (union type selon AssetType)
├── SpriteMetadata
│   ├── frameWidth, frameHeight: number
│   ├── frameCount: number
│   ├── columns, rows: number
│   ├── animations: AnimationDef[]
│   ├── sourceFormat: string               // "grid", "json_hash", "json_array", "aseprite", etc.
│   └── descriptorPath: string | null      // chemin du JSON/XML descripteur
├── TilesetMetadata
│   ├── tileWidth, tileHeight: number
│   ├── columns, rows: number
│   ├── tileCount: number
│   ├── spacing, margin: number
│   ├── animatedTiles: AnimatedTileDef[]
│   ├── wangSets: WangSetDef[]
│   └── sourceFormat: string               // "grid", "tiled_tsx", "ldtk", etc.
├── TilemapMetadata
│   ├── width, height: number              // en tuiles
│   ├── tileWidth, tileHeight: number
│   ├── orientation: string
│   ├── layerCount: number
│   ├── tilesetRefs: string[]              // IDs des tilesets utilisés
│   └── sourceFormat: string               // "tiled_tmx", "tiled_json", "ldtk"
├── BackgroundMetadata
│   ├── width, height: number
│   ├── isParallax: bool
│   ├── layers: ParallaxLayerDef[]
│   └── tileable: { x: bool, y: bool }
├── GuiMetadata
│   ├── elementType: string                // "button", "panel", "bar", "slider", etc.
│   ├── states: Map<string, string>        // état → chemin image
│   └── nineSlice: { l, t, r, b } | null
├── AudioMetadata
│   ├── format: string
│   ├── duration: number
│   ├── sampleRate: number
│   ├── channels: number
│   ├── category: "sfx" | "music" | "ambient"
│   └── loop: bool
├── FontMetadata
│   ├── format: "bmfont_text" | "bmfont_xml" | "ttf" | "otf"
│   ├── face: string
│   ├── size: number
│   ├── pages: string[]                    // chemins des images atlas
│   └── charCount: number
└── VfxMetadata
    ├── vfxType: "spritesheet" | "particle_plist" | "particle_json"
    ├── frameCount: number                 // si spritesheet
    └── emitterConfig: object | null       // si système de particules
```

---

## 12. Tableau récapitulatif des formats

### Formats de Tilemaps

| Format | Extension | Outil | Licence | Complexité parser |
|--------|-----------|-------|---------|-------------------|
| Tiled TMX | `.tmx` | Tiled | Libre (BSD) | ★★★ |
| Tiled JSON | `.tmj` | Tiled | Libre (BSD) | ★★☆ |
| Tiled TSX (tileset) | `.tsx` | Tiled | Libre (BSD) | ★★☆ |
| LDtk | `.ldtk` | LDtk | Libre (MIT) | ★★★ |
| LDtk Super Simple | `.ldtk` + PNGs | LDtk | Libre (MIT) | ★☆☆ |

### Formats de Fonts

| Format | Extension | Outil | Licence | Complexité parser |
|--------|-----------|-------|---------|-------------------|
| BMFont Text | `.fnt` | BMFont, Hiero, etc. | Libre | ★★☆ |
| BMFont XML | `.fnt`, `.xml` | BMFont, ShoeBox | Libre | ★★☆ |
| TTF/OTF | `.ttf`, `.otf` | Tout éditeur | Libre | ★★★★ (rasterisation) |

### Formats Audio

| Format | Extension | Licence | Complexité |
|--------|-----------|---------|------------|
| WAV | `.wav` | Libre | ★☆☆ |
| OGG Vorbis | `.ogg` | Libre | ★★☆ (dépendance vorbis) |
| MP3 | `.mp3` | Libre | ★★☆ (dépendance mp3) |
| FLAC | `.flac` | Libre | ★★☆ |

### Formats VFX / Particles

| Format | Extension | Outil | Complexité |
|--------|-----------|-------|------------|
| Spritesheet VFX | `.png` | Tout éditeur | ★☆☆ (=sprite classique) |
| Cocos2D Particles | `.plist` | Particle Designer | ★★☆ |

---

## 13. Priorités d'implémentation

### Phase 1 — Fondations (MVP Asset Library)

1. **Import de dossier/ZIP** avec scan récursif et détection de types par extension
2. **Manifest JSON** : génération automatique, stockage, rechargement
3. **Classification automatique** basée sur les chemins et noms de fichiers
4. **Thumbnail generation** pour les images (première frame pour les spritesheets)
5. **Asset Browser basique** : vue en grille, filtres par type, recherche texte
6. **Drag & Drop → SpriteRenderer** : glisser un sprite sur une entité

### Phase 2 — Tilemaps & Backgrounds

7. **Import Tiled TMX/JSON** : parser les tilemaps + tilesets associés
8. **Import Tiled TSX/TSJ** : tilesets externes
9. **Import LDtk** : parser les niveaux et tilesets
10. **Import Parallax backgrounds** : détection des couches, configuration scrolling
11. **Tileset Browser** : preview des tuiles individuelles dans le browser

### Phase 3 — Formats enrichis

12. **Import BMFont** (.fnt text + XML) : parser le descripteur, lier les pages atlas
13. **Import Audio** : indexation WAV/OGG/MP3 avec métadonnées (durée, sample rate)
14. **Import GUI/UI** : détection des états (normal/hover/pressed), données 9-slice
15. **Preview enrichie** : animation des sprites, lecture audio inline, preview font

### Phase 4 — Intelligence & Intégration profonde

16. **Heuristiques de frame size** : détection automatique de la taille des frames
17. **Auto-tagging** : classification intelligente basée sur le contenu (détection de patterns)
18. **Import Tiled complet** : objets, propriétés custom, tile animations, Wang sets
19. **Import LDtk complet** : entités, enums, auto-layers
20. **Drag & Drop avancé** : tilemap → scène complète, background → parallax configuré

### Phase 5 — Écosystème

21. **Pack registry** : catalogue en ligne de packs compatibles Toile
22. **Import depuis URL** : télécharger directement depuis itch.io, Kenney, etc.
23. **Export de pack** : créer un pack Toile redistribuable
24. **Versioning** : suivi des mises à jour de packs

---

## Notes d'implémentation pour Claude Code

### Architecture suggérée (extension de l'architecture existante)

```
toile/
├── asset_library/
│   ├── mod.rs (ou index.ts)           // API publique de la library
│   ├── library.rs                      // ToileAssetLibrary, stockage, index
│   ├── manifest.rs                     // Génération/parsing du manifest JSON
│   ├── scanner.rs                      // Scan récursif + détection de types
│   ├── classifier.rs                   // Classification automatique
│   ├── thumbnail.rs                    // Génération de thumbnails
│   ├── browser.rs                      // Logique du browser (filtres, recherche)
│   └── importers/
│       ├── tilemap/
│       │   ├── tiled_tmx.rs           // Parser TMX (XML)
│       │   ├── tiled_json.rs          // Parser TMJ (JSON)
│       │   ├── tiled_tsx.rs           // Parser TSX/TSJ
│       │   └── ldtk.rs               // Parser LDtk
│       ├── font/
│       │   ├── bmfont_text.rs         // Parser BMFont format texte
│       │   └── bmfont_xml.rs          // Parser BMFont format XML
│       ├── audio/
│       │   └── audio_metadata.rs      // Extraction métadonnées audio
│       ├── parallax/
│       │   └── parallax_detector.rs   // Détection et configuration des parallax
│       └── gui/
│           └── gui_detector.rs        // Détection des composants UI
├── importers/                          // (existant - sprites & skeletal)
│   ├── spritesheet/
│   └── skeletal/
└── ...
```

### Points d'attention

1. **Performance du scan** : pour les gros packs (60 000+ fichiers chez Kenney), le scan doit être asynchrone avec barre de progression. Utiliser le manifest en cache pour ne pas re-scanner à chaque ouverture.

2. **Gestion des chemins** : stocker des chemins relatifs au pack dans le manifest. Les chemins absolus sont calculés à la volée. Cela permet de déplacer le pack sans casser les références.

3. **Thumbnails** : les générer dans un dossier `.toile/thumbs/` à côté du manifest. Taille recommandée : 128×128 pixels. Pour les spritesheets, extraire la première frame. Pour les tilesets, montrer la grille entière redimensionnée.

4. **Tiled TMX** : la principale complexité est le système de GIDs (global tile IDs) qui nécessite de résoudre les références entre tilesets. Les flip bits dans les GIDs doivent être masqués avant de chercher la tuile.

5. **LDtk** : ignorer la section `defs` (sauf `tilesets` et `enums`) — les données utiles sont dupliquées dans les `levels` via les champs `__` (double underscore).

6. **Détection de format ambigu** : quand un fichier `.json` est trouvé, utiliser la cascade de détection décrite dans le document de référence des sprites (section 1, "Détection automatique du format") + les nouvelles clés pour LDtk (`"__header__"` + `"fileType": "LDtk Project JSON"`), Tiled JSON (`"type": "map"` + `"tiledversion"`) et Tiled tileset JSON (`"type": "tileset"`).

7. **Liens inter-assets** : quand un TMX référence un TSX qui référence un PNG, les trois doivent être liés dans le manifest (`relatedAssets`). Idem pour les BMFont (.fnt → .png) et les spritesheets (JSON → PNG).

8. **Re-import non destructif** : si l'utilisateur modifie des métadonnées (corrige une frame size, ajoute des tags), ces modifications doivent survivre à un re-scan du pack. Stocker les overrides utilisateur séparément du manifest auto-généré.
