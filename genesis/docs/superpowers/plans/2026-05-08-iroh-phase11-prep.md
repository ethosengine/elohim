---
title: iroh Phase 11 Cutover — Pre-Cutover Prep
status: design-only
created: 2026-05-08
parent: genesis/docs/superpowers/plans/2026-05-07-iroh-parallel-stack.md
related:
  - genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md (pending — gates plane scope)
  - elohim/elohim-storage/src/p2p_iroh/README.md
  - elohim/elohim-storage/tests/bench_blob_perf.rs
---

# iroh Phase 11 Cutover — Pre-Cutover Prep

Design + scaffolding work for the Phase 11 cutover gate. This document
holds:

1. Call-site catalogs for every libp2p surface that needs an iroh dispatch
   branch.
2. Design sketches for the dispatch shape (no runtime change yet — these
   become real code when Phase 11 implementation begins).
3. The rollback drill playbook.

**No code in this branch executes anything.** All sketches are markdown.
The Phase 11 verdict on which planes go iroh, which stay libp2p, and which
go dual-stack is owned by the architecture spec referenced above. This
document parameterizes the cutover so the spec's verdict slots in cleanly.

### P2P Design Gate scope of this doc

This document **catalogs and dispatches**; it does not introduce any new
notarized entities, sync messages, or wire-level data shapes. Two specific
clarifications for the design-gate audit:

- **All HTTP routes referenced** (`PUT /blob/{hash}`, `GET /blob/{hash}`,
  `POST /admin/seed/blob`) are **pre-existing**. They were designed under
  the p2p-design-gate at original introduction; this doc only catalogs how
  the cutover dispatches the existing routes onto the iroh transport. No
  new routes proposed here.
- **One new storage table is proposed** (`blob_address_index`, §3.5) and
  is explicitly **Category C (operational projection, transient)**. It is
  a local-only address mapping between two content-derived hashes
  (sha256, blake3) of the **same bytes** — both addresses are derivable
  from the blob content, so the source of truth is the content itself,
  not the index. The index is dropped post-cutover (§6.2). No DHT
  notarization needed; no `dht_anchor_hash` column applies. This matches
  the `peer_blob_inventory` precedent (`db/peer_blob_inventory.rs`
  comment: "Category C operational").

## Status — what's already done vs what this doc covers

| Phase 11 prereq | Status going in | This doc |
|---|---|---|
| #1 Backend wiring (sync, EPR, EPR-atom, shard, view-fed, identity, trust) | iroh ALPNs exist as trait-object backends; daemon still wires libp2p only | §5 catalogs every libp2p call site + dispatch sketch |
| #2 HTTP `/api/v1/blob/{hash}` graduation | Reads only from legacy SHA256 BlobStore | §3 designs the dual-format route |
| #3 Genesis seeder rewrite | Writes only to legacy BlobStore via HTTP | §4 catalogs writes + dual-write design |
| #4 Gossip topic broadcast wiring | iroh `IrohGossip` exists with `subscribe`/`broadcast`; daemon publishes only via libp2p | §2 catalogs all 6 publish sites + dispatch sketch |
| #5 Recovery e2e harness | Works on libp2p; no iroh harness | Out of scope here; sketch in §6 |
| #6 CI parity soak | Not yet scheduled | Out of scope — needs architecture spec first |
| #7 Alpha-cluster soak | Not yet started | Out of scope — needs operations plan |
| #8 Latency stress 10k round-trips | `bench_blob_perf` proves blob plane (4-294x bump). Other planes pending bench expansion | Bench expansion is a separate thread (see related prompt) |
| #9 Rollback drill playbook | Not written | §7 |
| #10 `peer_blob_inventory.blob_hash` drop migration | Migration exists for `blake3_hash` add; drop is post-cutover | §6 sketches the migration shape |

## Dispatch model — the dial we're sketching everywhere

The runtime selector is already in place (Phase 1):

```rust
pub enum TransportBackend { Libp2p, Iroh }
```

Loaded from `ELOHIM_TRANSPORT_BACKEND` env or the `transport_backend` TOML
field; `default()` is `Libp2p`.

The cutover shape at every call site is:

