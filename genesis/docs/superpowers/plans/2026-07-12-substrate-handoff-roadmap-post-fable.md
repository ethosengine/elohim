---
id: substrate-handoff-roadmap-post-fable
title: Substrate Handoff Roadmap — what must become true next (post-Fable, for Opus/GPT stewardship)
status: active
class: plan
created: 2026-07-12
steward: rust-architect
cites:
  - substrate-trust-contract-runbook | The Substrate Trust Contract | path: genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md
  - substrate-convergence-five-defect-arc | The Five-Defect Convergence Arc | path: genesis/docs/content/elohim-protocol/history/2026-07-12-substrate-convergence-five-defect-arc.md
  - genesis/data/timeline/backlog/dht-scale-envelope-and-web2-projection-at-planetary-scale.md
  - genesis/data/timeline/backlog/sovereign-turn-relay-transport-commons.md
  - genesis-pair-dht-unity-plan | Genesis-Pair DHT Unity | path: genesis/docs/superpowers/plans/2026-07-11-genesis-pair-dht-unity-plan.md
  - opus-handoff-roadmap-2026-07-12 | tiered detail + falsifiable experiments | path: genesis/docs/reviews/2026-07-12-opus-handoff-roadmap.md
  - cold-outside-substrate-review-2026-07-12 | the standing external audit, verbatim | path: genesis/docs/reviews/2026-07-12-cold-outside-substrate-review.md
---

# Substrate Handoff Roadmap (post-Fable)

> For agentic workers: REQUIRED SUB-SKILL: superpowers:executing-plans. This
> roadmap ranks the arcs; each arc becomes its own plan/shift when picked up.

**Provenance.** Closes the 2026-07-11/12 convergence arc. Inputs: (a) the
arc's own residue; (b) a deliberately context-blind adversarial review by an
outside-expert agent ("the stranger") — its verbatim findings are preserved
in `genesis/docs/reviews/2026-07-12-cold-outside-substrate-review.md`; treat
them as a standing external audit, re-read before disputing any ranking here.

## 1. The north star, confirmed twice

