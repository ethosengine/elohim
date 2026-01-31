/**
 * Economic Event Factory Service
 *
 * Transforms StagedTransaction (external bank data) into immutable EconomicEvent
 * entries in the Elohim ledger.
 *
 * Critical responsibility:
 * - Preserve external reference (plaidTransactionId) for reconciliation
 * - Map transaction type to correct LamadEventType
 * - Link to FinancialAsset (which account this affects)
 * - Store all original data for debugging and audit trails
 *
 * EconomicEvent is the immutable source of truth in Elohim's event-based accounting.
 * Once created, it cannot be changed - only corrected through new events.
 */

import { Injectable } from '@angular/core';

import { StagedTransaction } from '../models/transaction-import.model';

/**
 * EconomicEvent state from the model
 */
interface EventState {
  status: 'pending' | 'validated' | 'countersigned' | 'disputed' | 'corrected';
  timestamp: string;
  reasonCode?: string;
}

/**
 * ResourceMeasure from the model - currently unused but kept for future expansion
 */
// eslint-disable-next-line @typescript-eslint/no-unused-vars
interface ResourceMeasure {
  value: number;
  unit: string; // e.g., "USD"
}

/**
 * EconomicEvent - Immutable transaction record
 * Based on REA/ValueFlows ontology
 */
interface EconomicEvent {
  id: string;
  eventType: LamadEventType;
  timestamp: string;

  // Agents
  providerId: string; // Who provided/initiated
  receiverId: string; // Who received

  // Resource flow
  quantity: number;
  unit: string; // Currency or resource unit
  action: 'produce' | 'consume' | 'transfer' | 'use';

  // Description
  note?: string;

  // Audit trail
  metadata: Record<string, unknown>;
  state: EventState;
  createdAt: string;
  createdBy: string;
}

/**
 * Lamad event types (from existing model)
 */
type LamadEventType =
  | 'credit-transfer' // Income or expense
  | 'credit-retire' // Fee consumed
  | 'credit-produce' // Creating resource
  | 'credit-use'; // Using resource

/**
 * Request to create an EconomicEvent
 */
interface CreateEventRequest {
  eventType: LamadEventType;
  providerId: string;
  receiverId: string;
  quantity: number;
  unit: string;
  note?: string;
  metadata?: Record<string, unknown>;
}

@Injectable({
  providedIn: 'root',
})
export class EconomicEventFactoryService {
  // Action types as constants
  private static readonly TRANSFER_ACTION = 'transfer';
  private static readonly CONSUME_ACTION = 'consume';
  private static readonly PRODUCE_ACTION = 'produce';
  private static readonly USE_ACTION = 'use';
  private static readonly CREDIT_TRANSFER_EVENT = 'credit-transfer';
  private static readonly CREDIT_RETIRE_EVENT = 'credit-retire';

  constructor() {
    // Inject services when implementing full EconomicEvent persistence
    // private economicService: EconomicService,
    // private holochainClient: HolochainClientService,
  }

  /**
   * Creates an immutable EconomicEvent from an approved StagedTransaction
   *
   * This is the critical bridge between:
   * - External bank data (Plaid)
   * - Elohim's immutable event-sourced accounting system
   *
   * The resulting event is permanent and cannot be deleted.
   */
  async createFromStaged(staged: StagedTransaction): Promise<EconomicEvent> {
    // Validate that staged transaction is ready for conversion
    if (staged.reviewStatus !== 'approved') {
      throw new Error(
        `Cannot create event from non-approved transaction (status: ${staged.reviewStatus})`
      );
    }

    if (!staged.economicEventId) {
      // Determine event type based on transaction type
      const eventType = this.determineEventType(staged);

      // Determine provider/receiver based on direction
      const { providerId, receiverId } = this.determineAgents(staged);

      // Build the event request
      const request: CreateEventRequest = {
        eventType,
        providerId,
        receiverId,
        quantity: staged.amount.value,
        unit: staged.amount.unit,
        note: this.buildEventNote(staged),
        metadata: this.buildEventMetadata(staged),
      };

      // Create the immutable event
      return await this.createEvent(request, staged);
    } else {
      // Event already exists for this staged transaction
      throw new Error(`Event already created for staged transaction: ${staged.economicEventId}`);
    }
  }

  /**
   * Determines the event type based on transaction type and direction
   *
   * Mapping:
   * - debit → credit-transfer (outflow: provider gives to receiver)
   * - credit → credit-transfer (inflow: provider gives to receiver)
   * - fee → credit-retire (consumed/lost)
   * - transfer → credit-transfer (internal or external)
   */
  private determineEventType(staged: StagedTransaction): LamadEventType {
    switch (staged.type) {
      case 'credit':
        // Money coming in
        return EconomicEventFactoryService.CREDIT_TRANSFER_EVENT;

      case 'debit':
        // Money going out (expense)
        return EconomicEventFactoryService.CREDIT_TRANSFER_EVENT;

      case 'fee':
        // Fee consumed/lost
        return EconomicEventFactoryService.CREDIT_RETIRE_EVENT;

      case 'transfer':
        // Transfer between accounts
        return EconomicEventFactoryService.CREDIT_TRANSFER_EVENT;

      default:
        return EconomicEventFactoryService.CREDIT_TRANSFER_EVENT;
    }
  }