```rust
match config.transport_backend {
    TransportBackend::Libp2p => libp2p_path(...),
    TransportBackend::Iroh   => iroh_path(...),
}
```

Three architectural questions hover over every site (resolved by the
architecture spec, not here):

- Is this a plane that goes iroh-only, libp2p-only, or dual-stack?
- For dual-stack: best-effort to both, or primary + fallback?
- For iroh-only at maturity: what's the deprecation path for the libp2p
  branch (delete, gate behind feature, keep as fallback)?

§§2-5 sketch the dispatch but leave the verdict as a spec-derived input.

---

## §2 Gossip publish call-site catalog (prereq #4)

Six libp2p `gossipsub.publish(topic, bytes)` call sites. All in
`elohim/elohim-storage/src/p2p/mod.rs` inside the swarm event loop's
`handle_command` arm.

### 2.1 Inventory snapshot publish

- **Site:** `src/p2p/mod.rs:2192`
- **Topic:** `INVENTORY_TOPIC = "elohim/inventory/blob"`
- **Trigger:** `T22InventorySnapshot` task — periodic snapshot of local blob holdings
- **Payload:** `BlobInventorySnapshot` (rmp_serde)
- **Caller:** `P2PHandle::publish_inventory_snapshot` (direct, not via P2PCommand — uses `self.swarm.write().await`)

### 2.2 RecoveryInvitation publish

- **Site:** `src/p2p/mod.rs:2383`
- **Topic:** `RECOVERY_INVITATION_TOPIC = "recovery.invitation"`
- **Trigger:** `P2PCommand::PublishRecoveryInvitation(inv)` from M3 recovery flow
- **Payload:** `RecoveryInvitationGossip` (rmp_serde)

### 2.3 IdentityBinding publish (A.10)

- **Site:** `src/p2p/mod.rs:2415`
- **Topic:** `TOPIC_IDENTITY_BINDING = "elohim/identity/binding"`
- **Trigger:** `P2PCommand::PublishIdentityBinding(payload)` from `ReconcileController::on_agent_peer_binding`
- **Payload:** `IdentityBindingGossip` (rmp_serde)

### 2.4 RecoveryRevocation publish

- **Site:** `src/p2p/mod.rs:2442`
- **Topic:** `RECOVERY_REVOCATION_TOPIC = "recovery.revocation"`
- **Trigger:** `P2PCommand::PublishRecoveryRevocation(msg)` from M3/M4
- **Payload:** `RecoveryRevocationMessage` (rmp_serde)
- **Note:** `behaviour.rs:462` comment says publishers will migrate to `TOPIC_INTEGRITY_REVOCATION` ("elohim/integrity/revocation"); cutover may want to do that move at the same time.

### 2.5 EPR atom announce publish (D.3)

- **Site:** `src/p2p/mod.rs:2507`
- **Topic:** `topic_for(reach)` — dynamic, per-reach topic name
- **Trigger:** `P2PCommand::PublishEprAnnounce { topic, payload }` from EPR ingest path
- **Payload:** msgpack-encoded CID

### 2.6 GossipPublish (gossip-flood)

- **Site:** `src/p2p/mod.rs:2603`
- **Topic:** Caller-supplied (typically reach-scoped via `topic_for`)
- **Trigger:** `P2PCommand::GossipPublish { topic, payload }` from `gossip_flood::flood_feedback`
- **Payload:** `FeedbackSignal` (rmp_serde)

### 2.7 Iroh-side surface (target)

`src/p2p_iroh/gossip.rs` exposes:

```rust
pub fn topic_id_for(name: &str) -> TopicId  // BLAKE3(name)[..32]
pub async fn subscribe(name, bootstrap) -> (GossipSender, GossipReceiver)
// then sender.broadcast(payload)
```

Three architectural mismatches with libp2p gossipsub:

| Concern | libp2p gossipsub | iroh-gossip |
|---|---|---|
| Publish handle | Stateless — `swarm.behaviour_mut().gossipsub.publish(topic, bytes)` | Per-topic `GossipSender` from a prior `subscribe()` call |
| Topic identity | String name | 32-byte `TopicId` (we map via `BLAKE3(name)[..32]`) |
| Bootstrap | None — gossipsub mesh forms via libp2p connections | Each `subscribe(name, bootstrap)` takes a `Vec<NodeId>` of peers known to be on this topic |

