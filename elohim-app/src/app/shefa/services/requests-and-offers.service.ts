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

import { Observable } from 'rxjs';

import {
  EconomicEvent,
  CreateEventRequest,
  LamadEventType,
} from '@app/elohim/models/economic-event.model';
import {
  Intent,
  Proposal,
  Commitment,
  REAAgent,
  Measure,
  GovernanceLayer,
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
  ContactPreference,
  TimePreference,
  InteractionType,
  DateRange,
} from '@app/shefa/models/requests-and-offers.model';

import { EconomicService } from './economic.service';

@Injectable({
  providedIn: 'root',
})
export class RequestsAndOffersService {
  constructor(private economicService: EconomicService) {}

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
      contactPreference: requestDetails.contactPreference || 'email',
      contactValue: requestDetails.contactValue || '',
      timeZone: requestDetails.timeZone || 'UTC',
      timePreference: requestDetails.timePreference || 'any',
      interactionType: requestDetails.interactionType || 'virtual',
      dateRange: requestDetails.dateRange || { startDate: now, endDate: '' },
      serviceTypeIds: requestDetails.serviceTypeIds,
      requiredSkills: requestDetails.requiredSkills || [],
      budget: requestDetails.budget,
      mediumOfExchangeIds: requestDetails.mediumOfExchangeIds,
      status: 'pending', // Awaiting admin approval
      isPublic: false, // Hidden until approved
      links: requestDetails.links || [],
      createdAt: now,
      updatedAt: now,
    };

    // Step 3: Create REA Intent for the request
    // ServiceRequest = Intent to take (receive) service
    const intent: Intent = {
      id: `intent-${request.id}`,
      action: 'take',
      receiver: requesterId,
      resourceConformsTo: requestDetails.serviceTypeIds.join('|'), // Multi-type
      resourceQuantity: request.budget,
      note: `Request for ${request.title}`,
      metadata: {
        requestId: request.id,
        requestNumber: request.requestNumber,
      },
    };

    // Step 4: Create 'service-request-created' EconomicEvent
    const createdEvent = await this.economicService.createEvent({
      action: 'propose',
      provider: requesterId,
      receiver: 'shefa-coordination',
      resourceConformsTo: 'service-request',
      hasPointInTime: now,
      state: 'proposed',
      note: `Service request created: ${request.title}. Requester: ${requesterId}. Services: ${requestDetails.serviceTypeIds.join(', ')}`,
      metadata: {
        lamadEventType: 'service-request-created',
        requestId: request.id,
        requestNumber: request.requestNumber,
        requesterId,
        serviceTypeIds: requestDetails.serviceTypeIds,
        status: 'pending',
      },
    });

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
    const updatedEvent = await this.economicService.createEvent({
      action: 'modify',
      provider: requesterId,
      receiver: 'shefa-coordination',
      hasPointInTime: new Date().toISOString(),
      state: 'validated',
      note: `Service request updated: ${updated.title}. Changes: ${Object.keys(updates).join(', ')}`,
      metadata: {
        lamadEventType: 'service-request-updated',
        requestId: request.id,
        requestNumber: request.requestNumber,
        requesterId,
        changedFields: Object.keys(updates),
      },
    });

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
    await this.economicService.createEvent({
      action: 'raise',
      provider: requesterId,
      receiver: 'shefa-coordination',
      hasPointInTime: new Date().toISOString(),
      state: 'validated',
      note: `Request archived: ${request.title}`,
      metadata: {
        lamadEventType: 'service-request-archived',
        requestId,
      },
    });

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

    await this.economicService.createEvent({
      action: 'modify',
      provider: requesterId,
      receiver: 'shefa-coordination',
      hasPointInTime: new Date().toISOString(),
      state: 'validated',
      note: `Request deleted: ${request.title}`,
      metadata: {
        lamadEventType: 'service-request-deleted',
        requestId,
      },
    });

    await this.persistRequest(deleted);
  }

  /**
   * Get a specific request.
   */
  async getRequest(requestId: string): Promise<ServiceRequest | null> {
    // TODO: In production, fetch from Holochain DHT
    console.log('Fetching request:', requestId);
    return null;
  }

  /**
   * Get all requests by a user.
   */
  async getUserRequests(
    requesterId: string,
    filters?: {
      status?: 'active' | 'archived' | 'deleted';
      fromDate?: string;
      toDate?: string;
    }
  ): Promise<ServiceRequest[]> {
    // TODO: In production, query DHT for requests where requesterId matches
    // Apply filters if provided
    console.log(`Fetching requests for user ${requesterId}`, filters);
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
      contactPreference: offerDetails.contactPreference || 'email',
      contactValue: offerDetails.contactValue || '',
      timeZone: offerDetails.timeZone || 'UTC',
      timePreference: offerDetails.timePreference || 'any',
      interactionType: offerDetails.interactionType || 'virtual',
      hoursPerWeek: offerDetails.hoursPerWeek || 40,
      dateRange: offerDetails.dateRange || { startDate: now, endDate: '' },
      serviceTypeIds: offerDetails.serviceTypeIds,
      offeredSkills: offerDetails.offeredSkills || [],
      rate: offerDetails.rate,
      mediumOfExchangeIds: offerDetails.mediumOfExchangeIds,
      acceptsAlternativePayment: offerDetails.acceptsAlternativePayment || false,
      status: 'pending', // Awaiting admin approval
      isPublic: false, // Hidden until approved
      links: offerDetails.links || [],
      createdAt: now,
      updatedAt: now,
    };

    // Step 3: Create REA Intent for the offer
    // ServiceOffer = Intent to give (provide) service
    const intent: Intent = {
      id: `intent-${offer.id}`,
      action: 'give',
      provider: offerorId,
      resourceConformsTo: offerDetails.serviceTypeIds.join('|'), // Multi-type
      resourceQuantity: offer.rate,
      note: `Offer for ${offer.title}`,
      metadata: {
        offerId: offer.id,
        offerNumber: offer.offerNumber,
      },
    };

    // Step 4: Create 'service-offer-created' EconomicEvent
    const createdEvent = await this.economicService.createEvent({
      action: 'propose',
      provider: offerorId,
      receiver: 'shefa-coordination',
      resourceConformsTo: 'service-offer',
      resourceQuantity: offer.rate,
      hasPointInTime: now,
      state: 'proposed',
      note: `Service offer created: ${offer.title}. Offeror: ${offerorId}. Services: ${offerDetails.serviceTypeIds.join(', ')}. Rate: ${offer.rate.amount.value} ${offer.rate.amount.unit}/${offer.rate.per}`,
      metadata: {
        lamadEventType: 'service-offer-created',
        offerId: offer.id,
        offerNumber: offer.offerNumber,
        offerorId,
        serviceTypeIds: offerDetails.serviceTypeIds,
        rate: offer.rate.amount.value,
        status: 'pending',
      },
    });

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
    const updatedEvent = await this.economicService.createEvent({
      action: 'modify',
      provider: offerorId,
      receiver: 'shefa-coordination',
      hasPointInTime: new Date().toISOString(),
      state: 'validated',
      note: `Service offer updated: ${updated.title}. Changes: ${Object.keys(updates).join(', ')}`,
      metadata: {
        lamadEventType: 'service-offer-updated',
        offerId: offer.id,
        offerNumber: offer.offerNumber,
        offerorId,
        changedFields: Object.keys(updates),
      },
    });

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

    await this.economicService.createEvent({
      action: 'raise',
      provider: offerorId,
      receiver: 'shefa-coordination',
      hasPointInTime: new Date().toISOString(),
      state: 'validated',
      note: `Offer archived: ${offer.title}`,
      metadata: {
        lamadEventType: 'service-offer-archived',
        offerId,
      },
    });

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

    await this.economicService.createEvent({
      action: 'modify',
      provider: offerorId,
      receiver: 'shefa-coordination',
      hasPointInTime: new Date().toISOString(),
      state: 'validated',
      note: `Offer deleted: ${offer.title}`,
      metadata: {
        lamadEventType: 'service-offer-deleted',
        offerId,
      },
    });

    await this.persistOffer(deleted);
  }

  /**
   * Get a specific offer.
   */
  async getOffer(offerId: string): Promise<ServiceOffer | null> {
    // TODO: In production, fetch from Holochain DHT
    console.log('Fetching offer:', offerId);
    return null;
  }

  /**
   * Get all offers by a user.
   */
  async getUserOffers(
    offerorId: string,
    filters?: {
      status?: 'active' | 'archived' | 'deleted';
      fromDate?: string;
      toDate?: string;
    }
  ): Promise<ServiceOffer[]> {
    // TODO: In production, query DHT for offers where offerorId matches
    console.log(`Fetching offers for user ${offerorId}`, filters);
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
    const page = pagination?.page || 1;
    const pageSize = pagination?.pageSize || 20;

    console.log('Searching requests with filters:', filters);
    console.log(`Pagination: page ${page}, size ${pageSize}`);

    // Placeholder return for development
    return {
      requests: [],
      totalCount: 0,
      page,
      pageSize,
    };
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
    const page = pagination?.page || 1;
    const pageSize = pagination?.pageSize || 20;

    console.log('Searching offers with filters:', filters);
    console.log(`Pagination: page ${page}, size ${pageSize}`);

    return {
      offers: [],
      totalCount: 0,
      page,
      pageSize,
    };
  }

  /**
   * Get trending or featured requests.
   *
   * Return requests with high engagement (saves, contacts, etc).
   */
  async getTrendingRequests(limit = 10): Promise<ServiceRequest[]> {
    // TODO: In production, query DHT for requests with highest:
    // - Number of matches suggested
    // - Number of saves
    // - Number of contacts received
    // - Recency (if tied)
    console.log(`Fetching top ${limit} trending requests`);
    return [];
  }

  /**
   * Get trending or featured offers.
   */
  async getTrendingOffers(limit = 10): Promise<ServiceOffer[]> {
    // TODO: In production, query DHT for offers with highest:
    // - Number of matches suggested
    // - Number of saves
    // - Number of contacts received
    // - Recency (if tied)
    console.log(`Fetching top ${limit} trending offers`);
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
  async findMatchesForRequest(requestId: string, limit = 10): Promise<ServiceMatch[]> {
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

    console.log(
      `Finding matches for request ${requestId}. Limit: ${limit}. PLACEHOLDER IMPLEMENTATION.`
    );
    return [];
  }

  /**
   * Find requests that match an offer.
   *
   * Mirror of findMatchesForRequest for offers.
   * See ⚠️ notes on findMatchesForRequest about alignment review.
   */
  async findMatchesForOffer(offerId: string, limit = 10): Promise<ServiceMatch[]> {
    // PLACEHOLDER IMPLEMENTATION
    // TODO: Implement matching algorithm with alignment review
    // See findMatchesForRequest for design notes and governance requirements

    console.log(
      `Finding matches for offer ${offerId}. Limit: ${limit}. PLACEHOLDER IMPLEMENTATION.`
    );
    return [];
  }

  /**
   * Create a match between request and offer (manual matching).
   *
   * Admin or matching algorithm can suggest a match.
   */
  async createMatch(
    requestId: string,
    offerId: string,
    matchReason: string
  ): Promise<ServiceMatch> {
    // TODO: Implementation
    // 1. Get request and offer
    // 2. Analyze compatibility
    // 3. Create ServiceMatch
    // 4. Create 'service-match-suggested' event
    // 5. Return match
    throw new Error('Not yet implemented');
  }

  /**
   * Get match details.
   */
  async getMatch(matchId: string): Promise<ServiceMatch | null> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  /**
   * Update match status (contacted, negotiating, agreed, etc).
   */
  async updateMatchStatus(matchId: string, status: ServiceMatch['status']): Promise<ServiceMatch> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
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
    offerId: string,
    requestId: string,
    offerorMessage?: string
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
    throw new Error('Not yet implemented');
  }

  /**
   * Initiate contact/proposal from request to offer.
   *
   * Requester responding to an offer with their request.
   */
  async proposeRequestToOffer(
    requestId: string,
    offerId: string,
    requesterMessage?: string
  ): Promise<{
    match: ServiceMatch;
    proposal: Proposal;
    contactEvent: EconomicEvent;
  }> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
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
    proposalId: string,
    acceptedById: string,
    agreedTerms?: {
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
    throw new Error('Not yet implemented');
  }

  /**
   * Reject a proposal.
   *
   * Either party can reject the proposal.
   */
  async rejectProposal(
    proposalId: string,
    rejectedById: string,
    reason?: string
  ): Promise<Proposal> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
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
    commitmentId: string,
    completedById: string,
    deliverables?: {
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
    throw new Error('Not yet implemented');
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
    matchId: string,
    paymentDetails: {
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
    throw new Error('Not yet implemented');
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
    userId: string,
    preferences: Omit<UserPreferences, 'id' | 'userId' | 'updatedAt'>
  ): Promise<UserPreferences> {
    // TODO: Implementation
    // 1. Create or update UserPreferences
    // 2. Return preferences
    throw new Error('Not yet implemented');
  }

  /**
   * Get user preferences.
   */
  async getUserPreferences(userId: string): Promise<UserPreferences | null> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  /**
   * Get recommended requests for user based on their preferences.
   *
   * Use their preferences (time zone, service types, skills) to suggest requests.
   */
  async getRecommendedRequests(userId: string, limit = 10): Promise<ServiceRequest[]> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  /**
   * Get recommended offers for user based on their needs.
   */
  async getRecommendedOffers(userId: string, limit = 10): Promise<ServiceOffer[]> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  // ============================================================================
  // FAVORITES
  // ============================================================================

  /**
   * Save a request as favorite.
   *
   * User wants to remember this for later.
   */
  async saveRequest(userId: string, requestId: string, reason?: string): Promise<SavedRequest> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  /**
   * Save an offer as favorite.
   */
  async saveOffer(userId: string, offerId: string, reason?: string): Promise<SavedOffer> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  /**
   * Get user's saved requests.
   */
  async getSavedRequests(userId: string): Promise<SavedRequest[]> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  /**
   * Get user's saved offers.
   */
  async getSavedOffers(userId: string): Promise<SavedOffer[]> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  /**
   * Unsave a request.
   */
  async unsaveRequest(userId: string, savedRequestId: string): Promise<void> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  /**
   * Unsave an offer.
   */
  async unsaveOffer(userId: string, savedOfferId: string): Promise<void> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
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
    creatorId: string,
    typeDetails: Omit<ServiceType, 'id' | 'createdAt' | 'status'>
  ): Promise<ServiceType> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  /**
   * Update a service type (creator only).
   */
  async updateServiceType(
    serviceTypeId: string,
    creatorId: string,
    updates: Partial<ServiceType>
  ): Promise<ServiceType> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  /**
   * Get all active service types.
   */
  async getServiceTypes(filters?: { isTechnical?: boolean }): Promise<ServiceType[]> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  /**
   * Get service type by ID.
   */
  async getServiceType(id: string): Promise<ServiceType | null> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
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
    creatorId: string,
    mediumDetails: Omit<MediumOfExchange, 'id' | 'createdAt' | 'status'>
  ): Promise<MediumOfExchange> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  /**
   * Update a medium of exchange (creator only).
   */
  async updateMediumOfExchange(
    mediumId: string,
    creatorId: string,
    updates: Partial<MediumOfExchange>
  ): Promise<MediumOfExchange> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  /**
   * Get all active mediums of exchange.
   */
  async getMediumsOfExchange(filters?: {
    exchangeType?: MediumOfExchange['exchangeType'];
  }): Promise<MediumOfExchange[]> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  /**
   * Get medium of exchange by ID.
   */
  async getMediumOfExchange(id: string): Promise<MediumOfExchange | null> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  // ============================================================================
  // ADMINISTRATION & MODERATION
  // ============================================================================

  /**
   * Get requests pending admin approval.
   */
  async getPendingRequests(): Promise<ServiceRequest[]> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  /**
   * Get offers pending admin approval.
   */
  async getPendingOffers(): Promise<ServiceOffer[]> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  /**
   * Approve a request (admin only).
   *
   * Sets admin status to 'accepted' so it becomes visible.
   */
  async approveRequest(
    requestId: string,
    adminId: string
  ): Promise<{
    request: ServiceRequest;
    adminStatus: ListingAdminStatus;
  }> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  /**
   * Approve an offer (admin only).
   */
  async approveOffer(
    offerId: string,
    adminId: string
  ): Promise<{
    offer: ServiceOffer;
    adminStatus: ListingAdminStatus;
  }> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  /**
   * Reject a request (admin only).
   *
   * Sets admin status to 'rejected' with reason.
   */
  async rejectRequest(
    requestId: string,
    adminId: string,
    reason: string
  ): Promise<{
    request: ServiceRequest;
    adminStatus: ListingAdminStatus;
  }> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  /**
   * Reject an offer (admin only).
   */
  async rejectOffer(
    offerId: string,
    adminId: string,
    reason: string
  ): Promise<{
    offer: ServiceOffer;
    adminStatus: ListingAdminStatus;
  }> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  /**
   * Suspend a request temporarily (admin).
   */
  async suspendRequest(
    requestId: string,
    adminId: string,
    reason: string,
    suspendUntil?: string
  ): Promise<ListingAdminStatus> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  /**
   * Suspend an offer temporarily (admin).
   */
  async suspendOffer(
    offerId: string,
    adminId: string,
    reason: string,
    suspendUntil?: string
  ): Promise<ListingAdminStatus> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  // ============================================================================
  // ANALYTICS & REPORTING
  // ============================================================================

  /**
   * Get activity statistics.
   *
   * How many requests/offers, matches, completed, etc.
   */
  async getActivityStats(period: '7-days' | '30-days' | 'all-time'): Promise<{
    totalRequests: number;
    totalOffers: number;
    totalMatches: number;
    completedMatches: number;
    totalValueExchanged: Measure;
    mostActiveServiceTypes: string[];
  }> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  /**
   * Get user activity summary.
   *
   * How many requests/offers user has, matches, completed, reputation.
   */
  async getUserActivitySummary(userId: string): Promise<{
    requestsCreated: number;
    offersCreated: number;
    matchesInitiated: number;
    matchesCompleted: number;
    totalValueExchanged: Measure;
    reputation: number;
    averageRating: number;
  }> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  // ============================================================================
  // HELPER METHODS
  // ============================================================================

  private async persistRequest(request: ServiceRequest): Promise<void> {
    // TODO: Persist to Holochain DHT
    console.log('Persisting request:', request.requestNumber);
  }

  private async persistOffer(offer: ServiceOffer): Promise<void> {
    // TODO: Persist to Holochain DHT
    console.log('Persisting offer:', offer.offerNumber);
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
  const random = Math.random().toString(36).substring(2, 9);
  return `req-${timestamp}-${random}`;
}

/**
 * Generate a unique ID for a service offer.
 */
function generateOfferId(): string {
  const timestamp = Date.now().toString(36);
  const random = Math.random().toString(36).substring(2, 9);
  return `off-${timestamp}-${random}`;
}
