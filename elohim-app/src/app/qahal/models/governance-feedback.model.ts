/**
 * Governance & Feedback Models - The Protocol's Immune System
 *
 * "In systems theory every system needs meaningful feedback or it goes chaotic."
 * - Destin Sandlin, Smarter Every Day
 *
 * This module provides the constitutional feedback mechanisms that prevent
 * the Elohim Protocol from becoming another Facebook - where:
 * - 16 years of work can be stolen with no recourse
 * - Oversight boards make recommendations but aren't accountable
 * - Humans have no meaningful way to challenge decisions
 *
 * Core Principles:
 * 1. EVERY entity (content, human, assessment, path) has a governance state
 * 2. EVERY decision can be challenged
 * 3. EVERY challenge gets a response (with SLA)
 * 4. Escalation paths are CONSTITUTIONAL (not discretionary)
 * 5. Feedback loops are VISIBLE (transparency builds trust)
 * 6. Power is DISTRIBUTED (no single point of failure)
 *
 * The governance graph overlays the content graph:
 * - Content → has → GovernanceState
 * - GovernanceState → has → Reviews, Challenges, Appeals
 * - Challenges → resolved by → Elohim at appropriate level
 * - Appeals → escalate to → higher Elohim (family → community → network)
 * - Decisions → create → Precedent (constitutional evolution)
 *
 * Holochain mapping:
 * - GovernanceState as entry on DHT (public accountability)
 * - Votes as countersigned entries (tamper-proof)
 * - Challenges/Appeals linked to source entries
 * - Precedent forms constitutional DNA
 */

// ============================================================================
// Governance State (Every Entity Has One)
// ============================================================================

/**
 * GovernableEntity - Types of things that need governance.
 */
export type GovernableEntityType =
  | 'content' // ContentNode
  | 'path' // LearningPath
  | 'assessment' // AssessmentInstrument
  | 'contributor' // ContributorPresence
  | 'human' // Agent (human identity)
  | 'elohim' // ElohimAgent (yes, even AI needs oversight)
  | 'collective' // CollectiveKnowledgeMap (organizations)
  | 'governance-decision'; // Decisions themselves can be challenged

/**
 * ActorType - Types of actors that can perform governance actions.
 */
export type ActorType = 'human' | 'elohim' | 'collective';

/**
 * DeciderType - Types of decision makers (includes algorithm).
 */
export type DeciderType = 'algorithm' | ActorType;

/**
 * GovernanceState - The current governance status of any entity.
 *
 * Every entity in the system has a GovernanceState that tracks:
 * - Current trust level
 * - Review history
 * - Active challenges
 * - Pending actions
 */
export interface GovernanceState {
  /** The entity being governed */
  entityType: GovernableEntityType;
  entityId: string;

  /** Current trust/verification status */
  status: GovernanceStatus;

  /** How was this status determined? */
  statusBasis: StatusBasis;

  /** Active labels/flags on this entity */
  labels: GovernanceLabel[];

  /** Review history */
  reviews: ReviewRecord[];

  /** Active challenges (unresolved) */
  activeChallenges: Challenge[];

  /** Resolved challenges (for precedent) */
  resolvedChallenges: ResolvedChallenge[];

  /** Current restrictions (if any) */
  restrictions: Restriction[];

  /** Who is currently responsible for this entity's governance? */
  governingElohim: ElohimJurisdiction;

  /** When was governance state last updated? */
  lastUpdated: string;

  /** Hash of previous state (for audit trail) */
  previousStateHash?: string;
}

export type GovernanceStatus =
  | 'unreviewed' // New, not yet examined
  | 'auto-approved' // Passed automated checks
  | 'community-reviewed' // Reviewed by community members
  | 'elohim-reviewed' // Reviewed by Elohim agent
  | 'challenged' // Currently under challenge
  | 'restricted' // Limited due to concerns
  | 'suspended' // Temporarily removed from circulation
  | 'removed' // Permanently removed (with record)
  | 'appealing' // Removal under appeal
  | 'restored' // Previously removed, now restored
  | 'constitutional'; // Core protocol content (highest trust)

export interface StatusBasis {
  /** How was this status determined? */
  method: 'automated' | 'community-vote' | 'elohim-decision' | 'appeal-outcome' | 'constitutional';

  /** Evidence/reasoning for this status */
  reasoning: string;

  /** Who/what made this determination? */
  deciderId: string;
  deciderType: DeciderType;

  /** When was this determination made? */
  decidedAt: string;