The stranger's independent verdict matches the internal one: **the
conductor-terminated trust primitive is real and correctly implemented**
("no peer adopts a head from gossip; verify in your own conductor, then
serve" — StampMode Declare/GapFill, canonical-aware heal). Build ON it;
never around it. The differentiator that is NOT yet load-bearing: earned
authority (reach vocab 3-way drifted, HTTP enforcement gap filed) and REA
value-flow (aggregation undesigned, identity bindings unsigned). Say
"designed, not built" about those until they aren't.

## 2. The rig-first doctrine (the review's chief finding)

The zero-ICE-for-weeks incident proved the test topology has **no NAT, no
scale, no adversary** — so the three failure classes that matter most are
invisible to every current gate. Therefore the FIRST substrate investment is
not a feature: it is **the adversarial scale rig** — ~1000 containerized
conductors, corpus ≫ single-node RAM, real NAT emulation between "household"
and "cloud" segments, and a Sybil harness. Every scale claim below is
unfalsifiable until this rig exists. (It also finally gives the seam-smoke
suite a hostile environment to earn its keep in.)

## 3. Ranked arcs (each with its falsifiable measure)

1. **Bank the convergence + flip the gates** (2026-07-12 verify-and-bank
   leg; small). Measure: notary scenario 2 green ×2 fresh edge runs; apply
   the green-flip (seam-smoke `--gate`, de-`@wip` native-omni); spine
   node delta. Residue riders: T4 doorbell step-glue (3 undefined steps),
   heal-throughput smell (~10s/row × thousands post-restart).
2. **The adversarial scale rig** (§2). Measure: the three stranger
   experiments run at all — (a) convergence+latency SLO at N=1000 with
   `arc_factor<1`, (b) eclipse trial (k Sybils near a hot EPR's hash —
   honest reader still converges), (c) kill/partition trial against
   self-reported liveness.
3. **Sharded arc (`arc_factor < 1`) shipped and measured** — the single
   load-bearing scale lever; today full replication caps everything
   (per-node RAM ∝ corpus; OOM history already on file). Measure: rig
   experiment (a) passes with bounded per-node working set.
4. **Independent failure detection** — replace self-asserted `PeerStatus`
   with quorum-observed liveness (SWIM/phi-accrual class). Measure: rig
   experiment (c) — no custody/serve decision against a dead/partitioned
   peer beyond one detection interval.
5. **Storage quota + GC/eviction** — superseded heads, orphaned blobs,
   per-node quota; today the disk cost function is monotonic. Measure:
   one node, one week, appending corpus → bounded steady-state footprint
   (the stranger predicts this fails today; prove them wrong by building).
6. **Cross-signed identity binding** — the unsigned `AgentPeerBinding`
   (`STAGE1_SIGNATURE_SENTINEL`) gates ALL economic attribution; the
   forge-another-agent's-credit experiment currently SUCCEEDS. Measure:
   that experiment fails cryptographically. Prerequisite for any REA
   creator-payment work — sequence it BEFORE aggregation design.
7. **Reconcile-rail consolidation** — ~30 heal/sweep/backfill loops, ~40%
   on the shared `GapTracker` rail; a control plane is ONE loop shape.
   Measure: loop census where every reconciler either rides the rail or
   carries a documented exemption; new-loop epr-meta guard.
8. **Sovereign TURN + NAT-real CI leg** — the transport-commons exit
   already filed; the rig's NAT emulation doubles as the regression test
   that would have caught zero-ICE. Measure: seam smoke passes with
   household segment behind emulated symmetric NAT.
9. **Control-plane rollout/canary** — the heal/reconcile logic itself has
   no staged rollout or rollback (coordinator hot-swap is the only safe
   channel). Measure: a canary-node deploy path where one peer runs the
   new storage/coordinator for a soak window before the fleet.
10. **Earned-reach convergence** — unify the 3 reach vocabularies, close
    the HTTP enforcement gap, then the canary-governance seed
    (`epr-head-chrome-version-aware-optin-canary-governance`). Only after
    #6 (identity) — authority without attribution is theater.

## 4. What NOT to redo (verified, leave alone)

The trust primitive (I1-I3 + tests); the seam-smoke suite + validator gate
(dependency-free by hard requirement); the diagnostics + refresh actuators;
the epr-meta guards for the five defect classes; the tiered canonical-head
selection (staging/earned + earned-head guard, sweettest-covered as of
a7c13f912/0644132fb); the declare-everywhere propagation ladder (idempotent
by content, 8-attempt retry outlasting the observed ~10-min publish window);
the trust-contract runbook + the five-defect museum record. When a dataplane
probe reds, the runbook's per-red decision tree is the authority — probes
over prose, always.

## 5. Execution protocol (how an inheriting agent picks up an arc)

- **One arc = one shift/plan.** Author an Objective with the arc's OWN
  falsifiable measure (each arc in §3 names it). Never batch two arcs into
  one Objective — the measures interleave and neither stabilizes.
- **Verify before building.** Arcs 1, 3, 5 have partial prior work — run
  the arc's measure FIRST; a passing measure means the work is banking, not
  missing (the #1357 lesson: the cure was already on dev while the red was
  being re-diagnosed).
- **Tier discipline** (operator directive 2026-07-02): top-tier models
  orchestrate + judge; narrow crisply-defined legs (log absorption, census
  sweeps, mechanical migrations) go to Sonnet/Haiku. The rig (arc 2) is an
  orchestration-heavy build; the loop census (arc 7) is a delegation-heavy
  sweep.
- **Ceiling discipline:** anything touching live cluster state, re-key/
  reinstall on prod, or spend is operator-ceiling — capture, don't act.
  `ALLOW_DNA_REINSTALL` semantics and the coordinator hot-swap channel are
  documented in CLAUDE.md ("DNA changes don't redeploy by default").
- **No dumps:** an arc that stalls decomposes into backlog items under
  `genesis/data/timeline/backlog/` with its blocker named — never a parked
  half-plan.

## 6. The standing external audit (where the rankings come from)

The stranger's verbatim review is preserved at
`genesis/docs/reviews/2026-07-12-cold-outside-substrate-review.md`; the
tiered synthesis with full experiment definitions at
`genesis/docs/reviews/2026-07-12-opus-handoff-roadmap.md`. The three
one-liners to carry:

- *"The unit of trust is the notarized content head … well-chosen; the best
  idea in the repo; load-bearing in code, not aspiration."*
- *"It took FIVE stacked invisible defects to converge ONE head across TWO
  doorways. A mature reconciler has one failure mode with one probe."*
- *"The test rig has no NAT, no scale, and no adversary — so the three
  things that will actually kill this are exactly the three CI cannot
  currently see."*

Dispute an arc's ranking only after re-reading the section of the audit that
produced it.
