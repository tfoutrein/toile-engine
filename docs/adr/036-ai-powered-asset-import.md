# ADR-036 : Import d'assets assiste par IA

- **Statut :** Proposee
- **Date :** 2026-03-28
- **Concerne :** v0.5+

## Contexte

Le systeme d'import d'assets (ADR-032) repose sur des heuristiques en 3 passes (extension, chemin, grille) qui fonctionnent bien pour les packs standards (Kenney, itch.io) mais echouent regulierement :

| Probleme | Cause | Impact |
|----------|-------|--------|
| Taille de frame incorrecte | Grid detection heuristique | Sprites decoupes au mauvais endroit |
| Animations non detectees | Pas de parsing des noms d'animation | Pas d'idle/walk/run auto-configure |
| Classification erronee | Ambiguite path/extension | Sprites classes comme tileset ou inversement |
| FPS par defaut (10) | Pas d'info sur la vitesse d'animation | Animations trop rapides ou trop lentes |
| Atlas XML/JSON ignores | TexturePacker/Starling non parses | Frames manquees |

Or, la plupart des packs d'assets contiennent des **README**, **LICENSE**, et parfois des **metadata** qui decrivent precisement la structure. Ces fichiers sont affiches dans le File Browser mais jamais exploites pour l'import.

## Decision

**Ajouter une etape d'analyse IA avant l'import qui lit le README, la structure de fichiers, et optionnellement des images echantillons pour produire un "Import Plan" structure.**

### Architecture

```
┌──────────────────────────────────────────────────────────┐
│                    Import Pipeline                         │
│                                                          │
│  1. Scan directory                                       │
│     └─ Liste des fichiers + tailles                      │
│                                                          │
│  2. Detect documentation                                 │
│     └─ README.md, LICENSE, .txt, atlas descriptors       │
│                                                          │
│  3. ── AI Analysis (nouveau) ──                          │
│     │  Envoie a l'IA :                                   │
│     │  - Contenu du README                               │
│     │  - Arborescence des fichiers                       │
│     │  - Dimensions des images                           │
│     │  - Contenu des fichiers metadata (.json, .xml)     │
│     │                                                    │
│     │  L'IA retourne un Import Plan :                    │
│     │  - Classification corrigee par fichier             │
│     │  - Taille de frame pour chaque spritesheet         │
│     │  - Noms d'animations + indices de frames           │
│     │  - FPS recommande par animation                    │
│     │  - Tags suggerees                                  │
│     ▼                                                    │
│  4. Classification (existant, mais guidee par le plan)   │
│     └─ Heuristiques + overrides du plan IA               │
│                                                          │
│  5. Metadata + thumbnails + manifest                     │
└──────────────────────────────────────────────────────────┘
```

### 1. Collecte du contexte

Avant d'appeler l'IA, on collecte :

```rust
struct PackContext {
    /// Contenu des fichiers README/documentation (tronque a 4000 chars chacun).
    readme_contents: Vec<(String, String)>, // (filename, content)
    /// Arborescence complete des fichiers avec tailles.
    file_tree: Vec<(String, u64, Option<(u32, u32)>)>, // (path, size_bytes, image_dims)
    /// Contenu des fichiers metadata (.json, .xml, .fnt, .plist) — tronques.
    metadata_files: Vec<(String, String)>, // (filename, content_preview)
    /// Nombre total de fichiers par extension.
    extension_counts: HashMap<String, usize>,
}
```

**Ce qui est envoye a l'IA :**
- README complet (ou tronque si > 4000 chars)
- Arbre des fichiers avec dimensions des images
- Premiers 500 chars de chaque fichier metadata (JSON, XML, FNT, PLIST)
- Resume statistique : "142 PNG, 3 JSON, 1 README.md, 2 XML"

**Ce qui n'est PAS envoye :**
- Les images elles-memes (trop lourd, sauf si on active la vision plus tard)
- Les fichiers audio binaires
- Le contenu complet des gros fichiers metadata

### 2. Prompt IA

```
Tu es un expert en analyse de packs d'assets pour jeux 2D.

Voici un pack a importer :

README:
{readme_content}

Structure des fichiers (chemin, taille, dimensions si image) :
{file_tree}

Fichiers metadata :
{metadata_previews}

Analyse ce pack et produis un Import Plan en JSON :
{
  "pack_description": "Description courte du pack",
  "animations": [
    {
      "file": "Characters/Knight/Knight_Idle.png",
      "frame_width": 32,
      "frame_height": 32,
      "columns": 4,
      "rows": 1,
      "animations": [
        {"name": "idle", "frames": [0,1,2,3], "fps": 8, "looping": true}
      ]
    }
  ],
  "classifications": [
    {"file": "Tiles/Grass.png", "type": "tileset", "tile_width": 16, "tile_height": 16},
    {"file": "UI/Button.png", "type": "gui"}
  ],
  "tags": {
    "Characters/Knight/": ["knight", "player", "character"],
    "Enemies/Goblin/": ["goblin", "enemy"]
  }
}

Regles :
- frame_width/frame_height doivent diviser les dimensions de l'image
- Les noms d'animation standards : idle, walk, run, jump, attack, die, hurt, dash, fall, climb
- Types valides : sprite, tileset, background, gui, icon, vfx, prop
- Si le README indique des tailles de frames, utilise-les
- Si les noms de fichiers contiennent des indices (walk_01, walk_02), ce sont des frames separees d'une meme animation
- FPS typiques : idle=6-8, walk=8-10, run=10-12, attack=12-15
```

