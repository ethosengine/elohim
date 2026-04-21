# Phase 2a Batch C Pivot — P2P-Native Route Design

**Status:** Addendum to `2026-04-22-elohim-epr-storage-foundation-plan.md`. Supersedes Tasks 12-17 in that document.
**Date:** 2026-04-22
**Reason:** Original Batch C designed routes as single-node REST over a local diesel store — ignoring the existing `/elohim/epr/1.0.0` libp2p protocol (`p2p/epr_protocol.rs`), Kademlia DHT (`p2p/kad_store.rs`), and swarm behaviour (`p2p/behaviour.rs`) already active in elohim-storage. This pivot aligns the new EPR routes with the existing P2P substrate so content discovery is peer-federated from day one.

---

## What changed in the architecture

### What elohim-storage already has (discovered during Batch B review)

- **`EprProtocol`** — request-response libp2p protocol (`/elohim/epr/1.0.0`) with `EprRequest::{Resolve, Announce, ResolveBatch}` and `EprResponse::{Head, AccessDenied, NotFound, Announced}`. Handles access control inline (reach enforcement at protocol level).
- **`SledRecordStore`** — Kademlia record store using sled, for DHT provider records.
- **`ElohimStorageBehaviour`** — swarm behaviour composing Kademlia + request_response (for EprProtocol + shard_protocol + trust_protocol + sync_protocol) + mDNS + relay + dcutr + identify.
- **`handle_epr_request`** in `p2p/mod.rs` — the server side of the EprProtocol. Resolves local content, enforces reach, returns bytes or `AccessDenied`/`NotFound`.

Currently the content resolved by `handle_epr_request` is the legacy `EprHead` format (~500B content-addressed envelope with embedded lamad/shefa/qahal contexts), not the Phase 1 generalized `Envelope`. These two shapes must eventually reconcile — but that's a Phase 2c (or later) concern. This pivot keeps the two shapes parallel and ensures the new Envelope plugs into the SAME P2P substrate cleanly when reconciliation lands.

### New design principle

Phase 2a ships the EPR storage layer as a P2P-federated service, not a local DB with HTTP on top. The routes speak to an `EprStore` trait that has two implementations:

- **`LocalEprStore`** — wraps the diesel layer (from Batches A + B). Ships in Phase 2a.
- **`FederatedEprStore`** — wraps `LocalEprStore` + a swarm handle; on local miss, issues `EprRequest::Resolve` via libp2p; on ingest, announces via Kad `start_providing`. **Stubbed in Phase 2a** — the struct exists and its `LocalEprStore`-delegating methods are active, but the libp2p wiring for federated fetch/ingest is marked `TODO(phase-2c)` and falls through to local-only behaviour. This keeps the route contract stable while deferring the libp2p bridge work (which requires decisions on EprHead↔Envelope reconciliation).

The routes ONLY talk to the trait. Route code never changes when Phase 2c swaps LocalEprStore → FederatedEprStore at construction time.

### Route shape changes

| Original (Batch C) | Pivot |
|---|---|
| `POST /api/v1/epr` — "publish" | `PUT /api/v1/epr/:cid` — idempotent content-addressed put; re-put same CID = 200 not error |
| 404 on local miss | 404 header explains "not found locally or in provider network" — Phase 2c populates provider query |
| No provenance info in GETs | `X-Epr-Source: local` on all GETs; Phase 2c extends to `peer:<id>` |
| No provider query | New route `GET /api/v1/epr/:cid/providers` returns `{ "providers": [...] }` from DHT — Phase 2a returns `["local"]` stub |
| List is implicitly local | `GET /api/v1/epr` returns LOCAL atoms; new `?federated=true` query param documented as Phase 2c |

### What stays the same from the original Batch C

- All 6 routes land under `/api/v1/epr/...`
- Reach enforcement at envelope level (no payload parse)
- Schema-first wire contracts (from Task 2 in the original plan)
- Hyper-based dispatcher, following existing `economic_events.rs` pattern
- ts-rs wire view types (from Task 11 in Batch B)

---

## Revised Task List (Tasks 12-17)

Six tasks. TDD per task, atomic commit per task. Uses `RUSTFLAGS='--cfg getrandom_backend="custom"'`.

---

### Task 12 — `EprStore` trait + `LocalEprStore` + `FederatedEprStore` stub + API scaffolding

**Files:**
- Create: `elohim/elohim-storage/src/services/epr_store.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`
- Create: `elohim/elohim-storage/src/api/epr.rs` (dispatcher + stubs for 6 routes)
- Modify: `elohim/elohim-storage/src/api/mod.rs`

- [ ] **Step 1: Write the trait + implementations**

Create `elohim/elohim-storage/src/services/epr_store.rs`:

