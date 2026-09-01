---
id: "backlog-iroh-quinn-gso-assert-crashes-storage"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Task: iroh-quinn-proto 0.13.0 'untracked_bytes <= segment_size' assert aborts the whole storage process under view-federation burst — any peer can crash a storage's iroh leg; fleet pods share the surface"
slug: "iroh-quinn-gso-assert-crashes-storage"
written: "2026-08-31"
author: "shift 2026-08-31T02-40-fleet-carried-election-convergence (post-close monitor)"
status: "wip"
priority: "high"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-upgrade-propagation-p2p-design-arc"
  - "habit:dataplane-convergence"
tags: [iroh, quinn, crash, availability, dependency, security]
claimedBy: "codex"
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

## Grounded cure decision (2026-09-01)

The version premise was tested before editing: iroh 0.93 and 0.94 still depend
on the same affected `iroh-quinn-proto 0.13.0`, so neither is a cure. iroh 1.x
removes the affected crate family, but that migration remains coupled to the
Wave-3 Holochain/serde family move and is not disjoint from the current rung
lane. Quinn issue #2127 matches the measured assertion + poisoned-destructor
abort; its reporter and maintainer confirmed PR #2167 / commit `434c3586` as
the fix. The disjoint durable landing is therefore a one-line audited backport
of that commit into a crate-local vendored 0.13.0, retaining iroh 0.92's API and
wire contract. GSO stays enabled.

## Scope

- `elohim/elohim-storage/Cargo.toml` and its standalone `Cargo.lock`.
- `vendor/iroh-quinn-proto-0.13.0/**`, copied from the crates.io release and
  changed only by upstream fix `434c3586` plus a provenance/retirement note.
- This task atom and the owning `dataplane-convergence` habit evidence line.
- Focused regression evidence may use existing `p2p_iroh` and
  view-federation tests; no ALPN, framing, shared wire type, payload cap, or
  endpoint behavior changes are authorized.

## Disjointness contract

- MUST NOT edit `src/p2p_iroh/{announcer,peer_book,reconcile_peers}.rs`, which
  belong to the concurrent version-advertisement/rung lanes.
- MUST NOT edit `src/p2p/view_federation.rs`, `src/p2p_iroh/view_fed.rs`, their
  MAX_PAYLOAD/deployed-reader-floor constants, `http.rs`, any Jenkinsfile,
  deployment/orchestrator manifests, or mesh scripts.
- MUST preserve the exact iroh 0.92 / iroh-blobs 0.94 / iroh-gossip 0.92 pins.
  The Wave-3 iroh 1.x migration is a separate dependency arc.

## DoD + verification

- `cargo tree -i iroh-quinn-proto --features p2p-iroh` resolves the vendored
  path and the standalone lock no longer carries the registry checksum/source
  for that package.
- The backport line is identical to Quinn commit `434c3586`; no other
  published source file differs from crates.io.
- `just test-iroh` is green, exercising the feature-enabled iroh suite and the
  existing view-federation wire/MAX_PAYLOAD regression net.
- `just gate elohim-storage` is green. Dependency work requires real
  `cargo test`; `cargo check --locked` alone never satisfies this DoD.
- A household T2 soak/receipt exercises the original burst regime before this
  backlog atom becomes `done`; local proof advances it but does not substitute
  for fleet-shaped evidence.

## Implementation checkpoint (2026-09-01)

- Landed the audited Quinn #2167 / `434c3586` tail-loss-probe fix as a local
  patch of the published `iroh-quinn-proto 0.13.0`; a source-tree comparison
  against crates.io found that one upstream line as the only code difference.
- `cargo tree --features p2p-iroh -i iroh-quinn-proto` resolves the vendored
  crate. `just test-iroh` and `just gate elohim-storage` are green, including
  real `cargo test`, strict Clippy, schema contracts, integrations, and
  doctests.
- Status stays `wip`: the original ~891-exchange fleet burst/disconnect shape
  has not yet produced a household T2 receipt on this patch.

Story-graph handoff: `p2p dataplane ratchet / between view-federation burst ->
storage availability / missing node: a storage receiving a near-floor
view-federation burst survives the remote peer disappearing; probe = process
liveness plus a successful post-disconnect request / current state: local
dependency and protocol suites green, household T2 burst probe unminted`.
