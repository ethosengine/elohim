/**
 * Expertise Discovery Model - Find experts in any domain.
 *
 * The mastery graph naturally reveals expertise. This model supports
 * queries like "Who's the best at X?" without explicit reputation scores.
 *
 * Key insight: Expertise is emergent from actual learning and contribution
 * activity, not a gamified score that can be gamed.
 *
 * Use cases:
 * - "Who should review my contribution?"
 * - "Who can mentor me in X?"
 * - "Who are the active stewards of this content?"
 * - "Who's on the shortlist for X?"
 */

import { MasteryLevel } from './agent.model';

/**
 * ExpertiseQuery - Parameters for finding experts.
 */
export interface ExpertiseQuery {
  /** Content node IDs or tag patterns to search */
  domains: string[];

  /** Minimum mastery level required */
  minLevel?: MasteryLevel;

  /** Minimum freshness required (0.0-1.0) */
  minFreshness?: number;

  /** Require active contributions? */
  requireContributions?: boolean;

  /** Minimum contribution count if requiring contributions */
  minContributions?: number;

  /** Time window for "active" consideration */
  activeWithin?: string; // ISO 8601 duration

  /** Limit results */
  limit?: number;

  /** Exclude specific humans (e.g., self) */
  exclude?: string[];

  /** Sort order */
  sortBy?: ExpertiseSortField;
  sortDirection?: 'asc' | 'desc';
}

export type ExpertiseSortField =
  | 'expertiseScore' // Composite score
  | 'masteryLevel' // Highest mastery level
  | 'freshness' // Most recent engagement
  | 'contributions' // Most contributions
  | 'peerRecognition' // Best peer reviews
  | 'domainBreadth'; // Most domains covered

/**
 * ExpertCandidate - A human with expertise in queried domains.
 */
export interface ExpertCandidate {
  humanId: string;
  displayName: string;

  /** Avatar URL if available */
  avatarUrl?: string;

  /** Composite expertise score (0.0-1.0) */
  expertiseScore: number;

  /** Breakdown of score components */
  scoreBreakdown: ExpertiseScoreBreakdown;

  /** Content nodes they're expert in (within queried domains) */
  expertiseNodes: ExpertiseNodeSummary[];

  /** How available are they? (if they've indicated) */
  availability?: ExpertAvailability;

  /** Last activity in this domain */
  lastDomainActivity: string;

  /** How long they've been active in this domain */
  domainTenure?: string; // ISO 8601 duration
}

export interface ExpertiseScoreBreakdown {
  /** Average mastery level across domains (normalized 0-1) */
  masteryLevel: number;

  /** Average freshness across domains */
  freshness: number;

  /** Contribution activity score (normalized 0-1) */
  contributionActivity: number;

  /** Peer recognition score (average peer review ratings) */
  peerRecognition: number;

  /** Domain breadth (% of queried domains covered) */
  domainBreadth: number;

  /** Teaching/mentoring activity score */
  teachingActivity?: number;
}

export interface ExpertiseNodeSummary {
  contentId: string;
  contentTitle: string;
  level: MasteryLevel;
  freshness: number;
  contributionCount: number;
  lastEngagement: string;
}

export type ExpertAvailability =
  | 'available' // Open to mentorship/review requests
  | 'limited' // Available with constraints
  | 'unavailable'; // Not taking requests

/**
 * ExpertLeaderboard - Rankings for a content domain.
 */
export interface ExpertLeaderboard {
  /** Domain identifier or description */
  domain: string;

  /** Content node IDs included in this domain */
  domainContentIds: string[];

  /** When this leaderboard was computed */
  generatedAt: string;

  /** Time window for activity-based rankings */
  timeWindow: string; // ISO 8601 duration

  /** Top experts by composite expertise score */
  topExperts: ExpertCandidate[];

  /** Most active contributors (create level activity) */
  mostActive: ExpertCandidate[];

  /** Rising experts (steep mastery velocity) */
  rising: ExpertCandidate[];

  /** Most helpful (peer review and mentoring activity) */
  mostHelpful: ExpertCandidate[];

  /** Domain statistics */
  stats: DomainExpertiseStats;
}

export interface DomainExpertiseStats {
  /** Total humans with any mastery in this domain */
  totalLearners: number;

  /** Humans at or above attestation gate */
  expertsAboveGate: number;

  /** Humans at create level */
  creators: number;

  /** Average mastery level (numeric) */
  averageLevel: number;

  /** Average freshness across all mastery */
  averageFreshness: number;

  /** Total contributions in this domain */
  totalContributions: number;
}

/**
 * LeaderboardOptions - Configuration for leaderboard generation.
 */
export interface LeaderboardOptions {
  /** How many in each category */
  limit?: number;

  /** Time window for "active" and "rising" */
  timeWindow?: string; // ISO 8601 duration

  /** Include only fresh mastery? */
  freshOnly?: boolean;

  /** Minimum freshness threshold */
  minFreshness?: number;