The bootstrap requirement is the load-bearing wrinkle. iroh-gossip
**must know peer NodeIds** to join a topic. libp2p gossipsub does not.

### 2.8 Cutover dispatch design

Two patterns, pick per architecture-spec verdict:

**Pattern A: TopicSenderRegistry (eager subscribe)**

```rust
pub struct GossipBridge {
    backend: TransportBackend,
    libp2p_swarm: Option<Arc<RwLock<Swarm<...>>>>,
    iroh_senders: HashMap<String, GossipSender>,  // topic_name → sender
    iroh_gossip: Option<Arc<IrohGossip>>,
}

impl GossipBridge {
    pub async fn publish(&mut self, topic: &str, bytes: Vec<u8>) -> Result<()> {
        match self.backend {
            TransportBackend::Libp2p => {
                let topic = libp2p::gossipsub::IdentTopic::new(topic);
                self.libp2p_swarm.as_ref()
                    .unwrap()
                    .write().await
                    .behaviour_mut()
                    .gossipsub
                    .publish(topic, bytes)
                    .map(|_| ())
                    .map_err(...)
            }
            TransportBackend::Iroh => {
                let sender = self.ensure_subscribed(topic).await?;
                sender.broadcast(bytes.into()).await
            }
        }
    }

    async fn ensure_subscribed(&mut self, name: &str) -> Result<&GossipSender> {
        if !self.iroh_senders.contains_key(name) {
            // Bootstrap: from peer_blob_inventory or cross_stack_peer_map?
            let bootstrap = self.resolve_bootstrap(name).await?;
            let (sender, receiver) = self.iroh_gossip.as_ref().unwrap()
                .subscribe(name, bootstrap).await?;
            self.iroh_senders.insert(name.to_string(), sender);
            // Spawn task to drain receiver into existing handler paths
            tokio::spawn(receiver_drain_task(name.to_string(), receiver));
        }
        Ok(self.iroh_senders.get(name).unwrap())
    }
}
```

**Bootstrap source for `resolve_bootstrap`:**
- Static topics (inventory, identity-binding, recovery-*, integrity-*): read from
  `cross_stack_peer_map` for peers known to be on the substrate (any peer with a
  `node_id` populated).
- Dynamic reach-topics (`topic_for(reach)`): read from `peer_blob_inventory`
  filtered by reach context, joined with `cross_stack_peer_map` for `node_id`.
- Fallback: empty `Vec<NodeId>` — produces an isolated subscriber until
  another peer broadcasts; acceptable for best-effort gossip flood.

**Pattern B: Dual-stack best-effort**

For dual-stack planes, publish to both:

```rust
pub async fn publish_dual(&mut self, topic: &str, bytes: Vec<u8>) -> DualResult {
    let libp2p_result = self.publish_libp2p(topic, bytes.clone()).await;
    let iroh_result = self.publish_iroh(topic, bytes).await;
    DualResult { libp2p: libp2p_result, iroh: iroh_result }
}
```

Pattern B is appropriate during the cutover window (T-cutover − N days)
where peers may be on either stack. After the alpha-cluster soak proves
iroh-side has full subscriber base, drop Pattern B in favour of Pattern A.

### 2.9 Receive-path symmetry

For each topic, a receive path also exists. Currently the libp2p side
deserializes incoming gossipsub messages in the swarm event loop and
projects to the appropriate handler (e.g., `peer_blob_inventory`, signal
projection, etc.). The iroh-side `receiver_drain_task` referenced in
Pattern A must call the same projection functions — schemas are identical
(see plan §"Wire pattern (universal across Phases 5–9)").

### 2.10 Cutover order recommendation

The architecture spec will determine final scope. Provisional ordering
(based on current understanding):

1. **Inventory** — high-volume, well-tested, easy to validate via blob count parity. **First.**
2. **Identity-binding** — load-bearing for handshake; exercise carefully.
3. **EPR atom announce** — high-volume on the substrate; benefits most from iroh perf bump.
4. **Recovery-invitation / recovery-revocation** — low volume, stake-bearing; cutover **last** to maximise observation window.
5. **GossipPublish (gossip-flood)** — feedback-signal floods; cutover with EPR atom.
6. **Integrity-revocation** — pending the rename mentioned in §2.4.

