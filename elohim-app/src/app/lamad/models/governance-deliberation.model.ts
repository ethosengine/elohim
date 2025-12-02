/**
 * Governance Deliberation Models - Collective Decision-Making Infrastructure
 *
 * Inspired by:
 * - Loomio: Structured proposal types with vote + reasoning visibility
 * - Polis: AI-powered opinion clustering, bridging statement detection
 * - Forby: Dynamic voting on visible proposals with real-time consensus
 * - Wikipedia: Talk pages, edit history, protection logs
 *
 * This module extends the governance-feedback.model with:
 * 1. Context-aware graduated feedback selectors on all graph content
 * 2. Governance dimension context menus accessible from any entity
 * 3. Deliberation interfaces (Loomio-style proposals)
 * 4. Sensemaking visualizations (Polis-style clustering)
 * 5. Audit trail displays (Wikipedia-style history)
 *
 * Every piece of content, every human, every Elohim has a governance
 * dimension accessible via a consistent pattern.
 */

import {
  GovernanceLabel,
  Challenge,
  Appeal,
  Precedent,
  VotingMechanism,
  DiscussionThread,
  DiscussionCategory,
  DiscussionMessage,
  VersionRecord,
  GovernanceEvent,
  GovernanceEventType,
  ContentGovernanceSummary,
  GovernableEntityType
} from './governance-feedback.model';

// ============================================================================
// Context Menu - Entry Point to Governance Dimension
// ============================================================================

/**
 * GovernanceContextMenu - The entry point for governance on any entity.
 *
 * Every entity in the graph (content, path, human, Elohim, assessment)
 * has this context menu available. It's the "door" to the governance dimension.
 *
 * Access Pattern: Floating action button or context menu item showing:
 * - Current governance status (badge)
 * - Quick actions (flag, discuss, challenge)
 * - Link to full governance view
 */
export interface GovernanceContextMenu {
  /** Entity being governed */
  entityType: GovernableEntityType;
  entityId: string;
  entityTitle: string;

  /** Quick status summary */
  summary: ContentGovernanceSummary;

  /** Available quick actions based on user's permissions */
  availableActions: GovernanceQuickAction[];

  /** Active alerts requiring attention */
  alerts: GovernanceAlert[];

  /** Link to full governance view */
  fullViewRoute: string;
}

export interface GovernanceQuickAction {
  id: string;
  type: QuickActionType;
  label: string;
  icon: string;
  description: string;
  enabled: boolean;
  disabledReason?: string;
  requiresAttestation?: string[];
}

export type QuickActionType =
  | 'flag'              // Report a concern
  | 'discuss'           // Start a discussion thread
  | 'challenge'         // Challenge a label/decision
  | 'vote'              // Vote on active proposal
  | 'review'            // Request or provide review
  | 'edit'              // Propose edit (if allowed)
  | 'protect'           // Request protection
  | 'view-history'      // View full history
  | 'view-discussions'  // View talk page
  | 'subscribe';        // Subscribe to updates

export interface GovernanceAlert {
  id: string;
  type: AlertType;
  severity: 'info' | 'warning' | 'urgent';
  message: string;
  actionRequired?: GovernanceQuickAction;
  expiresAt?: string;
}

export type AlertType =
  | 'vote-open'           // Active vote needs your participation
  | 'challenge-pending'   // Your challenge needs attention
  | 'discussion-active'   // Active discussion on content you follow
  | 'label-applied'       // New label applied to content you own
  | 'review-requested'    // Someone requested you review this
  | 'sla-warning';        // SLA deadline approaching

// ============================================================================
// Graduated Feedback Selector (Loomio-inspired)
// ============================================================================

/**
 * GraduatedFeedbackSelector - Context-aware feedback component.
 *
 * Unlike simple up/down voting, this provides graduated, contextual
 * feedback options based on:
 * - What type of entity is being evaluated
 * - What aspect is being evaluated (accuracy, usefulness, etc.)
 * - The user's relationship to the content
 *
 * Inspired by Loomio's proposal types and gradients of agreement.
 */
export interface GraduatedFeedbackSelector {
  /** What's being evaluated */
  targetType: GovernableEntityType;
  targetId: string;

