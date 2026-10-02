---
id: "backlog-household-love-map-unreadable-by-its-couple"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Matthew and Jessica cannot read their own love map on the household: the intimate path has no steward recorded, so no one passes the intimate reach check"
slug: "household-love-map-unreadable-by-its-couple"
written: "2026-09-27"
author: "FCT v2 flight on the household mesh (steward: human:matthew)"
status: "open"
priority: "medium"
area: "elohim-storage/reach, genesis seed fixtures"
domain: "protocol"
jobs: [elohim-genesis]
relatedNodeIds:
  - "habit:reach-enforced-everywhere"
  - "backlog-security-content-head-route-bypasses-reach-gate"
tags: [reach, intimate-reach, love-map, household, fixtures, trustful-declared]
---
## Observed (household mesh, 2026-09-27)
`love-map-matthew-jessica` is seeded at intimate reach with `created_by = NULL`. After the read
routes gained a reach gate, anonymous readers are refused (correct). But reading as either member of
the couple also fails: `X-Agent-Cid: human-matthew-manager` or `human-jessica-spouse` answers
403 "No mutual intimate relationship with content steward", or "Reach authorization required" on a
peer where that human is not resolvable. With no steward on the row, there is no one to hold the
intimate relationship with.

## Why it matters
A love map exists to be read by the two people it maps. The reach gate now does its job against
everyone else; the fixture has to let the couple through, or the intimate tier has no positive path
anywhere on the household and every story that needs one can only prove refusals.

## Fix direction
Seed the love maps with their stewards (and the couple's consented intimate relationship) so the
positive path exists, then add the positive case to the reach-gate route tests (they currently use
`self` reach for the steward case because no mesh human can satisfy intimate).
