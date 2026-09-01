---
id: "backlog-task-late-joiner-peer-discovery-refresh"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Task: refresh the signed transport-manifest board after boot so running peers discover late joiners"
slug: "task-late-joiner-peer-discovery-refresh"
written: "2026-09-01"
author: "codex"
status: "complete"
priority: "high"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-late-joiner-peer-discovery-boot-only-board"
  - "backlog-upgrade-propagation-p2p-design-arc"
  - "habit:dataplane-convergence"
tags: [dataplane, discovery, bootstrap, late-joiner, membership, delegable]
claimedBy: "codex"
---

## Why

The doorway's bounded transport-manifest board is read during boot seeding and
afterward only when the iroh peer book is empty. A node with any boot peers
therefore never learns a peer that registers later unless gossip adjacency
already exists or the fleet restarts. The measured W2 incident is recorded in
`late-joiner-peer-discovery-boot-only-board`.

## P2P design-gate decision

- **Classification:** Ephemeral (C), T2 peer-hoster membership projection. No
  new entity is introduced: the existing signed transport announcement is
  projected into the in-memory peer book and can be reconstructed from the
  recurring board/manifest exchange.
- **Identity/address:** the existing iroh `NodeId`, authenticated by the
  announcement's signature. No content address or cross-namespace string join
  is added.
- **Protocol cost:** no DHT entry/link, head-plane item, coordinator function,
  SQLite or Automerge projection, or HTTP route; DNA-hash-neutral.
- **Concern canon:** C0=T2 placement; C2=monotone peer-book upsert;
  C3=periodic refresh; C4/C8/C14=existing empty/rejected/stale observations;
  C5/C9/C12=client-verified NodeId signature; C6a=one bounded GET per interval,
  1 MiB and 64-entry caps; C6b=idempotent upsert; C7=existing announce POST and
  board GET symmetry; C10=unchanged signed-manifest wire contract; C11=5-second
  request timeout. C1 and C13 are n/a because this adds no election or authority
  gradient.

## Scope

1. Change only the post-grace watch in
   `elohim/elohim-storage/src/p2p_iroh/doorway_bootstrap.rs` so it refreshes the
   board at the existing announce cadence even when the peer book is warm.
2. Keep boot-seed timing, response/entry caps, signature verification,
   self-filtering, monotone upsert, and metrics vocabulary unchanged.
3. Add a focused regression test: start with one peer in the book, expose a
   different signed peer on the board after boot, and prove the new peer lands
   without clearing the book or restarting.

## Disjointness contract

- MAY edit only `src/p2p_iroh/doorway_bootstrap.rs`, this task atom, and the
  source backlog atom when closing it with evidence.
- MUST NOT edit `src/p2p_iroh/{peer_book,reconcile_peers}.rs`, `src/main.rs`,
  `src/metrics.rs`, any transport-manifest wire type, Cargo manifests/lockfile,
  deployment manifests, Jenkinsfiles, or mesh scripts. Those surfaces belong
  to concurrent lanes.

## Definition of done

- The focused late-joiner unit test fails under the old empty-book predicate
  and passes with recurring refresh.
- The doorway-bootstrap unit suite passes with cargo exit 0.
- `just gate elohim-storage` is green, or any failure is proven outside this
  task's write set and recorded here.
- The source backlog item is closed only with test evidence; fleet evidence is
  not claimed because the household fixture cannot yet model organic late join.

## Completion evidence — 2026-09-01

- `a_warm_book_learns_a_peer_that_joins_after_boot` passed (1 passed, 0 failed)
  with `p2p p2p-iroh` enabled. The test starts with a non-empty book and proves
  the recurring watch adds a distinct signed board peer without a restart.
- The full `p2p_iroh::doorway_bootstrap::tests` module passed with cargo exit 0.
- `just gate elohim-storage` passed with exit 0, including format, clippy, 3,084
  core unit tests (0 failed), integration tests, and doc tests.
- The implementation touched only `doorway_bootstrap.rs`; the concurrent
  `peer_book.rs` and `reconcile_peers.rs` edits remain outside this task.
- Fleet acceptance remains deliberately unclaimed until the household mesh can
  stage an organic late joiner.
