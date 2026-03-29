# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository Overview

Polyglot monorepo for the Elohim Protocol - a distributed learning platform built on Holochain, Angular, and Rust. The main branch is `dev`.

## Build & Test Commands

### elohim-app (Angular 19 - main frontend)
```bash
pnpm install                       # From repo root (workspace install)
cd app/elohim-app
pnpm start                         # Dev server at localhost:4200 (proxies to doorway at :8888)
pnpm run build                     # Production build
pnpm run lint                      # ESLint
pnpm run lint:fix                  # ESLint with auto-fix
pnpm run lint:css                  # Stylelint (SCSS/CSS)
pnpm run format:check              # Prettier check
pnpm test                          # Unit tests (Vitest) with coverage
pnpm exec vitest run --config vite.config.ts                 # CI-mode tests
pnpm exec vitest run --config vite.config.ts "path.service"  # Single test file
pnpm run cypress:run               # E2E tests (Cucumber BDD)
pnpm run hc:start                  # Start Holochain + doorway + storage
pnpm run hc:start:seed             # Start with content seeding
```

### doorway (Rust gateway service)
```bash
cd doorway/doorway-service
RUSTFLAGS="" cargo build --release     # MUST override RUSTFLAGS (see gotchas)
RUSTFLAGS="" cargo test --lib --bins   # Unit tests
RUSTFLAGS="" cargo clippy -- -D warnings
cargo fmt --check
```

### doorway-app (Angular admin UI)
```bash
cd doorway/doorway-app
pnpm install                       # Or from repo root
pnpm start                         # Dev server
pnpm run build
pnpm exec eslint src --ext .ts,.html
```

### sophia (assessment engine - git submodule, pnpm)
```bash
cd sophia
pnpm install
pnpm build                         # Build all packages in dependency order
pnpm test                          # Jest tests
pnpm test -- --filter sophia-core  # Test specific package
pnpm lint
pnpm typecheck                     # Full type-check
```

### elohim-library (shared Angular libraries)
```bash
cd app/elohim-library
pnpm install                       # Or from repo root
cd projects/elohim-service && pnpm test   # Vitest tests for elohim-service
```

### steward/node (Rust P2P runtime)
```bash
cd steward/node
RUSTFLAGS="" cargo build           # MUST override RUSTFLAGS
RUSTFLAGS="" cargo test
```

### elohim/elohim-storage (Rust storage service)
```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
cargo test export_bindings         # Regenerate TypeScript types
```

### Protocol Schema Validation
```bash
pnpm run schema:test        # Schema self-tests (24 assertions)
pnpm run schema:validate    # Validate seed JSON against protocol schemas
pnpm run schema:check-dna   # Verify DNA constants match schema enums
pnpm run schema:codegen:ts  # Generate TypeScript from schemas (verification mode)
```

### Pre-push hooks
The `.husky/pre-push` hook auto-detects changed projects and runs their quality gates. Bypass with `HUSKY=0 git push`.

## Architecture

### Domain Pillars (app/elohim-app/src/app/)

The Angular app is organized into Hebrew-named domain pillars, each with its own services, models, and components:

| Pillar | Path Alias | Domain |
|--------|-----------|--------|
| **elohim** | `@app/elohim` | Protocol core - infrastructure, data loading, agents, trust |
| **imagodei** | `@app/imagodei` | Identity - auth, sessions, profiles, presence, relationships |
| **lamad** | `@app/lamad` | Learning - content, paths, assessments, mastery, practice |
| **qahal** | `@app/qahal` | Community - governance, affinity, consent |
| **shefa** | `@app/shefa` | Economy - stewardship, banking, resource flows |
| **doorway** | `@app/doorway` | Gateway integration |

Import via barrel exports: `import { ContentService } from '@app/lamad'`. The `elohim` pillar owns cross-pillar services.

### Data Flow: Rust-to-TypeScript Boundary

Types flow from Rust through auto-generation to TypeScript:

1. **elohim-storage** (`views.rs`) defines View types with `#[serde(rename_all = "camelCase")]` and `#[derive(TS)]`
2. **`cargo test export_bindings`** generates TypeScript types to `elohim/sdk/storage-client-ts/src/generated/`
3. **storage-client-ts** (`@elohim/storage-client`) exports ready-to-use camelCase types
4. **Adapters** (`app/elohim-app/src/app/elohim/adapters/`) add computed/derived fields only - never transform wire format

**Key rule**: snake_case never leaves the Rust boundary. TypeScript receives camelCase with parsed JSON and proper booleans. No `JSON.parse()`, no case conversion, no `toWire/fromWire` functions in TypeScript.

