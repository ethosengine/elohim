/**
 * Requests and Offers Domain Models
 *
 * Lifted from /research/requests-and-offers and integrated with Shefa REA/ValueFlows
 *
 * Key Insight: A Request/Offer is an Intent in REA/ValueFlows vocabulary.
 * - Request = "I intend to receive this service/resource"
 * - Offer = "I intend to provide this service/resource"
 * - Matching = Creating a Proposal that links request + offer intents
 *
 * This module builds on top of existing Intent/Proposal models in rea-bridge.model.ts
 * and adds the preference, timing, and contact patterns from requests-and-offers.
 *
 * Integration Points:
 * - Extends REAAgent (users, organizations)
 * - Extends Intent (requests, offers)
 * - Extends Proposal (matching)
 * - Uses ServiceType for resource specification
 * - Uses MediumOfExchange (already links to hREA ResourceSpec)
 *
 * References:
 * - /research/requests-and-offers - Source domain model
 * - rea-bridge.model.ts - REA/ValueFlows foundation
 * - protocol-core.model.ts - Token types
 */

import {
  Intent,
  Proposal,
  REAAgent,
  ResourceSpecification,
  Measure,
  GovernanceLayer,
} from '@app/elohim/models/rea-bridge.model';

// ============================================================================
// PREFERENCES & SETTINGS
// ============================================================================

/**
 * ContactPreference - How someone prefers to be contacted.
 *
 * Used when matching requests/offers to know how to coordinate.
 */
export type ContactPreference = 'email' | 'phone' | 'in-app' | 'other';

/**
 * TimePreference - When someone prefers to work/interact.
 *
 * Helps match people with compatible schedules.
 */
export type TimePreference =
  | 'morning' // 6am - 12pm
  | 'afternoon' // 12pm - 6pm
  | 'evening' // 6pm - midnight
  | 'flexible' // No strong preference
  | 'other'; // Custom (described in note)

/**
 * InteractionType - Virtual or in-person.
 *
 * Determines how request/offer can be fulfilled.
 */
export type InteractionType = 'virtual' | 'in-person' | 'hybrid' | 'either';

/**
 * DateRange - Time window for request/offer.
 *
 * Optional start and end dates for when availability exists.
 */
export interface DateRange {
  startDate?: string; // ISO 8601 date
  endDate?: string; // ISO 8601 date
  flexibleDates: boolean; // Can dates shift?
}

/**
 * UserPreferences - Preferences for a user in the network.
 *
 * These are stored on the user profile and affect how they appear
 * in matching and how they can be contacted.
 */
export interface UserPreferences {
  /** Unique identifier */
  id: string;

  /** User this preference set belongs to */
  userId: string;

  /** Preferred contact method */
  contactPreference: ContactPreference;

  /** Preferred contact address for that method */
  contactValue: string; // email@example.com or +1-555-1234

  /** Time zone (e.g., "UTC", "EST", "PST") */
  timeZone: string;

  /** When they prefer to work/interact */
  timePreference: TimePreference;

  /** Virtual or in-person? */
  interactionType: InteractionType;

  /** Hours per week they can contribute */
  availableHoursPerWeek?: number;

  /** Languages spoken */
  languages: string[];

  /** Skills/interests they want to develop */
  skillsToLearn: string[];

  /** Skills they can share */
  skillsToShare: string[];

  /** When these preferences were last updated */
  updatedAt: string;

  /** Arbitrary metadata */
  metadata?: Record<string, unknown>;
}

// ============================================================================
// SERVICE TYPE (CATEGORIZATION)
// ============================================================================

/**
 * ServiceType - A category of service that can be requested/offered.
 *
 * Examples: "Logo Design", "Cloud Consulting", "Translation", "Tutoring"
 *
 * Acts as a tag system for discovering requests/offers.
 * Each request/offer is tagged with one or more ServiceTypes.
 */
export interface ServiceType {
  /** Unique identifier */
  id: string;

  /** Name of service (e.g., "Logo Design") */
  name: string;

  /** Description of what this service involves */
  description: string;

  /** Is this technical or non-technical? */
  isTechnical: boolean;

