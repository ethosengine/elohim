---
id: "backlog-delete-fetch-from-remote-doorway"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Doctrine call made — DELETE fetch_from_remote_doorway and the FEDERATION.md claims that describe it as DeliveryRelay's final tier"
slug: "delete-fetch-from-remote-doorway"
written: "2026-08-23"
author: "fable-5 session 2026-08-23 (roadmap Lane A4; doctrine decided in the roadmap plan)"
status: "refined"
priority: "medium"
area: "doorway/federation"
domain: "protocol"
jobs: [elohim-edge]
relatedNodeIds:
  - "habit:doorway-failover"
cites:
  - genesis/docs/superpowers/plans/2026-08-23-doorway-federated-continuity-roadmap.md
  - genesis/docs/superpowers/plans/2026-07-31-doorway-federation-failover-sprint-plan.md
tags: [doorway, federation, dead-code, bounded-code-fix, codex-claimable, agent-agnostic]
---

# Delete `fetch_from_remote_doorway`

**The doctrine call (2026-08-23, roadmap plan Lane A4).** Sprint-plan Task 3.3 asked
"wire or delete". Decided: **delete.** A blob read already has two selection tiers
that are the right layers: the doorway picks a serving steward peer at selection
time (`select_route`, 2026-08-23), and the storage peer it picked race-fetches from
any custody holder over the p2p dataplane (`get_blob_or_heal` →
`race_fetch_with_swarm`). A doorway→doorway HTTP hop after a local 404 is a third
tier at the wrong layer (federation duplicating the dataplane's own fallthrough),
and today it is dead code with a lying doc comment — the worst state.

## Scope (doorway-service only; claim AFTER the 2026-08-23 Lane 0 batch is on dev)

1. Remove `fetch_from_remote_doorway` from `doorway/doorway-service/src/services/federation.rs`
   (~line 364-420) and any now-unused helpers/imports it alone used (loop-prevention header
   constant included, if nothing else reads it — grep first).
2. Remove the matching claims from `doorway/doorway-service/FEDERATION.md` ("final fallback tier
   when local storage returns 404"); leave the SPEC-DRIFT banner's other lines intact, and
   record the doctrine in one sentence where the claim used to be, citing this entry's slug.
3. `cache/delivery_relay.rs` must not reference it (it does not today — assert with grep).

## DoD / verification
- `just gate doorway` → `GATE_EXIT=0` echoed on its own line.
- `grep -rn fetch_from_remote_doorway doorway/` → no hits.
- No behavior change: no test deleted except any that only exercised the dead function.

## Disjointness
`services/federation.rs` + `FEDERATION.md` only. Do not touch `server/http.rs`, `routes/storage_proxy.rs`,
`routes/seed.rs`, or `services/route_registry.rs`.
