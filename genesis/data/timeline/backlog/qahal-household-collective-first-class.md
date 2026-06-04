---
title: "QAHAL PIVOT — the household collective as a first-class protocol story (design session prompt)"
created: 2026-06-04
domain: "design"
relatedNodeIds:
  - "memory:project_hub_optional_floor"
  - "memory:project_rea_compute_commitment_primitive"
tags: [qahal, household, collective, onboarding, reciprocity, rea, resilience, design-session, p2p-design-gate]
shift_objective: |
  Run the deep design session below (brainstorm → p2p-design-gate → spec). Operator-called
  pivot 2026-06-04: the household/collective context deserves FIRST-CLASS support —
  streamlined onboarding, balancing, and discovery — so a family, each with a device, hub
  or not, immediately sees the end-to-end benefits of the Elohim Protocol among themselves.
  Do NOT implement household-reciprocity seeds/scenarios piecemeal first; the session
  decides whether reciprocity is SEEDED as data or EMERGES from a qahal household-formation
  flow.
---

# Design-session prompt — Household collective as a first-class qahal story

## The operator's framing (verbatim intent, 2026-06-04)

> This is an example of a foundational hub/collective story that has to have 1st-class
> support, because this special context should have streamlined onboarding, balancing,
> and discovery — so a family, each with a device, with hub or not, immediately sees the
> end-to-end benefits of the Elohim Protocol among themselves.

## What tonight's investigation verified (input evidence — do not re-derive)

1. **The relationship layer is seeded; the REA layer is empty.** humans.json has the
   matthew/jessica/james(/susan) household with bidirectional consented intimate
   relationships — but ZERO reciprocal REA commitments exist between them. The only
   seeded custody pair routes through a REMOTE persona (matthew↔terrance).
2. **The household intent already existed and drifted.** genesis/Jenkinsfile's
   "Seed Custody Commitments" stage echo says "M1 matthew↔jessica pair" — the code
   seeds terrance. The protocol MEANT to start here.
3. **The delivery surface can't see households.** DeliveryPeer (elohim-storage
   p2p/mod.rs:302-315) carries neither householdId nor commitments; the
   observable-distribution household-counting scenarios can never pass. (Groundwork
   patch prepared 2026-06-04 — see "Prepared groundwork" below.)
4. **Scenario coverage treats household reciprocity as a shem/remote dependency** —
   human-resilience.feature's household reciprocation scenarios are @wip @requires:shem
   (mis-tagged per the shem≠multi-node rule); no scenario anywhere asserts named
   triad reciprocity. The household IS a 3-node cluster and should carry the story alone.
5. **Vocabulary verdict (p2p-design-gate, verified):** all primitives exist — no new
   entry types. `custody-blob` = intra-household share custody (view-wired:
   reciprocity_view, cluster_view). `replicates-dwelling` = CROSS-dwelling mutual backup
   (the epic's gertrude↔matthew "minimum viable backup relationship"). `delegates-compute`
   = bounded authority (canon: rea-compute-commitment-primitive.md; scope examples
   include household chores). Reach-enum drift (roadmap #13) — don't bake reach literals
   into new scenarios.
6. **The epic already promises this** (resilience README): the encrypted-bucket query
   over rea_commitments is what "makes intra-household share custody visible"; the
   Grandma Standard recovery test (2-of-3 household quorum, restored in under five
   minutes, no seed phrase) is the household instantiation.

## The design fork the session must settle FIRST

**Seeded vs emergent reciprocity.** Seeding triad commitments as genesis data makes the
scenarios green but fakes the protocol's actual promise: that a FAMILY FORMING A
HOUSEHOLD acquires these agreements through a lived flow. If qahal household-formation
is first-class, the agreements should EMERGE from onboarding (with consent UX), and
seed data should exercise THAT flow, not bypass it. Decide: (a) what household
formation creates by default (mutual custody? compute delegation? balancing policy?),
(b) what requires explicit member consent vs household-default-with-opt-out,
(c) what the seeder's role becomes (fixture for the formation flow's OUTPUT, or driver
of the formation flow itself).

## Session agenda (brainstorm → p2p-design-gate → spec)

1. **Onboarding**: a family of N devices (hub optional — the floor memory) forms a
   household qahal. What is the minimal ceremony? Who invites, who consents, what
   happens for kids' devices (james)? What does each member SEE immediately after?
2. **Balancing**: default reciprocity bundle at formation — mutual custody-blob for
   household-reach content, delegates-compute toward whoever runs the hub-ish node,
   storage quotas per the shefa balancing vocabulary (quiltPolicies just landed —
   tiered-quilt §4 v0.2 names storage-policy classes; is `household` a named policy?).
3. **Discovery**: household members discoverable to each other with zero config
   (mDNS LAN + DHT); the cluster/reciprocity views showing "stewarded bytes for people
   you love" (epic language) from minute one.
4. **The qahal entity model** (p2p-design-gate MANDATORY): is the household collective
   an existing qahal collective entity (account-package household-dowell exists) with
   new formation/defaults, or does formation need new coordinator functions? Category
   A/B/C per piece. No UUID-first thinking; content/agent-derived identity.
5. **Scenario architecture**: the household story as its own a2o spine
   (features/qahal/household-formation.feature + features/resilience/household-reciprocity.feature)
   — formation scenarios FIRST, reciprocity assertions as their consequence. Retag
   human-resilience household scenarios off @requires:shem. The five drafted scenario
   intents from 2026-06-04 (steady-state mesh / james-contributes-compute /
   member-offline continuity / reciprocity view / grandma-standard recovery) become the
   VALIDATION layer of the formation story.
6. **Immediate-benefit demo arc**: what can a family see end-to-end TODAY (content
   custody + serving across their devices) vs what the session must design
   (formation UX, balancing policy, recovery quorum wiring)?

## Prepared groundwork (do not lose)

- DeliveryPeer householdId+commitments enrichment: patch at
  `genesis/data/timeline/backlog/patches/deliverypeer-household-enrichment.patch`
  (verified-blocker fix; apply when the session blesses the delivery-surface shape).
- Seed-pair mechanics fully understood: deterministicPeerId(humanId, archetype),
  deployment-grounded archetypes (matthew:node, jessica:desktop, james:mobile),
  CUSTODY_PAIRS_JSON override path, idempotent 409-tolerant POST /api/v1/commitments.
- Jenkinsfile echo drift (line ~1814) — fix whenever that stage is next touched.
