---
title: "History: The elohim-node architecture founding arc (January 2026)"
id: elohim-node-architecture-founding-arc
type: history-gotcha
status: noted
tier: history
created: 2026-06-11
topic: [steward-node, elohim-node, sync, automerge, libp2p, cluster, nat-traversal, reach, config, design-arc]
# Provenance breadcrumb: the retiring island doc this record distills.
derived_from:
  - steward/node/ARCHITECTURE.md  # retired to git 2026-06-11 (steward/node island recompose; authored 2026-01-07 in 403ddd460 as elohim-node/ARCHITECTURE.md, 511 lines; moved unchanged 2026-03-10 in d35d1d5e3; +13-line Configuration section 2026-06-03 in 7b0eefee9). Recover via `git show 7b0eefee9:steward/node/ARCHITECTURE.md`.
canonical:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-02-elohim-hub-boundaries-design.md
  - genesis/docs/content/elohim-protocol/history/2026-06-11-p2p-dataplane-sync-engine-design-arc.md
  - .claude/skills/automerge-sync/SKILL.md
cites:
  - p2p-dataplane-sync-engine-design-arc | the March master-drawing sibling — owns the technology-bet ledger and the two-dialect doc-sync/storage-sync story this record points into | sha256:d509030b5f00acd0 | path: genesis/docs/content/elohim-protocol/history/2026-06-11-p2p-dataplane-sync-engine-design-arc.md
  - community-compute-founding-vision-arc | records the family-node-as-requirement inversion by the hub-optional floor — the same inversion this drawing sat on the pre-inversion side of | sha256:435254a4149365bb | path: genesis/docs/content/elohim-protocol/history/2026-06-11-community-compute-founding-vision-arc.md
  - elohim-hub-boundaries-design | the living epic that now owns hub composition and maps each of this drawing's modules to its Hub-trait destination | sha256:d7ffa707a34d126f | path: genesis/docs/content/elohim-protocol/architecture/2026-05-02-elohim-hub-boundaries-design.md
  - genesis/data/timeline/backlog/reach-vocabulary-frontend-strand.md
  - .claude/skills/automerge-sync/SKILL.md
  - steward/node/src/sync/stream.rs
  - steward/node/src/sync/merge.rs
  - steward/node/src/sync/coordinator.rs
  - steward/node/src/p2p/protocols.rs
  - steward/node/src/p2p/transport.rs
  - steward/node/src/p2p/nat.rs
  - steward/node/src/cluster/mod.rs
  - steward/node/src/api/mod.rs
  - steward/node/src/config.rs
  - steward/node/src/main.rs
  - steward/node/src/storage/reach.rs
  - steward/node/src/storage/blobs.rs
  - steward/node/src/elohim_service.rs
  - steward/node/src/dashboard/routes.rs
  - steward/node/Cargo.toml
memory_anchors:
  - project_hub_optional_floor
---

# History: The elohim-node architecture founding arc (January 2026)

> **Hot-context pointer (the one sentence to remember):**
> The January 2026 elohim-node drawing shipped its **sync engine nearly verbatim**
> (SyncState/EventKind/Automerge-over-SQLite) while every other layer either froze as
> a 3-line TODO stub (cluster, API, NAT modules) **as its mechanism migrated into a
> different home** (the libp2p swarm, the dashboard, elohim_service.rs) — and the
> running program grew five subsystems the drawing never imagined. For sync behavior
> read `.claude/skills/automerge-sync/SKILL.md`; for hub composition read the
> hub-boundaries epic; the retired drawing is git history.

This record is the steward/node sibling of the
`p2p-dataplane-sync-engine-design-arc` (2026-06-11), which owns the March master
drawing, the technology-bet ledger, and the two-dialect sync story
(`/elohim/doc-sync/1.0.0` vs `/elohim/storage-sync/1.0.0`). This record owns what
that arc does not: the **January scaffold drawing** itself — the component diagram of
elohim-node as a five-layer program (API / sync engine / cluster / P2P / storage) —
and its mechanism-by-mechanism fate inside `steward/node/src/`.