```rust
//! EprStore trait — the seam between REST routes and storage backends.
//!
//! LocalEprStore wraps the diesel layer (Phase 2a).
//! FederatedEprStore wraps LocalEprStore + a libp2p swarm handle and, on local
//! miss, issues EprRequest::Resolve via /elohim/epr/1.0.0 to discover peers
//! that hold the atom. Phase 2a ships FederatedEprStore with the libp2p bridge
//! stubbed as TODO(phase-2c); Phase 2c wires the swarm handle and flips the
//! construction site from LocalEprStore → FederatedEprStore.

use diesel::SqliteConnection;

use crate::db::epr_atoms::EprListQuery;
use crate::error::StorageError;
use crate::services::epr_service::{self, EprIngestResult, EprVerifyReport, FetchedEpr};
use crate::db::epr_atoms::EprAtom;
use elohim_epr::Epr;

/// Outcome of a fetch — the atom plus its source-of-truth provenance.
#[derive(Debug, Clone)]
pub struct FetchOutcome {
    pub fetched: FetchedEpr,
    pub source: EprSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EprSource {
    /// Served from the local diesel store.
    Local,
    /// Served by fetching from a remote peer and caching locally.
    /// The String is the remote peer id.
    Peer(String),
}

impl EprSource {
    pub fn header_value(&self) -> String {
        match self {
            EprSource::Local => "local".into(),
            EprSource::Peer(id) => format!("peer:{id}"),
        }
    }
}

/// Reference to a peer that claims to have a given CID.
#[derive(Debug, Clone)]
pub struct ProviderRef {
    pub peer_id: String,
    pub advertised_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl ProviderRef {
    pub fn local() -> Self {
        ProviderRef { peer_id: "local".into(), advertised_at: None }
    }
}

/// Abstraction over "where EPRs live." Route handlers never know whether the
/// store is local-only or federated.
pub trait EprStore: Send + Sync {
    /// Look up an EPR by CID. Returns None if the CID is not reachable by
    /// this store (i.e., not local and not available from peers).
    fn fetch(
        &self,
        conn: &mut SqliteConnection,
        cid: &str,
    ) -> Result<Option<FetchOutcome>, StorageError>;

    /// Idempotent put. If the CID already exists locally, validates inbound
    /// matches stored and returns the existing result (200, not 409).
    fn put(
        &self,
        conn: &mut SqliteConnection,
        epr: Epr,
    ) -> Result<EprIngestResult, StorageError>;

    /// List atoms held locally by this store. Federation across peers is a
    /// separate concern — see `list_federated` in Phase 2c.
    fn list(
        &self,
        conn: &mut SqliteConnection,
        q: &EprListQuery,
    ) -> Result<(Vec<EprAtom>, Option<String>), StorageError>;

    /// Verify a stored EPR against a caller-supplied public key. Local
    /// operation — no network required.
    fn verify(
        &self,
        conn: &mut SqliteConnection,
        cid: &str,
        public_key: &[u8; 32],
    ) -> Result<EprVerifyReport, StorageError>;

    /// Return the set of peers advertised as holding this CID. Phase 2a
    /// returns only `[ProviderRef::local()]` when the atom is held locally,
    /// or an empty Vec when it isn't. Phase 2c extends with DHT provider
    /// records via Kad `get_providers`.
    fn providers(
        &self,
        conn: &mut SqliteConnection,
        cid: &str,
    ) -> Result<Vec<ProviderRef>, StorageError>;
}

// ---------------------------------------------------------------------------
// LocalEprStore
// ---------------------------------------------------------------------------

pub struct LocalEprStore;

impl LocalEprStore {
    pub fn new() -> Self { LocalEprStore }
}

impl Default for LocalEprStore {
    fn default() -> Self { LocalEprStore::new() }
}

impl EprStore for LocalEprStore {
    fn fetch(
        &self,
        conn: &mut SqliteConnection,
        cid: &str,
    ) -> Result<Option<FetchOutcome>, StorageError> {
        Ok(epr_service::fetch_by_cid(conn, cid)?.map(|fetched| FetchOutcome {
            fetched,
            source: EprSource::Local,
        }))
    }

    fn put(
        &self,
        conn: &mut SqliteConnection,
        epr: Epr,
    ) -> Result<EprIngestResult, StorageError> {
        let cid = epr.envelope.cid.to_string();
        // Idempotent semantics: if the CID already exists, return the stored result.
        if let Some(existing) = epr_service::fetch_by_cid(conn, &cid)? {
            // Verify inbound canonical bytes match stored (catches collision attempts
            // where caller re-publishes a different signer/proof over the same cid).
            let inbound_canonical = epr.envelope
                .canonical_bytes(&epr.payload)
                .map_err(|e| StorageError::InvalidInput(format!("canonicalize: {e}")))?;
            if inbound_canonical != existing.atom.canonical_bytes {
                return Err(StorageError::InvalidInput(format!(
                    "cid {cid} already exists with different canonical bytes — not idempotent"
                )));
            }
            return Ok(EprIngestResult { cid });
        }
        epr_service::ingest(conn, epr)
    }

    fn list(
        &self,
        conn: &mut SqliteConnection,
        q: &EprListQuery,
    ) -> Result<(Vec<EprAtom>, Option<String>), StorageError> {
        epr_service::list(conn, q)
    }

    fn verify(
        &self,
        conn: &mut SqliteConnection,
        cid: &str,
        public_key: &[u8; 32],
    ) -> Result<EprVerifyReport, StorageError> {
        epr_service::verify(conn, cid, public_key)
    }

    fn providers(
        &self,
        conn: &mut SqliteConnection,
        cid: &str,
    ) -> Result<Vec<ProviderRef>, StorageError> {
        match epr_service::fetch_by_cid(conn, cid)? {
            Some(_) => Ok(vec![ProviderRef::local()]),
            None => Ok(vec![]),
        }
    }
}

// ---------------------------------------------------------------------------
// FederatedEprStore — Phase 2a stub; Phase 2c wires libp2p bridge
// ---------------------------------------------------------------------------

/// Federated store that, in Phase 2c, will bridge to the existing
/// /elohim/epr/1.0.0 libp2p protocol and Kad DHT. In Phase 2a it falls through
/// to LocalEprStore with explicit TODO markers for each federation seam.
pub struct FederatedEprStore {
    local: LocalEprStore,
    // TODO(phase-2c): swarm_handle: SwarmHandle — channel into the running
    // elohim-storage swarm so fetch/put can issue EprRequest::Resolve and
    // Kad start_providing. Requires resolving EprHead↔Envelope format
    // compatibility (see Phase 2c pivot doc).
}

impl FederatedEprStore {
    pub fn new() -> Self {
        FederatedEprStore { local: LocalEprStore::new() }
    }
}

impl Default for FederatedEprStore {
    fn default() -> Self { FederatedEprStore::new() }
}

impl EprStore for FederatedEprStore {
    fn fetch(
        &self,
        conn: &mut SqliteConnection,
        cid: &str,
    ) -> Result<Option<FetchOutcome>, StorageError> {
        if let Some(outcome) = self.local.fetch(conn, cid)? {
            return Ok(Some(outcome));
        }
        // TODO(phase-2c): on local miss, issue swarm_handle.resolve_epr(cid).
        // For each returned peer, send EprRequest::Resolve { id: cid }; if
        // EprResponse::Head arrives, decode + validate + LocalEprStore::put + return
        // FetchOutcome::Peer(peer_id). Give up after N peers or T timeout.
        Ok(None)
    }

    fn put(
        &self,
        conn: &mut SqliteConnection,
        epr: Epr,
    ) -> Result<EprIngestResult, StorageError> {
        let result = self.local.put(conn, epr)?;
        // TODO(phase-2c): self.swarm_handle.kad_start_providing(result.cid.parse()?).await?;
        // This announces to the DHT that this node holds the atom, so future
        // fetch requests from other peers can find us via EprRequest::Resolve.
        Ok(result)
    }

    fn list(
        &self,
        conn: &mut SqliteConnection,
        q: &EprListQuery,
    ) -> Result<(Vec<EprAtom>, Option<String>), StorageError> {
        // Federated list is a separate concern (spans many peers + aggregation
        // + cursor across them). For now, delegate to local.
        self.local.list(conn, q)
    }

    fn verify(
        &self,
        conn: &mut SqliteConnection,
        cid: &str,
        public_key: &[u8; 32],
    ) -> Result<EprVerifyReport, StorageError> {
        self.local.verify(conn, cid, public_key)
    }

    fn providers(
        &self,
        conn: &mut SqliteConnection,
        cid: &str,
    ) -> Result<Vec<ProviderRef>, StorageError> {
        let mut providers = self.local.providers(conn, cid)?;
        // TODO(phase-2c): extend providers with DHT provider records:
        //   let dht_providers = self.swarm_handle.kad_get_providers(cid).await?;
        //   providers.extend(dht_providers.into_iter().map(ProviderRef::from));
        Ok(providers)
    }
}
```

