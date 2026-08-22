---
id: "backlog-collective-participations-reconcile-and-scope"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "collective_participations never heal cross-peer (no reconcile/federation arm) and are written/read under mismatched AppContexts (qahal vs lamad)"
slug: "collective-participations-reconcile-and-scope"
written: "2026-08-22"
author: "doorway-breaker triage (CollectiveCommitted/MembershipCommitted msgpack decode fix follow-ups)"
status: "open"
priority: "medium"
tags: [dataplane, qahal, collectives, projection-reconcile, view-federation, app-context, bounded-code-fix]
---

# Participation rows: two queued follow-ups from the collective-signal triage

The msgpack decode cure (HoloHashB64-typed `ImagodeiSignal` mirror in
`elohim/elohim-storage/src/signals.rs` + direct typed decode in
`src/reconcile/holochain_app_signal.rs`) makes `CollectiveCommitted` /
`MembershipCommitted` project locally again. The triage exposed two adjacent
defects it deliberately did NOT fix:

## (a) No participations arm in projection_reconcile / view_federation

`projection_reconcile` and `view_federation` carry a **collectives** arm but
NO **collective_participations** arm. A participation row authored by a peer's
`post_commit` projection exists only on that peer — it never heals cross-peer.
Concretely: the triad assertions against `:8090` need jessica's and james's
participation rows visible from matthew's storage, and today they can never
arrive. Close by adding the participations arm to both surfaces (same
`dht_anchor_hash` idempotency key the local projector uses).

## (b) AppContext scope mismatch: written "qahal", read "lamad"

Account-import writes participations under AppContext `"qahal"`
(`elohim/elohim-storage/src/http.rs:11191`) while the legacy
`GET /db/collectives/{id}/participants` route reads ctx `"lamad"` — the same
class as the HUMANS_HAPP_ID drift (`elohim/elohim-storage/src/db/context.rs:6-21`).
Rows written by one surface are invisible to the other. Pick the canonical
context (and migrate or dual-read the stragglers) rather than patching one
route.

## Watch item

When `ALLOW_SEED_SHARD_MANIFEST`'s 403 is lifted: if the costeward leg derives
consent from participants, it lands on this surface — both (a) and (b) become
its preconditions.
