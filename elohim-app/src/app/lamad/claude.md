# Lamad: Agentic Implementation Guide

*This document provides layered guidance for AI agents implementing Lamad. Less capable agents should focus on narrowly scoped tasks in subdirectory `claude.md` files. More capable agents can use this root document for architectural context.*

## Document Hierarchy

```
lamad/
├── claude.md           <-- YOU ARE HERE (Architecture & Coordination)
├── LAMAD_API_SPECIFICATION_v1.0.md  <-- AUTHORITATIVE SPEC
├── IMPLEMENTATION_PLAN.md  <-- STATUS TRACKER (v5.0)
├── Imago Dei Framework.md  <-- Human-centered identity principles
├── models/claude.md    <-- Data model interfaces
├── services/claude.md  <-- Service layer implementation
├── components/claude.md <-- UI components
└── renderers/claude.md <-- Rendering system
```

**Authority Chain:**
1. `LAMAD_API_SPECIFICATION_v1.0.md` - The definitive source of truth for all interfaces
2. `IMPLEMENTATION_PLAN.md` - Status tracker and phase summaries
3. Directory-level `claude.md` files - Scoped guidance for specific modules

---

## Quick Reference: Current State

### MVP Status (as of 2025-11-27)

**The MVP service layer is feature-complete. REA economic interface contracts established. Four-dimensional relational map architecture complete. Governance deliberation and feedback profile systems defined.** All 21 phases implemented, 14 services active.

**Active Services:**
| Service | Purpose |
|---------|---------|
| DataLoaderService | JSON file loading (Holochain adapter point) |
| PathService | Path & step navigation |
| ContentService | Content access with reach checking, back-links |
| AgentService | Agent profiles and attestations |
| AffinityTrackingService | Engagement tracking (session-integrated) |
| ExplorationService | Graph traversal, pathfinding, rate limiting |
| KnowledgeMapService | Four-dimensional maps (domain, self, person, collective) |
| PathExtensionService | Learner-owned path mutations |
| TrustBadgeService | UI-ready trust badge computation |
| SearchService | Enhanced search with scoring and facets |
| SessionUserService | Temporary session identity for MVP |
| ProfileService | Human-centered profile (Imago Dei aligned) |
| ElohimAgentService | Autonomous constitutional guardians |
| GovernanceService | Constitutional moderation, deliberation, feedback |

**Active Components:**
- LamadHome (path-centric with tabs)
- LamadLayout (session human UI, upgrade prompts)
- PathOverview, PathNavigator (journey navigation)
- ContentViewer (with back-links)
- GraphExplorer (D3.js visualization)
- LearnerDashboard

**Active Renderers:**
- MarkdownRenderer, GherkinRenderer, IframeRenderer, QuizRenderer

---

## The Six-Layer Architecture

### Layer 1: Territory (Content Nodes)
The immutable knowledge graph. Content exists independently of how it's navigated.

**Key Interface:** `ContentNode`
```typescript
interface ContentNode {
  id: string;
  title: string;
  description: string;
  contentType: 'epic' | 'concept' | 'simulation' | 'video' | 'assessment' | ...;
  contentFormat: 'markdown' | 'html5-app' | 'video-embed' | 'quiz-json' | ...;
  content: string | object;  // Payload depends on format
  tags: string[];
  relatedNodeIds: string[];
  metadata: ContentMetadata;
}
```

### Layer 2: Journey (Learning Paths)
Curated sequences that add narrative meaning to Territory resources.

**Key Interfaces:** `LearningPath`, `PathStep`
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
  attestationsGranted?: string[];
}

interface PathStep {
  order: number;
  resourceId: string;  // Links to ContentNode.id
  stepTitle: string;
  stepNarrative: string;  // WHY this content matters HERE
  learningObjectives: string[];
  completionCriteria: string[];
  optional: boolean;
  attestationRequired?: string;
  attestationGranted?: string;
}
```

### Layer 3: Traveler (Humans & Progress)
Sovereign humans whose progress shapes their experience.

**Key Interfaces:** `Agent`, `AgentProgress`, `SessionUser`
```typescript
interface Agent {
  id: string;
  displayName: string;
  type: 'human' | 'organization' | 'ai-agent';
  visibility: 'public' | 'connections' | 'private';
}

interface AgentProgress {
  agentId: string;
  pathId: string;
  currentStepIndex: number;
  completedStepIndices: number[];
  stepAffinity: Record<number, number>;  // 0.0 to 1.0
  stepNotes: Record<number, string>;
  attestationsEarned: string[];
}

interface SessionUser {
  sessionId: string;       // Generated UUID
  displayName: string;
  isAnonymous: true;
  accessLevel: 'visitor';  // Always visitor for session humans
  stats: SessionStats;
}
```

### Layer 4: Profile (Human Identity - Imago Dei)
Human-centered identity view aligned with Imago Dei framework principles.

**Key Interfaces:** `HumanProfile`, `JourneyStats`
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
```

