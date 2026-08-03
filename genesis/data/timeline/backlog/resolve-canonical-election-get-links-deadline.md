---
id: "backlog-resolve-canonical-election-get-links-deadline"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Obey-path starvation: conductor DB-pool saturation on the election read (C11) + dominant exit unadjudicated until the probe counter deploys — NOT a B2/GetStrategy recurrence (disproven)"
slug: "resolve-canonical-election-get-links-deadline"
written: "2026-08-03"
author: "pipeline-landing shift (integrator; diagnosis corrected same shift by code-disproof)"
status: "backlog"
priority: "high"
tags: [dataplane, holochain, conductor, db-pool-saturation, backpressure, obey-path, convergence, concern-c11, concern-c8, misdiagnosis-corrected]
cites:
  - elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
  - elohim/elohim-storage/src/services/head_adoption.rs
  - genesis/data/timeline/backlog/content-gap-limit-cycle-blocks-convergence.md
  - genesis/data/timeline/backlog/susan-conductor-ws-dead-heal-pacing-blind-to-instant-errors.md
---

# Obey-path starvation — corrected diagnosis

## What was disproven (kept for the record — the misdiagnosis is the lesson)

First reading (this shift, ~03:30Z): "B2 recurrence — resolve_canonical_election
ships an uncured Network-strategy gather." **Disproven by primary sources**:
`git show da8975176` proves the election read has passed `GetStrategy::Local`
since birth; `holochain_cascade-0.6.0/src/lib.rs:789` proves a Local gather
never touches the network. The `Host("deadline has elapsed")` string is
`tokio::time::error::Elapsed` from `acquire_semaphore_permit`
(`holochain_sqlite db/access.rs:582`, 10s ACQUIRE_TIMEOUT) — the conductor's
DB read pool (4 permits on small pods; `cascading()` holds one from EACH of
cache/DHT/authored across the blocking query) is **saturated**. C11
backpressure, retryable. The arithmetic seconded the disproof: 34 errors in
79min ≈ 3% of ~900 probes/hr — resolve_error cannot explain a 0% obey rate;
~97% of probes exit through a different arm.

Instance of [[feedback_verify_the_measure_before_the_ranking]]: the ranking
(B2-class, forecast row) was written before the measure (the probe counter)
existed. The corrected forecast row carries C11.

## What is true and open

1. **The dominant obey exit is unadjudicated** until
   `elohim_content_election_obey_probe_total{outcome}` (shipped this shift)
   deploys. Read it FIRST: `no_election` dominant ⇒ link-gossip/visibility
   wall; `no_courier` dominant ⇒ hint/carried-record supply (susan's
   saturation poisons this fleet-wide — cross-cited); `resolve_error` ⇒ the
   ~3% DB-saturation class only.
2. **DB-timeout handling**: head_adoption currently drops the row
   (`return None`) on the resolve Err — treat as retryable backpressure
   (C11's declared-policy clause: defer-with-retry, counted) instead.
3. **Zome-call load doubling**: the obey probe is an additive zome call per
   row on a sweep that already calls resolve_content_head_local for the same
   rows — wave 4 roughly doubled conductor zome-call load. Structural lever:
   fold the election answer into the head answer the sweep already pays for,
   or pace the probe.
4. **Pod CPU**: `num_read_threads()` floors at 4 — more CPU widens the permit
   pool directly (operator/manifest lever).
5. **declare-path Network reads** (gather_content_chain + target get in
   declare_canonical_head_inner): genuine sweep-hot-path Network reads,
   deliberately NOT flipped — the first is the phantom-id gate (a cold local
   view would reject legitimate declares), the second's miss is the documented
   entry to the carried-record branch. A read-vs-write-path tension needing
   its own decision, not a silent flip.
6. **Not-unit-pinnable**: the zome suite cannot observe get_links strategy
   (host-fn boundary); the regression home is a sweettest (conductor holds
   links but not the winner's target) — recorded in the zome seam-registry
   gapNote.

The zome-side edits that shipped from the disproof (error text naming the
DB-saturation class; honest doc replacing the false "must never stall" claim;
seam-registry C6a answered-with-accurate-reasoning) are legibility-only and
ride the next coordinator swap — they do not move the obey rate.

Status: open, unowned. First action: read the probe histogram post-deploy.
