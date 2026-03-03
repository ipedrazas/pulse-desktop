# Pulse Implementation Plan

**The Developer's Cockpit — Software Knowledge Management System**


## 0. Current State

- Tauri v2 scaffold (Vue 3 + Vite + TypeScript frontend, Rust backend)
- Plugins already wired: `fs`, `dialog`, `clipboard-manager`, `opener`
- No application logic yet — only the default `greet` command
- Build toolchain: `bun` (frontend), `cargo` (backend)

---

## 1. Architecture

```
┌──────────────────────────────────────────────────────┐
│                   Tauri WebView                      │
│   Vue 3 + TypeScript + Pinia (state) + TailwindCSS   │
│                                                      │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌─────────┐  │
│  │ Project  │ │  Run     │ │ Health   │ │ Context │  │
│  │ Registry │ │  Center  │ │ Dashboard│ │ Manager │  │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬────┘  │
│       │             │            │             │     │
│  ─────┴─────────────┴────────────┴─────────────┴──── │
│                   invoke() / events                  │
└──────────────────────┬───────────────────────────────┘
                       │ IPC
┌──────────────────────┴───────────────────────────────┐
│                   Rust Core                          │
│                                                      │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────┐ │
│  │ Project      │  │ Task Runner  │  │ Capabilities│ │
│  │ Registry     │  │ (Tokio)      │  │ Trait       │ │
│  │ + SQLite     │  │              │  │ (Node, Go)  │ │
│  └──────────────┘  └──────────────┘  └─────────────┘ │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────┐ │
│  │ File Watcher │  │ a2           │  │ Search      │ │
│  │ (notify)     │  │ Integration  │  │ Subsystem   │ │
│  └──────────────┘  └──────────────┘  └─────────────┘ │
│  ┌──────────────┐  ┌──────────────┐                  │
│  │ Process      │  │ Observability│                  │
│  │ Manager      │  │ Connectors   │                  │
│  └──────────────┘  └──────────────┘                  │
└──────────────────────────────────────────────────────┘
```

### Key Principles
- **Thin UI / Heavy Core**: all I/O, process management, and data access lives in Rust.
  The frontend is a rendering layer that calls Tauri commands and reacts to events.
- **Capability-based extensibility**: ecosystem support (Node, Go, Python, Rust) is modular
  behind a trait, not hardcoded in UI.
- **Local-first**: all data in SQLite. No accounts, no cloud, no telemetry.
- **Human-in-the-loop**: Pulse shows what it will do, the user confirms. Every execution is logged.

---

## 2. Configuration Files

### `.a2.yaml` — Policy (the "what")
Shared, committed to the repo. Defines compliance checks and expected structure.
Pulse reads it, runs `a2`, and visualises pass/fail results.

### `.pulse.yaml` — Workflow (the "how")
Repo-committed. Defines the developer experience: context files for AI agents, task macros,
local service definitions. This separation prevents Pulse from becoming "a2 with a GUI."

```yaml
version: 1

context:
  files:
    - README_AI.md
    - prompts/system.md
    - .a2.yaml
  copy_bundle:
    max_bytes: 200000
    include_git:
      branch: true
      sha: true
      dirty: true

services:
  - name: api
    cwd: .
    type: node
    dev:
      command: pnpm dev
      ports: [3000]
    health:
      url: http://localhost:3000/health
  - name: worker
    cwd: ./cmd/worker
    type: go
    dev:
      command: air
      ports: []

macros:
  - id: bootstrap
    title: Bootstrap
    steps:
      - run: pnpm install
        cwd: .
      - run: go mod download
        cwd: .
  - id: test
    title: Tests (fast)
    steps:
      - run: pnpm test
        cwd: .
      - run: go test ./...
        cwd: .
  - id: clean
    title: Clean artifacts
    steps:
      - run: rm -rf node_modules dist
        cwd: .
        confirm: true

watchers:                          # Background Sandbox config
  - id: fast-test
    title: Fast tests on save
    enabled: false                 # opt-in
    glob: "src/**/*.{ts,tsx,js}"
    debounce_ms: 1500
    macro: test
    concurrency: 1                 # cancel previous run on new trigger

connectors:                        # Observability quick-links
  - id: k8s-logs
    title: Kubernetes logs
    command: kubectl logs -f deploy/{{service}} -n {{namespace}}
    variables:
      service: api
      namespace: default
  - id: vercel-logs
    title: Vercel logs
    url: https://vercel.com/{{team}}/{{project}}/logs
    command: vercel logs --follow
```

