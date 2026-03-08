/**
 * IEconomicEventFactory -- Abstract interface for transforming staged
 * bank transactions into immutable EconomicEvent ledger entries.
 *
 * Decouples event creation logic from the concrete service implementation.
 * Consumers inject the ECONOMIC_EVENT_FACTORY token; the default factory
 * resolves to EconomicEventFactoryService.
 *
 * Tests provide a mock IEconomicEventFactory via the same token -- no
 * concrete service import needed.
 *
 * @example
 * ```typescript
 * @Injectable({ providedIn: 'root' })
 * export class TransactionImportService {
 *   private readonly eventFactory = inject(ECONOMIC_EVENT_FACTORY);
 *
 *   async importApproved(staged: StagedTransaction): Promise<EconomicEvent> {
 *     return this.eventFactory.createFromStaged(staged);
 *   }
 * }
 * ```
 */

import { InjectionToken, inject } from '@angular/core';

import { EconomicEventsApiService } from '../services/economic-events-api.service';

import type { StagedTransaction } from '@app/shefa/models/transaction-import.model';
import type { REAAction, LamadEventType } from '@app/elohim/models';

/**
 * EconomicEvent -- Immutable transaction record based on REA/ValueFlows.
 *
 * Locally defined to avoid coupling the interface to the concrete service's
 * internal type. Matches the shape returned by EconomicEventFactoryService.
 */
export interface EconomicEvent {
  id: string;
  eventType: string;
  timestamp: string;
  providerId: string;
  receiverId: string;
  quantity: number;
  unit: string;
  action: 'produce' | 'consume' | 'transfer' | 'use';
  note?: string;
  metadata: Record<string, unknown>;
  state: {
    status: 'pending' | 'validated' | 'countersigned' | 'disputed' | 'corrected';
    timestamp: string;
    reasonCode?: string;
  };
  createdAt: string;
  createdBy: string;
}

/**
 * AppreciationDisplay -- Flattened appreciation record for UI consumption.
 *
 * Locally defined to decouple the interface from the concrete AppreciationService.
 * Matches the shape returned by the appreciation zome calls.
 */
export interface AppreciationDisplay {
  id: string;
  /** What is being appreciated (event ID, content ID, etc.) */
  appreciationOf: string;
  /** Who is expressing appreciation */
  appreciatedBy: string;
  /** Who receives the appreciation */
  appreciationTo: string;
  /** Quantity of appreciation */
  quantityValue: number;
  /** Unit of appreciation (e.g., 'recognition-points', 'affinity') */
  quantityUnit: string;
  /** Optional note */
  note: string | null;
  /** When created */
  createdAt: string;
}

/**
 * Input for creating an appreciation.
 */
export interface CreateAppreciationInput {
  /** What is being appreciated */
  appreciationOf: string;
  /** Who receives the appreciation */
  appreciationTo: string;
  /** Quantity of appreciation */
  quantityValue: number;
  /** Unit of appreciation */
  quantityUnit: string;
  /** Optional note */
  note?: string;
}

/**
 * Input for creating an economic event.
 */
export interface CreateEconomicEventInput {
  action: REAAction;
  providerId: string;
  receiverId: string;
  resourceConformsTo?: string;
  resourceInventoriedAs?: string;
  toResourceInventoriedAs?: string;
  resourceClassifiedAs?: string[];
  resourceQuantityValue?: number;
  resourceQuantityUnit?: string;
  effortQuantityValue?: number;
  effortQuantityUnit?: string;
  inputOf?: string;
  outputOf?: string;
  fulfills?: string[];
  realizationOf?: string;
  agreedIn?: string;
  satisfies?: string[];
  inScopeOf?: string[];
  note?: string;
  lamadEventType?: LamadEventType;
}

/**
 * Abstract economic event factory -- transforms staged transactions into
 * immutable EconomicEvent entries.
 *
 * Implementations handle validation, event type determination, agent
 * assignment, and metadata construction.
 */
export interface IEconomicEventFactory {
  /**
   * Create an immutable EconomicEvent from an approved StagedTransaction.
   *
   * @param staged - Approved staged transaction (reviewStatus must be 'approved')
   * @returns The created EconomicEvent
   * @throws If staged transaction is not approved or event already exists
   */
  createFromStaged(staged: StagedTransaction): Promise<EconomicEvent>;

  /**
   * Batch create events from multiple staged transactions.
   *
   * Skips non-approved transactions and continues on individual failures.
   *
   * @param stagedList - Array of staged transactions to process
   * @returns Successfully created EconomicEvents
   */
  createMultipleFromStaged(stagedList: StagedTransaction[]): Promise<EconomicEvent[]>;

  /**
   * Create a correction event for an existing EconomicEvent.
   *
   * In immutable event sourcing, errors are corrected by creating new events,
   * not by modifying existing ones.
   *
   * @param originalEventId - ID of the event to correct
   * @param correction - Partial fields to override
   * @param reason - Reason for the correction
   * @returns The correction EconomicEvent
   */
  createCorrectionEvent(
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
  ): Promise<EconomicEvent>;

  // ===========================================================================
  // Query Methods (from EconomicService)
  // ===========================================================================

  /**
   * Get economic events where the agent is the provider.
   */
  getEventsByProvider(agentId: string): Promise<EconomicEvent[]>;

  /**
   * Get economic events where the agent is the receiver.
   */
  getEventsByReceiver(agentId: string): Promise<EconomicEvent[]>;

  /**
   * Get economic events filtered by REA action type.
   */
  getEventsByAction(action: string): Promise<EconomicEvent[]>;

  /**
   * Get economic events filtered by Lamad event type.
   */
  getEventsByLamadType(lamadType: string): Promise<EconomicEvent[]>;

  /**
   * Create a new economic event from raw input.
   */
  createEconomicEvent(payload: CreateEconomicEventInput): Promise<EconomicEvent>;

  // ===========================================================================
  // Appreciation Methods (from AppreciationService)
  // ===========================================================================

  /**
   * Get appreciations received by an entity.
   */
  getAppreciationsFor(appreciatedId: string): Promise<AppreciationDisplay[]>;

  /**
   * Get appreciations given by an agent.
   */
  getAppreciationsBy(appreciatorId: string): Promise<AppreciationDisplay[]>;

  /**
   * Create a new appreciation record.
   */
  createAppreciation(payload: CreateAppreciationInput): Promise<AppreciationDisplay>;
}

/**
 * Injection token for the economic event factory.
 *
 * Default factory resolves to EconomicEventFactoryService which provides:
 * - StagedTransaction to EconomicEvent transformation
 * - Event type determination from transaction type
 * - Agent (provider/receiver) assignment based on direction
 * - Batch processing with error resilience
 *
 * Override in tests:
 * ```typescript
 * { provide: ECONOMIC_EVENT_FACTORY, useValue: mockEventFactory }
 * ```
 */
export const ECONOMIC_EVENT_FACTORY = new InjectionToken<IEconomicEventFactory>(
  'EconomicEventFactory',
  {
    providedIn: 'root',
    factory: () => inject(EconomicEventsApiService),
  }
);
