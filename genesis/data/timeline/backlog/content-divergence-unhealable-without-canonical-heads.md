---
id: "backlog-content-divergence-unhealable-without-canonical-heads"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Content divergence is structurally un-healable without canonical heads: heal fills-never-moves by design, no Declare ever runs over the content corpus, and the ch06 divergentAnchor gate thresholds a rotating-window sample; identity-fill reads 0 because alpha households were never formed per-member"
slug: "content-divergence-unhealable-without-canonical-heads"
written: "2026-07-27"
author: "claude (resiliency-saga orchestration — opus RCA on heal-never-moves)"
status: "open"
priority: "high"
ci_status: blocked
jobs: [elohim-edge]
tags: [projection-reconcile, divergent-anchor, canonical-head, declare, heal-fills-never-moves, identity-fill, household-formation, collectives, ch02, ch06, gate-spec, windowed-sample]
cites:
  - elohim/elohim-storage/src/p2p/projection_reconcile.rs
  - elohim/elohim-storage/src/db/content_diesel.rs
  - elohim/elohim-storage/src/services/identity_fill.rs
  - elohim/holochain/dna/imagodei/zomes/imagodei/src/qahal_coordinator.rs
  - self-heal-adam-projection-catchup-exhaustion-full-arc | adam post-restart catch-up exhaustion | path: genesis/data/timeline/backlog/self-heal-adam-projection-catchup-exhaustion-full-arc.md
  - turn-relay-pod-cidr-carveout-port-pool-shem-leg | TURN relay pair triage | path: genesis/data/timeline/backlog/turn-relay-pod-cidr-carveout-port-pool-shem-leg.md
---

# Divergent anchors cannot drain; identity-fill cannot discover — both saga reds need fabric-level acts, not storage code

RCA run 2026-07-27 (post relay-fix, transport healed) on why edge Dataplane
Validation stays red on `divergentAnchor <= 100` (ch06) and
`elohim_identity_fill_discovered_cids >= 1` (ch02).

## Finding 1 — the divergent class is structurally un-healable by heal (working as designed)

- A "divergent anchor" (`classify_content_gap`,
  `projection_reconcile.rs:1451-1467`) = row present + locally anchored + some
  peer advertises a different non-empty `dht_anchor_hash`. Only
  `StampOutcome::Stamped` converges it, and the only writer that may MOVE a
  declared head is `StampMode::Declare` — i.e. `POST
  /db/content/{id}/canonical-head` (`http.rs:12886`). Heal fills-never-moves
  (`content_diesel.rs:975-1002`).
- On alpha that route is exercised ONLY by CI for the EPR/SPA bundle slugs
  (`scripts/ci/stage-spa-blob.sh:118-131`, `Jenkinsfile authorHeadOnce`).
  The ~4,300-row content corpus has NO canonical heads: fleet-wide 3h heal
  outcomes show `refused_stale` = 0 everywhere (the provably-newer
  forward-move path is never reached), matthew = 84% `Refreshed` ("own
  conductor answered the head this row already holds") + 513
  `refused_declared` (no canonical-head link found).
- Consequence: divergence RISES with healthy transport (matthew 657 → 1,416
  in 2h post relay-fix) because more peers can advertise disagreement.
  **It will never drain on its own.**
- `content_missing: 0` alongside high divergence is the signature of
  "conductor healthy, cross-peer disagreement" — full-arc conductors always
  answer locally.

## Finding 2 — the ch06 gate thresholds a windowed sample, not a queue

`divergent_anchor` is a per-sweep sample over a rotating inventory window
(`sweep_offset` / `window.advance`, `projection_reconcile.rs:1493,1546,1605`).
The same unchanged pod legitimately reads 0 one hour and 3,500 the next
(susan: 1999, 602, 0, 0, 0, 3152, …). `divergentAnchor <= 100` will flap
regardless of real convergence. Re-spec candidates: a monotone backlog
measure (new instrumentation) or `elohim_projection_reconcile_converged`.
Gate-spec change is a saga-owner decision.

## Finding 3 — identity-fill discovers 0 because no household was ever formed per-member

`elohim_identity_fill_discovered_cids` has one write site
(`identity_fill.rs:302`); both discovery legs return `Ok(empty)` — no error
lines in 3h:

- Leg A (projection): every alpha family-collective row is a 2026-05-28
  seeder row with NULL `collective_cid`
  (`holochain_humans_replayer.rs:239` filters them out); zero
  `CollectiveCommitted` / `MembershipCommitted` signals in 24h — nothing was
  ever notarized.
- Leg B (own source chain): only jessica's conductor authored Memberships
  (9 households, 226 members; metric reads 9 there, 0 on all six others).
  Formation was single-conductor.
- Residual uncertainty: `collective_cid_is_household`
  (`qahal_coordinator.rs:425-440`) does a bare local `get()` and returns
  false on miss WITH NO LOG — a silent swallow that available evidence
  cannot distinguish from empty-chain. Cheap discriminator: call imagodei
  `get_my_household_collective_cids` on matthew's cell — `[]` ⇒ seeding gap;
  non-empty ⇒ zome fix (tracing + `GetOptions::network()`, coordinator-only,
  ships via hot-swap, no DNA move).

## Finding 4 — collectives has no reconcile arm (architectural gap)

`ProjectionInventory` serves only `rea_commitments` and `content`
(`projection_reconcile.rs:36-38,70-72`). The collectives projection has
exactly the "post_commit fires only on the authoring conductor" defect the
content arm was built to cure — jessica holds the rows, nobody else does,
nothing reconciles them.

## The levers (all fabric-level; storage code is not the blocker)

1. **Bounded canonical-head declaration sweep** (closes Finding 1): iterate
   diverging content ids against the existing
   `POST /db/content/{id}/canonical-head` route from the authoring steward's
   conductor (auth is currently god-mode open,
   `content_store/src/lib.rs:3053-3055`). First cut: matthew's ~513
   `refused_declared` ids; validate heal outcomes flip
   Refreshed→Stamped/healed, then decide the corpus-wide sweep. MUST be
   paced — eve/susan/gertrude are wedged (100% `timeout_exhausted`, 0 rows
   healed in 3h) and adam is degraded (244 missing + 97 timeouts / 3h).
   **Operator decision: scope + pacing + where it runs (CI step vs one-off).**
2. **Per-member household formation on alpha** (closes Finding 3): re-run
   formation so each member authors its own Membership on its own conductor
   and `CollectiveCommitted` stamps `collective_cid` non-NULL. No code, no
   restart, no DNA move. **Operator-scheduled seeding/ceremony.**
3. **Wedged conductors** eve/susan/gertrude (and adam's degradation):
   restart/memory-headroom is a cluster act. Until then they inject
   divergent advertisements they can never heal.
4. **Gate re-spec** (Finding 2) — saga-owner decision.
5. **Collectives reconcile arm** (Finding 4) — sprint-sized storage work;
   design should reuse the content arm's inventory/stamp machinery.

## Falsifiable predictions on record

- Declare one currently-divergent id → within ~1-2 sweeps after link gossip,
  every peer logs `HEALED content anchor` (not "head unchanged") and the id
  leaves the divergent sample. If `refused_stale` instead: the
  ordering-proof gate is the blocker, not the canonical link.
- `get_my_household_collective_cids` on matthew returns `[]` ⇒ seeding gap
  confirmed; non-empty ⇒ the silent-swallow zome fix is the cure.
