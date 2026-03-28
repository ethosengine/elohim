# Feedback as Information Flow — Protocol Design

**Date:** 2026-03-28
**Status:** Approved (brainstorm validated)
**Predecessor:** `2026-03-27-feedback-fourth-pillar-prompt.md` (thinking prompt)

## Summary

Feedback is a protocol primitive expressed as **information flow** through the existing three-legged coupling (knowledge + value + governance). The stool stays three-legged, but the legs gain a circulatory system: every content type must declare what it **claims** to produce, every claim must be **observable** (including by human flags/reports), and accumulated negative observations generate **obligations** that demand re-examination.

The protocol validates the shape of honesty — not the content of claims.

## Core Insight

Every system without feedback goes chaotic (Destin Sandlin / cybernetics). The costs of any system accumulate outside the accounting mechanism (Center for Humane Technology). The protocol's existing accounting tracks what happened (signals, economic events, recognition). Nothing tracks what it **cost** beyond the accounting boundary. Feedback makes the hidden balance sheet visible.

**Key principle:** Accumulated feedback creates a responsibility to re-examine the original justification. Not "you're wrong" — the protocol asking, honestly, "is the reasoning you gave when you declared this still holding up?"

**The therapeutic model:** Feedback is a mirror, not a judge. Mastery decay isn't punishment for forgetting. Standing adjustment isn't demotion for bad curation. They're honest reflections that create space for correction without shame.

## Design Decision: Three Layers, Not a Fourth Leg

Feedback is not a fourth structural leg. It is the information that flows back through the three existing legs. Three layers compose to form the feedback primitive:

1. **Claims** (declaration layer — manifest schema)
2. **Observations** (evidence layer — signals and instruments)
3. **Obligation Accumulation** (accountability layer — REA economics)

---

## Layer 1: Claims

Each content type's coupling gains a required `claims` array. A claim declares what outcome this content type asserts it produces, what observation would contradict that assertion, and how long the claim is presumed valid without fresh evidence.

### Schema Addition

`ThreeLegCoupling` gains a required `claims` property:

```json
"ThreeLegCoupling": {
  "required": ["value", "governance", "claims"],
  "properties": {
    "knowledge": { "$ref": "#/$defs/KnowledgeLeg" },
    "value": { "$ref": "#/$defs/ValueLeg" },
    "governance": { "$ref": "#/$defs/GovernanceLeg" },
    "claims": {
      "type": "array",
      "items": { "$ref": "#/$defs/ClaimDeclaration" },
      "minItems": 1
    }
  }
}
```

### ClaimDeclaration

```json
"ClaimDeclaration": {
  "required": ["asserts", "contradictedBy", "validityHorizon"],
  "properties": {
    "asserts": {
      "type": "string",
      "description": "Observation term this content type claims to produce. References vocabulary.observations."
    },
    "contradictedBy": {
      "type": "string",
      "description": "Observation term that would undermine this claim. References vocabulary.observations. Must have negative polarity."
    },
    "validityHorizon": {
      "type": "string",
      "description": "ISO 8601 duration. How long the claim is presumed valid without fresh evidence. Accumulated positive feedback extends it; accumulated contradictions shorten it.",
      "pattern": "^P"
    },
    "leg": {
      "type": "string",
      "enum": ["knowledge", "value", "governance"],
      "description": "Which coupling leg this claim is about. Optional — for documentation clarity."
    }
  }
}
```

### Example: Quiz Content Type

```json
{
  "description": "Lightweight graded question set",
  "coupling": {
    "value": {
      "onComplete": { "action": "produce", "recognition": "mastery-credit" }
    },
    "governance": {
      "defaultReach": "commons",
      "minimumReach": "community",
      "governanceModel": "steward-consent"
    },
    "claims": [
      {
        "asserts": "knowledge-retention",
        "contradictedBy": "retention-failure",
        "validityHorizon": "P30D",
        "leg": "knowledge"
      },
      {
        "asserts": "mastery-attestation-meaningful",
        "contradictedBy": "downstream-prerequisite-failure",
        "validityHorizon": "P90D",
        "leg": "value"
      }
    ]
  }
}
```

