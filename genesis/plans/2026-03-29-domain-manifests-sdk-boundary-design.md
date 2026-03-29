# Domain Manifests + SDK Boundary Design

## Problem

The SDK boundary is unclear. Lamad-specific vocabulary (path-visibility, step-type, completion-criteria) leaked into protocol schemas because lamad was prototyped first. The other three pillars (imagodei, shefa, qahal) have substantial Angular code but no manifests — their content types, coupling declarations, and metadata schemas don't exist. Avodah (work management) is an app built on shefa with no manifest declaring how its work-stories couple to economic primitives.

A developer coming to the protocol can't tell what's protocol-level (must be respected) vs app-level (can be replaced). The four domain vocabularies and the wire types are the same layer — they're both "what the protocol provides" — but they live in different places with no clear relationship.

## The Two Concerns

**SDK domains enforce integrity** — they define what signals the protocol MUST see to maintain global coherence. If your app uses a concept and doesn't produce the mastery signal, the protocol rejects it. Non-negotiable.

**App manifests enable composition** — they help a specific client translate domain primitives into human experience. A different client could render the same concept as a flashcard instead of markdown. The domain coupling is identical. The HX is theirs.

## Architecture

```
elohim/sdk/
├── schemas/              ← Structural primitives (envelope shape, validation rules)
│   ├── v1/enums/         ← Reach, ConstitutionalLayer, SubstrateSignal, MasteryLevel,
│   │                        EngagementType, ResourceNature (protocol-level ONLY)
│   ├── v1/inputs/        ← CreateContentInput, CreateEconomicEventInput, CreateAttestationInput
│   ├── v1/views/         ← ContentView, EconomicEventView
│   └── v1/manifest/      ← app-manifest.schema.json ("what any manifest must declare")
│
├── domains/              ← Protocol domain vocabulary (the four pillars)
│   ├── lamad/            ← Learning: concept, path, assessment, mastery coupling
│   │   ├── manifest.json
│   │   ├── schemas/      ← PathMetadata, ConceptMetadata, EprCompositeBody, etc.
│   │   ├── scripts/      ← codegen.mjs
│   │   └── CLAUDE.md
│   ├── imagodei/         ← Identity: human, attestation, presence, agency coupling
│   │   ├── manifest.json
│   │   ├── schemas/
│   │   └── CLAUDE.md
│   ├── shefa/            ← Economy: economic events, stewardship, resource flows
│   │   ├── manifest.json
│   │   ├── schemas/
│   │   └── CLAUDE.md
│   └── qahal/            ← Social + Governance: collectives, proposals, relationships
│       ├── manifest.json
│       ├── schemas/
│       └── CLAUDE.md
│
└── storage-client-ts/    ← Rust-generated runtime types

app/
├── lamad/                ← Reference client views for learning domain
│   ├── generated/        ←   types generated FROM sdk/domains/lamad
│   ├── renderers/        ←   markdown, sophia, gherkin renderers
│   └── components/       ←   (in elohim-app/src/app/lamad/)
├── imagodei/             ← Reference client views for identity domain
│   └── generated/
├── shefa/                ← Reference client views for economy domain
│   └── generated/
├── qahal/                ← Reference client views for social domain
│   └── generated/
├── avodah/               ← App manifest built on shefa
│   ├── manifest.json
│   ├── schemas/
│   └── generated/
└── elohim-app/           ← Angular shell (composes all domains + apps)
    └── src/app/
        ├── lamad/        ← Angular pillar (services, components, renderers)
        ├── imagodei/     ← Angular pillar
        ├── shefa/        ← Angular pillar
        ├── qahal/        ← Angular pillar
        └── avodah/       ← Angular app feature
```

## What the Protocol Owns (stays in sdk/schemas/)

| Enum | Why Protocol-Level |
|------|-------------------|
| `reach` | Gates content distribution at DHT layer |
| `constitutional-layer` | Authority hierarchy for governance |
| `substrate-signal` | The 7 infrastructure resource dimensions |
| `mastery-level` | `apply` gates governance participation — Bloom's ladder is constitutional |
| `engagement-type` | Drives recognition flows to substrate signals |
| `relationship-type` | Graph edge vocabulary for three-leg knowledge coupling |
| `validation-status` | Schema migration health (valid, migrated, degraded, healing) |
| Resource nature enums | rivalry, excludability, depletability, fungibility, circularity |
| Geospatial enums | place-type, place-status, spatial-context-type, capacity-model |

Wire types: `CreateContentInput`, `ContentView`, `CreateEconomicEventInput`, `EconomicEventView`, `CreateAttestationInput`

Manifest validation: `app-manifest.schema.json` (three-leg coupling + claims + observations structure)

