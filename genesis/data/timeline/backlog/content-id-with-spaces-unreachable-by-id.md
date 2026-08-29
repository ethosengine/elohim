---
id: "backlog-content-id-with-spaces-unreachable-by-id"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "2,306 of 3,015 seeded blob-bearing rows have SPACES in their ids; storage's GET /db/content/{id} never percent-decodes the path segment, so those rows are unreachable by id on every peer (author included) — and the recovery harness's P1 leg reads them as 'absent', masking the transport measurement"
slug: "content-id-with-spaces-unreachable-by-id"
written: "2026-08-29"
author: "M4 quiesce delta"
status: "open"
priority: "high"
jobs: [elohim-edge, elohim-genesis]
cluster: "arch-dataplane-refactor-backlog"
tags: [storage, http-route, percent-decoding, seed, content-id, recovery-harness]
---

Measured 2026-08-29 05:1xZ on the household mesh (3.4k corpus): `GET /db/content/scenario-value-scanner-…%20mental%20load…`
→ 404 `Content not found: …%20…` on matthew, jessica and james, while `/db/content?limit=500` lists the row on all
three (jessica holds all 3,454). Three defects in one shape:
1. **storage**: the `/db/content/{id}` handler matches the raw encoded segment; decode it (and re-encode nothing).
2. **harness**: `hc-mesh-recovery.sh`'s P1/P2 legs build `http://…/db/content/{id}` unquoted → `InvalidURL` → "absent";
   quote ids (`urllib.parse.quote`) so the leg measures replication, not URL grammar. Until then the cold-recovery
   number on the seeded corpus is NOT comparable across arms (the "absent" floor is the space-id count of that
   run's snapshot: 1,287 vs 2,188).
3. **seed**: content ids are slugs; a slug with spaces should be refused or normalised at seed time.
