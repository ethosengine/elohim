---
id: "backlog-security-content-head-route-bypasses-reach-gate"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "GET /db/content/{id}/head answers anonymously for intimate content that GET /db/content/{id} refuses — it reveals the row exists, its anchor, blobHash and updatedAt"
slug: "security-content-head-route-bypasses-reach-gate"
written: "2026-09-27"
author: "session 2026-09-27 FCT v2 recomposition (steward: human:matthew); found by the steward-publish tool's verification"
status: "partial"
priority: "high"
area: "elohim-storage/http"
domain: "protocol"
jobs: [elohim-holochain]
relatedNodeIds:
  - "habit:reach-enforced-everywhere"
  - "backlog-security-doorway-blob-pantry-ungated"
tags: [security, reach, privacy, intimate-reach, storage-http]
---
## Observed (household mesh, 2026-09-27)
`love-map-matthew-jessica` is a path at intimate reach: Matthew and Jessica's private love map.
Anonymous `GET /db/content/love-map-matthew-jessica` returns **403** on all three household peers,
as it should. Anonymous `GET /db/content/love-map-matthew-jessica/head` returns **200**, with the
row's existence, its DHT anchor, `blobHash` and `updatedAt`.

## Why it matters
Existence and timing are content. An intimate row's head says that a household keeps a love map
and when they last touched it; a `blobHash` is a stable identifier to correlate across peers. The
reach gate has to cover every route that reads a row, not only the one that returns its body.

## Fix direction
Apply the same reach check the body route applies to the head route (and audit sibling
`/db/content/{id}/*` read routes for the same gap). Add a reach-enforcement test that sweeps every
read route for an intimate fixture row, so a new route can't be born ungated.

## Sibling: relationship read routes (2026-09-27)
`GET /db/relationships?contentId=…` and the graph route apply no reach check at all, so the edges
of an intimate atom (a love map's structure) are readable anonymously. They must apply the source
atom's reach. Found by the P2P design gate for carrying edges inside the signed atom
(`lamad-teacher-authoring-backlog.md`, F16 design).

## Fixed 2026-09-27 (household-proven, uncommitted at time of writing)
`elohim-storage/src/api/content_reach_gate.rs` applies the body route's reach checks once at the
`/db` dispatch to `content/{id}`, `/head`, `/head-record`, `/schedule`, `relationships?contentId=`,
`relationships/graph/{id}` and `relationships/{relId}` (gated by the edge's source atom).
`tests/api/content_read_reach_gate.rs` builds its route list from `http::build_manifest()`, so a new
read route is swept automatically. Household: the love map answers anonymous 403 on all three peers
for every one of those routes (all were 200 before); commons rows unchanged.

## Still open
- Unfiltered `GET /db/relationships` (no `contentId`) still lists intimate atoms' edges.
- `relationships/graph/{commonsId}` can walk into an intimate neighbour.
- The body route's peer-fetch fallback (no local row) still serves without a reach check.
- The doorway manifest has no `public_if_reach` on `/head` and `/schedule` (storage is the authority; left as is).
