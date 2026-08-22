---
id: "backlog-apps-serve-arms-stamp-different-content-address-planes"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Doorway /apps serving: the slug arm stamps the bundle CID as X-Content-Address while the CID arm stamps a per-file sha256 and drops X-Content-Slug — one resource, two content identities"
slug: "apps-serve-arms-stamp-different-content-address-planes"
written: "2026-08-22"
author: "claude (wave3 full-lane triage, Act I household mesh)"
status: "backlog"
priority: "medium"
severity: medium
domain: D-delivery
source: "a2o wave3-full 2026-08-22, features/delivery/content-addressing.feature 'CID URL serves same content without slug lookup'; reproduced live on the mesh doorway"
relatedNodeIds:
  - genesis/a2o/features/delivery/content-addressing.feature
tags: [delivery, doorway, content-addressing, wire-contract, cache-invalidation]
cites:
  - doorway/doorway-service/src/routes/apps.rs
  - genesis/a2o/steps/delivery.steps.ts
---

# The two /apps serve arms answer with different content-address planes

Measured live on the household mesh doorway (2026-08-22):

```
GET /apps/evolution-of-trust/index.html
  → 200, X-Content-Address: bafkreihokma4…  X-Content-Slug: evolution-of-trust

GET /apps/bafkreihokma4…/index.html          (the SAME file, by the address the
  → 200, X-Content-Address: sha256-ee5301c9… slug arm just handed out)
```

`routes/apps.rs` has two header-stamping sites: the slug-resolution arm (~line 347)
stamps the app bundle's CID; the direct-address arm (~line 454) stamps `addr` — a
per-file sha256 digest from the extraction path — and stamps no `X-Content-Slug`.

## Why it matters

`X-Content-Address` exists so a client (and the planned service worker,
`features/delivery/content-addressing.feature` @browser-only scenarios) can cache by
content identity and revalidate across re-seeds. A client that follows the slug arm's
own answer to the CID URL receives a DIFFERENT identity for the same bytes, so
address-keyed caching can never converge: every resource has two names depending on
which door you came through. This is the two-serve-paths shape again (cf. the SSR
"page-borne wiring goes on BOTH serve paths" lesson): whatever the canonical address
plane is — bundle CID or per-file digest — both arms must stamp the SAME one, and the
CID arm should carry `X-Content-Slug` when the reverse mapping is known.

## Regression seatbelt

`features/delivery/content-addressing.feature` "CID URL serves same content without
slug lookup" asserts the captured slug-arm address round-trips through the CID arm.
It is red for exactly this divergence (wave3-full 2026-08-22) and turns green when
the arms agree; do not soften the step's `bafkrei…` pattern to paper over it —
deciding sha256-… as canonical instead is fine, but then BOTH arms and the step's
`CONTENT_ADDRESS_PATTERN` move together.
