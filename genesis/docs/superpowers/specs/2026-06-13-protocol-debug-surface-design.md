# Protocol Debug Surface — Reusable, Context-Routed, Native-Level

**Date:** 2026-06-13
**Status:** Design (supersedes the elohim-app-only handoff `genesis/docs/superpowers/plans/2026-06-13-self-healing-debug-view-handoff.md`, which it absorbs)
**Branch grounding:** `feat/frontend-eyes-sprint` HEAD (self-healing backend Plans A/B/C/D-core merged in via `db68a809a`; `/admin/self-healing` landed `82edc611e`).

## Problem & redirection

The committed handoff would expose doorway's `GET /admin/self-healing` read model as a debug view bolted onto elohim-app's **doorway pillar**, with a build-time flag, as a single component. The operator redirected:

> "This sort of view should be re-usable between doorway and tauri — don't just stick it on doorway. Put debug views in a hidden but accessible place that keeps doorway thin… like chrome://flags, a *native aspect of the shell*. We're trying to get as low-level as we can, so a hop into native (tauri) app territory makes sense. Also support the native *range* of debug levels."

Two reframes follow:
1. **Reuse axis = elohim-app's two deployment contexts** (web-via-doorway and tauri-direct-to-storage), *not* the two Angular apps. elohim-app is one build that runs in both (`steward/device/src-tauri/tauri.conf.json:15-16` → `frontendDist: "../ui"`). "Keep doorway thin" only parses if *doorway = the gateway service*, so the surface must not add doorway code.
2. **"chrome://flags" = a native aspect of the shell.** The surface lives at the **shell level**, hidden-but-accessible (reachable by URL, not advertised), reaching as low as the native (tauri IPC) layer where it can.

## Verified facts (load-bearing)

| Fact | Evidence |
|---|---|
| elohim-app runs in web **and** tauri (same build) | `tauri.conf.json:15-16` `frontendDist:"../ui"`, `devUrl:4200` |
| Per-context data router exists | `CONNECTION_STRATEGY` DI token, auto-detects `__TAURI__` → storage `:8090` (`connection-strategy.provider.ts:45-54`; `connection-strategy-factory.ts:43-59`) |
| Tauri hits the **same storage HTTP routes** directly | `elohim/elohim-storage/CLAUDE.md` "Tauri/Direct… same HTTP routes as the proxied path" |
| Runtime-adjustable level knob (native Angular range) | `LoggerService` 4 levels `debug/info/warn/error`, `setMinLevel()`, `recentLogs` signal (100), `getRecentLogs()`/`clearRecentLogs()` (`logger.service.ts:37-45,126-128,257-269`) |
| Deeper native range (Rust) | tracing `Off/Error/Warn/Info/Debug/Trace`, hardcoded `Info` (`steward/device/src-tauri/src/lib.rs:416-421`); node EnvFilter via `RUST_LOG` (`node/main.rs:68-73`) — **no JS↔Rust live bridge today** |
| Built-but-dark reuse asset | `HealthIndicatorComponent` + `HealthCheckService` (holochain/indexedDb/blobCache/network), mounted nowhere (`components/health-indicator/`, `services/health-check.service.ts`) |
| `StabilityStatusView` schema present, **not codegen-registered** | `elohim/sdk/schemas/v1/views/stability-status-view.schema.json` exists; only `generated-ts/views/` has it; **not** in `codegen-ts.mjs` `INTERFACE_FILES` |
| Stability signals reachable tauri-direct from storage | `/api/v1/status/projector` → `ProjectorStatusView{cursors,lag[].lagSeconds}` (`projector/status.rs:69-76`); `/p2p/status` → `ProjectionReconcileStatus{caught_up,divergent_anchor,pending,completed,failed,peers_asked,healed_total,sweeps}` + per-peer health (`p2p/projection_reconcile.rs:107-141`) |
| `/admin/self-healing` is doorway-composed, unauthenticated | `routes/self_healing.rs:32-51` (peers/conductor/render/warmup from doorway-process `AppState`); "operator-only is an ingress property, not enforced here" (`server/http.rs`) |

## The signal-reachability tiers (why per-context routing is necessary)

| Tier | Signals | web (doorway) | tauri (direct :8090) |
|---|---|---|---|
| **1 — storage-native** | projector lag, p2p peers + caughtUp + divergentAnchor, conductor connection (`/health`), arc-policy `observedN`, inventory parity | reachable (via composed endpoint, below) | **reachable directly** |
| **2 — doorway-role** | SSR render trace, warm-stream, conductor **pool**, self-healing **aggregation** | reachable (`/admin/self-healing`) | **no analog** (no SSR/pool in a single embedded node) |
| **3 — tauri-native** | `doorway_status` IPC, conductor admin port 4444, saved accounts, pending deep links | **no analog** | reachable (IPC) |

The crux: Tier 2 in tauri is **not "missing" — it has no analog** (role-specific N/A). Tier 1 is reconstructable in tauri from storage raw endpoints; Tier 3 is native-only. A reusable surface must therefore route per context and degrade *honestly*.

