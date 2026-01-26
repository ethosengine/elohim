/**
 * Consent Relationship Model - Consent as Ongoing Dialogue
 *
 * The Elohim Protocol approach to consent is NOT:
 * - Meta's "opt-in and trust us" model
 * - A settings page you set once and forget
 * - Blanket permissions for categories of use
 *
 * Instead, consent is:
 * - An ongoing RELATIONSHIP between you and data users
 * - Each request is a CONVERSATION, not a permission popup
 * - A way to discover who's interested in what you know
 * - An opportunity to learn, connect, and exchange value
 * - Continuously affirmed through periodic audits
 * - Recorded as attestations, creating an auditable trail
 *
 * Core principles:
 * 1. CONVERSATION > CHECKBOX - Consent requests open dialogue
 * 2. AFFIRMATION > SETTINGS - Periodic review, not set-and-forget
 * 3. VALUE FLOWS BOTH WAYS - Your data has value, capture some of it
 * 4. TRANSPARENCY - See exactly how your data was used
 * 5. ELOHIM AS GUARDIAN - Your agent helps manage, but never auto-approves
 *
 * hREA/Shefa Integration:
 * - Value exchanges are recorded as EconomicEvents
 * - Data access creates 'use' events (attention to your data)
 * - Value offered/received flows through the standard token system
 * - Consent grants create Commitments that are fulfilled by events
 *
 * References:
 * - Data Dignity: Lanier & Weyl (2018)
 * - Dynamic Consent: GDPR research community
 * - Radical Markets: Posner & Weyl (2018)
 */

import type { EconomicEvent, LamadEventType } from '@app/elohim/models/economic-event.model';
import type { Measure, ResourceClassification } from '@app/elohim/models/rea-bridge.model';

// =============================================================================
// Consent Request (The Invitation)
// =============================================================================

/**
 * A consent request - someone wants to use your data.
 * This is an invitation to a conversation, not a permission popup.
 */
export interface ConsentRequest {
  /** Unique request ID */
  id: string;

  /** Who's asking (study, researcher, organization) */
  requester: ConsentRequester;

  /** What they want */
  dataRequest: DataRequest;

  /** Why they want it (purpose/research question) */
  purpose: ConsentPurpose;

  /** What they're offering in return */
  valueOffer: ValueOffer;

  /** How long they want access */
  accessDuration: AccessDuration;

  /** Current status */
  status: ConsentRequestStatus;

  /** When this request was made */
  requestedAt: string;

  /** When this request expires (if not responded to) */
  expiresAt?: string;

  /** Thread of conversation about this request */
  conversationId?: string;

  /** Your Elohim's assessment of this request */
  elohimAssessment?: ElohimConsentAssessment;
}

/**
 * Who's requesting your data.
 */
export interface ConsentRequester {
  /** Type of requester */
  type: 'researcher' | 'study' | 'organization' | 'individual' | 'data-pool';

  /** Requester ID */
  id: string;

  /** Display name */
  name: string;

  /** Affiliation/institution */
  affiliation?: string;

  /** Verified status */
  verified: boolean;

  /** Your relationship to this requester */
  relationship?: 'unknown' | 'known' | 'trusted' | 'previous-grant';

  /** Previous interactions */
  previousRequests: number;
  previousGrants: number;

  /** Their reputation in the community */
  reputation?: RequesterReputation;
}

/**
 * Reputation metrics for a requester.
 */
export interface RequesterReputation {
  /** How many people have granted them consent */
  totalGrants: number;

  /** How many withdrawals/complaints */
  withdrawals: number;

  /** Average rating from data subjects */
  averageRating: number;

  /** Whether they've published results as promised */
  publicationRate: number;

  /** Community endorsements */
  endorsements: string[];
}

/**
 * What data they're requesting.
 */
export interface DataRequest {
  /** Type of data */
  dataType: RequestedDataType;

  /** Specific items requested */
  specificItems?: string[];

  /** Level of detail requested */
  granularity: 'aggregate-only' | 'summary' | 'detailed' | 'raw';

  /** Whether they need to link with other data */
  linkingRequired: boolean;

  /** What they'll link with */
  linkedDataTypes?: string[];

  /** Anonymization level */
  anonymization: 'full' | 'k-anonymity' | 'pseudonymous' | 'identified';

  /** Whether re-contact is requested */
  recontactRequested: boolean;
}

/**
 * Types of data that can be requested.
 */
