/**
 * Requests and Offers Service
 *
 * Service layer for peer-to-peer request/offer coordination in Shefa.
 *
 * Operationalizes the bulletin board pattern from /research/requests-and-offers
 * and integrates it with REA/ValueFlows for economic tracking.
 *
 * Core Workflows:
 * 1. Create Request/Offer (with preferences, service types, payment options)
 * 2. Search/Browse (filter by service type, time, location, etc.)
 * 3. Match (algorithmic or manual - suggest compatible pairs)
 * 4. Contact/Propose (initiate negotiation)
 * 5. Agree (create commitment, track agreement)
 * 6. Complete (mark work done, settle payment)
 *
 * Integration Points:
 * - EconomicService: Create events for request/offer lifecycle
 * - CommonsPool: Settlement of payments if mutual credit
 * - Intent/Proposal: REA integration for formal coordination
 * - Observer Protocol: Track completed work, reputation
 */

import { Injectable } from '@angular/core';

// @coverage: 59.7% (2026-02-05)

import { firstValueFrom } from 'rxjs';

import { EconomicEvent } from '@app/elohim/models/economic-event.model';
import {
  Intent,
  Proposal,
  Commitment,
  Measure,
  REAAction,
} from '@app/elohim/models/rea-bridge.model';
import {
  ServiceRequest,
  ServiceOffer,
  ServiceMatch,
  ServiceType,
  MediumOfExchange,
  UserPreferences,
  SavedRequest,
  SavedOffer,
  ListingAdminStatus,
  InteractionType,
} from '@app/shefa/models/requests-and-offers.model';

import { EconomicService } from './economic.service';

@Injectable({
  providedIn: 'root',
})
export class RequestsAndOffersService {
  constructor(private readonly economicService: EconomicService) {}

  // ============================================================================
  // REQUEST MANAGEMENT
  // ============================================================================

  /**
   * Create a service request.
   *
   * Creates:
   * - ServiceRequest (the listing)
   * - REA Intent (for economic coordination)
   * - EconomicEvent (request created)
   *
   * Request is initially pending admin approval.
   */
  async createRequest(
    requesterId: string,
    requestDetails: Omit<ServiceRequest, 'id' | 'requestNumber' | 'createdAt' | 'updatedAt'>
  ): Promise<{
    request: ServiceRequest;
    intent: Intent;
    createdEvent: EconomicEvent;
  }> {
    // Step 1: Validate request fields
    if (!requestDetails.title || requestDetails.title.trim().length === 0) {
      throw new Error('Request title is required');
    }
    if (!requestDetails.description || requestDetails.description.trim().length < 20) {
      throw new Error('Request description must be at least 20 characters');
    }
    if (!requestDetails.serviceTypeIds || requestDetails.serviceTypeIds.length === 0) {
      throw new Error('Request must specify at least one service type');
    }
    if (!requestDetails.mediumOfExchangeIds || requestDetails.mediumOfExchangeIds.length === 0) {
      throw new Error('Request must specify at least one payment method');
    }

    // Step 2: Create ServiceRequest entity
    const now = new Date().toISOString();
    const request: ServiceRequest = {
      id: generateRequestId(),
      requestNumber: `REQ-${Date.now().toString().slice(-10)}`,
      requesterId,
      title: requestDetails.title,
      description: requestDetails.description,
      contactPreference: requestDetails.contactPreference ?? 'email',
      contactValue: requestDetails.contactValue ?? '',
      timeZone: requestDetails.timeZone ?? 'UTC',
      timePreference: requestDetails.timePreference ?? 'any',
      interactionType: requestDetails.interactionType ?? 'virtual',
      dateRange: requestDetails.dateRange ?? { startDate: now, endDate: '', flexibleDates: false },
      serviceTypeIds: requestDetails.serviceTypeIds,
      requiredSkills: requestDetails.requiredSkills ?? [],
      budget: requestDetails.budget,
      mediumOfExchangeIds: requestDetails.mediumOfExchangeIds,
      status: 'active', // Note: 'pending' not in ServiceRequest status type
      isPublic: false, // Hidden until approved
      links: requestDetails.links ?? [],
      createdAt: now,
      updatedAt: now,
      // Intent properties
      action: 'take',
      receiver: requesterId,
      finished: false,
    };

    // Step 3: Create REA Intent for the request
    // ServiceRequest = Intent to take (receive) service
    const intent: Intent = {
      id: `intent-${request.id}`,
      action: 'take',
      receiver: requesterId,
      resourceConformsTo: requestDetails.serviceTypeIds.join('|'), // Multi-type
      resourceQuantity: request.budget?.amount, // Extract Measure from budget
      note: `Request for ${request.title}`,
      finished: false,
      classifiedAs: [request.id, request.requestNumber], // Use classifiedAs instead of metadata
    };

    // Step 4: Create 'service-request-created' EconomicEvent
    const createdEvent = await firstValueFrom(
      this.economicService.createEvent({
        action: 'transfer' as REAAction, // Note: 'propose' not in REAAction
        providerId: requesterId,
        receiverId: 'shefa-coordination',
        resourceConformsTo: 'service-request',
        note: `Service request created: ${request.title}. Requester: ${requesterId}. Services: ${requestDetails.serviceTypeIds.join(
          ', '
        )}. Request ID: ${request.id}. Request Number: ${request.requestNumber}.`,
        lamadEventType: 'content-create',
      })
    );

    // Step 5: Persist request (in production, to Holochain DHT)
    await this.persistRequest(request);

    return {
      request,
      intent,
      createdEvent,
    };
  }

