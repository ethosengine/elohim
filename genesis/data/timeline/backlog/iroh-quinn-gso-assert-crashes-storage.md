---
id: "backlog-iroh-quinn-gso-assert-crashes-storage"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "iroh-quinn-proto 0.13.0 'untracked_bytes <= segment_size' assert aborts the whole storage process under view-federation burst — any peer can crash a storage's iroh leg; fleet pods share the surface"
slug: "iroh-quinn-gso-assert-crashes-storage"
written: "2026-08-31"
author: "shift 2026-08-31T02-40-fleet-carried-election-convergence (post-close monitor)"
status: "open"
priority: "high"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-upgrade-propagation-p2p-design-arc"
  - "habit:dataplane-convergence"
tags: [iroh, quinn, crash, availability, dependency, security]
---

Measured 2026-08-31 18:26Z: minutes after the dual-peer-source cure landed and
the fleet made FIRST contact with the workspace supplier W2 (~891
evidence/head-record exchanges), W2's storage panicked inside
`iroh-quinn-proto-0.13.0/src/connection/mod.rs:654`
(`assertion failed: untracked_bytes <= segment_size as u64` — the upstream
QUIC GSO/pacing race), cascaded into a poisoned-mutex destructor panic
(`iroh-quinn-0.14.0/src/mutex.rs:138`) and ABORTED the whole process
(non-unwinding panic). Restarted cleanly.

Why it matters beyond the workspace: every fleet storage runs the same crate
pair on its iroh leg — a remote peer generating the same burst shape can take
down a fleet pod's entire storage process (availability, and a cheap remote
crash primitive). Cure directions: bump iroh/iroh-quinn to a release carrying
the upstream fix (dependency work ⇒ cargo test, not cargo check, per gospel);
meanwhile consider disabling GSO/segmentation offload via quinn transport
config if exposed. Also a candidate first scenario for the burst regime in
backlog-mesh-fixture-fidelity-regimes.
