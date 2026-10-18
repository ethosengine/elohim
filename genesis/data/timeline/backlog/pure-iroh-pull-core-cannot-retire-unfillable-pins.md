---
id: "backlog-pure-iroh-pull-core-cannot-retire-unfillable-pins"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The pure-iroh pull core cannot prove every peer was asked for a pin, so an unfillable pin there can never be retired and holds caughtUp false"
slug: "pure-iroh-pull-core-cannot-retire-unfillable-pins"
written: "2026-10-14"
author: "claude (performance sprint, operator-directed)"
status: "open"
priority: "medium"
severity: medium
nodes: [household, alpha]
relatedNodeIds:
  - "habit:idle-is-free"
tags: [elohim-storage, p2p-iroh, acquisition, pins, caughtUp, quiesce, performance, perf-convergence, plane-dataplane, unit-peer, phase-steady, lane-background]
cites:
  - elohim/elohim-storage/src/p2p_iroh/pull_core.rs
  - elohim/elohim-storage/src/p2p/acquisition.rs
  - genesis/data/timeline/backlog/ci-edge-inventory-convergence-caughtup-sawtooth-flap.md
---

**What (code read 2026-10-13, by the agent that reworked the retired-pin backoff and by Codex's review of its first
draft; nothing measured).** `IrohPullCore::drain_pins` picks the peer to ask from one rotation counter that it shares
with `drain_gaps` and advances once per drain. With two peers and 50 failing pins drained in two batches of 25, the
counter moves by two between one pin's retries, so every retry of that pin can reach the same peer. A spent retry
budget on this path therefore does not show that every peer in the book was asked.

**Consequence.** Retiring a pin on that evidence could strand content a second peer holds. The backoff change on
branch `fix/retired-pin-readmit-backoff` (`0099f38fe`, not on dev) makes the pure-iroh core run the shared tick with
`RetirePolicy::CoverageUnproven`: an exhausted pin stays active. On dev today the pure-iroh core does not retire
either. So on a pure-iroh node one unfillable pin holds `pull.caughtUp` false indefinitely, where the libp2p path
retires it and reports caught up.

**Not known.** Whether any deployed node runs pure-iroh pulls today; how often a pure-iroh node has an unfillable
pin.

**Fix shape, not built.** Record which providers were asked for each want and spend the budget only on distinct
ones, or rotate per want. Then switch the core to `EveryPeerProbed`.

**The libp2p drain has the same gap (added 2026-10-14).** `P2PNode::drain_acquisition_queue` picks each want's
peer with `rotated_peer_index(rotation, position, peers)`, and the rotation also advances once per drain, not per
want. With two peers and two stable batches of 25, every retry of a want goes to the peer it asked before. So on
libp2p a spent budget of `max(3, connected peers)` means "asked that many times", and a peer holding the bytes may
never have been asked before the pin is retired. On dev this retires a pin for 6 h and re-probes it; on the backoff
branch it is the reason the hold ladder stops at 24 h instead of a week (`PIN_READMIT_MAX_HOLD`). A round-3 attempt
to track the providers asked per want was reverted after Codex's review (`codex-review-pin-readmit-3.md`) showed
its extra-attempt rule could keep an exhausted pin pending forever while peers came and went. Any fix must keep
retirement live when the provider set churns or drops to zero.

**Done when.** A pure-iroh peer with one unfillable pin and two book peers reaches `caughtUp` with
`elohim_acquisition_pins_retired = 1` after both peers were asked.
