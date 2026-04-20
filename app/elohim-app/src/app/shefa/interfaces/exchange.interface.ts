/**
 * IExchange -- Abstract interface for peer-to-peer request/offer
 * coordination in Shefa.
 *
 * Decouples the request/offer lifecycle, matching, and coordination from
 * the concrete REA-integrated implementation. Consumers inject the
 * EXCHANGE token; the default factory resolves to
 * ExchangeApiService.
 *
 * Tests provide a mock IExchange via the same token -- no
 * concrete service import needed.
 *
 * @example
 * ```typescript
 * @Injectable({ providedIn: 'root' })
 * export class MarketplaceService {
 *   private readonly marketplace = inject(EXCHANGE);
 *
 *   async postRequest(userId: string, details: Omit<ServiceRequest, 'id' | 'requestNumber' | 'createdAt' | 'updatedAt'>) {
 *     return this.marketplace.createRequest(userId, details);
 *   }
 * }
 * ```
 */

import { InjectionToken, inject } from '@angular/core';

import { ExchangeApiService } from '../services/exchange-api.service';

import type { EconomicEvent } from '@app/elohim/models/economic-event.model';
import type { Intent } from '@app/elohim/models/rea-bridge.model';
import type { ServiceRequest, ServiceOffer, ServiceMatch } from '@app/shefa/models/exchange.model';

/**
 * Abstract exchange coordinator -- manages the lifecycle of service
 * requests and offers, matching, and discovery.
 *
 * Implementations handle REA intent creation, economic event tracking,
 * DHT persistence, and matching algorithms.
 */
export interface IExchange {
  // ==========================================================================
  // Request Lifecycle
  // ==========================================================================

  /**
   * Create a service request.
   *
   * Creates a ServiceRequest, REA Intent, and EconomicEvent.
   *
   * @param requesterId - ID of the user creating the request
   * @param requestDetails - Request fields (excluding auto-generated id, number, timestamps)
   * @returns Created request, intent, and economic event
   */
  createRequest(
    requesterId: string,
    requestDetails: Omit<ServiceRequest, 'id' | 'requestNumber' | 'createdAt' | 'updatedAt'>
  ): Promise<{
    request: ServiceRequest;
    intent: Intent;
    createdEvent: EconomicEvent;
  }>;

  /**
   * Update a service request.
   *
   * Only the requester can update their own request.
   *
   * @param requestId - ID of the request to update
   * @param requesterId - ID of the requester (for ownership verification)
   * @param updates - Partial fields to update
   * @returns Updated request and economic event
   */
  updateRequest(
    requestId: string,
    requesterId: string,
    updates: Partial<ServiceRequest>
  ): Promise<{
    request: ServiceRequest;
    updatedEvent: EconomicEvent;
  }>;

  /**
   * Archive a request (soft delete).
   *
   * Hides from search but keeps record for audit.
   *
   * @param requestId - ID of the request to archive
   * @param requesterId - ID of the requester (for ownership verification)
   * @returns Archived request
   */
  archiveRequest(requestId: string, requesterId: string): Promise<ServiceRequest>;

  /**
   * Delete a request.
   *
   * @param requestId - ID of the request to delete
   * @param requesterId - ID of the requester (for ownership verification)
   */
  deleteRequest(requestId: string, requesterId: string): Promise<void>;

  /**
   * Get a specific request by ID.
   *
   * @param requestId - ID of the request
   * @returns The request, or null if not found
   */
  getRequest(requestId: string): Promise<ServiceRequest | null>;

  /**
   * Get all requests by a user with optional filters.
   *
   * @param requesterId - ID of the requester
   * @param filters - Optional status and date range filters
   * @returns Array of matching requests
   */
  getUserRequests(
    requesterId: string,
    filters?: {
      status?: 'active' | 'archived' | 'deleted';
      fromDate?: string;
      toDate?: string;
    }
  ): Promise<ServiceRequest[]>;

  // ==========================================================================
  // Offer Lifecycle
  // ==========================================================================

  /**
   * Create a service offer.
   *
   * Creates a ServiceOffer, REA Intent, and EconomicEvent.
   *
   * @param offerorId - ID of the user creating the offer
   * @param offerDetails - Offer fields (excluding auto-generated id, number, timestamps)
   * @returns Created offer, intent, and economic event
   */
  createOffer(
    offerorId: string,
    offerDetails: Omit<ServiceOffer, 'id' | 'offerNumber' | 'createdAt' | 'updatedAt'>
  ): Promise<{
    offer: ServiceOffer;
    intent: Intent;
    createdEvent: EconomicEvent;
  }>;

  /**
   * Update a service offer.
   *
   * Only the offeror can update their own offer.
   *
   * @param offerId - ID of the offer to update
   * @param offerorId - ID of the offeror (for ownership verification)
   * @param updates - Partial fields to update
   * @returns Updated offer and economic event
   */
  updateOffer(
    offerId: string,
    offerorId: string,
    updates: Partial<ServiceOffer>
  ): Promise<{
    offer: ServiceOffer;
    updatedEvent: EconomicEvent;
  }>;

  /**
   * Archive an offer (soft delete).
   *
   * @param offerId - ID of the offer to archive
   * @param offerorId - ID of the offeror (for ownership verification)
   * @returns Archived offer
   */
  archiveOffer(offerId: string, offerorId: string): Promise<ServiceOffer>;

  /**
   * Delete an offer.
   *
   * @param offerId - ID of the offer to delete
   * @param offerorId - ID of the offeror (for ownership verification)
   */
  deleteOffer(offerId: string, offerorId: string): Promise<void>;

  /**
   * Get a specific offer by ID.
   *
   * @param offerId - ID of the offer
   * @returns The offer, or null if not found
   */
  getOffer(offerId: string): Promise<ServiceOffer | null>;

  /**
   * Get all offers by a user with optional filters.
   *
   * @param offerorId - ID of the offeror
   * @param filters - Optional status and date range filters
   * @returns Array of matching offers
   */
  getUserOffers(
    offerorId: string,
    filters?: {
      status?: 'active' | 'archived' | 'deleted';
      fromDate?: string;
      toDate?: string;
    }
  ): Promise<ServiceOffer[]>;

  // ==========================================================================
  // Matching
  // ==========================================================================

  /**
   * Find offers that match a request.
   *
   * Uses service type, time, interaction type, and exchange compatibility
   * to suggest matching offers.
   *
   * @param requestId - ID of the request to find matches for
   * @param limit - Maximum number of matches to return (default: 10)
   * @returns Ranked list of service matches
   */
  findMatchesForRequest(requestId: string, limit?: number): Promise<ServiceMatch[]>;

  /**
   * Find requests that match an offer.
   *
   * Mirror of findMatchesForRequest.
   *
   * @param offerId - ID of the offer to find matches for
   * @param limit - Maximum number of matches to return (default: 10)
   * @returns Ranked list of service matches
   */
  findMatchesForOffer(offerId: string, limit?: number): Promise<ServiceMatch[]>;
}

/**
 * Injection token for exchange coordination.
 *
 * Default factory resolves to ExchangeApiService which provides:
 * - Request/offer CRUD with REA Intent + EconomicEvent creation
 * - DHT-backed persistence and search
 * - Matching algorithm with fairness and transparency guarantees
 * - Proposal, commitment, and settlement workflows
 *
 * Override in tests:
 * ```typescript
 * { provide: EXCHANGE, useValue: mockExchange }
 * ```
 */
export const EXCHANGE = new InjectionToken<IExchange>('Exchange', {
  providedIn: 'root',
  factory: () => inject(ExchangeApiService),
});
