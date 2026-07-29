---
title: "Deliver-the-saga overnight shift — sprint result (2026-07-29)"
id: deliver-the-saga-overnight-sprint-result
tier: sprint-result
status: Closed
created: 2026-07-29
maintainers: Matthew Dowell + Claude Fable 5
sprint: resiliency-saga
topic: [resiliency-saga, household-formation, identity-fill, collectives-bootstrap, shem-hairpin, canonical-heads]
---

# Deliver-the-saga — overnight shift result

**Objective:** saga green-chapter count ≥ 7 (from 6), stability 2
consecutive with ≥1 fresh trigger. **Outcome at close: 6/10 measured, with
the complete ch02 delivery chain landed and shipping in wave 3** — the
morning's first edge Dataplane Validation after wave 3 is the deciding
measurement. Baseline floor (≥6) held throughout; no regression left behind.

## What the night landed (all on dev, pushed in 3 waves)

Day sprint (waves 0, 18 commits): ch04 sticky-regressed cure (epr-cli
regression re-commitment, later hardened with occurredAt ordering +
stale-recovery gate after adversarial review), ch06 station counters
de-@wip'd (PASSED on first CI run — new binary confirmed), identity_fill
120s timeout (jessica's permanent hang cured — confirmed live: her loop now
completes), james agencyPhase roster fix (his conductor identity now
exists — created in genesis #1383), hostAliases manifests (shem hairpin
bypass — B's conductor 0 → 28+ agents), gherkin pre-push lint, pnpm
pre-bake (storybook flake class killed), qahal batch-get.

Overnight (waves 1-3):

- **Formation converged run-over-run:** #1382 james unbindable → #1383
  bound + affirm attempted (steward-check gossip race named:
  qahal_coordinator.rs:497 issuer-stewardship validated LOCALLY on the
  member's conductor, full-arc local-only gets) → **#1384 2/3 AFFIRMED**
  (race winnable on calm fleet) → cure shipped (79e3cacfc: 6-step
  DHT-settle retry + founder-chain collective reuse).
- **The collectives bootstrap circle's last link:** james's identity_fill
  WORKED (created humans row from membership truth, 08:29Z) but nothing
  stamps the local collectives row's NULL cid → inventory stays empty →
  arm circular. Landed 91719540c + 251fbfcdd: identity-fill stamps the
  NULL collective_cid via a deterministic membership-truth join (matched
  seeded humans rows → single distinct slug; cid-form household_ids
  excluded; fill-only, 2240 tests green). First-pass cardinality guard
  was caught INERT against the real 20-family-row seed before shipping.
- **Sweettest fresh red resolved as flake:** content_visible_across_agents
  (#1376) — zero code-path overlap with the batch (proven), passed #1377
  and #1378; Harbor published.
- **B's island debt draining:** divergentAnchor 638 → 3117 peak →
  oscillating ~1200-2400 (windowed-scan flappiness, documented); caughtUp
  still false at close — the 503-shedding admission gate is working as
  designed while it drains.

## The morning verdict path (one read)

1. genesis #1386 (wave 3): "Results: N/3 affirmed" — cure makes 2/3+
   deterministic; watch the retry log lines fire (or not be needed).
2. Edge deploy ships the collectives stamp → within ~2 identity_fill
   cadences (≤10min post-churn): james's storage stamps household-dowell →
   `collectives_ids_discovered` goes non-zero fleet-wide (first time
   ever) → matthew's identity_fill leg-A discovers → ch02's two metric
   assertions green on alpha-A.
3. `elohim_identity_fill_collective_cid_stamped_total` ≥ 1 on james = the
   new fix's own confirmation counter.
4. ch10 needs ch02's joins PLUS B caughtUp — check B's /health first;
   the stewardingCollectives compare can only equalize non-zero after
   both.
5. ch06: heads still declared-divergent (A 08:56 vs B 10:30, different
   blobs). OPERATOR DECISION still open (which blob is intended); the
   carried-record declare toward the newest is the pre-authorized rail
   once B catches up. Ghost class: B cannot serve its own head-record
   until caught up (503).

## Ceilings for the operator (unchanged from handoff + one new)

- Head-direction decision for elohim-host-landing (A-old vs B-newer).
- matthew's captured-UUID chain migration (formation founder can then be
  matthew; 3/3 affirm becomes possible).
- NEW: agent-bindings stage failed 7/7 in EVERY genesis run tonight
  (60s call_zome timeouts — the app-port-4445-auth-timeout background
  class amplified; backlog'd). Formation survives without bindings, but
  the bindings stage needs its own triage pass.

## Watch-outs recorded

- humans.household_id carries TWO vocabularies: seeded rows = slug,
  live-created rows = raw collective cid (controller.rs:1085). The
  membership-join deliberately excludes cid-form values; a future
  normalization should unify the column's vocabulary.
- Each pre-cure formation run minted an orphan collective (at least 3 on
  the DHT: uhCkk2JLI…, uhCkkvxANH4…, #1384's). Founder-chain reuse stops
  the bleeding; existing orphans are inert but enumerable — cleanup is a
  qahal-governance decision, not a sweep.
- ci-observer validate-mode prompts must carry the project→pipeline
  mapping or they false-flag under_built.
- Jenkins wall clock ~1.5h behind session containers — never compare
  absolute timestamps across them.

## Wishlist / palette

No blockers; no shift-scoped settings entries were added (durable palette
sufficed under the operator's push grant).

## Close addendum (13:00Z)

Wave 3 in flight at budget close: edge #1256 BUILDING (ships the
collectives stamp + carries the published happ with the batch-get);
genesis #1386 queued behind it (formation with the cure — its
"Results: N/3 affirmed" line is the first morning read). B caughtUp
still false at close; anchor field oscillating (windowed-scan noise,
not signal). Genesis #1385 (pre-cure, mid-night): bindings stage 7/7
failed again — reinforces the bindings-triage ceiling item; formation
stage unaffected by the cure until #1386.
