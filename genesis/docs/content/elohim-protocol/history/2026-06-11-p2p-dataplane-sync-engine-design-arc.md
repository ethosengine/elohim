---
title: "History: The P2P dataplane + sync-engine design arc (March 2026)"
id: p2p-dataplane-sync-engine-design-arc
type: history-gotcha
status: noted
tier: history
created: 2026-06-11
topic: [p2p, dataplane, sync, automerge, libp2p, iroh, sharding, design-arc]
# Provenance breadcrumb: the two retiring island docs this record distills.
derived_from:
  - elohim/holochain/docs/P2P-DATAPLANE.md  # retired to git 2026-06-11 (holochain docs island recompose; authored 2026-03-10)
  - elohim/holochain/docs/SYNC-ENGINE.md    # retired to git 2026-06-11 (holochain docs island recompose; authored 2026-03-10)
canonical:
  - genesis/docs/content/elohim-protocol/history/2026-06-11-storage-dual-plane-design-arc.md
  - .claude/skills/automerge-sync/SKILL.md
cites:
  - storage-dual-plane-design-arc | the same-day sibling arc this record composes with — it owns ContentLocation/doorway-bootstrap/reach-inversion verdicts; this record owns the 4-layer drawing + sync mechanism ledger | sha256:2315c84345a2ef3c | path: genesis/docs/content/elohim-protocol/history/2026-06-11-storage-dual-plane-design-arc.md
  - conductor-agent-info-substrate-gossip | how conductor-side discovery actually landed in place of the doc's signal-server bootstrap flow | sha256:7ee98c749aadb58d | path: genesis/docs/content/elohim-protocol/history/2026-06-02-conductor-agent-info-substrate-gossip.md
  - dht-is-a-notary-not-a-byte-store | the canon that corrected the March drawing's one lost direction (who-has-what index on the DHT) | sha256:a1d408ef2478b288 | path: genesis/docs/content/elohim-protocol/history/2026-06-01-dht-is-a-notary-not-a-byte-store.md
  - iroh-libp2p-complementarity | why the libp2p bet aged into permanent dual transport rather than being replaced | sha256:29235aeb35aff128 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-08-iroh-libp2p-complementarity.md
  - tiered-quilt-stewardship-design | the three-truth-layer canon the 4-layer drawing's vocabulary grew into | sha256:9f9c6a1c391712b3 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-11-tiered-quilt-stewardship-design.md
  - elohim/elohim-storage/src/p2p/sync_protocol.rs
  - elohim/elohim-storage/src/p2p/shard_protocol.rs
  - elohim/elohim-storage/src/p2p/behaviour.rs
  - elohim/elohim-storage/src/sync/mod.rs
  - elohim/elohim-storage/src/sync/doc_store.rs
  - elohim/elohim-storage/src/sync/stream.rs
  - elohim/elohim-storage/src/sharding.rs
  - elohim/elohim-storage/src/blob_store.rs
  - elohim/elohim-storage/src/shard_service.rs
  - elohim/elohim-storage/src/p2p_iroh/sync.rs
  - elohim/elohim-storage/src/config.rs
  - elohim/elohim-storage/src/http.rs
  - elohim/elohim-storage/Cargo.toml
  - steward/node/src/p2p/protocols.rs
  - steward/node/src/sync/stream.rs
  - steward/node/src/sync/protocol.rs
  - steward/node/src/sync/merge.rs
  - steward/node/src/sync/coordinator.rs
  - elohim/sdk/storage-client-ts/src/client.ts
  - .claude/skills/automerge-sync/SKILL.md
  - genesis/data/timeline/backlog/arch-dataplane-refactor-backlog.md
  - genesis/data/timeline/backlog/stewarded-device-sync-feature-authoring.md
memory_anchors:
  - project_inventory_exchange_not_byte_replication
---

# History: The P2P dataplane + sync-engine design arc (March 2026)

> **Hot-context pointer (the one sentence to remember):**
> The March master drawing's layer vocabulary, protocol shapes, and technology bets
> nearly all won — but the sync engine **shipped twice in two divergent dialects**
> (position-based device sync in steward/node `/elohim/doc-sync/1.0.0`; heads-based
> storage sync in `/elohim/storage-sync/1.0.0`, reborn on iroh as `/elohim/sync/2.0.0`)
> and Phase 4 (governance conflict escalation) never shipped at all — for sync behavior
> read the live skill (`.claude/skills/automerge-sync/SKILL.md`), not the retired docs.

