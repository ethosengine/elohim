/**
 * Recovery Models - Elohim Social Recovery
 *
 * When a user loses access to their node, the Elohim network itself serves
 * as the recovery mechanism. Network participants (Elohim) can interview
 * the claimant, ask deep questions about their on-network history, validate
 * their humanity and identity, and provide attestations that unlock access.
 *
 * This is fundamentally different from traditional account recovery:
 * - No centralized authority holds recovery keys
 * - Recovery depends on your relationships in the network
 * - Interviewers use your actual network history as verification
 * - The process is human-mediated, not automated
 *
 * Design notes:
 * - Threshold-based: Need N of M attestations to recover
 * - Questions are generated from network history (paths, relationships)
 * - Confidence levels allow nuanced decisions
 * - Failed recoveries don't permanently lock out (can retry)
 */

// =============================================================================
// Recovery Request
// =============================================================================

/** Status of a recovery request */
export type RecoveryRequestStatus =
  | 'pending'       // Waiting for interviewers
  | 'interviewing'  // One or more interviews in progress
  | 'attested'      // Threshold reached, awaiting finalization
  | 'denied'        // Threshold of denials reached
  | 'completed'     // Recovery successful, credentials issued
  | 'expired'       // Request expired without resolution
  | 'cancelled';    // Claimant cancelled the request

/**
 * A request to recover an identity.
 */
export interface RecoveryRequest {
  /** Unique request identifier */
  id: string;

  /** Claimed identity (human ID or display name) */
  claimedIdentity: string;

  /** Doorway handling this recovery */
  doorwayId: string;

  /** Current status */
  status: RecoveryRequestStatus;

  /** When request was created */
  createdAt: Date;

  /** When request expires (if not completed) */
  expiresAt: Date;

  /** Attestations received so far */
  attestations: RecoveryAttestation[];

  /** Number of attestations required */
  requiredAttestations: number;

  /** Number of denials that would reject the request */
  denyThreshold: number;

  /** Additional context provided by claimant */
  claimantContext?: string;
}

/**
 * Summary of recovery progress.
 */
export interface RecoveryProgress {
  /** Number of affirmative attestations */
  affirmCount: number;

  /** Number of denials */
  denyCount: number;

  /** Number of abstentions */
  abstainCount: number;

  /** Total attestations needed */
  requiredCount: number;

  /** Percentage complete (0-100) */
  progressPercent: number;

  /** Whether threshold has been met */
  thresholdMet: boolean;

  /** Whether denied threshold has been reached */
  isDenied: boolean;
}

// =============================================================================
// Attestation
// =============================================================================

/** Decision an interviewer can make */
export type AttestationDecision = 'affirm' | 'deny' | 'abstain';

/**
 * An attestation from an Elohim interviewer.
 */
export interface RecoveryAttestation {
  /** Attestation identifier */
  id: string;

  /** Recovery request this is for */
  requestId: string;

  /** Elohim who conducted the interview */
  attesterId: string;

  /** Attester's display name */
  attesterDisplayName: string;

  /** Decision made */
  decision: AttestationDecision;

  /** Confidence level (0-100) */
  confidence: number;

  /** Private notes (not shared with claimant) */
  interviewNotes?: string;

  /** When attestation was submitted */
  timestamp: Date;

  /** Duration of interview in minutes */
  interviewDuration?: number;
}

// =============================================================================
// Interview
// =============================================================================

/** Type of interview question */
export type QuestionType =
  | 'network-history'  // Questions about their activity on the network
  | 'relationship'     // Questions about their connections
  | 'content'          // Questions about content they've engaged with
  | 'preference'       // Questions about their preferences/settings
  | 'challenge'        // Open-ended challenge questions
  | 'temporal';        // Questions about when things happened

/**
 * A question to ask during a recovery interview.
 */
export interface InterviewQuestion {
  /** Question identifier */
  id: string;

  /** Type of question */
  type: QuestionType;

  /** The question text */
  question: string;

  /** What a correct answer would demonstrate */
  expectedContext?: string;

  /** Difficulty level (higher = harder) */
  difficulty: number;

  /** Points awarded for correct answer */
  points: number;

