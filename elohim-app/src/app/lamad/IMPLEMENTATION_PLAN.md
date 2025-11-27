# Lamad MVP Implementation Plan v5.0

This document tracks the implementation status of the Lamad learning platform MVP.

**Goal:** A functional learning platform where humans can navigate curated paths, explore knowledge graphs, and access content with reach-based access control. The platform honors human dignity (Imago Dei principles), provides a frictionless path to Holochain network membership, and establishes interfaces for REA-based economic coordination.

**Related Documents:**
- [API Specification](./LAMAD_API_SPECIFICATION_v1.0.md) - Route patterns, data models, service contracts
- [Imago Dei Framework](./Imago%20Dei_%20A%20Framework%20for%20Human-Centered%20Digital%20Identity.md) - Human-centered identity principles
- [Economic Epic](../../docs/economic_coordination/epic.md) - REA at family scale, contributor stewardship

---

## Implementation Status

| Phase | Status | Description |
|-------|--------|-------------|
| Phase 1: Data Foundation | ✅ Complete | JSON schemas, data generation, 3,096 content nodes |
| Phase 2: Service Layer | ✅ Complete | DataLoaderService, PathService, AgentService |
| Phase 3: Rendering Engine | ✅ Complete | Dynamic renderer registry, markdown, gherkin, iframe, quiz |
| Phase 4: Journey UI | ✅ Complete | PathNavigator, PathOverview, path-centric home |
| Phase 5: Graph Explorer | ✅ Complete | D3.js visualization, hierarchical zoom |
| Phase 6: Integration | ✅ Complete | Build passes, routes working |
| Phase 7: Mastery System | 🔮 Post-MVP | Spaced repetition, concept quizzes |
| Phase 8: Graph API | ✅ Complete | ExplorationService with BFS, pathfinding, rate limiting |
| Phase 9: Knowledge Maps | ✅ Complete | KnowledgeMapService, PathExtensionService, mock data |
| Phase 10: Bidirectional Trust | ✅ Complete | Models, access control, "appears in paths" back-links |
| Phase 11: Trust Badges | ✅ Complete | TrustBadgeService, unified indicators, 24 mock attestations |
| Phase 12: Enhanced Search | ✅ Complete | SearchService with scoring, facets, suggestions |
| Phase 13: Session Human | ✅ Complete | Zero-friction identity, activity tracking, Holochain upgrade path |
| Phase 14: Human Profile | ✅ Complete | Imago Dei-aligned profile service, journey narrative |
| Phase 15: REA Economic Models | ✅ Complete | ValueFlows types, ContributorPresence, EconomicEvent |

### Quick Start

```bash
# Generate data
python scripts/generate_lamad_data.py

# Serve app
ng serve

# Key routes
/lamad                              → Path-centric home
/lamad/path/elohim-protocol         → Path overview
/lamad/path/elohim-protocol/step/0  → Step navigation
/lamad/explore                      → Graph explorer
/lamad/resource/:id                 → Direct content access (with back-links)
```

---

## Architectural Principles

### Lazy Loading is Non-Negotiable
Content is fetched by ID, one node at a time. Path metadata loads without loading step content. No service method returns "all nodes."

### Data Structures Mirror Holochain Entries
Every JSON structure translates directly to a Holochain entry type. IDs are opaque strings (become action hashes), no circular references.

### Path-Centric Navigation
Path-centric navigation (step progression with narrative) is primary. Graph exploration is secondary discovery.

### Territory and Journey Separation
Content nodes (Territory) exist independently of paths (Journey). Same content can appear in multiple paths.

### Human-Centered Terminology
We use "human" not "user" throughout the codebase. This aligns with Elohim Protocol principles and the Imago Dei framework - recognizing the dignity and agency of every person who engages with the platform.

---

## Completed Phases Summary

