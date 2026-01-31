/**
 * Economic Event Bridge Service
 *
 * This is the ONLY point where banking-bridge data crosses into Holochain.
 * When a StagedTransaction is approved, this service:
 * 1. Transforms it into an EconomicEvent
 * 2. Calls the Holochain zome to create the event
 * 3. Updates the staged transaction with the event ID
 *
 * This is the translation layer between legacy banking and the next-gen economy.
 */

import { Injectable, inject } from '@angular/core';

// @coverage: 98.5% (2026-02-04)

import { HolochainClientService } from '@app/elohim/services/holochain-client.service';

import { bankingStore, StagedTransactionLocal } from '../stores/banking-store';

/**
 * Economic event payload for API - camelCase with parsed JSON objects.
 * This matches the REA EconomicEvent entry type expected by elohim-storage.
 */
export interface EconomicEventPayload {
  id: string;
  action: string;
  provider: string;
  receiver: string;
  resourceConformsTo: string;
  resourceClassifiedAs: string[]; // Parsed JSON array
  resourceQuantityValue: number;
  resourceQuantityUnit: string;
  effortQuantityValue?: number;
  effortQuantityUnit?: string;
  hasBeginning?: string;
  hasEnd?: string;
  hasPointInTime: string;
  due?: string;
  note?: string;
  inputOf?: string;
  outputOf?: string;
  state: string;
  triggeredBy?: string;
  atLocation?: string;
  lamadEventType?: string;
  metadata: Record<string, unknown>; // Parsed JSON object
  createdAt: string;
}

/**
 * Result of committing a staged transaction to Holochain
 */
export interface CommitResult {
  success: boolean;
  economicEventId?: string;
  actionHash?: string;
  error?: string;
}

/**
 * Batch commit result
 */
export interface BatchCommitResult {
  totalAttempted: number;
  successCount: number;
  failureCount: number;
  results: {
    stagedId: string;
    result: CommitResult;
  }[];
}

@Injectable({
  providedIn: 'root',
})
export class EconomicEventBridgeService {
  private readonly holochain = inject(HolochainClientService);

  constructor() {
    // Dependency injection is handled via constructor field injection
    // All dependencies are marked with inject() decorators on class properties
  }

  /**
   * Commit a single approved staged transaction to Holochain as an EconomicEvent.
   *
   * This is the bridge: local staging → network signal
   */
  async commitStagedTransaction(stagedId: string): Promise<CommitResult> {
    try {
      // 1. Get the staged transaction from local store
      const staged = await bankingStore.getStaged(stagedId);
      if (!staged) {
        return { success: false, error: `Staged transaction ${stagedId} not found` };
      }

      if (staged.reviewStatus !== 'approved') {
        return {
          success: false,
          error: `Transaction ${stagedId} is not approved (status: ${staged.reviewStatus})`,
        };
      }

      if (staged.economicEventId) {
        return {
          success: true,
          economicEventId: staged.economicEventId,
          error: 'Already committed',
        };
      }

      // 2. Transform to EconomicEvent payload
      const eventPayload = this.transformToEconomicEvent(staged);

      // 3. Call Holochain zome to create the event
      const result = await this.holochain.callZome({
        zomeName: 'content_store',
        fnName: 'create_economic_event',
        payload: eventPayload,
      });

      if (!result.success) {
        return { success: false, error: result.error ?? 'Zome call failed' };
      }

      const economicEventId = eventPayload.id;
      const actionHash: string | undefined =
        typeof result.data === 'string' ? result.data : undefined;

      // 4. Update the staged transaction with the event ID
      staged.economicEventId = economicEventId;
      staged.reviewStatus = 'approved'; // Stays approved, now with event link
      await bankingStore.saveStaged(staged);
      return {
        success: true,
        economicEventId,
        actionHash,
      };
    } catch (err) {
      return { success: false, error: String(err) };
    }
  }

  /**
   * Commit all approved transactions in a batch to Holochain.
   */
  async commitBatch(batchId: string): Promise<BatchCommitResult> {
    const batch = await bankingStore.getBatch(batchId);
    if (!batch) {
      return {
        totalAttempted: 0,
        successCount: 0,
        failureCount: 0,
        results: [],
      };
    }

    const staged = await bankingStore.getStagedByBatch(batchId);
    const approved = staged.filter(s => s.reviewStatus === 'approved' && !s.economicEventId);

    const results: BatchCommitResult['results'] = [];
    let successCount = 0;
    let failureCount = 0;

    for (const transaction of approved) {
      const result = await this.commitStagedTransaction(transaction.id);
      results.push({ stagedId: transaction.id, result });

      if (result.success) {
        successCount++;
      } else {
        failureCount++;
      }
    }

    // Update batch status if all approved transactions are committed
    if (failureCount === 0 && successCount > 0) {
      batch.status = 'completed';
      batch.completedAt = new Date().toISOString();
      await bankingStore.saveBatch(batch);
    }

    return {
      totalAttempted: approved.length,
      successCount,
      failureCount,
      results,
    };
  }