- [ ] **Step 2: Register the module + the trait's AppContext handle**

Edit `elohim/elohim-storage/src/services/mod.rs`:

```rust
pub mod epr_store;
```

The route layer instantiates a concrete store at app start. For Phase 2a, use `FederatedEprStore::default()` at the construction site — it still delegates everything to local, but the switch to real federation is a one-line swap in Phase 2c (wire up `swarm_handle`).

Find where other services are constructed (e.g., `AppContext::new`, or wherever routes access services) and add:

```rust
// In AppContext or equivalent:
pub epr_store: std::sync::Arc<dyn crate::services::epr_store::EprStore>,

// In the constructor:
epr_store: std::sync::Arc::new(crate::services::epr_store::FederatedEprStore::default()),
```

If AppContext is not modifiable without cascading changes, create a free-standing factory function `pub fn default_epr_store() -> impl EprStore` in `epr_store.rs` and have the api/epr.rs dispatcher call it directly. Minimize existing-code disruption.

- [ ] **Step 3: Create `api/epr.rs` with dispatcher + 6 stubs**

```rust
//! EPR REST controller — routes under /api/v1/epr.
//!
//! Routes talk to an EprStore trait so P2P federation (Phase 2c) can be added
//! without route changes. Phase 2a ships FederatedEprStore with the libp2p
//! bridge stubbed — all calls fall through to LocalEprStore.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response, StatusCode};

use crate::db::AppContext;
use crate::error::StorageError;
use crate::services::epr_store::{default_epr_store, EprStore, FetchOutcome};
use crate::services::response::{self, not_found};

use super::{get_conn, parse_body};

pub async fn handle(
    req: Request<Incoming>,
    ctx: &AppContext,
    path_tail: &[&str],
) -> Result<Response<Full<Bytes>>, StorageError> {
    match (req.method(), path_tail) {
        (&Method::GET,  [cid])                    => get_epr(ctx, cid, &req).await,
        (&Method::GET,  [cid, "envelope"])        => get_envelope(ctx, cid, &req).await,
        (&Method::GET,  [cid, "payload"])         => get_payload(ctx, cid, &req).await,
        (&Method::GET,  [cid, "verify"])          => get_verify(ctx, cid, &req).await,
        (&Method::GET,  [cid, "providers"])       => get_providers(ctx, cid, &req).await,
        (&Method::PUT,  [cid])                    => put_epr(ctx, cid, req).await,
        (&Method::GET,  [])                       => list_epr(ctx, &req).await,
        _ => Ok(response::not_found()),
    }
}

// Stubs replaced by Tasks 13-17.
async fn get_epr(_: &AppContext, _: &str, _: &Request<Incoming>) -> Result<Response<Full<Bytes>>, StorageError> { Ok(not_found()) }
async fn get_envelope(_: &AppContext, _: &str, _: &Request<Incoming>) -> Result<Response<Full<Bytes>>, StorageError> { Ok(not_found()) }
async fn get_payload(_: &AppContext, _: &str, _: &Request<Incoming>) -> Result<Response<Full<Bytes>>, StorageError> { Ok(not_found()) }
async fn get_verify(_: &AppContext, _: &str, _: &Request<Incoming>) -> Result<Response<Full<Bytes>>, StorageError> { Ok(not_found()) }
async fn get_providers(_: &AppContext, _: &str, _: &Request<Incoming>) -> Result<Response<Full<Bytes>>, StorageError> { Ok(not_found()) }
async fn put_epr(_: &AppContext, _: &str, _: Request<Incoming>) -> Result<Response<Full<Bytes>>, StorageError> { Ok(not_found()) }
async fn list_epr(_: &AppContext, _: &Request<Incoming>) -> Result<Response<Full<Bytes>>, StorageError> { Ok(not_found()) }
```

