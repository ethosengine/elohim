---
id: "backlog-iroh-plane-codex-handoff"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "iroh plane handoff: where the dual-plane transport stands after the discovery cut, the contract to build against, and the claimable cuts in priority order"
slug: "iroh-plane-codex-handoff"
written: "2026-08-23"
author: "fable-5 session 2026-08-23 (handoff for Codex / any agent picking up Lane T)"
status: "refined"
priority: "high"
area: "dataplane/transport"
domain: "protocol"
jobs: [elohim]
relatedNodeIds:
  - "habit:dataplane-convergence"
  - "habit:blob-durability"
cites:
  - genesis/docs/superpowers/plans/2026-08-23-doorway-federated-continuity-roadmap.md
  - genesis/data/timeline/backlog/2026-08-23-mesh-transport-backend-knob.md
  - genesis/data/timeline/backlog/2026-08-23-iroh-receive-path-inventory-fetch.md
tags: [iroh, libp2p, dual-plane, transport, handoff, codex-claimable, agent-agnostic]
---

# iroh plane — handoff

This row is the one page to read before touching `ELOHIM_TRANSPORT_BACKEND=dual|iroh` work. It
says what is TRUE on the branch as of the commits below, what contract every further cut builds
against, and which cuts are open — each with its write-set so two agents never collide.

## 1. State (verified live on the 3-peer mesh, 2026-08-23 18:02 binary)

Until `972748a6d` the iroh leg of dual mode was an **idle listener**: every responder mounted, no
peer discovery (sovereign defaults register no discovery service; `gossip_receive.rs` subscribed
with an empty bootstrap), no sync initiator, no production caller for any iroh request client. On
alpha (dual since Wave-2 E3) and on the mesh, every `elohim_iroh_*` counter read zero. **"dual
boots" / "iroh node started" prove nothing. The claim is the counters moving.**

Landed, in order:

| sha | what |
|---|---|
| `ee2cf5b3e` | contract: `IrohPeerBook`, signed `TransportManifestAnnouncement`, `elohim_iroh_*` metrics |
| `cbc8edda5` | iroh sync-round driver (`p2p_iroh/sync_driver.rs`) + `tests/iroh_sync_driver.rs` |
| `9a3357098` `7a6cacc38` | hc-mesh.sh: `MESH_TRANSPORT_BACKEND` wins on restart; per-peer observed transport stamp |
| `9160e2a96` | a2o proof `features/dataplane/transport-dual-plane.feature` (counters, not the listener) |
| `972748a6d` | discovery: manifest announcer + verify/dedup receive arm + per-topic `join_peers` joiner; driver wired in `main.rs`; iroh-feature clippy drift cleared |
| `d980a80a8` | habit delta 2026-08-23d; roadmap Lane T corrected |

Evidence on the mesh (dual): `elohim_iroh_peers_known=2` on every peer within 33 s of restart ·
`gossip_neighbor_events_total{up}=18` (9 topics × 2 peers) · manifests relayed over iroh-gossip
itself · **39 Automerge changes applied over iroh** in the first populated round (matthew 18 /
jessica 8 / james 13), `sync_requests_total{sync_changes,ok}=51`, failures 0 ·
`transport-dual-plane.feature` 7/7 · regression guard content-sync 4/4, heal-on-read 2/2,
doorway-failover 10/10.

**What is still libp2p-only (do not claim otherwise in any doc):**
- Blob heal-on-read (`race_fetch_with_swarm`) and custody push (`transport_resolve.rs:47-67`
  extracts only the libp2p id; `IrohNode::fetch_blob_from` has zero production callers).
- The reactive inventory fetch on the iroh receive path (`inventory_fetch: None`).
- **Pure `iroh` mode has zero peers**: the manifest rides libp2p gossipsub first, so a node with
  no libp2p leg never receives one. `dual` is proven; `iroh` alone is not bootable yet.

## 2. The contract (build against this, do not re-derive it)

