# Bloom's Taxonomy Content Mastery System

## Vision: Beyond Khan Academy

Khan Academy's mastery model stops at "apply" and calls it "mastered." This is consumption proficiency, not true mastery. The Elohim Protocol recognizes that **true mastery is participatory** - it lives in the upper levels of Bloom's Taxonomy and requires active contribution, not passive consumption.

> "Virality is a privilege, not an entitlement."

The same applies to mastery: **advanced participation is a privilege earned through demonstrated competence.**

---

## Bloom's Taxonomy as Mastery Progression

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        BLOOM'S MASTERY LEVELS                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  CREATE      │ ████████████████████████████████████████ │ Active/Hot       │
│  ────────────┼──────────────────────────────────────────┼─────────────────  │
│  EVALUATE    │ ██████████████████████████████████████   │ Active/Hot       │
│  ────────────┼──────────────────────────────────────────┼─────────────────  │
│  ANALYZE     │ ████████████████████████████████████     │ Active/Warm      │
│  ════════════╪══════════════════════════════════════════╪═════════════════  │
│              │        ▲ ATTESTATION GATE ▲              │                   │
│  ════════════╪══════════════════════════════════════════╪═════════════════  │
│  APPLY       │ ██████████████████████████████           │ Passive/Earned   │
│  ────────────┼──────────────────────────────────────────┼─────────────────  │
│  UNDERSTAND  │ ████████████████████████                 │ Passive          │
│  ────────────┼──────────────────────────────────────────┼─────────────────  │
│  REMEMBER    │ ██████████████████                       │ Passive          │
│  ────────────┼──────────────────────────────────────────┼─────────────────  │
│  SEEN        │ ████████████                             │ Passive          │
│  ────────────┼──────────────────────────────────────────┼─────────────────  │
│  NOT_STARTED │ ░░░░░░░░░░                               │ Passive          │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Level Definitions

| Level | Code | How Achieved | Engagement Type |
|-------|------|--------------|-----------------|
| `not_started` | 0 | - | None |
| `seen` | 1 | Viewed content | Passive |
| `remember` | 2 | Basic recall quiz (identify, list, name) | Passive |
| `understand` | 3 | Comprehension assessment (explain, summarize) | Passive |
| `apply` | 4 | Mastery quiz - mixed content application | **Attestation Gate** |
| `analyze` | 5 | Contribute analysis, commentary, connections | Active |
| `evaluate` | 6 | Peer review, critique, quality assessment | Active |
| `create` | 7 | Original contribution, derivative works | Active |

---

## The Attestation Gate

The `apply` level is the **attestation gate** - the threshold between consumption and contribution.

### Below the Gate (Passive Learning)
- You can **practice** anything
- Progress is tracked privately
- No privileges beyond viewing
- This is where most Khan Academy users stay

### At the Gate (Apply Level)
Achieving `apply` requires:
1. Completing mastery quizzes that mix content from multiple related nodes
2. Demonstrating ability to use knowledge in novel contexts
3. Meeting minimum accuracy thresholds

**Path completion = 100% at Apply level.** This keeps things simple and fun:
- Completing a path means you've demonstrated you can *apply* the knowledge
- That's a real achievement - you're not just consuming, you can use it
- The upper levels (analyze, evaluate, create) are *bonus* - earned through participation, not required for "completion"

This mirrors Khan Academy's model (path mastery = apply) while adding the upper Bloom levels as participation incentives rather than completion requirements.

### Above the Gate (Active Participation)
Crossing the gate unlocks:

| Level | Privileges Unlocked |
|-------|---------------------|
| `analyze` | Comment, discuss, suggest connections |
| `evaluate` | Peer review others' contributions, rate quality |
| `create` | Author derivative content, propose edits, contribute to paths |

**Key Insight**: Those who have already achieved mastery can help determine what privileges are granted at each level. This is "consent of the governed" applied to learning communities.

---

## Freshness and Decay Model

### The Problem with Static Mastery

Khan Academy's mastery is a snapshot. Return after years and your "100% mastery" may show 49-70% because:
- New content was added to paths
- Standards/curricula evolved
- Your skills naturally decayed

