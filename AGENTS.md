# AGENTS.md — Codex Governance Contract (Elohim Protocol)

This is Codex's root gospel. The gospel body below the horizontal rule is the
SAME text as `CLAUDE.md`, projected verbatim from the `elohim-root-gospel`
package — the two cannot drift. THIS preamble is the codex-localized governance
contract: what Claude Code already receives from edit-time hooks but this repo
has not yet projected into Codex's current project-hook ABI. Read it first.
Until `epr doctor` reports the Codex hook projection present and the runtime
trusts it, this binds you *morally* where it cannot bind you *mechanically*, and
it names the gates that bind you mechanically regardless of harness.

## 1. First contact: inspect before changing

From a fresh clone, activate the checked-in reach gate and inspect the native
governance floor before substantive work:

`env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-onboarding-target cargo run --manifest-path elohim/eprfs/Cargo.toml -q -p elohim-epr-cli -- setup`

After that first build, use `epr doctor` (or the same `cargo run … -- doctor`)
to inspect harness projections and hook activation, `epr explain <path>` before
an unfamiliar edit, and `epr check` for local advice. Local checks never take
authority over the working tree. `epr ready [--target <ref>]` is the explicit
reach rehearsal: it evaluates committed changes against their merge base and
fails only findings that would prevent the requested push/merge boundary.

## 2. Editing agentic surfaces is package-first

The skills, subagents, hooks, and agent-docs (`.claude/*`, `.codex/*`,
`CLAUDE.md`, `AGENTS.md`) are PROJECTIONS of Elohim-native packages under
`.epr-meta/elohim/packages/`. Two disciplines coexist; the marker decides which:

- **A package marked `metadata.master: "package"` (planted) is authoritative.**
  Edit its package JSON, then regenerate ONLY that surface and re-verify:
  `node elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs project --write-fixtures --write-runtime --only <Kind:id>`
  then `… verify`. NEVER hand-edit the projected `.claude`/`.codex` file — the
  next projection clobbers it and `verify` reds on the drift.
- **A Claude-sourced package that is NOT planted (no `master`) is source-first.**
  Edit its `.claude/*` source directly, then re-import:
  `… project --write-fixtures` (or `elohim-agent:packages:write`). The package is
  a certified mirror; the fidelity gate proves `project(import(source)) === source`.

If unsure which a surface is, read its package's `metadata.master`. When in
doubt, do not edit the projection — edit the package or the source.

## 3. The `.epr-meta` compose-gate binds you morally

Claude enforces directory-local governance at edit time via a PreToolUse hook.
Current Codex supports project-local lifecycle hooks, but this repo has not yet
projected the shared definitions to `.codex/hooks.json`; until that projection
is present and trusted, the SAME rules bind you by contract instead of by
mechanism. Before writing a file, honor the nearest `.epr-meta` ancestor
(the cascade: closest manifest wins, root `/.epr-meta/manifest.md` is the base):
a `specs/` doc requires lifecycle frontmatter; `elohim/` carries an
interface-first-reuse advisory (reuse the existing trait/type before adding a
parallel one); code directories may carry YAGNI/over-engineering guards. A
manifest that would DENY a Claude edit should stop you too — you just won't get
a prompt. Treat a missing prompt as your own responsibility, not permission.

## 4. Gates that bind Codex mechanically (harness-independent)

These run outside any harness and WILL fail your push or CI:

- **`.husky/pre-push`** — project-detected quality gates (lint / format:check /
  typecheck / tests), plus `sweettest-check` when the push targets `dev`/`main`.
  Do not bypass with `--no-verify` unless the gates already ran green.
- **`pnpm run elohim-agent:packages:verify`** — projection/governance drift. If
  you touched any agentic surface or its package, this must be GREEN before commit.
- **cite-gen** owns `cites:` frontmatter. NEVER hand-write a slug, path, or
  `sha256:` fingerprint — run `python3 .claude/scripts/memory-kit/cite-gen.py`.
  A hand-edited fingerprint fails the cite gate.