- **`crate::p2p_iroh::IrohPeerBook`** (`src/p2p_iroh/peer_book.rs`) — the runtime set of dialable
  iroh peers. `snapshot(Some(&self_node_id))` (self-excluding), `get`, `upsert` (monotone on
  `announced_at_ms`), `remove`, `subscribe()` → `watch::Receiver<u64>` bumped on every change.
  Invariant: entries come ONLY from announcements whose signature verified against the NodeId.
- **`crate::p2p::transport_manifest_gossip::TransportManifestAnnouncement`** — topic
  `elohim/transport/manifest`; fields `iroh_node_id` (64-hex), `iroh_direct_addrs`,
  `iroh_relay_url`, `agent_cid`, `libp2p_peer_id`, `planes`, `announced_at_ms`, `signature`.
  `sign(secret, …)` / `verify()` / `to_bytes()` / `from_bytes()`. `agent_cid` and
  `libp2p_peer_id` are ROUTING HINTS, never attribution (see storage CLAUDE.md "attribution cut").
- **`crate::p2p::gossip_dispatch::TransportManifestSink`** — the transport-neutral hook;
  `IrohPeerBook` implements it under `p2p-iroh`. `GossipDispatchCtx` carries
  `transport_manifest_sink` + `self_iroh_node_id`.
- **Metrics** (`src/metrics.rs`, all registered): `elohim_iroh_peers_known` (gauge),
  `elohim_iroh_manifest_announcements_total{result=accepted|stale|bad_signature|decode_failed|self}`,
  `elohim_iroh_gossip_neighbor_events_total{direction=up|down}`,
  `elohim_iroh_gossip_received_total{topic}`, `elohim_iroh_sync_rounds_total`,
  `elohim_iroh_sync_requests_total{kind=list_documents|sync_changes, result=ok|request_failed|error_response}`
  (`dial_failed` reserved for a typed client error), `elohim_iroh_sync_changes_applied_total`,
  `elohim_iroh_blob_fetches_total{result=ok|dial_failed|not_found|verify_failed|error}` (declared,
  **no producer yet — T2 owns it**).
- **Wiring site**: `src/main.rs` — the `#[cfg(all(feature = "p2p", feature = "p2p-iroh"))]` block
  after iroh node creation builds the book, hands it to `P2PNode::set_transport_manifest_sink`,
  spawns `spawn_iroh_gossip_receive(IrohReceiveDeps{…})`, the announcer, and the sync driver.
- **Mesh knobs**: `MESH_TRANSPORT_BACKEND=dual ./hc-mesh.sh storage-restart` (app/elohim-app/scripts)
  re-execs the three storage peers from the recorded exe
  (`/projects/.cargo-target-pool/family/doorway/elohim__elohim-storage/dev/debug/elohim-storage`);
  rebuild into that slot with `CARGO_TARGET_DIR=<slot> cargo build --features "p2p p2p-iroh"`.
  `ELOHIM_TRANSPORT_MANIFEST_INTERVAL_SECS` overrides the 30 s announce cadence.

## 3. Claimable cuts, in priority order

Each names its write-set. Claim by editing `status:` on the named row (or this table) to `wip`
with your agent name; do not start a cut whose write-set overlaps one already `wip`.

