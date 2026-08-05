---
id: elohim-mutual-phase1-build-arc
status: noted
tier: history
canonical: genesis/docs/content/elohim-protocol/autonomous_entity/mutual/epic-elohim-mutual.md
derived_from:
  - genesis/docs/DEV-QUICK-START.md          # retired to git 2026-06-26 (Elohim Mutual dev quick-start, stub era)
  - genesis/docs/PHASE-1-BUILD-SUMMARY.md     # retired to git 2026-06-26 (Phase-1 build snapshot, 2025-12-22)
cites:
  - elohim-mutual-epic | epic-elohim-mutual | sha256:81d79e525a4ee305 | path: genesis/docs/content/elohim-protocol/autonomous_entity/mutual/epic-elohim-mutual.md
  - shefa-economic-infrastructure | Shefa | sha256:d3eb56a79069a6cc | path: genesis/docs/content/elohim-protocol/shefa.md
  - app/elohim-app/src/app/shefa/services/insurance-mutual.service.ts
---

# Elohim Mutual — Phase 1 Build Arc (distilled)

> Distilled history record. Two point-in-time build docs (Dec 2025) — a developer quick-start and a
> build summary — were relocated out of the `genesis/docs/` root and retired to git on 2026-06-26;
> their Phase-1 snapshot is captured here and superseded by the live subsystem. The original bodies
> live in git history (see `derived_from`).

## What Phase 1 built (2025-12-22)

The first vertical of **Elohim Mutual** — constitutional, autonomous mutual insurance as a Shefa-domain
application (canonical narrative: `autonomous_entity/mutual/epic-elohim-mutual.md`) — landed as
**domain models + service stubs**:

- **Five core domain models** — `MemberRiskProfile`, `CoveragePolicy` + `CoveredRisk`, `InsuranceClaim`,
  `AdjustmentReasoning` — immutable and event-sourced.
- **Insurance event types** layered over the existing immutable `EconomicEvent` ledger.
- **`InsuranceMutualService`** — method stubs for the MVP path (members → claims → paid).
- **Analysis docs** — a review document and an integration guide.

## Architecture patterns established (the durable lessons)

1. **Immutability through events** — state changes are appended events, never in-place mutations.
2. **Constitutional transparency** — every coverage/claim decision carries a constitutional basis.
3. **Information-asymmetry flip** — the mutual surfaces risk evidence *to* members, not against them.
4. **Graduated governance** — claims and coverage decisions route through Qahal governance by severity.
5. **Prevention-oriented economics** — a three-way premium split with a prevention incentive.

Integration points wired in design: the immutable `EconomicEvent` ledger, `CommonsPool` risk reserves,
the `PremiumGate` revenue model, the Observer protocol for risk/claim evidence, and Qahal governance for
coverage decisions.

## Status then → now

Phase 1 was **models + stubs**, explicitly not verified-stable. The subsystem has since grown into a live
implementation — `app/elohim-app/src/app/shefa/services/insurance-mutual.service.ts` and its
`README-INSURANCE-MUTUAL.md` (now the current developer reference), wired into `shefa.routes.ts`. These
two Dec-2025 build docs are therefore **superseded snapshots**, preserved here for trajectory.