## Lineage (git-dated)

- **2026-01-07** — authored as `elohim-node/ARCHITECTURE.md` (511 lines) in
  `403ddd460` ("Add sparse DHT pattern with P2P blob sync and elohim-node scaffold"),
  two months **before** the March P2P-DATAPLANE master drawing.
- **2026-03-10** — moved unchanged to `steward/node/ARCHITECTURE.md` in `d35d1d5e3`
  (steward/ restructure into device/ and node/ shells).
- **2026-06-03** — the only content edit ever: a 13-line "Configuration" section
  appended by `7b0eefee9`, the native-memory drain (a graduated memory entry, not a
  design pass). See the Configuration verdict below for why that matters.

## Designed vs shipped (verified against the build graph, 2026-06-11)

| Drawn (Jan 2026) | Verdict | Evidence |
|---|---|---|
| `SyncState` {local_position, peer_positions, outbox} + `SyncEvent` {position, doc_id, change_hash, kind, timestamp} | **Shipped near-verbatim**, grew `recent_events`/`max_recent` (in-memory replay window, max 1000); `peer_positions` keyed `String` not `PeerId` as drawn | `steward/node/src/sync/stream.rs:8-24,28-43` |
| `EventKind` {Local, New, Backfill, Outlier} | **Shipped verbatim** | `src/sync/stream.rs:47-59` |
| Automerge CRDT merge over SQLite | **Shipped** — `SyncEngine` (rusqlite, WAL, `AutoCommit`), automerge 0.5 | `src/sync/merge.rs:16-39`; `Cargo.toml:13,35` |
| SQLite schema: `documents` {id, automerge_data, **reach, owner**, updated_at, synced_at} + **`sync_events` table** with indexes | **Diverged.** Live `documents` = doc_id/data/updated_at only; the reach/owner columns and the entire `sync_events` event log **never shipped** — positions/events live in-memory in `SyncState` | `src/sync/merge.rs:33-39` (CREATE TABLE); no `sync_events` anywhere in `src/` |
| `/elohim/sync/1.0.0` protocol | **Renamed** `/elohim/doc-sync/1.0.0` to avoid collision with storage-sync in the unified swarm — the full three-way name lineage is the sibling arc's story, not restated here | `src/p2p/protocols.rs:15-18`; `p2p-dataplane-sync-engine-design-arc` |
| `/elohim/shard/1.0.0` | Constant shipped as drawn (`#[allow(dead_code)]`); a `ShardCodec` behaviour slot is live in the swarm | `src/p2p/protocols.rs:20`; `src/p2p/transport.rs:50` |
| `/elohim/cluster/1.0.0` + `ClusterHandler` | **Name-only.** Constant declared `#[allow(dead_code)]`; **no** cluster behaviour in `ElohimBehaviour`, no handler anywhere | `src/p2p/protocols.rs:22`; behaviour fields at `src/p2p/transport.rs:42-72` |
| Cluster layer: mDNS discovery / membership manager / leader election modules | **Stub-with-comment** — all three modules are 3-line TODO files (`wc -l`: discovery.rs 3, membership.rs 3, leader.rs 3). The discovery *mechanism* migrated homes: mDNS + Kademlia live **inside the p2p swarm** | `src/cluster/{discovery,membership,leader}.rs`; `src/p2p/transport.rs:56-58` (kademlia over `SledRecordStore`, mdns) |
| `NodeRole` {Primary, Replica, Observer} + `LeaderElection` (term/heartbeat) | **Never shipped** — zero hits for `NodeRole`/`LeaderElection`/`ClusterDiscovery` in `src/` | repo grep 2026-06-11 |
| `cluster_key` shared-secret auth | **Idea survived, home migrated**: a `ClusterConfig.cluster_key: Option<String>` config field plus the dashboard pairing flow's join key (`base64(operator_key:cluster_key:cluster_name)`) — not the drawn membership protocol | `src/config.rs:91-98`; `src/dashboard/setup.rs:131,275`; `src/dashboard/discovery.rs:151` |
| API layer: HTTP mgmt + gRPC device API + WebSocket realtime | **All three TODO stubs** (http.rs 3 lines, grpc.rs 3 lines, mod.rs TODO). The live HTTP surface grew in homes the drawing never had: `dashboard/` (axum mgmt UI, pairing, metrics) and `elohim_service.rs` (axum `POST /elohim/invoke` + `GET /elohim/health`, the elohim-agent constitutional stack) | `src/api/{mod,http,grpc}.rs`; `src/dashboard/routes.rs:1-15`; `src/elohim_service.rs:1-14` |
| gRPC protobuf `service ElohimNode` | **Designed-not-built**: `tonic = "0.11"` + `tonic-build` sit in Cargo.toml with **no** `proto/` dir and **zero** `tonic` usage in `src/` — an idle dependency | `Cargo.toml:62,99`; `ls proto` = ENOENT; grep 2026-06-11 |
| Transport preference QUIC / TCP-Noise / WebSocket / WebRTC | **2 of 4**: TCP+noise+yamux and QUIC shipped in the SwarmBuilder; WebSocket and WebRTC never (zero hits) | `src/p2p/transport.rs:412-420` |
| NAT traversal module (STUN / relay / DCUtR strategy ladder) | **Module is a 3-line TODO; the strategy shipped anyway** — as libp2p behaviours in the swarm: relay client + toggleable relay server + dcutr + autonat | `src/p2p/nat.rs` (3 lines); `src/p2p/transport.rs:60-68,421` |
| Blob store delegates bytes to elohim-storage | **Held, and composition grew richer than drawn**: `blobs.rs` (85 lines) plus the node directly importing elohim-storage's `SledRecordStore` (Kademlia store) and `StorageSyncCodec` (third request-response behaviour) — crate-level reuse, not just delegation | `src/storage/blobs.rs`; `src/p2p/transport.rs:30,52` |
| Reach enforcer (ACL) in the storage layer | **Dormant definition site.** `reach.rs` defines a 6-value `Reach` enum with `can_serve`/`should_replicate` policy fns — **zero consumers outside the file**; the coordinator carries a `trust_level: Reach` field marked `#[allow(dead_code)]`. Do NOT canonize any reach ladder from here — see vocabulary note below | `src/storage/reach.rs:1-30`; `src/sync/coordinator.rs:19-25`; grep 2026-06-11 |
| `SyncCoordinator` (positions per peer, routing, conflict escalation) | **Shipped, partially wired** — exists with reach-aware replication intent, several fields `#[allow(dead_code)]`; conflict *escalation* never shipped (the sibling arc records Phase-4 governance escalation as the no-code phase) | `src/sync/coordinator.rs:1-30` |
| Config: "declarative YAML, not CLI args" (§added 2026-06-03) | **Principle held, format claim never true.** The config IS file-first, schema-shaped, CLI-overridable — but it is **TOML** (`toml::from_str`, default `elohim-node.toml`); `serde_yaml` is used only for pod rules files. The section was appended by the memory drain four months after the code chose TOML — a graduated memory canonizing a format the program never had | `src/main.rs:41-42,83`; `src/config.rs:8-22`; `src/pod/decider.rs:36`; commit `7b0eefee9` |

