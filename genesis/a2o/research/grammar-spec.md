# The Grammar Fork: Human-Readable Gherkin + Agent-Resolved Protocol

**Status:** Research
**Date:** 2026-03-30
**Depends on:** [vision.md](vision.md)

---

## Principle: The Grammar Does Not Change

The Gherkin grammar — Feature, Scenario, Given/When/Then, Background, tags, tables, Scenario Outline, Examples — stays **exactly as it is**. No new keywords. No protocol syntax in the feature file body. No CIDs visible to scenario authors.

**Agents absorb all protocol complexity.**

This is the interpretability contract: the human-readable surface IS the interpretable claim. If the scenario becomes unreadable to a non-developer, the protocol has failed. The entire purpose of behavioral interpretability is that a human can read the claim, understand it, and judge whether the observation makes sense — without needing to understand content addressing, economic events, or DHT mechanics.

---

## What Changes: Tag Vocabulary

Existing Gherkin tags (`@e2e`, `@wip`, `@browser-only`, `@regression`, etc.) are free-form strings. The protocol defines a **tag vocabulary** that agents recognize and resolve. These tags are optional — scenarios without them still work. Tags are added incrementally by stewards, not required at authoring time.

### Content Addressing Tags

| Tag | Meaning | Example |
|-----|---------|---------|
| `@validates:{content-id}` | This scenario validates a specific ContentNode | `@validates:manifesto-content` |
| `@depends-on:{scenario-id}` | Prerequisite scenario that must pass first | `@depends-on:auth-lifecycle` |
| `@derived-from:{scenario-id}` | This scenario was forked from another | `@derived-from:basic-content-create` |

Content IDs in tags use **human-readable slugs**, not raw CIDs. Agents resolve slugs to CIDs via the content graph. This keeps tags readable:

```gherkin
@e2e @content @validates:governance-basics
Feature: Content Lifecycle
```

Not:

```gherkin
@validates:bafyreib3k7j2m4n5p6q7r8s9t0u1v2w3x4y5z
Feature: Content Lifecycle
```

### Economic Tags

| Tag | Meaning | Example |
|-----|---------|---------|
| `@mints:{observation-type}` | What observation this scenario produces on execution | `@mints:behavioral-claim` |
| `@freshness:{duration}` | ISO 8601 validity horizon | `@freshness:P30D` |

### Governance Tags

| Tag | Meaning | Example |
|-----|---------|---------|
| `@reach:{level}` | Observation reach level | `@reach:community` |
| `@replication:{count}` | Minimum independent replications for attestation | `@replication:3` |

### Composition with Existing Tags

Protocol tags compose with existing tags. Order doesn't matter. Agents parse protocol tags; Cucumber ignores them (they're just strings to Cucumber).

```gherkin
@e2e @auth @validates:auth-session @freshness:P7D @reach:commons
Feature: Auth Lifecycle
```

---

## The Dual Surface

### Surface 1: What Humans Write

Pure Gherkin. The feature file that a scenario author creates:

```gherkin
@e2e @content @validates:manifesto-content
Feature: Content Lifecycle

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  Scenario: Create content and verify retrieval
    Given human "Matthew" is logged in on doorway "alpha"
    When Matthew creates content titled "Governance Basics"
    Then the content should be created successfully
    And the content should be retrievable by ID
```

This is what goes in the `.feature` file. This is what humans read, review, and understand. This is the interpretable claim.

### Surface 2: What Agents Resolve

From that feature file, agents compute the full protocol representation. This is **never written by humans** — it's computed from the feature file content plus protocol context:

```json
{
  "contentType": "observation-scenario",
  "cid": "bafyrei...computed-from-content-hash",
  "title": "Create content and verify retrieval",
  "contentFormat": "gherkin",
  "coupling": {
    "knowledge": {
      "relationships": {
        "VALIDATES": ["manifesto-content"],
        "DEPENDS_ON": [],
        "CONTAINS": ["step:given-matthew-logged-in", "step:when-creates-content", "step:then-created-successfully", "step:then-retrievable-by-id"]
      }
    },
    "value": {
      "onExecute": {
        "action": "observe",
        "resourceConformsTo": "scenario-validity",
        "recognition": "observation-credit"
      }
    },
    "governance": {
      "defaultReach": "commons",
      "governanceModel": "steward-consent",
      "signalTypes": ["observation-signal"]
    },
    "claims": [
      {
        "asserts": "behavioral-claim-holds",
        "contradictedBy": "behavioral-claim-fails",
        "validityHorizon": "P30D"
      }
    ]
  }
}
```

### Resolution Rules

1. Tags starting with protocol vocabulary (`@validates:`, `@depends-on:`, `@mints:`, `@freshness:`, `@reach:`, `@replication:`, `@derived-from:`) are agent-resolved
2. Agents **MUST NOT** modify the feature file to add protocol metadata
3. Protocol metadata lives in a **sidecar** `.meta.json` file (in the content graph) or as a DHT entry — never in the `.feature` file itself
4. When a `.feature` file changes, the CID changes, and agents must re-resolve all dependent metadata
5. Slug-to-CID resolution uses the same content graph traversal as EPR link resolution

---

## Sidecar Metadata Pattern

Each `.feature` file may have a corresponding `.meta.json` sidecar that agents maintain. This parallels the EPR three-tier model:

| EPR Tier | Scenario Equivalent |
|----------|-------------------|
| **Head** (~500 bytes, gossipped) | Feature title, scenario CIDs, relationship edges, governance reach |
| **Document** (5-50 KB) | Full Gherkin text, three-leg coupling, claims, observation history |
| **Bytes** (any size) | Execution artifacts (ephemeral — destroyed after observation is minted) |