- [ ] **Step 4: Register in `api/mod.rs`**

```rust
pub mod epr;
```

And wire into the main `/api/v1/*` dispatcher where `economic-events`, `humans`, etc. are routed. Add the `"epr"` branch alongside.

- [ ] **Step 5: Build**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check -p elohim-storage 2>&1 | tail -5
```

Expected: clean.

- [ ] **Step 6: Commit**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1
git add elohim/elohim-storage/src/services/epr_store.rs elohim/elohim-storage/src/services/mod.rs elohim/elohim-storage/src/api/epr.rs elohim/elohim-storage/src/api/mod.rs
git commit -m "feat(epr): EprStore trait + Local/Federated impls + API scaffolding

The route layer depends on an EprStore trait so Phase 2c can flip
construction from LocalEprStore → FederatedEprStore without any
route changes. FederatedEprStore ships with libp2p seams stubbed
(TODO(phase-2c) markers at fetch/put/providers) so the contract
is defined now and the wiring drops in cleanly when
EprHead↔Envelope reconciliation lands.

PUT /api/v1/epr/:cid (idempotent content-addressed put) replaces the
original POST /api/v1/epr.
GET /api/v1/epr/:cid/providers added for peer-provider discovery.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 13 — `GET /api/v1/epr/:cid` + envelope view rendering

**Files:**
- Modify: `elohim/elohim-storage/src/api/epr.rs`

- [ ] **Step 1: Implement `get_epr` + envelope view helpers**

Replace the `get_epr` stub. Reach enforcement reads from the envelope's reach column only (no payload parse). Returns `X-Epr-Source: local` on the response — Phase 2c extends with `peer:<id>` when FederatedEprStore upgrades.

```rust
async fn get_epr(
    ctx: &AppContext,
    cid: &str,
    req: &Request<Incoming>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let include_canonical = req.uri().query()
        .map(|q| q.contains("includeCanonical=true"))
        .unwrap_or(false);

    let store = default_epr_store();
    let mut conn = get_conn(ctx)?;

    let Some(outcome) = store.fetch(&mut conn, cid)? else {
        return Ok(not_found());
    };
    if !reach_visible_to(&outcome.fetched.atom.reach, req) {
        return Ok(not_found());
    }

    let view = to_epr_view(&outcome.fetched, include_canonical);
    let body = serde_json::to_vec(&view)
        .map_err(|e| StorageError::Database(format!("serialize: {e}")))?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .header("X-Epr-Source", outcome.source.header_value())
        .body(Full::new(Bytes::from(body)))
        .unwrap())
}

pub(crate) fn to_envelope_view(fetched: &crate::services::epr_service::FetchedEpr) -> crate::views::EprEnvelopeView {
    let mut coupling = crate::views::EprCouplingView::default();
    for row in &fetched.coupling {
        match row.leg.as_str() {
            "knowledge" => coupling.knowledge = Some(row.target_cid.clone()),
            "value" => coupling.value = Some(row.target_cid.clone()),
            "governance" => coupling.governance = Some(row.target_cid.clone()),
            _ => {}
        }
    }
    crate::views::EprEnvelopeView {
        cid: fetched.atom.cid.clone(),
        kind: fetched.atom.kind.clone(),
        schema_ref: fetched.atom.schema_ref.clone(),
        schema_key: fetched.atom.schema_key.clone(),
        reach: fetched.atom.reach.clone(),
        coupling,
        claims: fetched.claims.iter().map(|c| c.claim_cid.clone()).collect(),
        supersedes: fetched.atom.supersedes.clone(),
        superseded_by: fetched.superseded_by.clone(),
        issued_at: fetched.atom.issued_at.clone(),
        proof: crate::views::EprSignatureView {
            signer: fetched.atom.signer_cid.clone(),
            algorithm: fetched.atom.proof_algorithm.clone(),
            signature: hex::encode(&fetched.atom.proof_bytes),
        },
    }
}

