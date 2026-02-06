/**
 * Collective Research Model - Multi-Participant Research Coordination
 *
 * Qahal handles collective/relational research paradigms:
 * - Dyadic studies (couples, mentor-mentee, parent-child)
 * - Group assessments (teams, circles, cohorts)
 * - Network studies (social capital, community cohesion)
 * - Deliberative research (before/after deliberation)
 * - Research pools (consent-based data aggregation)
 *
 * For individual assessment instruments:
 * @see @app/lamad/quiz-engine/models/research-assessment.model.ts
 *
 * For governance deliberation (Loomio/Polis-style):
 * @see ./governance-deliberation.model.ts
 *
 * Key principles:
 * - Multi-participant consent coordination
 * - Actor-Partner Interdependence Model (APIM) for dyadic data
 * - Research protocol governance (IRB-style oversight)
 * - Privacy-preserving aggregation
 *
 * References:
 * - Dyadic Data Analysis: Kenny, Kashy & Cook
 * - Deliberative Polling: James Fishkin, Stanford
 * - Polis: Computational Democracy Project
 * - Experience Sampling: Csikszentmihalyi & Larson
 */

import type {
  ResearchFramework,
  PersonalResearchConsent,
} from '@app/lamad/quiz-engine/models/research-assessment.model';

// =============================================================================
// Unit of Analysis
// =============================================================================

/**
 * The unit of analysis for collective research.
 */
export type CollectiveUnit =
  | 'dyad' // Pair of people (couples, mentor-mentee, etc.)
  | 'small-group' // 3-12 people (teams, families, cohorts)
  | 'large-group' // 13-100 people (classrooms, organizations)
  | 'network' // Social network/community
  | 'population'; // Aggregate across many individuals/groups

// =============================================================================
// Dyadic Research
// =============================================================================

/**
 * The relationship type within a dyad.
 * Determines distinguishability for APIM analysis.
 */
export type DyadType =
  | 'romantic-partner' // Couples (distinguishable by role or indistinguishable)
  | 'parent-child' // Parent-child dyads (distinguishable)
  | 'mentor-mentee' // Mentorship relationships (distinguishable)
  | 'sibling' // Sibling pairs (distinguishable by birth order or indistinguishable)
  | 'coworker' // Workplace pairs
  | 'friend' // Friendship dyads (typically indistinguishable)
  | 'caregiver-recipient' // Care relationships (distinguishable)
  | 'teacher-student' // Educational dyads (distinguishable)
  | 'accountability' // Accountability partners
  | 'matched-control'; // Experimentally matched pairs

/**
 * Whether dyad members can be distinguished by a non-arbitrary variable.
 * Affects statistical analysis methods (Pearson vs intraclass correlation).
 */
export type Distinguishability =
  | 'distinguishable' // Members have distinct roles (e.g., parent vs child)
  | 'indistinguishable' // Members cannot be naturally distinguished
  | 'mixed'; // Some aspects distinguishable, others not

/**
 * Formation method for a dyad
 */
export type DyadFormationMethod = 'mutual-invitation' | 'platform-matched' | 'researcher-assigned';

/**
 * Dyad status lifecycle
 */
export type DyadStatus = 'forming' | 'active' | 'paused' | 'dissolved';

/**
 * Reason for dyad dissolution
 */
export type DyadDissolutionReason =
  | 'completed'
  | 'withdrawn'
  | 'one-member-left'
  | 'researcher-ended';

/**
 * Consent status in a dyad
 */
export type ConsentStatus = 'pending' | 'consented' | 'declined' | 'withdrawn';

/**
 * Data visibility to partner
 */
export type PartnerVisibility = 'none' | 'aggregate' | 'full';

/**
 * A dyad (paired participants) in the research system.
 */
export interface ResearchDyad {
  /** Unique dyad ID */
  id: string;

  /** Type of dyad relationship */
  type: DyadType;

  /** Distinguishability */
  distinguishability: Distinguishability;

  /** Member 1 */
  member1: DyadMember;

  /** Member 2 */
  member2: DyadMember;

  /** When the dyad was formed */
  formedAt: string;

  /** How the dyad was formed */
  formationMethod: DyadFormationMethod;

  /** Studies this dyad is participating in */
  activeStudies: string[];

  /** Dyad status */
  status: DyadStatus;

  /** Dissolution reason (if dissolved) */
  dissolutionReason?: DyadDissolutionReason;
}

