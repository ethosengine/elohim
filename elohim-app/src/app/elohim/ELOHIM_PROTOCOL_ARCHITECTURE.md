# Elohim Protocol Architecture

## Overview

The Elohim Protocol is organized into five interconnected pillars, each owning specific concerns while collaborating through shared infrastructure. This document defines the pillar relationships, ownership boundaries, and composition patterns.

---

## The Five Pillars

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              ELOHIM (Protocol Core)                         │
│          Infrastructure, agents, data loading, source chain, trust          │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐     │
│  │   LAMAD     │   │   QAHAL     │   │   SHEFA     │   │  IMAGODEI   │     │
│  │  Learning   │   │  Community  │   │   Economy   │   │  Identity   │     │
│  │             │   │             │   │             │   │             │     │
│  │ - Content   │   │ - Consent   │   │ - REA       │   │ - Session   │     │
│  │ - Paths     │   │ - Governance│   │ - Events    │   │ - Profile   │     │
│  │ - Mastery   │   │ - Feedback  │   │ - Presence  │   │ - Attestation│    │
│  │ - Maps      │   │ - Places    │   │ - Tokens    │   │ - Journey   │     │
│  └─────────────┘   └─────────────┘   └─────────────┘   └─────────────┘     │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                           SHARED (Cross-Pillar)                             │
│         Services and utilities used by multiple pillars                     │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Pillar Descriptions

| Pillar | Hebrew | Meaning | Primary Concern |
|--------|--------|---------|-----------------|
| **Elohim** | אֱלֹהִים | Divine messengers | Protocol infrastructure, AI guardians |
| **Lamad** | לָמַד | To learn/teach | Content, learning paths, knowledge maps |
| **Qahal** | קהל | Assembly | Community, consent, governance, feedback |
| **Shefa** | שפע | Abundance/flow | Economic events, REA, recognition |
| **Imagodei** | Imago Dei | Image of God | Identity, profile, attestations |

---

## Model Ownership Matrix

Each model has a canonical location. Other pillars may re-export through barrel files but should not duplicate the model definition.

### Elohim (Protocol Core) - Canonical Models

| Model | File | Purpose |
|-------|------|---------|
| `protocol-core.model.ts` | ReachLevel, GovernanceLayer, IntimacyLevel, ConsentState | Shared primitives |
| `agent.model.ts` | Agent, AgentProgress, MasteryLevel | Base agent types |
| `elohim-agent.model.ts` | ElohimAgent, ConstitutionalLayer | AI guardian types |
| `source-chain.model.ts` | SourceChainEntry, LinkType | Holochain patterns |
| `trust-badge.model.ts` | TrustIndicator | Trust display |
| `rea-bridge.model.ts` | REAAction, Measure, ResourceSpecification | ValueFlows ontology |
| `economic-event.model.ts` | EconomicEvent, LamadEventType | Event audit trail |
| `contributor-presence.model.ts` | ContributorPresence, PresenceState | Stewardship lifecycle |
| `human-consent.model.ts` | ConsentRecord, ConsentState | Consent management |

### Lamad (Learning) - Canonical Models

| Model | File | Purpose |
|-------|------|---------|
| `content-node.model.ts` | ContentNode, ContentType | Territory |
| `learning-path.model.ts` | LearningPath, PathStep | Journey |
| `content-mastery.model.ts` | ContentMastery, MasteryLevel | Bloom's progression |
| `knowledge-map.model.ts` | KnowledgeMap (domain, self) | Knowledge visualization |
| `exploration.model.ts` | ExplorationQuery | Graph traversal |
| `search.model.ts` | SearchQuery, SearchResult | Discovery |
| `path-extension.model.ts` | PathExtension | Learner customization |
| `feedback-profile.model.ts` | FeedbackProfile, FeedbackMechanism | Engagement constraints |

### Qahal (Community) - Canonical Models

| Model | File | Purpose |
|-------|------|---------|
| `governance-deliberation.model.ts` | DeliberationProposal, Vote | Loomio-style deliberation |
| `governance-feedback.model.ts` | Challenge, Appeal, Precedent | Constitutional accountability |
| `human-affinity.model.ts` | AffinityRecord | Engagement tracking |
| `place.model.ts` | Place, BioregionalContext | Geographic context |

### Shefa (Economy) - Uses Elohim Models

