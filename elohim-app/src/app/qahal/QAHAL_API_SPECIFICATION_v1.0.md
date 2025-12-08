# Qahal API Specification v1.0
## Interface Control Document for Community Governance Infrastructure

**Document Version:** 1.0
**Last Updated:** 2025-12-08
**Status:** Extracted from LAMAD_API_SPECIFICATION for pillar clarity

---

## Part 0: The Vision

### Overview

Qahal (קהל - Hebrew: "assembly/congregation") is the community governance layer of the Elohim Protocol. While Lamad manages the Territory (content) and Journey (learning paths), Qahal manages the relationships, consent, governance, and feedback that make a community flourish.

This pillar is the protocol's immune system - protecting human flourishing through constitutional accountability while enabling genuine community sensemaking and deliberation.

### Conceptual Inspirations

**Loomio**: Structured proposal types (advice, consent, consensus, sense-check) with graduated feedback scales. Decisions are made transparently with clear phases: draft → discussion → voting → decided → implemented.

**Polis**: AI-powered opinion clustering and bridging statement detection for sensemaking. The system finds consensus across divisions rather than amplifying polarization.

**Wikipedia**: Talk pages, edit history, protection logs for full audit trails. Every change is accountable, every decision is reversible through proper process.

### Qahal's Role in the Protocol

Qahal provides:
1. **Consent-based relationships** - Graduated intimacy levels with explicit consent at transitions
2. **Governance dimension** - Every entity has an auditable governance layer
3. **Feedback profiles** - "Virality is a privilege, not an entitlement"
4. **Constitutional accountability** - Every decision can be challenged with SLA-bound responses
5. **Community deliberation** - Structured proposals, not mob rule

---

## Part 1: Governance Dimension Routes

### 1.1 The Governance Dimension - The Protocol's Immune System

Every entity in the protocol (content, path, human, Elohim, assessment, collective) has a governance dimension - an overlay that tracks its trust state, review history, active challenges, discussions, and audit trail.

**Core Route Pattern:**
```
/{app}/{entityType}:{entityId}/governance/{view}
```

Where `entityType` is one of: `content`, `path`, `assessment`, `contributor`, `human`, `elohim`, `collective`, or `governance-decision`.

Note: Routes use the parent app prefix (e.g., `/lamad/`, `/qahal/`) since governance is attached to entities in those apps.

**Route Patterns:**

```
# Context Menu and Summary
/{app}/{entityType}:{entityId}/governance                # Redirect to summary
/{app}/{entityType}:{entityId}/governance/summary        # Quick overview dashboard

# Full History View (Wikipedia-style)
/{app}/{entityType}:{entityId}/governance/history        # Full audit trail
/{app}/{entityType}:{entityId}/governance/versions       # Version/edit history
/{app}/{entityType}:{entityId}/governance/engagement     # Views, affinity, citations

# Discussions (Talk Pages)
/{app}/{entityType}:{entityId}/governance/discussions              # All discussion threads
/{app}/{entityType}:{entityId}/governance/discussions/new          # Start new thread
/{app}/{entityType}:{entityId}/governance/discussions/thread:{id}  # Specific thread

# Challenges and Appeals
/{app}/{entityType}:{entityId}/governance/challenges        # Active challenges
/{app}/{entityType}:{entityId}/governance/challenges/new    # File a challenge
/{app}/{entityType}:{entityId}/governance/challenges/{id}   # View specific challenge
/{app}/{entityType}:{entityId}/governance/appeals           # Appeals history

# Deliberation (Loomio-style Proposals)
/{app}/{entityType}:{entityId}/governance/proposals         # Active proposals
/{app}/{entityType}:{entityId}/governance/proposals/new     # Create proposal
/{app}/{entityType}:{entityId}/governance/proposals/{id}    # View/vote on proposal

# Sensemaking (Polis-style Clustering)
/{app}/{entityType}:{entityId}/governance/sensemaking       # Opinion clusters visualization
/{app}/{entityType}:{entityId}/governance/sensemaking/contribute  # Add statements

# Feedback
/{app}/{entityType}:{entityId}/governance/feedback/{context}  # Submit graduated feedback
```