## 5. Rails (non-negotiable)

- **Never `git add -A` / `git commit -a`** — the worktree is shared across
  concurrent sessions. Stage path-scoped: `git add <explicit paths>`.
- **Never push without green gates**, and never amend or force-push shared history.
- **Native (non-WASM) cargo needs `CARGO_TARGET_DIR`** at the pool slot per the
  disk policy; WASM/DNA workspaces stay plain cargo.
- **`RUSTFLAGS` gotcha:** the environment sets
  `RUSTFLAGS=--cfg getrandom_backend="custom"` for Holochain WASM. Native builds
  (doorway, steward/node) need `RUSTFLAGS=""`; `elohim/elohim-storage` keeps the
  custom backend flag.

---

# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository Overview

Polyglot monorepo for the Elohim Protocol - a distributed learning platform built on Holochain, Angular, and Rust. `dev` is the integration target (not a protected default): feature branches land on `dev` via local fast-forward merge — no PR. Release-grade PR review happens at `dev → main`.

## Build & Test Commands

Run `pnpm install` from repo root once (workspace install). Sophia is a git submodule with its own pnpm workspace.

### elohim-app (Angular 19, `app/elohim-app/`)
```bash
pnpm start                         # Dev server :4200 (proxies doorway :8888)
pnpm run build                     # Production build
pnpm run lint | lint:fix | lint:css | format:check
pnpm test                          # Vitest with coverage
pnpm exec vitest run --config vite.config.ts [pattern]   # CI / single-file
pnpm run cypress:run               # E2E (Cucumber BDD)
pnpm run hc:start[:seed]           # Start Holochain + doorway + storage [with seeding]
```

### Rust services (RUSTFLAGS override required — see Gotchas)
```bash
# doorway (doorway/doorway-service/) + steward/node — native builds
RUSTFLAGS="" cargo build --release && cargo test --lib --bins && cargo clippy -- -D warnings && cargo fmt --check

# elohim/elohim-storage/ — Holochain WASM build
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
cargo test export_bindings         # Regenerate TypeScript types
```
Prefer `cargo nextest run` over `cargo test` for unit/integration runs (installed at `/opt/rust/cargo/bin/cargo-nextest`; parallel, faster on warm caches; same `--lib`/`--test <name>`/filter syntax). It does NOT replace `cargo test export_bindings` (separate ts-rs harness) and skips `#[ignore]` by default like `cargo test`.

### Other Angular surfaces
- `doorway/doorway-app/`: `pnpm start | build`; lint via `pnpm exec eslint src --ext .ts,.html`
- `app/elohim-library/projects/elohim-service/`: `pnpm test` (Vitest)

### sophia (submodule)
```bash
cd sophia && pnpm install && pnpm build && pnpm test [-- --filter sophia-core] && pnpm lint && pnpm typecheck
```

### Schema validation & pre-push
```bash
pnpm run schema:{test,validate,check-dna,codegen:ts}
```
The `.husky/pre-push` hook auto-detects changed projects and runs their quality gates. Bypass with `git push --no-verify` (NOT `HUSKY=0` — `core.hooksPath=.husky` makes git invoke the hook directly, bypassing the npm wrapper that honors the `HUSKY` env var, so `HUSKY=0` is a no-op here; this doc-vs-behavior drift has cost shift time 4×).

**Gates are tiered and PVC-pressure-aware (2026-06-04).** `sweettest-check` is an integration-layer gate: it runs by default only when the push targets `dev`/`main` (force elsewhere: `RUN_SWEETTEST=1`); CI's DNA pipeline is the backstop. Native Rust gates build into per-crate cargo-target-pool slots. Under disk pressure the hook reads `genesis/agentic/pool-policy.json`: at the soft watermark it reclaims first via `cargo-pool enforce --yes` (guarded — never touches the active family or flock'd slots); at the hard ceiling it defers heavy Rust gates with a `DEFERRED-BY-PVC` banner (`FORCE_HEAVY_GATES=1` overrides). The same policy backs a PreToolUse hook that DENIES heavy cargo commands at the hard ceiling and denies native-workspace cargo lacking `CARGO_TARGET_DIR` (DNA/WASM workspaces exempt — they must stay plain cargo).