  /** Link to detailed decision record */
  decisionRecordId?: string;
}

// ============================================================================
// Labels (Taxonomy of Concerns)
// ============================================================================

/**
 * GovernanceLabel - A classification applied to an entity.
 *
 * Labels are the vocabulary of governance. They must be:
 * - Well-defined (not vague)
 * - Challengeable (not absolute)
 * - Graduated (not binary)
 */
export interface GovernanceLabel {
  id: string;

  /** Label type from taxonomy */
  labelType: LabelType;

  /** Severity/confidence */
  severity: LabelSeverity;

  /** Who applied this label? */
  appliedBy: string;
  appliedByType: 'algorithm' | 'human' | 'elohim';

  /** When applied */
  appliedAt: string;

  /** Evidence for this label */
  evidence: LabelEvidence[];

  /** Is this label currently active? */
  active: boolean;

  /** If challenged, link to challenge */
  challengeId?: string;

  /** Expiration (some labels are temporary) */
  expiresAt?: string;
}

export type LabelType =
  // Content Quality
  | 'unverified-claims' // Contains claims not yet verified
  | 'disputed-accuracy' // Accuracy contested by credible sources
  | 'outdated-information' // Information may be stale
  | 'missing-attribution' // Sources not properly cited
  | 'ai-generated' // Created by AI (not inherently bad)
  | 'ai-generated-undisclosed' // AI content presented as human

  // Safety Concerns
  | 'content-warning' // May be distressing (not removal-worthy)
  | 'age-restricted' // Not appropriate for minors
  | 'trauma-sensitive' // May trigger trauma responses
  | 'crisis-resource-needed' // Should link to crisis resources

  // Policy Violations
  | 'potential-harassment' // May constitute harassment
  | 'potential-misinformation' // May be false/misleading
  | 'potential-manipulation' // May be manipulative
  | 'potential-spam' // May be spam/promotional
  | 'potential-impersonation' // May be impersonating someone
  | 'copyright-concern' // May infringe copyright
  | 'privacy-concern' // May violate privacy

  // Positive Labels
  | 'fact-checked' // Verified by fact-checkers
  | 'expert-reviewed' // Reviewed by domain expert
  | 'community-trusted' // High community trust score
  | 'source-verified' // Sources have been verified
  | 'contributor-verified' // Contributor identity verified

  // Meta Labels
  | 'under-review' // Currently being reviewed
  | 'review-requested' // Community requested review
  | 'precedent-setting'; // May set governance precedent

export type LabelSeverity =
  | 'informational' // FYI, no action needed
  | 'advisory' // Caution advised
  | 'warning' // Significant concern
  | 'critical' // Serious violation
  | 'emergency'; // Immediate action required

export interface LabelEvidence {
  type: 'automated-detection' | 'human-report' | 'elohim-analysis' | 'external-source';
  description: string;
  sourceId?: string;
  confidence: number; // 0.0 - 1.0
  timestamp: string;
}

// ============================================================================
// Reviews (Ongoing Assessment)
// ============================================================================

/**
 * ReviewRecord - A review of an entity.
 */
export interface ReviewRecord {
  id: string;

  /** Who conducted this review? */
  reviewerId: string;
  reviewerType: ActorType;

  /** Type of review */
  reviewType: ReviewType;

  /** Outcome of review */
  outcome: ReviewOutcome;

  /** Detailed findings */
  findings: string;

  /** Labels added/removed as result */
  labelsAdded: string[];
  labelsRemoved: string[];

  /** Recommendations */
  recommendations: ReviewRecommendation[];

  /** When conducted */
  conductedAt: string;

  /** Time spent (for quality metrics) */
  durationMinutes?: number;

  /** Review quality score (meta-review) */
  qualityScore?: number;
}

export type ReviewType =
  | 'initial' // First review of new content
  | 'periodic' // Scheduled re-review
  | 'triggered' // Triggered by report/flag
  | 'appeal' // Review as part of appeal
  | 'precedent' // Review for precedent-setting
  | 'meta'; // Review of review quality

export type ReviewOutcome =
  | 'approved' // No concerns found
  | 'approved-with-labels' // Approved but labels added
  | 'needs-modification' // Creator should modify
  | 'escalate' // Needs higher-level review
  | 'restrict' // Should be restricted
  | 'remove' // Should be removed
  | 'no-action'; // Review complete, no change

export interface ReviewRecommendation {
  action: 'add-label' | 'remove-label' | 'modify-content' | 'add-context' | 'escalate' | 'restore';
  details: string;
  priority: 'low' | 'medium' | 'high' | 'urgent';
}

