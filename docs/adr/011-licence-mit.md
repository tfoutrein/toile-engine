# ADR-011 : Licence MIT

- **Statut :** Acceptée
- **Date :** 2026-03-14
- **Concerne :** v0.1

## Contexte

Le choix de licence est stratégique. L'exode Unity (2023) a montré que les développeurs de jeux priorisent la stabilité de licence et l'absence de risque corporate. Le moteur doit inspirer confiance : "personne ne peut changer les règles du jeu".

## Options considérées

### MIT
- **Pour :** la licence open-source la plus permissive et la plus simple. Permet tout usage (commercial, modification, distribution) avec une seule exigence : inclure le copyright. Godot utilise MIT. La majorité de l'écosystème Rust est MIT.
- **Contre :** ne protège pas contre la "propriétarisation" (quelqu'un peut forker et fermer le code). Pas de clause de brevet.

### Apache 2.0
- **Pour :** permissive comme MIT mais ajoute une clause de brevet explicite (protection contre les patent trolls). Bevy utilise MIT + Apache 2.0 dual.
- **Contre :** légèrement plus complexe que MIT. La clause de brevet peut décourager certaines entreprises (bien que ce soit son avantage).

### GPL v3
- **Pour :** protège contre la propriétarisation (les modifications doivent rester open-source). Garantit que le moteur reste libre.
- **Contre :** les studios de jeux évitent GPL. Les jeux créés avec un moteur GPL ont des implications légales floues. Réduit drastiquement l'adoption. Incompatible avec de nombreuses bibliothèques.

### LGPL
- **Pour :** compromis entre permissivité et copyleft. Les jeux peuvent être propriétaires, le moteur reste open.
- **Contre :** les règles de linkage sont confuses et font peur aux studios juridiquement. Moins adopté dans le game dev.

### BSL / SSPL / autre "source-available"
- **Pour :** protection commerciale du créateur.
- **Contre :** "source-available" n'est pas "open-source". Les développeurs post-Unity fuient exactement ce type de licence ambiguë.

## Décision

**MIT.**

1. **Confiance maximale.** Après l'épisode Unity, la licence est un signal de confiance. MIT dit : "vous pouvez faire absolument tout avec ce code, sans condition autre que le copyright." C'est le message le plus fort possible.

2. **Adoption maximale.** MIT est la licence la plus comprise et la plus acceptée universellement. Les studios la comprennent. Les juridiques l'approuvent. Les contributeurs la connaissent. Zéro friction.

3. **Alignement écosystème.** Godot (MIT), hecs (MIT/Apache), glam (MIT/Apache), kira (MIT/Apache), egui (MIT/Apache). Toute la stack est MIT-compatible.

4. **Le fork n'est pas un risque — c'est une feature.** Si quelqu'un forke et améliore Toile, c'est bon pour l'écosystème. Le projet original gagne en visibilité et peut réintégrer les améliorations. La "menace" de propriétarisation est théorique — en pratique, les projets open-source actifs ne sont jamais supplantés par un fork fermé.

**Note :** on pourrait ajouter Apache 2.0 en dual-licence (MIT OR Apache-2.0, comme Bevy) pour la clause de brevet. C'est une option à considérer mais pas bloquante pour le MVP.

## Conséquences

### Positives
- Signal de confiance maximal pour les développeurs post-Unity
- Zéro friction d'adoption (pas de question juridique)
- Compatible avec toutes les bibliothèques de la stack
- Les jeux créés avec Toile peuvent être 100% propriétaires

### Négatives
- Pas de protection contre la propriétarisation de forks (risque théorique, faible en pratique)
- Pas de clause de brevet explicite (mitigé : ajoutable via dual-licence Apache 2.0)
