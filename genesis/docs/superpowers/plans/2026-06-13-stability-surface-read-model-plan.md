# Stability Surface — Unified Self-Healing Read Model — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a single agent-consumable `GET /admin/self-healing` aggregate endpoint on doorway that composes every LANDED self-healing signal (peers, projector lag/caughtUp, render reliability, warmup, conductor health) into one camelCase Cat-C read model, with stable keys reserved (null + `// FOLLOW-ON`) for the four PENDING sibling signals (autoPreset, admission, upstreams, divergentAnchor wiring).

**Architecture:** The read model is **Cat C node-local operational state** — NO DHT entry, NO table, NO coordinator fn. It is a fresh projection composed at request time from in-process AppState snapshots (`peer_health.snapshot()`, `warmup_state` atomics, `render_trace_stats.snapshot()`, the cached `p2p_health`) plus one on-demand HTTP fetch of storage's `/api/v1/status/projector`. Aggregation is a **pure function** over injected component snapshots so it is unit-testable without a live cluster. The liveness `/health` path stays cheap and untouched — the heavy aggregate lives on its OWN endpoint `/admin/self-healing`, never gated into `/health`. This is the same legitimate-doorway-local-Operational-state class as the already-landed `/admin/capability`, `/admin/render-stats`, and `/admin/dashboard/topology`.

**Tech Stack:** Rust (doorway-service, native build with `RUSTFLAGS=""`); `hyper` + `serde` (hand-rolled serde struct, NO ts-rs — doorway has zero ts-rs); JSON Schema codegen (`pnpm run schema:codegen:ts`) for the TypeScript contract; Angular 19 + RxJS for the thin consumer service; `reqwest` for the on-demand storage fetch.

---

## Decisions

