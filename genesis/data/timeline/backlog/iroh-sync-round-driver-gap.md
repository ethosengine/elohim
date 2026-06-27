---
id: "backlog-iroh-sync-round-driver-gap"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "iroh content sync fills the DocStore but won't FLOW — iroh has no periodic sync-round driver (the 60s scheduler is libp2p-only)"
slug: "iroh-sync-round-driver-gap"
written: "2026-06-27"
author: "automerge content-sync plane sprint — iroh-mode leg finding (commit e4fb14727)"
status: "backlog"
priority: "medium"
jobs: [elohim]
---

## The gap

The content-sync plane lighting sprint wired the content-projection producer on BOTH
transports (libp2p spine + iroh-mode, commit `e4fb14727`). On iroh, the producer correctly
*fills* the Automerge DocStore on every content write (the responder
`SyncManagerBackend` / `src/p2p_iroh/sync_backend.rs` is namespace-parametric and serves the
`"elohim"` docs fine).

**But iroh content will not flow peer-to-peer**, because the iroh stack has **no periodic
sync-round DRIVER**:
- The 60s `initiate_sync_round` scheduler that lists `h_app_id="elohim"` is **libp2p-only**
  (`src/p2p/mod.rs:2401` schedules it; the round body is `src/p2p/mod.rs:6995`).
- `IrohSyncClient` (`src/p2p_iroh/sync.rs`) is the client helper but is invoked **only from
  tests/benches** (`tests/iroh_sync_real_backend.rs`, `tests/iroh_sync_parity.rs`,
  `tests/bench_sync_perf.rs`) — nothing drives it in the running daemon.

## What's needed (bounded)

An iroh-side periodic round driver, analogous to the libp2p `initiate_sync_round`, that lists
`h_app_id="elohim"` and drives `IrohSyncClient` against connected peers on a timer. It must
list the SAME namespace the producer writes (`"elohim"` — the load-bearing coupling; see
`PROJECTION_NAMESPACE` in `src/sync/projector.rs`).

## Notes

- Only matters when iroh is the selected transport (`transport_backend = Iroh`,
  `--features p2p-iroh`); default is libp2p, where the spine's convergence is proven.
- Verify on household-nodes / shem (shem online) once the driver lands: a doc authored under
  iroh on node A converges to node B.
- Deliberately NOT built in the sprint (out of the iroh-leg scope — wiring the producer, not a
  scheduler).

Domain D5 (data plane). Plan:
`genesis/docs/superpowers/plans/2026-06-27-automerge-content-sync-plane-lighting-plan.md`.
