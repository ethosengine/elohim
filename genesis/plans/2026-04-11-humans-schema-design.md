# Humans & Presences Schema Design — Genesis Cast as Protocol Data

**Date:** 2026-04-11
**Status:** Design approved, pending implementation
**Scope:** `genesis/data/humans/`, `genesis/data/presences/`, content cleanup migration, seeder stage, narrative coherence validators
**Precedent:** `genesis/plans/2026-04-10-collectives-schema-design.md`

## Source of Truth Declarations

Every schema, table, and entity touched by this sprint has an explicit source-of-truth declaration. Summary table — see the P2P Design Gate section below for per-entity rationale.

| Artifact | Classification | Source of Truth | Storage Projection |
|---|---|---|---|
| `Human` entity | Notarized (A) | Holochain DHT — imagodei DNA `Human` entry type (`imagodei_integrity/src/lib.rs:262`) | `humans` SQLite table (read-optimized projection) |
| `HumanRelationship` entity | Notarized (A) | Holochain DHT — imagodei DNA `HumanRelationship` entry type (`imagodei_integrity/src/lib.rs:351`) | `human_relationships` SQLite table with `dht_anchor_hash` column (`db/models.rs:234`) |
| `ContributorPresence` entity | Notarized (A) | Holochain DHT — imagodei DNA `ContributorPresence` entry type (`imagodei_integrity/src/lib.rs:470`) | `contributor_presences` SQLite table |
| Content → Presence citation edge | Derived (A2) | Seed-data declarative in this sprint (rides on `ContentNode.contributors[]` metadata). Future: Holochain Link on Content entry. | `content.metadata_json` — no dedicated table yet |
| Observer attribution | Operational (C) in this sprint | Rides on `ContributorPresence.metadata_json` (upgrade path to `Attestation` entries documented) | `contributor_presences.metadata_json` |
| `humans.schema.json` | Operational (C) | File at `genesis/data/humans/humans.schema.json` — IoC contract for enum vocabulary not yet in Rust (`agencyPhase`, `HumanRelationshipType`, etc.). Reconstruction: if lost, rebuild from the imagodei `Human` entry struct + this design doc. |
| `presences.schema.json` | Operational (C) | File at `genesis/data/presences/presences.schema.json` — IoC contract for observation model and presence frontmatter. Reconstruction: if lost, rebuild from the imagodei `ContributorPresence` entry struct + this design doc. |
| `genesis/data/humans/humans.json` | Derived artifact | Generated from `genesis/data/humans/*.md` frontmatter by `build-data.ts`. Not a source of truth; the markdown is. Pre-push hook catches drift. |
| `genesis/data/presences/presences.json` | Derived artifact | Generated from `genesis/data/presences/*.md` frontmatter by `build-data.ts`. Not a source of truth; the markdown is. Pre-push hook catches drift. |
| `genesis/data/humans/*.md` | Canonical source (operational) | Hand-authored markdown with YAML frontmatter. The authoritative human-readable and machine-parseable representation of the 33 genesis humans. Reconstruction: if lost, irrecoverable without git history (it IS the source). |
| `genesis/data/presences/*.md` | Canonical source (operational) | Hand-authored markdown with YAML frontmatter. Same pattern as humans. Reconstruction: same. |

**Anti-pattern avoided:** no table or schema in this sprint is introduced without declaring its source of truth inline. Every JSON Schema definition below includes a `description` field naming its authority and reconstruction strategy.

## Problem

The seed data that powers the genesis scenario has three overlapping surfaces that all try to describe people, but none of them individually lets you read a single file and understand who a human is, who they're connected to, and what protocol behavior they exercise:

| Surface | Purpose | Current state |
|---|---|---|
| `genesis/docs/humans/humans.json` | Structured registry of 33 humans | One big array, no narrative, no schema validation |
| `genesis/data/lamad/content/human-*.json` | Attempted human-readable descriptions | **Category error** — 27 duplicate stubs of the humans registry stored as `contentType: "human"` content nodes |
| `genesis/data/lamad/content/fct-contributor-*.json` | Author presences cited by Pete's FCT curriculum | **Category error** — 31 auto-generated stubs stored as `contentType: "human"` content nodes; should be `ContributorPresence` entries |
| `genesis/data/lamad/content/governance-organizations-*.json` | Organizations cited in governance content | **Category error** — 52 auto-generated stubs stored as `contentType: "collective"` content nodes; should be `ContributorPresence` or `Collective` entries depending on joinability |
| `genesis/data/lamad/content/governance-books-*.json` | Books referenced in governance content | **Category error** — 3 auto-generated stubs with placeholder text; should be real `ContentNode` entries with author presences |
| `genesis/a2o/features/*.feature` | Scenarios referencing humans by displayName | 42 files, 526 references, no validation that names resolve |

The humans registry has no schema, no referential integrity enforcement, and no protocol-aligned vocabulary. The relevant source of truth is the imagodei DNA (notarized `Human` and `HumanRelationship` entry types), but the seed data never claims that authority — the SQLite `humans` projection accepts any string. The `human_relationships.relationship_type` field in elohim-storage is an unconstrained `String` with only a Rust-code comment `// See HUMAN_RELATIONSHIP_TYPES` pointing at a constant module that does not exist.

**Meanwhile, the imagodei DNA already notarizes everything needed**: `Human`, `HumanRelationship`, `ContributorPresence`, `Attestation`, `StewardshipGrant`, `RecoveryRequest`, `KeyStewardship`, and 20+ related entry types. The protocol primitives are built; the seed data just doesn't exercise them cleanly. And the `ContributorPresence` primitive — designed for "recognition before registration" — is sitting dormant with fully-built HTTP routes (`/db/presences`, `/db/presences/{id}`, `/db/presences/{id}/stewardship`, `/db/presences/{id}/claim`) and zero seeded data.

## Architectural Context

### The Consilience Garden is the actual provenance of most governance content

In August 2021, four years before EthosEngine existed as a company, Matthew curated a Google Keen called **"Consilience Garden"** with 103 gems across 10 hand-organized sections. The Keen description, written in 2021, reads:

> *"Projects that are developing to solve our meta crisis of global problem solving capacity by restoring the health of our information commons by promoting public dialogue, civic virtue and participatory governance — the foundations of a functional, free, and inclusive society."*

This is the proto-thesis of the Elohim Protocol, written four years early. When Google shut Keen down, Matthew did a data checkout, and the resulting export became the auto-generated `governance-organizations-*.json`, `governance-books-*.json`, and adjacent content stubs in lamad. Those stubs have never been human-edited — they still contain placeholder text like `[to be filled in]` and `[Principle 1]`.

**This sprint migrates the Keen to its new home in the protocol and then deletes the Keen.** The Consilience Garden was the prototype; the protocol is where that thinking lives now. The migration preserves every gem with full fidelity (title, description, URL, image) and transforms each into either a `ContributorPresence` or a book `ContentNode`, depending on whether the gem references a person/organization or a work.

### The "gradual shape" pattern — layered derivation

Rather than picking one file format, the design uses a ladder where each layer is derived from the one above it and consumed by a different audience. No layer is a parallel source of truth; each is a projection for its audience.