### Sophia Integration

Sophia (forked from Khan Academy Perseus) renders assessments in three modes: mastery (graded), discovery (psychometric), reflection (open-ended). It distributes as a web component `<sophia-question>` via `sophia-element` UMD bundle, wrapped for Angular by `sophia-plugin` in elohim-library.

Sophia is the **rendering layer only** - it produces Recognition callbacks. Session management, aggregation, and interpretation belong in the consuming app's services (lamad pillar).

### Doorway Gateway

Rust service consolidating three functions: bootstrap (agent discovery), signal (WebRTC), and gateway (conductor proxy + caching). Serves both hosted users (browser via doorway.elohim.host) and local dev (proxied via Angular dev server at localhost:8888).

### Content Pipeline

`genesis/` contains source content (markdown, Gherkin) and seeder tools. Content flows: genesis docs -> elohim-import CLI -> seed data JSON -> seeder -> elohim-storage -> doorway -> elohim-app.

### Deployment Contexts

The app runs in four modes with different content loading paths:
- **Eclipse Che**: Dev server proxy to doorway (avoids CORS)
- **Local dev**: Same proxy pattern
- **Production**: Browser direct to doorway.elohim.host
- **Tauri desktop**: Direct HTTP to local elohim-storage sidecar at :8090

## Development Workflow

### Story-First Default

Before implementing a feature, find or write the a2o scenario that describes the learner's experience. The scenario is your specification. Implementation is done when the scenario passes.

1. **Feel the vision** — read the epic/manifesto context in `genesis/docs/content/elohim-protocol/`
2. **Find or write the scenario** — check `genesis/a2o/features/` for existing coverage, or write a new `.feature` file
3. **Implement** to make the scenario pass
4. **Commit scenario + implementation together**

| Pillar | A2O Features Directory |
|--------|----------------------|
| lamad | `genesis/a2o/features/lamad/` |
| imagodei | `genesis/a2o/features/auth/` |
| qahal | `genesis/a2o/features/` (governance scenarios) |
| elohim | `genesis/a2o/features/content/`, `federation/` |

### Exploration Fallback

When story-first isn't practical (prototyping, spikes), capture implementation intent before committing by appending to `.claude/data/dev-intent.jsonl` — a 3-4 sentence summary of what was built, the learner impact, and which a2o feature file needs updating. Then run `/close-loop` to generate scenario updates from your intent.

### P2P Design Gate (MANDATORY)

Before proposing design approaches for ANY feature involving data entities (tables, models, routes, sync messages), invoke the `p2p-design-gate` skill. This gates brainstorming step 3 — no approaches may be proposed until the skill's decision tree is completed and the user has validated the entity classifications.

**This rule exists because** AI agents default to relational-DB patterns (UUID primary keys, REST-first design, CID-as-column). The protocol requires P2P-native thinking: DHT entry types first, content addressing for identity, storage as projection not truth.

**The skill forces you to answer (5 categories: A/A2/B/B2/C):**
1. Is this entity notarized (A), derived via link (A2), agent-scoped (B), agent-scoped with attestation (B2), or operational (C)?
2. Does a DHT entry type ALREADY EXIST? (Lamad DNA is at ~73/~100, Mishpat DNA is at 11/~100 — do NOT create new types without checking headroom.)
3. Is identity content-derived (CID), agent-composite, or slug (must justify)?
4. What coordinator function creates it? What signal projects it? (Answer BEFORE designing the HTTP route.)

**If you're about to write "Option A: `GET /api/v1/thing`" without having answered these questions, STOP and invoke the skill.**

## Schema & Manifest Sources of Truth

Two authoritative schemas govern content types and formats. All generated artifacts derive from these — never hand-edit generated files.

| Source of Truth | Path | Governs | Generates |
|----------------|------|---------|-----------|
| **Protocol Schema** | `elohim/sdk/schemas/v1/` | DNA-notarized enums (ContentFormat, ContentType, etc.) — the protocol's law | `schema-enums.ts` via `pnpm run schema:codegen:ts`, Rust constants via `pnpm run schema:codegen:rs` |
| **Lamad Manifest** | `elohim/sdk/domains/lamad/manifest.json` | App vocabulary: content types, content formats, renderer mappings, relationships, signals, coupling rules | `manifest-types.ts` via `pnpm run lamad:codegen` |

### Key distinction: core vs extensible formats

