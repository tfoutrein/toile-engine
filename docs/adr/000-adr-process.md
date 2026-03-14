# ADR-000 : Utilisation des Architecture Decision Records

- **Statut :** Acceptée
- **Date :** 2026-03-14
- **Auteur(s) :** Fondateurs Toile Engine

## Contexte

Le projet Toile Engine est un moteur 2D ambitieux avec de nombreuses décisions techniques structurantes. Ces décisions doivent être documentées, traçables, et compréhensibles par les futurs contributeurs.

## Décision

Nous adoptons les **Architecture Decision Records (ADR)** comme méthode de documentation des décisions architecturales significatives.

Chaque ADR suit cette structure :
- **Statut** : Proposée / Acceptée / Remplacée / Dépréciée
- **Contexte** : pourquoi la décision est nécessaire
- **Options considérées** : alternatives évaluées
- **Décision** : le choix fait et pourquoi
- **Conséquences** : impacts positifs et négatifs

Les ADR sont numérotés séquentiellement et ne sont jamais supprimés. Une décision révisée produit un nouvel ADR qui référence et remplace l'ancien.

## Conséquences

- Chaque décision structurante a une trace écrite
- Les nouveaux contributeurs comprennent le "pourquoi" derrière chaque choix
- Les décisions peuvent être revisitées avec tout le contexte original