### Elohim's Graph-Relative Freshness

```typescript
interface MasteryFreshness {
  // Personal decay - time since you engaged
  lastEngagement: string;              // ISO 8601
  personalDecayFactor: number;         // 0.0-1.0, computed from time

  // Graph evolution - has the content/path changed?
  contentVersionAtMastery: string;     // Content hash when mastery achieved
  currentContentVersion: string;       // Current content hash
  graphEvolutionFactor: number;        // 0.0-1.0, % of content still valid

  // Composite freshness
  effectiveFreshness: number;          // Combined score 0.0-1.0

  // Refresh eligibility
  needsRefresh: boolean;               // True if below threshold
  refreshType: 'review' | 'retest' | 'relearn';
}
```

### Decay Vectors

1. **Personal Decay**: Time-based, models human memory curves
   ```
   personalDecayFactor = exp(-λ * daysSinceEngagement)
   ```
   Where λ varies by mastery level (higher levels decay slower if actively used)

2. **Graph Evolution**: Content changes propagate automatically
   - Content node updated → version changes
   - Path adds new steps → completion % drops
   - Related nodes evolve → connections may need refresh

3. **Activity Relative**: Engagement with graph-adjacent content counts
   - Working on related paths maintains freshness
   - Contributing to connected nodes keeps mastery warm
   - The graph structure itself creates natural refresh triggers

### The Right to Be Forgotten

Not everything needs to be remembered or renewed:

```typescript
interface ContentLifecycle {
  status: ContentLifecycleStatus;
  createdAt: string;
  publishedAt?: string;
  lastRefreshedAt?: string;

  // Expiration (optional)
  expiresAt?: string;                  // Content has a natural end date
  deprecatedAt?: string;               // Superseded but still accessible
  archivedAt?: string;                 // Read-only historical record
  forgottenAt?: string;                // Permanently removed

  // Refresh policy
  refreshPolicy: RefreshPolicy;

  // Who decides?
  lifecycleGovernance: 'author' | 'steward' | 'community' | 'elohim';
}

type ContentLifecycleStatus =
  | 'draft'           // Author working, not visible
  | 'published'       // Active, fresh content
  | 'stale'           // Needs review/refresh
  | 'deprecated'      // Superseded, redirects to successor
  | 'archived'        // Historical record, read-only
  | 'forgotten';      // Removed from graph

interface RefreshPolicy {
  // How often should this content be reviewed?
  refreshInterval?: string;            // ISO 8601 duration, e.g., "P6M"

  // Who can refresh?
  refreshPermissions: 'author' | 'contributors' | 'masters' | 'steward';

  // What happens if not refreshed?
  staleAction: 'flag' | 'deprecate' | 'archive' | 'forget';

  // Grace period before action
  gracePeriod?: string;                // ISO 8601 duration
}
```

### Impact on Mastery

When content lifecycle changes:

1. **Content Refreshed**: Masters get notified, can re-engage to maintain freshness
2. **Content Deprecated**: Mastery preserved but flagged as "legacy"
3. **Content Archived**: Mastery becomes historical record
4. **Content Forgotten**: Mastery gracefully removed (no attestation revocation)

---

## Data Model Updates

### New: BloomMasteryLevel Enum

**File**: `elohim-app/src/app/lamad/models/agent.model.ts`

Replace the current `MasteryLevel` type:

```typescript
/**
 * BloomMasteryLevel - Content mastery based on Bloom's Taxonomy.
 *
 * Progression from passive consumption to active contribution.
 * The 'apply' level is the attestation gate - crossing it unlocks
 * participation privileges in the content's governance.
 *
 * Reference: Bloom's Revised Taxonomy (Anderson & Krathwohl, 2001)
 */
export type BloomMasteryLevel =
  | 'not_started'    // 0 - No engagement
  | 'seen'           // 1 - Content viewed
  | 'remember'       // 2 - Basic recall demonstrated
  | 'understand'     // 3 - Comprehension demonstrated
  | 'apply'          // 4 - Application in novel contexts (ATTESTATION GATE)
  | 'analyze'        // 5 - Can break down, connect, contribute analysis
  | 'evaluate'       // 6 - Can assess, critique, peer review
  | 'create';        // 7 - Can author, derive, synthesize

/**
 * Numeric value for BloomMasteryLevel for comparison and persistence.
 */
export const BLOOM_LEVEL_VALUES: Record<BloomMasteryLevel, number> = {
  'not_started': 0,
  'seen': 1,
  'remember': 2,
  'understand': 3,
  'apply': 4,
  'analyze': 5,
  'evaluate': 6,
  'create': 7,
};

/**
 * The level at which participation privileges unlock.
 */
export const ATTESTATION_GATE_LEVEL: BloomMasteryLevel = 'apply';

/**
 * @deprecated Use BloomMasteryLevel instead
 */
export type MasteryLevel = BloomMasteryLevel;
```

### New: ContentMastery Interface

**File**: `elohim-app/src/app/lamad/models/content-mastery.model.ts`

```typescript
import { BloomMasteryLevel } from './agent.model';

/**
 * ContentMastery - A human's mastery state for a specific content node.
 *
 * This is the per-content-node tracking that powers:
 * - Khan Academy-style cross-path completion views
 * - Bloom's taxonomy progression
 * - Freshness/decay tracking
 * - Participation privilege gating
 *
 * Stored on human's private source chain (never DHT without consent).
 */
export interface ContentMastery {
  /** Content node ID */
  contentId: string;

  /** Human/agent ID */
  humanId: string;

  /** Current Bloom's level achieved */
  level: BloomMasteryLevel;

  /** When this level was achieved */
  levelAchievedAt: string;             // ISO 8601

  /** History of level progression */
  levelHistory: LevelProgressionEvent[];

  // =========================================================================
  // Freshness Tracking
  // =========================================================================

  /** Last engagement with this content */
  lastEngagementAt: string;            // ISO 8601

  /** Type of last engagement */
  lastEngagementType: EngagementType;

  /** Content version when mastery was achieved */
  contentVersionAtMastery: string;

  /** Computed freshness (0.0-1.0) */
  freshness: number;

  /** Does this need refresh? */
  needsRefresh: boolean;

  /** Suggested refresh action */
  refreshType?: 'review' | 'retest' | 'relearn';

  // =========================================================================
  // Assessment Evidence
  // =========================================================================

  /** Quiz/assessment results that contributed to this level */
  assessmentEvidence: AssessmentEvidence[];

  /** Peer evaluations received (for analyze+ levels) */
  peerEvaluations?: PeerEvaluation[];

  /** Contributions made (for create level) */
  contributions?: ContentContribution[];

  // =========================================================================
  // Participation Privileges
  // =========================================================================

  /** Privileges currently granted based on level */
  privileges: ContentPrivilege[];

  /** Privileges earned but suspended (e.g., due to freshness decay) */
  suspendedPrivileges?: ContentPrivilege[];
}

/**
 * EngagementType - How the human engaged with content.
 */
export type EngagementType =
  | 'view'              // Passive viewing
  | 'quiz'              // Took assessment
  | 'practice'          // Practice exercise
  | 'comment'           // Added comment/discussion
  | 'review'            // Peer reviewed content
  | 'contribute'        // Made contribution
  | 'path_step'         // Encountered in learning path
  | 'refresh';          // Explicit refresh engagement

/**
 * LevelProgressionEvent - Record of mastery level change.
 */
export interface LevelProgressionEvent {
  fromLevel: BloomMasteryLevel;
  toLevel: BloomMasteryLevel;
  timestamp: string;
  trigger: 'assessment' | 'engagement' | 'contribution' | 'decay' | 'refresh';
  evidence?: string;                   // Assessment ID, contribution ID, etc.
}

/**
 * AssessmentEvidence - Quiz/test result contributing to mastery.
 */
export interface AssessmentEvidence {
  assessmentId: string;
  assessmentType: 'recall' | 'comprehension' | 'application' | 'analysis';
  score: number;                       // 0.0-1.0
  passedAt: string;
  contributeToLevel: BloomMasteryLevel;
}

/**
 * PeerEvaluation - Evaluation from another human at evaluate+ level.
 */
export interface PeerEvaluation {
  evaluatorId: string;
  evaluatorLevel: BloomMasteryLevel;
  rating: number;                      // 0.0-1.0
  feedback?: string;
  timestamp: string;
}

/**
 * ContentContribution - Original contribution at create level.
 */
export interface ContentContribution {
  contributionType: 'comment' | 'edit' | 'derivative' | 'original';
  contributionId: string;
  createdAt: string;
  status: 'pending' | 'accepted' | 'rejected';
  peerReviewScore?: number;
}

/**
 * ContentPrivilege - What the human can do with this content.
 */
export interface ContentPrivilege {
  privilege: PrivilegeType;
  grantedAt: string;
  grantedByLevel: BloomMasteryLevel;
  active: boolean;
  suspendedReason?: 'freshness_decay' | 'moderation' | 'content_change';
}

export type PrivilegeType =
  | 'view'              // Basic viewing (always granted)
  | 'practice'          // Take practice quizzes (always granted)
  | 'comment'           // Add comments/discussion (analyze+)
  | 'suggest_edit'      // Propose content edits (analyze+)
  | 'peer_review'       // Review others' contributions (evaluate+)
  | 'rate_quality'      // Rate content quality (evaluate+)
  | 'create_derivative' // Create derivative content (create)
  | 'contribute_path'   // Add content to paths (create)
  | 'govern';           // Participate in content governance (create)
```