fn to_epr_view(fetched: &crate::services::epr_service::FetchedEpr, include_canonical: bool) -> crate::views::EprView {
    crate::views::EprView {
        envelope: to_envelope_view(fetched),
        payload: hex::encode(&fetched.atom.payload_bytes),
        canonical_bytes: if include_canonical {
            Some(hex::encode(&fetched.atom.canonical_bytes))
        } else { None },
    }
}

fn reach_visible_to(reach: &str, req: &Request<Incoming>) -> bool {
    match reach {
        "commons" | "public" => true,
        _ => req.headers()
            .get(hyper::header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false),
    }
}
```

- [ ] **Step 2: Build**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check -p elohim-storage 2>&1 | tail -5
```

- [ ] **Step 3: Commit**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1
git add elohim/elohim-storage/src/api/epr.rs
git commit -m "feat(epr): GET /api/v1/epr/:cid via EprStore trait + X-Epr-Source header

Full EprView response (envelope + hex payload; optional canonical
bytes via ?includeCanonical=true). Reach enforcement at envelope
level. 404 (not 403) on reach denial. X-Epr-Source: local on all
responses — Phase 2c extends to peer:<id> via FederatedEprStore.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 14 — `GET /api/v1/epr/:cid/envelope` + `GET /api/v1/epr/:cid/payload`

Combine two small routes in one task (both are straightforward variants of Task 13's fetch pattern).

**Files:**
- Modify: `elohim/elohim-storage/src/api/epr.rs`

- [ ] **Step 1: Replace both stubs**

```rust
async fn get_envelope(
    ctx: &AppContext,
    cid: &str,
    req: &Request<Incoming>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let store = default_epr_store();
    let mut conn = get_conn(ctx)?;
    let Some(outcome) = store.fetch(&mut conn, cid)? else { return Ok(not_found()); };
    if !reach_visible_to(&outcome.fetched.atom.reach, req) { return Ok(not_found()); }

    let body = serde_json::to_vec(&to_envelope_view(&outcome.fetched))
        .map_err(|e| StorageError::Database(format!("serialize: {e}")))?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .header("X-Epr-Source", outcome.source.header_value())
        .body(Full::new(Bytes::from(body)))
        .unwrap())
}

async fn get_payload(
    ctx: &AppContext,
    cid: &str,
    req: &Request<Incoming>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let store = default_epr_store();
    let mut conn = get_conn(ctx)?;
    let Some(outcome) = store.fetch(&mut conn, cid)? else { return Ok(not_found()); };
    if !reach_visible_to(&outcome.fetched.atom.reach, req) { return Ok(not_found()); }

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(hyper::header::CONTENT_TYPE, "application/octet-stream")
        .header("X-Epr-Cid", &outcome.fetched.atom.cid)
        .header("X-Epr-Source", outcome.source.header_value())
        .body(Full::new(Bytes::from(outcome.fetched.atom.payload_bytes.clone())))
        .unwrap())
}
```

- [ ] **Step 2: Build**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check -p elohim-storage 2>&1 | tail -5
```

- [ ] **Step 3: Commit**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1
git add elohim/elohim-storage/src/api/epr.rs
git commit -m "feat(epr): envelope-only + payload endpoints

GET /api/v1/epr/:cid/envelope returns JSON envelope (no payload).
GET /api/v1/epr/:cid/payload returns raw bytes as
application/octet-stream with X-Epr-Cid and X-Epr-Source headers.
Both honor reach enforcement + provenance pattern.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 15 — `GET /api/v1/epr/:cid/verify?publicKey=<hex>` + `GET /api/v1/epr/:cid/providers`

Two small routes together.

**Files:**
- Modify: `elohim/elohim-storage/src/api/epr.rs`
- Modify: `elohim/elohim-storage/src/services/response.rs` (add `bad_request` helper if not present)

- [ ] **Step 1: Add `response::bad_request` helper if missing**

Inspect `services/response.rs`. If no `bad_request(msg: &str) -> Response<...>`, add one matching the style of the existing helpers (e.g., `response::json`, `response::not_found`).

- [ ] **Step 2: Replace both stubs**

```rust
async fn get_verify(
    ctx: &AppContext,
    cid: &str,
    req: &Request<Incoming>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let query = req.uri().query().unwrap_or("");
    let Some(pk_hex) = query.split('&').find_map(|p| p.strip_prefix("publicKey=")) else {
        return Ok(response::bad_request("publicKey query parameter required"));
    };
    let Ok(pk_bytes) = hex::decode(pk_hex) else {
        return Ok(response::bad_request("publicKey must be 64 hex chars"));
    };
    if pk_bytes.len() != 32 {
        return Ok(response::bad_request("publicKey must decode to 32 bytes"));
    }
    let mut pk = [0u8; 32];
    pk.copy_from_slice(&pk_bytes);

    let store = default_epr_store();
    let mut conn = get_conn(ctx)?;
    let Some(outcome) = store.fetch(&mut conn, cid)? else { return Ok(not_found()); };
    if !reach_visible_to(&outcome.fetched.atom.reach, req) { return Ok(not_found()); }

    let report = store.verify(&mut conn, cid, &pk)?;
    let view = crate::views::EprVerifyView {
        cid: report.cid,
        verified: report.verified,
        stages_run: report.stages_run,
        stages_skipped: report.stages_skipped,
        error: report.error.map(|e| crate::views::EprVerifyErrorView {
            stage: e.stage,
            message: e.message,
        }),
    };
    let body = serde_json::to_vec(&view)
        .map_err(|e| StorageError::Database(format!("serialize: {e}")))?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .header("X-Epr-Source", outcome.source.header_value())
        .body(Full::new(Bytes::from(body)))
        .unwrap())
}