export type RequestedDataType =
  | 'discovery-assessment' // Your Enneagram, MBTI, etc.
  | 'research-response' // Your responses to research instruments
  | 'learning-journey' // Your path through content
  | 'deliberation-positions' // Your positions in deliberations
  | 'network-position' // Your place in social networks
  | 'temporal-patterns' // ESM/EMA data over time
  | 'dyadic-data' // Data from your paired relationships
  | 'group-participation' // Your role in group assessments
  | 'aggregate-contribution'; // Just contribute to aggregate stats

/**
 * Why they want the data.
 */
export interface ConsentPurpose {
  /** Purpose category */
  category:
    | 'academic-research'
    | 'community-insight'
    | 'personal-connection'
    | 'commercial'
    | 'governance';

  /** Description of purpose */
  description: string;

  /** Research question (if research) */
  researchQuestion?: string;

  /** How results will be used */
  intendedUse: string;

  /** Who will see the results */
  resultAudience: 'researchers-only' | 'academic-publication' | 'community-members' | 'public';

  /** Will you get access to results? */
  resultsSharedWithYou: boolean;

  /** IRB/ethics approval */
  ethicsApproval?: {
    body: string;
    reference: string;
    verified: boolean;
  };
}

/**
 * What they're offering in return.
 *
 * Connects to hREA/Shefa:
 * - tokenResourceType maps to ResourceClassification for token types
 * - measure uses the standard Measure type for quantity
 * - When consent is granted, this becomes an Intent/Commitment
 */
export interface ValueOffer {
  /** Type of value offered */
  type: ValueOfferType;

  /** Description */
  description: string;

  /**
   * For token/monetary offers: the resource type being offered.
   * Maps to ResourceClassification from rea-bridge.model.ts
   * Examples: 'recognition', 'currency', 'learning-token', 'care-token'
   */
  tokenResourceType?: ResourceClassification;

  /**
   * Quantity offered (amount + unit).
   * Uses standard hREA Measure type.
   */
  measure?: Measure;

  /** Non-monetary benefits */
  benefits?: string[];

  /** Access to research results */
  resultsAccess?: 'none' | 'summary' | 'full-report' | 'co-authorship';

  /** Recognition/attribution */
  attribution?: 'anonymous' | 'acknowledged' | 'named';

  /**
   * If this offer was already committed, the Commitment ID.
   * Links to hREA Commitment for tracking fulfillment.
   */
  commitmentId?: string;
}

/**
 * Types of value that can be offered.
 */
export type ValueOfferType =
  | 'none' // Requesting altruistic contribution
  | 'results-access' // Access to research findings
  | 'reciprocal-data' // They share their data too
  | 'community-benefit' // Benefits the community you're part of
  | 'tokens' // Protocol tokens
  | 'monetary' // Fiat compensation
  | 'reputation' // Reputation/attestation in return
  | 'connection' // Relationship with requester
  | 'negotiable'; // Open to discussion

/**
 * How long they want access.
 */
export interface AccessDuration {
  /** Duration type */
  type: 'one-time' | 'study-duration' | 'time-limited' | 'ongoing' | 'until-withdrawn';

  /** End date if time-limited */
  endsAt?: string;

  /** Study ID if study-duration */
  studyId?: string;

  /** Review period for ongoing access */
  reviewIntervalDays?: number;
}

/**
 * Current status of a consent request.
 */
export type ConsentRequestStatus =
  | 'pending' // Awaiting your response
  | 'in-conversation' // Active dialogue
  | 'deferred' // You asked to decide later
  | 'granted' // You said yes
  | 'declined' // You said no
  | 'negotiating' // Discussing terms
  | 'expired' // Request timed out
  | 'withdrawn'; // Requester withdrew

// =============================================================================
// Consent Response (Your Answer)
// =============================================================================

/**
 * Your response to a consent request.
 * This isn't just yes/no - it's a nuanced decision.
 */
export interface ConsentResponse {
  /** Response ID */
  id: string;

  /** Request being responded to */
  requestId: string;

  /** Your decision */
  decision: ConsentDecision;

  /** When you responded */
  respondedAt: string;

  /** Your reasoning (optional, for your records) */
  reasoning?: string;

  /** Modified terms (if negotiated) */
  modifiedTerms?: ConsentTerms;

  /** When this consent should be reviewed */
  reviewAt?: string;

  /** This becomes an attestation */
  attestationId?: string;
}

