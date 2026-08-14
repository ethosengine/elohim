---
id: "backlog-spin-divergent-undeclared-rows-block-a-convergence"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "SPIN-class divergence: ~13 anchor-divergent UNDECLARED content rows on each ethosengine pod have NO discharge path — ContestPeer is declared-only, ghost-decay is Absent-only"
slug: "spin-divergent-undeclared-rows-block-a-convergence"
written: "2026-08-14"
author: "claude (ci-investigator trace, saga leg-2 shift 2026-08-14T02-42)"
status: "open"
priority: "high"
tags: [projection-reconcile, spin, divergence, heads-converge, notary-authority, dataplane, resiliency-saga, miss-ledger, contest-peer]
cites:
  - elohim/elohim-storage/src/p2p/projection_reconcile.rs
  - elohim/elohim-storage/src/services/head_adoption.rs
  - genesis/data/timeline/backlog/susan-conductor-ws-dead-heal-pacing-blind-to-instant-errors.md
  - genesis/a2o/features/dataplane/resiliency-saga/06-heads-converge.feature
---

# The ~13-row SPIN population: anchor-divergent + undeclared = no lever reaches it

Traced 2026-08-14 (ci-investigator, Prometheus + Loki + code; medium-high confidence
on mechanism, low on id-set stability). This is the blocker that kept A-converged=0.0
through banked measure run #1345 (edge, 2026-08-14T01:30Z) and holds
`known_divergent{content}=13` on matthew, jessica, and james simultaneously.

## Mechanism (code-grounded)

- `classify_content_gap` (`projection_reconcile.rs:2839-2854`): a row is
  `ContentGap::Divergent` when it has a local `dht_anchor_hash` AND a peer advertises
  a different, non-empty anchor — both sides hold genuine roots that disagree.
- Step 4b (`:3176-3199`) partitions Divergent by `declared_head_action_hash`:
  declared → `divergent_refused` (heal is forbidden to move a declared head; that
  plane is 1000-1800/pod and separate); **undeclared → `MissLedger` with
  `divergent=true`** — this is exactly what
  `elohim_projection_reconcile_known_divergent{stream="content"}` publishes.