### Phases 1-5: Walking Skeleton ✅
- **Models:** LearningPath, ContentNode, PathStep, ContentReach levels
- **Services:** DataLoaderService, PathService, AffinityTrackingService
- **Renderers:** MarkdownRenderer, GherkinRenderer, IframeRenderer, QuizRenderer
- **Components:** PathNavigator, PathOverview, ContentViewer, GraphExplorer, LamadHome
- **Data:** 3,096 content nodes, graph relationships, path definitions

### Phase 8: Exploration Service ✅
- **ExplorationService** with BFS traversal, Dijkstra pathfinding, semantic pathfinding
- Attestation-gated depth limits (0-1 authenticated, 2 graph-researcher, 3 advanced-researcher)
- Rate limiting with hourly quotas per tier
- Query cost estimation

### Phase 9: Knowledge Maps & Path Extensions ✅
- **KnowledgeMapService** for polymorphic maps (domain, person, collective)
- **PathExtensionService** for learner-owned path mutations (insertions, annotations, reorderings)
- Mock data in `assets/lamad-data/knowledge-maps/` and `extensions/`
- DataLoaderService updated with map/extension loading methods

### Phase 10: Bidirectional Trust ✅
- **ContentAttestation** model with types, scopes, revocation
- **ContentReach** levels: private → invited → local → community → federated → commons
- **ContentService** with:
  - `getContentWithAccessCheck()` - reach-based access control
  - `getContainingPaths()` - Wikipedia-style "appears in paths" back-links
- **ContentViewer** updated with back-links UI

### Phase 11: Trust Badges ✅
- **TrustBadge** model with BadgeDisplay, BadgeWarning, BadgeAction
- **TrustIndicator** unified model (badges + flags with polarity and priority)
- **TrustIndicatorSet** for complete trust state
- **TrustBadgeService** computing UI-ready badge data from attestations
- Mock data: 24 attestations covering all trust levels and states

### Phase 12: Enhanced Search ✅
- **SearchQuery** with text, filters, sorting, pagination
- **SearchResult** with relevanceScore (0-100), matchedFields, highlights
- **SearchFacets** for filter UI (counts by type, reach, trust, tags)
- **SearchService** with:
  - `search()` - full search with scoring and facets
  - `suggest()` - autocomplete suggestions
  - `getTagCloud()` - tag counts for discovery

### Phase 13: Session Human ✅
Zero-friction entry with Holochain upgrade path.

- **SessionUser model** (`session-user.model.ts`):
  - `SessionUser` - temporary identity with sessionId, displayName, stats
  - `SessionStats` - engagement metrics (nodesViewed, pathsStarted, pathsCompleted)
  - `SessionActivity` - activity history for timeline
  - `SessionPathProgress` - path progress with notes
  - `HolochainUpgradePrompt` - contextual upgrade suggestions
  - `UpgradeTrigger` - meaningful moments (first-affinity, path-completed, etc.)
  - `SessionMigration` - data package for Holochain transition

- **Content Access model** (`content-access.model.ts`):
  - `AccessLevel`: visitor → member → attested
  - `ContentAccessLevel`: open → gated → protected
  - `ContentAccessMetadata` - per-content access requirements
  - `AccessCheckResult` - detailed access denial with actions
  - `ACCESS_PRESETS` - common configurations

- **SessionUserService** (`session-user.service.ts`):
  - Session lifecycle: create, restore, touch
  - Activity tracking: views, affinity, path progress
  - Content access control: `checkContentAccess()`, `canAccessContent()`
  - Upgrade prompts: contextual triggers, dismiss, active list
  - Migration: `prepareMigration()`, `clearAfterMigration()`

- **Service Integration**:
  - `AffinityTrackingService` - session-scoped storage keys
  - `AgentService` - delegates identity to session, access checks
  - `LamadLayoutComponent` - session human UI, upgrade banner, modal

### Phase 14: Human Profile (Imago Dei) ✅
Human-centered identity view aligned with Imago Dei framework.