  /**
   * Transform a StagedTransaction into an EconomicEvent payload.
   *
   * This is where the translation happens:
   * - Bank transaction → REA value flow
   * - Plaid categories → Elohim resource classifications
   * - Debit/Credit → REA consume/produce actions
   */
  private transformToEconomicEvent(staged: StagedTransactionLocal): EconomicEventPayload {
    const now = new Date().toISOString();

    // Map transaction type to REA action
    const action = this.mapTransactionTypeToAction(staged.type);

    // Determine provider/receiver based on debit/credit
    // For debit: user is provider (spending), merchant is receiver
    // For credit: merchant is provider, user is receiver
    const isDebit = staged.type === 'debit' || staged.type === 'fee';
    const provider = isDebit ? staged.stewardId : (staged.merchantName ?? 'external');
    const receiver = isDebit ? (staged.merchantName ?? 'external') : staged.stewardId;

    // Resource classification from category
    const resourceClassifications = [staged.category, staged.type, 'bank-import'];

    // Build metadata preserving Plaid provenance
    const metadata = {
      source: 'plaid-import',
      plaid_transaction_id: staged.plaidTransactionId,
      plaid_account_id: staged.plaidAccountId,
      financial_asset_id: staged.financialAssetId,
      batch_id: staged.batchId,
      original_description: staged.description,
      category_confidence: staged.categoryConfidence,
      category_source: staged.categorySource,
    };

    // Return camelCase InputView with parsed JSON objects
    return {
      id: `ee-${staged.plaidTransactionId}`, // Deterministic ID from Plaid ID
      action,
      provider,
      receiver,
      resourceConformsTo: 'currency',
      resourceClassifiedAs: resourceClassifications,
      resourceQuantityValue: Math.abs(staged.amount.value),
      resourceQuantityUnit: staged.amount.unit,
      hasPointInTime: staged.timestamp,
      note: staged.description,
      state: 'completed',
      lamadEventType: 'bank_transaction',
      metadata,
      createdAt: now,
    };
  }

  /**
   * Map Plaid transaction type to REA action.
   *
   * REA actions: consume, produce, transfer, use, cite, work, accept, modify, etc.
   * Bank transactions map primarily to consume (spending) and produce (income).
   */
  private mapTransactionTypeToAction(type: StagedTransactionLocal['type']): string {
    switch (type) {
      case 'debit':
        return 'consume'; // Spending reduces resources
      case 'credit':
        return 'produce'; // Income increases resources
      case 'transfer':
        return 'transfer'; // Moving between accounts
      case 'fee':
        return 'consume'; // Bank fees reduce resources
      default:
        return 'consume';
    }
  }

  /**
   * Get the EconomicEvent for a staged transaction (if committed).
   */
  async getCommittedEvent(stagedId: string): Promise<{ eventId: string; event: unknown } | null> {
    const staged = await bankingStore.getStaged(stagedId);
    if (!staged?.economicEventId) {
      return null;
    }

    const result = await this.holochain.callZome({
      zomeName: 'content_store',
      fnName: 'get_economic_event',
      payload: { id: staged.economicEventId },
    });

    if (!result.success || !result.data) {
      return null;
    }

    return {
      eventId: staged.economicEventId,
      event: result.data,
    };
  }

  /**
   * Check if a Plaid transaction has already been committed.
   * Prevents double-commits across sessions.
   */
  async isAlreadyCommitted(plaidTransactionId: string): Promise<boolean> {
    // First check local store
    const staged = await bankingStore.checkDuplicate(plaidTransactionId);
    if (staged?.economicEventId) {
      return true;
    }

    // Could also check Holochain directly by querying for event with this ID
    const eventId = `ee-${plaidTransactionId}`;
    const result = await this.holochain.callZome({
      zomeName: 'content_store',
      fnName: 'get_economic_event',
      payload: { id: eventId },
    });

    return result.success && result.data !== null;
  }
}