  /** Who created this service type */
  creatorId: string;

  /** When created */
  createdAt: string;

  /** Can only creator modify/delete? */
  isAuthorOnly: boolean;

  /** Status of this service type */
  status: 'active' | 'archived' | 'deleted';

  /** Arbitrary metadata */
  metadata?: Record<string, unknown>;
}

// ============================================================================
// MEDIUM OF EXCHANGE
// ============================================================================

/**
 * MediumOfExchange - What can be exchanged in a request/offer.
 *
 * Examples: EUR, USD, Hours (time banking), Credits (mutual credit)
 *
 * Can be currency, time banking, or mutual credit tokens.
 * Each medium links to an hREA ResourceSpecification for economic tracking.
 */
export interface MediumOfExchange {
  /** Unique identifier */
  id: string;

  /** Code (e.g., "EUR", "USD", "TIME", "CARE-TOKEN") */
  code: string;

  /** Human-readable name (e.g., "Euro", "Time Hour", "Care Token") */
  name: string;

  /** Description of what this medium represents */
  description?: string;

  /** Type of medium */
  exchangeType: 'currency' | 'time-banking' | 'mutual-credit' | 'barter';

  /** Link to hREA ResourceSpecification (for economic integration) */
  resourceSpecHreaId?: string;

  /** Who created this medium of exchange */
  creatorId: string;

  /** When created */
  createdAt: string;

  /** Can only creator modify/delete? */
  isAuthorOnly: boolean;

  /** Status */
  status: 'active' | 'archived' | 'deleted';

  /** Arbitrary metadata */
  metadata?: Record<string, unknown>;
}

// ============================================================================
// REQUEST (EXTENDS INTENT)
// ============================================================================

/**
 * ServiceRequest - Someone is requesting a service/resource.
 *
 * Extends REA Intent with request-specific preferences and timing.
 *
 * In REA terms:
 * - action: 'take' (receiver intends to take a resource)
 * - resourceConformsTo: The service being requested
 * - resourceQuantity: How much (hours, units, etc.)
 *
 * In requests-and-offers terms:
 * - title: What they want
 * - description: Details and requirements
 * - preferences: When, how, contact method
 * - serviceTypes: Categories/tags
 * - budget: What they can offer/pay
 */
export interface ServiceRequest extends Omit<Intent, 'id' | 'createdAt'> {
  /** Unique identifier */
  id: string;

  /** Human-readable request ID (e.g., "REQ-2025-001234") */
  requestNumber: string;

  /** Member requesting the service */
  requesterId: string;

  /** Organization (if any) making the request */
  organizationId?: string;

  /** Simple title (e.g., "Need logo design for startup") */
  title: string;

  /** Detailed description (e.g., "Looking for a designer to create a modern logo") */
  description: string;

  // ─────────────────────────────────────────────────────────────────
  // Preferences & Timing
  // ─────────────────────────────────────────────────────────────────

  /** Contact preference for this request */
  contactPreference: ContactPreference;

  /** Contact address (email, phone, username) */
  contactValue: string;

  /** When they need/prefer this (date range) */
  dateRange: DateRange;

  /** Time zone */
  timeZone: string;

  /** Time of day preference */
  timePreference: TimePreference;

  /** Virtual, in-person, or hybrid? */
  interactionType: InteractionType;

  /** Estimated hours needed (if time-based) */
  estimatedHours?: number;

  // ─────────────────────────────────────────────────────────────────
  // Service & Resources
  // ─────────────────────────────────────────────────────────────────

  /** Service type IDs (what category of service) */
  serviceTypeIds: string[];

  /** Resources/skills needed */
  requiredSkills: string[];

  /** Budget or payment options */
  budget?: {
    amount: Measure;
    mediumOfExchangeId: string; // What they can pay with
  };

  /** Alternative exchange options (time, barter, etc.) */
  mediumOfExchangeIds: string[];

  // ─────────────────────────────────────────────────────────────────
  // Status & Lifecycle
  // ─────────────────────────────────────────────────────────────────

  /** Status of request */
  status: 'active' | 'archived' | 'deleted';

