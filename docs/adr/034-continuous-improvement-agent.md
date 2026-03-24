# ADR-034 : Agent d'amelioration continue — Bug Reporter + Log Watcher → GitHub Issues

- **Statut :** Proposee
- **Date :** 2026-03-24
- **Concerne :** v0.5 (MVP)

## Contexte

Le copilote IA de l'editeur (ADR-033) peut creer et modifier des scenes, mais il n'a aucun mecanisme pour signaler les **bugs du moteur Toile** qu'il rencontre "sur le terrain".

Distinction fondamentale :
- **Bug projet** : l'utilisateur a mal configure son event sheet, son prefab est incomplet → c'est le role du copilote de l'aider a corriger (dans la conversation)
- **Bug moteur** : le moteur ne sauvegarde pas les prefabs sur disque, les collisions produisent des NaN, un event sheet valide ne s'execute pas → c'est un bug dans le code de Toile qui doit etre corrige dans le repo

Seuls les **bugs moteur** doivent etre remontes. Le bon endroit pour ca : **GitHub Issues** sur `tfoutrein/toile-engine`. C'est tracable, public, avec labels, et Claude Code peut les lire via `gh issue list` pour les corriger.

L'objectif est de creer une **boucle d'amelioration continue** :
1. L'editeur detecte un bug moteur (via le copilote ou le log watcher)
2. Une GitHub Issue est creee automatiquement avec tout le contexte
3. Claude Code (en CLI) lit les issues et corrige le code

## Decision

**Creer des GitHub Issues automatiquement depuis l'editeur quand un bug moteur est detecte, via l'API GitHub (gh CLI ou API REST).**

### Architecture

```
┌──────────────────────────────────────────────────────┐
│                    Toile Editor                       │
│                                                      │
│  ┌──────────────┐                                    │
│  │ Copilote IA  │──── report_bug ────┐               │
│  │ (detecte un  │                    │               │
│  │  bug moteur) │                    ▼               │
│  └──────────────┘          ┌─────────────────┐       │
│                            │  Issue Creator   │       │
│  ┌──────────────┐          │  (deduplique +   │       │
│  │ Log Watcher  │─────────→│   cree l'issue)  │       │
│  │ (periodique) │          └────────┬────────┘       │
│  └──────────────┘                   │                │
└─────────────────────────────────────┼────────────────┘
                                      │ gh issue create
                                      ▼
                    ┌──────────────────────────────┐
                    │  GitHub Issues                │
                    │  tfoutrein/toile-engine           │
                    │                              │
                    │  label: auto-detected        │
                    │  label: bug / crash / perf   │
                    └──────────────┬───────────────┘
                                   │ gh issue list
                                   ▼
                    ┌──────────────────────────────┐
                    │  Claude Code (CLI)            │
                    │  "corrige les issues auto"    │
                    │  → gh issue list --label      │
                    │  → lit le contexte            │
                    │  → corrige le code source     │
                    │  → commit + PR + close issue  │
                    └──────────────────────────────┘
```

### 1. Nouvel outil copilote : `report_bug`

Ajoute aux tool definitions du copilote IA (dans `tools.rs`).

Le copilote utilise cet outil **uniquement** quand il identifie un probleme dans le moteur Toile — pas quand l'utilisateur a fait une erreur dans son projet.

```json
{
    "name": "report_bug",
    "description": "Reporter un bug dans le moteur Toile (pas un probleme du projet utilisateur). Cree une GitHub Issue sur tfoutrein/toile-engine avec le contexte technique.",
    "input_schema": {
        "type": "object",
        "properties": {
            "severity": {
                "type": "string",
                "enum": ["bug", "crash", "perf", "enhancement"],
                "description": "bug = comportement incorrect, crash = panic/erreur fatale, perf = probleme de performance, enhancement = fonctionnalite manquante"
            },
            "title": {
                "type": "string",
                "description": "Resume court du bug (une ligne, en anglais)"
            },
            "description": {
                "type": "string",
                "description": "Description detaillee : comportement observe vs attendu, etapes de reproduction, logs pertinents"
            },
            "component": {
                "type": "string",
                "enum": ["editor", "runner", "events", "behaviors", "collision", "scene", "prefabs", "renderer", "audio", "other"],
                "description": "Composant de Toile concerne"
            },
            "logs": {
                "type": "array",
                "items": { "type": "string" },
                "description": "Lignes de log pertinentes"
            }
        },
        "required": ["severity", "title", "description", "component"]
    }
}
```

**Quand le copilote doit l'utiliser :**
- Un tool call echoue pour une raison interne (ex: `save_as_prefab` ne cree pas le fichier)
- Les logs du Game Runner montrent une erreur dans le moteur (ex: `NaN` dans la physique)
- Un behavior ne se comporte pas comme documente
- Un event sheet valide ne s'execute pas correctement