/**
 * Your decision on a consent request.
 */
export type ConsentDecision =
  | 'grant' // Yes, you can use my data
  | 'grant-modified' // Yes, but with different terms
  | 'decline' // No, not this time
  | 'decline-permanent' // No, don't ask again
  | 'defer' // Ask me again later
  | 'counter-offer'; // I want different terms

/**
 * Terms of a consent grant.
 */
export interface ConsentTerms {
  /** Granularity you're allowing */
  granularity: 'aggregate-only' | 'summary' | 'detailed' | 'raw';

  /** Anonymization required */
  anonymization: 'full' | 'k-anonymity' | 'pseudonymous' | 'identified';

  /** Duration of access */
  duration: AccessDuration;

  /** Value you're receiving */
  valueReceived: ValueOffer;

  /** Can they re-contact you? */
  allowRecontact: boolean;

  /** Can they share with others? */
  allowSharing: boolean;

  /** Notification preferences */
  notifications: {
    notifyOnUse: boolean;
    notifyOnPublication: boolean;
    notifyOnSharing: boolean;
  };

  /** Custom conditions */
  customConditions?: string[];
}

// =============================================================================
// Consent Grant (The Attestation)
// =============================================================================

/**
 * A consent grant - recorded as an attestation.
 * This is the "contract" between you and the data user.
 */
export interface ConsentGrant {
  /** Grant ID (also the attestation ID) */
  id: string;

  /** Request that led to this grant */
  requestId: string;

  /** Your response */
  responseId: string;

  /** Who you granted consent to */
  grantee: ConsentRequester;

  /** What data you granted access to */
  grantedData: DataRequest;

  /** Terms of the grant */
  terms: ConsentTerms;

  /** When granted */
  grantedAt: string;

  /** When this grant expires */
  expiresAt?: string;

  /** When you should review this grant */
  reviewAt: string;

  /** Current status */
  status: ConsentGrantStatus;

  /** Usage log - how has your data been used? */
  usageLog: DataUsageEvent[];

  /** Value you've received */
  valueReceived: ValueReceivedLog[];
}

/**
 * Status of a consent grant.
 */
export type ConsentGrantStatus =
  | 'active' // Currently in effect
  | 'paused' // Temporarily suspended
  | 'under-review' // You're reviewing whether to continue
  | 'expired' // Time-limited grant ended
  | 'withdrawn' // You withdrew consent
  | 'violated' // Terms were violated
  | 'completed'; // Study/purpose completed

/**
 * Log of how your data was used under a grant.
 *
 * Connects to hREA/Shefa:
 * - Data usage creates 'use' EconomicEvents in the data economy
 * - Your data is treated as a resource that others can 'use' (not consume)
 * - This creates an auditable trail of data access
 */
export interface DataUsageEvent {
  /** Event ID */
  id: string;

  /** When the use occurred */
  occurredAt: string;

  /** Type of use */
  type: 'accessed' | 'analyzed' | 'aggregated' | 'shared' | 'published' | 'deleted';

  /** Description */
  description: string;

  /** Who used it */
  usedBy: string;

  /** Purpose of this specific use */
  purpose: string;

  /**
   * The EconomicEvent that recorded this data usage.
   * Action: 'use' - uses your data without consuming it
   * Resource: your data/assessment response
   * Provider: you (the data subject)
   * Receiver: the data user
   */
  economicEventId?: string;

  /** Outcome/result if any */
  outcome?: string;

  /** Link to publication if published */
  publicationLink?: string;
}

/**
 * Log of value you've received from a grant.
 *
 * Connects to hREA/Shefa:
 * - Each value receipt is backed by an EconomicEvent
 * - The event records the actual transfer in the economic layer
 * - This provides full auditability and constitutional governance
 */
export interface ValueReceivedLog {
  /** When received */
  receivedAt: string;

  /** Type of value */
  type: ValueOfferType;

  /** Description */
  description: string;

  /**
   * Quantity received.
   * Uses standard hREA Measure type.
   */
  measure?: Measure;

  /**
   * The EconomicEvent that recorded this value transfer.
   * Links to the immutable audit trail in Shefa.
   */
  economicEventId?: string;

  /**
   * Resource type received.
   * Maps to ResourceClassification.
   */
  resourceType?: ResourceClassification;

  /** Link to results if results-access */
  resultsLink?: string;
}

