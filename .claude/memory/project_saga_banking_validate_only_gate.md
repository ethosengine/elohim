---
index: false
name: saga-banking-validate-only-gate
title: "Saga banking = validate-only run; gate reads matthew"
description: "Bank saga/notary measures via [edge:validate-only] (exists since 2026-07-30); the quiesce gate reads MATTHEW only, not the shem trio — bites on every banking attempt"
metadata: 
  node_type: memory
  title: Saga banking = validate-only run; quiesce gate reads matthew
  type: project
  originSessionId: 88cc5b6b-b8f7-4748-9355-792108825b4c
  modified: 2026-08-10T13:27:30.551Z
---

Two facts the 2026-08-09 overnight shift lost, which kept the saga register wedged at 8/11 for weeks:

1. **`VALIDATE_ONLY` edge pipeline mode already exists** — landed 2026-07-30 (b2dfd0de2, on dev): `[edge:validate-only]` HEAD-commit tag or the `VALIDATE_ONLY` param runs ONLY Dataplane Validation against the live fleet, no build/deploy, no 7-pod restart. Every `[build:edge]` banking run is measurement-by-deploy: it restarts the fleet it measures (~20min+ churn, hours of catch-up). The sprint wishlist re-requested this mode as missing — instrument amnesia.
2. **The fleet-quiesce gate's predicate reads storage-A (matthew) ONLY** (`scripts/ci/fleet-quiesce-gate.sh`): A caughtUp + A actionable ≤ QUIESCE_ACTIONABLE_TOLERANCE (CI path sets 2) + A unmeasured=0 + both doorways 200. The shem trio's divergence stock is invisible to it. "Gate blocked on the trio cure" (it-23 of the limit-cycle journal) was a misdiagnosis — the blocker is matthew's own residual actionable rows (anchor-gap class, H3: unanchored originals; batch-3 backfill is the cure).

**How to apply:** to bank a locally-green saga/notary measure, first check `elohim_projection_reconcile_divergent_actionable{pod="elohim-matthew-alpha-0"}` ≤ 2, then push an empty commit tagged `[edge:validate-only]` (tag-last on HEAD). Never fire `[build:edge]` just to measure. Related: [[project_angular22_node24_campaign]].