**Quand il ne doit PAS l'utiliser :**
- L'utilisateur a oublie un tag sur une entite
- Un prefab n'existe pas parce que l'utilisateur ne l'a pas cree
- La scene est vide parce que l'utilisateur n'a rien ajoute

Le system prompt du copilote inclura cette distinction.

### 2. Log Watcher dans l'editeur

Un composant periodique qui analyse les logs de l'editeur et du Game Runner pour detecter les bugs moteur.

**Sources des logs :**
- Logs de l'editeur (via un `log::Log` custom qui capture dans un ring buffer)
- Logs du Game Runner (deja captures dans `game_logs`)

**Regles de detection — uniquement des patterns qui revelent un bug moteur :**

| Pattern | Severite | Composant | Interpretation |
|---------|----------|-----------|----------------|
| `panic` / `unwrap failed` / `thread.*panicked` | crash | auto-detect | Crash dans le moteur |
| `NaN` / `Infinity` dans un calcul physique | crash | collision | Bug numerique |
| `index out of bounds` | crash | auto-detect | Acces hors limites |
| Meme `[WARN]` repete 50+/sec | perf | auto-detect | Boucle de rechargement |
| `texture.*destroyed` / `wgpu.*error` | bug | renderer | Bug de rendu |
| `event sheet.*parse.*error` sur un fichier valide | bug | events | Bug de parsing |
| `failed to write` / `permission denied` sur ops internes | bug | auto-detect | Bug I/O |

**Ce que le watcher ignore** (problemes projet, pas moteur) :
- `prefab not found: X` → l'utilisateur n'a pas cree le prefab
- `sprite not found: X` → asset manquant dans le projet
- `no entity with tag X` → configuration du projet

**Implementation :**

```rust
pub struct LogWatcher {
    editor_log_buffer: Arc<Mutex<VecDeque<LogEntry>>>,
    reported_hashes: HashSet<u64>,
    last_analysis: Instant,
    analysis_interval: Duration, // 30 secondes
    pending_issues: Vec<DetectedIssue>,
}
```

Le watcher produit des `DetectedIssue` qui sont ensuite envoyees a l'Issue Creator.

### 3. Issue Creator — creation des GitHub Issues

Composant responsable de creer les issues sur GitHub. Utilise soit `gh` CLI, soit l'API GitHub REST.

**Option A : `gh` CLI (recommandee pour la Phase 1)**

```rust
fn create_github_issue(issue: &DetectedIssue) -> Result<String, String> {
    let label = match issue.severity {
        Severity::Crash => "crash",
        Severity::Bug => "bug",
        Severity::Perf => "performance",
        Severity::Enhancement => "enhancement",
    };

    let body = format!(
        "## Auto-detected by Toile Editor\n\n\
         **Component:** {}\n\
         **Severity:** {}\n\
         **Source:** {}\n\n\
         ## Description\n\n{}\n\n\
         ## Relevant logs\n\n```\n{}\n```\n\n\
         ## System info\n\n\
         - Toile version: {}\n\
         - OS: {}\n\n\
         ---\n\
         *This issue was automatically created by the Toile Editor bug reporter.*",
        issue.component, label, issue.source,
        issue.description,
        issue.logs.join("\n"),
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
    );

    let output = std::process::Command::new("gh")
        .args(["issue", "create",
               "--repo", "tfoutrein/toile-engine",
               "--title", &issue.title,
               "--body", &body,
               "--label", label,
               "--label", "auto-detected"])
        .output()
        .map_err(|e| format!("gh not found: {e}"))?;

    // Retourne l'URL de l'issue creee
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}
```

**Deduplication :**
- Avant de creer, `gh issue list --label auto-detected --search "title"` pour verifier si une issue similaire existe deja
- Hash local (titre + composant) stocke en memoire pour eviter les appels API redondants dans une meme session

**Consentement utilisateur :**
- Premiere fois : popup "Toile veut creer une GitHub Issue pour un bug detecte. Autoriser ?"
- Option dans Settings : "Auto-report bugs to GitHub" (on/off)
- Toujours afficher un toast quand une issue est creee ("Bug reported: #42")

### 4. Capture des logs de l'editeur

Ajouter un log collector qui capture les messages dans un ring buffer partage :

```rust
struct EditorLogCollector {
    buffer: Arc<Mutex<VecDeque<LogEntry>>>,
    inner: env_logger::Logger, // delegation pour la sortie stderr habituelle
    max_lines: usize,          // 2000
}

impl log::Log for EditorLogCollector {
    fn log(&self, record: &log::Record) {
        // Capturer dans le buffer
        let mut buf = self.buffer.lock().unwrap();
        buf.push_back(LogEntry {
            timestamp: Instant::now(),
            level: record.level(),
            message: record.args().to_string(),
            target: record.target().to_string(),
        });
        if buf.len() > self.max_lines { buf.pop_front(); }

        // Delegation a env_logger pour l'affichage normal
        self.inner.log(record);
    }
}
```

