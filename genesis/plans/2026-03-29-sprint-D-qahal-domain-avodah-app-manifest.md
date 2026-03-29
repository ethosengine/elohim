# Sprint D: Qahal Domain Manifest + Avodah App Manifest

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create `elohim/sdk/domains/qahal/` for the social + governance domain, and `app/avodah/manifest.json` as an app manifest built on shefa's economic primitives. Qahal is the social layer — governance today, community/social networking tomorrow (inspired by OneBody church CMS meets P2P topology).

**Architecture:** Qahal declares both current governance types (proposals, challenges, deliberation) and future social types (post, event, group, message) in its vocabulary. Future types are declared but marked as unimplemented — the manifest is the contract, implementation follows. Avodah is an app-level manifest that references shefa's domain and adds work management content types.

**Tech Stack:** JSON Schema, Node.js codegen, pnpm

**Parent design:** `genesis/plans/2026-03-29-domain-manifests-sdk-boundary-design.md`

**Reference pattern:** `elohim/sdk/domains/lamad/` (created in Sprint A)

> **P2P note:** Governance types (proposal, challenge, appeal) are Category A (DHT-notarized). Social types (post, event, group) will be Category B (agent-scoped) or A2 (derived via link) when implemented. Work-story and work-project are Category C (operational) stored in projection only. These schemas describe metadata shapes — no new storage tables.

---

## Part 1: Qahal Domain Manifest

### Task 1: Create qahal manifest

**Files:**
- Create: `elohim/sdk/domains/qahal/manifest.json`

**Step 1:** Create the manifest. Qahal owns two vocabularies:

**Governance types (implemented today):**
- `collective` — A community or group. Category A. Coupling: collective creation → governance structure (governance), community presence (value).
- `proposal` — A governance proposal for collective decision. Category A. Coupling: proposal submission → governance-decision signal.
- `challenge` — A challenge to a manifest, content, or decision. Category A. Coupling: challenge filed → constitutional review.
- `appeal` — An appeal of a governance decision. Category A. Coupling: appeal filed → escalation to higher constitutional layer.
- `statement` — A sensemaking statement for Polis-style deliberation. Category B2. Coupling: opinion expression → bridging signal.

**Social types (declared for future implementation):**
- `post` — A social post or status update. Future.
- `event` — A community event (gathering, meetup, service). Future.
- `group` — A named group within a collective (small group, committee, ministry). Future.
- `message` — A direct or group message. Future.
- `thread` — A discussion thread. Future.

