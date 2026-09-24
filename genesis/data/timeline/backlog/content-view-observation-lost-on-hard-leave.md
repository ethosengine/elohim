---
id: "backlog-content-view-observation-lost-on-hard-leave"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A content view is never witnessed when the reader leaves by closing the tab or crossing to another app bundle"
slug: "content-view-observation-lost-on-hard-leave"
written: "2026-09-24"
author: "agent:implementer@claude-opus-5-5 (Lane A, task A9 — seam found while authoring the scenario)"
status: "backlog"
priority: "medium"
domain: "D2"
area: "rea-runtime ObservationEmitterService + lamad content viewer"
tags: [observation, attention, pagehide, keepalive, bundle-seam, lane-a]
relatedNodeIds:
  - attention-witnessed-privately
cites:
  - genesis/docs/superpowers/plans/2026-09-25-post-station-4-participant-classifier-search-attention-sprint.md
  - app/elohim-library/projects/elohim-rea-runtime/src/lib/observation-emitter.service.ts
  - app/lamad/src/app/components/content-viewer/content-viewer.component.ts
  - genesis/a2o/features/lms/attention-witnessed-privately.feature
shift_objective: |
  Make a content view reach the reader's node however they leave the page. Flush every open
  view on pagehide / visibilitychange-to-hidden with a request that survives unload (fetch with
  keepalive: true, carrying the same auth the HttpClient path carries), and keep the in-app
  end() as the normal path, so a view is posted exactly once. Done when an a2o step that leaves
  /resource/{id} by a full navigation to /lamad/me/stream finds the entry in the stream, and the
  in-app leave still posts once.
---

# A hard leave loses the witness

**Chain.** attention-witnessed-privately. The seam lies between "the reader leaves the page" and
"the node holds the note". The missing node is: **the note is sent on every way of leaving the page.
Probe: a full-document navigation away from `/resource/{id}` still produces one answered
`POST /api/v1/observations`. Current state: red.**

**Mechanism.** `ObservationEmitterService.end(ref)` posts through Angular's `HttpClient`, and
the content viewer calls it from its route-param change and from `ngOnDestroy`. Neither runs
when the document unloads. That covers a tab close, a typed address, a reload, and any
**cross-bundle** link. The last case matters most: the viewer lives in the elohim-app shell
(`/resource/:id`), and the person's own stream lives in the separately served lamad bundle
(`/lamad/me/stream`). The most natural journey, from reading to "show me what I read", is
therefore a full navigation, and it drops the note.

**How the scenario handles it today.** `attention-witnessed-privately.feature` has the
reader move on in-app ("moves on from the page to the home page without leaving the app").
The step routes inside the shell and waits for the POST's answer. The feature's narrative
says the story covers moving on inside the app and not closing the tab. The scenario covers
the path that works and does not claim more.

**Direction.**

1. In the emitter, register `pagehide` (and `visibilitychange` to `hidden`) and flush every open
   view with `fetch(url, { method: 'POST', keepalive: true, headers, body })`. `sendBeacon`
   cannot set the auth header the doorway needs.
2. Mark flushed refs closed so a later in-app `end()` does not post twice. A view that
   becomes visible again after a hidden flush starts a new view.
3. Add a second a2o step variant that leaves by `page.goto` and asserts the entry, and keep the
   in-app variant as it is.