---

## 3. Data Model — SQLite

Use `sqlx` with compile-time checked queries and migrations.

```sql
-- Project registry
CREATE TABLE projects (
    id          TEXT PRIMARY KEY,   -- hash(root_path + remote_url)
    name        TEXT NOT NULL,
    root_path   TEXT NOT NULL UNIQUE,
    remote_url  TEXT,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    last_opened TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Known context / config files per project
CREATE TABLE project_files (
    project_id  TEXT NOT NULL REFERENCES projects(id),
    path        TEXT NOT NULL,
    kind        TEXT NOT NULL,      -- 'pulse_yaml', 'a2_yaml', 'readme_ai', 'system_prompt'
    last_hash   TEXT,
    updated_at  TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (project_id, path)
);

-- Every execution Pulse performs
CREATE TABLE runs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id  TEXT NOT NULL REFERENCES projects(id),
    kind        TEXT NOT NULL,      -- 'a2', 'macro', 'test', 'audit', 'build', 'watcher'
    macro_id    TEXT,               -- nullable, links to .pulse.yaml macro id
    status      TEXT NOT NULL DEFAULT 'running',  -- 'running', 'success', 'failure', 'cancelled'
    command     TEXT NOT NULL,
    cwd         TEXT NOT NULL,
    env_keys    TEXT,               -- JSON array of env key names (never values)
    exit_code   INTEGER,
    started_at  TEXT NOT NULL DEFAULT (datetime('now')),
    finished_at TEXT,
    pid         INTEGER
);

-- Chunked log storage (supports streaming to UI)
CREATE TABLE run_logs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id      INTEGER NOT NULL REFERENCES runs(id),
    stream      TEXT NOT NULL DEFAULT 'stdout',  -- 'stdout' | 'stderr'
    chunk       TEXT NOT NULL,
    ts          TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Snapshots: dependency audits, API maps, port scans, etc.
CREATE TABLE snapshots (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id  TEXT NOT NULL REFERENCES projects(id),
    kind        TEXT NOT NULL,      -- 'deps_node', 'deps_go', 'api_map', 'env_parity', 'ports'
    commit_sha  TEXT,
    payload     TEXT NOT NULL,      -- JSON blob
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Agent session tracking (worklog)
CREATE TABLE agent_sessions (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id  TEXT NOT NULL REFERENCES projects(id),
    title       TEXT,
    tool        TEXT,               -- 'claude', 'cursor', 'copilot', etc.
    task_summary TEXT,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Managed processes (port tracking, service lifecycle)
CREATE TABLE processes (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id  TEXT NOT NULL REFERENCES projects(id),
    service_name TEXT NOT NULL,
    pid         INTEGER NOT NULL,
    command     TEXT NOT NULL,
    ports       TEXT,               -- JSON array of declared ports
    started_at  TEXT NOT NULL DEFAULT (datetime('now')),
    stopped_at  TEXT
);
```

---

## 4. Rust Core — Module Layout

