---
id: "backlog-doorway-coherence-servedbundle-naming-and-relay-window-followups"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Doorway coherence/servedBundle follow-ups from the one-head-delivered sprint (Task F2/F3) — naming, the self-agreeing digest, the post-restart relay window, and a dev bundle that bypasses the serving doorway"
slug: "doorway-coherence-servedbundle-naming-and-relay-window-followups"
written: "2026-09-27"
author: "one-head-delivered sprint, Lane F tasks F2/F3 (recorded post-hoc; no cluster fit found in CLUSTERS.md)"
status: "open"
priority: "medium"
jobs: [elohim-edge, app]
relatedNodeIds:
  - "habit:doorway-failover"
cites:
  - .superpowers/sdd/2026-09-26-one-head-delivered-sprint-plan/task-F3-report.md
  - .superpowers/sdd/2026-09-26-one-head-delivered-sprint-plan/task-F2-report.md
  - doorway/doorway-service/src/routes/coherence.rs
  - app/elohim-app/src/app/elohim/services/storage-api.service.ts
tags: [doorway, federation, coherence, servedBundle, relay-window, storage-api, dev-bundle, follow-up]
---

Four related follow-ups surfaced by Task F2/F3 (served-under-standing / apex-transition symmetry work),
none with a matching backlog cluster in `CLUSTERS.md` today — recorded here for neighbor discovery until
one forms.

1. **Rename coherence `declaredHead` → `servedBundle`.** The field is read as `headActionHash` by callers
   who shouldn't assume that; it names the served bundle, not a head action hash. Source: task-F3-report.md
   line 194 ("Rename coherence `declaredHead` to `servedBundle` so it is not read as `headActionHash`.").
2. **The coherence digest folds in each doorway's own `commitment_id`, so A/B digests never agree.**
   `compare_to_peer` and the digest include a per-doorway unique `commitment_id` (`project-epr-98f0…` on A
   vs `project-epr-70dc…` on B), so two doorways never "agree" on the digest even with identical heads.
   Story 1.5's `pair head` line should compare the EPR ID at the covering mount instead, once story 1.4
   gives one head per EPR. Source: task-F3-report.md line 44.
3. **A listed non-holder relays for ~60-90s after a restart (bounded, but unnamed as a wait condition).**
   Measured: it takes 30s for the non-holder's own head to reconcile plus one discovery tick for the
   holder's contract to reach its name-route table. A lane run straight after a mesh restart fails scenario
   1 until this window passes; the wait should be named so a lane can gate on it rather than discover it
   by failure. Source: task-F3-report.md lines 73, 152, 198.
4. **The dev bundle compiles `environment.holochain.storageUrl` and bypasses the serving doorway.**
   `app/elohim-app/src/app/elohim/services/storage-api.service.ts:140-141` reads
   `environment.holochain?.storageUrl ?? resolveDoorwayUrl(environment.client?.doorwayUrl ?? '')` — a
   compiled `storageUrl` (development/native environments both set `http://localhost:8090`) always wins,
   so `resolveDoorwayUrl` (which would return `location.origin` in a non-Tauri browser) is never reached.
   This makes the runtime-endpoint scenario red on the household mesh. Candidate cure (not started):
   `resolveDoorwayUrl(environment.holochain?.storageUrl ?? environment.client?.doorwayUrl ?? '')` so the
   serving origin wins in a browser while Tauri/SSR keep the configured sidecar — check the `ng serve` dev
   proxy covers `/db/*` first. Source: task-F2-report.md lines 46-49, 55; progress.md ("Task F2: complete
   … frontier: dev bundle compiles storageUrl and bypasses the serving doorway").
