# Outils de Création pour Moteur 2D — Recherche Complète

## Table des matières
1. [Éditeurs visuels des moteurs 2D existants](#1-éditeurs-visuels-des-moteurs-2d-existants)
2. [Frameworks UI pour construire un éditeur](#2-frameworks-ui-pour-construire-un-éditeur)
3. [Systèmes de scripting visuel](#3-systèmes-de-scripting-visuel)
4. [Éditeur de scènes / niveaux](#4-éditeur-de-scènes--niveaux)
5. [Éditeur d'animations](#5-éditeur-danimations)
6. [Éditeur de particules](#6-éditeur-de-particules)
7. [Gestion des assets](#7-gestion-des-assets)
8. [Langages de scripting pour non-devs](#8-langages-de-scripting-pour-non-devs)
9. [Hot-reloading](#9-hot-reloading)
10. [Analyse des exemples réussis](#10-analyse-des-exemples-réussis)
11. [Recommandations MVP](#11-recommandations-mvp)

---

## 1. Éditeurs visuels des moteurs 2D existants

### Godot
- **L'éditeur est construit avec son propre moteur** — approche "dogfooding" qui garantit la robustesse de l'UI
- **Architecture en arbre de scènes** : tout est un nœud dans un arbre (Sprite2D, CollisionShape2D, AudioStreamPlayer...). Intuitif car ça reflète la pensée des artistes sur les couches
- **Panneau inspecteur** : sélectionner un nœud affiche ses propriétés avec des widgets appropriés (color pickers, curve editors, sélecteurs de ressources)
- **Canvas 2D** : manipulation directe des sprites avec des gizmos (poignées move/rotate/scale)
- **Accessibilité** : GDScript est Python-like et accessible. Les signaux sont connectables via l'UI sans code

### Unity 2D
- **Architecture à composants** : GameObjects avec composants attachés. L'inspecteur montre tous les composants sérialisés
- **Éditeur de tilemap** : peinture de tiles intégrée avec auto-tiling, rule tiles, palette
- **Éditeur de sprites** : découpage de sprite sheets, édition de pivot points, formes physiques 2D
- **Système de prefabs** : templates réutilisables que les non-programmeurs peuvent instancier et personnaliser

### GameMaker
- **Room Editor** : placement d'instances d'objets, layers de tiles, configuration viewports/caméras
- **Modèle objet/événement** : les "Objects" répondent à des "Events" (Create, Step, Draw, Collision, Key Press). Chaque événement contient des actions ou du code GML
- **Drag-and-drop (DnD)** : alternative visuelle au GML. L'une des interfaces non-programmeur les plus réussies en 2D
- **Éditeur de sprites** : éditeur pixel art intégré avec support d'animation
- **Sequences** : système d'animation timeline pour les cutscenes

### RPG Maker
- **Domain-specific** : conçu exclusivement pour les JRPGs → réduit massivement la complexité
- **Éditeur de maps** : peinture par tiles avec auto-tile. Couches multiples (sol, bâtiments, décorations, événements)
- **Système d'événements** : le cœur logique. Des événements placés sur la carte contiennent des listes de commandes (Show Text, Change Variable, Conditional Branch, Move Route). C'est de la programmation visuelle via une interface de liste de commandes
- **Base de données** : éditeur structuré pour les données (personnages, classes, compétences, items, ennemis, états)
- **Assets intégrés** : livré avec tilesets, sprites, musiques, effets sonores

### Construct 3
- **Browser-based** : tourne entièrement dans le navigateur, zéro friction d'installation
- **Event sheets** : la feature signature. La logique est exprimée comme des paires condition-action dans un format tableur. Pas de syntaxe à mémoriser
- **Behaviors** : packages de logique pré-construits (Platform, 8-Direction, Physics, Bullet) attachables sans scripting
- **Layout editor** : WYSIWYG avec drag-and-drop

### Principes d'accessibilité pour non-programmeurs

| Principe | Description |
|----------|-------------|
| **Manipulation directe** | Cliquer, glisser, redimensionner directement sur un canvas |
| **Feedback immédiat** | Les changements sont visibles instantanément (WYSIWYG) |
| **Divulgation progressive** | Les choses simples sont simples ; les features avancées sont disponibles mais pas écrasantes |
| **Défauts sensés** | Les objets doivent "juste marcher" quand placés |
| **Découvrabilité** | Les features trouvables via menus et inspecteurs, pas la documentation |
| **Vocabulaire du domaine** | Utiliser les termes des artistes (layers, sprites, frames) pas ceux des programmeurs (instances, buffers, shaders) |
| **Undo partout** | Les non-programmeurs expérimentent plus et ont besoin d'un undo/redo robuste |
| **Templates et presets** | Partir d'un exemple fonctionnel est plus facile que construire de zéro |

---

## 2. Frameworks UI pour construire un éditeur

### Dear ImGui

- **Avantages** : paradigme immediate-mode parfait pour les outils de jeu. Incroyablement rapide à itérer. Se rend via le backend de rendu du jeu. Écosystème massif d'extensions (node editors, color pickers, plot widgets). Battle-tested en studio AAA (Blizzard, Epic).
- **Inconvénients** : look utilitaire par défaut. Accessibilité (lecteurs d'écran, navigation clavier) faible. Pas conçu pour les applications end-user.
- **Idéal pour** : éditeurs internes/dev, outils debug, prototypage rapide

### egui (Rust)

- **Avantages** : Rust pur, immediate-mode. Si le moteur est Rust, c'est le choix naturel. Support WASM intégré via eframe. Bonne intégration wgpu.
- **Inconvénients** : écosystème plus petit que Dear ImGui. Performance préoccupante pour des UI très complexes. Moins battle-tested.
- **Idéal pour** : moteurs Rust avec potentiel de déploiement web

### Qt

- **Avantages** : le gold standard des applications desktop professionnelles. Maya, Houdini, Substance Designer l'utilisent. Widget set extrêmement mature : tree views, tables, docking, rich text, accessibilité. Look professionnel d'emblée.
- **Inconvénients** : dépendance lourde. Complexité de licence (LGPL ou commercial cher). Intégration avec le rendu du jeu complexe. Overkill pour des éditeurs simples.
- **Idéal pour** : éditeurs grade professionnel où le polish et l'accessibilité importent

### Electron / Tauri

- **Electron** : UI avec HTML/CSS/JS — l'écosystème UI le plus riche. Mais gourmand en mémoire (embarque Chromium).
- **Tauri** : beaucoup plus léger (utilise le webview système). Backend Rust. Binaires plus petits. Peut appeler du Rust directement depuis le frontend.
- **Idéal pour** : éditeurs qui priorisent la richesse UI et les technologies web

### Matrice de comparaison

| Facteur | Dear ImGui | egui | Qt | Electron/Tauri |
|---------|-----------|------|-----|----------------|
| Intégration rendu jeu | Excellente | Excellente | Modérée | Faible-Modérée |
| Polish UI end-user | Faible | Faible-Moyen | Excellent | Excellent |
| Vitesse de développement | Très rapide | Rapide | Modérée | Rapide |
| Accessibilité non-dev | Faible | Faible | Haute | Haute |
| Écosystème widgets | Large | Croissant | Massif | Massif |
| Performance mémoire | Excellente | Bonne | Bonne | Faible (Electron) / Bonne (Tauri) |

---

## 3. Systèmes de scripting visuel

### Node-Based (graphe de nœuds)

**Exemples** : Unreal Blueprints, Godot VisualScript, Blender shader/geometry nodes, Unity Visual Scripting

- Les nœuds représentent des opérations (Add, Compare, Branch, Spawn). Les fils transportent données ou flux d'exécution
- **Forces** : visuel et spatial, pas d'erreurs de syntaxe (connexions type-checkées), auto-documentant
- **Faiblesses** : problème "spaghetti" (logique complexe = enchevêtrement de fils plus difficile à lire que du code), verbeux (a = b + c nécessite plusieurs nœuds), découvrabilité limitée dans une grande bibliothèque de nœuds
- **Surprise** : les recherches montrent que le node-based n'est PAS toujours plus accessible que du simple code pour les débutants. La complexité spatiale peut être écrasante.

### Event Sheets (style Construct) — **Recommandé**

- Logique organisée en lignes : conditions (gauche) et actions (droite). Quand les conditions sont remplies, les actions s'exécutent.
- **Forces** : extrêmement lisible ("SI le joueur touche l'ennemi, ALORS détruire l'ennemi et ajouter 10 au score"). Pas de problème spaghetti. Très rapide à écrire. Barrière d'entrée la plus basse.
- **Faiblesses** : moins flexible que le node-based pour les flux de données complexes. Peut devenir long (mitigé par le groupement et les fonctions).
- **Implémentation** : plus simple qu'un éditeur de graphe de nœuds. Nécessite un système de plugins condition/action extensible et un parser d'expressions.

### Behavior Trees

- Structure en arbre de nœuds pour les décisions IA : Selectors (essayer les enfants jusqu'au succès), Sequences (exécuter en ordre), Conditions, Actions.
- **Excellent pour l'IA** spécifiquement, pas pour la logique de jeu générale
- Meilleur comme complément à un autre système de scripting

### State Machines visuelles

- États connectés par des transitions avec conditions. Les objets sont dans un état à la fois.
- **Parfait pour** : états de personnage (idle, walking, jumping, attacking), états de jeu (menu, playing, paused)
- Pas suffisant comme seul système de scripting

### Recommandation

Pour une accessibilité maximale : **event sheets** (style Construct) pour la logique principale + **state machines visuelles** pour la gestion d'états + optionnellement **behavior trees** pour l'IA. Éviter le node-based comme système primaire sauf si le public cible a déjà un background technique.

---

## 4. Éditeur de scènes / niveaux

### Features essentielles — Tier 1 (MVP)

| Feature | Pourquoi essentiel |
|---------|-------------------|
| **Canvas avec pan/zoom** | Naviguer dans des scènes plus grandes que le viewport. Canvas infini fluide. |
| **Placement d'objets (drag & drop)** | Glisser des assets depuis un browser vers le canvas. L'interaction cœur. |
| **Sélection et transformation** | Cliquer, glisser, poignées rotate/scale. Multi-sélection avec box selection. |
| **Snap à la grille** | Essentiel pour les jeux en tiles. Toggle-able avec taille configurable. |
| **Inspecteur de propriétés** | Sélectionner un objet, voir et éditer ses propriétés dans un panneau. |
| **Undo/redo** | Non-négociable. Doit couvrir toutes les opérations. Pattern Command. |
| **Gestion des layers** | Au minimum : ordre front-to-back. Idéalement : layers nommés avec visibilité/verrouillage. |
| **Sauvegarder/charger** | Sérialiser la scène en JSON. |

### Features — Tier 2 (attendues par les utilisateurs)

| Feature | Description |
|---------|-------------|
| **Peinture de tilemap** | Outils brush, fill, eraser. Auto-tiling. Palette de tiles avec prévisualisation. |
| **Copier/coller/dupliquer** | Opérations d'édition standard. |
| **Aligner et distribuer** | Aligner les objets sélectionnés sur les bords/centres. |
| **Arbre d'objets/hiérarchie** | Vue arbre de tous les objets, permettant réordonnement et parentage. |
| **Prefabs/templates** | Sauvegarder un objet configuré comme template réutilisable. |
| **Prévisualisation caméra** | Montrer ce que la caméra de jeu verra. |
| **Raccourcis clavier** | Ctrl+Z, Ctrl+C, Delete, etc. |

### Notes d'implémentation

- **Pattern Command pour undo/redo** : chaque action crée un objet Command avec `execute()` et `undo()`. Stocker une pile.
- **Sérialisation de scène** : JSON (lisible, diff-friendly). Format binaire optionnel pour les grosses scènes.
- **Systèmes de coordonnées** : décider tôt espace écran vs espace monde. L'éditeur doit convertir constamment entre les deux.

---

## 5. Éditeur d'animations

### Animation timeline

- **Vue timeline** : axe horizontal = temps (frames ou secondes), axe vertical = propriétés ou layers
- **Keyframes** : marqueurs sur la timeline indiquant une valeur de propriété à un instant donné. Interpolation entre keyframes.
- **Propriétés animables** : position, rotation, scale, opacité, frame de sprite, teinte de couleur
- **Courbes d'easing** : linear, ease-in, ease-out, bezier. Éditeur visuel de courbes.
- **Onion skinning** : montrer les frames précédentes/suivantes en semi-transparent. Essentiel pour l'animation dessinée à la main.
- **Playback** : play, pause, stop, loop, contrôle de vitesse, scrubbing

### Animation de sprites

- **Éditeur de strip de frames** : afficher toutes les frames d'une animation en séquence
- **Durée par frame** : contrôle de timing par frame
- **Prévisualisation** : playback en boucle à la vitesse du jeu
- **Modes ping-pong et loop**

### Intégration avec les outils externes

#### Aseprite
- Standard de facto pour l'animation pixel art
- Exporte des sprite sheets (atlas) avec métadonnées JSON décrivant positions, durées, tags, slices
- **Approche d'intégration** : importer la paire .json + .png. Parser le JSON pour créer automatiquement les clips d'animation. Surveiller le fichier pour réimporter automatiquement (hot reload).

#### Spine
- Outil d'animation squelettique professionnel
- Exporte JSON ou binaire décrivant bones, slots, attachments, animations, skins, contraintes
- **Approche d'intégration** : utiliser ou porter la bibliothèque runtime Spine officielle
- Runtime open-source mais nécessite une licence Spine pour l'éditeur

### Recommandation MVP

Commencer par un **éditeur simple d'animation frame-based** (strip de frames, durée par frame, prévisualisation) et **import Aseprite** (parser les métadonnées JSON). L'animation timeline de propriétés est Tier 2. L'animation squelettique (Spine) est Tier 3.

---

## 6. Éditeur de particules

### Pourquoi l'édition visuelle est indispensable

Les systèmes de particules sont visuels par nature. Tweaker des nombres dans du code et redémarrer est extrêmement improductif. Un éditeur visuel avec prévisualisation temps réel est quasi obligatoire.

### Features essentielles

| Feature | Description |
|---------|-------------|
| **Prévisualisation temps réel** | Voir l'effet pendant l'édition. Requirement #1. |
| **Forme d'émetteur** | Point, cercle, rectangle, ligne. Gizmo visuel. |
| **Durée de vie** | Min/max lifetime avec indication visuelle. |
| **Taux d'émission** | Particules par seconde, mode burst. |
| **Vélocité** | Vitesse initiale, direction, angle de spread. |
| **Gravité/forces** | Vecteur gravité, vent, attracteurs/répulseurs. |
| **Taille au cours de la vie** | Éditeur de courbe. |
| **Couleur au cours de la vie** | Éditeur de gradient. |
| **Rotation** | Rotation initiale, vélocité angulaire. |
| **Sprite/texture** | Sélection de texture, support sprite sheets animés. |
| **Mode de blend** | Additive, alpha blend, multiply. |
| **Presets** | Bibliothèque d'effets communs (feu, fumée, étincelles, pluie, neige, explosion). |

### Widgets clés à développer

- **Éditeur de courbe** : mini-canvas montrant une courbe de 0 à 1 (lifetime normalisé) en X vers la valeur de propriété en Y. Points de contrôle déplaçables, handles de Bezier. Réutilisable pour l'animation, l'easing audio, etc.
- **Éditeur de gradient** : barre horizontale montrant la transition de couleur. Color stops ajoutables.

### Recommandation MVP

Système de particules avec **édition via inspecteur** (sliders, widgets de courbe, widgets de gradient) et **prévisualisation temps réel** dans le viewport de l'éditeur de scène. Inclure 5-10 presets. Un éditeur standalone de particules est un nice-to-have.

---

## 7. Gestion des assets

### Asset Browser

- **Prévisualisations thumbnails** : montrer sprites, tiles et animations comme thumbnails visuels
- **Hiérarchie de dossiers** : miroir de la structure de fichiers du projet
- **Recherche et filtrage** : par nom, par type (sprites, sons, scripts, scènes)
- **Drag vers le canvas** : glisser un asset directement sur l'éditeur de scène
- **Menus contextuels** : renommer, supprimer, dupliquer, montrer dans l'explorateur, réimporter
- **Vues grille et liste** : toggle entre grille d'icônes (assets visuels) et liste (infos détaillées)

### Pipeline d'import

**Approche recommandée : import transparent (style Godot)**

Les fichiers source vivent dans le répertoire du projet. Le moteur les importe automatiquement dans un format interne (cache dans un dossier .import). Les utilisateurs travaillent avec les fichiers originaux. Les changements déclenchent un réimport.

- **Avantages** : modèle mental simple, fonctionne avec les éditeurs externes (Photoshop, Aseprite)
- **Principe** : l'artiste sauvegarde un PNG, il apparaît dans le moteur. Magique.

### Settings d'import par type

| Type d'asset | Settings clés |
|-------------|--------------|
| **Images/Sprites** | Filtrage (nearest pour pixel art, linear pour HD), atlas packing, découpage sprite sheet |
| **Audio** | Conversion de format, streaming vs préchargé, settings de loop |
| **Polices** | Taille, jeu de caractères, rendu SDF |
| **Tilemaps** | Taille des tiles, config auto-tile, setup collision |

### Génération de thumbnails

- Générer les thumbnails à l'import et les cacher
- Sprites : downscale de l'image
- Animations : utiliser la première frame
- Audio : visualisation de waveform
- Scènes : miniature rendue de la scène

---

## 8. Langages de scripting pour non-devs

### Spectre d'accessibilité

```
Plus Accessible                                    Moins Accessible
     |                                                    |
     v                                                    v
Event Sheets > State Machines visuelles > DSL custom > Lua > Python > JS > C# > C++
```

### Lua
- **Accessibilité** : modérée. Syntaxe simple mais reste un "vrai" langage
- **Forces** : runtime minuscule, trivial à embarquer, excellente performance avec LuaJIT, track record massif (WoW, Roblox, Love2D, Defold)
- **Faiblesses** : tableaux indexés à 1 (déroutant), metatables confusants, messages d'erreur cryptiques

### DSL custom (style GDScript)
- **Accessibilité** : haute. Syntaxe inspirée Python avec built-ins spécifiques au jeu
- **Forces** : conçu pour le game dev → opérations courantes concises. Intégration éditeur serrée (autocomplétion pour les noms de nœuds, signaux...)
- **Faiblesses** : pas utile hors du moteur. Concevoir un bon DSL est difficile.

### JavaScript/TypeScript
- **Accessibilité** : modérée. Largement connu, ressources massives
- **Forces** : la plupart des gens avec une expérience de code connaissent du JS
- **Faiblesses** : pas conçu pour le game dev, "the bad parts"

### Wren
- **Accessibilité** : haute. Syntaxe propre, class-based. Petit langage.
- **Forces** : conçu pour l'embedding dans les jeux. Concurrence par fibers (idéal pour les coroutines de jeu)
- **Faiblesses** : communauté minuscule, ressources limitées, développement ralenti

### Recommandation

Offrir un **DSL custom à syntaxe Python-like** (style GDScript) comme langage de scripting texte primaire, avec un **système de scripting visuel** (event sheets) comme interface primaire pour les non-programmeurs. Le DSL doit être :
- À base d'indentation (pas d'accolades)
- Spécifique au domaine du jeu (`on collision with "enemy":`, `move_toward(target, speed)`)
- Intégré à l'éditeur (autocomplétion, documentation inline, surlignage d'erreurs)
- Optionnellement, Lua comme alternative pour les power-users

---

## 9. Hot-reloading

### Pourquoi c'est crucial

Le hot-reloading est la feature de productivité la plus impactante. Le cycle "changer quelque chose, redémarrer, naviguer jusqu'à l'endroit, tester" est dévastateur pour le flux créatif. Le raccourcir à "changer, voir le résultat immédiatement" transforme l'expérience.

### Quoi hot-reloader

| Type d'asset | Difficulté | Approche |
|-------------|-----------|----------|
| **Sprites/textures** | Facile | File watch + reload texture. Upload GPU rapide. |
| **Audio** | Facile | Recharger le buffer. Redémarrer les instances en cours. |
| **Données de scène/niveau** | Modérée | Re-parser, diff contre l'état actuel, appliquer les changements. Délicat : maintenir l'état runtime. |
| **Code de script** | Difficile | Recharger, re-binder les fonctions. Défi : préserver l'état des objets. |
| **Shaders** | Modérée | Recompiler, swapper le programme. Feedback visuel immédiat et satisfaisant. |
| **Configuration** | Facile | Re-parser, appliquer les nouvelles valeurs. |
| **Tilemaps** | Facile-Modérée | Recharger les données, reconstruire les batches de rendu. |

### Stratégies de hot-reload de scripts

1. **Redémarrage complet avec snapshot d'état** : sérialiser l'état du jeu, recharger tous les scripts, désérialiser. Simple mais peut être lent.
2. **Remplacement au niveau fonction** : remplacer des fonctions individuelles en gardant l'état des objets. Fonctionne bien avec Lua.
3. **Remplacement au niveau module** : recharger un module entier, réinitialiser les objets affectés. Bon compromis.

### Décisions clés

- **Auto-reload sur sauvegarde** (comme Godot) plutôt que bouton de reload
- **Gestion d'erreurs** : quand un script rechargé a des erreurs, le jeu ne doit PAS crasher. Afficher l'erreur en overlay et continuer avec la dernière version fonctionnelle.
- **Préservation d'état** : préserver l'état transform/physique, réinitialiser l'état script

### Recommandation MVP

Commencer par le **hot-reload de textures/sprites** (file watching + re-upload texture) et le **push de propriétés éditeur→jeu** (changer une valeur dans l'inspecteur met à jour le jeu en cours). Le hot-reload de scripts peut être reporté post-MVP.

---

## 10. Analyse des exemples réussis

### RPG Maker — Pourquoi ça marche

1. **Spécificité de domaine extrême** : fait UN type de jeu (JRPGs). Chaque feature est optimisée pour ce genre.
2. **Design dirigé par la base de données** : personnages, items, compétences entrés dans des formulaires structurés, pas construits via du code.
3. **Commandes d'événements, pas de code** : "Show Text" ouvre un dialogue pour taper le texte. "Conditional Branch" offre des dropdowns. Pas de syntaxe, pas d'erreurs de frappe.
4. **Magie de l'auto-tile** : peindre avec une tile "herbe" et les bords/coins sont gérés automatiquement.
5. **Assets intégrés** : livré avec tilesets, sprites, musiques. Un jeu complet possible sans créer un seul asset.
6. **Communauté** : bibliothèque massive de plugins, tilesets, générateurs de personnages.

**Leçon clé** : la contrainte est une feature. Limiter le scope élimine la paralysie du choix.

### GameMaker — Pourquoi ça marche

1. **Double piste de scripting** : DnD pour les débutants, GML pour les intermédiaires/avancés. Le DnD montre même le GML équivalent, enseignant le langage progressivement.
2. **Modèle mental objet/événement** : les objets "sont" des choses, les événements "arrivent" aux choses. Mappe directement la pensée des non-programmeurs.
3. **Éditeur de sprites intégré** : pas besoin de changer d'outil. Réduit la friction énormément.
4. **Export en un bouton** : compiler et exécuter pour toute plateforme cible.
5. **Écosystème de tutoriels** : investissement massif dans les tutoriels officiels + communauté YouTube.

**Leçon clé** : le modèle mental événementiel et les outils intégrés baissent la barrière, GML fournit un chemin de progression.

### Construct 3 — Pourquoi ça marche

1. **Zéro installation** : ouvrir un onglet de navigateur, commencer à faire un jeu. Élimine la plus grande barrière.
2. **Event sheets = génie** : le modèle condition-action est le système de scripting visuel le plus lisible. Se lit comme une liste de tâches.
3. **Les Behaviors abstraient la complexité** : attacher un behavior "Platform" donne instantanément gravité, saut et collision. Les utilisateurs construisent un platformer jouable en minutes.
4. **Prévisualisation instantanée** : appuyer sur play et le jeu tourne dans un autre onglet.
5. **Cohérence visuelle** : chaque dialogue suit les mêmes patterns. Réduit la charge cognitive.

**Leçon clé** : accessibilité à chaque niveau — pas d'installation, pas de syntaxe, feedback instantané, défauts fonctionnels.

### Patterns communs

1. **Fast time-to-fun** : quelque chose qui bouge à l'écran en quelques minutes
2. **Visual-first** : l'interaction primaire est cliquer, glisser, sélectionner dans des menus
3. **Complexité progressive** : les choses simples sont simples, les choses complexes sont possibles
4. **Assets/templates intégrés** : ne pas partir d'un canvas vide
5. **Outils intégrés** : minimiser les changements de contexte entre applications
6. **Communauté forte** : forums, tutoriels, partage d'assets

---

## 11. Recommandations MVP

### Approche par phases

#### Phase 1 — Fondation (Shell de l'éditeur)

**Objectif** : créer, sauvegarder et charger une scène simple avec des sprites placés.

| Composant | Scope |
|-----------|-------|
| **Framework éditeur** | Layout à panneaux dockables, barre de menu, raccourcis clavier. **Dear ImGui** pour le dev rapide (avec theming poussé) ou **Tauri** pour un UI poli web-like. |
| **Éditeur de scène (canvas)** | Pan, zoom, grille. Placer des sprites par drag depuis l'asset browser. Sélectionner, bouger, tourner, scaler avec gizmos. Multi-sélection. |
| **Asset browser** | Basé sur le filesystem. Thumbnails pour les images. Drag-to-canvas. Recherche. |
| **Inspecteur de propriétés** | Propriétés de l'objet sélectionné. Éditer position, rotation, scale, référence sprite. |
| **Undo/redo** | Pattern Command couvrant toutes les opérations. |
| **Sérialisation de scène** | Sauver/charger en JSON. |
| **Bouton Play** | Exécuter la scène courante dans le moteur. |

#### Phase 2 — Logique de jeu

**Objectif** : ajouter du comportement aux objets sans écrire de code.

| Composant | Scope |
|-----------|-------|
| **Système d'event sheets** | Paires condition-action. Conditions : on creation, on step, on key press, on collision. Actions : move, set position, set variable, destroy, spawn, play sound. |
| **Behaviors** | Packages pré-construits : "8-Direction Movement", "Platform Character", "Bullet/Projectile", "Solid", "Physics". Attacher via inspecteur. |
| **Scripting simple** | Langage embarqué minimal (Lua ou DSL custom). Éditeur avec syntax highlighting et affichage d'erreurs. |
| **Types d'objets/templates** | Définir des templates avec propriétés, behaviors et logique d'événements par défaut. |

#### Phase 3 — Création de contenu

**Objectif** : créer du contenu de jeu riche efficacement.

| Composant | Scope |
|-----------|-------|
| **Éditeur de tilemap** | Palette de tiles, outils brush/fill/eraser, auto-tiling, layers multiples. |
| **Éditeur d'animation de sprites** | Strip de frames, durée par frame, prévisualisation, import Aseprite. |
| **Système de particules** | Édition par inspecteur avec prévisualisation temps réel. Widgets de courbe et gradient. 10 presets. |
| **Hot-reload des assets** | File watching pour les textures, réimport auto. |
| **Gestion des layers** | Layers nommés avec visibilité/verrouillage, réordonnement. |

#### Phase 4 — Polish et features avancées

| Composant | Scope |
|-----------|-------|
| **Animation timeline** | Animation de propriétés avec keyframes, courbes, multi-track. |
| **State machine visuelle** | Pour les états de personnage et le flux de jeu. |
| **Gestionnaire audio** | Bus audio, prévisualisation audio spatial, système d'événements sonores. |
| **Hot-reload de scripts** | Recharger sans redémarrer le jeu. |
| **Build/export** | Export one-click vers les plateformes cibles. |
| **Système de prefabs** | Templates réutilisables avec héritage. |
| **Templates de projet** | "Commencer depuis Platformer", "Commencer depuis Top-Down RPG", etc. |

### Facteurs critiques de succès

1. **Time-to-fun sous 5 minutes** : un nouvel utilisateur doit avoir un personnage qui bouge lors de sa première session. Livrer avec des assets de démarrage et un wizard "Nouveau Projet" avec templates de genre.
2. **Jamais d'écran vide** : toujours une scène par défaut, une caméra, une grille visible. La première chose vue doit être accueillante, pas vide.
3. **Messages d'erreur pour humains** : pas "Expected ')' at line 42 column 16" mais "Il y a un problème dans votre script à la ligne 42 — il semble manquer une parenthèse fermante".
4. **Tutoriels/tooltips intégrés** : aide contextuelle ("Qu'est-ce que c'est ?" tooltips, walkthroughs guidés à la première utilisation).
5. **Core stable avant les features** : un undo/redo rock-solid, save/load et éditeur de scène importent plus qu'un système de particules. Les utilisateurs pardonnent les features manquantes mais pas la perte de leur travail.

### Résumé technologies

| Décision | Recommandation | Raison |
|----------|---------------|--------|
| **Framework UI** | Dear ImGui (C++/Rust) ou Tauri (web) | ImGui pour la vitesse d'itération ; Tauri pour le polish |
| **Scripting primaire non-dev** | Event sheets (style Construct) | Plus basse barrière d'entrée, succès prouvé |
| **Scripting texte** | Lua ou DSL custom | Lua éprouvé et facile à embarquer ; un DSL peut être plus accessible |
| **Format de scène** | JSON | Lisible, diff-friendly |
| **Pipeline d'assets** | File-watch + auto-import (style Godot) | Plus intuitif pour les non-programmeurs |
| **Intégration animation** | Import JSON Aseprite | Standard de facto pour le pixel art 2D |