```
src-tauri/src/
├── main.rs
├── lib.rs                  # Tauri builder, plugin registration
├── db/
│   ├── mod.rs              # Pool init, migrations
│   └── migrations/         # sqlx migrations
├── commands/               # Tauri #[command] handlers
│   ├── projects.rs         # CRUD, open folder, recent list
│   ├── runs.rs             # Execute macro, stream logs, cancel
│   ├── a2.rs               # Run a2, parse JSON output
│   ├── context.rs          # Build + copy context bundle
│   ├── health.rs           # Dependency snapshots, env parity
│   ├── search.rs           # Text search (ripgrep), later semantic
│   ├── processes.rs        # Start/stop/list managed services, port tracking
│   └── connectors.rs       # Observability connector execution
├── capabilities/
│   ├── mod.rs              # Capability trait definition
│   ├── node.rs             # Node/TS: detect, outdated, audit, scripts
│   ├── go.rs               # Go: detect, list -m -u, govulncheck, targets
│   └── detect.rs           # Heuristic project-type detection
├── runner/
│   ├── mod.rs              # Tokio-based task runner
│   ├── process.rs          # Spawn, stream stdout/stderr, capture PID
│   └── queue.rs            # Per-project job queue with cancellation
├── watcher/
│   ├── mod.rs              # notify-based file watcher
│   └── debounce.rs         # Debounced trigger → macro execution
├── search/
│   ├── mod.rs
│   ├── text.rs             # ripgrep wrapper (ignore-aware fast search)
│   └── semantic.rs         # (Phase 4) embeddings + sqlite-vss
└── models.rs               # Shared Rust structs (serde)
```

### Capability Trait

```rust
pub trait Capability: Send + Sync {
    fn name(&self) -> &str;
    fn detect(&self, root: &Path) -> f32;               // 0.0–1.0 confidence
    fn dependency_snapshot(&self, root: &Path) -> Result<serde_json::Value>;
    fn quick_actions(&self, root: &Path) -> Vec<MacroDefinition>;
    fn health_checks(&self, root: &Path) -> Vec<HealthCheck>;  // optional
    fn endpoint_map(&self, root: &Path) -> Option<Vec<Endpoint>>;  // optional, Phase 3
}
```

---

## 5. Frontend — Vue 3 Structure

```
src/
├── main.ts
├── App.vue
├── router/
│   └── index.ts            # vue-router: sidebar navigation
├── stores/                 # Pinia state
│   ├── projects.ts
│   ├── runs.ts
│   └── processes.ts
├── composables/
│   ├── useTauriCommand.ts  # typed invoke() wrapper
│   └── useTauriEvent.ts   # event listener lifecycle management
├── layouts/
│   └── MainLayout.vue      # Sidebar + content area + status bar
├── views/
│   ├── ProjectList.vue     # Registry / recent projects
│   ├── ProjectDashboard.vue # Per-project overview (health, recent runs)
│   ├── RunCenter.vue       # Macro runner, log streaming, run history
│   ├── ContextManager.vue  # Context bundle viewer, copy-for-agent
│   ├── HealthDashboard.vue # Dependencies, env parity, ports
│   ├── FileBrowser.vue     # Read-only tree + code viewer
│   ├── SearchView.vue      # Cross-project text search
│   └── Settings.vue
├── components/
│   ├── Sidebar.vue
│   ├── StatusBar.vue       # Git branch, dirty state, active processes
│   ├── LogStream.vue       # Real-time log output (xterm.js or plain)
│   ├── RunCard.vue         # Single run: status, duration, exit code
│   ├── ConfirmGate.vue     # Approval dialog for dangerous commands
│   ├── DependencyTable.vue
│   ├── EnvParityDiff.vue
│   ├── PortList.vue
│   └── ServiceCard.vue     # Running service: name, ports, health, stop button
└── styles/
    └── tailwind.css
```

### UI Framework Choices
- **TailwindCSS**: utility-first, fast to iterate, no component library lock-in
- **xterm.js** (or a simpler `<pre>` with ANSI support): for log streaming
- **Monaco** (optional, Phase 3+): read-only code viewer in file browser. Start with
  syntax-highlighted `<pre>` blocks using Shiki for MVP.
