---
id: steward-node-gospel
cites:
  - "elohim-hub-boundaries-design | the orchestration epic ABOVE this crate — hub composition truth (Hub trait, crate-map L171-178, two-swarms); this gospel covers crate mechanics only | sha256:d7ffa707a34d126f | path: genesis/docs/content/elohim-protocol/architecture/2026-05-02-elohim-hub-boundaries-design.md"
  - "p2p-dataplane-sync-engine-design-arc | protocol lineage home — the /elohim/doc-sync vs /elohim/storage-sync two-dialect divergence and its recorded convergence open question; cite, never restate | sha256:d509030b5f00acd0 | path: genesis/docs/content/elohim-protocol/history/2026-06-11-p2p-dataplane-sync-engine-design-arc.md"
  - genesis/data/timeline/backlog/reach-vocabulary-frontend-strand.md
  - .claude/skills/automerge-sync/SKILL.md
  - "steward-device-gospel | device-side Tauri shell gospel — the ephemeral spoke this always-on node serves; device concerns route there | sha256:1d7a9fb8f5da3e01 | status: stale — target content moved on; re-verify | path: steward/device/CLAUDE.md"
  - steward/node/simulation/P2P-COMPUTE-FOOTPRINT.md
  - "elohim-seam-map-concern-routing | the concern-routing atlas — this crate owns the hub cluster ops / hub-internal swarm seam (§3.12); routes any where-does-this-go? question | sha256:7fd48274fae5e8c5 | status: stale — target content moved on; re-verify | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
---

# steward/node — elohim-node (always-on P2P runtime)

This crate **implements** the elohim-hub boundary design; it does not author hub
architecture. Hub-composition decisions — the `Hub` trait, `HouseholdHub` /
`CollectiveHub`, what-lives-in-which-crate — belong to the epic
`elohim-hub-boundaries-design`. Its crate-map table (lines 171-178) and its
"Two libp2p swarms" section are the authority for *why* these modules exist and
where they migrate. **This gospel covers crate mechanics only.** When a question
is "should this be a hub method / which hub owns it," route to the epic
(layered-drift rule: the layer above owns the shape).

## Seam map — you are here

This crate owns the **hub cluster ops** seam (atlas §3.12 — the hub-INTERNAL
swarm: blade-to-blade mDNS discovery, leader election, pod consensus, replica/PVC
placement, plus the enablement "hubbiness dial" and identity-preserving tier
graduation).

Any "where does this go?" concern routes through the concern-routing atlas:
`elohim-seam-map-concern-routing`.

