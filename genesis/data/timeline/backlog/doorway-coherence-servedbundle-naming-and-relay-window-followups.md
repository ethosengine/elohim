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

## 2026-10-01 missing station: bound slug definition and channel delivery

**Chain:** doorway-failover / between "the peer verifies the current native slug definition and its notarized release-channel binding" → "both doorways serve the same governed landing version through withdrawal and recovery" / **missing node: bound-slug authority and serving provenance are reconciled without confusing the native document winner with the channel manifest that elects its browser/server bytes.**

**Assertion + named probe — `bound-slug-authority-and-serving-provenance`:** use the existing own-conductor `resolve_content_head_local` read for the slug and its bound channel, the existing content/head HTTP reads, and the release adoption receipt. Compare the slug winner and signed binding across peers separately from the channel winner, manifest artifact hashes and applied browser/server hashes. Each served version must trace to that channel election and receipt while its native document definition remains truthful. Equal blobs alone are not proof of equivalent authority and cannot authorize a head stamp. This names a missing verification station, not a new endpoint or implementation policy.

**Current state:** Matthew, Jessica and James agree on native landing winner uhCkk1Di_CcjnuAl-Wojn5NDWamQkfOjnMYLdzu75gVTcp_vq6Wu1 while their SQL heads differ (iV7/LN5/2oy); their served browser blob and runtime:app-bundle:alpha:dev binding agree. Current `project_authenticated_content_head` skips the entire stamp for a retained binding (`p2p/projection_reconcile.rs:6589`); `release_channel_owns_serving` compares channel metadata, with no same-blob exception. `AppBundleVehicle` projects browser/server bytes and never stamps a slug head (`release_adoption/apply.rs:1619`). Native-delivery plan lines 533–539 expressly hold a bound slug before any pointer/head stamp. No supported path was found that reconciles the SQL document head under these constraints. Manual SQL stamps, unbind/rebind, another root affirmation and changes to the frozen convergence assertion do not discharge this station. The habit remains RED; current serving and whole-campaign convergence are unproved.

- 2026-10-02 campaign 1.4 landing blocker: final-source diagnostic `household-campaign14-serving-browser-20261002T0603Z` failed bundle staging's blobHash PATCH with HTTP 503/native "Content with id … already exists. Use update_content"; owned author cancellation calls subsequently timed out. Smallest unblock: reuse the already-authored native root in the existing bundle update/retry path, prove cancellation/readback, then obtain a genuine full current-source serving receipt. Scope-stop evidence: `genesis/a2o/reports/recovery/campaign-1.4-restart-20261001/functional-landing-serving-stop-20261002.json`. This is separate from the static bound landing mismatch and from Shem's deferred performance investigation; no repair or gate waiver made here.
- 2026-10-02 authorized continuation: native-root retry repaired at `7f3c6ade0`; the `1121Z-observers` run passed both visitors but exposed undersized setup/cancellation observers and a late restart of an already-cleaned fixture. Phase composition is repaired without changing serving deadlines; full serving receipt remains required. One captured old fixture returned deregistration 502 before its guarded stop (`interrupted-observers-cleanup.json`); retain that roster-cleanup follow-up here, separate from the static landing mismatch and the Shem performance work.
- 2026-10-02 necessary functional repair: `1143Z-phases` failed all five serving stations; a reproduced observer-cancellation race released the cell/file lock while the offered native write could still commit. Transfer the existing scheduling lease and capacity permit to that response, then require the unchanged full serving proof. The regression failed before repair and all 22 write-gate tests pass afterward; runtime proof remains pending. Performance investigation remains on Shem.