async fn get_providers(
    ctx: &AppContext,
    cid: &str,
    req: &Request<Incoming>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let store = default_epr_store();
    let mut conn = get_conn(ctx)?;

    // Reach check: we need to know the atom's reach to enforce, but if the
    // atom isn't locally known and we can't reach peers yet (Phase 2c), we
    // can't enforce reach before returning providers. For Phase 2a: if the
    // atom isn't local, return [] (no-op disclosure).
    if let Some(outcome) = store.fetch(&mut conn, cid)? {
        if !reach_visible_to(&outcome.fetched.atom.reach, req) { return Ok(not_found()); }
    }

    let providers = store.providers(&mut conn, cid)?;
    let provider_strings: Vec<String> = providers.into_iter().map(|p| p.peer_id).collect();
    let body = serde_json::to_vec(&serde_json::json!({
        "cid": cid,
        "providers": provider_strings,
    })).map_err(|e| StorageError::Database(format!("serialize: {e}")))?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap())
}
```

- [ ] **Step 3: Build**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check -p elohim-storage 2>&1 | tail -5
```

- [ ] **Step 4: Commit**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1
git add elohim/elohim-storage/src/api/epr.rs elohim/elohim-storage/src/services/response.rs
git commit -m "feat(epr): verify + providers endpoints

GET /api/v1/epr/:cid/verify validates stored EPR against a caller-
supplied ed25519 public key (hex query param). Stage 4 reported as
skipped pending Phase 3.

GET /api/v1/epr/:cid/providers returns peer ids that advertise
holding the atom. Phase 2a returns ['local'] when held locally,
[] otherwise. Phase 2c extends with Kad DHT get_providers.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 16 — `PUT /api/v1/epr/:cid` (idempotent) + `GET /api/v1/epr` (list)

Two routes together. Both write-shaped (PUT) and read-shaped (list) complete the basic CRUD surface.

**Files:**
- Modify: `elohim/elohim-storage/src/api/epr.rs`

- [ ] **Step 1: Replace both stubs**

