# ADR-014 : Pile de scènes et transitions

- **Statut :** Acceptée
- **Date :** 2026-03-14
- **Concerne :** v0.2

## Contexte

La v0.1 n'a pas de gestion de scènes multiples. Le `Game` trait gère un seul état. Pour un jeu réel, on a besoin de : menu principal → gameplay → pause → game over, avec des transitions visuelles entre les scènes, et la possibilité de "stacker" une scène par-dessus une autre (ex: pause overlay).

## Options considérées

### Scene graph (arbre de scènes imbriquées)
- **Pour :** Très flexible. Permet des scènes composées (HUD par-dessus le gameplay).
- **Contre :** Complexe à implémenter. Godot-level de complexité. Overkill pour la v0.2.

### State machine (enum de scènes)
- **Pour :** Simple. L'utilisateur définit un enum et les transitions.
- **Contre :** Pas de stacking (on ne peut pas avoir pause par-dessus gameplay). Rigide.

### Pile de scènes (stack)
- **Pour :** Simple et puissant. Push (empile une scène), Pop (retire la scène du dessus), Replace (remplace le dessus). La scène du dessus reçoit les updates, les scènes en dessous sont optionnellement visibles (pour les overlays transparents). Transitions visuelles (fade, slide) entre push/pop.
- **Contre :** Moins flexible qu'un arbre. Pas de scènes parallèles indépendantes.

## Décision

**Pile de scènes (scene stack) avec transitions.**

C'est le modèle le plus adapté aux jeux 2D :
- `push(scene)` : empile (ex: ouvrir le menu pause)
- `pop()` : dépile (ex: fermer le menu pause, retour au gameplay)
- `replace(scene)` : remplace le dessus (ex: menu → gameplay)
- La scène du dessus est active (reçoit update/draw)
- Les scènes en dessous peuvent être dessinées si marquées "transparent" (overlay)
- Les transitions (fade, slide, wipe) sont des scènes intermédiaires automatiques

## Conséquences

### Positives
- Modèle mental simple et intuitif
- Gère les cas courants : menu, gameplay, pause, game over, cutscenes
- Les overlays (HUD, pause) sont naturels via le stacking
- Les transitions visuelles enrichissent l'expérience

### Négatives
- Pas de scènes parallèles (ex: deux joueurs avec des scènes indépendantes)
- La mémoire des scènes stackées reste allouée (mitigé par des callbacks freeze/thaw)