```
Layer 0: Gherkin scenarios (.feature files)
   audience: product thinking, scenario review
   purpose: describe behavior
           ↑ references entities from layer 1

Layer 1: Entity markdown + YAML frontmatter (canonical source)
   audience: narrative authoring, Obsidian-style browsing, code review
   purpose: describe entities
           ↓ generator/validator reads frontmatter

Layer 2: Generated JSON (humans.json, presences.json)
   audience: seeder, CI, any machine consumer
   purpose: pre-parsed, type-checked data
           ↓ seeder POSTs

Layer 3: SQLite rows + DHT entries (live system state)
   audience: running peers
   purpose: queryable, notarized
```

The canonical source is **the markdown layer**. JSON artifacts are generated deterministically from markdown and checked in so CI doesn't regenerate them, but a pre-push hook verifies freshness. Same pattern as `schema:codegen:ts`.

**Each entity is one markdown file.** Opening `matthew-manager.md` in any markdown editor shows the whole person — frontmatter data, narrative prose, relationship connections, and an auto-generated "Scenarios featuring Matthew" section. Opening the `presences/` directory feels like opening the cast list of a novel.

### End-to-end type alignment

The validator imports real TypeScript types from `@elohim/storage-client` as compile-time guards. If Rust adds a field to `CreateHumanInputView` or `CreateContributorPresenceInputView`, `cargo test export_bindings` regenerates the TypeScript types, the validator's imports fail to compile until the frontmatter interface adds the field, and the schema catches missing values at parse time. **The compile error IS the contract enforcement.**

For fields that don't exist in Rust yet (e.g., `agencyPhase` enum, `observations[]`, human relationship type vocabulary), JSON Schema is the IoC contract — authoritative until Rust catches up. This is an operational (Category C) source of truth declaration: the schema file IS the authority, with reconstruction strategy documented above. When a future sprint promotes a field into Rust, the migration is mechanical: move the constant from the validator into a Rust module, export via ts-rs, delete the hand-rolled constant, no seed-data rewrite required.

### Multi-observer attribution and the value-flow thesis

Every `ContributorPresence` carries an append-only `observations[]` array where each observer adds their own entry. The design intentionally supports multiple observers — including the same observer multiple times with different content contexts — because each observation represents a distinct moment of recognition that should eventually route recognition flows back through the contributor-weight ratios.

The worked example from this sprint's scoping conversation:

> Person A uploads a book to their node → creates author presence with Observation #1 (observerId: A). Person B, in a disconnected peer network, does the same → independent presence, Observation #1 (observerId: B). Person C connects to both networks; elohim graph traversal discovers the two presences share an `externalIdentifiers[]` entry (Wikipedia URL, ORCID) and proposes a merge. The author eventually joins, attests identity, claims the merged presence, and sees the accumulated recognition from every reader plus the two observers (A and B) who first noticed them worth attributing. The author then has the option to mint Elohim Tokens for USD exit (minus commons stewardship) or exchange them inside the shefa market.

This sprint's job: plant the substrate cleanly so the future elohim-traversal work, the future shefa-flow wiring, and the future elohim-token exit paths don't have to guess at missing data.

### Pete is a synthetic genesis persona; Matthew is a real historical observer