---

## §3 HTTP `/api/v1/blob/{hash}` route graduation design (prereq #2)

### 3.1 Current state

`src/http.rs:604-612`:

```rust
(Method::PUT, p) if p.starts_with("/blob/") => {
    let hash = p.strip_prefix("/blob/").unwrap_or("");
    self.handle_put_blob(req, hash).await
}
(Method::GET, p) if p.starts_with("/blob/") => {
    let hash = p.strip_prefix("/blob/").unwrap_or("");
    let agent_id = Self::extract_agent_id(&req);
    self.handle_get_blob(hash, agent_id.as_deref()).await
}
```

Both call into `self.blob_store: BlobStore` (legacy SHA256-keyed). The
hash format is `sha256-{64 hex}` or raw SHA-256 hex.

### 3.2 Two address formats during cutover

| Stack | Format | Disk path |
|---|---|---|
| libp2p (default) | `sha256-{64 hex}` or raw hex | `<storage_dir>/blobs/` |
| iroh | BLAKE3 (raw 64 hex) | `<storage_dir>/blobs_iroh/` |

The HTTP route receives a hash string — could be either format. Format
discrimination is needed.

### 3.3 Discrimination strategies

**Strategy A: Prefix-tagged**

```
GET /blob/sha256-<64hex>  → libp2p BlobStore
GET /blob/blake3-<64hex>  → IrohBlobStore
GET /blob/<64hex>         → ambiguous: try by transport_backend default
```

Pros: explicit, no guessing.
Cons: existing callers send `sha256-<hex>` or raw `<hex>`; raw hex is ambiguous (both are 64 hex chars).

**Strategy B: Backend-routed (recommended for cutover window)**

```
match (transport_backend, hash_format) {
    (Libp2p, sha256_hex) => libp2p_blob_store.get(hash),
    (Libp2p, blake3_hex) => 404 (we don't serve blake3 in libp2p mode),
    (Iroh,   blake3_hex) => iroh_blob_store.get_bytes(hash.parse()?),
    (Iroh,   sha256_hex) => 404 (we don't serve sha256 in iroh mode),
}
```

Pros: simple, the runtime mode is already known.
Cons: a peer requesting a blob in the "wrong" format gets 404. During
cutover this means cross-format federation (one peer on libp2p, another
on iroh) cannot share blobs by HTTP. **Requires either dual-write at
seed time, or a side-by-side address index.**

**Strategy C: Dual-read fallback (recommended for cutover-stable mode)**

```
fn handle_get_blob(hash) {
    // Determine format
    let fmt = classify_hash(hash);
    match (transport_backend, fmt) {
        (Libp2p, _) => libp2p_blob_store.get(hash).or_else(|| {
            // Fallback: was this seeded under iroh? Try iroh store with
            // reverse-mapped blake3 hash from address index.
            if let Some(blake3) = address_index.lookup_blake3(hash) {
                iroh_blob_store.get_bytes(blake3)
            } else { 404 }
        }),
        (Iroh, _) => iroh_blob_store.get_bytes(hash).or_else(|| {
            if let Some(sha256) = address_index.lookup_sha256(hash) {
                libp2p_blob_store.get(sha256)
            } else { 404 }
        }),
    }
}
```

Pros: survives mixed-format federation.
Cons: requires an address index `(sha256, blake3)` populated at seed time
by computing both hashes. The address index is a new Diesel table.

### 3.4 Recommendation

Use **Strategy B** during single-mode operation (default deploy). Add
**Strategy C** as a transition aid during the cutover window only. Drop
the fallback after `peer_blob_inventory.blob_hash` migration completes
and all peers have re-seeded under iroh.

### 3.5 Migration: `blob_address_index`

New Diesel table (proposal — drop after cutover):

```sql
CREATE TABLE blob_address_index (
    sha256_hex   TEXT NOT NULL UNIQUE,
    blake3_hex   TEXT NOT NULL UNIQUE,
    seeded_at    TEXT NOT NULL,
    PRIMARY KEY (sha256_hex)
);
CREATE INDEX idx_blob_addr_blake3 ON blob_address_index(blake3_hex);
```

