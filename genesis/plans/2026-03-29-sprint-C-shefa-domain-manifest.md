# Sprint C: Shefa Domain Manifest

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create `elohim/sdk/domains/shefa/` with manifest, metadata schemas, codegen, and CLAUDE.md. Economy domain: REA economic events, stewardship, resource flows, exchange.

**Architecture:** Shefa primarily uses protocol REA primitives (EconomicEvent, Agreement, Commitment) rather than declaring many custom content types. Its manifest declares how those protocol primitives couple, what signals they produce, and what metadata they carry. Codegen produces typed interfaces for stewardship, exchange, and agreement metadata.

**Tech Stack:** JSON Schema, Node.js codegen, pnpm

**Parent design:** `genesis/plans/2026-03-29-domain-manifests-sdk-boundary-design.md`

**Reference pattern:** `elohim/sdk/domains/lamad/` (created in Sprint A)

> **P2P note:** EconomicEvent, Agreement, Commitment are Category A (DHT-notarized). Stewardship allocations are Category A2 (derived via link). These schemas describe metadata shapes for existing REA entities — no new storage tables.

---

### Task 1: Create shefa manifest

**Files:**
- Create: `elohim/sdk/domains/shefa/manifest.json`

**Step 1:** Create the manifest. Shefa's vocabulary is distinct from the other domains — it wraps protocol REA primitives with economic semantics rather than defining many new content types.

Content types owned by shefa (few, because REA types are protocol-level):
- `stewardship-context` — Narrative context for a stewardship relationship. Category A2. Coupling: stewardship creates value flows to stewards (value), steward accountability (governance).

Content types REFERENCED from protocol (shefa declares coupling, not the type):
- Economic events, agreements, commitments are protocol primitives. Shefa declares the domain-specific coupling (what signals they produce in the economy context).

Signals:
- `economic-event-recorded` — substrateSignal: compute, economicAction: produce
- `stewardship-allocated` — substrateSignal: attention, economicAction: produce
- `resource-transferred` — substrateSignal: resource, economicAction: transfer
- `obligation-fulfilled` — substrateSignal: compute, economicAction: produce
- `custodian-attestation` — substrateSignal: compute, economicAction: produce
- `insurance-claim` — substrateSignal: resource, economicAction: consume

Observations (must include negatives):
- `distribution-health` — polarity: negative, archetype: distribution-health (are flows equitable?)
- `cost-accumulation` — polarity: negative, archetype: cost-accumulation (externalities building up?)
- `obligation-fulfillment` — polarity: positive, archetype: outcome-correlation (are commitments honored?)
- `stewardship-effectiveness` — polarity: negative, archetype: outcome-divergence (does curation help or hurt?)

Rendering: shefa has its own views (dashboard, journal, resource explorer) but these are app-level, declared in the reference client only.

**Step 2:** Validate manifest against protocol schema.

**Step 3:** Commit.

```bash
git add elohim/sdk/domains/shefa/manifest.json
git commit -m "feat(shefa): create economy domain manifest with REA coupling declarations"
```

### Task 2: Create shefa metadata schemas

**Files:**
- Create: `elohim/sdk/domains/shefa/schemas/stewardship-metadata.schema.json`
- Create: `elohim/sdk/domains/shefa/schemas/exchange-metadata.schema.json`
- Create: `elohim/sdk/domains/shefa/schemas/agreement-metadata.schema.json`

**Step 1:** Create `stewardship-metadata.schema.json`:

```json
{
  "$id": "shefa:schema:metadata:stewardship",
  "title": "StewardshipMetadata",
  "description": "Metadata for stewardship allocations and stewardship-context content.",
  "type": "object",
  "properties": {
    "allocationStrategy": { "type": "string", "description": "equal | weighted | custodial" },
    "affinityScore": { "type": "number", "description": "0.0-1.0 stewardship affinity" },
    "custodianRole": { "type": "string", "description": "original-creator | curator | reviewer | community" },
    "circulationRights": { "type": "string", "description": "What the steward can do with the content" },
    "demurrageRate": { "type": "number", "description": "Rate at which standing decays without activity" }
  },
  "additionalProperties": true
}
```

**Step 2:** Create `exchange-metadata.schema.json`:

```json
{
  "$id": "shefa:schema:metadata:exchange",
  "title": "ExchangeMetadata",
  "description": "Metadata for exchange/marketplace interactions.",
  "type": "object",
  "properties": {
    "offerType": { "type": "string" },
    "requestType": { "type": "string" },
    "terms": { "type": "string" },
    "expiresAt": { "type": "string", "description": "ISO 8601" },
    "reciprocityExpected": { "type": "boolean" }
  },
  "additionalProperties": true
}
```

**Step 3:** Create `agreement-metadata.schema.json`:

```json
{
  "$id": "shefa:schema:metadata:agreement",
  "title": "AgreementMetadata",
  "description": "Metadata for REA agreements between parties.",
  "type": "object",
  "properties": {
    "parties": { "type": "array", "items": { "type": "string" } },
    "obligations": { "type": "array", "items": { "type": "object" } },
    "fulfillmentCriteria": { "type": "string" },
    "state": { "type": "string", "description": "proposed | active | fulfilled | cancelled" }
  },
  "additionalProperties": true
}
```

**Step 4:** Wire `metadataSchema` refs into manifest.

**Step 5:** Commit.

### Task 3: Create shefa codegen

**Files:**
- Create: `elohim/sdk/domains/shefa/scripts/codegen.mjs`

**Step 1:** Copy lamad codegen pattern. Adjust:
- Manifest: `../manifest.json`
- Schemas: `../schemas/`
- Output: `app/elohim-app/src/app/shefa/generated/`

**Step 2:** Produces:
- `metadata-types.ts` — `StewardshipMetadata`, `ExchangeMetadata`, `AgreementMetadata`
- `coupling-map.ts` — `SHEFA_COUPLING_MAP`
- `manifest-types.ts` — content type lists, signal map

**Step 3:** Run codegen, verify output.

**Step 4:** Add `pnpm run shefa:codegen` script.

**Step 5:** Commit.

### Task 4: Create CLAUDE.md

**Files:**
- Create: `elohim/sdk/domains/shefa/CLAUDE.md`

Cover:
- Shefa owns the economy vocabulary — how value flows through the protocol
- REA primitives (EconomicEvent, Agreement, Commitment) are protocol-level; shefa declares domain coupling
- Stewardship as anti-capture: mastery gate + affinity lifecycle (attention doesn't build standing, only curation acts)
- Key distinction: shefa is value accounting, not currency — demurrage, circulation rights, obligation tracking
- Reference to `app/elohim-app/src/app/shefa/` for the Angular view layer
- Codegen command

**Commit.**

### Task 5: Verify

```bash
node elohim/sdk/domains/shefa/scripts/codegen.mjs
ls app/elohim-app/src/app/shefa/generated/
pnpm run schema:test
cd app/elohim-app && pnpm exec ng build --configuration=development
```