### Layer 5: Economic (REA Coordination)
ValueFlows-based economic coordination for recognition, stewardship, and value flows.

**Key Interfaces:** `ContributorPresence`, `EconomicEvent`
```typescript
// Lifecycle: unclaimed → stewarded → claimed
type PresenceState = 'unclaimed' | 'stewarded' | 'claimed';

interface ContributorPresence extends REAAgent {
  type: 'contributor-presence';
  presenceState: PresenceState;
  accumulatedRecognition: AccumulatedRecognition;
  stewardship?: PresenceStewardship;
  claim?: PresenceClaim;
}

interface EconomicEvent {
  id: string;
  action: REAAction;  // 'use' | 'cite' | 'produce' | 'transfer' | 'accept'
  provider: string;   // Who gave
  receiver: string;   // Who received
  resourceQuantity?: Measure;
  hasPointInTime: string;
}

type LamadEventType =
  | 'content-view'      // Human viewed content
  | 'affinity-mark'     // Human marked affinity (recognition flows)
  | 'presence-claim'    // Contributor claimed their presence
  | 'recognition-transfer'; // Recognition transferred on claim
```

### Layer 6: Governance (Constitutional Moderation)
The protocol's immune system - deliberation, feedback profiles, and constitutional accountability.

**Key Interfaces:** `GovernanceState`, `FeedbackProfile`, `DeliberationProposal`
```typescript
// Governance state for any entity
interface GovernanceState {
  entityId: string;
  entityType: GovernableEntityType;
  status: GovernanceStatus;
  labels: GovernanceLabel[];
  challenges: Challenge[];
  appealHistory: Appeal[];
}

// Feedback Profile - virality as privilege, NOT entitlement
interface FeedbackProfile {
  id: string;
  permittedMechanisms: FeedbackMechanism[];  // NO "LIKES"
  emotionalConstraints?: EmotionalReactionConstraints;
  currentLevel: FeedbackProfileLevel;
  profileEvolution: ProfileEvolution[];
}

// Key principle: Facebook-style "likes" are fundamentally pernicious
// Replaced with: approval-vote (up/down), emotional reactions with context
type FeedbackMechanism =
  | 'approval-vote'         // Up/down (replaces "like")
  | 'emotional-reaction'    // "I feel ___ about this"
  | 'graduated-usefulness'  // Loomio-style scales
  | 'discussion-only'       // No amplification
  | 'view-only';            // No engagement permitted
```

---

## Service Layer Summary

| Service | Purpose | Depends On |
|---------|---------|------------|
| `DataLoaderService` | JSON file fetching (Holochain adapter) | HttpClient |
| `PathService` | Path & step navigation | DataLoader |
| `ContentService` | Territory access, reach checking, back-links | DataLoader, AgentService |
| `AgentService` | Auth, progress, attestations | DataLoader, SessionUserService |
| `AffinityTrackingService` | Engagement tracking | SessionUserService, localStorage |
| `ExplorationService` | BFS traversal, pathfinding | DataLoader, AgentService |
| `KnowledgeMapService` | Four-dimensional maps (domain, self, person, collective) | DataLoader |
| `PathExtensionService` | Learner path mutations | DataLoader |
| `TrustBadgeService` | UI-ready trust badge computation | DataLoader, AgentService |
| `SearchService` | Enhanced search with scoring, facets | DataLoader, TrustBadgeService |
| `SessionUserService` | Temporary session identity, activity tracking | localStorage |
| `ProfileService` | Human-centered profile (Imago Dei aligned) | SessionUser, Path, Affinity, Agent |
| `GovernanceService` | Constitutional moderation, deliberation, feedback | DataLoader, AgentService |
| `RendererRegistryService` | Content format → Component mapping | None |

---

## Route Structure

```typescript
const LAMAD_ROUTES: Routes = [
  { path: '', component: LamadHomeComponent },
  { path: 'path/:pathId', component: PathOverviewComponent },
  { path: 'path/:pathId/step/:stepIndex', component: PathNavigatorComponent },
  { path: 'resource/:resourceId', component: ContentViewerComponent },
  { path: 'me', component: LearnerDashboardComponent },
  { path: 'explore', component: MeaningMapComponent },  // Deprecated, keep for research
];
```

---

## Session Human Architecture (MVP)

The MVP uses a **session human** model that provides:
1. **Zero-friction entry** - Anyone can explore immediately without signup
2. **Progress tracking** - Session state stored in localStorage
3. **Holochain upgrade path** - Session data migrates when human installs app