Populated by:
- The seeder at write time (computes both hashes when uploading).
- A one-time backfill that walks `<storage_dir>/blobs/` and computes
  blake3 for each.

Dropped after cutover via the same migration that drops
`peer_blob_inventory.blob_hash`.

### 3.6 Other blob-store call sites (in addition to the route)

`grep -n "self.blob_store" src/http.rs` — 25+ sites. Most are GET-side
(reading a manifest, projector replays, distribution-detail views). All
need the same treatment: read by hash → format-discriminate → dispatch.
Suggested encapsulation: a `BlobBridge` enum that owns both stores and
exposes a uniform `async fn get(&self, hash_or_addr: &str) -> Result<Vec<u8>>`.

### 3.7 PUT-side / store side

`handle_put_blob` calls `BlobStore::store(data)` which computes SHA256
and persists. In iroh mode, it must call `IrohBlobStore::add_bytes` which
returns BLAKE3. The HTTP response shape (current: returns the hash) needs
to either:

- Return both hashes (sha256 + blake3) during the cutover window, **or**
- Return only the active-mode hash and let callers re-issue under the
  other format if they need it.

Whichever option the architecture spec endorses, the seeder
(§4) needs to know about it.

---

## §4 Genesis seeder rewrite call-site catalog (prereq #3)

### 4.1 Current path

The TypeScript seeder under `genesis/seeder/` issues HTTP PUT to
`/admin/seed/blob` (proxied through doorway) for each blob it pre-uploads.
The doorway then proxies to elohim-storage's `PUT /blob/{hash}` route
which calls `BlobStore::store_and_announce`.

Key seeder files:

- `genesis/seeder/src/doorway-client.ts:466` — POST `/admin/seed/blob`
- `genesis/seeder/src/blob-upload-result.ts` — result type
- `genesis/seeder/src/genesis-pack.ts` — orchestrates pack-and-upload
- `genesis/seeder/src/seed-commitments.ts` — seeds custody-blob commitments

### 4.2 Cutover shape

Three paths, depending on architecture-spec verdict on dual-write:

**Path A: Single-stack write (post-cutover)**

Seeder writes to whichever stack `transport_backend` is set to. No
change to seeder code; the storage-side `PUT /blob/{hash}` route handles
the dispatch.

**Path B: Dual-write during cutover**

Seeder still issues one HTTP PUT. Storage-side route writes to BOTH
`BlobStore` and `IrohBlobStore`, computing both hashes. Returns a
response like:

```json
{
  "sha256_hex": "...",
  "blake3_hex": "...",
  "size": 12345
}
```

The seeder records both hashes in `blob_address_index` (§3.5) on the
storage side via a follow-up admin call.

**Path C: Seeder-side dual-issuance**

Seeder issues two PUTs (one per stack). More HTTP round-trips, but no
change to storage-side route. Useful if storage runs in single-mode
and we want to seed both clusters separately.

### 4.3 Recommendation

**Path B during the cutover window**, gated behind the `dual_write`
config flag on storage-side. Drop Path B after cutover; move to Path A.

### 4.4 Touch points in seeder

```
genesis/seeder/src/doorway-client.ts
  - method `seedBlob(bytes) -> { sha256, blake3? }` — extend response
    handling to capture optional blake3
genesis/seeder/src/genesis-pack.ts
  - record both hashes in pack manifest (so re-seed can verify either)
genesis/seeder/src/blob-upload-result.ts
  - extend type with blake3 field
genesis/seeder/src/seed-commitments.ts
  - custody-blob commitments may need blake3 too — check with REA layer
```

---

## §5 Per-plane backend wiring sketches (prereq #1)

For each request-response plane, the daemon currently sends via libp2p's
`request_response::Behaviour` and receives via the swarm event loop. The
iroh side already exists as `Iroh<Plane>Client` + `<Plane>Backend` trait
objects. Cutover wires the daemon to dispatch on `transport_backend`.

### 5.1 Sync plane (`/elohim/sync/2.0.0`)