| # | cut | tier | write-set | proof |
|---|---|---|---|---|
| T4 | **`inventory_fetch` on the iroh receive path** — row `iroh-receive-path-inventory-fetch` (unblocked now: `gossip_receive.rs` edit landed). Supply a real `InventoryFetch` impl for `IrohReceiveDeps` that enqueues the same gaps into the shared replication queue; no iroh dial for bytes here | Codex | `src/p2p_iroh/gossip_receive.rs`, a new `src/p2p_iroh/inventory_fetch.rs`, one test | inventory snapshot received over iroh leaves the same queued gaps as over libp2p (parity test) |
| T2 | **iroh blob fetch in heal-on-read** — `race_fetch_with_swarm` candidates carry iroh NodeIds (book lookup by `libp2p_peer_id`/`agent_cid`); race both planes in `dual`; produce `elohim_iroh_blob_fetches_total` | Opus | `src/p2p/blob_swarm.rs`, `src/p2p/blob_fetch.rs`, `src/p2p_iroh/node.rs` (fetch wrapper) | heal-on-read 2/2 still green AND `elohim_iroh_blob_fetches_total{ok}` ≥ 1 on the mesh after a SIGSTOP of the libp2p-preferred holder |
| T3 | **custody push over iroh** — `transport_resolve` returns the iroh id when the book has it; `push_shard` via `IrohShardClient` | Opus | `src/services/transport_resolve.rs`, `src/p2p/blob_swarm.rs` (push leg) | custody-announce rows appear with the push carried on iroh (log + a counter you add) |
| T0′ | **pure-iroh bootstrap** — a node with no libp2p leg must still obtain a manifest set: doorway `GET /p2p/manifests` (signed announcements it has seen) and/or the pkarr station (Lane A5) | Opus | doorway `routes/`, `src/p2p_iroh/announcer.rs` (seed from resolver) | `ELOHIM_TRANSPORT_BACKEND=iroh` mesh reaches `peers_known=2` |
| T5 | **per-mode CI** — the mesh stage runs `libp2p` and `dual`; `@transport:` tag reported per mode; `cargo test --test sync_iroh_convergence` beside the libp2p check in `dataplane-convergence` | Sonnet | `genesis/a2o/**` (coordinate with the in-flight `transport-comparison-matrix` work — uncommitted on the branch as of this row), CI scripts | both modes stamped in the sprint report |
| T7 | **book eviction policy** — drop a peer after N consecutive `request_failed`, re-admit on next verified manifest; `remove()` exists, nothing calls it | Codex | `src/p2p_iroh/sync_driver.rs`, `src/p2p_iroh/peer_book.rs` (a failure counter) | unit test: N failures → removed; fresh manifest → back |
| T8 | **blind-reader pass** on `transport-dual-plane.feature` (owed per `genesis/a2o/.epr-meta` rule `a2o-story-blind-reader-review`; the feature was committed without it) | any | the feature file only | verdict READY or named deferrals |

Not claimable yet: iroh 0.92 → 1.0 lift (campaign `2026-08-04-holochain-iroh-convergence-upgrade-campaign`);
`KeyEnvelope` / blind custody (swarm-curve spec §9, design-gated).

## 4. Definition of done for any cut above

```
cd elohim/elohim-storage
export CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/doorway/elohim__elohim-storage/dev
cargo test --features "p2p p2p-iroh" --lib -- <your module>;              echo EXIT=$?
cargo test --features "p2p p2p-iroh" --test 'iroh_*' -- --test-threads=1;  echo EXIT=$?
cargo clippy --features "p2p p2p-iroh" --all-targets -- -D warnings 2>&1 | grep -E "^error" ; echo EXIT=${PIPESTATUS[0]}
cargo fmt
cargo build --features "p2p p2p-iroh";                                     echo EXIT=$?
cd /projects/elohim/app/elohim-app/scripts && MESH_TRANSPORT_BACKEND=dual ./hc-mesh.sh storage-restart
cd /projects/elohim && just test mesh features/dataplane/transport-dual-plane.feature;  echo EXIT=$?
just test mesh features/resilience/app-blob-heal-on-read.feature;                      echo EXIT=$?
just test mesh features/dataplane/content-sync.feature;                                echo EXIT=$?
```
- Echo `EXIT=` on its own line; never judge a cargo run from piped output. `cargo nextest` is not
  installed. The gate's clippy does NOT run the iroh feature — run the `--features "p2p p2p-iroh"
  --all-targets` form yourself or the drift comes back.
- content-sync can read 3/4 when the 30 s step lands just before the 60 s sync tick — rerun once
  before calling it a regression.
- Leave the mesh in `dual` when you finish.

## 5. Commit discipline (shared worktree — other agents are live in it)

- `git commit -m "<msg>" -- <path> <path>` — path-limited, every time. Never `git add -A`, never
  `--amend`, never push (the operator pushes). Stage an untracked file with `git add <file>` first.
- Do not touch files another cut lists in its write-set while it is `wip`.
- Append a delta line to `genesis/manifests/habits.yaml` (`dataplane-convergence` or
  `blob-durability`) with the evidence — commit shas, counter values, lane results. No status flip
  without fleet evidence (`[build:edge] [edge:validate-only]`, operator-run).
