---
id: "backlog-doorway-http-rs-modularization"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Modularize doorway-service server/http.rs — at LoC hard ceiling (7022 lines), ordered seam extraction preserving RouteRegistry wildcard dispatch"
slug: "doorway-http-rs-modularization"
written: "2026-07-10"
author: "rust-architect (architecture finding 53c05bb1c007, operator-requested)"
status: "backlog"
priority: "medium"
tags: [architecture, refactor, doorway, mod-decomposition, code-health, dev-qol]
cites:
  - doorway/doorway-service/src/server/http.rs
  - doorway/doorway-service/src/server/mod.rs
  - doorway/doorway-service/src/server/CLAUDE.md
shift_objective: |
  Decompose doorway-service/src/server/http.rs (7022 lines, at/over the LoC
  hard ceiling) into focused sibling sub-modules under src/server/, extracting
  the natural seams in the ordered sequence below. Each extraction is one
  self-contained, independently-verifiable step: move the item cluster + its
  co-located #[cfg(test)] module to a new sibling file, adjust visibility
  (private fn -> pub(super)/pub(crate)), re-export through http.rs or mod.rs as
  needed, then run the crate's native gates (RUSTFLAGS="" cargo build/test
  --lib --bins + clippy -D warnings + cargo fmt --check) before starting the
  next step. The RouteRegistry wildcard-dispatch discipline (server/CLAUDE.md)
  MUST stay intact: classify_dispatch keeps its unconditional delegation, no
  path-prefix guard is introduced in the wildcard arm. Plain native Rust — no
  DNA-hash, no WASM, no ts-rs concerns. handle_request + run + wiring stay in
  http.rs as the thin dispatch core; everything else moves out.
---

# Modularize doorway-service `server/http.rs`

## Problem

`doorway/doorway-service/src/server/http.rs` is **7022 lines** — at/over the
crate's LoC hard ceiling. It has accreted the entire doorway request lifecycle
into one file: `AppState` + all its constructors, the auth-posture / op-gate
machinery, the admission membrane, the liveness/watchdog subsystem, the EPR
dispatch + SSR render orchestration, sitemap generation, bootstrap/signal/blob
host routing, the response helpers, the `handle_request` dispatch match itself,
and ~1900 lines of co-located `#[cfg(test)]` modules.

The file is still coherent (one clear owner: the HTTP boundary), so this is a
mechanical **seam extraction**, not a redesign. No behavior changes. The goal is
to get `handle_request` + `run` + the `AppState` wiring down to a thin dispatch
core and move each self-contained concern to a sibling module under
`src/server/`.

## Constraints (refactor-safety)

- **Plain native Rust.** Gated only by `RUSTFLAGS="" cargo build/test --lib
  --bins`, `clippy -D warnings`, `cargo fmt --check`. No DNA-hash concern, no
  WASM target, no ts-rs cross-crate import-path trap. This is the low-risk class.
- **RouteRegistry discipline is load-bearing** (`src/server/CLAUDE.md`). The
  wildcard arm of `handle_request` must keep delegating **unconditionally** to
  `classify_dispatch`. Do NOT reintroduce a `p.starts_with("/api/v1/")`-style
  prefix guard while moving the dispatch code — that regression was already paid
  for once. `Disposition` + `classify_dispatch` move together and keep their
  exact semantics.
- **Visibility.** Most extracted items are currently private `fn`. Since every
  target module is a sibling under `server/`, `pub(super)` (or `pub(crate)`
  where `handle_request`/`run` need them) is sufficient — nothing needs to leak
  past the crate.
- **Re-export surface.** `server/mod.rs` currently does
  `pub use http::{run, AppState}`. When `AppState` moves to `server/state.rs`,
  either re-export it through `http.rs` (`pub use crate::server::state::AppState;`)
  or update `mod.rs` to `pub use state::AppState;`. Verify no downstream crate
  imports break (`rg 'server::http::AppState' ; rg 'server::AppState'`).
- **Tests move with their target.** Each `#[cfg(test)] mod …` block relocates to
  the sibling module that owns the code it exercises, so per-module test
  ownership stays clean.
- **One extraction per commit**, path-scoped, each independently green. A killed
  build mid-step is safe to resume; do not batch multiple seams into one step.

## Natural seams (measured from the current file)