/**
 * A member of a dyad.
 */
export interface DyadMember {
  /** Human ID */
  humanId: string;

  /** Role in dyad (for distinguishable dyads) */
  role?: string;

  /** Consent status for this dyad */
  consentStatus: ConsentStatus;

  /** When they joined */
  joinedAt?: string;

  /** Visibility of their data to partner */
  partnerVisibility: PartnerVisibility;
}

/**
 * A dyadic assessment instrument.
 * Questions may reference self, partner, or relationship.
 */
export interface DyadicAssessment {
  /** Assessment ID */
  id: string;

  /** Study ID */
  studyId: string;

  /** Title */
  title: string;

  /** Description */
  description: string;

  /** Framework */
  framework: ResearchFramework;

  /** Type of dyad */
  dyadType: DyadType;

  /** Distinguishability */
  distinguishability: Distinguishability;

  /** Common questions for both members */
  commonQuestions: DyadicQuestion[];

  /** Role-specific questions (for distinguishable dyads) */
  roleSpecificQuestions?: {
    role1: { label: string; questions: DyadicQuestion[] };
    role2: { label: string; questions: DyadicQuestion[] };
  };

  /** Whether partner must complete for data to be valid */
  partnerRequired: boolean;

  /** Order of completion */
  completionOrder: 'simultaneous' | 'sequential' | 'any';

  /** Analysis model */
  analysisModel: DyadicAnalysisModel;
}

/**
 * A question in a dyadic assessment.
 */
export interface DyadicQuestion {
  /** Question ID */
  id: string;

  /** Question text (may include {partner} placeholder) */
  text: string;

  /** What the question references */
  reference: 'self' | 'partner' | 'relationship' | 'both';

  /** Question type */
  type: 'likert' | 'slider' | 'multiple-choice' | 'ranking' | 'open-ended';

  /** Scale configuration */
  scale?: {
    min: number;
    max: number;
    minLabel: string;
    maxLabel: string;
  };

  /** For APIM: actor or partner effect predictor */
  apimRole?: 'actor' | 'partner' | 'dyadic';

  /** Constructs measured */
  measures: string[];
}

/**
 * Analysis model for dyadic data.
 */
export type DyadicAnalysisModel =
  | 'apim' // Actor-Partner Interdependence Model
  | 'common-fate' // Both members affected by shared factors
  | 'mutual-influence' // Bidirectional influence
  | 'actor-only'; // No partner effects expected

/**
 * Result from a dyadic assessment.
 */
export interface DyadicResult {
  /** Result ID */
  id: string;

  /** Study ID */
  studyId: string;

  /** Assessment ID */
  assessmentId: string;

  /** Dyad ID */
  dyadId: string;

  /** Member 1 responses */
  member1: DyadMemberResponse;

  /** Member 2 responses */
  member2: DyadMemberResponse;

  /** Relationship-level computed measures */
  relationshipMeasures?: Record<string, number>;

  /** Nonindependence (correlation between members) */
  nonindependence: number;

  /** Status */
  status: 'partial' | 'complete';

  /** Completed when both finish */
  completedAt?: string;
}

/**
 * Individual member's responses in a dyadic assessment.
 */
export interface DyadMemberResponse {
  /** Human ID (pseudonymized for research) */
  humanId: string;

  /** Role in dyad */
  role?: string;

  /** When completed */
  completedAt: string;

  /** Subscale scores */
  subscaleScores: Record<string, number>;

  /** Item-level responses */
  itemResponses?: Record<string, unknown>;
}

// =============================================================================
// Group Research
// =============================================================================

/**
 * A research group (team, circle, cohort).
 */
export interface ResearchGroup {
  /** Group ID */
  id: string;

  /** Group name */
  name: string;

  /** Group type */
  type: 'team' | 'circle' | 'cohort' | 'classroom' | 'organization' | 'community';

  /** Members */
  members: GroupMember[];

  /** Minimum/maximum size */
  sizeConstraints: { min: number; max: number };

  /** When formed */
  formedAt: string;

  /** Active studies */
  activeStudies: string[];

  /** Status */
  status: 'forming' | 'active' | 'paused' | 'dissolved';

  /** Facilitator/leader (if any) */
  facilitatorId?: string;
}

/**
 * A member of a research group.
 */
export interface GroupMember {
  /** Human ID */
  humanId: string;

  /** Role in group */
  role?: string;

