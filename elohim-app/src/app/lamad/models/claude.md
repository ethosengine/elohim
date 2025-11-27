# Lamad Models: Implementation Guide

*Data models for the Lamad learning platform. All models are complete and stable.*

**Last updated:** 2025-11-27

## Model Architecture

```
Six-Layer Model:
├── Territory (Content)
│   ├── content-node.model.ts      # ContentNode, ContentType, ContentReach
│   └── content-attestation.model.ts # ContentAttestation types
│
├── Journey (Learning Paths)
│   ├── learning-path.model.ts     # LearningPath, PathStep, PathStepView
│   └── path-extension.model.ts    # Learner mutations
│
├── Traveler (Humans)
│   ├── agent.model.ts             # Agent, AgentProgress
│   ├── attestations.model.ts      # Agent attestations
│   └── user-affinity.model.ts     # Engagement tracking
│
├── Discovery & Trust
│   ├── exploration.model.ts       # Graph traversal queries
│   ├── knowledge-map.model.ts     # Polymorphic maps
│   ├── trust-badge.model.ts       # UI-ready trust indicators
│   ├── search.model.ts            # Search with scoring/facets
│   └── elohim-agent.model.ts      # Constitutional AI agents
│
├── Session & Access
│   ├── session-user.model.ts      # SessionUser, SessionStats, upgrade prompts
│   └── content-access.model.ts    # Access levels, gated content requirements
│
├── Profile (Imago Dei)
│   └── profile.model.ts           # HumanProfile, JourneyStats, TimelineEvent
│
└── REA Economic Coordination (hREA/Unyt)
    ├── rea-bridge.model.ts            # ValueFlows ontology (Agent, Resource, Event, Process)
    ├── contributor-presence.model.ts  # Stewardship lifecycle for absent contributors
    └── economic-event.model.ts        # Immutable value flow audit trail
```

---

## File Inventory

| File | Status | Description |
|------|--------|-------------|
| `content-node.model.ts` | ✅ Complete | ContentNode, ContentType, ContentReach, ContentFormat |
| `content-attestation.model.ts` | ✅ Complete | Attestation types, scopes, revocation |
| `learning-path.model.ts` | ✅ Complete | LearningPath, PathStep, PathStepView |
| `path-extension.model.ts` | ✅ Complete | Learner path mutations |
| `agent.model.ts` | ✅ Complete | Agent, AgentProgress, MasteryLevel |
| `attestations.model.ts` | ✅ Complete | Agent attestation system |
| `user-affinity.model.ts` | ✅ Complete | UserAffinity tracking |
| `exploration.model.ts` | ✅ Complete | ExplorationQuery, ExplorationResult |
| `knowledge-map.model.ts` | ✅ Complete | Polymorphic maps (domain, person, collective) |
| `trust-badge.model.ts` | ✅ Complete | TrustBadge, TrustIndicator, unified display |
| `search.model.ts` | ✅ Complete | SearchQuery, SearchResult, SearchFacets |
| `elohim-agent.model.ts` | ✅ Complete | Constitutional AI guardian models |
| `session-user.model.ts` | ✅ Complete | SessionUser, SessionStats, upgrade prompts, migration |
| `content-access.model.ts` | ✅ Complete | AccessLevel, ContentAccessMetadata, gated content |
| `profile.model.ts` | ✅ Complete | HumanProfile, JourneyStats, TimelineEvent (Imago Dei aligned) |
| `rea-bridge.model.ts` | ✅ Complete | ValueFlows ontology: REAAgent, EconomicResource, Process, Commitment |
| `contributor-presence.model.ts` | ✅ Complete | ContributorPresence, PresenceStewardship, recognition accumulation |
| `economic-event.model.ts` | ✅ Complete | EconomicEvent, LamadEventType, event streams |
| `index.ts` | ✅ Complete | Barrel exports for all models |

---

## Key Interfaces

### ContentNode (Territory)
```typescript
interface ContentNode {
  id: string;
  title: string;
  description: string;
  contentType: ContentType;
  contentFormat: ContentFormat;
  content: string | object;
  tags: string[];
  reach: ContentReach;
  relatedNodeIds: string[];
  metadata: ContentMetadata;
}

type ContentReach = 'private' | 'invited' | 'local' | 'community' | 'federated' | 'commons';
```

