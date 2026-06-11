---
id: "backlog-qahal-feedback-profile-vision-remainder"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "FeedbackProfile dimension: the persisted, evolving engagement-privilege layer the types already promise"
slug: "qahal-feedback-profile-vision-remainder"
written: "2026-06-11"
author: "claude"
status: "envisioned"
priority: "medium"
tags: [qahal, governance, feedback-profile, reactions, virality, p2p-design-gate, vision]
derived_from:
  - app/elohim-app/src/app/qahal/QAHAL_API_SPECIFICATION_v1.0.md   # retired to git 2026-06-11
cites:
  - app/lamad/src/app/models/feedback-profile.model.ts
  - app/lamad/src/app/components/content-viewer/content-viewer.component.ts
  - app/elohim-app/src/app/qahal/components/reaction-bar/reaction-bar.component.ts
  - app/elohim-app/src/app/qahal/components/feedback-mechanism-gateway/feedback-mechanism-gateway.component.ts
  - qahal-domain-gospel | subject home owning the mechanism ladder this vision composes with; manifest extension lands there | sha256:002d11309d8d9620 | path: elohim/sdk/domains/qahal/CLAUDE.md
  - elohim/sdk/domains/qahal/manifest.json
  - qahal-architecture-vision | kindred friction-gradient principle (power concentration) — same shape, different object; do not conflate | sha256:6a519b464b586832 | path: genesis/docs/superpowers/specs/2026-05-21-qahal-architecture-vision.md
  - genesis/docs/content/elohim-protocol/social_medium/epic.md
  - genesis/data/timeline/backlog/lamad-bundle-cross-pillar-cutover.md
shift_objective: |
  Take the FeedbackProfile dimension from client-side template fiction to a real,
  persisted, evolving engagement-privilege layer. Start at the p2p-design-gate:
  classify profile state, durable reactions, and upgrade requests as DHT entry
  types (A/A2/B/B2/C) BEFORE any HTTP route. Then land the vocabulary extension
  in elohim/sdk/domains/qahal/manifest.json, wire the substrate read path the
  gateway's MechanismSelection projection can consume, and close the typed-but-
  unenforced constraint gaps (authorCanHide/hideReaction, criticalRequiresReasoning,
  most-restrictive-wins path inheritance).
---

# FeedbackProfile dimension: the persisted, evolving engagement-privilege layer the types already promise

## Layer declaration

This is a **cross-layer vision item**: the type vocabulary lives in the lamad bundle
(`app/lamad/src/app/models/feedback-profile.model.ts`), the consuming UI lives in the
qahal pillar of the shell (`app/elohim-app/src/app/qahal/`), the vocabulary home a
manifest extension would land in is the qahal subject home (`elohim/sdk/domains/qahal/`),
and the missing piece is substrate-side. **Any pickup MUST pass the p2p-design-gate
first** — this creates new data entities (per-content profile state, durable reactions,
profile-upgrade requests), so DHT entry-type classification (notarized A / derived-link
A2 / agent-scoped B / B2 / operational C) comes before any `GET /api/v1/...` route is
sketched. The retired spec designed this surface REST-first; do not inherit that shape.

## What exists (do not re-implement)

The retired spec's Part 2 design is substantially *typed*, and partially *rendered*:

- **The full vocabulary is implemented as TypeScript** in
  `app/lamad/src/app/models/feedback-profile.model.ts` (972 lines): the 12-mechanism
  friction hierarchy (`FeedbackMechanism`, lines 42-61, with `MECHANISM_FRICTION`
  lines 76-89), the 8 emotional reaction types (5 supportive + 3 critical, lines
  105-116, categorized lines 127-136), `EmotionalReactionConstraints` with
  `requireAttribution`/`authorCanHide`/`criticalRequiresReasoning` (lines 151-169),
  Elohim-mediated reactions with proceed-behavior and monitoring telemetry (lines
  180-327), `FeedbackProfile` with determination + upgrade/downgrade evolution
  (lines 479-511, triggers lines 631-645), per-content-type default profiles
  (lines 724-880), and a most-restrictive-wins helper (lines 890-902).
