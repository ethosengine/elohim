# Sprint B: Imagodei Domain Manifest

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create `elohim/sdk/domains/imagodei/` with manifest, metadata schemas, codegen, and CLAUDE.md. Identity domain: humans, attestations, presence, agency, relationships.

**Architecture:** Domain manifest declares content types with three-leg coupling + claims. Metadata schemas define typed interfaces per content type. Codegen produces TypeScript to `app/elohim-app/src/app/imagodei/generated/`. Follows the pattern established by lamad in Sprint A.

**Tech Stack:** JSON Schema, Node.js codegen, pnpm

**Parent design:** `genesis/plans/2026-03-29-domain-manifests-sdk-boundary-design.md`

**Reference pattern:** `elohim/sdk/domains/lamad/` (created in Sprint A)

> **P2P note:** All content types here are Category A (DHT-notarized) or Category B (agent-scoped). No new storage tables — these schemas describe metadata shapes for existing identity entities.

---

### Task 1: Create imagodei manifest

**Files:**
- Create: `elohim/sdk/domains/imagodei/manifest.json`

**Step 1:** Create the manifest following the lamad pattern. Content types owned by imagodei:

- `human` — A person in the network. Category B (agent-scoped). Coupling: identity creation → presence signal (value), governance participation rights (governance).
- `role` — A functional role a human can hold. Category A2 (derived via link). Coupling: role assignment → capability grant (value), role accountability (governance).
- `contributor` — A contributor presence in the content graph. Category A (notarized). Coupling: contribution → stewardship standing (value), attribution rights (governance).

Each content type must declare:
- `coupling.knowledge` — relationships (RELATES_TO, ATTESTS, IDENTIFIES)
- `coupling.value` — onConsume/onComplete/onContribute economic flows
- `coupling.governance` — defaultReach, minimumReach, governanceModel, signalTypes
- `coupling.claims` — at least one outcome claim with validity horizon

Signals:
- `identity-created` — substrateSignal: attention, economicAction: produce
- `presence-established` — substrateSignal: attention, economicAction: produce
- `attestation-granted` — substrateSignal: compute, economicAction: produce
- `attestation-revoked` — substrateSignal: compute, economicAction: consume
- `agency-progressed` — substrateSignal: attention, economicAction: produce
- `relationship-formed` — substrateSignal: attention, economicAction: produce

Observations (must include at least one negative polarity):
- `identity-retention` — polarity: positive, archetype: retention-check (do people stay?)
- `attestation-accuracy` — polarity: negative, archetype: outcome-correlation (do attested claims hold up?)
- `identity-abandonment` — polarity: negative, archetype: outcome-divergence (identity created but never used)

Rendering section:
- No imagodei-specific renderers declared (identity UI is embedded in the app shell, not content-rendered)

**Step 2:** Validate the manifest against the protocol schema:

```bash
node elohim/sdk/schemas/scripts/test-manifest-schema.mjs
```

Manually test by loading the imagodei manifest with AJV against `app-manifest.schema.json`.

**Step 3:** Commit.

```bash
git add elohim/sdk/domains/imagodei/manifest.json
git commit -m "feat(imagodei): create identity domain manifest with coupling declarations"
```

### Task 2: Create imagodei metadata schemas

**Files:**
- Create: `elohim/sdk/domains/imagodei/schemas/human-metadata.schema.json`
- Create: `elohim/sdk/domains/imagodei/schemas/presence-metadata.schema.json`

**Step 1:** Create `human-metadata.schema.json`:

```json
{
  "$id": "imagodei:schema:metadata:human",
  "title": "HumanMetadata",
  "description": "Metadata shape for contentType 'human'. Identity attributes stored in ContentView.metadata.",
  "type": "object",
  "properties": {
    "displayName": { "type": "string" },
    "bio": { "type": "string" },
    "location": { "type": "string" },
    "agencyStage": { "type": "string", "description": "visitor | observer | participant | steward | elder" },
    "profileReach": { "type": "string", "description": "How visible this profile is (maps to protocol reach)" },
    "category": { "type": "string", "description": "person | organization | collective" },
    "affinities": { "type": "array", "items": { "type": "string" } },
    "externalIdentifiers": { "type": "object", "description": "External identity links (email, DID, social)" }
  },
  "additionalProperties": true
}
```

**Step 2:** Create `presence-metadata.schema.json`:

```json
{
  "$id": "imagodei:schema:metadata:presence",
  "title": "PresenceMetadata",
  "description": "Metadata for contentType 'contributor'. Contributor presence in the content graph.",
  "type": "object",
  "properties": {
    "presenceState": { "type": "string", "description": "unclaimed | claimed | active | dormant" },
    "establishingContentIds": { "type": "array", "items": { "type": "string" } },
    "affinityTotal": { "type": "number" },
    "uniqueEngagers": { "type": "integer" },
    "citationCount": { "type": "integer" },
    "recognitionScore": { "type": "number" }
  },
  "additionalProperties": true
}
```

**Step 3:** Wire `metadataSchema` refs into the manifest content type declarations.

**Step 4:** Commit.

### Task 3: Create imagodei codegen

**Files:**
- Create: `elohim/sdk/domains/imagodei/scripts/codegen.mjs`

**Step 1:** Copy the lamad codegen pattern (`elohim/sdk/domains/lamad/scripts/codegen.mjs`). Adjust:
- Manifest path: `../manifest.json`
- Schema directory: `../schemas/`
- Output directories:
  - `app/elohim-app/src/app/imagodei/generated/`
  - (No seeder output needed — seeder doesn't seed identity types from JSON files)

**Step 2:** The codegen should produce:
- `metadata-types.ts` — `HumanMetadata`, `PresenceMetadata` interfaces
- `content-node-types.ts` — `isHumanNode()`, `isContributorNode()` type guards
- `coupling-map.ts` — `IMAGODEI_COUPLING_MAP` with value flows per content type
- `manifest-types.ts` — content type lists, signal map

**Step 3:** Run codegen and verify output:

```bash
node elohim/sdk/domains/imagodei/scripts/codegen.mjs
ls app/elohim-app/src/app/imagodei/generated/
```

**Step 4:** Add `pnpm run imagodei:codegen` script to root `package.json`.

**Step 5:** Commit.

### Task 4: Create CLAUDE.md

**Files:**
- Create: `elohim/sdk/domains/imagodei/CLAUDE.md`

**Step 1:** Write CLAUDE.md following lamad pattern. Cover:
- What imagodei owns (identity vocabulary)
- Content types and their coupling
- Metadata schemas
- Codegen command
- How identity differs from other domains (embedded in app shell, not content-rendered)
- Key files in `app/elohim-app/src/app/imagodei/` (services, models)

**Step 2:** Commit.

### Task 5: Verify

**Step 1:** Run all verification:

```bash
# Imagodei codegen
node elohim/sdk/domains/imagodei/scripts/codegen.mjs

# Generated files exist
ls app/elohim-app/src/app/imagodei/generated/

# Protocol schema tests still pass
pnpm run schema:test

# App builds
cd app/elohim-app && pnpm exec ng build --configuration=development
```

**Step 2:** Commit any fixups.
