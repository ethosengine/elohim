---
id: qahal-api-spec-extraction-arc
status: noted
tier: history
derived_from:
  - app/elohim-app/src/app/qahal/QAHAL_API_SPECIFICATION_v1.0.md   # retired to git 2026-06-11 (qahal island recompose)
cites:
  - app/elohim-app/src/app/qahal/models/governance-feedback.model.ts
  - app/elohim-app/src/app/qahal/models/governance-deliberation.model.ts
  - app/elohim-app/src/app/qahal/services/index.ts
  - app/elohim-app/src/app/qahal/community.routes.ts
  - app/elohim-app/src/app/qahal/components/reaction-bar/reaction-bar.component.ts
  - app/elohim-app/src/app/elohim/services/governance.service.ts
  - app/elohim-library/projects/elohim-service/src/angular/services/governance-api.service.ts
  - app/lamad/src/app/models/feedback-profile.model.ts
  - elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs
  - elohim/sdk/domains/qahal/types/src/lib.rs
  - elohim/sdk/domains/qahal/manifest.json
  - qahal-domain-gospel | the subject home where the spec's surviving vocabulary and mechanism ladder are now canon | sha256:002d11309d8d9620 | path: elohim/sdk/domains/qahal/CLAUDE.md
  - mishpat-domain-gospel | the judgment-substrate boundary the spec never anticipated — qahal surface escalates into mishpat | sha256:d3a335f06f37c884 | path: elohim/sdk/domains/mishpat/CLAUDE.md
  - qahal-architecture-vision | the gospel-tier vision spec that superseded this ICD as qahal's forward canon | sha256:6a519b464b586832 | path: genesis/docs/superpowers/specs/2026-05-21-qahal-architecture-vision.md
---

# The Qahal API Spec Extraction Arc (v1.0)

## What it specified

`QAHAL_API_SPECIFICATION_v1.0.md` (582 lines, dated 2025-12-08, status line:
"Extracted from LAMAD_API_SPECIFICATION for pillar clarity") was the interface
control document for community governance. Its parent,
`LAMAD_API_SPECIFICATION_v1.0.md`, was itself retired 2026-06-11 in the lamad
island recompose (commit c8cb7ebe3 deleted it, 2538 lines; see the sibling
record `lamad-mvp-implementation-arc`) — so this retirement closes the last
branch of that document family. The spec carried: the "protocol's immune
system" framing and the Loomio/Polis/Wikipedia conceptual-inspiration trio
(Part 0 — both originate HERE and recur across qahal canon); the entity-scoped
route pattern `/{app}/{entityType}:{entityId}/governance/{view}` (Part 1);
constitutional SLA guarantees (1hr ack / 3d response / 14d resolution / 7d
appeal / 4hr emergency, §1.5); the FeedbackProfile dimension ("virality is a
privilege, not an entitlement", a 12-mechanism friction hierarchy, emotional
reaction types, Part 2); a 31-method TypeScript `GovernanceService` interface
(Part 3); and the type vocabulary (Part 4).

## The type system survived nearly intact

The spec's vocabulary landed in code close to verbatim and is still live:

