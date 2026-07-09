---
id: "backlog-content-head-election-vs-reach-fork-arbitration"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "content_head election among genuinely-competing heads (sub-commons fork/merge/compete) — DEFERRED, and the reach ≠ head ≠ replication guard that must precede it"
slug: "content-head-election-vs-reach-fork-arbitration"
written: "2026-07-09"
author: "frontend-eyes-sprint (C3 refocus)"
status: "deferred"
priority: "medium"
area: "substrate/content-versioning-authority"
domain: "operator"
jobs: [elohim]
---

## Origin

Decomposed from a premature `elohim/elohim-storage/src/content_head_election.rs` (written + unit-passing, then removed 2026-07-09) that reached for "elect among competing heads" to explain the `elohim-host-landing` divergence. Operator ontology-guard corrected the framing: that divergence is NOT a legitimate fork awaiting arbitration — it is ONE commons-reach head that failed to replicate coherently (each doorway independently built + notarized its own bundle at deploy). Election was the wrong tool AND carried a reach↔head conflation. This item preserves the genuinely-future work and the guard, so neither is lost or re-conflated.

## The guard (must hold before ANY election work) — three orthogonal planes

Do NOT conflate these. Each answers a different question; each has a different governor.

| Plane | Question | Governed by | Landing-page truth |
|---|---|---|---|
| **Reach** | *Who may see this EPR?* (audience/visibility) | earned attestation/standing (amber → … → commons) | **commons** (the protocol's public face — correct) |
| **content_head** | *Which VERSION is canonical?* | `declare_content_head` (authority over THIS EPR's versioning) | exactly **one** head |
| **Replication / custody** | *How many peers serve the head's bytes?* | custody commitments, salvage | **both** doorways serve that one head |

- Earning commons reach ≠ declaring the head. An EPR at commons reach still has exactly one head; a private-reach EPR also has a head. The two axes are independent.
- `elohim-host-landing` is a **replication-coherence** problem (one head, fractured by per-host deploy build+notarize), NOT a head-election problem and NOT a reach problem. Its cure is *one build → one head → replicated* (replication plane + the single-notarized-head arc on dev), never arbitration.
- **C3 near-term is reach-CLEAN**: `resolve_head` serves *the* declared head (declaration-over-recency, single head). It must carry no reach coupling. See the plan `2026-07-01-crdt-content-dataplane-full1c-implementation-plan.md` C3.

## The deferred work — fork arbitration (only when sub-commons forking is REAL)

When peers below the commons head genuinely diverge/fork/merge/compete (the [[project_earned_reach_governance_pr_ceremony_vision]] world), there WILL be multiple legitimate competing heads for one id, and the network must elect one by EARNED authority. The removed module's sound kernel, to be rebuilt then (reframed, reach-clean):

- **Recency is never the tiebreak.** A lone/declared head elects; ≥2 competing undeclared heads surface as an explicit `Divergent` (honest "no earned head yet"), never silently last-writer.
- **The winner is chosen by earned authority** (reach-cohort edit-membership + author/community signature — plan C5, blocked-by-env on the DNA/notary pipeline), tended by Elohim, NOT by a deterministic lock (first-writer-wins is a dev convenience, not the goal — see the notary-authority feature prose).
- **`Divergent` escalates to the earned-authority election**, it does not resolve locally.

## Definition of done (for the deferred item)

1. A real sub-commons fork scenario exists (collaborative/forked authoring of one id) with an a2o scenario.
2. The earned-authority criterion (C5 reach-cohort + signature) is wired and DNA-verifiable.
3. Only THEN: rebuild the election decision as a pure, reach-clean function, fed by the earned-authority signal — with the three-plane guard above cited in its module doc.

## Blocked-by

Earned-authority substrate (plan C5) — `@requires:alpha-cluster-6peer` / DNA-notary pipeline. Do not build election before the fork scenario and the earned criterion both exist.
