/**
 * Contributor Presence Model - Stewardship Lifecycle for Absent Contributors
 *
 * From the Manifesto (Part IV-B: Contributor Presence and the Economics of Honor):
 * "When someone's work is referenced in Lamad—whether a book author, a video
 * contributor, or a researcher—a presence is established for them. This presence
 * is not an account they control (yet), but a place where recognition can
 * accumulate."
 *
 * Core Concept:
 * Traditional systems require contributors to join before their work can be
 * honored. This inverts that: recognition flows FIRST, invitation follows.
 * The Elohim steward these presences until contributors claim them.
 *
 * Lifecycle:
 * ┌─────────────────────────────────────────────────────────────────────┐
 * │                                                                     │
 * │   UNCLAIMED ──────────► STEWARDED ──────────► CLAIMED               │
 * │                                                                     │
 * │   Content           Elohim begins         Contributor verifies      │
 * │   referenced,       active care,          identity, claims          │
 * │   presence          recognition           their presence            │
 * │   auto-created      accumulates                                     │
 * │                                                                     │
 * └─────────────────────────────────────────────────────────────────────┘
 *
 * Holochain mapping:
 * - Entry type: "contributor_presence"
 * - Links: content_node → contributor_presence (many-to-many via citation)
 * - Links: elohim_agent → contributor_presence (stewardship relationship)
 * - Claims verified via external identity proofs
 *
 * REA Integration:
 * - Presence is a special type of Agent (type: 'contributor-presence')
 * - Recognition flows are EconomicEvents (action: 'appreciate')
 * - Claiming is an EconomicEvent (action: 'accept')
 * - Accumulated value tracked as EconomicResource
 */

import {
  REAAgent,
  ExternalIdentifier,
  EconomicResource,
  Commitment,
  Measure
} from './rea-bridge.model';
import { OpenGraphMetadata } from './open-graph.model';

// ============================================================================
// Presence State
// ============================================================================

/**
 * PresenceState - The lifecycle state of a contributor presence.
 */
export type PresenceState =
  | 'unclaimed'   // Presence exists, no steward assigned
  | 'stewarded'   // Elohim actively stewarding, recognition accumulating
  | 'claimed';    // Contributor has verified identity and claimed presence

/**
 * State transition rules.
 */
export const PRESENCE_STATE_TRANSITIONS: Record<PresenceState, PresenceState[]> = {
  'unclaimed': ['stewarded'],           // Can only move to stewarded
  'stewarded': ['claimed'],             // Can only move to claimed
  'claimed': [],                        // Terminal state (within this model)
};

// ============================================================================
// Contributor Presence Entity
// ============================================================================

/**
 * ContributorPresence - A placeholder identity for an external contributor.
 *
 * This extends the REA Agent model with presence-specific fields.
 */
export interface ContributorPresence extends REAAgent {
  /** Always 'contributor-presence' */
  type: 'contributor-presence';

  /** Current lifecycle state */
  presenceState: PresenceState;

  /** External identifiers that can be used to verify claims */
  externalIdentifiers: ExternalIdentifier[];

  /** The content that established this presence */
  establishingContentIds: string[];

  /** When this presence was first created */
  establishedAt: string;

  /** Accumulated recognition (the "pot of value" waiting for the contributor) */
  accumulatedRecognition: AccumulatedRecognition;

  /** Current stewardship (if stewarded) */
  stewardship?: PresenceStewardship;

  /** Claim details (if claimed) */
  claim?: PresenceClaim;

  /** Invitation history */
  invitations: PresenceInvitation[];

  /** Nested stewardship (other presences this one may steward after claiming) */
  nestedStewardship?: string[]; // Other ContributorPresence IDs

  // =========================================================================
  // Social Graph Metadata (for sharing contributor profiles)
  // =========================================================================

  /**
   * Open Graph metadata for social sharing.
   * When a contributor profile is shared, this provides rich preview cards.
   * Typically populated when presence is claimed or stewarded.
   */
  socialMetadata?: OpenGraphMetadata;
}

// ============================================================================
// Accumulated Recognition
// ============================================================================

/**
 * AccumulatedRecognition - The value that has flowed to this presence.
 *
 * This is the "pot of recognition" that awaits the contributor.
 * When they claim, all of this transfers to their verified identity.
 */
export interface AccumulatedRecognition {
  /** Total affinity points from views/engagement */
  affinityTotal: number;

  /** Number of unique humans who have engaged */
  uniqueEngagers: number;

  /** Number of citations in other content */
  citationCount: number;