### D1 — Endpoint home: `/admin/self-healing` (NOT `/health?level=trace`)
Decided from C1. The 2026-06-13 doorway-freeze fix deliberately keeps the health path cheap (`handle_health_probe` touches only cached state, `http.rs:931-935`). Loading a heavy aggregate onto `/health` reintroduces the freeze-coupling that fix removed. The aggregate gets its OWN explicit match arm. `/health` and `/health/startup` are NOT modified by the aggregate task (they ARE modified by Task 2's caughtUp plumb, which only widens an existing cached struct — cheap).

### D2 — Cat C node-local, swap-test-by-design-failing
Per spec §3g and §6: this is operational, node-local, never notarized. There is no schema-shareable canonical authorship here — a node serves its OWN runtime state. This is the doorway/CLAUDE.md carve-out: *"Doorway-local Operational state (cache stats, federation peer list) is legitimate doorway-resident state."* State this in the match-arm comment so a reviewer does not reject on the no-new-proxy-files rule.

### D3 — LANDED vs PENDING (from C2/C4/C5) and how this plan stays SELF-CONTAINED
This plan aggregates **only what exists today** and reserves stable keys for PENDING sibling fields as `null`/absent with a `// FOLLOW-ON` note — it never fakes data and does NOT hard-block on the A (inbound-admission) / B (upstream-self-protection) / Auto (auto-config) sibling plans.

| Read-model field | Status | Source (verified) |
|---|---|---|
| `peers[]` | **LANDED** | `state.peer_health.snapshot()` → `Vec<PeerHealthSnapshot>` (`elohim/elohim-compute/src/peers.rs:95-108`) |
| `projector.lagSeconds` | **LANDED** | storage `GET /api/v1/status/projector` → `ProjectorStatusView.lag[].lagSeconds` (`elohim/elohim-storage/src/projector/status.rs:66,73`) — reachable once Task 1 registers it |
| `projector.caughtUp` / `projector.divergentAnchor` | **LANDED in storage, PENDING in doorway plumb** | storage `/p2p/status` `projectionReconcile.caughtUp` / `.divergentAnchor` (`ProjectionReconcileStatus`, `elohim-storage/src/p2p/projection_reconcile.rs`); discarded by both doorway pollers — un-discarded by Task 2 |
| `render` (reliability proxy) | **LANDED** | `state.render_trace_stats.snapshot()` → `RenderTraceSnapshot` (`elohim/elohim-render/src/stats.rs:41-52`) |
| `warmup` | **LANDED** | `state.warmup_state` 5 atoms (`doorway/doorway-service/src/projection/warm_stream.rs:236-243`) |
| `conductor` | **LANDED** | `build_health_response` already composes `ConductorHealth` (`health.rs:93-108`) |
| `autoPreset` | **PENDING (auto-config sibling)** | no `auto_preset` field in doorway/elohim-compute today — key emitted as `null` + `// FOLLOW-ON` |
| `admission` | **PENDING (inbound-admission sibling)** | doorway accept loop unbounded (`server/http.rs:1129`); no inbound semaphore — key emitted as `null` + `// FOLLOW-ON` |
| `upstreams[]` | **PENDING (upstream-self-protection sibling)** | no `WarmStreamHealth`/`UpstreamBreakers`/`is_open()` in tree (grep: zero hits) — key emitted as `[]` (empty) + `// FOLLOW-ON` |

**Dependency ordering:** ideally lands AFTER the Auto/admission/upstream siblings so the reserved keys light up immediately, but is designed to land BEFORE them — each sibling's landing is a one-line wire-up replacing a `None`/`vec![]` literal at the named seam. The plan is self-contained: it ships value (peers + projector + render + warmup + conductor in one call) on day one.

### D4 — Hand-written serde struct, schema-driven TS contract (from C4)
Doorway has ZERO ts-rs (`Cargo.toml:144` excludes it). The Rust read-model struct is a hand-written `#[derive(Serialize)] #[serde(rename_all = "camelCase")]` in a NEW doorway route module. The TypeScript contract is generated from a NEW JSON Schema `stability-status-view.schema.json` in `elohim/sdk/schemas/v1/views/` registered in `INTERFACE_FILES` — the same schema-driven pattern as `p2p-status-view`. The two are kept coherent by convention (camelCase keys match); there is no automated Rust↔schema contract test for doorway structs (that harness is storage-only), so the self-review checks key-name parity by eye.

### D5 — Pure aggregation function (from C5)
The core is `compose_self_healing(snapshots) -> SelfHealingView` — a free function taking already-collected component snapshots (no `AppState`, no I/O), so it is unit-tested with hand-built fixtures. The handler `handle_self_healing(state)` does the I/O (read AppState snapshots + fetch storage projector) then calls the pure composer. Tests target the pure function directly; one handler smoke test uses the existing `test_state()` pattern (`health.rs:293`).

### D6 — Reachability (from C1/C3)
`/admin/self-healing` is covered by `is_service_path` already (`http.rs:1301` lists `"/admin"`, prefix-matched at `:1318`), so it is excluded from the EPR router and reaches its explicit arm — NO `is_service_path` edit needed. The match arm is added ABOVE the registry/wildcard fallback alongside the other `/admin/*` arms (`http.rs:2326-2416`). CORS is applied at the response-wrap layer (`apply_cors_headers`, `http.rs:1157`), which wraps the return of `handle_request`, so the new arm gets CORS automatically. **Task 1's projector-route fix** is the only manifest/registration change: storage's `/api/v1/status/projector` has an explicit handler arm (`elohim-storage/src/http.rs:894`) but is ABSENT from storage's `build_manifest()` (verified: only `write-through` and `arc-policy` status routes are declared) — so the doorway RouteRegistry never learns the path is proxiable and a direct doorway request 404s via `classify_dispatch`. The fix is the CLAUDE.md-blessed one-liner: add a `Route::get("/api/v1/status/projector")` entry to storage's `build_manifest()`.

### D7 — Plan D (elevate poller) note
This endpoint is exactly what the future "elevate poller" (spec §10 P2 / opportunity #15) will read to drive detect→recover→verify→elevate. The keys are designed for machine consumption (stable camelCase, null-not-absent for not-yet-computable scalars, empty-array for not-yet-populated collections). Building that poller is OUT of scope here.

### D8 — Out of scope (named follow-on plans)
- **Full Angular stability page** — mount `HealthIndicatorComponent`, add a `caughtUp` chip to `ConnectionIndicator`, a navigable `/shefa/health` route, render-verify with `pnpm look`. Sibling FRONTEND plan. This plan delivers only the agent/UI-consumable ENDPOINT + the typed contract + a one-method service stub.
- **REA actuation surface** (`tune_knob`, `quarantine_peer`, `delegates-compute` grants) — spec §5. This plane is READ-only.
- **Arc-shrink** (separate thread). **Auto preset derivation** / **inbound admission** / **upstream circuit-breakers** — sibling spec plans whose landing wires the reserved keys.
- **The Plan D elevate poller** — consumes this endpoint; not built here.

### D9 — Coordination scar (from prompt)
A parallel thread owns `warm_stream.rs` / conductor config / `target_arc_factor`. This plan reads `WarmupState` ONLY through its existing public atomic fields (the same read `startup_check` already does at `health.rs:359-369`); it does NOT add fields to `WarmupState` or touch the warm-stream task. Selective-stage: commit only the files each task names.

## Canonical type & function names

| Name | Kind | File | Role |
|---|---|---|---|
| `SelfHealingView` | Rust struct (`Serialize`, camelCase) | `doorway/doorway-service/src/routes/self_healing.rs` | top-level read model |
| `AdmissionView` | Rust struct | same | `admission` block (PENDING, emitted `null`) |
| `UpstreamView` | Rust struct | same | one `upstreams[]` entry (PENDING, list empty) |
| `ProjectorView` | Rust struct | same | `projector` block |
| `PeerView` | Rust struct | same | one `peers[]` entry |
| `RenderView` | Rust struct | same | `render` block |
| `WarmupView` | Rust struct | same | `warmup` block |
| `ConductorView` | Rust struct | same | `conductor` block |
| `SelfHealingInputs` | Rust struct (plain, no derive) | same | injected snapshot bundle for the pure composer |
| `compose_self_healing` | Rust free fn | same | `(SelfHealingInputs) -> SelfHealingView` — PURE |
| `handle_self_healing` | Rust async fn | same | `(Arc<AppState>) -> Response` — I/O + calls composer |
| `fetch_projector_status` | Rust async fn | same | on-demand GET of storage `/api/v1/status/projector` |
| `caught_up` / `divergent_anchor` | Rust fields on `P2PHealth` | `doorway/doorway-service/src/routes/health.rs:77` | widened P2P health (Task 2) |
| `StabilityStatusView` | TS interface | `app/elohim-app/src/app/generated/stability-status-view.ts` (generated) | wire contract |
| `SelfHealingService` | Angular service | `app/elohim-app/src/app/elohim/services/self-healing.service.ts` | one-method GET |
| `getSelfHealing` | TS method | same | `(): Observable<StabilityStatusView>` |

## File Structure

| File | New/Modify | Responsibility |
|---|---|---|
| `elohim/elohim-storage/src/http.rs` | Modify (`build_manifest()` ~line 9562) | Task 1: declare `/api/v1/status/projector` so doorway proxies it |
| `doorway/doorway-service/src/routes/health.rs` | Modify (`P2PHealth` ~line 75-84) | Task 2: widen with `caught_up` / `divergent_anchor` |
| `doorway/doorway-service/src/main.rs` | Modify (poller site A ~line 467-472) | Task 2: un-discard projectionReconcile |
| `doorway/doorway-service/src/server/http.rs` | Modify (poller site B ~line 1111-1116; match arm ~line 2340) | Task 2 (site B) + Task 8 (match arm) |
| `doorway/doorway-service/src/routes/self_healing.rs` | **Create** | Tasks 3-7: read-model structs, pure composer, projector fetch, handler |
| `doorway/doorway-service/src/routes/mod.rs` | Modify (~line 36-78) | Task 8: re-export `handle_self_healing` |
| `elohim/sdk/schemas/v1/views/stability-status-view.schema.json` | **Create** | Task 9: JSON Schema (TS contract source) |
| `elohim/sdk/schemas/scripts/codegen-ts.mjs` | Modify (`INTERFACE_FILES` ~line 52-71) | Task 9: register the schema |
| `app/elohim-app/src/app/generated/stability-status-view.ts` | **Generated** (do not hand-edit) | Task 9: emitted TS interface |
| `app/elohim-app/src/app/elohim/services/self-healing.service.ts` | **Create** | Task 10: thin consumer service |
| `app/elohim-app/src/app/elohim/services/index.ts` | Modify (~line 94) | Task 10: barrel export |

## Build / test commands (VERIFIED — C5)

The ambient env is hostile: `RUSTFLAGS=--cfg getrandom_backend="custom"` and `RUSTC_WRAPPER=sccache` are set. BOTH must be overridden for the native doorway build, and a `/tmp` target dir avoids the pool-slot fingerprint ENOENT.

```bash
# Doorway unit tests (filter = inline mod-tests module name, e.g. self_healing)
RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins self_healing 2>&1 | tail -60
# Doorway clippy + fmt gates
RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo clippy -- -D warnings 2>&1 | tail -40
cargo fmt --check
```
Run from `/projects/elohim/doorway/doorway-service`. Plain `cargo test` (NOT nextest here). Never `&&`-chain a gate whose exit code you read — pipe to `tail`.

```bash
# Storage build (Task 1) — Holochain WASM workspace KEEPS the custom flag, default target dir
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib build_manifest_includes 2>&1 | tail -40
```

```bash
# TS codegen + Angular (Tasks 9-10) — from repo root
pnpm run schema:codegen:ts
cd /projects/elohim/app/elohim-app && pnpm exec vitest run --config vite.config.ts self-healing.service
pnpm run lint
```

---

## Task 1: Register `/api/v1/status/projector` in storage build_manifest (reachability fix)

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs:9562-9576` (the `/api/v1/status` block in `build_manifest()`)
- Test: `elohim/elohim-storage/src/http.rs` (inline `mod tests`, near the existing `build_manifest_includes_auth_me_route` test at ~line 12272)

- [ ] **Step 1: Write the failing test**

Add inside the inline `#[cfg(test)] mod tests` in `elohim/elohim-storage/src/http.rs` (mirror the existing `build_manifest_includes_auth_me_route` test):

```rust
    /// Verify build_manifest() declares /api/v1/status/projector so doorway's
    /// RouteRegistry learns it is proxiable (the path has an explicit storage
    /// handler arm but was absent from the manifest → doorway 404 — wf1 fix).
    #[test]
    fn build_manifest_includes_projector_status_route() {
        let manifest = build_manifest();
        let found = manifest
            .routes
            .iter()
            .any(|r| r.method == "GET" && r.path == "/api/v1/status/projector");
        assert!(
            found,
            "GET /api/v1/status/projector missing from build_manifest — doorway \
             cannot proxy projector lag/caughtUp to the stability surface"
        );
    }
```

(If the `DoorwayRoutes` field/`Route` field names differ from `manifest.routes` / `r.method` / `r.path`, match the access pattern used in `build_manifest_includes_auth_me_route` at line ~12272 — read it first.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib build_manifest_includes_projector_status_route 2>&1 | tail -40`
Expected: FAIL — assertion `GET /api/v1/status/projector missing from build_manifest`.

- [ ] **Step 3: Write minimal implementation**

In `build_manifest()`, immediately after the `arc-policy` route block (`http.rs:9571-9576`), add:

```rust
        // Projector cursor + reconciliation-lag (operational, Cat-C). The
        // path has an explicit handler arm above (http.rs:894) but must ALSO
        // be declared here so a doorway RouteRegistry learns it is proxiable —
        // single-target, as always. Consumed by /admin/self-healing.
        .route(
            Route::get("/api/v1/status/projector")
                .handler("projector_status")
                .cache_ttl(5)
                .build(),
        )
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib build_manifest_includes_projector_status_route 2>&1 | tail -40`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim && git add elohim/elohim-storage/src/http.rs
git commit -m "fix(storage): declare /api/v1/status/projector in build_manifest (doorway reachability)

$(printf 'Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>')"
```

---

## Task 2: Plumb caughtUp / divergentAnchor through doorway P2PHealth into /health

**Files:**
- Modify: `doorway/doorway-service/src/routes/health.rs:75-84` (`P2PHealth` struct)
- Modify: `doorway/doorway-service/src/main.rs:467-472` (poller site A)
- Modify: `doorway/doorway-service/src/server/http.rs:1111-1116` (poller site B — the SECOND writer to the same lock)
- Test: `doorway/doorway-service/src/routes/health.rs` (inline `mod tests`)

> Source shape (LANDED): storage `/p2p/status` emits `projectionReconcile` (a `ProjectionReconcileStatus`, `elohim-storage/src/p2p/projection_reconcile.rs`) with `caughtUp: bool` and `divergentAnchor: usize`, OR `null` when the reconcile task is not spawned. Both doorway pollers parse `/p2p/status` into `P2PHealth` and drop this block. We widen the struct and BOTH pollers.

- [ ] **Step 1: Write the failing test**

Add to the inline `#[cfg(test)] mod tests` in `health.rs`:

```rust
    #[test]
    fn p2p_health_carries_reconcile_caught_up_and_divergent_anchor() {
        let h = P2PHealth {
            enabled: true,
            peer_count: 2,
            peer_id: Some("p".to_string()),
            caught_up: Some(true),
            divergent_anchor: Some(0),
        };
        let json = serde_json::to_value(&h).unwrap();
        assert_eq!(json["caughtUp"], serde_json::json!(true));
        assert_eq!(json["divergentAnchor"], serde_json::json!(0));
    }

    #[test]
    fn p2p_health_omits_reconcile_fields_when_absent() {
        let h = P2PHealth {
            enabled: true,
            peer_count: 0,
            peer_id: None,
            caught_up: None,
            divergent_anchor: None,
        };
        let json = serde_json::to_value(&h).unwrap();
        assert!(json.get("caughtUp").is_none(), "caughtUp must be omitted when None");
        assert!(json.get("divergentAnchor").is_none());
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins p2p_health_carries 2>&1 | tail -40`
Expected: FAIL — `struct P2PHealth has no field named caught_up`.

- [ ] **Step 3: Write minimal implementation**

Widen the struct in `health.rs:75-84`:

```rust
/// P2P network health from elohim-storage sidecar
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct P2PHealth {
    /// Whether P2P networking is enabled
    pub enabled: bool,
    /// Number of connected P2P peers
    pub peer_count: usize,
    /// Local peer ID
    pub peer_id: Option<String>,
    /// Projection-reconcile caught-up flag (from storage projectionReconcile).
    /// None when storage's reconcile task is not spawned (block is null).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caught_up: Option<bool>,
    /// Count of anchors that diverged during reconcile (from projectionReconcile).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub divergent_anchor: Option<usize>,
}
```

In `main.rs:467-472` (poller site A) replace the `P2PHealth { ... }` literal:

```rust
                            let recon = &status["projectionReconcile"];
                            let health = doorway::routes::health::P2PHealth {
                                enabled: true,
                                peer_count: status["connectedPeers"].as_u64().unwrap_or(0) as usize,
                                peer_id: status["peerId"].as_str().map(|s| s.to_string()),
                                caught_up: recon["caughtUp"].as_bool(),
                                divergent_anchor: recon["divergentAnchor"].as_u64().map(|n| n as usize),
                            };
```

In `server/http.rs:1111-1116` (poller site B) replace its `P2PHealth { ... }` literal identically (note this poller binds the JSON to `body`, not `status`):

```rust
                            let recon = &body["projectionReconcile"];
                            let health = crate::routes::health::P2PHealth {
                                enabled: true,
                                peer_count: body["connectedPeers"].as_u64().unwrap_or(0) as usize,
                                peer_id: body["peerId"].as_str().map(String::from),
                                caught_up: recon["caughtUp"].as_bool(),
                                divergent_anchor: recon["divergentAnchor"].as_u64().map(|n| n as usize),
                            };
```

(`serde_json::Value` indexing returns `Value::Null` for a missing/null `projectionReconcile`, and `Null["caughtUp"]` is also `Null` whose `.as_bool()` is `None` — so the null-block case yields `None` safely, no panic.)

- [ ] **Step 4: Run test to verify it passes**

Run: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins p2p_health 2>&1 | tail -40`
Expected: PASS (both new tests + existing `liveness_returns_200_even_without_conductor`).

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim && git add doorway/doorway-service/src/routes/health.rs doorway/doorway-service/src/main.rs doorway/doorway-service/src/server/http.rs
git commit -m "feat(doorway): plumb caughtUp/divergentAnchor through P2PHealth (both pollers)

$(printf 'Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>')"
```

---

## Task 3: Read-model structs (SelfHealingView + blocks)

**Files:**
- Create: `doorway/doorway-service/src/routes/self_healing.rs`
- Test: same file (inline `mod tests`)

- [ ] **Step 1: Write the failing test**

Create `self_healing.rs` with ONLY the test module first (so the build fails on the missing types):

```rust
//! `GET /admin/self-healing` — the unified self-healing read model.
//!
//! Cat C node-local Operational state (spec §6): NO DHT entry, NO table, NO
//! coordinator fn — a fresh projection composed at request time from in-process
//! AppState snapshots + one on-demand fetch of storage's projector status. Fails
//! the swap test by design (a node serves its OWN runtime state). This is the
//! same legitimate doorway-local class as /admin/capability and /admin/render-stats.
//!
//! Agent-consumable: plain HTTP JSON, camelCase, stable keys. PENDING sibling
//! fields (autoPreset, admission, upstreams) are emitted as null/empty with a
//! `// FOLLOW-ON` seam so each sibling plan's landing is a one-line wire-up.

use serde::Serialize;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_serializes_with_stable_camelcase_keys() {
        let view = SelfHealingView {
            auto_preset: None,
            admission: None,
            upstreams: vec![],
            projector: ProjectorView {
                lag_seconds: Some(0),
                caught_up: Some(true),
                divergent_anchor: Some(0),
            },
            peers: vec![],
            render: RenderView {
                total: 0,
                degenerate_rate: 0.0,
            },
            warmup: WarmupView {
                in_progress: false,
                attempts: 0,
                completed: true,
                last_error: None,
            },
            conductor: ConductorView {
                connected: true,
                connected_workers: 1,
                total_workers: 1,
            },
        };
        let json = serde_json::to_value(&view).unwrap();
        // Reserved PENDING keys are present and null/empty, never absent/faked.
        assert!(json.get("autoPreset").is_some(), "autoPreset key must be present");
        assert_eq!(json["autoPreset"], serde_json::Value::Null);
        assert_eq!(json["admission"], serde_json::Value::Null);
        assert_eq!(json["upstreams"], serde_json::json!([]));
        // LANDED scalars present and camelCase.
        assert_eq!(json["projector"]["caughtUp"], serde_json::json!(true));
        assert_eq!(json["projector"]["divergentAnchor"], serde_json::json!(0));
        assert_eq!(json["render"]["degenerateRate"], serde_json::json!(0.0));
        assert_eq!(json["warmup"]["inProgress"], serde_json::json!(false));
        assert_eq!(json["conductor"]["connectedWorkers"], serde_json::json!(1));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins self_healing 2>&1 | tail -40`
Expected: FAIL — `cannot find type SelfHealingView in this scope` (the module must also be wired in Task 8 to compile; for this task, add `pub mod self_healing;` to `routes/mod.rs` BEFORE running — see note). To run this task in isolation, temporarily declare the module: add `pub mod self_healing;` near the other `pub mod` lines in `routes/mod.rs` (the re-export of the handler comes in Task 8).

- [ ] **Step 3: Write minimal implementation**

Insert ABOVE the `#[cfg(test)]` block in `self_healing.rs`:

```rust
/// Top-level self-healing read model. Cat C node-local. Keys are STABLE for
/// machine consumption — null (not absent) for not-yet-computable scalars,
/// empty array for not-yet-populated collections.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelfHealingView {
    /// Resource snapshot + derived Auto config + reasons.
    /// FOLLOW-ON: wire when the auto-config plan lands
    /// (AppState.auto_preset via elohim_compute::limits).
    pub auto_preset: Option<serde_json::Value>,
    /// Inbound admission: { maxInflight, available, shedTotal }.
    /// FOLLOW-ON: wire when the inbound-admission sibling plan lands
    /// (doorway accept-loop semaphore — server/http.rs:1129 is unbounded today).
    pub admission: Option<AdmissionView>,
    /// Per-upstream circuit/health state.
    /// FOLLOW-ON: populate when the upstream-self-protection sibling plan lands
    /// (WarmStreamHealth + UpstreamBreakers / PeerHealthRegistry::is_open()).
    pub upstreams: Vec<UpstreamView>,
    pub projector: ProjectorView,
    pub peers: Vec<PeerView>,
    pub render: RenderView,
    pub warmup: WarmupView,
    pub conductor: ConductorView,
}

/// PENDING (inbound-admission sibling). Shape reserved for forward-compat.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdmissionView {
    pub max_inflight: usize,
    pub available: usize,
    pub shed_total: u64,
}

/// PENDING (upstream-self-protection sibling). One upstream entry.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpstreamView {
    pub endpoint: String,
    /// "closed" | "half-open" | "open"
    pub circuit: String,
    pub error_streak: u32,
    pub last_good: Option<String>,
    pub skipped: bool,
}

/// Projector lag + reconcile caught-up state (LANDED).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectorView {
    /// Max reconciliation lag in seconds across (pillar, kind); None when not computable.
    pub lag_seconds: Option<i64>,
    /// None when storage's reconcile task is not spawned (projectionReconcile null).
    pub caught_up: Option<bool>,
    pub divergent_anchor: Option<usize>,
}

/// One peer entry (LANDED — from PeerHealthRegistry::snapshot()).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerView {
    pub peer: String,
    /// "Healthy" | "Degraded" | "Offline" (serialized ServiceHealth value).
    pub status: String,
    pub last_seen: Option<String>,
}

/// Render reliability proxy for projector/render health (LANDED).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderView {
    pub total: u64,
    pub degenerate_rate: f64,
}

/// Warmup progress (LANDED — WarmupState atoms, read-only).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WarmupView {
    pub in_progress: bool,
    pub attempts: u32,
    pub completed: bool,
    pub last_error: Option<String>,
}

/// Conductor health (LANDED — same source as /health ConductorHealth).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConductorView {
    pub connected: bool,
    pub connected_workers: usize,
    pub total_workers: usize,
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins self_healing 2>&1 | tail -40`
Expected: PASS — `view_serializes_with_stable_camelcase_keys`.

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim && git add doorway/doorway-service/src/routes/self_healing.rs doorway/doorway-service/src/routes/mod.rs
git commit -m "feat(doorway): SelfHealingView read-model structs (Cat C, PENDING seams reserved)

$(printf 'Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>')"
```

---

## Task 4: Pure composer — SelfHealingInputs → SelfHealingView

**Files:**
- Modify: `doorway/doorway-service/src/routes/self_healing.rs`
- Test: same file (inline `mod tests`)

> The composer is PURE: it takes already-collected snapshots (NO `AppState`, NO I/O), so it is testable with hand-built fixtures and a sibling plan's landing only changes the inputs the handler injects, not the composer.

- [ ] **Step 1: Write the failing test**

Add to `mod tests`:

```rust
    use elohim_compute::peers::PeerHealthSnapshot;
    use elohim_compute::ServiceHealth;
    use elohim_render::stats::RenderTraceSnapshot;

    fn sample_inputs() -> SelfHealingInputs {
        SelfHealingInputs {
            projector_lag_seconds: Some(7),
            p2p_caught_up: Some(false),
            p2p_divergent_anchor: Some(2),
            peers: vec![PeerHealthSnapshot {
                peer_id: "peer-a".to_string(),
                address: "addr".to_string(),
                health: ServiceHealth::Degraded,
                reason: "reconnecting".to_string(),
                signals_received: 3,
                last_signal_at: None,
                reconnect_attempts: 1,
            }],
            render: RenderTraceSnapshot::default(),
            warmup: Some((false, 4u32, true, Some("boom".to_string()))),
            conductor: (true, 2, 4),
        }
    }

    #[test]
    fn compose_maps_landed_fields_and_reserves_pending() {
        let view = compose_self_healing(sample_inputs());
        // PENDING reserved
        assert!(view.auto_preset.is_none());
        assert!(view.admission.is_none());
        assert!(view.upstreams.is_empty());
        // LANDED projector
        assert_eq!(view.projector.lag_seconds, Some(7));
        assert_eq!(view.projector.caught_up, Some(false));
        assert_eq!(view.projector.divergent_anchor, Some(2));
        // LANDED peers — ServiceHealth maps to its string form
        assert_eq!(view.peers.len(), 1);
        assert_eq!(view.peers[0].peer, "peer-a");
        assert_eq!(view.peers[0].status, "Degraded");
        // LANDED warmup
        assert_eq!(view.warmup.attempts, 4);
        assert_eq!(view.warmup.last_error.as_deref(), Some("boom"));
        // LANDED conductor
        assert_eq!(view.conductor.connected_workers, 2);
        assert_eq!(view.conductor.total_workers, 4);
    }

    #[test]
    fn compose_handles_absent_warmup() {
        let mut inputs = sample_inputs();
        inputs.warmup = None;
        let view = compose_self_healing(inputs);
        // No warmup task → safe defaults, not a panic.
        assert!(!view.warmup.in_progress);
        assert_eq!(view.warmup.attempts, 0);
        assert!(view.warmup.last_error.is_none());
    }
```

(Verify the import paths `elohim_compute::peers::PeerHealthSnapshot`, `elohim_compute::ServiceHealth`, `elohim_render::stats::RenderTraceSnapshot` against the crate re-exports — `PeerHealthSnapshot` is at `elohim/elohim-compute/src/peers.rs:19`, `ServiceHealth` is re-exported at the `elohim_compute` crate root per `peers.rs:15 use crate::ServiceHealth`, `RenderTraceSnapshot` is at `elohim/elohim-render/src/stats.rs:41`. Adjust the path if a re-export shortens it; `RenderTraceSnapshot` must `derive(Default)` for `::default()` — it does per C1's quote of stats.rs.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins self_healing 2>&1 | tail -40`
Expected: FAIL — `cannot find function compose_self_healing` / `cannot find type SelfHealingInputs`.

- [ ] **Step 3: Write minimal implementation**

Add to `self_healing.rs` (above the test module). Map `ServiceHealth` to a string via `format!("{:?}", ...)` (the enum derives Debug; `Healthy`/`Degraded`/`Offline` are the C2-quoted variants):

```rust
use elohim_compute::peers::PeerHealthSnapshot;
use elohim_render::stats::RenderTraceSnapshot;

/// Already-collected component snapshots injected into the PURE composer.
/// Collected by the handler (Task 6); plain data so the composer does no I/O.
pub struct SelfHealingInputs {
    pub projector_lag_seconds: Option<i64>,
    pub p2p_caught_up: Option<bool>,
    pub p2p_divergent_anchor: Option<usize>,
    pub peers: Vec<PeerHealthSnapshot>,
    pub render: RenderTraceSnapshot,
    /// (in_progress, attempts, completed, last_error) — None when no warmup task.
    pub warmup: Option<(bool, u32, bool, Option<String>)>,
    /// (connected, connected_workers, total_workers)
    pub conductor: (bool, usize, usize),
}

/// PURE: compose the read model from injected snapshots. No I/O, no AppState.
/// PENDING sibling fields are reserved (None / empty) here — a sibling's
/// landing changes only the inputs the handler injects, never this function.
pub fn compose_self_healing(inputs: SelfHealingInputs) -> SelfHealingView {
    let peers = inputs
        .peers
        .into_iter()
        .map(|p| PeerView {
            peer: p.peer_id,
            status: format!("{:?}", p.health),
            last_seen: p.last_signal_at.map(|t| t.to_rfc3339()),
        })
        .collect();

    let warmup = match inputs.warmup {
        Some((in_progress, attempts, completed, last_error)) => WarmupView {
            in_progress,
            attempts,
            completed,
            last_error,
        },
        None => WarmupView {
            in_progress: false,
            attempts: 0,
            completed: false,
            last_error: None,
        },
    };

    let (connected, connected_workers, total_workers) = inputs.conductor;

    SelfHealingView {
        // FOLLOW-ON: auto-config sibling sets this from AppState.auto_preset.
        auto_preset: None,
        // FOLLOW-ON: inbound-admission sibling sets this from the accept-loop semaphore.
        admission: None,
        // FOLLOW-ON: upstream-self-protection sibling populates this.
        upstreams: Vec::new(),
        projector: ProjectorView {
            lag_seconds: inputs.projector_lag_seconds,
            caught_up: inputs.p2p_caught_up,
            divergent_anchor: inputs.p2p_divergent_anchor,
        },
        peers,
        render: RenderView {
            total: inputs.render.total,
            degenerate_rate: inputs.render.degenerate_rate,
        },
        warmup,
        conductor: ConductorView {
            connected,
            connected_workers,
            total_workers,
        },
    }
}
```

(If `RenderTraceSnapshot.total` is typed `usize` not `u64`, change `RenderView.total` to match and cast in the test — read `stats.rs:41-52`. C1 quotes `total` and `degenerate_rate: f64`.)

- [ ] **Step 4: Run test to verify it passes**

Run: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins self_healing 2>&1 | tail -40`
Expected: PASS — `compose_maps_landed_fields_and_reserves_pending` + `compose_handles_absent_warmup`.

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim && git add doorway/doorway-service/src/routes/self_healing.rs
git commit -m "feat(doorway): pure compose_self_healing(SelfHealingInputs) composer

$(printf 'Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>')"
```

---

## Task 5: On-demand storage projector fetch (fetch_projector_status)

**Files:**
- Modify: `doorway/doorway-service/src/routes/self_healing.rs`
- Test: same file (inline `mod tests`)

> No AppState cache holds projector lag — the handler fetches it on demand from storage (`state.ssr_http_client` + `state.args.storage_url`), mirroring `query_storage_p2p_status` (`federation.rs:277`). `fetch_projector_status` parses the response into `Option<i64>` (max lagSeconds) and is fault-tolerant: any error → `None`, never fails the aggregate.

- [ ] **Step 1: Write the failing test**

Add a pure-parse helper test (the network call itself is exercised by the handler smoke test in Task 7; here we test the parse of a known body):

```rust
    #[test]
    fn parse_projector_lag_takes_max_across_kinds() {
        let body = serde_json::json!({
            "cursors": [],
            "lag": [
                { "pillar": "lamad", "kind": "content", "lagSeconds": 3 },
                { "pillar": "lamad", "kind": "path", "lagSeconds": 11 },
                { "pillar": "mishpat", "kind": "commitment", "lagSeconds": null }
            ]
        });
        assert_eq!(parse_projector_lag(&body), Some(11));
    }

    #[test]
    fn parse_projector_lag_none_when_no_lag_entries() {
        let body = serde_json::json!({ "cursors": [], "lag": [] });
        assert_eq!(parse_projector_lag(&body), None);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins self_healing 2>&1 | tail -40`
Expected: FAIL — `cannot find function parse_projector_lag`.

- [ ] **Step 3: Write minimal implementation**

Add to `self_healing.rs`:

```rust
use std::sync::Arc;

/// Parse the max `lagSeconds` across all (pillar, kind) lag entries in a
/// ProjectorStatusView JSON body. None when there are no numeric lag entries.
fn parse_projector_lag(body: &serde_json::Value) -> Option<i64> {
    body["lag"]
        .as_array()?
        .iter()
        .filter_map(|e| e["lagSeconds"].as_i64())
        .max()
}

/// Fetch storage's /api/v1/status/projector and return the max lagSeconds.
/// Fault-tolerant: any error (storage down, parse fail, no URL) → None. NEVER
/// fails the aggregate — the stability surface degrades a field, not the call.
async fn fetch_projector_status(state: &Arc<crate::server::AppState>) -> Option<i64> {
    let base = state.args.storage_url.as_ref()?;
    let url = format!("{}/api/v1/status/projector", base.trim_end_matches('/'));
    let resp = state.ssr_http_client.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let body: serde_json::Value = resp.json().await.ok()?;
    parse_projector_lag(&body)
}
```

(Confirm `crate::server::AppState` is the re-export path — C5 says `AppState` is re-exported as `crate::server::AppState`. `state.ssr_http_client` is `Arc<reqwest::Client>` (`http.rs:215`); `state.args.storage_url` is `Option<String>`.)

- [ ] **Step 4: Run test to verify it passes**

Run: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins self_healing 2>&1 | tail -40`
Expected: PASS — both parse tests.

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim && git add doorway/doorway-service/src/routes/self_healing.rs
git commit -m "feat(doorway): fault-tolerant fetch_projector_status + lag parse

$(printf 'Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>')"
```

---

## Task 6: Handler — handle_self_healing (I/O + compose + JSON response)

**Files:**
- Modify: `doorway/doorway-service/src/routes/self_healing.rs`
- Test: covered by Task 7 (smoke test via `test_state()`)

> The handler does the I/O: read AppState snapshots, read the cached `p2p_health` non-blocking, fetch projector lag, then call the pure composer and serialize. It reuses the `json_response`-style serialize idiom (mirror `admin.rs:990`).

- [ ] **Step 1: Write the failing test**

(No new test here — the failing test is Task 7's smoke test. To keep TDD honest, write Task 7's test FIRST if executing strictly; this task supplies the impl it needs. For granularity, implement the handler now and verify compile.)

- [ ] **Step 2: Run to verify it fails (compile)**

Run: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo build --lib 2>&1 | tail -20`
Expected (before impl, with Task 7's test present): FAIL — `cannot find function handle_self_healing`.

- [ ] **Step 3: Write minimal implementation**

Add to `self_healing.rs`:

```rust
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Response, StatusCode};

/// Handle `GET /admin/self-healing`. Composes the Cat-C node-local read model
/// from in-process snapshots + one on-demand storage projector fetch. No auth
/// (in-cluster reads, same rationale as /admin/capability; operator-only is an
/// ingress property, not enforced here).
pub async fn handle_self_healing(
    state: Arc<crate::server::AppState>,
) -> Response<Full<Bytes>> {
    // Non-blocking read of the cached P2P health (carries caughtUp/divergentAnchor
    // after Task 2). try_read so a held write-lock never stalls the read model.
    let (p2p_caught_up, p2p_divergent_anchor) = match state.p2p_health.try_read() {
        Ok(guard) => match guard.as_ref() {
            Some(h) => (h.caught_up, h.divergent_anchor),
            None => (None, None),
        },
        Err(_) => (None, None),
    };

    let peers = state.peer_health.snapshot();
    let render = state.render_trace_stats.snapshot();

    let warmup = state.warmup_state.as_ref().map(|ws| {
        use std::sync::atomic::Ordering::Relaxed;
        (
            ws.in_progress.load(Relaxed),
            ws.attempts.load(Relaxed),
            ws.completed.load(Relaxed),
            ws.last_error.lock().ok().and_then(|g| g.clone()),
        )
    });

    let conductor = match &state.pool {
        Some(pool) => (pool.is_healthy(), pool.connected_count(), pool.worker_count()),
        None => (false, 0, 0),
    };

    let projector_lag_seconds = fetch_projector_status(&state).await;

    let view = compose_self_healing(SelfHealingInputs {
        projector_lag_seconds,
        p2p_caught_up,
        p2p_divergent_anchor,
        peers,
        render,
        warmup,
        conductor,
    });

    match serde_json::to_string_pretty(&view) {
        Ok(json) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .header("Cache-Control", "no-store")
            .body(Full::new(Bytes::from(json)))
            .unwrap(),
        Err(_) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Full::new(Bytes::from("Failed to serialize self-healing view")))
            .unwrap(),
    }
}
```

(Verify: `state.pool` exposes `is_healthy()`/`connected_count()`/`worker_count()` — these are the exact methods `build_health_response` uses at `health.rs:122-126`. `state.warmup_state.last_error` is `std::sync::Mutex<Option<String>>` per warm_stream.rs:236-243; `.lock().ok()` guards against poison without panic. Imports `http_body_util::Full`, `hyper::body::Bytes`, `hyper::{Response, StatusCode}` mirror `admin.rs`/`federation.rs` — copy their `use` lines if names differ.)

- [ ] **Step 4: Run to verify it compiles**

Run: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo build --lib 2>&1 | tail -20`
Expected: builds (handler present; Task 8 wires the route; Task 7 adds the smoke test).

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim && git add doorway/doorway-service/src/routes/self_healing.rs
git commit -m "feat(doorway): handle_self_healing handler (snapshots + projector fetch + compose)

$(printf 'Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>')"
```

---

## Task 7: Handler smoke test (test_state)

**Files:**
- Modify: `doorway/doorway-service/src/routes/self_healing.rs` (inline `mod tests`)

> `AppState::new(args)` is synchronous + DB-free (`mongo: None`, no pool, no warmup) — the canonical `test_state()` pattern (`health.rs:293`). With no storage URL, `fetch_projector_status` returns `None`; with no pool, conductor is `(false, 0, 0)`; with no warmup task, warmup defaults apply. The handler must return 200 with valid JSON carrying all reserved keys.

- [ ] **Step 1: Write the failing test**

Add to `mod tests`:

```rust
    use crate::config::Args;
    use crate::server::AppState;
    use clap::Parser;

    fn test_state() -> AppState {
        let args = Args::parse_from(["doorway", "--listen", "127.0.0.1:0"]);
        AppState::new(args)
    }

    #[tokio::test]
    async fn handler_returns_200_with_reserved_keys_on_bare_state() {
        let state = Arc::new(test_state());
        let resp = handle_self_healing(state).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body_bytes = http_body_util::BodyExt::collect(resp.into_body())
            .await
            .unwrap()
            .to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        // Reserved PENDING keys present (null/empty), LANDED blocks present.
        assert!(json.get("autoPreset").is_some());
        assert_eq!(json["autoPreset"], serde_json::Value::Null);
        assert_eq!(json["admission"], serde_json::Value::Null);
        assert_eq!(json["upstreams"], serde_json::json!([]));
        assert!(json.get("projector").is_some());
        assert!(json.get("peers").is_some());
        assert!(json.get("render").is_some());
        assert!(json.get("warmup").is_some());
        assert_eq!(json["conductor"]["connected"], serde_json::json!(false));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins handler_returns_200 2>&1 | tail -40`
Expected: it should PASS if Task 6 landed (handler exists). If executing strictly TDD, this test was the failing driver for Task 6 — run it after Task 6's impl. (If `#[tokio::test]` needs the `macros` feature, it is already enabled — other async handler tests exist in the crate.)

- [ ] **Step 3: (impl already present from Task 6)**

No new impl. If the test fails on a missing `tokio` test macro, confirm `tokio` dev-deps carry `macros` + `rt` (they do — async tests exist crate-wide).

- [ ] **Step 4: Run test to verify it passes**

Run: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins self_healing 2>&1 | tail -40`
Expected: PASS — all `self_healing` tests green.

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim && git add doorway/doorway-service/src/routes/self_healing.rs
git commit -m "test(doorway): handle_self_healing 200 + reserved-keys smoke test

$(printf 'Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>')"
```

---

## Task 8: Wire the route — match arm + mod re-export

**Files:**
- Modify: `doorway/doorway-service/src/routes/mod.rs:36-78` (add `handle_self_healing` to a `pub use` block)
- Modify: `doorway/doorway-service/src/server/http.rs:2340` (add match arm after `/admin/render-stats`)

- [ ] **Step 1: Write the failing test**

Add to `routes/mod.rs` an inline compile-canary OR rely on the dispatch wiring being exercised by a new test in `http.rs`'s inline tests. Simplest: add a focused dispatch test in `self_healing.rs` is not possible (dispatch is in `http.rs`). Instead, add to the inline `mod tests` in `server/http.rs` (mirror existing dispatch tests if present), OR verify via build + a curl-shaped assertion. Use this minimal mod-level test in `routes/mod.rs` is not idiomatic; instead assert the re-export compiles by referencing it in the handler smoke test path. **Concrete failing step:** before editing, the match arm does not exist, so add the arm-presence as a build requirement — write the arm's caller reference test in `http.rs` tests:

```rust
    // in server/http.rs inline mod tests
    #[test]
    fn self_healing_handler_is_reexported() {
        // Compile-time canary: the route module's handler is reachable via the
        // routes facade exactly as the match arm calls it.
        let _f: fn(
            std::sync::Arc<crate::server::AppState>,
        ) -> _ = crate::routes::handle_self_healing;
    }
```

(If the `fn ... -> _` placeholder return is rejected by the compiler, simplify to `let _ = crate::routes::handle_self_healing;` which still fails to resolve until the re-export exists.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins self_healing_handler_is_reexported 2>&1 | tail -30`
Expected: FAIL — `cannot find function handle_self_healing in module crate::routes`.

- [ ] **Step 3: Write minimal implementation**

In `routes/mod.rs`, ensure the module is declared (`pub mod self_healing;` near the other `pub mod` lines — added in Task 3) and add to a `pub use` line (mirror the `handle_admin_capability, handle_admin_render_stats` re-export at line 36-37):

```rust
pub use self_healing::handle_self_healing;
```

In `server/http.rs`, add the match arm immediately after the `/admin/render-stats` arm (`http.rs:2336-2339`):

```rust
        // Unified self-healing read model — Cat C node-local Operational state
        // (2026-06-13-self-healing-control-plane-design.md §6). Composed
        // fresh per request from in-process snapshots + on-demand projector
        // fetch. Legitimate doorway-local aggregate (NOT a per-domain proxy):
        // fails the swap test by design — a node serves its OWN runtime state,
        // same class as /admin/capability + /admin/render-stats. No auth
        // (operator-only is an ingress property). Reserved keys for PENDING
        // sibling signals (autoPreset/admission/upstreams) are null/empty.
        (Method::GET, "/admin/self-healing") => {
            to_boxed(routes::handle_self_healing(Arc::clone(&state)).await)
        }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins self_healing 2>&1 | tail -40`
Expected: PASS — re-export canary + all `self_healing` tests. Then run the full gate:
`RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo clippy -- -D warnings 2>&1 | tail -40` (expect no warnings) and `cargo fmt --check` (expect clean).

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim && git add doorway/doorway-service/src/routes/mod.rs doorway/doorway-service/src/server/http.rs
git commit -m "feat(doorway): wire GET /admin/self-healing route + handler re-export

$(printf 'Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>')"
```

---

## Task 9: JSON Schema + TS codegen for the wire contract

**Files:**
- Create: `elohim/sdk/schemas/v1/views/stability-status-view.schema.json`
- Modify: `elohim/sdk/schemas/scripts/codegen-ts.mjs:52-71` (`INTERFACE_FILES`)
- Generated (do not hand-edit): `app/elohim-app/src/app/generated/stability-status-view.ts` (+ 5 sibling dirs)

> The schema is the TS contract source (doorway has no ts-rs). Keys MUST match the Rust `SelfHealingView` camelCase output exactly (D4). PENDING blocks are nullable; collections default empty. Mirror the `p2p-status-view.schema.json` style (source-of-truth annotation, `additionalProperties`).

- [ ] **Step 1: Write the schema (the "test" is codegen freshness + tsc)**

Create `elohim/sdk/schemas/v1/views/stability-status-view.schema.json`:

```json
{
  "$id": "epr:schema:view:stability-status-view",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "StabilityStatusView",
  "description": "Unified self-healing read model served at doorway GET /admin/self-healing. Source of truth: doorway in-process runtime state + storage projector status (Operational, Category C — node-local, never notarized). A node serves its OWN state.",
  "type": "object",
  "additionalProperties": false,
  "required": ["upstreams", "projector", "peers", "render", "warmup", "conductor"],
  "properties": {
    "autoPreset": {
      "description": "Resource snapshot + derived Auto config + reasons. Null until the auto-config sibling plan lands.",
      "type": ["object", "null"]
    },
    "admission": {
      "description": "Inbound admission state. Null until the inbound-admission sibling plan lands.",
      "type": ["object", "null"],
      "additionalProperties": false,
      "properties": {
        "maxInflight": { "type": "integer" },
        "available": { "type": "integer" },
        "shedTotal": { "type": "integer" }
      },
      "required": ["maxInflight", "available", "shedTotal"]
    },
    "upstreams": {
      "description": "Per-upstream circuit/health. Empty until the upstream-self-protection sibling plan lands.",
      "type": "array",
      "items": {
        "type": "object",
        "additionalProperties": false,
        "properties": {
          "endpoint": { "type": "string" },
          "circuit": { "type": "string", "enum": ["closed", "half-open", "open"] },
          "errorStreak": { "type": "integer" },
          "lastGood": { "type": ["string", "null"] },
          "skipped": { "type": "boolean" }
        },
        "required": ["endpoint", "circuit", "errorStreak", "lastGood", "skipped"]
      }
    },
    "projector": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "lagSeconds": { "type": ["integer", "null"] },
        "caughtUp": { "type": ["boolean", "null"] },
        "divergentAnchor": { "type": ["integer", "null"] }
      },
      "required": ["lagSeconds", "caughtUp", "divergentAnchor"]
    },
    "peers": {
      "type": "array",
      "items": {
        "type": "object",
        "additionalProperties": false,
        "properties": {
          "peer": { "type": "string" },
          "status": { "type": "string" },
          "lastSeen": { "type": ["string", "null"] }
        },
        "required": ["peer", "status", "lastSeen"]
      }
    },
    "render": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "total": { "type": "integer" },
        "degenerateRate": { "type": "number" }
      },
      "required": ["total", "degenerateRate"]
    },
    "warmup": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "inProgress": { "type": "boolean" },
        "attempts": { "type": "integer" },
        "completed": { "type": "boolean" },
        "lastError": { "type": ["string", "null"] }
      },
      "required": ["inProgress", "attempts", "completed", "lastError"]
    },
    "conductor": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "connected": { "type": "boolean" },
        "connectedWorkers": { "type": "integer" },
        "totalWorkers": { "type": "integer" }
      },
      "required": ["connected", "connectedWorkers", "totalWorkers"]
    }
  }
}
```

Register it in `INTERFACE_FILES` (`codegen-ts.mjs`, after the `p2p-status-view` / `replication-status-view` lines ~61-63). The `src` intermediate name derives from the schema basename (`stability-status-view.schema.json` → `stability-status-view.ts`):

```javascript
  { src: 'views/stability-status-view.ts', dest: 'stability-status-view.ts' },
```

- [ ] **Step 2: Run codegen to verify it emits (and fails the freshness gate before regen)**

Run: `cd /projects/elohim && pnpm run schema:codegen:ts 2>&1 | tail -30`
Expected: generates `stability-status-view.ts` into all six `GENERATED_OUTPUT_DIRS` (incl. `app/elohim-app/src/app/generated/`). Then verify the generated interface has the camelCase keys: `grep -n "autoPreset\|caughtUp\|degenerateRate" app/elohim-app/src/app/generated/stability-status-view.ts` — expect hits.

- [ ] **Step 3: (impl is the schema + registration above)**

No further code. Confirm the generated TS compiles in the app context (Task 10's lint/build covers it).

- [ ] **Step 4: Verify freshness gate is satisfied**

Run: `cd /projects/elohim && pnpm run schema:codegen:ts -- --verify 2>&1 | tail -10` (if the flag is supported; otherwise re-run codegen and confirm `git status` shows the generated files added, not drifting).
Expected: no drift reported.

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim && git add elohim/sdk/schemas/v1/views/stability-status-view.schema.json elohim/sdk/schemas/scripts/codegen-ts.mjs app/elohim-app/src/app/generated/stability-status-view.ts genesis/seeder/src/generated/stability-status-view.ts app/elohim-library/projects/elohim-service/src/generated/stability-status-view.ts doorway/doorway-app/src/app/generated/stability-status-view.ts app/lamad/src/generated/stability-status-view.ts app/elohim-library/projects/elohim-identity/src/generated/stability-status-view.ts
git commit -m "feat(schema): stability-status-view contract + TS codegen distribution

$(printf 'Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>')"
```

(If a given output dir does not receive the file — check the actual `GENERATED_OUTPUT_DIRS` list and only `git add` the dirs that codegen wrote. Run `git status` after codegen to see exactly which files changed.)

---

## Task 10: Thin Angular consumer service (NAMED but minimal)

**Files:**
- Create: `app/elohim-app/src/app/elohim/services/self-healing.service.ts`
- Modify: `app/elohim-app/src/app/elohim/services/index.ts:94` (barrel export)
- Test: `app/elohim-app/src/app/elohim/services/self-healing.service.spec.ts`

> One-method service that GETs the read model. Mirrors `AcquisitionService` (HttpClient + StorageClientService base URL). This is the ENDPOINT consumer stub ONLY — mounting any component / building the `/shefa/health` page is the OUT-of-scope sibling frontend plan (D8).

- [ ] **Step 1: Write the failing test**

Create `self-healing.service.spec.ts`:

```typescript
import { HttpClient } from '@angular/common/http';
import { of } from 'rxjs';
import { describe, expect, it, vi } from 'vitest';

import { SelfHealingService } from './self-healing.service';

describe('SelfHealingService', () => {
  it('GETs /admin/self-healing against the storage base URL', async () => {
    const view = {
      autoPreset: null,
      admission: null,
      upstreams: [],
      projector: { lagSeconds: 0, caughtUp: true, divergentAnchor: 0 },
      peers: [],
      render: { total: 0, degenerateRate: 0 },
      warmup: { inProgress: false, attempts: 0, completed: true, lastError: null },
      conductor: { connected: true, connectedWorkers: 1, totalWorkers: 1 },
    };
    const http = { get: vi.fn().mockReturnValue(of(view)) } as unknown as HttpClient;
    const storage = { getStorageBaseUrl: () => 'http://localhost:8888' } as {
      getStorageBaseUrl: () => string;
    };

    const svc = new SelfHealingService(http, storage as never);
    const result = await new Promise(resolve => svc.getSelfHealing().subscribe(resolve));

    expect(http.get).toHaveBeenCalledWith('http://localhost:8888/admin/self-healing');
    expect(result).toEqual(view);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /projects/elohim/app/elohim-app && pnpm exec vitest run --config vite.config.ts self-healing.service 2>&1 | tail -30`
Expected: FAIL — cannot resolve `./self-healing.service`.

- [ ] **Step 3: Write minimal implementation**

Create `self-healing.service.ts`:

```typescript
/**
 * Thin consumer of the doorway self-healing read model (GET /admin/self-healing).
 *
 * The ENDPOINT consumer stub only — it does NOT render. Mounting a stability
 * page / HealthIndicator / a caughtUp chip / the /shefa/health route is the
 * OUT-of-scope sibling FRONTEND plan. This delivers the typed contract + one
 * GET so an agent or a future UI can read the surface.
 *
 * Cat C node-local: a node serves its OWN state. In doorway mode the base URL
 * resolves to the doorway proxy; in direct/Tauri mode to the local sidecar.
 */
import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { type Observable } from 'rxjs';

import { StorageClientService } from './storage-client.service';

import type { StabilityStatusView } from '../../generated/stability-status-view';

@Injectable({ providedIn: 'root' })
export class SelfHealingService {
  private readonly http = inject(HttpClient);
  private readonly storage = inject(StorageClientService);

  // Test-friendly constructor (Angular DI uses inject() above; the explicit
  // params let the spec construct without TestBed).
  constructor(http?: HttpClient, storage?: StorageClientService) {
    if (http) this.http = http;
    if (storage) this.storage = storage;
  }

  /** GET the unified self-healing read model for this node. */
  getSelfHealing(): Observable<StabilityStatusView> {
    const base = this.storage.getStorageBaseUrl();
    return this.http.get<StabilityStatusView>(`${base}/admin/self-healing`);
  }
}
```

(If the lint config forbids the optional-param constructor pattern, drop it and make the spec use `TestBed.configureTestingModule` with `provideHttpClientTesting()` instead — check how `acquisition.service.spec.ts` constructs its instance and match it. The generated type name is `StabilityStatusView` per the schema `title`; confirm the exact exported symbol in the generated file after Task 9.)

- [ ] **Step 4: Run test to verify it passes**

Run: `cd /projects/elohim/app/elohim-app && pnpm exec vitest run --config vite.config.ts self-healing.service 2>&1 | tail -30`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim
# Add the barrel export (mirror line 94 AcquisitionService export):
#   export { SelfHealingService } from './self-healing.service';
git add app/elohim-app/src/app/elohim/services/self-healing.service.ts app/elohim-app/src/app/elohim/services/self-healing.service.spec.ts app/elohim-app/src/app/elohim/services/index.ts
git commit -m "feat(app): SelfHealingService thin consumer of /admin/self-healing

$(printf 'Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>')"
```

(Edit `index.ts` to add `export { SelfHealingService } from './self-healing.service';` next to the `AcquisitionService` export at line 94 before committing.)

---

## Self-Review

**1. Spec coverage (against §6 read model + §3f + §2c + the prompt's IN scope):**
- IN(1) aggregate `/admin/self-healing` composing autoPreset / admission / upstreams / projector{lag,caughtUp,divergentAnchor} / peers — Tasks 3-8 (LANDED fields populated; PENDING reserved null/empty with `// FOLLOW-ON`). ✔
- IN(2) two wf1 backend fixes: register `/api/v1/status/projector` (Task 1 — the CLAUDE.md-blessed `build_manifest()` one-liner, corrected from C3's mistaken "doorway build_manifest" framing) + un-discard projectionReconcile caughtUp/divergentAnchor through `P2PHealth` into `/health` (Task 2, BOTH poller sites). ✔
- IN(3) doorway-local camelCase serde struct + unit-tested PURE aggregation (`compose_self_healing`, Task 4) testable without a live cluster (`SelfHealingInputs` fixtures). ✔
- IN(4) thin Angular consumer: typed `StabilityStatusView` (Task 9) + one-method `SelfHealingService.getSelfHealing()` (Task 10). ✔
- HARD CONSTRAINTS: Cat C (D2, stated in Architecture + match-arm comment); `/health` stays cheap (D1 — aggregate on separate arm; Task 2 only widens a cached struct); SELF-CONTAINED (D3 — LANDED now, PENDING null + FOLLOW-ON, never faked); agent-consumable (D7 — stable camelCase, null-not-absent); reachability (D6 — `is_service_path` already covers `/admin`, Task 1 registers the projector path); coordination scar (D9 — WarmupState read via public atoms only, selective-stage). ✔
- OUT named: full Angular page, REA actuation, arc-shrink, Plan D poller (D8). ✔

**2. Placeholder scan:** No "TBD"/"add error handling"/"similar to Task N". Every code step shows actual code. The two soft spots flagged inline (not placeholders, but verify-then-adjust): (a) `RenderTraceSnapshot.total` integer width (`usize` vs `u64`) — instructed to read `stats.rs:41-52` and match; (b) the Angular constructor/TestBed pattern — instructed to match `acquisition.service.spec.ts`. Both name the exact file to confirm against. The `cites:` fingerprint is `sha256:PENDING` — run `cite-gen` before sealing (semantic-links skill); acceptable in a Draft.

**3. Type consistency:** `SelfHealingView` / `compose_self_healing` / `SelfHealingInputs` / `handle_self_healing` / `fetch_projector_status` / `parse_projector_lag` used identically across Tasks 3-8. Field names match between Rust struct (Task 3), composer (Task 4), JSON schema (Task 9), and TS contract: `autoPreset, admission{maxInflight,available,shedTotal}, upstreams[]{endpoint,circuit,errorStreak,lastGood,skipped}, projector{lagSeconds,caughtUp,divergentAnchor}, peers[]{peer,status,lastSeen}, render{total,degenerateRate}, warmup{inProgress,attempts,completed,lastError}, conductor{connected,connectedWorkers,totalWorkers}`. `P2PHealth.caught_up`/`divergent_anchor` (Task 2) feed `projector.caughtUp`/`divergentAnchor` (Task 6 handler reads them from the cached struct). TS symbol `StabilityStatusView` (schema title) vs Rust `SelfHealingView` (intentional — schema title drives the TS name; documented in Canonical names table). Build/test commands consistent (RUSTFLAGS="" + /tmp target for doorway; custom flag for storage).