### 5. Workflow Claude Code (CLI)

Claude Code peut travailler sur les issues auto-detectees :

```
Utilisateur : "Corrige les bugs auto-detectes"
Claude Code :
  1. gh issue list --repo tfoutrein/toile-engine --label auto-detected --state open
  2. Pour chaque issue, par severite :
     a. Lit le titre, description, composant, logs
     b. Localise le code concerne (grep dans le crate indique)
     c. Comprend et corrige le bug
     d. Commit avec "fix: <titre> (closes #N)"
  3. Push + PR si demande
```

### 6. Ajouts au system prompt du copilote

```
REPORT DE BUGS :
- report_bug : signaler un bug dans le MOTEUR Toile (pas le projet utilisateur)
- Utilise-le quand : un tool call echoue pour une raison interne, les logs montrent
  un crash/NaN/panic, un behavior ne fonctionne pas comme documente
- Ne l'utilise PAS quand : l'utilisateur a mal configure quelque chose (tag manquant,
  prefab pas cree, scene vide)
- Le bug sera cree comme GitHub Issue sur tfoutrein/toile-engine
```

## Phasage

### Phase 1 : report_bug tool + gh issue create (immediat)
- Ajouter `report_bug` aux tools du copilote
- Execution via `gh issue create` avec labels `auto-detected` + severite
- Deduplication basique (recherche par titre avant creation)
- Popup de consentement + option dans Settings
- Ajout au system prompt

### Phase 2 : Log Watcher + log collector (v0.5)
- Installer le log collector au demarrage de l'editeur
- Log watcher avec regles de detection moteur (crash, NaN, repeated warns)
- Les issues detectees passent par le meme Issue Creator
- Toast dans l'editeur quand une issue est creee

### Phase 3 : Panel Issues dans l'editeur (v0.5)
- Panneau UI listant les issues GitHub ouvertes (via `gh issue list`)
- Badge dans la toolbar ("3 bugs")
- Possibilite de fermer/commenter depuis l'editeur

### Phase 4 : Boucle complete (v1.0)
- Claude Code en agent schedule : verifie periodiquement les nouvelles issues et propose des PRs
- Le copilote IA peut aussi consulter les issues existantes ("est-ce un bug connu ?")
- Metriques : issues ouvertes/fermees, temps moyen de resolution, composants les plus bugges

## Options considerees

### Option A : Fichier JSON local `issues.toile.json` (rejetee)
- Les bugs moteur ne sont pas lies a un projet utilisateur specifique
- Un fichier local n'est pas visible par les autres utilisateurs ni par les mainteneurs
- Claude Code travaille sur le repo Toile, pas sur les projets de jeu

### Option B : GitHub Issues (retenue)
- Tracabilite complete (qui, quand, pourquoi)
- Visible par tous les utilisateurs et contributeurs
- Claude Code peut lire/fermer via `gh` CLI naturellement
- Labels et milestones pour organiser
- Les PRs peuvent reference `closes #N` pour fermer automatiquement
- Deja en place (le repo existe sur GitHub)

### Option C : GitHub Issues + fichier local de cache (hybride, possible en Phase 3)
- Le fichier local sert de cache pour eviter les appels API repetitifs
- Synchronisation periodique avec GitHub
- Complexite supplementaire pour un gain marginal en Phase 1

## Consequences

### Positives
- Les bugs moteur trouves "sur le terrain" remontent automatiquement au repo
- Plus de bugs "silencieux" que personne ne reporte
- Claude Code peut travailler de maniere autonome sur les corrections
- Les utilisateurs beneficient des corrections sans avoir a reporter eux-memes
- Historique complet des bugs dans GitHub (pas de perte)

### Negatives
- Necessite `gh` CLI installe et authentifie (friction au setup)
- Necessite une connexion internet pour creer les issues
- Risque de bruit si les regles de detection sont trop agressives → etre conservateur

### Risques
- **Faux positifs** : le copilote ou le log watcher pourrait confondre un probleme projet avec un bug moteur → le system prompt doit etre tres explicite sur la distinction, et les regles du log watcher ne matchent que des patterns clairement internes
- **Spam d'issues** : deduplication obligatoire (recherche avant creation) + rate limit (max 3 issues par session)
- **Vie privee** : les logs envoyes dans l'issue ne doivent pas contenir de donnees sensibles du projet utilisateur → anonymiser les noms d'entites/scenes dans le body
- **gh non installe** : fallback gracieux (log un warning, pas de crash) + message dans Settings pour guider l'installation
