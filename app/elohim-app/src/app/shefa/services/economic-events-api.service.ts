/**
 * EconomicEventsApiService — Thin HTTP client for economic event creation.
 *
 * Calls doorway `/api/v1/economic-events/*` endpoints, implementing
 * IEconomicEventFactory. Replaces the fat EconomicEventFactoryService
 * when the business logic lives behind the Rust API boundary.
 */

import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { firstValueFrom } from 'rxjs';

import type {
  AppreciationDisplay,
  CreateAppreciationInput,
  CreateEconomicEventInput,
  IEconomicEventFactory,
} from '../interfaces/economic-event-factory.interface';
import type { EconomicEvent } from '@app/elohim/models';
import type { StagedTransaction } from '@app/shefa/models/transaction-import.model';

@Injectable({ providedIn: 'root' })
export class EconomicEventsApiService implements IEconomicEventFactory {
  private readonly http = inject(HttpClient);

  async createFromStaged(staged: StagedTransaction): Promise<EconomicEvent> {
    return firstValueFrom(this.http.post<EconomicEvent>('/api/v1/economic-events', { staged }));
  }

  async createMultipleFromStaged(stagedList: StagedTransaction[]): Promise<EconomicEvent[]> {
    return firstValueFrom(
      this.http.post<EconomicEvent[]>('/api/v1/economic-events/bulk', { stagedList })
    );
  }

  async createCorrectionEvent(
    originalEventId: string,
    correction: Partial<{
      eventType: string;
      providerId: string;
      receiverId: string;
      quantity: number;
      unit: string;
      note?: string;
      metadata?: Record<string, unknown>;
    }>,
    reason: string
  ): Promise<EconomicEvent> {
    return firstValueFrom(
      this.http.post<EconomicEvent>('/api/v1/economic-events', {
        type: 'correction',
        originalEventId,
        correction,
        reason,
      })
    );
  }

  // ===========================================================================
  // Query Methods
  // ===========================================================================

  async getEventsByProvider(agentId: string): Promise<EconomicEvent[]> {
    return firstValueFrom(
      this.http.get<EconomicEvent[]>('/api/v1/economic-events', { params: { provider: agentId } })
    );
  }

  async getEventsByReceiver(agentId: string): Promise<EconomicEvent[]> {
    return firstValueFrom(
      this.http.get<EconomicEvent[]>('/api/v1/economic-events', { params: { receiver: agentId } })
    );
  }

  async getEventsByAction(action: string): Promise<EconomicEvent[]> {
    return firstValueFrom(
      this.http.get<EconomicEvent[]>('/api/v1/economic-events', { params: { action } })
    );
  }

  async getEventsByLamadType(lamadType: string): Promise<EconomicEvent[]> {
    return firstValueFrom(
      this.http.get<EconomicEvent[]>('/api/v1/economic-events', { params: { lamadType } })
    );
  }

  async createEconomicEvent(payload: CreateEconomicEventInput): Promise<EconomicEvent> {
    return firstValueFrom(this.http.post<EconomicEvent>('/api/v1/economic-events', payload));
  }

  // ===========================================================================
  // Appreciation Methods
  // ===========================================================================

  async getAppreciationsFor(appreciatedId: string): Promise<AppreciationDisplay[]> {
    return firstValueFrom(
      this.http.get<AppreciationDisplay[]>('/api/v1/economic-events/appreciations', {
        params: { for: appreciatedId },
      })
    );
  }

  async getAppreciationsBy(appreciatorId: string): Promise<AppreciationDisplay[]> {
    return firstValueFrom(
      this.http.get<AppreciationDisplay[]>('/api/v1/economic-events/appreciations', {
        params: { by: appreciatorId },
      })
    );
  }

  async createAppreciation(payload: CreateAppreciationInput): Promise<AppreciationDisplay> {
    return firstValueFrom(
      this.http.post<AppreciationDisplay>('/api/v1/economic-events/appreciations', payload)
    );
  }
}
