---
id: plan-automerge-content-sync-plane-lighting
title: Light the Automerge content-sync plane — pure-Rust two-node convergence spine
status: Spine + stretch (G5 doorway, G6 frontend) + iroh-mode landed & verified (2026-06-27); follow-up = iroh sync-round driver
domain: D5
sprint: dataplane-automerge-spine (D5 forward slice; not in roadmap 1-6 — operator-requested 2026-06-27)
requires_env: [household-nodes]
cites:
  - p2p-dataplane-sync-engine-design-arc | History: The P2P dataplane + sync-engine design arc (March 2026) | sha256:d509030b5f00acd0 | path: genesis/docs/content/elohim-protocol/history/2026-06-11-p2p-dataplane-sync-engine-design-arc.md
  - genesis/docs/superpowers/plans/2026-06-14-dataplane-proofs-plan.md
  - resiliency-card-p2p-weave-sprint-plan | Resiliency-card + P2P-sync + Operational-Weave sprint | sha256:834716e333f5b01f | path: genesis/docs/superpowers/plans/2026-06-21-resiliency-card-p2p-weave-sprint-plan.md
  - dna-signal-as-epr-envelope | DnaSignal as EPR Envelope | sha256:507652ee91a75aa1 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-15-dna-signal-as-epr-envelope.md
---

# Light the Automerge Content-Sync Plane — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make peer content actually sync over the already-wired `/elohim/storage-sync/1.0.0` Automerge engine by adding the one missing organ — a producer that projects each content write into the Automerge DocStore — and prove two storage nodes converge that doc over real libp2p.

**Architecture:** The CRDT sync engine in `elohim-storage` is fully wired end-to-end in pure Rust (60s scheduler → `ListDocuments` → `SyncChanges` → `apply_changes` → sled) but **inert**: nothing fills the DocStore, so every sync round round-trips empty. This plan adds a go-forward producer subscribed to the existing `EventBus` that writes one Automerge doc per content id (`doc_id = "node:{id}"`) under `h_app_id = "elohim"`, then proves convergence with a slim two-node libp2p integration test. Browser legs (doorway `/sync` carriage, frontend ESM) are quarantined as parallel stretch — no browser caller exists today, and node-to-node convergence needs neither.

**Tech Stack:** Rust, `automerge` 3.x, libp2p 0.54.1 (`request_response` + `tcp`/`noise`/`yamux`), sled, tokio broadcast `EventBus`.

## Execution Outcome (2026-06-27) — SPINE LANDED + INDEPENDENTLY VERIFIED

The spine (G2, G1, G4, G3) is implemented, committed (commit-only, branch `feat/frontend-eyes-sprint`), and the convergence proof was re-run independently of the implementer:

- **G3 proof:** `doc_authored_on_a_converges_to_b ... ok` (0.35s) — node B converged `node:edit-prop-1` with `title == "Edited v2"` and `heads_a == heads_b` (full CRDT convergence) over real libp2p. Has teeth: RED at 30.35s (deadline) when `apply_changes` is skipped.
- **Unit gate:** 7/7 `--lib sync::` pass (G1/G2/G4 included). Baseline `tests/sync_integration` 5/5 still green. `cargo fmt --check` clean; `cargo clippy --features p2p` (lib+bins) green.
- **Commits:** `5606c6b77` (G2), `44e3ad502` (G1), `6f3e26212` (G4), `87c03f0cf` (G3), `64a02a3cb` (fmt).

**Forced deviations from this plan's code (real API, not faked):**
1. **No `DocType` enum** — `infer_doc_type` is a private `&self` method returning `String` (`"graph"`/`"path"`/…/`"unknown"`). G2 added a `"content"` arm; the test asserts the string on a `DocStore` instance.
2. **No `SyncManager::save`** — the real mutate+persist idiom (per `tests/sync_integration.rs:191`) is `apply_changes(ns, doc_id, vec![doc.save()])` — the same path a peer's changes take, so local projection and remote merge converge identically. The producer uses that.
3. **Reused `db::models::Content`** instead of a new `ContentRow` (`body ← content_body`, `metadata ← metadata_json`, already a JSON string).
4. **Added `SyncManager::get_doc_field(h_app_id, doc_id, field) -> Result<String, StorageError>`** (+ `use automerge::ReadDoc`) — the read accessor G1/G3 needed; `get_heads` already existed.
5. **Wiring site is `main.rs:~2574`** (the `Services::new` co-scope where `services.events` + `pool` + `p2p_node` are all live), not `:2611`. `services` Arc cloned so `services.events` survives `with_services(...)`; called as `elohim_storage::sync::projector::...` (binary crate).
6. **`PROJECTION_NAMESPACE` const introduced in G1** (cleaner than literal-then-promote), so G4 added only the guard test.