  /**
   * Update a service request.
   *
   * Only requester can update their own request.
   * Creates new event for modification.
   */
  async updateRequest(
    requestId: string,
    requesterId: string,
    updates: Partial<ServiceRequest>
  ): Promise<{
    request: ServiceRequest;
    updatedEvent: EconomicEvent;
  }> {
    // Step 1: Get existing request
    const request = await this.getRequest(requestId);
    if (!request) {
      throw new Error(`Request ${requestId} not found`);
    }

    // Step 2: Verify requester owns this request
    if (request.requesterId !== requesterId) {
      throw new Error(`Only requester can update this request`);
    }

    // Step 3: Merge updates with existing request
    const updated: ServiceRequest = {
      ...request,
      ...updates,
      id: request.id, // Never update ID
      requestNumber: request.requestNumber, // Never update number
      requesterId: request.requesterId, // Never update requester
      createdAt: request.createdAt, // Never update created date
      updatedAt: new Date().toISOString(), // Update modification time
    };

    // Step 4: Create 'service-request-updated' EconomicEvent
    const updatedEvent = await firstValueFrom(
      this.economicService.createEvent({
        action: 'modify',
        providerId: requesterId,
        receiverId: 'shefa-coordination',
        note: `Service request updated: ${updated.title}. Changes: ${Object.keys(updates).join(', ')}. Request ID: ${request.id}.`,
        lamadEventType: 'content-flag',
      })
    );

    // Step 5: Persist updated request
    await this.persistRequest(updated);

    return {
      request: updated,
      updatedEvent,
    };
  }

  /**
   * Archive a request (soft delete).
   *
   * Hides it from search but keeps record.
   */
  async archiveRequest(requestId: string, requesterId: string): Promise<ServiceRequest> {
    // Get and verify ownership
    const request = await this.getRequest(requestId);
    if (!request) {
      throw new Error(`Request ${requestId} not found`);
    }
    if (request.requesterId !== requesterId) {
      throw new Error(`Only requester can archive this request`);
    }

    // Archive by setting status
    const archived: ServiceRequest = {
      ...request,
      status: 'archived',
      updatedAt: new Date().toISOString(),
    };

    // Create event
    await firstValueFrom(
      this.economicService.createEvent({
        action: 'raise',
        providerId: requesterId,
        receiverId: 'shefa-coordination',
        note: `Request archived: ${request.title}. Request ID: ${requestId}.`,
        lamadEventType: 'content-flag',
      })
    );

    await this.persistRequest(archived);
    return archived;
  }

  /**
   * Delete a request (admin only or requester within grace period).
   *
   * Hard delete if within grace period, soft otherwise.
   */
  async deleteRequest(requestId: string, requesterId: string): Promise<void> {
    const request = await this.getRequest(requestId);
    if (!request) {
      throw new Error(`Request ${requestId} not found`);
    }

    if (request.requesterId !== requesterId) {
      throw new Error(`Only requester can delete this request`);
    }

    // TODO: Implement hard/soft delete logic based on grace period
    // For now, just mark as deleted
    const deleted: ServiceRequest = {
      ...request,
      status: 'deleted',
      updatedAt: new Date().toISOString(),
    };

    await firstValueFrom(
      this.economicService.createEvent({
        action: 'modify',
        providerId: requesterId,
        receiverId: 'shefa-coordination',
        note: `Request deleted: ${request.title}. Request ID: ${requestId}.`,
        lamadEventType: 'content-flag',
      })
    );

    await this.persistRequest(deleted);
  }

  /**
   * Get a specific request.
   */
  async getRequest(_requestId: string): Promise<ServiceRequest | null> {
    // TODO: In production, fetch from Holochain DHT
    return null;
  }

  /**
   * Get all requests by a user.
   */
  async getUserRequests(
    _requesterId: string,
    _filters?: {
      status?: 'active' | 'archived' | 'deleted';
      fromDate?: string;
      toDate?: string;
    }
  ): Promise<ServiceRequest[]> {
    // TODO: In production, query DHT for requests where requesterId matches
    // Apply filters if provided
    return [];
  }

  // ============================================================================
  // OFFER MANAGEMENT
  // ============================================================================

