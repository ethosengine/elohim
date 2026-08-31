---
id: "backlog-upgrade-propagation-p2p-design-arc"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Design arc (operator course-set 2026-08-31): p2p hApp upgrade/revert propagation — mixed-version peers keep communicating, no big-bang fleet rolls; the crux before inviting app developers onto the SDK"
slug: "upgrade-propagation-p2p-design-arc"
written: "2026-08-31"
author: "shift 2026-08-31T02-40-fleet-carried-election-convergence"
status: "open"
priority: "high"
jobs: [elohim-edge, elohim-holochain]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-late-joiner-peer-discovery-boot-only-board"
  - "backlog-mesh-fixture-fidelity-regimes"
  - "habit:dataplane-convergence"
tags: [upgrade, rollback, dna-lineage, sdk, dataplane, brainstorm-class]
---

Operator course-set (2026-08-31, verbatim essence): *"Full support for upgrade
propagation revert/upgrade over p2p is another engineering feat to achieve
before I'd feel confident for people to start trying to use it. Holochain
encrypts data to ONE hApp bundle — this seam cannot be upgraded by any
external push; the network itself must agree on how it evolves."* The
multi-hour roll-per-change dev cycle is the wall-clock this arc structurally
retires.

Existing inventory (all proven live 2026-08-31, the carried-election shift):
- **Coordinator hot-swap over one admin call** — running conductor, no
  re-key, no reboot (admin `update_coordinators`, applied to W2 at 14:00Z).
- **Mixed-version wire discipline** — additive serde(default) fields with
  byte-identity pins; old and new peers conversed through a rolling window.
- **Same-hash happ splice** — old integrity + new coordinator wasm repacked,
  DNA hash preserved (manual instance of the version-lineage crossing).
- **Elected canonical heads carried peer-to-peer with local wasm
  verification** — structurally the mechanism for network-agreed evolution:
  "which bundle is canonical" is a head election over an artifact the storage
  plane already replicates.
- The genuinely hard remainder: DATA REBINDING across DNA lineage
  (HC 0.6 gates `lineage:` behind unstable-migration; we own the conductor
  fork). Read `2026-06-11-dna-upgrade-governance.md` first.

Brainstorm-class: route through /brainstorm (p2p-design-gate applies —
upgrade artifacts, lineage records, and adoption elections are data
entities). Not to be ground as shift iterations.