**Plan-gate correction (for the integrator):** the original combined gate `cargo test … --test sync_libp2p_convergence --lib sync::projector` applies the positional filter `sync::projector` to BOTH targets, so the integration test (name has no `sync::`) silently runs "0 tests". Run the two targets SEPARATELY (corrected in the Spine gate below).

**Deferred follow-ups (captured, not dropped):**
- **iroh-mode producer wiring** — the iroh `SyncManager` (`main.rs:2121`) lives only under the non-default `p2p-iroh` feature, outside the `--features p2p` gate, so it can't be compile-verified here. iroh-mode content sync was already inert (not regressed). The producer is transport-neutral → small follow-up.
- **Pre-existing `--all-targets` clippy debt (11 errors, NONE in spine files)** — the documented `feedback_pvc_deferral_hides_gate_debt` pattern; for the integrator/shift to triage. The plan's literal clippy gate (lib+bins) is green.

## Stretch + iroh Outcome (2026-06-27) — ALL THREE LANDED & INDEPENDENTLY VERIFIED

After the spine, the two stretch legs AND iroh-mode were carried on (operator "carry on"). Commits on `feat/frontend-eyes-sprint`, commit-only:

- **iroh-mode producer** (`e4fb14727`) — `spawn_content_projection_listener` wired on the iroh path (clonable `Arc<SyncManager>` taken before the move into `SyncManagerBackend`, spawned under `#[cfg(feature="p2p-iroh")]`). Builds under `--features p2p-iroh` (2m41s) and `--features p2p`. **NEW FINDING (bounded follow-up, backlogged):** iroh now *fills* the DocStore but content won't *flow* P2P — the 60s `initiate_sync_round` driver is libp2p-only; iroh has no periodic round driver (its `IrohSyncClient` is invoked only from tests/benches). See backlog `iroh-sync-round-driver-gap.md`.
- **G5 doorway `/sync` carriage** (`f37b28509`) — grounding found the storage `/sync/v1` HANDLERS ALREADY EXIST (`http.rs:981` → `handle_sync_request`), so this was the smaller manifest+gating path. Declared the 5 `/sync/v1` routes in `build_manifest()` + **FLIPPED** the `test_manifest_builds` guard that forbade `/sync`; added `/sync` to doorway `is_service_path` + route-claims fixture + `shakeout_service_path_guards_sync` unit test; added `/sync` to the storage EPR-alias `RESERVED_URL_PREFIXES`. Doorway 762/0; storage manifest+validator tests green. **SECURITY-ADJACENT (operator review at merge):** `/sync` is now exposed through doorway — GET reads unauthenticated/uncached (inherit the known `http-reach-enforcement-gap`), POST `/changes` `auth_required`. No browser caller consumes it yet.
- **G6 frontend ESM** (`9357b10df`) — the smoke-test GATE caught that the plan's "wasm base64-inlined" premise was FALSE (automerge 3.2.4 `browser` export → wasm-bindgen bundler entry → breaks Zone.js Angular builds). Working fix: a tsconfig path-alias to automerge's `fullfat_base64.js` (base64 entry), proven on real dev+prod `ng build` (automerge lands in a lazy chunk, no initial-budget hit). SDK ESM flip (`module: ESNext`, `"type":"module"`, dropped the `new Function` import hack) breaks no consumer (40/40 SDK tests; consumers are type-only). Added `ContentDocSyncService` (thin reactive, signals, `node:{id}` under `"elohim"`, no-leak teardown) + 5 tests. **CAVEAT (operator's call):** the alias hardcodes an internal automerge dist path (bypasses package `exports`, degrades automerge types to `any` in app source) — brittle across upgrades; cleaner long-term fix is bundler-only resolution or making elohim-app zoneless.

Independently re-verified by the orchestrator: storage `--features p2p` build + spine 7/7 + `test_manifest_builds`; doorway `shakeout_service_path_guards_sync` + `reserved_prefixes_fixture_agrees_with_is_service_path` + 762/0; `ng build --configuration development` green.

## Global Constraints

- **THE load-bearing constraint:** the producer MUST write docs under `h_app_id = "elohim"`. The live sync timer `initiate_sync_round` hardcodes `h_app_id: "elohim"` (`elohim/elohim-storage/src/p2p/mod.rs:6996`); a doc written under any other namespace (e.g. `"lamad"`) sits inert forever. `h_app_id` on the sync plane is a sync-partition label, NOT the DNA app id.
- **Go-forward only.** The producer projects on *new writes*. Corpus back-fill of already-seeded SQL rows is explicitly OUT of scope (it is O(total rows), re-incurred each PVC/sled reset) — a separate, idempotent, gated migration ships it later. Existing seeded content does not retroactively sync until then.
- **One doc per content id, flat fields only** (`id`, `title`, `contentType`, `contentFormat`, `reach`, `body`, `metadata`, `updatedAt`). NEVER mutate `graph:`/`path:` docs on a content write (one node touches many edges → unbounded derived-data fan-out). Skip `ContentBulkCreated` (the write path already pauses p2p sync for bulk ≥50, `http.rs:4460`).
- **RUSTFLAGS discipline:** these are NATIVE integration tests. Build with `RUSTFLAGS=""` (the ambient WASM `getrandom` flag leaks → `undefined __getrandom_v03_custom` at link). Use a `/tmp` `CARGO_TARGET_DIR` (the `/projects`-volume fingerprint ENOENT), `RUSTC_WRAPPER=""` (sccache null-byte trap), and plain `cargo test` (no nextest in-container).
- **Commit-only.** Land commits on the current branch; the integrator pushes/merges. Do NOT `git push`.
- **Compose, don't fork.** Reuse `spawn_logging_listener` (the EventBus subscriber idiom), `SyncManager` (`get_or_create_doc`/`save` — no new public types), and the existing two-node swarm harness. Do NOT add a fourth sync dialect.

---

## File Structure

| File | Responsibility | Task |
|------|----------------|------|
| `elohim/elohim-storage/src/sync/doc_store.rs` (modify ~:298) | `infer_doc_type` learns the `node:`/`content:` prefix | G2 |
| `elohim/elohim-storage/src/sync/projector.rs` (create) | The producer: EventBus listener + `project_content_doc` | G1 |
| `elohim/elohim-storage/src/sync/mod.rs` (modify) | `pub mod projector;` export | G1 |
| `elohim/elohim-storage/src/main.rs` (modify ~:2611) | Spawn the listener next to `with_sync_manager` | G1 |
| `elohim/elohim-storage/tests/sync_libp2p_convergence.rs` (create) | Two-node libp2p convergence proof (verification floor) | G3 |

---

## Task G2: Teach `infer_doc_type` the `node:` prefix

Smallest, independent unit. Do it first — it's the warm-up that confirms the build env and gives the producer a correct doc-type classification to target.

**Files:**
- Modify: `elohim/elohim-storage/src/sync/doc_store.rs` (the `infer_doc_type` fn, ~:298-310 — today it knows `graph:` / `path:` / `personal:` / `community:`)
- Test: same file's `#[cfg(test)] mod tests` (or wherever doc_store unit tests live)

**Interfaces:**
- Consumes: the existing `DocType` enum and `infer_doc_type(doc_id: &str) -> DocType` (read the exact enum variants + signature at `doc_store.rs:298` before editing — match the existing naming convention).
- Produces: `infer_doc_type("node:abc")` classifies as the content/node doc type (add a `Content` variant to `DocType` only if no existing variant fits; mirror the existing variants' derive/match style).

- [ ] **Step 1: Write the failing test.** In the doc_store test module:

```rust
#[test]
fn infer_doc_type_recognizes_node_prefix() {
    // "node:" is the content-node doc namespace the projector writes.
    assert_eq!(infer_doc_type("node:abc-123"), DocType::Content);
}
```

(If the enum has no `Content` variant yet, this won't compile — that's the failing state; add the variant in Step 3.)

- [ ] **Step 2: Run it and confirm it fails.**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/tmp/elohim-sync-test-target \
  cargo test --features p2p --lib sync::doc_store::tests::infer_doc_type_recognizes_node_prefix -- --nocapture
```
Expected: FAIL (no `DocType::Content` variant, or wrong classification).

- [ ] **Step 3: Implement.** Add a `Content` variant to `DocType` (mirror existing variants), and a match arm in `infer_doc_type` before the default:

```rust
} else if doc_id.starts_with("node:") || doc_id.starts_with("content:") {
    DocType::Content
}
```
Match the surrounding `else if` chain style exactly (the grounding shows it as a starts_with chain).

- [ ] **Step 4: Run it and confirm it passes.** (same command as Step 2) → PASS.

- [ ] **Step 5: Commit.**

```bash
git add elohim/elohim-storage/src/sync/doc_store.rs
git commit -m "feat(sync): infer_doc_type recognizes node:/content: doc prefix"
```

---

## Task G1: The content-projection producer

The keystone. A second `EventBus` subscriber (alongside `spawn_logging_listener` and the SSE forwarder) that turns each content write into an Automerge doc in the DocStore under `"elohim"`.

**Files:**
- Create: `elohim/elohim-storage/src/sync/projector.rs`
- Modify: `elohim/elohim-storage/src/sync/mod.rs` (add `pub mod projector;`)
- Modify: `elohim/elohim-storage/src/main.rs` (~:2611, next to the `with_sync_manager` wiring site)
- Test: `elohim/elohim-storage/src/sync/projector.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes:
  - `EventBus` (`src/services/events.rs:80`) + its subscriber idiom in `spawn_logging_listener` (`events.rs:153-171`) — the exact `recv()` / `Lagged` / `Closed` loop to mirror.
  - `StorageEvent::ContentCreated` / `StorageEvent::ContentUpdated` (read the exact variant payloads in `events.rs` — grounding says they carry only `{id, title, content_type}`, so the producer re-SELECTs the full row by id).
  - `SyncManager` (`src/sync/mod.rs:35`): `get_or_create_doc(h_app_id, doc_id)` (`:53`) and `save` (confirm exact signatures/async-ness at those lines).
  - The DB pool (the same `Arc<Pool>`/`PgPool`-or-sqlite handle the content API uses) to re-read the full content row by id.
  - `infer_doc_type` `node:` classification from Task G2.
- Produces:
  - `pub fn spawn_content_projection_listener(events: Arc<EventBus>, sync: Arc<SyncManager>, pool: <PoolType>) -> tokio::task::JoinHandle<()>` — spawned once at startup.
  - `async fn project_content_doc(sync: &SyncManager, content: &ContentRow)` — writes/mutates `doc_id = format!("node:{}", content.id)` under `h_app_id = "elohim"`.
  - `fn content_doc_id(id: &str) -> String` → `format!("node:{id}")`.

- [ ] **Step 1: Write the failing test** (producer-level integration; no libp2p). In `projector.rs` tests:

```rust
#[tokio::test]
async fn producer_projects_content_create_into_docstore_under_elohim() {
    // Arrange: an in-memory DocStore + SyncManager (mirror tests/sync_integration.rs:17-30).
    let sync = test_sync_manager();           // helper that builds DocStore+StreamTracker+SyncManager on a temp sled
    let content = ContentRow {
        id: "edit-prop-1".into(),
        title: "v1".into(),
        content_type: "concept".into(),
        content_format: "markdown".into(),
        reach: "household".into(),
        body: "hello".into(),
        metadata: serde_json::json!({}),
        updated_at: "2026-06-27T00:00:00Z".into(),
    };

    // Act: project once.
    project_content_doc(&sync, &content).await;

    // Assert: doc exists under "elohim" at node:{id}, with the title, and non-empty heads.
    let heads = sync.get_heads("elohim", "node:edit-prop-1").await.expect("doc exists");
    assert!(!heads.is_empty(), "projected doc must have heads");
    let value = sync.get_doc_field("elohim", "node:edit-prop-1", "title").await.unwrap();
    assert_eq!(value, "v1");
}
```
(Use whatever read accessor `SyncManager`/`DocStore` actually exposes — `get_heads` is named in the grounding; if there is no `get_doc_field`, read the doc via the DocStore's existing getter and extract `title`. Confirm at `sync/mod.rs` / `doc_store.rs`.)

- [ ] **Step 2: Run it and confirm it fails.**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/tmp/elohim-sync-test-target \
  cargo test --features p2p --lib sync::projector::tests::producer_projects_content_create_into_docstore_under_elohim -- --nocapture
```
Expected: FAIL (module/`project_content_doc` not defined).

- [ ] **Step 3: Implement `project_content_doc` + the doc-id helper.**

```rust
pub fn content_doc_id(id: &str) -> String {
    format!("node:{id}")
}

/// Project a single content row into its Automerge doc under the "elohim" sync namespace.
/// THE NAMESPACE IS LOAD-BEARING: initiate_sync_round (p2p/mod.rs:6996) only lists "elohim".
pub async fn project_content_doc(sync: &SyncManager, content: &ContentRow) {
    let doc_id = content_doc_id(&content.id);
    // get_or_create_doc returns the live Automerge doc handle (confirm exact API at sync/mod.rs:53)
    let mut doc = sync.get_or_create_doc("elohim", &doc_id).await;
    doc.transact(|tx| {
        tx.put(automerge::ROOT, "id", content.id.as_str())?;
        tx.put(automerge::ROOT, "title", content.title.as_str())?;
        tx.put(automerge::ROOT, "contentType", content.content_type.as_str())?;
        tx.put(automerge::ROOT, "contentFormat", content.content_format.as_str())?;
        tx.put(automerge::ROOT, "reach", content.reach.as_str())?;
        tx.put(automerge::ROOT, "body", content.body.as_str())?;
        tx.put(automerge::ROOT, "metadata", content.metadata.to_string().as_str())?;
        tx.put(automerge::ROOT, "updatedAt", content.updated_at.as_str())?;
        Ok(())
    });
    sync.save("elohim", &doc_id, doc).await; // persist + register heads (confirm save signature)
}
```
Adapt the exact `transact`/`put`/`save` calls to the project's automerge wrapper — the grounding says `SyncManager` exposes `get_or_create_doc` + `save` and the merge path already works; match `tests/sync_integration.rs` for how a doc is mutated and saved there.

- [ ] **Step 4: Run it and confirm it passes.** (same command as Step 2) → PASS.

- [ ] **Step 5: Write the listener + spawn it.** In `projector.rs`:

```rust
pub fn spawn_content_projection_listener(
    events: Arc<EventBus>,
    sync: Arc<SyncManager>,
    pool: PoolHandle, // the same pool type the content API uses
) -> tokio::task::JoinHandle<()> {
    let mut rx = events.subscribe();           // mirror spawn_logging_listener (events.rs:157)
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(StorageEvent::ContentCreated { id, .. })
                | Ok(StorageEvent::ContentUpdated { id, .. }) => {
                    // re-SELECT the full row (events carry only id/title/content_type)
                    match load_content_row(&pool, &id).await {
                        Ok(Some(content)) => project_content_doc(&sync, &content).await,
                        Ok(None) => tracing::warn!(%id, "projector: content row vanished"),
                        Err(e) => tracing::error!(%id, error=%e, "projector: load failed"),
                    }
                }
                Ok(_) => {} // ignore other events (incl. ContentBulkCreated — see Global Constraints)
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(skipped = n, "projector: event bus lagged");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    })
}
```
`load_content_row` should use the existing content-read query (reuse the content API's row loader; do NOT hold a connection across the project call — use the pool).

- [ ] **Step 6: Wire it in `main.rs`** (~:2611, next to `with_sync_manager`). BEFORE `services` moves into `with_services`, clone what the listener needs:

```rust
// next to the existing `node.sync_manager()` / with_sync_manager wiring (~main.rs:2611)
#[cfg(feature = "p2p")]
{
    let projector_events = services.events.clone();      // Arc<EventBus>
    let projector_sync = node.sync_manager().clone();    // Arc<SyncManager> (whichever transport is live)
    let projector_pool = pool.clone();
    crate::sync::projector::spawn_content_projection_listener(
        projector_events, projector_sync, projector_pool,
    );
}
```
Confirm the live `SyncManager`: the iroh branch (`main.rs:2121`) builds its own `SyncManager` on the same `sync.sled` — spawn the listener pointed at whichever is active, under the same `#[cfg(feature="p2p")]`/transport guard the sync manager itself uses.