  /** Formal endorsements received */
  endorsements: PresenceEndorsement[];

  /** Total "recognition value" (computed metric) */
  recognitionScore: number;

  /** Breakdown by content that contributed */
  byContent: ContentRecognition[];

  /** When recognition started accumulating */
  accumulatingSince: string;

  /** Last recognition event */
  lastRecognitionAt: string;

  /** As REA resource (for transfer on claim) */
  asResource: EconomicResource;
}

/**
 * ContentRecognition - Recognition attributed to specific content.
 */
export interface ContentRecognition {
  /** The content node */
  contentId: string;

  /** Content title */
  contentTitle: string;

  /** Affinity accumulated for this content */
  affinityTotal: number;

  /** View count */
  viewCount: number;

  /** Citation count */
  citationCount: number;
}

/**
 * PresenceEndorsement - A formal endorsement of this presence's work.
 */
export interface PresenceEndorsement {
  /** Unique identifier */
  id: string;

  /** Who endorsed */
  endorserId: string;

  /** Endorser display name */
  endorserName: string;

  /** Type of endorser */
  endorserType: 'human' | 'organization' | 'elohim' | 'community';

  /** Endorsement message */
  message?: string;

  /** When endorsed */
  endorsedAt: string;

  /** Endorsement weight (based on endorser's standing) */
  weight: number;
}

// ============================================================================
// Presence Stewardship
// ============================================================================

/**
 * PresenceStewardship - Active care of a presence by an Elohim.
 *
 * From the Manifesto:
 * "The Elohim—autonomous constitutional agents—steward these presences.
 * They maintain the integrity of the recognition ledger, prevent fraud,
 * and periodically extend invitations to contributors."
 */
export interface PresenceStewardship {
  /** The Elohim stewarding this presence */
  stewardId: string;

  /** Elohim display name */
  stewardName: string;

  /** Constitutional layer of the steward */
  stewardLayer: string;

  /** When stewardship began */
  startedAt: string;

  /** The commitment record (REA Commitment) */
  commitment: Commitment;

  /** Stewardship activities performed */
  activities: StewardshipActivity[];

  /** Next scheduled review */
  nextReviewAt?: string;

  /** Stewardship quality score (for accountability) */
  qualityScore: number;
}

/**
 * StewardshipActivity - Record of stewardship actions.
 */
export interface StewardshipActivity {
  /** Unique identifier */
  id: string;

  /** Type of activity */
  type: StewardshipActivityType;

  /** When performed */
  performedAt: string;

  /** Description */
  description: string;

  /** Outcome */
  outcome?: string;

  /** Constitutional reasoning (audit trail) */
  constitutionalReasoning?: string;
}

/**
 * StewardshipActivityType - Types of stewardship activities.
 */
export type StewardshipActivityType =
  | 'recognition-audit'      // Verified recognition is legitimate
  | 'fraud-prevention'       // Prevented fraudulent claims
  | 'invitation-sent'        // Sent invitation to contributor
  | 'identity-research'      // Researched contributor's identity
  | 'content-verification'   // Verified content attribution
  | 'nested-stewardship'     // Managed nested presences
  | 'quality-review'         // Reviewed presence quality
  | 'escalation';            // Escalated to higher governance

// ============================================================================
// Presence Invitation
// ============================================================================

/**
 * PresenceInvitation - An invitation sent to a contributor to claim their presence.
 *
 * "Periodically, the Elohim extend invitations: 'Your work has been valued.
 * Recognition awaits you. Would you like to claim your presence?'"
 */
export interface PresenceInvitation {
  /** Unique identifier */
  id: string;

  /** Which channel was used */
  channel: InvitationChannel;

  /** Channel-specific destination */
  destination: string;

  /** When sent */
  sentAt: string;

  /** Current status */
  status: InvitationStatus;

  /** If responded: when */
  respondedAt?: string;

  /** If responded: the response */
  response?: InvitationResponse;

  /** Message content (template-based) */
  message: string;

  /** Recognition summary included in invitation */
  recognitionSummary: {
    totalScore: number;
    uniqueEngagers: number;
    topContent: string[];
  };

  /** Sent by which Elohim */
  sentBy: string;
}

/**
 * InvitationChannel - How invitations can be sent.
 */
export type InvitationChannel =
  | 'email'           // Direct email
  | 'twitter'         // Twitter DM or mention
  | 'github'          // GitHub issue/discussion
  | 'website-contact' // Contact form on contributor's website
  | 'orcid'           // ORCID messaging
  | 'publisher'       // Through publisher
  | 'community';      // Through mutual community connection