### Non-Learning Example: Cooperative Marketplace Listing

```json
{
  "description": "A product listing in a cooperative marketplace",
  "coupling": {
    "value": {
      "onConsume": { "action": "use", "recognition": "market-engagement" }
    },
    "governance": {
      "defaultReach": "community",
      "minimumReach": "community",
      "governanceModel": "cooperative-consent"
    },
    "claims": [
      {
        "asserts": "fair-price",
        "contradictedBy": "buyer-regret-above-threshold",
        "validityHorizon": "P60D",
        "leg": "value"
      },
      {
        "asserts": "supply-diversity",
        "contradictedBy": "supplier-concentration",
        "validityHorizon": "P180D",
        "leg": "governance"
      }
    ]
  }
}
```

---

## Layer 2: Observations

The manifest vocabulary gains an `observations` section alongside the existing `signals`. Observations are the vocabulary of evidence — things the system watches for.

### Schema Addition

`Vocabulary` gains a required `observations` property:

```json
"Vocabulary": {
  "required": ["contentTypes", "observations"],
  "properties": {
    "contentTypes": { ... },
    "contentFormats": { ... },
    "relationships": { ... },
    "signals": { ... },
    "observations": {
      "type": "object",
      "additionalProperties": { "$ref": "#/$defs/ObservationDeclaration" },
      "minProperties": 1
    }
  }
}
```

### ObservationDeclaration

```json
"ObservationDeclaration": {
  "required": ["description", "instrument", "polarity"],
  "properties": {
    "description": {
      "type": "string",
      "description": "Human-readable description of what this observation means."
    },
    "instrument": {
      "type": "string",
      "description": "Instrument archetype that produces this observation. References a protocol-defined archetype or app-defined extension."
    },
    "polarity": {
      "type": "string",
      "enum": ["positive", "negative"],
      "description": "Positive observations extend claim validity horizons. Negative observations shorten them."
    }
  }
}
```

### Polarity Enforcement

The protocol validates that the vocabulary contains at least one negative-polarity observation. An app that only declares positive observations is rejected — you cannot ship feedback that only amplifies.

### Instrument Archetypes

The protocol defines instrument archetypes — named patterns for how observations are produced. Apps reference these; the protocol validates the reference resolves.

| Archetype | Source | What It Observes | Cycle |
|-----------|--------|-----------------|-------|
| `retention-check` | Automated | Does the claimed outcome persist over time? | Short (days-weeks) |
| `outcome-correlation` | Automated | Does the forward event produce the downstream effect? | Medium (weeks-months) |
| `distribution-health` | Automated | Is power/resource/role concentrating beyond thresholds? | Medium |
| `cost-accumulation` | Automated | Are externalities building outside the accounting? | Continuous |
| `outcome-divergence` | Automated | Did a governance decision produce its intended effect? | Long (months) |
| `community-report` | Human | A person observed something instruments didn't catch | Event-driven |

The `community-report` archetype connects existing flags/reports to the feedback system. A flag is a human-initiated observation that feeds into the same accumulation cycle as instrument readings.

### Example: Lamad Observations

```json
"observations": {
  "knowledge-retention": {
    "description": "Learner can recall and apply concept after interval",
    "instrument": "retention-check",
    "polarity": "positive"
  },
  "retention-failure": {
    "description": "Learner cannot recall or apply concept after interval",
    "instrument": "retention-check",
    "polarity": "negative"
  },
  "mastery-attestation-meaningful": {
    "description": "Mastery-attested learners succeed in downstream prerequisites",
    "instrument": "outcome-correlation",
    "polarity": "positive"
  },
  "downstream-prerequisite-failure": {
    "description": "Mastery-attested learners fail in downstream content",
    "instrument": "outcome-correlation",
    "polarity": "negative"
  },
  "stewardship-not-concentrated": {
    "description": "Content domain has diverse steward participation",
    "instrument": "distribution-health",
    "polarity": "positive"
  },
  "concentration-above-threshold": {
    "description": "Single steward controls >40% of domain content",
    "instrument": "distribution-health",
    "polarity": "negative"
  },
  "content-misleading": {
    "description": "Content framing doesn't match substance (human-reported)",
    "instrument": "community-report",
    "polarity": "negative"
  },
  "content-outdated": {
    "description": "Content no longer reflects current understanding (human-reported)",
    "instrument": "community-report",
    "polarity": "negative"
  }
}
```

