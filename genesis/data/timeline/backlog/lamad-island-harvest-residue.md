---
id: "backlog-lamad-island-harvest-residue"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Lamad island harvest residue — durable constraints + vision fragments awaiting their homes"
slug: "lamad-island-harvest-residue"
written: "2026-06-11"
author: "lamad island recompose (handoff + api-spec harvest agents, code-verified)"
status: "backlog"
priority: "medium"
tags: [lamad, island-recompose, harvest, a2o-candidates, constants-extraction]
cites:
  - app/lamad/CLAUDE.md
  - genesis/docs/content/elohim-protocol/lamad.md
---

# Harvest residue from the retired lamad island

Durable constraints + still-true vision fragments extracted (code-verified) from the retired
`app/lamad/docs/{UI-API-HANDOFF,INSTRUMENT_AGGREGATION_SPRINT_NOTES,LAMAD_API_SPECIFICATION_v1.0}.md`
(git history). Each item names its proposed home; landing them is this entry's work.

## Part A — handoff harvest (durable engineering constraints)

*Harvested 2026-06-11 from `app/lamad/docs/{UI-API-HANDOFF,INSTRUMENT_AGGREGATION_SPRINT_NOTES}.md`
(retired to git). Evidence anchors: `path.service.ts`, `agent.service.ts`, `assessment.service.ts`,
`services/MIGRATION.md`, `src/app/claude.md`.*

Both source files RETIRE (artifact_kind handoff / run-output → retention git). The
short list below is what survives. Everything not listed (Khan chip mock-ups, component
code samples, 3,397-node / 98.4%-coverage data stats, sprint API tour) is superseded
sprint mechanics — drop with the file.

## 1. `lamad-progress-{agentId}-{pathId}` localStorage key is a cross-bundle contract
- **Constraint:** the shell app and the lamad EPR-app bundle independently read/write
  the same localStorage key shape; a unilateral key change in either bundle silently
  forks Traveler progress.
- **Evidence:** `app/elohim-app/src/app/elohim/services/data-loader.service.ts:791,815,837`
  and `app/lamad/src/app/services/data-loader.service.ts:801,825,847` (same template
  string in two codebases); also `app/elohim-app/src/app/elohim/services/agent.service.ts:132,566`.
- **Proposed home:** gospel one-liner in `app/lamad/src/app/claude.md` (bundle-side) —
  "progress localStorage keys are a shared contract with the shell; key shape changes
  must land in both bundles."

## 2. `__global__` reserved pseudo-pathId (cross-Journey Territory completion)
- **Constraint:** global content completion lives in a progress record with reserved
  `pathId: '__global__'` carrying `completedContentIds`; every consumer that enumerates
  path progress MUST filter it out, and cross-journey state derives as
  `completedInOtherPath = isCompletedGlobally && !completedInThisPath`.
- **Evidence:** write/read sites `app/elohim-app/src/app/elohim/services/agent.service.ts:379-541`;
  filter sites `agent.service.ts:655-656` and
  `app/lamad/src/app/components/lamad-home/lamad-home.component.ts:154`; derivation
  `app/lamad/src/app/services/path.service.ts:451,499`. Documented only in
  `app/lamad/src/app/services/MIGRATION.md:15,23` (which stays with the services, not
  the retiring docs island).
