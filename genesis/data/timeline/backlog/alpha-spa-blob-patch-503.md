---
id: "backlog-alpha-spa-blob-patch-503"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "alpha.elohim.host SPA-blob PATCH/verify 503s during App deploy (blob upload succeeds) — adam storage peer or PATCH endpoint unhealthy"
slug: "alpha-spa-blob-patch-503"
written: "2026-06-27"
author: "overnight doorway-deploy + genesis fan-out shift (2026-06-27T03)"
status: "backlog"
priority: "medium"
jobs: [elohim]
---

## The symptom

In App build `elohim/dev #1563`, the "Upload SPA Blob" stage's **alpha.elohim.host**
legs failed differently from elohim.host: the blob PUT SUCCEEDS
(`✓ blob uploaded, already_cached:true, forwarded_to_storage:true`) but the subsequent
PATCH/verify step returns **503 on every attempt** (all 3 retries), leaving all three
alpha legs (elohim-host-landing browser+server, lamad-spa browser) STALE:

```
✓ [elohim-host-landing] blob uploaded (via /admin/seed/blob)
curl: (22) The requested URL returned error: 503
⚠ [elohim-host-landing] attempt 1/3 against https://alpha.elohim.host failed — retrying in 5s
... (×3) ... ERROR: host left STALE
```
(build #1563 log lines ~5812–5819, 5994–6001, 6064–6071.)

This is **distinct from the elohim.host 401** (which was the missing seed-PUT auth header,
fixed in `6dd2ea5dd`). alpha is `DEV_MODE` so its PUT is never gated — the failure is on
the **PATCH `/db/content/{slug}` (or its read-back verify) returning 503**, i.e. the
matthew/adam storage backend behind doorway-alpha is shedding or unhealthy at deploy time,
not an auth problem.

## Likely cause / where to look

- The 503 is a runtime storage-health/shed signal, not a repo bug — same family as the
  conductor-leak / adam-peer-health work (`project_storage_metrics_surface_and_leak_verdict`,
  the doorway M1–M5 watchdog/shed). The PATCH hits the storage write path; a 503 there =
  admission-shed or backend-down during the deploy window.
- The bounded retry (`stage-spa-blob.sh`, `adcb695d4`) already absorbs *transient* 503s; a
  PERSISTENT 503 across all 3 attempts means the backend was down/shedding for the whole
  window — operator/cluster territory, not a longer retry.

## Proposed next step

Operator-side: confirm the doorway-alpha storage backend's health at the #1563 deploy
window (Loki `smaps`/shed metrics, doorway `/metrics` M3 shed counters). If the storage
peer was OOM-restarting (the open conductor-leak), this 503 is downstream of that and
clears when the leak is contained. If the PATCH endpoint itself 503s under normal health,
that's a doorway/storage write-path bug to scope. Re-check on the next App deploy
(`emitAppDeployJunit` now names the per-host leg, so the signal is visible in test results).

## Evidence / refs

- ci-investigator verdict this shift (build #1563 alpha legs = 503 PATCH, elohim.host legs = 401 PUT).
- Shift journal: `.claude/shifts/2026-06-27T03-overnight-doorway-deploy-genesis-fanout.journal.md` (iter-4).
- Memory: `project_storage_metrics_surface_and_leak_verdict`, `project_prod_main_lag_vs_alpha_dev` (leg 2 = SPA-blob staging).