// ============================================================================
// Challenges (The Right to Contest)
// ============================================================================

/**
 * Challenge - A formal contestation of a governance decision.
 *
 * EVERY governance decision can be challenged. This is constitutional.
 * The system MUST respond within SLA. Silence is not an option.
 */
export interface Challenge {
  id: string;

  /** What is being challenged? */
  targetType: 'label' | 'status' | 'restriction' | 'removal' | 'decision' | 'review';
  targetId: string;

  /** Who is challenging? */
  challengerId: string;
  challengerType: ActorType;

  /** Standing - why does challenger have right to challenge? */
  standing: ChallengeStanding;

  /** Grounds for challenge */
  grounds: ChallengeGrounds;

  /** Evidence supporting challenge */
  evidence: ChallengeEvidence[];

  /** Requested outcome */
  requestedOutcome: string;

  /** Current state of challenge */
  state: ChallengeState;

  /** Timeline */
  filedAt: string;
  responseDeadline: string; // SLA - system MUST respond by this time
  respondedAt?: string;

  /** Assigned handler */
  assignedTo?: string;
  assignedElohimLevel: ElohimLevel;

  /** Response (when resolved) */
  response?: ChallengeResponse;
}

export interface ChallengeStanding {
  /** Basis for standing */
  basis:
    | 'content-owner'
    | 'affected-party'
    | 'community-member'
    | 'public-interest'
    | 'constitutional';

  /** Explanation of standing */
  explanation: string;

  /** Verified? */
  verified: boolean;
}

export interface ChallengeGrounds {
  /** Primary ground for challenge */
  primary: ChallengeGroundType;

  /** Secondary grounds */
  secondary?: ChallengeGroundType[];

  /** Detailed argument */
  argument: string;

  /** Precedents cited */
  precedentsCited?: string[];
}

export type ChallengeGroundType =
  | 'factual-error' // The facts are wrong
  | 'misapplication' // Policy applied incorrectly
  | 'context-ignored' // Important context was missed
  | 'bias' // Decision reflects bias
  | 'inconsistency' // Inconsistent with similar cases
  | 'procedural-violation' // Process wasn't followed
  | 'proportionality' // Response disproportionate to concern
  | 'new-evidence' // New evidence available
  | 'changed-circumstances' // Circumstances have changed
  | 'constitutional'; // Violates protocol constitution

export interface ChallengeEvidence {
  type: 'document' | 'testimony' | 'precedent' | 'expert-opinion' | 'data';
  description: string;
  contentId?: string;
  url?: string;
  submittedAt: string;
}

export type ChallengeState =
  | 'filed' // Just submitted
  | 'acknowledged' // System has acknowledged receipt
  | 'under-review' // Being actively reviewed
  | 'additional-info-needed' // Challenger needs to provide more
  | 'escalated' // Moved to higher level
  | 'resolved' // Decision made
  | 'appealed'; // Being appealed to higher level

export interface ChallengeResponse {
  /** Outcome */
  outcome: 'upheld' | 'partially-upheld' | 'denied' | 'referred';

  /** Reasoning (MUST be provided) */
  reasoning: string;

  /** Actions taken */
  actionsTaken: GovernanceAction[];

  /** Respondent */
  responderId: string;
  responderType: 'human' | 'elohim';
  responderLevel: ElohimLevel;

  /** Response time (for SLA tracking) */
  responseTime: number; // milliseconds from filing

  /** Can be appealed? */
  appealable: boolean;
  appealDeadline?: string;

  /** Precedent value */
  precedentValue: 'none' | 'informative' | 'binding';
}

// ============================================================================
// Appeals (Escalation Path)
// ============================================================================

/**
 * ResolvedChallenge - A challenge that has been resolved.
 */
export interface ResolvedChallenge extends Challenge {
  state: 'resolved' | 'appealed';
  response: ChallengeResponse;
  resolvedAt: string;

  /** If appealed, link to appeal */
  appealId?: string;
}

/**
 * Appeal - Escalation of a challenge to higher authority.
 *
 * Appeals follow constitutional hierarchy:
 * Individual Elohim → Family Elohim → Community Elohim → Network Council
 */
export interface Appeal {
  id: string;

  /** Original challenge being appealed */
  challengeId: string;

  /** Appellant */
  appellantId: string;

  /** Grounds for appeal (must be new or procedural) */
  grounds: AppealGrounds;

  /** Current level */
  currentLevel: ElohimLevel;

