---
id: "backlog-deploy-spa-blob-silent-unstable-on-degraded-node-shallow-health"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Upload SPA Blob silently degrades to UNSTABLE + elohim.host 404s when an alpha node's storage data-path is stuck while shallow /health stays 200"
slug: "deploy-spa-blob-silent-unstable-on-degraded-node-shallow-health"
written: "2026-06-29"
author: "pipeline-shakeout shift"
status: "open"
priority: "medium"
ci_status: backlog
jobs: [elohim]
tags: [ci, deploy, stage-spa-blob, health-check, readiness, doorway, conductor, alpha-b, elohim-host, catchError, observability]
cites:
  - scripts/ci/stage-spa-blob.sh
  - Jenkinsfile
  - genesis/data/conductor-leak-rca-diverse-eyes-synthesis-2026-06-18.md
---

# Deploy leg silently UNSTABLE + elohim.host 404 when a node's data-path is stuck behind a green /health

## Observed (2026-06-29, app build elohim/dev #1573, after the principle-7 trigger-glob fix 0393bee62)

The app pipeline re-ran to recover the stale elohim.host SPA mount. Outcome:
- `Upload SPA Blob` stage → **UNSTABLE** (catchError-swallowed), build continued to E2E.
- `https://elohim.host/` stayed **404** (the recovery did NOT land).

## Root cause (high confidence — direct probes + ci-investigator log read + Prometheus)

All 6 deploy legs (`{alpha.elohim.host[matthew], elohim.host[adam/alpha-b]} × {elohim-host-landing browser, elohim-host-landing server, lamad-spa browser}`) failed with **HTTP 503** across all 3 retry attempts. But the failure is NOT where it looks:

- The blob `PUT /admin/seed/blob` **succeeded** every attempt (`{"success":true,...,"size":9985486}`) → **auth/admin-X-API-Key is valid; not the gate.**
- The **503 is from the second step** — the content-row staging (`PATCH /db/content/{slug}` that sets the blobHash → the mount).
- Live read-vs-write disambiguation at the time:

  | probe | alpha.elohim.host (matthew) | elohim.host (adam / alpha-b) |
  |---|---|---|
  | `/health` | 200 | **200** (shallow, doorway up) |
  | `/db/stats` | 200 | **000** (no response / timeout) |
  | `/db/content/elohim-host-landing` | 200 | **503** |

  So adam's **storage/conductor DATA path is stuck** while its doorway front answers `/health` 200. matthew is healthy now but its content row still carried the OLD blobHash (`1c345187…`, not the freshly-built `6059704c…`) — matthew's PATCH failed in the same window and matthew has since recovered.

- Prometheus on `elohim-adam-alpha-0` (container `elohim-node`, ns `elohim-alpha`, node `shem`): `kube_pod_container_status_restarts_total = 3` **flat for 3h+** (no crash-loop during deploy); working-set ~0.95–1.09 GB (modest, **no OOMKilled**); last terminated reason `Error` >3h ago. → degraded-but-not-crashed: the storage/conductor process is up but the data path hangs/sheds. This is a DIFFERENT signature from the OOM-climb in [[conductor-leak-rca]] — capture, don't conflate.

## The two repo-fixable gaps (the durable lessons)

1. **Shallow `/health` masks a dead data-path.** doorway `/health` is admission-exempt and returns 200 even when its upstream storage/conductor data path is 503/timeout. A deploy that trusts `/health` (or a human probing it) reads "healthy" while the node can't serve content. **Fix candidate:** a deeper readiness probe (hit `/db/stats` or a cheap conductor-backed read) for deploy-target selection and for the operator-facing health.

2. **`Upload SPA Blob` catchError→UNSTABLE-swallow hides a persistent deploy failure.** A sustained all-legs-failed deploy turns the app build merely UNSTABLE (reads as "fine-ish" on the board) while `elohim.host/` silently 404s — the same alpha-b-leg-swallow pattern noted in memory. **Fix candidate:** either (a) a pre-stage data-path readiness gate that skips/flags a degraded host loudly (and does NOT silently swallow), or (b) make a host-left-STALE persistent failure fail the stage (or emit a named, alerting junit failure that the board surfaces), so a stale mount is never silent.

## Immediate operator action (live-cluster, not repo)

Restart/heal adam's storage backend (`elohim-adam-alpha-0`, container `elohim-node`, ns `elohim-alpha`, node `shem`) — a restart should clear the stuck data path (it has NOT auto-recovered in 3h+). Then re-run the app deploy (`[build:app]` or re-run #1573's `Upload SPA Blob`): the content-row PATCH lands → `elohim.host/` recovers 404→200. The same re-deploy also refreshes matthew's stale blobHash. Repo side is already correct (blob uploaded, build green-bar, principle-7 fix landed + verified).
