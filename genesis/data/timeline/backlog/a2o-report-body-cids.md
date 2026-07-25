---
id: "backlog-a2o-report-body-cids"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "epr flow fulfill computes a scenario's resource CID from the CURRENT working-tree file, not from what the report actually tested"
slug: "a2o-report-body-cids"
written: "2026-07-25"
author: "claude (saga-status.py implementation session)"
status: "envisioned"
priority: "low"
relatedNodeIds:
  - "elohim/eprfs/epr-cli/src/flow/fulfill.rs"
  - "genesis/a2o/schemas/sprint-report.schema.json"
  - "genesis/a2o/scripts/lib/aggregate.ts"
tags: [eprfs, a2o, provenance, cid, sprint-report]
---

`fulfill.rs::fulfill` resolves a fulfilling/dismissing event's `resource` via `body_cid_of_file(&
root.join(commit_path))` — i.e. it reads the feature file OFF DISK at fulfill-time, not from
anything the sprint-report itself captured. In CI this is normally fine (the report is fulfilled
against the same checkout that produced it), but it means the REA event's resource CID is
provenance-fragile: if `epr flow fulfill` ever runs against a DIFFERENT commit than the one the
report's cucumber run actually executed (a delayed/batched fulfill step, a report replayed against
a newer branch, a rebase between test-run and fulfill-run), the minted event's `resource` silently
attests to the CURRENT file content, not the tested one — the fulfillment's provenance claim
("this exact scenario body went green") would be wrong without any error surfacing.

`genesis/a2o/schemas/sprint-report.schema.json`'s `byConcern[].scenarios[]` currently only carries
`{name, status, surface}` — a path string, no content hash. Candidate follow-up: have the a2o
aggregator (`genesis/a2o/scripts/lib/aggregate.ts`) stamp each scenario's feature-file body CID
into the report at generation time (the report already has the file in hand at that point), so
`fulfill.rs` can assert `resource == report's stamped cid` (or at minimum warn on mismatch) instead
of trusting the working tree unconditionally. Low priority — no observed incident, just a
provenance gap noticed while tracing `fulfill.rs`'s resource derivation for `saga-status.py`'s
dismiss-association logic.