- **Profile model** (`profile.model.ts`):
  - `HumanProfile` - core identity + growth indicators (imagodei-core)
  - `JourneyStats` - growth-oriented metrics (not consumption)
  - `CurrentFocus` - active learning paths (imagodei-experience)
  - `DevelopedCapability` - attestations earned (imagodei-gifts)
  - `TimelineEvent` - significant moments (imagodei-experience)
  - `ContentEngagement` - meaningful encounters (imagodei-synthesis)
  - `NoteWithContext` - personal reflections (imagodei-synthesis)
  - `ResumePoint` - smart continuation suggestion
  - `PathWithProgress`, `PathsOverview` - organized path views

- **ProfileService** (`profile.service.ts`):
  - `getProfile()` - complete identity view
  - `getProfileSummary()` - compact header/card display
  - `getJourneyStats()` - aggregated growth metrics
  - `getCurrentFocus()` - paths in progress
  - `getDevelopedCapabilities()` - earned attestations
  - `getTimeline()` - transformation events
  - `getTopEngagedContent()` - high-affinity content
  - `getAllNotes()` - personal reflections with context
  - `getResumePoint()` - smart "continue here" suggestion
  - `getPathsOverview()` - in-progress, completed, suggested

### Phase 15: REA Economic Models ✅
Interface contracts for hREA/Unyt economic coordination.

- **REA Bridge model** (`rea-bridge.model.ts`):
  - `REAAction` - ValueFlows action vocabulary (use, cite, produce, transfer, accept)
  - `ResourceSpecification` - Resource type definitions (content, attention, recognition, credential)
  - `EconomicResource` - Current state derived from event history
  - `REAAgent` - Extended agent type including `contributor-presence`
  - `Process` - Learning paths as value-creating transformations
  - `Commitment` / `Intent` - Future flows and promises
  - `Agreement` - Governance documents and contracts
  - `Appreciation` / `Claim` - Recognition flows and entitlements

- **Contributor Presence model** (`contributor-presence.model.ts`):
  - `ContributorPresence` - Placeholder identity for external contributors
  - `PresenceState` - Lifecycle: unclaimed → stewarded → claimed
  - `AccumulatedRecognition` - Value awaiting the contributor
  - `PresenceStewardship` - Elohim care record with activities
  - `PresenceInvitation` - Outreach to contributors
  - `PresenceClaim` - Verification and transfer process
  - `NestedStewardshipOffer` - Claimed contributors stewarding others

- **Economic Event model** (`economic-event.model.ts`):
  - `EconomicEvent` - Core REA event (provider, receiver, action, resource)
  - `LamadEventType` - 25 Lamad-specific event types
  - `createEventFromRequest()` - Helper for typed event creation
  - `EventQuery` - Search/filter events
  - `AgentEventLedger` - Agent-perspective view
  - `NetworkEconomicMetrics` - System-wide transparency metrics
  - `hREAEventAdapter` / `UnytEventAdapter` - Integration interfaces

---

## Current State

**The MVP service layer is feature-complete. REA economic interface contracts established.**

The MVP walking skeleton is complete with:
- Path-based learning journeys with step navigation
- Graph exploration with D3.js visualization
- Dynamic content rendering (markdown, gherkin, iframe, quiz)
- Affinity tracking for engagement
- Reach-based access control (models + service methods)
- "Appears in paths" back-links for content discovery
- Zero-friction session identity with Holochain upgrade path
- Human-centered profile with Imago Dei alignment
- **REA/ValueFlows interface contracts for hREA integration**
- **Contributor Presence stewardship lifecycle models**
- **Economic event stream models for value flow tracking**

### Active Services

