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
  EconomicEvent,
  IEconomicEventFactory,
} from '../interfaces/economic-event-factory.interface';
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
}
