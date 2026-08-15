---
id: "plan-dataplane-sdk-path"
kind: "plan"
contentType: "plan"
contentFormat: "markdown"
title: "The path to a packageable p2p dataplane SDK — ranked blockers and critical path"
slug: "dataplane-sdk-path"
written: "2026-08-14"
author: "claude (dataplane blockers survey, operator-directed)"
status: "active"
priority: "high"
habit: "dataplane-sdk (PROPOSED — probe: CI substrate-verify migrated to the .dataplane() facade and green; until minted, this doc's honesty anchor is the convergence work in flight under notary-authority)"
cites:
  - genesis/data/timeline/backlog/eprfs-ipfs-analog-dataplane-sdk-surface.md
  - genesis/data/timeline/backlog/arch-dataplane-sdk-proposal.md
  - genesis/data/timeline/backlog/arch-dataplane-refactor-backlog.md
  - genesis/data/timeline/backlog/spin-divergent-undeclared-rows-block-a-convergence.md
  - genesis/data/timeline/backlog/iroh-sync-round-driver-gap.md
  - genesis/data/timeline/backlog/elohim-sdk-native-mode-silent-write-loss.md
tags: [dataplane, sdk, eprfs, convergence, critical-path, architecture]
---

# The path to a packageable p2p dataplane SDK

**The target is already named.** The SDK seam is "compose inward, add a manifest"
(seam map §3.5). The intended surface is the eprfs/IPFS-analog — `add / get /
provide / resolve` by CID over the blob + CRDT planes
(`eprfs-ipfs-analog-dataplane-sdk-surface.md`) — with the `.dataplane()` typed
facade (`arch-dataplane-sdk-proposal.md`) as the TS half.

**Where we are (2026-08-14):** digging into blocker 1 — closing the convergence
algebra. This doc exists so the rest of the path stays visible while we do.

## Blockers, ranked by how structurally they block packaging (not by effort)

### 1. The convergence algebra isn't closed — absorbing non-goal states  ← IN FLIGHT

An SDK's core promise is "put a CID in, it converges wherever reach says it
should." Today convergence is a fleet-operated outcome with named stuck states
that have no exit:

- **SPIN class** (canonicalized 2026-08-14): anchor-divergent + undeclared rows
  have no discharge path — ContestPeer requires `local_declared`
  (`projection_reconcile.rs:981`), ghost-decay requires `Answer::Absent`
  (`head_adoption.rs:790`). 13 rows on 3 peers prove it live.
- **MissLedger is in-memory** (restarts amnesia it); the exhausted-veto premise
  was falsified 2026-08-07 but the durable-state question stands.
- The family: `content-gap-limit-cycle-blocks-convergence`,
  `declare-sweep-hash-only-cannot-converge-missing-action`,
  `deploy-bridgeless-host-never-converges`,
  `genesis-pair-cross-conductor-fetch-blocks-canonical-convergence`.

You can't package a state machine whose reachable states include "stuck forever,
silently." Protocol-level; everything below is packaging.

### 2. Head-plane cost is count-bound and trust-flat — "add" isn't a cheap verb

The 2026-08-08 arithmetic: ~3.5k A-class heads ÷ 200/tick ÷ 300s = the 2.5h
quiesce floor; bytes irrelevant. Every head is a per-id conductor round-trip, an
election participant, a divergence surface. The batching cure the CRDT plane got
(`ListDocumentsSince{corpus_digest}`) never reached the head plane. An SDK
consumer seeding 10k items buys hours of churn. Lever stack designed, mostly
days-scale (`arch-dataplane-refactor-backlog.md` L1–L5): batched head externs
(coordinator hot-swap, no DNA move), head-plane corpus digest, seed-path reach
expression, composite roots (Row 16), trust-priced election gossip.

### 3. The runtime doesn't factor — god-object, not a library

`p2p/mod.rs`: 154-field P2PNode, 15-arm `select!`, 2,207-line
`handle_behaviour_event`, 9 pending-request maps, `swarm_tx` threaded raw into
HTTP handlers. The seams an SDK must consume don't exist as seams:

- **EprValidator** (backlog item 13): today an SDK/bridge cannot validate an EPR
  without importing the full 1,600-line service stack.
- **P2PDispatcher trait** (item 3): kill the swarm-enum coupling so consumers
  get semantic ops.
- **Shared codec crate** (item 2) + **SwarmConfig unification** (item 14):
  elohim-storage and steward/node already carry two silently-diverging
  SwarmBuilder chains — a preview of what an SDK fork-consumer would suffer.

The minimal extraction set is 2 + 3 + 13 — not the whole 16-item backlog.

### 4. Dual-transport half-integration — iroh fills but doesn't flow

The SDK boundary needs a transport-agnostic contract; today the contract is
"libp2p, plus an iroh that half-works." The iroh DocStore fills on every write,
but no sync-round driver exists — the 60s scheduler is libp2p-only, and
`IrohSyncClient` is invoked only from tests (`iroh-sync-round-driver-gap.md`).
Add `iroh-lane-bootstrap-publish-dark` and the relay/DNS bypass. Either iroh
reaches parity behind the same contract, or SDK v1 explicitly pins libp2p and
labels iroh experimental — the current ambiguity can't ship.

### 5. The existing SDK artifacts are honesty traps

- `crates/elohim-sdk` `ClientMode::Native` without `sync_url` used to destroy
  queued writes on `flush()`. **Fixed 2026-08-15:** `flush()` now returns
  `SdkError::InvalidMode` before taking a batch, and a regression test proves the
  queued write remains.
- Its `sync`/`full` features were compile-broken since introduction (fixed
  2026-08-07). **Direct coverage added 2026-08-15:** one shared gate now runs
  formatting, all-feature clippy, and the no-default/default/isolated-feature/
  all-feature test matrix from pre-push and the edge pipeline.
- TS side: `StorageClient` exposes no typed ops over the ~35 dataplane routes;
  every consumer hand-rolls curl+jq (substrate-verify is 594 lines of it).
- The `.dataplane()` proposal is `ci_status: blocked` on three design questions
  never brainstormed: single-base-URL vs multi-peer addressing,
  healing-orchestration locus (client fan-out vs node primitive), and whether
  `schemas/v1/inputs/` becomes the convention.

### 6. Trust and lifecycle substrate the SDK would inherit broken

- The trust gradient is absent: `verify_trust_context` only at handshake,
  `TrustService::handle` a stub, flat 3600s TTL, and the `dev_mode` socket
  deliberately inert — so there's no "simulacra" collapse for dev networks,
  meaning an SDK developer's inner loop pays live-network trust compute or gets
  nothing. A dev-loop story is non-negotiable for an SDK.
- No draft→graduate content path exists (Category B drafts prescribed by the
  p2p-design-gate, unimplemented) — "author locally, publish when ready" has no
  substrate verb.
- Doorway is a single-target proxy (islands, no fan-out) — which is exactly why
  design question 1 (addressing model) can't be dodged.

## Not blockers (for fairness — live and proven)

The SDK grammar itself (manifests, the four cheap moves) · the ts-rs/view-schema
codegen pipeline · the automerge CRDT plane on libp2p · blob store + custody
machinery · `epr-composite` containers.

## Critical path

1. **Close the convergence algebra** — SPIN discharge + durable MissLedger + the
   named stuck-state family. Protocol first. ← current work (2026-08-14)
2. **Head-plane L1–L3** (batched externs, corpus digest, seed-path reach) —
   days-scale, mostly coordinator hot-swap; makes "add" cheap and CI loops fast
   as a side effect.
3. **One brainstorm** answering the three blocked design questions, then the
   `.dataplane()`/eprfs facade with CI (substrate-verify migration) as first
   consumer — the SDK gets proven daily instead of aspirationally.
4. **Factor the minimal seam set** (codec crate, P2PDispatcher, EprValidator) in
   parallel with 3.
5. **Decide the transport story** (iroh parity vs pinned-libp2p v1) as an
   explicit contract line, not an ambient state.

The Native-mode silent write loss and its missing direct CI coverage were closed
2026-08-15; neither remains on the critical path.
