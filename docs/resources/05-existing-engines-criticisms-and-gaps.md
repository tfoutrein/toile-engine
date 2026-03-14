# Défauts des Moteurs 2D Existants & Opportunités

## Table des matières
1. [Critiques par moteur](#1-critiques-par-moteur)
2. [Problèmes transversaux à tous les moteurs](#2-problèmes-transversaux-à-tous-les-moteurs)
3. [Post-mortems de jeux indie — Ce que les moteurs n'offraient pas](#3-post-mortems-de-jeux-indie)
4. [Le phénomène "engine fatigue" et l'exode Unity](#4-le-phénomène-engine-fatigue-et-lexode-unity)
5. [Niches mal servies](#5-niches-mal-servies)
6. [Ce que les développeurs souhaitent](#6-ce-que-les-développeurs-souhaitent)
7. [Opportunités pour un nouveau moteur](#7-opportunités-pour-un-nouveau-moteur)

---

## 1. Critiques par moteur

### Godot

**Physique 2D cassée**
La physique est décrite comme "completely broken" par des utilisateurs récurrents. Les bugs sont nombreux, les inconsistances sans fin, et le support limité. Des développeurs ont abandonné des idées de jeux basés sur la physique après des échecs répétés.

**Performance tilemap**
- L'éditeur de TileMap souffre de lag sévère avec des cartes larges (200×400+ tiles)
- Godot 4.3 a introduit des régressions de performance : crashs et lag massif lors du placement de tiles
- Les TileMapLayers multiples dégradent la performance à mesure que le monde grandit
- **L'éditeur** lag, mais le jeu exporté tourne bien — ce qui est frustrant pour le workflow

**Système UI/GUI problématique**
- Le styling par thèmes est contre-intuitif : propriétés mal nommées, localisées de façon bizarre
- L'UI ne se met souvent pas à jour visuellement tant qu'on ne ferme pas et rouvre pas la scène
- Les hiérarchies UI deviennent des "arbres massifs de nœuds avec des noms super longs et similaires"

**Export web trop lourd**
Un jeu simple pèse 20+ MB en export Godot vs 2 MB pour le même jeu en Phaser 3. C'est un obstacle majeur pour la distribution web.

**Shaders limités**
Le langage de shaders manque de certaines fonctions GLSL/HLSL, a des boucles capricieuses, des arrays dynamiques très limités, et les shaders ne peuvent pas facilement voir toutes les lumières en Forward+.

**Features incomplètes / transitions cassées**
Les features sont parfois cassées en milieu de développement avec la promesse de remplacement "bientôt"... qui prend des années. Les bugs mineurs sont prioritisés au détriment des problèmes majeurs.

**Documentation lacunaire**
Manque de documentation et de tutoriels par rapport à Unity/Unreal. La part de marché plus petite signifie moins de ressources tierces.

Sources :
- [Godot Performance Limitations - Forum](https://forum.godotengine.org/t/what-are-godots-performance-limitations-in-2d/27061)
- [Godot TileMap Editor Slow - GitHub Issue](https://github.com/godotengine/godot/issues/72405)
- [Godot Pros & Cons - TrustRadius](https://www.trustradius.com/products/godot/reviews?qs=pros-and-cons)
- [My Thoughts on Godot - Pandaqi Blog](https://pandaqi.com/blog/reviews-and-thoughts/my-thoughts-on-godot-engine/)
- [Godot Shader Limitations - GitHub Proposal](https://github.com/godotengine/godot-proposals/issues/14105)

---

### Unity (2D)

**Crise de confiance — Runtime Fee**
En septembre 2023, Unity a annoncé des frais par installation de jeu. L'annonce a provoqué un exode massif de développeurs indie. Même après le retrait de la mesure, la confiance n'a pas été restaurée. Les développeurs ont appris que leurs outils peuvent devenir des liabilités du jour au lendemain.

**La 2D est un citoyen de seconde classe**
Unity est fondamentalement un moteur 3D. La 2D est "greffée" dessus. Résultat :
- Pipeline de rendu complexe et opaque pour de la simple 2D
- Le système URP (Universal Render Pipeline) a connu des départs/licenciements dans l'équipe
- Overhead inutile : des centaines de MB pour un projet "hello world" 2D
- DOTS/ECS est puissant mais pas accessible aux débutants

**Complexité croissante**
L'interface est écrasante pour les débutants. La courbe d'apprentissage est raide. Chaque version ajoute de la complexité sans simplifier l'existant.

**Performance en retrait**
Lag derrière d'autres moteurs en graphiques et framerate pour des cas équivalents.

Sources :
- [Why Developers Leave Unity - GeekWire](https://www.geekwire.com/2023/heres-why-so-many-video-game-developers-are-suddenly-abandoning-the-unity-engine/)
- [Goodbye Unity - Medium](https://medium.com/codex/goodbye-unity-b783a48c8b5d)
- [URP Team Quit/Laid Off - Unity Forums](https://discussions.unity.com/t/urp-team-quit-laid-off/939320)
- [Why Developers Leaving Unity](https://www.gameslearningsociety.org/wiki/why-are-developers-leaving-unity/)

---

### GameMaker

**Scaling difficile**
Excellent pour les prototypes et petits jeux, mais une fois qu'on essaie de scaler, le scripting (GML) devient bizarre et limité. Les patterns avancés sont maladroits.

**GML n'est pas un vrai langage**
GML manque de features de langage modernes (generics, types forts, modules). Le passage de DnD à GML est un saut, et GML vers quelque chose de plus puissant est un autre saut sans passerelle.

**Licensing et prix**
Le modèle de licence est devenu plus restrictif et cher au fil du temps.

**Pas de 3D sérieuse**
Si le projet évolue vers de la 3D (même partielle), GameMaker ne suit plus.

---

### Construct 3

**Modèle par abonnement**
L'abonnement obligatoire irrite les développeurs indie. Pas d'achat perpétuel.

**HTML5 uniquement**
Tous les jeux sont en HTML5, ce qui impose des limites de performance et complique le déploiement natif desktop/mobile.

**Plafond de complexité**
Les event sheets deviennent difficiles à gérer pour des projets larges. Pas de vrai système de modules/namespaces.

**DnD limité**
Le système drag-and-drop, bien que accessible, a un plafond bas pour les logiques complexes.

---

### Love2D

**Pas d'éditeur visuel**
Aucun éditeur — tout est code. Cela exclut les non-programmeurs et rend le level design fastidieux.

**Communauté minuscule**
La communauté est petite, ce qui limite les ressources, tutoriels, et bibliothèques disponibles.

**Lua uniquement**
Pas de choix de langage. Lua a ses particularités (indexation à 1, pas de classes natives) qui frustrent.

**Pas de pipeline d'assets**
Aucun outil intégré de gestion d'assets, de packaging, ou de build automatisé.

---

### RPG Maker

**Enfermement dans un genre**
Uniquement des JRPGs 2D. Tout projet qui s'écarte du moule devient une lutte contre le moteur.

**Animations limitées**
L'animation de marche par défaut n'a que 3 frames. Améliorer cela nécessite du scripting, inaccessible aux débutants.

**Système de combat rigide**
Uniquement des combats au tour par tour sans plugins communautaires.

**Performance avec des animations larges**
Mauvaise performance avec des animations haute résolution, surtout dans RPG Maker MV.

**Customisation limitée sans plugins**
Le moteur force des éléments de game design spécifiques. La personnalisation profonde nécessite des plugins, créant une dépendance communautaire.

Sources :
- [Godot vs RPG Maker - Aircada](https://aircada.com/blog/godot-vs-rpg-maker)
- [RPG Maker Limitations - Slant](https://www.slant.co/versus/1068/1085/~godot_vs_rpgmaker)

---

### Bevy (Rust)

**Pas d'éditeur**
Le reproche #1. "Trop de travail pour des scènes simples", "impossible de savoir où sont les choses sans exécuter le jeu", "boucle d'itération lente pour tweaker des scènes". Les artistes sont bloqués.

**Artefacts 2D**
Le moteur tourne toujours en 3D internement. Les sprites avec alpha se coupent les uns les autres de façon bizarre quand ils se chevauchent.

**UI et audio immatures**
Assets, UI et audio sont "notably immature". Construire une UI simple demande un effort disproportionné.

**API instable**
Pas encore en 1.0. L'API change fréquemment, cassant le code existant entre versions. La documentation est souvent en retard.

**Temps de compilation**
Les projets Rust ont des temps de compilation longs, ce qui ralentit l'itération.

Sources :
- [Bevy 2D Production Readiness - GitHub Discussion](https://github.com/bevyengine/bevy/discussions/14722)
- [Bevy Not Ready for Large-Scale - Hacker News](https://news.ycombinator.com/item?id=39842636)
- [Bevy's Fifth Birthday - Substack](https://rinoxide.substack.com/p/bevys-fifth-birthday-the-editor)

---

### Defold

**Système de communication restrictif**
Communication par messages uniquement entre game objects — peut sembler rigide comparé à d'autres patterns.

**Système GUI dédié et difficile à réutiliser**
Le type de collection GUI est difficile à recombiner librement.

**Communauté petite**
Moins de ressources, tutoriels et plugins que les alternatives majeures.

---

### Phaser (web)

**Web uniquement**
Pas de builds natifs desktop/mobile sans wrapper (Electron, Cordova).

**Performance limitée**
JavaScript impose un plafond de performance pour les jeux complexes.

**Pas d'éditeur visuel**
Tout est code, comme Love2D.

---

## 2. Problèmes transversaux à tous les moteurs

### Bloat et overhead

Les développeurs se plaignent de projets qui pèsent des centaines de MB pour des jeux 2D simples. Les moteurs embarquent des systèmes 3D, VR, networking — inutiles pour de la 2D.

> "90% des features fournies par ces moteurs ne sont jamais utilisées" — Noel Berry (co-créateur de Celeste)

### Updates qui cassent les projets

Les mises à jour de moteurs brisent régulièrement le code existant. Les développeurs se retrouvent à devoir réécrire du code qui fonctionnait, ou à rester sur des versions anciennes sans les correctifs de sécurité.

### Pipeline de rendu opaque

Impossible de debugger les problèmes de rendu quand le pipeline est une boîte noire. Les développeurs veulent comprendre et contrôler chaque couche.

### Networking / Multiplayer quasi-absent

Le rollback netcode pour les jeux 2D (fighting games, platformers coopératifs) est :
- Extrêmement complexe à implémenter
- Nécessite que le jeu soit **100% déterministe** sur toutes les machines
- Budget CPU de ~1.1ms pour resimulation de 15 frames dans un jeu à 60fps
- Artefacts visuels lors des rollbacks
- Aucun moteur 2D n'offre de solution intégrée satisfaisante

### Localisation douloureuse

- Intégration CI/CD difficile
- Les traducteurs ne voient pas le contexte visuel
- Formats de fichiers différents nécessitent des workflows séparés
- L'overflow de texte dans des langues plus longues n'est pas géré automatiquement

### Accessibilité (pour les joueurs) ignorée

- Quasi aucun moteur n'offre de support natif pour les lecteurs d'écran
- Les modes daltonien sont rarement intégrés au moteur (chaque dev doit les implémenter)
- Le remapping de contrôles est laissé au développeur
- Pas de standards d'accessibilité dans les moteurs

> "Ce serait un progrès majeur si les moteurs de jeu fournissaient des features d'accessibilité intégrées comme les modes daltonien, le text-to-speech, et des composants UI accessibles."

### Version control problématique

Les formats de scène binaires ou verbeux génèrent des conflits de merge irrésolvables. Les fichiers de métadonnées (.meta dans Unity) polluent les diffs.

---

## 3. Post-mortems de jeux indie

### Celeste (FNA/MonoGame, custom framework "Monocle")

- L'équipe a construit son propre framework C# (Scene→Entity→Component) par-dessus XNA/FNA
- **Frustration** : le portage console nécessitait de transpiler le C# en C++ via un outil (BRUTE) — "worked but was not ideal"
- **Leçon** : Noel Berry (co-créatrice) est devenue une avocate du développement sans moteur, citant le contrôle total et la capacité à créer des outils sur mesure pour l'équipe

### Hollow Knight (Unity)

- Team Cherry a utilisé **PlayMaker** (scripting visuel) pour créer TOUS les ennemis et éléments interactifs, car le code C# seul était trop lent à itérer
- L'éclairage 2D a été fait avec des **formes transparentes softées** plutôt que le système de lumières Unity — workaround car le système natif ne convenait pas
- Dépendance à des assets tiers (2D Toolkit) pour compenser les lacunes de Unity en 2D

### Stardew Valley (XNA/MonoGame custom)

- Eric Barone a construit un **moteur propriétaire sur mesure** en C# sur XNA
- **Limitations** : shaders avancés, moteur physique, et pipeline d'assets moins accessibles, nécessitant des workarounds manuels
- **Portage** : XNA ne supportait que Windows/Xbox 360. Le portage vers Mac/Linux/consoles a nécessité un travail énorme et une migration vers MonoGame
- **Scaling** : XNA n'était pas assez efficace pour gérer des fermes très grandes ou beaucoup plus de PNJs
- **Support déclinant** : après l'arrêt de XNA par Microsoft en 2013, la communauté s'est réduite

### Dead Cells (custom engine en Haxe)

- Motion Twin a construit Dead Cells sur un moteur custom en **Haxe** (compilé en C) avec le framework Heaps
- Leur approche "hand-designed procedural" (morceaux de salles créés à la main, assemblés procéduralement) nécessitait un outillage custom qu'aucun moteur n'offrait
- Ils ont construit un **ECS custom** pour la performance avec beaucoup d'ennemis et de projectiles simultanés
- Le pipeline de rendu custom (pixel art avec éclairage per-pixel, particules) était nécessaire pour le style visuel distinctif
- Le choix d'un langage niche a compliqué le recrutement mais donné une performance optimale

### Cuphead (Unity)

- Studio MDHR a choisi Unity pour le C# et les outils 2D, malgré un pipeline d'art extrêmement non-standard (animation dessinée à la main sur papier)
- **Pipeline d'animation custom** : dessiner → encrer → scanner → nettoyer → colorier dans Photoshop → importer dans Unity. Aucun support moteur pour ce workflow. Outils custom pour gérer des milliers de frames dessinées à la main.
- **Chaque frame d'animation = un sprite unique full-résolution** (pas d'animation squelettique, pas de réutilisation). Demandes mémoire et rendu énormes.
- Le système de combat précis boss-pattern nécessitait physique et collision custom
- Le layering parallax profond nécessitait un système de caméra et de couches custom
- **Leçon** : quand le style artistique est le différenciateur principal, le pipeline d'assets du moteur est quasi certainement inadéquat

### Hades (moteur custom propriétaire — Supergiant Games)

- Supergiant utilise son propre moteur depuis Bastion, amélioré itérativement sur 4 jeux
- **Intégration serrée design tools ↔ jeu** : les designers tweakent gameplay, dialogues, encounters sans intervention de programmeur
- **Système de narration procédurale custom** : le tracking d'états relationnels à travers des centaines de runs nécessitait un scripting profondément intégré
- **Simulation déterministe** : essentielle pour le replay de séquences exactes en testing et équilibrage
- **Itération rapide** : hot-reloading de scripts et données, test instantané play-from-here
- **Leçon** : les studios qui prévoient plusieurs jeux bénéficient énormément d'investir dans un moteur custom ajusté à leur genre et workflow

### Factorio (moteur custom, C++/Lua)

- Wube Software a construit un moteur custom parce qu'**aucun moteur existant ne pouvait gérer l'échelle de simulation** (millions d'items, convoyeurs, inserters, réseaux logistiques mis à jour chaque tick)
- **Simulation extrême** : simulation déterministe lockstep de millions d'entités — bien au-delà de tout moteur généraliste
- **Multiplayer déterministe** : le modèle lockstep nécessite un déterminisme bit-perfect sur tous les clients
- **Rendu custom pour mondes énormes** : batching et culling custom pour des usines avec des milliers de sprites animés
- **Framework de modding Lua** : intégration Lua profonde comme objectif de conception dès le départ
- **Leçon** : les jeux de simulation à grande échelle nécessiteront toujours des moteurs custom. Les moteurs généralistes optimisent pour le cas commun (centaines à quelques milliers d'entités), pas pour l'extrême.

### Leçon commune

Les jeux indie 2D les plus réussis ont souvent **construit ou fortement customisé leurs outils**, contournant les limitations des moteurs existants. Le pattern récurrent : le moteur fournit la base, mais les équipes passent un temps significatif à créer des outils sur mesure que le moteur aurait dû fournir.

Sources :
- [Making Games in 2025 Without an Engine - Noel Berry](https://www.noelberry.ca/posts/making_games_in_2025/)
- [Hollow Knight - Inside the Mind of a Bug](https://www.teamcherry.com.au/blog/inside-the-mind-of-a-bug-unity-and-playmaker)
- [Stardew Valley Engine Deep Dive](https://cruiseship.cloud/blog/2025/11/11/what-engine-does-stardew-valley-use/)

---

## 4. Le phénomène "engine fatigue" et l'exode Unity

### L'exode Unity (2023-2024)

Après l'annonce du Runtime Fee :
- **Godot** a vu un afflux massif de développeurs, devenant le choix #1 des indie 2D
- **Defold**, **GameMaker**, **Construct** ont aussi bénéficié de migrations
- Beaucoup de développeurs ont commencé à explorer le **développement sans moteur**

### "Engine fatigue" — un phénomène croissant

Le sentiment communautaire a évolué vers ce que les développeurs appellent "engine fatigue" :
- Fatigue des cycles de mises à jour qui cassent les projets
- Fatigue des pipelines de rendu opaques impossibles à debugger
- Fatigue des projets de centaines de MB pour des jeux 2D simples
- Fatigue du risque business (le moteur peut changer ses conditions à tout moment)

### La renaissance du "sans moteur" (2025-2026)

Noel Berry (Celeste), et d'autres développeurs influents, prônent l'utilisation de **bibliothèques légères** (SDL3, Raylib, sokol) plutôt que de moteurs monolithiques :

> "La philosophie est 'bibliothèque, pas framework' : tirez exactement ce dont vous avez besoin, composez les systèmes vous-même, et comprenez chaque couche."

Les outils modernes (Zig, Odin, Rust) rendent le développement bas niveau beaucoup moins pénible qu'avec C/C++ historiquement.

Sources :
- [Game Dev Without An Engine: 2025/2026 Renaissance - SitePoint](https://www.sitepoint.com/game-dev-without-an-engine-the-2025-2026-renaissance/)
- [Real Reasons to Build Custom Engines - Game Developer](https://www.gamedeveloper.com/programming/real-reasons-not-to-build-custom-game-engines-in-2024)
- [Picking a Game Engine in 2025 - GameDev.net](https://www.gamedev.net/blogs/entry/2295692-picking-a-game-engine-in-2025-without-crying/)

---

## 5. Niches mal servies

### Jeux 2D avec éclairage/shaders avancés
Pixel art + lumières dynamiques + ombres + shaders custom = un cauchemar dans la plupart des moteurs. Les solutions existantes sont des workarounds (Hollow Knight utilise des formes transparentes au lieu du vrai éclairage). Un moteur avec un pipeline de lumières 2D natif et puissant serait différenciant.

### Grands mondes 2D ouverts
Les tilemaps massives causent des problèmes de performance dans Godot et d'autres. Le streaming/chargement par chunks, le LOD 2D, et l'instanciation efficace de grandes étendues sont mal supportés.

### Multiplayer 2D avec rollback netcode
Aucun moteur 2D n'offre de solution de netcode intégrée. Les développeurs de fighting games, platformers coopératifs et jeux compétitifs doivent tout implémenter eux-mêmes. Le rollback nécessite du déterminisme total — une contrainte architecturale qui doit être pensée dès le core engine.

### Génération procédurale
Peu d'outils intégrés pour la génération procédurale de niveaux, de donjons, de mondes. Les développeurs doivent coder tout de zéro.

### Jeux physique-heavy en 2D
Les moteurs physiques 2D intégrés (Godot, Unity) sont soit buggés, soit insuffisants pour des jeux qui reposent fortement sur la physique (Noita, Angry Birds, physics puzzles). Box2D est solide mais l'intégration est souvent pénible.

### Accessibilité native
Aucun moteur ne fournit nativement : lecteurs d'écran, modes daltonien, remapping de contrôles, text-to-speech, navigation UI par clavier/gamepad. C'est toujours laissé au développeur.

---

## 6. Ce que les développeurs souhaitent

### Contrôle et transparence
- Pouvoir debugger chaque couche du moteur
- Comprendre le pipeline de rendu
- Fixer les bugs soi-même sans attendre un patch
- Pas de boîtes noires

### Légèreté et focus 2D
- Un moteur construit POUR la 2D, pas de la 2D greffée sur de la 3D
- Taille de projet minimale
- Temps de démarrage instantané
- Export web léger (< 5 MB)

### Stabilité et confiance
- Pas de breaking changes entre versions
- Pas de risque business (licensing stable, idéalement open source)
- API stable sur laquelle construire des jeux sur plusieurs années

### Itération rapide
- Hot-reload de TOUT (assets, scripts, shaders, scènes)
- Boucle edit→test en secondes, pas en minutes
- Éditeur intégré au runtime (pas un process séparé)

### Outils pour non-programmeurs
- Scripting visuel qui passe à l'échelle (event sheets > node-based)
- Éditeur de niveaux WYSIWYG
- Behaviors pré-construits (plateforme, top-down, physique)
- Templates de projet fonctionnels immédiatement

### Pipeline d'assets fluide
- Import automatique des fichiers source
- Hot-reload des textures depuis Aseprite/Photoshop
- Packing d'atlas automatique
- Gestion des formats sans friction

---

## 7. Opportunités pour un nouveau moteur

### Le "Missing Middle"

Il y a un gap entre :
- Les outils **simples mais limités** (Construct, RPG Maker) — accessibles mais plafond bas
- Les moteurs **puissants mais complexes** (Unity, Godot) — capables mais écrasants

**L'opportunité** : un moteur 2D qui est aussi accessible que Construct pour commencer, mais aussi puissant que Godot pour les projets ambitieux. Complexité progressive sans plafond artificiel.

### Focus 2D pur

Aucun moteur majeur n'est construit exclusivement pour la 2D avec une performance de premier rang. Godot est le plus proche mais traîne du poids 3D. Un moteur 100% 2D pourrait offrir :
- Taille d'export minimale (web < 3 MB)
- Performance supérieure sur les scénarios 2D (tilemaps géantes, milliers de sprites)
- Pipeline de rendu 2D optimisé (lumières 2D, ombres, particules, shaders)
- Éditeur de tilemap qui ne lag pas

### Open source et licensing stable

L'exode Unity a montré que les développeurs veulent de la stabilité et de l'ownership. Un moteur open source avec une gouvernance claire et sans surprise de licensing comble un besoin émotionnel autant que pratique.

### Accessibilité intégrée

Premier moteur à offrir nativement le support d'accessibilité (lecteurs d'écran, daltonisme, remapping). Différenciant majeur et aligné avec les exigences légales croissantes.

### Multiplayer 2D first-class

Architecture déterministe dès le core, avec rollback netcode intégré ou facilement ajouteable. Répondrait à une demande massive non satisfaite.

### Pilotable par IA

Un moteur conçu dès le départ pour être manipulé par des IA assistantes (Claude, GPT, etc.) serait une proposition de valeur unique. Formats déclaratifs, API consistante, feedback structuré.

*(Voir le document dédié : `06-ai-driven-engine.md`)*

---

## 8. Dream Feature List — Synthèse de la communauté

### Tier 1 — Demandes quasi-universelles

1. **Pipeline de rendu pixel-perfect** avec solutions intégrées pour le sub-pixel movement, le camera smoothing, et le scaling de résolution
2. **Système de tilemap first-class** avec auto-tiling basé sur des règles, support efficace de grands mondes, et collision intégrée
3. **Archétypes de character controller** — platformer, top-down, isométrique — qui "feel good" out-of-the-box, avec paramètres exposés pour le game feel (coyote time, input buffering, courbes d'accélération)
4. **Déploiement cross-plateforme one-click** qui marche vraiment — surtout web (WASM) et mobile
5. **Pricing fair, prédictible et permanent** — idéalement open source
6. **Éditeur rapide et stable** avec des temps d'itération sub-seconde
7. **Documentation excellente** avec référence API complète, guides conceptuels et exemples curated

### Tier 2 — Forte demande des développeurs expérimentés

8. **Export console** sans middleware prohibitif
9. **Rollback netcode intégré** pour le multiplayer 2D
10. **Time-travel debugging** — rembobiner, stepper, inspecter l'état du jeu à n'importe quel point
11. **Simulation déterministe** pour les replays, le netcode et les tests automatisés
12. **Overlays de debug visuels** — formes de collision, raycasts, pathfinding, état IA — toujours disponibles sans code custom
13. **Éclairage 2D intégré** avec normal maps, ombres soft, et bonne performance
14. **Framework d'accessibilité intégré** — support lecteur d'écran, filtres daltonien, remapping input, ajustement de difficulté
15. **Fichiers de scène conçus pour le version control** — lisibles, merge-friendly, métadonnées minimales

### Tier 3 — Demande significative de communautés spécifiques

16. **Toolkit de génération procédurale** — fonctions de bruit, WFC, pipelines de génération graph-based
17. **Framework de modding** — scripting Lua sandboxé, système d'override d'assets, gestion de mods
18. **Système de localisation** — tables de strings, pseudolocalisation, support RTL, fallbacks de polices
19. **Scripting visuel pour designers** qui est réellement puissant (pas un jouet)
20. **Système de dialogue/narration intégré** avec branching, conditions et localisation
21. **Édition de sprites intégrée** — au minimum : palette swaps, édition de frames simple, support natif format Aseprite
22. **Hot-reloading de tout** — code, assets, shaders, scènes — sans redémarrage
23. **Outils de profiling conçus pour la 2D** — analyse de draw calls, visualisation de batches, heatmaps d'overdraw
24. **Outils d'analytics et de playtesting** — heatmaps, enregistrement de sessions, agents de playtesting automatisés

### Tier 4 — Tournés vers l'avenir

25. **Playtesting assisté par IA** — bots qui jouent au jeu et reportent les problèmes
26. **Édition collaborative en temps réel** — plusieurs personnes éditant un niveau simultanément
27. **Intégration de génération d'assets par IA** — générer des variations de sprites, effets de particules, ou art placeholder depuis le moteur
28. **Analytics self-hosted / respectueux de la vie privée**
29. **Outils de conformité légale** — flows de consentement RGPD, checklists d'audit accessibilité

---

## 9. Les 3 méta-thèmes

### La 2D n'est PAS de la 3D simplifiée

La frustration la plus profonde : les moteurs traitent la 2D comme une version réduite de la 3D. Mais le dev 2D a ses propres besoins uniques : pipelines pixel art, mondes tile-based, animation frame-by-frame, effets screen-space, physique de platformer, gestion de résolution. Un moteur construit **2D-first**, pas 2D-en-sous-ensemble, est ce dont les développeurs rêvent.

### Contrôle vs. Commodité — Aucun moteur ne trouve le bon équilibre

Les moteurs (Unity, Godot, GameMaker) offrent la commodité mais retirent le contrôle. Les frameworks (MonoGame, FNA, Raylib) offrent le contrôle sans commodité. Les développeurs veulent les deux : des défauts opinionés qui marchent out-of-the-box, avec la possibilité de remplacer n'importe quel système par une implémentation custom. L'idéal est la **"divulgation progressive de complexité"** — les choses simples sont simples, les choses complexes sont possibles.

### Confiance et durabilité

Après le fiasco de pricing Unity 2023, les développeurs se soucient profondément de la fiabilité à long terme de leur choix de moteur. L'idéal : open source (ou licence irrévocable), modèle de financement durable, gouvernance communautaire, et aucune entité corporate unique qui puisse changer les règles. C'est autant un problème business/communautaire que technique.
