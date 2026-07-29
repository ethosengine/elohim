---
id: "backlog-declare-sweep-hash-only-cannot-converge-missing-action"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Bulk declare sweep declares hash-only (carried_record None) — can never converge a peer that lacks the action; wire it to /head-record"
slug: "declare-sweep-hash-only-cannot-converge-missing-action"
written: "2026-07-29"
author: "resilience-cards-converge sprint"
status: "open"
priority: "medium"
tags: [dataplane, canonical-heads, content-store, declare, saga-06]
cites:
  - genesis/data/timeline/backlog/content-divergence-unhealable-without-canonical-heads.md
  - genesis/data/timeline/backlog/declare-route-sheds-harder-plus-no-chain-gate-leg2-findings.md
---

# The internal bulk declare sweep is hash-only, so it converges nothing a peer doesn't already hold

Observed 2026-07-29 (adam + susan, ongoing): `declare_canonical_head`
errors in two flavors on scenario-* ids —

- `lib.rs:3362 Guest("declare_canonical_head: no content found for id …")`
- `lib.rs:3384 Guest("declare_canonical_head: target action ActionHash(…)
  is not retrievable")`

Reading `declare_canonical_head_inner`
(elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs): the
`not retrievable` arm fires ONLY when `carried_record` is `None` — i.e.
the internal sweep declares hash-only, while the sprint-3
carried-record path (declare-carries-Record) exists precisely for the
peer-lacks-action case. On a full-arc fleet a local get miss is terminal,
so a hash-only declare against a missing action can never succeed, ever —
it just burns zome calls and error volume (the same class blamed on
"adam's exhaustion" overnight 07-27/28).

**Smallest change:** the sweep's caller fetches
`{authoring-doorway}/db/content/{id}/head-record` (reads without a key —
verified 200/3306 bytes on doorway-A) and passes `record` through,
mirroring scripts/ci/stage-spa-blob.sh DECLARE_ONLY. Converge on evidence,
not on gossip luck.

Status: OPEN (bounded storage/zome-caller change; native-side caller, no
DNA hash move if confined to the sweep's storage-side caller).
