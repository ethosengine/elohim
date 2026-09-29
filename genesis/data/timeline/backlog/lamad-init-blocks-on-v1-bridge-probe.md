---
id: "backlog-lamad-init-blocks-on-v1-bridge-probe"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "adam's lamad cell is wedged in init(): the v1-bridge probe (cross-cell call to lamad-v1) never returns, so every zome call gives up after 30 s and the apex can take no signed update"
slug: "lamad-init-blocks-on-v1-bridge-probe"
written: "2026-09-29"
author: "claude-opus-5-5 (FCT v2 visibility shift, genesis #1593)"
status: "open"
priority: "high"
area: "lamad-dna/content_store + shem runtime"
domain: "protocol"
jobs: [elohim-holochain, elohim-genesis]
relatedNodeIds:
  - "habit:dataplane-convergence"
cites:
  - genesis/data/timeline/backlog/lamad-teacher-authoring-backlog.md
tags: [conductor, init, healing, lamad-v1, adam, apex]
---

# adam's lamad init is wedged on the v1-bridge probe

## What is true (genesis #1593, 2026-09-29 ~02:00 UTC)

Every zome call to adam's lamad cell (elohim-adam-alpha-conductor, the apex
`elohim.host`) fails with `Another zome function has triggered the init() callback,
which has been blocking this zome call for longer than 30 seconds. Giving up.`:

- steward-grade on adam: 220 of 257 ids failed with it (the rest `current`, which never
  touch the conductor);
- the adam seed's `PATCH manifesto` and `PATCH foundations-christian-technology` 503
  with it — so the apex still serves the 2026-09-04 v1 course.

matthew's cell answered the same calls (one transient init failure in #1592).
adam's doorway `/health` uptime was 38,930 s (up since ~15:30 UTC 2026-09-28),
`p2p.caughtUp:false`, `divergentAnchor:60`.

`content_store::init` (`elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs`
~1148) first runs `healing_impl::init_healing()` → `hc_rna` `check_v1_on_startup`
(`elohim/holochain/rna/rust/src/healing_orchestrator.rs` ~93) → a synchronous
`call_v1("coordinator", "is_data_present")` into the `lamad-v1` role. The error is
logged and swallowed, but the call itself can block — a cross-cell call from inside
`init()` holds the cell's init lock for as long as the other cell takes to answer.

## Direction

- Operator (shem): restart adam's conductor / disable the stuck `lamad-v1` cell, then
  re-read `elohim.host/db/content/foundations-christian-technology`. The next genesis
  run's grade + seed will then land v2 on the apex.
- Coordinator-only cure (hot-swap, no DNA hash move): take the v1 probe out of
  `init()` — heal lazily on first query (the orchestrator already heals on demand) or
  bound it; `init()` must never block on another cell. Sweettest: an init with a
  lamad-v1 cell that never answers still returns Pass within seconds.

## Current decision

Coordinator cure landed (2026-09-29): `content_store::init` no longer probes v1 — it
only registers the entry-type providers (in-memory, no calls); `healing_impl::init_healing`
is gone. v1 healing stays lazy inside the reads that miss (`healing_integration`).
Coordinator-only: `content_store_integrity.wasm` byte-identical, packed lamad DNA hash
unchanged (`uhC0kLJHygE_XFy1DFnLyMQMaWmpzR1hMX7gAFBmfWrnRUyI-iYlt` before and after).
Proof: sweettest `lamad_init_nonblocking` — beside a `lamad-v1` cell whose
`coordinator::is_data_present` sleeps 45 s, the pre-cure DNA took 45.8 s on the first
call and a concurrent call got `CellError(InitTimeout)` at 30.0 s (the fleet's error);
the cured DNA answers both in under 1 s. Still open until the fleet runs the new
coordinator: the DNA pipeline builds it and `ALLOW_COORDINATOR_UPDATE` hot-swaps it
(`sync_coordinators`, no re-key); adam's already-wedged cell also needs the operator
conductor restart above, since a hot-swap does not interrupt an init already in flight.

Follow-ups, not in this cure: (1) a `lamad-v1` cell that hangs still stalls each
`get_content_by_id` *miss* (lazy `heal_content_from_v1` bridge call) until it answers —
bounded to that read, not the init lock; (2) the hc_rna docs and generator teach the
wedge — the `rna/rust/src/healing_orchestrator.rs` module doc, `rna/README.md`,
`rna/rust/ARCHITECTURE.md`, `rna/rust/GENERATOR_QUICKSTART.md` and
`rna/templates/self-healing.rs.template` all call `check_v1_on_startup` from `init()`.
Editing hc_rna source is deferred on purpose: `content_store_integrity` depends on
hc_rna, so even a doc edit there must be checked against the integrity wasm bytes.
