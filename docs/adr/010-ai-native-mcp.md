# ADR-010 : Serveur MCP natif et design AI-first

- **Statut :** Acceptée
- **Date :** 2026-03-14
- **Concerne :** v0.1

## Contexte

Le positionnement central de Toile est d'être **AI-native** : un moteur conçu dès le départ pour être piloté par des assistants IA (Claude, GPT, Copilot, Cursor). Les MCP servers pour Unity et Godot existent (mcp-unity avec 149+ outils, godot-mcp) mais sont des ajouts après coup, greffés sur des moteurs qui n'ont pas été conçus pour ce mode d'interaction. Toile peut faire mieux en intégrant le contrôle IA dans son architecture fondamentale.

## Décision

Le moteur intègre nativement les éléments suivants pour le pilotage par IA :

### 1. Serveur MCP intégré

Un serveur MCP (Model Context Protocol) est livré avec le moteur, pas comme un plugin tiers. Il expose des outils pour chaque opération du moteur :

**Scènes :** `list_scenes`, `create_scene`, `load_scene`, `save_scene`, `delete_scene`
**Entités :** `create_entity`, `get_entity`, `update_entity`, `delete_entity`, `list_entities`
**Composants :** `add_component`, `remove_component`, `set_component_property`, `get_component_property`
**Assets :** `list_assets`, `import_asset`, `get_asset_info`
**Exécution :** `play_scene`, `stop_scene`, `pause_scene`, `step_frame`
**Inspection :** `get_game_state`, `take_screenshot`, `read_console`
**Projet :** `get_project_config`, `set_project_config`

Chaque outil retourne un JSON structuré :

```json
{
  "status": "success",
  "data": { "entityId": "player_01", "components": { ... } },
  "metadata": { "timestamp": "...", "scene": "forest_level" }
}
```

### 2. Erreurs structurées

Toutes les erreurs retournent un JSON machine-parseable :

```json
{
  "status": "error",
  "error": {
    "code": "COMPONENT_NOT_FOUND",
    "message": "Entity 'player' does not have component 'health'",
    "entity": "player",
    "requested": "health",
    "available": ["transform", "sprite", "collider", "script"],
    "suggestion": "Did you mean 'collider'?"
  }
}
```

L'IA peut s'auto-corriger grâce au contexte, aux alternatives disponibles, et aux suggestions.

### 3. `llms.txt`

Un fichier `llms.txt` (per le [standard llms.txt](https://llmstxt.org/)) est livré avec chaque release. Il contient :
- La description du moteur en une phrase
- La liste de tous les outils MCP avec leur signature
- Le format de scène JSON avec exemples
- Les types de composants et leurs propriétés
- L'API Lua complète

Ce fichier réduit la consommation de tokens de 90%+ par rapport à la documentation HTML, permettant à l'IA de tenir toute l'API en contexte.

### 4. JSON Schema pour tout

Chaque format de données a un JSON Schema publié :
- `scene-v1.json` — format de scène
- `project-v1.json` — configuration projet
- `component-*.json` — définition de chaque type de composant

Les LLMs peuvent contraindre leur sortie à ces schémas, garantissant la validité structurelle.

### 5. Mode headless

`toile run --headless` exécute le jeu sans affichage. Le moteur :
- Tourne le game loop normalement
- Expose l'état via le MCP server
- Permet de prendre des screenshots (rendu offscreen)
- Émet les logs sur stdout en JSON

Cela permet aux boucles de test IA : l'IA modifie → le moteur exécute → l'IA vérifie le résultat.

### 6. CLI complète

Chaque opération GUI a un équivalent CLI :

```bash
toile new mon-jeu              # Créer un projet
toile run                      # Exécuter le jeu
toile run --headless           # Exécuter sans affichage
toile build --platform web     # Builder pour le web
toile add-entity player        # Ajouter une entité
toile list-entities            # Lister les entités
toile export                   # Exporter le projet
```

## Pourquoi pas juste un plugin tiers (comme Unity/Godot) ?

Les MCP servers greffés après coup souffrent de :
- **Décalage API** : le serveur MCP expose un sous-ensemble de l'API moteur, souvent en retard
- **Fragile** : les mises à jour du moteur cassent le MCP server
- **Pas de feedback structuré** : les erreurs sont des strings, pas des objets machine-parseables
- **Pas de mode headless** : le moteur doit tourner avec un affichage

En intégrant le MCP nativement, chaque feature du moteur est automatiquement exposée aux IA. Le MCP n'est pas un ajout — c'est une vue sur l'architecture.

## Conséquences

### Positives
- Tout assistant IA (Claude Code, Cursor, Copilot, Codex) peut piloter Toile via MCP
- Le feedback structuré permet l'auto-correction par l'IA
- Le mode headless permet le testing automatisé par IA
- Le `llms.txt` réduit la friction de contexte pour les LLMs
- Différenciateur marché : aucun moteur 2D n'offre ça nativement

### Négatives
- Scope additionnel en v0.1 (mitigé : le MCP est en semaines 11-12, après le core engine)
- Maintenance du MCP server en parallèle de l'API moteur (mitigé : le MCP est une vue sur l'API, pas une API séparée)
- Le `llms.txt` doit être mis à jour à chaque release (mitigé : génération automatique depuis les commentaires de code)
