---
id: "backlog-content-id-with-spaces-unreachable-by-id"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "2,306 of 3,015 seeded blob-bearing rows have SPACES in their ids; storage's GET /db/content/{id} never percent-decodes the path segment, so those rows are unreachable by id on every peer (author included) — and the recovery harness's P1 leg reads them as 'absent', masking the transport measurement"
slug: "content-id-with-spaces-unreachable-by-id"
written: "2026-08-29"
author: "M4 quiesce delta"
status: "wip"
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

## 2026-08-29 defects 1 + 2 cured (local evidence); defect 3 deliberately NOT taken

1. **storage**: `decode_path_id` percent-decodes at the LEAF of every id-bearing arm (`/db/content/{id}`, its
   `/schedule` · `/head` · `/head-record` · `/canonical-head` suffixes, `/db/allocations/content/{id}`) — after the
   suffix match, so an encoded `/` cannot re-route; lossy-UTF-8, never a refusal. 4 unit tests incl. the measured
   value-scanner shape.
2. **harness**: `hc-mesh-recovery.sh` P1 now `urllib.parse.quote(id, safe='')` — the leg measures replication, not
   URL grammar. The next cold-recovery run on the full corpus is the first comparable number.
3. **seed**: measured 2,671 of 3,460 ids in `genesis/data/lamad/content/**` carry spaces (the value-scanner
   generator mints `scenario-value-scanner-<…>-<title with spaces>`). Content ids are identity — relationships and
   paths reference them — so normalising is a content MIGRATION (re-mint + rewrite every referrer + reseed the fleet),
   not a bug fix. Left open on purpose; the storage cure makes the rows reachable as they are.

## 2026-08-29 MEASURED on the household mesh (dual, 3-peer, full corpus seeded)

`just mesh recovery warm jessica --label cut=ratchet-2026-08-29 --label ids=quoted-decoded` → **PASS, 439 s**
(record in `genesis/a2o/reports/recovery/recovery-timeline.jsonl`, `failing_legs: []`). P1 drained honestly:
absent 8 → 7 → 6 → 1 → 0, and every straggler was a NON-space id (`fct-bible-2-kings-23-10` last) — the space-id
absent floor is gone. Direct probe on the recovered peer: `GET /db/content/<percent-encoded space id>` → 200
(54 of the 56 anonymously-listed blob-bearing rows on the survivor carry spaces). The number is the first P1 on the
full corpus that measures replication rather than URL grammar; it is a fresh-seed dual arm, so not comparable to
the 258 s homo-iroh warm PASS of 2026-08-28 — compare within arms from here on. (Seeder note: `just seed apply mesh
content` exits 1 on its conductor post-flight — "1/3433 entries written" — while the doorway→storage import
reports 3433 inserted / 8920 relationships; the local seed never DHT-anchors, so read storage, not the seeder's rc.)
Defect 3 (seed slug normalisation) stays open.

## 2026-08-29 same class, second route: `/sync/v1/{hAppId}/docs/{docId}` does not percent-decode `docId`

`GET /sync/v1/elohim/docs/node%3Ascenario-…%20…` → `{"error":"Document not found: node:scenario-…%20…"}` — the
`%20` reaches the DocStore lookup verbatim, so no space-id doc can be read over HTTP (the heads route works for
space-free ids). Same cure as defect 1: `decode_path_id` at the leaf. Not taken in the shape-3 cut; it blocked an
investigation read, not a dataplane path.