Shefa uses models from `elohim/models/`:
- `rea-bridge.model.ts`
- `economic-event.model.ts`
- `contributor-presence.model.ts`

### Imagodei (Identity) - Canonical Models

| Model | File | Purpose |
|-------|------|---------|
| `session-human.model.ts` | SessionHuman, SessionStats | Temporary identity |
| `profile.model.ts` | HumanProfile, JourneyStats | Human-centered view |
| `attestations.model.ts` | AgentAttestation | Credentials earned BY humans |

---

## Shared Module

Services used by multiple pillars live in `shared/`:

```
shared/
├── models/
│   └── trust-badge-config.ts
├── services/
│   ├── affinity-tracking.service.ts
│   ├── governance.service.ts
│   ├── human-consent.service.ts
│   └── profile.service.ts
└── utils/
    ├── access-control.helper.ts
    └── id-generator.ts
```

### When to Use Shared

Move to `shared/` when:
- Two or more pillars import the service
- The service has no single "owning" pillar
- The service implements cross-pillar linking logic

Keep in pillar when:
- Only one pillar uses it
- The service is tightly coupled to pillar models
- Moving would create circular dependencies

---

## Cross-Pillar Linking

The `CrossPillarLinkType` in `protocol-core.model.ts` defines how entities across pillars relate:

```typescript
type CrossPillarLinkType =
  // Lamad ↔ Qahal
  | 'content_governance'      // Content has governance dimension
  | 'path_governance'         // Path has governance dimension
  | 'feedback_profile'        // Content has feedback constraints

  // Lamad ↔ Shefa
  | 'content_economic_event'  // Content view creates economic event
  | 'path_economic_event'     // Path completion creates event
  | 'contributor_presence'    // Content attributed to presence

  // Lamad ↔ Imagodei
  | 'agent_progress'          // Agent has progress on path
  | 'content_mastery'         // Agent has mastery on content

  // Qahal ↔ Shefa
  | 'governance_economic_event' // Governance action creates event
  | 'challenge_economic_cost'   // Challenge may have economic cost

  // Qahal ↔ Imagodei
  | 'consent_relationship'    // Consent between agents
  | 'attestation_governance'  // Attestation grants governance rights

  // Shefa ↔ Imagodei
  | 'agent_economic_ledger'   // Agent has economic activity
  | 'presence_claim'          // Presence claimed by agent

  // All pillars
  | 'elohim_stewardship';     // Elohim stewards entity
```

---

## Specification Documents

Each pillar has its own API specification:

| Pillar | Specification | Content |
|--------|--------------|---------|
| Lamad | `lamad/LAMAD_API_SPECIFICATION_v1.0.md` | Routes, models, services for learning |
| Qahal | `qahal/QAHAL_API_SPECIFICATION_v1.0.md` | Governance, feedback, consent |
| Elohim | This document | Architecture overview |
| Shefa | (Future) | Economic events, REA |
| Imagodei | (Future) | Identity, session management |

---

## Import Conventions

### Preferred: Direct Pillar Import

```typescript
// Import from canonical location
import { DataLoaderService } from '@app/elohim/services';
import { ContentNode } from '@app/lamad/models';
import { GovernanceService } from '@app/elohim/services';
```

### Acceptable: Barrel Re-export

```typescript
// Works via re-export, but prefer direct
import { DataLoaderService } from '@app/lamad/services';
```

### Avoid: Duplicate Definitions

Never create duplicate model definitions. If you need a type, import it from its canonical location or add a re-export to the pillar's barrel file.

---

## Holochain Migration Path

All models are designed to migrate to Holochain entry types:

1. **IDs** become ActionHash or EntryHash
2. **Progress** moves to agent's private source chain
3. **Attestations** become countersigned DHT entries
4. **ReachLevel** determines DHT publication scope
5. **Timestamps** remain ISO 8601 strings

The `DataLoaderService` in `elohim/services/` is the single point of change for Holochain migration.

---

## Constitutional Principles

All pillars operate under the Elohim Protocol's constitutional principles:

1. **Data Sovereignty** - Humans own their data (progress, attestations live on their chain)
2. **Graduated Intimacy** - Privacy levels with consent at transitions
3. **Constitutional Accountability** - Every decision can be challenged
4. **Fog of War** - Visibility earned through demonstrated capability
5. **Care Economics** - Value through care/stewardship, not extraction

See the manifesto at `elohim-app/src/assets/docs/manifesto.md` for full vision.
