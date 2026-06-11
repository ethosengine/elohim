---
id: "backlog-ci-genesis-household-founder-binding"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Household formation FATAL: founder never seeded — identity seeder's id-blind exists-check + doorway-phase exclusion (genesis #1119)"
slug: "ci-genesis-household-founder-binding"
written: "2026-06-11"
author: "agentic-developer (EPR durability arc, pipeline shakeout)"
status: "wip"
priority: "high"
ci_status: pending-verification
jobs: [elohim-genesis]
tags: [ci, genesis, seeding, household-formation, conductor-identities, imagodei]
cites:
  - genesis/seeder/src/seed-conductor-identities.ts
  - genesis/seeder/src/seed-household-formation.ts
  - genesis/seeder/src/__tests__/seed-conductor-identities.spec.ts
---

# Household formation FATAL — founder never seeded on any conductor

## Symptom (genesis #1119, the FIRST run to reach the conductors)

`Seed Household Formation` exited 1: `FATAL: no conductor found for the
founder (human-matthew-manager).` Directly downstream: zero households
form, the 5 resilience a2o scenarios fail (`"content-alpha" is stewarded by
0 households; expected ≥2`), and the ceremony custody-blob seeding (the
REAL-peer-id Stage-2 path) never runs.

## Root cause — three compounding seeder bugs

1. **Id-blind exists-check**: `seedHumanOnConductor` declared a human
   "exists" when get_my_human returned ANY human on the probed conductor —
   it never compared ids. #1119's identity stage reported all five
   node/device humans `[=] existing` against *matthew's* pod: a no-op lie.
2. **First-reachable-wins targeting**: every human probed the CONDUCTOR_URLS
   list in order, so whichever profile sat on the first pod satisfied
   everyone's probe. No conductor affinity existed.
3. **Doorway-phase exclusion**: the target filter was
   `agencyPhase ∈ {node, device}` — matthew (doorway) hosts a full conductor
   and is the household founder, yet was never a seeding target at all.

Bonus drift: the formation roster (and commitment fixtures + qahal a2o
steps) used `human-james-student`, an id that does not exist in humans.json
(canonical: `human-james-son`).

## Fix landed (same session)

- Identity-aware exists check (`extractHumanId` — flat + wrapped shapes);
  a different id on the human's OWN pod is now a loud `conflict` result.
- Name-affine targeting via the `elohim-<name>-<env>` service convention
  (the exact rule genesis/Jenkinsfile uses to build CONDUCTOR_URLS);
  no-pod humans are soft `skipped`; legacy walk retained for non-affine
  local-dev URL sets.
- `doorway` added to the phase filter (doorway-first sort).
- `human-james-student` → `human-james-son` fleet-wide.
- 9 new unit tests on the pure helpers; 311 seeder tests green.

## Verification path

Next genesis run with these commits: identity stage should show matthew
`[+] created` (or `[C] conflict` if his pod's agent already embodies a
different profile — that outcome needs operator attention: one agent = one
Human, and a reinstall mints a new agent key), then formation proceeds past
founder binding. Done = formation completes (or fails LATER than founder
binding) and the resilience scenarios see ≥1 household.

shift_objective: |
  Get Seed Household Formation past founder binding on a real genesis run:
  verify matthew's conductor embodies human-matthew-manager (or surface the
  conflict squatter id for the operator), formation creates/reuses the
  family-dowell collective, and ceremony custody commitments seed with
  Stage-2 real peer ids.
