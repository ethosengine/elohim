---
id: "backlog-a2o-blob-backed-content-idempotent-create-h-app-id-mismatch"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Blob-backed content create 500s on UNIQUE constraint despite the idempotent-create helper — possible h_app_id scoping mismatch"
slug: "a2o-blob-backed-content-idempotent-create-h-app-id-mismatch"
written: "2026-07-03"
author: "blob-durability-suite-green shift"
status: "resolved"
priority: "low"
ci_status: resolved
fingerprints: []
jobs: [elohim-edge]
relatedNodeIds: [blob-durability]
tags: [ci, a2o, resilience, idempotency, content-create]
cites:
  - genesis/a2o/steps/resilience.steps.ts
  - genesis/a2o/features/resilience/grandma-photos-survive-node-loss.feature
---

# `storagePostContent` idempotent-create check didn't catch a pre-existing row

## The failure

Discovered 2026-07-03 (edge #1144). Scenario "A blob-backed album lights stewarding via
the operator seed lever" (`grandma-photos-survive-node-loss.feature`) fails at the `Given
a blob-backed content item "grandma-album-1974" exists with reach "intimate"` step:

```
AssertionError [ERR_ASSERTION]: POST /db/content failed: 500 {"error":"Internal error:
Insert failed: UNIQUE constraint failed: content.h_app_id, content.id"}
```

The step calls `storagePostContent` (`genesis/a2o/steps/resilience.steps.ts`), which is
documented and designed to be idempotent — GET `/db/content/{id}` first; if 200, treat as
a no-op; otherwise POST. The 500 means the GET existence-check returned non-200 (content
NOT found) even though a row with the same `(h_app_id, id)` composite key already exists
— i.e. the GET and the POST appear to resolve to *different* `h_app_id` scopes, or the
GET's 404 is a false negative for some other reason.

`grandma-album-1974` is a fixed content id on a real, persistent, shared alpha cluster —
plausible that a prior test run (or a different scenario) already created it under a
different operating scope.

## Why not fixed in the same shift

Single scenario (not blocking others), and the actual cause (h_app_id scoping between the
GET check and the POST create, or something else in `storagePostContent`/the storage
API's scope resolution) needs real investigation — not a one-line fix I could verify
locally without live cluster access to reproduce the exact scope mismatch.

## Proposed next step

Investigate `storagePostContent`'s GET call vs the POST's `h_app_id` resolution (does the
GET need an explicit scope header/param the POST supplies implicitly?). If confirmed,
either fix the GET to match the POST's scope, or make the content id include a
per-run-unique suffix so the fixture never collides across CI runs (matches the "test
fixture must be idempotent, not the storage" principle already documented at the top of
`storagePostContent`).

## Resolution (2026-07-03, same shift, second look)

Not an h_app_id mismatch — both GET and POST `/db/content` resolve through the same
legacy-prefix branch of `extract_app_context` (`http.rs:3882-3921`), so h_app_id scoping
was never divergent. The real cause: `handle_db_content_by_id`'s GET handler
(`http.rs:4752-4873`) applies a reach gate — a restricted-reach row (`intimate` here) with
no `Authorization`/`X-Agent-Id` header on the request returns **403 Forbidden**, not 404,
and *only* takes that branch when the row was actually found (`Ok(Some(view))`; a
genuinely-missing id 404s). `storagePostContent`'s idempotency check only treated `200` as
"exists" and fell through to POST on anything else, so a re-run against an
already-existing restricted-reach id POSTed again and hit the real UNIQUE constraint.
Fixed by also treating `403` as "exists, not readable by us" — a no-op, matching the
200 case. Commit lands the fix directly in `genesis/a2o/steps/resilience.steps.ts`
(`storagePostContent`).