- `governance-feedback.model.ts` (1371 lines): `GovernableEntityType` (line 42 —
  the spec's exact 8 values including `'elohim'` "even AI needs oversight"),
  `Challenge` (303), `Appeal` (449), `Precedent` (605), `DiscussionThread` (895),
  `GovernanceSLA` (1319). `DEFAULT_GOVERNANCE_SLA` (1348) carries the spec's
  constitutional times exactly: PT1H / P3D / P14D / P7D / PT4H, plus the
  default-favor-challenger breach consequence.
- `governance-deliberation.model.ts` (930 lines): `GovernanceContextMenu` (52),
  `FeedbackContext` (168, spec's 15 values), `DeliberationProposal` (471),
  `ProposalPhase` (510), `SensemakingVisualization` (584),
  `GovernanceHistoryView` (673).
- The Part 2 friction-hierarchy intuition matured into the 8-level Governance
  Mechanism Ladder (`elohim/sdk/domains/qahal/CLAUDE.md` §Governance Mechanism
  Ladder): levels 0-2 stay Angular; levels 3-7 route through
  `FeedbackMechanismGatewayComponent` to `PsephosBallotWrapperComponent`
  ("Governance levels 3-7 route here via the gateway's renderTarget ===
  'psephos'", component header).

## Every delivery assumption was refactored away

1. **The service moved pillars and went thin.** `GovernanceService` lives in
   the *elohim* pillar (`app/elohim-app/src/app/elohim/services/governance.service.ts`,
   504 lines — partial against the spec's 31-method contract), re-exported by
   qahal's `services/index.ts`. It is backed by `GovernanceApiService`
   (elohim-library), which calls doorway `/api/v1/governance/*`; challenge
   submission is "MVP: still localStorage, wired in Sprint 3"
   (governance.service.ts:238).
2. **Spec-era client services became server-side projections.** Per the
   retirement comments in `qahal/services/index.ts`: `SignalAccumulationService`
   retired by M-POLICY-1 → `GovernanceApiService.getAccumulationStatus()`;
   `MechanismSelectionService` retired by M-POLICY-2 →
   `getMechanismSelection()` (substrate computes level/mechanism/renderTarget);
   the wave continued with M-REA-3 (`GovernanceRecognitionService` →
   `postParticipation()`).
3. **Routes went namespace-scoped.** The entity-scoped pattern was never
   registered in any router; `community.routes.ts` registers
   `/community/governance/{sensemaking,challenges,disposition,proxy-votes}`
   with query params instead. Yet the `GOVERNANCE_ROUTES` builders
   (governance-deliberation.model.ts:819-850) still mint spec-pattern
   `/lamad/{entityType}:{entityId}/governance/{view}` URLs — each annotated
   `route-literal-ok` with claims-minting migration tracked in epr-routing
   captures. The spec's URL grammar outlived its router.
4. **The substrate materialized as a DNA, not a TypeScript interface.** What
   Part 3 imagined as service methods became Holochain integrity entry types:
   `Precedent` / `Discussion` / `GovernanceState`
   (`mishpat_integrity/src/lib.rs` lines 18/48/71, `EntryTypes` enum at 311),
   with wire types in the `qahal_types` crate (55 `pub struct`/`pub enum`
   declarations in `elohim/sdk/domains/qahal/types/src/lib.rs`). Boundary
   canon: "qahal is the app/social governance surface; mishpat is the judgment
   substrate it escalates into" (`elohim/sdk/domains/mishpat/CLAUDE.md:11`).
5. **Canon moved to subject homes.** Vocabulary now lives in
   `elohim/sdk/domains/qahal/manifest.json` (5 implemented governance
   contentTypes — collective/proposal/challenge/appeal/statement — plus 5
   social types marked `"status": "planned"`); the vision canon in
   `genesis/docs/superpowers/specs/2026-05-21-qahal-architecture-vision.md`.

## What never shipped as substrate

Part 2's FeedbackProfile dimension never became a persisted layer — but its
vocabulary DID get typed, in the *lamad bundle* rather than the qahal pillar:
`app/lamad/src/app/models/feedback-profile.model.ts` (972 lines — the
12-mechanism friction hierarchy, mediated emotional reactions, profile
evolution types — whose header promises a Holochain `feedback_profile` entry
type that no Rust source implements). Qahal's `ReactionBarComponent` renders
the mediated-reaction UI from it through a cross-bundle `@app/lamad` import,
and `governance-deliberation.model.ts:156` defers to it. Nothing persists,
evolves, or substrate-gates a profile, and the qahal manifest carries no
feedbackProfile vocabulary. The design (virality as privilege, no-likes
principle, emotional-reaction constraints, intellectual-humility
up/downgrades) and its verified gap ledger are preserved in the backlog entry
`qahal-feedback-profile-vision-remainder`.

## Why it matters for the future

- **Interface-control documents age at the delivery layer first; vocabulary
  outlives plumbing.** The Part 4 types and SLA constants landed near-verbatim
  and still anchor live code; every delivery assumption around them — which
  pillar hosts the service, client vs server computation, URL shape, substrate
  as TS interface vs DNA — was rewritten within six months.
- **The substrate arrived from outside the spec.** Mishpat (the judgment DNA)
  has no antecedent in this document; the spec's flat service interface gave
  no hint that escalation would split into a separate truth layer. Specs that
  define only the app surface under-constrain where authority lands.
- **Dead route grammar can stay load-bearing.** `GOVERNANCE_ROUTES` mints URLs
  for a router that never existed — tolerated because each literal is
  annotated and tracked toward claims-minting. A spec's most durable residue
  may be its naming scheme, not its endpoints.
- **Vocabulary can ship in the wrong pillar and still work.** The
  feedback-profile types landed in the lamad bundle during the lamad MVP era
  and qahal consumes them cross-bundle — a B18c-class coupling that the
  vocabulary-home decision (qahal manifest vs lamad content metadata) must
  eventually resolve.