// =============================================================================
// Consent Audit (Periodic Review)
// =============================================================================

/**
 * A consent audit - periodic review of your active grants.
 * This is how consent stays alive rather than set-and-forget.
 */
export interface ConsentAudit {
  /** Audit ID */
  id: string;

  /** When this audit was triggered */
  triggeredAt: string;

  /** Trigger type */
  triggerType: AuditTrigger;

  /** Grants being reviewed */
  grantsUnderReview: ConsentGrantSummary[];

  /** Your overall consent posture */
  consentSummary: ConsentPostureSummary;

  /** Status */
  status: 'pending' | 'in-progress' | 'completed' | 'deferred';

  /** When completed */
  completedAt?: string;

  /** Decisions made during this audit */
  decisions?: AuditDecision[];
}

/**
 * What triggered this audit.
 */
export type AuditTrigger =
  | 'scheduled' // Regular periodic review
  | 'usage-threshold' // Your data was used X times
  | 'new-publication' // Something was published using your data
  | 'grant-expiring' // A grant is about to expire
  | 'user-initiated' // You requested a review
  | 'policy-change' // Requester changed their terms
  | 'violation-detected'; // Potential terms violation

/**
 * Summary of a grant for audit review.
 */
export interface ConsentGrantSummary {
  /** Grant ID */
  grantId: string;

  /** Grantee name */
  granteeName: string;

  /** What data */
  dataType: RequestedDataType;

  /** When granted */
  grantedAt: string;

  /** Times used since last review */
  usagesSinceLastReview: number;

  /** Value received since last review */
  valueReceivedSinceLastReview: string;

  /** Any issues detected */
  issues?: string[];

  /** Recommended action */
  recommendedAction: 'continue' | 'review' | 'modify' | 'withdraw';
}

/**
 * Summary of your overall consent posture.
 */
export interface ConsentPostureSummary {
  /** Total active grants */
  activeGrants: number;

  /** Grants by category */
  grantsByCategory: Record<string, number>;

  /** Total uses of your data */
  totalUses: number;

  /** Total value received */
  totalValueReceived: string;

  /** Grants expiring soon */
  expiringGrants: number;

  /** Grants needing attention */
  grantsNeedingAttention: number;
}

/**
 * A decision made during an audit.
 */
export interface AuditDecision {
  /** Grant ID */
  grantId: string;

  /** Decision */
  decision: 'continue' | 'modify' | 'pause' | 'withdraw';

  /** Modified terms if modified */
  modifiedTerms?: Partial<ConsentTerms>;

  /** Reasoning */
  reasoning?: string;

  /** Next review date */
  nextReviewAt?: string;
}

// =============================================================================
// Elohim Consent Guardian
// =============================================================================

/**
 * Your Elohim's assessment of a consent request.
 * Your agent helps you manage consent, but never auto-approves.
 */
export interface ElohimConsentAssessment {
  /** Assessment timestamp */
  assessedAt: string;

  /** Overall recommendation */
  recommendation:
    | 'likely-approve'
    | 'worth-considering'
    | 'needs-review'
    | 'likely-decline'
    | 'flag-concern';

  /** Confidence in recommendation (0-1) */
  confidence: number;

  /** Factors considered */
  factors: ConsentFactor[];

  /** Similar past decisions you've made */
  similarPastDecisions?: {
    grantId: string;
    decision: ConsentDecision;
    similarity: number;
  }[];

  /** Questions the Elohim suggests you ask */
  suggestedQuestions?: string[];

  /** Concerns to consider */
  concerns?: string[];

  /** Why this might be valuable */
  potentialValue?: string[];
}

/**
 * A factor the Elohim considered.
 */
export interface ConsentFactor {
  /** Factor type */
  type:
    | 'requester-reputation'
    | 'purpose-alignment'
    | 'value-fairness'
    | 'privacy-risk'
    | 'past-behavior'
    | 'community-benefit';

  /** Weight of this factor */
  weight: number;

  /** Score for this factor (-1 to 1) */
  score: number;

  /** Explanation */
  explanation: string;
}

// =============================================================================
// Consent Conversation
// =============================================================================

/**
 * A conversation about a consent request.
 * Consent isn't just yes/no - it's dialogue.
 */
export interface ConsentConversation {
  /** Conversation ID */
  id: string;

  /** Consent request this conversation is about */
  requestId: string;

  /** Participants */
  participants: ConversationParticipant[];