| Seam | Items (current line anchors) | Co-located tests | Target module |
|---|---|---|---|
| Response helpers | `to_boxed`, `not_found_response`, `service_unavailable_response`, `bad_request_response` (5302–5356) + `BoxBody` alias | — | `server/responses.rs` |
| Liveness / watchdog | `HEARTBEAT_INTERVAL_MS`, `HEALTH_STALE_MS_DEFAULT`, `health_stale_threshold_ms`, `main_runtime_wedged`, `spawn_liveness_heartbeat`, `watchdog_liveness_response`, `root_unavailable_html`, `root_unavailable_response`, `handle_watchdog_probe`, `spawn_health_listener` (1316–1705) | `root_unavailable_tests`, `watchdog_tests` | `server/liveness.rs` |
| Sitemap | `render_sitemap`, `serve_sitemap`, `sitemap_response`, `fetch_commons_ids`, `fetch_ids_for_reach` (2894–3046) | — | `server/sitemap.rs` |
| Admission membrane | `DOORWAY_ADMISSION_RETRY_AFTER_SECS`, `apply_membrane`, `admission_exempt`, `catching_up_response` (5357–5494) | `admission_tests` | `server/admission.rs` |
| Auth posture / op-gate | `AuthPosture`, `determine_auth_posture`, `build_ssr_user_credential`, `build_chrome_context_json`, `resolve_agent_cid_from_request`, `extract_bearer_from_req`, `AuthorizeVerdict`, `OpGateDecision`, `compute_op_gate_decision`, `NoCredentialDecision`, `no_credential_decision`, `call_authorize_operation`, `make_op_gate_forbidden`, `apply_gate_check`, `infer_gate_event` (1008–1315, 1889–1979) | `op_gate_tests`, `gate_layer_tests` | `server/op_gate.rs` |
| Dispatch classification | `Disposition`, `classify_dispatch` (49–116) + path classifiers `is_service_path`, `is_reserved_url_path`, `is_auth_owned_path`, `is_spa_route_subpath`, `derive_app_subpath`, `AUTH_OWNED_PATHS`, `anon_reach_readable` (1979–2168, 2836–2857) | `shakeout_tests`, `handoff_routing_tests`, `dispatch_classification_tests` | `server/dispatch.rs` |
| EPR dispatch + SSR orchestration | `EPR_DISPATCH_TIMEOUT_SECS`, `epr_dispatch_shed_response`, `dispatch_to_projected_epr`, `epr_universal_root`, `dispatch_epr_universal`, `SsrFallback`, `SsrFallbackReason`, `with_ssr_skipped_header`, `epr_should_serve_ssr`, `render_output_is_empty`, `ssr_fallback_response`, `projected_shell_url`, `compose_render_with_shell`, `serve_ssr_route`, `maybe_inject_chrome`, `ssr_html_response_with_observability`, `ssr_spa_shell_fallback_with_error`, `ssr_spa_shell_fallback_with_skip_reason`, `epr_universal_root` (2425–3696, 5494–5619) | `epr_dispatch_breaker_tests`, `ssr_session_tests`, `epr_universal_tests`, `epr_claims_dispatch_tests` | `server/epr_ssr.rs` (split into `server/epr_dispatch.rs` + `server/render_orchestration.rs` if the single file lands >1500 LoC) |
| Bootstrap / signal / blob host routing | `maybe_contribute_observation`, `handle_k2_bootstrap_put`, `handle_k2_bootstrap_get`, `handle_bootstrap_request`, `handle_signal_request`, `handle_blob_verify` (4961–5302) | — | `server/host_routing.rs` |
| AppState + constructors | `AppState` struct + `init_storage_proxy_client`, `init_ssr_http_client`, `init_renderer`, `impl AppState` (116–1008) | — | `server/state.rs` |
| **Retained core** | `handle_request` (3696–4961), `run` (1705–1889), imports, `BoxBody` (until responses split), module declarations | — | `server/http.rs` (thin) |

## Ordered decomposition (leaf-first, each step independently green)

Ordered so every step depends only on already-extracted or still-in-`http.rs`
code — never on a not-yet-extracted sibling. Leaf helpers first, the big
dispatch/state seams last, `handle_request` never moves.

1. **`server/responses.rs`** — the four response helpers + `BoxBody` alias.
   Zero inbound dependencies, referenced everywhere; extracting first means every
   later module imports a stable `responses::*`. Lowest risk, highest churn-reduction.
2. **`server/liveness.rs`** — health/watchdog/heartbeat/root-unavailable + their
   two test modules. Self-contained subsystem; only touches `AppState` by `&`/`Arc`.
3. **`server/sitemap.rs`** — sitemap render + reach-id fetch helpers. Isolated;
   depends only on `AppState` + `responses`.
4. **`server/admission.rs`** — `apply_membrane` + `admission_exempt` +
   `catching_up_response` + `admission_tests`. The membrane is a clean pre-dispatch
   band; verify `handle_request` still calls it in the same position.