### 3. Import Plan (reponse IA)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ImportPlan {
    pack_description: String,
    animations: Vec<AnimationPlan>,
    classifications: Vec<ClassificationOverride>,
    tags: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnimationPlan {
    file: String,
    frame_width: u32,
    frame_height: u32,
    columns: u32,
    rows: u32,
    animations: Vec<AnimationDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClassificationOverride {
    file: String,
    #[serde(rename = "type")]
    asset_type: String,
    tile_width: Option<u32>,
    tile_height: Option<u32>,
}
```

### 4. Integration dans le pipeline

Le plan IA est utilise comme **override** des heuristiques :

```rust
fn import_pack_with_ai(pack_dir: &Path, plan: &ImportPlan) {
    for file in scanned_files {
        // 1. Chercher dans le plan IA
        if let Some(anim_plan) = plan.animations.iter().find(|a| a.file == file.path) {
            // Utiliser les tailles de frame et animations du plan
            metadata = SpriteMetadata {
                frame_width: anim_plan.frame_width,
                frame_height: anim_plan.frame_height,
                columns: anim_plan.columns,
                rows: anim_plan.rows,
                animations: anim_plan.animations.clone(),
            };
        } else if let Some(classif) = plan.classifications.iter().find(|c| c.file == file.path) {
            // Utiliser le type du plan
            asset_type = parse_type(&classif.asset_type);
        } else {
            // Fallback aux heuristiques existantes
            asset_type = classifier::classify(&file);
            metadata = build_metadata_heuristic(&file);
        }
    }
}
```

### 5. UI dans l'editeur

**Avant l'import :**
1. L'utilisateur clique "Import" et selectionne un dossier/ZIP
2. L'editeur scanne le contenu et affiche un apercu
3. Bouton "Analyze with AI" lance l'analyse
4. L'Import Plan s'affiche dans une preview :
   - "Found 12 spritesheets, 3 tilesets, 45 individual sprites"
   - "Detected animations: Knight (idle 4f, walk 6f, run 8f, jump 4f)"
   - L'utilisateur peut valider ou modifier avant import
5. Bouton "Import with AI plan" lance l'import guide

**Sans IA (fallback) :**
- L'import classique (heuristiques) reste disponible
- Bouton "Import (basic)" pour les utilisateurs sans cle API

### 6. Cache du plan

Le plan IA est sauvegarde dans le pack :
```
pack_directory/
  toile-asset-manifest.json    (existant)
  toile-import-plan.json       (nouveau — cache du plan IA)
```

Au re-import, si le plan existe et que les fichiers n'ont pas change, on le reutilise sans rappeler l'IA.

### 7. Vision (Phase 2)

Si le provider supporte la vision (Anthropic Claude, OpenAI GPT-4o), on peut aussi envoyer des miniatures des spritesheets pour que l'IA "voie" les frames :

- Redimensionner chaque spritesheet a max 256x256
- Encoder en base64
- Ajouter au prompt : "Voici l'image de Knight_Walk.png, determine le nombre de frames et la taille"

Cela permet de detecter les grilles meme quand le README ne les decrit pas.

## Phasage

### Phase 1 : Analyse README + structure (immediat)
- Collecter README + arbre de fichiers + metadata previews
- Envoyer a l'IA via le provider configure (Anthropic ou OpenAI-compat)
- Parser le plan JSON retourne
- Appliquer comme overrides dans le pipeline d'import
- UI : bouton "Analyze with AI" dans le dialog d'import
- Cache du plan dans `toile-import-plan.json`

### Phase 2 : Vision sur les spritesheets (v1.0)
- Envoyer des miniatures des images ambigues
- L'IA detecte visuellement les grilles et frames
- Utile pour les packs sans README ni metadata

### Phase 3 : Apprentissage des patterns (v1.0+)
- Memoriser les corrections manuelles de l'utilisateur
- Enrichir les heuristiques avec les patterns appris
- L'IA apprend le style de chaque "editeur" de pack (Kenney, itch.io, OpenGameArt)

## Options considerees

### Option A : Parser plus de formats metadata (rejetee seule)
- Ajouter des parsers pour TexturePacker JSON, Starling XML, Cocos2D plist
- Insuffisant : ne couvre pas les packs sans metadata
- Combine avec l'IA comme fallback

### Option B : IA seule sans heuristiques (rejetee)
- Trop lent et couteux pour chaque import
- Necessite une connexion internet
- L'IA peut halluciner des tailles de frame incorrectes

### Option C : Heuristiques + IA comme override (retenue)
- Les heuristiques gerent le cas commun rapidement
- L'IA corrige les cas ambigus
- L'utilisateur valide avant import
- Fallback gracieux sans IA (heuristiques seules)

## Consequences

### Positives
- Les packs complexes sont importes correctement du premier coup
- Les README sont enfin exploites (pas juste affiches)
- Les animations sont correctement nommees et configurees
- Reduction drastique des corrections manuelles post-import

### Negatives
- Cout API pour chaque analyse (1-2 appels par pack)
- Latence reseau (2-5 secondes pour l'analyse)
- Necessite une cle API configuree
- L'IA peut se tromper sur les tailles de frame → l'utilisateur doit valider

### Risques
- L'IA pourrait halluciner des animations qui n'existent pas → validation obligatoire
- Les packs tres gros (1000+ fichiers) generent un prompt trop long → tronquer le file tree aux 200 premiers fichiers + resume
- Le cache du plan peut devenir obsolete si les fichiers changent → invalider si les timestamps changent