  /** What aspect is being evaluated */
  feedbackContext: FeedbackContext;

  /** Available response options */
  options: FeedbackOption[];

  /** Selected option (if already responded) */
  currentResponse?: FeedbackResponse;

  /** Can user change their response? */
  canModify: boolean;

  /** Response deadline (if time-limited) */
  deadline?: string;

  /** Show reasoning from others? */
  showOthersReasoning: boolean;

  /** Aggregate results visualization */
  aggregateView?: FeedbackAggregateView;
}

/**
 * FeedbackContext - WHAT aspect is being evaluated.
 *
 * Note: This is distinct from FeedbackMechanism (see feedback-profile.model.ts).
 * - FeedbackMechanism = HOW you can engage (approval-vote, graduated-accuracy, etc.)
 * - FeedbackContext = WHAT aspect you're evaluating (accuracy, usefulness, etc.)
 *
 * A FeedbackProfile determines which mechanisms are available.
 * When using a graduated mechanism, the context determines which scale to show.
 *
 * Example flow:
 * 1. Content has FeedbackProfile with 'graduated-accuracy' permitted
 * 2. User engages with the 'graduated-accuracy' mechanism
 * 3. System shows the 'accuracy' context scale (Accurate → False)
 */
export type FeedbackContext =
  // Content quality
  | 'accuracy'            // Is this accurate?
  | 'usefulness'          // Was this useful to you?
  | 'clarity'             // Is this clear and understandable?
  | 'depth'               // Is this sufficiently thorough?
  | 'timeliness'          // Is this current/up-to-date?

  // Content appropriateness
  | 'appropriateness'     // Is this appropriate for the context?
  | 'sensitivity'         // Does this handle sensitive topics well?

  // Governance decisions
  | 'label-agreement'     // Do you agree with this label?
  | 'decision-agreement'  // Do you agree with this decision?
  | 'proposal-position'   // What's your position on this proposal?

  // Contributor evaluation
  | 'contribution-value'  // How valuable was this contribution?
  | 'trust-level'         // How much do you trust this contributor?

  // Elohim evaluation
  | 'elohim-helpfulness'  // Was this Elohim helpful?
  | 'elohim-accuracy'     // Was this Elohim's guidance accurate?
  | 'elohim-fairness';    // Was this Elohim fair?

/**
 * FeedbackOption - A single option in a graduated feedback selector.
 */
export interface FeedbackOption {
  id: string;
  value: number;  // Numeric value for aggregation
  label: string;
  description: string;
  icon?: string;
  color?: string;

  /** Is reasoning required for this option? */
  requiresReasoning: boolean;

  /** Prompt for reasoning if required */
  reasoningPrompt?: string;
}

/**
 * Pre-defined feedback scales based on context.
 */