The hook has TWO project-detection paths that emit DIFFERENT names: manifest-driven (`graph-walker.mjs`, fine-grained names like `epr-ts`) and a grep fallback (coarser names like `elohim-epr`). When adding a sub-project to any `build-manifest.json` `gate.projects` map, also add a matching case to `run_gate`'s fallback `case` statement — a missing case hits the `*) Unknown project` default and aborts the whole push.

## Architecture

### Seam Map — concern routing (read FIRST when "where does this live?")

Before reasoning about *where* a concern belongs — across hardware · OS/packaging · runtime/footprint · mods/plugins · SDK · bridges · clients · app-manifests · the role seams (doorway / peer-hoster / aggregation / hub-cluster) — consult the **concern-routing atlas**: `genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md`. It maps the full device spectrum (**smartwatch → home storage rack**) × the composition stack, with a concern-routing table that makes a problem *self-locate*. The dominant failure mode is **misrouting** (a substrate-identity bug wearing an aggregation costume; a packaging fact read as a dataplane fact; a UI-render gradient confused with hardware tiering).

The disambiguator you'll reach for most — **what do you ADD?** a *manifest* → **SDK seam** (compose inward, integrity by construction); a *crate* → **bridge seam** (translate an external protocol outward); *native code* → **mod/plugin seam** (extend the runtime downward). Orthogonal to those seams are the four **participation tracks** — *how a running node participates*: T1 DHT-notary floor · T2 substrate (libp2p/iroh) · T3 spoke (HTTP/WS) · T4 doorway projection. Seams are where you add capability; tracks are how a thing participates — they meet but never collapse. The atlas also carries a hyperscaler-parity crosswalk and names the **inversion** (the social/governance/trust/recovery plane has no hyperscaler equivalent — human-scale's ceiling, not its catch-up).

**Operating the live substrate — read the trust contract FIRST when a dataplane probe reds:** `genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md` — the invariants the dataplane holds (verify-locally-then-serve; canonical channels alone move declared heads; heal fills-never-moves; restart churn ≈20min; fresh actions need publish time), the probe watching each one (per-seam smoke in Dataplane Validation, `GET /db/p2p/conductor-diagnostics`, `POST /admin/steward-peers/refresh`, the per-deploy `✓ canonical head propagated` line), and the per-red decision tree. It exists because scenario-2 divergence was five stacked invisible defects; the probes now name them. When the doc and live behavior disagree, the probes are the authority.

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

1. **elohim-storage** (`elohim/elohim-storage/src/views.rs`) defines View types with `#[serde(rename_all = "camelCase")]` and `#[derive(TS)]`
2. **`cargo test export_bindings`** generates TypeScript types to `elohim/sdk/storage-client-ts/src/generated/`
3. **storage-client-ts** (`@elohim/storage-client`) exports ready-to-use camelCase types
4. **Adapters** (`app/elohim-app/src/app/elohim/adapters/`) add computed/derived fields only - never transform wire format

**Key rule**: snake_case never leaves the Rust boundary. TypeScript receives camelCase with parsed JSON and proper booleans. No `JSON.parse()`, no case conversion, no `toWire/fromWire` functions in TypeScript.

**ts-rs cross-crate trap**: ts-rs computes import paths in generated TS from the Rust *source* crate's location, not the `export_to` directory. Moving a ts-rs-anchored type into a different crate while consumers stay put emits broken `../../../../` import paths in every referencing `.ts`. Move ALL `#[derive(TS)]` types together to one crate in a single atomic migration — never partial/incremental cross-crate moves — and verify byte-identical generated TS via sha256 diff.

### Sophia Integration

