---
status: Draft
---

# Records Lifecycle — Master Orchestration Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete the in-flight records-lifecycle canonical-architecture spec by parallelizing the deep architectural-composition work (Part A primitive walkthroughs + Part B application archetype full-drafts), surfacing concerns the agents discover, then using those findings to finalize the gap-closure plan (Part D) interactively with operators.

**Architecture:** Findings-driven orchestration. Phase 1 dispatches parallel Opus agents to write deep architectural composition (primitives + applications) AND surface bottlenecks, chokepoints, and anti-patterns they discover. Phase 2 synthesizes those findings with operator review. Phase 3 finalizes the Part D substrate-gap plan informed by the findings (the original ten-gap list may grow, shrink, or shift). Phase 4 closes out — Part C placeholders, final commit. Part C composability stress-test is **deferred** — placeholders point to dev-work measurement scenarios that will populate it.

**Tech Stack:** Markdown spec editing with YAML frontmatter contract; reading from Holochain DNA Rust + Diesel migrations + pillar manifest JSON + storage views + bridge crates; parallel agent dispatch via `Agent` tool; findings synthesis via inline operator collaboration.

---

## Why findings-driven (operator's reframe)

The original orchestration had Part D execution running in parallel with the primitive walkthroughs and application archetype drafts. The operator's reframe (during this plan's authoring): **closing substrate gaps requires knowing what gaps actually exist after deep architectural composition exposes them**. Parallel agents writing the primitive walkthroughs and application drafts will surface concerns — bottlenecks, chokepoints, anti-pattern temptations — that the brainstorming conversation didn't catch. Writing Part D before those findings is premature; the gap list will likely shift.

So Part D becomes a **post-findings** plan, finalized only after Phase 1 returns. The scaffolding currently in [`2026-05-24-records-lifecycle-part-d-substrate-gaps-plan.md`](./2026-05-24-records-lifecycle-part-d-substrate-gaps-plan.md) is a starting point that will be revised, not the final.

## Architectural context all dispatched agents must absorb

Any agent dispatched against the sub-plans below MUST first read:

1. [`genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md`](../../content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md) — the foundation; especially §1 (Motivation), §2 (the eight primitives table), §2.1 (the 11-sub-section primitive walkthrough template), and Part A.1 (EPR walkthrough) as the depth/quality exemplar
2. [`genesis/docs/content/elohim-protocol/architecture/applications/mint-monarch-application-design.md`](../../content/elohim-protocol/architecture/applications/mint-monarch-application-design.md) — the application-archetype exemplar
3. [`genesis/docs/content/elohim-protocol/architecture/INDEX.md`](../../content/elohim-protocol/architecture/INDEX.md) — the frontmatter-glue contract and the architecture-vs-sprint-shape distinction
4. [`genesis/docs/content/elohim-protocol/architecture/applications/INDEX.md`](../../content/elohim-protocol/architecture/applications/INDEX.md) — the systems-architect audience framing
5. The relevant epic (per the spec being edited's `realizes:` field) — the why beneath the what
6. The relevant code surfaces (per the spec's "Code anchors" section) — what currently exists in `elohim/holochain/dna/`, `elohim/elohim-storage/`, `elohim/sdk/`, `bridges/`

The conversation that produced these specs framed them as **technical archetypes parallel to the human-story archetypes in `value_scanner/epic.md`**. Each archetype is a *proof* that the substrate's theory holds for a familiar pattern, intelligible to a systems architect fluent in SQL/GraphQL/Kafka/S3/Spring Batch/Redis. The composition is concrete; the math is concrete; the code anchors are concrete; no hand-waving.

Every agent additionally MUST return a **concerns report** alongside their spec content. The concerns report is structured and surfaces:

- **Bottlenecks**: where does this primitive/application risk overwhelming a peer's storage, gossip rate, or compute?
- **Chokepoints**: does this introduce a centralized actor, a single hub that's load-bearing, or a coordination party that becomes a single point of failure?
- **Anti-patterns**: does this conflict with substrate principles ("no new DHT entry types," "reach earned not declared," "no sovereignty — stewardship," "floor permissive," "subsumption-by-merit," "mass-conservation discipline")? Where is the temptation to violate them strongest?
- **Substrate gaps**: does the composition require something the substrate doesn't have yet? (Beyond the 10 gaps already named in the records-lifecycle spec.)
- **Cross-spec drift risk**: does any choice in this archetype/primitive risk conflicting with another archetype/primitive's choice?

The concerns report is the load-bearing output of Phase 1 — at least as important as the spec content. Operators read these reports in Phase 2 to finalize Part D.

## The four sub-plans

| # | Plan | Worker | Phase | Output |
|---|---|---|---|---|
| 1 | [`2026-05-24-records-lifecycle-part-a-primitives-plan.md`](./2026-05-24-records-lifecycle-part-a-primitives-plan.md) | 7 parallel Opus agents | 1 | Spec content (A.2–A.8) + concerns reports |
| 2 | [`2026-05-24-records-lifecycle-applications-plan.md`](./2026-05-24-records-lifecycle-applications-plan.md) | 7 parallel Opus agents | 1 (after #1 lands) | Application full-drafts + concerns reports |
| 3 | [`2026-05-24-architecture-frontmatter-normalization-plan.md`](./2026-05-24-architecture-frontmatter-normalization-plan.md) | 1 parallel Sonnet agent | 1 | Normalized frontmatter on 14 specs |
| 4 | [`2026-05-24-records-lifecycle-part-d-substrate-gaps-plan.md`](./2026-05-24-records-lifecycle-part-d-substrate-gaps-plan.md) | Operators | 3 | Part D spec subsections, revised by findings |

## Phase 1 — Parallel dispatch

**Goal:** Get deep architectural composition into the spec AND surface concerns.

- [ ] **Step 1.1: Dispatch 7 Opus agents against the primitives plan**

```
For (primitive, section_letter) in [
  (Event, A.2),
  (Resource, A.3),
  (Observation, A.4),
  (Commitment, A.5),
  (Attestation, A.6),
  (FeedbackSignal, A.7),
  (Links, A.8),
]:
  Agent(
    description="Write Part <section_letter> <primitive> walkthrough",
    subagent_type="rust-architect",
    prompt="<primitives plan path> — assigned primitive: <primitive>; section: <section_letter>"
  )
```

Each agent returns: (1) Spec content written into records-lifecycle Part A.<N>; (2) structured concerns report.

- [ ] **Step 1.2: Dispatch 7 Opus agents against the applications plan**

```
For app in [khan-academy, google-drive, google-photos, meta-facebook, patreon, requests-offers, aws-compute]:
  Agent(
    description="Upgrade <app> archetype to full-draft",
    subagent_type="rust-architect",  // or general-purpose per the app's primary surface
    prompt="<applications plan path> — assigned application: <app>"
  )
```

Each agent returns: (1) Full-draft replacing composition-draft in `applications/<app>-application-design.md`; (2) structured concerns report.

**Dispatch ordering note:** Steps 1.1 and 1.2 CAN run concurrently — agents writing applications can reference the EPR (A.1) exemplar and the Mint/Monarch exemplar without waiting for the in-flight primitive walkthroughs. If quality concerns arise from cross-citation (e.g., an application archetype references a primitive walkthrough that's still being written), Step 1.2 can wait for Step 1.1 to land. Operator discretion.

- [ ] **Step 1.3: Dispatch 1 Sonnet agent against frontmatter normalization plan**

```
Agent(
  description="Normalize frontmatter on 14 migrated architecture specs",
  subagent_type="general-purpose",
  prompt="<frontmatter plan path>"
)
```

Returns: 14 spec frontmatters normalized; no concerns report needed (mechanical work).

## Phase 2 — Findings synthesis (operator + me)

**Goal:** Aggregate concerns reports across all 14 deep-think returns and identify what they mean for the substrate gap list.

- [ ] **Step 2.1: Read every concerns report**

Each agent's return contains a structured concerns section. Read them in this order:

1. All seven primitive concerns reports (these surface the substrate's most fundamental tensions)
2. All seven application concerns reports (these surface composition-level patterns and cross-archetype drift risks)

Track:
- Concerns that REPEAT across multiple agents (these are the load-bearing ones)
- Concerns that are SPECIFIC to one primitive/application (might be application-layer not substrate-layer)
- Concerns that CONTRADICT each other (force a substrate-design decision)

- [ ] **Step 2.2: Update the gap list**

The original "10 gaps" came from the brainstorming inventory. Findings may:
- ADD gaps (e.g., "every application needs a way to express X but no primitive captures X")
- REMOVE gaps (e.g., "what we thought was a gap actually fits naturally into existing primitive Y")
- REFRAME gaps (e.g., "the dissolution gap is actually two separable concerns: end-of-life and reach-revocation")
- SHIFT priorities (e.g., "bridge pattern is more urgent than re-elevation because three applications demand it")

Produce the revised gap list as a section in the Part D plan (overwriting its current scaffold).

- [ ] **Step 2.3: Categorize chokepoints**

For each chokepoint surfaced by agents, classify it:
- **Substrate-floor chokepoint**: needs Part D gap closure (validation rule, coordinator function, new link type, manifest declaration)
- **Application-layer chokepoint**: can be addressed in the archetype's own design without substrate change
- **Out-of-scope chokepoint**: belongs in a future spec (note and defer)

The substrate-floor chokepoints feed Part D. The application-layer ones may amend the archetype drafts. The out-of-scope ones get logged for later.

- [ ] **Step 2.4: Surface anti-pattern violations**

If any primitive/application's composition violates a substrate principle (most likely: "no new DHT entry types" or "floor permissive"), flag for redesign before Phase 3. These are critical findings — they mean the archetype draft itself needs revision, not just substrate work.

## Phase 3 — Finalize Part D plan with findings

**Goal:** Rewrite the Part D plan informed by Phase 2 synthesis, then execute it interactively.

- [ ] **Step 3.1: Rewrite Part D plan**

Replace the current scaffold in [`2026-05-24-records-lifecycle-part-d-substrate-gaps-plan.md`](./2026-05-24-records-lifecycle-part-d-substrate-gaps-plan.md) with the findings-informed gap list. Each gap subsection still follows the template (motivation, design, code anchors, validation rules, manifest declarations, migration story, test surface) but now reflects what the agents actually surfaced.

- [ ] **Step 3.2: Execute Part D plan inline**

Operator-led. Each subsection lands as its own commit per the Part D plan's task structure.

## Phase 4 — Close out

- [ ] **Step 4.1: Add Part C placeholders**

Replace the current Part C stub with placeholders pointing to:
- The Mint/Monarch + multi-application working set the development sprints are already populating
- The 8B-user back-of-envelope math (already sketched in records-lifecycle §1.1)
- A future plan that closes Part C with actual measured numbers from the dev cluster

Part C scenarios will shake out from real dev work, not from authored fiction. Operator decision per the brainstorming.

- [ ] **Step 4.2: Single commit checkpoint**

```bash
git add genesis/docs/content/elohim-protocol/ \
        genesis/docs/superpowers/plans/2026-05-24-*.md
git status  # verify no stragglers
git commit -m "feat(architecture): records-lifecycle spec landed; primitives + applications full-drafts; gap closure"
```

## Success criteria for the master orchestration

- [ ] All 7 primitive walkthroughs land in records-lifecycle Part A, matching EPR depth, with concerns reports surfaced
- [ ] All 7 application archetypes upgraded from composition-draft to full-draft, matching Mint/Monarch depth, with concerns reports surfaced
- [ ] All 14 migrated architecture-tier specs have normalized frontmatter
- [ ] Findings synthesis produced a revised gap list reflecting deep architectural composition (not just the brainstorming inventory)
- [ ] Part D plan is rewritten informed by findings, then executed inline
- [ ] Part C contains placeholders pointing to dev-work measurement
- [ ] Single coherent commit captures the spec completion
- [ ] No stale references anywhere in the repo

## Risks and mitigations

| Risk | Mitigation |
|---|---|
| Agent quality drift — a walkthrough hand-waves a stress point | Phase 2 review gate; concerns report includes "what I couldn't fully resolve" as a category |
| Agent concerns report is shallow / generic | Plan headers paste the concerns framework (bottlenecks / chokepoints / anti-patterns / gaps / drift); agents follow the structure |
| Agent invents a new primitive | Plan explicitly forbids — compositions must use only the eight primitives; reviewer checks |
| Findings synthesis surfaces conflicting recommendations across agents | Operator decides; document the choice in the Part D plan rationale |
| Frontmatter normalization breaks a YAML parser | Plan instructs: validate each file's frontmatter parses after edit |
| Part C placeholders look like abandoned work | Placeholder text explicitly says "deferred to dev-sprint measurement" with forward pointers |
| The "10 gap" count grows significantly after findings | This is the system working as intended — better to discover gaps now than during implementation |

## Execution handoff

Operator-led orchestration. The dispatch and synthesis steps need operator review at the Phase boundaries:

- After Phase 1 returns: operator reviews concerns reports before Phase 2 synthesis
- After Phase 2 synthesis: operator approves revised gap list before Phase 3 Part D rewrite
- During Phase 3: each Part D subsection lands as its own commit with operator review

Ready to begin Phase 1 dispatches when the operator approves this orchestration.