  /** When joined */
  joinedAt: string;

  /** Consent status */
  consentStatus: ConsentStatus;

  /** Active participant? */
  active: boolean;
}

/**
 * A group assessment instrument.
 */
export interface GroupAssessment {
  /** Assessment ID */
  id: string;

  /** Study ID */
  studyId: string;

  /** Title */
  title: string;

  /** Description */
  description: string;

  /** Framework */
  framework: ResearchFramework;

  /** Size constraints */
  groupSize: { min: number; max: number };

  /** Individual member questions */
  memberQuestions: GroupQuestion[];

  /** Group-level questions */
  groupQuestions?: GroupLevelQuestion[];

  /** How to aggregate individual responses */
  aggregation: AggregationConfig;

  /** Whether deliberation is part of assessment */
  includesDeliberation: boolean;

  /** Deliberation config if included */
  deliberation?: DeliberativeResearchConfig;
}

/**
 * A question for individual group members.
 */
export interface GroupQuestion {
  /** Question ID */
  id: string;

  /** Question text */
  text: string;

  /** Reference point */
  reference: 'self' | 'group' | 'specific-member' | 'task' | 'leader';

  /** Type */
  type: 'likert' | 'slider' | 'ranking' | 'allocation' | 'open-ended' | 'sociometric';

  /** For sociometric: relationship type being measured */
  sociometricType?: 'trust' | 'influence' | 'communication' | 'friendship' | 'advice';

  /** Scale configuration */
  scale?: { min: number; max: number; minLabel: string; maxLabel: string };

  /** Constructs measured */
  measures: string[];
}

/**
 * A group-level question.
 */
export interface GroupLevelQuestion {
  /** Question ID */
  id: string;

  /** Question text */
  text: string;

  /** Who answers */
  answeredBy: 'facilitator' | 'consensus' | 'majority-vote' | 'designated-reporter' | 'all-agree';

  /** Type */
  type: 'likert' | 'open-ended' | 'ranking' | 'collective-response';
}

/**
 * Configuration for aggregating individual responses to group level.
 */
export interface AggregationConfig {
  /** Statistical method */
  method: 'mean' | 'median' | 'consensus' | 'variance' | 'network-centrality' | 'custom';

  /** For consensus: agreement threshold */
  consensusThreshold?: number;

  /** Compute within-group agreement (rwg) */
  computeAgreement: boolean;

  /** Compute intraclass correlation (ICC) */
  computeICC: boolean;

  /** Minimum response rate for valid aggregation */
  minimumResponseRate: number;
}

/**
 * Result from a group assessment.
 */
export interface GroupResult {
  /** Result ID */
  id: string;

  /** Study ID */
  studyId: string;

  /** Assessment ID */
  assessmentId: string;

  /** Group ID */
  groupId: string;

  /** Group size at time of assessment */
  groupSize: number;

  /** Response rate */
  responseRate: number;

  /** Individual member responses */
  memberResponses: GroupMemberResponse[];

  /** Aggregated group scores */
  groupScores: Record<string, number>;

  /** Within-group agreement (rwg) */
  agreement?: Record<string, number>;

  /** Intraclass correlation (ICC) */
  icc?: Record<string, number>;

  /** Sociometric network (if collected) */
  sociometricNetwork?: SociometricNetwork;

  /** Deliberation outcomes (if applicable) */
  deliberationOutcomes?: DeliberativeOutcome;

  /** Completed timestamp */
  completedAt: string;
}

/**
 * Individual member response in group assessment.
 */
export interface GroupMemberResponse {
  /** Human ID (pseudonymized) */
  humanId: string;

  /** Role in group */
  role?: string;

  /** When completed */
  completedAt: string;

  /** Subscale scores */
  subscaleScores: Record<string, number>;

  /** Pre-deliberation scores (if applicable) */
  preDeliberationScores?: Record<string, number>;

  /** Post-deliberation scores (if applicable) */
  postDeliberationScores?: Record<string, number>;
}

/**
 * Sociometric network captured from group assessment.
 */
export interface SociometricNetwork {
  /** Relationship type */
  relationshipType: string;

  /** Nodes (members) */
  nodes: { id: string; role?: string }[];

  /** Edges (relationships) */
  edges: SociometricEdge[];

  /** Network-level metrics */
  metrics: {
    density: number;
    centralization: number;
    reciprocity: number;
  };
}

