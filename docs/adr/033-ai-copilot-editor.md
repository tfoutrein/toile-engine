# ADR-033 : Copilote IA integre dans l'editeur — Claude + MCP tools

- **Statut :** Proposee
- **Date :** 2026-03-19
- **Concerne :** v0.5 (MVP), v1.0 (enrichi)

## Contexte

Toile Engine dispose d'un serveur MCP avec 20 outils pour manipuler les scenes (entites, tilemaps, prefabs, particules). Ce serveur est utilise par des clients externes (Claude Desktop, Cursor, etc.) mais pas depuis l'editeur lui-meme.

L'objectif est d'integrer un **copilote IA directement dans l'editeur** : l'utilisateur tape une instruction en langage naturel ("Cree un niveau platformer avec 5 plateformes et un joueur") et Claude execute les actions via les outils MCP, avec les resultats visibles en temps reel dans le viewport.

## Decision

**Integrer l'API Claude (Anthropic) dans l'editeur avec les outils MCP comme tool definitions.**

### Architecture

```
┌─────────────────────────────────────────────┐
│                 Toile Editor                  │
│                                              │
│  ┌──────────┐  ┌─────────────┐  ┌─────────┐│
│  │ Chat UI  │→ │ Claude API  │→ │ Tool    ││
│  │ (prompt  │  │ (HTTP call  │  │ Executor││
│  │  panel)  │← │  with tools)│← │ (scene  ││
│  └──────────┘  └─────────────┘  │  ops)   ││
│                                  └────┬────┘│
│                                       ↓     │
│                              ┌──────────────┐│
│                              │ Scene / Game ││
│                              │   (live)     ││
│                              └──────────────┘│
└─────────────────────────────────────────────┘
```

### Composants

#### 1. Settings Panel
- Nouveau mode `EditorMode::Settings` ou fenetre modale
- Champs :
  - **API Key** : cle Anthropic (stockee chiffree dans `~/.toile/config.json`)
  - **Model** : choix du modele (claude-sonnet-4-6, claude-opus-4-6, etc.)
  - **System prompt** : contexte additionnel (description du jeu en cours)
- Validation de la cle (test avec un appel simple)

#### 2. Chat Panel
- Panneau lateral ou mode dedie (`EditorMode::AICopilot`)
- Interface de chat :
  - Historique des messages (user + assistant)
  - Champ de saisie en bas
  - Affichage des tool calls en cours (avec icones)
  - Bouton "Stop" pour interrompre
- Streaming des reponses (affichage progressif)

#### 3. Tool Definitions
Reutilisation exacte des schemas MCP existants (`toile-mcp/src/lib.rs`) :

| Outil | Description |
|-------|-------------|
| `create_entity` | Creer une entite dans la scene |
| `update_entity` | Modifier position, taille, rotation, layer |
| `delete_entity` | Supprimer une entite |
| `list_entities` | Lister les entites |
| `create_tilemap` | Creer une tilemap |
| `set_tile` / `fill_rect` | Peindre des tiles |
| `create_prefab` / `instantiate_prefab` | Gerer les prefabs |
| `create_particle_emitter` | Creer des effets particules |
| `take_screenshot` | Capturer le viewport et renvoyer l'image a Claude |
| + outils v0.5 : `add_behavior`, `set_entity_tags`, `create_event_sheet` |

#### 4. Tool Executor
Au lieu de passer par le serveur MCP (process externe), les outils sont executes **directement sur la scene en memoire** :

```rust
fn execute_tool(scene: &mut SceneData, tool: &str, args: &serde_json::Value) -> Result<String, String> {
    match tool {
        "create_entity" => { /* scene.add_entity(...) */ }
        "update_entity" => { /* scene.find_entity_mut(...) */ }
        // ... memes operations que toile-mcp mais sur la scene live
    }
}
```

Cela evite de lancer un process MCP separe et permet des mises a jour instantanees du viewport.

### Flux utilisateur

1. L'utilisateur ouvre le panneau IA (bouton "🤖 AI" dans la toolbar)
2. Il tape : "Ajoute 3 plateformes en escalier avec un joueur en bas a gauche"
3. L'editeur envoie le message a l'API Claude avec :
   - Le system prompt (contexte du jeu + instructions)
   - L'etat actuel de la scene (JSON simplifie)
   - Les tool definitions
