# Experience-Story EPRs — Design Spec

**Status:** Design (brainstorm complete, awaiting user review)
**Date:** 2026-04-18
**Authors:** Matthew Dowell + Opus 4.7
**Pillar coupling:** lamad (app-schema vocabulary) on top of protocol primitives (ContentNode, Link)
**Depends on:** existing `ContentNode` entry type in lamad DNA; existing `human` / `collective` / `role` / `feature` contentTypes; existing `epr-composite` core protocol contentFormat
**Related:** `rakia/docs/plans/build-attestation-integration.md` (sibling pattern for build attestations); brit Phase 2a (`elohim/brit/docs/plans/2026-04-16-phase-2a-build-attestation-primitives.md`)
**Defers:** sub-project A (matthew peer persistence), C (doorway export/re-upload route), D (shefa valueflow from inter-pipeline diffs) — each gets its own spec depending on this one

---

## 1. Problem

The a2o test suite (47 features / 385 scenarios) produces rich diagnostic artifacts on every run — Playwright traces, browser console captures, doorway-correlated observation reports, screenshots — but those artifacts die with the Jenkins pod. There is no durable, addressable, network-legible record that "Matthew, on commit X, exercised feature Y in role Z and this is what happened." Failures vanish. Successes carry no accumulated value. Diffs between runs are not events the protocol can witness.

The deeper problem: the protocol has no app-schema for representing **lived persona experience as durable, attestable evidence**. We have Content. We have Humans, Roles, Collectives, Features. We have the EPR (`EntityPortalReference`) addressing model. We do not yet have a ContentNode type that joins these into a stable narrative anchor that grows over time — a story whose worth accumulates from witnessed moments.

This spec defines that anchor: the `experience-story` ContentNode and its supporting Tier-2 attestation links and Tier-3 moment records.

## 2. Foundational principle — both signals carry value

Agile rewards "working software." Elohim **witnesses both green and red as legitimate evidence** and lets discernment recognize the worth of each.

A scenario that fails in a way that exposes a real-world constraint is discovery, not defect. A scenario that fails *as expected* validates a failure mode and increases confidence that the system handles it correctly. A scenario that turns green after being red is recovery. A scenario that has been green for 100 runs continues to be baseline-confirmation. A scenario that fails on a brand-new compute context where it has historically passed is structural-validation evidence — *proof* the failure is not environmental flake.

This spec consequently does not use signed numeric story-points. Story-points carry **valence** (the kind of evidence) alongside magnitude. The data model never reduces "what happened" to a single positive/negative scalar; the discernment layer assigns valence + magnitude per attestation, and the rolled-up worth of an `experience-story` is a multi-dimensional vector of recognized contributions.

This is the elohim difference from agile orthodoxy. The vocabulary is borrowed (story, sprint, backlog, velocity); the value function is not.

## 3. Three tiers — what gets durable, where

The design separates three distinct entities with three different durability needs. This separation is what keeps the DHT entry budget viable while still making every moment exportable, queryable, and re-seedable.

| Tier | Entity | Classification (per p2p-design-gate) | Source of Truth | Volume |
|---|---|---|---|---|
| **1** | `experience-story` — the narrative junction | **Notarized (A)** — new contentType on existing `ContentNode` entry type (public visibility) | Holochain DHT | ~50–200 total, ever |
| **2** | `:story-point` — a discerned attestation, dual-emitted as a Link AND as an `EconomicEvent` (existing entry type, REA semantics — see §5.4) | **Derived (A2)** for the link + **Notarized (A)** via existing `EconomicEvent` entry type — no new DNA capacity consumed | Holochain DHT | Bounded by discernment rate, not pipeline rate |
| **3** | `experience-moment` — one persona × one scenario × one run | **Agent-scoped (B)** — private source-chain entry; see §6.1 for entry-type decision | Persona's source-chain → projected to that persona's elohim-storage sqlite | High volume (~770/run) but never gossipped |