  /**
   * Determines provider and receiver based on transaction direction
   *
   * For bank account transactions:
   * - Debit (expense): steward is provider (giving away), external is receiver
   * - Credit (income): external is provider (giving to steward), steward is receiver
   * - Fee: steward is provider (bank takes fee), bank is receiver
   */
  private determineAgents(staged: StagedTransaction): { providerId: string; receiverId: string } {
    switch (staged.type) {
      case 'credit':
        // Income: someone pays steward
        return {
          providerId: 'external-party', // Unknown external source
          receiverId: staged.stewardId,
        };

      case 'debit':
        // Expense: steward pays someone
        return {
          providerId: staged.stewardId,
          receiverId: 'external-party',
        };

      case 'fee':
        // Fee: steward loses money to bank
        return {
          providerId: staged.stewardId,
          receiverId: staged.merchantName ?? 'fee-collector',
        };

      case 'transfer':
        // Transfer: depends on context (assume out for now)
        return {
          providerId: staged.stewardId,
          receiverId: 'external-account',
        };

      default:
        return {
          providerId: staged.stewardId,
          receiverId: 'unknown',
        };
    }
  }

  /**
   * Builds human-readable note for the event
   *
   * Includes merchant name, original description, and source information
   */
  private buildEventNote(staged: StagedTransaction): string {
    const parts: string[] = [];

    // Merchant name
    if (staged.merchantName) {
      parts.push(staged.merchantName);
    }

    // Original description
    if (staged.description && staged.description !== staged.merchantName) {
      parts.push(`(${staged.description})`);
    }

    // Source indicator
    parts.push(`[Imported from ${staged.plaidAccountId}]`);

    return parts.join(' ');
  }

  /**
   * Builds metadata for the event
   *
   * CRITICAL: Preserves external reference (plaidTransactionId) for reconciliation
   * This allows linking back to the original bank transaction if needed.
   */
  private buildEventMetadata(staged: StagedTransaction): Record<string, unknown> {
    return {
      // === EXTERNAL REFERENCE (CRITICAL for reconciliation) ===
      plaidTransactionId: staged.plaidTransactionId,
      plaidAccountId: staged.plaidAccountId,

      // === CATEGORIZATION INFO ===
      category: staged.category,
      categoryConfidence: staged.categoryConfidence,
      categorySource: staged.categorySource, // 'ai' | 'plaid' | 'manual' | 'rule'

      // === BUDGET LINKAGE ===
      budgetId: staged.budgetId,
      budgetCategoryId: staged.budgetCategoryId,

      // === IMPORT CONTEXT ===
      importBatchId: staged.batchId,
      importedAt: new Date().toISOString(),
      importSource: 'plaid',

      // === MERCHANT INFO ===
      merchantName: staged.merchantName,

      // === PRESERVE ORIGINAL DATA for debugging ===
      plaidRawData: staged.plaidRawData,

      // === AUDIT TRAIL ===
      eventFactory: 'economic-event-factory-service',
      stagedTransactionId: staged.id,
    };
  }

  /**
   * Creates the immutable EconomicEvent
   *
   * In the real implementation, this would:
   * 1. Call EconomicService.createEvent()
   * 2. Store to Holochain DHT
   * 3. Link to FinancialAsset
   * 4. Emit EconomicEventCreated signal
   */
  private async createEvent(
    request: CreateEventRequest,
    staged: StagedTransaction
  ): Promise<EconomicEvent> {
    // Inject and use EconomicService when available
    // This method will be fully implemented when EconomicService is available
    // const event = await this.economicService.createEvent(request);

    // Currently returns mock implementation - ready for service injection
    const event: EconomicEvent = {
      id: `event-${Date.now()}-${(crypto.getRandomValues(new Uint32Array(1))[0] / 2 ** 32).toString(36).substring(2, 11)}`,
      eventType: request.eventType,
      timestamp: new Date().toISOString(),

      providerId: request.providerId,
      receiverId: request.receiverId,

      quantity: request.quantity,
      unit: request.unit,
      action: this.determineAction(request.eventType),

      note: request.note,

      metadata: request.metadata ?? {},
      state: {
        status: 'validated',
        timestamp: new Date().toISOString(),
        reasonCode: 'auto-imported',
      },

      createdAt: new Date().toISOString(),
      createdBy: staged.stewardId,
    };

    // Production: Store to Holochain via content_store zome
    return await Promise.resolve(event);
  }

  /**
   * Determines the action based on event type
   */
  private determineAction(eventType: LamadEventType): 'produce' | 'consume' | 'transfer' | 'use' {
    switch (eventType) {
      case 'credit-transfer':
        return 'transfer';
      case 'credit-retire':
        return 'consume';
      case 'credit-produce':
        return 'produce';
      case 'credit-use':
        return 'use';
      default:
        return 'transfer';
    }
  }

  /**
   * Batch creates events from multiple staged transactions
   */
  async createMultipleFromStaged(stagedList: StagedTransaction[]): Promise<EconomicEvent[]> {
    const events: EconomicEvent[] = [];

    for (const staged of stagedList) {
      if (staged.reviewStatus === 'approved') {
        try {
          const event = await this.createFromStaged(staged);
          events.push(event);
        } catch {
          // Event creation failed for this transaction - skip and continue with next
          // Allows resilient batch processing without blocking subsequent transactions
        }
      }
    }
    return events;
  }

  /**
   * Corrects an economic event (creates a new "correction" event)
   *
   * In immutable event sourcing, errors are corrected by creating new events,
   * not by modifying existing ones.
   *
   * Correction event logic implementation pending
   */
  async createCorrectionEvent(
    _originalEventId: string,
    _correction: Partial<CreateEventRequest>,
    _reason: string
  ): Promise<EconomicEvent> {
    return await Promise.reject(new Error('Correction events not yet implemented'));
  }
}
