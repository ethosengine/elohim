---
status: Draft
cites:
  - ../specs/2026-05-29-epr-reachability-economics.md   # the related doc this derives from
---

# EPR-App Delivery — Substrate Shakeout Sprint Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make a seed through `alpha.elohim.host` light up `elohim.host/` and `elohim.host/lamad` — by finishing the outstanding *implementation* on the EPR-app delivery path (the code gaps), so the doorway EprRouter can populate and project the sponsored front doors, and project-epr commitments replicate across peers in a mesh.

**Architecture:** project-epr commitments are notarized DHT `Commitment`s (Category A) projected to each peer's local SQLite (`rea_commitments`, `dht_anchor_hash` set) and read by the doorway EprRouter over an HTTP route. The doorway is a **thin web2 projection** (`project-epr` = a *sponsorship / named-front-door* primitive, NOT a sitemap — see `genesis/docs/superpowers/specs/2026-05-29-epr-reachability-economics.md`). This sprint closes the code path: (1) the missing storage read route the EprRouter already calls, (2) the subscriber-gating bug that times out writes, (3) retryable error codes, (4) a DHT discovery anchor + (5) a storage-side reconciler so a project-epr commitment authored on one peer projects onto the others in the same kitsune mesh. Cross-*doorway* reach (alpha↔apex) and SPA-blob staging are operator/deploy dependencies, tracked but not sprint tasks.

**Tech Stack:** Rust (elohim-storage: axum/hyper HTTP + Diesel/SQLite + holochain_client; content_store DNA: HDK/HDI), TypeScript (genesis seeder — operator lane only). Holochain 0.6.

**Branch:** Cut a fresh branch off **`origin/dev`** — `fix/epr-app-delivery-shakeout`. `origin/dev` already carries the prior delivery fixes (E5 connect-race `2565643a7`, Gap-F `d5ac52411`, EprRouter missed-event `a87eeacce`, DNA-drift auto-reinstall `c60b6e036`). **Do NOT branch from the current working tree** (`sprint/cross-pillar-cleanup`) — it is diverged and missing all of these. (See "Branch & Merge Strategy" below.)

---

## Context: Why /lamad 404s (verified on origin/dev, 2026-05-29)

A 15-agent grounded shakeout traced the seed→landing path. The binding blocker is **not** cross-peer replication — it is upstream of it:

| Hop | Status | Note |
|---|---|---|
| seed | partial | seeds 6 specs to ONE doorway (alpha); apex never seeded (operator) |
| storage-write | ✅ wired | conductor-first write correct on dev (E5+Gap-F+DNA-drift landed) |
| dna-postcommit | ✅ wired | Gap-F `deny_unknown_fields` present; `ReaCommitmentCommitted` fires |
| signal-subscriber | ⚠ wired-but-misgated | subscriber nested under `registry.infrastructure` (Task 2) |
| **epr-router** | ❌ **broken** | **doorway calls `GET /db/rea_commitments` which 404s on storage (Task 1)** |
| cross-peer-projection | ❌ broken | non-author peers never project gossiped commitments (Tasks 4+5) |
| cross-doorway-reach | ❌ broken | alpha/apex bootstrap stores don't bridge (operator/deferred) |
| spa-blob | partial | blob staging to apex backend (operator) |

**The gate (rank 1):** `doorway/doorway-service/src/projection/epr_router.rs:24` does
`GET {storage}/db/rea_commitments?action=project-epr&doorwayId={id}` then `.error_for_status()?`.
Storage `handle_db_request` (http.rs:2825) has arms for `content`/`relationships`/`humans`/… but
**no `rea_commitments` arm** → falls to `"Unknown database endpoint"` 404 (http.rs:3169) → the
router's `replace_all()` is never reached → EprRouter empty → `/lamad` 404 on **every** doorway,
even one whose SQL holds the commitments. The translator `find_active_projections()`
(rea_commitments.rs:548) is built and unit-tested but has **zero HTTP callers.** This sprint wires it.