  /** Appeal history (if escalated multiple times) */
  escalationHistory: AppealEscalation[];

  /** State */
  state: AppealState;

  /** Timeline */
  filedAt: string;
  responseDeadline: string;

  /** Final decision (when resolved) */
  finalDecision?: AppealDecision;
}

export interface AppealGrounds {
  type: 'procedural-error' | 'new-evidence' | 'misapplication' | 'constitutional-question';
  argument: string;
  newEvidence?: ChallengeEvidence[];
}

export interface AppealEscalation {
  fromLevel: ElohimLevel;
  toLevel: ElohimLevel;
  reason: string;
  escalatedAt: string;
  escalatedBy: string;
}

export type AppealState =
  | 'filed'
  | 'accepted' // Appeal accepted for review
  | 'rejected' // Appeal not accepted (no standing/grounds)
  | 'under-review'
  | 'escalated'
  | 'resolved';

export interface AppealDecision {
  outcome: 'affirmed' | 'reversed' | 'modified' | 'remanded';
  reasoning: string;
  actions: GovernanceAction[];
  precedentCreated?: Precedent;
  decidedBy: string;
  decidedByLevel: ElohimLevel;
  decidedAt: string;
}

// ============================================================================
// Elohim Jurisdiction
// ============================================================================

export type ElohimLevel =
  | 'individual' // Personal Elohim (AI assistant level)
  | 'family' // Family Elohim (household/small group)
  | 'community' // Community Elohim (local community)
  | 'network' // Network Council (protocol-wide)
  | 'constitutional'; // Constitutional matters only

export interface ElohimJurisdiction {
  /** Primary responsible Elohim */
  primaryElohimId: string;
  level: ElohimLevel;

  /** Backup/oversight Elohim */
  oversightElohimId?: string;
  oversightLevel?: ElohimLevel;

  /** Jurisdiction basis */
  basis:
    | 'content-origin'
    | 'creator-affiliation'
    | 'subject-matter'
    | 'geographic'
    | 'constitutional';

  /** Can this be appealed to higher level? */
  appealPath: ElohimLevel[];
}

// ============================================================================
// Governance Actions
// ============================================================================

export interface GovernanceAction {
  type: GovernanceActionType;
  targetEntityId: string;
  details: string;
  effectiveAt: string;
  expiresAt?: string;
  reversible: boolean;
  executedBy: string;
}

export type GovernanceActionType =
  | 'add-label'
  | 'remove-label'
  | 'add-restriction'
  | 'remove-restriction'
  | 'suspend'
  | 'unsuspend'
  | 'remove'
  | 'restore'
  | 'modify-visibility'
  | 'require-context'
  | 'assign-review'
  | 'create-precedent';

// ============================================================================
// Restrictions
// ============================================================================

export interface Restriction {
  id: string;
  type: RestrictionType;
  reason: string;
  appliedAt: string;
  appliedBy: string;
  expiresAt?: string;
  challengeable: boolean;
  challengeId?: string;
}

export type RestrictionType =
  | 'visibility-limited' // Only visible to some
  | 'interaction-limited' // Can view but not interact
  | 'distribution-limited' // Cannot be shared/cited
  | 'monetization-disabled' // No recognition flows
  | 'requires-warning' // Must show warning before display
  | 'requires-context' // Must show context/fact-check
  | 'age-gated' // Age verification required
  | 'geographic-limited'; // Limited to certain regions

// ============================================================================
// Precedent (Constitutional Evolution)
// ============================================================================

/**
 * Precedent - A decision that informs future decisions.
 *
 * Unlike Facebook's ad-hoc moderation, Elohim governance builds
 * a body of precedent that creates predictable, fair outcomes.
 */
export interface Precedent {
  id: string;

  /** Case that created this precedent */
  sourceCase: {
    type: 'challenge' | 'appeal' | 'constitutional-review';
    id: string;
  };

  /** Summary of the rule established */
  rule: string;

  /** Detailed reasoning */
  reasoning: string;

  /** When established */
  establishedAt: string;

  /** By whom */
  establishedBy: string;
  establishedByLevel: ElohimLevel;

  /** Binding level */
  bindingLevel: PrecedentBinding;

  /** Scope - what does this precedent apply to? */
  scope: PrecedentScope;

  /** Related precedents (for coherence) */
  relatedPrecedents: string[];

  /** Has this been superseded? */
  supersededBy?: string;

  /** Times cited */
  citationCount: number;

  /** Constitutional alignment */
  constitutionalBasis?: string;
}