  /** Whether answer can be verified against network data */
  verifiable: boolean;
}

/**
 * A response to an interview question.
 */
export interface InterviewResponse {
  /** Question this responds to */
  questionId: string;

  /** The claimant's answer */
  answer: string;

  /** Interviewer's assessment (if verifiable) */
  assessment?: ResponseAssessment;
}

/** Assessment of a response */
export type ResponseAssessment =
  | 'correct'          // Answer matches network data
  | 'partially-correct' // Some accuracy, some errors
  | 'incorrect'        // Answer doesn't match
  | 'unverifiable';    // Cannot be verified against data

/**
 * A complete interview session.
 */
export interface RecoveryInterview {
  /** Interview identifier */
  id: string;

  /** Recovery request this is for */
  requestId: string;

  /** Interviewer's agent ID */
  interviewerId: string;

  /** Interviewer's display name */
  interviewerDisplayName: string;

  /** Status of the interview */
  status: InterviewStatus;

  /** Questions asked */
  questions: InterviewQuestion[];

  /** Responses received */
  responses: InterviewResponse[];

  /** When interview started */
  startedAt: Date;

  /** When interview ended (if completed) */
  completedAt?: Date;

  /** Duration in minutes */
  duration?: number;

  /** Calculated score (0-100) based on responses */
  score?: number;
}

/** Status of an interview session */
export type InterviewStatus =
  | 'scheduled'   // Interview is scheduled
  | 'in-progress' // Interview is happening now
  | 'completed'   // Interview finished, attestation pending
  | 'attested'    // Interview resulted in attestation
  | 'abandoned';  // Interview was abandoned

// =============================================================================
// Recovery Credentials
// =============================================================================

/**
 * Credentials issued after successful recovery.
 */
export interface RecoveryCredential {
  /** Credential identifier */
  id: string;

  /** Recovery request that led to this */
  requestId: string;

  /** Recovered human ID */
  humanId: string;

  /** New agent public key (for new node) */
  agentPubKey: string;

  /** When credential was issued */
  issuedAt: Date;

  /** When credential expires (must be used before) */
  expiresAt: Date;

  /** One-time use token for claiming */
  claimToken: string;

  /** Whether credential has been claimed */
  claimed: boolean;
}

// =============================================================================
// Question Generation
// =============================================================================

/**
 * Context for generating interview questions.
 * Based on the claimant's network history.
 */
export interface QuestionGenerationContext {
  /** Paths the claimant has engaged with */
  pathHistory: PathHistorySummary[];

  /** Connections in their network */
  connections: ConnectionSummary[];

  /** Content they've created or engaged with */
  contentHistory: ContentHistorySummary[];

  /** Timeline of significant events */
  timeline: TimelineEventSummary[];

  /** Preferences and settings */
  preferences: PreferenceSummary;
}

/** Summary of path engagement for question generation */
export interface PathHistorySummary {
  pathId: string;
  pathTitle: string;
  completedAt?: Date;
  stepsCompleted: number;
  totalSteps: number;
}

/** Summary of connections for question generation */
export interface ConnectionSummary {
  displayName: string;
  connectionType: string;
  interactionCount: number;
}

/** Summary of content engagement for question generation */
export interface ContentHistorySummary {
  contentType: string;
  title: string;
  engagedAt: Date;
}

/** Summary of timeline events for question generation */
export interface TimelineEventSummary {
  eventType: string;
  description: string;
  occurredAt: Date;
}

/** Summary of preferences for question generation */
export interface PreferenceSummary {
  theme?: string;
  favoriteTopics?: string[];
  notificationSettings?: string;
}

// =============================================================================
// Interviewer Queue
// =============================================================================

/**
 * A recovery request as seen by potential interviewers.
 */
export interface PendingRecoveryRequest {
  /** Request identifier */
  requestId: string;

  /** Claimed identity (partially masked for privacy) */
  maskedIdentity: string;

  /** Doorway handling the request */
  doorwayName: string;

  /** When request was created */
  createdAt: Date;

  /** Time remaining before expiry */
  expiresIn: number;

  /** Current attestation progress */
  progress: RecoveryProgress;

