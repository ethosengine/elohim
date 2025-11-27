# Lamad Services: Implementation Guide

*Service layer for the Lamad learning platform. All MVP services complete. REA models ready for integration.*

**Last updated:** 2025-11-27

## Service Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     UI Components                            │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│  PathService  │  ContentService  │  AgentService            │
│  (Journey)    │  (Territory)     │  (Traveler)              │
├───────────────┴──────────────────┴──────────────────────────┤
│  ExplorationService  │  SearchService  │  TrustBadgeService │
│  (Discovery)         │  (Search)       │  (Trust)           │
├───────────────┬──────────────────┬──────────────────────────┤
│  KnowledgeMapService │  PathExtensionService                │
│  (Maps)              │  (Learner Mutations)                 │
├───────────────┴──────────────────┴──────────────────────────┤
│  SessionUserService  │  ProfileService                      │
│  (Session Human)     │  (Imago Dei Profile)                 │
├─────────────────────────────────────────────────────────────┤
│  [Future] ContributorPresenceService  │  EconomicEventService│
│  (Stewardship)                        │  (REA Value Flows)  │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│                   DataLoaderService                          │
│              (JSON now, Holochain/hREA later)                │
└─────────────────────────────────────────────────────────────┘
```

---

## File Inventory

### Core Services (Active)
| Service | Status | Key Methods |
|---------|--------|-------------|
| `data-loader.service.ts` | ✅ | getPath, getContent, getPathIndex, getContentIndex, getAttestations, getGraph |
| `path.service.ts` | ✅ | getPath, getPathStep, getNextStep, getPreviousStep |
| `content.service.ts` | ✅ | getContent, searchContent, getContentWithAccessCheck, getContainingPaths |
| `agent.service.ts` | ✅ | getCurrentAgentId, getAttestations, checkContentAccess, isSessionUser |
| `affinity-tracking.service.ts` | ✅ | getAffinity, setAffinity, trackView, incrementAffinity |
| `exploration.service.ts` | ✅ | exploreNeighborhood, findPath, estimateCost, getRateLimitStatus |
| `knowledge-map.service.ts` | ✅ | getMapIndex, createDomainMap, createPersonMap, requestElohimSynthesis |
| `path-extension.service.ts` | ✅ | createExtension, addInsertion, addAnnotation, applyExtension |
| `trust-badge.service.ts` | ✅ | getBadge, getCompactBadge, getIndicators, getIndicatorsForContent |
| `search.service.ts` | ✅ | search, suggest, getTagCloud |
| `elohim-agent.service.ts` | ✅ | Constitutional AI agent invocation |
| `session-user.service.ts` | ✅ | Session human management, activity tracking, content access |
| `profile.service.ts` | ✅ | Human-centered profile view (Imago Dei aligned) |

### Deleted Services (Cleanup Complete)
Deprecated services have been removed. Components now use active services directly:
- `document-graph.service.ts` → Use DataLoaderService or ContentService
- `learning-path.service.ts` → Use PathService
- `navigation.service.ts` → Use PathService

---

## Key Service Contracts

### DataLoaderService
The ONLY service that knows about the data source. All other services depend on this abstraction.

```typescript
interface DataLoaderService {
  getPath(pathId: string): Observable<LearningPath>;
  getContent(resourceId: string): Observable<ContentNode>;
  getPathIndex(): Observable<PathIndexEntry[]>;
  getContentIndex(): Observable<ContentIndexEntry[]>;
  getAttestations(contentId: string): Observable<ContentAttestation[]>;
  getKnowledgeMap(mapId: string): Observable<KnowledgeMap>;
  getPathExtension(extensionId: string): Observable<PathExtension>;
}
```

### ContentService
Territory access with reach-based access control and back-links.

```typescript
interface ContentService {
  getContent(resourceId: string): Observable<ContentNode>;
  searchContent(query: string): Observable<ContentNode[]>;
  getContentWithAccessCheck(resourceId: string): Observable<ContentAccessResult>;
  getContainingPaths(resourceId: string): Observable<ContainingPath[]>;
  getContainingPathsSummary(resourceId: string): Observable<PathSummary[]>;
}
```

### TrustBadgeService
Computes UI-ready trust indicators from attestations.

```typescript
interface TrustBadgeService {
  getBadge(contentId: string): Observable<TrustBadge>;
  getCompactBadge(contentId: string): Observable<CompactTrustBadge>;
  getIndicators(contentId: string): Observable<TrustIndicatorSet>;
  getIndicatorsForContent(contentIds: string[]): Observable<Map<string, TrustIndicatorSet>>;
  meetsReachRequirement(contentReach: ContentReach, requiredReach: ContentReach): boolean;
}
```

### SearchService
Enhanced search with relevance scoring and faceted filtering.

```typescript
interface SearchService {
  search(query: SearchQuery): Observable<SearchResults>;
  suggest(partialQuery: string, limit?: number): Observable<SearchSuggestions>;
  getTagCloud(): Observable<Array<{ tag: string; count: number }>>;
}
```

### ExplorationService
Graph traversal with attestation-gated depth limits and rate limiting.

```typescript
interface ExplorationService {
  exploreNeighborhood(nodeId: string, depth: number): Observable<ExplorationResult>;
  findPath(startId: string, endId: string): Observable<PathResult>;
  estimateCost(query: ExplorationQuery): number;
  getRateLimitStatus(): RateLimitStatus;
}
```

### SessionUserService
Temporary session identity for MVP with Holochain upgrade path.

```typescript
interface SessionUserService {
  // Session lifecycle
  session$: Observable<SessionUser | null>;
  getSession(): SessionUser | null;
  getSessionId(): string;
  setDisplayName(name: string): void;