export const FEEDBACK_SCALES: Record<FeedbackContext, FeedbackOption[]> = {
  // Accuracy scale (fact-checking)
  'accuracy': [
    { id: 'accurate', value: 2, label: 'Accurate', description: 'Verified and correct', icon: '✓', color: 'green', requiresReasoning: false },
    { id: 'mostly-accurate', value: 1, label: 'Mostly Accurate', description: 'Minor issues', icon: '~', color: 'light-green', requiresReasoning: true, reasoningPrompt: 'What needs correction?' },
    { id: 'uncertain', value: 0, label: 'Uncertain', description: 'Cannot verify', icon: '?', color: 'gray', requiresReasoning: false },
    { id: 'inaccurate', value: -1, label: 'Inaccurate', description: 'Contains errors', icon: '✗', color: 'orange', requiresReasoning: true, reasoningPrompt: 'What is incorrect?' },
    { id: 'false', value: -2, label: 'False', description: 'Significantly misleading', icon: '⚠', color: 'red', requiresReasoning: true, reasoningPrompt: 'Please provide evidence' },
  ],

  // Usefulness scale (personal value)
  'usefulness': [
    { id: 'transformative', value: 3, label: 'Transformative', description: 'Changed my understanding', icon: '★', color: 'gold', requiresReasoning: false },
    { id: 'very-useful', value: 2, label: 'Very Useful', description: 'Significantly helped', icon: '✓✓', color: 'green', requiresReasoning: false },
    { id: 'useful', value: 1, label: 'Useful', description: 'Somewhat helpful', icon: '✓', color: 'light-green', requiresReasoning: false },
    { id: 'neutral', value: 0, label: 'Neutral', description: 'Neither helpful nor unhelpful', icon: '−', color: 'gray', requiresReasoning: false },
    { id: 'not-useful', value: -1, label: 'Not Useful', description: 'Did not help me', icon: '✗', color: 'orange', requiresReasoning: true, reasoningPrompt: 'What were you looking for?' },
  ],

  // Proposal position (Loomio-style consent)
  'proposal-position': [
    { id: 'strongly-agree', value: 3, label: 'Strongly Agree', description: 'Fully support this', icon: '✓✓', color: 'green', requiresReasoning: false },
    { id: 'agree', value: 2, label: 'Agree', description: 'Support with minor concerns', icon: '✓', color: 'light-green', requiresReasoning: false },
    { id: 'abstain', value: 0, label: 'Abstain', description: 'No strong opinion', icon: '−', color: 'gray', requiresReasoning: false },
    { id: 'disagree', value: -1, label: 'Disagree', description: 'Have concerns', icon: '✗', color: 'orange', requiresReasoning: true, reasoningPrompt: 'What are your concerns?' },
    { id: 'block', value: -3, label: 'Block', description: 'Cannot proceed as-is', icon: '⊘', color: 'red', requiresReasoning: true, reasoningPrompt: 'Why must this be blocked?' },
  ],

  // Label agreement
  'label-agreement': [
    { id: 'agree', value: 1, label: 'Agree', description: 'Label is appropriate', icon: '✓', color: 'green', requiresReasoning: false },
    { id: 'unsure', value: 0, label: 'Unsure', description: 'Cannot determine', icon: '?', color: 'gray', requiresReasoning: false },
    { id: 'disagree', value: -1, label: 'Disagree', description: 'Label is inappropriate', icon: '✗', color: 'red', requiresReasoning: true, reasoningPrompt: 'Why is this label wrong?' },
  ],

  // Placeholder for other contexts - would be filled in similarly
  'clarity': [],
  'depth': [],
  'timeliness': [],
  'appropriateness': [],
  'sensitivity': [],
  'decision-agreement': [],
  'contribution-value': [],
  'trust-level': [],
  'elohim-helpfulness': [],
  'elohim-accuracy': [],
  'elohim-fairness': [],
};

export interface FeedbackResponse {
  optionId: string;
  value: number;
  reasoning?: string;
  respondedAt: string;
  responderId: string;
  /** Is this response visible to others? */
  public: boolean;
}

export interface FeedbackAggregateView {
  totalResponses: number;
  distribution: Array<{
    optionId: string;
    count: number;
    percentage: number;
  }>;

  /** Average value across all responses */
  averageValue: number;

  /** Consensus strength (0-1, 1 = unanimous) */
  consensusStrength: number;

  /** Are there bridging opportunities? (Polis-inspired) */
  bridgingOpportunities?: BridgingOpportunity[];
}

/**
 * BridgingOpportunity - Polis-inspired cross-group agreement detection.
 *
 * When opinion groups disagree on most things but agree on something,
 * that's a bridging opportunity worth surfacing.
 */
export interface BridgingOpportunity {
  description: string;
  groups: string[];
  agreementLevel: number;
  suggestedAction?: string;
}

// ============================================================================
// Deliberation Interface (Loomio-inspired Proposals)
// ============================================================================

/**
 * DeliberationProposal - A structured proposal for collective decision.
 *
 * Based on Loomio's proposal types:
 * - Advice: Seek input before deciding
 * - Consent: Proceed unless blocked
 * - Consensus: Seek full agreement
 * - Sense Check: Gauge sentiment before formalizing
 */
export interface DeliberationProposal {
  id: string;

  // Proposal type determines voting rules
  type: ProposalType;

  // *Target entity
  targetType: GovernableEntityType;
  targetId: string;

  /// *Proposal content
  title: string;
  description: string;
  proposedAction: string;

  // *Proposer info
  proposerId: string;
  proposedAt: string;

  // *Timeline
  discussionEndsAt?: string;
  votingEndsAt: string;

