---
id: placement-seam-check-brief
title: Adversarial seam check of the placement-contract interfaces against the code that must fit them
status: Active
class: architecture
gap: plans__2026-10-20-placement-contract-architecture-plan#4
actor: agent:reviewer@claude-opus-5-5
habits: [materialize-what-is-read]
cites:
  - "placement-contract-architecture-plan | The placement contract's architecture | sha256:d2a76864bcbbebd1 | path: genesis/docs/superpowers/plans/2026-10-20-placement-contract-architecture-plan.md"
  - "placement-interfaces-brief | The composed interfaces of the placement contract | sha256:4fd4e27a28eb1d07 | path: genesis/docs/superpowers/plans/task-placement-interfaces-brief.md"
---

Item I2 (gap `#4`). For each seam (A–I) of
`genesis/docs/superpowers/specs/2026-10-20-placement-contract-interfaces-design.md`, try to break the
interface against the existing code that must implement or adapt it, and against the design spec's rulings
and the orchestrator's R1–R6 (listed in `task-placement-interfaces-brief.md`). The code to test against
includes the `EprfsStorage` implementors, `PlacementStrategy` and the salvage author, ark-core's berth,
candidate and intent types, the app manifest schema, the intents schemas, `apps-sw.ts` and
`storage-client-ts`.

Each finding names the seam, the code (path:line), the break and the revision. A reviser applies the
findings that survive. The report lists each finding as applied or refused, with why.
Report: `task-placement-seam-check-report.md` with `gap:`, `actor:` and `status:`.
