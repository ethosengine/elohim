---
id: "backlog-governance-ratification-single-write-capturable"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "SECURITY: percentage-threshold governance ratification is capturable by a single write (approve >= 0; eligibility_predicate None at every site)"
slug: "governance-ratification-single-write-capturable"
written: "2026-08-23"
author: "monetary-posture research pass (mint)"
status: "backlog"
priority: "high"
tags: [security, governance, ratification, elohim-storage, research-mint]
cites:
  - elohim/elohim-storage/src/services/governance_action_tally.rs
  - elohim/elohim-storage/src/db/responsibility_demand_configs.rs
---
# Single-write-capturable ratification

A percentage-threshold governance action carries no `"m"` value, so `derive_status` computes the
threshold as `approve >= 0` — satisfied by the first write. `eligibility_predicate` is `None` at every
write site. **First-quorum-wins, permanently.**

- `elohim/elohim-storage/src/services/governance_action_tally.rs:151-161`
- `elohim/elohim-storage/src/db/responsibility_demand_configs.rs:143`

## Why this is the highest-leverage governance defect in the tree

The 2026-08-07 red-team audit stated it plainly: *"This must be fixed before any claim about
council-ratified parameters is defensible."* It is re-verified open as of 2026-08-23.

**Every "the council decides" sentence in the succession and monetary documents is contingent on this
fix**, and both now say so in their own voice — canon
`genesis/docs/content/elohim-protocol/succession.md` §9.3 concedes it in the protocol's public
companion, and [the monetary posture](epr:monetary-posture-internal-currencies-external-fiat-2026-08-23) §5 makes it a precondition rather than a caveat.

The whole detector-reports/council-decides discipline — the thing that keeps an elohim a clerk rather
than a planner — rests on a council's ratification actually binding. Today it does not.

## Fix shape

Give the percentage-threshold path a real `m`, populate `eligibility_predicate` at the write sites,
and add a test that a single approving write does NOT reach ratified status.

**Standalone, not folded into a cluster**: operationally-atomic security defect per `CLUSTERS.md`.

Minted from [the succession evidence bridge](epr:succession-without-conquest-mutualist-lineage-2026-08-23) §2.6.2.