  /**
   * Create a service offer.
   *
   * Creates:
   * - ServiceOffer (the listing)
   * - REA Intent (for economic coordination)
   * - EconomicEvent (offer created)
   *
   * Offer is initially pending admin approval.
   */
  async createOffer(
    offerorId: string,
    offerDetails: Omit<ServiceOffer, 'id' | 'offerNumber' | 'createdAt' | 'updatedAt'>
  ): Promise<{
    offer: ServiceOffer;
    intent: Intent;
    createdEvent: EconomicEvent;
  }> {
    // Step 1: Validate offer fields
    if (!offerDetails.title || offerDetails.title.trim().length === 0) {
      throw new Error('Offer title is required');
    }
    if (!offerDetails.description || offerDetails.description.trim().length < 20) {
      throw new Error('Offer description must be at least 20 characters');
    }
    if (!offerDetails.serviceTypeIds || offerDetails.serviceTypeIds.length === 0) {
      throw new Error('Offer must specify at least one service type');
    }
    if (!offerDetails.mediumOfExchangeIds || offerDetails.mediumOfExchangeIds.length === 0) {
      throw new Error('Offer must specify at least one payment method');
    }

    // Step 2: Create ServiceOffer entity
    const now = new Date().toISOString();
    const offer: ServiceOffer = {
      id: generateOfferId(),
      offerNumber: `OFR-${Date.now().toString().slice(-10)}`,
      offerorId,
      title: offerDetails.title,
      description: offerDetails.description,
      contactPreference: offerDetails.contactPreference ?? 'email',
      contactValue: offerDetails.contactValue ?? '',
      timeZone: offerDetails.timeZone ?? 'UTC',
      timePreference: offerDetails.timePreference ?? 'any',
      interactionType: offerDetails.interactionType ?? 'virtual',
      hoursPerWeek: offerDetails.hoursPerWeek ?? 40,
      dateRange: offerDetails.dateRange ?? { startDate: now, endDate: '', flexibleDates: false },
      serviceTypeIds: offerDetails.serviceTypeIds,
      offerredSkills: offerDetails.offerredSkills ?? [],
      rate: offerDetails.rate,
      mediumOfExchangeIds: offerDetails.mediumOfExchangeIds,
      acceptsAlternativePayment: offerDetails.acceptsAlternativePayment ?? false,
      status: 'active', // Note: 'pending' not in ServiceOffer status type
      isPublic: false, // Hidden until approved
      links: offerDetails.links ?? [],
      createdAt: now,
      updatedAt: now,
      // Intent properties
      action: 'give',
      provider: offerorId,
      finished: false,
    };

    // Step 3: Create REA Intent for the offer
    // ServiceOffer = Intent to give (provide) service
    const intent: Intent = {
      id: `intent-${offer.id}`,
      action: 'give',
      provider: offerorId,
      resourceConformsTo: offerDetails.serviceTypeIds.join('|'), // Multi-type
      resourceQuantity: offer.rate?.amount, // Extract Measure from rate
      note: `Offer for ${offer.title}`,
      finished: false,
      classifiedAs: [offer.id, offer.offerNumber], // Use classifiedAs instead of metadata
    };

    // Step 4: Create 'service-offer-created' EconomicEvent
    const createdEvent = await firstValueFrom(
      this.economicService.createEvent({
        action: 'transfer' as REAAction, // Note: 'propose' not in REAAction
        providerId: offerorId,
        receiverId: 'shefa-coordination',
        resourceConformsTo: 'service-offer',
        note: `Service offer created: ${offer.title}. Offeror: ${offerorId}. Services: ${offerDetails.serviceTypeIds.join(
          ', '
        )}. Offer ID: ${offer.id}. Offer Number: ${offer.offerNumber}.`,
        lamadEventType: 'content-create',
      })
    );

    // Step 5: Persist offer (in production, to Holochain DHT)
    await this.persistOffer(offer);

    return {
      offer,
      intent,
      createdEvent,
    };
  }

  /**
   * Update a service offer.
   *
   * Only offeror can update their own offer.
   */
  async updateOffer(
    offerId: string,
    offerorId: string,
    updates: Partial<ServiceOffer>
  ): Promise<{
    offer: ServiceOffer;
    updatedEvent: EconomicEvent;
  }> {
    // Step 1: Get existing offer
    const offer = await this.getOffer(offerId);
    if (!offer) {
      throw new Error(`Offer ${offerId} not found`);
    }

    // Step 2: Verify offeror owns this offer
    if (offer.offerorId !== offerorId) {
      throw new Error(`Only offeror can update this offer`);
    }

    // Step 3: Merge updates
    const updated: ServiceOffer = {
      ...offer,
      ...updates,
      id: offer.id,
      offerNumber: offer.offerNumber,
      offerorId: offer.offerorId,
      createdAt: offer.createdAt,
      updatedAt: new Date().toISOString(),
    };

    // Step 4: Create update event
    const updatedEvent = await firstValueFrom(
      this.economicService.createEvent({
        action: 'modify',
        providerId: offerorId,
        receiverId: 'shefa-coordination',
        note: `Service offer updated: ${updated.title}. Changes: ${Object.keys(updates).join(', ')}. Offer ID: ${offer.id}.`,
        lamadEventType: 'content-flag',
      })
    );

    // Step 5: Persist updated offer
    await this.persistOffer(updated);

    return {
      offer: updated,
      updatedEvent,
    };
  }

