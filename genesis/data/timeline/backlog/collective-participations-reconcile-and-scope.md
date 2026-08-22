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

## (c) Same class, third instance: canonical `humans` rows never heal cross-peer (household-vocabulary split)

The saga ch10 card-tells-truth divergence (2026-08-22: doorway A said
`stewardingCollectives: 1`, doorway B said 2 for the SAME `elohim-host-landing`
custody facts) was this class wearing the resilience card. The custody plane
AGREED — both peers' `shard_manifests` + `shard_locations` held both holder
agents — but the slug-keyed fixture `humans` rows (`human-matthew-manager` →
`household-dowell`, seeded via doorway A) exist only on matthew: there is no
humans/membership reconcile arm, so jessica/james only ever get the
`identity_fill` CREATE fallback (`id = agent:{pubkey}`,
`household_id = collective:{action_hash}` verbatim). One physical household then
splits into two id vocabularies inside a single peer's fold.

**Read-side cure landed** (bounded, same branch):
`services/household_resilience.rs` now canonicalizes cid-form
`humans.household_id` values through the local `collectives.collective_cid`
alias (which the collectives arm DOES replicate — both peers already held
`household-dowell ↔ collective:uhCkkoQQ…`) in the holder relation, the
replication-commitment relation, and the commitment-backed-collectives count.
Counts now agree; the fold serves only local truth (verify-locally-then-serve
intact).

**Still open in this class:** the canonical slug `humans` rows themselves
(display names, and any peer whose `collectives` projection lacks the
`collective_cid` alias) never converge cross-peer — jessica renders label
`household-dowell` where matthew renders `Dowell Household`, and her placeholder
collectives row carries no region (regional-distribution buckets still
diverge). Closing that is the humans/participations reconcile arm this file
tracks — one arm shape, three projections (participations, humans, collectives
metadata refresh).
