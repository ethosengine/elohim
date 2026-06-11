---
id: "backlog-lamad-expertise-discovery-integration"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Expertise discovery — model scaffold exists with zero consumers; service + P2P design gate pending"
slug: "lamad-expertise-discovery-integration"
written: "2026-06-11"
author: "lamad island recompose (avodah authorship pass)"
status: "envisioned"
priority: "low"
tags: [lamad, mastery, expertise, discovery, mentorship, p2p-design-gate]
cites:
  - app/lamad/src/app/models/expertise-discovery.model.ts
  - app/lamad/src/app/models/index.ts
  - app/lamad/src/app/interfaces/agent.interface.ts
  - elohim/sdk/domains/lamad/manifest.json
---

# Expertise discovery — model scaffold exists, integration never started

**Layer declaration:** lamad is a design-domain lens over the shared EPR core
(per the subject-routing locus graph,
`genesis/docs/superpowers/specs/2026-06-11-subject-routing-locus-graph-design.md`).
This entry assumes the mastery substrate already exposed to lamad — `MasteryLevel`
and mastery accessors now live in `@elohim/service`
(`app/lamad/src/app/models/expertise-discovery.model.ts` line 17 imports
`MasteryLevel` from `@elohim/service/angular/models/agent.model`; the
`ILamadAgent` token contract exposes `getAllContentMastery()` at
`app/lamad/src/app/interfaces/agent.interface.ts:49`). It does NOT restate
that substrate; it proposes a consumer of it.

## What

The mastery graph naturally reveals expertise: "who actually knows X" emerges
from learning and contribution activity rather than a gameable reputation
score. The original lamad backlog (app/lamad/docs/BACKLOG.md, last updated
2024-11-29) envisioned: expert finding by domain, reviewer matching for
contributions, mentor matching for learners, leaderboards (top / rising /
most-helpful), and privacy controls (discoverability, domain visibility,
leaderboard opt-in/out).

## Current state (verified 2026-06-11)

- `app/lamad/src/app/models/expertise-discovery.model.ts` EXISTS — 15
  interfaces (`ExpertiseQuery`, `ExpertCandidate`, `ExpertLeaderboard`,
  `ReviewerMatch`, `MentorMatch`, `ExpertiseVisibility`,
  `MentorshipPreferences`, `MasteryVelocity`, ...). It survived the B18
  bundle extraction and the Slice 2.1 mastery-type migration (its
  `MasteryLevel` import was repointed to `@elohim/service`).
- ZERO consumers: repo-wide grep for `ExpertiseQuery|ExpertCandidate|
  ExpertiseDiscoveryService` matches only the model file itself; the model is
  not even exported from the models barrel
  (`app/lamad/src/app/models/index.ts` has no `expertise` line).
- No `ExpertiseDiscoveryService` exists anywhere in `app/`.

So this is a pure scaffold: typed vision, no integration, untouched for ~18
months. Status `envisioned` reflects that honestly (the 2024 island rated it
"Medium" priority; nothing has pulled on it since).

## Why it still matters

- **For learners**: find mentors who demonstrably know what they teach.
- **For contributors**: find reviewers qualified to give meaningful feedback.
- **For governance (qahal)**: surface who should participate in content
  stewardship — a substrate-grounded alternative to self-nomination.
- **For the protocol**: routes questions to humans who can answer them,
  which is the human-amplification thesis in one feature.

## Readiness / gating

1. **P2P design gate is MANDATORY before any implementation** (per
   CLAUDE.md): expertise queries aggregate agent-scoped mastery data across
   agents. The model's own privacy surface (`ExpertiseVisibility`,
   leaderboard opt-in) implies attestation-style classification questions —
   is an expertise profile agent-scoped-with-attestation (B2)? Is a
   leaderboard a derived projection (A2) or operational cache (C)? None of
   this was answered in 2024; the model predates the gate.
2. Mastery substrate dependency is now REAL rather than aspirational:
   `ContentMasteryService` exists (`app/lamad/src/app/services/
   content-mastery.service.ts`, consumed by `path.service.ts`), and mastery
   types are SDK-level. The original "Depends On: Content Mastery system
   being fully implemented" blocker has substantially cleared.
3. Cheap first step when picked up: decide barrel export + write the a2o
   scenario for ONE use case (reviewer matching is the smallest), then run
   the p2p-design-gate before any service code.

OPEN QUESTION: should the 15-interface scaffold be trimmed to the one
use-case actually pulled first (reviewer matching), rather than implementing
the full query/leaderboard surface? The model's breadth is 2024 vision, not
validated demand.

OPEN QUESTION: leaderboards may be in tension with the protocol's
anti-gamification stance (the model's own header argues against "a gamified
score that can be gamed") — qahal/governance review should weigh in before
that sub-feature is built.