For future types, declare the coupling structure (what signals they'll produce) but note `"status": "planned"` in the description. The manifest declares the contract; implementation follows.

Signals:
- `governance-decision` — substrateSignal: compute, economicAction: produce
- `community-report` — substrateSignal: attention, economicAction: produce (flags/reports)
- `challenge-filed` — substrateSignal: compute, economicAction: produce
- `appeal-filed` — substrateSignal: compute, economicAction: produce
- `consensus-reached` — substrateSignal: compute, economicAction: produce
- `social-engagement` — substrateSignal: attention, economicAction: use
- `relationship-formed` — substrateSignal: attention, economicAction: produce

Observations (must include negatives):
- `governance-outcome` — polarity: negative, archetype: outcome-divergence (did the decision help or harm?)
- `social-health` — polarity: negative, archetype: distribution-health (isolation, polarization, attention exhaustion)
- `participation-breadth` — polarity: negative, archetype: outcome-correlation (are the same few people deciding everything?)
- `community-growth` — polarity: positive, archetype: retention-check (is the community retaining members?)

**Step 2:** Validate manifest.

**Step 3:** Commit.

```bash
git add elohim/sdk/domains/qahal/manifest.json
git commit -m "feat(qahal): create social + governance domain manifest with future social types"
```

### Task 2: Create qahal metadata schemas

**Files:**
- Create: `elohim/sdk/domains/qahal/schemas/collective-metadata.schema.json`
- Create: `elohim/sdk/domains/qahal/schemas/proposal-metadata.schema.json`
- Create: `elohim/sdk/domains/qahal/schemas/challenge-metadata.schema.json`
- Create: `elohim/sdk/domains/qahal/schemas/statement-metadata.schema.json`

**Step 1:** Create `collective-metadata.schema.json`:

```json
{
  "$id": "qahal:schema:metadata:collective",
  "title": "CollectiveMetadata",
  "description": "Metadata for contentType 'collective'. Community identity and governance structure.",
  "type": "object",
  "properties": {
    "memberCount": { "type": "integer" },
    "governanceModel": { "type": "string", "description": "steward-consent | community-vote | constitutional | consensus" },
    "constitutionalLayer": { "type": "string", "description": "individual | family | community | provincial | bioregional | global" },
    "geoBoundary": { "type": "string", "description": "H3 cell or place ID for geographic grounding" },
    "parentCollectiveId": { "type": "string", "description": "Parent collective in governance hierarchy" },
    "description": { "type": "string" },
    "visibility": { "type": "string", "description": "private | community | public" }
  },
  "additionalProperties": true
}
```

**Step 2:** Create `proposal-metadata.schema.json`:

```json
{
  "$id": "qahal:schema:metadata:proposal",
  "title": "ProposalMetadata",
  "description": "Metadata for contentType 'proposal'. Governance proposal for collective decision-making.",
  "type": "object",
  "properties": {
    "mechanism": { "type": "string", "description": "consent | ranked-choice | score | approval | plurality | quadratic" },
    "quorum": { "type": "number", "description": "Minimum participation ratio (0.0-1.0)" },
    "deadline": { "type": "string", "description": "ISO 8601 voting deadline" },
    "collectiveId": { "type": "string", "description": "Collective this proposal belongs to" },
    "options": { "type": "array", "items": { "type": "object" } },
    "state": { "type": "string", "description": "draft | open | closed | ratified | rejected" },
    "constitutionalLayer": { "type": "string" }
  },
  "additionalProperties": true
}
```

**Step 3:** Create `challenge-metadata.schema.json`:

```json
{
  "$id": "qahal:schema:metadata:challenge",
  "title": "ChallengeMetadata",
  "description": "Metadata for contentType 'challenge'. Constitutional challenge to a decision, manifest, or content.",
  "type": "object",
  "properties": {
    "targetEprId": { "type": "string", "description": "EPR ID of the challenged artifact" },
    "targetType": { "type": "string", "description": "What is being challenged (manifest | content | decision)" },
    "reason": { "type": "string" },
    "escalationPath": { "type": "string", "description": "Which constitutional layer to escalate to" },
    "state": { "type": "string", "description": "filed | under-review | upheld | dismissed" },
    "filedBy": { "type": "string" }
  },
  "additionalProperties": true
}
```

**Step 4:** Create `statement-metadata.schema.json`:

```json
{
  "$id": "qahal:schema:metadata:statement",
  "title": "StatementMetadata",
  "description": "Metadata for contentType 'statement'. Polis-style sensemaking statement.",
  "type": "object",
  "properties": {
    "polarity": { "type": "string", "description": "agree | disagree | pass" },
    "bridgingScore": { "type": "number", "description": "How well this statement bridges opinion clusters (0.0-1.0)" },
    "clusterAffinity": { "type": "string", "description": "Which opinion cluster this statement belongs to" },
    "deliberationId": { "type": "string", "description": "Parent deliberation/proposal this statement belongs to" },
    "isDivisive": { "type": "boolean", "description": "Whether this statement divides clusters" }
  },
  "additionalProperties": true
}
```

**Step 5:** Wire `metadataSchema` refs into manifest.

**Step 6:** Commit.

### Task 3: Create qahal codegen

**Files:**
- Create: `elohim/sdk/domains/qahal/scripts/codegen.mjs`

**Step 1:** Copy lamad codegen pattern. Output to `app/elohim-app/src/app/qahal/generated/`.

**Step 2:** Produces:
- `metadata-types.ts` — `CollectiveMetadata`, `ProposalMetadata`, `ChallengeMetadata`, `StatementMetadata`
- `coupling-map.ts` — `QAHAL_COUPLING_MAP`
- `manifest-types.ts` — content type lists, signal map
- `content-node-types.ts` — `isCollectiveNode()`, `isProposalNode()` type guards

**Step 3:** Run codegen, verify output.

**Step 4:** Add `pnpm run qahal:codegen` script.

**Step 5:** Commit.

### Task 4: Create qahal CLAUDE.md

**Files:**
- Create: `elohim/sdk/domains/qahal/CLAUDE.md`

Cover:
- Qahal is the social layer, not just governance
- Vision: OneBody (church CMS) meets P2P topology — subsumes Meta/Facebook/LinkedIn
- Current: governance (proposals, votes, challenges, sensemaking, Polis-style deliberation)
- Future: social networking (posts, events, groups, messages, threads) — vocabulary declared, not implemented
- Governance mechanism ladder: graduated feedback → formal proposals → challenges → appeals
- Psephos renderer for formal ballots (levels 3-7), Angular components for casual governance (levels 0-2)
- Key services: CollectiveService, MechanismSelectionService, SignalAccumulationService, BracketSynthesisService

**Commit.**

---

## Part 2: Avodah App Manifest

### Task 5: Create avodah app manifest

**Files:**
- Create: `app/avodah/manifest.json`
- Create: `app/avodah/schemas/work-story-metadata.schema.json`
- Create: `app/avodah/schemas/work-project-metadata.schema.json`

**Step 1:** Create `app/avodah/manifest.json`. This is an APP manifest (not a domain manifest) — it builds on `sdk/domains/shefa` by referencing shefa's economic primitives.

Content types:
- `work-story` — A work item in a project. Coupling: task completion → economic event in shefa (action: produce, resource: work-output). Governance: steward-consent. Claims: task completes within cadence, work produces declared value.
- `work-project` — A container for work stories. Coupling: project milestone → economic event. Governance: community visibility. Claims: project delivers on stated purpose.

Signals:
- `task-completed` — substrateSignal: compute, economicAction: produce
- `sprint-completed` — substrateSignal: compute, economicAction: produce
- `cadence-reset` — substrateSignal: time, economicAction: use

Observations:
- `delivery-health` — polarity: negative, archetype: outcome-divergence (are tasks completing on cadence?)
- `cadence-sustainability` — polarity: negative, archetype: cost-accumulation (is the work pace sustainable?)
- `task-completion-rate` — polarity: positive, archetype: outcome-correlation

**Step 2:** Create `work-story-metadata.schema.json`:

```json
{
  "$id": "avodah:schema:metadata:work-story",
  "title": "WorkStoryMeta",
  "description": "Metadata for work-story content type. Project task with cadence and attestation gates.",
  "type": "object",
  "properties": {
    "status": { "type": "string", "enum": ["backlog", "todo", "in-progress", "review", "done"] },
    "priority": { "type": "string", "enum": ["low", "medium", "high", "urgent"] },
    "visibility": { "type": "string", "enum": ["private", "community", "exchange"] },
    "cadence": {
      "type": "object",
      "properties": {
        "frequency": { "type": "string", "enum": ["daily", "weekly", "monthly", "custom"] },
        "resetBehavior": { "type": "string", "enum": ["reset-to-backlog", "reset-to-todo", "archive"] }
      }
    },
    "attestationGates": {
      "type": "array",
      "description": "Lamad content IDs that must be mastered before this task can be started",
      "items": { "type": "string" }
    },
    "exchangePublish": { "type": "boolean", "description": "Whether to publish to shefa exchange as a service request" },
    "projectId": { "type": "string" },
    "assignedTo": { "type": "string" }
  },
  "additionalProperties": true
}
```

**Step 3:** Create `work-project-metadata.schema.json`:

```json
{
  "$id": "avodah:schema:metadata:work-project",
  "title": "WorkProjectMeta",
  "description": "Metadata for work-project content type. Container for work stories with board configuration.",
  "type": "object",
  "properties": {
    "visibility": { "type": "string", "enum": ["private", "community"] },
    "columns": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "id": { "type": "string" },
          "label": { "type": "string" },
          "isTerminal": { "type": "boolean" }
        }
      }
    },
    "members": { "type": "array", "items": { "type": "string" } },
    "defaultCadence": { "type": "string" }
  },
  "additionalProperties": true
}
```

**Step 4:** Wire `metadataSchema` refs into manifest.

**Step 5:** Commit.

```bash
git add app/avodah/
git commit -m "feat(avodah): create work management app manifest built on shefa domain"
```

### Task 6: Create avodah codegen

**Files:**
- Create: `app/avodah/scripts/codegen.mjs`

**Step 1:** Copy codegen pattern. Output to `app/elohim-app/src/app/avodah/generated/`.

**Step 2:** Produces:
- `metadata-types.ts` — `WorkStoryMeta`, `WorkProjectMeta`
- `coupling-map.ts` — `AVODAH_COUPLING_MAP`
- `manifest-types.ts`
- `content-node-types.ts` — `isWorkStoryNode()`, `isWorkProjectNode()`

**Step 3:** Run codegen, verify, add `pnpm run avodah:codegen`.

**Step 4:** Commit.

### Task 7: Full verification

```bash
# All domain codegens
node elohim/sdk/domains/qahal/scripts/codegen.mjs
node app/avodah/scripts/codegen.mjs

# Generated files
ls app/elohim-app/src/app/qahal/generated/
ls app/elohim-app/src/app/avodah/generated/

# Protocol tests
pnpm run schema:test

# App builds
cd app/elohim-app && pnpm exec ng build --configuration=development
```