## Architecture

### Home: a shell-level `/debug` surface in elohim-app

A new **DebugShellComponent** routed at top-level `/debug` (`app.routes.ts`), **not** under the doorway pillar. Reused across both contexts for free because elohim-app *is* the code in both. Doorway gains **zero** code; storage gains **zero** code (see "Backend: untouched").

**Hidden-but-accessible (the faithful chrome://flags model):** the `/debug` route **always resolves by URL** — there is **no redirecting guard**. Discoverability is what's gated: a nav entry to `/debug` appears only when debug mode is on (dev-mode default-on). This matches chrome://flags exactly (the page exists; you type the URL; you flip flags there). It is honest about security: the route gates nothing, and the endpoint it reads is already public (see Security).

> **Rejected:** a hidden gesture on `protocol-omni`. That component is a provenance-true *trust* surface ("Anything added here must be provenance-true, never decorative" — `app/elohim-app/src/app/elohim/CLAUDE.md`); a secret debug gesture is exactly the decorative overload it forbids. chrome://flags has no visible affordance anyway.

### The reusable seam: a lens registry + a context service

- **`DebugContextService`** (`elohim/services/`): exposes the active `ConnectionMode` (`'doorway' | 'tauri' | 'direct'`), resolved storage base URL, `__TAURI__` presence, and env name — derived from the existing connection strategy. This is the single per-context router lenses consult.
- **`DebugLens`** registry: an ordered list of small standalone "lens" card components. Each lens injects what it needs, consults `DebugContextService` for sourcing, and renders three honest states per row: **real value**, **pending wire-up** (e.g. `autoPreset`/`admission`/`upstreams` reserved null/`[]`), and **N/A in this context** (Tier-2-in-tauri, Tier-3-in-web). The shell just renders the registry; lenses own their own availability.

### Initial lenses (MVP)

1. **Stability** (`StabilityStatusView` shape — one TS type, two data sources):
   - web (`'doorway'`): `GET /admin/self-healing` → full view (render/warmup/pool real).
   - tauri/direct: **compose the same `StabilityStatusView` client-side** from storage raw endpoints — `projector` ← `/api/v1/status/projector` + caughtUp/divergentAnchor from `/p2p/status`; `peers` ← `/p2p/status`; `conductor` ← `/health`; `render`/`warmup`/`upstreams`/`admission`/`autoPreset` → N/A ("doorway-role — not applicable on this node"). Optional extra native row: arc-policy `observedN`.
   - **Bounded cost (named, not hidden):** the tauri path is a *second* composition that can drift from Rust `compose_self_healing()`. Mitigation: it builds the **codegen'd `StabilityStatusView`** type (compiler enforces shape) and fills only the storage-reachable subset, leaving doorway-role fields at their schema-reserved null/`[]`.
2. **Health**: mount the dark `HealthIndicatorComponent` (finally gives it a home) — holochain/indexedDb/blobCache/network, works in both contexts.
3. **Logging** (the native-level lens): a level selector bound to `LoggerService.setMinLevel()` over the **native 4-level Angular range**, persisted to `localStorage` (chrome://flags-style sticky); a live log viewer over `getRecentLogs()` with level filter + clear. In tauri it additionally **displays** the native Rust 6-level range (read-only; live-adjust deferred — see Levels).
4. **Connection/context**: active mode, resolved base URLs, `__TAURI__`, env, `connectionMode` config — pure-client, cheap, answers "what context am I in".
5. **Feature flags**: read-only display of `FeatureFlags`, explicitly noting the `useGraphqlTopology` doc-says-false/set-true-everywhere drift.
6. **Native** (tauri-only): `doorway_status` IPC (connected, doorway_url, agent-key presence, is_steward, has_key_bundle), saved-accounts count, pending-deep-link count, conductor admin port. Renders "native-only — not available in web" otherwise.

### Levels: support the native range *per layer*

- **Angular layer:** the 4 native levels, **live-adjustable now** (`setMinLevel`), sticky via `localStorage`. This is the only runtime-adjustable verbosity that exists today and is the primary knob.
- **Native/Rust layer (tauri):** the 6-level tracing range is **displayed now**; **runtime adjustment is deferred** to a follow-on that adds a `set_log_level` Tauri IPC command + a `reload`-able tracing `EnvFilter` (neither exists today — there is no JS↔Rust level bridge). The lens never invents a custom scale: it shows whichever layer's *native* enum it is reading/adjusting.

### Read-only plane (+ client-local toggles only)

The surface is **diagnostics + client-local toggles only**: log level, debug-nav visibility, and (optionally) local feature-flag display/overrides scoped to *this client*. **No backend actuation** — that honors the stability program's decision (D8) that the read plane is separate from the REA actuation plane (`tune_knob`/`quarantine_peer`). chrome://flags *is* itself client-local toggles, so this matches the metaphor rather than weakening it.

### Backend: untouched (the genuinely-thinnest path)

- **Doorway:** no change. `/admin/self-healing` already exists and is the one legitimately doorway-resident composition.
- **Storage:** no change. The tauri lens reads endpoints that already exist (`/api/v1/status/projector`, `/p2p/status`, `/health`, `/api/v1/status/arc-policy`); tauri hits them directly at `:8090`.
- **Dev server only:** add `/admin` to `app/elohim-app/proxy.conf.mjs` **and** `proxy.conf.alpha.mjs` so the *local dev* web context can reach `/admin/self-healing` through the `:4200→:8888` proxy (an unproxied `/admin/*` silently returns `index.html`). This is a dev-proxy edit, not doorway code, and is unnecessary in prod (doorway serves `/admin/*` same-origin).
- **Codegen prerequisite:** register `stability-status-view` in `codegen-ts.mjs` `INTERFACE_FILES` and distribute (`pnpm run schema:codegen:ts`); it is authored but not yet distributed to consumer dirs.
- **Contract hygiene:** add a `doorway/doorway-service/tests/schema_contract.rs` case locking `SelfHealingView` ↔ `stability-status-view.schema.json` (cheap; the web lens consumes the composed endpoint).

> **Deferred (not MVP):** registering `/p2p/status` in storage's `build_manifest()` so doorway auto-proxies it. Only needed if a *future web lens* wants raw per-peer P2P detail richer than the composed `/admin/self-healing` — the stability lens does not. Left as a flagged follow-on.

## Security (carried forward, reconciled with prod-reachability)

`/admin/self-healing` is **unauthenticated and currently publicly reachable** on prod/alpha doorway (no ingress `/admin/*` rule found; "operator-only" is aspirational). It exposes **peer IDs + per-peer topology/health** — a deanonymization/targeting surface in a privacy-oriented P2P protocol, materially more sensitive than render counters though carrying no secrets/PII.

Honest position for this design:
- The debug surface **re-exposes an already-public endpoint**, so it adds **no new leak**.
- The `localStorage` debug flag **protects nothing** — it is UI visibility, not access control.
- **Ingress-protecting `/admin/*`** (restrict to operator networks) remains an **operator-owned cluster decision**, out of scope here. Flag it; do not build endpoint auth as part of this work. Keep the debug-nav default **off** in prod/alpha.

## Operator veto-points (recommended answers given; flagged because they interpret the operator's words)

1. **doorway-app out of scope.** The operator said the literal word "doorway," but "keep doorway *thin*" + the reuse axis point at the gateway service and elohim-app's two contexts. The `/threshold/` operator dashboard (browser-only, shares only `@elohim/identity`) is a *possible later third consumer* via a library graduation — not in this MVP. **Veto if you actually meant the operator dashboard.**
2. **Tauri 6-level live adjustment deferred.** Given "*make sure* you support the native range of levels," confirm that **display-the-native-6-now / live-adjust-later (IPC follow-on)** satisfies the intent, vs. wanting the JS↔Rust level bridge built in this pass.

## Out of scope / YAGNI

- No new storage composed `/api/v1/status/stability` endpoint (client composes Tier-1 trivially; storage can't compose doorway-role parts anyway).
- No general runtime feature-flag *framework* — just a minimal sticky debug-nav toggle.
- No backend actuation, no endpoint auth, no ingress changes (operator-owned).
- No graduation of the lens registry to `@elohim/service` yet (revisit if doorway-app becomes a consumer).

## Components & files (for the implementation plan)

**Create (elohim-app):** `app/elohim-app/src/app/debug/` — `debug-shell.component.{ts,html,scss}`, `debug-lens.ts` (registry + `DebugLens` interface), `lenses/{stability,health,logging,connection,flags,native}-lens.component.ts`; `elohim/services/debug-context.service.ts`; `services/debug-mode.service.ts` (sticky nav toggle).
**Reuse:** `HealthIndicatorComponent`, `LoggerService`, `CONNECTION_STRATEGY`.
**Modify:** `app.routes.ts` (top-level `/debug`, no redirecting guard), nav (entry gated on debug-mode), `proxy.conf.mjs` + `proxy.conf.alpha.mjs` (`/admin`), `codegen-ts.mjs` (`INTERFACE_FILES`), `environment.types.ts` (`showDebug?` flag, default false).
**Generated (distribute, don't hand-edit):** `stability-status-view.ts` across the 6 consumer dirs.
**Test (doorway):** `doorway/doorway-service/tests/schema_contract.rs` case for `stability-status-view`.

## Done-when

- `/debug` resolves by URL in all contexts; nav entry shows only when debug-mode on; prod default off.
- Stability lens shows the full composed view in web and a client-composed subset (with honest doorway-role N/A) in tauri, both typed by the codegen'd `StabilityStatusView`.
- Logging lens live-adjusts the Angular level (sticky) + shows the recent-log stream; tauri additionally displays the native Rust range.
- Health/Connection/Flags/Native lenses render with honest per-context availability.
- Doorway and storage source unchanged; only dev-proxy + codegen + a contract test touch the backend tree.
- Security note surfaced to the operator; codegen `--verify` green.