  /** Whether current user has already attested */
  alreadyAttested: boolean;

  /** Priority score (higher = more urgent) */
  priority: number;
}

// =============================================================================
// Helpers
// =============================================================================

/**
 * Calculate recovery progress from attestations.
 */
export function calculateProgress(
  attestations: RecoveryAttestation[],
  requiredAttestations: number,
  denyThreshold: number
): RecoveryProgress {
  const affirmCount = attestations.filter(a => a.decision === 'affirm').length;
  const denyCount = attestations.filter(a => a.decision === 'deny').length;
  const abstainCount = attestations.filter(a => a.decision === 'abstain').length;

  const progressPercent = Math.min(100, Math.round((affirmCount / requiredAttestations) * 100));
  const thresholdMet = affirmCount >= requiredAttestations;
  const isDenied = denyCount >= denyThreshold;

  return {
    affirmCount,
    denyCount,
    abstainCount,
    requiredCount: requiredAttestations,
    progressPercent,
    thresholdMet,
    isDenied,
  };
}

/**
 * Get display info for attestation decision.
 */
export function getDecisionDisplay(decision: AttestationDecision): { label: string; color: string; icon: string } {
  const displays: Record<AttestationDecision, { label: string; color: string; icon: string }> = {
    'affirm': { label: 'Affirmed', color: '#22c55e', icon: 'check_circle' },
    'deny': { label: 'Denied', color: '#ef4444', icon: 'cancel' },
    'abstain': { label: 'Abstained', color: '#6b7280', icon: 'remove_circle' },
  };
  return displays[decision];
}

/**
 * Get display info for recovery status.
 */
export function getRecoveryStatusDisplay(status: RecoveryRequestStatus): { label: string; color: string; icon: string } {
  const displays: Record<RecoveryRequestStatus, { label: string; color: string; icon: string }> = {
    'pending': { label: 'Pending', color: '#f59e0b', icon: 'hourglass_empty' },
    'interviewing': { label: 'Interviewing', color: '#3b82f6', icon: 'record_voice_over' },
    'attested': { label: 'Attested', color: '#22c55e', icon: 'verified' },
    'denied': { label: 'Denied', color: '#ef4444', icon: 'gpp_bad' },
    'completed': { label: 'Completed', color: '#22c55e', icon: 'check_circle' },
    'expired': { label: 'Expired', color: '#6b7280', icon: 'schedule' },
    'cancelled': { label: 'Cancelled', color: '#6b7280', icon: 'cancel' },
  };
  return displays[status];
}

/**
 * Get display info for question type.
 */
export function getQuestionTypeDisplay(type: QuestionType): { label: string; icon: string } {
  const displays: Record<QuestionType, { label: string; icon: string }> = {
    'network-history': { label: 'Network History', icon: 'history' },
    'relationship': { label: 'Relationships', icon: 'people' },
    'content': { label: 'Content', icon: 'article' },
    'preference': { label: 'Preferences', icon: 'settings' },
    'challenge': { label: 'Challenge', icon: 'psychology' },
    'temporal': { label: 'Timeline', icon: 'schedule' },
  };
  return displays[type];
}

/**
 * Mask identity for privacy in interviewer queue.
 */
export function maskIdentity(identity: string): string {
  if (identity.length <= 4) {
    return '*'.repeat(identity.length);
  }
  const visible = Math.ceil(identity.length * 0.3);
  return identity.substring(0, visible) + '*'.repeat(identity.length - visible);
}

/**
 * Calculate interview score from responses.
 */
export function calculateInterviewScore(
  questions: InterviewQuestion[],
  responses: InterviewResponse[]
): number {
  if (questions.length === 0) return 0;

  const maxPoints = questions.reduce((sum, q) => sum + q.points, 0);
  let earnedPoints = 0;

  for (const response of responses) {
    const question = questions.find(q => q.id === response.questionId);
    if (!question) continue;

    switch (response.assessment) {
      case 'correct':
        earnedPoints += question.points;
        break;
      case 'partially-correct':
        earnedPoints += question.points * 0.5;
        break;
      // incorrect and unverifiable earn no points
    }
  }

  return Math.round((earnedPoints / maxPoints) * 100);
}
