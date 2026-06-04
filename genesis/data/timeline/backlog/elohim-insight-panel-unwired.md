---
title: "Elohim insight panel + banner-notification consumer never built — 4 a2o scenarios are the spec"
created: 2026-06-04
domain: "code"
tags: [elohim, presence, insight, banner, a2o, angular, code-domain]
shift_objective: |
  ElohimPresenceService exists (agent invocation, BannerNotice stream, cost$ observable) but
  three rendering pieces were never built: (1) the insight panel component emitting
  [data-testid=elohim-insight-section] + reasoning/cost child testids (selectors.ts:511-522
  registry is complete, zero templates emit them); (2) a banner-notification consumer
  ([data-testid=banner-notification] — BannerService has no rendering subscriber); (3) the
  lamad caller — ElohimPresenceService.onDiscoveryCompleted() has ZERO callers from the
  assessment-completion path. Scenario-first spec: the 4 @wip'd scenarios in
  features/elohim/elohim-presence.feature define the surface (insight after discovery,
  constitutional reasoning transparency, computation cost, test-connection banner). Done when
  those scenarios un-wip and pass. Testid alignment already fixed (test-backend →
  elohim-test-connection-btn, 2026-06-04). Conductor state is NOT on the causal path (default
  backend is mock).
---

Discovered + verified during the 2026-06-04 local shakeout sprint (4 consistent full-suite
runs; RCA grounded in zero-result grep sweeps). Angular-architect + component work; pairs
with the lamad Recognition-callback interpretation ownership (CLAUDE.md: session management
and interpretation belong to lamad services).
