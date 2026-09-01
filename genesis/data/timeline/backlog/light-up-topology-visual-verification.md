---
id: "backlog-light-up-topology-visual-verification"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Visual verification of the three light-up-topology legibility surfaces (EPR context menu · resilience posture/badge · shefa topology)"
slug: "light-up-topology-visual-verification"
written: "2026-06-03"
author: "cartographer"
status: "refined"
priority: "high"
themes: [light-up-the-topology, legibility-glue, visual-verification, a2o-browser-tier, epr-context-menu, resilience-badge, peer-topology]
relatedNodeIds:
  - "genesis/docs/superpowers/plans/2026-05-29-light-up-the-topology-sprint-kickoff.md"
  - "genesis/docs/superpowers/specs/2026-05-29-durability-topology-felt-resilience.md"
  - "genesis/docs/superpowers/specs/2026-05-29-epr-reachability-economics.md"
  - "genesis/docs/content/elohim-protocol/history/2026-06-02-light-up-topology-operational-visibility-arc.md"
  - "genesis/a2o/features/resilience/observable-distribution.feature"
  - "genesis/a2o/features/elohim-core/epr-link-hypercard.feature"
  - "genesis/a2o/features/lms/epr-link-navigation.feature"
  - "memory:project_household_living_core_lived_contrast_diffusion"
  - "memory:project_placement_signals_are_shefa_inputs"
tags: [topology, epr, resilience, browser-tier, household-testable]
shift_objective: |
  Drive the three "light up the topology" legibility surfaces to a tier-3 visual verdict on
  household-nodes (no shem needed). The single highest-coverage move first: wire the already-built
  `<elohim-context-menu>` Lit primitive (app/elohim-elements/elohim-core/src/elohim-context-menu.ts)
  into EprLinkComponent (app/elohim-app/src/app/elohim/components/epr-link/epr-link.component.ts) —
  open on right-click/long-press, populate ContextMenuItem[] with EPR actions (Open · About this EPR ·
  navigate-to-Network/resilience-tab · Copy EPR link · reuse qahal ContextMenuOnlyComponent's
  flag/challenge/feedback outputs). This unblocks the THREE @wip @browser-only scenarios in
  genesis/a2o/features/elohim-core/epr-link-hypercard.feature (chip resolve, unreachable-preview,
  right-click context menu). Then de-@wip the two household-scale browser scenarios already authored
  in observable-distribution.feature ("Operator can see their household device cluster" → /shefa/cluster,
  "Doorway operator dashboard topology tab is reachable") — both render-ready (my-cluster.component.html
  has 8 testids, peer-topology.component.ts has an inline template with 5 testids). Leave the
  @requires:shem multi-tenant scenarios held — they are correctly env-blocked, not regressed. Verify
  via `pnpm hc:start:seed` + `pnpm look /shefa/cluster --as Matthew` for a screenshot, then run the
  browser cucumber profile against the de-@wip'd scenarios. Done = three screenshots (epr context menu
  open, /shefa/cluster device tiles, doorway topology tab) + green household-scale scenarios.
---

# Visual verification of the three light-up-topology legibility surfaces

The operator asked: "Where are we at in delivering visual verification of the 'light up the topology'
surfaces?" — specifically (E) EPR-link context menu, (A) resilience posture/progressive badge,
(shefa) compute-resource network-graph topology (MyCluster / PeerTopology).

**The state, in one line:** the substrate and components are built; the gap is *integration wiring +
de-@wip'ing already-authored browser scenarios*, and — critically — **the household-scale slice is
fully testable on household-nodes today**. The history arc's "topology a2o @wip scenarios are env-blocked
browser-tier, HELD, not green" caveat is true but narrower than it sounds: the env-block is `@requires:shem`,
which the a2o contract defines as the *remote multi-tenant canvas only*. The household (matthew/jessica/james)
is itself a 3-node cluster — single-household device-cluster and doorway-topology-tab scenarios carry no
shem tag and are fair game now.

## Why this is the single highest-coverage handle

Epic E's context-menu wiring is the **one move that touches all three surfaces**: the context menu is
the connective tissue that makes the EPR-link surface (E) navigate *into* the resilience/Network tab (A)
and is the felt entry into the topology (shefa). It is integration, not greenfield — the
`<elohim-context-menu>` Lit primitive is built and story-covered (Library A default + B designed stories
under graphos), and `EprLinkComponent` already landed as a thin Angular wrapper around the Lit element
(commit 10516614e). The only missing piece is the consumer wiring: zero `<elohim-context-menu>` consumers
exist in elohim-app/src today (verified — the only ContextMenu* matches are qahal's separate
ContextMenuOnly/GovernanceContextMenu).

## Readiness, evidence-backed

- **Rendering path is local-Playwright-reachable.** `pnpm look <url> --as Matthew` (genesis/a2o/scripts/look.ts)
  renders headless in Che and writes a screenshot; `pnpm hc:start:seed` brings up the household substrate.
  First run needs `pnpm a2o:setup` (Chromium to XDG cache).
- **Testids exist** for /shefa/cluster (8: my-cluster-page, -summary, -totals, device-tile, …) and
  /shefa/peers (5: peer-topology-page, -summary, -cliff, …). The progressive resilience glyph lives on
  EprRelationshipCardComponent (app/elohim-app/src/app/elohim/components/epr-relationship-card/).
- **Scenarios already exist** — they just need de-@wip'ing or step-def wiring, not authoring:
  - epr-link-hypercard.feature: 3 @wip @browser-only (incl. "right-click opens context menu")
  - observable-distribution.feature: "household device cluster" + "doorway topology tab" are @browser-only
    *without* shem (testable now); the peer-aggregation + reciprocity scenarios are @requires:shem (held).
  - lamad/epr-link-navigation.feature: 2 scenarios, NOT @wip — relationship-card nav with reach+resilience badges.

## What stays blocked (correctly, not a defect)

Multi-peer counts ("3 peer households · 3 reciprocating"), reciprocity inflow/outflow rows, and the
full-network 5-conductor resilience scenarios carry `@requires:shem` and are runtime-skipped (HELD).
These need the remote multi-tenant canvas; do not chase them on household-nodes. The progressive
distribution-badge details tier ("4-dot expansion + diversity hint") similarly needs multi-household
shape to assert exact counts, though the badge *renders* household-scale.

## Born-linked

Extends the canonical seed `2026-05-29-light-up-the-topology-sprint-kickoff.md` (Epics A + E), grounded
in both 2026-05-29 vision specs and the 2026-06-02 history arc. This is the legibility-glue front of
Sprint 5 in the standing vision-readiness roadmap (vision 5 / readiness 9).
