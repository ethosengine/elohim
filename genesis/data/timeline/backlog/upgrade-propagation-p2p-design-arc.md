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

## The velocity ladder (operator-harvested 2026-08-31)

The change-class taxonomy, measured against the 2026-08-31 arc (three fleet
rolls paid for changes that mostly had — or nearly had — atomic mechanisms).
Cheap iterations everywhere possible are what buy the wall-clock to shake out
the genuinely hard seams; each rung is a workable item on this arc, ordered by
leverage-per-line-of-code:

| # | Rung | Change class it frees | State |
|---|------|----------------------|-------|
| 1 | **Decouple conductor lifecycle from storage** — the conductor is storage's CHILD PROCESS in the fleet pod, so every storage binary swap kills it and resets its arcs. Let storage swap in place (exec-swap or split into separately-imaged containers) while the conductor keeps running. All three 2026-08-31 fixes were storage-side; on the mesh and on W2 storage restarted repeatedly with the conductor untouched. | native storage/doorway binaries — the class that manufactured every catch-up queue today | **pull-forward candidate: highest leverage per line** |
| 2 | **Staggered rolls** — when a restart IS needed, never all 7 pods at once; the fleet never enters the all-arcs-Empty regime (see backlog `staggered-conductor-fleet-restarts`). | restart-shaped changes generally | existing backlog, fold in |
| 3 | **Config as runtime surface, not boot env** — flags (e.g. ELOHIM_OBEY_CARRIED_ELECTION, read via OnceLock at boot) become watchable at runtime; protocol-native endgame: config as declared EPRs, so a flag flip is a head declaration that converges. | config flips — one boolean cost a full roll on 2026-08-31 | missing |
| 4 | **Coordinator hot-swap gets its own delivery vehicle** — the mechanism (admin `update_coordinators`, no re-key, no churn) is atomic and proven twice on 2026-08-31, but it only ships bundled inside the pod-image roll. Separate the vehicle: coordinator bundles propagate as artifacts and apply without a roll. | coordinator zome logic | mechanism exists; vehicle missing |
| 5 | **Binary/wasm artifacts as ELECTED CONTENT** — an update is a content-addressed artifact with an earned canonical head; peers verify locally and adopt at their own pace; mixed versions coexist via the additive-wire discipline; **revert = the election moving back to the prior head**. The carried-election machinery IS this kernel. | everything above the DNA line | kernel proven (carried-election); propagation + adoption design open |
| 6 | **DNA lineage migration** — the irreducible network-agreement seam (hash = network identity; data rebinds across lineage). Once rungs 1-5 exist it becomes RARE and governed — a constitutional event, not a Tuesday. | integrity zomes / DNA | the hard kernel; dna-upgrade-governance is the prior art |

Cross-cutting lesson (2026-08-31): even atomic changes paid big-bang prices
because everything ships in ONE vehicle (the pod image). **Separate the
delivery vehicle by change class.**

Brainstorm-class: route through /brainstorm (p2p-design-gate applies —
upgrade artifacts, lineage records, and adoption elections are data
entities). Rung 1 may be pulled forward as bounded shift work ahead of the
full design arc. Not to be ground as unplanned shift iterations.