**Examples:**
```
/lamad/content:epic-social-medium/governance/summary
/lamad/content:epic-social-medium/governance/discussions
/lamad/content:epic-social-medium/governance/discussions/thread:accuracy-dispute-01
/lamad/content:epic-social-medium/governance/challenges/new
/lamad/content:epic-social-medium/governance/proposals
/lamad/content:epic-social-medium/governance/sensemaking

/lamad/human:agent-xyz/governance/summary
/lamad/elohim:community-guardian/governance/summary
/lamad/elohim:community-guardian/governance/challenges

/lamad/path:love-map-intro/governance/discussions
/lamad/assessment:big-five-personality/governance/summary
```

**Global Governance Routes:**
```
/qahal/governance/precedents                 # Browse constitutional precedents
/qahal/governance/precedents/{id}            # View specific precedent
/qahal/governance/sla-dashboard              # System-wide SLA monitoring
/qahal/governance/elohim-oversight           # Elohim accountability dashboard
```

### 1.2 Behavior Contract: Governance Summary

When accessing the governance summary for any entity, the system returns:
- Current governance status (unreviewed, auto-approved, community-reviewed, verified, disputed, restricted, removed)
- Active labels and their severity
- Count of open discussions, active challenges, pending votes
- Quick actions available based on user's permissions and attestations
- Recent governance events (last 5-10)
- Link to full governance history

The governance dimension is always accessible. Even content that has been "removed" still has its governance dimension available for audit purposes. The content itself may be hidden, but the record of why it was removed, who challenged it, and what appeals were filed remains visible for constitutional accountability.

### 1.3 Context Menu UI Pattern

Every entity in the UI should display a governance context menu (floating action button or menu item) showing:
- Status badge (visual indicator of governance state)
- Alert indicators (if votes pending, challenges filed, etc.)
- Quick actions: Flag, Discuss, Challenge, View History
- Link to full governance view

This ensures that the governance dimension is never more than one click away from any piece of content.

### 1.4 Graduated Feedback Contexts

Different entity types and situations call for different feedback scales:

| Context | Used For | Scale Type |
|---------|----------|------------|
| `accuracy` | Content truth claims | Accurate → False |
| `usefulness` | Learning value | Transformative → Not Useful |
| `proposal-position` | Deliberation voting | Strongly Agree → Block |
| `label-agreement` | Governance label validation | Agree → Disagree |
| `elohim-fairness` | AI oversight | Fair → Unfair |
| `trust-level` | Contributor evaluation | High Trust → No Trust |

Each feedback response can optionally include reasoning, which is required for negative responses and visible to others (creating accountability).

### 1.5 SLA Guarantees

The governance system operates under constitutional SLA guarantees:
- Challenge acknowledgment: 1 hour
- Challenge initial response: 3 days
- Challenge final resolution: 14 days
- Appeal response: 7 days
- Emergency response: 4 hours

SLA violations are themselves governable events that trigger automatic escalation.

---

## Part 2: Feedback Profile Dimension

### 2.1 Feedback Profile - Virality as Privilege

Every piece of content has a FeedbackProfile that governs HOW it can be engaged with. This is orthogonal to ContentReach (which governs WHERE content can go).

**Core Insight:**

> "Virality is a privilege, not an entitlement."

The feedback profile determines what amplification mechanisms are even *possible* for content. A community library announcement may permit low-friction engagement (approval voting), while a mugshot magazine should NEVER have access to viral mechanisms, regardless of its reach level.

### 2.2 Key Principles

1. **NO "LIKES"** - The Facebook-style like is fundamentally pernicious. Replaced with:
   - Approval voting (up/down) as minimum baseline
   - Emotional reactions with context selection
   - All mechanisms are Elohim-gated

2. **EMOTIONAL REACTION CONSTRAINTS** - Guards against "tyranny of the laughing emoji":
   - Personal content restricts critical reactions (only supportive reactions permitted)
   - Attribution required (no anonymous mockery)
   - Authors can hide harmful reactions
   - Critical reactions require reasoning

3. **INTELLECTUAL HUMILITY** (Micah 6:8 - "walk humbly"):
   - Profiles can UPGRADE through trust-building
   - Profiles can DOWNGRADE through new evidence (retractions, new research)
   - The system acknowledges it could be wrong

