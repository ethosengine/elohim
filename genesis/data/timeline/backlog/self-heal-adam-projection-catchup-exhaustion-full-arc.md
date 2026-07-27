---
id: "backlog-self-heal-adam-projection-catchup-exhaustion"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "adam (B / elohim.host) projection catch-up stalls after a deploy restart — full-arc heal conductor calls exceed the 15s per-attempt timeout, doorway stays 503 catching-up for 80min+, retry-exhausting cross-doorway declares"
slug: "self-heal-adam-projection-catchup-exhaustion-full-arc"
written: "2026-07-27"
author: "claude (resiliency-saga sprint-3 delivery — ch06 runtime blocker RCA)"
status: "open"
priority: "high"
ci_status: blocked
jobs: [elohim-edge]
tags: [self-heal-exhaustion, projection-reconcile, catch-up, full-arc, target-arc-factor, adam, shem, restart-churn, heal-timeout, ch06, declare]
cites:
  - resiliency-saga-sprint3-objective | Resiliency Saga Sprint 3 Objective | path: genesis/docs/superpowers/plans/2026-07-26-resiliency-saga-sprint3-objective.md
  - elohim/elohim-storage/src/p2p/projection_reconcile.rs
  - genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md
---

# adam's post-restart catch-up cannot complete under full-arc load

## The finding (2026-07-27, live, blocking ch06 delivery)

After the sprint-3 coordinator hot-swap restarted the alpha conductors (edge
#1243 deploy, ~23:40 UTC), **adam** (backing doorway B / elohim.host) entered a
projection catch-up it has not completed 80+ minutes later — 4× the ~20-min
restart-churn the substrate trust contract expects. Symptom chain:

- `GET https://elohim.host/db/content/*` → `503 {"status":"catching-up"}`; the
  doorway `/health` is otherwise green (conductor connected, 7/7 pools healthy,
  uptime advancing — not crash-looping).
- adam's `projection-reconcile` logs each sweep (01:02, 01:05, 01:07):
  `heal complete … caught_up: false, content_healed: 0,
  content_local_anchored: 4188, content_divergent_anchor: 3599,
  content_ids_discovered: 8717` — i.e. thousands of gaps, **zero healed**.
- Cause, repeated every sweep: `projection-reconcile[content]: conductor
  resolve failed; retry next sweep — Request timeout: heal conductor call
  exceeded per-attempt timeout 15s (transient)`. Also on the REA leg
  (`conductor get failed … 15s`).

adam holds a **full-arc** working set (`target_arc_factor=1` → authority for
every hash; RAM/latency ∝ corpus — see [[project_per_node_memory_is_conductor_authority_arc]],
where james OOMed the same way). On the shem multi-tenant node, conductor
calls routinely exceed the 15s heal timeout, so heal makes zero forward
progress and the projection never reaches `caught_up`. The `503 catching-up`
gate then refuses all reads AND blocks the doorway from accepting a
`declare_canonical_head` — so the declare-carries-Record cross-declare that
would converge B's anchor (saga ch06) **cannot land**, and `authorHeadOnce`'s
retry ladder (`DECLARE_MAX_ATTEMPTS=24` × ~30s ≈ 12 min) exhausts against the
503 long before adam recovers.

## Why this is a ceiling item, not a shift fix

The sprint-3 code is delivered and verified working — hot-swap applied on all
conductors, `/head-record` live, A re-authored its head, the declare mechanism
is sound. The blocker is **substrate capacity**, not code:

- Can't be fixed from the dev seat (cluster ops are operator-owned; no kubectl).
- A code-side redeploy would restart adam into the same catch-up storm.
- The real levers are substrate decisions: **`target_arc_factor < 1` for adam**
  (shrink the working set so conductor calls finish under 15s — the scale lever
  named in [[project_per_node_memory_is_conductor_authority_arc]]), and/or
  raising/backpressuring the 15s heal per-attempt timeout, and/or moving adam
  off the contended shem node.

## What would land ch06 once adam is healthy

adam serving 200 (caught_up=true) → re-run the App pipeline's `authorHeadOnce`
(a `[build:app]` push) so the declare-carries-Record cross-declare lands A's
head on B against a responsive conductor. The mechanism is proven; it needs a
declare cycle against a non-503 B.

## Operator decision

1. Reduce adam's `target_arc_factor` below 1 (or relocate adam off shem) so
   post-restart catch-up completes within the trust-contract window.
2. Until then, every deploy that restarts adam re-opens an 80min+ 503 window on
   elohim.host — the restart-churn contract (~20min) does not hold for this node.