### LearningPath (Journey)
```typescript
interface LearningPath {
  id: string;
  title: string;
  description: string;
  purpose: string;
  steps: PathStep[];
  difficulty: 'beginner' | 'intermediate' | 'advanced';
  estimatedDuration: string;
  visibility: 'public' | 'organization' | 'private';
}

interface PathStep {
  order: number;
  resourceId: string;  // Links to ContentNode.id
  stepTitle: string;
  stepNarrative: string;  // WHY this content matters HERE
  learningObjectives: string[];
  completionCriteria: string[];
  optional: boolean;
}
```

### TrustIndicator (Unified Badges + Flags)
```typescript
interface TrustIndicator {
  id: string;
  polarity: 'positive' | 'negative';  // Badge vs Flag
  priority: number;
  icon: string;
  label: string;
  description: string;
  color: BadgeColor;
  verified: boolean;
  source: IndicatorSource;
}
```

### SearchQuery
```typescript
interface SearchQuery {
  text: string;
  contentTypes?: ContentType[];
  reachLevels?: ContentReach[];
  trustLevels?: TrustLevel[];
  tags?: string[];
  sortBy?: 'relevance' | 'title' | 'trustScore' | 'newest';
  page?: number;
  pageSize?: number;
}
```

### SessionUser (NEW)
```typescript
interface SessionUser {
  sessionId: string;        // Generated UUID
  displayName: string;
  isAnonymous: true;
  accessLevel: 'visitor';   // Always visitor for session humans
  createdAt: string;
  lastActiveAt: string;
  stats: SessionStats;
}

interface SessionStats {
  nodesViewed: number;
  pathsStarted: number;
  pathsCompleted: number;
  stepsCompleted: number;
  totalSessionTime: number;
  sessionCount: number;
}
```

### ContentAccessMetadata
```typescript
type AccessLevel = 'visitor' | 'member' | 'attested';
type ContentAccessLevel = 'open' | 'gated' | 'protected';

interface ContentAccessMetadata {
  accessLevel: ContentAccessLevel;
  requirements?: ContentAccessRequirement;
  restrictionReason?: string;
  unlockPath?: string;  // Path that grants access
}

interface ContentAccessRequirement {
  minLevel: AccessLevel;
  requiredAttestations?: string[];
  requiredPaths?: string[];
  requiresGovernanceApproval?: boolean;
}
```

### HumanProfile (Imago Dei Aligned)
```typescript
interface HumanProfile {
  id: string;
  displayName: string;
  isSessionBased: boolean;
  journeyStartedAt: string;
  lastActiveAt: string;
  journeyStats: JourneyStats;
  currentFocus: CurrentFocus[];
  developedCapabilities: DevelopedCapability[];
}

interface JourneyStats {
  territoryExplored: number;    // Content viewed (breadth)
  journeysStarted: number;       // Paths begun
  journeysCompleted: number;     // Paths completed (milestones)
  stepsCompleted: number;        // Total steps across all paths
  meaningfulEncounters: number;  // High-affinity content
  timeInvested: number;          // Learning time (ms)
  sessionsCount: number;         // Return visits
}

interface TimelineEvent {
  id: string;
  type: TimelineEventType;
  timestamp: string;
  title: string;
  description?: string;
  resourceId?: string;
  resourceType?: 'path' | 'content' | 'attestation';
  significance: 'milestone' | 'progress' | 'activity';
}
```

### ContributorPresence (REA Stewardship)
```typescript
// Lifecycle: unclaimed → stewarded → claimed
type PresenceState = 'unclaimed' | 'stewarded' | 'claimed';

interface ContributorPresence extends REAAgent {
  type: 'contributor-presence';
  presenceState: PresenceState;
  externalIdentifiers: ExternalIdentifier[];  // For claim verification
  establishingContentIds: string[];           // Content that created this presence
  accumulatedRecognition: AccumulatedRecognition;
  stewardship?: PresenceStewardship;          // If stewarded by Elohim
  claim?: PresenceClaim;                      // If claimed by contributor
  invitations: PresenceInvitation[];          // Outreach history
}

interface AccumulatedRecognition {
  affinityTotal: number;        // Total affinity points
  uniqueEngagers: number;       // Unique humans who engaged
  citationCount: number;        // Citations in other content
  endorsements: PresenceEndorsement[];
  recognitionScore: number;     // Computed composite score
  byContent: ContentRecognition[];
}
```