4. **PATH INHERITANCE: MOST RESTRICTIVE WINS**:
   - When content appears in path context, use the more restrictive profile

### 2.3 Route Patterns

```
# Feedback Profile Management
/{app}/{entityType}:{entityId}/feedback-profile              # View current profile
/{app}/{entityType}:{entityId}/feedback-profile/mechanisms   # Available mechanisms
/{app}/{entityType}:{entityId}/feedback-profile/history      # Profile change history
/{app}/{entityType}:{entityId}/feedback-profile/upgrade      # Request profile upgrade

# Emotional Reactions (when permitted)
/{app}/{entityType}:{entityId}/reactions                     # View reactions
/{app}/{entityType}:{entityId}/reactions/add                 # Add reaction
/{app}/{entityType}:{entityId}/reactions/{id}/hide           # Author hides reaction

# Engagement (based on permitted mechanisms)
/{app}/{entityType}:{entityId}/approve                       # Approval vote
/{app}/{entityType}:{entityId}/share                         # Share with context
```

### 2.4 Feedback Mechanisms (Friction Hierarchy)

| Level | Mechanism | Description |
|-------|-----------|-------------|
| Low | `approval-vote` | Up/down voting (replaces "like") |
| Low | `emotional-reaction` | "I feel ___ about this" with context |
| Low | `affinity-mark` | Personal connection marker (private) |
| Medium | `graduated-usefulness` | Loomio-style scale |
| Medium | `graduated-accuracy` | Fact-checking scale |
| Medium | `share-with-context` | Amplification requires context |
| High | `proposal-vote` | Formal voting with reasoning |
| High | `challenge` | Constitutional challenge |
| High | `discussion-only` | No amplification |
| High | `citation` | Academic reference |
| High | `peer-review` | Formal review |
| None | `view-only` | No engagement permitted |

### 2.5 Emotional Reaction Types

**Supportive (safe for personal content):**
- `moved` - This moved me emotionally
- `grateful` - I am grateful for this
- `inspired` - This inspires me
- `hopeful` - This gives me hope
- `grieving` - This connects to grief/loss

**Critical (require accountability, may be restricted):**
- `challenged` - This challenged my thinking
- `concerned` - This concerns me
- `uncomfortable` - This makes me uncomfortable

---

## Part 3: GovernanceService

The governance service manages the constitutional feedback layer - the protocol's immune system. It handles graduated feedback, deliberation proposals, sensemaking, discussions, challenges, and appeals.

