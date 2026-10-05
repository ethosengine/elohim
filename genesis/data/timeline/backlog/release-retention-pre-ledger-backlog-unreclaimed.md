---
id: "backlog-release-retention-pre-ledger-backlog-unreclaimed"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Release retention cannot reclaim the releases a peer held before its ledger existed"
slug: "release-retention-pre-ledger-backlog-unreclaimed"
written: "2026-10-05"
author: "claude-opus-5-5 (shift conductor-roll-wasm-retention; operator note)"
status: "open"
priority: "medium"
jobs: [elohim-edge]
cluster: "storage-footprint"
relatedNodeIds:
  - "habit:release-retention"
tags: [storage, retention, backfill, operator-decision]
---

Release retention (`5f41d33a1`) acts only on releases its ledger names, and the ledger starts
empty at deploy. The releases matthew accumulated before it (19 between 2026-09-21 and
2026-10-03, about 449 MB in the blob store plus a near-equal iroh copy) carry no record of which
release they belonged to, so the pass never touches them. The extraction cache (~1.1 GB on
matthew) is separate and is cleared at first boot on that build.

Two ways to reclaim the backlog; the operator has not chosen:

1. **Backfill the ledger.** An admin-keyed storage route accepts pre-ledger rows; the ordinary
   pass then releases them with every protection intact. Sources for the rows, most trusted
   first: the app pipeline's build logs (`scripts/ci/publish-app-release.sh` prints each
   release's four bundle sha256s, in build order); superseded custody-commitment rows on the
   peer (unconfirmed on the fleet, none on the household); a reachability sweep (avoid: the
   quiet peers hold ~40 unidentified shards each). Estimated half a day for the route plus the
   build-log reader.
2. **Clear storage.** The operator noted on 2026-10-05 that the backlog may have to be reclaimed
   by clearing storage instead. This is destructive and operator-owned.

Sequence either after the first fleet read of the retention gauges, not in the same push. If
custody rotation is not superseding old pledges on the fleet, a backfill frees little; the
`elohim_release_retention_blobs_held{reason="pledged"}` gauge shows that directly. Adam's rise
(~190 MB, 2026-09-21 to 2026-09-24) is uninvestigated.