/**
 * InvitationStatus - State of an invitation.
 */
export type InvitationStatus =
  | 'pending'     // Sent, awaiting response
  | 'delivered'   // Confirmed delivered
  | 'opened'      // Confirmed opened (if trackable)
  | 'responded'   // Contributor responded
  | 'bounced'     // Failed to deliver
  | 'expired';    // No response within window

/**
 * InvitationResponse - How the contributor responded.
 */
export type InvitationResponse =
  | 'interested'       // Wants to claim
  | 'not-interested'   // Declined
  | 'more-info'        // Wants more information
  | 'not-me'           // Wrong person
  | 'later';           // Interested but not now

// ============================================================================
// Presence Claim
// ============================================================================

/**
 * PresenceClaim - Record of a contributor claiming their presence.
 *
 * This is the moment of transition: recognition transfers to the
 * verified identity, and the contributor joins the network.
 */
export interface PresenceClaim {
  /** When the claim was initiated */
  initiatedAt: string;

  /** When the claim was verified */
  verifiedAt: string;

  /** Verification method used */
  verificationMethod: ClaimVerificationMethod;

  /** Evidence provided */
  evidence: ClaimEvidence[];

  /** The verified agent ID (their new Holochain identity) */
  verifiedAgentId: string;

  /** Recognition transferred */
  recognitionTransferred: Measure;

  /** Elohim who facilitated the claim */
  facilitatedBy: string;

  /** Constitutional reasoning for accepting claim */
  constitutionalReasoning: string;

  /** Any conditions on the claim */
  conditions?: string[];
}

/**
 * ClaimVerificationMethod - How identity was verified.
 */
export type ClaimVerificationMethod =
  | 'domain-verification'  // Prove control of website domain
  | 'social-verification'  // Prove control of social account
  | 'email-verification'   // Prove control of email
  | 'orcid-verification'   // ORCID authentication
  | 'github-verification'  // GitHub authentication
  | 'publisher-attestation' // Publisher vouches for identity
  | 'community-vouching'   // Community members vouch
  | 'cryptographic-proof'; // Existing cryptographic identity

/**
 * ClaimEvidence - Evidence supporting a claim.
 */
export interface ClaimEvidence {
  /** Type of evidence */
  type: string;

  /** Description */
  description: string;

  /** Reference/link */
  reference?: string;

  /** When provided */
  providedAt: string;

  /** Verified by */
  verifiedBy: string;

  /** Verification timestamp */
  verifiedAt: string;
}

// ============================================================================
// Presence Operations
// ============================================================================

/**
 * CreatePresenceRequest - Request to create a new presence.
 *
 * Typically triggered when content is added that references an
 * external contributor not yet in the system.
 */
export interface CreatePresenceRequest {
  /** Display name for the contributor */
  displayName: string;

  /** Known external identifiers */
  externalIdentifiers: ExternalIdentifier[];

  /** Content that references this contributor */
  referencingContentIds: string[];

  /** How the contributor is attributed in the content */
  attribution: string;

  /** Requester (usually the content author or system) */
  requestedBy: string;

  /** When requested */
  requestedAt: string;

  /** Any notes about the contributor */
  notes?: string;
}

/**
 * AssignStewardshipRequest - Request to assign stewardship to an Elohim.
 */
export interface AssignStewardshipRequest {
  /** The presence to steward */
  presenceId: string;

  /** The Elohim to assign (or 'auto' for system selection) */
  stewardId: string;

  /** Why this stewardship is being assigned */
  reason: string;

  /** Priority level */
  priority: 'low' | 'normal' | 'high';

  /** Requested by */
  requestedBy: string;
}

/**
 * InitiateClaimRequest - Request to begin the claim process.
 */
export interface InitiateClaimRequest {
  /** The presence being claimed */
  presenceId: string;

  /** Proposed verification method */
  verificationMethod: ClaimVerificationMethod;

  /** Initial evidence */
  initialEvidence: ClaimEvidence[];

  /** Claimant's new agent ID */
  claimantAgentId: string;

  /** Any message from the claimant */
  message?: string;
}

/**
 * SendInvitationRequest - Request to send an invitation.
 */
export interface SendInvitationRequest {
  /** The presence to invite */
  presenceId: string;

  /** Channel to use */
  channel: InvitationChannel;

  /** Destination address */
  destination: string;

  /** Custom message (optional, template used otherwise) */
  customMessage?: string;

  /** Requested by (usually the steward Elohim) */
  requestedBy: string;
}