This record covers what the same-day sibling
`storage-dual-plane-design-arc` (2026-06-11) does **not**: the upstream 4-layer
master drawing as origin of the layer vocabulary, the technology-bet ledger, and the
SYNC-ENGINE mechanism-by-mechanism design→shipped trace. The sibling already records,
and this record does not re-derive: ContentLocation DHT entries never built; doorway
signal-server libp2p bootstrap never built (live: config multiaddrs,
`elohim/elohim-storage/src/config.rs:99-102` `p2p_bootstrap_nodes`); acknowledgment
tiers superseded by REA; delivery-side reach filtering philosophy-inverted;
WriteBuffer/cache-core extraction.

## The master drawing that named the layers

P2P-DATAPLANE.md (2026-03-10) drew the four-layer stack — **experience / sync / data
(elohim-storage + libp2p) / trust (Holochain DHT)** — and the founding split: "The DHT
stores WHO HAS WHAT, not WHAT." That vocabulary is the design-arc *origin* of what is
now the three-truth-layer canon (DHT notarizes, libp2p operates, bytes in the
quilt/pantry — `2026-05-11-tiered-quilt-stewardship-design.md`,
`2026-06-01-dht-is-a-notary-not-a-byte-store.md`). The four-layer drawing's one
direction the canon *corrected*: the March doc still put a who-has-what index
(ContentLocation) and shard-manifest storage **on** the DHT; the notary canon and the
sibling record trace why that direction lost. A live breadcrumb of the old direction
survives in code comments: `elohim/elohim-storage/src/sharding.rs:9-10` still says the
manifest "is designed to be stored in Holochain DHT."

## Technology-bet ledger (verified live, 2026-06-11)

| March bet | Verdict | Evidence |
|---|---|---|
| libp2p, not Hyperswarm | **Won — then aged into dual transport.** The 2026-03 doc predates iroh entirely; the "why libp2p" rationale (multi-transport, NAT traversal) held, but elohim-storage now runs libp2p AND iroh side by side, "dual-stack permanent" | `src/p2p/` (35 modules), `src/p2p_iroh/` (incl. `sync.rs`), `shard_service.rs:8-10`, `2026-05-08-iroh-libp2p-complementarity.md` |
| Kademlia + mDNS discovery | **Won verbatim** (Kademlia now sled-backed) | `src/p2p/behaviour.rs:73-74` (Kademlia<SledRecordStore>), `:115` (mdns) |
| Relay + DCUTR NAT traversal | **Won verbatim**, grown into relay-mode roles (client/server/both) | `behaviour.rs:35-52,116-121`; mirrored in `steward/node/src/p2p/transport.rs:56-64` |
| Automerge, not Yjs | **Won in mechanism; version claim drifted.** Both docs say "Automerge 3.0" (the JS line); both Rust crates ship `automerge = "0.5"` | `elohim/elohim-storage/Cargo.toml:146`, `steward/node/Cargo.toml:13`; gotcha #1 in the automerge-sync skill |
| Custom Matrix-inspired stream positions | **Shipped — in two divergent shapes** (see ledger below) | `steward/node/src/sync/stream.rs`, `elohim/elohim-storage/src/sync/stream.rs` |
| Tauri for native experience | **Won** | `steward/device/src-tauri/` exists and is an owned agent scope |
| Pinecone (Matrix P2P overlay) as complement | **Path not taken.** Referenced as local path `/research/matrix/pinecone/` — that tree was never imported into the repo (no `/research` dir exists); no Rust bindings ever appeared; the "second routing layer" slot was eventually filled by iroh instead | repo-wide grep: pinecone appears only in the retiring doc + one sprint prompt |
| Doorway as stateless web bridge | **Won** — outside this record's scope; see the sibling record and doorway gospel | `storage-dual-plane-design-arc` |

## The protocol-name lineage

SYNC-ENGINE.md named one wire protocol, `/elohim/sync/1.0.0`. Reality split it three ways:

1. **`/elohim/doc-sync/1.0.0`** — steward/node (crate `elohim-node`), the device→node
   sync world; explicitly "renamed from `/elohim/sync/1.0.0`"
   (`steward/node/src/p2p/protocols.rs:15-18`).
2. **`/elohim/storage-sync/1.0.0`** — elohim-storage's libp2p CRDT sync; renamed "to
   avoid collision with elohim-node's `/elohim/doc-sync/1.0.0` when both run in the
   unified swarm" (`src/p2p/sync_protocol.rs:30-32`).
3. **`/elohim/sync/2.0.0`** — the iroh ALPN reclaimed the original name at 2.0.0,
   reusing the libp2p wire types unchanged: "Cutover removes the libp2p transport,
   never the message schema" (`src/p2p_iroh/sync.rs:5-6,56`).

`/elohim/shard/1.0.0` shipped **verbatim** (`src/p2p/shard_protocol.rs:10`).

## SYNC-ENGINE mechanism ledger (design → shipped)

The design shipped as **two engines that never reconverged**: steward/node implements
the doc nearly verbatim (position-based, device→node); elohim-storage evolved past it
(heads-based, Automerge-native, app-namespaced).

| Designed mechanism | Verdict | Evidence |
|---|---|---|
| `SyncMessage` enum {SyncRequest/SyncResponse/DocRequest/DocResponse/Announce} | **Shipped verbatim — in steward/node only** | `steward/node/src/sync/protocol.rs:8-34` |
| `EventKind` {Local, New, Backfill, Outlier} | **Shipped verbatim — steward/node only**; the vocabulary does not exist in elohim-storage | `steward/node/src/sync/stream.rs:47-59`; zero EventKind hits in `elohim/elohim-storage/src/` |
| `AgentStream` {position, recent_events, max_recent} | **Renamed/evolved** → `SyncState` (adds `peer_positions`, `outbox`) | `steward/node/src/sync/stream.rs:8-24` |
| Position-based wire sync ("give me events since N") | **Steward/node only.** Storage's wire protocol is **heads-based** instead: `GetHeads`/`SyncChanges` (have_heads + optional bloom filter)/`GetChanges`/`AnnounceChange` (eager-push change_data)/`ListDocuments`, all namespaced by `h_app_id` — a vocabulary the doc never had | `src/p2p/sync_protocol.rs:46-97` (requests), `:100-165` (responses) |
| Stream positions (per-agent monotonic) | **Evolved** in storage to per-peer-per-document `StreamPosition` tracked by `StreamTracker`, sled-persisted | `elohim/elohim-storage/src/sync/stream.rs:25-46` |
| SQLite `documents` table | **Shipped near-verbatim in steward/node** (rusqlite, WAL); **storage chose sled instead** (`DocStore`, `sync.sled`) | `steward/node/src/sync/merge.rs:33-38`; `elohim/elohim-storage/src/sync/doc_store.rs:14-30` |
| SQLite `events` table (position/doc_id/change_hash/kind) | **Never shipped anywhere.** Steward keeps events in in-memory VecDeques (`SyncState.recent_events`, max 1000); storage keeps positions in sled, no event log | no `CREATE TABLE events` in either crate |
| Browser IndexedDB `DocStore` ("holochain-cache-core" section) | **Never shipped as designed.** Browsers consume the sync engine over HTTP `/sync/v1/{h_app_id}/docs[/{doc_id}[/heads|/changes]]` via storage-client-ts (`listDocuments`/`getHeads`/`getChangesSince`/`applyChanges`); "indexeddb" survives only as a cache-*tier* name in the resolution chain, with no Automerge docs or events in it | `elohim/elohim-storage/src/http.rs:876,2659-2700`; `elohim/sdk/storage-client-ts/src/client.ts:116-184`; `elohim/elohim-cache-core/src/resolution.rs:8,52` |
| Automatic CRDT merge | **Shipped** (Automerge `AutoCommit` in steward's `SyncEngine`; `SyncManager` over `DocStore` in storage, wired in main and served by both transports) | `steward/node/src/sync/merge.rs:16-19`; `elohim/elohim-storage/src/sync/mod.rs`, `src/main.rs:1279-1299`, `src/p2p_iroh/sync_backend.rs` |
| Governance-escalation `ConflictResolution` {Automatic, Governance{conflict_id, options}} | **Never shipped.** Zero hits for `ConflictResolution`/`ConflictOption`/`get_conflicts` in any crate; Phase 4 of the doc's implementation plan is the one phase that produced no code | repo-wide grep 2026-06-11 |
| Periodic peer sync scheduling | **Shipped (partially wired)** — `SyncCoordinator` exists with reach-aware replication policy, but several fields/commands carry `#[allow(dead_code)]` | `steward/node/src/sync/coordinator.rs:1-50` |

## Shard protocol + sharding strategy

- `ShardRequest::{Get, Have, Push}` / `ShardResponse::{Data, Have, PushAck, NotFound,
  Error}` shipped **verbatim as drawn** (`src/p2p/shard_protocol.rs:24-31,77-88`), then
  **grew beyond the design**: `ListContent` (reach-filtered EPR inventory listing) and
  `GetContent` (full content-record replication) variants (`:31-44,89-99`) — the
  metadata-inventory direction recorded in
  `project_inventory_exchange_not_byte_replication`. Handling is now transport-neutral
  (`shard_service.rs`) so libp2p and iroh answer identically.
- Sharding table — designed ≤16MB `none` / 16-100MB `chunked` / >100MB `rs-4-7`:
  - ≤16MB single-shard: **verbatim** (`sharding.rs:22` `SINGLE_SHARD_MAX = 16MB`;
    `blob_store.rs:42` `MAX_INLINE_SIZE = 16MB`, chunks of 1MB at `:39`).
  - RS threshold: **drifted 100MB → 64MB** (`sharding.rs:28` `RS_THRESHOLD = 64MB`).
  - `rs-4-7` (4 data + 3 parity): **verbatim default**
    (`sharding.rs:97-98`, `reed_solomon_erasure` at `:13`), and the documented
    encoding vocabulary **grew** an `rs-8-12` value (`:45` — field doc-comment
    vocabulary; rs-4-7 remains the only wired default).

## How discovery actually landed (supersession pointer)

The doc's bootstrap flow (extend doorway `/signal/{pubkey}` to hand out libp2p
multiaddrs) never shipped — recorded in the sibling
`storage-dual-plane-design-arc`, not re-blessed here. What landed: static
`p2p_bootstrap_nodes` multiaddrs in config (`src/config.rs:99-102`), Kademlia + mDNS
(`behaviour.rs`), and — for the Holochain-conductor side of discovery — conductor
agent-info substrate gossip
(`2026-06-02-conductor-agent-info-substrate-gossip.md`, merged behind a default-false
flag, soak held).