  // *Current phase
  phase: ProposalPhase;

  // *Discussion thread
  discussionThreadId: string;

  // *Voting configuration
  votingConfig: VotingConfiguration;

  // *Current results
  results?: ProposalResults;

  // *Outcome (when decided)
  outcome?: ProposalOutcome;
}

export type ProposalType =
  | 'advice'        // Seeking input, proposer decides
  | 'consent'       // Proceed unless objection
  | 'consensus'     // Seek unanimous agreement
  | 'sense-check'   // Gauge sentiment, non-binding
  | 'ranked-choice' // Multiple options, ranked preference
  | 'dot-vote'      // Allocate limited votes across options
  | 'score-vote';   // Score each option independently

export type ProposalPhase =
  | 'draft'         // Being written
  | 'discussion'    // Open for discussion before voting
  | 'voting'        // Voting open
  | 'closed'        // Voting closed, awaiting decision
  | 'decided'       // Outcome determined
  | 'implemented';  // Action taken

export interface VotingConfiguration {
  mechanism: VotingMechanism;

  /** Options to vote on (for multi-option proposals) */
  options?: Array<{
    id: string;
    label: string;
    description: string;
  }>;

  /** For dot voting: how many dots each voter gets */
  dotsPerVoter?: number;

  /** For score voting: min and max scores */
  scoreRange?: { min: number; max: number };

  /** Quorum requirement */
  quorumPercentage?: number;

  /** Threshold for passage */
  passageThreshold?: number;  // e.g., 0.66 for 2/3 majority
}

export interface ProposalResults {
  totalEligible: number;
  totalVoted: number;
  participationRate: number;
  quorumMet: boolean;

  // *query results by option
  optionResults: Array<{
    optionId: string;
    votes: number;
    percentage: number;
    score?: number;
    rank?: number;
  }>;

  // For consent: any blocks?
  blocks?: Array<{
    voterId: string;
    reason: string;
  }>;

  // Current recommendation based on results
  recommendation: 'pass' | 'fail' | 'unclear' | 'blocked';
}

export interface ProposalOutcome {
  decision: 'approved' | 'rejected' | 'modified' | 'withdrawn' | 'tabled';
  reasoning: string;
  decidedBy: string;
  decidedAt: string;
  actionsTriggered: string[];
  precedentCreated?: string;
}

// ============================================================================
// Sensemaking Visualization (Polis-inspired)
// ============================================================================

/**
 * SensemakingVisualization - AI-powered opinion clustering display.
 *
 * Inspired by Polis: shows where groups of opinion exist,
 * what divides them, and what bridges them.
 */
export interface SensemakingVisualization {
  // What's being sense-made
  targetType: GovernableEntityType;
  targetId: string;
  topic: string;

  // statistics
  participantCount: number;
  statementCount: number;
  voteCount: number;
  clusters: OpinionCluster[];

  // Cross-cluster consensus (bridging statements)
  consensusStatements: ConsensusStatement[];

  // Most divisive statements
  divisiveStatements: DivisiveStatement[];

  // Visualization data for rendering
  visualizationData: ClusterVisualizationData;
}

export interface OpinionCluster {
  id: string;
  name?: string;
  participantCount: number;
  participantPercentage: number;

  // What characterizes this cluster?
  characteristicStatements: Array<{
    statementId: string;
    text: string;
    agreementLevel: number;  // *How much this cluster agrees
  }>;

  // Centroid position for visualization
  centroid: { x: number; y: number };
}

export interface ConsensusStatement {
  id: string;
  text: string;
  overallAgreement: number;

  // *Agreement level per cluster
  clusterAgreement: Array<{
    clusterId: string;
    agreement: number;
  }>;

  bridgingPotential: string;  // Why this matters
}

export interface DivisiveStatement {
  id: string;
  text: string;
  variance: number;  // *How spread out the opinions are

  // How each cluster feels
  clusterPositions: Array<{
    clusterId: string;
    position: number;  // Range: -1 to 1
  }>;
}

export interface ClusterVisualizationData {
  projection: 'pca' | 'tsne' | 'umap';  // 2D projection of high-dimensional opinion space

