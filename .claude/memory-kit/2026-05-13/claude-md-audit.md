# CLAUDE.md Review — 2026-05-13
Read-only ceremony. Walks every CLAUDE.md in the repo and classifies by drift signal + content patterns. Operator-gated; no auto-edits.
**Threshold**: drift_score ≥ 3.0 flags a file.
**Total files audited**: 25

## Summary by classification
| Classification | Count |
|---|---:|
| MAYBE-UNNECESSARY | 3 |
| UNDER-WROTE | 1 |
| OVER-WROTE | 0 |
| DRIFTED-FACTUAL | 18 |
| DRIFTED-NORMATIVE | 0 |
| OVER-BUDGET | 1 |
| OVER-IMPERATIVE | 2 |
| FRESH | 0 |
| MISSING-CLAUDE-MD (directories with no doc) | 6 |
| OPTED-OUT (explicit `.no-claude.md` markers) | 1 |

## Directories opted out of CLAUDE.md

Each has a `.no-claude.md` marker file with operator rationale. These are excluded from MISSING-CLAUDE-MD candidacy. Decision chain is auditable — surrounding CLAUDE.md updates may reference these.

| Directory | Marker | Decided | Rationale (excerpt) |
|---|---|---|---|
| `genesis/graphos/design-assets/raw/project/preview` | `genesis/graphos/design-assets/raw/project/preview/.no-claude.md` | 2026-05-13 | This directory contains static design assets (preview HTML + CSS for the graphos |

## Directories that may need a CLAUDE.md

Thresholds: ≥15 files OR ≥4 subdirs, ≥2 distinct complexity extensions, nearest ancestor CLAUDE.md ≥2 levels up. Re-tunable via `MISSING_TUNABLES` in this script.

Showing top 6, sorted by likely orphaning (file count desc, ancestor distance desc):

| Directory | Direct files | Subdirs | Distinct exts | Ancestor (distance) |
|---|---:|---:|---|---|
| `app/elohim-library` | 14 | 8 | 2 (.js .ts) | `<repo-root>` (2↑) |
| `app/elohim-app/src/app` | 10 | 14 | 3 (.css .html .ts) | `app/elohim-app` (2↑) |
| `app/elohim-app/src/app/lamad/components/learner-dashboard` | 5 | 5 | 3 (.css .html .ts) | `app/elohim-app` (5↑) |
| `.claude/scripts` | 5 | 4 | 2 (.py .sh) | `<repo-root>` (2↑) |
| `doorway/doorway-app/src/app` | 5 | 4 | 3 (.css .html .ts) | `doorway/doorway-app` (2↑) |
| `app/elohim-app/src/app/account/components/security-signin-pane` | 4 | 4 | 3 (.css .html .ts) | `app/elohim-app` (5↑) |

## Per-file findings

### MAYBE-UNNECESSARY (3)

#### `app/elohim-app/src/app/elohim/adapters/CLAUDE.md` — drift 0.00, mtime 2026-04-15, 175 lines
- Signals: 0 direct edits, 0 scope edits, **0 structural ops** (mv/cp/rm) since last audit
- Fit: 1 files in direct scope, 1 distinct extensions, 0 sub-CLAUDE.md scopes; ratio 6.25 (expected ~28 lines)
- **Imperatives without rationale** (2; consider adding a `because…` or removing):
  - L4: They do NOT transform wire format - that's already handled by Rust.
  - L10: They don't parse JSON, convert case, or transform types.

#### `app/elohim-library/projects/elohim-service/src/adapters/CLAUDE.md` — drift 0.00, mtime 2026-04-15, 104 lines
- Signals: 0 direct edits, 0 scope edits, **0 structural ops** (mv/cp/rm) since last audit
- Fit: 3 files in direct scope, 1 distinct extensions, 0 sub-CLAUDE.md scopes; ratio 3.59 (expected ~29 lines)
- **Dead paths cited** (5):
  - `conductor-normalizer.spec.ts`
  - `zome-wire-types.ts`
  - `index.ts`
  - `api.rs`
  - `connection/index.ts`
- **Imperatives without rationale** (5; consider adding a `because…` or removing):
  - L18: These are different jobs. **Wire normalization** happens at the transport boundary before the typed pipeline. **Derived fields** happen afte…
  - L28: Doorway does NOT normalize conductor responses — it passes them through as-is (`api.rs`: "Return whatever the DNA returned — no interpretati…
  - L70: Safe entry point when you don't know which format you have. Detects snake_case via `content_type` field presence, normalizes if needed, pass…
  - L80: The conductor normalizer handles **content specifically**, mapping to the schema-generated `ContentView` type with correct field renames (`c…
  - L92: The return type is always `ContentView`. That's the contract.

#### `doorway/doorway-service/src/server/CLAUDE.md` — drift 0.00, mtime 2026-04-28, 34 lines
- Signals: 0 direct edits, 0 scope edits, **0 structural ops** (mv/cp/rm) since last audit
- Fit: 3 files in direct scope, 1 distinct extensions, 0 sub-CLAUDE.md scopes; ratio 1.17 (expected ~29 lines)
- **Dead paths cited** (1):
  - `http.rs`
- **Imperatives without rationale** (1; consider adding a `because…` or removing):
  - L7: - **NO** → Do NOT add it here. Add the endpoint to elohim-storage and register it in `build_manifest()`. The RouteRegistry auto-discovers it…

### UNDER-WROTE (1)

#### `app/elohim-app/src/app/shefa/CLAUDE.md` — drift 0.00, mtime 2026-04-15, 45 lines
- Signals: 0 direct edits, 0 scope edits, **0 structural ops** (mv/cp/rm) since last audit
- Fit: 151 files in direct scope, 4 distinct extensions, 0 sub-CLAUDE.md scopes; ratio 0.35 (expected ~127 lines)

### DRIFTED-FACTUAL (18)

#### `CLAUDE.md` — drift 2.67, mtime 2026-05-01, 281 lines
- Signals: 0 direct edits, 27 scope edits, **0 structural ops** (mv/cp/rm) since last audit
- Fit: 18489 files in direct scope, 11 distinct extensions, 24 sub-CLAUDE.md scopes; ratio 1.41 (expected ~200 lines)
- **Over budget**: 281 lines (warn at 200)
- **Dead paths cited** (6):
  - `rustfmt.toml`
  - `views.rs`
  - `manifest-types.ts`
  - `schema-enums.ts`
  - `clippy.toml`
  - `tests/schema_contract.rs`
- **Imperatives without rationale** (13; consider adding a `because…` or removing):
  - L32: RUSTFLAGS="" cargo build --release     # MUST override RUSTFLAGS (see gotchas)
  - L68: RUSTFLAGS="" cargo build           # MUST override RUSTFLAGS
  - L114: 4. **Adapters** (`app/elohim-app/src/app/elohim/adapters/`) add computed/derived fields only - never transform wire format
  - L116: **Key rule**: snake_case never leaves the Rust boundary. TypeScript receives camelCase with parsed JSON and proper booleans. No `JSON.parse(…
  - L174: 2. Does a DHT entry type ALREADY EXIST? (Lamad DNA is at ~73/~100, Mishpat DNA is at 11/~100 — do NOT create new types without checking head…
  - L175: 3. Is identity content-derived (CID), agent-composite, or slug (must justify)?
  - L182: Two authoritative schemas govern content types and formats. All generated artifacts derive from these — never hand-edit generated files.
  - L193: **Seed data must use lamad manifest formats, not core protocol formats.** The renderer map only knows about formats declared in the lamad ma…
  - ...and 5 more.

#### `.claude/scripts/memory-kit/CLAUDE.md` — drift 2.39, mtime 2026-05-13, 123 lines
- Signals: 1 direct edits, 0 scope edits, **0 structural ops** (mv/cp/rm) since last audit
- Fit: 8 files in direct scope, 1 distinct extensions, 0 sub-CLAUDE.md scopes; ratio 3.84 (expected ~32 lines)
- **Dead paths cited** (20):
  - `path-update-scan/apply.py`
  - `project_shared_lib_pattern.md`
  - `dedupe-memory-scan.py`
  - `claude-md-audit.py`
  - `project_three_temporal_perspectives.md`
  - `project_historian_pattern_surface_agent.md`
  - `memory-review.py`
  - `project_memory_in_repo_two_tier.md`
  - `cleanup-scan.py`
  - `project_signal_driven_audit_ceremonies.md`
  - ...and 10 more.
- **Imperatives without rationale** (2; consider adding a `because…` or removing):
  - L28: skill-audit.py                       ← always-loaded skill descriptions
  - L121: - Not always-active — skills are deferred-loaded; subagents dispatched on demand

#### `app/elohim-app/CLAUDE.md` — drift 0.00, mtime 2026-04-15, 196 lines
- Signals: 0 direct edits, 0 scope edits, **0 structural ops** (mv/cp/rm) since last audit
- Fit: 1141 files in direct scope, 8 distinct extensions, 2 sub-CLAUDE.md scopes; ratio 0.98 (expected ~200 lines)
- **Dead paths cited** (4):
  - `src/app/elohim/services/storage-client.service.ts`
  - `elohim-library/.../doorway-connection-strategy.ts`
  - `doorway-connection-strategy.ts`
  - `src/app/elohim/services/content.service.ts`

#### `app/elohim-library/projects/perseus-plugin/CLAUDE.md` — drift 0.00, mtime 2026-04-15, 146 lines
- Signals: 0 direct edits, 0 scope edits, **0 structural ops** (mv/cp/rm) since last audit
- Fit: 6 files in direct scope, 3 distinct extensions, 0 sub-CLAUDE.md scopes; ratio 3.11 (expected ~47 lines)
- **Dead paths cited** (3):
  - `elohim-app/.../perseus/perseus-element-loader.ts`
  - `elohim-library/projects/perseus-plugin/src/perseus-element.tsx`
  - `elohim-app/.../perseus/perseus-wrapper.component.ts`
- **Imperatives without rationale** (4; consider adding a `because…` or removing):
  - L44: **Why**: setTimeout callbacks can be "orphaned" during page reloads, causing the Promise to never resolve.
  - L64: Perseus uses `--wb-semanticColor-*` variables without fallbacks. Both light and dark mode must define these in `styles.css`.
  - L69: **Symptom**: Quiz shows "No question loaded" message, never renders question.
  - L129: This perseus-plugin module **must bundle or handle SVG assets** during its build so consumers don't need special webpack configuration.

#### `doorway/CLAUDE.md` — drift 0.00, mtime 2026-04-30, 147 lines
- Signals: 0 direct edits, 0 scope edits, **0 structural ops** (mv/cp/rm) since last audit
- Fit: 1 files in direct scope, 0 distinct extensions, 3 sub-CLAUDE.md scopes; ratio 7.35 (expected ~20 lines)
- **Dead paths cited** (8):
  - `RECOVERY-SPRINT-PLAN.md`
  - `SCALING.md`
  - `http.rs`
  - `ARCHITECTURE.md`
  - `governance.rs`
  - `FEDERATION.md`
  - `RECOVERY-PROTOCOL.md`
  - `REACH.md`
- **Imperatives without rationale** (5; consider adding a `because…` or removing):
  - L14: **NEVER create per-domain proxy files in doorway-service.** This is the most important rule for this crate.
  - L26: We deleted 13 identical ~150-line proxy files (governance, attestations, contributors, steward, presence, economic_events, exchange, custodi…
  - L32: **Doorway forwards each request to a SINGLE storage target.** It does NOT iterate `STORAGE_URLS` looking for which peer holds a particular b…
  - L49: - Never know or care which physical peer holds which blob.
  - L53: The tiered blob cache (`TieredBlobCache`) is allocated and its cleanup loop runs, but its blob tier is never written on the storage-proxy pa…

#### `doorway/doorway-app/CLAUDE.md` — drift 0.00, mtime 2026-04-15, 48 lines
- Signals: 0 direct edits, 0 scope edits, **0 structural ops** (mv/cp/rm) since last audit
- Fit: 75 files in direct scope, 5 distinct extensions, 0 sub-CLAUDE.md scopes; ratio 0.49 (expected ~97 lines)
- **Dead paths cited** (7):
  - `doorway-service/src/routes/admin.rs`
  - `../CLAUDE.md`
  - `doorway-admin.service.ts`
  - `doorway-app/src/app/models/doorway.model.ts`
  - `doorway-app/src/app/services/doorway-admin.service.ts`
  - `doorway.model.ts`
  - `doorway-auth.service.ts`

#### `doorway/doorway-service/CLAUDE.md` — drift 0.00, mtime 2026-04-15, 67 lines
- Signals: 0 direct edits, 0 scope edits, **0 structural ops** (mv/cp/rm) since last audit
- Fit: 166 files in direct scope, 2 distinct extensions, 1 sub-CLAUDE.md scopes; ratio 0.56 (expected ~119 lines)
- **Dead paths cited** (20):
  - `SCALING.md`
  - `src/cache/resolution.rs`
  - `src/services/route_registry.rs`
  - `src/main.rs`
  - `src/routes/elohim_agent.rs`
  - `src/projection/subscriber.rs`
  - `REACH.md`
  - `src/routes/collectives.rs`
  - `src/routes/admin.rs`
  - `src/routes/identity.rs`
  - ...and 10 more.
- **Imperatives without rationale** (3; consider adding a `because…` or removing):
  - L8: RUSTFLAGS="" cargo build --release     # MUST override RUSTFLAGS (Holochain WASM env breaks native builds)
  - L14: The system sets `RUSTFLAGS=--cfg getrandom_backend="custom"` for Holochain WASM. This breaks native Rust builds. Always override with `RUSTF…
  - L43: Almost always, you should NOT touch doorway-service when adding a new route. Instead:

#### `elohim/elohim-cache-core/CLAUDE.md` — drift 0.00, mtime 2026-04-15, 110 lines
- Signals: 0 direct edits, 0 scope edits, **0 structural ops** (mv/cp/rm) since last audit
- Fit: 12 files in direct scope, 1 distinct extensions, 0 sub-CLAUDE.md scopes; ratio 3.24 (expected ~34 lines)
- **Dead paths cited** (2):
  - `/wasm/elohim-cache-core/elohim_cache_core.js`
  - `cache/types.ts`
- **Imperatives without rationale** (1; consider adding a `because…` or removing):
  - L19: IMPORTANT: The system sets `RUSTFLAGS=--cfg getrandom_backend="custom"` for Holochain WASM. This breaks native builds for this crate. Always…

#### `elohim/elohim-storage/CLAUDE.md` — drift 0.00, mtime 2026-05-12, 268 lines
- Signals: 0 direct edits, 0 scope edits, **0 structural ops** (mv/cp/rm) since last audit
- Fit: 558 files in direct scope, 3 distinct extensions, 0 sub-CLAUDE.md scopes; ratio 1.34 (expected ~200 lines)
- **Over budget**: 268 lines (warn at 200)
- **Dead paths cited** (7):
  - `views.rs`
  - `http.rs`
  - `src/db/models.rs`
  - `src/db/mod.rs`
  - `src/views.rs`
  - `src/http.rs`
  - `tests/schema_contract.rs`
- **Imperatives without rationale** (3; consider adding a `because…` or removing):
  - L8: **snake_case should NEVER leave the Rust boundary.**
  - L75: │  - NEVER exposed to HTTP directly               │
  - L165: View types must match their JSON Schema in `../sdk/schemas/v1/views/`.

#### `elohim/sdk/CLAUDE.md` — drift 0.00, mtime 2026-04-15, 96 lines
- Signals: 0 direct edits, 0 scope edits, **0 structural ops** (mv/cp/rm) since last audit
- Fit: 41 files in direct scope, 1 distinct extensions, 7 sub-CLAUDE.md scopes; ratio 2.0 (expected ~48 lines)
- **Dead paths cited** (3):
  - `schemas/CLAUDE.md`
  - `domains/lamad/CLAUDE.md`
  - `elohim-storage/src/views.rs`
- **Imperatives without rationale** (3; consider adding a `because…` or removing):
  - L9: If someone could become the bank, the credential authority, the governance board, or the content landlord by controlling it — it's a protoco…
  - L68: **Key rule:** snake_case never leaves the Rust boundary. TypeScript receives camelCase with parsed JSON and proper booleans. No `JSON.parse(…
  - L93: 2. **Observations** — positive and negative evidence (must include negatives)

#### `elohim/sdk/domains/imagodei/CLAUDE.md` — drift 0.00, mtime 2026-04-15, 144 lines
- Signals: 0 direct edits, 0 scope edits, **0 structural ops** (mv/cp/rm) since last audit
- Fit: 10 files in direct scope, 2 distinct extensions, 0 sub-CLAUDE.md scopes; ratio 3.51 (expected ~41 lines)
- **Dead paths cited** (5):
  - `metadata-types.ts`
  - `content-node-types.ts`
  - `coupling-map.ts`
  - `manifest-types.ts`
  - `manifest.json`
- **Imperatives without rationale** (2; consider adding a `because…` or removing):
  - L97: Edit the schema first, then regenerate. Never hand-write types that a schema should own.
  - L117: The manifest requires `attestation-inaccuracy` as a negative observation. If attested claims don't hold up in downstream work, the system mu…

#### `elohim/sdk/domains/lamad/CLAUDE.md` — drift 0.00, mtime 2026-04-15, 203 lines
- Signals: 0 direct edits, 0 scope edits, **0 structural ops** (mv/cp/rm) since last audit
- Fit: 72 files in direct scope, 2 distinct extensions, 0 sub-CLAUDE.md scopes; ratio 2.82 (expected ~72 lines)
- **Over budget**: 203 lines (warn at 200)
- **Dead paths cited** (13):
  - `content-node-types.ts`
  - `manifest-types.ts`
  - `body-types.ts`
  - `manifest.json`
  - `create-content-input.ts`
  - `create-economic-event-input.ts`
  - `content-view.ts`
  - `app-manifest.schema.json`
  - `metadata-types.ts`
  - `coupling-map.ts`
  - ...and 3 more.
- **Imperatives without rationale** (3; consider adding a `because…` or removing):
  - L89: Edit the schema first, then regenerate. Never hand-write types that a schema should own.
  - L113: Renderers don't call economic event APIs directly. They emit `RendererCompletionEvent`. The `SignalHarnessService` reads `LAMAD_COUPLING_MAP…
  - L127: The manifest schema (`app-manifest.schema.json`) rejects content types without `value` and `governance` legs. Claims (feedback) are also req…

#### `elohim/sdk/domains/qahal/CLAUDE.md` — drift 0.00, mtime 2026-04-15, 133 lines
- Signals: 0 direct edits, 0 scope edits, **0 structural ops** (mv/cp/rm) since last audit
- Fit: 64 files in direct scope, 2 distinct extensions, 0 sub-CLAUDE.md scopes; ratio 1.96 (expected ~68 lines)
- **Dead paths cited** (4):
  - `metadata-types.ts`
  - `coupling-map.ts`
  - `manifest-types.ts`
  - `content-node-types.ts`

#### `elohim/sdk/domains/shefa/CLAUDE.md` — drift 0.00, mtime 2026-04-15, 126 lines
- Signals: 0 direct edits, 0 scope edits, **0 structural ops** (mv/cp/rm) since last audit
- Fit: 43 files in direct scope, 2 distinct extensions, 0 sub-CLAUDE.md scopes; ratio 2.21 (expected ~57 lines)
- **Dead paths cited** (7):
  - `metadata-types.ts`
  - `project-pillar-topology-power-responsibility.md`
  - `coupling-map.ts`
  - `project-steward-affinity-anti-capture.md`
  - `project-resource-nature-circularity.md`
  - `manifest-types.ts`
  - `manifest.json`
- **Imperatives without rationale** (1; consider adding a `because…` or removing):
  - L101: Edit the schema first, then regenerate. Never hand-write types that a schema should own.

#### `elohim/sdk/schemas/CLAUDE.md` — drift 0.00, mtime 2026-04-15, 153 lines
- Signals: 0 direct edits, 0 scope edits, **0 structural ops** (mv/cp/rm) since last audit
- Fit: 316 files in direct scope, 1 distinct extensions, 0 sub-CLAUDE.md scopes; ratio 0.82 (expected ~186 lines)
- **Dead paths cited** (10):
  - `v1/enums/my-enum.schema.json`
  - `create-content-input.ts`
  - `v1/inputs/create-thing-input.schema.json`
  - `create-economic-event-input.ts`
  - `content-view.ts`
  - `elohim-storage/views.rs`
  - `schema-enums.ts`
  - `create-attestation-input.ts`
  - `v1/views/thing-view.schema.json`
  - `economic-event-view.ts`
- **Imperatives without rationale** (3; consider adding a `because…` or removing):
  - L11: Never hand-write a TypeScript interface that mirrors a schema. It will drift.
  - L122: These schemas are the **protocol layer** — wire types, enums, generic metadata bag. They don't know what `PathMetadata.thumbnailUrl` means. …
  - L131: App schemas `$ref` protocol schemas for shared primitives. Protocol never refs app schemas.

#### `elohim/sdk/storage-client-ts/CLAUDE.md` — drift 0.00, mtime 2026-04-15, 197 lines
- Signals: 0 direct edits, 0 scope edits, **0 structural ops** (mv/cp/rm) since last audit
- Fit: 577 files in direct scope, 1 distinct extensions, 0 sub-CLAUDE.md scopes; ratio 0.98 (expected ~200 lines)
- **Dead paths cited** (1):
  - `elohim-storage/src/views.rs`
- **Imperatives without rationale** (2; consider adding a `because…` or removing):
  - L8: **DO NOT modify generated files manually.**
  - L11: Changes must be made in Rust, then regenerated.

#### `genesis/a2o/CLAUDE.md` — drift 0.00, mtime 2026-04-15, 47 lines
- Signals: 0 direct edits, 0 scope edits, **0 structural ops** (mv/cp/rm) since last audit
- Fit: 190 files in direct scope, 4 distinct extensions, 0 sub-CLAUDE.md scopes; ratio 0.32 (expected ~147 lines)
- **Dead paths cited** (1):
  - `src/framework/pages/selectors.ts`
- **Imperatives without rationale** (1; consider adding a `because…` or removing):
  - L22: - Background: always include `Given doorway "alpha" at "E2E_DOORWAY_ALPHA"`

#### `steward/device/CLAUDE.md` — drift 0.00, mtime 2026-04-15, 204 lines
- Signals: 0 direct edits, 0 scope edits, **0 structural ops** (mv/cp/rm) since last audit
- Fit: 76 files in direct scope, 1 distinct extensions, 0 sub-CLAUDE.md scopes; ratio 3.09 (expected ~66 lines)
- **Over budget**: 204 lines (warn at 200)
- **Dead paths cited** (7):
  - `src-tauri/src/doorway.rs`
  - `src-tauri/src/lib.rs`
  - `src-tauri/tauri.conf.json`
  - `src-tauri/src/main.rs`
  - `src-tauri/src/identity.rs`
  - `src-tauri/Cargo.toml`
  - `doorway.json`
- **Imperatives without rationale** (4; consider adding a `because…` or removing):
  - L17: - **App + Node Steward**: Steward app + always-on elohim-node daemon. Both are peers stewarded by the same person, providing internal resili…
  - L105: 3. App restart required (conductor must reinit with new network config)
  - L189: - elohim.happ must be built and placed in `workdir/` (CI does this, or `just dna-build`)
  - L190: - Angular UI must be built into `ui/` for production (CI does this, or `just app-build`)

### OVER-BUDGET (1)

#### `elohim/sdk/domains/CLAUDE.md` — drift 0.00, mtime 2026-04-15, 267 lines
- Signals: 0 direct edits, 0 scope edits, **0 structural ops** (mv/cp/rm) since last audit
- Fit: 69 files in direct scope, 2 distinct extensions, 4 sub-CLAUDE.md scopes; ratio 3.81 (expected ~70 lines)
- **Over budget**: 267 lines (warn at 200)
- **Imperatives without rationale** (3; consider adding a `because…` or removing):
  - L72: - No hand-copied structs, no comment saying "must match zome"
  - L171: The types crate must compile for both `wasm32-unknown-unknown` (zome) and
  - L177: When using `#[serde(skip_serializing_if = "Option::is_none")]`, always pair

### OVER-IMPERATIVE (2)

#### `app/lamad/CLAUDE.md` — drift 0.00, mtime 2026-04-15, 17 lines
- Signals: 0 direct edits, 0 scope edits, **0 structural ops** (mv/cp/rm) since last audit
- Fit: 0 files in direct scope, 0 distinct extensions, 0 sub-CLAUDE.md scopes; ratio 0.85 (expected ~20 lines)
- **Imperatives without rationale** (1; consider adding a `because…` or removing):
  - L12: Do not hand-edit — regenerate with `pnpm run lamad:codegen`.

#### `elohim/holochain/dna/CLAUDE.md` — drift 0.00, mtime 2026-04-15, 72 lines
- Signals: 0 direct edits, 0 scope edits, **0 structural ops** (mv/cp/rm) since last audit
- Fit: 92 files in direct scope, 2 distinct extensions, 0 sub-CLAUDE.md scopes; ratio 0.88 (expected ~82 lines)
- **Imperatives without rationale** (4; consider adding a `because…` or removing):
  - L15: If yes, it MUST be notarized here. If someone could become "the bank" by
  - L18: that capability must live on distributed infrastructure where no one can
  - L60: Clients write to the conductor. Storage listens and indexes. Never write
  - L72: RUSTFLAGS is set in the justfile. Don't override it.

---
_Operator-gated. To act: revise flagged sections in their CLAUDE.md files. After making changes, optionally reset signals for that file by editing `.claude/memory-kit/claude-md-drift.json` (set `last_audited` to today and zero out `direct_edits` / `scope_edits`); the next audit will start from a fresh signal baseline._