- **A working reaction UI**: `ReactionBarComponent`
  (`app/elohim-app/src/app/qahal/components/reaction-bar/reaction-bar.component.ts`)
  renders all 8 reaction types, runs the mediation dialog (constitutional reasoning,
  suggested alternatives, "proceed anyway"), and records signals to the governance
  API (`recordSignal`, line 358) plus REA participation (`postParticipation`,
  line 364).
- **A server-side mechanism gate already exists**: the
  `FeedbackMechanismGatewayComponent` loads a substrate-computed MechanismSelection
  projection (M-POLICY-2) — "No client-side ladder constants" (component header) —
  realizing the governance mechanism ladder levels 0-7
  (`elohim/sdk/domains/qahal/CLAUDE.md` §Governance Mechanism Ladder).

## The vision remainder (verified gaps, 2026-06-11)

1. **Profiles are client-side fiction.** The only profile "load" is
   `content-viewer.component.ts:716-746`: a static template lookup by contentType,
   minted fresh per render (`createProfileFromTemplate(template, 'profile-' + node.id)`,
   line 723). Nothing persists, fetches, or notarizes a profile. The model's own
   header promises a Holochain mapping ("Entry type: feedback_profile",
   feedback-profile.model.ts:24-27) — grep across `elohim/holochain/**/*.rs` and
   `elohim/elohim-storage/src/views.rs` finds zero `feedback_profile` entities.
2. **No upgrade/downgrade lifecycle.** `ProfileChange`/`UpgradeEligibility`/
   `DowngradeVulnerability` are typed but nothing creates a profile change; the
   spec's `requestProfileUpgrade`/`ProfileUpgradeRequest` have zero implementations
   anywhere in app/elohim-app, app/lamad, or app/elohim-library (grep verified).
   "Profiles upgrade through trust-building, downgrade on new evidence" is pure
   aspiration today.
3. **Constraints are typed but unenforced.** `authorCanHide` is consumed by no hide
   UI (`hiddenByAuthor` is never written; the spec's `hideReaction` has zero
   implementations); `criticalRequiresReasoning` never produces a reasoning prompt
   outside the mediation path (reaction-bar's `submitReaction` takes optional
   context, line 326).
4. **Path inheritance is dead code.** `getMostRestrictiveProfile` (most-restrictive-
   wins) has no caller outside its defining module.
5. **No manifest vocabulary.** `elohim/sdk/domains/qahal/manifest.json` vocabulary
   covers contentTypes/relationships/signals/quiltPolicies/observations — no
   feedbackProfile section. That manifest is the home a vocabulary extension lands in.

## Compose with (kindred, distinct)

- The **mechanism ladder** partially realizes the friction hierarchy server-side —
  but it derives level from GovernanceState × contentType × Manifest, not from a
  persisted, evolving per-content profile. The remainder is the *entity*, not the gate.
- **Friction-gradient limitarianism**
  (`genesis/docs/superpowers/specs/2026-05-21-qahal-architecture-vision.md` §2.8)
  is about POWER concentration — standing accrual, collective size, council
  authority — not content-engagement amplification. Same graduated-friction shape,
  different object; don't conflate them in design.
- The **social_medium epic** carries the why: "There's no accidental virality"
  (`genesis/docs/content/elohim-protocol/social_medium/epic.md`).
- The **cross-bundle import** (shell qahal `ReactionBarComponent` importing
  `@app/lamad/models/feedback-profile.model` — the elohim-app tsconfig maps
  `@app/lamad/*` to `../lamad/src/app/*`, i.e. the lamad bundle's source tree)
  is B18c-class residue of the lamad pillar→bundle decomposition, tracked in
  `lamad-bundle-cross-pillar-cutover`. The vocabulary-home decision below
  resolves this coupling too.

OPEN QUESTION: should profile state live in the qahal vocabulary (it governs
community engagement) or attach to lamad content metadata (it's per-content)?
The p2p-design-gate classification should settle this before any code.
