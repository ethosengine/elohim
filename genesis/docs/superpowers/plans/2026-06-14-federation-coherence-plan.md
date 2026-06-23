# Cross-Edge EPR/Projection Coherence — Divergence Detection (F-COHERENCE)

> For agentic workers: REQUIRED SUB-SKILL: superpowers:subagent-driven-development (or executing-plans). Steps use checkbox (- [ ]) syntax.
> Working draft — NOT cite-sealed. Authored against the Federation Web2 Contract Ledger (`/projects/elohim/FEDERATION-WEB2-LEDGER-2026-06-14.md`) and the P2P Dataplane Contract Ledger (`/projects/elohim/P2P-DATAPLANE-CONTRACT-LEDGER-2026-06-14.md`). House style mirrors `2026-06-14-dataplane-diagnostic-plan.md`.

## 1. Context / why + the A/B-divergence facet it closes

**The confirmed live diagnostic:** the two production hostnames are TWO independent doorways, NOT load-balanced. `doorway-alpha.elohim.host` → doorway-alpha → MATTHEW conductor; `elohim.host` (apex) → doorway-alpha-b → ADAM conductor. Each edge is an island by construction: its EPR router is seeded from its OWN `STORAGE_URL` only (`projection/epr_router.rs::fetch_projections_from_storage`, boot + 30s refresh in `main.rs`), refreshed independently, with **no doorway↔doorway head comparison anywhere.** Cross-edge EPR-head coherence depends ENTIRELY on matthew↔adam conductor DHT gossip. When that partitions (DNA-hash skew, bootstrap islanding) or one edge is down, the two hostnames serve **different heads with NO DETECTION** — the operator saw two "EPR heads" (`e0352a7`/`8a2c65e`) and could not tell whether it was content divergence or deploy-version skew (it was the latter — those are `DEPLOY_VERSION`/git SHAs surfaced by matthew's crash-restart, not content CIDs, which are `bafy…`).

**The facet this plan closes:** make per-edge head state externally inspectable, and make cross-edge divergence VISIBLE — detect-only, never edge reconciliation. The doorway's job is to surface divergence and emit a structured alarm; it does NOT author cross-edge truth (that would violate `doorway/CLAUDE.md` "views served THROUGH not BY"; the real fix to "never partition" is F-BOOTSTRAP + F-DEPLOY).

**Why it's nearly free:** the doorway already holds a per-pillar head fingerprint. `EprRouter` is `url_path → EprProjectionView`; `EprProjectionView.epr_id` is the projected EPR atom CID (`elohim-views/src/projection.rs:25`); `replace_all` bumps a monotonic `generation` (`epr_router.rs:187,377`). A doorway can checksum `sorted[(url_path, epr_id)]` with **zero new fetch** — the divergence detector is a pure read over existing router state. The transport for cross-edge compare already exists too: the federation peer cache (`services/federation.rs:579-743`) is discovered and `/health`-probed on a periodic loop; we add a sibling coherence probe inside it.

**p2p-design-gate classification (done up front):** every new entity here is **Cat-C node-local read-model** — derived from existing router/peer state, no DHT entry, no table, no coordinator fn, no notarized actuation (detection only; remediation is substrate-owned). Cite the class; do not re-litigate (matches the dataplane ledger's §p2p-design-gate stance for runtime read-models).

---

## 2. OWNED FILES (verbatim from federation ledger §2) + collision statement

**MUTATE (M):**
- `doorway/doorway-service/src/services/federation.rs` — add the cross-edge coherence probe inside the EXISTING `refresh_peer_cache` loop body (ledger C-FED scope-split: **F-COHERENCE owns the probe-loop edit in `services/federation.rs`**; F-EDGE owns the DIFFERENT file `routes/federation.rs`). New `PeerCoherenceCache` type + `fetch_peer_coherence` helper + `refresh_coherence` co-located here; populate alongside the peer refresh.
- `doorway/doorway-service/src/server/http.rs` — ONE additive match arm registering `GET /api/v1/federation/coherence` (ledger C-HTTP: additive append-only; F-BOOTSTRAP adds a disjoint `/admin/bootstrap-coherence` arm; integrator merges mechanically).

**CREATE (C):**
- `doorway/doorway-service/src/routes/coherence.rs` — SOLE owner. Self-fingerprint endpoint + cross-edge `CoherenceView`. Houses `EprHeadFingerprint`, `CoherenceManifest`, `router_fingerprint`, `CoherenceView`, `PeerCoherence`, `handle_federation_coherence`. (Ledger CN2: F-EDGE DROPPED its proposed `routes/edge_coherence.rs` and `EdgeCoherenceView` — this is the single cross-edge coherence module.)

**CONSUME — do NOT mutate (sequenced hand-off):**
- `doorway/doorway-service/src/routes/self_healing.rs` — the `coherence` block into `SelfHealingView` is a **sequenced additive hand-off into P-DIAGNOSTIC** (dataplane ledger RESOLUTION-G: `self_healing.rs` is P-DIAGNOSTIC's SOLE owner). F-COHERENCE does NOT mutate this file in parallel; it lands its block AFTER P-DIAGNOSTIC's `anchor` block via the established `// FOLLOW-ON` seam (verified present at `self_healing.rs:12,34-45`). This is Wave-F3 work (X-COH-DIAG).

**Collision statement.** Every file above is either SOLE-owned by F-COHERENCE (`routes/coherence.rs`), a scope-split single-mutator (`services/federation.rs` probe loop — F-EDGE never touches it), or an additive-only append (`http.rs` one arm). **This plan mutates NO file owned by another federation plan and NO file owned by any dataplane plan.** Specifically verified against both ledgers:
- `routes/federation.rs` → **F-EDGE only** (`/p2p-peers`). Not touched here.
- `routes/self_healing.rs`, `main.rs`, `routes/health.rs` → **dataplane P-DIAGNOSTIC** (RESOLUTION-G). Touched here ONLY as a sequenced hand-off (the `coherence` block), never in parallel.
- `storage_proxy.rs`, `epr.rs` → **dataplane P-DEFENSE**. Not touched.
- `elohim-storage/*`, `elohim-compute/*`, `steward/*`, DNA, `sdk/schemas/*` → dataplane territory. Not touched.

---

## 3. NEW PRIMITIVES owned + CONSUMED (skip-if-present)

### OWNED (this plan defines — federation ledger FS1, FS2, FS3)

| Primitive | Home | Shape | Class |
|---|---|---|---|
| `struct EprHeadFingerprint` | `doorway::routes::coherence` | `{ url_path: String, epr_id: String }` | Cat-C |
| `struct CoherenceManifest` | `doorway::routes::coherence` | `{ doorway_id: String, generation: u64, heads: Vec<EprHeadFingerprint>, digest: String, build_id: Option<String> }` (`digest` = a CIDv1 dag-cbor `bafyrei…` minted over the canonically dag-cbor-serialized sorted `(url_path,epr_id)` head set) | Cat-C |
| `fn router_fingerprint` | `doorway::routes::coherence` | `pub fn router_fingerprint(router: &EprRouter, doorway_id: &str, build_id: Option<&str>) -> CoherenceManifest` (PURE) | — |
| `struct CoherenceView` | `doorway::routes::coherence` | `{ self_digest: String, self_build_id: Option<String>, peers: Vec<PeerCoherence>, in_agreement: bool }` | Cat-C |
| `struct PeerCoherence` | `doorway::routes::coherence` | `{ doorway_id: String, reachable: bool, digest: Option<String>, build_id: Option<String>, agrees: bool, divergent_paths: Vec<String> }` | Cat-C |
| `type PeerCoherenceCache` | `doorway::services::federation` | `Arc<RwLock<Vec<PeerCoherence>>>` (probe results, refreshed in the existing loop) | Cat-C |

These are NOT shared with other plans (no other plan defines them; F-EDGE CONSUMES `CoherenceView` read-only). `CoherenceManifest` reuses the existing `EprRouter` accessor surface — it needs ONE new pure read returning `(url_path, epr_id)` pairs (a sibling of the existing `mount_url_paths()` at `epr_router.rs:382`), added in `routes/coherence.rs` as a free function over `&EprRouter` table accessors (NOT a mutation of `EprRouter`).

### CONSUMED (skip-if-present clause — verbatim from ledger)

> *"Before landing this type, verify the named owner module already exposes it. If present, VERIFY-ONLY (import + use). If absent at your integration point, land the owner plan's verbatim definition only as a temporary local shim, flag it in your plan's hand-off notes, and delete the shim when the owner lands."*

**Already-shipped doorway-local (VERIFY-ONLY — confirmed present):**
- `EprRouter` + `.generation()` (`epr_router.rs:377`) + `.mount_url_paths()` (`:382`) — read-only consumption. CONFIRMED.
- `PeerCache` / `refresh_peer_cache` / `fetch_single_peer` / `get_cached_peers` (`services/federation.rs:579-743`) — the probe loop transport. CONFIRMED.
- `EprProjectionView.epr_id` / `.url_path` (`elohim-views/src/projection.rs:25,29`) — the head CID. CONFIRMED.
- `state.args.doorway_id: Option<String>` (`federation.rs:69` usage) — the manifest's `doorway_id`. CONFIRMED.
- `elohim_compute::BuildInfo::new("elohim-doorway").commit` (`health.rs:459`, `build_info.rs:13`) — the doorway's OWN build SHA, the `build_id` source. CONFIRMED (see SEAM-DELTA — this is BETTER than the ledger's assumed `servingContext.buildId` storage path).

**Cross-layer consumes (from named dataplane plans):**
- **P-DIAGNOSTIC** — `SelfHealingView` `// FOLLOW-ON` seam (`self_healing.rs`); `P2PStatusInfo.self_cid_present` bool (dataplane S9). HARD on the seam for the Wave-F3 `coherence` block; SOFT on `self_cid_present` (enriches `PeerCoherence.agrees`, ignored if absent).
- **P-DEFENSE** — `elohim_compute::backoff::jittered` (dataplane S7) for probe retry cadence. SOFT, skip-if-present shim (a local `Duration::from_secs(probe_interval)` is the fallback until `jittered` lands; the probe already piggybacks the existing 5s-tick loop so it needs no retry of its own — `jittered` is an enrichment, not a blocker).

---

## 4. DEPENDENCY EDGES

### Intra-federation (federation ledger §4)

| Edge | Type | Reason |
|---|---|---|
| F-COHERENCE → F-BOOTSTRAP | **SOFT** | cross-edge head agreement is only *achievable* once the genesis pair shares a bootstrap table and converges DHT; F-COHERENCE can DETECT divergence regardless (detection is the point), so SOFT — ships standalone, lights green only after F-BOOTSTRAP. No file/type dependency. |
| F-EDGE → F-COHERENCE | **HARD (inbound)** | F-EDGE CONSUMES `CoherenceView` (FS3) and dropped its own `edge_coherence.rs` (C-EDGE-COH). F-COHERENCE is a PRODUCING ROOT; nothing here blocks on F-EDGE. |
| F-DEPLOY → F-COHERENCE | **HARD (inbound)** | `verify-pair-coherence.sh` (FS6) curls F-COHERENCE's `GET /api/v1/federation/coherence` (FS1) for served-head equality. F-COHERENCE supplies the endpoint; nothing here blocks on F-DEPLOY. |

F-COHERENCE has **zero outbound HARD federation edges** → it is a producing root, dispatchable in WAVE F1.

### Cross-layer (federation ledger §5 — into the dataplane plan set)

| Edge id | → Dataplane track (ledger ref) | Type | Reason |
|---|---|---|---|
| **X-COH-DIAG** | P-DIAGNOSTIC (`self_healing.rs` SOLE owner, RESOLUTION-G) | **HARD** | The `coherence` block into `SelfHealingView`/`compose_self_healing` is a SEQUENCED additive hand-off — lands AFTER P-DIAGNOSTIC's `anchor` block via the `// FOLLOW-ON` seam, NOT a parallel mutation. The standalone `GET /api/v1/federation/coherence` route works WITHOUT this; only the `/admin/self-healing` surfacing is sequenced (Wave F3). |
| **X-COH-CID** | P-DIAGNOSTIC (`P2PStatusInfo.self_cid_present`, S9) | **SOFT** | `PeerCoherence.agrees` works on `epr_id` digest alone; richer when it also consults the per-edge `self_cid_present` bool. Consume if present, ignore if absent. |
| **X-COH-DEF** | P-DEFENSE (`jittered` S7) | **SOFT** | the coherence probe DEFERS its retry cadence to `jittered` rather than inventing retry. Skip-if-present shim until P-DEFENSE lands. |

**Cycle check:** none. F-COHERENCE has only inbound HARD edges (from F-EDGE, F-DEPLOY) and one HARD outbound *sequencing* edge to P-DIAGNOSTIC that is deferred to Wave F3 (its core deliverable — the route + detector — has no such dependency).

---

## 5. Build / test commands (per-crate RUSTFLAGS + /tmp target + plain cargo)

doorway-service (all Rust tasks — native; RUSTFLAGS MUST be empty):
```
cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib coherence 2>&1 | tail -40
cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib federation 2>&1 | tail -40
```

Final gates:
```
cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins 2>&1 | tail -40
cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo clippy -- -D warnings 2>&1 | tail -40
cd /projects/elohim/doorway/doorway-service && cargo fmt --check
```

Rules (memory): `RUSTFLAGS=""` for doorway (native; the WASM custom-getrandom flag leaks → `undefined __getrandom_v03_custom` at link); `RUSTC_WRAPPER=""` (sccache spawn-ENOENT); `/tmp` target dir (fingerprint-ENOENT on pool slots); **plain `cargo test`, NEVER nextest** (container quirk); never `&&`-pipe a gate exit code (use `2>&1 | tail -N`). NO NEW dep for the digest: it is minted as a CIDv1 dag-cbor (`bafyrei…`) via the `cid` + `multihash_codetable` + `serde_ipld_dagcbor` crates the doorway ALREADY pulls — the exact idiom lives in `doorway/src/routes/blob.rs` (raw codec `0x55`) and `elohim-storage/src/epr_codec.rs::encode_epr_head` (dag-cbor codec `0x71`, the form used here). Confirm the three crates are present (`grep -E '^(cid|multihash|serde_ipld_dagcbor|serde_ipld)' doorway/doorway-service/Cargo.toml`); they ship with the federation/blob plane, so this adds ZERO new crates. (`sha2` is only the multihash *inside* the CID — never the standalone address.)

---

## TASK 1 — `routes/coherence.rs`: `EprHeadFingerprint` + `CoherenceManifest` + `router_fingerprint` (PURE)

Files:
- C `doorway/doorway-service/src/routes/coherence.rs`
- M `doorway/doorway-service/src/routes/mod.rs` (declare `pub mod coherence;` + re-export — additive, this file is module-decl only; no other plan touches the coherence line).

- [ ] Write the failing test FIRST — in a new `#[cfg(test)] mod tests` at the bottom of `coherence.rs`. Build an `EprRouter`, `replace_all` two known projections, assert the fingerprint is deterministic and order-independent:
```rust
    #[test]
    fn router_fingerprint_is_deterministic_and_camel_case() {
        let router = crate::projection::epr_router::EprRouter::new();
        router.replace_all(vec![
            sample_projection("/lamad", "epr-lamad-aaa"),
            sample_projection("/qahal", "epr-qahal-bbb"),
        ]);
        let m = router_fingerprint(&router, "alpha-elohim-host", Some("e0352a7"));
        assert_eq!(m.doorway_id, "alpha-elohim-host");
        assert_eq!(m.heads.len(), 2);
        // digest stable regardless of HashMap iteration order
        let m2 = router_fingerprint(&router, "alpha-elohim-host", Some("e0352a7"));
        assert_eq!(m.digest, m2.digest);
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains("\"doorwayId\""), "{json}");
        assert!(json.contains("\"buildId\":\"e0352a7\""), "{json}");
    }

    #[test]
    fn different_heads_produce_different_digest() {
        let a = crate::projection::epr_router::EprRouter::new();
        a.replace_all(vec![sample_projection("/lamad", "epr-AAA")]);
        let b = crate::projection::epr_router::EprRouter::new();
        b.replace_all(vec![sample_projection("/lamad", "epr-BBB")]);
        assert_ne!(
            router_fingerprint(&a, "x", None).digest,
            router_fingerprint(&b, "x", None).digest
        );
    }
```
  (`sample_projection(url_path, epr_id)` builds an `EprProjectionView` literal — copy the field set from `projection.rs:184-195` `Default`/test path; or reuse an existing helper if one exists in scope.)
- [ ] Run, expect FAIL (no `router_fingerprint` symbol): `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib coherence 2>&1 | tail -40`.
- [ ] Write minimal implementation:
```rust
//! `GET /api/v1/federation/coherence` — per-edge EPR-head self-fingerprint +
//! cross-edge divergence detection (DETECT-ONLY).
//!
//! Cat-C node-local read-model (doorway/CLAUDE.md "views served THROUGH not BY"):
//! a fresh projection composed from THIS doorway's EprRouter table — any doorway
//! projecting the same substrate emits the same self-fingerprint (swap-test clean
//! for the self leg). The cross-edge `CoherenceView` is observability-only: it
//! makes divergence VISIBLE and never reconciles (reconciliation would author
//! cross-edge truth — forbidden). The real "never partition" fix is F-BOOTSTRAP +
//! F-DEPLOY; this plan is the detector the diagnostic asked for.

use serde::Serialize;
use cid::Cid;
use multihash_codetable::{Code, MultihashDigest};

use crate::projection::epr_router::EprRouter;

/// dag-cbor multicodec (0x71) — mirrors `elohim-storage/src/epr_codec.rs`.
const DAG_CBOR_CODEC: u64 = 0x71;

/// One pillar's projected EPR head on this doorway. `epr_id` is the projected
/// EPR atom CID (`EprProjectionView.epr_id`), NOT a content blob CID and NOT a
/// deploy build SHA — those are distinct (see `CoherenceManifest.build_id`).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EprHeadFingerprint {
    pub url_path: String,
    pub epr_id: String,
}

/// This doorway's full routing-table fingerprint. `digest` is a content-stable
/// CIDv1 dag-cbor address (`bafyrei…`) minted over the canonically
/// dag-cbor-serialized sorted `(url_path, epr_id)` head set, so two edges agree
/// iff their digests match. `build_id` is the deploy git SHA (the operator's "two EPR
/// heads" symptom was actually build_id skew, not content divergence) — carried
/// alongside so deploy-skew is reported, never confused with content skew.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoherenceManifest {
    pub doorway_id: String,
    pub generation: u64,
    pub heads: Vec<EprHeadFingerprint>,
    pub digest: String,
    pub build_id: Option<String>,
}

/// PURE read over the live router table. No fetch, no lock held across await.
pub fn router_fingerprint(
    router: &EprRouter,
    doorway_id: &str,
    build_id: Option<&str>,
) -> CoherenceManifest {
    let mut heads: Vec<EprHeadFingerprint> = router
        .head_fingerprints() // NEW free accessor — see note
        .into_iter()
        .map(|(url_path, epr_id)| EprHeadFingerprint { url_path, epr_id })
        .collect();
    heads.sort_by(|a, b| a.url_path.cmp(&b.url_path).then(a.epr_id.cmp(&b.epr_id)));
    // Canonical address = CIDv1(dag-cbor, Sha2-256(canonical dag-cbor bytes)).
    // The codec (0x71) must describe the actual preimage, so the sorted head set
    // is dag-cbor-serialized (NOT a hand-rolled byte concat) before hashing —
    // same idiom as `elohim-storage/src/epr_codec.rs::encode_epr_head`.
    let bytes = serde_ipld_dagcbor::to_vec(&heads).expect("dag-cbor encode of head set");
    let mh = Code::Sha2_256.digest(&bytes);
    let digest = Cid::new_v1(DAG_CBOR_CODEC, mh).to_string(); // bafyrei…
    CoherenceManifest {
        doorway_id: doorway_id.to_string(),
        generation: router.generation(),
        heads,
        digest,
        build_id: build_id.map(str::to_string),
    }
}
```
  **`head_fingerprints()` accessor:** add a sibling of `mount_url_paths()` to `EprRouter` — BUT `epr_router.rs` is F-COHERENCE-adjacent (no other plan owns it; it is doorway-local and not in either ledger's file map). Add `pub fn head_fingerprints(&self) -> Vec<(String, String)> { self.table.read().expect("router lock poisoned").values().map(|p| (p.url_path.clone(), p.epr_id.clone())).collect() }` right after `mount_url_paths()` (`:385`). This is the ONE `epr_router.rs` line F-COHERENCE owns; flag it in hand-off notes (it is additive, read-only, no collision — F-EDGE reads the router but does not mutate it).
- [ ] Run, expect PASS: same `coherence` command.
- [ ] Commit (selective-stage): `git add doorway/doorway-service/src/routes/coherence.rs doorway/doorway-service/src/routes/mod.rs doorway/doorway-service/src/projection/epr_router.rs` + message:
```
feat(doorway): EprHeadFingerprint + CoherenceManifest + pure router_fingerprint

Cat-C node-local self-fingerprint of this edge's EPR routing table. Zero
new fetch — pure read over EprRouter. digest = CIDv1 dag-cbor (bafyrei…)
over the canonical dag-cbor of the sorted (url_path, epr_id) head set;
build_id carries the deploy SHA so deploy-skew is named distinctly from
content-head skew.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
```

## TASK 2 — `GET /api/v1/federation/coherence` self-fingerprint route + http.rs arm

Files:
- M `doorway/doorway-service/src/routes/coherence.rs` (add `handle_federation_coherence`).
- M `doorway/doorway-service/src/server/http.rs` (ONE additive match arm — ledger C-HTTP).

- [ ] Write the failing test — append to `coherence.rs` `mod tests` a handler test that constructs a minimal `AppState` (reuse the `AppState` test-builder pattern at `http.rs:4346`/`:4430`) and asserts the response body is a valid `CoherenceManifest` with the configured `doorway_id`:
```rust
    #[tokio::test]
    async fn coherence_route_serves_self_manifest() {
        let state = crate::server::http::test_app_state_with_doorway_id("alpha-elohim-host");
        state.epr_router.replace_all(vec![sample_projection("/lamad", "epr-lamad-aaa")]);
        let resp = handle_federation_coherence(state).await;
        assert_eq!(resp.status(), hyper::StatusCode::OK);
        let body = body_to_string(resp).await;
        let m: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(m["doorwayId"], "alpha-elohim-host");
        assert!(m["digest"].as_str().unwrap().starts_with("bafy"));
    }
```
  (If no `test_app_state_with_doorway_id` helper exists, build the minimal `AppState` inline from the `:4430` pattern, setting `args.doorway_id = Some(...)`. `body_to_string` mirrors existing handler-test helpers in the crate — reuse one.)
- [ ] Run, expect FAIL: same `coherence` command.
- [ ] Write minimal implementation in `coherence.rs`:
```rust
use std::sync::Arc;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Response, StatusCode};

/// `GET /api/v1/federation/coherence` — this edge's self-fingerprint.
/// The single missing primitive: per-edge head externally inspectable.
pub async fn handle_federation_coherence(
    state: Arc<crate::server::AppState>,
) -> Response<Full<Bytes>> {
    let doorway_id = state.args.doorway_id.clone().unwrap_or_else(|| "unknown".into());
    let build = elohim_compute::BuildInfo::new("elohim-doorway");
    let manifest = router_fingerprint(&state.epr_router, &doorway_id, Some(&build.commit));
    let body = serde_json::to_string(&manifest)
        .unwrap_or_else(|_| r#"{"error":"serialize"}"#.into());
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}
```
  In `server/http.rs`, add ONE arm next to the existing federation arms (after `(Method::GET, "/api/v1/federation/p2p-peers")` at `:2284`):
```rust
        (Method::GET, "/api/v1/federation/coherence") => {
            crate::routes::coherence::handle_federation_coherence(state.clone()).await
        }
```
  Add `/api/v1/federation/coherence` to the `is_service_path` / non-EPR-router service-prefix allow-list IF such a list gates `/api/v1/federation/*` (verify: `grep -n "federation" http.rs | grep -i "service_path\|reserved"`); the sibling `/api/v1/federation/*` routes already register, so the prefix is almost certainly already service-owned — confirm, don't assume.
- [ ] Run, expect PASS + whole-crate compile: `cargo test --lib coherence` then `cargo test --lib --bins`.
- [ ] Commit: `git add doorway/doorway-service/src/routes/coherence.rs doorway/doorway-service/src/server/http.rs` + message `feat(doorway): GET /api/v1/federation/coherence self-fingerprint route`.

## TASK 3 — Cross-edge probe in the federation refresh loop → `PeerCoherenceCache`

Files:
- M `doorway/doorway-service/src/services/federation.rs` (add `PeerCoherenceCache`, `fetch_peer_coherence`, `refresh_coherence`; call from the existing `refresh_peer_cache`/discovery loop — ledger C-FED: F-COHERENCE owns the loop body).
- M `doorway/doorway-service/src/routes/coherence.rs` (the comparison logic + `CoherenceView`/`PeerCoherence` consumed by the cross-edge view; lives in coherence.rs so the federation service stays transport-only).

Note the type split: `PeerCoherence` (the DATA) and the comparison helper live in `routes::coherence` (the read-model home); `services::federation` holds the CACHE + the PROBE (the transport). `federation.rs` imports `PeerCoherence` from `routes::coherence`.

- [ ] Write the failing test FIRST — pure comparison logic in `coherence.rs`:
```rust
    #[test]
    fn compare_marks_agreement_and_divergent_paths() {
        let me = sample_manifest("alpha", &[("/lamad","A"),("/qahal","B")]);
        let peer = sample_manifest("apex", &[("/lamad","A"),("/qahal","C")]);
        let pc = compare_to_peer(&me, "apex", true, Some(&peer));
        assert_eq!(pc.doorway_id, "apex");
        assert!(pc.reachable);
        assert!(!pc.agrees);
        assert_eq!(pc.divergent_paths, vec!["/qahal".to_string()]);
    }

    #[test]
    fn compare_unreachable_peer_does_not_agree() {
        let me = sample_manifest("alpha", &[("/lamad","A")]);
        let pc = compare_to_peer(&me, "apex", false, None);
        assert!(!pc.reachable);
        assert!(!pc.agrees);
    }

    #[test]
    fn coherence_view_in_agreement_when_all_peers_match() {
        let me = sample_manifest("alpha", &[("/lamad","A")]);
        let peers = vec![compare_to_peer(&me, "apex", true, Some(&sample_manifest("apex", &[("/lamad","A")])))];
        let view = build_coherence_view(&me, peers);
        assert!(view.in_agreement);
    }
```
  (`sample_manifest(id, &[(path,epr)])` builds a `CoherenceManifest` directly.)
- [ ] Run, expect FAIL: `cargo test --lib coherence`.
- [ ] Write minimal implementation in `coherence.rs`:
```rust
/// Per-peer divergence verdict (Cat-C). `agrees` = same digest AND reachable.
/// `divergent_paths` names the pillars whose heads differ (operator-actionable).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerCoherence {
    pub doorway_id: String,
    pub reachable: bool,
    pub digest: Option<String>,
    pub build_id: Option<String>,
    pub agrees: bool,
    pub divergent_paths: Vec<String>,
}

/// Cross-edge view (Cat-C, observability-only — NEVER reconciles).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoherenceView {
    pub self_digest: String,
    pub self_build_id: Option<String>,
    pub peers: Vec<PeerCoherence>,
    pub in_agreement: bool,
}

pub fn compare_to_peer(
    me: &CoherenceManifest,
    peer_id: &str,
    reachable: bool,
    peer: Option<&CoherenceManifest>,
) -> PeerCoherence {
    match (reachable, peer) {
        (true, Some(p)) => {
            let agrees = p.digest == me.digest;
            let mut divergent: Vec<String> = Vec::new();
            if !agrees {
                let mine: std::collections::HashMap<&str, &str> =
                    me.heads.iter().map(|h| (h.url_path.as_str(), h.epr_id.as_str())).collect();
                for h in &p.heads {
                    match mine.get(h.url_path.as_str()) {
                        Some(my_epr) if *my_epr == h.epr_id => {}
                        _ => divergent.push(h.url_path.clone()),
                    }
                }
                for path in mine.keys() {
                    if !p.heads.iter().any(|h| h.url_path == *path) {
                        divergent.push((*path).to_string());
                    }
                }
                divergent.sort();
                divergent.dedup();
            }
            PeerCoherence {
                doorway_id: peer_id.to_string(),
                reachable: true,
                digest: Some(p.digest.clone()),
                build_id: p.build_id.clone(),
                agrees,
                divergent_paths: divergent,
            }
        }
        _ => PeerCoherence {
            doorway_id: peer_id.to_string(),
            reachable,
            digest: None,
            build_id: None,
            agrees: false,
            divergent_paths: Vec::new(),
        },
    }
}

pub fn build_coherence_view(me: &CoherenceManifest, peers: Vec<PeerCoherence>) -> CoherenceView {
    let in_agreement = peers.iter().all(|p| p.agrees);
    CoherenceView {
        self_digest: me.digest.clone(),
        self_build_id: me.build_id.clone(),
        peers,
        in_agreement,
    }
}
```
  In `services/federation.rs` (transport + cache):
```rust
/// Probe results from sibling doorways' /api/v1/federation/coherence.
pub type PeerCoherenceCache =
    Arc<tokio::sync::RwLock<Vec<crate::routes::coherence::PeerCoherence>>>;

pub fn new_peer_coherence_cache() -> PeerCoherenceCache {
    Arc::new(tokio::sync::RwLock::new(Vec::new()))
}

/// Fetch one peer's CoherenceManifest. Bounded timeout; reuses the federation
/// HTTP idiom. (X-COH-DEF: retry cadence DEFERS to elohim_compute::backoff::
/// jittered when present — until then the existing loop interval is the cadence.)
async fn fetch_peer_coherence(
    client: &reqwest::Client,
    peer_url: &str,
) -> Option<crate::routes::coherence::CoherenceManifest> {
    let url = format!("{}/api/v1/federation/coherence", peer_url.trim_end_matches('/'));
    match client.get(&url).timeout(std::time::Duration::from_secs(5)).send().await {
        Ok(r) if r.status().is_success() => r.json().await.ok(),
        _ => None,
    }
}
```
  Add a `refresh_coherence(self_manifest, peers, client, cache)` that maps each cached peer through `fetch_peer_coherence` → `compare_to_peer` → stores the `Vec<PeerCoherence>`, and CALL it from inside the existing discovery loop (`spawn_peer_discovery_task` body at `:732`, after `refresh_peer_cache`). It needs the self-manifest, which requires the `EprRouter` + `doorway_id` + `BuildInfo` — thread an `Arc<EprRouter>` + `doorway_id` + `PeerCoherenceCache` into `spawn_peer_discovery_task` (additive params; the single caller in `main.rs` is a sequenced, F-COHERENCE-owned signature change — main.rs is NOT in either ledger's file map as owned, but flag the call-site edit in hand-off notes since dataplane P-DIAGNOSTIC touches `main.rs:483-503`, a DIFFERENT block; verify no line overlap before staging).
- [ ] Run, expect PASS: `cargo test --lib coherence` then `cargo test --lib federation`.
- [ ] Commit: `git add doorway/doorway-service/src/routes/coherence.rs doorway/doorway-service/src/services/federation.rs doorway/doorway-service/src/main.rs` + message `feat(doorway): cross-edge coherence probe in federation refresh loop`.

## TASK 4 — Divergence ALARM (structured WARN, FallbackOutcome precedent)

Files:
- M `doorway/doorway-service/src/services/federation.rs` (emit the WARN inside `refresh_coherence` when any peer `!agrees`).

- [ ] Implementation (no pure unit test — it's a logging side-effect verified by compile + manual; the COMPARISON it gates is already tested in Task 3). Inside `refresh_coherence`, after building the `Vec<PeerCoherence>`:
```rust
    for pc in &results {
        if pc.reachable && !pc.agrees {
            warn!(
                self_doorway = %self_manifest.doorway_id,
                peer_doorway = %pc.doorway_id,
                self_digest = %self_manifest.digest,
                peer_digest = ?pc.digest,
                self_build = ?self_manifest.build_id,
                peer_build = ?pc.build_id,
                divergent_paths = ?pc.divergent_paths,
                "CROSS-EDGE EPR HEAD DIVERGENCE — two doorways serving different heads"
            );
        }
    }
```
  This reuses the FallbackOutcome "make-the-degraded-state-loud" precedent (`epr_router.rs:39-60` doc-comment: the degraded state hid for days at DEBUG). Naming BOTH `doorway_id`s + the build SHAs lets the operator instantly tell content-skew (`peer_digest` differs, builds equal) from deploy-skew (builds differ — the actual symptom they saw). Optional Loki-visible counter: an `AtomicU64` `coherence_divergence_total` on the cache struct, incremented here, surfaced in Task 5's view — leave as a one-line follow-on if the counter accessor isn't trivially threadable.
- [ ] Run, expect PASS (compile): `cargo test --lib --bins`.
- [ ] Commit: `git add doorway/doorway-service/src/services/federation.rs` + message `feat(doorway): structured WARN alarm on cross-edge EPR head divergence`.

## TASK 5 — [WAVE F3, X-COH-DIAG] `coherence` block into `SelfHealingView` (sequenced hand-off to P-DIAGNOSTIC)

> **SEQUENCING GATE:** Do NOT start until P-DIAGNOSTIC's `anchor` block has landed on the integration branch (dataplane ledger Wave 3, RESOLUTION-G). `self_healing.rs` is P-DIAGNOSTIC's SOLE owner — this is an additive wire-up through the `// FOLLOW-ON` seam, NOT a parallel mutation. Tasks 1–4 (the route + detector — the CORE deliverable) have NO such gate and ship in Wave F1.

Files:
- M `doorway/doorway-service/src/routes/self_healing.rs` (add `coherence: CoherenceView` to `SelfHealingView` via the seam; populate in `compose_self_healing` from `state` in `handle_self_healing`).

- [ ] Write the failing test — append to `self_healing.rs` `mod tests`:
```rust
    #[test]
    fn view_surfaces_coherence_block() {
        let view = compose_self_healing(SelfHealingInputs {
            coherence: Some(sample_coherence_view()),
            ..sample_inputs()
        });
        let json = serde_json::to_string(&view).unwrap();
        assert!(json.contains("\"coherence\""), "{json}");
        assert!(json.contains("\"inAgreement\""), "{json}");
    }
```
- [ ] Run, expect FAIL: `cargo test --lib self_healing`.
- [ ] Minimal implementation — add `pub coherence: crate::routes::coherence::CoherenceView,` to `SelfHealingView` (after `conductor` at `:50`); add a `coherence: Option<CoherenceView>` input to `SelfHealingInputs`; in `compose_self_healing` set `coherence: inputs.coherence.unwrap_or_else(empty_coherence_view)` (an `empty_coherence_view()` helper: `self_digest:"".into(), peers:vec![], in_agreement:true, self_build_id:None` — honest "no peers compared yet"); in `handle_self_healing`, read `state.peer_coherence_cache` and the self-manifest into the input. Add the 2 fields to `sample_inputs()` as `coherence: None`.
- [ ] Run, expect PASS: `cargo test --lib self_healing`.
- [ ] Commit: `git add doorway/doorway-service/src/routes/self_healing.rs` + message `feat(doorway): coherence block on SelfHealingView (X-COH-DIAG hand-off)`.

## TASK 6 — [WAVE F3, X-COH-CID, SOFT] enrich `agrees` with `self_cid_present`

> Optional enrichment. Only after P-DIAGNOSTIC plumbs `P2PStatusInfo.self_cid_present` to a doorway-readable field (dataplane S9). If absent, SKIP — the digest comparison stands alone.

- [ ] If the field is present in the peer's `/p2p/status` / coherence response, AND a peer reports `self_cid_present == false`, downgrade `agrees → false` even on digest match (a peer with no anchor publishes nothing — its head is stale-by-construction). Add a test mirroring Task 3's `compare_to_peer` with the bool present. Commit per the per-task pattern.

---

## 6. p2p-class of new entities

All Cat-C node-local read-models (federation ledger §1, §3; matches dataplane ledger's runtime-read-model stance):
- `EprHeadFingerprint`, `CoherenceManifest`, `CoherenceView`, `PeerCoherence`, `PeerCoherenceCache` — **Cat-C**. Derived from existing `EprRouter` table + peer-probe state. No DHT entry, no table, no coordinator fn, no content-addressed identity, no notarized actuation.
- **No Cat-A here.** This track is detection-only; remediation (the "never partition" fix) is substrate-owned (F-BOOTSTRAP shared bootstrap table, F-DEPLOY atomic pair barrier). A doorway that re-fetched a peer's head to "reconcile" would author cross-edge truth — explicitly rejected (`doorway/CLAUDE.md` swap test + No Blob Fan-Out).
- Swap test: the SELF leg (`router_fingerprint` / `GET /api/v1/federation/coherence`) is swap-test CLEAN — any doorway projecting the same substrate emits the same self-fingerprint. The cross-edge `CoherenceView` is legitimate doorway-local Operational state (like the federation peer list) — observability, not authored content.

---

## 7. // FOLLOW-ON seams (for the integration pass / named siblings)

1. **`/admin/self-healing` `coherence` block (X-COH-DIAG).** Wave-F3 sequenced behind P-DIAGNOSTIC's `anchor` block via the `self_healing.rs:34-45` `// FOLLOW-ON` seam. One-line wire-up; Task 5.
2. **`self_cid_present` enrichment of `agrees` (X-COH-CID).** SOFT; lands when P-DIAGNOSTIC plumbs the bool. Task 6.
3. **`jittered` probe cadence (X-COH-DEF).** SOFT; swap the loop-interval fallback for `elohim_compute::backoff::jittered` when P-DEFENSE lands it.
4. **Loki divergence counter.** An `AtomicU64 coherence_divergence_total` surfaced in the view + WARN — left as a one-line follow-on if the counter accessor isn't trivially threadable in Task 4.
5. **F-DEPLOY `verify-pair-coherence.sh` (FS6) consumes this route.** The script curls `GET /api/v1/federation/coherence` on both edges for served-head equality; its head-equality leg flips from WARN to FAILURE once this route is live (F-DEPLOY Wave-F3, X-DEPLOY-DIAG). No code here — the route IS the contract.
6. **F-EDGE consumes `CoherenceView`.** F-EDGE dropped `edge_coherence.rs` and imports `CoherenceView` from `routes::coherence` (C-EDGE-COH). No code here — the type IS the contract.
7. **Angular stability-lens render of `coherence`.** The `/admin/self-healing` consumer page exists (`stability-lens.component.ts`); rendering the new `coherence` block is a named frontend sibling follow-on (eyes-first; `pnpm look` on the stability lens). The schema field lands verbatim for it — NOTE: unlike P-DIAGNOSTIC's `anchor` (which has a `stability-status-view.schema.json` contract), `coherence` is doorway-local-only with no Rust schema_contract; if a shared schema is desired so a sibling doorway serves an equivalent view, that is a follow-on schema addition (out of this plan's detect-only scope).

---

## Dispatch note

- **Isolated-worktree, subagent-driven, commit-only.** Run from a dedicated worktree off the integration branch. The integrator pushes/merges (memory: commit-only; never `git push`).
- **Wave F1 (roots, parallel with F-BOOTSTRAP):** Tasks 1–4 — the self-fingerprint route + cross-edge detector + alarm. Fully independent of every other plan (producing root). HARD-consumed by F-EDGE (FS3) and F-DEPLOY (FS1) downstream.
- **Wave F3 (sequenced):** Task 5 (X-COH-DIAG, behind P-DIAGNOSTIC's `anchor` block) and Task 6 (X-COH-CID, SOFT).
- **Selective-stage** each commit (concurrent sessions share the worktree per memory) — per-task `git add` lists name exact files only; never bulk-revert ambient mods.
- **RUSTFLAGS="" is load-bearing** for doorway (native build; the WASM custom-getrandom flag link-fails with `undefined __getrandom_v03_custom`). `/tmp` target dir + `RUSTC_WRAPPER=""` + plain `cargo test`.
- **Verify before staging `main.rs`:** the `spawn_peer_discovery_task` call-site signature change (Task 3) touches `main.rs` — confirm zero line overlap with P-DIAGNOSTIC's `main.rs:483-503` poll block (different region: the spawn call is in the startup/task-wiring section, the poll is the 30s status loop). They are disjoint; flag in the integrator hand-off note.