The protocol schema defines **broad core formats** (`markdown`, `html`, `interactive`, `video`, `audio`, `external`, `epr-composite`) that are DNA-notarized. The lamad manifest defines **specific extensible formats** (`sophia-quiz-json`, `html5-app`, `gherkin`, etc.) that map to Angular renderers.

**Seed data must use lamad manifest formats, not core protocol formats.** The renderer map only knows about formats declared in the lamad manifest. Using a core format like `interactive` (which has no renderer) causes content to fall through to the raw JSON fallback.

| Content | Correct `contentFormat` | Wrong |
|---------|------------------------|-------|
| Sophia quiz/assessment | `sophia-quiz-json` | `interactive` |
| HTML5 simulation | `html5-app` | `interactive` |
| Discovery assessment | `sophia-quiz-json` | `interactive` |

### Validation commands
```bash
pnpm run schema:validate         # Validate seed JSON against protocol schemas
pnpm run schema:check-dna        # Verify DNA constants match schema enums
pnpm run schema:codegen:ts       # Regenerate TS from protocol schemas (verification mode)
pnpm run lamad:codegen            # Regenerate manifest-types.ts from lamad manifest
```

## Critical Gotchas

### RUSTFLAGS Override Required
The system sets `RUSTFLAGS=--cfg getrandom_backend="custom"` for Holochain WASM builds. This breaks native Rust builds for doorway and steward/node. Always use:
```bash
RUSTFLAGS="" cargo build   # For doorway/doorway-service, steward/node
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build  # For elohim/elohim-storage
```

### Jenkinsfile Size Limit
`Jenkinsfile` (root, elohim-app pipeline) is ~1325 lines, near the 64KB JVM CPS method size limit. Helper methods are extracted to `// STAGE HELPER METHODS` section. Never add inline logic to stages.

### Jenkins params.MODE Null on First Build
MultiBranch pipeline params are null until the Jenkinsfile runs once. Always use `(params.MODE ?: 'auto')`.

### sophia-element UMD Must Be Pre-built
The sophia-element UMD bundle must be built before elohim-app builds. The `prebuild` script checks for it. Build with:
```bash
cd sophia && pnpm install && pnpm build && pnpm build:umd
```

### pnpm Workspace
All TypeScript/Node.js projects use pnpm workspaces. Run `pnpm install` from the repo root to install all dependencies. Sophia is excluded (git submodule with its own pnpm workspace). Use `pnpm --filter <package-name> <command>` to target specific packages. The `libsodium-wrappers` package is overridden to `^0.8.2` in root `package.json` to fix a broken ESM relative import in 0.7.x that fails with pnpm's strict module resolution.

### libp2p 0.53 API (steward/node)
Requires `macros` + `ed25519` features. Use `with_codec()` not `new()` for request-response. Swarm uses `StreamExt::next()` not `select_next_event()`.

## CI/CD

Central orchestrator pattern: only `genesis/orchestrator/Jenkinsfile` receives GitHub webhooks, analyzes changesets, and triggers downstream pipelines. Downstream jobs use `overrideIndexTriggers(false)` and validate `UpstreamCause` or `UserIdCause`. Pipeline definitions are in `genesis/orchestrator/Jenkinsfile`'s `PIPELINES` map.

| Pipeline | Jenkinsfile | Trigger |
|----------|-------------|---------|
| App | `Jenkinsfile` (root) | Auto via orchestrator |
| Edge | `elohim/holochain/Jenkinsfile` | Auto via orchestrator |
| DNA (Lamad) | `elohim/holochain/dna/Jenkinsfile` | Auto via orchestrator |
| DNA (Mishpat) | `elohim/holochain/dna/mishpat/Jenkinsfile` | Auto via orchestrator |
| Genesis | `genesis/Jenkinsfile` | Auto via orchestrator |
| Sophia | `sophia/Jenkinsfile` | Auto via orchestrator |
| Steward | `steward/Jenkinsfile` | Manual only |

## Code Style

### TypeScript/Angular
- ESLint 9 flat config with SonarQube parity rules
- Prettier: 100 char width, single quotes, trailing commas
- Import order: builtin -> external -> `@app/*` -> `@elohim/*`
- Strict TypeScript with Angular strict templates
- Path aliases defined in `app/elohim-app/tsconfig.json`

### Rust
- `cargo fmt` + clippy with `-D warnings`
- `clippy.toml` and `rustfmt.toml` in doorway/doorway-service/

### Sophia (React/TypeScript)
- pnpm workspace, Jest + @testing-library/react
- Packages prefixed `@ethosengine/*` (sophia) or `@khanacademy/*` (math utilities)
- psyche-core must NEVER depend on perseus packages