// ============================================================================
// Presence Discovery
// ============================================================================

/**
 * PresenceIndexEntry - Lightweight listing for discovery.
 */
export interface PresenceIndexEntry {
  id: string;
  displayName: string;
  presenceState: PresenceState;
  recognitionScore: number;
  uniqueEngagers: number;
  contentCount: number;
  hasActiveSteward: boolean;
  establishedAt: string;
  lastActivityAt: string;
}

/**
 * PresenceSearchCriteria - Search/filter criteria for presences.
 */
export interface PresenceSearchCriteria {
  /** Filter by state */
  states?: PresenceState[];

  /** Filter by minimum recognition score */
  minRecognitionScore?: number;

  /** Filter by stewardship status */
  hasSteward?: boolean;

  /** Search by name */
  nameQuery?: string;

  /** Filter by external identifier type */
  hasIdentifierType?: ExternalIdentifier['type'];

  /** Filter by content domain */
  contentDomain?: string;

  /** Sort order */
  sortBy?: 'recognition' | 'established' | 'activity' | 'name';

  /** Pagination */
  limit?: number;
  offset?: number;
}

// ============================================================================
// Recognition Flow Events
// ============================================================================

/**
 * RecognitionFlowEvent - Event recording recognition flow to a presence.
 *
 * These aggregate into AccumulatedRecognition.
 */
export interface RecognitionFlowEvent {
  /** Unique identifier */
  id: string;

  /** The presence receiving recognition */
  presenceId: string;

  /** Type of recognition */
  type: RecognitionFlowType;

  /** Source of recognition */
  sourceAgentId: string;

  /** Through what content */
  contentId: string;

  /** Quantity */
  quantity: Measure;

  /** When it occurred */
  occurredAt: string;

  /** Additional context */
  context?: Record<string, unknown>;
}

/**
 * RecognitionFlowType - Types of recognition that can flow.
 */
export type RecognitionFlowType =
  | 'view'              // Someone viewed the contributor's content
  | 'affinity'          // Someone marked affinity with the content
  | 'citation'          // Someone cited the content
  | 'endorsement'       // Someone formally endorsed
  | 'path-inclusion'    // Content included in a learning path
  | 'synthesis-reference'; // Elohim referenced in synthesis

// ============================================================================
// Nested Stewardship
// ============================================================================

/**
 * NestedStewardshipOffer - Offer for a claimed contributor to steward others.
 *
 * From the Manifesto:
 * "When a contributor claims their presence, they may find that others have
 * cited them—and those citers have their own unclaimed presences. The
 * contributor can choose to become a steward of the presences of those who
 * built upon their work."
 */
export interface NestedStewardshipOffer {
  /** Unique identifier */
  id: string;

  /** The claimed contributor receiving this offer */
  offeredTo: string; // ContributorPresence ID (claimed)

  /** The unclaimed presence they could steward */
  presenceToSteward: string; // ContributorPresence ID (unclaimed)

  /** Relationship that creates this opportunity */
  relationship: NestedRelationship;

  /** Recognition already accumulated at nested presence */
  accumulatedRecognition: number;

  /** When offered */
  offeredAt: string;

  /** Response status */
  status: 'pending' | 'accepted' | 'declined';

  /** Response timestamp */
  respondedAt?: string;
}

/**
 * NestedRelationship - How the nested presence relates to the steward.
 */
export type NestedRelationship =
  | 'cited-by'        // Nested presence cited the steward's work
  | 'builds-upon'     // Nested presence's work builds on steward's
  | 'same-domain'     // Same knowledge domain
  | 'collaborator'    // Known collaborator
  | 'student-of';     // Was a student/mentee

// ============================================================================
// Default Configurations
// ============================================================================

/**
 * Default stewardship parameters.
 */
export const DEFAULT_STEWARDSHIP_CONFIG = {
  /** Days between invitation attempts */
  invitationIntervalDays: 90,

  /** Maximum invitation attempts before pause */
  maxInvitationAttempts: 4,

  /** Recognition threshold to trigger invitation */
  invitationThreshold: 100, // recognition score

  /** Days of inactivity before stewardship review */
  reviewIntervalDays: 180,

  /** Minimum stewardship quality score */
  minQualityScore: 0.7,
};

/**
 * Recognition score weights.
 */
export const RECOGNITION_SCORE_WEIGHTS = {
  view: 0.1,
  affinity: 1.0,
  citation: 5.0,
  endorsement: 10.0,
  pathInclusion: 3.0,
  synthesisReference: 2.0,
};