```typescript
interface GovernanceService {
  // =========================================================================
  // Context Menu & Summary
  // =========================================================================

  /**
   * Get the governance context menu for any entity.
   * Returns status summary, available actions, and active alerts.
   * This is the "door" to the governance dimension.
   */
  getContextMenu(
    entityType: GovernableEntityType,
    entityId: string
  ): Promise<GovernanceContextMenu>;

  /**
   * Get detailed governance summary for an entity.
   * Includes status, labels, active items counts, and recent events.
   */
  getGovernanceState(
    entityType: GovernableEntityType,
    entityId: string
  ): Promise<GovernanceState>;

  // =========================================================================
  // Graduated Feedback (Loomio-inspired)
  // =========================================================================

  /**
   * Get the feedback selector configuration for a specific context.
   */
  getFeedbackSelector(
    entityType: GovernableEntityType,
    entityId: string,
    context: FeedbackContext
  ): Promise<GraduatedFeedbackSelector>;

  /**
   * Submit graduated feedback on an entity.
   * Optionally includes reasoning (required for negative feedback).
   */
  submitFeedback(
    entityType: GovernableEntityType,
    entityId: string,
    context: FeedbackContext,
    response: FeedbackResponse
  ): Promise<void>;

  /**
   * Get aggregate feedback view for an entity.
   */
  getFeedbackAggregate(
    entityType: GovernableEntityType,
    entityId: string,
    context: FeedbackContext
  ): Promise<FeedbackAggregateView>;

  // =========================================================================
  // Deliberation (Loomio-style Proposals)
  // =========================================================================

  /**
   * List proposals for an entity.
   */
  getProposals(
    entityType: GovernableEntityType,
    entityId: string,
    filters?: { phase?: ProposalPhase }
  ): Promise<DeliberationProposal[]>;

  getProposal(proposalId: string): Promise<DeliberationProposal>;

  createProposal(
    proposal: Omit<DeliberationProposal, 'id' | 'proposedAt' | 'phase' | 'results'>
  ): Promise<DeliberationProposal>;

  voteOnProposal(proposalId: string, vote: ProposalVote): Promise<void>;

  // =========================================================================
  // Sensemaking (Polis-inspired)
  // =========================================================================

  /**
   * Get sensemaking visualization for an entity.
   * Shows opinion clusters, consensus statements, and divisive statements.
   */
  getSensemakingVisualization(
    entityType: GovernableEntityType,
    entityId: string
  ): Promise<SensemakingVisualization>;

  submitStatement(
    entityType: GovernableEntityType,
    entityId: string,
    statement: string
  ): Promise<string>;

  voteOnStatement(
    statementId: string,
    vote: 'agree' | 'disagree' | 'pass'
  ): Promise<void>;

  // =========================================================================
  // History (Wikipedia-style Audit Trail)
  // =========================================================================

  getHistoryView(
    entityType: GovernableEntityType,
    entityId: string,
    tab: HistoryTab,
    filters?: HistoryFilters
  ): Promise<GovernanceHistoryView>;

  // =========================================================================
  // Discussions (Talk Pages)
  // =========================================================================

  getDiscussions(
    entityType: GovernableEntityType,
    entityId: string
  ): Promise<DiscussionThread[]>;

  getDiscussionThread(threadId: string): Promise<DiscussionThread>;

  createDiscussionThread(
    entityType: GovernableEntityType,
    entityId: string,
    category: DiscussionCategory,
    topic: string,
    initialMessage: string
  ): Promise<DiscussionThread>;

  postMessage(
    threadId: string,
    message: Omit<DiscussionMessage, 'id' | 'timestamp' | 'reactions' | 'edited' | 'hidden'>
  ): Promise<DiscussionMessage>;

  // =========================================================================
  // Challenges & Appeals (Constitutional Accountability)
  // =========================================================================

  /**
   * File a constitutional challenge.
   * Every decision in the system can be challenged.
   */
  fileChallenge(
    challenge: Omit<Challenge, 'id' | 'filedAt' | 'state'>
  ): Promise<Challenge>;

  getChallenges(
    entityType: GovernableEntityType,
    entityId: string
  ): Promise<Challenge[]>;

  respondToChallenge(
    challengeId: string,
    response: ChallengeResponse
  ): Promise<void>;

  fileAppeal(
    challengeId: string,
    appeal: Omit<Appeal, 'id' | 'filedAt'>
  ): Promise<Appeal>;

  // =========================================================================
  // Subscriptions & Notifications
  // =========================================================================

  subscribeToEntity(
    entityType: GovernableEntityType,
    entityId: string,
    events: AlertType[]
  ): Promise<void>;

  unsubscribeFromEntity(
    entityType: GovernableEntityType,
    entityId: string
  ): Promise<void>;

  // =========================================================================
  // Global Governance (Precedents & Oversight)
  // =========================================================================

  getPrecedents(
    filters?: PrecedentFilters,
    pagination?: PaginationParams
  ): Promise<PrecedentListResult>;

  getPrecedent(precedentId: string): Promise<Precedent>;

  getSLADashboard(): Promise<SLADashboard>;

  // =========================================================================
  // Feedback Profile (Virality as Privilege)
  // =========================================================================

  getFeedbackProfile(
    entityType: GovernableEntityType,
    entityId: string
  ): Promise<FeedbackProfile>;

  requestProfileUpgrade(
    entityType: GovernableEntityType,
    entityId: string,
    requestedMechanisms: FeedbackMechanism[],
    justification: string
  ): Promise<ProfileUpgradeRequest>;

  getEmotionalReactions(
    entityType: GovernableEntityType,
    entityId: string
  ): Promise<EmotionalReactionSummary>;

  addEmotionalReaction(
    entityType: GovernableEntityType,
    entityId: string,
    reaction: EmotionalReactionInput
  ): Promise<void>;

  hideReaction(
    entityType: GovernableEntityType,
    entityId: string,
    reactionId: string
  ): Promise<void>;
}
```