## The program the drawing never imagined

The retired doc describes a *smaller* program than exists. Live subsystems with no
square in the January diagram:

- **`pod/`** — autonomous operations: admission, capacity, consensus, decider/executor
  rule engine, compute-REA accounting (13 modules).
- **`dashboard/`** — axum management UI: setup, pairing, peer discovery, metrics.
- **`update/`** — OTA: manifest, download, apply.
- **`network/`** — operator layer: registration, operator, sync-state.
- **`elohim_service.rs`** — the elohim-agent constitutional stack behind
  `POST /elohim/invoke`.
- **bitswap** — optional block-exchange behaviour, config-gated
  (`src/p2p/transport.rs:70-72`, `src/config.rs` `BitswapConfig`).

The drift direction is the inverse of the usual museum specimen: most retiring docs
overclaim; this one **under-describes** — its five-layer skeleton is recognizably the
crate's `src/` layout, but half the skeleton's flesh grew on different bones.

## Philosophy inversion: the ephemerality spectrum

The doc's founding frame — "Devices Are Ephemeral, Nodes Are Stable", the
phone→laptop→node→cluster→network spectrum with "Design for the left. Guarantee the
right.", and "Device joins → sync to node" — makes the always-on node the **anchor of
participation**. The hub-optional floor canon inverted this: a laptop is a full
participant; hubs are convenience, never a gate (memory anchor
`project_hub_optional_floor`). This is the same inversion the
`community-compute-founding-vision-arc` records for family-node-as-requirement — cite
that arc for the inversion's full story; this record only marks that the elohim-node
drawing was written on the pre-inversion side of it. The *useful* residue of the
spectrum survives as engineering posture (a node that stays up syncs more), not as a
participation requirement.