  /** Messages in the conversation */
  messages: ConsentMessage[];

  /** Current status */
  status: 'active' | 'concluded' | 'abandoned';

  /** Started at */
  startedAt: string;

  /** Last activity */
  lastActivityAt: string;

  /** Outcome if concluded */
  outcome?: {
    decision: ConsentDecision;
    responseId: string;
  };
}

/**
 * A participant in a consent conversation.
 */
export interface ConversationParticipant {
  /** Human or Elohim ID */
  id: string;

  /** Type */
  type: 'data-subject' | 'requester' | 'elohim-guardian' | 'mediator';

  /** Display name */
  name: string;
}

/**
 * A message in a consent conversation.
 */
export interface ConsentMessage {
  /** Message ID */
  id: string;

  /** Sender */
  senderId: string;

  /** Message type */
  type: 'question' | 'answer' | 'proposal' | 'counter-proposal' | 'clarification' | 'decision';

  /** Content */
  content: string;

  /** Sent at */
  sentAt: string;

  /** If this is a proposal, the terms proposed */
  proposedTerms?: ConsentTerms;
}

// =============================================================================
// Consent Defaults (Emergent from Decisions, Not Settings)
// =============================================================================

/**
 * Your consent patterns - emergent from your decisions, not configured.
 * Your Elohim learns these to help filter requests.
 */
export interface ConsentPatterns {
  /** Human ID */
  humanId: string;

  /** Patterns by requester type */
  byRequesterType: Record<string, ConsentPattern>;

  /** Patterns by data type */
  byDataType: Record<RequestedDataType, ConsentPattern>;

  /** Patterns by purpose */
  byPurpose: Record<string, ConsentPattern>;

  /** Patterns by value type */
  byValueType: Record<ValueOfferType, ConsentPattern>;

  /** Overall consent rate */
  overallGrantRate: number;

  /** Average response time */
  averageResponseTimeHours: number;

  /** Topics you consistently decline */
  consistentDeclines: string[];

  /** Topics you consistently approve */
  consistentApprovals: string[];

  /** Last updated */
  lastUpdated: string;
}

/**
 * A pattern in your consent decisions.
 */
export interface ConsentPattern {
  /** Number of decisions */
  decisionCount: number;

  /** Grant rate */
  grantRate: number;

  /** Common terms you grant with */
  commonTerms?: Partial<ConsentTerms>;

  /** Common concerns you raise */
  commonConcerns?: string[];

  /** Trend (increasing grants, decreasing, stable) */
  trend: 'increasing' | 'decreasing' | 'stable';
}

// =============================================================================
// Service Interface
// =============================================================================

/**
 * Service interface for consent relationship management.
 */
export interface ConsentRelationshipService {
  // ===== Receiving requests
  getPendingRequests(): Promise<ConsentRequest[]>;
  getRequest(requestId: string): Promise<ConsentRequest>;

  // ===== Responding to requests
  respond(
    requestId: string,
    response: Omit<ConsentResponse, 'id' | 'respondedAt'>
  ): Promise<ConsentResponse>;
  startConversation(requestId: string, message: string): Promise<ConsentConversation>;
  proposeTerms(requestId: string, terms: ConsentTerms): Promise<ConsentMessage>;

  // ===== Managing grants
  getActiveGrants(): Promise<ConsentGrant[]>;
  getGrant(grantId: string): Promise<ConsentGrant>;
  pauseGrant(grantId: string): Promise<void>;
  withdrawGrant(grantId: string, reason?: string): Promise<void>;
  modifyGrant(grantId: string, newTerms: Partial<ConsentTerms>): Promise<ConsentGrant>;

  // ===== Audits
  getPendingAudits(): Promise<ConsentAudit[]>;
  startAudit(): Promise<ConsentAudit>;
  submitAuditDecisions(auditId: string, decisions: AuditDecision[]): Promise<void>;

  // ===== Patterns (read-only, emergent)
  getMyConsentPatterns(): Promise<ConsentPatterns>;

  // ===== Elohim integration
  getElohimAssessment(requestId: string): Promise<ElohimConsentAssessment>;
  askElohim(requestId: string, question: string): Promise<string>;

  // ===== Transparency
  getUsageLog(grantId: string): Promise<DataUsageEvent[]>;
  getValueReceived(grantId: string): Promise<ValueReceivedLog[]>;
  getMyDataFootprint(): Promise<ConsentPostureSummary>;
}
