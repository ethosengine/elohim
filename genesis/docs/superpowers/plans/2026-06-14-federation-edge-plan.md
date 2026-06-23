# Federation Edge — Server / Load-Balancing / Absorption / CDN — Implementation Plan (F-EDGE)

> For agentic workers: REQUIRED SUB-SKILL: superpowers:subagent-driven-development (or executing-plans). Steps use checkbox (- [ ]) syntax.
> Working draft — NOT cite-sealed. Authored against the Federation Web2 Ledger (`/projects/elohim/FEDERATION-WEB2-LEDGER-2026-06-14.md`) and the P2P-Dataplane Contract Ledger (`/projects/elohim/P2P-DATAPLANE-CONTRACT-LEDGER-2026-06-14.md`).

## 1. Context / why + the A/B-divergence facet it closes

**Goal:** Fix the doorway *edge topology* read-models and posture so the gateway layer tells the truth about the two-island reality and stops compounding it. Three concrete defects in the GATEWAY layer (NOT the dataplane):

1. **`/api/v1/federation/p2p-peers` under-reports** (`total:1` self vs live `peerCount 13`). `handle_federation_p2p_peers` (`routes/federation.rs:208`) builds ONE `P2PPeerInfo` per *storage backend* from `peerId`/`listenAddresses` (`:297-321`) and NEVER reads `connectedPeers`. `total` = count of storage instances polled (1), not mesh peer count. There is also a **dead StatefulSet branch** (`:229-254`) for a `headless_service_base` topology doorway no longer has (both edges are `Deployment{replicas:1}`).

2. **The two hostnames serve potentially different EPR heads with NO detection.** `elohim.host`/adam and `doorway-alpha.elohim.host`/matthew are independent islands by construction (per-pod EprRouter fetching only its own `STORAGE_URL`; cross-edge head coherence depends ENTIRELY on matthew↔adam DHT gossip). The `e0352a7`/`8a2c65e` "two heads" the operator saw are **deploy-version git SHAs** (`DEPLOY_VERSION_PLACEHOLDER` → `app.kubernetes.io/version`, alpha.yaml:33/56/73), surfaced by matthew crash-restart — NOT content CIDs (those are `bafy…`). Either way: no surface tells the operator the edges diverged.

3. **The blob miss-then-200 forward path lacks an `immutable` Cache-Control** (`storage_proxy.rs:492-497`, VERIFIED missing — the pantry-HIT path at `:369-370` HAS it). Without it, a front CDN / nginx `proxy_cache` on `/blob/*` cannot be purge-free, so the content-addressed bytes (the one safely-cacheable class) stay un-CDN-able.

**A/B-divergence facet closed by F-EDGE:** the *detection-and-honesty* facet of the edge seam. F-EDGE makes `/p2p-peers` report the real mesh (so an operator looking at peers sees `13`, not `1`), CONSUMES F-COHERENCE's cross-edge head-divergence read-model (it does NOT build its own — see §2 collision C-EDGE-COH), and prepares the `/blob/*` CDN posture (gated on the dataplane landing the immutable header). The deep divergence DETECTOR is F-COHERENCE's deliverable; the deep absorption/breaker tuning is the dataplane's (P-DEFENSE). F-EDGE owns the *peer-projection truth* + the *manifest/CDN posture* only.

**p2p-class (preview, full §6):** the rewritten `P2PPeersResponse` is **Cat-C node-local Operational read-model** (any doorway computes its own from its own storage `/p2p/status`; swap-test clean). No Cat-A entity in this track.

---

## 2. OWNED FILES (verbatim from federation ledger §2 "F-EDGE") + collision statement

**MUTATE (M):**
- `doorway/doorway-service/src/routes/federation.rs` — **SOLE owner** of the `handle_federation_p2p_peers` REWRITE (project `connectedPeers` + peer list, NOT one-row-per-backend) + `P2PPeerInfo`/`P2PPeersResponse` projection change + retire the dead StatefulSet branch (`:229-254`). (Federation ledger §2 F-EDGE; C-FED scope-split.)
- `genesis/orchestrator/manifests/doorway/alpha.yaml` — **region-owner** of the `metadata.annotations` (Ingress) LB/CDN annotations + optional `proxy_cache` on `/blob/*`, AND the `DEPLOY_VERSION` env addition in the `env:` block (additive). (C-MANIFEST: F-EDGE owns `annotations`/cache region in BOTH files.)
- `genesis/orchestrator/manifests/doorway/alpha-b.yaml` — **region-owner** of the `metadata.annotations` (Ingress) LB/CDN annotations + `proxy_cache` + `DEPLOY_VERSION` env. (C-MANIFEST: F-EDGE owns `annotations`; F-DEPLOY owns `rules`/failover-backend posture on this same file — disjoint regions, keep that line.)