- **vue-router**: sidebar-driven navigation between views
- **Pinia**: reactive state management, synced with Tauri event stream

---

## 6. Feature Breakdown

### 6.1 Project Registry
- Open folder via native dialog → detect project type → store in SQLite
- Recent projects list, sorted by `last_opened`
- Project identity: `sha256(root_path + remote_url)` for stable IDs
- Auto-detect `.pulse.yaml` and `.a2.yaml` on open
- Git metadata: current branch, dirty state, last commit (shown in status bar)

### 6.2 a2 Policy Engine
- Parse `.a2.yaml` and display rules
- "Run a2" button → execute `a2 -f json` → parse structured output
- Visualise pass/fail per check with file:line links
- Store results in `runs` + `run_logs`
- Quick-fix: link a2 failures to macros that can resolve them

### 6.3 Macro Runner (Run Center)
- Parse `macros` from `.pulse.yaml`
- **Plan view**: show steps, cwd, and env overlays before execution
- **Confirm gates**: steps with `confirm: true` or dangerous tokens require approval
- Execute via Tokio task runner, stream stdout/stderr to UI in real time
- Store every run in `runs` table with exit code, duration, and logs
- Cancel running macros (send SIGTERM to process group)
- Run history: filterable list of past runs per project

### 6.4 Context Bundle Manager
- Read `context.files` from `.pulse.yaml`
- Display each file's content with syntax highlighting
- "Copy Bundle" action: concatenate selected files + git metadata into clipboard
- Include: file contents, branch, SHA, dirty state, active task summary
- Configurable `max_bytes` to prevent accidental mega-pastes

### 6.5 Dependency & Health Dashboard
- **Node capability**: `pnpm outdated --json`, `pnpm audit --json`
- **Go capability**: `go list -m -u -json all`, `govulncheck ./...`
- Display as tables: package, current, latest, severity
- Store results as `snapshots` → show trends ("3 new vulnerabilities since last week")
- Refresh on demand or on project open

### 6.6 Environment Parity Check
- Diff `.env.example` keys vs `.env` keys
- Show: missing keys, extra keys, empty-value keys
- **Never display secret values** — only presence/absence
- Store as snapshot for historical tracking

### 6.7 Port & Service Management
- Parse `services` from `.pulse.yaml`
- Start/stop services from the UI (spawns process, tracks PID in `processes` table)
- Show declared ports and their status (listening / free)
- Health checks: poll `health.url` when service is running
- Port conflict detection: probe declared ports before starting
- Scoped to processes Pulse started — no global port scanning

### 6.8 File Browser
- Rust-side: enumerate project tree, respect `.gitignore`, cache results
- UI: virtualised tree (only render visible nodes)
- Read-only code viewer with syntax highlighting (Shiki for MVP)
- Click file:line references from a2/test output to navigate directly

### 6.9 Background Sandbox (Watchers)
*This was detailed in the draft but reduced to a bullet in the final-draft. Restoring it here.*

- Defined in `.pulse.yaml` under `watchers`
- **File watching**: Rust `notify` crate, debounced (configurable, default 1500ms)
- **Trigger**: glob pattern match → execute linked macro
- **Concurrency**: per-project limit (default 1), new trigger cancels previous run
- **Scope**: only "light" tasks by default (lint, unit tests). Builds require explicit opt-in.
- **UI**: status indicator per watcher (idle / running / pass / fail), last result, log access
- **Resource guard**: watchers auto-pause if CPU load exceeds threshold or if the project
  is not the active one (configurable)
- All watcher-triggered runs stored in `runs` with `kind = 'watcher'`

### 6.10 Observability Connectors
*This was a full section in the draft but reduced to a single bullet in the final-draft.
Restoring it as a first-class feature.*