/**
 * An edge in a sociometric network.
 */
export interface SociometricEdge {
  /** Source node */
  source: string;

  /** Target node */
  target: string;

  /** Edge weight */
  weight: number;

  /** Is relationship reciprocated? */
  reciprocated?: boolean;
}

// =============================================================================
// Deliberative Research
// =============================================================================

/**
 * Configuration for deliberative research (before/after studies).
 */
export interface DeliberativeResearchConfig {
  /** Pre-deliberation survey */
  preSurvey: boolean;

  /** Post-deliberation survey */
  postSurvey: boolean;

  /** Type of deliberation */
  deliberationType: 'small-group' | 'plenary' | 'online-async' | 'ai-moderated' | 'polis-style';

  /** Duration */
  durationMinutes: number;

  /** Balanced briefing materials provided? */
  balancedBriefing: boolean;

  /** Expert Q&A included? */
  expertQA: boolean;

  /** Moderation style */
  moderation: 'facilitator' | 'peer' | 'ai-assisted' | 'unmoderated';

  /** Capture bridging statements (Polis-style)? */
  captureBridging: boolean;
}

/**
 * Outcome from deliberative research.
 */
export interface DeliberativeOutcome {
  /** Opinion change magnitude (average) */
  opinionChangeMagnitude: number;

  /** Direction of change */
  opinionChangeDirection: 'polarized' | 'convergent' | 'mixed' | 'stable';

  /** Knowledge gain (if measured) */
  knowledgeGain?: number;

  /** Deliberation quality score */
  qualityScore?: number;

  /** Bridging statements identified */
  bridgingStatements?: BridgingStatement[];

  /** Consensus items */
  consensusItems?: ConsensusItem[];

  /** Opinion clusters (Polis-style) */
  clusters?: OpinionCluster[];
}

/**
 * A bridging statement that crosses group divides.
 */
export interface BridgingStatement {
  /** Statement text */
  text: string;

  /** Overall agreement */
  overallAgreement: number;

  /** Agreement across clusters */
  clusterAgreement: { clusterId: string; agreement: number }[];

  /** Why this bridges divides */
  bridgingAnalysis: string;
}

/**
 * An item where consensus was reached.
 */
export interface ConsensusItem {
  /** Item/statement */
  text: string;

  /** Agreement level */
  agreementLevel: number;

  /** Pre-deliberation level (if measured) */
  preDeliberationLevel?: number;

  /** Change from pre to post */
  change?: number;
}

/**
 * An opinion cluster from Polis-style analysis.
 */
export interface OpinionCluster {
  /** Cluster ID */
  id: string;

  /** Cluster name (if labeled) */
  name?: string;

  /** Number of participants */
  participantCount: number;

  /** Characteristic statements */
  characteristicStatements: { text: string; agreementLevel: number }[];

  /** Centroid position for visualization */
  centroid: { x: number; y: number };
}

// =============================================================================
// Experience Sampling Coordination
// =============================================================================

/**
 * Coordinated ESM study across multiple participants.
 */
export interface ESMStudy {
  /** Study ID */
  id: string;

  /** Title */
  title: string;

  /** Description */
  description: string;

  /** Study duration (days) */
  durationDays: number;

  /** Sampling strategy */
  strategy: 'signal-contingent' | 'event-contingent' | 'interval-contingent';

  /** Signals per day (for signal-contingent) */
  signalsPerDay?: number;

  /** Interval (for interval-contingent) */
  intervalMinutes?: number;

  /** Response window (minutes) */
  responseWindowMinutes: number;

  /** Prompts in the study */
  prompts: ESMStudyPrompt[];

  /** Enrolled participants */
  participants: ESMParticipant[];

  /** Status */
  status: 'recruiting' | 'active' | 'completed' | 'archived';

  /** Start and end dates */
  startDate?: string;
  endDate?: string;
}

/**
 * A prompt in an ESM study.
 */
export interface ESMStudyPrompt {
  /** Prompt ID */
  id: string;

  /** Prompt text */
  text: string;

  /** Type */
  type: 'likert' | 'slider' | 'multiple-choice' | 'open-ended' | 'photo';

  /** Scale */
  scale?: { min: number; max: number; minLabel: string; maxLabel: string };

  /** Required? */
  required: boolean;

  /** Estimated seconds to answer */
  estimatedSeconds: number;

  /** Constructs measured */
  measures: string[];