## Reach vocabulary: three ladders in one directory (do not canonize)

The retired doc used an informal 4-tier ladder (private/family/community/commons);
`steward/node/README.md` (§Reach-Based Replication table) documents 5 values
(private/invited/local/neighborhood/commons); `src/storage/reach.rs:7-26` defines
**6** (adds `Municipal`) —
and none of the three matches the protocol schema enum. The reconciliation is owned
by `genesis/data/timeline/backlog/reach-vocabulary-frontend-strand.md`; this record
deliberately blesses none of them.

## Where current truth lives

- **Sync behavior** (positions, delta sync, doc lifecycle, conflict mechanics):
  `.claude/skills/automerge-sync/SKILL.md`.
- **Hub composition + two-swarms layout**: the hub-boundaries epic
  (`genesis/docs/content/elohim-protocol/architecture/2026-05-02-elohim-hub-boundaries-design.md`,
  crate-map table).
- **Crate mechanics + concern routing**: the steward/node gospel
  (`steward/node/CLAUDE.md`, created in the same recompose that retired this doc).
- **The two-dialect sync lineage + protocol renames**: the sibling
  `p2p-dataplane-sync-engine-design-arc`.

## OPEN QUESTIONS

- OPEN QUESTION: `/elohim/cluster/1.0.0` is a `#[allow(dead_code)]` constant with no
  behaviour and no handler (`src/p2p/protocols.rs:22`). Is family-cluster
  membership/leader-election still design intent anywhere, or should the constant and
  the three stub modules go on the next `src/cluster` touch? No backlog entry tracks
  it either way.
- OPEN QUESTION: `tonic 0.11` + `tonic-build` remain in `Cargo.toml` with no `proto/`
  dir and zero source usage. Idle-dependency removal candidate, or a held seat for
  the device gRPC API the drawing wanted?
- OPEN QUESTION: the designed `sync_events` SQLite event log never shipped anywhere;
  steward/node keeps sync events in bounded in-memory VecDeques (`max_recent: 1000`,
  `src/sync/stream.rs`). Durability of sync positions across node restarts is
  therefore unhandled in this crate — deliberate (re-sync is cheap) or a gap?
- OPEN QUESTION: `genesis/docs/content/elohim-protocol/resilience/README.md:624`
  claims steward `reach.rs` is "LIVE"; the build graph says it is a dormant
  definition site with zero policy-fn consumers. Report-only overclaim finding —
  owner is the resilience epic, not this record.