- Defined in `.pulse.yaml` under `connectors`
- Each connector has: `id`, `title`, `command` (CLI), optional `url` (dashboard link)
- Template variables: `{{service}}`, `{{namespace}}`, `{{team}}`, etc.
- **MVP (Phase 2)**: "Open in browser" + "Copy command to clipboard"
- **Phase 3**: embedded log pane — execute the CLI command and stream output into
  the same `LogStream` component used by the Run Center
- Built-in connector templates for common providers:
  - Kubernetes: `kubectl logs`, `kubectl get pods`
  - Docker: `docker compose logs -f`
  - Vercel: `vercel logs`
  - AWS: `aws logs tail`
- Connectors require explicit user opt-in per project (no accidental prod access)

### 6.11 Search
- **Phase 1**: text search using ripgrep (`rg`) across the active project.
  Respects `.gitignore`. Results link to file browser.
- **Phase 2**: cross-project search (search all registered projects)
- **Phase 4**: semantic search using local embeddings
  - Embeddings: local model (e.g., `all-MiniLM-L6-v2` via `ort`/ONNX Runtime in Rust)
  - Vector storage: `sqlite-vss` extension
  - Chunking: by function/class (tree-sitter), keep file path + language + commit SHA metadata
  - Index on project open, update incrementally on file change

### 6.12 Worklog (Unified Session Tracker)
*From draft section 8: unifies context + sessions + tasks into one concept.*

- Per-project "current task": title, notes, links (commits, PRs, files)
- Last agent prompt and outcome summary
- Recent runs attached to the current task
- Persisted in `agent_sessions` table
- Survives app restarts — pick up where you left off

### 6.13 API Map (Automated Documentation)
- **Phase 3**: heuristic-based endpoint discovery
- Start with one framework: Express/Nest (Node capability) or net/http + chi/gin (Go capability)
- Fast path: ripgrep-based pattern matching for route definitions
- Better path: tree-sitter AST parsing for supported languages
- Display as table: method, path, handler, file:line
- Store as snapshot → diff between versions
- Mermaid diagram generation for folder structure and DB schema
  (integrate with migration tools like Prisma, Drizzle, Alembic rather than inferring)

---

## 7. Security Model

### Command Execution
- **Allowlist**: per project type, only known-safe binaries (npm, pnpm, go, docker, kubectl, etc.)
- **Dangerous token detection**: commands containing `rm -rf`, `curl | sh`, `> /dev/`,
  pipe to `sh`/`bash`, `--force`, `DROP TABLE`, etc. require explicit confirmation
- **Execution logging**: every command stored in `runs` with cwd, env key names, exit code

### Secrets
- `.env` values are **never stored** in the database. Store only paths + key names + checksums.
- Credentials use OS-native keychain (macOS Keychain via `security-framework` crate)
- Environment parity checks show key presence only, never values

### Process Isolation
- Spawned processes run as the current user (no privilege escalation)
- Each process tracked by PID in `processes` table
- Cleanup on app exit: signal managed processes with SIGTERM
- No shell injection: commands are parsed and executed with explicit arg arrays, not `sh -c`

---

## 8. Phased Roadmap

### Phase 1 — Foundation (MVP)

The minimum viable cockpit. A developer can register projects, run policy checks and
macros, and copy context for AI agents.

| # | Feature | Rust | Frontend |
|---|---------|------|----------|
| 1 | Project setup: SQLite + migrations + Tauri state | `db/`, `models.rs` | — |
| 2 | Project registry: open folder, detect type, recent list | `commands/projects.rs`, `capabilities/detect.rs` | `ProjectList.vue` |
| 3 | `.pulse.yaml` + `.a2.yaml` parsing | `commands/projects.rs` | `ProjectDashboard.vue` |
| 4 | a2 integration: run + parse JSON output + visualise | `commands/a2.rs` | `RunCenter.vue` (results panel) |
| 5 | Macro runner: execute steps, stream logs, confirm gates | `commands/runs.rs`, `runner/` | `RunCenter.vue`, `LogStream.vue`, `ConfirmGate.vue` |
| 6 | Run history: list past runs, view logs | `commands/runs.rs` | `RunCenter.vue` (history tab) |
| 7 | Context bundle: view files + copy to clipboard | `commands/context.rs` | `ContextManager.vue` |
| 8 | Status bar: git branch, dirty state, last commit | `commands/projects.rs` | `StatusBar.vue` |
| 9 | App shell: sidebar navigation, main layout | — | `MainLayout.vue`, `Sidebar.vue`, `router/` |