| Service | Status | Key Methods |
|---------|--------|-------------|
| DataLoaderService | ✅ | getPath, getContent, getPathIndex, getContentIndex, getKnowledgeMap, getPathExtension, getAttestations |
| PathService | ✅ | getPath, getPathStep, getNextStep, getPreviousStep |
| ContentService | ✅ | getContent, searchContent, getContentWithAccessCheck, getContainingPaths |
| AgentService | ✅ | getCurrentAgentId, getAttestations, getAgent, checkContentAccess |
| AffinityTrackingService | ✅ | getAffinity, setAffinity, trackView, incrementAffinity |
| ExplorationService | ✅ | exploreNeighborhood, findPath, estimateCost, getRateLimitStatus |
| KnowledgeMapService | ✅ | getMapIndex, createDomainMap, createPersonMap, requestElohimSynthesis |
| PathExtensionService | ✅ | createExtension, addInsertion, addAnnotation, applyExtension |
| TrustBadgeService | ✅ | getBadge, getCompactBadge, getIndicators, getIndicatorsForContent |
| SearchService | ✅ | search, suggest, getTagCloud |
| SessionUserService | ✅ | getSession, recordActivity, checkContentAccess, triggerUpgradePrompt, prepareMigration |
| ProfileService | ✅ | getProfile, getTimeline, getCurrentFocus, getPathsOverview, getResumePoint |

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
- [ ] Unit tests for services (ProfileService, SessionUserService priority)
- [ ] Accessibility audit

### Priority 5: UI Polish
- [ ] Mobile responsive improvements
- [ ] Trust badge UI component (render TrustIndicator data)
- [ ] Search facet sidebar and highlight rendering

### Priority 6: Cleanup
- [x] Remove deprecated services (document-graph, learning-path, navigation)
- [x] Update service barrel exports (index.ts)
- [ ] Verify all components use active services only

### Post-MVP: Phase 7 Mastery System
- Spaced repetition scheduling
- Concept quizzes with mastery levels
- Progress degradation over time
- Mastery attestations

### Post-MVP: Holochain Integration
- Replace localStorage with Holochain conductor
- Session migration to agent source chain
- Attestation verification on DHT
- Network membership flow

---

## File Structure

```
src/app/lamad/
├── models/
│   ├── content-node.model.ts       # ContentNode, ContentType, ContentReach
│   ├── learning-path.model.ts      # LearningPath, PathStep, PathStepView
│   ├── agent.model.ts              # Agent, AgentProgress
│   ├── content-attestation.model.ts # Attestation model with types
│   ├── trust-badge.model.ts        # UI-ready trust indicators (badges + flags)
│   ├── search.model.ts             # Enhanced search with scoring and facets
│   ├── knowledge-map.model.ts      # Polymorphic knowledge maps
│   ├── path-extension.model.ts     # Learner path extensions
│   ├── exploration.model.ts        # Graph exploration queries
│   ├── session-user.model.ts       # Session human identity
│   ├── content-access.model.ts     # Tiered access control
│   ├── profile.model.ts            # Human profile (Imago Dei)
│   ├── rea-bridge.model.ts         # ValueFlows ontology (hREA integration)
│   ├── contributor-presence.model.ts # Stewardship lifecycle for contributors
│   └── economic-event.model.ts     # Immutable value flow audit trail
├── services/
│   ├── data-loader.service.ts      # JSON file loading (Holochain adapter point)
│   ├── path.service.ts             # Path navigation
│   ├── content.service.ts          # Direct content access with reach checking
│   ├── agent.service.ts            # Agent profiles and attestations
│   ├── affinity-tracking.service.ts # Engagement tracking
│   ├── exploration.service.ts      # Graph traversal and pathfinding
│   ├── knowledge-map.service.ts    # Map CRUD and synthesis
│   ├── path-extension.service.ts   # Extension mutations
│   ├── trust-badge.service.ts      # UI-ready trust badge computation
│   ├── search.service.ts           # Enhanced search with facets
│   ├── session-user.service.ts     # Session human lifecycle
│   └── profile.service.ts          # Human profile aggregation
├── components/
│   ├── lamad-home/                 # Path-centric home with tabs
│   ├── lamad-layout/               # Layout with session human UI
│   ├── path-overview/              # Path detail with step list
│   ├── path-navigator/             # Step-by-step navigation
│   ├── content-viewer/             # Content display with back-links
│   ├── graph-explorer/             # D3.js knowledge graph
│   └── learner-dashboard/          # Agent progress view
├── renderers/
│   ├── renderer-registry.service.ts # Dynamic renderer selection
│   ├── markdown-renderer/          # Markdown with syntax highlighting
│   ├── gherkin-renderer/           # Gherkin with keyword coloring
│   ├── iframe-renderer/            # External content embedding
│   └── quiz-renderer/              # Interactive assessments
└── LAMAD_API_SPECIFICATION_v1.0.md # Full API contract

src/assets/lamad-data/
├── paths/                          # Learning path definitions
├── content/                        # Content nodes
├── graph/                          # Graph overview and relationships
├── agents/                         # Agent profiles
├── progress/                       # Agent progress (localStorage in prototype)
├── attestations/                   # Content attestations
├── knowledge-maps/                 # Knowledge map definitions
└── extensions/                     # Path extensions
```