### Access Levels
| Level | Identity | Can Access |
|-------|----------|------------|
| `visitor` | Session human (localStorage) | Open content only |
| `member` | Holochain AgentPubKey | Gated content |
| `attested` | Member + attestations | Protected content |

### Content Access Tiers
| Tier | Description | Example |
|------|-------------|---------|
| `open` | Freely explorable by anyone | General learning content |
| `gated` | Requires Holochain identity | Community discussions |
| `protected` | Requires attestations + path completion | CSAM handling training |

### Session-to-Network Flow
```
1. Human explores freely as visitor (session identity)
2. Meaningful moments trigger upgrade prompts
3. Human installs Holochain app
4. prepareMigration() packages session data
5. Session data imports to agent's source chain
6. clearAfterMigration() removes localStorage
7. Human continues with full network identity
```

### Upgrade Prompts
Session humans see contextual prompts encouraging Holochain installation:
- `first-affinity`: When human marks first content as resonant
- `path-completed`: After completing a learning path
- `notes-saved`: When saving personal notes
- `network-feature`: When trying a gated feature

---

## Relational Maps Architecture

Learning is fundamentally about building relationship. Lamad supports four types of knowledge maps:

| Map Type | Question | Inspiration |
|----------|----------|-------------|
| **Domain** | What do I know? | Khan Academy's "World of Math" |
| **Self** | Who am I? | Delphic maxim "γνῶθι σεαυτόν" (know thyself) |
| **Person** | Who do I know? | Gottman's Love Maps research |
| **Collective** | What do we know? | Organizational knowledge management |

### Self-Knowledge Maps

The self-knowledge map is unique: the mapper and subject are the same person (reflexive). It integrates with the Imago Dei framework:

```typescript
interface SelfKnowledgeMap extends KnowledgeMap {
  mapType: 'self';
  imagoDeiDimensions: ImagoDeiDimension[];  // core, experience, gifts, synthesis
  valuesHierarchy: PersonalValue[];          // What matters most, in priority order
  lifeChapters: LifeChapter[];               // Narrative structure of one's journey
  discoveredGifts: DiscoveredGift[];         // Strengths uncovered through self-examination
  shadowAreas: ShadowArea[];                 // Growth areas and blind spots
  vocation?: VocationalClarity;              // Calling, purpose, gift-to-need alignment
  domainReflections: DomainReflection[];     // How domain learning reveals the self
}
```

Theological grounding: "Love your neighbor as yourself" (Mark 12:31) implies you must first know yourself. Self-knowledge is prerequisite to loving others well.

---

## Human Profile (Imago Dei Framework)

The ProfileService provides a human-centered view of identity aligned with the Imago Dei framework:

| Imago Dei Module | ProfileService Method | Purpose |
|------------------|----------------------|---------|
| `imagodei-core` | `getProfile()` | Stable identity center |
| `imagodei-experience` | `getTimeline()`, `getCurrentFocus()` | Learning and transformation |
| `imagodei-gifts` | `getDevelopedCapabilities()` | Skills and attestations earned |
| `imagodei-synthesis` | `getTopEngagedContent()`, `getAllNotes()` | Growth and meaning-making |

**Key Design Principles:**
- Use "human" not "user" throughout codebase
- Growth-oriented metrics (not consumption metrics)
- Narrative view of journey (not activity logs)
- Honor dignity and agency

---

## Technical Conventions

### Date Fields
All timestamp fields use **ISO 8601 string format** (not Date objects):
```typescript
createdAt: string;  // "2025-11-27T14:30:00Z" - CORRECT
createdAt: Date;    // Do not use
```

### Attestation Model Distinction
Three distinct attestation models exist - do NOT confuse them:

| Model | Purpose |
|-------|---------|
| **Agent Attestations** (`attestations.model.ts`) | Credentials earned BY humans/agents |
| **Content Attestations** (`content-attestation.model.ts`) | Trust credentials granted TO content |
| **Content Access** (`content-access.model.ts`) | Access tier requirements (visitor/member/attested) |

- `AttestationAccessRequirement`: What attestations unlock which content
- `ContentAccessRequirement`: What access level is required for content

---

## Critical Implementation Constraints

### 1. Lazy Loading is NON-NEGOTIABLE
- Never load "all paths" or "all content"
- Content fetched by ID, one node at a time
- Path metadata loads without step content
- Step content loads only when navigating to that step

### 2. Fog of War
- Humans can only access: completed steps, current step, or one step ahead
- Attestation gates can lock content until prerequisites met
- This is pedagogical wisdom, not artificial scarcity

### 3. IDs are Opaque Strings
- Never parse or depend on ID format
- In prototype: human-readable slugs (`epic-social-medium`)
- In production: Holochain hashes (`uhCkk...`)