Example sidecar: `content-lifecycle.meta.json`

```json
{
  "featureFile": "features/content/content-lifecycle.feature",
  "featureCid": "bafyrei...feature-hash",
  "scenarios": [
    {
      "name": "Create content and verify retrieval",
      "cid": "bafyrei...scenario-hash",
      "validates": ["manifesto-content"],
      "dependsOn": [],
      "freshness": "P30D",
      "reach": "commons",
      "lastObservation": "2026-03-28T14:22:00Z",
      "validityScore": 0.92,
      "replicationCount": 4
    }
  ]
}
```

The sidecar is **generated by agents, not authored by humans**. It is the protocol's view of the scenario — the enriched representation that makes economic events and replication possible.

---

## Scenarios as ContentNodes

### New Content Type: `observation-scenario`

Added to a new **a2o domain manifest** (following the structure of `elohim/sdk/domains/lamad/manifest.json`):

```json
{
  "observation-scenario": {
    "description": "A behavioral claim expressed as Given/When/Then, content-addressed and peer-verifiable",
    "coupling": {
      "knowledge": {
        "relationships": {
          "VALIDATES": ["concept", "lesson", "assessment", "path", "epic", "article", "exercise", "scenario", "reflection", "discussion"],
          "DEPENDS_ON": ["observation-scenario"],
          "DERIVED_FROM": ["observation-scenario"],
          "CONTAINS": ["observation-step"]
        }
      },
      "value": {
        "onExecute": {
          "action": "observe",
          "resourceConformsTo": "scenario-validity",
          "recognition": "observation-credit"
        }
      },
      "governance": {
        "defaultReach": "commons",
        "minimumReach": "community",
        "governanceModel": "steward-consent",
        "signalTypes": ["observation-signal", "replication-achieved", "validity-expired"]
      },
      "claims": [
        {
          "asserts": "behavioral-claim-holds",
          "contradictedBy": "behavioral-claim-fails",
          "validityHorizon": "P30D"
        }
      ]
    }
  }
}
```

### New Content Type: `observation-step`

The atomic unit — a single Given, When, or Then clause:

```json
{
  "observation-step": {
    "description": "A single precondition, action, or postcondition within an observation scenario",
    "coupling": {
      "knowledge": {
        "relationships": {
          "BELONGS_TO": ["observation-scenario"],
          "RELATES_TO": ["observation-step"]
        }
      },
      "value": {
        "onExecute": {
          "action": "observe",
          "resourceConformsTo": "step-validity",
          "recognition": "observation-credit"
        }
      },
      "governance": {
        "defaultReach": "commons",
        "governanceModel": "steward-consent"
      }
    }
  }
}
```

Steps are shared across scenarios via CID deduplication. If two scenarios share the same `Given human "Matthew" is logged in on doorway "alpha"` step, they reference the same step CID. Step reuse maps to content reuse.

---

## Interaction with Existing Feature Files

### Migration Path

1. **Sprint 0**: Existing 30 `.feature` files get CIDs computed from their content. **No file modification required.** CID assignment is a read-only operation.
2. **Sprint 2**: Stewards optionally add protocol tags (`@validates:`, `@freshness:`, etc.) to feature files. This is additive and non-breaking — Cucumber ignores unknown tags.
3. **Ongoing**: New scenarios written with protocol tags from the start. Existing scenarios gain tags as stewards curate their domains.

### Step Definition Mapping

Existing step definitions (`steps/*.steps.ts`, `steps/ui/*.steps.ts`) become the **instrument** that executes the observation. Step definitions are not ContentNodes — they are executors, analogous to the instruments in the feedback design (`retention-check`, `outcome-correlation`, `community-report`).

The step definition is the "how" of observation. The scenario step is the "what" — the claim. The instrument executes the claim and reports the result. This separation means:

- Step definitions can be refactored without changing scenario CIDs
- Multiple implementations can execute the same scenario (HTTP vs Playwright vs Steward device)
- The claim (Gherkin text) is stable; the instrument (step definition) evolves

### `@wip` Scenarios

Scenarios tagged `@wip` are **incomplete observations** — hypotheses without instruments. They still get CIDs (the claim exists even if the instrument doesn't). In economic terms, a `@wip` scenario is an unfulfilled `Intent` — an expressed desire for an observation that hasn't been produced yet. This maps to the REA model: `Intent` → `Commitment` → `EconomicEvent`.

---

## Open Questions

1. **Scenario Outlines**: Parameterized scenarios with `Examples:` tables generate multiple scenario instances. Should each example row get its own CID, or should the outline get one CID with the table as metadata? Per-row CIDs enable finer-grained tracking but multiply DHT entries.

2. **Background as Precondition**: `Background` steps run before every scenario in a feature. Should they be factored into the scenario CID (making CIDs feature-file-dependent) or treated as implicit shared preconditions?

3. **Feature-Level vs Scenario-Level**: The current design assigns CIDs at the scenario level. Should features (which group scenarios) also be ContentNodes? Features carry the narrative context ("Why these scenarios matter") that scenarios lack individually.

4. **Slug Stability**: Protocol tags use human-readable slugs (`@validates:manifesto-content`). If the referenced content's slug changes, tags break. How does the protocol handle slug evolution? CIDs don't change, but slugs do.

5. **Cross-Manifest References**: Observation scenarios `VALIDATES` content from the lamad manifest. How do cross-manifest relationships work? The a2o manifest references lamad content types, but the lamad manifest has no knowledge of a2o. Is this a one-way dependency, or does lamad need to declare that it can be validated?