  // Points for rendering
  points: Array<{
    participantId: string;  // Anonymous or identified
    x: number;
    y: number;
    clusterId: string;
  }>;

  // Cluster boundaries
  clusterBoundaries: Array<{
    clusterId: string;
    polygon: Array<{ x: number; y: number }>;
  }>;
}

// ============================================================================
// Governance History View (Wikipedia-inspired)
// ============================================================================

/**
 * GovernanceHistoryView - Model for displaying full audit trail.
 *
 * Like Wikipedia's combination of Edit History, Talk Page, and Protection Log.
 */
export interface GovernanceHistoryView {
  // Entity being viewed
  entityType: GovernableEntityType;
  entityId: string;
  entityTitle: string;
  activeTab: HistoryTab;

  // Tab-specific data
  tabs: {
    summary: HistorySummaryTab;
    versions: HistoryVersionsTab;
    discussions: HistoryDiscussionsTab;
    governance: HistoryGovernanceTab;
    engagement: HistoryEngagementTab;
  };
  filters: HistoryFilters;
  pagination: {
    currentPage: number;
    pageSize: number;
    totalItems: number;
  };
}

export type HistoryTab =
  | 'summary'      // Overview dashboard
  | 'versions'     // Edit history
  | 'discussions'  // Talk page
  | 'governance'   // Labels, challenges, appeals
  | 'engagement';  // Views, affinity, citations

export interface HistorySummaryTab {
  /** Quick stats */
  stats: {
    versions: number;
    contributors: number;
    discussions: number;
    challenges: number;
    daysSinceCreation: number;
  };

  /** Health indicators */
  health: {
    accuracy: 'verified' | 'disputed' | 'unknown';
    freshness: 'current' | 'needs-review' | 'stale';
    consensus: 'strong' | 'moderate' | 'contested';
  };

  /** Recent activity feed */
  recentActivity: GovernanceEvent[];

  /** Active items requiring attention */
  activeItems: {
    openDiscussions: number;
    pendingChallenges: number;
    activeVotes: number;
  };
}

export interface HistoryVersionsTab {
  versions: VersionRecord[];
  compareMode: boolean;
  selectedVersions?: [number, number];  // For diff view
}

export interface HistoryDiscussionsTab {
  threads: DiscussionThread[];
  newThreadForm?: {
    category: DiscussionCategory;
    topic: string;
    initialMessage: string;
  };
}

export interface HistoryGovernanceTab {
  activeLabels: GovernanceLabel[];

  // Label history
  labelHistory: Array<{
    label: GovernanceLabel;
    action: 'added' | 'removed' | 'challenged';
    timestamp: string;
    actor: string;
  }>;

  challenges: {
    active: Challenge[];
    resolved: Challenge[];
  };

  appeals: Appeal[];

  relevantPrecedents: Precedent[];
}

export interface HistoryEngagementTab {
  // Time series data
  timeSeries: {
    views: Array<{ date: string; count: number }>;
    affinityMarks: Array<{ date: string; count: number }>;
    citations: Array<{ date: string; count: number }>;
  };

  // Top engagers (anonymized if needed)
  topEngagers: Array<{
    engagerId: string;
    engagementScore: number;
    lastEngaged: string;
  }>;

  // Citation network
  citations: {
    citedBy: Array<{ entityId: string; entityTitle: string }>;
    cites: Array<{ entityId: string; entityTitle: string }>;
  };
}

export interface HistoryFilters {
  dateRange?: { start: string; end: string };
  eventTypes?: GovernanceEventType[];
  actors?: string[];
  searchText?: string;
}

// ============================================================================
// Route Patterns for Governance UI
// ============================================================================

/**
 * Governance routes follow the pattern:
 * /lamad/{entityType}:{entityId}/governance/{view}
 *
 * Examples:
 * /lamad/content:epic-social-medium/governance/summary
 * /lamad/content:epic-social-medium/governance/history
 * /lamad/content:epic-social-medium/governance/discussions
 * /lamad/content:epic-social-medium/governance/discussions/thread:123
 * /lamad/content:epic-social-medium/governance/challenges
 * /lamad/content:epic-social-medium/governance/challenges/new
 * /lamad/content:epic-social-medium/governance/proposals
 * /lamad/content:epic-social-medium/governance/proposals/new
 * /lamad/content:epic-social-medium/governance/sensemaking
 *
 * /lamad/human:agent-xyz/governance/summary
 * /lamad/elohim:community-guardian/governance/summary
 * /lamad/path:love-map-intro/governance/summary
 */