---

## Part 4: Types

```typescript
type GovernableEntityType =
  | 'content' | 'path' | 'assessment' | 'contributor'
  | 'human' | 'elohim' | 'collective' | 'governance-decision';

type FeedbackContext =
  | 'accuracy' | 'usefulness' | 'clarity' | 'depth' | 'timeliness'
  | 'appropriateness' | 'sensitivity' | 'label-agreement'
  | 'decision-agreement' | 'proposal-position' | 'contribution-value'
  | 'trust-level' | 'elohim-helpfulness' | 'elohim-accuracy' | 'elohim-fairness';

type ProposalPhase = 'draft' | 'discussion' | 'voting' | 'closed' | 'decided' | 'implemented';

type HistoryTab = 'summary' | 'versions' | 'discussions' | 'governance' | 'engagement';

type AlertType =
  | 'vote-open' | 'challenge-pending' | 'discussion-active'
  | 'label-applied' | 'review-requested' | 'sla-warning';

type FeedbackMechanism =
  | 'approval-vote' | 'emotional-reaction' | 'affinity-mark'
  | 'graduated-usefulness' | 'graduated-accuracy' | 'share-with-context'
  | 'proposal-vote' | 'challenge' | 'discussion-only' | 'citation' | 'peer-review'
  | 'view-only';

type EmotionalReactionType =
  | 'moved' | 'grateful' | 'inspired' | 'hopeful' | 'grieving'
  | 'challenged' | 'concerned' | 'uncomfortable';

interface ProposalVote {
  optionId?: string;
  ranking?: string[];
  scores?: Record<string, number>;
  reasoning?: string;
}

interface ChallengeResponse {
  decision: 'upheld' | 'rejected' | 'modified';
  reasoning: string;
  precedentCited?: string[];
  modifiedAction?: string;
}

interface ProfileUpgradeRequest {
  id: string;
  entityType: GovernableEntityType;
  entityId: string;
  requestedMechanisms: FeedbackMechanism[];
  justification: string;
  requestedAt: string;
  requestedBy: string;
  status: 'pending' | 'approved' | 'denied';
  reviewedBy?: string;
  reviewedAt?: string;
  reviewReasoning?: string;
}

interface SLADashboard {
  challengesMeetingAcknowledgmentSLA: number;
  challengesMeetingResolutionSLA: number;
  appealsMeetingSLA: number;
  challengesByAge: Array<{
    ageCategory: string;
    count: number;
    percentageOfSLA: number;
  }>;
  slaComplianceHistory: Array<{
    period: string;
    complianceRate: number;
  }>;
  activeEscalations: number;
  escalationsByLevel: Record<string, number>;
}
```

---

## Part 5: Implementation Guidance

The governance service operates as an overlay on all other entities. Every content node, learning path, human agent, Elohim agent, and even governance decisions themselves have a governance dimension that can be accessed through this service.

The `getContextMenu` method is the entry point. It should return quickly and provide just enough information for the UI to render a governance badge and quick action menu. Think of it as the "door" to the governance dimension.

The feedback methods implement Loomio-inspired graduated feedback. Unlike simple up/down voting, feedback is context-specific. Negative feedback requires reasoning to ensure accountability.

The deliberation methods handle formal proposals. Proposal types (advice, consent, consensus) have different voting rules and passage thresholds.

The sensemaking methods implement Polis-style opinion clustering. Bridging statements (statements that clusters agree on despite disagreeing on most things) are surfaced as opportunities for finding common ground.

The challenge and appeal methods implement constitutional accountability. Every challenge MUST get a response within the SLA. Precedents created through challenge resolutions form the evolving constitutional DNA.

---

## Cross-References

- **Lamad Pillar:** `lamad/LAMAD_API_SPECIFICATION_v1.0.md` - Content and learning path routes
- **Elohim Pillar:** `elohim/ELOHIM_PROTOCOL_ARCHITECTURE.md` - How pillars compose
- **Shefa Pillar:** Economic event tracking for governance actions
- **Imagodei Pillar:** Agent identity for governance participants