- **Proposed home:** a2o scenario candidate — "content completed in one Journey shows
  as mastered in another Journey referencing the same node." `grep -rl
  'cross-journey\|completedInOtherPath\|mastered in other' genesis/a2o/features` = 0
  hits today, so this behavior has no regression scenario.

## 3. Cross-Journey completion has TWO denominators (steps vs unique Territory)
- **Constraint:** `getPathCompletionByContent()` returns both
  `stepCompletionPercentage` (steps in THIS journey) and
  `contentCompletionPercentage` (unique Territory nodes, counting nodes mastered
  anywhere). UI that shows only step % understates a returning Traveler's standing.
- **Evidence:** `app/lamad/src/app/services/path.service.ts:355` (API),
  consumed by `app/lamad/src/app/components/path-overview/path-overview.component.ts`.
  The *principle* ("mastery accrues against content, not against a path") is already
  homed at `app/lamad/src/app/claude.md` Philosophy section (lines ~46-65); the
  two-denominator wire shape is not stated anywhere outside code.
- **Proposed home:** same a2o scenario as #2 (one scenario can assert both numbers).

## 4. Affinity/score band thresholds 0.4 / 0.7 are scattered magic numbers
- **Constraint (parameter-bearing):** the 0–0.4 encountered / 0.4–0.7 growing /
  0.7–1.0 high-affinity banding from the handoff is live but encoded as unrelated
  literals: `hierarchical-graph.service.ts:684,701` (>=0.4 → in-progress),
  `agent.service.ts:783` (>0.7 → highAffinityPaths),
  `sophia-renderer.component.ts:1099` + `perseus-renderer.component.ts:765` (>=0.7
  passing), `content-mastery.service.ts:278`, `mastery-visualization.ts:288-298`
  (0.7 fresh / 0.4 stale). The retiring handoff was the only place declaring it as
  one system of bands.
- **Proposed home:** backlog (small) — extract shared affinity-band constants; until
  then this note is the only cross-site record.

## 5. ProgressMigrationService exists but was never wired
- **Constraint/gap:** `migrateAllProgress()` is not called from any component
  (`grep -rn 'migrateAllProgress' app --include='*.ts'` hits only the service + spec);
  the guard key `lamad-migration-v1-completed` appears only in
  `app/lamad/src/app/services/MIGRATION.md:112`, never in code. The handoff's
  "Option 2: One-Time Startup (Recommended)" was never implemented.
- **Proposed home:** backlog candidate — either wire the startup migration or declare
  the rail dead (localStorage-MVP progress is slated for substrate-backed storage).
- **OPEN QUESTION:** is localStorage progress migration still wanted at all, or moot
  once progress moves to source-chain/storage projection?

## 6. Instrument-aggregation open questions are genuinely unhomed
- **Constraint:** sprint-notes attestation types `discovery-subscale`,
  `reflection-insight`, `psychometric-profile` were never implemented (0 hits in
  app/ or elohim/sdk). The models DID land
  (`app/lamad/src/app/models/knowledge-map.model.ts` — SelfKnowledgeMap,
  SelfKnowledgeLink, GiftCategory;
  `app/lamad/src/app/quiz-engine/services/longitudinal-tracking.service.ts`), and the
  psyche bridge methods exist in sophia
  (`sophia/packages/psyche-survey/src/recognize-resonance.ts`, sophia-core/index.ts).
  The four open questions (sprint notes lines 44-48) remain undecided and appear
  nowhere else — only a passing psychometric mention at
  `genesis/docs/content/elohim-protocol/architecture/imagodei-surfaces-design.md:58`.
- **Proposed home:** backlog candidate carrying the four questions verbatim
  (psychometric privacy never-DHT/source-chain-only; on-chain vs ephemeral reflection
  interpretations; validity thresholds for attestation minting; instrument version
  comparability). These are p2p-design-gate inputs when instrument aggregation revives.
- **OPEN QUESTION:** privacy classification of psychometric results (source-chain-only
  vs DHT) — must be answered via p2p-design-gate before any attestation minting lands.

## 7. Assessment localStorage key correction (supersedes sprint-notes claim)
- The sprint notes' pattern `lamad-assessment-{instrumentId}-{sessionId}` is WRONG vs
  current code. Actual shapes (`app/lamad/src/app/services/assessment.service.ts:103,411,435,455`):
  prefix `lamad-assessment-` + `session-{agentId}-{assessmentId}` /
  `result-{agentId}-{assessmentId}` / `attestations-{agentId}`.
- **Disposition:** drop (code is authoritative); recorded here only so the retiring
  doc's stale pattern isn't re-harvested later.

## Already homed elsewhere → DROP (no action)
- **psyche-core must NEVER depend on perseus packages** — root `/projects/elohim/CLAUDE.md`
  Code Style section, Sophia bullet. Verbatim present.
- **Sophia = rendering layer only; aggregation/interpretation in lamad services** —
  root CLAUDE.md "Sophia Integration" section.
- **Territory/Journey/Traveler terminology + "learner not user"** —
  `app/lamad/src/app/claude.md` Philosophy section + line 210.
- **Lazy loading / fog-of-war / Territory-vs-Journey load-bearing constraints** —
  `app/lamad/src/app/claude.md` "Three load-bearing constraints".
- **AgentService location claim in handoff is stale** — it lives at
  `app/elohim-app/src/app/elohim/services/agent.service.ts` (elohim pillar owns
  cross-pillar services per root CLAUDE.md), not `src/app/lamad/services/` as the
  handoff says. Supersession evidence, not a constraint.

## Part B — API-spec v1.0 vision fragments (proposed home: genesis lamad.md §inspirations or the mastery seed)

*Harvested 2026-06-11 from `app/lamad/docs/LAMAD_API_SPECIFICATION_v1.0.md` (2025-11-27, retired to git).
These fragments are lamad-lens VISION material — they assume (and must cite, never restate) the shared EPR
substrate: the 8-value Reach enum, the lamad manifest, the universal `/epr/{id}` address, and the
reach-earning machinery. None of them may be read as wire-contract or vocabulary authority.*

Only 5 fragments survived the scan. Everything else in the 2,538-line file is
superseded (per-Part dispositions in the harvest summary, not repeated here).

---

## FRAGMENT 1 — Khan Academy "World of Math" inspiration mapping

**Source:** spec lines 20–24 (Part 0, Conceptual Inspirations).
**Land at:** the mastery seed (vision preamble for orientation/mastery).
**Still true:** the mapping's targets all exist — `epic` is a live manifest
content type (elohim/sdk/domains/lamad/manifest.json `vocabulary.contentTypes.epic`),
mastery is Bloom's-taxonomy progression (`content-mastery.model.ts` per
app/lamad/src/app/claude.md Models table), recommendation exists
(app/lamad/src/app/services/path-recommendation.service.ts).
**Not homed:** "World of Math" / Khan Academy appears ONLY in this docs island
(grep hits: LAMAD_API_SPECIFICATION_v1.0.md, IMPLEMENTATION_PLAN.md,
IMPLEMENTATION_ARCHIVE.md — all app/lamad/docs/). The canonical vision doc
genesis/docs/content/elohim-protocol/lamad.md never mentions it.

> Lamad's orientation model adapts Khan Academy's "World of Math": a target
> subject represents the mastery goal (for the reference deployment, "The
> Elohim Protocol" stands where Khan puts mathematical proficiency). What Khan
> calls categories, lamad calls **epics** — major domains of understanding.
> Individual content nodes serve the role of skills — atomic units to be
> mastered. The mastery-challenge idea — "your next recommended skill is
> computed from demonstrated progress" — becomes orientation: the system
> tracks the journey toward the target subject and surfaces what the learner
> is ready for next.

---

## FRAGMENT 2 — Zelda: Breath of the Wild fog-of-war inspiration (the WHY)

**Source:** spec lines 34–38 (Part 0) and line 2179 ("The fog-of-war principle
is architectural, not just optimization").
**Land at:** the mastery seed (rationale paragraph), or a one-line rationale
addition to app/lamad/src/app/claude.md §"Three load-bearing constraints".
**Still true:** fog-of-war is implemented and load-bearing —
app/lamad/src/app/claude.md:64 ("a learner sees completed, current, or next
step only"), app/lamad/src/app/services/path.service.ts:63/130/190
(`maxAccessible = maxCompleted + 2`).
**Not homed:** the implementation carries the RULE but not the rationale. The
Zelda/Sheikah-Tower framing and the "cognitive respect" justification exist
nowhere outside this spec (grep for Zelda/Sheikah across repo: only this file).

> The inspiration is Breath of the Wild's map: Sheikah Towers are visible from
> a distance — orientation toward goals — but surrounding terrain stays
> obscured until you make the journey and climb. Access is earned through
> demonstrated capability; once earned, the map reveals progressively.
> Applied to knowledge: you can SEE that advanced concepts exist (orientation),
> but detail stays gated until the foundation is built. This is not artificial
> scarcity — it is cognitive respect and pedagogical wisdom encoded into the
> architecture, protecting learners from overwhelm while enabling discovery.

---

## FRAGMENT 3 — Fog of war as an AI-agent constraint (graph traversal is costly by design)

**Source:** spec lines 175–177 (Part 1.1 critical constraint) and lines
1864–1868 (Part 5.3, Agent Autonomy with Constraints).
**Land at:** lamad-domain gospel (elohim/sdk/domains/lamad/CLAUDE.md) or the
mastery seed — it is a domain design commitment, not app trivia.
**Still true:** ExplorationService implements exactly this —
app/lamad/src/app/services/exploration.service.ts:33 ("exploration is
intentional, not casual"), :34 ("All queries have visible computational cost
and are rate-limited based on attestations"), :67
(`TIER_ADVANCED_RESEARCHER`), rate-limit state at :73–:138.
**Not homed:** the code comment carries the mechanism; the design COMMITMENT —
that this constraint binds AI agents identically to humans — appears in no
current doc.

> Making graph traversal expensive is intentional, not an optimization gap.
> Even a superintelligent AI agent cannot "see" the entire knowledge graph
> without paying the computational cost of walking it step by step with
> declared purpose. Elohim agents that generate paths face the same
> constraints as human researchers: declare purpose before traversing,
> acknowledge estimated cost before executing, accept rate limits, and leave
> an audit trail. Exploration requires intentionality rather than being a
> free zero-cost operation — for every class of agent.

(OPEN QUESTION: the spec's specific quotas — 10 depth-1/hour, 25 depth-2/hour,
5 pathfinding/hour — were not verified against exploration.service.ts
constants; treat the numbers as code-owned, not residue.)

---

## FRAGMENT 4 — Elohim vs Lamad: the actors and the medium

**Source:** spec lines 54–66 (Part 0, Terminology Distinction).
**Land at:** lamad-domain gospel (a short "what lamad is NOT" boundary
paragraph) — or a history record if the gospel stays terse.
**Still true:** consistent with the current layering — the lamad domain owns
vocabulary/coupling (elohim/sdk/domains/lamad/CLAUDE.md), while agent
intelligence is a separate concern (root CLAUDE.md pillar table: `elohim`
pillar owns cross-pillar services/agents/trust).
**Not homed:** "ghost in the machine" framing appears only here and in an
unrelated FCT doc (genesis/docs/content/fct/Foundations for Christian
Technology.md); the library/living-environment line exists nowhere else.

> Lamad is the maps and paths of meaning: node types, metadata, the graph of
> available knowledge, and the footprints — affinity records of where a
> learner has traveled. Elohim are the active agents that animate it:
> negotiating access against attestations and readiness, tracking learning
> patterns, coordinating goals with available paths. **Without Elohim, Lamad
> is a library. With Elohim, Lamad becomes a living learning environment.**
> Content nodes, path structures, and relationship graphs are Lamad concerns;
> deciding what to show next and when to unlock advanced material are Elohim
> concerns.

---

## FRAGMENT 5 — "Learning builds affinity; teaching earns reach" (the lamad aphorism)

**Source:** spec lines 44–52 (Part 0, Why "Lamad"?).
**Land at:** genesis/docs/content/elohim-protocol/lamad.md ("What Is Lamad?"
section enrichment) — it completes the etymology already homed there.
**Still true:** both halves are live substrate facts — affinity is the lamad
engagement model (genesis/docs/content/elohim-protocol/lamad.md §"Affinity,
Not Mastery"); reach is earned, never self-asserted (elohim/sdk/CLAUDE.md
§"Feedback + governance ARE the reach-earning machinery";
elohim/sdk/schemas/v1/enums/reach.schema.json).
**Not homed:** genesis lamad.md gives the etymology only as 'Hebrew: "to
learn"'; the bidirectional reading and the aphorism appear nowhere else
(grep "learning builds affinity"/"teaching earns reach": zero hits outside
this spec).

> לָמַד (lamad) is a single Hebrew word encompassing both "to learn" and "to
> teach" — both sides of the educational exchange. The name encodes the
> protocol loop: **learning builds affinity** (relationship with content),
> and **teaching earns reach** (the right to guide others). Knowledge flows
> in both directions; neither side is passive consumption.

---

## Explicitly judged ALREADY HOMED (not residue)

- **Gottman Love Maps** — app/lamad/src/app/models/knowledge-map.model.ts:12,
  1052–1134 (PersonKnowledgeMap + Gottman categories, with attribution);
  genesis/docs/content/elohim-protocol/lamad.md §"Three Meaning Maps";
  genesis/data/lamad/knowledge-maps/map-person-template.json:75
  (`"basedOn": "Gottman Love Maps"`); consent model in
  app/lamad/src/app/models/human-consent.model.ts.
- **Territory / Journey / Traveler terminology** —
  app/lamad/src/app/claude.md:46–56 (the trio plus Maps, gospel-tier).
- **"Attention is sacred"** — genesis/docs/content/elohim-protocol/social_medium/epic.md.
- **Bidirectional trust symmetry (content earns reach)** — the entire insight,
  attestation types, stacking, trust score, and Holochain mapping live in
  app/lamad/src/app/models/content-attestation.model.ts:1–60 (header carries
  the symmetry principle verbatim), wired to the protocol Reach enum via
  REACH_LEVEL_VALUES from @elohim/storage-client.
- **Fog-of-war as rule + lazy loading** — app/lamad/src/app/claude.md:62–66;
  app/lamad/src/app/services/path.service.ts:190.