**Rust dependencies to add**: `sqlx` (SQLite), `serde_yaml`, `tokio` (async runtime),
`notify` (file watcher — wire foundation but don't expose watchers yet),
`sha2` (project identity hashing)

**Frontend dependencies to add**: `vue-router`, `pinia`, `tailwindcss`,
`@tauri-apps/plugin-shell` (command execution)

### Phase 2 — Visibility & Health

The cockpit gains instruments. The developer can see the health of their project
at a glance without leaving Pulse.

| # | Feature | Rust | Frontend |
|---|---------|------|----------|
| 1 | Node capability: outdated + audit snapshots | `capabilities/node.rs` | `DependencyTable.vue` |
| 2 | Go capability: outdated + govulncheck | `capabilities/go.rs` | `DependencyTable.vue` |
| 3 | Health dashboard: per-project summary cards | `commands/health.rs` | `HealthDashboard.vue` |
| 4 | Environment parity check | `commands/health.rs` | `EnvParityDiff.vue` |
| 5 | Service management: start/stop, PID tracking, port status | `commands/processes.rs`, `runner/process.rs` | `ServiceCard.vue`, `PortList.vue` |
| 6 | Observability connectors (MVP): open URL + copy command | `commands/connectors.rs` | Connector buttons in dashboard |
| 7 | Worklog: current task + last agent session per project | `commands/projects.rs` | `ProjectDashboard.vue` (worklog section) |
| 8 | Text search (single project) | `commands/search.rs`, `search/text.rs` | `SearchView.vue` |
| 9 | Run history trends: success rate, duration over time | — | `ProjectDashboard.vue` (chart) |

**Rust dependencies to add**: `which` (find binaries on PATH),
`tauri-plugin-shell` (v2 shell command support)

### Phase 3 — Automation & Intelligence

The cockpit starts working proactively. Background watchers catch issues before you do.
Observability data streams into Pulse. Code navigation improves.

| # | Feature | Rust | Frontend |
|---|---------|------|----------|
| 1 | Background watchers: file watch + debounced macro execution | `watcher/` | Watcher status indicators |
| 2 | Watcher configuration UI: enable/disable per watcher | — | `Settings.vue` or per-project config |
| 3 | Observability connectors (v2): embedded log streaming | `commands/connectors.rs` | `LogStream.vue` (reuse) |
| 4 | File browser: tree + read-only code viewer | `commands/projects.rs` (enumerate) | `FileBrowser.vue` |
| 5 | API map (one framework): endpoint discovery + display | `capabilities/node.rs` or `go.rs` | API map table |
| 6 | Cross-project search | `commands/search.rs` | `SearchView.vue` (scope selector) |
| 7 | a2 quick-fix: link failures to macros | `commands/a2.rs` | Fix button in results |
| 8 | Snapshot diffing: compare dependency/API snapshots over time | — | Diff viewer component |

**Rust dependencies to add**: `tree-sitter` + language grammars (for API map AST parsing)

### Phase 4 — Deep Intelligence

| # | Feature |
|---|---------|
| 1 | Semantic search: local embeddings + sqlite-vss |
| 2 | Mermaid diagram generation (folder structure, DB schema) |
| 3 | Additional capabilities: Python, Rust, Java |
| 4 | Multi-workspace support (groups of projects) |
| 5 | Plugin system: user-defined capabilities loaded at runtime |

---

## 9. Dependency Inventory

### Rust (Cargo.toml additions)

```toml
# Phase 1
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }
tokio = { version = "1", features = ["full"] }
serde_yaml = "0.9"
sha2 = "0.10"
notify = { version = "7", features = ["macos_fsevent"] }
tauri-plugin-shell = "2"

# Phase 2
which = "7"

# Phase 3
tree-sitter = "0.24"
tree-sitter-javascript = "0.23"
tree-sitter-typescript = "0.23"
tree-sitter-go = "0.23"
```

### Frontend (package.json additions)

```json
{
  "dependencies": {
    "vue-router": "^4",
    "pinia": "^3",
    "@tauri-apps/plugin-shell": "~2"
  },
  "devDependencies": {
    "tailwindcss": "^4",
    "@tailwindcss/vite": "^4"
  }
}
```

---

## 10. Key Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| State management | Pinia | Official Vue store, TypeScript-first, simple API |
| CSS framework | Tailwind v4 | Utility-first, fast iteration, no component library dependency |
| Database | SQLite via sqlx | Local-first, compile-time checked queries, migrations built-in |
| Async runtime | Tokio | Required by sqlx, natural fit for process spawning and file watching |
| Config format | YAML (.pulse.yaml) | Human-readable, already used by a2, familiar to the target audience |
| Log streaming | Tauri events | Backend emits `run:log` events, frontend appends to log buffer reactively |
| Code viewer | Shiki (MVP) → Monaco (later) | Shiki is lightweight SSR-friendly highlighter; Monaco is heavy but powerful |
| Process management | PID tracking + SIGTERM | Simple, cross-platform (with minor OS differences), no container overhead |
| Project identity | SHA-256 of path+remote | Stable across renames, survives remote URL changes gracefully |
| Command execution | Explicit arg arrays | Prevents shell injection; commands are never passed through `sh -c` |

---

## 11. What's New vs. Final-Draft

Features restored or added that were lost between draft → final-draft:

1. **Background Sandbox / Watchers** (6.9): full file-watching system with debounce,
   cancellation, concurrency limits, and resource guards. Was reduced to a single bullet.

2. **Observability Connectors** (6.10): first-class connector system with templates for
   k8s, Docker, Vercel, AWS. Two-phase delivery: links first, embedded streaming later.
   Was dropped entirely from the final-draft.

3. **Port & Service Management** (6.7): process lifecycle management, PID tracking,
   health checks, port conflict detection. Was implicit in the final-draft but not specified.

4. **API Map tooling** (6.13): ripgrep heuristics → tree-sitter AST parsing pipeline
   with framework-specific support. Was a single bullet in Phase 3.

5. **Semantic Search depth** (6.11): local embeddings, sqlite-vss, chunking strategy
   by function/class, incremental indexing. Was mentioned but without implementation detail.

6. **Worklog** (6.12): unified concept from draft section 8, merging context + sessions +
   tasks into a persistent per-project cockpit state. Was partially covered as "Worklog System"
   but without the unifying design.

7. **Snapshot diffing**: compare health/dependency/API snapshots across time. Not in
   the final-draft at all.

---

## 12. Open Questions

1. **Vue component library**: use headless components (Radix Vue / Headless UI) or build
   from scratch with Tailwind? Headless libraries speed up accessible UI development.

2. **Terminal emulation**: xterm.js (full terminal) vs. plain `<pre>` with ANSI parsing
   for log streaming? xterm.js is heavier but handles interactive output better.

3. **Tauri v2 shell plugin**: the `tauri-plugin-shell` v2 plugin has a scoped command
   allowlist. This pairs well with the security model but needs careful configuration
   to avoid being too restrictive for macro execution.

4. **a2 binary distribution**: should Pulse bundle `a2` or expect it on PATH? Bundling
   gives a better out-of-box experience; PATH dependency is simpler.