The genesis cast includes fictional personas (Pete Pastor, Pastor Pete's FCT curriculum, Dr. Dolittle, Tiffany, etc.) and one real human (Matthew Dowell). Both are protocol-valid as observers:

- **Matthew's observations are historical** — rooted in the real 2021 Consilience Garden keen creation event
- **Pete's observations are narrative** — rooted in the genesis story where Pete builds his FCT curriculum and cites contributors

Both are load-bearing for the test suite because scenarios run against the same seeded world regardless of which observer is historical vs narrative. The `presences/README.md` distinguishes them for archaeological honesty without creating a two-tier entity system.

## P2P Design Gate

### Entity: Human
- **Classification**: Notarized (A) — DHT entry type `Human` exists in imagodei integrity zome (`imagodei_integrity/src/lib.rs:262`)
- **Justification**: Identity is foundational. If a human's display name, affinities, or profile reach could be silently altered, every attestation, relationship, and stewardship claim referencing them becomes untrustworthy.
- **Content Address Strategy**: Slug (`human-{name-slug}`). `displayName` is mutable; CID doesn't apply. Not agent-scoped.
- **Source of Truth**: Holochain DHT (imagodei DNA). SQLite `humans` table is a read-optimized projection; seed-data layer POSTs to doorway which projects to SQLite, with DHT notarization happening via pull-based seeding when peers ingest.
- **Coordinator Zome**: `imagodei::create_human`
- **Storage Projection**: `humans` table
- **HTTP Route**: `GET/POST /db/humans` (existing)
- **Anti-Pattern Caught**: Category error — humans were being stored as `contentType: "human"` content nodes in lamad (27 duplicates). Schema sprint enforces the correct layer.

### Entity: HumanRelationship
- **Classification**: Notarized (A) — DHT entry type `HumanRelationship` in imagodei (`imagodei_integrity/src/lib.rs:351`). Rich primitive with consent, custody, intimacy, bidirectionality, expires_at, context_json.
- **Justification**: Custody, emergency access, and stewardship claims flow through relationships. Already notarized in DNA.
- **Content Address Strategy**: Agent-scoped composite `(party_a_id, party_b_id, relationship_type)`.
- **Source of Truth**: Holochain DHT. SQLite `human_relationships` table has `dht_anchor_hash` column (confirmed `db/models.rs:234`).
- **Coordinator Zome**: `imagodei::create_human_relationship`
- **Storage Projection**: `human_relationships` table
- **HTTP Route**: `GET/POST /db/human-relationships` (existing)
- **Anti-Pattern Caught**: Missing enum vocabulary. `relationship_type` is an unconstrained `String` in Rust; the schema becomes the IoC contract until Rust catches up. Same pattern as collectives.schema.json for `governanceLayer`.

### Entity: ContributorPresence
- **Classification**: Notarized (A) — DHT entry type `ContributorPresence` in imagodei (`imagodei_integrity/src/lib.rs:470`). Fully-designed lifecycle: unclaimed → stewarded → claimed, with recognition accumulation, claim process, external identifiers, stewardship fields.
- **Justification**: Recognition-before-registration is the protocol's mechanism for routing value back to cited contributors who haven't joined. If accumulated affinity, citation counts, or endorsements could be silently altered, the entire claim mechanism is broken.
- **Content Address Strategy**: Slug (`presence-{name-slug}`). Stable across the unclaimed→claimed transition, which is critical for citation stability.
- **Source of Truth**: Holochain DHT. SQLite `contributor_presences` table.
- **Coordinator Zome**: `imagodei::create_contributor_presence`, `imagodei::initiate_stewardship`, `imagodei::initiate_claim`
- **Storage Projection**: `contributor_presences` table (exists, per `http.rs:42`)
- **HTTP Route**: `GET/POST /db/presences`, `GET/DELETE /db/presences/{id}`, `POST /db/presences/{id}/stewardship`, `POST /db/presences/{id}/claim` — **all exist** at `http.rs:3646-3761`
- **Anti-Pattern Caught**: Category errors in both directions — FCT contributors stored as `contentType: "human"` content nodes (31 files), governance orgs stored as `contentType: "collective"` content nodes (52 files). Neither is content. Both are presences.

### Entity: Observer Attribution (seed-data only, rides on metadata)
- **Classification**: Operational (C) — rides on `ContributorPresence.metadata_json`
- **Justification**: We need to carry "who observed this presence and why" in seed data, but we do NOT mint a new entry type. `observations[]` lives in the metadata bag on the presence for this sprint. When shefa flow-wiring happens in a later sprint, each observation gets promoted to a real `Attestation` entry with `attestation_type: "presence-observation"` and `agent_id: observer`. The `Attestation` primitive already exists; promotion is mechanical.
- **Source of Truth**: DHT (as part of the presence's `metadata_json`). Later sprint splits into separate `Attestation` entries.
- **Anti-Pattern Caught**: Considered "create new entry type for observer attribution." Rejected — would burn imagodei entry-type headroom on something that rides on metadata until flow semantics are designed. YAGNI wins.

### Entity: Content → Presence citation edges
- **Classification**: Derived (A2) — logically anchored via link on the Content entry. For this sprint: seed-data declarative only, no HTTP surface yet.
- **Justification**: When FCT Module 12 cites Virginia Eubanks, the citation is a **relationship or attribute** of the content, not a standalone entity. It has no meaning without its parent content. Matches `CollectiveRelationship` precedent (Category A2) from the collectives sprint.
- **Content Address Strategy**: Composite tuple `(content_entry_hash, presence_id, contribution_type)`
- **Source of Truth**: For this sprint — declarative seed data on `ContentNode.contributors[]` (rides in metadata_json at Rust boundary). Future: Holochain Link on Content entry.
- **HTTP Route**: None yet.
- **Anti-Pattern Caught**: "REST as starting point" — briefly tempted to propose `POST /db/content/{id}/contributors`. Rejected. The link is the truth; the HTTP route comes later when flow wiring needs it.

### Design Constraints Discovered

1. **Zero Rust changes.** Every entity maps to an existing imagodei DNA primitive with existing HTTP routes. This is the cleanest possible sprint shape.
2. **Seeder gains one new stage.** `seed-presences.ts` runs between `seed-humans` and `seed-collectives` (humans own the stewarded presences; presences must exist before collectives can reference them via governance orgs).
3. **DNA capacity untouched.** Imagodei stays at 28 entry types. Lamad stays at ~73. No entry type budget spent.
4. **`establishing_content_ids_json` field on `ContributorPresence`** is designed exactly for the content-citation case — it carries the list of content IDs citing the presence. The seeder populates it from the inverse of `contributors[]` edges on content.
5. **Cross-file referential integrity is larger than collectives.** humans.json → collectives.json (via `communities[]`), humans.json → humans.json (via `guardianIds[]`, relationships), humans.json → organizations (via `organizations[].id`), presences.json → humans.json (via `observedBy`, `stewardedBy`), presences.json → lamad content (via `observations[].contextContentId` and `works[].citedInContentIds`), account-packages/*.json → all three.
6. **Observer attribution upgrade path is clean.** Metadata bag now → `Attestation` entry later. The `Attestation` entry type already exists, so the upgrade is purely a projection change in a future sprint, not a DNA migration.

## Design

### File topology

```
genesis/data/
├── humans/
│   ├── humans.schema.json                  NEW — IoC contract for frontmatter
│   ├── matthew-manager.md                  canonical source (frontmatter + narrative)
│   ├── jessica-spouse.md
│   ├── adam-firstman.md
│   ├── ...  (33 files, one per human)
│   ├── relationships.md                    NEW — single file with relationships[] frontmatter
│   ├── humans.json                         GENERATED — pre-parsed array
│   └── README.md                           GENERATED — index/cast-list
│
├── presences/
│   ├── presences.schema.json               NEW — IoC contract (operational source of truth; projection target is contributor_presences table, notarized in imagodei DNA)
│   ├── virginia-eubanks.md                 canonical source per presence
│   ├── daniel-schmachtenberger.md
│   ├── consilience-project.md
│   ├── james-p-carse.md                    author of book ContentNode
│   ├── kim-stanley-robinson.md
│   ├── ...  (~110 files after dedup)
│   ├── relationships.md                    NEW — presence-to-presence edges
│   ├── presences.json                      GENERATED — pre-parsed array
│   ├── README.md                           GENERATED with Consilience Garden narrative
│   └── images/
│       ├── placeholder-person.webp         committed placeholder
│       ├── placeholder-organization.webp   committed placeholder
│       ├── placeholder-generic.webp        committed placeholder
│       └── {presence-id}.webp              downloaded + optimized per presence
│
├── collectives/                            existing — minor additions this sprint
│   └── collectives.json                    MODIFIED — adds joinable orgs migrated from Keen
│
├── account-packages/*.json                  REGENERATED — 33 packages (4 new + 29 updated)
└── lamad/content/
    ├── manifesto.json                      MODIFIED — new contributors[] array (~98 Keen presences)
    ├── fct-module-*.json                   MODIFIED — new contributors[] arrays, relatedNodeIds rewritten
    ├── book-finite-and-infinite-games.json NEW — replaces stub with real book node + author link
    ├── book-collapse-of-complex-societies.json NEW
    ├── book-ministry-for-the-future.json   NEW
    ├── fct-contributor-*.json              DELETED (31 files)
    ├── human-*.json                        DELETED (27 files)
    ├── governance-organizations-*.json     DELETED (52 files)
    └── governance-books-*.json             DELETED (3 files)

genesis/seeder/src/
├── validate-humans.ts                      NEW — hand-rolled, imports from @elohim/storage-client
├── validate-presences.ts                   NEW — same pattern
├── validate-content.ts                     NEW or EXPANDED — checks contributors[] references
├── seed-humans.ts                          MODIFIED — reads markdown via build-data.ts
├── seed-presences.ts                       NEW — POSTs to /db/presences
├── seed-collectives.ts                     MODIFIED — accepts new joinable orgs
├── account-package.ts                      MODIFIED — resolves presence IDs
├── build-data.ts                           NEW — markdown→JSON generator
└── ...existing

genesis/scripts/
├── migrate-content-to-presences.ts         NEW, ONE-SHOT — deleted in same commit after execution
└── migration-author-map.json               NEW, ONE-SHOT — hand-mapped book→author

genesis/data/presences/                      entire directory created by migration
genesis/Jenkinsfile                          MODIFIED — new "Seed Presences" stage
.husky/pre-push                              MODIFIED — new validation hooks
.claude/file-relationships.json              MODIFIED — humans-presences-sync rules
```

**After the migration commit**, the `Consilience_Garden-nvBCVtcYER7C9s3H6zxS/` directory at the repo root is deleted. Goodbye Keen.

### Seeder execution order

```
seed-sqlite       content — including content with new contributors[] arrays
    ↓
seed-humans       33 registered humans from humans/*.md
    ↓
seed-presences    ~110 cited presences from presences/*.md   ← NEW STAGE
    ↓
seed-collectives  holonic collectives + migrated joinable governance orgs
    ↓
seed-accounts     account packages now reference humans + presences + collectives
```

Ordering rationale: humans must exist before presences (observers), presences must exist before collectives (governance orgs referenced by collectives), all must exist before accounts (which tie them together).

### `humans.schema.json` structure

**Source of truth:** this schema file is operational (Category C) — it is the IoC contract for fields not yet in Rust. The notarized `Human` entry type in imagodei DNA is the authority for the core fields; this schema adds the seed-data vocabulary riding on metadata_json projection.

Frontmatter schema for each markdown file in `genesis/data/humans/*.md`:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "elohim:protocol:humans",
  "title": "Human Frontmatter Schema",
  "type": "object",
  "required": ["id", "displayName", "category", "profileReach"],
  "properties": {
    "id": {
      "type": "string",
      "pattern": "^human-[a-z0-9][a-z0-9-]*[a-z0-9]$",
      "description": "Unique slug. Must match filename stem (matthew-manager.md → human-matthew-manager)."
    },
    "displayName": { "type": "string", "minLength": 1 },
    "bio": { "type": ["string", "null"] },
    "agencyPhase": {
      "type": ["string", "null"],
      "enum": ["doorway", "hosted", "device", "node", "retired", null],
      "description": "Graduated capability phase. Rides in metadata_json."
    },
    "category": {
      "type": "string",
      "enum": ["core-family", "workplace", "community", "affinity", "local-economy", "newcomer", "visitor", "edge-case", "red-team"]
    },
    "profileReach": {
      "type": "string",
      "enum": ["private", "self", "intimate", "trusted", "familiar", "community", "public", "commons", "hidden"]
    },
    "location": {
      "type": ["object", "null"],
      "properties": {
        "layer": { "enum": ["neighborhood", "municipality", "city", "county_regional", "bioregion", "nation", "global"] },
        "name": { "type": "string" },
        "h3Cell": { "type": ["string", "null"] }
      },
      "required": ["layer", "name"]
    },
    "organizations": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["id", "name", "role"],
        "properties": {
          "id": { "type": "string", "description": "Must reference a collective or presence id" },
          "name": { "type": "string" },
          "role": { "type": "string" }
        }
      }
    },
    "communities": {
      "type": "array",
      "items": { "type": "string", "description": "Must reference a collective id" }
    },
    "affinities": {
      "type": "array",
      "items": { "type": "string" }
    },
    "guardianIds": {
      "type": "array",
      "items": { "type": "string", "description": "Must reference another human id" }
    },
    "ageCategory": {
      "type": ["string", "null"],
      "enum": ["minor", "adult", "elder", null]
    },
    "isPseudonymous": { "type": "boolean", "default": false },
    "acceptingConnections": { "type": "boolean", "default": true },
    "languagePreferences": {
      "type": ["object", "null"],
      "properties": {
        "primary": { "type": "string" },
        "secondary": { "type": ["string", "null"] },
        "learningLevel": { "enum": ["beginner", "intermediate", "advanced", null] }
      }
    },
    "accessibilityNeeds": {
      "type": "array",
      "items": { "type": "string" }
    },
    "flags": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["type", "reason"],
        "properties": {
          "type": { "type": "string" },
          "reason": { "type": "string" },
          "count": { "type": ["integer", "null"] },
          "severity": { "enum": ["info", "caution", "warning", "restriction", null] }
        }
      }
    },
    "claimedAttestations": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["claim", "status"],
        "properties": {
          "claim": { "type": "string" },
          "status": { "enum": ["pending", "verified", "unverified", "disputed", "revoked"] },
          "challengedAt": { "type": ["string", "null"], "format": "date" },
          "verifiedBy": { "type": ["string", "null"], "description": "Human id of verifier" }
        }
      },
      "description": "Claims made BY this human about their own credentials. Contrast with Attestation entries issued ABOUT them by others."
    }
  },
  "additionalProperties": false
}
```

### Human relationship type vocabulary

The largest new piece — the enum that's currently a `String` in Rust with no source of truth. Dimensioned vocabulary where each type declares its semantic properties.

```json
"$defs": {
  "HumanRelationshipType": {
    "type": "string",
    "enum": [
      "spouse", "parent-of", "child-of", "sibling", "grandparent-of", "grandchild-of", "extended-family",
      "guardian-of", "ward-of", "caregiver-of", "key-steward-of",
      "coworker", "supervises", "reports-to", "business-partner",
      "neighbor", "congregation-member", "community-member",
      "mentor-of", "mentee-of", "learning-partner",
      "acquaintance"
    ]
  }
}
```

**Semantic table** (documented in schema `relationshipTypeSemantics` field; this is operational source of truth until a future sprint promotes the vocabulary into a notarized Rust constant module):

| Type | Dimension | Directionality | Default Intimacy | Custody-enabling | Notes |
|---|---|---|---|---|---|
| `spouse` | family | bidirectional | intimate | yes | Auto-custody default |
| `parent-of` | family | directional | intimate | yes (parent→child) | Inverse: `child-of` |
| `child-of` | family | directional | intimate | no | Inverse of `parent-of` |
| `sibling` | family | bidirectional | trusted | no | |
| `grandparent-of` | family | directional | trusted | no | Inverse: `grandchild-of` |
| `grandchild-of` | family | directional | trusted | no | |
| `extended-family` | family | bidirectional | trusted | no | Aunts/uncles/cousins/in-laws/chosen-family |
| `guardian-of` | caregiving | directional | trusted | yes (guardian→ward) | Formal responsibility, not necessarily family |
| `ward-of` | caregiving | directional | trusted | no | Inverse of `guardian-of` |
| `caregiver-of` | caregiving | directional | trusted | no | Informal care (elder, childcare) |
| **`key-steward-of`** | **caregiving** | **directional** | **trusted** | **no** | **Recovery network — declarative at genesis; runtime triggers RecoveryRequest** |
| `coworker` | work | bidirectional | connection | no | Requires `context` field with org id |
| `supervises` | work | directional | connection | no | Inverse: `reports-to` |
| `reports-to` | work | directional | connection | no | |
| `business-partner` | work | bidirectional | trusted | no | Cross-organizational |
| `neighbor` | community | bidirectional | connection | no | Geographic proximity |
| `congregation-member` | community | bidirectional | connection | no | Shared faith community |
| `community-member` | community | bidirectional | recognition | no | Generic shared community |
| `mentor-of` | learning | directional | trusted | no | Inverse: `mentee-of` |
| `mentee-of` | learning | directional | trusted | no | |
| `learning-partner` | learning | bidirectional | connection | no | Peer learning |
| `acquaintance` | weak-tie | bidirectional | recognition | no | |

**Migration from existing genesis data** — all 13 currently-used relationship types map cleanly:

| Current (humans.json) | New vocabulary |
|---|---|
| `spouse` | `spouse` |
| `parent` | `parent-of` |
| `grandparent` | `grandparent-of` |
| `sibling` | `sibling` |
| `coworker` | `coworker` |
| `neighbor` | `neighbor` |
| `congregation_member` | `congregation-member` |
| `learning_partner` | `learning-partner` |
| `mentee` | `mentee-of` |
| `business_partner` | `business-partner` |
| `acquaintance` | `acquaintance` |
| `network_connection` | `acquaintance` (absorbed — same semantic) |
| `community_member` | `community-member` |

Migration is a deterministic rename (underscore→hyphen + `parent` → `parent-of` family).

### Relationship placement — single-source, per-entity auto-generated view

`relationships.md` is the single source of truth for all human-to-human relationships. Each human's markdown file gets an auto-generated `## Relationships` section derived from the central file. Same pattern as the collectives sprint's `relationships[]` top-level array.

```markdown
# genesis/data/humans/relationships.md
---
relationships:
  - source: human-matthew-manager
    target: human-jessica-spouse
    type: spouse
    intimacyLevel: intimate
    startedAt: 2010-06-15
  - source: human-matthew-manager
    target: human-james-son
    type: parent-of
    intimacyLevel: intimate
  - source: human-matthew-manager
    target: human-jessica-spouse
    type: key-steward-of
    intimacyLevel: intimate
    notes: "Jessica is in Matthew's recovery network"
---

# Human Relationships

This file is the single source of truth for all relationships between the
33 humans in the genesis cast. Individual human markdown files have an
auto-generated "Relationships" section derived from this file.
```

### `presences.schema.json` structure

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "elohim:protocol:presences",
  "title": "ContributorPresence Frontmatter Schema — operational source of truth for observation model; notarized projection target is contributor_presences DHT entry type",
  "type": "object",
  "required": ["id", "displayName", "presenceType", "observations"],
  "properties": {
    "id": {
      "type": "string",
      "pattern": "^presence-[a-z0-9][a-z0-9-]*[a-z0-9]$"
    },
    "displayName": { "type": "string", "minLength": 1 },
    "presenceType": {
      "enum": ["person", "organization"],
      "description": "Discriminator. Rides in metadata_json."
    },
    "bio": { "type": ["string", "null"] },
    "observations": {
      "type": "array",
      "minItems": 1,
      "items": { "$ref": "#/$defs/Observation" }
    },
    "primaryStewardId": {
      "type": ["string", "null"],
      "description": "Current steward's human id. Defaults to observations[0].observerId if unset."
    },
    "stewardshipStartedAt": { "type": ["string", "null"], "format": "date-time" },
    "externalIdentifiers": {
      "type": "array",
      "items": { "$ref": "#/$defs/ExternalIdentifier" }
    },
    "sameAsPresenceIds": {
      "type": "array",
      "items": { "type": "string", "pattern": "^presence-" },
      "default": [],
      "description": "Known duplicate presences. Empty at genesis; populated by future merge work."
    },
    "works": {
      "type": "array",
      "items": { "$ref": "#/$defs/Work" }
    },
    "suggestedCollectiveIds": {
      "type": "array",
      "items": { "type": "string", "description": "Soft hints for the claim flow" }
    },
    "tags": {
      "type": "array",
      "items": { "type": "string" },
      "description": "Free-form tags. Keen sections migrate here (digital-governance-and-democracy, media-and-books, etc.)"
    },
    "image": {
      "type": ["object", "null"],
      "required": ["local"],
      "properties": {
        "local": {
          "type": "string",
          "pattern": "^images/[a-z0-9-]+\\.webp$"
        },
        "placeholder": { "type": "boolean" },
        "sourceUrl": { "type": ["string", "null"] }
      }
    },
    "note": { "type": ["string", "null"] }
  },
  "additionalProperties": false,
  "$defs": {
    "Observation": {
      "type": "object",
      "required": ["observerId", "observedAt", "context"],
      "properties": {
        "observerId": { "type": "string", "description": "Human id. Must reference humans/." },
        "observedAt": { "type": "string", "format": "date-time" },
        "context": { "type": "string", "minLength": 1 },
        "contextContentId": { "type": ["string", "null"], "description": "Content node id. Must reference if non-null." }
      }
    },
    "ExternalIdentifier": {
      "type": "object",
      "required": ["type", "value"],
      "properties": {
        "type": {
          "enum": ["orcid", "isni", "wikipedia", "wikidata", "linkedin", "twitter", "mastodon", "personal-domain", "doi", "isbn", "arxiv", "github", "homepage", "email"]
        },
        "value": { "type": "string", "minLength": 1 }
      }
    },
    "Work": {
      "type": "object",
      "required": ["title", "kind"],
      "properties": {
        "title": { "type": "string" },
        "kind": { "enum": ["book", "paper", "talk", "podcast", "video", "project", "organization", "website", "other"] },
        "year": { "type": ["integer", "null"] },
        "url": { "type": ["string", "null"] },
        "citedInContentIds": {
          "type": "array",
          "items": { "type": "string" }
        }
      }
    }
  }
}
```

### Content → presence citation edges

Each `ContentNode` in `genesis/data/lamad/content/*.json` gains an optional `contributors[]` array that rides in `metadata_json` at the Rust boundary:

```json
{
  "id": "fct-module-12-fairness-justice",
  "title": "Fairness and Justice",
  "content": "...",
  "relatedNodeIds": [...],
  "stewardedBy": [...],
  "contributors": [
    {
      "presenceId": "presence-virginia-eubanks",
      "contributionType": "cited",
      "weight": null,
      "context": "Cited 'Automating Inequality' for the data-and-inequality section"
    }
  ]
}
```

**Contribution type vocabulary:**

| Type | Use | Example |
|---|---|---|
| `author` | Primary creator of cited work | Virginia Eubanks → *Automating Inequality* content node |
| `co-author` | Shared authorship | Kahneman + Tversky → behavioral economics papers |
| `cited` | Work is referenced inside this content | Virginia Eubanks → FCT Module 12 (Pete authored, cited her) |
| `inspired` | Shaped the thinking without explicit citation | Stafford Beer → manifesto |
| `endorser` | Publicly supports this content | |
| `editor` | Edited someone else's work | |
| `translator` | Translated | |
| `interviewee` | Subject of interview content | Schmachtenberger → Future Thinkers podcast |
| `speaker` | Gave talk captured as content | |

**Weights** default to `null` across the board — weight assignment is deferred to a dedicated shefa-flow-wiring sprint. Null means "unassigned, routes via default policy later."

### Two-way graph consistency

Forward edge: `content.contributors[]` → presence
Reverse edge: `presence.works[].citedInContentIds[]` → content

The validator enforces bidirectional agreement:

```
For every content node C with contributors[P, ...]:
  assert presence P exists
  assert P.works contains an entry with citedInContentIds including C.id
    OR contribution type is 'inspired' (inspiration doesn't require a concrete work entry)

For every presence P with works[].citedInContentIds[C, ...]:
  assert content node C exists
  assert C.contributors contains an entry with presenceId = P.id
```

The validator also exposes a `--fix` flag that, given an edge declared on only one side, materializes the mirror and writes it back. Useful during authoring.

### Migration plan

One-shot script `genesis/scripts/migrate-content-to-presences.ts`. Dry-run by default. Single entry point structured in five phases that share in-memory state.

**Phase 1: Parse Keen** — read `Consilience_Garden-.../keen.json`, build presence records for all 103 gems. Each gets one observation by Matthew dated 2021-08-07 with context `"Collected in Consilience Garden keen, section: {section.title}"`. Tags: slugified section name + `consilience-garden`. External identifiers: `[{type: homepage, value: metalink.url}]`. `presenceType` classified heuristically from URL pattern (wikipedia/twitter/linkedin/personal → person; foundation/org/project/gov → organization). Images downloaded best-effort with placeholder fallback.

**Phase 2: Parse FCT contributor stubs** — read the 31 `fct-contributor-*.json` files, build presence records. Deduplicate against Phase 1 presences by fuzzy name match (Keen + FCT overlaps get a single presence with two observations — Matthew 2021, Pete 2026). `primaryStewardId` is set to Pete for modules Pete cites in.

**Phase 3: Keen "Media and Books" → book ContentNodes** — create new `book-*.json` content nodes for the books in the Keen, using real metadata from `metalink`. Hand-mapped author table (3-4 books):

```json
{
  "Finite and Infinite Games": { "authorSlug": "james-p-carse", "displayName": "James P. Carse" },
  "The Collapse of Complex Societies": { "authorSlug": "joseph-tainter", "displayName": "Joseph Tainter" },
  "The Ministry for the Future": { "authorSlug": "kim-stanley-robinson", "displayName": "Kim Stanley Robinson" }
}
```

Each book gets a `contributors[]` entry pointing at the author presence; each author presence gets a `works[].citedInContentIds[]` entry pointing back at the book. Bidirectional.

**Phase 4: Build rewrite plan** — scan all `genesis/data/lamad/content/*.json` files. For each file:
- Find `relatedNodeIds` entries matching dead-file patterns (`fct-contributor-*`, `human-*`, `governance-organizations-*`, `governance-books-*`)
- Remove dead references
- Add corresponding presence to `contributors[]` with inferred type
- Special case: `manifesto.json` gains `contributors[]` listing all Keen-migrated presences with `contributionType: inspired, weight: null`

**Phase 5: Execute** (gated on `--execute` flag):
- Write all presence `.md` files with frontmatter + narrative body + auto-observations block
- Download images best-effort; track failures in warnings
- Write new book content nodes
- Write modified content files
- Delete 113 dead content files (31 FCT + 27 humans + 52 orgs + 3 books)
- Delete `Consilience_Garden-.../` directory
- Delete the migration script itself and its helper files

**Execution safety:**
- Dry-run default produces a diff report to stdout without touching filesystem
- `--execute` runs inside a `git stash` try/restore-on-failure wrapper
- Final commit is a massive reviewable diff in a feature branch
- Pre-push hook runs the full validator chain

### Image capture pipeline

Uses `sharp` (added as devDependency for the migration; removed when script is deleted) for format conversion and resizing.

```typescript
async function acquireImage(presence: PresenceRecord): Promise<ImageField> {
  // 1. Prefer local gem_images/ from Keen checkout
  if (presence.sourceGemId && localGemImageExists(presence.sourceGemId)) {
    const src = `Consilience_Garden-.../gem_images/${presence.sourceGemId}.{webp,png}`;
    await sharp(src).resize(500, 500, { fit: 'cover' }).webp({ quality: 80 }).toFile(targetPath);
    return { local: `images/${targetName}`, placeholder: false };
  }

  // 2. Download external metalink.image
  if (presence.externalImageUrl) {
    try {
      const response = await fetch(presence.externalImageUrl, { timeout: 10_000 });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const buffer = Buffer.from(await response.arrayBuffer());
      await sharp(buffer).resize(500, 500, { fit: 'cover' }).webp({ quality: 80 }).toFile(targetPath);
      return { local: `images/${targetName}`, placeholder: false, sourceUrl: presence.externalImageUrl };
    } catch (err) {
      warnings.push(`Image download failed for ${presence.id}: ${err.message} — using placeholder`);
    }
  }

  // 3. Fall back to placeholder by presenceType
  const placeholderName = {
    'person': 'placeholder-person.webp',
    'organization': 'placeholder-organization.webp',
  }[presence.presenceType] ?? 'placeholder-generic.webp';

  return {
    local: `images/${placeholderName}`,
    placeholder: true,
    sourceUrl: presence.externalImageUrl ?? null
  };
}
```

**Tauri offline story:** external URLs rot over time; desktop peers can't render a presence card if the image 404's. Local capture means the genesis world renders fully offline from day one.

**Size budget:** ~100 downloads × 500×500 webp at 80% quality ≈ 3-8MB total. Plus three ~50KB placeholder files. ~5-10MB total committed binaries — comfortable for git without LFS.

### Referential integrity rules (validators)

These validators operate on the operational source of truth layer (markdown frontmatter) before the data is projected into the notarized DHT entries. JSON Schema can't express cross-file references. Hand-rolled validators enforce:

**`validate-humans.ts`:**
1. Slug consistency: filename stem matches frontmatter id
2. Uniqueness: no duplicate ids across markdown files
3. DisplayName uniqueness: no two humans share a displayName (catches Pete aliasing)
4. `guardianIds[]` resolve to existing humans
5. `organizations[].id` resolves to a collective id OR a presence id
6. `communities[]` resolve to collective ids
7. Every relationship `source`/`target` resolves to a human
8. Directionality enforcement: `parent-of` requires both parties exist; validator materializes `child-of` as inverse (not hand-authored)
9. Bidirectional consistency for bidirectional types
10. Age/guardianship coherence: `ageCategory: minor` without `guardianIds` warns (handles Tiffany red-team case — warn, don't error)
11. No circular `guardianIds` chains
12. `coworker` relationships require `context` (must be an organization id)

**`validate-presences.ts`:**
1. Slug consistency
2. Uniqueness across presence files
3. Every `observations[].observerId` resolves to an existing human
4. Every `observations[].contextContentId` resolves to an existing content node (or is null)
5. `primaryStewardId` resolves to an existing human (or is null)
6. `externalIdentifiers[]` uniqueness: no two presences share the same `(type, value)` pair — early merge-conflict detection
7. `sameAsPresenceIds[]` consistency: mutual references OR declared in relationships.md
8. `works[].citedInContentIds[]` entries resolve to existing content nodes
9. At least one observation (non-empty array)
10. `image.local` path resolves to an existing file in `presences/images/`
11. `suggestedCollectiveIds[]` resolve to existing collective ids

**`validate-content.ts` (new or expanded):**
1. Every `contributors[].presenceId` resolves to an existing presence
2. Every `contributors[].contributionType` in the vocabulary
3. Weights, when specified, in `[0, 1]` and sum to ≤ 1.0
4. Bidirectional consistency with `presence.works[].citedInContentIds[]`
5. `stewardedBy` and `contributors[]` are orthogonal (warn if a contributor with weight > 0 is also a steward — fee-collection anti-pattern)

**Narrative coherence checks (CI, post-commit not pre-push):**
1. Every human in markdown produces an account package (generator + diff)
2. Every `human "Name"` pattern in a2o feature files resolves to a humans.json entry (excluding scenarios tagged for synthetic registration)
3. DisplayName uniqueness across all registered humans

### Mapping frontmatter to Rust view types

**Humans → `CreateHumanInputView`:**

| Frontmatter field | Rust field | Transport |
|---|---|---|
| `id` | `id` | direct |
| `displayName` | `display_name` | direct |
| `bio` | `bio` | direct |
| `profileReach` | `profile_reach` | direct |
| `affinities` | `affinities` | direct |
| `agencyPhase` | (metadata) | metadata_json |
| `category` | (metadata) | metadata_json |
| `location` | (metadata) | metadata_json |
| `organizations` | (metadata) | metadata_json |
| `communities` | (metadata) | metadata_json |
| `guardianIds` | (metadata) | metadata_json |
| `ageCategory` | (metadata) | metadata_json |
| `isPseudonymous` | (metadata) | metadata_json |
| `acceptingConnections` | (metadata) | metadata_json |
| `languagePreferences` | (metadata) | metadata_json |
| `accessibilityNeeds` | (metadata) | metadata_json |
| `flags` | (metadata) | metadata_json |
| `claimedAttestations` | (metadata) | metadata_json |

**Presences → `CreateContributorPresenceInputView`:**

| Frontmatter field | Rust field | Transport |
|---|---|---|
| `id` | `id` | direct |
| `displayName` | `display_name` | direct |
| `bio` | `note` | direct (repurposing) |
| `presenceType` | (metadata) | metadata_json |
| `observations` | (metadata) | metadata_json |
| `primaryStewardId` | `steward_id` | direct |
| `stewardshipStartedAt` | `stewardship_started_at` | direct |
| `externalIdentifiers` | `external_identifiers_json` | JSON-serialized to String field |
| `sameAsPresenceIds` | (metadata) | metadata_json |
| `works` | (metadata) | metadata_json |
| `suggestedCollectiveIds` | (metadata) | metadata_json |
| `tags` | (metadata) | metadata_json |
| `image` | `image` | direct |

**Initial state at POST time:**
- `presence_state`: `"stewarded"` if `primaryStewardId` is set (always true after generator defaults), else `"unclaimed"`
- `established_at`: earliest `observations[].observedAt`
- `accumulating_since`: same as `established_at`
- `affinity_total`, `unique_engagers`, `citation_count`, `recognition_score`: all start at 0 (accumulated at runtime)

## Red Team Analysis

### Divorce, death, remarriage
**Handled:** `HumanRelationship.expires_at` for runtime termination; `RelationshipRenewal` for transitions; `AgentRetirement` for death; remarriage = new relationship entry; attestations from former parties persist (valid at issue time, revocation is a separate act). **Seed data is current-state only; history is runtime concern.**

### Children becoming adults
**Handled:** `ageCategory: minor` + `guardianIds[]` at t=0; `StewardshipGrant`/`PolicyInheritance` DNA primitives handle runtime transitions; `guardian-of` relationship has `expires_at` for legal-majority triggers.

### Identity theft / compromised keys
**Schema addition — `key-steward-of` relationship type.** Operational source of truth is the humans schema; the notarized projection is the existing `HumanRelationship` DHT entry type with a new type value. Declares recovery network at genesis time: "Matthew's key is stewarded by Jessica, Dan, and Pete for recovery purposes." Runtime triggers `RecoveryRequest`/`RecoveryVote`/`KeyRevocation` — all existing DNA primitives. No new DNA work.

### Cross-cultural family structures
**Handled:** `spouse` allows multiple instances (polycules have N pairwise); `extended-family` is the chosen-family catch-all; `caregiver-of` handles non-formal care; collective naming convention `household-{name}` (not `couple-{a}-{b}`) handles triads/communes/multi-gen households; collectives' fuzzy reach ranges support any scale without hard-coding nuclear-family assumptions.

### Estrangement and withdrawal
**Handled:** `HumanRelationship.expires_at` terminates; `acceptingConnections: false` blocks new connections; collective `participates-in` allows withdrawal (collectives sprint's dissenter protection); attestations persist until revoked via superseding entry. **Attestation revocation mechanism is deferred** (no DNA primitive yet; future work).

### False attestations
**Schema addition — `claimedAttestations[]` field on humans.** This rides on the notarized `Human` entry's metadata_json projection; operational source of truth is the humans schema. Captures red-team personas (Dr. Dolittle claiming unverified MD) and legitimate foreign-credential cases (Ronald the refugee). Status enum: `pending`, `verified`, `unverified`, `disputed`, `revoked`. Contrast with `Attestation` entries (issued by others). Runtime: `IdentityChallenge`/`ChallengeSupport`/`IdentityFreeze` DNA primitives handle disputes.

### Duplicate presence merges (new)
**Substrate in place:** `externalIdentifiers[]` uniqueness check catches accidental genesis-time duplicates; `sameAsPresenceIds[]` carries the join once detected; observations **never get rewritten on merge** — each original observer's attribution stays attached to its original presence, and merge just links the presences. **Future work:** elohim graph traversal discovers merge candidates by external-identifier overlap; claim process at the human's side validates.

### Observer-over-time (new)
**Handled:** `observations[]` allows multiple entries with same `observerId` — each content-citation moment is a distinct observation with its own context and content reference. Matches the "each piece of content could trigger a value flow event back to the contributor presence" semantic from scoping.

### Image replacement (new)
**Substrate in place:** `image.placeholder: true` flag surfaces the opportunity; `image.sourceUrl` preserved even when placeholder is in use; runtime replacement flow is future work (image contributors get credited under a future contribution type).

### Reclaim after death (historical figures)
**Handled via existing DNA:** `primaryStewardId` stays with observer; `claim_verification_method` field supports estate/executor verification; `claim_facilitated_by` field supports mediated claims. **`deceasedAt` field explicitly NOT added this sprint** — it's research work (getting death dates right for 100+ figures), and the claim process is where deceased-vs-alive matters at runtime. Backwards-compatible to add later.

## Narrative Coherence Fixes

Evidence-based cross-check against humans.json, account-packages, collectives.json, and 42 a2o feature files found:

### Clean (majority)
- 27/27 internal humans.json relationships resolve
- 23/23 `communities[]` refs → collectives.json
- 15/15 `organizations[].id` refs → collectives.json
- All account-package collective refs resolve (the collectives-rename commit landed cleanly)
- 29/29 account-package `humanId` refs → humans.json (no orphans)

### Real drift to fix in this sprint

1. **4 humans without account packages:** `human-ezra-newcomer`, `human-levi-contributor`, `human-miriam-author`, `human-susan-household`. Fix: account-package generator naturally closes the gap when run.

2. **Pete name alias drift:** Pete's `displayName` is `"Pastor Pete"` in humans.json; a2o scenarios use `human "Pete"`; fixture lookup can't resolve. **Fix: rename to `displayName: "Pete"`.** The pastoral role is still captured by `category: affinity` + `organizations[].role: pastor`. Consistency with other humans (single-word first names).

3. **Sammy bug:** `auth/fixture-humans.feature` line 19 references `human "Sammy"` — no such human in registry. Likely a typo. **Fix: flag in sprint PR; 1-line correction during review.**

### Observational (backlog, not sprint)

10 humans never referenced in a2o scenarios. Four are red-team personas that specifically need scenarios to exercise protective behavior: **Dr. Dolittle** (unverified credentials), **Tiffany** (unguarded minor), **Renold** (exclusionary behavior), **Ginny** (value scanner target). Flag as story-harvest item for a dedicated red-team scenario coverage sprint.

## Files to create/modify

All artifacts below respect the source of truth classifications declared at the top of this doc — schemas are operational IoC contracts, markdown files are canonical operational sources, JSON files are derived projections from markdown, and the underlying entities are notarized in the imagodei DNA.

| Action | File | Purpose |
|---|---|---|
| **Create** | `genesis/data/humans/humans.schema.json` | IoC contract (operational source of truth; projection of notarized Human entry type) |
| **Create** | `genesis/data/humans/matthew-manager.md` | ...and 32 others | canonical markdown source per human |
| **Create** | `genesis/data/humans/relationships.md` | central relationship registry |
| **Create** | `genesis/data/humans/humans.json` | generated artifact |
| **Create** | `genesis/data/humans/README.md` | generated index |
| **Create** | `genesis/data/presences/presences.schema.json` | IoC contract (operational source of truth; projection of notarized ContributorPresence entry type) |
| **Create** | `genesis/data/presences/*.md` | ~110 canonical markdown sources |
| **Create** | `genesis/data/presences/relationships.md` | presence-to-presence edges |
| **Create** | `genesis/data/presences/presences.json` | generated artifact |
| **Create** | `genesis/data/presences/README.md` | generated with Consilience Garden narrative |
| **Create** | `genesis/data/presences/images/placeholder-*.webp` | 3 committed placeholders |
| **Create** | `genesis/data/presences/images/*.webp` | ~100 downloaded + optimized |
| **Create** | `genesis/data/lamad/content/book-*.json` | real book content nodes replacing stubs |
| **Modify** | `genesis/data/lamad/content/manifesto.json` | new contributors[] |
| **Modify** | `genesis/data/lamad/content/fct-module-*.json` | contributors[] + relatedNodeIds rewrites |
| **Modify** | `genesis/data/collectives/collectives.json` | joinable governance orgs migrated from Keen (opt-in, small) |
| **Regenerate** | `genesis/data/account-packages/*.json` | 4 new, 29 with presence refs |
| **Delete** | `genesis/data/lamad/content/fct-contributor-*.json` | 31 orphan stubs |
| **Delete** | `genesis/data/lamad/content/human-*.json` | 27 duplicates |
| **Delete** | `genesis/data/lamad/content/governance-organizations-*.json` | 52 orphan stubs |
| **Delete** | `genesis/data/lamad/content/governance-books-*.json` | 3 orphan stubs |
| **Delete** | `genesis/docs/humans/humans.json` | moved to `genesis/data/humans/` |
| **Delete** | `Consilience_Garden-nvBCVtcYER7C9s3H6zxS/` | entire directory, after migration |
| **Create** | `genesis/seeder/src/validate-humans.ts` | hand-rolled validator |
| **Create** | `genesis/seeder/src/validate-presences.ts` | hand-rolled validator |
| **Create** | `genesis/seeder/src/validate-content.ts` | or expand existing; contributors[] checks |
| **Create** | `genesis/seeder/src/seed-presences.ts` | new seeder stage |
| **Create** | `genesis/seeder/src/build-data.ts` | markdown→JSON generator |
| **Modify** | `genesis/seeder/src/seed-humans.ts` | reads from build-data output |
| **Modify** | `genesis/seeder/src/seed-collectives.ts` | accepts joinable orgs |
| **Modify** | `genesis/seeder/src/account-package.ts` | resolves presence IDs |
| **Modify** | `genesis/seeder/package.json` | new pnpm scripts |
| **Create + Delete in same commit** | `genesis/scripts/migrate-content-to-presences.ts` | one-shot migration |
| **Create + Delete in same commit** | `genesis/scripts/migration-author-map.json` | helper for book migration |
| **Modify** | `genesis/Jenkinsfile` | Seed Presences stage |
| **Modify** | `.husky/pre-push` | new validation hooks, freshness check |
| **Modify** | `.claude/file-relationships.json` | humans-presences-sync rules |

## Not in scope

- **No Rust changes.** Every primitive exists; every HTTP route exists. Schema IS the source of truth for fields that don't have Rust columns yet.
- **No new DHT entry types.** Imagodei stays at 28; Lamad stays at ~73. No entry type budget spent.
- **No shefa flow ratio wiring.** Weights default to `null`; ratio assignment is a dedicated future sprint.
- **No `Attestation` entries for observations.** Rides on metadata bag for this sprint; clean upgrade path documented.
- **No `PresenceMerge`/`SameAs` link type.** Substrate (`externalIdentifiers[]`, `sameAsPresenceIds[]`) is in place; future elohim-traversal sprint does the merge discovery work.
- **No runtime image replacement flow.** Substrate (`image.placeholder: true`, `image.sourceUrl`) is in place; future contribution-type sprint wires replacement credit.
- **No `deceasedAt` field on presences.** Backwards-compatible to add when needed; claim process handles deceased-vs-alive at runtime.
- **No a2o scenario coverage for red-team personas.** Flagged as story-harvest item for a dedicated sprint (Dolittle, Tiffany, Renold, Ginny, James minor-flow).
- **No attestation revocation mechanism.** No DNA primitive exists yet; future work.
- **No collectives markdown retrofit.** Keep collectives.json as-is; future sprint can retrofit to match the humans/presences markdown pattern if desired.
- **No content-addressed book CIDs.** Current IPFS sprint handles content addressing; this sprint uses slug IDs.

## Relationship to existing protocol concepts

| Concept | How this sprint connects |
|---|---|
| **Consilience Garden (2021)** | Migrated to protocol presences; directory deleted; narrative preserved in README |
| **EPR three-leg coupling** | Presences + content form the lamad leg of flow routing (recognition → contributor) |
| **ReachLevels** | `profileReach` field uses protocol enum; inherits collectives work |
| **Graduated capability** | `agencyPhase` enum declares initial state; `StewardshipGrant`/`PolicyInheritance` handle runtime transitions |
| **Recovery network** | `key-steward-of` relationship type declares at genesis; `RecoveryRequest`/`RecoveryVote` handle runtime |
| **Claim process** | `externalIdentifiers[]` and `sameAsPresenceIds[]` are the merge substrate for future elohim graph traversal |
| **Observer attribution** | `observations[]` metadata bag → future `Attestation` entries with `type: presence-observation` |
| **Shefa flow wiring** | `contributors[].weight` is the future flow-routing substrate (null now, populated in dedicated sprint) |
| **Elohim token exit** | Presence claim flow + `recognition_score` accumulation feed the token minting path (separate initiative) |
| **Tauri offline story** | Local image capture means genesis world renders fully offline |
| **Story-first development** | This sprint's data powers existing a2o scenarios and surfaces 4 red-team scenario gaps for backlog |
| **`legacy_prefixes` Diesel routing** | Not touched this sprint (already fixed in earlier commit) |

## Commit shape

Single reviewable PR on `dev` branch. Pre-push hook runs full validator chain (humans + presences + content + collectives + bidirectional consistency + freshness check). No `HUSKY=0` — if anything is broken, push fails loudly and we fix at the source.

**Sprint execution landmarks:**
1. Create schemas + validators (no data yet, just contracts)
2. Run migration script in dry-run; review diff
3. Run migration script with `--execute`; commit generated data + script-deletion together
4. Regenerate account packages for the 4 missing humans
5. Fix Pete displayName rename + Sammy typo in a2o features
6. Verify all a2o scenarios still pass (no behavior changes expected)
7. Land on dev