## Where current truth lives

For sync behavior (positions, delta sync, doc lifecycle, conflict mechanics, gotchas
including the 0.5-vs-3.0 version trap), the **live reference is
`.claude/skills/automerge-sync/SKILL.md`** — it already points at the shipped
steward/node files and the storage-client-ts methods. Read the skill, not the retired
SYNC-ENGINE.md.

## OPEN QUESTIONS

- OPEN QUESTION: The governance-escalation `ConflictResolution` (Phase 4) never
  shipped, yet the automerge-sync skill still presents it in present-tense reference
  style (SKILL.md "Governance Override" section, :203-213). No backlog entry tracks
  it. Does it remain qahal-bound design intent, or should the skill mark it
  unimplemented?
- OPEN QUESTION: The two sync dialects (steward/node position-based vs storage
  heads-based) never reconverged, and the doc's single-engine drawing implies they
  were meant to be one. The device→node story is tracked
  (`stewarded-device-sync-feature-authoring.md` backlog); whether the *protocols*
  should converge (e.g., storage-sync absorbing doc-sync post-iroh-cutover) is
  nowhere decided.
- OPEN QUESTION: `sharding.rs:9-10` still documents manifests as "designed to be
  stored in Holochain DHT" — a pre-notary-canon direction. Comment-level drift only;
  no code stores manifests on the DHT.
- OPEN QUESTION: The RS threshold drift (designed >100MB, shipped >64MB at
  `sharding.rs:28`) is undocumented — deliberate retune or transcription drift?