Sophia (forked from Khan Academy Perseus) renders assessments in three modes: mastery (graded), discovery (psychometric), reflection (open-ended). It distributes as a web component `<sophia-question>` via `sophia-element` UMD bundle, wrapped for Angular by `sophia-plugin` in elohim-library.

Sophia is the **rendering layer only** - it produces Recognition callbacks. Session management, aggregation, and interpretation belong in the consuming app's services (lamad pillar).

### Doorway Gateway

Rust service consolidating three functions: bootstrap (agent discovery), signal (WebRTC), and gateway (conductor proxy + caching). Serves both hosted users (browser via doorway.elohim.host) and local dev (proxied via Angular dev server at localhost:8888).

### Bridges (`bridges/`)

Pluggable interop crates that translate external protocols to and from elohim's canonical EPR-REA substrate. Runtimes consume the bridges relevant to their job: `doorway-service` consumes web2 bridges (`atproto`, `activitypub`, planned); `elohim-storage` consumes protocol-shaped bridges (`valueflows` for hREA / VF-GraphQL). See `bridges/CLAUDE.md` for the pattern and `genesis/docs/superpowers/specs/2026-05-20-wave3-valueflows-hrea-interop-design.md` for Wave 3 substrate work.

### Content Pipeline

`genesis/` contains source content (markdown, Gherkin) and seeder tools. Content flows: genesis docs -> elohim-import CLI -> seed data JSON -> seeder -> elohim-storage -> doorway -> elohim-app.

### Deployment Contexts

The app runs in four modes with different content loading paths:
- **Eclipse Che**: Dev server proxy to doorway (avoids CORS)
- **Local dev**: Same proxy pattern
- **Production**: Browser direct to doorway.elohim.host
- **Tauri desktop**: Direct HTTP to local elohim-storage sidecar at :8090

