/**
 * RequestsOffersApiService — Thin HTTP client for request/offer coordination.
 *
 * Calls doorway `/api/v1/requests-offers/*` endpoints, implementing
 * IRequestsAndOffers. Replaces the fat RequestsAndOffersService when the
 * business logic lives behind the Rust API boundary.
 */

import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { catchError } from 'rxjs/operators';

import { firstValueFrom, of } from 'rxjs';

import type { IRequestsAndOffers } from '../interfaces/requests-and-offers.interface';
import type { EconomicEvent } from '@app/elohim/models/economic-event.model';
import type { Intent } from '@app/elohim/models/rea-bridge.model';
import type {
  ServiceMatch,
  ServiceOffer,
  ServiceRequest,
} from '@app/shefa/models/requests-and-offers.model';

const BASE = '/api/v1/requests-offers';

@Injectable({ providedIn: 'root' })
export class RequestsOffersApiService implements IRequestsAndOffers {
  private readonly http = inject(HttpClient);

  // ==========================================================================
  // Request Lifecycle
  // ==========================================================================

  async createRequest(
    requesterId: string,
    requestDetails: Omit<ServiceRequest, 'id' | 'requestNumber' | 'createdAt' | 'updatedAt'>
  ): Promise<{ request: ServiceRequest; intent: Intent; createdEvent: EconomicEvent }> {
    return firstValueFrom(
      this.http.post<{ request: ServiceRequest; intent: Intent; createdEvent: EconomicEvent }>(
        `${BASE}/requests`,
        { ...requestDetails, requesterId }
      )
    );
  }

  async updateRequest(
    requestId: string,
    requesterId: string,
    updates: Partial<ServiceRequest>
  ): Promise<{ request: ServiceRequest; updatedEvent: EconomicEvent }> {
    return firstValueFrom(
      this.http.put<{ request: ServiceRequest; updatedEvent: EconomicEvent }>(
        `${BASE}/requests/${requestId}`,
        { requesterId, ...updates }
      )
    );
  }

  async archiveRequest(requestId: string, requesterId: string): Promise<ServiceRequest> {
    return firstValueFrom(
      this.http.post<ServiceRequest>(`${BASE}/requests/${requestId}/archive`, { requesterId })
    );
  }

  async deleteRequest(requestId: string, requesterId: string): Promise<void> {
    return firstValueFrom(
      this.http.delete<void>(`${BASE}/requests/${requestId}`, {
        body: { requesterId },
      })
    );
  }

  async getRequest(requestId: string): Promise<ServiceRequest | null> {
    return firstValueFrom(
      this.http
        .get<ServiceRequest>(`${BASE}/requests/${requestId}`)
        .pipe(catchError(() => of(null)))
    );
  }

  async getUserRequests(
    requesterId: string,
    filters?: { status?: 'active' | 'archived' | 'deleted'; fromDate?: string; toDate?: string }
  ): Promise<ServiceRequest[]> {
    const params: Record<string, string> = { requesterId };
    if (filters?.status) params['status'] = filters.status;
    if (filters?.fromDate) params['fromDate'] = filters.fromDate;
    if (filters?.toDate) params['toDate'] = filters.toDate;
    return firstValueFrom(
      this.http.get<ServiceRequest[]>(`${BASE}/requests`, { params }).pipe(catchError(() => of([])))
    );
  }

  // ==========================================================================
  // Offer Lifecycle
  // ==========================================================================

  async createOffer(
    offerorId: string,
    offerDetails: Omit<ServiceOffer, 'id' | 'offerNumber' | 'createdAt' | 'updatedAt'>
  ): Promise<{ offer: ServiceOffer; intent: Intent; createdEvent: EconomicEvent }> {
    return firstValueFrom(
      this.http.post<{ offer: ServiceOffer; intent: Intent; createdEvent: EconomicEvent }>(
        `${BASE}/offers`,
        { ...offerDetails, offerorId }
      )
    );
  }

  async updateOffer(
    offerId: string,
    offerorId: string,
    updates: Partial<ServiceOffer>
  ): Promise<{ offer: ServiceOffer; updatedEvent: EconomicEvent }> {
    return firstValueFrom(
      this.http.put<{ offer: ServiceOffer; updatedEvent: EconomicEvent }>(
        `${BASE}/offers/${offerId}`,
        { offerorId, ...updates }
      )
    );
  }

  async archiveOffer(offerId: string, offerorId: string): Promise<ServiceOffer> {
    return firstValueFrom(
      this.http.post<ServiceOffer>(`${BASE}/offers/${offerId}/archive`, { offerorId })
    );
  }

  async deleteOffer(offerId: string, offerorId: string): Promise<void> {
    return firstValueFrom(
      this.http.delete<void>(`${BASE}/offers/${offerId}`, { body: { offerorId } })
    );
  }

  async getOffer(offerId: string): Promise<ServiceOffer | null> {
    return firstValueFrom(
      this.http.get<ServiceOffer>(`${BASE}/offers/${offerId}`).pipe(catchError(() => of(null)))
    );
  }

  async getUserOffers(
    offerorId: string,
    filters?: { status?: 'active' | 'archived' | 'deleted'; fromDate?: string; toDate?: string }
  ): Promise<ServiceOffer[]> {
    const params: Record<string, string> = { offerorId };
    if (filters?.status) params['status'] = filters.status;
    if (filters?.fromDate) params['fromDate'] = filters.fromDate;
    if (filters?.toDate) params['toDate'] = filters.toDate;
    return firstValueFrom(
      this.http.get<ServiceOffer[]>(`${BASE}/offers`, { params }).pipe(catchError(() => of([])))
    );
  }

  // ==========================================================================
  // Matching
  // ==========================================================================

  async findMatchesForRequest(requestId: string, limit?: number): Promise<ServiceMatch[]> {
    const params: Record<string, string> = { requestId };
    if (limit !== undefined) params['limit'] = String(limit);
    return firstValueFrom(
      this.http
        .get<ServiceMatch[]>(`${BASE}/offers/matching/${requestId}`, { params })
        .pipe(catchError(() => of([])))
    );
  }

  async findMatchesForOffer(offerId: string, limit?: number): Promise<ServiceMatch[]> {
    const params: Record<string, string> = { offerId };
    if (limit !== undefined) params['limit'] = String(limit);
    return firstValueFrom(
      this.http.get<ServiceMatch[]>(`${BASE}/matches`, { params }).pipe(catchError(() => of([])))
    );
  }
}