---

## Holochain Migration Notes

When transitioning from JSON prototype to Holochain:

1. **DataLoaderService** is the only service that knows about the data source
2. Replace HTTP calls with Holochain conductor calls
3. IDs become action hashes
4. **SessionUserService.prepareMigration()** packages session data for transfer
5. Progress moves from localStorage to agent's private source chain
6. Attestations become DHT entries with cryptographic verification
7. Reach enforcement moves from client to conductor validation
8. Network membership replaces "visitor" access level

### hREA Integration

When integrating with hREA (Holochain REA):

1. **EconomicEvent** maps directly to hREA zome entries via GraphQL API
2. **ContributorPresence** uses hREA Agent with custom extension fields
3. Recognition transfer on claim uses countersigned events
4. **hREAEventAdapter** interface defines the translation layer
5. Resource specifications align with ValueFlows vocabulary

### Unyt Integration

When integrating with Unyt/HoloFuel:

1. **UnytEventAdapter** interface handles mutual credit transactions
2. Currency events use `credit-issue`, `credit-transfer`, `credit-retire` types
3. Credit limits based on contribution metrics (learning, stewardship)
4. Countersigned transactions for value transfers

---

## Identity Architecture

### Access Levels

```
visitor (session human)
    ↓ Install Holochain app
member (network identity)
    ↓ Earn attestations
attested (credentialed member)
```

### Content Access Tiers

| Tier | Who Can Access | Example |
|------|---------------|---------|
| `open` | Anyone (visitor+) | Public documentation |
| `gated` | Members only | Community discussions |
| `protected` | Attested members with path completion | CSAM handling training |

### Session-to-Network Flow

1. Human explores freely as visitor (session identity)
2. Meaningful moments trigger upgrade prompts
3. Human installs Holochain app
4. `prepareMigration()` packages session data
5. Session data imports to agent's source chain
6. `clearAfterMigration()` removes localStorage
7. Human continues with full network identity

---

## Economic Architecture (REA)

### Contributor Presence Lifecycle

```
UNCLAIMED ──────────► STEWARDED ──────────► CLAIMED
   │                      │                     │
   │ Content              │ Elohim begins       │ Contributor verifies
   │ referenced,          │ active care,        │ identity, claims
   │ presence             │ recognition         │ their presence
   │ auto-created         │ accumulates         │
   │                      │                     │
   ▼                      ▼                     ▼
Recognition           Recognition           Recognition flows
accumulates →         accumulates →         directly to
at presence           with invitation       contributor agent
                      queue
```

### Value Flow Events

| Event Type | Action | Resource | Description |
|------------|--------|----------|-------------|
| `content-view` | use | attention | Human viewed content |
| `affinity-mark` | raise | recognition | Human marked affinity |
| `citation` | cite | recognition | Content cited another |
| `path-complete` | produce | credential | Human completed path |
| `presence-claim` | accept | recognition | Contributor claimed presence |
| `recognition-transfer` | transfer | recognition | Recognition transferred on claim |

### Recognition Economics

From the Economic Epic (De Beers' Cybersyn on P2P):
- Recognition flows to contributor presences even before they join
- Elohim steward presences with constitutional accountability
- Claiming transfers accumulated recognition to verified identity
- The attractor: more valuable to participate than to extract

---

*Last updated: 2025-11-27*
