# Moteur 2D Pilotable par IA — Recherche & Architecture

## Table des matières
1. [Paysage des outils AI pour le game dev (2024-2026)](#1-paysage-des-outils-ai)
2. [MCP (Model Context Protocol) et moteurs de jeu](#2-mcp-et-moteurs-de-jeu)
3. [Moteurs programmatiques / API-driven](#3-moteurs-programmatiques)
4. [Formats de description de scène AI-friendly](#4-formats-de-scène-ai-friendly)
5. [Ce qui rend une API "AI-friendly"](#5-api-ai-friendly)
6. [Plateformes de création de jeux par IA](#6-plateformes-de-création-par-ia)
7. [Principes de design pour un système contrôlable par IA](#7-principes-de-design)
8. [Hot-reload + workflow IA](#8-hot-reload--workflow-ia)
9. [Recommandations d'architecture](#9-recommandations-darchitecture)

---

## 1. Paysage des outils AI pour le game dev

### Tendances majeures 2024-2026

- **SEELE** : plateforme AI-native permettant la génération text-to-game via interface LLM conversationnelle. Réduit le temps de prototypage de 80-90%.
- **Scenario** : outil de génération d'art IA optimisé pour le game dev (sprites cohérents, sprite sheets).
- **Inworld / Eastworld** : création de personnalités PNJ avec backstories, connaissances, et capacité d'action.
- **"Vibe Coding"** : terme inventé par Andrej Karpathy (début 2025) décrivant l'utilisation de l'IA comme langage de programmation de haut niveau. Y Combinator a rapporté que 25% de leur batch Winter 2025 avait des codebases à 95%+ générées par IA.

La tendance est claire : des outils LLM spécialisés pour le jeu remplacent les IA généralistes dans les workflows de game dev.

---

## 2. MCP et moteurs de jeu

Le MCP (Model Context Protocol) émerge comme le **pont critique** entre les assistants IA et les moteurs de jeu.

### Implémentations existantes

#### Unity MCP
- [mcp-unity](https://github.com/CoderGamester/mcp-unity) : connecte l'éditeur Unity aux assistants IA (Cursor, Claude Code, Codex, Windsurf, GitHub Copilot). Capacités :
  - Créer, renommer, supprimer des GameObjects et ajuster les transforms
  - Ajouter, retirer, modifier des composants
  - Gérer les assets (textures, modèles, audio), materials, shaders
  - Créer/modifier des prefabs et instances
  - Exécuter des items de menu, sélectionner des game objects par path
- [Unity MCP officiel](https://docs.unity3d.com/Packages/com.unity.ai.assistant@2.0/manual/unity-mcp-overview.html) : package MCP officiel Unity (v2.0.0-pre.1)

#### Godot MCP
- [tugcantopaloglu/godot-mcp](https://github.com/tugcantopaloglu/godot-mcp) : **149 outils** couvrant networking, rendu 2D/3D, contrôles UI, audio, arbres d'animation, I/O fichier, exécution de code runtime, inspection de propriétés, manipulation de scènes, gestion de signaux, physique, et création de projet.
- [GDAI MCP](https://gdaimcp.com/) : 95+ outils incluant gestion de scènes, diagnostics GDScript LSP, débogueur DAP, capture d'écran, injection d'input, introspection ClassDB, et bibliothèque d'assets CC0.

#### Unreal Engine MCP
- Serveur MCP complet permettant le contrôle d'Unreal via Remote Control API, construit en TypeScript.

#### Hub unifié
- [GameDev MCP Hub](https://lobehub.com/mcp/yourusername-gamedev-mcp-hub) : agrège 5+ serveurs MCP game dev avec 165+ outils.

### Leçon clé pour notre moteur

Construire un **serveur MCP first-class** doit être une priorité. Il donne à tout assistant IA (Claude, Copilot, Cursor, Codex) le contrôle direct sur le moteur. Le serveur MCP devrait exposer des outils pour :

- **Gestion de scènes/niveaux** : créer, charger, sauvegarder, lister
- **CRUD d'entités** : créer, lire, modifier, supprimer des entités
- **Manipulation de composants** : ajouter/retirer/modifier des composants sur les entités
- **Gestion d'assets** : importer, lister, configurer les assets
- **Configuration projet** : paramètres, résolution, physics settings
- **Exécution** : lancer/arrêter le jeu
- **Debug** : lire la sortie console/debug
- **Feedback visuel** : prendre des screenshots de l'état courant

---

## 3. Moteurs programmatiques

### Caractéristiques des moteurs faciles à contrôler par IA

#### Fichiers de scène textuels
**Godot** se distingue comme exceptionnellement AI-friendly parce que ses fichiers `.tscn` sont du texte lisible par un humain. Tout (fichiers de scène, scripts) peut être créé dans un éditeur de texte, et le code généré par LLM peut être utilisé tel quel. C'est un avantage massif sur les formats binaires.

#### Moteurs headless
- **@headless-game-engine/core** : moteur JS minimaliste, framework-agnostique. Exécute des scènes avec GameObjects, Components et Systems sans rendering.
- **Excalibur.js** : moteur 2D TypeScript pour le web, peut potentiellement tourner headless.

#### VibeGame Engine
[VibeGame](https://github.com/dylanebert/vibegame) est spécifiquement conçu pour le développement assisté par IA :
- Moteur déclaratif de haut niveau construit sur three.js, rapier et bitecs
- Syntaxe déclarative XML-like pour définir les game objects
- Architecture ECS séparant données (composants) et comportement (systèmes)
- Livré avec un fichier `llms.txt` contenant la documentation conçue spécifiquement pour l'IA

### Implications de design pour notre moteur

- **Tout en texte** : fichiers de scène, config, références d'assets — tout doit être du texte lisible (JSON, YAML, ou format texte custom)
- **Mode headless** : le moteur doit pouvoir tourner sans affichage pour le testing et la validation par IA
- **Interface CLI** : commandes pour créer des projets, ajouter des entités, exécuter des jeux, exporter, etc.
- **API programmatique** : chaque opération GUI doit avoir un équivalent API

---

## 4. Formats de scène AI-friendly

### Formats existants

- **Godot .tscn** : lisible par un humain, format custom INI-like. Très adapté à la génération par LLM.
- **VGDL (Video Game Description Language)** : framework pour décrire règles de jeu et niveaux. Les humains et LLMs peuvent facilement comprendre et générer du VGDL.
- **A-Frame (HTML-based)** : syntaxe déclarative HTML-like pour les scènes 3D, trivialement lisible par les LLMs.

### Bonnes pratiques pour les formats AI-friendly

1. **Utiliser JSON Schema** : définir un schéma strict pour le format de scène. Les LLMs peuvent être contraints à produire du JSON valide selon un schéma, garantissant la correction.
2. **Plat plutôt qu'imbriqué** : minimiser la profondeur d'imbrication. Les LLMs ont du mal avec les structures profondément imbriquées.
3. **Références nommées plutôt que IDs** : utiliser des noms lisibles comme `"player_spawn"` plutôt que des IDs numériques comme `42`.
4. **Génération hiérarchique** : générer du haut niveau (layout du monde) vers le bas niveau (propriétés individuelles des tiles).

### Format recommandé

```json
{
  "scene": {
    "name": "forest_level",
    "size": { "width": 800, "height": 600 },
    "entities": [
      {
        "name": "player",
        "components": {
          "transform": { "x": 100, "y": 300 },
          "sprite": { "asset": "hero.png", "width": 32, "height": 32 },
          "physics": { "bodyType": "dynamic", "mass": 1.0 },
          "playerControl": { "speed": 200, "jumpForce": 400 }
        }
      },
      {
        "name": "ground",
        "components": {
          "transform": { "x": 400, "y": 580 },
          "sprite": { "asset": "grass_tile.png", "width": 800, "height": 40 },
          "physics": { "bodyType": "static" }
        }
      }
    ]
  }
}
```

Ce format est :
- **Validable** contre un JSON Schema
- **Facile à générer** correctement par un LLM
- **Déclaratif** (décrit quoi, pas comment)
- **Lisible et éditable** par un humain

---

## 5. API AI-friendly

### Principes de design d'API pour LLMs

#### 1. Fonctionnalité auto-descriptive
Les noms de fonctions, paramètres et documentation doivent clairement décrire ce que l'API fait.

```
✅ createEntity("player", { x: 100, y: 200 })
❌ ce("p", [100, 200])
```

#### 2. Patterns d'interaction simplifiés
Favoriser des appels API simples et directs plutôt que des séquences d'interaction complexes. Une fonction qui fait une chose bien.

#### 3. Indirection réduite
Structurer le code pour qu'un LLM n'ait pas à naviguer de nombreuses couches d'abstraction.

#### 4. Conventions de nommage consistantes
Utiliser des patterns verbe-nom de façon cohérente :
```
createEntity, removeEntity
addComponent, removeComponent
setProperty, getProperty
loadScene, saveScene
```

#### 5. Types TypeScript forts
Les interfaces et types TypeScript servent de documentation lisible par les machines. Les LLMs entraînés sur du code TypeScript peuvent inférer l'usage correct à partir des types seuls.

#### 6. Déclaratif plutôt qu'impératif
Au lieu d'instructions étape par étape, laisser les utilisateurs décrire l'état final désiré.

**Impératif** (mauvais pour IA) :
```javascript
const entity = engine.createEntity()
engine.addComponent(entity, "transform")
engine.setPosition(entity, 100, 200)
engine.addComponent(entity, "sprite")
engine.setSprite(entity, "hero.png")
```

**Déclaratif** (bon pour IA) :
```json
{ "name": "hero", "components": { "transform": { "x": 100, "y": 200 }, "sprite": { "asset": "hero.png" } } }
```

### Ce que les LLMs font bien en game dev
- Génération de code boilerplate
- Scaffolding d'architecture système
- Patterns de jeu standard (inventaire, dialogue, combat)
- Application cohérente de patterns à travers un codebase

### Ce qui est difficile pour les LLMs
- APIs profondément imbriquées avec beaucoup d'indirection
- Conventions de nommage inconsistantes
- Effets de bord et état mutable difficile à raisonner
- Formats binaires ou nécessitant des outils spéciaux
- APIs nécessitant le maintien d'un état complexe sur plusieurs appels

---

## 6. Plateformes de création par IA

### Exemples existants

| Plateforme | Description |
|-----------|-------------|
| **Rosebud AI** | Environnement intégré de création 3D/2D depuis un prompt. Pas de code requis, tout dans le navigateur. |
| **Gambo** | "Premier agent de vibe coding pour jeux". Crée des jeux complets depuis un seul prompt. |
| **VibeGame** | Moteur déclaratif open-source construit pour le dev assisté par IA (Hugging Face). |
| **DreamGarden** | Assistant IA pour le game design qui travaille semi-autonomement. "Fait pousser" un jardin de plans et actions depuis un objectif de haut niveau. |

### Pattern commun

Toutes les plateformes réussies partagent :
**Input déclaratif (langage naturel ou données structurées) → Génération IA → Preview instantanée → Raffinement itératif**

---

## 7. Principes de design pour un système contrôlable par IA

### 7.1 Déclaratif plutôt qu'impératif
Décrire l'état désiré, pas les étapes pour y arriver. L'IA génère une description de scène, pas une séquence de commandes.

### 7.2 Formats validés par schéma
Le LLM peut converser librement, mais ne peut pas exécuter — l'autorité d'exécution réside entièrement dans la validation de schéma. Utiliser JSON Schema pour valider tous les inputs. Cela empêche les sorties malformées de l'IA de corrompre l'état du jeu.

### 7.3 Commandes déterministes avec feedback clair
Chaque commande doit retourner une réponse structurée indiquant succès/échec plus l'état résultant :

```json
{
  "status": "success",
  "entityId": "hero_01",
  "state": { "position": { "x": 100, "y": 200 } }
}
```

Le défi architectural fondamental : les LLMs produisent des sorties stochastiques, tandis que les moteurs de jeu nécessitent des inputs déterministes. Le pont est **validation + feedback structuré**.

### 7.4 Undo/Redo au niveau API
Le [Pattern Command](https://gameprogrammingpatterns.com/command.html) est l'approche standard :
- Chaque mutation est un objet command avec `execute()` et `undo()`
- Les commandes sont stockées dans une pile d'historique
- Les agents IA peuvent expérimenter en toute sécurité sachant que chaque action est réversible

### 7.5 Messages d'erreur structurés
Les erreurs doivent être parseable par une machine, pas juste lisibles par un humain :

```json
{
  "error": "COMPONENT_NOT_FOUND",
  "message": "Entity 'player' does not have component 'health'",
  "entity": "player",
  "requestedComponent": "health",
  "availableComponents": ["transform", "sprite", "physics"],
  "suggestion": "Did you mean 'playerControl'?"
}
```

Cela permet à l'IA de s'auto-corriger en sachant exactement ce qui a mal tourné et quelles alternatives existent.

### 7.6 Documentation consommable par l'IA

Le [standard llms.txt](https://llmstxt.org/) (proposé par Jeremy Howard, septembre 2024) fournit :
- Un résumé structuré, AI-friendly de la documentation
- Format Markdown lisible par humains et LLMs
- Réduit la consommation de tokens de 90%+ vs HTML

Notre moteur devrait livrer :
1. Un fichier **`llms.txt`** résumant l'API
2. Des **types TypeScript** / interfaces comme docs machine-readable
3. Des **fichiers JSON Schema** pour tous les formats de données (scènes, configs, assets)
4. Des **fichiers d'exemple** pour chaque feature

---

## 8. Hot-reload + workflow IA

### La boucle de développement IA idéale

```
L'IA génère/modifie du code ou un fichier de scène
  → Le file watcher détecte le changement
    → Le moteur hot-reload le module changé
      → L'état du jeu se met à jour en temps réel
        → Un screenshot/état est renvoyé à l'IA pour vérification
          → L'IA fait la prochaine itération
```

### Approches de hot-reload

| Approche | Langage | Détails |
|----------|---------|---------|
| **Live++** | C++ | Patche le code machine des exécutables en cours, utilisé par 100+ entreprises |
| **Unreal Live Coding** | C++ | Compile et charge la nouvelle logique sans fermer le projet |
| **HMR (Hot Module Replacement)** | JS/TS | Support natif via rechargement de modules, parfait pour un moteur web |
| **Rechargement de bibliothèque partagée** | C/Rust | La boucle principale vérifie si le fichier .so/.dll a été modifié et le recharge |

### Boucle serrée avec l'IA

L'insight clé : **plus la boucle de feedback est rapide, plus le développement assisté par IA est efficace**. Cibler des temps de reload sub-seconde. Fournir une API de screenshot ou de snapshot d'état pour que l'IA puisse "voir" le résultat de ses changements programmatiquement.

### Pour un moteur avec architecture ECS

L'ECS supporte naturellement le hot-reload :
- Les **systèmes** (logique) peuvent être swappés sans perdre les données d'entités/composants
- Les **composants** (données) persistent à travers les reloads
- Le **fichier de scène** peut être re-parsé et diffé contre l'état courant, avec application incrémentale des changements

---

## 9. Recommandations d'architecture

### Table récapitulative

| Principe | Implémentation |
|----------|---------------|
| **Format de scène** | JSON avec validation JSON Schema ; structure plate lisible avec références nommées |
| **Style d'API** | Déclaratif, typé (TypeScript), nommage verbe-nom consistant, auto-descriptif |
| **Serveur MCP** | Serveur MCP first-class exposant toutes les opérations du moteur comme outils |
| **CLI** | CLI complet pour création de projet, gestion d'entités, build, exécution |
| **Hot-reload** | File watcher + diffing incrémental de scène + hot-swap de modules |
| **Undo/redo** | Pattern Command avec pile d'historique, exposé via API |
| **Gestion d'erreurs** | Erreurs JSON structurées avec contexte et suggestions |
| **Documentation** | llms.txt + types TypeScript + JSON Schema + exemples |
| **Mode headless** | Moteur exécutable sans affichage pour testing/validation IA |
| **Inspection d'état** | API pour requêter l'état complet du jeu, prendre des screenshots, lire la console |
| **Architecture ECS** | Data-driven, compositionnel, naturellement AI-friendly |
| **Boucle de feedback** | Chaque commande retourne un résultat structuré avec le nouvel état |

### Le workflow humain + IA en pratique

```
┌─────────────┐     ┌──────────────┐     ┌─────────────────┐
│   Humain    │────→│  Assistant   │────→│   Moteur 2D     │
│  (créatif)  │     │  IA (Claude) │     │                 │
│             │     │              │     │  ┌───────────┐  │
│ "Crée un    │     │ Génère JSON  │     │  │ MCP Server│  │
│  niveau     │     │ de scène +   │────→│  │           │  │
│  forêt      │     │ code script  │     │  │ - scenes  │  │
│  avec des   │     │              │     │  │ - entities│  │
│  ennemis"   │     │ Lit le       │←────│  │ - assets  │  │
│             │     │ feedback     │     │  │ - run/stop│  │
│ "Plus de    │     │ structuré    │     │  │ - inspect │  │
│  lumière    │     │              │     │  │ - screenshot│ │
│  ambiante"  │     │ Itère        │     │  └───────────┘  │
│             │     │              │     │                 │
│ "Parfait !" │     │              │     │  Hot-reload     │
└─────────────┘     └──────────────┘     │  sub-seconde    │
                                         └─────────────────┘
```

### Ce qui nous différencie

Un moteur 2D conçu **dès le départ** pour le pilotage par IA serait une proposition de valeur unique sur le marché :
- Aucun moteur existant n'a été conçu avec l'IA comme citoyen de première classe
- Les MCP servers pour Unity/Godot sont des **ajouts après coup** — notre MCP serait **natif**
- Le format de scène JSON + JSON Schema rend la génération par LLM **fiable** (pas de guessing)
- Le mode headless permet le **testing automatisé par IA** — l'IA peut jouer au jeu et rapporter des bugs
- Le feedback structuré crée une **boucle fermée** qui permet l'itération autonome

Sources :
- [MCP Unity](https://github.com/CoderGamester/mcp-unity)
- [Godot MCP (149 tools)](https://github.com/tugcantopaloglu/godot-mcp)
- [GDAI MCP Server](https://gdaimcp.com/)
- [VibeGame Engine](https://github.com/dylanebert/vibegame)
- [llms.txt Standard](https://llmstxt.org/)
- [LLM-Friendly API Design Patterns](https://github.com/nibzard/awesome-agentic-patterns/blob/main/patterns/llm-friendly-api-design.md)
- [Schema-Gated Agentic AI](https://arxiv.org/html/2603.06394)
- [Command Pattern - Game Programming Patterns](https://gameprogrammingpatterns.com/command.html)
- [AI Game Dev Tools](https://github.com/Yuan-ManX/ai-game-devtools)
- [SEELE AI Gaming Platform](https://www.seeles.ai/resources/blogs/llm-gaming-how-we-use-ai-for-game-development)
- [GDC Talk: AI-Powered Prototyping with MCP](https://schedule.gdconf.com/session/build-faster-iterate-more-ai-powered-prototyping-with-the-model-context-protocol-mcp/915811)
- [DreamGarden: Growing Games from a Single Prompt](https://arxiv.org/html/2410.01791v1)