**DNA changes don't redeploy by default**: a DNA-content change (new hash, same role structure) does NOT reach running conductors on a normal edge redeploy — the conductor data dir is a persistent PVC and the install stale-check is role-structure-only, so a new hash reads as "not stale." Forcing a reinstall is gated behind `ALLOW_DNA_REINSTALL` (default false; reinstall mints a new agent key, which on prod needs migration/lineage, not a blind wipe). Wired per-env in `elohim/holochain/Jenkinsfile` (non-prod=true). If you force-reinstall on some peers but not all in a namespace, they land on different DNA hashes → different DHTs → P2P partition; the alpha genesis pair must both get the flag. **Coordinator-zome-only changes never move the DNA hash at all** (the hash covers integrity zomes + modifiers only — `uhCok…` in errors is a wasm hash, `uhC0k…` a DNA hash): that class is healed by `happ_manager::sync_coordinators` via the conductor's `update_coordinators` hot-swap (no re-key, no DHT churn), gated by `ALLOW_COORDINATOR_UPDATE` (defaults to `ALLOW_DNA_REINSTALL`'s value). When a shipped DNA fix doesn't land, first check which zome class the diff touched.

**Cluster ops are operator-owned**: never run `kubectl` (read or write) from the dev environment. "Clean up X resource" / "fix the live ingress" means make the repo manifests in `genesis/manifests/` (and orchestrator manifests) coherent so the next pipeline reconciles — the repo is the cleanup surface, the live cluster is the operator's. Read cluster state via Jenkins MCP or in-repo manifests, not `kubectl get`.

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

### Frontend Eyes (available rails)

Frontend review/refinement is eyes-first: render before reading source. Rails available to any agent (run from `genesis/a2o`):

- **`pnpm look <url> [--as <FixtureHuman>]`** (`genesis/a2o/scripts/look.ts`) — render any URL headless; writes `reports/look/<slug>/{shot.png,capture.json}` (console/pageerror/failed-request/httpError capture). Deployed app: `https://doorway-alpha.elohim.host`.
- **`pnpm graphos {list|story|sheet}`** (`genesis/a2o/scripts/graphos.ts`) — enumerate/render the graphos component library + design guide from the deployed Storybook (`https://storybook.elohim.host`, latest dev) or a local `pnpm storybook` (`--base http://localhost:6006`). `sheet <component>` = the full cell/theme matrix (Library A `default` vs Library B `designed` sections) in ONE composite image. Design: `genesis/docs/superpowers/specs/2026-06-11-graphos-look-design.md`.
- **`pnpm reports:serve`** (port 4201) — the operator sees the same artifacts the agent reads (symmetric vision).
- Can't-find ≠ never-implemented: if a described view doesn't render, suspect present reachability (capture.json `httpErrors`, routes, env) before concluding absence.

### Story Harvest (on branch finish and after debugging)

When using `finishing-a-development-branch`, invoke `story-harvest` between Step 1 (tests pass) and Step 3 (present options). When using `systematic-debugging` and a root cause is identified and fixed, invoke `story-harvest` before closing the debugging session. The skill identifies engineering constraints discovered during development — especially parameter-bearing discoveries (memory limits, concurrency thresholds, cache sizes) that inform operator presets and peer diversity configuration — and scaffolds a2o regression scenarios to preserve them.

### P2P Design Gate (MANDATORY)

Before proposing design approaches for ANY feature involving data entities (tables, models, routes, sync messages), invoke the `p2p-design-gate` skill — gates brainstorming step 3. This exists because AI agents default to relational-DB patterns (UUID primary keys, REST-first, CID-as-column); the protocol requires P2P-native thinking (DHT entry types first, content addressing for identity, storage as projection not truth).

**The skill forces you to answer:** (1) Is the entity notarized (A), derived via link (A2), agent-scoped (B), agent-scoped with attestation (B2), or operational (C)? (2) Does a DHT entry type already exist? (Lamad DNA ~73/~100; Mishpat ~11/~100 — check headroom.) (3) Is identity content-derived (CID), agent-composite, or slug (must justify)? (4) What coordinator function creates it; what signal projects it? Answer BEFORE designing the HTTP route. If you're about to write `GET /api/v1/thing` without having answered these, STOP and invoke the skill.

### Memory cleanup trigger (SessionStart)

The SessionStart MEMORY BUDGET headline carries a `cleanup:` gate line. When it reads **`cleanup: ⚠ … due`** (≈ a week of heavy development has accumulated drift past the threshold), run the **`memory-stasis-loop`** workflow before substantive work — it drains every memory discipline (compaction · dumps · decompose · MAP/path · roadmap · memkit · MemPalace) back to stasis and self-resets the drift baseline. The reset means it fires at most ~once per heavy-dev week; on a quiet week the gate reads `held ✅` and you skip it. (This is the local, drift-accurate auto-trigger — the activity accumulators are session-local, so the trigger lives here in gospel, not in a remote cron.)

### Substrate scope trigger (SessionStart)

The same headline carries a `scope:` gate line — the planning-layer analog of how CI reconciles to `ELOHIM_REMOTE_COMPUTE_STATUS`. The substrate signal has **two homes that must agree**: `ELOHIM_REMOTE_COMPUTE_STATUS` (the Jenkins *Probe Substrate* stage sets it from a live pool probe — the CI/runtime layer self-reconciles each run) and `genesis/manifests/cluster-state.yaml` (the durable declaration the planning layer reads). When a capability goes down or comes back, flip cluster-state (evidence-backed — mirror the probe, never aspirational). A **third home is now derived, never hand-written**: `genesis/orchestrator/data/deployments.json` `suspended` flags (deploy-render + seed + a2o test gates) reconcile from each human's `nodeTypes` × cluster-state `provides_node_types` with `suspendedBy: scope-reconcile:<cap>` provenance — hand-flipping them is how the homes drifted on 2026-06-03 (11 shem-only humans stayed declared-deployed; 503 storm). `scope-reconcile.py --apply` cascades all of it; operator-manual suspensions (no marker) are never touched. Then:

- `scope: aligned ✅` — the held/ tree matches the substrate; nothing to do.
- **`scope: ⚠ N ready to expand (cap)`** — a capability returned; held docs (specs · plans · a2o features) are ready to move back onto the plate. Run **`scope-reconcile.py --apply`** — it `git mv`s them live↔held; inbound `cites:` flip `HELD-CITE ↔ healthy` automatically (content-addressed, so a move never breaks a link). This is **stories + architecture shifting together with the substrate**: narrow the plate when a capability is lost (focused dev loops + deployments on what's actually verifiable), expand it effortlessly when the capability returns — and continue with the enabled context intact.
- **`scope: ⚠ N to hold (cap)`** — a capability was lost; live docs that need it move to `held/` (out of the planner/runner scan path) so they don't false-fail. Same command.
- **`scope: ⚠ N to return to plate`** — a held doc now has satisfiable work (a capability returned, OR it's a *mixed* plan with household-testable gaps). It belongs back on the plate. Same command.
- **`⚠ unknown-cap: X`** — a `requires_env` cap in a `.md` doc matches no `cluster-state.yaml` resource name (vocab drift, e.g. `harbor` vs `harbor-registry`). Reconcile the name; an unknown cap conservatively blocks held→live escape. (a2o `.feature` `@requires:` tags are a mixed hardware+fixture namespace — unknown ones there are fixture preconditions, not drift, and are ignored.)

**Flipping state (developer control).** shem is a runtime capability, but you flip it deliberately — you don't wait for a probe. `scope-reconcile.py --set shem=off|on [--apply]` edits the durable home (cluster-state.yaml) and prints the coherent runtime export; `eval "$(scope-reconcile.py --env)"` sets `ELOHIM_REMOTE_COMPUTE_STATUS` *derived from* cluster-state so the two homes cannot disagree. That's the full-control flip: narrow the plate (`--set shem=off`), run the verifiable slice as a focused dev loop, then expand it back (`--set shem=on`) and resume with the enabled context intact. **Scope is gap-granular (`iroh ≠ shem`).** A plan can be *mixed*: most gaps testable on `household-nodes` now, only a few needing an unavailable cap. Don't hold it whole — that benches the testable work. Each gap resolves a `requires_env`, defaulting to the doc-level frontmatter value and overridable per gap with an inline `@requires:<cap>` tag (isomorphic with a2o's per-scenario `@requires:`). A gap is BLOCKED-BY-ENV iff `resolved_requires_env ⊄ available` — *regardless of which directory its doc sits in*. Convention: a **uniformly-blocked** plan sets a doc-level `requires_env` (every gap inherits → held whole); a **mixed** plan declares *no* doc-level `requires_env` and tags only its divergent gaps. The budget (`placement-audit --ledger`) counts a held gap as BLOCKED-BY-ENV, not active OPEN; `scope-reconcile` holds a doc whole only if *every* gap is blocked. Resolver: `.claude/scripts/_lib/env_scope.py`.

The move stays one command (a scope decision, operator-in-loop) — the gate just makes the readiness impossible to miss. Spec: `genesis/docs/superpowers/specs/2026-06-02-scope-tree-reconciler-design.md`.

## Schema & Manifest Sources of Truth

Two authoritative schemas govern content types and formats; all generated artifacts derive from these (never hand-edit generated files). **Protocol Schema** (`elohim/sdk/schemas/v1/`) governs DNA-notarized enums — generates `<consumer>/src/generated/schema-enums.ts` via `pnpm run schema:codegen:ts` and Rust constants via `pnpm run schema:codegen:rs`. **Lamad Manifest** (`elohim/sdk/domains/lamad/manifest.json`) governs app vocabulary (content types/formats, renderer mappings, relationships, signals, coupling rules) — generates `<consumer>/src/generated/manifest-types.ts` via `pnpm run lamad:codegen`.

### Key distinction: core vs extensible formats

The protocol schema defines **broad core formats** (`markdown`, `html`, `interactive`, `video`, `audio`, `external`, `epr-composite`) that are DNA-notarized. The lamad manifest defines **specific extensible formats** (`sophia-quiz-json`, `html5-app`, `gherkin`, etc.) that map to Angular renderers. **Seed data must use lamad manifest formats, not core protocol formats** — the renderer map only knows about formats declared in the lamad manifest. Using a core format like `interactive` (which has no renderer) causes content to fall through to the raw JSON fallback.

| Content | Correct `contentFormat` | Wrong |
|---------|------------------------|-------|
| Sophia quiz/assessment | `sophia-quiz-json` | `interactive` |
| HTML5 simulation | `html5-app` | `interactive` |
| Discovery assessment | `sophia-quiz-json` | `interactive` |

### View Schema Contract (HTTP wire shapes)
View schemas in `elohim/sdk/schemas/v1/views/` define the JSON wire format for HTTP API responses (source of truth for the Rust-to-TypeScript boundary). **Pattern:** Write the schema -> Rust structs match (`#[serde(rename_all = "camelCase")]`) -> validation harness (`elohim/elohim-storage/tests/schema_contract.rs`) catches drift -> TS codegen generates interfaces. **Conventions:** see `elohim/sdk/schemas/v1/views/CONVENTIONS.md` for the 10 rules.

**Adding a new view:** (1) write `{name}.schema.json` in `elohim/sdk/schemas/v1/views/`; (2) write matching Rust struct in elohim-storage; (3) add schema contract test; (4) add to `INTERFACE_FILES` in `elohim/sdk/schemas/scripts/codegen-ts.mjs`; (5) run `pnpm run schema:codegen:ts`; (6) pre-push hook validates codegen freshness automatically.

## Critical Gotchas

### RUSTFLAGS Override Required
The system sets `RUSTFLAGS=--cfg getrandom_backend="custom"` for Holochain WASM builds, which breaks native Rust builds. Use `RUSTFLAGS=""` for doorway/doorway-service and steward/node; keep the custom backend flag for elohim/elohim-storage.

### Jenkinsfile Size Limit
Root `Jenkinsfile` (elohim-app pipeline) sits near the 64KB JVM CPS method size limit — **breached 2026-06-10 at 1596 lines** (`MethodTooLargeException` at Jenkinsfile compile; builds #1519/#1520 died before ANY stage ran, so the red looks total and stageless). Line count is only a proxy: large inline `sh """…"""` heredocs in helpers are what inflate the single CPS dispatch method (top-level helpers inline into it — `// STAGE HELPER METHODS` placement alone does NOT save bytecode). Rule: helpers stay heredoc-free — bash bodies live in `scripts/ci/*.sh`, called as `sh "bash '${env.WORKSPACE}/scripts/ci/<name>.sh' args…"` (secrets via `withEnv`, never argv).

### Jenkins params.MODE Null on First Build
MultiBranch pipeline params are null until the Jenkinsfile runs once. Always use `(params.MODE ?: 'auto')`.

### sophia-element UMD Must Be Pre-built
Build before elohim-app builds; the `prebuild` script checks. Run: `cd sophia && pnpm install && pnpm build && pnpm build:umd`.

### pnpm Workspace
All TypeScript/Node.js projects use pnpm workspaces (sophia excluded — submodule). Target packages with `pnpm --filter <name> <cmd>`. The `libsodium-wrappers` package is overridden to `^0.8.2` in root `package.json` to fix a broken ESM relative import in 0.7.x that fails with pnpm's strict module resolution.

### libp2p API (steward/node + elohim-storage)
Both crates declare `version = "0.54"` and resolve `0.54.1` (Cargo.lock verified 2026-06-11 — the earlier "node 0.53 vs storage 0.54" split is STALE). Requires `macros` + `ed25519` features. `request_response` behaviours are constructed via `Behaviour::new([(Proto, ProtocolSupport::Full)], cfg)` — `with_codec()` was a pre-0.54 idiom, no longer used. Swarm event loop uses `StreamExt::next()` (not the deprecated `select_next_event()`).

## CI/CD

Central orchestrator pattern: only `genesis/orchestrator/Jenkinsfile` receives GitHub webhooks, analyzes changesets, and triggers downstream pipelines. Downstream jobs use `overrideIndexTriggers(false)` and validate `UpstreamCause` or `UserIdCause`. Recurring CI/orchestrator traps (NOT_BUILT/superseded ≠ regression, host-green ≠ CI-green, baseline-rollback over-build, sccache poisoning, `#[ignore]` is a CI no-op): see the frequency-ranked museum record `genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md`.

> **Recurring CI/orchestrator watch-outs** (read before debugging a "regression"): `NOT_BUILT`/`ABORTED`/superseded builds read as 0-failures (lossy measure); `#[ignore]` is a CI no-op (DNA sweettests run `--run-ignored all`); webhook double-fire; baseline-rollback over-build. The frequency-ranked museum record is the canonical home: `genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md` (orchestrator-specific watch-outs detailed in `genesis/orchestrator/README.md`).

Pipeline metadata is declared in per-project `build-manifest.json` files. The `genesis/orchestrator/graph-walker.mjs` + `build-graph.groovy` walk these manifests to build a dependency graph and determine which pipelines to trigger. `pipeline-registry.mjs` exposes the metadata to JavaScript consumers.

**Force dispatch:** Use commit tag syntax `[build:edge|dna|app|genesis|sophia|steward|all]` to force-dispatch specific pipelines on any trigger type (webhook, timer, manual, replay). Example: `git commit --allow-empty -m "test E2E [build:edge]"`.

**Jenkins MCP is anonymous (OIDC-protected):** all `mcp__jenkins__*` READ tools work; `triggerBuild`/`updateBuild` are denied — don't call them. Trigger builds only via a fresh `git push` with a `[build:*]` tag. Never add an `Authorization` header to the MCP registration (it triggers a 50-redirect OIDC login loop).

**New Jenkinsfiles:** any in-container git op (other than `checkout scm`) needs `sh 'git config --global --add safe.directory "*"'` first, or git fails with `dubious ownership` (checkout runs as a different UID than the build container). The multibranch job `elohim-holochain` loads the **DNA** Jenkinsfile (`elohim/holochain/dna/Jenkinsfile`), NOT the edge one — verify via the console's `Obtained …/Jenkinsfile from <sha>` line before editing "the holochain Jenkinsfile."

| Pipeline | Jenkinsfile | Manifest |
|----------|-------------|----------|
| App | `Jenkinsfile` (root) | `app/elohim-app/build-manifest.json` |
| Edge | `elohim/holochain/Jenkinsfile` | `elohim/holochain/build-manifest.json` |
| DNA (Lamad) | `elohim/holochain/dna/Jenkinsfile` | `elohim/holochain/dna/build-manifest.json` |
| DNA (Mishpat) | `elohim/holochain/dna/mishpat/Jenkinsfile` | `elohim/holochain/dna/mishpat/build-manifest.json` |
| Genesis | `genesis/Jenkinsfile` | `genesis/build-manifest.json` |
| Sophia | `sophia/Jenkinsfile` | `sophia/build-manifest.json` |
| Steward | `steward/Jenkinsfile` | `steward/build-manifest.json` |

## Code Style

- **TypeScript/Angular**: ESLint 9 flat config with SonarQube parity rules; Prettier (100 char width, single quotes, trailing commas); import order builtin → external → `@app/*` → `@elohim/*`; strict TypeScript + Angular strict templates; path aliases in `app/elohim-app/tsconfig.json`.
- **Rust**: `cargo fmt` + clippy with `-D warnings`; configs at `doorway/doorway-service/clippy.toml` and `rustfmt.toml`.
- **Sophia (React/TypeScript)**: pnpm workspace, Jest + @testing-library/react; packages prefixed `@ethosengine/*` (sophia) or `@khanacademy/*` (math utilities); psyche-core must NEVER depend on perseus packages.