  /**
   * Archive an offer (soft delete).
   */
  async archiveOffer(offerId: string, offerorId: string): Promise<ServiceOffer> {
    const offer = await this.getOffer(offerId);
    if (!offer) {
      throw new Error(`Offer ${offerId} not found`);
    }
    if (offer.offerorId !== offerorId) {
      throw new Error(`Only offeror can archive this offer`);
    }

    const archived: ServiceOffer = {
      ...offer,
      status: 'archived',
      updatedAt: new Date().toISOString(),
    };

    await firstValueFrom(
      this.economicService.createEvent({
        action: 'raise',
        providerId: offerorId,
        receiverId: 'shefa-coordination',
        note: `Offer archived: ${offer.title}. Offer ID: ${offerId}.`,
        lamadEventType: 'content-flag',
      })
    );

    await this.persistOffer(archived);
    return archived;
  }

  /**
   * Delete an offer.
   */
  async deleteOffer(offerId: string, offerorId: string): Promise<void> {
    const offer = await this.getOffer(offerId);
    if (!offer) {
      throw new Error(`Offer ${offerId} not found`);
    }

    if (offer.offerorId !== offerorId) {
      throw new Error(`Only offeror can delete this offer`);
    }

    const deleted: ServiceOffer = {
      ...offer,
      status: 'deleted',
      updatedAt: new Date().toISOString(),
    };

    await firstValueFrom(
      this.economicService.createEvent({
        action: 'modify',
        providerId: offerorId,
        receiverId: 'shefa-coordination',
        note: `Offer deleted: ${offer.title}. Offer ID: ${offerId}.`,
        lamadEventType: 'content-flag',
      })
    );

    await this.persistOffer(deleted);
  }

  /**
   * Get a specific offer.
   */
  async getOffer(_offerId: string): Promise<ServiceOffer | null> {
    // TODO: In production, fetch from Holochain DHT
    return null;
  }

  /**
   * Get all offers by a user.
   */
  async getUserOffers(
    _offerorId: string,
    _filters?: {
      status?: 'active' | 'archived' | 'deleted';
      fromDate?: string;
      toDate?: string;
    }
  ): Promise<ServiceOffer[]> {
    // TODO: In production, query DHT for offers where offerorId matches
    return [];
  }

  // ============================================================================
  // SEARCH & DISCOVERY
  // ============================================================================

  /**
   * Search requests by criteria.
   *
   * Returns paginated results matching filter criteria.
   */
  async searchRequests(
    filters: {
      serviceTypeIds?: string[]; // Filter by service categories
      searchText?: string; // Text search (title + description)
      timeZone?: string; // Only show requests in this timezone
      interactionType?: InteractionType; // Virtual/InPerson/Hybrid
      minDate?: string; // Requests needed by this date
      maxDate?: string; // Requests available until this date
      budgetMin?: number; // Min budget
      budgetMax?: number; // Max budget
      mediumOfExchangeIds?: string[]; // Only show requests accepting these
    },
    pagination?: { page: number; pageSize: number }
  ): Promise<{
    requests: ServiceRequest[];
    totalCount: number;
    page: number;
    pageSize: number;
  }> {
    // TODO: In production, query Holochain DHT with multiple filters
    // 1. Filter by service types (any matching serviceTypeIds)
    // 2. Filter by text search (match title + description)
    // 3. Filter by timezone if provided
    // 4. Filter by interaction type if provided
    // 5. Filter by date range if provided
    // 6. Filter by budget range if provided
    // 7. Filter by accepted payment methods (any matching mediumOfExchangeIds)
    // 8. Only return requests with status='pending' (approved)
    // 9. Exclude archived/deleted
    // 10. Sort by recency (newest first)
    // 11. Apply pagination
    const page = pagination?.page ?? 1;
    const pageSize = pagination?.pageSize ?? 20;

    // Placeholder return for development
    return await Promise.resolve({
      requests: [],
      totalCount: 0,
      page,
      pageSize,
    });
  }

  /**
   * Search offers by criteria.
   *
   * Mirror of searchRequests for offers.
   */
  async searchOffers(
    filters: {
      serviceTypeIds?: string[];
      searchText?: string;
      timeZone?: string;
      interactionType?: InteractionType;
      rateMin?: number;
      rateMax?: number;
      mediumOfExchangeIds?: string[];
      hoursPerWeekMin?: number;
    },
    pagination?: { page: number; pageSize: number }
  ): Promise<{
    offers: ServiceOffer[];
    totalCount: number;
    page: number;
    pageSize: number;
  }> {
    // TODO: In production, query Holochain DHT with multiple filters
    // 1. Filter by service types (any matching)
    // 2. Filter by text search
    // 3. Filter by timezone if provided
    // 4. Filter by interaction type if provided
    // 5. Filter by hourly rate range if provided
    // 6. Filter by accepted payment methods
    // 7. Filter by minimum hours per week if provided
    // 8. Only return offers with status='pending' (approved)
    // 9. Exclude archived/deleted
    // 10. Sort by rate (lowest first) or newest
    // 11. Apply pagination
    const page = pagination?.page ?? 1;
    const pageSize = pagination?.pageSize ?? 20;

    return await Promise.resolve({
      offers: [],
      totalCount: 0,
      page,
      pageSize,
    });
  }

