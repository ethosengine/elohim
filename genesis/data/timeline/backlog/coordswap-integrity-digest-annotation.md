---
id: "backlog-coordswap-integrity-digest-annotation"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Conductor STS happ-digest annotation should track the INTEGRITY digest only — coordinator-only bundle changes must ride the rung-1 hot-swap, not roll conductor pods"
slug: "coordswap-integrity-digest-annotation"
written: "2026-09-01"
author: "shift velocity-rungs-overnight"
status: "open"
priority: "high"
jobs: [elohim-edge, elohim-holochain]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-upgrade-propagation-p2p-design-arc"
tags: [coordswap, conductor-split, cycle-time]
---

Observed live (edge #1408, 2026-09-01): a coordinator-only happ change (the
zome_build_info extern) moved the FULL-bundle digest, whose pod-template
annotation legitimately rolled every conductor STS — staggered (rung 3
worked on its first occasion), but rolled. With rung 1 live, a
coordinator-only diff should never roll a conductor: the coordswap stage
delivers it. Fix: the conductor pod-template annotation tracks a digest
over the INTEGRITY zomes + modifiers only (the DNA-hash-equivalent), so
coordinator-only bundle moves leave conductor pods untouched and integrity
moves still roll (correctly — those need the reinstall/migration path).
DoD: a coordinator-only DNA build leaves all conductor STSs `unchanged`
while the DNA pipeline's COORDSWAP stage reports the swap; an
integrity-touching build still rolls, staggered.