### New: ContentLifecycle Interface

**File**: `elohim-app/src/app/lamad/models/content-lifecycle.model.ts`

```typescript
/**
 * ContentLifecycle - Lifecycle state and refresh policy for content.
 *
 * Implements "the right to be forgotten" - not everything needs to
 * be remembered or renewed indefinitely.
 */
export interface ContentLifecycle {
  /** Content node ID */
  contentId: string;

  /** Current lifecycle status */
  status: ContentLifecycleStatus;

  /** When content was first created */
  createdAt: string;

  /** When content was published (became visible) */
  publishedAt?: string;

  /** When content was last refreshed/updated */
  lastRefreshedAt?: string;

  /** Who last refreshed it */
  lastRefreshedBy?: string;

  // =========================================================================
  // Expiration & Archival
  // =========================================================================

  /** Content has a natural expiration date */
  expiresAt?: string;

  /** Content superseded by newer content */
  deprecatedAt?: string;
  supersededBy?: string;               // ContentNode ID of replacement

  /** Content archived (read-only historical record) */
  archivedAt?: string;

  /** Content permanently removed */
  forgottenAt?: string;
  forgetReason?: 'author_request' | 'governance' | 'expiration' | 'policy';

  // =========================================================================
  // Refresh Policy
  // =========================================================================

  /** How often should this content be reviewed? */
  refreshInterval?: string;            // ISO 8601 duration, e.g., "P6M"

  /** When is the next refresh due? */
  nextRefreshDue?: string;

  /** Who can refresh? */
  refreshPermissions: RefreshPermission;

  /** What happens if not refreshed on time? */
  staleAction: StaleAction;

  /** Grace period before stale action */
  gracePeriod?: string;                // ISO 8601 duration

  // =========================================================================
  // Version Tracking
  // =========================================================================

  /** Current content version (hash or semver) */
  currentVersion: string;

  /** Version history for freshness comparisons */
  versionHistory: VersionEvent[];

  // =========================================================================
  // Governance
  // =========================================================================

  /** Who decides lifecycle transitions? */
  lifecycleGovernance: LifecycleGovernance;
}

export type ContentLifecycleStatus =
  | 'draft'           // Author working, not visible
  | 'published'       // Active, fresh content
  | 'stale'           // Needs review/refresh
  | 'deprecated'      // Superseded, redirects to successor
  | 'archived'        // Historical record, read-only
  | 'forgotten';      // Removed from graph

export type RefreshPermission =
  | 'author'          // Only original author
  | 'contributors'    // Author + contributors
  | 'masters'         // Anyone at create level
  | 'steward'         // Content steward only
  | 'community';      // Community governance

export type StaleAction =
  | 'flag'            // Just mark as stale, keep accessible
  | 'deprecate'       // Mark deprecated, suggest alternatives
  | 'archive'         // Move to archive, read-only
  | 'forget';         // Remove from graph

export type LifecycleGovernance =
  | 'author'          // Author controls lifecycle
  | 'steward'         // Designated steward controls
  | 'community'       // Community vote decides
  | 'elohim';         // AI-assisted governance

export interface VersionEvent {
  version: string;
  timestamp: string;
  changedBy: string;
  changeType: 'minor' | 'major' | 'refresh' | 'deprecation';
  summary?: string;
}
```