export type PrecedentBinding =
  | 'persuasive' // Informative but not binding
  | 'binding-local' // Binding within community
  | 'binding-network' // Binding protocol-wide
  | 'constitutional'; // Part of constitution

export interface PrecedentScope {
  entityTypes: GovernableEntityType[];
  labelTypes?: LabelType[];
  contexts?: string[];
  exceptions?: string[];
}

// ============================================================================
// Content Governance History (Wikipedia-style Changelog + Talk Page)
// ============================================================================

/**
 * ContentGovernanceHistory - The full changelog for any piece of content.
 *
 * Like Wikipedia's combination of:
 * - Edit History (what changed, when, by whom)
 * - Talk Page (meta-conversation about the content)
 * - Protection Log (governance actions)
 * - View History (who's engaging)
 *
 * But structured for graph-native content with multiple dimensions of feedback.
 *
 * This is the PUBLIC RECORD of everything that's happened to a piece of content.
 * Transparency builds trust. Hidden moderation breeds resentment.
 */
export interface ContentGovernanceHistory {
  /** The content being tracked */
  entityType: GovernableEntityType;
  entityId: string;

  /** Human-readable title for reference */
  entityTitle: string;

  /** Creation record */
  creation: CreationRecord;

  /** Version history (content changes) */
  versionHistory: VersionRecord[];

  /** Current version */
  currentVersion: number;

  /** Governance timeline (all governance events, chronological) */
  governanceTimeline: GovernanceEvent[];

  /** Meta-conversations (Talk pages) */
  discussions: DiscussionThread[];

  /** Active discussion count (for UI) */
  activeDiscussionCount: number;

  /** Attribution changelog (contributor changes) */
  attributionHistory: AttributionChange[];

  /** Relationship changelog (graph connections) */
  relationshipHistory: RelationshipChange[];

  /** Reach/visibility changelog */
  reachHistory: ReachChange[];

  /** Protection status changelog */
  protectionHistory: ProtectionChange[];

  /** Aggregate engagement metrics over time */
  engagementHistory: EngagementSnapshot[];

  /** Summary statistics */
  stats: GovernanceHistoryStats;
}

/**
 * CreationRecord - The birth certificate of content.
 */
export interface CreationRecord {
  createdAt: string;
  createdBy: string;
  creatorType: ActorType;

  /** Original context (how was this created?) */
  creationContext: CreationContext;

  /** Initial governance state */
  initialStatus: GovernanceStatus;

  /** Initial labels applied */
  initialLabels: string[];

  /** Derivation (if derived from other content) */
  derivedFrom?: DerivationRecord;
}

export type CreationContext =
  | 'original' // Created from scratch
  | 'imported' // Imported from external source
  | 'forked' // Forked from existing content
  | 'synthesized' // AI-synthesized
  | 'collaborative' // Created by multiple contributors
  | 'converted' // Converted from another format
  | 'restored'; // Restored from deletion

export interface DerivationRecord {
  sourceId: string;
  sourceTitle: string;
  derivationType: 'fork' | 'adaptation' | 'translation' | 'excerpt' | 'synthesis';
  relationship: string;
  licenseCompatibility: boolean;
}

/**
 * VersionRecord - A snapshot of content at a point in time.
 */
export interface VersionRecord {
  version: number;
  timestamp: string;

  /** Who made this change? */
  authorId: string;
  authorType: ActorType;

  /** What changed? */
  changeType: VersionChangeType;

  /** Summary of changes */
  summary: string;

  /** Detailed change description */
  description?: string;

  /** Diff from previous version (if applicable) */
  diff?: ContentDiff;

  /** Size change */
  sizeChange: number; // bytes added/removed

  /** Was this reviewed? */
  reviewStatus: 'pending' | 'approved' | 'reverted';

  /** If reverted, why? */
  revertReason?: string;

  /** Can this version be restored? */
  restorable: boolean;

  /** Content hash (for integrity verification) */
  contentHash: string;
}

export type VersionChangeType =
  | 'initial' // First version
  | 'edit' // Content edit
  | 'correction' // Factual correction
  | 'clarification' // Clarification without changing meaning
  | 'expansion' // Added content
  | 'reduction' // Removed content
  | 'restructure' // Reorganized without changing content
  | 'metadata-update' // Only metadata changed
  | 'format-change' // Format conversion
  | 'revert' // Reverted to previous version
  | 'merge' // Merged from another version/fork
  | 'auto-update'; // Automated update (e.g., link fixing)