  // Activity tracking
  recordContentView(nodeId: string): void;
  recordAffinityChange(nodeId: string, value: number): void;
  recordPathStarted(pathId: string): void;
  recordStepCompleted(pathId: string, stepIndex: number): void;
  recordPathCompleted(pathId: string): void;

  // Content access control
  getAccessLevel(): 'visitor' | 'member' | 'attested';
  checkContentAccess(metadata: ContentAccessMetadata): AccessCheckResult;
  canAccessContent(metadata: ContentAccessMetadata): boolean;

  // Upgrade prompts
  upgradePrompts$: Observable<HolochainUpgradePrompt[]>;
  triggerUpgradePrompt(trigger: UpgradeTrigger): void;
  dismissUpgradePrompt(promptId: string): void;

  // Migration
  prepareMigration(): SessionMigration | null;
  clearAfterMigration(): void;
}
```

**Access Levels:**
- `visitor`: Session human, open content only
- `member`: Holochain identity, gated content
- `attested`: Member with attestations, protected content

**Content Tiers:**
- `open`: Anyone can access
- `gated`: Requires Holochain identity
- `protected`: Requires attestations + path completion

### ProfileService (NEW - Imago Dei Aligned)
Human-centered identity view aligned with Imago Dei framework.

```typescript
interface ProfileService {
  // Core Profile (imagodei-core)
  getProfile(): Observable<HumanProfile>;
  getProfileSummary(): Observable<ProfileSummaryCompact>;

  // Journey Statistics
  getJourneyStats(): Observable<JourneyStats>;

  // Current Focus (imagodei-experience)
  getCurrentFocus(): Observable<CurrentFocus[]>;

  // Developed Capabilities (imagodei-gifts)
  getDevelopedCapabilities(): Observable<DevelopedCapability[]>;

  // Learning Timeline (imagodei-experience)
  getTimeline(limit?: number): Observable<TimelineEvent[]>;

  // Content Engagement (imagodei-synthesis)
  getTopEngagedContent(limit?: number): Observable<ContentEngagement[]>;

  // Notes (imagodei-synthesis)
  getAllNotes(): Observable<NoteWithContext[]>;

  // Resume Point
  getResumePoint(): Observable<ResumePoint | null>;

  // Paths Overview
  getPathsOverview(): Observable<PathsOverview>;
}
```

**Imago Dei Alignment:**
- `imagodei-core`: Stable identity center → `getProfile()`
- `imagodei-experience`: Learning and transformation → `getTimeline()`, `getCurrentFocus()`
- `imagodei-gifts`: Developed capabilities → `getDevelopedCapabilities()`
- `imagodei-synthesis`: Growth and meaning-making → `getTopEngagedContent()`, `getAllNotes()`

---

## Critical Constraints

### 1. Lazy Loading is NON-NEGOTIABLE
**NEVER** create methods that return all paths or all content:
```typescript
// DON'T DO THIS
getAllPaths(): Observable<LearningPath[]>
getAllContent(): Observable<ContentNode[]>
```

### 2. DataLoaderService is the Holochain Adapter Point
Only DataLoaderService knows about the data source. When migrating to Holochain:
- Replace HTTP calls with conductor calls
- IDs become action hashes
- Progress moves to private source chain

### 3. Observable Patterns
```typescript
shareReplay(1)           // Cache single result
switchMap                // Sequential dependent calls
forkJoin                 // Parallel independent calls
catchError(() => of(null)) // Optional data
```

---

## Import Pattern

```typescript
import {
  DataLoaderService,
  PathService,
  ContentService,
  AgentService,
  TrustBadgeService,
  SearchService
} from '../services';
```

---

## Notes for Agents

**The service layer is feature-complete.** These services provide the full API for UI implementation.

### Terminology
- Use "human" not "user" in all new code
- Use "journey" not "consumption"
- Use "meaningful encounters" not "views"

### Do NOT:
- Add new services without explicit instruction
- Modify working service interfaces
- Create methods that load "all" data
- Use deprecated services in new code

### Testing
Services need unit tests. Priority order:
1. ProfileService (aggregation logic)
2. SessionUserService (session lifecycle, migration)
3. SearchService (complex scoring logic)
4. TrustBadgeService (indicator computation)
5. ContentService (access control)
6. ExplorationService (rate limiting)
