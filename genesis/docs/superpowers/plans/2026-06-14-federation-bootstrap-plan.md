# Bootstrap Store Islanding — per-pod in-memory → shared/coherent (F-BOOTSTRAP)

> For agentic workers: REQUIRED SUB-SKILL: superpowers:subagent-driven-development (or executing-plans). Steps use checkbox (- [ ]) syntax.
> Working draft — NOT cite-sealed. Authored against the Federation (web2/edge) Contract Ledger (`/projects/elohim/FEDERATION-WEB2-LEDGER-2026-06-14.md`) and reconciled with the P2P Dataplane Contract Ledger (`/projects/elohim/P2P-DATAPLANE-CONTRACT-LEDGER-2026-06-14.md`).

## 1. Context / why + the A/B-divergence facet it closes

**The facet.** The two doorways are islands by construction, and the bootstrap layer is the *first* island a peer hits — before any DHT gossip can converge. The kitsune2 bootstrap store is per-pod in-memory: `K2BootstrapStore { spaces: DashMap<String, DashMap<String, K2Entry>> }` (`bootstrap/k2.rs:51-53`), built fresh by `BootstrapStore::new()` (`bootstrap/store.rs:74-80`), instantiated with NO persistent backing at four AppState construction sites (`server/http.rs:404,504,613,724`). Cleanup is a local 60s task (`store.rs:223`). Zero cross-replica or cross-edge sync.

**The mechanism is pinpointed.** The deploy Jenkinsfile routes each conductor to a doorway by persona: `isRemote ? doorwayB : doorwayA` (`elohim/holochain/Jenkinsfile:594-596`), where `doorwayA.bootstrapUrl = https://doorway-alpha.elohim.host/bootstrap` and `doorwayB.bootstrapUrl = https://elohim.host/bootstrap` (`Jenkinsfile:317-324`). So **matthew (local/genesis) PUTs its agent-info ONLY into doorway-A's in-memory store; adam (remote/shem/genesis) PUTs ONLY into doorway-B's**. The two peers that MUST find each other to seed the DHT publish into two disjoint tables that never reconcile. Every pod restart wipes the table ("fresh per boot"). This is the structural islanding root that makes the two-EPR-head / cross-edge-divergence symptom *possible at all*: if the genesis pair cannot mutually discover, the conductor-level DHT gossip that F-COHERENCE relies on to keep the two edges' EPR heads coherent never has a path to converge.