  /**
   * Get trending or featured requests.
   *
   * Return requests with high engagement (saves, contacts, etc).
   */
  async getTrendingRequests(_limit = 10): Promise<ServiceRequest[]> {
    // TODO: In production, query DHT for requests with highest:
    // - Number of matches suggested
    // - Number of saves
    // - Number of contacts received
    // - Recency (if tied)
    return [];
  }

  /**
   * Get trending or featured offers.
   */
  async getTrendingOffers(_limit = 10): Promise<ServiceOffer[]> {
    // TODO: In production, query DHT for offers with highest:
    // - Number of matches suggested
    // - Number of saves
    // - Number of contacts received
    // - Recency (if tied)
    return [];
  }

  // ============================================================================
  // MATCHING
  // ============================================================================

  /**
   * Find offers that match a request.
   *
   * Uses service type, time, interaction type, and exchange compatibility
   * to suggest matching offers.
   *
   * Returns ranked list of ServiceMatch objects.
   *
   * ⚠️ PLACEHOLDER: Matching algorithm requires alignment review with Elohim Protocol
   *
   * This is a critical component that should be reviewed for:
   * - Fairness in matching (avoiding preferential treatment)
   * - Transparency in scoring (explainable recommendations)
   * - Economic neutrality (not biasing toward payment methods)
   * - Privacy preservation (not leaking requester/offeror preferences)
   * - Accessibility (ensuring matches for lower-visibility listings)
   *
   * See: docs/alignment-reviews/service-matching-algorithm.md
   */
  async findMatchesForRequest(_requestId: string, _limit = 10): Promise<ServiceMatch[]> {
    // PLACEHOLDER IMPLEMENTATION
    // TODO: Implement matching algorithm with alignment review
    //
    // High-level approach (to be reviewed):
    // 1. Get the request
    // 2. Get all active offers (status='pending' = approved by admin)
    // 3. For each offer, calculate compatibility score:
    //    a. Service type overlap (exact match scores higher)
    //    b. Time preference compatibility (can both work together?)
    //    c. Interaction type compatibility (virtual vs in-person)
    //    d. Medium of exchange overlap (any payment method both accept)
    //    e. Budget vs rate alignment (if applicable)
    // 4. Filter to matches with minimum acceptable quality (e.g., >= 70)
    // 5. Sort by quality score (highest first)
    // 6. Return top N matches
    //
    // Must ensure:
    // - Score calculation is transparent and auditable
    // - No preferential treatment based on requester/offeror identity
    // - Privacy: don't leak requester's budget or timing constraints
    // - Fairness: all offers get equal consideration regardless of creator
    // - Inclusion: matching algorithm doesn't create systemic exclusions

    return [];
  }

  /**
   * Find requests that match an offer.
   *
   * Mirror of findMatchesForRequest for offers.
   * See ⚠️ notes on findMatchesForRequest about alignment review.
   */
  async findMatchesForOffer(_offerId: string, _limit = 10): Promise<ServiceMatch[]> {
    // PLACEHOLDER IMPLEMENTATION
    // TODO: Implement matching algorithm with alignment review
    // See findMatchesForRequest for design notes and governance requirements

    return [];
  }

  /**
   * Create a match between request and offer (manual matching).
   *
   * Admin or matching algorithm can suggest a match.
   */
  async createMatch(
    _requestId: string,
    _offerId: string,
    _matchReason: string
  ): Promise<ServiceMatch> {
    // TODO: Implementation
    // 1. Get request and offer
    // 2. Analyze compatibility
    // 3. Create ServiceMatch
    // 4. Create 'service-match-suggested' event
    // 5. Return match
    return await Promise.reject(new Error('Not yet implemented'));
  }

  /**
   * Get match details.
   */
  async getMatch(_matchId: string): Promise<ServiceMatch | null> {
    // TODO: Implementation
    return await Promise.reject(new Error('Not yet implemented'));
  }

  /**
   * Update match status (contacted, negotiating, agreed, etc).
   */
  async updateMatchStatus(
    _matchId: string,
    _status: ServiceMatch['status']
  ): Promise<ServiceMatch> {
    // TODO: Implementation
    return await Promise.reject(new Error('Not yet implemented'));
  }

  // ============================================================================
  // COORDINATION & PROPOSAL
  // ============================================================================