---

## Layer 3: Obligation Accumulation

Observations produce evidence. Evidence accumulates against claim validity. When validity drops below threshold, the system generates obligations — expressed as REA economic events.

### The Accumulation Cycle

```
Instrument produces observation
        ↓
  positive polarity → extends claim validity horizon
  negative polarity → shortens claim validity horizon
        ↓
  validity horizon expires → claim enters "review" state
        ↓
  review produces one of:
    • revalidation (horizon resets, claim stands)
    • correction (claim parameters adjusted)
    • escalation (elohim deliberation required)
```

### REA Expression

Observations are recorded as standard `EconomicEvent`s — no new DHT entry type needed:

```json
{
  "action": "observe",
  "resourceConformsTo": "claim-validity",
  "provider": "instrument:retention-check",
  "receiver": "claim:comprehension@concept:algebraic-identity",
  "resourceQuantityValue": -0.15,
  "lamadEventType": "claim-observation",
  "metadata": {
    "polarity": "negative",
    "observation": "retention-failure",
    "evidenceBasis": "3-of-5-learners-failed-recall-at-14d"
  }
}
```

The `observe` action is valid in ValueFlows. The resource is `claim-validity` — a measurable quantity that increases or decreases. Existing EconomicEvent infrastructure handles routing, tracing, and storage.

### Obligation Generation

When accumulated validity drops below threshold:

```json
{
  "action": "commit",
  "resourceConformsTo": "review-obligation",
  "provider": "instrument:retention-check",
  "receiver": "governance:steward-consent@concept:algebraic-identity",
  "metadata": {
    "triggerClaim": "knowledge-retention",
    "validityAtTrigger": 0.28,
    "accumulatedEvidence": 7
  }
}
```

The obligation sits in the governance-scoped pool — visible on the balance sheet.

### Three Escalation Paths

| Path | Trigger | Actor | Outcome |
|------|---------|-------|---------|
| **Automated revalidation** | Fresh positive evidence arrives | Instrument | Validity recovers, obligation dissolves |
| **Steward correction** | Obligation visible to steward | Human | Content/assessment/claim adjusted, obligation dissolves |
| **Elohim escalation** | Obligation persists past SLA | Elohim agent | Systemic investigation, governance mechanism triggered |

These mirror the circularity deficit accumulator: obligations build until the system responds, then dissipate. Self-healing.

### Community Reports as Observation Sources

Flags and reports feed the same accumulation cycle:

1. Human flags content (level 0 governance — always available)
2. Flag produces a `community-report` observation with negative polarity
3. Elohim engages: "Can you say more about what felt misleading?"
4. Human's response becomes structured evidence in the FeedbackTrace
5. Multiple flags on the same claim accumulate → shorten validity → generate obligation
6. Steward sees not just "3 flags" but traced narrative of what was observed and why

The flag becomes a conversation, the conversation becomes evidence, the evidence becomes accountability.

---

## Elohim Integration: The Feedback Narrator

### FeedbackTrace

Parallel to StageTrace (recognition distribution narrative), every claim observation produces a traceable record:

```
FeedbackTrace {
  claimId: "comprehension@concept:algebraic-identity"
  validityBefore: 0.72
  observation: "retention-failure"
  source: "instrument:retention-check" | "community-report:human-abc"
  evidenceBasis: "recall-accuracy 0.3 at 14d interval"
  validityAfter: 0.57
  obligationGenerated: false
  correctionApplied: null
}
```

### Three Narrative Roles

