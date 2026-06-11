---
id: bloom-mastery-progression-design
status: Draft
class: protocol-canonical
domain: lamad
pillar: lamad
written: 2026-06-11
derived_from: app/lamad/docs/BLOOM-MASTERY-DESIGN.md
cites:
  - attestation-consolidation-design | Attestation Consolidation | sha256:220c0a2a68c2a805 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-11-attestation-consolidation-design.md
  - elohim-protocol-specification | protocol-specification | sha256:659b0d47078b298f | path: genesis/docs/content/elohim-protocol/protocol-specification.md
  - elohim/sdk/domains/lamad/manifest.json
  - elohim/sdk/domains/lamad/manifest/graph.json
  - elohim/sdk/domains/lamad/manifest/signals.json
  - elohim/sdk/domains/lamad/manifest/observations.json
  - elohim/sdk/domains/lamad/manifest/attestations.json
  - elohim/sdk/schemas/v1/enums/mastery-level.schema.json
  - elohim/sdk/schemas/v1/attestation/subtypes/mastery-metadata.schema.json
---

# Bloom-Taxonomy Mastery Progression — Canonical Architecture Seed

## 0. Layer declaration (what this document is, and what it assumes)

Lamad is a design-domain **lens over the shared EPR core** (per the subject-routing locus
graph, `genesis/docs/superpowers/specs/2026-06-11-subject-routing-locus-graph-design.md`).
This document declares the lamad mastery-progression design; it does **not** restate the
layers below it. It assumes:

- **Substrate**: the protocol `MasteryLevel` enum
  (`elohim/sdk/schemas/v1/enums/mastery-level.schema.json` — DNA constant `MASTERY_LEVELS`
  in `content_store_integrity` per that schema's `_dna` block), the EPR Document's optional
  `bloomLevel` field (`genesis/docs/content/elohim-protocol/protocol-specification.md`,
  Tier 2 EPR Document, ~line 156), and the consolidated Attestation primitive
  (`2026-05-11-attestation-consolidation-design.md` — "D2").
- **Domain**: the lamad manifest (`elohim/sdk/domains/lamad/manifest.json` and its
  `manifest/*.json` concern files) — the subject home (`lamad-domain-gospel`,
  `elohim/sdk/domains/lamad/CLAUDE.md`).
- **Pillar bundle**: `app/lamad/` consumes the above (`lamad-bundle-gospel`,
  `app/lamad/CLAUDE.md`); model/service citations below are the consumer-side
  implementation evidence, not the source of truth.

## 1. Thesis: mastery is participatory

Khan Academy's mastery model stops at "apply" and calls it mastered — consumption
proficiency. The Elohim Protocol's mastery progression continues into the upper levels of
Bloom's Revised Taxonomy (Anderson & Krathwohl, 2001), where mastery **requires active
contribution**: analysis, peer evaluation, and creation. Path completion is defined at the
gate (100% at `apply`) so completion stays achievable and fun; the upper levels are
participation privileges earned beyond completion, not completion requirements.
(Vision retained from `app/lamad/docs/BLOOM-MASTERY-DESIGN.md`; the gate framing is LIVE —
see §3.)

## 2. The eight-level progression — AS IMPLEMENTED

The protocol enum (substrate layer) is the single vocabulary:

| Level | Value | Engagement |
|---|---|---|
| `not_started` | 0 | none |
| `seen` | 1 | passive |
| `remember` | 2 | passive |
| `understand` | 3 | passive |
| `apply` | 4 | **attestation gate** |
| `analyze` | 5 | active |
| `evaluate` | 6 | active |
| `create` | 7 | active |

LIVE citations:
- Schema: `elohim/sdk/schemas/v1/enums/mastery-level.schema.json` — 8 core values, plus 3
  *extensible* legacy aliases (`recognize`, `recall`, `synthesize`) mapped
  recognize/recall→remember, synthesize→create per its `_tiers` block.
- TypeScript: `MasteryLevel` union and `MASTERY_LEVEL_VALUES` in
  `app/elohim-library/projects/elohim-service/src/angular/models/agent.model.ts:219-241`;
  comparison helper `compareMasteryLevels` (:266).
- **Naming note (design→landed delta):** the source design proposed renaming to
  `BloomMasteryLevel` with `MasteryLevel` deprecated (BLOOM-MASTERY-DESIGN.md:217-249).
  What landed is the inverse: the Bloom semantics live **as** `MasteryLevel`, and
  `BLOOM_LEVEL_VALUES` survives only as a deprecated alias of `MASTERY_LEVEL_VALUES`
  (agent.model.ts:246).

## 3. The attestation gate — AS IMPLEMENTED

`apply` is the threshold between consumption and contribution.

- `ATTESTATION_GATE_LEVEL: MasteryLevel = 'apply'` and `isAboveGate()` —
  agent.model.ts:253, :258.
- Privilege ladder (`PRIVILEGE_REQUIREMENTS`,
  `app/lamad/src/app/models/content-mastery.model.ts:191-201`):
  `view`/`practice` → always; `comment`/`suggest_edit` → analyze+;
  `peer_review`/`rate_quality` → evaluate+;
  `create_derivative`/`contribute_path`/`govern` → create.
- Server-side check surface: `/api/v1/mastery/.../check-privilege` route family
  (`elohim/elohim-storage/src/api/mastery.rs` header comment, routes
  `/api/v1/mastery[/{contentId}][/engagement|/assessment|/batch|/path/{pathId}|/stats|/check-privilege]`).
- Gate crossing is a first-class event: `LevelUpEvent.isGateLevel`
  (`app/lamad/src/app/models/learner-mastery-profile.model.ts:203-224`).

Privilege **enforcement in upper-level features** (comment/peer-review/derivative UI) is
Vision remainder — see §8 and the gap ledger.

## 4. Per-level mechanics: XP economy, levels, streaks — AS IMPLEMENTED

- `MASTERY_XP_WEIGHTS` (learner-mastery-profile.model.ts:61-70): not_started 0, seen 1,
  remember 3, understand 5, apply 10, analyze 15, evaluate 20, create 30 — upper-Bloom
  contribution weighted ~3x the gate level.
- `LEARNER_LEVELS` — 8 tiers Newcomer→Creator at XP thresholds
  0/100/500/1500/4000/8000/15000/30000 (learner-mastery-profile.model.ts:46-55), with
  `getLearnerLevel`/`getLearnerLevelProgress` helpers (:79-108).
- Streaks: `StreakInfo` + `StreakRecordContent` source-chain entries
  (learner-mastery-profile.model.ts:165-194); orchestrated by `MasteryStatsService`
  (`app/lamad/src/app/services/mastery-stats.service.ts` — composes
  `LearnerMasteryProfile`, subscribes to `ContentMasteryService.levelUp$`, bridges
  level-ups to the points economy).
- Mastery state itself: `ContentMasteryService`
  (`app/lamad/src/app/services/content-mastery.service.ts`) — dual backend (visitor
  localStorage via `LocalSourceChainService`; hosted/native via `LEARNER_BACKEND`),
  immutable `mastery-record` source-chain entries (`ENTRY_TYPE_MASTERY_RECORD`).
  This is the **Category B2 private progress record** of D2 §11.2 — agent-scoped, never
  DHT without consent.

## 5. Freshness and decay — AS IMPLEMENTED (with a client/server drift)

- Client model: `FRESHNESS_THRESHOLDS` FRESH 0.7 / STALE 0.4 / CRITICAL 0.2
  (content-mastery.model.ts:294-303); per-level exponential `DECAY_RATES`
  (λ from seen 0.05 down to create 0.005 — content-mastery.model.ts:309-318), implementing
  "higher levels decay slower."
- Server projection: `elohim/elohim-storage/src/db/content_mastery.rs` — flat
  `FRESHNESS_DECAY_PER_DAY: f32 = 0.05` (:97), periodic `apply_freshness_decay` (:423-426),
  engagement resets `freshness_score` to 1.0 (:343), refresh-queue query ordered by
  ascending freshness (:211).
- UI: refresh/practice queue on the learner dashboard
  (`app/lamad/src/app/components/learner-dashboard/refresh-queue/refresh-queue.component.ts`).

**DRIFT (cited, unresolved):** the per-level decay curve exists only client-side; the
storage projection decays a flat 0.05/day for every level. OPEN QUESTION: which side is
canonical for decay, and should the server adopt `DECAY_RATES`?

Graph-relative freshness (graph-evolution factor, activity-relative refresh) and the
content-lifecycle "right to be forgotten" are Vision remainder — see §8.

## 6. Graph layer — AS IMPLEMENTED

Declared in `elohim/sdk/domains/lamad/manifest/graph.json`:

- `MASTERY_OF` edge: ContributorDID → EprHead, **weighted + temporal** (:34-40).
- `MasteryRecord` node: `{contributor_did, concept_cid, level, attested_at}` (:52-59).
- Datalog rules: `prerequisite_chain` (:81-83) and **`mastery_frontier`** (:86-89) —
  "concepts a contributor could now approach: prereqs satisfied, target not yet mastered."
- Liveness: rules are registered into a live datalog engine and syntax-tested in
  `elohim/elohim-storage/tests/lamad_manifest_registration.rs`
  (`mastery_frontier_rule_body_syntax_valid`, :141-165).

The frontier rule is the implemented form of the design's "fog of war"/what's-next
mechanic: mastery state is graph state, and the next-step query is a graph query — not a
recommendation-engine bolt-on.

## 7. The lamad ↔ attestation coupling (composes with D2 — does not fork it)

D2 (`2026-05-11-attestation-consolidation-design.md`) resolved the split this design
needs (its §11.2):

- **Private progress** = `ContentMastery`, Category B2, stays in lamad (storage projection
  + source chain; §4 above). NOT consolidated into core.
- **Public proof** = `attestation:mastery`, a `Content` entry with the
  `content_type: "attestation:<subtype>"` discriminator (D2 §3.1) — Category A, minted
  *from* private progress "when policy fires."

The lamad manifest instantiates D2 §6.1's declaration shape — LIVE:

- `elohim/sdk/domains/lamad/manifest/attestations.json` → `attestation:mastery`:
  subject_kinds `["agent"]`, metadata schema
  `$ref ../../../schemas/v1/attestation/subtypes/mastery-metadata.schema.json`,
  uniqueness anchor `attestation:mastery:{subject_cid}:{concept_cid}:{issuer_cid}`,
  revocable_by issuer | domain-steward.
- Signal: `mastery-achieved` (`manifest/signals.json:8-13`) — substrateSignal `attention`,
  economicAction `produce`, resourceType `mastery-attestation`. This is the three-leg
  value coupling: leveling up *produces* an attestation resource that flows value to
  content stewards.
- Feedback leg: `mastery-attestation-meaningful` (`manifest/observations.json:12`,
  instrument `outcome-correlation`, polarity positive) with its mandatory negative twin
  `downstream-prerequisite-failure` — mastery attestations are graded by whether attested
  learners actually succeed downstream. This is how mastery attestation *earns* its
  standing rather than self-asserting it (the reach-earning machinery,
  `elohim/sdk/CLAUDE.md` §Feedback).
- Minting surface: the generic `issue_attestation` coordinator + manifest-driven
  validator exist in elohim DNA
  (`elohim/holochain/dna/elohim/zomes/content_store/src/attestation.rs`,
  `content_store_integrity/src/attestation_validator.rs`). The mastery-specific
  **auto-minting policy** (private progress crosses gate → attestation issued) is not
  implemented — gap ledger.

## 7b. Vocabulary reconciliation — OPEN QUESTIONS (verified drift)

1. OPEN QUESTION: `mastery-metadata.schema.json` constrains the attestation's
   `mastery_level` to `["familiar", "proficient", "expert", "teaching"]` — a 4-tier
   vocabulary **disjoint from** the 8-level Bloom `MasteryLevel` enum. No mapping
   (which Bloom level mints which tier) is declared anywhere I could find.
2. OPEN QUESTION: the protocol specification's EPR Document `bloomLevel` comment
   (`protocol-specification.md` ~line 156) enumerates
   `not_started | remember | understand | apply | analyze | evaluate | create` —
   **`seen` is missing** relative to the schema enum. Spec-comment drift or deliberate
   (EPR head doesn't carry sub-remember states)?
3. The schema's extensible aliases (`recognize`, `recall`, `synthesize`) exist only for
   imported legacy content (mastery-level.schema.json `_tiers.extensible`); new surfaces
   must not emit them.

## 8. §Vision remainder (designed, never built)

The following subsystems from the source design have **no implementation** beyond (at
most) type definitions; each is itemized with line refs in the companion gap ledger
(`gap-ledger-mastery.md`):

- **Content lifecycle / right to be forgotten** — types-only
  (`app/lamad/src/app/models/content-lifecycle.model.ts` exists and is exported via
  `models/index.ts:157`, but no `ContentLifecycleService`, no consumer, and `ContentNode`
  carries no `lifecycle`/`contentVersion` fields).
- **Graph-relative freshness** — `graphEvolutionFactor` and activity-relative refresh
  unimplemented; the API accepts `contentVersionAtMastery`
  (api/mastery.rs `InitializeMasteryRequest`) but there is no current-version source to
  compare against.
- **Expertise discovery** — types-only
  (`app/lamad/src/app/models/expertise-discovery.model.ts`; not exported from
  `models/index.ts`; no service). Tracked in
  `genesis/data/timeline/backlog/lamad-expertise-discovery-integration.md`.
- **Upper-level participation features** (analyze comments, evaluate peer review, create
  contribution flows), `MasteryQuizComponent`, `FreshnessAlertComponent` — no such
  components exist in `app/lamad/src/app/components/`.
- **Privilege suspension on decay** — `ContentPrivilege.suspendedReason` exists in the
  model (content-mastery.model.ts:160-166) but no code path sets it.
- **attestation:mastery auto-minting policy** — see §7.
- Source design's own open questions (gate flexibility per content type, domain-variant
  decay curves, cross-platform mastery credit, assessment anti-gaming) remain open —
  BLOOM-MASTERY-DESIGN.md:999-1009.
