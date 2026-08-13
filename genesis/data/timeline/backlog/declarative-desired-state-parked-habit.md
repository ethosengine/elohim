---
id: "declarative-desired-state-parked-habit"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Parked habit: declarative-desired-state — peers reconcile signed desired-state like coordinator hot-swap"
slug: "declarative-desired-state-parked-habit"
written: "2026-08-13"
author: "claude (habit displacement, operator-directed 2026-08-13)"
status: "envisioned"
priority: "medium"
tags: [parked-habit, desired-state, deployment, brit, eprfs, coordinator-hot-swap]
cites:
  - genesis/manifests/habits.yaml
  - genesis/data/timeline/backlog/agentic-harness-borrows-backlog.md
---

# Parked habit: declarative-desired-state

**Why parked (2026-08-13):** displaced from the 12/12 `habits.yaml` register to admit
`dev-system-equilibrium` (agentic-harness-borrows row 7, operator-directed). This habit's own
`first_move` already declared itself deliberately parked — *"park until this node is green"* —
referring to a different habit's completion as its precondition, and it carried no `evidence:`
field. A self-declared-parked commitment occupying a register slot is exactly what the covenant's
displacement rule exists for. **The habit is not wrong; it is not yet observable.**

**Return condition:** when its precondition greens (the brit metadata plane / eprfs FUSE
projection direction becomes testable), this re-enters the register through the normal covenant
(displace one or wait), entering `unwired` with the first_move below intact.

## The habit block, verbatim (as displaced)

```yaml
  - id: declarative-desired-state
    invariant: >
      Peers reconcile signed desired-state (images, config, seeds) the way
      coordinators hot-swap: a version bump converges every peer without
      imperative cluster surgery. CI builds and publishes; it does not deploy.
    status: unwired
    active: false
    first_move: >
      Write the red: scenario — bump a signed release manifest; every peer
      converges (image + config) with no kubectl in the path; a peer that was
      down converges on wake. (In-protocol precedent: happ_manager::
      sync_coordinators. Endgame direction: the substrate as filesystem —
      EPR heads riding files (brit metadata plane → elohimfs/FUSE projection);
      park until this node is green.)
    refs:
      - "memory: project_brit_next_gen_epr_meta_foundation"
      - "elohim/holochain/Jenkinsfile — ALLOW_COORDINATOR_UPDATE hot-swap (the model)"
```