## What Leaked from Lamad (tag for migration)

| Enum | Issue | Action |
|------|-------|--------|
| `path-visibility` | draft/unlisted are lamad metadata, not protocol reach | Tag `"_migration": "domain:lamad"`, move to `sdk/domains/lamad/schemas/` |
| `step-type` | Path sequencing pedagogy, not protocol concern | Tag `"_migration": "domain:lamad"` |
| `completion-criteria` | Protocol cares gate exists, lamad defines gate logic | Tag `"_migration": "domain:lamad"` |
| `content-type` values | 32 values mix protocol + all domains | Future: split into domain-registered types (requires DNA changes) |
| `content-format` extensible | sophia, gherkin, perseus are rendering hints | Future: core formats stay protocol, extensible move to domain manifests |

These enums stay in `sdk/schemas/` for now (DNA depends on them) but get tagged so future work knows they're misplaced.

## Domain Manifest Vocabulary

### sdk/domains/lamad/ — Learning

**Content types owned:** concept, lesson, assessment, exercise, reflection, discussion, article, path, epic, scenario, feature, practice, quiz, course-module, module, simulation, discovery-assessment, instrument

**Metadata schemas:**
- `PathMetadata` — pathType, difficulty, thumbnailUrl, estimatedDuration, version, purpose
- `ConceptMetadata` — summary, sourcePath, relatedNodeIds, estimatedMinutes, thumbnailUrl, bloomsLevel
- `AssessmentMetadata` — instrument, mode (mastery/discovery/reflection), scoringRules
- `EprCompositeBody` — sections tree with items, refs, completion criteria

**Signals:** learning-signal, mastery-achieved, assessment-completed, path-completed, practice-engagement, contribution-created, peer-review-completed

**Key coupling:** assessment completion produces mastery-attestation; mastery at `apply` level gates governance participation

### sdk/domains/imagodei/ — Identity

**Content types owned:** human, role, contributor