**What this plan closes.** Move the k2 bootstrap store behind a `trait K2Store` and add a mongo-backed implementation (`MongoK2Store`) on a FIXED shared database (`elohim-bootstrap`), so both doorways read/write ONE bootstrap table. Both pods already reach the same mongo server (`alpha.yaml:154` and `alpha-b.yaml:184` both set `MONGODB_URI=mongodb://mongodb.elohim-alpha:27017`; they differ only in per-doorway `MONGODB_DB`). Mongo TTL on `expires_at` replaces the local cleanup task and survives pod restart, killing "fresh per boot." A `GET /admin/bootstrap-coherence` Cat-C read-model makes per-space/per-agent skew observable. The genesis pair converges on one table; cross-edge discovery becomes a DHT-gossip problem (the dataplane's), not a bootstrap-island problem.

**Scope discipline.** This plan is the bootstrap-store layer ONLY. It touches NO swarm/behaviour/libp2p file (that is dataplane P-TRANSPORT), NO `P2PStatusInfo`/`self_healing.rs` (P-DIAGNOSTIC), NO conductor URL routing in the Jenkinsfile (URLs stay; the stores reconcile *underneath*). The legacy kitsune1 store (`store.rs` DashMap path) has the same defect but is dead weight for the HC-0.6 genesis pair — scoped out to a documentation note (Task 6).

---

## 2. OWNED FILES (verbatim from federation ledger §2) + collision statement

**MUTATE (M):**
- `doorway/doorway-service/src/bootstrap/k2.rs` — SOLE owner. Refactor `K2BootstrapStore` → `trait K2Store` + rename current impl `MemK2Store`. (Ledger §2 F-BOOTSTRAP.)
- `doorway/doorway-service/src/bootstrap/store.rs` — SOLE owner. Thread a `Box<dyn K2Store>` (env-selected) into `BootstrapStore::new`.
- `doorway/doorway-service/src/bootstrap/mod.rs` — SOLE owner (re-exports for the new trait + mongo impl; ledger lists `bootstrap/*` as F-BOOTSTRAP territory).
- `doorway/doorway-service/src/db/schemas/mod.rs` — SOLE owner of the additive `pub mod bootstrap_entry;` line.
- `doorway/doorway-service/src/server/http.rs` — ONE additive route arm registering `GET /admin/bootstrap-coherence`, plus an `.await` at the two existing k2 call sites (`:3418`, `:3456`) — see SEAM-DELTA in §10. **Additive append-only per ledger C-HTTP** (F-COHERENCE registers a disjoint `/api/v1/federation/coherence` arm; F-BOOTSTRAP registers `/admin/bootstrap-coherence`; mechanical merge).
- `genesis/orchestrator/manifests/doorway/alpha.yaml` + `alpha-b.yaml` — add `BOOTSTRAP_MONGODB_DB=elohim-bootstrap` env to BOTH (ledger C-MANIFEST: additive-disjoint — F-BOOTSTRAP owns the `env:` block additions; F-EDGE owns annotations; F-DEPLOY owns alpha-b posture).

**CREATE (C):**
- `doorway/doorway-service/src/bootstrap/k2_mongo.rs` — SOLE owner (`MongoK2Store`).
- `doorway/doorway-service/src/db/schemas/bootstrap_entry.rs` — SOLE owner (`BootstrapEntryDoc` + TTL + unique index).
- `doorway/doorway-service/src/routes/bootstrap_coherence.rs` — SOLE owner (the `/admin/bootstrap-coherence` read-model; isolated in a NEW module so it does NOT touch F-COHERENCE's `routes/coherence.rs`).

**Collision statement.** Every file above is SOLE-owned by F-BOOTSTRAP except the two shared additive-merge files explicitly resolved in the ledger:
- `server/http.rs` — additive append-only route arm (ledger C-HTTP); F-COHERENCE adds a disjoint arm; no key collision.
- `genesis/orchestrator/manifests/doorway/{alpha,alpha-b}.yaml` — additive-disjoint 3-way (ledger C-MANIFEST); F-BOOTSTRAP writes only the `env:` `BOOTSTRAP_MONGODB_DB` key; no other plan writes that key.

This plan touches **no file owned by another federation plan** (F-COHERENCE's `routes/coherence.rs`/`services/federation.rs`, F-EDGE's `routes/federation.rs`, F-DEPLOY's `Jenkinsfile`/`verify-pair-coherence.sh`) and **no dataplane-owned file** (verified against dataplane ledger §2: nothing here is in `elohim-storage/*`, `elohim-compute/*`, `steward/*`, DNA, or `sdk/schemas/*`). `db::MongoClient` (`db/mongo.rs:75 inner()`) is consumed read-only, not mutated.

---

## 3. NEW PRIMITIVES owned + CONSUMED (skip-if-present)

### Owned (F-BOOTSTRAP)

| Primitive | Home | Shape (canonical, from ledger §3) |
|---|---|---|
| `trait K2Store` | `doorway::bootstrap::k2` | `#[async_trait] { async fn put_at(&self, space:&str, agent:&str, body:&[u8], now_micros:i64) -> Result<K2PutOutcome,String>; async fn get_at(&self, space:&str, now_micros:i64) -> Vec<u8>; async fn cleanup(&self, now_micros:i64) -> usize; async fn stats(&self) -> (usize,usize); }` — mirrors current INHERENT methods so handler semantics are unchanged. **SEAM-DELTA: async** (mongo ops are async; see §10). |
| `struct MemK2Store` | `doorway::bootstrap::k2` | Current `K2BootstrapStore` renamed; `#[async_trait] impl K2Store` wrapping the existing sync DashMap bodies (zero behavior change; the validation/parse core stays inherent + sync). |
| `struct MongoK2Store` | `doorway::bootstrap::k2_mongo` | `{ coll: Collection<BootstrapEntryDoc> }` over `db::MongoClient::inner()` against the `elohim-bootstrap` DB; `#[async_trait] impl K2Store`. |
| `struct BootstrapEntryDoc` | `doorway::db::schemas::bootstrap_entry` | `{ _id, metadata: Metadata, space: String, agent: String, raw_body: Vec<u8>, expires_at: bson::DateTime }` + `impl IntoIndexes` (TTL `expire_after(Duration::from_secs(0))` on `expires_at`; unique compound `(space,agent)`). |
| `struct BootstrapCoherenceView` | `doorway::routes::bootstrap_coherence` | `#[serde(rename_all="camelCase")] { backend: String, spaces: usize, agents: usize, per_space: Vec<SpaceSkew> }`; `SpaceSkew { space: String, agents: usize }`. Cat-C read-model. |

### Consumed — skip-if-present (verbatim clause from ledger)

*"Before landing this type, verify the named owner module already exposes it. If present, VERIFY-ONLY (import + use). If absent at your integration point, land the owner plan's verbatim definition only as a temporary local shim, flag it in your plan's hand-off notes, and delete the shim when the owner lands."*

| Consumed primitive | Owner | Edge | Use |
|---|---|---|---|
| `db::MongoClient` + `MongoClient::inner() -> &Client` (FS8) | doorway-local (BUILT, `db/mongo.rs:75`) | — | `MongoK2Store` calls `client.database("elohim-bootstrap").collection(...)` via `inner()` to target a DB independent of the per-doorway `db_name`. VERIFY-ONLY. |
| `IntoIndexes` / `MutMetadata` / `Metadata` (BUILT, `db/mongo.rs:19`, `db/schemas/metadata.rs`) | doorway-local | — | `BootstrapEntryDoc` implements `IntoIndexes` (TTL + unique) exactly as `oauth_session.rs:238` (`expire_after(0)` on `expires_at`) and `host.rs:155` (unique). VERIFY-ONLY. |
| `jittered(base,max,attempt)` (dataplane FS12 / P-DEFENSE S7, `elohim_compute::backoff`) | **dataplane P-DEFENSE** | **X-BOOT-DEF, SOFT** | mongo reconnect / startup index-create retry cadence ONLY. **Skip-if-present shim:** if `elohim_compute::backoff::jittered` is absent at integration, use a local 3-line `fn jittered_shim` (flagged in hand-off notes) and delete on integration. The store functions without it (mongo driver has its own connection retry); jitter only smooths startup. |

---

## 4. DEPENDENCY EDGES

### Intra-federation (from ledger §4 DAG)
F-BOOTSTRAP is a **producing ROOT** — zero inbound HARD federation edges; it is the genesis-pair islanding root fix, fully independent of the other three federation plans.

| Edge | Type | Reason |
|---|---|---|
| F-COHERENCE → F-BOOTSTRAP | **SOFT (outbound, on us)** | cross-edge head agreement is only *achievable* once the genesis pair shares a bootstrap table and converges DHT; F-COHERENCE can DETECT divergence regardless. We do not block F-COHERENCE. |
| F-DEPLOY → F-BOOTSTRAP | **SOFT (outbound, on us)** | the partition guard is most valuable once islanding is fixed; the guard works independent of bootstrap sharing. We do not block F-DEPLOY. |
| F-BOOTSTRAP → (any federation plan) | **NONE** | fully independent; nothing internal HARD-blocks on us, we HARD-block on nothing internal. |

### Cross-layer (federation → dataplane; ledger §5)

| Edge id | → Dataplane track | Type | Contract |
|---|---|---|---|
| **X-BOOT-TRANS** | P-TRANSPORT (libp2p peer-discovery / `connection_limits`, S14) | **SOFT** | shared bootstrap fixes the discovery SEED; P-TRANSPORT owns the libp2p swarm discovery once seeded. F-BOOTSTRAP touches NO swarm/behaviour file — strictly the pre-DHT bootstrap-store layer. Complementary, not blocking. |
| **X-BOOT-DEF** | P-DEFENSE (`jittered`, S7) | **SOFT** | mongo reconnect / index-create cadence consumes `jittered`; skip-if-present shim until it lands. |
| **X-BOOT-DIAG** | P-DIAGNOSTIC (`SelfHealingView.anchor` block) | **SOFT** | the `/admin/bootstrap-coherence` read-model could later soft-feed P-DIAGNOSTIC's anchor block; F-BOOTSTRAP declares the future consume, does NOT define P-DIAGNOSTIC's schema, does NOT mutate `self_healing.rs`. |

**Cycle check:** none. All edges either point AT us (SOFT) or are SOFT cross-layer consumes. F-BOOTSTRAP can land first, alone.

---

## 5. Build / test commands (per-crate RUSTFLAGS + /tmp target + plain cargo)

All work is in `doorway/doorway-service` — **native crate, RUSTFLAGS MUST be empty.**

```
# Unit tests for a single module under work:
cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib bootstrap 2>&1 | tail -40
cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib bootstrap_coherence 2>&1 | tail -40

# Mongo integration test (Task 5) — gated #[ignore], needs a reachable mongo:
cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib mongo_k2 -- --ignored 2>&1 | tail -40

# Final gates:
cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins 2>&1 | tail -40
cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo clippy -- -D warnings 2>&1 | tail -40
cd /projects/elohim/doorway/doorway-service && cargo fmt --check
```

Rules (memory): `RUSTFLAGS=""` for doorway native; `RUSTC_WRAPPER=""` (sccache spawn-ENOENT); `/tmp` target dir (fingerprint-ENOENT on pool slot); **plain `cargo test`, NEVER nextest**; never `&&`-pipe a gate exit code (use `2>&1 | tail -N`). Manifest tasks (Task 7) are **doc/lint only** — `git diff` review + YAML sanity, no cargo.

---

## 6. p2p-class of new entities (p2p-design-gate)

All new entities are **Cat-C node/edge-local operational read-models or caches** — none notarized, none a DHT entry, no coordinator fn, no new content-addressed identity:

- `BootstrapEntryDoc` — **Cat-C operational.** Edge-local ephemeral discovery cache, TTL-bounded. Bootstrap is *pre-DHT* infrastructure (it exists to let peers find each other so the DHT can form). NOT notarized. Identity is the kitsune `(space, agent)` b64url composite (the protocol-given key), NOT a CID — justified: the agent-info body is itself signed (ed25519-validated at `k2.rs` PUT), so the bootstrap row is a transient mirror of a signed external artifact, not a source of truth.
- `K2Store` / `MemK2Store` / `MongoK2Store` — storage-backend abstractions, no entity class.
- `BootstrapCoherenceView` / `SpaceSkew` — **Cat-C** node-local read-model (counts only).

No Cat-A actuation in this surface. (The federation ledger's one Cat-A item is F-DEPLOY's coordinator-update flag, not ours.) **Do not re-litigate; class cited.**

---

## 7. Task-by-task (TDD)

> Sequencing within the plan: Task 1 (trait extraction) is the unblock-everything refactor and must land first. Tasks 2–5 build on it. Task 6 (legacy doc) and Task 7 (manifests) are independent and can run any time.

### TASK 1 — Extract `trait K2Store`; rename impl `MemK2Store` (zero behavior change)

Files: `bootstrap/k2.rs` (SOLE), `bootstrap/mod.rs` (re-export), `bootstrap/store.rs` (hold trait object), `server/http.rs` (the two call sites become `.await`).

- [ ] Write the failing test FIRST — append to `k2.rs` `#[cfg(test)] mod tests`, asserting the store is usable through the trait object (this fails to compile until the trait exists):
```rust
    #[tokio::test]
    async fn mem_store_roundtrips_through_trait_object() {
        let s: Box<dyn K2Store> = Box::new(MemK2Store::new());
        // PUT a valid signed body via the inherent validated path, then GET via trait.
        // (reuse the existing valid-body fixture helper in this test module)
        let now = current_time_micros();
        let body = valid_agent_info_body();           // existing test helper
        let (space_b64, agent_b64) = fixture_space_agent(); // existing test helper
        let out = s.put_at(&space_b64, &agent_b64, &body, now).await.unwrap();
        assert_eq!(out, K2PutOutcome::Stored);
        let got = s.get_at(&space_b64, now).await;
        assert!(got.starts_with(b"[") && got.len() > 2, "non-empty array");
        assert_eq!(s.stats().await, (1, 1));
    }
```
- [ ] Run, expect FAIL (no `K2Store` / no `MemK2Store`): `cargo test --lib bootstrap` (full cmd §5).
- [ ] Implement:
  1. `use async_trait::async_trait;` at top of `k2.rs`.
  2. Define the trait above the impl:
```rust
/// Storage backend for kitsune2 bootstrap agent-info. Async so a mongo-backed
/// impl can share one table across doorway replicas/edges (kills genesis-pair
/// islanding). The PUT VALIDATION (ed25519 + path-match + span cap) stays an
/// inherent sync helper; only the STORAGE verbs are abstracted here.
#[async_trait]
pub trait K2Store: Send + Sync {
    async fn put_at(&self, space_b64: &str, agent_b64: &str, body: &[u8], now_micros: i64)
        -> Result<K2PutOutcome, String>;
    async fn get_at(&self, space_b64: &str, now_micros: i64) -> Vec<u8>;
    async fn cleanup(&self, now_micros: i64) -> usize;
    async fn stats(&self) -> (usize, usize);
}
```
  3. Rename `pub struct K2BootstrapStore` → `pub struct MemK2Store` (keep `#[derive(Default)]`, keep inherent `new()`, keep the inherent `ParsedAgentInfo::parse` validation core unchanged).
  4. Move the bodies of the current inherent `put_at`/`get_at`/`cleanup`/`stats` into `#[async_trait] impl K2Store for MemK2Store` (they stay synchronous internally — DashMap — just wrapped in async fns). Keep `put`/`get` (clock-using wrappers) inherent OR move callers to `put_at`/`get_at` with `current_time_micros()`. **Note:** current `cleanup`/`stats` take no `now`; align to the trait's `now_micros` param (cleanup) and drop the param mismatch — `stats()` keeps `()`.
  5. `bootstrap/mod.rs`: change `pub use k2::{K2BootstrapStore, K2PutOutcome};` → `pub use k2::{K2Store, MemK2Store, K2PutOutcome};`.
  6. `bootstrap/store.rs`: change field `k2: super::k2::K2BootstrapStore` → `k2: Box<dyn super::k2::K2Store>`; `BootstrapStore::new()` constructs `Box::new(MemK2Store::new())`; `k2()` returns `&dyn K2Store`; `cleanup()` becomes `async` (it calls `self.k2.cleanup(now).await`) OR keep the legacy DashMap cleanup sync and only the k2 leg awaits — see §10 SEAM-DELTA on `spawn_cleanup_task`.
  7. `server/http.rs:3418` `store.k2().put(...)` → `store.k2().put_at(&space_b64, &agent_b64, &body, current_time_micros()).await`; `:3456` `store.k2().get(rest)` → `store.k2().get_at(rest, current_time_micros()).await` (handlers are already `async`).
- [ ] Run, expect PASS: `cargo test --lib bootstrap`.
- [ ] `cargo fmt` + `cargo clippy -- -D warnings`.
- [ ] Commit (selective-stage): `git add src/bootstrap/k2.rs src/bootstrap/mod.rs src/bootstrap/store.rs src/server/http.rs` →
```
feat(doorway): extract async trait K2Store; rename impl MemK2Store

Storage verbs behind a trait so a shared mongo-backed bootstrap table can
kill genesis-pair islanding. No behavior change (10 k2 unit tests green).

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
```

### TASK 2 — `BootstrapEntryDoc` schema (TTL + unique index)

Files: `db/schemas/bootstrap_entry.rs` (C), `db/schemas/mod.rs` (M, additive `pub mod`).

- [ ] Write the failing test FIRST — in `bootstrap_entry.rs` `#[cfg(test)] mod tests`:
```rust
    #[test]
    fn doc_declares_ttl_and_unique_indexes() {
        let idx = BootstrapEntryDoc::default().into_indices();
        let names: Vec<_> = idx.iter()
            .filter_map(|(_, o)| o.name.clone()).collect();
        assert!(names.iter().any(|n| n == "bootstrap_expires_at_ttl"));
        assert!(names.iter().any(|n| n == "bootstrap_space_agent_unique"));
    }
```
  (Mirror the exact `into_indices()` test shape used in `oauth_session.rs`/`host.rs` test modules.)
- [ ] Run, expect FAIL (no module).
- [ ] Implement, modeled on `oauth_session.rs` (TTL `expire_after(0)` on `expires_at`) + `host.rs` (unique compound). Mongo TTL fires on `expires_at` reaching wall-clock, replacing the in-pod cleanup task:
```rust
pub const BOOTSTRAP_COLLECTION: &str = "k2_bootstrap";

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct BootstrapEntryDoc {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _id: Option<ObjectId>,
    #[serde(default)]
    pub metadata: Metadata,
    pub space: String,         // b64url space id
    pub agent: String,         // b64url agent id
    pub raw_body: Vec<u8>,     // verbatim signed agent-info (bson Binary)
    pub expires_at: DateTime,  // TTL key (from the signed body's expires_at)
}

impl IntoIndexes for BootstrapEntryDoc {
    fn into_indices(self) -> Vec<(Document, IndexOptions)> {
        vec![
            (doc! { "expires_at": 1 },
             IndexOptions::builder()
                 .expire_after(std::time::Duration::from_secs(0))
                 .name("bootstrap_expires_at_ttl".to_string()).build()),
            (doc! { "space": 1, "agent": 1 },
             IndexOptions::builder()
                 .unique(true)
                 .name("bootstrap_space_agent_unique".to_string()).build()),
        ]
    }
}
```
  Add `pub mod bootstrap_entry;` to `db/schemas/mod.rs` (additive; do NOT re-export into the crowded `db::` top-level `pub use` unless needed — `MongoK2Store` imports the module path directly).
- [ ] Run, expect PASS: `cargo test --lib bootstrap_entry`.
- [ ] `cargo fmt` + clippy. Commit: `git add src/db/schemas/bootstrap_entry.rs src/db/schemas/mod.rs` → `feat(doorway): BootstrapEntryDoc with TTL + unique (space,agent) index`.

### TASK 3 — `MongoK2Store` (the islanding fix)

Files: `bootstrap/k2_mongo.rs` (C), `bootstrap/mod.rs` (M, additive re-export).

- [ ] Write the failing test FIRST — a sync unit asserting the validated PUT path still rejects bad bodies before any mongo call (so the test needs no live mongo); the cross-pod proof is the `#[ignore]` integration test in Task 5:
```rust
    #[tokio::test]
    async fn mongo_store_rejects_invalid_body_before_db() {
        // Construct MongoK2Store with a never-connected collection is awkward;
        // instead assert the shared validation helper is reused: extract
        // `validate_put(space,agent,body,now) -> Result<ParsedAgentInfo,String>`
        // as a pub(crate) fn in k2.rs and unit-test it here (no DB).
        let e = crate::bootstrap::k2::validate_put("bad", "bad", b"{}", 0);
        assert!(e.is_err());
    }
```
- [ ] Run, expect FAIL.
- [ ] Implement:
  1. In `k2.rs` (Task-1 module), extract the PUT validation core (currently inline in the inherent `put_at`) into `pub(crate) fn validate_put(space_b64,agent_b64,body,now_micros) -> Result<ParsedAgentInfo,String>` so BOTH `MemK2Store` and `MongoK2Store` reuse the identical ed25519/path-match/span-cap logic (no validation drift between backends). `MemK2Store::put_at` calls it then inserts into DashMap. **This is the single most important coherence guarantee — validation MUST NOT fork.**
  2. `k2_mongo.rs`: `MongoK2Store { coll: Collection<BootstrapEntryDoc> }`. Construct from `&MongoClient` + the shared DB name:
```rust
pub fn new(mongo: &MongoClient, db_name: &str) -> Self {
    let coll = mongo.inner()
        .database(db_name)            // e.g. "elohim-bootstrap" — NOT mongo.db_name()
        .collection::<BootstrapEntryDoc>(BOOTSTRAP_COLLECTION);
    Self { coll }
}
```
     `#[async_trait] impl K2Store`:
       - `put_at`: `validate_put(...)?`; on `Stored`, `update_one` with upsert keyed on `(space,agent)` setting `raw_body`+`expires_at` (mongo TTL drops it; the unique index enforces one row per agent — replaces the per-pod arbitrary `.iter().next()` eviction at `k2.rs:156`); on `Removed` (tombstone), `delete_one`.
       - `get_at`: `find_many({"space":space, "expires_at":{"$gt": now}})`, assemble the SAME `[...]` byte array `MemK2Store::get_at` produces (reuse the assembly).
       - `cleanup`: no-op `0` (mongo TTL owns expiry) — doc-comment it.
       - `stats`: `count_documents` total + `distinct("space")` len.
  3. `bootstrap/mod.rs`: add `pub mod k2_mongo; pub use k2_mongo::MongoK2Store;`.
- [ ] Run, expect PASS: `cargo test --lib bootstrap`.
- [ ] clippy + fmt. Commit: `git add src/bootstrap/k2_mongo.rs src/bootstrap/k2.rs src/bootstrap/mod.rs` → `feat(doorway): MongoK2Store — shared bootstrap table (genesis-pair islanding fix)`.

### TASK 4 — Env-select mem-vs-mongo in `BootstrapStore::new`

Files: `bootstrap/store.rs` (SOLE).

- [ ] Write the failing test FIRST — assert the constructor honors `BOOTSTRAP_MONGODB_DB`:
```rust
    #[tokio::test]
    async fn bootstrap_store_selects_mem_when_no_mongo() {
        let s = BootstrapStore::new(None);              // None mongo => mem
        assert_eq!(s.k2().stats().await, (0, 0));
    }
```
- [ ] Run, expect FAIL (signature mismatch).
- [ ] Implement: change `BootstrapStore::new()` → `BootstrapStore::new(mongo: Option<&MongoClient>) -> Self`. Inside:
```rust
let k2: Box<dyn K2Store> = match (mongo, std::env::var("BOOTSTRAP_MONGODB_DB").ok()) {
    (Some(m), Some(db)) => Box::new(MongoK2Store::new(m, &db)),
    _ => Box::new(MemK2Store::new()),
};
```
  Update the FOUR call sites in `server/http.rs` (`:404,:504,:613,:724`) from `BootstrapStore::new()` → `BootstrapStore::new(mongo.as_ref())` (each site already has the `mongo: Option<MongoClient>` in scope — verified `http.rs:118 pub mongo: Option<MongoClient>`). **Index creation:** `MongoK2Store::new` (or a one-time init in `with_services`) must call `create_indexes` for `BootstrapEntryDoc` (mirror how `MongoCollection::new` creates indices via `IntoIndexes`); if `inner().database(db).collection()` bypasses the `MongoCollection` index path, add an explicit `coll.create_indexes(...)` in `new` (idempotent). Flag in hand-off if the index-create wiring needs `with_services` placement.
- [ ] Run, expect PASS. clippy + fmt. Commit: `git add src/bootstrap/store.rs src/server/http.rs` → `feat(doorway): BootstrapStore env-selects MongoK2Store via BOOTSTRAP_MONGODB_DB`.

### TASK 5 — Cross-pod proof (`#[ignore]` integration test)

Files: `bootstrap/k2_mongo.rs` test module (SOLE).

- [ ] Write an `#[ignore]`-gated integration test: two `MongoK2Store` handles over the SAME `elohim-bootstrap` DB but constructed from two `MongoClient`s with DISTINCT `db_name` (`doorway-alpha` vs `doorway-alpha-b`) — proving the shared bootstrap table is independent of the per-doorway DB. PUT a valid signed body on handle A; GET on handle B returns it. **This is the cross-edge proof current per-pod tests cannot express.**
```rust
    #[tokio::test]
    #[ignore = "needs a reachable mongo at MONGODB_TEST_URI"]
    async fn put_on_a_is_visible_on_b_across_distinct_db_names() {
        let uri = std::env::var("MONGODB_TEST_URI").unwrap();
        let a = MongoClient::new(&uri, "doorway-alpha").await.unwrap();
        let b = MongoClient::new(&uri, "doorway-alpha-b").await.unwrap();
        let sa = MongoK2Store::new(&a, "elohim-bootstrap");
        let sb = MongoK2Store::new(&b, "elohim-bootstrap");
        let now = current_time_micros();
        let (space, agent, body) = signed_fixture();
        sa.put_at(&space, &agent, &body, now).await.unwrap();
        let got = sb.get_at(&space, now).await;
        assert!(got.windows(body.len()).any(|w| w == &body[..]), "B sees A's PUT");
    }
```
- [ ] Run (only when a mongo is reachable): `cargo test --lib mongo_k2 -- --ignored` (full cmd §5). Otherwise it is skipped by default (`#[ignore]`), keeping the default suite green with no infra.
- [ ] Commit: `git add src/bootstrap/k2_mongo.rs` → `test(doorway): cross-pod bootstrap visibility proof (#[ignore], shared elohim-bootstrap DB)`.

### TASK 6 — `GET /admin/bootstrap-coherence` divergence read-model (Cat-C)

Files: `routes/bootstrap_coherence.rs` (C), `server/http.rs` (M, ONE additive route arm).

- [ ] Write the failing test FIRST — pure composer over `(agents, spaces, per_space)` counts:
```rust
    #[test]
    fn coherence_view_serializes_camel_case() {
        let v = BootstrapCoherenceView {
            backend: "mongo".into(), spaces: 1, agents: 2,
            per_space: vec![SpaceSkew { space: "s".into(), agents: 2 }],
        };
        let j = serde_json::to_string(&v).unwrap();
        assert!(j.contains("\"perSpace\""), "{j}");
        assert!(j.contains("\"backend\":\"mongo\""), "{j}");
    }
```
- [ ] Run, expect FAIL.
- [ ] Implement: `BootstrapCoherenceView` + `SpaceSkew` (`#[serde(rename_all="camelCase")]`). Handler `handle_bootstrap_coherence(state) -> Response` reads `state.bootstrap.k2().stats().await` for totals and (for the mongo backend) a per-space aggregation; `backend` reports `"mongo"|"mem"`. **Reuses `bootstrap.stats()`/`k2.stats()` as-is** (`store.rs:201`, `k2.rs:221`). Register ONE arm in the `server/http.rs` route match for `GET /admin/bootstrap-coherence` (additive append-only per ledger C-HTTP — disjoint from F-COHERENCE's `/api/v1/federation/coherence`). Add `pub mod bootstrap_coherence;` to `routes/mod.rs`.
- [ ] Run, expect PASS: `cargo test --lib bootstrap_coherence`.
- [ ] clippy + fmt. Commit: `git add src/routes/bootstrap_coherence.rs src/routes/mod.rs src/server/http.rs` → `feat(doorway): GET /admin/bootstrap-coherence read-model (Cat-C)`.

### TASK 7 — Manifests: `BOOTSTRAP_MONGODB_DB` env on BOTH doorways (doc/lint only)

Files: `genesis/orchestrator/manifests/doorway/alpha.yaml` + `alpha-b.yaml` (M, additive `env:` key per ledger C-MANIFEST).

- [ ] In BOTH files' container `env:` block (adjacent to the existing `MONGODB_URI`/`MONGODB_DB` entries — `alpha.yaml:154`, `alpha-b.yaml:184`), add:
```yaml
        - name: BOOTSTRAP_MONGODB_DB
          value: "elohim-bootstrap"
```
  The VALUE is identical in both files (the whole point — one shared table). Do NOT change `MONGODB_DB` (it stays per-doorway for the projection cache). No cargo; verify by `git diff` + a YAML indentation check.
- [ ] Commit: `git add genesis/orchestrator/manifests/doorway/alpha.yaml genesis/orchestrator/manifests/doorway/alpha-b.yaml` → `feat(manifests): share k2 bootstrap table across doorways via BOOTSTRAP_MONGODB_DB`.

### TASK 8 — Legacy kitsune1 store note (doc only)

Files: `bootstrap/store.rs` doc-comment (SOLE).

- [ ] Add a `// LEGACY:` doc-comment on the legacy kitsune1 DashMap path in `store.rs` recording that it has the SAME islanding defect but is dead weight for the HC-0.6 genesis pair (k2 exclusive); a mongo-backing migration is deferred unless a kitsune1 peer reappears. (Recommendation S/LOW from the review.)
- [ ] Commit: `git add src/bootstrap/store.rs` → `docs(doorway): note legacy kitsune1 bootstrap store islanding (dead-for-genesis)`.

---

## 8. // FOLLOW-ON seams (for the integration pass / named siblings)

1. **`jittered` consume (X-BOOT-DEF).** If P-DEFENSE's `elohim_compute::backoff::jittered` lands before integration, replace the local `jittered_shim` (if used) in `MongoK2Store` index-create retry with the real one; delete the shim. SOFT.
2. **`/admin/bootstrap-coherence` → P-DIAGNOSTIC anchor block (X-BOOT-DIAG).** P-DIAGNOSTIC owns `SelfHealingView.anchor` / `self_healing.rs`. A future one-line consume can fold `BootstrapCoherenceView.spaces/agents` into the anchor surface. We declare the consume; we do NOT mutate `self_healing.rs`. SOFT, integration-pass-only.
3. **DHT-seed convergence belongs to P-TRANSPORT (X-BOOT-TRANS).** Once the shared table seeds discovery, the libp2p swarm discovery + `connection_limits` floor are P-TRANSPORT's. F-BOOTSTRAP stops at the pre-DHT layer. Complementary.
4. **Multi-replica scale-out (Axis-1).** Shared mongo backing ALSO fixes the per-pod arbitrary per-space cap eviction (`k2.rs:156` `.iter().next()`) for a future >1-replica doorway; the unique `(space,agent)` index makes the cap deterministic across replicas. Noted for the scale-out plan; no work here beyond the index already landing in Task 2.
5. **`spawn_cleanup_task` async-ification.** `BootstrapStore::cleanup` becomes partly async (k2 leg awaits, mongo leg is a no-op). The cleanup spawn loop (`store.rs:223`) may keep the legacy-DashMap sync cleanup and `.await` only the k2 leg, or be retired entirely for the mongo backend (TTL owns expiry). Integration decides; Task 1 keeps it compiling either way.

---

## 9. Dispatch note

- **Isolated-worktree, subagent-driven, commit-only.** Run from a dedicated worktree off the integration branch (Wave **F1** root — F-BOOTSTRAP is fully independent; start immediately, in parallel with F-COHERENCE). The integrator pushes/merges (memory: commit-only; never `git push`).
- **Internal order:** Task 1 first (trait extraction unblocks 2–5); Tasks 6 (read-model), 7 (manifests), 8 (doc) are independent and may interleave. The mongo proof (Task 5) is `#[ignore]` — default suite stays green with no infra.
- **Selective-stage** each commit (concurrent sessions share the worktree per memory) — the per-task `git add` lists name exact files only; never bulk-revert ambient mods.
- **RUSTFLAGS discipline:** doorway is native → `RUSTFLAGS=""` everywhere here. Do NOT carry the elohim-storage WASM getrandom flag into this crate (link-fails `undefined __getrandom_v03_custom`).
- **No cluster ops.** Manifest edits make the repo coherent for the next pipeline; never `kubectl`. Do NOT cite-seal (working draft).

---

## 10. SEAM-DELTA — discovered, not in either ledger

| Delta | Detail |
|---|---|
| **`K2Store` MUST be async, not sync** | The federation ledger §3 lists `trait K2Store { fn put_at(...) -> Result; ... }` (sync, mirroring current inherent methods). But mongo driver ops (`update_one`/`find_many`/`count_documents`) are async, and the PUT/GET handlers (`http.rs handle_k2_bootstrap_put/get`) are ALREADY `async fn`. So the trait must be `#[async_trait]` and the two existing call sites (`http.rs:3418`, `:3456`) gain `.await`. `async-trait = "0.1"` is present and used (`ssr.rs:24`, `admin_users.rs:50`), so no new dep. This changes the trait descriptor from the ledger's sync shape — captured in §3 and Task 1. (The validation core stays sync via `pub(crate) fn validate_put`.) |
| **`validate_put` must be extracted to prevent backend validation drift** | Not in the ledger. The ed25519 + path-match + 30-min span-cap validation currently lives INLINE in the inherent `put_at`. Both `MemK2Store` and `MongoK2Store` must call the IDENTICAL logic or the two backends could diverge on what they accept — re-introducing a subtle island. Task 3 extracts `pub(crate) fn validate_put` as the single source. This is a hard coherence requirement, not optional. |
| **`BootstrapStore::new()` signature changes** (`() -> Self` → `(Option<&MongoClient>) -> Self`) | The ledger says "thread a `Box<dyn K2Store>` handle into `BootstrapStore::new`" but the four call sites (`http.rs:404,504,613,724`) currently call `BootstrapStore::new()` with no args. Each site has `mongo: Option<MongoClient>` in scope, so the change is mechanical, but it IS a signature change at four sites — noted so the integrator doesn't treat it as additive-only. |
| **Index creation placement** | `MongoClient::inner().database(db).collection()` BYPASSES the `MongoCollection::new` path that auto-creates `IntoIndexes` indices. So `MongoK2Store::new` (or a one-time `with_services` init) must explicitly `create_indexes` for `BootstrapEntryDoc`. Not a blocker (idempotent), but a wiring detail absent from the ledger. |
