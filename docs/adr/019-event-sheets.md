# ADR-019 : Système d'Event Sheets (scripting visuel)

- **Statut :** Acceptée
- **Date :** 2026-03-15
- **Concerne :** v0.3

## Contexte

Le positionnement de Toile est le "missing middle" entre les outils simples (Construct, RPG Maker) et les moteurs complexes (Unity, Godot). Pour atteindre les non-programmeurs, un système de scripting visuel est nécessaire. La v0.1 offre Lua, mais Lua reste du code — inaccessible aux artistes et designers.

## Options considérées

### Node-based (style Unreal Blueprints)
- **Pour :** visuel et spatial, pas d'erreurs de syntaxe, peut exprimer des logiques complexes.
- **Contre :** problème "spaghetti" (les fils s'enchevêtrent), verbeux pour les opérations simples, les recherches montrent que ce n'est PAS toujours plus accessible que du code. Difficile à implémenter (éditeur de graphe avec zoom/pan/connexions).

### Event Sheets (style Construct)
- **Pour :** format tableur lisible ("SI ceci ALORS cela"), pas de spaghetti, très rapide à écrire, barrière d'entrée la plus basse de toutes les approches visuelles. Succès prouvé (Construct a des millions d'utilisateurs). Se lit comme de l'anglais.
- **Contre :** moins flexible que le node-based pour les flux de données complexes, peut devenir long pour les gros projets (mitigé par le groupement et les fonctions).

### DSL custom (style GDScript)
- **Pour :** syntaxe Python-like intuitive, intégration éditeur maximale.
- **Contre :** concevoir un bon DSL est un projet de plusieurs mois, retarderait la v0.3. Reste du code texte — pas visuel.

## Décision

**Event Sheets (style Construct).**

1. **Accessibilité maximale.** Le format condition-action se lit comme du langage naturel. Un designer peut lire "Quand le joueur appuie sur Espace → jouer l'animation 'saut'" sans formation.

2. **Succès prouvé.** Construct 3 a démontré que ce modèle fonctionne pour des jeux commerciaux. C'est le système de scripting visuel le plus lisible.

3. **Implémentable.** L'UI est un tableau avec des colonnes, pas un éditeur de graphe. L'implémentation est significativement plus simple que le node-based.

4. **Coexistence avec Lua.** Les event sheets couvrent 80% des cas d'usage des non-programmeurs. Les cas avancés utilisent toujours Lua. Les deux systèmes coexistent.

## Architecture

### Modèle de données

```
EventSheet {
    name: String,
    events: Vec<Event>,
}

Event {
    conditions: Vec<Condition>,
    actions: Vec<Action>,
    sub_events: Vec<Event>,  // imbrication
    enabled: bool,
    group: Option<String>,
}

Condition {
    kind: ConditionKind,  // enum avec tous les types
    params: HashMap<String, Value>,
    negated: bool,
}

Action {
    kind: ActionKind,
    params: HashMap<String, Value>,
}
```

### Exécution
Les event sheets sont compilés en une liste de commandes à chaque sauvegarde. L'exécution est top-to-bottom : pour chaque event, évaluer les conditions, si toutes vraies, exécuter les actions. Les sous-événements sont évalués récursivement.

### Expressions
Un parser d'expressions inline permet d'écrire `player.x + 10` ou `random(1, 100)` dans les champs d'action. Le parser produit un AST évalué au runtime.

## Conséquences

### Positives
- Les non-programmeurs peuvent créer de la logique de jeu
- Format lisible et maintenable
- Coexiste avec Lua pour les cas avancés
- Sérialisable en JSON (compatible MCP)

### Négatives
- Scope d'implémentation significatif (UI éditeur + runtime + parser d'expressions)
- Performance moindre que Lua pour les boucles serrées (acceptable pour la logique de jeu)
- Deux systèmes de scripting à maintenir (event sheets + Lua)