**Current libp2p sites** (3):
- `src/p2p/mod.rs:5225` — sync_protocol.send_request (pull from peer)
- `src/p2p/mod.rs:5244` — sync_protocol.send_request (push to peer)
- `src/p2p/mod.rs:5303` — sync_protocol.send_request (heartbeat)

**Dispatch sketch:**

```rust
async fn sync_send(&self, peer: PeerLike, req: SyncRequest) -> Result<SyncResponse> {
    match self.config.transport_backend {
        TransportBackend::Libp2p => {
            let peer_id = peer.as_libp2p_peer_id()?;
            // existing send_request + oneshot await
        }
        TransportBackend::Iroh => {
            let node_id = peer.as_iroh_node_id()
                .or_else(|p| cross_stack_peer_map::iroh_for_libp2p(&p))
                .ok_or(NoIrohMappingError)?;
            self.iroh_sync_client.request(node_id, req).await
        }
    }
}
```

**Open question** (resolved by architecture spec): does `PeerLike`
canonically carry both libp2p and iroh identifiers? Current code uses
`libp2p::PeerId` directly. Cutover needs a richer abstraction.

### 5.2 EPR plane (`/elohim/epr/2.0.0`)

**Current libp2p sites:**
- `src/p2p/mod.rs:2287` — epr_protocol.send_request (Fetch by reach-tag)
- `src/p2p/mod.rs:6309` — epr_protocol.send_request (Federation)
- `src/p2p/mod.rs:6353` — epr_protocol.send_request (?)

**Dispatch:** identical shape to sync plane.

### 5.3 EPR-atom plane (`/elohim/epr-atom/2.0.0`)

**Current libp2p sites:**
- `src/p2p/mod.rs:2537` — `epr_atom.send_request` (FetchBatch)
- `src/p2p/mod.rs:2564` — `epr_atom.send_request` (Fetch)
- `src/p2p/mod.rs:2590` — `epr_atom.send_request` (Announce)
- `src/p2p/mod.rs:2673` — `epr_atom.send_request` (IntegrityNotify)
- `src/p2p/mod.rs:6411` — federation fetch

**Dispatch:** identical shape to sync.

### 5.4 Shard plane (`/elohim/shard/2.0.0`)

**Current libp2p sites:**
- `src/p2p/mod.rs:2311` — shard_protocol.send_request (Get)
- `src/p2p/mod.rs:2335` — shard_protocol.send_request (Have)
- `src/p2p/mod.rs:4997` — shard_protocol.send_request (Push)

### 5.5 View-federation plane (`/elohim/view-federation/2.0.0`)

**Current libp2p sites:** view_federation request-response — `behaviour_mut().view_federation.send_request` (single site at line ~1104 area, plus reach paths).

### 5.6 Identity-handshake plane (`/elohim/identity-handshake/2.0.0`)

**Current libp2p sites:**
- `src/p2p/mod.rs:2966` — identity_handshake.send_request (initial handshake)
- `src/p2p/mod.rs:3003` — identity_handshake.send_request (re-handshake)

**Note:** handshake fires on every new connection. iroh side fires on
every new iroh connection. Dispatch is per-connection-event, not per-
P2PCommand.

### 5.7 Trust plane (`/elohim/trust/2.0.0`)

**Current libp2p sites:** trust_protocol.send_request — co-located with handshake.

### 5.8 Blob plane (`/elohim/blob/1.0.0` libp2p, `iroh_blobs::ALPN` iroh)

**Current libp2p sites:**
- `src/p2p/mod.rs:2641` — `behaviour_mut().blob_protocol.send_request` (P2PCommand::FetchBlob)
- `src/p2p/blob_fetch.rs::race_fetch` — orchestrates parallel fetch across candidates

**iroh equivalent:** `IrohNode::fetch_blob_from(peer_addr, hash)` —
iroh-blobs handles streaming + verification.

**Dispatch wrinkle:** the libp2p-side race-fetch parallelism may not
translate directly to iroh-blobs. iroh-blobs has a single-source fetch
API; race-fetch would need to spawn N futures each calling
`fetch_blob_from(peer_i, hash)` and select the first success.

### 5.9 Receive-path symmetry

For each plane, the receiver side already exists on the iroh trait-object
backends (e.g., `SyncBackend`, `EprBackend`). Cutover hooks the daemon's
existing handler functions (currently called from libp2p's swarm event
loop) into the iroh backend's `handle_request` — same handler functions,
different invocation path.