### Updated: ContentNode Interface

**File**: `elohim-app/src/app/lamad/models/content-node.model.ts`

Add to the existing ContentNode interface:

```typescript
// In ContentNode interface, add:

/**
 * Lifecycle state for this content.
 * Controls freshness, refresh policies, and archival.
 * See content-lifecycle.model.ts for full interface.
 */
lifecycle?: ContentLifecycle;

/**
 * Version identifier for mastery freshness comparisons.
 * Updated on any substantive content change.
 * Format: ISO 8601 timestamp or content hash.
 */
contentVersion?: string;
```

### Updated: AgentProgress Interface

**File**: `elohim-app/src/app/lamad/models/agent.model.ts`

The existing `AgentProgress` tracks path navigation. We add mastery tracking:

```typescript
// In AgentProgress interface, add:

/**
 * Cross-path mastery state for content encountered in this path.
 * Keys are contentId, values are ContentMastery snapshots.
 *
 * Full mastery state lives in dedicated storage (ContentMasteryService),
 * but path progress includes relevant snapshots for:
 * - Display during path navigation
 * - Fog-of-war calculations
 * - Path completion requirements
 */
contentMastery?: Record<string, ContentMasterySnapshot>;

// Add new interface:
export interface ContentMasterySnapshot {
  level: BloomMasteryLevel;
  freshness: number;
  levelAchievedAt: string;
  hasGatePrivileges: boolean;         // Is at or above apply level?
}
```

---

## Service Updates Required

### New: ContentMasteryService

**File**: `elohim-app/src/app/lamad/services/content-mastery.service.ts`

```typescript
/**
 * ContentMasteryService - Manages Bloom's taxonomy mastery for content.
 *
 * Responsibilities:
 * - Track mastery level per content node per human
 * - Compute freshness based on engagement and graph evolution
 * - Grant/revoke privileges at attestation gate
 * - Handle mastery refresh and decay
 *
 * Storage: localStorage initially, Holochain source chain in production.
 */
@Injectable({ providedIn: 'root' })
export class ContentMasteryService {
  // Key methods to implement:

  getMastery(contentId: string): Observable<ContentMastery | null>;
  setMasteryLevel(contentId: string, level: BloomMasteryLevel, evidence?: AssessmentEvidence): void;
  recordEngagement(contentId: string, type: EngagementType): void;
  computeFreshness(contentId: string): number;
  getPrivileges(contentId: string): ContentPrivilege[];
  hasPrivilege(contentId: string, privilege: PrivilegeType): boolean;
  refreshMastery(contentId: string): Observable<ContentMastery>;
  getMasteryStats(): Observable<MasteryStats>;
}
```

### New: ContentLifecycleService

**File**: `elohim-app/src/app/lamad/services/content-lifecycle.service.ts`

```typescript
/**
 * ContentLifecycleService - Manages content lifecycle and refresh policies.
 *
 * Responsibilities:
 * - Track content lifecycle state
 * - Trigger refresh notifications
 * - Handle deprecation, archival, and forgetting
 * - Propagate lifecycle changes to mastery freshness
 */
@Injectable({ providedIn: 'root' })
export class ContentLifecycleService {
  // Key methods to implement:

  getLifecycle(contentId: string): Observable<ContentLifecycle | null>;
  updateStatus(contentId: string, status: ContentLifecycleStatus, reason?: string): void;
  refreshContent(contentId: string, newVersion: string): void;
  deprecateContent(contentId: string, successorId?: string): void;
  archiveContent(contentId: string): void;
  forgetContent(contentId: string, reason: string): void;
  getStaleContent(): Observable<ContentNode[]>;
  getDueForRefresh(): Observable<ContentNode[]>;
}
```

