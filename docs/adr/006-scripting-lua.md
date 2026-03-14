# ADR-006 : Lua comme langage de scripting

- **Statut :** Acceptée
- **Date :** 2026-03-14
- **Concerne :** v0.1
- **Dépend de :** ADR-001 (Rust)

## Contexte

Le moteur a besoin d'un langage de scripting pour permettre aux développeurs de jeux d'écrire la logique de gameplay sans recompiler le moteur. Ce langage doit être hot-reloadable (modifier un script et voir le résultat immédiatement), accessible aux non-experts, et performant pour la logique de jeu.

## Options considérées

### DSL custom (style GDScript)
- **Pour :** conçu spécifiquement pour le game dev. Syntaxe Python-like optimisée pour les opérations de jeu. Intégration éditeur maximale (autocomplétion, documentation inline). L'approche qui a fait le succès de Godot.
- **Contre :** concevoir un bon DSL est un projet de **plusieurs mois** (parser, VM/compilateur, debugger, messages d'erreur, documentation). Un mauvais DSL est pire que pas de DSL. Pas de documentation existante ni de communauté. Retarderait le MVP de façon inacceptable.

### Lua (via mlua)
- **Pour :** 30 ans de track record en jeux (WoW, Roblox, Love2D, Defold, Factorio). LuaJIT offre des performances quasi-natives. mlua fournit des bindings Rust sûrs et ergonomiques. Hot-reload natif par design. Langage petit = une IA peut contenir toute l'API en contexte. Communauté existante, documentation massive.
- **Contre :** indexation à 1 (confus pour les devs venant d'autres langages). Pas de classes natives (metatables obscures). Pas de typage statique. Pas de valeur en dehors du moteur (compétence non-transférable).

### JavaScript/TypeScript
- **Pour :** le langage le plus connu au monde. Écosystème massif. TypeScript ajoute le typage statique. Potentiel de partage de code avec l'export web.
- **Contre :** runtime lourd (V8 ou équivalent). L'embedding en Rust est complexe (deno_core, boa). Performance inférieure à LuaJIT pour les boucles serrées. "The bad parts" du langage. Hot-reload plus complexe.

### Wren
- **Pour :** conçu pour l'embedding en jeux. Syntaxe propre, class-based. Fibers pour les coroutines. Petit et élégant.
- **Contre :** communauté minuscule. Peu de ressources d'apprentissage. Développement ralenti. Les bindings Rust sont moins matures que mlua. Risque de dépendre d'un projet qui peut devenir abandonné.

### Rhai
- **Pour :** Rust pur (pas de FFI). Syntaxe proche de Rust/JS. Conçu pour l'embedding.
- **Contre :** performance inférieure à LuaJIT. Communauté petite. Moins connu que Lua dans le game dev. Les LLMs produisent moins bien du Rhai que du Lua (moins de données d'entraînement).

## Décision

**Lua via mlua (backend LuaJIT).**

1. **Livrer le MVP.** Un DSL custom retarderait le MVP de mois. Lua existe, fonctionne, et a prouvé son adéquation au game dev pendant 30 ans. C'est le choix pragmatique.

2. **Hot-reload natif.** Les modules Lua se rechargent au runtime par design (`package.loaded[mod] = nil; require(mod)`). On obtient le hot-reload de scripts quasi gratuitement.

3. **Performance LuaJIT.** Pour la logique de gameplay (IA ennemis, mécaniques de jeu, gestion d'événements), LuaJIT est largement suffisant — et souvent quasi-natif grâce au JIT.

4. **AI-friendly.** Lua est un petit langage. Les LLMs le maîtrisent très bien. L'API complète du moteur tient en quelques pages de documentation — parfait pour le contexte limité des LLMs.

5. **Communauté.** Les développeurs venant de Love2D, Defold, Factorio modding connaissent déjà Lua. C'est un pont vers ces communautés.

**Note :** un DSL custom (style GDScript) reste une option post-1.0 quand on aura assez de maturité pour savoir exactement ce que le DSL devrait offrir. Lua n'est pas un choix temporaire — c'est un choix permanent qui pourra être complété (pas remplacé) par un DSL plus tard.

## Conséquences

### Positives
- Scripting fonctionnel en quelques jours de développement, pas en mois
- Hot-reload natif sans infrastructure complexe
- Performance LuaJIT quasi-native
- Les LLMs génèrent du Lua de bonne qualité
- Communauté game dev existante autour de Lua

### Négatives
- Indexation à 1 (confusion pour les débutants venant de Python/JS)
- Pas de typage statique (les erreurs sont au runtime)
- Pas de classes natives (les patterns OOP nécessitent des metatables)
- Compétence non-transférable hors du game dev
- Les messages d'erreur Lua peuvent être cryptiques (mitigé par des wrappers d'erreur côté moteur)