- Each sweep, `resolve_content_head` on the own conductor answers **the head the row
  already holds** → `StampOutcome::Refreshed` ("head unchanged — divergence NOT
  resolved") → real work, zero convergence, id re-discovered next sweep. The
  reconciler's own docstring (`:809-818`) names this the **SPIN signature**: "the two
  peers hold genuinely different roots … no amount of healing can converge them —
  only a canonical channel can."
- **Both existing levers are structurally out of reach:**
  - `try_adopt_canonical_head`'s ContestPeer branch requires `local_declared`
    (`declared_divergence_should_route_to_contest`, `:981-991`) — these rows are the
    undeclared half; the admission gate is unreachable by definition.
  - `ELOHIM_GHOST_DECLARATION_DECAY` (default off) applies only to `Answer::Absent`
    rows (`head_adoption.rs:790-830`) — these rows have anchors on both sides; the
    decay arm never sees them regardless of the flag.
- Retry machinery (`MAX_RETRIES=3`, `MISS_READMIT_SWEEPS=12`, 300s tick) cycles them
  Refreshed→dormant→re-admitted forever.

## Evidence

- Prometheus 2026-08-14T02:49Z: `known_divergent{content}` = 13/13/13 on
  matthew/jessica/james; stable ≈4h since the 2026-08-13 21:49-22:49Z restart
  cluster re-built the (in-memory, per-process) MissLedger.
- Loki (matthew, `Refreshed` outcome class, 1h): 18 unique ids, e.g.
  `elohim-observer-protocol`,
  `scenario-value-scanner-elderly-scenarios-individual-privacy rights preserved despite family care coordination needs`,
  `scenario-governance-appellant-scenarios-community-appealing community resource allocation decision`,
  `scenario-public-observer-board-member-scenarios-district-board member understands community priorities beyond meeting testimony`,
  `scenario-value-scanner-caregiver-scenarios-family-family cultural recognition of caregiving as sacred work`,
  `value-scanner-organizations-commons-engine-designing-commons-oriented-economies-readme`.
  Zero `Healed` outcomes in the same window.
- **Premise revision vs the leg-2 Objective:** the population is COUNT-stable
  (~13) but id-membership sampled 5h apart showed zero overlap — either churn
  through the ledger or a broader Refreshed-eligible set; "one frozen shared
  contested set minted in the crash window" is NOT confirmed. Cross-pod id overlap
  also unverified (one Loki 502, not retried).

## Current decision (the design question — needs /brainstorm, not more iteration)

Who moves an undeclared genuine-root conflict? Candidates from the reconciler's own
framing: (a) route undeclared divergence to a declare-then-contest path (mint the
declaration so ContestPeer becomes reachable — but that authors a head the node
never declared, exactly what the refused-plane discipline forbids elsewhere);
(b) a DHT head-election / canonical-link arrival so `resolve_content_head` stops
answering the stale root; (c) an operator-seat canonical stamp for bounded sets.
This is notary-authority habit domain (heads move via canonical channels only —
the p2p-design-gate + trust-contract invariants apply). Do NOT tune sweep cadence,
retries, or dormancy at it — the trace shows retrying converges nothing.

## CURE LANDED (2026-08-14, same session) — symmetric self-candidacy supplies the election

Design answered through the p2p-design-gate (no new entry types, routes, tables,
or sync messages; coordinator zome untouched — storage-side only), resolving to
option (b): supply the existing DHT election so canonical-link arrival is what
moves `resolve_content_head` off the stale root. The doc's objection to (a) held
and shaped the cure — nothing mints a *declaration*; the mint is an election
*candidate* (the two authority planes RCA-v3 wall 1 distinguished).

**Decisive code fact:** peer-head candidacy is impossible for this class BY
DESIGN — the head-record responder's hint-inflation guard (2026-08-02,
`view_federation.rs`) answers `Absent` for anchored-but-undeclared rows precisely
so a requester can never turn a peer's anchor into a declaration. So the
discharge is **symmetric SELF-CANDIDACY**: each side nominates its OWN
genuinely-held root (`contest_divergent`, zome gates pass by construction —
chain exists, target locally retrievable), `select_canonical_winner` arbitrates
deterministically, and both rows converge through the shipped obey/HealCanonical
paths. Nothing is stamped; the election is the canonical channel.

Mechanism (all elohim-storage):
- **Admission** — the `Refreshed` heal outcome (the LAST refusal site with no
  candidate admission) routes actionable-divergent undeclared ids to
  `adopt_candidates` via the new pure predicate
  `undeclared_divergence_should_route_to_contest` (`projection_reconcile.rs`);
  DB read-failure fails closed (never admits).
- **Decision** — `decide_head_action` gains a sixth input
  (`peer_divergent_anchor`) and one new arm `ContestDivergent`; exhaustive tests
  pin that no pre-existing table row changed and the arm fires from exactly one
  corner.
- **Execution** — `contest_divergent` (`head_adoption.rs`): backoff-gated,
  `(id, target)`-idempotent, one declare per id per process, minted under
  `elohim_content_canonical_links_minted_total{source="contest_divergent_self"}`.
- **Config** — `contest_undeclared_divergence`, default ON (same reasoning as
  `contest_two_way_declared`: a node with a chain nominating its own view is a
  safe supply act); env kill-switch `CONTEST_UNDECLARED_DIVERGENCE=0`.
- **Formal closure** — the C3 liveness contract gains
  `the_spin_class_discharge_is_flag_shaped`: flag OFF proves the kill-switch
  returns the class to its absorbing pre-cure spin (manual-only exit); flag ON
  (shipped default) proves the discharge via the live predicates. The main
  48-state table's scope note now names why this class was invisible to it
  (its peer dimension models declaration advertisements only).

Local gates: 2,671 lib tests + integration green; registered in
`elohim-storage/seam-registry.yaml`. Status stays **open** until the
verification hooks below fire live.

## Verification hooks once a lever exists

- `known_divergent{stream="content"}` on the three ethosengine pods drains to ≤2
  and STAYS there across a MissLedger rebuild (pod restart) — the in-memory reset
  means a post-restart re-climb is the honest signal the conflict is still live.
- Saga ch06 heads-converge (@concern under the edge Dataplane Validation tag set)
  flips green on a banked validate-only run.
