# App Manifest SDK Boundary Design

## Problem

The Elohim Protocol conflates protocol-level primitives with application-level vocabulary. Content types like `assessment`, `work-story`, `garden-plot` are hardcoded into `content-type.schema.json` alongside protocol structural types. Every new app requires modifying the protocol schema. This doesn't scale, and it's architecturally wrong — the protocol should validate **coupling structure**, not **vocabulary**.

## Vision

A P2P-native developer growing up in the elohim network doesn't "register an app." They create a new kind of content. They describe what it is, how it couples to the three legs (knowledge + value + governance), and how it renders. The protocol validates the coupling. The app validates its own vocabulary. The elohim agent understands both through contextual reasoning.

The app manifest IS an EPR — content-addressed, stewarded, governed. It's the contract between an application and the protocol.

## Architecture

### Three-Layer Validation Stack

```
┌─────────────────────────────────────────────────────────────────┐
│  ELOHIM AGENT (contextual understanding)                        │
│  Reads manifest + content + signals. Reasons about meaning.     │
│  Mediates between app vocabulary and protocol understanding.    │
├─────────────────────────────────────────────────────────────────┤
│  APP MANIFEST (vocabulary validation)                           │
│  Per-app EPR declaring content types, body schemas, coupling    │
│  rules, relationship patterns, signal mappings, renderers.      │
│  The app owns its vocabulary. Protocol doesn't enumerate it.    │
├─────────────────────────────────────────────────────────────────┤
│  PROTOCOL CORE (coupling validation)                            │
│  EPR Head structure, stewardship allocation shape, reach        │
│  levels, substrate signals (attention/compute/storage/          │
│  bandwidth/resource). Validates structure, not vocabulary.      │
├─────────────────────────────────────────────────────────────────┤
│  STORAGE (permissive)                                           │
│  Accepts any content type string. Stores everything.            │
│  The data plane doesn't judge — it serves.                      │
├─────────────────────────────────────────────────────────────────┤
│  DNA (notarized core)                                           │
│  Immutable structural validation. EPR Head shape.               │
│  Stewardship coupling. Agent identity. Economic events.         │
└─────────────────────────────────────────────────────────────────┘
```

### What Lives Where

**Protocol core** (coupling structure — `elohim/sdk/schemas/`):
- EPR Head shape (id, stewardship, reach, governance layer)
- Reach levels (private, self, intimate, trusted, familiar, community, public, commons)
- Stewardship allocation structure (fractional, multi-steward)
- Substrate signal types (attention, compute, storage, bandwidth, resource)
- Validation status lifecycle (draft, active, disputed, superseded)
- `contentType` field exists and is `string` — protocol validates presence, not content

**App manifest** (vocabulary + rendering — EPR in the protocol):
- Content type vocabulary with JSON Schema for each type's content body
- Content format vocabulary with renderer registrations
- Three-leg coupling declarations per content type
- Relationship pattern declarations
- Semantic signal mappings (app signals on top of substrate signals)

### Protocol Core Signals (Substrate)

Every EPR interaction implicitly generates substrate signals. These are the physical-layer fundamentals that any app's value flows compose on top of:

| Signal | What it captures | What consumes it |
|--------|-----------------|------------------|
| **attention** | Time, focus, engagement depth | Constitutional review (spiral detection) |
| **compute** | Inference cycles, validation, rendering | Infrastructure capacity planning |
| **storage** | Bytes replicated, shards held | Carrying capacity, steward recognition |
| **bandwidth** | Bytes transferred, peers served | Network health, steward recognition |
| **resource** | Material quantities (REA units) | Circularity deficit accumulator |

These are captured by the protocol regardless of which app created the content. When someone views a garden-plot or completes an assessment, the substrate signals flow automatically. Apps add semantic meaning on top.

```
Protocol (substrate)          App Manifest (semantic)
---------------------         -------------------------
attention signal      <--+    "learning-signal" (lamad)
compute signal        <--+    "tending-event" (garden)
storage signal        <--+    "governance-participation" (qahal)
bandwidth signal      <--+    "stewardship-act" (shefa)
resource signal       <---    "harvest-yield" (garden, with quantity/unit)
```

### App Manifest Structure

The manifest is an EPR — a ContentNode with `contentType: "app-manifest"`:

```json
{
  "id": "manifest-lamad",
  "version": "1.0.0",
  "name": "lamad",
  "description": "Learning and mastery platform",

  "vocabulary": {
    "contentTypes": {
      "assessment": {
        "description": "Graded evaluation of understanding",
        "schema": { "$ref": "schemas/assessment.schema.json" },
        "coupling": {
          "knowledge": {
            "relationships": {
              "MEASURES": "concept",
              "GENERATES": "mastery-record"
            }
          },
          "value": {
            "onConsume": {
              "action": "use",
              "resourceConformsTo": "learning",
              "recognition": "steward-weighted"
            },
            "onComplete": {
              "action": "produce",
              "resourceConformsTo": "mastery-attestation",
              "recognition": "author + steward"
            }
          },
          "governance": {
            "defaultReach": "community",
            "minimumReach": "intimate",
            "governanceModel": "steward-consent",
            "signalTypes": ["mastery-achieved", "assessment-completed"]
          }
        }
      },
      "lesson": { ... },
      "path": { ... },
      "discovery-assessment": { ... },
      "instrument": { ... }
    },

    "contentFormats": {
      "sophia": {
        "description": "Sophia assessment engine format",
        "renderer": "sophia-renderer",
        "mimeType": "application/vnd.sophia+json"
      },
      "sophia-quiz-json": {
        "description": "Sophia quiz data",
        "renderer": "sophia-renderer"
      },
      "perseus": {
        "description": "Perseus exercise format",
        "renderer": "perseus-renderer"
      }
    },

    "relationships": {
      "MEASURES": {
        "description": "Assessment measures understanding of a concept",
        "source": ["assessment"],
        "target": ["concept"]
      },
      "PRACTICES": {
        "description": "Exercise practices a concept",
        "source": ["exercise"],
        "target": ["concept"]
      }
    },

    "signals": {
      "learning-signal": {
        "description": "Content consumption event",
        "substrateSignal": "attention",
        "economicAction": "use",
        "resourceType": "learning"
      },
      "mastery-achieved": {
        "description": "Learner reached mastery threshold",
        "substrateSignal": "attention",
        "economicAction": "produce",
        "resourceType": "mastery-attestation"
      },
      "assessment-completed": {
        "description": "Assessment finished with score",
        "substrateSignal": "compute",
        "economicAction": "use",
        "resourceType": "evaluation"
      }
    }
  },

  "rendering": {
    "sophia-renderer": {
      "component": "SophiaRendererComponent",
      "formats": ["sophia", "sophia-quiz-json"],
      "platform": "angular"
    },
    "perseus-renderer": {
      "component": "PerseusRendererComponent",
      "formats": ["perseus", "perseus-json", "perseus-quiz-json"],
      "platform": "angular"
    },
    "markdown-renderer": {
      "component": "MarkdownRendererComponent",
      "formats": ["markdown", "gherkin", "plaintext", "text", "html"],
      "platform": "angular"
    }
  }
}
```

### Value Flow Per Content Type

Each content type declares its full REA event pattern:

| Content Type | onConsume (use) | onComplete (produce) | Recognition |
|-------------|----------------|---------------------|-------------|
| lesson | learning signal | mastery update | steward-weighted |
| assessment | engagement event | mastery attestation | author + steward |
| path | journey signal | path-completion attestation | all stewards in path |
| instrument | psychometric signal | self-knowledge attestation | instrument author |
| discovery-assessment | engagement event | affinity discovery | author + steward |

These map to hREA `EconomicEvent` entries. The manifest declares WHAT and WHEN. The protocol enforces that value flows correctly through stewardship allocations.

### How a New App Uses This

A developer building a community garden tool:

1. **Creates a manifest EPR** declaring `garden-plot`, `planting-schedule`, `harvest-report` with their schemas, coupling, and value flows
2. **Builds rendering components** — a map view for plots, a calendar for schedules, a dashboard for harvests
3. **Publishes the manifest** — it's an EPR, stewarded by the developer, governed by the garden community
4. **The protocol validates** — every garden-plot has valid stewardship, reach, and economic event declarations
5. **The elohim agent reads the manifest** — it understands what a garden-plot is, how to reason about soil health, how to aggregate garden signals for constitutional review

The developer never modifies the protocol schema. They compose with protocol primitives. The manifest IS their SDK contract.

### Codegen From Manifest

The manifest enables type generation for the app:

```
manifest-lamad.json
    |
    +--> TypeScript types (content body shapes per content type)
    +--> Validation functions (content body validation per type)
    +--> Renderer registry (format -> component mapping)
    +--> Signal emitters (economic event helpers per content type)
    +--> Coupling validators (three-leg coupling checks)
```

This replaces the current approach of hardcoding content types in `schema-enums.ts`. The protocol codegen generates coupling structure types. The app codegen generates vocabulary types from the manifest.

### Migration Path (Parallel Build)

**Phase 1: Create the Lamad manifest (EPR)**
- Extract lamad-specific vocabulary from `content-type.schema.json` extensible tier
- Extract content body schemas from existing JSON data
- Extract value flow declarations from seed.ts and mastery.service.ts
- Extract rendering registrations from content-node.model.ts
- Write the manifest as `genesis/data/manifests/manifest-lamad.json`

**Phase 2: Build manifest codegen**
- Create `manifest:codegen:ts` script that reads app manifests
- Generate `LamadContentType`, `LamadContentFormat` union types
- Generate content body validation functions
- Generate signal emitter helpers
- Run alongside existing `schema:codegen:ts` (parallel, not replacing)

**Phase 3: Wire manifest types into lamad pillar**
- Import generated lamad types alongside protocol types
- `ContentType = ProtocolContentType | LamadContentType` (same pattern as today's `WireContentType | AppContentTypeExtension`)
- Validate content bodies against manifest schemas
- Run both validation paths in parallel (existing + manifest)

**Phase 4: Cutover**
- Once confident: protocol schema `extensible` tier empties out
- Protocol validates coupling only
- App manifest validates vocabulary
- Remove hardcoded content types from protocol schema
- Enforce manifest-based validation

### What This Design Does NOT Cover (Yet)

- **Manifest discovery**: How apps find each other's manifests on the network (Kademlia?)
- **Manifest governance**: How manifest changes are governed (steward consent? community vote?)
- **Cross-app relationships**: How a garden app references a lamad assessment (manifest composition?)
- **Manifest versioning**: How manifests evolve without breaking existing content
- **Elohim agent manifest consumption**: How the agent reads and reasons about manifests at runtime

These will emerge through building the first manifest (lamad) and discovering what's missing.

### Connection to Current Work

The compile-time typing work done in this session (schema IoC enforcement) is scaffolding:
- **Keeps working** during the parallel build phase
- **Catches bugs** in the monorepo where we control all code
- **Demonstrates the pattern** of schema-as-contract that the manifest generalizes
- **Gets replaced** by manifest-based codegen when the cutover happens

The `content-type.schema.json` core tier may survive as protocol structural types (or may not — the DNA validates EPR coupling structure, and "content type" might be purely app vocabulary). The extensible tier migrates into app manifests.