- [ ] **Step 7: Add the module export.** In `src/sync/mod.rs`: `pub mod projector;`

- [ ] **Step 8: Build the whole crate to confirm wiring compiles.**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/tmp/elohim-sync-test-target \
  cargo build --features p2p
```
Expected: clean build (no borrow-after-move at the wiring site).

- [ ] **Step 9: Commit.**

```bash
git add elohim/elohim-storage/src/sync/projector.rs elohim/elohim-storage/src/sync/mod.rs elohim/elohim-storage/src/main.rs
git commit -m "feat(sync): content-projection producer fills Automerge DocStore under elohim namespace"
```

---

## Task G4: Namespace-coupling guard

Make the `"elohim"` coupling loud so a future refactor can't silently re-break sync. Folded into G1's file; separate task because a reviewer could accept the producer but want a stronger guard.

**Files:**
- Modify: `elohim/elohim-storage/src/sync/projector.rs`

- [ ] **Step 1: Add a guard test asserting the projection namespace.**

```rust
#[test]
fn projection_namespace_matches_sync_timer() {
    // initiate_sync_round (p2p/mod.rs:6996) lists ONLY "elohim". If that ever changes,
    // this constant and the producer must move together or sync silently dies.
    const PROJECTION_NS: &str = "elohim";
    assert_eq!(PROJECTION_NS, crate::sync::projector::PROJECTION_NAMESPACE);
}
```

- [ ] **Step 2: Promote the literal to a named const** in `projector.rs` and use it in `project_content_doc`:

```rust
/// The sync-partition namespace. MUST equal the h_app_id listed by initiate_sync_round
/// (p2p/mod.rs:6996). Changing one without the other silently disables content sync.
pub const PROJECTION_NAMESPACE: &str = "elohim";
```
Replace the `"elohim"` string literals in `project_content_doc` with `PROJECTION_NAMESPACE`.

- [ ] **Step 3: Run the guard test → PASS** (same lib-test command, filter `projection_namespace_matches_sync_timer`).

- [ ] **Step 4: Commit.**

```bash
git add elohim/elohim-storage/src/sync/projector.rs
git commit -m "test(sync): guard the load-bearing elohim projection namespace coupling"
```

---

## Task G3: Two-node libp2p convergence proof (verification floor)

The deliverable's proof: a doc written into node A's DocStore converges into node B's over real libp2p. This closes the **zero-integration-coverage gap** on the swarm wiring (`initiate_sync_round → handle_sync_request → handle_sync_response`) — `tests/sync_integration.rs` proves only the merge logic, not the libp2p leg. Seeds the DocStore directly via the producer's `project_content_doc`, so it also exercises G1 end-to-end.

**Files:**
- Create: `elohim/elohim-storage/tests/sync_libp2p_convergence.rs`

**Interfaces:**
- Consumes: the swarm harness in `tests/harness/mod.rs` — `spawn_test_node` (`:132-180`), `dial`/`wait_for_connection` (`:674-700`); the DocStore+StreamTracker+SyncManager construction from `tests/sync_integration.rs:17-30`; `project_content_doc` (G1); `SyncManager::get_heads` (read-side).
- Produces: `#[tokio::test] async fn doc_authored_on_a_converges_to_b()`.