  /**
   * Initiate contact/proposal from offer to request.
   *
   * Offeror responding to a request with their offer.
   *
   * Creates:
   * - ServiceMatch
   * - REA Proposal (linking request intent + offer intent)
   * - Contact event (for audit trail)
   */
  async proposeOfferToRequest(
    _offerId: string,
    _requestId: string,
    _offerorMessage?: string
  ): Promise<{
    match: ServiceMatch;
    proposal: Proposal;
    contactEvent: EconomicEvent;
  }> {
    // TODO: Implementation
    // 1. Get offer and request
    // 2. Verify offer/request still active and accepted
    // 3. Check if already matched
    // 4. Create ServiceMatch
    // 5. Create REA Proposal linking request intent + offer intent
    // 6. Create 'service-proposal-created' event
    // 7. Mark as 'contacted' in match
    // 8. Return match, proposal, event
    return await Promise.reject(new Error('Not yet implemented'));
  }

  /**
   * Initiate contact/proposal from request to offer.
   *
   * Requester responding to an offer with their request.
   */
  async proposeRequestToOffer(
    _requestId: string,
    _offerId: string,
    _requesterMessage?: string
  ): Promise<{
    match: ServiceMatch;
    proposal: Proposal;
    contactEvent: EconomicEvent;
  }> {
    // TODO: Implementation
    return await Promise.reject(new Error('Not yet implemented'));
  }

  /**
   * Accept a proposal (agree to terms).
   *
   * Creates:
   * - REA Commitment (to fulfill the work)
   * - REA Agreement (terms of the work)
   * - 'service-agreed' event
   */
  async acceptProposal(
    _proposalId: string,
    _acceptedById: string,
    _agreedTerms?: {
      rate: Measure;
      schedule: string;
      deliverables: string;
    }
  ): Promise<{
    proposal: Proposal;
    commitment: Commitment;
    agreementEvent: EconomicEvent;
  }> {
    // TODO: Implementation
    // 1. Get proposal (request intent + offer intent)
    // 2. Verify both parties present in proposal
    // 3. Create REA Commitment from proposal
    // 4. Create REA Agreement with terms
    // 5. Create 'service-agreed' event
    // 6. Update match status to 'agreed'
    // 7. Return proposal, commitment, event
    return await Promise.reject(new Error('Not yet implemented'));
  }

  /**
   * Reject a proposal.
   *
   * Either party can reject the proposal.
   */
  async rejectProposal(
    _proposalId: string,
    _rejectedById: string,
    _reason?: string
  ): Promise<Proposal> {
    // TODO: Implementation
    return await Promise.reject(new Error('Not yet implemented'));
  }

  // ============================================================================
  // COMPLETION & SETTLEMENT
  // ============================================================================

  /**
   * Mark work as complete.
   *
   * One party submits that work is done. Other can accept or dispute.
   *
   * Creates:
   * - 'service-work-completed' event
   * - Triggers payment settlement workflow
   */
  async markWorkComplete(
    _commitmentId: string,
    _completedById: string,
    _deliverables?: {
      description: string;
      links: string[];
    }
  ): Promise<{
    commitment: Commitment;
    completionEvent: EconomicEvent;
  }> {
    // TODO: Implementation
    // 1. Get commitment
    // 2. Verify completer is one of the parties
    // 3. Create 'service-work-completed' event
    // 4. Trigger settlement workflow (payment collection)
    // 5. Return commitment and event
    return await Promise.reject(new Error('Not yet implemented'));
  }

  /**
   * Settle payment for completed work.
   *
   * Facilitates payment from requester to offeror.
   * Can be mutual credit, fiat transfer, or barter.
   *
   * Creates:
   * - EconomicEvent (payment transfer)
   * - Updates CommonsPool if mutual credit
   * - May create recognition/reputation event
   */
  async settlePayment(
    _matchId: string,
    _paymentDetails: {
      amount: Measure;
      mediumOfExchangeId: string;
      paymentMethod: 'mutual-credit' | 'fiat-transfer' | 'barter';
      note?: string;
    }
  ): Promise<{
    settlement: EconomicEvent;
    reputation?: EconomicEvent; // If creating reputation flow
  }> {
    // TODO: Implementation
    // 1. Get match, request, offer
    // 2. Validate payment amount against offer rate
    // 3. Create payment event
    // 4. If mutual credit: deduct from requester, add to offeror (via CommonsPool or direct transfer)
    // 5. Create reputation/recognition event
    // 6. Mark match as 'completed'
    // 7. Return settlement event
    return await Promise.reject(new Error('Not yet implemented'));
  }

  // ============================================================================
  // PREFERENCES & SETTINGS
  // ============================================================================

  /**
   * Set user preferences.
   *
   * How they like to be contacted, when they work, where they are.
   */
  async setUserPreferences(
    _userId: string,
    _preferences: Omit<UserPreferences, 'id' | 'userId' | 'updatedAt'>
  ): Promise<UserPreferences> {
    // TODO: Implementation
    // 1. Create or update UserPreferences
    // 2. Return preferences
    return await Promise.reject(new Error('Not yet implemented'));
  }