  /** Condition for showing (optional) */
  condition?: ESMCondition;
}

/**
 * Condition for showing an ESM prompt.
 */
export interface ESMCondition {
  /** Condition type */
  type: 'time-of-day' | 'location' | 'activity' | 'previous-response' | 'day-of-week';

  /** Operator */
  operator: 'equals' | 'in' | 'not-in' | 'greater-than' | 'less-than';

  /** Value */
  value: unknown;
}

/**
 * A participant in an ESM study.
 */
export interface ESMParticipant {
  /** Human ID */
  humanId: string;

  /** Enrollment date */
  enrolledAt: string;

  /** Consent status */
  consentStatus: 'pending' | 'consented' | 'declined' | 'withdrawn';

  /** Response rate so far */
  responseRate: number;

  /** Prompts delivered */
  promptsDelivered: number;

  /** Prompts completed */
  promptsCompleted: number;

  /** Status */
  status: 'active' | 'paused' | 'completed' | 'withdrawn';
}

// =============================================================================
// Network/Community Research
// =============================================================================

/**
 * A network study in a community.
 */
export interface NetworkStudy {
  /** Study ID */
  id: string;

  /** Title */
  title: string;

  /** Network type */
  type: 'ego-network' | 'full-network' | 'affiliation' | 'two-mode';

  /** Relationship types being measured */
  relationshipTypes: NetworkRelationType[];

  /** Name generators (for ego-network) */
  nameGenerators?: NameGenerator[];

  /** Name interpreters */
  nameInterpreters?: NameInterpreter[];

  /** Position generator (for social capital) */
  positionGenerator?: PositionGenerator;

  /** Community/population scope */
  scope: 'community' | 'organization' | 'open';

  /** Scope ID (community or org ID) */
  scopeId?: string;

  /** Participants */
  participants: NetworkParticipant[];

  /** Status */
  status: 'recruiting' | 'active' | 'completed' | 'archived';
}

/**
 * Type of relationship in network study.
 */
export interface NetworkRelationType {
  /** Type ID */
  id: string;

  /** Label */
  label: string;

  /** Directed? */
  directed: boolean;

  /** Strength measurement */
  strength: 'binary' | 'ordinal' | 'continuous';

  /** Valence */
  valence: 'positive' | 'negative' | 'neutral' | 'mixed';
}

/**
 * Name generator for ego-network studies.
 */
export interface NameGenerator {
  /** Generator ID */
  id: string;

  /** Prompt */
  prompt: string;

  /** Max names to elicit */
  maxNames: number;

  /** Relationship type generated */
  relationshipType: string;
}

/**
 * Name interpreter for collecting alter attributes.
 */
export interface NameInterpreter {
  /** Interpreter ID */
  id: string;

  /** Prompt about the alter */
  prompt: string;

  /** Response type */
  type: 'categorical' | 'ordinal' | 'open-ended';

  /** Options for categorical */
  options?: string[];

  /** Attribute measured */
  attribute: string;
}

/**
 * Position generator for social capital.
 */
export interface PositionGenerator {
  /** Positions/occupations */
  positions: { id: string; label: string; prestige: number }[];

  /** Access types to measure */
  accessTypes: ('know' | 'family' | 'friend' | 'acquaintance')[];
}

/**
 * Participant in a network study.
 */
export interface NetworkParticipant {
  /** Human ID */
  humanId: string;

  /** Enrolled date */
  enrolledAt: string;

  /** Consent status */
  consentStatus: 'pending' | 'consented' | 'declined' | 'withdrawn';

  /** Completed data collection? */
  completed: boolean;
}

// =============================================================================
// Research Data Pool
// =============================================================================

/**
 * A research data pool for consent-based data aggregation.
 */
export interface ResearchDataPool {
  /** Pool ID */
  id: string;

  /** Pool name */
  name: string;

  /** Description */
  description: string;

  /** Purpose */
  purpose: string;

  /** Governance */
  governance: DataPoolGovernance;

  /** Access rules */
  accessRules: DataPoolAccessRule[];

  /** Included assessment types */
  includedAssessments: string[];

  /** Minimum consent level for inclusion */
  minimumConsent: PersonalResearchConsent;

  /** Current contributor count */
  contributorCount: number;

  /** Data quality metrics */
  qualityMetrics: DataQualityMetrics;

  /** When created */
  createdAt: string;