### Updated: AffinityTrackingService

The current service tracks 0.0-1.0 affinity. Integration with mastery:

```typescript
// Add method to AffinityTrackingService:

/**
 * Get effective engagement score combining affinity and mastery level.
 * Affinity is engagement depth, mastery is competency.
 */
getEffectiveEngagement(contentId: string): {
  affinity: number;
  masteryLevel: BloomMasteryLevel;
  freshness: number;
  composite: number;                   // Weighted combination
};
```

### Updated: PathService

Integration for path-based mastery tracking:

```typescript
// Add to PathService:

/**
 * Get mastery overview for all content in a path.
 */
getPathMasteryOverview(pathId: string): Observable<{
  contentMastery: ContentMasterySnapshot[];
  averageLevel: BloomMasteryLevel;
  gatePassedPercentage: number;
  freshPercentage: number;
}>;
```

### Updated: KnowledgeMapService

Integration for domain-level mastery:

```typescript
// Update KnowledgeMapService:

/**
 * Use BloomMasteryLevel instead of old MasteryLevel.
 */
updateMastery(mapId: string, contentNodeId: string, level: BloomMasteryLevel): void;
getMasteryLevel(mapId: string, contentNodeId: string): BloomMasteryLevel;
```

---

## Component Updates Required

### High Priority (Direct Mastery Display)

| Component | Location | Changes |
|-----------|----------|---------|
| `PathOverviewComponent` | `components/path-overview/` | Display Bloom levels instead of simple completion, show gate passage %, add freshness indicators |
| `ContentViewerComponent` | `components/content-viewer/` | Show current mastery level, privilege indicators, refresh prompts |
| `AffinityCircleComponent` | `components/affinity-circle/` | Add mastery level ring, freshness indicator, gate passage visual |
| `LearnerDashboardComponent` | `components/learner-dashboard/` | Overview of mastery distribution, stale content alerts |

### Medium Priority (Mastery-Aware Navigation)

| Component | Location | Changes |
|-----------|----------|---------|
| `PathNavigatorComponent` | `components/path-navigator/` | Show step mastery levels, gate requirements |
| `GraphExplorerComponent` | `components/graph-explorer/` | Color-code nodes by mastery, show privilege availability |
| `LamadHomeComponent` | `components/lamad-home/` | Mastery summary widgets, refresh recommendations |

### Lower Priority (Future Enhancement)

| Component | Location | Changes |
|-----------|----------|---------|
| `LamadLayoutComponent` | `components/lamad-layout/` | Global mastery stats in header/sidebar |
| New: `MasteryQuizComponent` | `components/mastery-quiz/` | Bloom-level appropriate assessment UI |
| New: `FreshnessAlertComponent` | `components/freshness-alert/` | Stale mastery notifications |

---

## Migration Strategy

### Phase 1: Model Introduction
1. Add new model files without removing old ones
2. Add `BloomMasteryLevel` alongside deprecated `MasteryLevel`
3. Add `ContentMastery` interface
4. Add `ContentLifecycle` interface

### Phase 2: Service Implementation
1. Implement `ContentMasteryService`
2. Implement `ContentLifecycleService`
3. Update `AffinityTrackingService` integration
4. Add migration for existing progress data

### Phase 3: Component Updates
1. Update `AffinityCircleComponent` with mastery ring
2. Update `PathOverviewComponent` with Bloom levels
3. Update `ContentViewerComponent` with mastery display
4. Add privilege indicators to content interactions

### Phase 4: Assessment Integration
1. Create Bloom-level assessments (recall, comprehend, apply)
2. Integrate with existing `AssessmentService`
3. Add gate-crossing assessment flows