```rust
async fn put_epr(
    ctx: &AppContext,
    path_cid: &str,
    req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    use elohim_epr::{Coupling, Envelope, Epr, EprKind, Reach, Signature};
    use std::str::FromStr;

    let (_parts, body) = req.into_parts();
    let input: crate::views::EprPublishInput = parse_body(body).await?;

    // Path CID must match envelope CID — this enforces the content-addressed contract
    // at the route level.
    if input.envelope.cid != path_cid {
        return Ok(response::bad_request(&format!(
            "path cid {} does not match envelope cid {}",
            path_cid, input.envelope.cid
        )));
    }

    // Rehydrate the Rust Epr from wire view.
    let cid = cid::Cid::from_str(&input.envelope.cid)
        .map_err(|e| StorageError::InvalidInput(format!("bad cid: {e}")))?;
    let schema_ref = cid::Cid::from_str(&input.envelope.schema_ref)
        .map_err(|e| StorageError::InvalidInput(format!("bad schemaRef: {e}")))?;
    let signer = cid::Cid::from_str(&input.envelope.proof.signer)
        .map_err(|e| StorageError::InvalidInput(format!("bad signer: {e}")))?;

    let kind = match input.envelope.kind.as_str() {
        "Content" => EprKind::Content,
        "Agent" => EprKind::Agent,
        "Manifest" => EprKind::Manifest,
        "Claim" => EprKind::Claim,
        "Observation" => EprKind::Observation,
        "EconomicEvent" => EprKind::EconomicEvent,
        "Commitment" => EprKind::Commitment,
        "Attestation" => EprKind::Attestation,
        "Delegation" => EprKind::Delegation,
        other => return Ok(response::bad_request(&format!("unknown kind: {other}"))),
    };

    let reach = match input.envelope.reach.as_str() {
        "private" => Reach::Private,
        "self" => Reach::SelfScope,
        "intimate" => Reach::Intimate,
        "trusted" => Reach::Trusted,
        "familiar" => Reach::Familiar,
        "community" => Reach::Community,
        "public" => Reach::Public,
        "commons" => Reach::Commons,
        other => return Ok(response::bad_request(&format!("unknown reach: {other}"))),
    };

    let coupling = Coupling {
        knowledge: input.envelope.coupling.knowledge.as_deref().map(cid::Cid::from_str).transpose()
            .map_err(|e| StorageError::InvalidInput(format!("bad knowledge cid: {e}")))?,
        value: input.envelope.coupling.value.as_deref().map(cid::Cid::from_str).transpose()
            .map_err(|e| StorageError::InvalidInput(format!("bad value cid: {e}")))?,
        governance: input.envelope.coupling.governance.as_deref().map(cid::Cid::from_str).transpose()
            .map_err(|e| StorageError::InvalidInput(format!("bad governance cid: {e}")))?,
    };

    let claims = input.envelope.claims.iter()
        .map(|s| cid::Cid::from_str(s))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| StorageError::InvalidInput(format!("bad claims cid: {e}")))?;

    let supersedes = input.envelope.supersedes.as_deref().map(cid::Cid::from_str).transpose()
        .map_err(|e| StorageError::InvalidInput(format!("bad supersedes cid: {e}")))?;

    let sig_bytes = hex::decode(&input.envelope.proof.signature)
        .map_err(|e| StorageError::InvalidInput(format!("bad signature hex: {e}")))?;
    if sig_bytes.len() != 64 {
        return Ok(response::bad_request("signature must decode to 64 bytes"));
    }

    // issued_at is already an RFC3339 String in the wire view. Parse for the Envelope.
    let issued_at = chrono::DateTime::parse_from_rfc3339(&input.envelope.issued_at)
        .map_err(|e| StorageError::InvalidInput(format!("bad issuedAt: {e}")))?
        .with_timezone(&chrono::Utc);

    let envelope = Envelope {
        cid,
        kind,
        schema_ref,
        schema_key: input.envelope.schema_key,
        reach,
        coupling,
        claims,
        supersedes,
        superseded_by: None,  // server-derived
        issued_at,
        proof: Signature::ed25519(signer, sig_bytes),
    };

    let payload = hex::decode(&input.payload)
        .map_err(|e| StorageError::InvalidInput(format!("bad payload hex: {e}")))?;

    let epr = Epr { envelope, payload };

    let store = default_epr_store();
    let mut conn = get_conn(ctx)?;
    let result = store.put(&mut conn, epr)?;

    // Idempotent: 200 if it already existed (exact-match), 201 if new. For simplicity
    // in Phase 2a we always return 200 — caller inspects nothing but the CID, and
    // the canonical-bytes check in LocalEprStore::put rejects non-idempotent re-puts.
    let body = serde_json::to_vec(&result)
        .map_err(|e| StorageError::Database(format!("serialize: {e}")))?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap())
}

async fn list_epr(
    ctx: &AppContext,
    req: &Request<Incoming>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    use crate::db::epr_atoms::EprListQuery;
    let query = req.uri().query().unwrap_or("");

    let mut list_query = EprListQuery { limit: 50, ..Default::default() };
    let caller_authed = req
        .headers()
        .get(hyper::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);

    for kv in query.split('&') {
        if let Some(v) = kv.strip_prefix("kind=")       { list_query.kind = Some(v.into()); }
        else if let Some(v) = kv.strip_prefix("reach=")     { list_query.reach = Some(v.into()); }
        else if let Some(v) = kv.strip_prefix("schemaRef=") { list_query.schema_ref = Some(v.into()); }
        else if let Some(v) = kv.strip_prefix("after=")     { list_query.after_cid = Some(v.into()); }
        else if let Some(v) = kv.strip_prefix("limit=")     {
            if let Ok(n) = v.parse::<i64>() { list_query.limit = n.clamp(1, 200); }
        }
    }

    if !caller_authed {
        if let Some(r) = &list_query.reach {
            if !matches!(r.as_str(), "commons" | "public") {
                return Ok(response::json(&crate::views::EprListView {
                    items: vec![], next_cursor: None,
                }));
            }
        } else {
            list_query.reach = Some("commons".into());
        }
    }

    let store = default_epr_store();
    let mut conn = get_conn(ctx)?;
    let (atoms, next_cursor) = store.list(&mut conn, &list_query)?;

    // N+1 is acceptable for Phase 2a (limit clamped to 200). Phase 2b may optimize.
    let mut items = Vec::with_capacity(atoms.len());
    for atom in &atoms {
        if let Some(outcome) = store.fetch(&mut conn, &atom.cid)? {
            items.push(to_envelope_view(&outcome.fetched));
        }
    }

    Ok(response::json(&crate::views::EprListView { items, next_cursor }))
}
```

- [ ] **Step 2: Build**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build 2>&1 | tail -5
```

- [ ] **Step 3: Commit**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1
git add elohim/elohim-storage/src/api/epr.rs
git commit -m "feat(epr): PUT (idempotent) + list endpoints

PUT /api/v1/epr/:cid validates path CID matches envelope CID, rehydrates
to Rust Epr, runs the 3-stage validator via EprStore::put. Idempotent:
re-put with identical canonical bytes returns 200 (not 409). Mismatched
bytes under the same CID rejected as InvalidInput.

GET /api/v1/epr?filters returns paged local list. Unauthed callers
default to reach=commons; limit clamped to 200. Federation across peers
is Phase 2c (?federated=true reserved).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 17 — Route provenance test + schema addition for providers view

**Files:**
- Create: `elohim/sdk/schemas/v1/views/epr-providers-view.schema.json`
- Modify: `elohim/elohim-storage/tests/schema_contract.rs` (add providers view conformance test)
- Modify: `elohim/elohim-storage/src/views.rs` (add `EprProvidersView`)

- [ ] **Step 1: Write the providers schema**

```json
{
  "$id": "epr:schema:view:epr-providers",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "EprProvidersView",
  "description": "Source of truth: peer providers advertising that they hold the atom at the given cid. Phase 2a returns ['local'] when held locally, [] otherwise. Phase 2c extends with Kad DHT provider records. Category C — operational (DHT query, reconstructed per request).",
  "type": "object",
  "required": ["cid", "providers"],
  "additionalProperties": false,
  "properties": {
    "cid": { "type": "string", "description": "CIDv1 base32 of the queried atom" },
    "providers": {
      "type": "array",
      "items": {
        "type": "string",
        "description": "Peer identifier — 'local' for this node, or libp2p PeerId for remote peers"
      }
    }
  }
}
```

- [ ] **Step 2: Add the view struct**

Append to `elohim/elohim-storage/src/views.rs`:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../sdk/storage-client-ts/src/generated/")]
pub struct EprProvidersView {
    pub cid: String,
    pub providers: Vec<String>,
}
```

