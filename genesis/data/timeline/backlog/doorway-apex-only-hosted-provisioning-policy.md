---
id: "backlog-doorway-apex-only-hosted-provisioning-policy"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "find_least_loaded spreads hosted-user provisioning onto household conductors — apex doorways need an apex-only provisioning policy"
slug: "doorway-apex-only-hosted-provisioning-policy"
written: "2026-07-30"
author: "claude (integrator shift — surfaced by the hosted-agent cap lever review)"
status: "open"
priority: "medium"
jobs: [elohim-edge]
tags: [doorway, provisioning, hosted-agents, household-nodes, capacity, pool-map, mishpat, delegates-compute]
cites:
  - doorway/doorway-service/src/conductor/pool_map.rs
  - genesis/data/timeline/backlog/self-heal-adam-projection-catchup-exhaustion-full-arc.md
---

# Hosted-user provisioning can land on household conductors

Doorway B's `CONDUCTOR_URLS` spans all six alpha conductors, and
`find_least_loaded` picks the conductor with the most remaining headroom. This
predates the cap lever, but the `DOORWAY_MAX_AGENTS_PER_CONDUCTOR=32` cap on
doorway B amplifies it: with adam seeded at ~35/32 (at capacity), every NEW
hosted-user signup on B now provisions onto a household conductor (matthew /
jessica / james sit at 1-2 agents of 32) instead of the multi-tenant apex.

That placement is an architecture smell, not just a tuning artifact: household
nodes are personal participants — the hub-optional floor says hubs graduate
convenience, never absorb strangers' hosted cells. A stranger's hosted agent on
matthew's household conductor is misplaced custody.

## The decision this needs

A provisioning-eligibility policy separate from the capacity cap — e.g. an
apex-only allowlist (or an `is_multi_tenant` flag per conductor entry) that
`find_least_loaded` filters by, so at-capacity apex → clean 503 refusal rather
than household spillover. Longer-term this is Mishpat-shaped: hosting-eligibility
as a bounded `delegates-compute` commitment per conductor, not an env list —
same trajectory as the admin X-API-Key displacement noted in the catch-up
backlog entry.

## Bounds

- Current impact on dev alpha is small (signups are mostly e2e personas), but
  every such persona placed on a household conductor grows that household's
  space population — the exact arc-convergence cost the cap exists to contain.
- Not blocking the cap deploy; refusal-vs-spread is a policy call the operator
  should make deliberately.