export const GOVERNANCE_ROUTES = {
  summary: (entityType: string, entityId: string) =>
    `/lamad/${entityType}:${entityId}/governance/summary`,

  history: (entityType: string, entityId: string) =>
    `/lamad/${entityType}:${entityId}/governance/history`,

  discussions: (entityType: string, entityId: string) =>
    `/lamad/${entityType}:${entityId}/governance/discussions`,

  discussionThread: (entityType: string, entityId: string, threadId: string) =>
    `/lamad/${entityType}:${entityId}/governance/discussions/thread:${threadId}`,

  challenges: (entityType: string, entityId: string) =>
    `/lamad/${entityType}:${entityId}/governance/challenges`,

  newChallenge: (entityType: string, entityId: string) =>
    `/lamad/${entityType}:${entityId}/governance/challenges/new`,

  proposals: (entityType: string, entityId: string) =>
    `/lamad/${entityType}:${entityId}/governance/proposals`,

  newProposal: (entityType: string, entityId: string) =>
    `/lamad/${entityType}:${entityId}/governance/proposals/new`,

  sensemaking: (entityType: string, entityId: string) =>
    `/lamad/${entityType}:${entityId}/governance/sensemaking`,

  precedents: () =>
    `/lamad/governance/precedents`,

  precedent: (precedentId: string) =>
    `/lamad/governance/precedents/${precedentId}`,
};

// ============================================================================
// Service Interface for Governance Deliberation
// ============================================================================

/**
 * GovernanceDeliberationService - Service contract for governance deliberation components.
 */
export interface GovernanceDeliberationService {
  // *Context menu
  getContextMenu(entityType: string, entityId: string): Promise<GovernanceContextMenu>;

  // *Feedback
  getFeedbackSelector(entityType: string, entityId: string, context: FeedbackContext): Promise<GraduatedFeedbackSelector>;
  submitFeedback(entityType: string, entityId: string, context: FeedbackContext, response: FeedbackResponse): Promise<void>;
  getFeedbackAggregate(entityType: string, entityId: string, context: FeedbackContext): Promise<FeedbackAggregateView>;

  // *Deliberation
  getProposals(entityType: string, entityId: string, filters?: { phase?: ProposalPhase }): Promise<DeliberationProposal[]>;
  getProposal(proposalId: string): Promise<DeliberationProposal>;
  createProposal(proposal: Omit<DeliberationProposal, 'id' | 'proposedAt' | 'phase' | 'results'>): Promise<DeliberationProposal>;
  voteOnProposal(proposalId: string, vote: unknown): Promise<void>;

  // *Sensemaking
  getSensemakingVisualization(entityType: string, entityId: string): Promise<SensemakingVisualization>;
  submitStatement(entityType: string, entityId: string, statement: string): Promise<string>;
  voteOnStatement(statementId: string, vote: 'agree' | 'disagree' | 'pass'): Promise<void>;

  // *History
  getHistoryView(entityType: string, entityId: string, tab: HistoryTab, filters?: HistoryFilters): Promise<GovernanceHistoryView>;

  // *Discussions
  getDiscussionThread(threadId: string): Promise<DiscussionThread>;
  createDiscussionThread(entityType: string, entityId: string, category: DiscussionCategory, topic: string, initialMessage: string): Promise<DiscussionThread>;
  postMessage(threadId: string, message: Omit<DiscussionMessage, 'id' | 'timestamp' | 'reactions' | 'edited' | 'hidden'>): Promise<DiscussionMessage>;

  // *Challenges
  fileChallenge(challenge: Omit<Challenge, 'id' | 'filedAt' | 'state'>): Promise<Challenge>;
  respondToChallenge(challengeId: string, response: unknown): Promise<void>;

  // *Subscriptions
  subscribeToEntity(entityType: string, entityId: string, events: AlertType[]): Promise<void>;
  unsubscribeFromEntity(entityType: string, entityId: string): Promise<void>;
}