  /**
   * Get user preferences.
   */
  async getUserPreferences(_userId: string): Promise<UserPreferences | null> {
    // TODO: Implementation
    return await Promise.reject(new Error('Not yet implemented'));
  }

  /**
   * Get recommended requests for user based on their preferences.
   *
   * Use their preferences (time zone, service types, skills) to suggest requests.
   */
  async getRecommendedRequests(_userId: string, _limit = 10): Promise<ServiceRequest[]> {
    // TODO: Implementation
    return await Promise.reject(new Error('Not yet implemented'));
  }

  /**
   * Get recommended offers for user based on their needs.
   */
  async getRecommendedOffers(_userId: string, _limit = 10): Promise<ServiceOffer[]> {
    // TODO: Implementation
    return await Promise.reject(new Error('Not yet implemented'));
  }

  // ============================================================================
  // FAVORITES
  // ============================================================================

  /**
   * Save a request as favorite.
   *
   * User wants to remember this for later.
   */
  async saveRequest(_userId: string, _requestId: string, _reason?: string): Promise<SavedRequest> {
    // TODO: Implementation
    return await Promise.reject(new Error('Not yet implemented'));
  }

  /**
   * Save an offer as favorite.
   */
  async saveOffer(_userId: string, _offerId: string, _reason?: string): Promise<SavedOffer> {
    // TODO: Implementation
    return await Promise.reject(new Error('Not yet implemented'));
  }

  /**
   * Get user's saved requests.
   */
  async getSavedRequests(_userId: string): Promise<SavedRequest[]> {
    // TODO: Implementation
    return await Promise.reject(new Error('Not yet implemented'));
  }

  /**
   * Get user's saved offers.
   */
  async getSavedOffers(_userId: string): Promise<SavedOffer[]> {
    // TODO: Implementation
    return await Promise.reject(new Error('Not yet implemented'));
  }

  /**
   * Unsave a request.
   */
  async unsaveRequest(_userId: string, _savedRequestId: string): Promise<void> {
    // TODO: Implementation
    return await Promise.reject(new Error('Not yet implemented'));
  }

  /**
   * Unsave an offer.
   */
  async unsaveOffer(_userId: string, _savedOfferId: string): Promise<void> {
    // TODO: Implementation
    return await Promise.reject(new Error('Not yet implemented'));
  }

  // ============================================================================
  // SERVICE TYPES (CATEGORIZATION)
  // ============================================================================

  /**
   * Create a service type.
   *
   * Used to categorize requests/offers (e.g., "Logo Design", "Tutoring").
   */
  async createServiceType(
    _creatorId: string,
    _typeDetails: Omit<ServiceType, 'id' | 'createdAt' | 'status'>
  ): Promise<ServiceType> {
    // TODO: Implementation
    return await Promise.reject(new Error('Not yet implemented'));
  }

  /**
   * Update a service type (creator only).
   */
  async updateServiceType(
    _serviceTypeId: string,
    _creatorId: string,
    _updates: Partial<ServiceType>
  ): Promise<ServiceType> {
    // TODO: Implementation
    return await Promise.reject(new Error('Not yet implemented'));
  }

  /**
   * Get all active service types.
   */
  async getServiceTypes(_filters?: { isTechnical?: boolean }): Promise<ServiceType[]> {
    // TODO: Implementation
    return await Promise.reject(new Error('Not yet implemented'));
  }

  /**
   * Get service type by ID.
   */
  async getServiceType(_id: string): Promise<ServiceType | null> {
    // TODO: Implementation
    return await Promise.reject(new Error('Not yet implemented'));
  }

  // ============================================================================
  // MEDIUMS OF EXCHANGE
  // ============================================================================

  /**
   * Create a medium of exchange.
   *
   * Define what can be paid with (EUR, USD, Hours, Care Tokens, etc).
   */
  async createMediumOfExchange(
    _creatorId: string,
    _mediumDetails: Omit<MediumOfExchange, 'id' | 'createdAt' | 'status'>
  ): Promise<MediumOfExchange> {
    // TODO: Implementation
    return await Promise.reject(new Error('Not yet implemented'));
  }

  /**
   * Update a medium of exchange (creator only).
   */
  async updateMediumOfExchange(
    _mediumId: string,
    _creatorId: string,
    _updates: Partial<MediumOfExchange>
  ): Promise<MediumOfExchange> {
    // TODO: Implementation
    return await Promise.reject(new Error('Not yet implemented'));
  }

  /**
   * Get all active mediums of exchange.
   */
  async getMediumsOfExchange(_filters?: {
    exchangeType?: MediumOfExchange['exchangeType'];
  }): Promise<MediumOfExchange[]> {
    // TODO: Implementation
    return await Promise.reject(new Error('Not yet implemented'));
  }