### Phase 5: Active Participation
1. Implement analyze-level features (comments, connections)
2. Implement evaluate-level features (peer review)
3. Implement create-level features (contributions)

---

## Key Design Decisions

### 1. Why Bloom's Taxonomy?

- **Established pedagogy**: Well-researched, widely accepted framework
- **Natural progression**: Each level builds on previous
- **Clear gate**: "Apply" is a natural threshold between consumption and production
- **Active engagement**: Upper levels require contribution, not just consumption

### 2. Why Graph-Relative Freshness?

- **Natural decay**: Paths evolving naturally triggers mastery review
- **Reduced complexity**: No artificial timers for most decay
- **Holistic tracking**: Graph structure captures knowledge relationships
- **Emergent behavior**: Freshness emerges from graph activity

### 3. Why "Right to Be Forgotten"?

- **Graceful deprecation**: Not all knowledge is eternal
- **Reduced noise**: Stale content naturally exits the graph
- **Human respect**: Author/community control over content lifecycle
- **Storage efficiency**: Don't maintain mastery for forgotten content

### 4. Why Privilege Gating?

- **Quality filter**: Those who demonstrate competence can contribute
- **Moderation efficiency**: Pre-filtered by mastery, less spam/noise
- **Incentive alignment**: Earning privileges motivates mastery
- **Community trust**: Masters help govern their domains

---

## Expertise Discovery: "Who's the Best at X?"

The mastery graph naturally reveals expertise. This is **incredibly valuable** information that emerges from the system without explicit reputation scores.

### The Graph Reveals Experts

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     EXPERTISE DISCOVERY QUERIES                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  "Who knows the most about climate economics?"                              │
│  ────────────────────────────────────────────                               │
│  Query: Find humans with CREATE level on climate-economics-* nodes         │
│  Rank by: freshness × contribution count × peer review score               │
│                                                                             │
│  "Who should review my Holochain architecture?"                             │
│  ───────────────────────────────────────────────                            │
│  Query: Find humans at EVALUATE+ on holochain-* AND architecture-*         │
│  Rank by: intersection of domains × active contribution recency            │
│                                                                             │
│  "Who are the rising experts in cooperative governance?"                    │
│  ─────────────────────────────────────────────────────────                  │
│  Query: Find humans with steep mastery velocity in governance-* nodes      │
│  Rank by: level progression speed × engagement frequency                   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Expertise Signals from Mastery

| Signal | Source | Weight |
|--------|--------|--------|
| **Mastery Level** | ContentMastery.level | How deep is their knowledge? |
| **Freshness** | ContentMastery.freshness | Is it current or stale? |
| **Contribution Count** | ContentMastery.contributions | Have they actively contributed? |
| **Peer Review Score** | ContentMastery.peerEvaluations | What do other experts think? |
| **Domain Breadth** | Count of related nodes at create level | How comprehensive? |
| **Mastery Velocity** | LevelProgressionEvent history | How quickly did they advance? |
| **Teaching Activity** | Evaluations given to others | Do they help others learn? |

### New: ExpertiseDiscoveryService

**File**: `elohim-app/src/app/lamad/services/expertise-discovery.service.ts`