  /** Status */
  status: 'accepting' | 'frozen' | 'archived';
}

/**
 * Governance model for a data pool.
 */
export interface DataPoolGovernance {
  /** Governance type */
  type: 'community' | 'institutional' | 'consortium' | 'open';

  /** Steward */
  steward: string;

  /** Review board for access requests */
  reviewBoard?: string[];

  /** Voting mechanism */
  votingMechanism?: 'majority' | 'consensus' | 'steward-decision';

  /** Data use agreement required? */
  dataUseAgreementRequired: boolean;
}

/**
 * Access rule for a data pool.
 */
export interface DataPoolAccessRule {
  /** Who this applies to */
  applicableTo: 'contributors' | 'approved-researchers' | 'public';

  /** Data access level */
  dataAccess: 'aggregate-only' | 'pseudonymized' | 'identified';

  /** Purpose restrictions */
  purposeRestrictions?: string[];

  /** Required approvals */
  requiredApprovals?: string[];
}

/**
 * Data quality metrics for a pool.
 */
export interface DataQualityMetrics {
  /** Completeness rate */
  completenessRate: number;

  /** Validity rate */
  validityRate: number;

  /** Average completion time */
  averageCompletionTimeSeconds: number;

  /** Careless response rate (detected) */
  carelessResponseRate: number;
}

// =============================================================================
// Research Study Protocol
// =============================================================================

/**
 * A research study protocol (IRB-style governance).
 */
export interface ResearchStudy {
  /** Study ID */
  id: string;

  /** Title */
  title: string;

  /** Principal investigators */
  investigators: Investigator[];

  /** Description */
  description: string;

  /** Research questions */
  researchQuestions: string[];

  /** Unit of analysis */
  unit: CollectiveUnit | 'individual';

  /** Target sample size */
  targetSampleSize: number;

  /** Eligibility criteria */
  eligibility: EligibilityCriterion[];

  /** Assessments in the study */
  assessments: StudyAssessment[];

  /** Required consent level */
  requiredConsent: PersonalResearchConsent;

  /** Ethics approval */
  ethicsApproval?: EthicsApproval;

  /** Data retention policy */
  dataRetention: DataRetentionPolicy;

  /** Status */
  status:
    | 'draft'
    | 'under-review'
    | 'approved'
    | 'recruiting'
    | 'active'
    | 'completed'
    | 'archived';

  /** Timestamps */
  createdAt: string;
  approvedAt?: string;
  startedAt?: string;
  completedAt?: string;
}

/**
 * An investigator on a study.
 */
export interface Investigator {
  /** Human ID (if on platform) */
  humanId?: string;

  /** Name */
  name: string;

  /** Institution */
  institution?: string;

  /** Role */
  role: 'principal' | 'co-investigator' | 'research-assistant';

  /** ORCID */
  orcid?: string;
}

/**
 * Eligibility criterion for study participation.
 */
export interface EligibilityCriterion {
  /** Field to check */
  field: string;

  /** Operator */
  operator: 'equals' | 'not-equals' | 'greater-than' | 'less-than' | 'in' | 'not-in' | 'exists';

  /** Value */
  value: unknown;

  /** Inclusion or exclusion */
  type: 'inclusion' | 'exclusion';

  /** Description for participants */
  description: string;
}

/**
 * An assessment within a study.
 */
export interface StudyAssessment {
  /** Assessment ID */
  assessmentId: string;

  /** Timing */
  timing: 'baseline' | 'mid-study' | 'endpoint' | 'follow-up' | 'on-demand';

  /** Wave (for longitudinal) */
  wave?: number;

  /** Required or optional */
  required: boolean;

  /** Estimated time (minutes) */
  estimatedMinutes: number;
}

/**
 * Ethics approval for a study.
 */
export interface EthicsApproval {
  /** Approving body */
  body: string;

  /** Reference number */
  referenceNumber: string;

  /** Approval date */
  approvedAt: string;

  /** Expiration */
  expiresAt?: string;

  /** Document hash */
  documentHash: string;
}

/**
 * Data retention policy.
 */
export interface DataRetentionPolicy {
  /** Raw data retention (days) */
  rawDataRetentionDays: number;

  /** Aggregated data retention (days) */
  aggregatedDataRetentionDays: number;

  /** Auto-destroy after retention? */
  autoDestroy: boolean;

  /** Allow early deletion requests? */
  allowEarlyDeletion: boolean;
}