  /** Include humans who opted out of leaderboards? */
  includeOptedOut?: boolean;
}

// =========================================================================
// Expert Matching (for specific use cases)
// =========================================================================

/**
 * ReviewerMatch - Finding reviewers for a contribution.
 */
export interface ReviewerMatchQuery {
  /** Content node being contributed to */
  contentId: string;

  /** Type of contribution (affects reviewer requirements) */
  contributionType: 'comment' | 'edit' | 'derivative' | 'original';

  /** Author to exclude */
  authorId: string;

  /** Prefer reviewers who've reviewed similar content */
  preferExperience?: boolean;

  /** Number of reviewers needed */
  reviewersNeeded: number;
}

export interface ReviewerMatch {
  candidate: ExpertCandidate;

  /** Fit score for this specific review task */
  fitScore: number;

  /** Why they're a good fit */
  fitReasons: string[];

  /** Their review history for this content type */
  reviewHistory?: {
    totalReviews: number;
    averageRating: number;
    lastReview: string;
  };
}

/**
 * MentorMatch - Finding mentors for a learner.
 */
export interface MentorMatchQuery {
  /** Learner's ID */
  learnerId: string;

  /** Content nodes the learner is working on */
  learningContentIds: string[];

  /** Learner's current levels for context */
  currentLevels: Record<string, MasteryLevel>;

  /** Prefer mentors with teaching experience */
  preferTeachers?: boolean;

  /** Number of mentor suggestions */
  limit?: number;
}

export interface MentorMatch {
  candidate: ExpertCandidate;

  /** Fit score for this mentoring relationship */
  fitScore: number;

  /** Why they're a good fit */
  fitReasons: string[];

  /** How many levels ahead they are */
  levelAdvantage: number;

  /** Their mentoring history */
  mentoringHistory?: {
    menteeCount: number;
    averageProgress: number; // How much mentees advance
    lastActive: string;
  };
}

// =========================================================================
// Privacy & Visibility
// =========================================================================

/**
 * ExpertiseVisibility - Human's preferences for expertise discovery.
 *
 * Expertise is powerful information - humans control how discoverable
 * they are in the expertise graph.
 */
export interface ExpertiseVisibility {
  humanId: string;

  /** Can others see my expertise? */
  discoverability: ExpertDiscoverability;

  /** Which domains can I be found in? */
  visibleDomains: string[] | 'all' | 'none';

  /** Hidden domains (override for 'all') */
  hiddenDomains?: string[];

  /** Am I available for mentorship? */
  mentorshipAvailable: boolean;

  /** Mentorship preferences if available */
  mentorshipPreferences?: MentorshipPreferences;

  /** Am I available for reviews? */
  reviewAvailable: boolean;

  /** Can I appear on leaderboards? */
  leaderboardOptIn: boolean;

  /** Can my expertise be used for expert routing? */
  expertRoutingOptIn: boolean;

  /** Last updated */
  updatedAt: string;
}

export type ExpertDiscoverability =
  | 'public' // Anyone can find me
  | 'network' // Only my connections
  | 'community' // Only my community members
  | 'private'; // Only explicit grants

export interface MentorshipPreferences {
  /** Maximum mentees at once */
  maxMentees?: number;

  /** Current mentee count */
  currentMentees?: number;

  /** Preferred domains for mentoring */
  preferredDomains?: string[];

  /** Time commitment available */
  availability?: 'minimal' | 'moderate' | 'significant';

  /** Preferred communication method */
  preferredContact?: 'in-app' | 'email' | 'external';
}

// =========================================================================
// Expert Activity Tracking
// =========================================================================

/**
 * ExpertActivity - Track expert engagement for freshness and ranking.
 */
export interface ExpertActivity {
  humanId: string;
  contentId: string;
  activityType: ExpertActivityType;
  timestamp: string;

  /** Details about the activity */
  details?: Record<string, unknown>;
}

export type ExpertActivityType =
  | 'mastery_achieved' // Reached a new mastery level
  | 'contribution_made' // Made a contribution
  | 'review_given' // Gave a peer review
  | 'review_received' // Received a peer review
  | 'mentoring_started' // Started mentoring someone
  | 'mentoring_completed' // Completed a mentoring relationship
  | 'question_answered' // Answered a question in domain
  | 'content_refreshed'; // Refreshed stale mastery

/**
 * MasteryVelocity - Rate of mastery advancement.
 *
 * Used to identify "rising experts" - those advancing quickly.
 */
export interface MasteryVelocity {
  humanId: string;
  contentId: string;

  /** Time to reach current level from previous */
  timeToCurrentLevel: string; // ISO 8601 duration

  /** Average time per level in this domain */
  averageTimePerLevel: string;

  /** Percentile compared to other learners */
  velocityPercentile: number;

  /** Trend: accelerating, steady, or decelerating */
  trend: 'accelerating' | 'steady' | 'decelerating';
}