- [ ] **Step 1: Write the failing test.** Build a slim two-node harness carrying only `sync_protocol: request_response::Behaviour<SyncCodec>` + `identify` (copy the SwarmBuilder tcp+noise+yamux from `harness/mod.rs:132-180`, swap the behaviour). Each node wraps a real production `SyncManager` over its own temp `DocStore`.

```rust
#[tokio::test]
async fn doc_authored_on_a_converges_to_b() {
    // Two real storage SyncManagers, each on its own temp sled, joined by a real libp2p swarm.
    let mut node_a = TestSyncNode::spawn().await;   // builds SwarmBuilder(tcp+noise+yamux)+SyncCodec behaviour
    let mut node_b = TestSyncNode::spawn().await;
    node_b.dial(node_a.listen_addr()).await;
    node_b.wait_for_connection().await;

    // Author on A via the real producer path.
    let content = sample_content("edit-prop-1", "Edited v2");
    project_content_doc(&node_a.sync, &content).await;

    // Drive sync: B initiates a round (ListDocuments{h_app_id:"elohim"} → SyncChanges → apply_changes).
    let deadline = std::time::Duration::from_secs(30);
    let converged = poll_until(deadline, || async {
        node_b.sync.get_heads("elohim", "node:edit-prop-1").await
            .map(|h| !h.is_empty()).unwrap_or(false)
    }).await;
    assert!(converged, "node B did not converge node:edit-prop-1 within 30s");

    let title = node_b.sync.get_doc_field("elohim", "node:edit-prop-1", "title").await.unwrap();
    assert_eq!(title, "Edited v2", "converged value must match A");
}
```
(`poll_until` is a tiny local helper; reuse the poll idiom from the existing acceptance tests. If B's sync is timer-driven rather than callable, either lower the timer for the test or expose a `sync.run_round_now()` test hook — confirm how `initiate_sync_round` is triggered in `p2p/mod.rs` and prefer driving it directly in-test over waiting 60s.)

- [ ] **Step 2: Run it and confirm it fails** (before the harness/driver exist, or because convergence doesn't happen yet).

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/tmp/elohim-sync-test-target \
  cargo test --features p2p --test sync_libp2p_convergence -- --nocapture --test-threads=1
```
Expected: FAIL (compile error first, then assertion until the round is driven correctly). Add `BINDGEN_EXTRA_CLANG_ARGS` if clang-21 bindgen errors appear.

- [ ] **Step 3: Implement `TestSyncNode` + the sync driver** until convergence happens. Wire the inbound `SyncRequest` handler to call the real `handle_sync_request`/`apply_changes` path (reuse, don't reimplement — point the test behaviour at the production handlers in `p2p/mod.rs`).

- [ ] **Step 4: Run it and confirm it passes.** (same command) → PASS. Note wall-clock; if it relies on the 60s timer, switch to driving the round directly so the test is fast and deterministic.

- [ ] **Step 5: Baseline-sanity the existing merge test still passes.**

```bash
RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/tmp/elohim-sync-test-target \
  cargo test --features p2p --test sync_integration -- --nocapture
```
Expected: PASS (we added a sibling test file; did not touch merge logic).

- [ ] **Step 6: Commit.**

```bash
git add elohim/elohim-storage/tests/sync_libp2p_convergence.rs
git commit -m "test(sync): two-node libp2p convergence proof for content-sync plane"
```

---

## Spine gate (definition of done for the deliverable)

- [ ] **Pre-push gates green** on the touched crate:

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/tmp/elohim-sync-test-target cargo fmt --check
RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/tmp/elohim-sync-test-target cargo clippy --features p2p -- -D warnings
# Run the two targets SEPARATELY — a shared positional filter silently runs "0 tests" on one (see Execution Outcome).
RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/tmp/elohim-sync-test-target cargo test --features p2p --lib sync:: -- --nocapture
RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/tmp/elohim-sync-test-target cargo test --features p2p --test sync_libp2p_convergence -- --nocapture --test-threads=1
```

- [ ] **FLOOR verification (household-nodes, live stack):** `POST /db/content` on one M/J/J node, then read the peer's DocStore directly (storage :8090, NOT through doorway): `GET /sync/v1/elohim/docs` lists `node:{id}`. (If the `/sync/v1` read route isn't exposed on the storage surface yet, assert convergence via the integration test only and capture the read-route as the first browser-leg gap.)

- [ ] **STRONGER verification (shem, cross-tenant — now ONLINE):** author on one tenant's node, observe `node:{id}` + heads converge into a node under a *different* tenant (`p2p/status` Automerge-doc count increments). Proves the plane crosses the tenant boundary, not just the household LAN.

- [ ] **Story-harvest:** per CLAUDE.md, after the spine is green invoke `story-harvest` to scaffold an a2o regression scenario for content-sync convergence (parameter-bearing discovery: the `"elohim"` namespace coupling, the 30s convergence deadline).

---

## OPTIONAL STRETCH (parallel legs — do NOT gate the spine)

Both are **latent today** — no browser caller consumes doc-sync — and neither blocks node-to-node convergence. Develop in isolated worktrees if pursued (different surfaces, parallel-safe). **Each requires an operator confirmation before flipping a deliberate guard.**

### Stretch G5: Doorway `/sync` carriage (storage Rust + doorway Rust)
Two-surface change, no websocket, no new handler:
- (a) Declare `/sync/v1` GET routes + POST `/changes` in storage `build_manifest()` (`elohim/elohim-storage/src/http.rs:~10313`) and **flip the test that currently forbids them** (`http.rs:~12696`, `assert!(!paths…starts_with("/sync"))`) — this is a deliberate guard; **confirm with operator that exposing `/sync` is intended before flipping**.
- (b) Add `/sync` to doorway `is_service_path` (`doorway/doorway-service/src/server/http.rs:1928`) + the reserved-prefix fixture (`elohim/sdk/fixtures/route-claims.vectors.json`) + agreement test (`epr_router.rs:771`) + an `is_service_path` unit test. **Both gates required** or the EPR router shadows GET `/sync` to the SPA (the `/auth/portal` incident shape — see `project_doorway_main_route_needs_is_service_path`).

### Stretch G6: Frontend ESM migration (frontend)
- `@automerge/automerge` reaches the app transitively via `@elohim/storage-client`; the SDK already has `AutomergeSync` (`sdk/storage-client-ts/src/sync.ts`) + HTTP client, gated by a `new Function('return import(...)')` CJS-escape hack (`sync.ts:16-31`).
- Clean fix: flip SDK `tsconfig.json` `module: commonjs → ESNext` + `"type":"module"`, drop the hack; wasm is base64-inlined in automerge 3.x (no asset plumbing).
- Then a thin `ContentDocSyncService` polling `sync.sync(docId)` into a signal.
- **UNVERIFIED — smoke-test first:** that an Angular/esbuild build swallows automerge's inlined-wasm default entry without complaint. Verify before estimating this closed.

---

## Task G7: Resilience-card surfacing + browser-leg proof (operator-requested 2026-06-27)

Give the (tree-shaken) `ContentDocSyncService` a real consumer that (a) surfaces live per-content doc-sync on the resilience card and (b) genuinely exercises the browser leg. **Design decision** recorded in `specs/2026-06-19-resilience-facings-select-fold-aggregate-design.md` §12: doc-sync is a **client-composed sibling signal**, NOT a server-fold facing (plane separation + determinism contract). Per-content meaning: `synced` (doc present) / `pending` (null) + `updatedAt`. **P2P-design-gate: PASS** — pure read-projection over the existing `node:{contentId}` doc; no new entity/route/identity. Cross-references the weave sprint `plans/2026-06-21-resiliency-card-p2p-weave-sprint-plan.md` (which surfaces only blob-shard sync, never CRDT convergence).

**Deployment gate (honest):** the live shem cross-peer→browser proof is BLOCKED until the producer (spine, this branch) is merged+deployed — feature branches don't deploy. The runnable-now proof is the **local stack** (my producer) + the look-rail harness; shem cross-tenant is documented for post-deploy.

- **G7a `[frontend, sequential-blocker]`** — promote `ContentDocSyncService` from `app/elohim-app/src/app/elohim/services/content-doc-sync.service.ts` into `app/elohim-library/projects/elohim-service/src/` (carry the `AUTOMERGE_SYNC_FACTORY` token + the automerge base64 tsconfig alias); the card renders in the lamad bundle, so only the shared library can feed both surfaces.
- **G7b `[frontend, after G7a]`** — `<elohim-resilience-snapshot>` (`app/elohim-library/projects/elohim-service/src/resilience/resilience-snapshot/`): additive optional `@Input() docSync` + `get docSyncSummary()` (beside `peersLiveSummary`/`regionSummary`, `.component.ts:96-112`) + a "Doc sync" `<dt>/<dd>` row in BOTH `#contextBody` (after `.html:83`) and `#fullCardBody` (after `.html:111`), inheriting the existing `isUnmeasured` honesty (null → "not yet synced", never a fake "synced"). Unit test the summary + the null/honesty case.
- **G7c `[frontend, after G7a/G7b]`** — consume in lamad `content-viewer.component.ts:217-218` (`docSync$ = svc.watchContent(contentId)`) → `[docSync]` at `content-viewer.component.html:91-97`. **Verify lamad's build resolves the automerge alias** (the one new build-integration risk from promotion).
- **G7d `[frontend, parallel]`** — add `'/sync'` to the proxy context array in BOTH `app/elohim-app/proxy.conf.mjs` and `proxy.conf.alpha.mjs` (dev-proxy gap — `/sync/*` currently falls through to the SPA).
- **G7e `[frontend, parallel]`** — lazy `dev/doc-sync` harness route (`app/elohim-app/src/app/app.routes.ts:6`) + standalone component: inject `ContentDocSyncService`, read `id` query param, `watchContent(id)`, render fields with `data-testid` (`docsync-title`, `docsync-reach`, `docsync-body`). This un-tree-shakes the service and is the look-rail render target.
- **G7f `[test, after G7d/G7e + local stack]`** — browser-leg proof via the look rail: a non-bulk `POST /db/content` so `ContentCreated` projects `node:{id}`, then `pnpm look 'http://localhost:4200/dev/doc-sync?id=<id>' --wait-testid docsync-title`. Success: `shot.png` shows the converged fields; `capture.json` shows `GET …/sync/v1/elohim/docs/node%3A<id>/changes` → 200 non-empty, no `/sync` httpError, no pageerror.
- **G7g `[test, STRETCH/post-deploy]`** — shem cross-tenant: author on a non-Matthew peer, prove the libp2p round (~60s, libp2p-only) carries `node:{id}` into the browser's node. Runs once the producer is deployed.

### G7 Outcome (2026-06-27) — LANDED + VERIFIED (recovered after a mid-flight session crash)

The frontend agent crashed before committing; its work was recovered from the worktree, build-verified, and committed `1b70c0532`. G7a–G7e: ContentDocSyncService promoted to `@elohim/service`, "Doc sync" sibling row on `<elohim-resilience-snapshot>`, lamad consumer, `/sync` proxy, `/dev/doc-sync` harness. Verified: elohim-app + lamad dev builds green (automerge base64 alias resolves in all 3 contexts); harness 2/2, service 5/5, card 26/26.

**G7f browser-leg proof — PASSED (the deployed form):**
- Production `ng build` runtime via look rail (`reports/look/docsync-prod`): harness renders, **zero httpErrors, no `automerge_wasm_bg` fetch** — automerge base64 loads cleanly inline in a real browser.
- Full fetch→apply→render seam (`reports/look/docsync-converged`): served the production `dist` + a **wire-faithful fake `/sync`** (a REAL automerge change, same lib version) → the real `AutomergeSync` client fetched it over real `/sync` HTTP, applied it, and rendered the converged doc (`Status: synced`, all fields). Only the change ORIGIN was authored in node; cross-peer origin is proven by G3.

**NEW FINDING (backlogged `dev-serve-automerge-wasm-bundler-entry-gap.md`):** the automerge base64 alias works in the production build but NOT under `ng serve` (dev pre-bundles the wasm-bindgen bundler entry → `automerge_wasm_bg.wasm` 500). Doc-sync is broken under `pnpm start` local dev; production unaffected. The look rail caught it.

**G7g (shem cross-tenant) remains deployment-gated** — the producer + `/sync` carriage are committed but not deployed; the live cross-peer→browser proof runs once the integrator merges+deploys.

---

## Self-Review

**Spec coverage:** the user's spec = "next deliverable verifiable slice to get data syncing, workflow for parallel gap-closing." Covered: the slice = light the inert Automerge plane (G1-G4 spine); verifiable = G3 + two-tier live check; parallel = G5/G6 stretch on independent surfaces. The already-working DHT content plane and the 503 `signal/emit` carrier are deliberately out of scope (named in Architecture).

**Placeholder scan:** code steps carry real code; the few inferred signatures (`SyncManager::save`, `StorageEvent` payloads, `DocType` variants, the sync-round trigger) are explicitly flagged "confirm exact signature at <file:line>" because they require reading the live source — that is a verification instruction, not a TBD.

**Type consistency:** `content_doc_id`/`PROJECTION_NAMESPACE`/`project_content_doc`/`spawn_content_projection_listener` are used consistently across G1, G3, G4. `DocType::Content` introduced in G2 is consumed by G1's classification.

**Complementary work captured (not bloated in):** the content-edit DHT sweettest (`content_edit_visible_across_agents`, the OTHER sync plane) and the corpus back-fill migration are real adjacent work — captured to backlog, not folded into this slice.