  /** Is this request visible/searchable? */
  isPublic: boolean;

  /** When created */
  createdAt: string;

  /** When updated */
  updatedAt: string;

  /** Links to external resources (images, documents, etc.) */
  links: string[];

  /** Arbitrary metadata */
  metadata?: Record<string, unknown>;

  // ─────────────────────────────────────────────────────────────────
  // Matching & Coordination (populated after matching)
  // ─────────────────────────────────────────────────────────────────

  /** Matched offers (when service type matches, proposer offers) */
  matchedOfferIds?: string[];

  /** Created proposals (request + offer pairings) */
  proposalIds?: string[];

  /** Accepted proposal (if matched and accepted) */
  acceptedProposalId?: string;
}

// ============================================================================
// OFFER (EXTENDS INTENT)
// ============================================================================

/**
 * ServiceOffer - Someone is offering a service/resource.
 *
 * Extends REA Intent with offer-specific preferences and timing.
 *
 * In REA terms:
 * - action: 'give' (provider intends to give a resource)
 * - resourceConformsTo: The service being offered
 * - resourceQuantity: How much they can provide
 *
 * In requests-and-offers terms:
 * - title: What they offer
 * - description: Expertise, capability, what they can do
 * - preferences: When, how, contact method
 * - serviceTypes: Categories/tags for what they offer
 * - rates: What they charge/want
 */
export interface ServiceOffer extends Omit<Intent, 'id' | 'createdAt'> {
  /** Unique identifier */
  id: string;

  /** Human-readable offer ID (e.g., "OFR-2025-005678") */
  offerNumber: string;

  /** Member offering the service */
  offerorId: string;

  /** Organization (if any) making the offer */
  organizationId?: string;

  /** Simple title (e.g., "Logo design and branding services") */
  title: string;

  /** Detailed description (e.g., "Professional designer with 10 years experience") */
  description: string;

  // ─────────────────────────────────────────────────────────────────
  // Preferences & Availability
  // ─────────────────────────────────────────────────────────────────

  /** Preferred contact method for offers */
  contactPreference: ContactPreference;

  /** Contact address (email, phone, username) */
  contactValue: string;

  /** When they're available */
  dateRange: DateRange;

  /** Time zone */
  timeZone: string;

  /** Time of day preference */
  timePreference: TimePreference;

  /** Virtual, in-person, or hybrid? */
  interactionType: InteractionType;

  /** Hours per week available */
  hoursPerWeek?: number;

  // ─────────────────────────────────────────────────────────────────
  // Service & Compensation
  // ─────────────────────────────────────────────────────────────────

  /** Service type IDs (categories of what they offer) */
  serviceTypeIds: string[];

  /** Skills they're offering */
  offerredSkills: string[];

  /** Their rate or pricing */
  rate?: {
    amount: Measure;
    per: 'hour' | 'project' | 'month' | 'other';
    mediumOfExchangeId: string;
  };

  /** What payment methods they accept */
  mediumOfExchangeIds: string[];

  /** Are they open to barter/other exchange? */
  acceptsAlternativePayment: boolean;

  // ─────────────────────────────────────────────────────────────────
  // Status & Lifecycle
  // ─────────────────────────────────────────────────────────────────

  /** Status of offer */
  status: 'active' | 'archived' | 'deleted';

  /** Is this offer visible/searchable? */
  isPublic: boolean;

  /** When created */
  createdAt: string;

  /** When updated */
  updatedAt: string;

  /** Links to portfolio, examples, credentials, etc. */
  links: string[];

  /** Arbitrary metadata */
  metadata?: Record<string, unknown>;

  // ─────────────────────────────────────────────────────────────────
  // Matching & Coordination
  // ─────────────────────────────────────────────────────────────────

  /** Matched requests (when service type matches, requester requests) */
  matchedRequestIds?: string[];

  /** Created proposals (offer + request pairings) */
  proposalIds?: string[];

  /** Accepted proposal (if matched and accepted) */
  acceptedProposalId?: string;
}

// ============================================================================
// REQUEST-OFFER MATCHING
// ============================================================================

