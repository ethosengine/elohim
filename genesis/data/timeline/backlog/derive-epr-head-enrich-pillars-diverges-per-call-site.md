---
id: "backlog-derive-epr-head-enrich-pillars-diverges-per-call-site"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "derive_epr_head's two call sites deliberately disagree on enrich_pillars — the HTTP CBOR head and the P2P MessagePack head are different documents for one id"
slug: "derive-epr-head-enrich-pillars-diverges-per-call-site"
written: "2026-09-21"
author: "serving-edge failover-balance-stream campaign, 2026-09-21 review (design-pass follow-up F1)"
status: "open"
priority: "low"
jobs: [elohim]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "habit:doorway-failover"
tags: [epr-head, content-addressing, dag-cbor, p2p, design-follow-up]
---

**The fact.** `derive_epr_head` (`elohim/elohim-storage/src/epr_head.rs:76-91`) takes an `enrich_pillars: bool`
that, when true, "issue[s] extra queries to populate `shefa` stewardship data and `qahal` attestation
requirements" (`:74-75`). Its two call sites pass opposite values, each deliberately: the HTTP dag-cbor arm of
`GET /epr-head/<id>` (`http.rs:13128`) calls it `(gate_provenance=true, enrich_pillars=false)`, commented "No
pillar enrichment: shefa/qahal remain at their empty defaults for the public metadata surface... this arm encodes
the canonical EprHead directly" (`:13120-13126`). The P2P local-resolution arm
(`epr_service.rs::resolve_epr_head_locally`, `:307-314`) calls it `(gate_provenance=false, enrich_pillars=true)`,
commented "Pillar enrichment ON: peers receive full stewardship + attestation context" (`:311-313`).

**Evidence.** Both choices are independently correct for their own consumer — the public HTTP metadata surface
and the internal peer-resolution path have different disclosure needs. The divergence was named, not discovered
as a bug, by the 2026-09-21 EPR-head-envelope design pass:
`genesis/a2o/reports/recovery/serving-edge-20260920/epr-head-envelope-design.md` §"A second, un-named divergence"
(around line 60) and its follow-up F1 (§7, ~line 661): "`derive_epr_head`'s two call sites produce two documents
for one id." Latent today because the P2P arm ships MessagePack and mints no CID (design doc, C7 row, ~line 624)
— so no consumer yet re-encodes the P2P-side answer to dag-cbor and compares it against the HTTP-side CID.

**Why it matters.** The moment any consumer treats the P2P-resolved head and the HTTP-served head as
interchangeable representations of "the EPR head for id X" — e.g., a future receiver that re-encodes a gossiped
P2P head to dag-cbor to compute its address — it will get a different address than the origin's own HTTP
`GET /epr-head/<id>` would advertise for the same id, because the two documents carry different field sets. This
is the same swap-test question the doorway-failover invariant already asks of doorways; here it is asked of the
two internal call sites of one function.

**Smallest next step.** Decide, before any consumer starts comparing these documents, whether `EprHead` should
have one canonical enrichment level (and a separate, explicitly-partial "public metadata" projection), or whether
the two documents are legitimately different types that should stop sharing one function name. p2p-design-gate
question: is enrichment level part of content-derived identity, or purely a display/authorization concern layered
on top of one canonical document. Full agenda: design doc §7 F1.

**Links.** Design doc: `genesis/a2o/reports/recovery/serving-edge-20260920/epr-head-envelope-design.md` (F1, §7;
C7 row). Sibling finding from the same reading: `epr-head-envelope-differs-per-peer.md`. Habit:
`doorway/doorway-service/.epr-meta/doorway-failover.habit.md`.