### P2P Design Gate (passed)

> **P2P audit note:** Every HTTP route in this plan (`GET /db/rea_commitments`, `POST /api/v1/commitments`) serves the **existing `Commitment` entry type** — there is **no new entry type, table, or migration** in Phase 1. The routes *follow from* the DHT design (3a/3b already shipped); Task 1 adds only the missing **3c read** over the local projection. The one DHT change in the whole plan is Task 5's `LinkTypes::DoorwayToProjection` (a Category-A2 link, not an entry type). Automated line-level route flags are expected here and are answered by this classification.

- **project-epr Commitment** — Category A (Notarized; existing `Commitment` entry type; `dht_anchor_hash` set; content-derived id `project-epr-{sha256(...)}`). Coordinator `content_store::create_rea_commitment` (3a) + post_commit `ReaCommitmentCommitted` → `rea_projection` → SQL (3b) already exist. This sprint adds the missing **3c read route** (Task 1) over the *local* projection — no DHT change.
- **EprProjectionView** — Category C (operational view derived from the projection).
- **Cross-peer reconciler** (Tasks 4+5) — reconciliation controller (DHT manifest → SQL projection). Requires a **Category A2 discovery link** (`StringAnchor("doorway_projections", doorwayId)` → Commitment) because `create_rea_commitment` today only anchors by `commitment_id`/`provider`/`receiver` (no action/doorway anchor), and the DHT has no query. New `LinkTypes` variant only — no new entry type. `get_links` is legal in the coordinator (HDK), illegal in validators (HDI).

---

## Branch & Merge Strategy

- This sprint's delivery code lands on `fix/epr-app-delivery-shakeout` off `origin/dev`.
- The diverged `sprint/cross-pillar-cleanup` branch (Sprint-3 dwelling-hub work: mutuality/capacity/bounds/donut) is a **separate** concern — it does NOT gate this acceptance test and must be merged to dev on its own track. Its working-tree `schema_contract.rs` + `epr-publish-input.schema.json` edits (the 4→0 fix) must land **atomically with their views.rs pair** (`EprPublishInput.event`); do not cherry-pick them onto this branch in isolation.
- Push only after the operator's RESET_STORAGE run confirms alpha (per operator sequencing). CI is the gate of record (`HUSKY=0` push permitted per pipeline-vs-sprint convention).

## Build commands (every Rust step)

elohim-storage requires the custom getrandom backend; redirect the target dir to the pool:
```bash
export CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev
export RUSTFLAGS='--cfg getrandom_backend="custom"'
```
(For the DNA tasks use plain `cargo` in the DNA workspace — do NOT redirect target/; `hc dna pack` canonicalizes `./target`.)

---

# PHASE 1 — Delivery-critical (lights up the front doors)

Completing Phase 1 + the operator's apex seed makes both doorways serve `/` and `/lamad`. No DNA change.

