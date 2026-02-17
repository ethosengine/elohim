# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository Overview

Polyglot monorepo for the Elohim Protocol - a distributed learning platform built on Holochain, Angular, and Rust. The main branch is `dev`.

## Build & Test Commands

### elohim-app (Angular 19 - main frontend)
```bash
cd elohim-app
npm install
npm start                          # Dev server at localhost:4200 (proxies to doorway at :8888)
npm run build                      # Production build
npm run lint                       # ESLint
npm run lint:fix                   # ESLint with auto-fix
npm run lint:css                   # Stylelint (SCSS/CSS)
npm run format:check               # Prettier check
npm test                           # Unit tests (Karma/Jasmine) with coverage
ng test --watch=false --browsers=ChromeHeadlessCI  # CI-mode tests
npx ng test --include='**/path.service.spec.ts'    # Single test file
npm run cypress:run                # E2E tests (Cucumber BDD)
npm run hc:start                   # Start Holochain + doorway + storage
npm run hc:start:seed              # Start with content seeding
```

### doorway (Rust gateway service)
```bash
cd doorway
RUSTFLAGS="" cargo build --release     # MUST override RUSTFLAGS (see gotchas)
RUSTFLAGS="" cargo test --lib --bins   # Unit tests
RUSTFLAGS="" cargo clippy -- -D warnings
cargo fmt --check
```

### doorway-app (Angular admin UI)
```bash
cd doorway-app
npm install
npm start                          # Dev server
npm run build
npx eslint src --ext .ts,.html
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
cd elohim-library
npm install
npx jest                           # Jest tests for elohim-service
```

### elohim-node (Rust P2P runtime)
```bash
cd elohim-node
RUSTFLAGS="" cargo build           # MUST override RUSTFLAGS
RUSTFLAGS="" cargo test
```

### holochain/elohim-storage (Rust storage service)
```bash
cd holochain/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
cargo test export_bindings         # Regenerate TypeScript types
```

### Pre-push hooks
The `.husky/pre-push` hook auto-detects changed projects and runs their quality gates. Bypass with `HUSKY=0 git push`.

## Architecture

### Domain Pillars (elohim-app/src/app/)

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
2. **`cargo test export_bindings`** generates TypeScript types to `holochain/sdk/storage-client-ts/src/generated/`
3. **storage-client-ts** (`@elohim/storage-client`) exports ready-to-use camelCase types
4. **Adapters** (`elohim-app/src/app/elohim/adapters/`) add computed/derived fields only - never transform wire format

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

## Critical Gotchas

### RUSTFLAGS Override Required
The system sets `RUSTFLAGS=--cfg getrandom_backend="custom"` for Holochain WASM builds. This breaks native Rust builds for doorway and elohim-node. Always use:
```bash
RUSTFLAGS="" cargo build   # For doorway, elohim-node
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build  # For elohim-storage
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

### libp2p 0.53 API (elohim-node)
Requires `macros` + `ed25519` features. Use `with_codec()` not `new()` for request-response. Swarm uses `StreamExt::next()` not `select_next_event()`.

## CI/CD

Central orchestrator pattern: only `orchestrator/Jenkinsfile` receives GitHub webhooks, analyzes changesets, and triggers downstream pipelines. Downstream jobs use `overrideIndexTriggers(false)` and validate `UpstreamCause` or `UserIdCause`. Pipeline definitions are in `orchestrator/Jenkinsfile`'s `PIPELINES` map.

| Pipeline | Jenkinsfile | Trigger |
|----------|-------------|---------|
| App | `Jenkinsfile` (root) | Auto via orchestrator |
| Edge | `holochain/Jenkinsfile` | Auto via orchestrator |
| DNA | `holochain/dna/Jenkinsfile` | Auto via orchestrator |
| Genesis | `genesis/Jenkinsfile` | Auto via orchestrator |
| Sophia | `sophia/Jenkinsfile` | Auto via orchestrator |
| Steward | `steward/Jenkinsfile` | Manual only |

## Code Style

### TypeScript/Angular
- ESLint 9 flat config with SonarQube parity rules
- Prettier: 100 char width, single quotes, trailing commas
- Import order: builtin -> external -> `@app/*` -> `@elohim/*`
- Strict TypeScript with Angular strict templates
- Path aliases defined in `elohim-app/tsconfig.json`

### Rust
- `cargo fmt` + clippy with `-D warnings`
- `clippy.toml` and `rustfmt.toml` in doorway/

### Sophia (React/TypeScript)
- pnpm workspace, Jest + @testing-library/react
- Packages prefixed `@ethosengine/*` (sophia) or `@khanacademy/*` (math utilities)
- psyche-core must NEVER depend on perseus packages
