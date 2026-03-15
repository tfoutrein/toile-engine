# ADR-022 : Templates de projet

- **Statut :** Acceptée
- **Date :** 2026-03-15
- **Concerne :** v0.3

## Contexte

Le "time-to-fun" est un facteur critique d'adoption. Un nouvel utilisateur devrait avoir un jeu qui bouge à l'écran en moins de 5 minutes. Les templates de projet fournissent un point de départ fonctionnel — pas une page blanche.

## Décision

**4 templates de projet intégrés au CLI `toile new`.**

### Templates

| Template | Contenu |
|----------|---------|
| **Empty** | Scène vide avec caméra. Le minimum. |
| **Platformer** | Joueur avec behavior Platform, sol avec Solid, quelques plateformes, ennemis avec Bullet, collectibles. Tilemap de base. Event sheet pour le score et les vies. |
| **Top-Down** | Joueur avec behavior TopDown, murs Solid, ennemis patrouilleurs, items. Vue de dessus. |
| **Shoot-em-up** | Joueur qui tire (Bullet), vagues d'ennemis, explosions (particules), score. Scrolling vertical. |

### Contenu de chaque template
- `Toile.toml` (manifeste projet)
- `scenes/main.json` (scène pré-configurée avec entités + behaviors)
- `prefabs/` (prefabs pour les entités réutilisables)
- `assets/` (sprites basiques, sons)
- `scripts/` (event sheets ou Lua)
- `llms.txt` (documentation IA)

### CLI

```bash
toile new my-game                    # template Empty par défaut
toile new my-game --template platformer
toile new my-game --template topdown
toile new my-game --template shmup
```

### MCP

```
create_project_from_template { name: "my-game", template: "platformer" }
```

## Conséquences

### Positives
- Time-to-fun < 5 minutes : `toile new my-game --template platformer && toile run`
- Démonstration immédiate des capabilities du moteur
- Les templates servent aussi de documentation par l'exemple
- Chaque template est un jeu jouable minimal, pas juste un squelette

### Négatives
- Chaque template est un petit jeu à créer et maintenir
- Les templates doivent évoluer avec le moteur (breaking changes)
- Risque que les templates deviennent obsolètes si pas maintenus