export interface ContentDiff {
  type: 'text' | 'structured' | 'binary';
  additions: number;
  deletions: number;
  changes: DiffChange[];
}

export interface DiffChange {
  location: string; // Path or line number
  type: 'add' | 'remove' | 'modify';
  before?: string;
  after?: string;
}

/**
 * GovernanceEvent - Any governance-related event in the timeline.
 */
export interface GovernanceEvent {
  id: string;
  timestamp: string;

  /** Event type */
  eventType: GovernanceEventType;

  /** Actor who triggered this event */
  actorId: string;
  actorType: 'human' | 'elohim' | 'algorithm' | 'collective';

  /** Description of what happened */
  description: string;

  /** Detailed data (type depends on eventType) */
  details: Record<string, unknown>;

  /** Related entities */
  relatedEntities?: { type: string; id: string }[];

  /** Visibility of this event */
  visibility: 'public' | 'participants-only' | 'elohim-only';
}

export type GovernanceEventType =
  // Status changes
  | 'status-change'
  | 'label-added'
  | 'label-removed'
  | 'label-challenged'

  // Reviews
  | 'review-requested'
  | 'review-completed'
  | 'review-escalated'

  // Challenges
  | 'challenge-filed'
  | 'challenge-acknowledged'
  | 'challenge-responded'
  | 'challenge-resolved'

  // Appeals
  | 'appeal-filed'
  | 'appeal-escalated'
  | 'appeal-decided'

  // Restrictions
  | 'restriction-applied'
  | 'restriction-removed'
  | 'protection-changed'

  // Votes
  | 'vote-initiated'
  | 'vote-completed'

  // Meta
  | 'discussion-started'
  | 'discussion-resolved'
  | 'precedent-cited'
  | 'precedent-created';

/**
 * DiscussionThread - Meta-conversation about content (Talk page).
 */
export interface DiscussionThread {
  id: string;

  /** Thread topic */
  topic: string;

  /** What aspect of content is this about? */
  category: DiscussionCategory;

  /** Thread status */
  status: 'open' | 'resolved' | 'stale' | 'archived' | 'locked';

  /** Who started this discussion? */
  startedBy: string;
  startedAt: string;

  /** Resolution (if resolved) */
  resolution?: ThreadResolution;

  /** Messages in thread */
  messages: DiscussionMessage[];

  /** Participants */
  participants: string[];

  /** Is this thread linked to a governance action? */
  linkedGovernanceAction?: string;

  /** Tags for organization */
  tags: string[];

  /** Visibility */
  visibility: 'public' | 'contributors-only' | 'elohim-only';
}

export type DiscussionCategory =
  | 'accuracy' // Is this accurate?
  | 'neutrality' // Is this neutral/biased?
  | 'sources' // Source quality/citation
  | 'structure' // Organization of content
  | 'scope' // What should be included?
  | 'style' // Writing style concerns
  | 'attribution' // Who should be credited?
  | 'merge-proposal' // Should this merge with other content?
  | 'split-proposal' // Should this be split?
  | 'deletion-discussion' // Should this be deleted?
  | 'protection-request' // Request for protection
  | 'general'; // General discussion

export interface DiscussionMessage {
  id: string;
  authorId: string;
  authorType: 'human' | 'elohim';
  timestamp: string;
  content: string;

  /** Is this a reply to another message? */
  replyTo?: string;

  /** Message type */
  type:
    | 'comment'
    | 'proposal'
    | 'support'
    | 'oppose'
    | 'neutral'
    | 'question'
    | 'answer'
    | 'summary';

  /** Has this been edited? */
  edited: boolean;
  editHistory?: MessageEdit[];

  /** Reactions (lightweight feedback) */
  reactions: MessageReaction[];

  /** Is this message hidden/collapsed? */
  hidden: boolean;
  hiddenReason?: string;
}

export interface MessageEdit {
  editedAt: string;
  previousContent: string;
  reason?: string;
}

export interface MessageReaction {
  type: 'agree' | 'disagree' | 'helpful' | 'unhelpful' | 'question';
  count: number;
  reactors: string[]; // For transparency
}

export interface ThreadResolution {
  resolvedAt: string;
  resolvedBy: string;
  outcome: 'consensus' | 'no-consensus' | 'withdrawn' | 'superseded' | 'implemented';
  summary: string;
  actionsTaken?: string[];
}

/**
 * AttributionChange - Changes to who gets credit.
 */