| Role | Trigger | Audience | Style |
|------|---------|----------|-------|
| **Mirror** | Individual claim observation | Learner | Therapeutic — "here's what I see, here's what you might do" |
| **Advisor** | Obligation generated | Steward | Actionable — "here's the pattern, here are options" |
| **Sentinel** | Systemic pattern across claims | Governance | Systemic — escalates through mechanism ladder |

**Mirror example (to learner):**
> "Your algebra mastery was strong two months ago. The retention check suggests it's faded — that's normal, not failure. Want to revisit the core concepts? A 15-minute refresher would restore it."

**Advisor example (to steward):**
> "Three concepts in this path have validity below 0.4. Learners are completing the quizzes but not retaining at 30 days. The assessments might need more application-based questions."

**Sentinel example (to governance):**
> "Across the algebra domain, retention is 40% lower than geometry. This isn't a single content problem — it may be a structural issue with how algebraic concepts are sequenced."

---

## Protocol Enforcement Summary

### Schema Validates (structural)

1. Every content type has at least one claim (`claims.minItems: 1`)
2. Every claim references observations that exist in the vocabulary (`asserts` and `contradictedBy` resolve)
3. The vocabulary declares at least one negative-polarity observation
4. Every observation references a valid instrument archetype
5. Every claim has a validity horizon (ISO 8601 duration)

### Schema Does NOT Validate (semantic)

- Whether the claims are true — instruments determine that at runtime
- Whether the instruments are good — quality concern, not schema concern
- What the right validity horizon is — apps judge for their domain
- How correction happens — protocol provides escalation paths, apps choose

### Codegen

Existing `schema:codegen:ts` pipeline generates:
- `ClaimDeclaration` interface
- `ObservationDeclaration` interface
- Per-app observation vocabulary types (discriminated unions, like content types today)

Existing `schema:validate` gains: "does this manifest declare feedback?" check.

---

## Existing Infrastructure This Builds On

| Existing Pattern | How Feedback Uses It |
|-----------------|---------------------|
| `content_mastery.rs` freshness decay (5%/day) | Already implements `retention-check` archetype |
| `steward_standing.rs` dispute penalty | Already implements `distribution-health` archetype |
| `resource_nature.rs` circularity obligations | Template for obligation accumulation cycle |
| `recognition_pipeline_service.rs` StageTrace | Template for FeedbackTrace |
| `imagodei_observations.rs` trust_delta + visibility layers | Constitutional memory for observation persistence |
| `feedback-profile.model.ts` mechanism gating + mediation | Connects flags/reports to observation vocabulary |
| `aggregation-instruments.scaffold.ts` filter → aggregate → evaluate | Pattern for feedback instrument implementation |
| Governance mechanism ladder (levels 0-7) | Escalation path for sentinel role |
| EconomicEvent with `observe` action | REA-native expression of claim observations |

---

## What This Design Does NOT Cover (Future Work)

- **Correction propagation:** When a concept's mastery decays, do downstream concepts also decay? Graph-aware correction is a separate design.
- **Feedback gaming:** If retention checks can be gamed the same way quizzes are gamed, the instruments need adversarial robustness. Separate design.
- **Feedback-as-externality recursion:** Do feedback instruments themselves impose costs that need observation? Probably yes at scale, but not in the first implementation.
- **Cross-app feedback:** When one app's content claims affect another app's outcomes, how do observations flow across manifest boundaries? Protocol-level concern for later.
- **Instrument implementation details:** This design defines archetypes and declaration patterns. The actual instrument algorithms (how retention-check works, what thresholds trigger obligations) are implementation decisions per archetype.

---

## Relationship to Previous Work

- **Supersedes:** The `ThreeLegCoupling → FourLegCoupling` direction from `project-feedback-as-primitive.md`. The stool stays three-legged; feedback is information flow, not a structural leg.
- **Builds on:** `2026-03-27-feedback-fourth-pillar-prompt.md` (thinking prompt that surfaced the externalities and design questions).
- **Connects to:** Governance gateway sprints 5-9 (challenge/appeal, signal accumulation, Polis sensemaking, elohim deliberation).
