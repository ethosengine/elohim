---
id: "backlog-heal-pointer-bytes-ordering-blocking-serve"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Heal converges the pointer before the bytes; serving then BLOCKS unboundedly in heal-on-read instead of degrading"
slug: "heal-pointer-bytes-ordering-blocking-serve"
written: "2026-07-02"
author: "shift-genesis-verdicts-green-followup"
shift_objective: "post-close live observation during the 7-cast scale-back deploy"
status: "backlog"
priority: "high"
themes: [dataplane, heal-on-read, blob-custody, serving, backpressure]
relatedNodeIds:
  - "genesis/data/timeline/backlog/dataplane-peer-fallback-and-blob-replication.md"
  - "genesis/data/timeline/backlog/cluster-to-shem-p2p-request-starvation-11-peer-blackout.md"
tags: [p2p, serving, spine:sync-scale-honesty]
---

# Heal pointer/bytes ordering + unbounded blocking serve

Observed LIVE 2026-07-02 ~19:2xZ on adam during the 7-cast scale-back deploy
(edge #1139), a complete specimen of two coupled gaps:

**Sequence observed:** (1) adam's rollout+newly-healthy conductor churn
reverted his `elohim-host-landing` row to `blobHash: null` (clobber source
unconfirmed — adam is Loki-blind, task #7; reinstall-projection and
reconcile-churn both fit); (2) within minutes the CRDT plane COUNTER-HEALED
the row to `blobHash: sha256-b23a…` `trust: published` — the self-heal loop
proven end-to-end in prod, unattended, twice in 24h; (3) BUT the healed
pointer references a NEWER bundle whose BYTES had not yet replicated to adam
— so `GET /` (apps resolver → get_blob_or_heal → race_fetch) **blocked past
30s per request with zero bytes** instead of serving a degraded answer.
/health stayed fast throughout; only blob-backed routes hung.

**Gap 1 — pointer/bytes ordering:** `heal_content_row` writes a blobHash
whose bytes may be absent locally, and nothing eagerly enqueues the blob
fetch at heal time — serving discovers the absence lazily, per-request.
Fix candidate: when the reverse-projection heal writes a blob_hash with no
local bytes, immediately enqueue the fetch (acquisition/gap_queue), making
pointer-heal imply bytes-heal.

**Gap 2 — unbounded blocking heal-on-read:** get_blob_or_heal holds the HTTP
request open while race_fetch runs (observed >30s). Under starvation or
byte-lag every landing request stacks up. Fix candidate: time-bound the
in-request heal (~2-5s), then return the EPR-head-aware syncing-status body
(the .epr-meta peer-fallback invariant's option (b)) with Retry-After —
degrade legibly, heal in background. Same class the substrate-verify
propagation check tripped on (jessica curl-000 while healthy, genesis #1232).

**Verification hook:** the existing federation-deploy + blob-replication
concerns cover the END state; a new scenario should pin the TRANSITION state:
"pointer healed, bytes lagging → root answers ≤5s with syncing status, then
converges to 200."