export interface AttributionChange {
  timestamp: string;
  changeType: 'contributor-added' | 'contributor-removed' | 'share-adjusted' | 'role-changed';
  contributorId: string;
  previousState?: Record<string, unknown>;
  newState: Record<string, unknown>;
  reason: string;
  approvedBy: string;
}

/**
 * RelationshipChange - Changes to graph connections.
 */
export interface RelationshipChange {
  timestamp: string;
  changeType: 'link-added' | 'link-removed' | 'link-type-changed';
  relatedEntityId: string;
  relatedEntityType: string;
  relationshipType: string;
  previousRelationship?: string;
  reason: string;
  changedBy: string;
}

/**
 * ReachChange - Changes to visibility/access level.
 */
export interface ReachChange {
  timestamp: string;
  previousReach: string;
  newReach: string;
  reason: string;
  changedBy: string;
  challengeable: boolean;
}

/**
 * ProtectionChange - Changes to edit protection.
 */
export interface ProtectionChange {
  timestamp: string;
  changeType: 'protected' | 'unprotected' | 'protection-level-changed';
  protectionLevel: ProtectionLevel;
  reason: string;
  expiresAt?: string;
  changedBy: string;
}

export type ProtectionLevel =
  | 'none' // Anyone can edit
  | 'semi' // Established contributors only
  | 'extended' // Extended-confirmed contributors
  | 'full' // Elohim only
  | 'cascade'; // Protection cascades to included content

/**
 * EngagementSnapshot - Point-in-time engagement metrics.
 */
export interface EngagementSnapshot {
  timestamp: string;
  period: 'hourly' | 'daily' | 'weekly' | 'monthly';

  views: number;
  uniqueViewers: number;
  averageTimeSpent: number;
  affinityMarks: number;
  citations: number;
  shares: number;

  /** Sentiment indicators */
  sentiment: {
    positive: number;
    neutral: number;
    negative: number;
  };
}

/**
 * GovernanceHistoryStats - Summary statistics.
 */
export interface GovernanceHistoryStats {
  totalVersions: number;
  totalContributors: number;
  totalDiscussions: number;
  totalChallenges: number;
  totalReviews: number;

  daysSinceCreation: number;
  daysSinceLastEdit: number;
  daysSinceLastReview: number;

  averageVersionsPerMonth: number;
  controversyScore: number; // Based on challenges/reverts
  stabilityScore: number; // Based on recent edit frequency
}

/**
 * ContentGovernanceSummary - Lightweight summary for UI display.
 */
export interface ContentGovernanceSummary {
  entityId: string;
  entityTitle: string;

  currentStatus: GovernanceStatus;
  activeLabels: { type: LabelType; severity: LabelSeverity }[];

  lastEdited: string;
  lastEditedBy: string;

  versionCount: number;
  contributorCount: number;

  activeDiscussions: number;
  activeChallenges: number;

  protectionLevel: ProtectionLevel;

  /** Quick health indicators */
  health: {
    accuracy: 'verified' | 'disputed' | 'unknown';
    freshness: 'current' | 'needs-review' | 'stale';
    consensus: 'strong' | 'moderate' | 'contested';
  };
}

// ============================================================================
// Voting (Collective Decision-Making)
// ============================================================================

/**
 * GovernanceVote - A vote on a governance matter.
 */
export interface GovernanceVote {
  id: string;

  /** What is being voted on? */
  subject: VoteSubject;

  /** Who can vote? */
  eligibility: VoteEligibility;

  /** Voting mechanism */
  mechanism: VotingMechanism;

  /** Timeline */
  opensAt: string;
  closesAt: string;

  /** Quorum requirements */
  quorum: QuorumRequirement;

  /** Current state */
  state: VoteState;

  /** Results (when complete) */
  results?: VoteResults;

  /** Actions triggered by outcome */
  triggeredActions?: GovernanceAction[];
}

export interface VoteSubject {
  type:
    | 'label-application'
    | 'removal-decision'
    | 'appeal-outcome'
    | 'precedent-adoption'
    | 'constitutional-amendment';
  entityId: string;
  question: string;
  options: VoteOption[];
  context: string;
}

export interface VoteOption {
  id: string;
  label: string;
  description: string;
}

export interface VoteEligibility {
  /** Who can vote? */
  basis:
    | 'all-members'
    | 'affected-community'
    | 'domain-experts'
    | 'elohim-council'
    | 'stake-weighted';

  /** Minimum requirements */
  requirements?: {
    minTrustScore?: number;
    minTenure?: string; // ISO 8601 duration
    requiredAttestations?: string[];
  };

  /** Estimated eligible voters */
  estimatedEligible: number;
}