**CREATE (C):** none. (The ledger's earlier `routes/edge_coherence.rs` is DROPPED — see C-EDGE-COH below.)

**CONSUME-ONLY (do NOT mutate):**
- `doorway/doorway-service/src/routes/coherence.rs` — F-COHERENCE's `CoherenceView` (FS3). F-EDGE consumes; does NOT create a parallel module (C-EDGE-COH).
- `doorway/doorway-service/src/services/federation.rs` — `get_cached_peers`/`PeerCache` (FS9), read-only. F-COHERENCE owns the probe-loop body in this file (C-FED).
- `doorway/doorway-service/src/projection/epr_router.rs` — `EprRouter` (FS10) read-only self-head.
- `doorway/doorway-service/src/routes/storage_proxy.rs` + `routes/epr.rs` — **dataplane P-DEFENSE territory**. F-EDGE FLAGS the missing immutable header (`storage_proxy.rs:492-497`) and the per-request `Client::new()` (`epr.rs:40`); it does NOT mutate either (C-EDGE-PROXY → X-EDGE-DEF).
- `doorway/doorway-service/src/server/http.rs` — F-EDGE adds NO route arm (the `/p2p-peers` route already exists at `http.rs:2284`; the rewrite is in the handler in `federation.rs`, not a new arm).

**COLLISION STATEMENT.** Every MUTATE file above is either SOLE-owned by F-EDGE (`routes/federation.rs`) or an additive-disjoint region of a shared manifest (C-MANIFEST: F-EDGE owns the Ingress `annotations` + `DEPLOY_VERSION` env in both manifests; F-BOOTSTRAP owns `BOOTSTRAP_MONGODB_DB` env; F-DEPLOY owns alpha-b `rules`/failover posture — NO shared YAML key). **F-EDGE touches NO file owned by another FEDERATION plan** (it dropped `edge_coherence.rs`, does not touch `coherence.rs`/`services/federation.rs`/`self_healing.rs`/`Jenkinsfile`/`bootstrap/*`) **and NO file owned by any DATAPLANE plan** (it flags but does not mutate `storage_proxy.rs`/`epr.rs`; it touches nothing under `elohim-storage/*`, `elohim-compute/*`, `steward/*`, DNA, or `sdk/schemas/*`). Verified against federation ledger §2 NO-DOUBLE-OWN and dataplane ledger §2 file map.

**Specific collision resolutions inherited (verbatim handles):**
- **C-EDGE-COH** — F-EDGE DROPS `routes/edge_coherence.rs` + `EdgeCoherenceView`; the cross-edge head-divergence detector IS F-COHERENCE's `routes::coherence::CoherenceView` (FS3). F-EDGE consumes it.
- **C-FED** — `routes/federation.rs` (F-EDGE) vs `services/federation.rs` (F-COHERENCE) are different files; scope-split. F-EDGE solely owns the `routes/federation.rs` peer-projection rewrite.
- **C-EDGE-PROXY** — `storage_proxy.rs` immutable Cache-Control + `epr.rs` pooled-client are HARD hand-offs to dataplane P-DEFENSE (X-EDGE-DEF). F-EDGE's CDN manifest work is GATED on that header landing.
- **C-MANIFEST** — 3-way additive-disjoint on the two doorway manifests.

---

## 3. NEW PRIMITIVES THIS PLAN OWNS + CONSUMED (skip-if-present)

### OWNS

| Primitive | Home | Shape |
|---|---|---|
| `P2PPeersResponse` (REWRITE) | `doorway::routes::federation` | `{ peers: Vec<P2PPeerInfo>, total: usize, connected_peer_count: Option<usize> }` — `total` = mesh `connectedPeers` from the routed storage `/p2p/status` (NOT backend count); `connected_peer_count` carries the honest count distinctly. **Additive field** to the existing struct; existing `peers`/`total` keys preserved (total semantics fixed). |
| `fn handle_federation_p2p_peers` (REWRITE) | `doorway::routes::federation` | `(state: Arc<AppState>) -> Response<Full<Bytes>>` — projects mesh peers from the routed storage `connectedPeers`/peer list; the dead `headless_service_base` StatefulSet branch (`:229-254`) is retired. |

Both are **Cat-C node-local read-models** — no DHT entry, no table, no coordinator fn, no new route (the route exists at `http.rs:2284`). The federation ledger §1 already lists these as "rewritten `P2PPeersResponse` = Cat-C". No new shared type leaves doorway-service.

### CONSUMED (skip-if-present clause, verbatim from federation ledger §1)

*"Before landing this type, verify the named owner module already exposes it. If present, VERIFY-ONLY (import + use). If absent at your integration point, land the owner plan's verbatim definition only as a temporary local shim, flag it in your plan's hand-off notes, and delete the shim when the owner lands."*

| Consumed primitive | Owner | Edge | How F-EDGE uses it |
|---|---|---|---|
| `CoherenceView` / `PeerCoherence` (FS3) | **F-COHERENCE** (`doorway::routes::coherence`) | **HARD** intra-federation | F-EDGE's "two hostnames serve different heads" concern is DELEGATED here. F-EDGE does NOT define a parallel type. The cross-edge surface = F-COHERENCE's `GET /api/v1/federation/coherence`. If `routes::coherence` is absent at integration, F-EDGE's `/p2p-peers` + manifest work proceed standalone (they do NOT import `CoherenceView`); the delegation is documentation, not a compile dependency. |
| `get_cached_peers` / `PeerCache` (FS9) | already-shipped (`doorway::services::federation:741`) | n/a (read-only) | Read-only; F-EDGE does not edit the loop (F-COHERENCE owns the loop body per C-FED). F-EDGE reads cached peers only IF a future cross-edge compare needs them — for v1 the `/p2p-peers` rewrite reads the routed storage `/p2p/status` directly (existing `query_storage_p2p_status` idiom). |
| `EprRouter.generation()` / `mount_url_paths()` / `dispatch()` (FS10) | already-shipped (`doorway::projection::epr_router`) | n/a (read-only) | Read-only self-head if F-EDGE ever needs it; v1 does not. |

**CROSS-LAYER consumes (from named dataplane plans):**

| Consumed | Dataplane owner (ledger ref) | Edge | Use |
|---|---|---|---|
| blob `immutable` Cache-Control on the miss-then-200 path (`storage_proxy.rs:492-497`) | **P-DEFENSE** (dataplane §2 line 113 `epr.rs` family; S11 `storage_proxy.rs`) | **HARD** (X-EDGE-DEF) | F-EDGE does NOT add the header (P-DEFENSE territory). F-EDGE's `/blob/*` `proxy_cache`/CDN manifest stanza is GATED on it: a purge-free CDN on content-addressed bytes is only safe once the immutable header is on the miss path. |
| `epr.rs:40` per-request `reqwest::Client::new()` → pooled client | **P-DEFENSE** (dataplane §2 line 113) | **HARD** (X-EDGE-DEF) | F-EDGE only FLAGS this (the EPR-head read path bypasses the breaker). Hand-off, never mutated. |
| storage `/p2p/status` peer-list contract (`connectedPeers` + peer-list field names) | **P-DIAGNOSTIC** (dataplane S9/§3) | **SOFT** (X-EDGE-PEERS) | The `/p2p-peers` rewrite reads `connectedPeers` (already present in `P2PHealth`/`main.rs:487`); F-EDGE coordinates field names to avoid a parallel vocabulary. Works standalone on the existing `/p2p/status` shape. |
| `self_cid_present` / `provide_loop_enabled` (`P2PStatusInfo.anchor`, S9/§3) | **P-DIAGNOSTIC** | **SOFT** | If a future cross-edge compare consults per-edge content-presence it consumes these; v1 does not. Named for cohesion only. |

---

## 4. DEPENDENCY EDGES (intra-federation + cross-layer, HARD/SOFT)

**Intra-federation (federation ledger §4 DAG):**

| Edge | Type | Reason |
|---|---|---|
| F-EDGE → F-COHERENCE | **HARD** | F-EDGE's cross-edge head-divergence concern is DELEGATED to F-COHERENCE's `CoherenceView` (FS3); F-EDGE dropped `edge_coherence.rs` (C-EDGE-COH). **BUT** F-EDGE's `/p2p-peers` fix + manifest work are INDEPENDENT and land before F-COHERENCE — the HARD edge is only on the *delegated* divergence-surface ownership, not on F-EDGE's own deliverables. (Federation ledger: "F-EDGE's `/p2p-peers` fix + manifest work are independent and can land before F-COHERENCE.") |
| F-EDGE → F-BOOTSTRAP | none | F-BOOTSTRAP (islanding root fix) makes cross-edge head agreement *achievable*; F-EDGE does not depend on it to compile or function. |

**Cross-layer (federation ledger §5):**

| Edge id | → Dataplane track | Type | Reason |
|---|---|---|---|
| **X-EDGE-DEF** | P-DEFENSE (`storage_proxy.rs` immutable header; `epr.rs` pooled client) | **HARD** | F-EDGE does not mutate either file. The `/blob/*` CDN manifest is GATED on the immutable Cache-Control fix; `epr.rs` is a flag-only hand-off. |
| **X-EDGE-PEERS** | P-DIAGNOSTIC (storage `/p2p/status` peer-list contract) | **SOFT** | `/p2p-peers` rewrite reads the same `connectedPeers` fields; coordinate names. Works standalone. |

**Dispatch wave:** **WAVE F2** (federation ledger §7). The `/p2p-peers` rewrite is INDEPENDENT and may begin in WAVE F1. The cross-edge consumption is a documentation delegation (no compile edge). The `/blob/*` CDN-enable is a **WAVE F3** cross-layer sequenced hand-off behind P-DEFENSE's header.

**Cycle check:** F-EDGE has zero outbound HARD edges that any plan depends on in return → terminal consumer. No cycles.

---

## 5. Build / test commands (per-crate RUSTFLAGS + /tmp target + plain cargo)

doorway-service (Task 1 — native; RUSTFLAGS MUST be empty, /tmp target, plain cargo, no nextest):
```
cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib federation 2>&1 | tail -40
```

Final gates (Task 1):
```
cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins 2>&1 | tail -40
cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo clippy -- -D warnings 2>&1 | tail -40
cd /projects/elohim/doorway/doorway-service && cargo fmt --check
```

Manifest tasks (Tasks 2–3) are **doc/lint only** — no cargo. YAML structural check:
```
cd /projects/elohim && python3 -c "import yaml,sys; list(yaml.safe_load_all(open('genesis/orchestrator/manifests/doorway/alpha.yaml'))); list(yaml.safe_load_all(open('genesis/orchestrator/manifests/doorway/alpha-b.yaml'))); print('yaml OK')"
```

Rules (memory): `RUSTFLAGS=""` for doorway (native); `RUSTC_WRAPPER=""` (sccache spawn-ENOENT); `/tmp` target dir (fingerprint-ENOENT on pool slot); **plain `cargo test`, NEVER nextest** (container has no nextest); never `&&`-pipe a gate exit code (use `2>&1 | tail -N`). Manifests describe CHANGES only — the edge Jenkinsfile + cluster apply are operator-owned.

---

## TASK 1 — Rewrite `/api/v1/federation/p2p-peers` to project mesh peers + retire dead StatefulSet branch

**Files:** `doorway/doorway-service/src/routes/federation.rs` (SOLE owner).

Current state (VERIFIED): `handle_federation_p2p_peers` (`:208`) calls `query_storage_p2p_status` (`:277`) which reads `peerId`/`listenAddresses`/`natStatus`/`relayMode` and returns ONE `P2PPeerInfo` per backend; `total` = `peers.len()` (`:256`); the `headless_service_base` loop (`:229-254`) is dead (both edges are `Deployment{replicas:1}`, `headless_service_base` unset). `connectedPeers` is NEVER read.

- [ ] **Write the failing test** — append to `routes/federation.rs` `#[cfg(test)] mod tests`. Assert the response carries the mesh count, not the backend count. Use a stub JSON value mirroring storage `/p2p/status` and a pure projection fn (extract one):
```rust
    #[test]
    fn p2p_peers_reports_mesh_connected_count_not_backend_count() {
        // storage /p2p/status shape: one backend, but 13 connected mesh peers.
        let status = serde_json::json!({
            "peerId": "12D3KooSELF",
            "listenAddresses": ["/ip4/10.0.0.1/tcp/4001"],
            "natStatus": "public",
            "relayMode": "client",
            "connectedPeers": 13
        });
        let resp = project_p2p_peers(&status);
        // self is one row; the mesh count is surfaced honestly and is 13, not 1.
        assert_eq!(resp.connected_peer_count, Some(13), "{resp:?}");
        assert_eq!(resp.total, 13, "total must be mesh count, not backend count");
        assert_eq!(resp.peers.len(), 1, "self row still present");
        assert_eq!(resp.peers[0].peer_id, "12D3KooSELF");
    }

    #[test]
    fn p2p_peers_tolerates_missing_connected_peers_field() {
        let status = serde_json::json!({
            "peerId": "12D3KooSELF",
            "listenAddresses": [],
            "relayMode": "client"
        });
        let resp = project_p2p_peers(&status);
        assert_eq!(resp.connected_peer_count, None);
        assert_eq!(resp.total, 1, "fallback: self-only when mesh count absent");
    }
```
- [ ] Run, expect FAIL: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib federation 2>&1 | tail -40` — expect `cannot find function project_p2p_peers` / `no field connected_peer_count`.
- [ ] **Write minimal implementation:**
  1. Add `connected_peer_count: Option<usize>` to `P2PPeersResponse` (after `total` at `:200`):
```rust
    /// Honest mesh peer count from the routed storage's `connectedPeers`
    /// (`/p2p/status`). None when storage is unreachable or the field is absent.
    /// `total` mirrors this when present (the OLD `total` = backend-count bug:
    /// it reported 1 even when 13 peers were connected — see plan §1).
    pub connected_peer_count: Option<usize>,
```
  2. Extract a PURE projection fn from the storage JSON (testable; mirrors the existing `query_storage_p2p_status` parse at `:297-321` but ALSO reads `connectedPeers`):
```rust
    /// Pure projection of one storage /p2p/status body → the federation peers
    /// response. `total` = mesh `connectedPeers` when present, else the self-row
    /// count (degraded honesty: "we can only see ourselves"). Cat-C node-local.
    fn project_p2p_peers(status: &serde_json::Value) -> P2PPeersResponse {
        let mut peers = Vec::new();
        if let Some(peer_id) = status["peerId"].as_str() {
            let multiaddrs = status["listenAddresses"].as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();
            let nat_status = status["natStatus"].as_str().map(String::from);
            let relay_mode = status["relayMode"].as_str().unwrap_or("client");
            let mut capabilities = vec!["shard".to_string(), "sync".to_string()];
            if relay_mode == "server" || relay_mode == "both" {
                capabilities.push("relay".to_string());
            }
            peers.push(P2PPeerInfo { peer_id: peer_id.to_string(), multiaddrs, capabilities, nat_status });
        }
        let connected_peer_count = status["connectedPeers"].as_u64().map(|n| n as usize);
        let total = connected_peer_count.unwrap_or(peers.len());
        P2PPeersResponse { peers, total, connected_peer_count }
    }
```
  3. Rewrite `handle_federation_p2p_peers` (`:208-274`) to: fetch the routed storage `/p2p/status` ONCE (reuse the existing `reqwest::Client::builder().timeout(5s)` idiom at `:216`), parse to `serde_json::Value`, call `project_p2p_peers`, serialize. **DELETE the `headless_service_base` StatefulSet branch (`:229-254`) entirely** + the now-unused `query_storage_p2p_status` helper (`:277-322`) if nothing else references it (grep first; it is only called from this handler). Keep the `Cache-Control: public, max-age=30` header (`:263`) — `/p2p-peers` is mutable, NOT CDN-cacheable; short TTL is correct.
- [ ] Run, expect PASS: same `federation` command.
- [ ] Run full gates: `cargo test --lib --bins`, `cargo clippy -- -D warnings`, `cargo fmt --check` (commands in §5). Confirm no dead-code warning from the removed branch.
- [ ] Commit (selective-stage):
```
git add doorway/doorway-service/src/routes/federation.rs
git commit -m "fix(doorway): /p2p-peers reports mesh connectedPeers, not backend count

handle_federation_p2p_peers reported total:1 (one storage backend) while
13 peers were connected — it never read connectedPeers. Now projects the
mesh count honestly via a pure project_p2p_peers fn and retires the dead
StatefulSet headless-DNS branch (both edges are Deployment{replicas:1}).
Cat-C node-local read-model; coordinates connectedPeers field name with
the dataplane P-DIAGNOSTIC /p2p/status contract (X-EDGE-PEERS).

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

> **X-EDGE-PEERS hand-off note:** the `connectedPeers` field name is the storage `/p2p/status` contract P-DIAGNOSTIC also reads (`main.rs:487` already plumbs it into `P2PHealth`). If P-DIAGNOSTIC renames or enriches the peer-list shape, this projection consumes the new name verbatim — do NOT invent a parallel vocabulary.

---

## TASK 2 — `DEPLOY_VERSION` env + LB/CDN annotations on alpha.yaml (doc/lint only)

**Files:** `genesis/orchestrator/manifests/doorway/alpha.yaml` (F-EDGE region-owner: `env:` `DEPLOY_VERSION` + Ingress `annotations`). NO cargo.

Why `DEPLOY_VERSION` env: the operator's "two heads" `e0352a7`/`8a2c65e` ARE the deploy git SHAs (`DEPLOY_VERSION_PLACEHOLDER` → `app.kubernetes.io/version` label, alpha.yaml:33/56/73), but the running binary only knows `CARGO_PKG_VERSION` (`env!`, `health.rs:209`). For F-COHERENCE's `CoherenceManifest.build_id` (FS1) and F-DEPLOY's `verify-pair-coherence.sh` (FS6) to surface the *deployed* SHA, the pod must carry it as an env. F-EDGE adds the env line; F-COHERENCE reads it (`build_id`). **This is the manifest leg that lets the deploy-version skew become reportable.**

- [ ] Add to the `env:` block of the `elohim-doorway-alpha` Deployment container (alongside `DOORWAY_ID` at `:157`), reusing the existing placeholder convention (`DEPLOY_VERSION_PLACEHOLDER` is already substituted by the pipeline at `:33/56/73`):
```yaml
            - name: DEPLOY_VERSION
              value: "DEPLOY_VERSION_PLACEHOLDER"
```
- [ ] Add LB/CDN-readiness annotations to the Ingress `metadata.annotations` (after `:411` `configuration-snippet`), **commented as draft for the `/blob/*` CDN, GATED on X-EDGE-DEF**:
```yaml
    # --- /blob/* CDN posture (DRAFT — enable only after dataplane P-DEFENSE lands
    #     the immutable Cache-Control on the storage_proxy miss-then-200 path,
    #     storage_proxy.rs:492-497; see plan X-EDGE-DEF). Content-addressed bytes
    #     are immutable (CID = cache key, never mutates) so a purge-free proxy_cache
    #     is safe ONCE the header is present. Do NOT cache any EPR-head/view route.
    # nginx.ingress.kubernetes.io/server-snippet: |
    #   location ~ ^/blob/ {
    #     proxy_cache blob_immutable;
    #     proxy_cache_valid 200 365d;
    #     proxy_cache_key $uri;        # CID-keyed; immutable
    #     add_header X-Blob-Cache $upstream_cache_status;
    #   }
```
  Leave `upstream-hash-by: $binary_remote_addr` (`:402`) UNTOUCHED — with `replicas:1` it is inert; do not remove it (no behavior change, avoids churn).
- [ ] Validate YAML: `python3 -c "import yaml; list(yaml.safe_load_all(open('genesis/orchestrator/manifests/doorway/alpha.yaml'))); print('OK')"` (from repo root).
- [ ] Commit:
```
git add genesis/orchestrator/manifests/doorway/alpha.yaml
git commit -m "chore(doorway-manifest): DEPLOY_VERSION env + draft /blob CDN posture (alpha)

DEPLOY_VERSION carries the deployed git SHA into the pod so F-COHERENCE's
build_id and F-DEPLOY's pair-coherence gate can surface deploy-version skew
(the operator's e0352a7/8a2c65e symptom). The /blob/* proxy_cache stanza is
commented DRAFT — gated on dataplane P-DEFENSE landing the immutable
Cache-Control on the blob miss path (X-EDGE-DEF).

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

> **C-MANIFEST hand-off:** F-BOOTSTRAP adds `BOOTSTRAP_MONGODB_DB` to the SAME `env:` block (disjoint key); F-DEPLOY does NOT touch alpha.yaml. Integrator merges the env additions mechanically — no shared key.

---

## TASK 3 — `DEPLOY_VERSION` env + LB/CDN annotations on alpha-b.yaml (doc/lint only)

**Files:** `genesis/orchestrator/manifests/doorway/alpha-b.yaml` (F-EDGE region-owner: `env:` `DEPLOY_VERSION` + Ingress `annotations` ONLY; F-DEPLOY owns the `rules`/failover posture on this file — keep that line).

- [ ] Add the same `DEPLOY_VERSION` env to the `elohim-doorway-alpha-b` container `env:` block (alongside `DOORWAY_ID` at `:193`):
```yaml
            - name: DEPLOY_VERSION
              value: "DEPLOY_VERSION_PLACEHOLDER"
```
- [ ] Add the same DRAFT `/blob/*` CDN annotation block to the alpha-b Ingress `metadata.annotations` (after `:417` `upstream-hash-by`), identical to Task 2's commented stanza. Leave `upstream-hash-by` (`:417`) UNTOUCHED.
- [ ] **Do NOT touch** the `affinity`/`nodeAffinity` `requiredDuringScheduling … remote … NO fallback` region (`:101-123`) or the Ingress `rules:` (`:432-443`). Those are F-DEPLOY's apex-failover/fail-loud posture (C-MANIFEST). The fail-loud apex (no matthew fallback) is LOAD-BEARING and deliberate — see §"Open decisions" (recommend NO auto-failover).
- [ ] Validate YAML (alpha-b leg of the §5 check).
- [ ] Commit:
```
git add genesis/orchestrator/manifests/doorway/alpha-b.yaml
git commit -m "chore(doorway-manifest): DEPLOY_VERSION env + draft /blob CDN posture (alpha-b)

Symmetric with alpha.yaml. Touches only the env block + Ingress annotations;
the remote-affinity fail-loud posture and Ingress rules stay F-DEPLOY's
(C-MANIFEST disjoint regions).

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## 6. p2p-class of new entities (p2p-design-gate)

- **`P2PPeersResponse` (rewritten) + `project_p2p_peers`** = **Cat-C node-local Operational read-model.** Swap test: any doorway computes its own `/p2p-peers` from its own routed storage `/p2p/status` — no doorway authors canonical content; the count is the doorway's *local view* of the mesh. No DHT entry type, no content-addressed identity, no coordinator fn, no notarized actuation. (Federation ledger §1: "rewritten `P2PPeersResponse` = Cat-C". Doorway CLAUDE.md: "Doorway-local Operational state (cache stats, federation peer list) is legitimate doorway-resident state.")
- **`DEPLOY_VERSION` env** = operational pod metadata, not an entity. Consumed by F-COHERENCE's `build_id` (Cat-C `CoherenceManifest`).
- **NO Cat-A in this track.** The only notarized actuation in the federation surface is the deploy coordinator-update flag (F-DEPLOY, consuming the shipped `Mishpat::Commitment`/`ALLOW_COORDINATOR_UPDATE` binary path) — NOT F-EDGE.

Per the gate: new runtime entities are Cat-C node-local read-models. Cited, not re-litigated.

---

## 7. // FOLLOW-ON seams (for the integration pass / named siblings)

1. **`/blob/*` CDN enable (X-EDGE-DEF, WAVE F3).** Uncomment the `proxy_cache`/`server-snippet` stanza in BOTH manifests ONLY after dataplane P-DEFENSE adds the `immutable` Cache-Control to `storage_proxy.rs:492-497` (the miss-then-200 forward path — VERIFIED missing; the pantry-HIT path at `:369-370` already has it). Until then the stanza stays commented DRAFT. **SEAM owner: integration pass coordinating with P-DEFENSE.**
2. **`epr.rs:40` pooled-client migration (X-EDGE-DEF, HARD hand-off).** The EPR-head read path uses a per-request `reqwest::Client::new()` (10s timeout, no breaker) — it bypasses `forward_to_storage`'s `UpstreamBreakers`. Dataplane P-DEFENSE owns the per-request-client residual cleanup (dataplane ledger §2 line 113 lists `epr.rs`). F-EDGE FLAGS only. **SEAM owner: P-DEFENSE.**
3. **Cross-edge head surfacing in `/p2p-peers` (optional enrichment).** Once F-COHERENCE's `CoherenceView` (FS3) is live, the `/p2p-peers` response *could* gain a `cross_edge_coherent: bool` field read from `routes::coherence`. Deliberately NOT in v1 (keeps the peer route a pure mesh-count read; coherence has its own endpoint `GET /api/v1/federation/coherence`). **SEAM owner: integration pass, only if the operator wants a one-stop peer+coherence view.**
4. **Real LB / health-gated apex failover (operator decision — see Open decisions).** NOT auto-wired. The detection (F-COHERENCE) lands first; a deliberate operator-flipped read-only fallback could later consume it. **SEAM owner: operator + F-COHERENCE.**

---

## Open decisions (operator-only) + recommendation

1. **Real LB + health-gated failover for apex (elohim.host → matthew)?** **Recommend NO automatic failover; surface divergence and let the operator flip a deliberate fallback.** The A=matthew/B=adam pinning is load-bearing (genesis pair; `requiredDuringScheduling … remote … NO fallback` is the *intended fail-loud*, alpha-b.yaml:101-123). Auto-failover would let apex silently serve a DIVERGENT head from matthew — strictly worse than a clean 503. Keep `requiredDuringScheduling` for B. Detection comes from F-COHERENCE.
2. **CDN for immutable-CID responses?** **Recommend YES, scoped to `/blob/<hash>` ONLY** — content-addressed bytes are safely cacheable forever (CID = cache key, never mutates). Prereq: the immutable Cache-Control on the miss path (X-EDGE-DEF, dataplane P-DEFENSE). Then a front CDN / nginx `proxy_cache` is purge-free. Do NOT CDN any EPR-head/view route (mutable). No new origin-coherence problem because CIDs are immutable.
3. **Atomic A+B deploy gate?** **Recommend YES** — but it is **F-DEPLOY's `deployGenesisPairAtomic` (FS7)**, NOT a separate F-EDGE flag (C-DEPLOY: F-EDGE's skew-gate intent folds into F-DEPLOY's barrier as a sub-assertion that A and B land identical `DEPLOY_VERSION`). F-EDGE contributes the assertion semantics + the `DEPLOY_VERSION` env (Tasks 2–3); F-DEPLOY writes the Groovy.

---

## Dispatch note

- **Isolated-worktree, subagent-driven, commit-only.** Run from a dedicated worktree off the integration branch. The integrator pushes/merges (memory: commit-only; never `git push`).
- **Sequencing:** Task 1 (`/p2p-peers` rewrite) is fully INDEPENDENT — begin in WAVE F1, complete in F2. Tasks 2–3 (manifest env + DRAFT CDN) are doc/lint, independent, may land any time in F1/F2. The CDN-enable uncomment is a WAVE F3 cross-layer hand-off behind P-DEFENSE (FOLLOW-ON seam 1).
- **Selective-stage** each commit (concurrent sessions may share the worktree per memory) — the per-task `git add` lists name exact files only; never bulk-revert ambient mods.
- **RUSTFLAGS=""** for doorway (native) is load-bearing — mixing the WASM `getrandom` flag link-fails with `undefined __getrandom_v03_custom`. `/tmp` target dir + `RUSTC_WRAPPER=""` + plain `cargo test` (no nextest in container).
- **Manifests + edge Jenkinsfile are operator-applied** — F-EDGE describes the manifest CHANGES (the repo is the cleanup surface); never `kubectl`. The `/blob/*` CDN stanza ships commented so a premature apply is inert.