**Metadata schemas:**
- `HumanMetadata` — displayName, bio, location, agencyStage, profileReach
- `AttestationMetadata` — scope, attestationType, evidence, grantor (uses protocol's CreateAttestationInput)
- `PresenceMetadata` — presenceState, externalIdentifiers, establishingContentIds

**Signals:** identity-created, presence-established, attestation-granted, attestation-revoked, agency-progressed, relationship-formed

**Key coupling:** identity creation establishes presence; attestations produce trust signals that gate reach expansion

### sdk/domains/shefa/ — Economy

**Content types owned:** (shefa primarily uses protocol REA primitives, not custom content types)

**Metadata schemas:**
- `StewardshipMetadata` — allocationStrategy, affinityScore, custodianRole
- `ExchangeMetadata` — offerType, requestType, terms
- `AgreementMetadata` — parties, obligations, fulfillmentCriteria

**Signals:** economic-event-recorded, stewardship-allocated, resource-transferred, obligation-fulfilled, insurance-claim, custodian-attestation

**Key coupling:** every economic event carries value + governance + feedback; stewardship standing accumulates from curation acts (not attention)

### sdk/domains/qahal/ — Social + Governance

**Content types owned:** collective, proposal, challenge, appeal, statement

**Future social types (declared in vocabulary now, implemented later):** post, event, group, message, thread

**Metadata schemas:**
- `CollectiveMetadata` — memberCount, governanceModel, constitutionalLayer, geoBoundary
- `ProposalMetadata` — mechanism (consent, ranked-choice, score), quorum, deadline, options
- `ChallengeMetadata` — targetEprId, reason, escalationPath
- `StatementMetadata` — polarity, bridgingScore, clusterAffinity

**Signals:** governance-decision, community-report, social-engagement, relationship-formed, challenge-filed, appeal-filed, consensus-reached

**Key coupling:** governance acts produce consent signals; social acts produce attention + relationship signals; feedback observations include social health (isolation, polarization)

### app/avodah/ — Work Management (on shefa)

**Content types owned:** work-story, work-project

**Metadata schemas:**
- `WorkStoryMeta` — status, priority, cadence, attestationGates, visibility, exchangePublish
- `WorkProjectMeta` — columns, visibility, members

**Signals:** task-completed, sprint-completed, cadence-reset

**Key coupling:** task completion produces economic events in shefa's REA layer; work-story completion can trigger mastery signals if attestation gates reference lamad content

## Codegen Flow

Each domain has its own codegen that produces types for consumers:

```
sdk/domains/lamad/scripts/codegen.mjs
    reads: sdk/domains/lamad/manifest.json + schemas/
    produces: app/elohim-app/src/app/lamad/generated/
              genesis/seeder/src/generated/

sdk/domains/imagodei/scripts/codegen.mjs
    reads: sdk/domains/imagodei/manifest.json + schemas/
    produces: app/elohim-app/src/app/imagodei/generated/

sdk/domains/shefa/scripts/codegen.mjs
    reads: sdk/domains/shefa/manifest.json + schemas/
    produces: app/elohim-app/src/app/shefa/generated/

sdk/domains/qahal/scripts/codegen.mjs
    reads: sdk/domains/qahal/manifest.json + schemas/
    produces: app/elohim-app/src/app/qahal/generated/
```

App-level manifests follow the same pattern:

```
app/avodah/scripts/codegen.mjs
    reads: app/avodah/manifest.json + schemas/
    references: sdk/domains/shefa/manifest.json (parent domain)
    produces: app/elohim-app/src/app/avodah/generated/
```

## Sprint Structure

Four parallel sprints (can be executed by separate agents):

### Sprint A: Move lamad + create domain structure

1. Create `elohim/sdk/domains/` directory structure
2. Move `app/lamad/manifest.json` → `sdk/domains/lamad/manifest.json`
3. Move `app/lamad/schemas/` → `sdk/domains/lamad/schemas/`
4. Move `app/lamad/scripts/codegen.mjs` → `sdk/domains/lamad/scripts/codegen.mjs`
5. Update codegen output paths (still generates to `app/elohim-app/src/app/lamad/generated/` and `genesis/seeder/src/generated/`)
6. Create `sdk/domains/lamad/CLAUDE.md`
7. Update `app/lamad/CLAUDE.md` to reference new location
8. Tag leaked enums with `"_migration": "domain:lamad"` in their schema files
9. Verify: `pnpm run lamad:codegen`, `pnpm run schema:test`, app builds

### Sprint B: Create imagodei domain manifest

1. Create `sdk/domains/imagodei/manifest.json` — content types (human, role, contributor), coupling declarations, signals, observations
2. Create `sdk/domains/imagodei/schemas/` — HumanMetadata, AttestationMetadata, PresenceMetadata
3. Create `sdk/domains/imagodei/scripts/codegen.mjs`
4. Create `sdk/domains/imagodei/CLAUDE.md`
5. Generate types to `app/elohim-app/src/app/imagodei/generated/`
6. Verify: codegen runs, app builds

### Sprint C: Create shefa domain manifest

1. Create `sdk/domains/shefa/manifest.json` — REA coupling, stewardship vocabulary, signals, observations
2. Create `sdk/domains/shefa/schemas/` — StewardshipMetadata, ExchangeMetadata, AgreementMetadata
3. Create `sdk/domains/shefa/scripts/codegen.mjs`
4. Create `sdk/domains/shefa/CLAUDE.md`
5. Generate types to `app/elohim-app/src/app/shefa/generated/`
6. Verify: codegen runs, app builds

### Sprint D: Create qahal domain manifest + avodah app manifest

1. Create `sdk/domains/qahal/manifest.json` — governance + social vocabulary, coupling, signals, observations (declare future social types in vocabulary)
2. Create `sdk/domains/qahal/schemas/` — CollectiveMetadata, ProposalMetadata, ChallengeMetadata, StatementMetadata
3. Create `sdk/domains/qahal/scripts/codegen.mjs`
4. Create `sdk/domains/qahal/CLAUDE.md`
5. Create `app/avodah/manifest.json` — work management, references shefa domain
6. Create `app/avodah/schemas/` — WorkStoryMeta, WorkProjectMeta
7. Create `app/avodah/scripts/codegen.mjs`
8. Generate types, verify builds

## Not In This Batch

- Refactoring `content-type.schema.json` into domain-registered types (requires DNA changes)
- Moving `path-visibility`, `step-type`, `completion-criteria` to domain schemas (tagged only)
- Wiring signal harness for non-lamad domains (harness exists, coupling maps needed)
- Conductor normalizer for non-lamad content types
- Renderer registration from non-lamad manifests

## Exit Criteria

1. `elohim/sdk/domains/` exists with four domain manifests + schemas + CLAUDE.md
2. `app/avodah/` has its own manifest referencing shefa domain
3. Each domain's codegen produces types to its `app/elohim-app/.../generated/` directory
4. Lamad manifest has moved from `app/lamad/` to `sdk/domains/lamad/`
5. Leaked enums tagged with `"_migration": "domain:lamad"`
6. App builds, seeder compiles, schema tests pass
7. A developer reading the codebase can answer: "what does the protocol require?" (sdk/schemas/) vs "what does each domain define?" (sdk/domains/) vs "how does this app render it?" (app/)