4. Claude repond avec des tool_use (create_entity x4, update_entity...)
5. L'editeur execute chaque tool call sur la scene
6. Le viewport se met a jour en temps reel
7. Claude confirme avec un message texte ("J'ai cree 3 plateformes et le joueur")
8. L'utilisateur peut continuer la conversation ou modifier manuellement

### Dependances techniques

- **Crate `reqwest`** : client HTTP async pour appeler l'API Anthropic
- **Crate `tokio`** : deja dans le workspace (pour le MCP server)
- **API Anthropic Messages** : `POST https://api.anthropic.com/v1/messages` avec `tools` parameter
- **Streaming** : SSE (Server-Sent Events) pour affichage progressif

### System Prompt

```
Tu es l'assistant IA integre dans Toile Editor, un editeur de jeux 2D.

Scene actuelle : "{scene_name}" avec {entity_count} entites.
Viewport : {viewport_width}x{viewport_height}, camera a ({cam_x}, {cam_y}).

Tu peux utiliser les outils suivants pour manipuler la scene :
- create_entity : creer un sprite, un obstacle, un personnage
- update_entity : deplacer, redimensionner, renommer
- delete_entity : supprimer
- list_entities : voir ce qui existe
- create_tilemap, set_tile, fill_rect : creer des niveaux tiles
- create_prefab, instantiate_prefab : templates reutilisables
- add_behavior : ajouter Platform, TopDown, Solid, Sine, etc.
- set_entity_tags : definir Player, Solid, Coin, Enemy
- create_event_sheet : regles de jeu (conditions → actions)

Reponds de maniere concise. Execute les actions demandees immediatement.
```

## Phasage

### Phase 1 : Settings + Chat basique (v0.5)
- Page Settings avec cle API
- Chat panel avec envoi/reception (non-streaming)
- 5 outils de base : create_entity, update_entity, delete_entity, list_entities, load_scene
- Execution directe sur la scene

### Phase 2 : Streaming + outils complets (v0.5)
- Streaming SSE des reponses
- Tous les 20 outils MCP disponibles
- Contexte automatique (scene state envoye a chaque message)
- Historique de conversation persistant par projet

### Phase 3 : Vision + feedback visuel (v0.5/v1.0)
- **Screenshot tool** : apres chaque serie de modifications, capturer le viewport et l'envoyer a Claude comme image
- Claude peut "voir" le resultat et se corriger ("Le joueur est trop a droite, je le deplace")
- Mecanisme :
  1. Tool executor termine les modifications
  2. L'editeur fait un rendu du viewport dans un buffer offscreen
  3. L'image (PNG, max 1024px) est ajoutee au contexte de conversation
  4. Claude recoit `[image du viewport actuel]` et peut reagir
- Declenchement : automatique apres chaque batch de tool calls, ou sur demande ("montre-moi le resultat")
- Nouvel outil `take_screenshot` disponible pour Claude

### Phase 4 : Copilote avance (v1.0)
- Suggestions proactives ("Cette entite n'a pas de collider, voulez-vous en ajouter ?")
- Mode "describe & build" : l'utilisateur decrit le jeu, Claude le construit etape par etape
- Integration avec l'Asset Library ("Utilise le sprite du pack Kenney pour le joueur")
- Undo/redo des actions IA
- Multi-turn avec memoire du projet

## Consequences

### Positives
- Les non-programmeurs peuvent creer des jeux par description naturelle
- Acceleration massive du workflow (un paragraphe → un niveau complet)
- Le MCP server existant est reutilise (pas de nouvelle logique a creer)
- Differenciateur fort vs Construct/Godot/Unity

### Negatives
- Necessite une cle API Anthropic (cout pour l'utilisateur)
- Latence reseau sur chaque interaction
- Les hallucinations IA peuvent creer des scenes incorrectes (mitige par le live preview)
- La taille du contexte limite le nombre d'entites describables (~100k tokens)

### Risques
- L'API Anthropic peut changer → abstraire derriere un trait `AIProvider`
- La cle API est sensible → stockage chiffre dans le config
- Le streaming SSE peut etre bloque par des firewalls corporate
- La conversation peut diverger → bouton "Reset conversation" + system prompt strict