Discernment reads Tier 3 (raw moments accumulating on the persona's storage) and decides when to mint Tier 2 (attestation links) on Tier 1 (the durable story).

## 4. Tier 1 — the `experience-story` ContentNode

### 4.1 Identity

An `experience-story` is identified by a **(subject, role, feature)** triple, where:

- **subject** is a reference to an existing ContentNode of contentType `human` OR `collective` (e.g., `human:matthew-manager`, `collective:maintainers`)
- **role** is a reference to a ContentNode of contentType `role` (e.g., `role:as-entrepreneur`, `role:as-supplier`, `role:as-maintainer`)
- **feature** is a reference to a ContentNode of contentType `feature` (the gherkin feature file's canonical entry, e.g., `feature:learning-journey`)

The triple is realized in the link graph, not the entry payload:

```
                    (experience-story:CID)
                   /          |          \
              :hasSubject  :inRole    :exercises
               /              |              \
        ContentNode      ContentNode     ContentNode
       (human OR         (role)          (feature)
        collective)
```

`:hasSubject` / `:inRole` / `:exercises` are typed Holochain Links from the experience-story to existing ContentNodes. The triple is its identity — uniqueness enforced by the link composition. Two experience-stories with the same triple are the same story.

### 4.2 Address strategy

- **Canonical address:** Holochain `EntryHash` (CIDv1) of the experience-story ContentNode itself.
- **Human-readable EPR alias:** `epr:experience-story/{subject-slug}/{role-slug}/{feature-slug}` resolved by the existing `epr-resolver.service.ts` to the latest version of the corresponding experience-story.
- **Versioning:** experience-story ContentNodes are mostly stable; minor metadata revisions emit a new CID linked via `:previousVersion` per existing protocol convention.

### 4.3 Payload (lean)

The experience-story ContentNode carries minimal data — most context lives in its links and attestations.

```yaml
contentType: experience-story
contentFormat: epr-composite
title: "Matthew's experience-story of Learning Journey, as-entrepreneur"
description: "Witnessed evidence of human:matthew-manager exercising
              feature:learning-journey in role:as-entrepreneur."
content: |
  # Subject: human:matthew-manager
  # Role: role:as-entrepreneur
  # Feature: feature:learning-journey
  #
  # This story accumulates witnessed moments and recognized attestations.
  # Resolve the link graph to read its history.
tags: [experience-story, "@as-entrepreneur"]   # category tags for discoverability
relatedNodeIds:
  - human:matthew-manager
  - role:as-entrepreneur
  - feature:learning-journey
```

Pillar coupling: `experience-story` (Tier 1) and `experience-moment` (Tier 3) are added to `elohim/sdk/domains/lamad/manifest.json`'s contentTypes list. No new core protocol enum value is required. The link type `:story-point` (Tier 2) and the subject/role/feature link types (`:hasSubject`, `:inRole`, `:exercises`) are added to the lamad integrity zome's `LinkTypes` enum.

## 5. Tier 2 — the `:story-point` attestation link

### 5.1 What it is

A typed Holochain Link from an `experience-story` (Tier 1) to an `experience-moment` (Tier 3, on the recording persona's source chain), with metadata in the link tag. The link IS the attestation; the tag carries the discernment output.

### 5.2 Tag schema (≤ 256 bytes target)

```json
{
  "v": 1,
  "valence": "progress",
  "magnitude": "meaningful",
  "evidenceType": "first-pass-green",
  "computeFingerprint": "matthew-alpha:device-family-node-base:abc123",
  "runId": "pipeline-42",
  "commit": "abc123def",
  "momentEntryHash": "uhCEk...",
  "discernerId": "discernment-service-v1-mechanical"
}
```

### 5.3 Valence vocabulary (the value function)

Story-points carry one of these valences. None is "negative." All are recognized contributions to the story's worth, weighted by `magnitude` (`small | meaningful | significant`). A regression is value too: knowing the system moved backward is worth knowing.

| Valence | When it fires | Example |
|---|---|---|
| `progress` | The system can now do something it couldn't before, or does it better. | First-time green; performance threshold newly met. |
| `discovery` | The system surfaced previously-unknown behavior — typically a failure exposing a real constraint. | New error class on a previously-green scenario; a flake-class failure proven deterministic on a specific compute context. |
| `regression` | The story moved backward — a known-cause failure recurs on a scenario that had been passing, or a trend reverses. | A scenario that flipped green→red with a previously-seen error class; a recovery that itself reverts. |
| `validation` | An expected failure mode confirmed itself. The system is correctly refusing to do something it should refuse. | A `@validates-failure-mode` scenario fails as designed under unrelated commit changes. |
| `witness` | A **different** compute context (different fingerprint) attests the same status — independent re-validation. Direct input to brit/rakia reach computation. | Adam-on-laptop's run produces the same green result Matthew-on-edge-node already attested; or another fingerprint confirms a previously-isolated failure as structural. |
| `refinement` | The **same** compute context produces a richer or improved attestation — better performance, more captured evidence, more detail. | A scenario that was barely green now passes with 30% timing headroom; a moment whose new sidecar (e.g., trace) reveals previously-invisible behavior. |
| `confirmation` | Steady-state baseline — the system continues to behave as it has, on the same compute context, with no improvement. Lowest magnitude. | Same scenario green for the 100th run on the same fingerprint with no measurable change. |

`magnitude` modulates each: a `confirmation/small` is the cheapest attestation; a `discovery/significant` or `regression/significant` carries the most weight.

### 5.4 REA mapping — `:story-point` IS an EconomicEvent

The `:story-point` attestation is not just a structural link; it is a Resource-Event-Agent event in the protocol's REA economy. Co-emitted with the link, an `EconomicEvent` (existing DHT entry type, see `app/elohim-app/src/app/shefa/`) carries the value-flow semantics:

| REA dimension | This design |
|---|---|
| **Resource** | The `experience-story` (the asset whose value accumulates). |
| **Event** | The attestation itself. |
| **Provider (agent)** | The persona whose moment fed the discerner (e.g., `human:matthew-manager`) — the one whose validating work produced the evidence. |
| **Receiver (agent)** | The subject of the experience-story (often the same persona; for `collective` subjects, the collective). |
| **Action** | `produce` for `progress` / `discovery` / `refinement`; `cite` for `witness` / `confirmation`; protocol's existing action vocabulary covers all valences. |
| **Quantity** | `magnitude` mapped to a numeric value (small=1, meaningful=3, significant=8 — the agile fibonacci familiarity is intentional, but the unit is *attested story-points*, not estimated points). |
| **lamadEventType** | `experience-attestation` (new value to add to lamad EconomicEvent type vocabulary). |

So Tier 2 has a **dual representation**:

1. **Structural link** (`:story-point` from experience-story to experience-moment) — cheap, queryable via Holochain link traversal, used by the dashboard to walk the evidence trail.
2. **EconomicEvent entry** (existing DHT entry type) — feeds shefa's REA economy. Sub-project D consumes the EconomicEvent stream to compute valueflows, velocity per persona/role, story-worth aggregates, and reach.

Both are minted in the same coordinator call. They share `momentEntryHash`, `valence`, `magnitude`, and `evidenceType` metadata. Neither duplicates the other's role: the link is the graph edge, the event is the value flow.

### 5.5 Discernment as a gateable, stubbable seam

Per the project's broader pattern (most elohim services are stubbed pending sophisticated implementations), the v1 mechanical discerner (§7) is itself a stub: a pure-function gate that fires deterministically given a moment and prior attestations. The real architecture is the **gate**, not the v1 implementation. Tests should mock the discernment call directly with explicit `(moment, priorAttestations) → Optional<StoryPointTag>` fixtures, exercising each valence at each magnitude. When sophisticated discernment lands (sophia-mediated, steward-curated, or REA-weighted), the gate doesn't move — the implementation behind it does.

### 5.4 Compute fingerprint

The fingerprint identifies the validating compute environment. Diversity of fingerprints attesting the same experience-story is the protocol's natural reach signal — it ports directly into brit/rakia's reach computation (`self → community → public`).

```
{pod}:{deviceArchetype}:{archetypeRevisionHash}
```

Same archetype across many pods adds magnitude but not diversity. Different archetypes attesting the same story is what compounds reach.

### 5.5 The discernment interface

This spec defines what an attestation **carries** but is deliberately silent on **who decides** to mint one. The interface is:

```
input:  experience-moment + recent-history-of-this-experience-story
output: Optional<{valence, magnitude, evidenceType}>  // None = "do not attest this moment"
```

A v1 mechanical discernment ships in this spec (Section 7); a sophisticated discernment service (sophia-mediated, REA-weighted, steward-curated) is a follow-on spec with the same interface.

## 6. Tier 3 — the `experience-moment` composite

### 6.1 What it is

One persona's first-person recording of one scenario at one moment in time. Lives as a private source-chain entry on the recording persona's Holochain agent, projected into that persona's local elohim-storage sqlite for query convenience. Never gossipped to the public DHT.

The composite shape is **M1** from the brainstorm: living narrative document for humans, sidecar JSON/CSV blobs for machines, pulled on demand.

**Entry-type decision (flagged as open question #6):** The lamad `ContentNode` entry type is currently defined with public visibility. Two options for Tier 3:

- **Option α (recommended, simpler):** Reuse existing `ContentNode` with `contentType: experience-moment` and add it to the lamad manifest. This requires adding **private visibility** as a variant on `ContentNode` in the lamad integrity zome — either a new entry type `PrivateContentNode` with identical schema, or visibility as a runtime field. Costs one integrity-zome revision.
- **Option β (cleaner isolation):** Define a new private-only entry type `ExperienceMoment` in lamad integrity zome. Costs one of lamad's ~27 remaining entry-type slots but keeps ContentNode semantics untouched.

Implementation plan will pick one; the shape of the composite document is identical either way.

**Multi-subject scenarios:** When a single a2o scenario exercises multiple personas (e.g., "the six protocol humans"), the a2o framework emits **one experience-moment per participating persona**, each recorded on that persona's own source-chain. The six moments share a common `runId` and common scenario reference, but each is first-person ("Matthew's experience of this scenario," "Jessica's experience of the same scenario"). Each feeds its own subject's `experience-story` for that role+feature combination.

### 6.2 The narrative document (`epr-composite` contentFormat)

```markdown
---
schema: experience-moment/v1
recordedAt: 2026-04-18T14:32:11Z
subject:
  ref: human:matthew-manager
  archetype: device-family-node-base
role: role:as-entrepreneur
feature: feature:learning-journey
scenario:
  name: "Welcome flow loads in under 2s"
  uri: features/lamad/learning-journey.feature
  line: 47
  tags: ["@e2e", "@lamad", "@regression"]
status: passed | failed | pending | skipped
duration_ms: 1842
commit: abc123def
runId: pipeline-42
computeContext:
  pod: matthew-alpha
  namespace: elohim-alpha
  deviceArchetype: device-family-node-base
  archetypeRevisionHash: abc123
  cpu: 1000m
  memory: 2Gi
artifacts:                                # blob refs, not embedded
  cucumber: blob:bafkrei.../cucumber-excerpt.json
  observation: blob:bafkrei.../observation.json
  screenshot: blob:bafkrei.../FAIL-...png  # only on failure
  trace: blob:bafkrei.../trace.zip         # only when E2E_TRACE=true
relatedExperienceStory: epr:experience-story/matthew-manager/as-entrepreneur/learning-journey
---

## Gherkin (the scenario as it ran)

Feature: Learning Journey
  As a returning learner with an active path
  I want to resume where I left off

  Scenario: Welcome flow loads in under 2s
    Given Matthew is logged in on doorway "alpha"
    When Matthew navigates to "/welcome"
    Then the dashboard loads within 2000ms
    And the active path card appears

## Observation summary

3 errors observed: 1 from doorway, 2 from elohim-storage.
Severity: 0 critical, 3 warnings.
[See observation.json sidecar for detail.]
```

The narrative document is what humans read. The sidecar JSON/CSV blobs are what machines parse and what the `experience-moment` projection table indexes for query.

### 6.3 Sidecar artifacts

Sidecars are independent blobs, content-addressed (CID), referenced from the moment's frontmatter. They are stored in the persona's local blob store (or elohim-storage's blob projection of it). Pull-on-demand: the moment is small, the sidecars are fetched only when needed (e.g., when the user opens the experience-moment in the dashboard, or when discernment needs the observation report to classify valence).

| Sidecar | Format | When emitted |
|---|---|---|
| cucumber-excerpt.json | JSON (cucumber-report.json slice) | Always |
| observation.json | JSON (doorway observation report) | Always when observation session was active |
| FAIL-*.png | PNG | On failure |
| trace.zip | Playwright trace ZIP | Only when `E2E_TRACE=true` |
| console-errors.json | JSON | When console.error or page error occurred |

### 6.4 Storage projection (matthew-alpha's sqlite)

A new table `experience_moments` in the persona's elohim-storage sqlite. Source of truth: the persona's source-chain. The sqlite row is a denormalized projection for query convenience.

```
experience_moments:
  source_chain_action_hash  TEXT NOT NULL    -- the source-chain entry hash; identity
  recorded_at               TIMESTAMP NOT NULL
  subject_ref               TEXT NOT NULL
  role_ref                  TEXT NOT NULL
  feature_ref               TEXT NOT NULL
  scenario_name             TEXT NOT NULL
  scenario_uri              TEXT NOT NULL
  scenario_line             INTEGER
  status                    TEXT NOT NULL    -- passed | failed | pending | skipped
  duration_ms               INTEGER
  commit_sha                TEXT NOT NULL
  run_id                    TEXT NOT NULL
  compute_fingerprint       TEXT NOT NULL
  artifact_blob_cids        JSONB             -- {cucumber, observation, screenshot, trace, console}
  related_experience_story  TEXT              -- EPR alias
  -- Source of truth: persona's Holochain source-chain
  -- This row is a projection; do NOT mutate without re-deriving from source-chain
```

No `dht_anchor_hash` column — moments are agent-scoped, not on the DHT.

## 7. v1 mechanical discernment — the default that ships with this spec

The mechanical discerner is intentionally simple. It establishes the interface and prevents the system from shipping with discernment as a TODO. Sophisticated discernment is a follow-on spec.

### 7.1 Where the discerner runs and what it sees

The v1 mechanical discerner is a service co-located with each persona's elohim-storage instance (i.e., running on matthew-alpha's pod alongside matthew's storage). It subscribes to the post-commit signal emitted when an `experience-moment` is recorded to that persona's source-chain. It queries the global `experience_stories` / `story_point_attestations` projection (read-only) for prior attestations on the relevant experience-story, then applies §7.3 rules. A positive decision calls `experience_story::attest` on matthew's conductor, which writes the Tier-2 link.

This keeps v1 deliberately local — each persona discerns its own evidence. Sophisticated discernment (follow-on spec) may involve cross-persona peer review, sophia-mediated scoring, or steward ratification; those require a different deployment topology.

### 7.2 Inputs

For each new `experience-moment` that the discerner sees on a persona's storage:

- The moment itself (status, scenario, compute fingerprint, error class if failed, sidecar manifest, duration)
- The most recent attestation (if any) on the same `experience-story` from any compute fingerprint
- The most recent attestation (if any) on the same `experience-story` from this moment's compute fingerprint
- The set of error classes ever seen on prior attestations for this experience-story (for novel-vs-recurring classification in rule 2)

### 7.3 Decision rules (in order)

```
1. If status == passed AND no prior attestation on this story exists:
     valence=progress, magnitude=meaningful, evidenceType=first-pass-green
     → MINT

2. If status == failed AND prior attestation was passed (any fingerprint):
     If error class is new (not seen in prior attestations on this story):
       valence=discovery, magnitude=meaningful, evidenceType=novel-failure-class
     Else:
       valence=regression, magnitude=meaningful, evidenceType=known-cause-recurrence
     → MINT

3. If status == failed AND scenario tags include @validates-failure-mode:
     valence=validation, magnitude=meaningful, evidenceType=failure-mode-confirmed
     → MINT

4. If status == passed AND prior attestation was failed (any fingerprint):
     valence=progress, magnitude=meaningful, evidenceType=recovery
     → MINT

5. If status matches prior attestation status AND compute fingerprint is new
   for this experience-story (a different validator):
     valence=witness, magnitude=meaningful, evidenceType=cross-fingerprint-attestation
     → MINT

6. If status matches prior attestation status, fingerprint matches prior
   attestation's fingerprint, AND moment carries materially richer evidence
   (new sidecar artifact present that wasn't on prior attestation, OR duration
   improvement >20%, OR new captured detail):
     valence=refinement, magnitude=small, evidenceType=evidence-enriched
     → MINT

7. Else:
     → DO NOT MINT  (steady-state — the moment is recorded on the persona's chain
                     but no DHT attestation is added; baseline confirmation is
                     implicit in the absence of an attestation, not a positive event)
```

Note that `confirmation` valence is reserved for cases where the discernment service has explicit reason to mark steady-state (e.g., a periodic baseline-snapshot triggered by a scheduler rather than by a moment event). The v1 mechanical rules do not mint confirmation attestations — silence is the steady-state signal. Sophisticated discernment may add confirmation rules later.

### 7.4 Why these rules survive the DHT budget

Steady-state runs (rule 7) mint zero attestations. A pipeline that runs hourly with no changes burns zero DHT entries. Attestation rate scales with *meaningful change rate*, not pipeline rate. With the dual-emission model (each attestation = 1 link + 1 EconomicEvent entry), realistic change cadence (a few flips, a couple of new fingerprints, occasional refinements per day) yields roughly 2× the link count in EconomicEvent entries. Even so, this stays in the low thousands per year — within the ~3000-entry total budget after accounting for ~150 experience-stories — and the EconomicEvent entry type already exists, so no new DNA capacity is consumed by the dual emission.

### 7.5 What this spec does NOT decide about discernment

- Whether sophia is invoked to score evidence value (follow-on)
- Whether human stewards ratify high-value attestations (follow-on)
- How REA economic events emitted here aggregate into shefa valueflows (follow-on — sub-project D)
- How recurring `regression/known-cause-recurrence` attestations get pruned or rolled up (follow-on — operational concern; flagged as open question)
- Confirmation-as-positive-event semantics (deferred — silence is steady-state in v1)

## 8. HTTP routes (designed last, per p2p-design-gate)

Two routes are introduced by THIS spec; sub-project C will introduce more for export/re-upload.

```
GET /api/v1/experience-stories/{epr-alias-or-cid}
  → { story metadata, link graph snapshot (subject/role/feature),
      latest attestations (paginated), summary stats by valence }

GET /api/v1/experience-stories
  → ?subject=&role=&feature=&valence=&since=
  → paginated list of experience-stories matching the filter
```

Both serve the **storage projection**, not the source of truth. The projection is denormalized in the global elohim-storage sqlite (NOT the persona-local one — the global DHT projection holds Tier 1 + Tier 2 cross-persona). Projection invariant: `dht_anchor_hash NOT NULL` on the `experience_stories` and `story_point_attestations` tables.

(Routes for fetching individual experience-moments and their sidecar artifacts belong to sub-project C — they cross the agent-scope boundary and require export-flow design.)

## 9. Coordinator zome functions

```
lamad_coordinator::experience_story::
  create_or_get(subject_ref, role_ref, feature_ref) -> EntryHash
    // Idempotent: same triple → same EntryHash
  attest(experience_story: EntryHash,
         experience_moment: EntryHash,
         tag: StoryPointTag) -> AttestationHandles
    // Dual-emit: creates the typed Link (Tier 2 structural) AND the
    // EconomicEvent entry (Tier 2 REA) atomically. Returns both hashes.
    // AttestationHandles { link_action_hash, economic_event_entry_hash }
  list_attestations(experience_story: EntryHash) -> Vec<Link>
  get(experience_story: EntryHash) -> Option<ExperienceStory>

lamad_coordinator::experience_moment::
  record(moment_payload: MomentPayload) -> EntryHash
    // Private source-chain entry; emits post-commit signal for storage projection
  list_my_moments(experience_story: EntryHash) -> Vec<EntryHash>
    // Caller's own source-chain only
```

Post-commit signal handlers in elohim-storage project Tier 1 to the global `experience_stories` sqlite table, and Tier 2 to BOTH `story_point_attestations` (link projection — keeps the structural edge denormalized for graph queries) and the existing `economic_events` table (REA projection — already maintained by shefa). Tier 3 projects to the calling persona's local `experience_moments` sqlite. The two Tier-2 projections share the `momentEntryHash` foreign key for cross-walk; sub-project D consumes the `economic_events` stream.

## 10. Anti-pattern check (per p2p-design-gate)

| Anti-pattern | Status |
|---|---|
| UUID primary key for notarized entity | ✅ Avoided — Tier 1 uses `EntryHash` (CIDv1); Tier 2 uses link `ActionHash`; Tier 3 uses source-chain `ActionHash`. |
| REST route as design starting point | ✅ Avoided — coordinator zome (§9) and storage projections (§4–§6) designed before routes (§8). |
| CID stored as relational FK | ✅ Avoided — link graph is the truth; storage projection denormalizes for query but is regenerable. |
| Standalone table for agent state | ✅ Avoided — Tier 3 lives on persona source-chain; the persona-local sqlite is per-agent, not a shared table. |
| Three address formats undefined | ✅ Resolved — canonical CID + EPR alias for Tier 1; `ActionHash` for Tier 2 + Tier 3; declared in §4.2 and §6.4. |
| Missing source-of-truth declaration | ✅ Each tier in §3 declares source of truth; sqlite migration comments will repeat per CONVENTIONS.md rule. |
| Creating new entry type when one exists | ✅ No new DHT entry types created — Tier 1 uses existing `ContentNode` with new `contentType` (app-schema); Tier 2 uses existing Link primitive; Tier 3 uses existing `ContentNode`. |
| Putting granular data on the DHT | ✅ Tier 3 (granular per-scenario records) stays on persona source-chain. Only Tier 1 (story anchor) and Tier 2 (discerned attestation) gossip publicly. |

## 11. Acceptance criteria

- A `lamad:codegen` run regenerates `manifest-types.ts` to include `experience-story` as a recognized contentType.
- An a2o test scenario can be exercised by `human:matthew-manager` via the existing `PlaywrightDevice`, and on completion the framework records an `experience-moment` to matthew-alpha's source-chain. The moment surfaces in matthew-alpha's `experience_moments` sqlite projection within 5 seconds.
- The v1 mechanical discerner classifies the moment per §7.2 rules and, if a mint is warranted, calls `experience_story::attest`. The resulting Tier-2 link is queryable via `GET /api/v1/experience-stories/{cid}` within 10 seconds.
- A second run of the same scenario on the same compute fingerprint with unchanged status and no enriched evidence produces no new Tier-2 link (rule 7 — steady-state).
- A failure on a scenario tagged `@validates-failure-mode` mints an attestation with `valence=validation`, not `discovery` or `progress`.
- Total DHT entries created across 100 simulated pipeline runs at typical change rate (≤5 flips per run, ≤2 new fingerprints introduced) stays under 1500 — counting both `:story-point` links and their co-emitted EconomicEvent entries.
- The discernment gate is independently mockable: a unit test can call `discern(moment, priorAttestations)` directly with fixture data and assert the returned `Optional<StoryPointTag>` for each of the seven valences (`progress` / `discovery` / `regression` / `validation` / `witness` / `refinement` and the explicit none-case), without standing up Holochain.
- A passing scenario re-run on the same compute fingerprint with materially improved duration (>20% faster) mints a `refinement` attestation; an identical re-run mints nothing (rule 7).
- A passing scenario validated by a previously-unseen compute fingerprint mints a `witness` attestation, not `confirmation`.

## 12. What this spec is NOT (sub-project boundaries)

| Sub-project | Status | Will need its own spec |
|---|---|---|
| **A** — Matthew peer persistence (StatefulSet PVC, Jenkins lifecycle exception) | Pre-requisite for production use of Tier 3 sqlite projections | Yes |
| **C** — Doorway export/re-upload route for experience-moments + sidecar blobs | Operator workflow ("download to workspace, re-seed after wipe") | Yes — depends on this spec for the moment shape |
| **D** — Inter-pipeline diff → shefa valueflow (REA EconomicEvent emission from Tier-2 attestations) | The valueflow consequence | Yes — depends on this spec for the attestation tag schema |
| **Sophisticated discernment** (sophia-mediated, steward-curated, REA-weighted) | Replacement for v1 mechanical (§7) using same interface | Yes |

This spec defines the **data model and the v1 mechanical discernment that must ship with it**. Everything else is downstream and depends on the shapes here.

### 12.1 Cross-cutting meta-concern: compute substrate accounting

The underlying compute consumed to run a2o scenarios — CPU-seconds, memory-seconds, storage bytes, bandwidth on matthew-alpha's pod — is itself a REA resource. The codebase already emits `compute-contribution` EconomicEvents for this (see `app/elohim-app/src/app/shefa/services/compute-event-api.service.ts`). When that substrate is fully wired, each `:story-point` EconomicEvent emitted by this spec should be **linkable** to the compute-contribution events that funded the work — closing the economic loop: compute flows in → evidence value flows out. The protocol can then reason about story-points-per-compute-hour, incentivize diverse validators who contribute compute toward high-value stories, and allocate future compute accordingly.

This linkage is **explicitly out of scope for this spec**. It is a meta-concern at the elohim-protocol core level — where compute and value economies meet — and likely belongs in a future shefa cross-economy spec that touches both substrate accounting and evidence attestation. This spec's coordinator `attest()` function should leave room in its output (perhaps an optional `linkedComputeContributionHashes` field on the EconomicEvent) for that linkage to slot in later without schema-breaking changes.

## 13. Open questions

1. **Role provenance.** Existing `role` ContentNodes — are there enough authored already to cover Matthew's likely contexts (`as-entrepreneur`, `as-learner`, `as-father`)? If not, sub-project A will need to seed them. *(Not blocking this spec; flagged for the implementation plan.)*
2. **Collective subjects.** First test cases use `human` subjects only. The first `collective` subject (e.g., `collective:maintainers as-supplier exercising fair-exchange-protocol`) should be exercised in an integration test before we declare `subject` polymorphism stable.
3. **Pruning policy for `regression/known-cause-recurrence`.** A persistently-flaky scenario could mint a regression attestation per run under rule 2's else branch. Need a follow-on operational policy (probably: aggregate same-class recurrences within a time window, or de-duplicate via debounce on the discerner side).
4. **EPR alias collision strategy.** What happens if a role gets renamed? The slug-based EPR alias would silently shift. Need a stable-slug-derivation rule before this lands. *(Likely: hash of role's CID, not its slug.)*
5. **Witness vs. discerner.** The protocol verb is "witness." This spec uses "discerner" for the v1 mechanical service. Reconcile naming when the sophisticated discernment spec lands — these may collapse into one role with two implementations.
6. **Private ContentNode vs. dedicated private entry type.** Flagged in §6.1. Implementation plan must pick α (reuse ContentNode with private visibility) or β (new `ExperienceMoment` entry type in lamad integrity zome). Affects which migration lands first.
7. **EconomicEvent action vocabulary alignment.** §5.4 maps `progress`/`discovery`/`refinement` → `produce` and `witness`/`confirmation` → `cite`. The mapping for `regression` and `validation` is not yet decided — `regression` could be `decrement` (if such action exists in REA vocabulary) or `produce` with a discriminator field; `validation` arguably is `cite` (citing the failure mode definition). Implementation plan should grep `app/elohim-app/src/app/shefa/` and the storage-client EconomicEvent type to lock the mapping against the existing action enum.
8. **Magnitude→quantity scale.** §5.4 proposes small=1, meaningful=3, significant=8 (fibonacci). This should be a config constant, not hardcoded, so sub-project D can re-tune as the network learns what story-point magnitudes mean operationally.

## 14. References

- `genesis/a2o/CLAUDE.md` — a2o test harness conventions
- `genesis/a2o/src/framework/devices/playwright-device.ts:184–217` — current capture surface
- `genesis/a2o/steps/common.steps.ts:139–222` — observation session lifecycle
- `rakia/docs/plans/build-attestation-integration.md` — sibling pattern for build-state attestations
- `elohim/sdk/domains/lamad/manifest.json` — where the new contentType is declared
- `elohim/sdk/schemas/v1/views/CONVENTIONS.md` — view-schema rules the projection must follow
- `.claude/skills/p2p-design-gate/SKILL.md` — the gate this spec was filtered through
- `.claude/skills/epr-content-addressing/SKILL.md` — EPR resolution model the alias plugs into
- `.claude/skills/rea-economics/SKILL.md` — REA pattern that sub-project D will draw on
