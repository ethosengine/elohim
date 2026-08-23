---
id: "backlog-iroh-receive-path-inventory-fetch"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "iroh gossip receive path: wire the reactive inventory_fetch so an inventory learned over iroh can trigger the same fetch-on-gossip libp2p does"
slug: "iroh-receive-path-inventory-fetch"
written: "2026-08-23"
author: "fable-5 session 2026-08-23 (roadmap Lane T4; dual-plane grounding)"
status: "refined"
priority: "medium"
area: "dataplane/transport"
domain: "protocol"
jobs: [elohim]
relatedNodeIds:
  - "habit:dataplane-convergence"
cites:
  - genesis/docs/superpowers/plans/2026-08-23-doorway-federated-continuity-roadmap.md
tags: [iroh, libp2p, dual-plane, gossip, inventory, bounded-code-fix, codex-claimable, agent-agnostic]
---

# `inventory_fetch` on the iroh receive path

**Why.** `elohim/elohim-storage/src/p2p_iroh/gossip_receive.rs:19-26` dispatches every
must-receive topic through the transport-neutral `gossip_dispatch::handle_gossip`, but passes
`inventory_fetch = None` — documented there as "a libp2p-command-path operation". So an
inventory snapshot received over iroh populates the inventory table correctly but never
triggers the reactive fetch that the libp2p path triggers. In `dual` mode this is masked by
the libp2p leg; in `iroh` mode it is a silent convergence gap.

## Scope (elohim-storage only; claim AFTER the 2026-08-23 Lane 0 batch is on dev)
1. Find the libp2p `inventory_fetch` closure/handle that `gossip_dispatch` receives (grep
   `inventory_fetch` in `p2p/mod.rs` and `gossip_dispatch.rs`) and define the trait/fn shape it
   needs (likely "queue these content ids / blob hashes for replication").
2. On the iroh side, supply an implementation that enqueues the same gaps through the shared
   replication queue (the queue is transport-neutral — `p2p/mod.rs` "Queued content gaps for
   replication"); do NOT dial over iroh for the bytes here (that is Lane T2, Opus-tier) — just make
   the gap known to the same queue, so in `dual` the existing libp2p fetcher drains it and in
   `iroh` the gap is at least visible (counted, logged) rather than silently dropped.
3. Unit test with the existing iroh parity harness style (`tests/iroh_gossip_parity.rs`): an
   inventory snapshot delivered via the iroh receive path leaves the same queued gaps as via libp2p.

## DoD / verification
- `cd elohim/elohim-storage && just test-iroh` (builds with `--features "p2p p2p-iroh"`) → `EXIT=0`
  echoed; `just gate elohim-storage` → `GATE_EXIT=0`.
- `cargo nextest` is not installed; plain cargo. `CARGO_TARGET_DIR` must be the pool slot.

## Disjointness
`p2p_iroh/gossip_receive.rs`, `gossip_dispatch.rs` (signature only), one test file. Do not touch
`http.rs`, `sharding.rs`, `p2p/blob_fetch.rs`, `p2p/blob_swarm.rs` (Lane 0 / T2 write-sets).