```typescript
/**
 * ExpertiseDiscoveryService - Find experts in any domain.
 *
 * The graph reveals who knows what. This service queries the mastery
 * graph to surface experts without explicit reputation systems.
 *
 * Use cases:
 * - "Who should review my contribution?"
 * - "Who can mentor me in X?"
 * - "Who are the active stewards of this content?"
 * - "Who's on the shortlist for X?"
 */
@Injectable({ providedIn: 'root' })
export class ExpertiseDiscoveryService {

  /**
   * Find humans with expertise in a content domain.
   */
  findExperts(query: ExpertiseQuery): Observable<ExpertCandidate[]>;

  /**
   * Find potential reviewers for a contribution.
   */
  findReviewers(contentId: string, excludeAuthor?: string): Observable<ExpertCandidate[]>;

  /**
   * Find potential mentors for a learner's current frontier.
   */
  findMentors(learnerId: string, contentIds: string[]): Observable<ExpertCandidate[]>;

  /**
   * Get expertise leaderboard for a content domain.
   */
  getLeaderboard(contentIds: string[], options?: LeaderboardOptions): Observable<ExpertLeaderboard>;

  /**
   * Get rising experts (steep mastery velocity).
   */
  getRisingExperts(contentIds: string[], timeWindow?: string): Observable<ExpertCandidate[]>;
}

export interface ExpertiseQuery {
  /** Content node IDs or tag patterns to search */
  domains: string[];

  /** Minimum mastery level required */
  minLevel?: BloomMasteryLevel;

  /** Minimum freshness required */
  minFreshness?: number;

  /** Require active contributions? */
  requireContributions?: boolean;

  /** Limit results */
  limit?: number;

  /** Exclude specific humans (e.g., self) */
  exclude?: string[];
}

export interface ExpertCandidate {
  humanId: string;
  displayName: string;

  /** Composite expertise score (0.0-1.0) */
  expertiseScore: number;

  /** Breakdown of score components */
  scoreBreakdown: {
    masteryLevel: number;
    freshness: number;
    contributionActivity: number;
    peerRecognition: number;
    domainBreadth: number;
  };

  /** Content nodes they're expert in */
  expertiseNodes: Array<{
    contentId: string;
    level: BloomMasteryLevel;
    freshness: number;
    contributionCount: number;
  }>;

  /** How available are they? (if they've indicated) */
  availability?: 'available' | 'limited' | 'unavailable';

  /** Last activity in this domain */
  lastDomainActivity: string;
}

export interface ExpertLeaderboard {
  domain: string;
  generatedAt: string;

  /** Top experts by expertise score */
  topExperts: ExpertCandidate[];

  /** Most active contributors (create level activity) */
  mostActive: ExpertCandidate[];

  /** Rising experts (recent rapid advancement) */
  rising: ExpertCandidate[];

  /** Most helpful (peer review activity) */
  mostHelpful: ExpertCandidate[];
}

export interface LeaderboardOptions {
  /** How many in each category */
  limit?: number;

  /** Time window for "active" and "rising" */
  timeWindow?: string;              // ISO 8601 duration

  /** Include only fresh mastery? */
  freshOnly?: boolean;
}
```

### Privacy-Respecting Discovery

The expertise graph is powerful but must respect privacy:

```typescript
export interface ExpertiseVisibility {
  /** Can others see my expertise? */
  discoverability: 'public' | 'network' | 'private';

  /** Which domains can I be found in? */
  visibleDomains: string[] | 'all' | 'none';

  /** Am I available for mentorship? */
  mentorshipAvailable: boolean;

  /** Am I available for reviews? */
  reviewAvailable: boolean;

  /** Can I appear on leaderboards? */
  leaderboardOptIn: boolean;
}
```

### The Value Proposition

1. **For Learners**: Find mentors who actually know what they're teaching
2. **For Contributors**: Find reviewers who can give meaningful feedback
3. **For Stewards**: Identify who should participate in content governance
4. **For the Community**: Surface hidden experts, reduce knowledge silos
5. **For Elohim**: Route questions to humans who can actually answer them

This isn't a gamified reputation score - it's **emergent expertise visibility** from actual learning and contribution activity.

---

## Open Questions

1. **Gate Flexibility**: Should different content types have different gate levels? (e.g., safety content might gate at "understand")

2. **Decay Curves**: What's the right decay rate? Should it vary by domain? (Math skills decay differently than historical facts)

3. **Privilege Customization**: How much control should content authors have over privilege definitions?

4. **Cross-Platform Mastery**: If someone demonstrates mastery elsewhere (Khan, Coursera), can they claim credit?

5. **Assessment Rigor**: How do we prevent gaming the system with minimal assessments?

---

## References

- Anderson, L.W. & Krathwohl, D.R. (2001). A Taxonomy for Learning, Teaching, and Assessing
- Khan Academy Mastery Learning: https://www.khanacademy.org/about/blog/post/mastery-learning
- Ebbinghaus Forgetting Curve: For decay modeling
- W3C Decentralized Identifiers: https://www.w3.org/TR/did-core/