5. **`server/op_gate.rs`** — auth-posture + op-gate + gate-check + two test
   modules. Larger but cohesive; confirm `AuthPosture`/`OpGateMode` re-exports
   satisfy `handle_request`'s call sites.
6. **`server/dispatch.rs`** — `Disposition`, `classify_dispatch`, the path
   classifiers + three test modules. **Discipline gate:** diff `classify_dispatch`
   for byte-identical delegation logic; the wildcard arm in `handle_request` must
   still call `classify_dispatch` with no added prefix guard.
7. **`server/epr_ssr.rs`** — EPR dispatch + SSR render orchestration + four test
   modules (the largest seam, ~1400 LoC incl. tests). Split into `epr_dispatch.rs`
   + `render_orchestration.rs` if it exceeds ~1500 LoC on its own. Depends on
   `dispatch`, `responses`, `AppState`.
8. **`server/host_routing.rs`** — bootstrap/signal/blob-verify/k2 handlers +
   `maybe_contribute_observation`. Independent handler cluster.
9. **`server/state.rs`** — `AppState` + `init_*` constructors + `impl AppState`.
   Done last because everything above borrows `AppState`; moving it last means
   each prior step compiled against the in-file definition and only the final
   step flips the re-export. Update `server/mod.rs` `pub use` accordingly and
   sweep `rg 'server::http::AppState'` crate-wide (incl. `main.rs`, other crates).

After each step: `RUSTFLAGS="" cargo build --lib --bins`, `cargo test --lib
--bins`, `cargo clippy -- -D warnings`, `cargo fmt --check`. Commit path-scoped.

## Expected outcome

`http.rs` retains `handle_request`, `run`, `spawn_health_listener` (or moves it
with liveness), imports, and module declarations — target well under the ceiling
(~1600–1800 LoC). Nine new focused sibling modules under `src/server/`, each with
its own tests. No behavior change; the RouteRegistry wildcard dispatch is
byte-identical. `server/CLAUDE.md`'s route-registry gate stays valid (the gate
lives at the `handle_request` dispatch match, which does not move).

## Readiness notes

- **Re-verified 2026-07-11** (finding 53c05bb1c007 re-fired at 7022 lines, +8
  from the 7014 at first authoring). The seam map above still holds: all 12
  co-located test modules named in the seam table are present
  (`root_unavailable_tests`, `watchdog_tests`, `shakeout_tests`,
  `handoff_routing_tests`, `epr_dispatch_breaker_tests`, `ssr_session_tests`,
  `gate_layer_tests`, `dispatch_classification_tests`, `epr_universal_tests`,
  `epr_claims_dispatch_tests`, `admission_tests`, `op_gate_tests`), the item
  clusters sit where the table places them, and `server/mod.rs` still does
  `pub use http::{run, AppState}`. The +8-line drift is concentrated at the tail
  past `apply_membrane` (response helpers `to_boxed`/`not_found_response`/
  `service_unavailable_response`/`bad_request_response` now at 5310–5344;
  admission `admission_exempt`/`catching_up_response`/`apply_membrane` at
  5365–5411) — the line anchors in the seam table are approximate execution
  guides, not invariants; re-`grep` the item names at extraction time. **Step-9
  confirmation:** every downstream consumer imports `crate::server::AppState`
  (the `mod.rs` re-export), not `server::http::AppState` — so moving `AppState`
  to `server/state.rs` while preserving the re-export is transparent to all
  callers (`routes/{apps,journal,admin_cache,federation,self_healing,health,
  elohim_agent,stream}.rs` verified). The `rg 'server::http::AppState'` sweep
  should come back empty; the load-bearing sweep target is `server::AppState`.
- **Ready now.** Pure mechanical extraction on plain native Rust; no upstream
  dependency, no substrate/DNA coupling. Blocked by nothing.
- **Watch-out — visibility churn.** The single biggest source of per-step
  friction: each private `fn` that `handle_request` still calls must become
  `pub(super)`/`pub(crate)` when it moves. Sweep `rg '<fn_name>'` before each
  step and expect clippy `dead_code`/`unused` deltas as items relocate.
- **Watch-out — test-module imports.** Co-located tests use bare-name calls; on
  move they need `use super::*;` or explicit `use crate::server::<module>::*;`.
- **Watch-out — `AppState` field visibility.** Handlers reach `AppState` fields
  directly; if those fields are private and handlers move out, the fields need
  `pub(crate)` or accessor methods. Prefer `pub(crate)` fields over a mechanical
  accessor sprawl for this pass; note any that deserve encapsulation as follow-up.
- **Do NOT** attempt to also split `handle_request`'s match body across modules in
  this pass — the dispatch match is the retained core and the registry-discipline
  anchor. Extract the handlers it *calls*, not the match itself.