### EconomicEvent (ValueFlows)
```typescript
// The core building block of REA accounting
interface EconomicEvent {
  id: string;
  action: REAAction;            // 'use' | 'cite' | 'produce' | 'transfer' | etc.
  provider: string;             // Who gave
  receiver: string;             // Who received
  resourceConformsTo?: string;  // ResourceSpecification.id
  resourceQuantity?: Measure;
  hasPointInTime: string;
  inputOf?: string;             // Process.id (if part of process)
  fulfills?: string;            // Commitment.id (if fulfilling promise)
  state: EventState;
}

// Lamad-specific event types
type LamadEventType =
  | 'content-view'      // Human viewed content
  | 'affinity-mark'     // Human marked affinity (recognition flows)
  | 'citation'          // Content cited another
  | 'path-complete'     // Human completed path
  | 'presence-claim'    // Contributor claimed their presence
  | 'recognition-transfer'; // Recognition transferred on claim
```

### REA Resource Types
```typescript
type ResourceClassification =
  | 'content'      // Learning content
  | 'attention'    // Human engagement
  | 'recognition'  // Value acknowledgment
  | 'credential'   // Earned attestations
  | 'curation'     // Curated paths
  | 'synthesis'    // AI-generated maps
  | 'stewardship'  // Care of presences
  | 'currency';    // Mutual credit (Unyt)
```

---

## Import Pattern

```typescript
// All models are exported from index.ts
import {
  // Territory
  ContentNode, ContentType, ContentReach,

  // Journey
  LearningPath, PathStep, PathStepView,

  // Traveler
  Agent, AgentProgress,

  // Trust
  TrustBadge, TrustIndicator,
  SearchQuery, SearchResult, SearchFacets,

  // Session & Access
  SessionUser, SessionStats, HolochainUpgradePrompt,
  AccessLevel, ContentAccessLevel, ContentAccessMetadata,

  // Profile
  HumanProfile, JourneyStats, TimelineEvent,

  // REA Economic Coordination
  REAAction, REAAgent, EconomicResource, Process, Commitment,
  ResourceSpecification, ResourceClassification,
  ContributorPresence, PresenceState, AccumulatedRecognition,
  PresenceStewardship, PresenceInvitation,
  EconomicEvent, LamadEventType, EventQuery,
  createEventFromRequest
} from '../models';
```

---

## Notes for Agents

**The model layer is complete.** These interfaces are stable and used by all services.

### Terminology
- Use "human" not "user" in all new code
- Use "journey" not "consumption"
- Use "meaningful encounters" not "views"
- Use "presence" for external contributors not yet in the network
- Use "recognition" not "reward" for value flows

### Do NOT:
- Add new model files without explicit instruction
- Modify existing interfaces (they're used across the codebase)
- Remove exports from index.ts

### REA/ValueFlows Integration
The REA models establish contracts for hREA/Unyt integration:
- `rea-bridge.model.ts` - Core ValueFlows types compatible with hREA GraphQL API
- `contributor-presence.model.ts` - Stewardship lifecycle (unclaimed → stewarded → claimed)
- `economic-event.model.ts` - Immutable event stream for value flow audit trail

Key concepts from the Economic Epic:
- "REA doesn't ask 'how much money?' It asks 'what actually happened?'"
- Events are recorded from "independent view" - as transactions between parties
- Recognition flows to contributor presences even before they join the network

### Holochain Migration Notes
When migrating to Holochain:
- `id` fields become action hashes
- `SessionUser` migrates via `prepareMigration()` to agent's source chain
- Progress moves to agent's private source chain
- Attestations become DHT entries with crypto verification
- Reach enforcement moves from client to conductor validation
- `EconomicEvent` maps directly to hREA zome entries
- `ContributorPresence` uses hREA Agent with custom extension fields
- Recognition transfer on claim uses countersigned events