### 4. Territory vs Journey Separation
- Content nodes are generic, reusable across paths
- Path steps add context: "Why does THIS matter HERE?"
- Same video can appear in marriage path and workplace path with different narratives

### 5. Human-Centered Terminology
- Use "human" not "user"
- Use "journey" not "consumption"
- Use "meaningful encounters" not "views"

---

## Completed Implementation Phases

| Phase | Status | Description |
|-------|--------|-------------|
| 1: Data Foundation | ✅ | JSON schemas, 3,097 content nodes |
| 2: Service Layer | ✅ | DataLoader, Path, Content, Agent services |
| 3: Rendering Engine | ✅ | Registry pattern, 4 renderers |
| 4: Journey UI | ✅ | PathNavigator, PathOverview, path-centric home |
| 5: Graph Explorer | ✅ | D3.js visualization |
| 6: Integration | ✅ | Build passes, routes working |
| 7: Mastery System | 🔮 Post-MVP | Spaced repetition, concept quizzes |
| 8: Graph API | ✅ | ExplorationService with BFS, pathfinding |
| 9: Knowledge Maps | ✅ | KnowledgeMapService, PathExtensionService |
| 10: Bidirectional Trust | ✅ | ContentAttestation, reach-based access |
| 11: Trust Badges | ✅ | TrustBadgeService, unified indicators |
| 12: Enhanced Search | ✅ | SearchService with scoring, facets |
| 13: Session Human | ✅ | Zero-friction identity, upgrade path |
| 14: Human Profile | ✅ | Imago Dei-aligned profile service |
| 15: REA Economic Models | ✅ | ValueFlows types, ContributorPresence, EconomicEvent |
| 16: Relational Maps | ✅ | Self-knowledge maps, four-map architecture |
| 17: Psychometric Assessments | ✅ | Validated instruments, pattern detection |
| 18: Governance & Feedback | ✅ | Constitutional moderation, challenges, appeals |
| 19: Governance Deliberation | ✅ | Loomio/Polis/Wikipedia-inspired deliberation |
| 20: Feedback Profile | ✅ | Virality as privilege, emotional reaction constraints |
| 21: Cohesion Review | ✅ | Model standardization, documentation alignment |

See `IMPLEMENTATION_PLAN.md` for detailed phase summaries.

---

## Remaining Work

### Priority 1: Profile UI
- [ ] Profile page component (render HumanProfile data)
- [ ] Journey timeline visualization
- [ ] Resume point card ("Continue where you left off")
- [ ] Paths overview with progress bars

### Priority 2: Content Access UI
- [ ] Gated content lock indicator
- [ ] Access denial modal with unlock actions
- [ ] "Join Network" flow (Holochain install placeholder)

### Priority 3: Polish & Bugs
- [ ] CSS budget cleanup (content-viewer, lamad-layout exceed 6kb)
- [ ] Error handling refinement
- [ ] Loading state animations

### Priority 4: Testing
- [ ] Cypress e2e tests for path navigation
- [ ] Unit tests for services (ProfileService, SessionUserService)
- [ ] Accessibility audit

---

## For Agents

**The MVP service layer is feature-complete. REA interface contracts established.** Future work should focus on:
1. UI implementation using existing services (Profile, Content Access)
2. Testing and polish (not new features)
3. Holochain/hREA migration (when ready)
4. Contributor Presence service implementation (uses REA models)
5. Economic event tracking service (uses REA models)

### Do NOT:
- Add new services without explicit instruction
- Modify working interfaces
- Load "all content" or "all paths" anywhere
- Use "user" terminology - use "human" instead
- Use "creator" terminology - use "contributor" instead

---

## Terminology Quick Reference

| Term | Meaning |
|------|---------|
| **Territory** | The content graph (ContentNodes) |
| **Journey** | A curated learning path |
| **Traveler** | A human/agent in the system |
| **Human** | A person using the platform (never "user") |
| **Contributor** | Someone who creates content (never "creator") |
| **Elohim** | Active intelligence agents (services, AI) |
| **Lamad** | The static structure (graph, paths) |
| **Affinity** | How deeply you've engaged with content (0.0-1.0) |
| **Attestation** | Earned credential/badge |
| **Fog of War** | Content visibility earned through progression |
| **Imago Dei** | Human-centered identity framework |
| **Self Map** | Reflexive knowledge map ("know thyself") |
| **Person Map** | Knowledge about another person (Gottman love maps) |
| **Domain Map** | Knowledge about a subject area |
| **Collective Map** | Shared knowledge within a community |
| **Session Human** | Temporary visitor identity (localStorage) |
| **Presence** | Placeholder identity for external contributors |
| **Recognition** | Value acknowledgment flowing to contributors |
| **REA** | Resources, Events, Agents - accounting ontology |
| **ValueFlows** | REA implementation standard (hREA uses this) |