### 5.10 Common dispatch utility (proposed)

Rather than 7 copies of the dispatch pattern, propose a generic helper:

```rust
async fn send_typed_request<P: Plane>(
    backend: TransportBackend,
    peer: PeerLike,
    request: P::Request,
    libp2p_swarm: &Arc<RwLock<Swarm<...>>>,
    iroh_client: &P::IrohClient,
) -> Result<P::Response> { ... }
```

Where `Plane` is a trait abstracting the request/response types and the
behaviour field name. Avoids per-plane copy-paste.

---

## §6 Recovery e2e + migration deferred items

### 6.1 Recovery e2e (prereq #5)

Current recovery integration tests run on the libp2p side via the
existing `tests/harness/mod.rs`. For iroh:

- Build a sibling `tests/harness_iroh/mod.rs` that uses
  `parity_harness::TwoNodeFixture` for the iroh side.
- Translate the recovery scenarios from the existing libp2p tests
  to drive both stacks.
- Acceptance: every recovery scenario passes on both stacks with
  identical observed projections.

This is implementation work, not in scope here.

### 6.2 `peer_blob_inventory.blob_hash` drop migration (prereq #10)

Already added: `2026-05-08-033248_peer_blob_inventory_blake3_hash`
(adds `blake3_hash` as NULL).

Post-cutover migration (sketch):

```sql
-- migrations/YYYY-MM-DD-HHMMSS_drop_peer_blob_inventory_blob_hash/up.sql
-- Run only after all peers have been writing blake3_hash for ≥1 week
-- and the blob_address_index has been populated for ≥1 week.

ALTER TABLE peer_blob_inventory DROP COLUMN blob_hash;
DROP TABLE blob_address_index;
```

Down migration: re-add the columns NULL. Code paths that reference
`blob_hash` must be removed before this migration is applied.

---

## §7 Rollback drill playbook (prereq #9)

The cutover is reversible by env-var flip. This section is the
operator-facing playbook.

### 7.1 Pre-conditions for rollback

You're rolling back if any of:

- iroh stack peer-set diverges from libp2p baseline (>5% blob count delta sustained ≥1h)
- iroh stack throughput collapses below libp2p baseline at any size class
- iroh-discovery (`discovery_n0()`) becomes unreachable for >15 min
- `bench_blob_perf` regresses below libp2p baseline (run on production
  metrics, not loopback)
- Any iroh ALPN fails handshake at >0.1% rate sustained ≥1h

### 7.2 Rollback procedure

**Step 1 — Flip the env on every elohim-node:**

```bash
# Cluster-wide via household-fabric operator
elohim-fabric set-env ELOHIM_TRANSPORT_BACKEND=libp2p --restart
```

Or per-node:

```bash
ELOHIM_TRANSPORT_BACKEND=libp2p systemctl restart elohim-storage
```

**Step 2 — Verify libp2p path is live:**

```bash
# Each node:
curl -s http://localhost:8090/admin/transport-status
# Expect: {"backend":"libp2p","peers_connected":<N>,"alpns":[]}
```

**Step 3 — Reseed inventory:**

```bash
# On any node:
curl -X POST http://localhost:8090/admin/inventory/republish
```

This forces a `T22InventorySnapshot` publish on libp2p so peers see
the rolled-back node's holdings.

**Step 4 — Watch for peer reconvergence:**

Metrics to watch (via the production dashboard, not the iroh bench):
- `gossipsub_peers_subscribed_to_inventory` — should stabilize at full peer count
- `peer_blob_inventory_rows_count` — should stop dropping
- `blob_fetch_success_rate` — should recover to baseline (~99%+)

Reconvergence target: ≤5 minutes.

**Step 5 — Document the rollback:**

In `~/.claude/projects/-projects-elohim/memory/`:

```
project_iroh_phase11_rollback_<date>.md — describes:
- What triggered the rollback (which symptom)
- How long iroh was active before rollback
- Reconvergence time
- What's needed before next attempt
```

### 7.3 Post-rollback diagnosis

If rollback happened, re-cutover only after:

- Root cause identified (a specific iroh ALPN behavior, a discovery
  failure mode, a peer-map gap, a soak-time issue, etc.)
- A regression test added (likely in `tests/iroh_*` or `tests/bench_*`)
  that would have caught the symptom on loopback
- Architecture spec re-reviewed for whether the failed plane is iroh-cutover material

### 7.4 What does NOT rollback automatically

- **Diesel migrations** — `peer_blob_inventory.blake3_hash` stays
  populated even after rollback. This is fine: NULL is allowed, libp2p
  path doesn't read this column.
- **`blobs_iroh/` directory** — stays on disk. Re-cutover finds the
  blobs already present.
- **`cross_stack_peer_map`** — keeps its rows. Rollback doesn't
  invalidate the peer mapping.
- **iroh secret key (`iroh.key`)** — stays on disk. Re-cutover uses the
  same NodeId.

The only non-reversible artifacts are the migrations themselves
(addition is reversible by code-path tolerance for NULL; the schema
addition stays).

---

## §8 Open questions awaiting architecture spec

These resolve when `genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md`
lands.

1. **Plane scope** — which planes go iroh-only, which stay libp2p-only,
   which go dual-stack? Affects every dispatch sketch in §§2-5.
2. **Browser/edge transport** — does the substrate have any role for
   browser-edge peers, or are those exclusively doorway-projected?
   Affects whether `BlobBridge` (§3.6) needs a third arm.
3. **Discovery seam** — `discovery_n0()` vs Kademlia vs both? Affects
   bootstrap-resolution in §2.8.
4. **Hub-vs-edge split** — household-hub and collective-hub may be
   substrate-iroh while peers around them are doorway-libp2p. Is the
   intra-substrate gossip fully iroh, or does it bridge?
5. **Doorway role** — does doorway speak iroh too (becoming a substrate
   participant) or stay libp2p-only as the federation projection?
6. **Cutover timeline** — gradual (one plane per release) or atomic
   (one cutover release)?

§§2-5 are written so the answers slot in without rewriting catalogs.

---

## §9 What's NOT in this doc

- Implementation. Every code block is a sketch, not real code.
- Tests for the cutover. Tests are written when implementation lands.
- Plane-by-plane perf benches. Bench expansion is a separate thread.
- The architecture spec itself. That thread is owned separately.
- Phase 11 timeline. Owned by the user when the spec lands.

---

## Appendix A — call-site index (one-line summary)

| Site | File:line | Plane | Dispatch class |
|---|---|---|---|
| inventory snapshot publish | p2p/mod.rs:2192 | gossip | §2.1 |
| recovery_invitation publish | p2p/mod.rs:2383 | gossip | §2.2 |
| identity_binding publish | p2p/mod.rs:2415 | gossip | §2.3 |
| recovery_revocation publish | p2p/mod.rs:2442 | gossip | §2.4 |
| epr_atom announce publish | p2p/mod.rs:2507 | gossip | §2.5 |
| gossip_flood publish | p2p/mod.rs:2603 | gossip | §2.6 |
| HTTP PUT /blob/{hash} | http.rs:604 | blob | §3 |
| HTTP GET /blob/{hash} | http.rs:608 | blob | §3 |
| race_fetch (P2PCommand::FetchBlob) | p2p/blob_fetch.rs | blob | §5.8 |
| sync_protocol.send_request | p2p/mod.rs:5225/5244/5303 | sync | §5.1 |
| epr_protocol.send_request | p2p/mod.rs:2287/6309/6353 | epr | §5.2 |
| epr_atom.send_request | p2p/mod.rs:2537/2564/2590/2673/6411 | epr-atom | §5.3 |
| shard_protocol.send_request | p2p/mod.rs:2311/2335/4997 | shard | §5.4 |
| view_federation.send_request | p2p/mod.rs:~1104 | view-fed | §5.5 |
| identity_handshake.send_request | p2p/mod.rs:2966/3003 | identity | §5.6 |
| trust.send_request | (co-located w/ handshake) | trust | §5.7 |
| seeder POST /admin/seed/blob | genesis/seeder/src/doorway-client.ts:466 | blob | §4 |

Total dispatch sites: 25+. Each sketched in §§2-5. None implemented in this branch.
