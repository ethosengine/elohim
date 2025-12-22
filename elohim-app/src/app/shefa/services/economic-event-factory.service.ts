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
import { StagedTransaction } from '@elohim/models/transaction-import.model';

/**
 * EconomicEvent state from the model
 */
interface EventState {
  status: 'pending' | 'validated' | 'countersigned' | 'disputed' | 'corrected';
  timestamp: string;
  reasonCode?: string;
}

/**
 * ResourceMeasure from the model
 */
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

/**
 * Response from event creation
 */
interface EventCreationResult {
  event: EconomicEvent;
  success: boolean;
  errors?: string[];
}

@Injectable({
  providedIn: 'root',
})
export class EconomicEventFactoryService {
  constructor(
    // private economicService: EconomicService,  // TODO: Inject actual service
    // private holochainClient: HolochainClientService,
  ) {}

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
      const event = await this.createEvent(request, staged);

      return event;
    } else {
      // Event already exists for this staged transaction
      throw new Error(
        `Event already created for staged transaction: ${staged.economicEventId}`
      );
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
        return 'credit-transfer';

      case 'debit':
        // Money going out (expense)
        return 'credit-transfer';

      case 'fee':
        // Fee consumed/lost
        return 'credit-retire';

      case 'transfer':
        // Transfer between accounts
        return 'credit-transfer';

      default:
        return 'credit-transfer';
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
  private determineAgents(
    staged: StagedTransaction
  ): { providerId: string; receiverId: string } {
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
          receiverId: staged.merchantName || 'fee-collector',
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
    // TODO: Replace with actual EconomicService call
    // const event = await this.economicService.createEvent(request);

    // Mock implementation for now
    const event: EconomicEvent = {
      id: `event-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
      eventType: request.eventType,
      timestamp: new Date().toISOString(),

      providerId: request.providerId,
      receiverId: request.receiverId,

      quantity: request.quantity,
      unit: request.unit,
      action: this.determineAction(request.eventType),

      note: request.note,

      metadata: request.metadata || {},
      state: {
        status: 'validated',
        timestamp: new Date().toISOString(),
        reasonCode: 'auto-imported',
      },

      createdAt: new Date().toISOString(),
      createdBy: staged.stewardId,
    };

    // TODO: In production, store to Holochain
    // await this.holochainClient.callZome('content_store', 'create_economic_event', event);

    console.log('[EconomicEventFactory] Created event:', {
      eventId: event.id,
      type: event.eventType,
      plaidTransactionId: staged.plaidTransactionId,
      amount: `${event.quantity} ${event.unit}`,
    });

    return event;
  }

  /**
   * Determines the action based on event type
   */
  private determineAction(
    eventType: LamadEventType
  ): 'produce' | 'consume' | 'transfer' | 'use' {
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
  async createMultipleFromStaged(
    stagedList: StagedTransaction[]
  ): Promise<EconomicEvent[]> {
    const events: EconomicEvent[] = [];

    for (const staged of stagedList) {
      if (staged.reviewStatus === 'approved') {
        try {
          const event = await this.createFromStaged(staged);
          events.push(event);
        } catch (error) {
          console.error(
            `Failed to create event from staged transaction ${staged.id}:`,
            error
          );
          // Continue with next transaction
        }
      }
    }

    console.log(
      `[EconomicEventFactory] Created ${events.length} events from ${stagedList.length} staged transactions`
    );

    return events;
  }

  /**
   * Corrects an economic event (creates a new "correction" event)
   *
   * In immutable event sourcing, errors are corrected by creating new events,
   * not by modifying existing ones.
   *
   * TODO: Implement correction event logic
   */
  async createCorrectionEvent(
    originalEventId: string,
    correction: Partial<CreateEventRequest>,
    reason: string
  ): Promise<EconomicEvent> {
    throw new Error('Correction events not yet implemented');
  }
}