/**
 * ServiceMatch - A potential or actual match between a request and offer.
 *
 * In REA terms, this becomes a Proposal that links request intent + offer intent.
 * The Proposal creates a container for negotiation/agreement.
 */
export interface ServiceMatch {
  /** Unique identifier */
  id: string;

  /** The request being matched */
  requestId: string;

  /** The offer being matched */
  offerId: string;

  /** Why are these matched? (common service types, compatible times, etc.) */
  matchReason: string;

  /** Confidence/quality of the match (0-100) */
  matchQuality: number;

  /** Shared service types between request and offer */
  sharedServiceTypes: string[];

  /** Compatible time preferences? */
  timeCompatible: boolean;

  /** Compatible interaction types? */
  interactionCompatible: boolean;

  /** Compatible mediums of exchange? */
  exchangeCompatible: boolean;

  // ─────────────────────────────────────────────────────────────────
  // Match Lifecycle
  // ─────────────────────────────────────────────────────────────────

  /** Status of this match */
  status: 'suggested' | 'contacted' | 'negotiating' | 'agreed' | 'completed' | 'rejected';

  /** When match was created (identified) */
  createdAt: string;

  /** When match was last updated */
  updatedAt: string;

  /** When agreement/commitment was made (if applicable) */
  agreedAt?: string;

  /** When work was completed (if applicable) */
  completedAt?: string;

  // ─────────────────────────────────────────────────────────────────
  // REA Integration
  // ─────────────────────────────────────────────────────────────────

  /** REA Proposal ID created from this match */
  proposalId?: string;

  /** REA Agreement ID if terms agreed */
  agreementId?: string;

  /** Commitment IDs if work ongoing */
  commitmentIds?: string[];

  /** Events created from this match */
  eventIds?: string[];

  /** Arbitrary metadata */
  metadata?: Record<string, unknown>;
}

// ============================================================================
// SAVED PREFERENCES & FAVORITES
// ============================================================================

/**
 * SavedRequest - User saved a request they're interested in.
 *
 * Allows users to bookmark requests without contacting immediately.
 */
export interface SavedRequest {
  /** Unique identifier */
  id: string;

  /** User who saved it */
  saverId: string;

  /** Request that was saved */
  requestId: string;

  /** Reason they saved it (optional) */
  reason?: string;

  /** When saved */
  savedAt: string;

  /** Did they contact the requester? */
  hasContacted: boolean;

  /** If contacted, when? */
  contactedAt?: string;
}

/**
 * SavedOffer - User saved an offer they're interested in.
 *
 * Allows users to bookmark offers without contacting immediately.
 */
export interface SavedOffer {
  /** Unique identifier */
  id: string;

  /** User who saved it */
  saverId: string;

  /** Offer that was saved */
  offerId: string;

  /** Reason they saved it (optional) */
  reason?: string;

  /** When saved */
  savedAt: string;

  /** Did they contact the offeror? */
  hasContacted: boolean;

  /** If contacted, when? */
  contactedAt?: string;
}

// ============================================================================
// ADMINISTRATIVE & STATUS
// ============================================================================

/**
 * ListingStatus - Administrative status of a request or offer.
 *
 * Different from request.status (active/archived/deleted).
 * This tracks administrative moderation state.
 */
export interface ListingAdminStatus {
  /** Unique identifier */
  id: string;

  /** What this status applies to */
  listingType: 'request' | 'offer';

  /** ID of the request or offer */
  listingId: string;

  /** Status of the listing */
  statusType:
    | 'pending' // Awaiting admin review
    | 'accepted' // Approved and visible
    | 'rejected' // Rejected by admin
    | 'suspended-temporarily' // Suspended with end date
    | 'suspended-indefinitely'; // Suspended without end date

  /** Reason for status (especially for suspended/rejected) */
  reason?: string;

  /** If temporarily suspended, when does suspension end? */
  suspendedUntil?: string;

  /** Who set this status */
  setByAdminId: string;

  /** When status was set */
  setAt: string;

  /** Notes about the status change */
  notes?: string;

  /** Arbitrary metadata */
  metadata?: Record<string, unknown>;
}

// Note: Types are exported inline with their type/interface declarations above.