  /**
   * Get medium of exchange by ID.
   */
  async getMediumOfExchange(_id: string): Promise<MediumOfExchange | null> {
    // TODO: Implementation
    return await Promise.reject(new Error('Not yet implemented'));
  }

  // ============================================================================
  // ADMINISTRATION & MODERATION
  // ============================================================================

  /**
   * Get requests pending admin approval.
   */
  async getPendingRequests(): Promise<ServiceRequest[]> {
    // TODO: Implementation
    return await Promise.reject(new Error('Not yet implemented'));
  }

  /**
   * Get offers pending admin approval.
   */
  async getPendingOffers(): Promise<ServiceOffer[]> {
    // TODO: Implementation
    return await Promise.reject(new Error('Not yet implemented'));
  }

  /**
   * Approve a request (admin only).
   *
   * Sets admin status to 'accepted' so it becomes visible.
   */
  async approveRequest(
    _requestId: string,
    _adminId: string
  ): Promise<{
    request: ServiceRequest;
    adminStatus: ListingAdminStatus;
  }> {
    // TODO: Implementation
    return await Promise.reject(new Error('Not yet implemented'));
  }

  /**
   * Approve an offer (admin only).
   */
  async approveOffer(
    _offerId: string,
    _adminId: string
  ): Promise<{
    offer: ServiceOffer;
    adminStatus: ListingAdminStatus;
  }> {
    // TODO: Implementation
    return await Promise.reject(new Error('Not yet implemented'));
  }

  /**
   * Reject a request (admin only).
   *
   * Sets admin status to 'rejected' with reason.
   */
  async rejectRequest(
    _requestId: string,
    _adminId: string,
    _reason: string
  ): Promise<{
    request: ServiceRequest;
    adminStatus: ListingAdminStatus;
  }> {
    // TODO: Implementation
    return await Promise.reject(new Error('Not yet implemented'));
  }

  /**
   * Reject an offer (admin only).
   */
  async rejectOffer(
    _offerId: string,
    _adminId: string,
    _reason: string
  ): Promise<{
    offer: ServiceOffer;
    adminStatus: ListingAdminStatus;
  }> {
    // TODO: Implementation
    return await Promise.reject(new Error('Not yet implemented'));
  }

  /**
   * Suspend a request temporarily (admin).
   */
  async suspendRequest(
    _requestId: string,
    _adminId: string,
    _reason: string,
    _suspendUntil?: string
  ): Promise<ListingAdminStatus> {
    // TODO: Implementation
    return await Promise.reject(new Error('Not yet implemented'));
  }

  /**
   * Suspend an offer temporarily (admin).
   */
  async suspendOffer(
    _offerId: string,
    _adminId: string,
    _reason: string,
    _suspendUntil?: string
  ): Promise<ListingAdminStatus> {
    // TODO: Implementation
    return await Promise.reject(new Error('Not yet implemented'));
  }

  // ============================================================================
  // ANALYTICS & REPORTING
  // ============================================================================

  /**
   * Get activity statistics.
   *
   * How many requests/offers, matches, completed, etc.
   */
  async getActivityStats(_period: '7-days' | '30-days' | 'all-time'): Promise<{
    totalRequests: number;
    totalOffers: number;
    totalMatches: number;
    completedMatches: number;
    totalValueExchanged: Measure;
    mostActiveServiceTypes: string[];
  }> {
    // TODO: Implementation
    return await Promise.reject(new Error('Not yet implemented'));
  }

  /**
   * Get user activity summary.
   *
   * How many requests/offers user has, matches, completed, reputation.
   */
  async getUserActivitySummary(_userId: string): Promise<{
    requestsCreated: number;
    offersCreated: number;
    matchesInitiated: number;
    matchesCompleted: number;
    totalValueExchanged: Measure;
    reputation: number;
    averageRating: number;
  }> {
    // TODO: Implementation
    return await Promise.reject(new Error('Not yet implemented'));
  }

  // ============================================================================
  // HELPER METHODS
  // ============================================================================

  private async persistRequest(_request: ServiceRequest): Promise<void> {
    // TODO: Persist to Holochain DHT
  }

  private async persistOffer(_offer: ServiceOffer): Promise<void> {
    // TODO: Persist to Holochain DHT
  }
}

// ============================================================================
// STANDALONE HELPER FUNCTIONS
// ============================================================================

/**
 * Generate a unique ID for a service request.
 * Uses timestamp + random suffix.
 * In production, would use UUID or Holochain hash.
 */
function generateRequestId(): string {
  const timestamp = Date.now().toString(36);
  const random = (crypto.getRandomValues(new Uint32Array(1))[0] / 2 ** 32)
    .toString(36)
    .substring(2, 9);
  return `req-${timestamp}-${random}`;
}

/**
 * Generate a unique ID for a service offer.
 */
function generateOfferId(): string {
  const timestamp = Date.now().toString(36);
  const random = (crypto.getRandomValues(new Uint32Array(1))[0] / 2 ** 32)
    .toString(36)
    .substring(2, 9);
  return `off-${timestamp}-${random}`;
}