export type VotingMechanism =
  | 'simple-majority'
  | 'supermajority' // 2/3 or 3/4
  | 'ranked-choice'
  | 'quadratic' // Quadratic voting
  | 'conviction' // Conviction voting (time-weighted)
  | 'consent'; // Consent-based (no strong objections)

export interface QuorumRequirement {
  type: 'percentage' | 'absolute' | 'none';
  value?: number;
  description: string;
}

export type VoteState =
  | 'draft'
  | 'scheduled'
  | 'open'
  | 'closed'
  | 'tallying'
  | 'finalized'
  | 'contested';

export interface VoteResults {
  totalEligible: number;
  totalVoted: number;
  participationRate: number;
  quorumMet: boolean;
  optionResults: {
    optionId: string;
    votes: number;
    percentage: number;
  }[];
  outcome: string;
  certifiedBy: string;
  certifiedAt: string;
}

// ============================================================================
// Feedback Loop Metrics (System Health)
// ============================================================================

/**
 * GovernanceFeedbackMetrics - Health indicators for governance system.
 *
 * If these metrics go bad, the system is going chaotic (Destin's warning).
 */
export interface GovernanceFeedbackMetrics {
  /** Time period for metrics */
  period: {
    start: string;
    end: string;
  };

  /** Response times */
  challengeResponseTime: {
    median: number; // milliseconds
    p95: number;
    slaViolations: number;
  };

  /** Challenge outcomes */
  challengeOutcomes: {
    total: number;
    upheld: number;
    denied: number;
    partiallyUpheld: number;
    upheldRate: number; // High rate may indicate over-moderation
  };

  /** Appeal rates */
  appealMetrics: {
    totalChallenges: number;
    appealed: number;
    appealRate: number;
    reversalRate: number; // High reversal rate indicates problems
  };

  /** Consistency */
  consistencyMetrics: {
    similarCasesSameOutcome: number; // percentage
    precedentFollowed: number; // percentage
    inconsistenciesReported: number;
  };

  /** Community engagement */
  communityEngagement: {
    reviewsContributed: number;
    challengesFiled: number;
    votesParticipated: number;
    averageTrustScore: number;
  };

  /** Red flags */
  redFlags: GovernanceRedFlag[];
}

export interface GovernanceRedFlag {
  type:
    | 'sla-breach'
    | 'high-reversal-rate'
    | 'low-participation'
    | 'inconsistency-spike'
    | 'challenge-surge';
  severity: 'warning' | 'critical';
  description: string;
  detectedAt: string;
  recommendedAction: string;
}

// ============================================================================
// SLA Guarantees
// ============================================================================

/**
 * GovernanceSLA - Service Level Agreements for governance.
 *
 * These are CONSTITUTIONAL, not discretionary.
 */
export interface GovernanceSLA {
  /** Acknowledgment of challenge */
  challengeAcknowledgment: string; // "PT1H" = 1 hour

  /** Initial response to challenge */
  challengeInitialResponse: string; // "P3D" = 3 days

  /** Final resolution of challenge */
  challengeFinalResolution: string; // "P14D" = 14 days

  /** Appeal response */
  appealResponse: string; // "P7D" = 7 days

  /** Emergency (safety) response */
  emergencyResponse: string; // "PT4H" = 4 hours

  /** What happens on SLA breach */
  breachConsequences: SLABreachConsequence[];
}

export interface SLABreachConsequence {
  slaType: string;
  consequence: 'auto-escalate' | 'default-favor-challenger' | 'public-report' | 'elohim-review';
  description: string;
}

/**
 * Default SLA values (can be customized per community)
 */
export const DEFAULT_GOVERNANCE_SLA: GovernanceSLA = {
  challengeAcknowledgment: 'PT1H', // 1 hour
  challengeInitialResponse: 'P3D', // 3 days
  challengeFinalResolution: 'P14D', // 14 days
  appealResponse: 'P7D', // 7 days
  emergencyResponse: 'PT4H', // 4 hours
  breachConsequences: [
    {
      slaType: 'challengeAcknowledgment',
      consequence: 'auto-escalate',
      description: 'Auto-escalate to oversight Elohim',
    },
    {
      slaType: 'challengeFinalResolution',
      consequence: 'default-favor-challenger',
      description: 'If no response in 14 days, challenge is upheld by default',
    },
    {
      slaType: 'emergencyResponse',
      consequence: 'public-report',
      description: 'Emergency SLA breach is publicly reported',
    },
  ],
};