Update `api/epr.rs`'s `get_providers` to use `EprProvidersView` instead of the ad-hoc `json!` macro — stays within the schema contract discipline:

```rust
// In get_providers, replace the json!(...) body with:
let view = crate::views::EprProvidersView {
    cid: cid.to_string(),
    providers: provider_strings,
};
let body = serde_json::to_vec(&view)
    .map_err(|e| StorageError::Database(format!("serialize: {e}")))?;
```

- [ ] **Step 3: Add schema contract test**

Add to `elohim/elohim-storage/tests/schema_contract.rs`:

```rust
#[test]
fn epr_providers_view_conforms() {
    use elohim_storage::views::EprProvidersView;
    let v = EprProvidersView {
        cid: "bafyrei...".into(),
        providers: vec!["local".into()],
    };
    let json = serde_json::to_value(&v).unwrap();
    validate_against_schema("views/epr-providers-view.schema.json", &json);
}

#[test]
fn epr_providers_view_schema_parses() {
    let _ = load_view_schema("epr-providers-view.schema.json");  // or the existing helper name
}
```

(Also add `epr-providers-view.schema.json` to the source-of-truth declaration enforcement test that was extended in Task 3.)

- [ ] **Step 4: Build + test**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test -p elohim-storage --test schema_contract 2>&1 | tail -5
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -p elohim-storage -- -D warnings 2>&1 | tail -5
```

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1
git add elohim/sdk/schemas/v1/views/epr-providers-view.schema.json elohim/elohim-storage/src/views.rs elohim/elohim-storage/src/api/epr.rs elohim/elohim-storage/tests/schema_contract.rs
git commit -m "feat(epr): EprProvidersView schema + contract test

Codifies the providers endpoint's wire contract. Six endpoints now
have full 6-layer IoC coverage (schema + Rust struct + contract test
+ ts-rs export + service trait + response pattern). Phase 2c
extends the providers list with Kad DHT records.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Pivot batch gate

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build -p elohim-storage 2>&1 | tail -3
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test -p elohim-storage --test schema_contract 2>&1 | tail -3
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test -p elohim-storage --test schema_contract_diesel_epr 2>&1 | tail -3
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -p elohim-storage -- -D warnings 2>&1 | tail -3
```

All clean. No route-level integration tests yet — those land in original Batch D (Tasks 18-22 of the original plan), with any needed adjustment for PUT (not POST) and the new providers route.

## What this pivot commits to

- **Route contract is stable.** Phase 2c's flip from `LocalEprStore` to a fully-wired `FederatedEprStore` is a construction-site change only. Routes, views, schemas, TS client — all unchanged.
- **P2P-native from day one in shape.** Even though Phase 2a ships with libp2p bridge stubbed, the code paths exist and the trait seams are defined. There's no centralized-server pattern baked in; there's a local-only fallback with explicit `TODO(phase-2c)` markers at every seam.
- **Reconciliation is acknowledged.** EprHead (existing `/elohim/epr/1.0.0` protocol format) vs generalized Envelope (Phase 1) is a real reconciliation cost. This pivot does NOT attempt the reconciliation — it's Phase 2c's scope. What this pivot does is ensure Phase 2a doesn't make that reconciliation harder: every new surface speaks the generalized Envelope; no new surface extends EprHead.
- **CID is path identity for PUT.** `PUT /api/v1/epr/:cid` enforces the content-addressed contract at the route boundary. Client cannot submit an envelope whose `cid` disagrees with the path.

## What this pivot explicitly defers to Phase 2c

- Actual libp2p bridge wiring inside `FederatedEprStore` (fetch-on-miss, put-announce, providers-from-DHT)
- EprHead ↔ Envelope format reconciliation
- Peer-source provenance (beyond `local`) in `X-Epr-Source` header
- `?federated=true` list query path
- Cross-peer batching (`EprRequest::ResolveBatch` integration)

---

## Original plan Task 17 handling

The ORIGINAL plan's Task 17 was `POST /api/v1/epr`. That no longer exists as a route — replaced by `PUT /api/v1/epr/:cid` in Task 16 of this pivot. The ORIGINAL Task 17's parse-and-rehydrate logic is subsumed into the new Task 16.

The pivot Task 17 (above) is NEW work — EprProvidersView schema and contract test, net-new surface.

## Task count

- Pivot Tasks 12-17 = 6 tasks (vs original 6)
- Same atomic-commit discipline
- Route count: 7 (vs original 6) — the new `GET /api/v1/epr/:cid/providers` is added

## Report format expected from the subagent

Per task: commit SHA, files changed, adaptations noted. At end: batch gate outputs, total commits added, any DONE_WITH_CONCERNS.