Confusion-to-avoid: the hub-internal swarm (here) ≠ Track-2 hub-to-hub federation
(that's `elohim-storage`, §3.10/3.11) — debugging blade consensus in **this**
crate means you're in the right crate; debugging it in `elohim-storage/src/p2p`
means the wrong one.

## Binary identity

- `[package] name = "elohim-node"`, single `[[bin]] name = "elohim-node", path = "src/main.rs"` (`steward/node/Cargo.toml:2,101-103`). Description: "Always-on infrastructure runtime for the Elohim Protocol."
- Entry is `#[tokio::main] async fn main()` with a `clap`-derived `Cli` (config path, data dir, node-id/cluster-name overrides, `--enable-bitswap`, and an optional `pod` subcommand) (`src/main.rs:36-66`).
- Boot wires, in order: pod (optional background task), shared `elohim_storage::BlobStore`, the unified P2P swarm + sync engine + coordinator, a minimal blob-serving storage HTTP server, the elohim-agent service, and the axum dashboard router (`src/main.rs:107-298`).

## Swarm & behaviour (the live P2P spine)

`ElohimBehaviour` (`#[derive(NetworkBehaviour)]`, `src/p2p/transport.rs:42-73`) is one swarm with one peer identity combining:

- `doc_sync` — `request_response::Behaviour<SyncCodec>` over `/elohim/doc-sync/1.0.0` (node's own protocol).
- `shard_protocol` — `request_response::Behaviour<ShardCodec>`, **re-exported from `elohim_storage::p2p::shard_protocol`** (`transport.rs:31-33`).
- `storage_sync` — `request_response::Behaviour<StorageSyncCodec>` over `/elohim/storage-sync/1.0.0`, **re-exported from `elohim_storage::p2p::sync_protocol`** (`transport.rs:34-37`).
- `kademlia` — `kad::Behaviour<SledRecordStore>`, sled-backed store **re-exported from `elohim_storage::p2p::kad_store`** (`transport.rs:30`, built at `transport.rs:448-450`).
- `mdns`, `relay_client`, `relay_server` (`Toggle`, disabled by default — `transport.rs:457`), `dcutr`, `identify` (`/elohim/id/1.0.0`, `transport.rs:461-462`), `autonat`, and `bitswap` (`Toggle<elohim_bitswap::Bitswap>`, path dep `elohim/elohim-bitswap` — `Cargo.toml:40`; gated by config/`--enable-bitswap`, `transport.rs:484-488`).

Builder: `SwarmBuilder::with_existing_identity(...).with_tokio().with_tcp(...).with_quic().with_relay_client(...).with_behaviour(...)` — each `request_response` behaviour is constructed via `Behaviour::new([(Proto, ProtocolSupport::Full)], cfg)` (`transport.rs:412-507`). The Ed25519 keypair is persisted to `<data_dir>/node_key` (`transport.rs:544`).

**Mechanism granularity — storage_sync / shard are composed-and-forwarded but NOT yet handled.** The swarm event loop (`transport.rs:189-378`) decodes `StorageSync` and `ShardProtocol` request events and forwards them as `SwarmEvent::StorageSyncRequest` / `ShardRequest`, but those variants are `#[allow(dead_code)]` (`transport.rs:108-122`) and the coordinator's match arm for them is an **empty no-op** (`src/sync/coordinator.rs:135-138`). That arm's comment claims they are "handled by dedicated handlers" — **no such handlers exist anywhere in this crate** (grep-verified 2026-06-11). Only `doc_sync` events drive real logic today (`coordinator.rs:113-130`). Treat shard/storage-sync in this crate as **wired-but-inert** — the storage substrate's own swarm is where those protocols are actually serviced.

## Protocol identifiers

`src/p2p/protocols.rs:18-22`: `DOC_SYNC_PROTOCOL = "/elohim/doc-sync/1.0.0"`, `SHARD_PROTOCOL = "/elohim/shard/1.0.0"` (`#[allow(dead_code)]`), `CLUSTER_PROTOCOL = "/elohim/cluster/1.0.0"` (`#[allow(dead_code)]`, no behaviour wired). The doc-sync name was **renamed from `/elohim/sync/1.0.0`** to avoid collision with storage's `/elohim/storage-sync/1.0.0` in the unified swarm (rationale comment, `protocols.rs:15-17`). The two sync dialects (`/elohim/doc-sync/1.0.0` vs `/elohim/storage-sync/1.0.0`) and their convergence question are recorded in the design arc — **cite, never restate**: `2026-06-11-p2p-dataplane-sync-engine-design-arc.md`. Codec is MessagePack with a 4-byte big-endian length prefix and 10 MB cap (`protocols.rs:24-25,104-105,144`).

## libp2p version (corrective anchor)

`Cargo.toml:16` declares `libp2p = "0.54"`; `Cargo.lock` resolves `0.54.1` — the **same major as elohim-storage**. Any "node 0.53 vs storage 0.54" claim (older root `CLAUDE.md` text, the `libp2p-transport` skill) is **STALE**; this crate has been 0.54 since before the steward/ restructure (d35d1d5e3). Features enabled: `tokio, tcp, quic, noise, yamux, mdns, kad, identify, relay, dcutr, request-response, macros, ed25519, autonat, serde` (`Cargo.toml:16-32`). Idioms that hold here: `request_response` behaviours with explicit codecs, `StreamExt::next()` event loop (`transport.rs:193`), `macros` + `ed25519` features required.

## Sync engine (`src/sync/`)

- `merge.rs` — `SyncEngine` over a `rusqlite` `documents.db` in WAL mode; documents are Automerge `AutoCommit` binaries (`automerge = "0.5"`). The schema is **`documents(doc_id TEXT PK, data BLOB, updated_at INTEGER)` only** — **no reach/owner columns, no `sync_events` table** (`merge.rs:32-38`).
- `stream.rs` — `SyncState` holds `peer_positions: HashMap<String, u64>` (keyed by `String`, not `PeerId`) **in memory**; `EventKind` is `Local | New | Backfill | Outlier` (`stream.rs:8-58`). Positions do not persist across restart.
- `coordinator.rs` — `SyncCoordinator` bridges `SwarmEvent`s to the engine; only doc-sync requests/responses are acted on today.

Live runtime behavior of the Automerge sync loop is documented in `.claude/skills/automerge-sync/SKILL.md` — route there for behavior, not to this gospel.

## Live HTTP surface

The real HTTP surface is **`src/dashboard/` (axum router) + `src/elohim_service.rs`**, not `src/api/`.

`elohim_service.rs` wires the `elohim-agent` crate into the node: it defines the `POST /elohim/invoke` and `GET /elohim/health` contract types, builds `ElohimAgentService` with a `MockBackend` (training-wheels phase, no live LLM — `elohim_service.rs:410-420`), initializes the constitutional stack via `constitution::StackContext`, and stands up an `AdmissionController` (budget/queue/priority) so invokes pass through pod admission before execution (`elohim_service.rs:18-39`; routes registered in `src/main.rs:276-289`). A minimal `GET /blob/{hash}` endpoint is also served directly from `main.rs` against the shared `BlobStore` (`src/main.rs:226-246`).

## Stubs and not-built (recorded, not blessed)

- `src/api/{http,grpc}.rs` — **3-line TODO stubs** ("Implement management API" / "Implement device sync API"); `src/api/mod.rs` is an 11-line TODO. `tonic = "0.11"` + `tonic-build` are present in `Cargo.toml` but **unused** — the gRPC device API is **designed-not-built**.
- `src/cluster/{discovery,membership,leader}.rs` — **3-line TODO stubs**. The mDNS discovery / Kademlia membership they describe actually live in the p2p swarm (`ElohimBehaviour.mdns` + `.kademlia`); leader election is **not built**. The epic maps `cluster/*` to `HouseholdHub::cluster()` (crate-map L172).
- `src/storage/reach.rs` — `Reach` enum + `can_serve` / `should_replicate` / `replication_policy`. The enum is referenced as a `trust_level` field default in `coordinator.rs:24,99`, but the three policy functions have **zero production callers** (only unit tests inside `reach.rs`). This is a **recorded dormant definition site** — do NOT canonize any reach vocabulary here; route all reach questions to `reach-vocabulary-frontend-strand.md`.

## Pod subsystem (`src/pod/`)

An autonomous "cluster operator" (monitor → analyze → decide → execute, with consensus) for local-cluster orchestration: storage replication/eviction, workload balancing, cache management, health recovery (`pod/mod.rs` doc-comment). Modules present and built: `admission.rs`, `analyzer.rs`, `capacity.rs`, `compute_rea.rs` (REA compute-commitment accounting), `consensus.rs`, `decider.rs`, `executor.rs` + `actions/`, `monitor.rs`, `protocol.rs`, `models.rs`, `cli.rs`. Exposed both as a background task (`config.pod.enabled`) and as a `pod` CLI subcommand (`main.rs:107-131`). The epic keeps `pod/*` inside `HouseholdHub` (crate-map L175).

## Config (`src/config.rs`)

TOML file (`elohim-node.toml` seed) + env + CLI overrides via `serde` structs (`main.rs:81-101`). Sections: `Config`, `NodeConfig`, `SyncConfig`, `ClusterConfig`, `P2PConfig`, `StorageConfig`, `BitswapConfig`, `ApiConfig`, `PodConfig` (`config.rs:9-156`), plus an `update: UpdateConfig` field whose type lives in `src/update/` (`config.rs:6,17`).

## Build rails

- **Native build — clear RUSTFLAGS.** This is a native (not WASM) workspace: `RUSTFLAGS=""` (the Holochain `getrandom_backend="custom"` flag leaks in from the env and breaks the link). Root-CLAUDE.md gotcha "RUSTFLAGS Override Required."
- Set `CARGO_TARGET_DIR` to this workspace's cargo-target-pool slot before any cargo command (native workspaces must not balloon a legacy `target/`).

## Concern routing (this gospel routes; it does not restate)

- Hub composition / `Hub` trait / which-hub-owns-it → `elohim-hub-boundaries-design.md` (crate-map L171-178, two-swarms §).
- Sync protocol lineage, the doc-sync↔storage-sync two-dialect convergence question → `2026-06-11-p2p-dataplane-sync-engine-design-arc.md` + live behavior in `.claude/skills/automerge-sync/SKILL.md`.
- Reach vocabulary / levels → `reach-vocabulary-frontend-strand.md` (never canonize reach here).
- Multi-node testnet harness (compose 2×2 families + bare-process `spawn-testnet.sh`/persona variant) and the per-human compute-footprint analysis (dated 2026-04; predates the per-human deployments era) → `steward/node/simulation/` (`P2P-COMPUTE-FOOTPRINT.md` + `README.md`).
- Device-side Tauri shell that hosts this node → `steward/device/CLAUDE.md`.

## Philosophy rail — hub-optional floor

This node is **infrastructure for resilience, not a participation requirement**. The hub-optional floor is canon: a single laptop with no hub is a full participant; hubs (and this always-on node) are a convenience that improves availability and replication, never a gate on belonging or function. Design the node so that everything it offers is an *enhancement* of what a lone device already does — never a precondition the device cannot satisfy alone.