### Task 1: Wire `GET /db/rea_commitments?action=project-epr&doorwayId=X` on storage

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs` (`handle_db_request` ~2825 — add a `rea_commitments` arm; `build_manifest` ~9438 — add `Route::get`)
- Reuse: `elohim/elohim-storage/src/db/rea_commitments.rs:548` `find_active_projections(conn, ctx, doorway_id) -> Result<Vec<EprProjectionView>, StorageError>` (already tested at `:757 find_active_projections_filters_by_doorway_id`)
- Test: `elohim/elohim-storage/tests/db_rea_commitments_route.rs` (new integration test) OR extend an existing http test module

- [ ] **Step 1: Write the failing test**

Create `elohim/elohim-storage/tests/db_rea_commitments_route.rs`. Seed two project-epr commitments (scopes `doorway:alpha-elohim-host|epr:lamad-spa` and `doorway:elohim-host|epr:lamad-spa`) directly via the diesel layer in a temp DB, then assert the route returns only the matching doorway's projections as `Vec<EprProjectionView>`.

```rust
// Mirrors the harness used by other http/db integration tests (temp SQLite + AppContext).
#[tokio::test]
async fn db_rea_commitments_route_returns_project_epr_projections_for_doorway() {
    let (server, ctx) = test_support::spawn_storage_with_temp_db().await;
    test_support::insert_project_epr_commitment(&ctx, "doorway:alpha-elohim-host|epr:lamad-spa", "/lamad");
    test_support::insert_project_epr_commitment(&ctx, "doorway:elohim-host|epr:lamad-spa", "/lamad");

    let resp = server
        .get("/db/rea_commitments?action=project-epr&doorwayId=alpha-elohim-host")
        .await;
    assert_eq!(resp.status(), 200);

    let views: Vec<elohim_views::EprProjectionView> = resp.json();
    assert_eq!(views.len(), 1, "only the alpha-scoped projection should match");
    assert_eq!(views[0].url_path, "/lamad");
    assert!(views[0].doorway_id.contains("alpha-elohim-host"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path elohim/elohim-storage/Cargo.toml --test db_rea_commitments_route -- --nocapture`
Expected: FAIL — route returns 404 `"Unknown database endpoint"` (so `resp.status()` is 404, assertion fails). This reproduces the rank-1 blocker.

- [ ] **Step 3: Add the route arm in `handle_db_request`**

In `elohim/elohim-storage/src/http.rs`, inside `handle_db_request`, after the existing `content`/`relationships` arms and BEFORE the final `"Unknown database endpoint"` fallthrough (~line 3169), add:

```rust
// project-epr projection read — the doorway EprRouter's source.
// Source of truth is the DHT Commitment; this serves the local SQL projection.
if resource_path == "rea_commitments" {
    if method != Method::GET {
        return Ok(response::method_not_allowed());
    }
    let query = req.uri().query().unwrap_or("");
    let params: std::collections::HashMap<_, _> =
        url::form_urlencoded::parse(query.as_bytes()).into_owned().collect();

    // Only project-epr is exposed here; other actions use /api/v1/commitments.
    if params.get("action").map(String::as_str) != Some("project-epr") {
        return Ok(response::error_response(StorageError::InvalidInput(
            "GET /db/rea_commitments requires action=project-epr".into(),
        )));
    }
    let doorway_id = match params.get("doorwayId") {
        Some(d) if !d.is_empty() => d.clone(),
        _ => {
            return Ok(response::error_response(StorageError::InvalidInput(
                "GET /db/rea_commitments requires doorwayId".into(),
            )))
        }
    };

    let mut conn = self.get_conn()?;
    let ctx = AppContext::from(&app_ctx); // same conversion other db arms use
    let views = db::rea_commitments::find_active_projections(&mut conn, &ctx, &doorway_id)?;
    return Ok(response::ok(&views));
}
```

(Match the exact `AppContext` construction used by neighboring arms — e.g. `handle_db_content_list` shows how `app_ctx` becomes the `ctx: &AppContext` passed to `db::*`. `url` is already a dependency; if not, mirror the query-parse helper used elsewhere.)

- [ ] **Step 4: Register the route in `build_manifest`**

In `build_manifest` (~line 9438, alongside `Route::get("/db/content")` etc.) add:

```rust
Route::get("/db/rea_commitments")
    .description("List project-epr projection commitments for a doorway (EprRouter source)"),
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test --manifest-path elohim/elohim-storage/Cargo.toml --test db_rea_commitments_route -- --nocapture`
Expected: PASS (200, exactly one alpha-scoped projection).

- [ ] **Step 6: Regression — full storage test suite + clippy**

Run:
```bash
cargo test --manifest-path elohim/elohim-storage/Cargo.toml --test schema_contract
cargo clippy --manifest-path elohim/elohim-storage/Cargo.toml -- -D warnings
```
Expected: schema_contract still `ok` (208 passed); clippy clean.

- [ ] **Step 7: Commit**

```bash
git add elohim/elohim-storage/src/http.rs elohim/elohim-storage/tests/db_rea_commitments_route.rs
git commit -m "fix(storage): add GET /db/rea_commitments project-epr route — populates doorway EprRouter

The doorway EprRouter fetches GET /db/rea_commitments?action=project-epr&doorwayId=X
but storage had no arm for it (404 Unknown database endpoint), so replace_all() was
never reached and /lamad 404'd on every doorway. Wires the existing-and-tested
find_active_projections() into handle_db_request + build_manifest."
```

---

### Task 2: ~~Spawn the rea_projection subscriber under the app/lamad HcClient~~ — ELIMINATED (do not implement)

> **ELIMINATED by runtime evidence (handoff `9c243873f`, 2026-05-29).** The workflow's static rank-7 (the subscriber being gated under `registry.infrastructure` would miss the lamad cell's signal) was disproven by the operator's cluster-state check: **signal delivery is app-wide in `holochain_client` 0.9.0-dev.5**, so the subscriber on `registry.infrastructure` *does* receive the lamad cell's `ReaCommitmentCommitted`. The real root cause was Theory A — Gap-F DNA never installed (drift probe timed out), now fixed by `9c243873f` (file-based bundle-hash probe). **Do NOT move the subscriber.** This task is intentionally void; skip its steps below.

- [ ] **Step 1: Write the failing/guard test**

Add a unit test asserting the subscriber-spawn helper does not require `registry.infrastructure`. Extract the subscribe logic into a function `spawn_rea_projection_subscriber(hc: Arc<HcClient>, pool: DbPool, ...)` and test that it is callable with a lamad-role client.

```rust
#[test]
fn rea_projection_subscriber_spawns_under_lamad_client() {
    // Given a registry with infrastructure=None but lamad=Some,
    // the chooser picks the lamad client (the write path's dependency).
    let chosen = choose_rea_signal_client(/* infrastructure */ None, /* lamad */ Some(()), /* imagodei */ None);
    assert!(chosen.is_some(), "subscriber must spawn when lamad is present even if infrastructure is absent");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path elohim/elohim-storage/Cargo.toml choose_rea_signal_client`
Expected: FAIL — `choose_rea_signal_client` not defined.

- [ ] **Step 3: Implement the chooser + hoist the subscribe call**

Add `choose_rea_signal_client` (prefer lamad → imagodei → infrastructure) and move the `subscribe_rea_projection_signals(...)` call out of the infrastructure-only block in `main.rs` so it runs under the chosen client:

```rust
fn choose_rea_signal_client<T>(infra: Option<T>, lamad: Option<T>, imagodei: Option<T>) -> Option<T> {
    // The project-epr write path round-trips through lamad; subscribe on the same conductor
    // so ReaCommitmentCommitted (author-local) lands in SQL. Fall back to any available client.
    lamad.or(imagodei).or(infra)
}
```
Then in startup, replace the nested call with one driven by `choose_rea_signal_client(registry.infrastructure.clone(), registry.lamad.clone(), registry.imagodei.clone())`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path elohim/elohim-storage/Cargo.toml choose_rea_signal_client`
Expected: PASS.

- [ ] **Step 5: Build + clippy**

Run: `cargo build --manifest-path elohim/elohim-storage/Cargo.toml && cargo clippy --manifest-path elohim/elohim-storage/Cargo.toml -- -D warnings`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/main.rs
git commit -m "fix(storage): spawn rea_projection subscriber under lamad client, not infrastructure-only

project-epr writes round-trip through the lamad bridge; the projection subscriber
was nested under registry.infrastructure, so an infra-only connect failure left
project-epr writes succeeding at the conductor but timing out the 1s SQL poll -> 500."
```

---

### Task 3: Map conductor-path errors to retryable HTTP codes (503/504)

**Files:**
- Modify: `elohim/elohim-storage/src/services/response.rs` (`error_response` ~113)
- Modify: `elohim/elohim-storage/src/services/rea_commitment_service.rs` (`create_via_conductor` poll-timeout error)

**Problem:** `StorageError::Conductor` has no arm in `error_response` (falls to 500); the 1s projection-poll timeout returns `StorageError::Internal` (500). Seeders/clients cannot distinguish transient conductor-readiness from hard failure. The E5 commit message claims "503 lamad bridge unavailable" but the code returns 500.

- [ ] **Step 1: Write the failing test**

In `response.rs` tests (alongside the existing `error_response` tests at ~225):

```rust
#[test]
fn conductor_error_maps_to_503() {
    let resp = error_response(StorageError::Conductor("lamad bridge unavailable".into()));
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path elohim/elohim-storage/Cargo.toml conductor_error_maps_to_503`
Expected: FAIL — currently 500 (no `Conductor` arm).

- [ ] **Step 3: Add the mapping**

In `error_response` (response.rs), add next to the `Connection`/`Timeout` arms:

```rust
StorageError::Conductor(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg.clone()),
```
And in `rea_commitment_service.rs::create_via_conductor`, change the poll-timeout `Err(StorageError::Internal(...))` to `Err(StorageError::Timeout(...))` (already maps to 504) so a client can retry past a slow post-commit:

```rust
Err(StorageError::Timeout(format!(
    "REA commitment {} written via conductor but projection did not land in local SQL within 1s \
     — retry (transient post-commit latency); check rea_projection subscriber if persistent",
    id
)))
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path elohim/elohim-storage/Cargo.toml conductor_error_maps_to_503`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/services/response.rs elohim/elohim-storage/src/services/rea_commitment_service.rs
git commit -m "fix(storage): conductor-path errors map to 503/504 so seeders can distinguish retryable readiness"
```

---

### Task 4: Substrate debt — repair sweettest compile errors (reciprocity_view + recognition)

**Files:**
- Investigate/Modify: `elohim/holochain/tests/sweettest/src/**` (recognition_participation_via_route.rs and any reciprocity_view usage)

**Note:** these are pre-existing compile errors on the integration line (HC 0.6 API drift, sibling to landed fixes `03c432cd2`/`a726d7617`). Reproduce first to get exact errors — do not guess.

- [ ] **Step 1: Reproduce — capture the exact compile errors**

Run (native; sweettest links the conductor — confirm the crate's documented flags first via its README/Cargo, default to native):
```bash
export CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__holochain__tests__sweettest/dev
cargo check --manifest-path elohim/holochain/tests/sweettest/Cargo.toml 2>&1 | tee /tmp/sweettest-check.log
```
Expected: FAIL — record every `error[E...]` (likely `ReciprocityView`/`recognition` type or API drift).

- [ ] **Step 2: Fix each error, smallest change first**

For each error, apply the minimal API-drift repair (mirror the patterns in `03c432cd2`/`a726d7617`: `await_consistency` error coercion, `session_human` peer-exchange API, type/field renames). Show the diff per error.

- [ ] **Step 3: Verify it compiles**

Run: `cargo check --manifest-path elohim/holochain/tests/sweettest/Cargo.toml`
Expected: clean compile (0 errors).

- [ ] **Step 4: Commit**

```bash
git add elohim/holochain/tests/sweettest/
git commit -m "fix(sweettest): repair reciprocity_view + recognition compile errors (HC 0.6 API drift)"
```

---

# PHASE 2 — Architectural cross-peer close ("option 3")

Makes a project-epr commitment authored on one peer project onto every peer **in the same kitsune mesh** (so any doorway-edge backed by any mesh peer can project it). Cross-*doorway* (alpha↔apex, separate signal servers) additionally needs the bootstrap bridge OR the operator fan-out seed (see Operator Dependencies). This is the "epr commitments enabling resiliency + delivery" piece.

### Task 5: Add a project-epr DHT discovery anchor (Category A2 link)

**Files:**
- Modify: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` (`LinkTypes` enum — add `DoorwayToProjection`)
- Modify: `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` (`create_rea_commitment` ~11869 — create the anchor link for project-epr; add `list_projections_for_doorway`)

**Why:** `create_rea_commitment` anchors only by `commitment_id`/`provider`/`receiver`. A remote peer cannot enumerate project-epr commitments for a doorway from the DHT (no query). Add a doorway-scoped anchor so any peer can `get_links` them. No new entry type — one new `LinkTypes` variant (cheap; redeploys cleanly via the c60b6e036 DNA-drift reinstall).

- [ ] **Step 1: Write the failing sweettest**

In sweettest, author a project-epr commitment on conductor A, then assert conductor B (same network, after `await_consistency`) can call `list_projections_for_doorway("alpha-elohim-host")` and get it.

```rust
#[tokio::test(flavor = "multi_thread")]
async fn project_epr_commitment_is_discoverable_by_doorway_anchor_cross_conductor() {
    let (conductors, ..) = two_agent_conductors().await; // exchange_peer_info + await_consistency
    // A authors project-epr scoped doorway:alpha-elohim-host|epr:lamad-spa
    let _ = call_create_rea_commitment(&conductors[0], project_epr_input("alpha-elohim-host", "lamad-spa")).await;
    await_consistency(&conductors).await;
    let found: Vec<ReaCommitmentOutput> = conductors[1]
        .call(&cell_b, "list_projections_for_doorway", "alpha-elohim-host".to_string())
        .await;
    assert!(found.iter().any(|c| c.commitment.action == "project-epr"));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --manifest-path elohim/holochain/tests/sweettest/Cargo.toml project_epr_commitment_is_discoverable_by_doorway_anchor_cross_conductor`
Expected: FAIL — `list_projections_for_doorway` undefined.

- [ ] **Step 3: Add the LinkType + anchor link + list fn**

In `content_store_integrity/src/lib.rs` `LinkTypes` enum, add `DoorwayToProjection`.
In `content_store/src/lib.rs` `create_rea_commitment`, after the existing `commitment_id`/`provider`/`receiver` anchors, when `commitment.action == "project-epr"`, parse the doorwayId from `in_scope_of` (`doorway:{id}|epr:{epr}`) and link:

```rust
if commitment.action == "project-epr" {
    if let Some(doorway_id) = parse_doorway_id_from_scope(&commitment.in_scope_of_json) {
        let anchor = StringAnchor::new("doorway_projections", &doorway_id);
        let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;
        create_link(anchor_hash, action_hash.clone(), LinkTypes::DoorwayToProjection, ())?;
    }
}
```
Add the reader (HDK — `get_links` is legal here):

```rust
#[hdk_extern]
pub fn list_projections_for_doorway(doorway_id: String) -> ExternResult<Vec<ReaCommitmentOutput>> {
    let anchor = StringAnchor::new("doorway_projections", &doorway_id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;
    let links = get_links(LinkQuery::try_new(anchor_hash, LinkTypes::DoorwayToProjection)?, GetStrategy::default())?;
    let mut out = Vec::new();
    for link in links {
        if let Some(target) = link.target.into_action_hash() {
            if let Some(c) = get_rea_commitment_by_action(target)? { out.push(c); }
        }
    }
    Ok(out)
}
```

- [ ] **Step 4: Pack + run the test**

Run (DNA workspace — plain cargo, no target redirect):
```bash
cargo test --manifest-path elohim/holochain/tests/sweettest/Cargo.toml project_epr_commitment_is_discoverable_by_doorway_anchor_cross_conductor
```
Expected: PASS (B discovers A's projection via the anchor).

- [ ] **Step 5: Full integrity + clippy + fmt**

Run: `cargo test -p content_store_integrity && cargo clippy -- -D warnings && cargo fmt --check`
Expected: clean (integrity suite green).

- [ ] **Step 6: Commit**

```bash
git add elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs elohim/holochain/tests/sweettest/
git commit -m "feat(dna): doorway_projections anchor + list_projections_for_doorway — make project-epr commitments DHT-discoverable per doorway"
```

---

### Task 6: Storage-side REA reconciler — project gossiped commitments on non-author peers

**Files:**
- Create: `elohim/elohim-storage/src/rea_dht_reconcile.rs`
- Modify: `elohim/elohim-storage/src/main.rs` (spawn the reconciler task)
- Reuse: `elohim/elohim-storage/src/rea_projection.rs::upsert_with_anchor`

**Why:** `post_commit`/`ReaCommitmentCommitted` is author-local; remote peers never get the signal. The reconciler closes intra-mesh cross-peer projection: on a consistency/join signal or periodic tick, list project-epr commitments from the DHT (Task 5's `list_projections_for_doorway` for this node's doorwayId) and upsert each into local SQL via the existing `dht_anchor_hash` path.

- [ ] **Step 1: Write the failing test**

Two-conductor sweettest (or a storage integration test with a mock HcClient): author a project-epr commitment on peer A; assert that after the reconciler runs on peer B, B's SQL `rea_commitments` contains it with `dht_anchor_hash` set (so B's `find_active_projections` returns it → B's doorway can project it).

```rust
#[tokio::test]
async fn reconciler_projects_remote_authored_commitment_into_local_sql() {
    let env = two_peer_storage_env().await; // A + B share a network
    author_project_epr_on(&env.peer_a, "elohim-host", "lamad-spa").await;
    env.await_consistency().await;
    run_rea_reconcile(&env.peer_b, "elohim-host").await.unwrap();
    let rows = db::rea_commitments::find_active_projections(&mut env.peer_b.conn(), &env.peer_b.ctx, "elohim-host").unwrap();
    assert_eq!(rows.len(), 1, "peer B reconciled the remote-authored projection into its SQL");
    assert!(rows[0].dht_anchor_hash.is_some());
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --manifest-path elohim/elohim-storage/Cargo.toml reconciler_projects_remote_authored_commitment_into_local_sql`
Expected: FAIL — `run_rea_reconcile` undefined.

- [ ] **Step 3: Implement the reconciler**

Create `rea_dht_reconcile.rs`:

```rust
//! REA DHT reconciler — projects project-epr commitments authored by OTHER peers.
//! post_commit is author-local; this closes intra-mesh cross-peer projection by
//! listing from the DHT (content_store::list_projections_for_doorway) and upserting
//! into the local SQL projection with dht_anchor_hash. Source of truth stays the DHT.
pub async fn run_rea_reconcile(
    hc: &Arc<HcClient>,
    pool: &DbPool,
    ctx: &AppContext,
    doorway_id: &str,
) -> Result<usize, StorageError> {
    let payload = rmp_serde::to_vec_named(&doorway_id.to_string())?;
    let out = hc.call_zome("content_store", "list_projections_for_doorway", &payload).await?;
    let commitments: Vec<ReaCommitmentOutput> = rmp_serde::from_slice(&out)?;
    let mut conn = pool.get()?;
    let mut n = 0;
    for c in commitments {
        rea_projection::upsert_with_anchor(&mut conn, ctx, &c.commitment, c.action_hash.clone())?;
        n += 1;
    }
    Ok(n)
}
```

Spawn in `main.rs` after the conductor connects: on the network/consistency signal (preferred) and a slow periodic tick (e.g. 60s) as a safety net, for this node's configured `DOORWAY_ID`.

- [ ] **Step 4: Run the test**

Run: `cargo test --manifest-path elohim/elohim-storage/Cargo.toml reconciler_projects_remote_authored_commitment_into_local_sql`
Expected: PASS.

- [ ] **Step 5: Build + clippy**

Run: `cargo build --manifest-path elohim/elohim-storage/Cargo.toml && cargo clippy --manifest-path elohim/elohim-storage/Cargo.toml -- -D warnings`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/rea_dht_reconcile.rs elohim/elohim-storage/src/main.rs
git commit -m "feat(storage): REA DHT reconciler — project-epr commitments authored on other mesh peers project into local SQL"
```

---

## Operator Dependencies (NOT sprint tasks — shift-operator lane)

These gate the *cross-doorway* acceptance test but are deploy/pipeline work. Coordinate; do not implement on this branch.

1. **Reconcile apex DOORWAY_ID vs seed scope (rank 3).** `genesis/orchestrator/manifests/doorway/alpha-b.yaml:181` sets `DOORWAY_ID: apex-elohim-host`, which never matches any seeded scope (seeds use `elohim-host`). Lowest-risk: change it to `elohim-host`; retire the orphan `prod.yaml` `prod-elohim-host` id.
2. **Seed every live doorway with its own scope + restart it (rank 4).** `genesis/Jenkinsfile seedProjectionsStage` POSTs to ONE alpha doorway. Until Tasks 5+6 land AND a cross-doorway bridge exists, apex MUST be seeded directly (POST apex-scoped commitments to the adam-backed apex doorway, then restart `elohim-doorway-alpha-b`).
3. **Stage SPA blobs to the apex backend (rank 5).** Run `stageSpaBlobs` PATCH+upload against adam storage; verify `GET /db/content/lamad-spa` reports non-empty blobHash and `GET /apps/lamad-spa/index.html` → 200 (GET, not HEAD).
4. **Verify `c60b6e036` reinstalled the Gap-F DNA** on both `elohim-matthew-alpha` and `elohim-adam-alpha` conductors (gated by `ALLOW_DNA_REINSTALL`); re-probe `POST /api/v1/commitments?action=project-epr` → 201 and `GET ?action=project-epr` → rows on each backend.
5. **(Optional) conductor agent-info bridge** — only if a shared alpha↔apex mesh is preferred over fan-out seeding. Needs the missing `gossipsub.subscribe(CONDUCTOR_AGENT_INFO_TOPIC)` in `behaviour.rs` + `ENABLE_CONDUCTOR_AGENT_INFO_GOSSIP=true` in both StatefulSets. Treat as separate backlog; fan-out seed is the deterministic path.

## Out of Scope (next epic — Role 2)

Per `genesis/docs/superpowers/specs/2026-05-29-epr-reachability-economics.md`: the content-addressed **resolver** (doorway-as-proxy for un-projected commons EPRs), the **finance-bridge / toll** economics, **standing-gated peer serve**, and **doorway-as-governable-asset**. These are a distinct truth-flow deserving their own P2P design gate.

## Definition of Done (acceptance)

- **Phase 1 + operator apex seed:** `GET https://alpha.elohim.host/lamad` → 200 SPA; `GET https://elohim.host/lamad` → 200 SPA; `GET /api/v1/commitments?action=project-epr` → non-empty on each backend; `GET /db/rea_commitments?action=project-epr&doorwayId=<id>` → matching projections.
- **Phase 2:** a project-epr commitment authored on one mesh peer appears (via reconciler) in another mesh peer's `find_active_projections`, with `dht_anchor_hash` set — proven by the two-conductor sweettest.
- All Rust gates green: `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`; `schema_contract` 208/0; sweettest compiles + the two new tests pass.

## Self-Review

- **Spec coverage:** rank 1 → Task 1; rank 7 → Task 2; rank 6 → Task 3; substrate-debt → Task 4; rank-2/cross-peer (DHT discovery + reconciler) → Tasks 5+6; rank 3/4/5 → Operator Dependencies; Role-2 → Out of Scope. ✅
- **Placeholders:** Task 1/2/3 carry concrete code grounded in verified signatures (`find_active_projections`, `error_response`, the `resource_path ==` arm pattern). Tasks 5/6 carry code sketches grounded in the observed `StringAnchor`/`create_link`/`get_links`/`upsert_with_anchor` patterns — the executing agent confirms exact types against the DNA at implementation time. Task 4 is reproduce-first (no fabricated error text). ✅
- **Type consistency:** `find_active_projections(conn, ctx, doorway_id) -> Vec<EprProjectionView>` used consistently in Tasks 1 & 6; `list_projections_for_doorway(String) -> Vec<ReaCommitmentOutput>` defined in Task 5 and consumed in Task 6; `LinkTypes::DoorwayToProjection` defined and used in Task 5. ✅
