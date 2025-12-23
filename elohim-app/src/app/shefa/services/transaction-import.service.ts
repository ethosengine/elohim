/**
 * Transaction Import Service
 *
 * Orchestrates the complete 8-stage import pipeline:
 *
 * 1. FETCH      → Pull transactions from Plaid API
 * 2. NORMALIZE  → Convert to internal format
 * 3. DEDUPLICATE→ Remove already-imported transactions
 * 4. STAGE      → Create StagedTransaction records
 * 5. CATEGORIZE → AI categorization (async, non-blocking)
 * 6. REVIEW     → Present to user for approval
 * 7. APPROVE    → User approves transactions
 * 8. CREATE     → Create EconomicEvents + reconcile budgets
 *
 * Pipeline is designed to:
 * - Be resumable at any stage if it fails
 * - Never lose data
 * - Provide clear user feedback
 * - Maintain transaction integrity
 * - Respect immutability of EconomicEvents
 */

import { Injectable } from '@angular/core';
import { Observable, Subject, BehaviorSubject } from 'rxjs';

import {
  PlaidConnection,
  ImportBatch,
  ImportRequest,
  StagedTransaction,
  PlaidTransaction,
  CategorizationResult,
} from '../models/transaction-import.model';

import { PlaidIntegrationService } from './plaid-integration.service';
import { DuplicateDetectionService } from './duplicate-detection.service';
import { AICategorizationService } from './ai-categorization.service';
import { EconomicEventFactoryService } from './economic-event-factory.service';
import { BudgetReconciliationService } from './budget-reconciliation.service';

/**
 * Pipeline progress tracking
 */
interface PipelineProgress {
  stage:
    | 'created'
    | 'fetching'
    | 'normalizing'
    | 'deduplicating'
    | 'staging'
    | 'categorizing'
    | 'reviewing'
    | 'approving'
    | 'completed'
    | 'error';
  progress: number; // 0-100
  message: string;
  itemsProcessed: number;
  itemsTotal: number;
}

/**
 * Normalized internal transaction format
 */
interface NormalizedTransaction {
  plaidTransactionId: string;
  plaidAccountId: string;
  timestamp: string;
  type: 'debit' | 'credit' | 'transfer' | 'fee';
  amount: number;
  currency: string;
  description: string;
  merchantName?: string;
  raw: PlaidTransaction;
}

/**
 * Import statistics
 */
interface ImportStats {
  totalFetched: number;
  duplicateCount: number;
  newCount: number;
  categorizedCount: number;
  approvedCount: number;
  failedCount: number;
  duration: number; // Milliseconds
}

@Injectable({
  providedIn: 'root',
})
export class TransactionImportService {
  // Progress tracking
  private progress$ = new BehaviorSubject<PipelineProgress>({
    stage: 'created',
    progress: 0,
    message: 'Ready to import',
    itemsProcessed: 0,
    itemsTotal: 0,
  });

  // Error tracking
  private errors$ = new Subject<{ stage: string; error: string }>();

  // Batch status tracking (in-memory, would be backed by DHT in production)
  private batches: Map<string, ImportBatch> = new Map();

  // Staged transactions (in-memory)
  private stagedTransactions: Map<string, StagedTransaction> = new Map();

  constructor(
    private plaid: PlaidIntegrationService,
    private duplicates: DuplicateDetectionService,
    private aiCategorization: AICategorizationService,
    private eventFactory: EconomicEventFactoryService,
    private budgetReconciliation: BudgetReconciliationService
  ) {}

  /**
   * Observables for UI
   */
  getProgress$(): Observable<PipelineProgress> {
    return this.progress$.asObservable();
  }

  getErrors$(): Observable<{ stage: string; error: string }> {
    return this.errors$.asObservable();
  }

  // ============================================================================
  // MAIN PIPELINE: FETCH → STAGE → CATEGORIZE
  // ============================================================================

  /**
   * Main entry point: orchestrates full import pipeline
   *
   * Stages 1-5: Fetch → Normalize → Deduplicate → Stage → Categorize
   *
   * Returns an ImportBatch that user can review and approve.
   */
  async executeImport(request: ImportRequest): Promise<ImportBatch> {
    const startTime = Date.now();

    try {
      // Create batch record
      const batch = await this.createBatch(request);

      // Stage 1: Fetch from Plaid
      this.updateProgress('fetching', 10, 'Fetching transactions from Plaid...');

      const connection = await this.getConnection(request.connectionId);
      const plaidTransactions = await this.plaid
        .fetchTransactions(connection, request.dateRange)
        .toPromise();

      if (!plaidTransactions || plaidTransactions.length === 0) {
        this.updateProgress('staging', 20, 'No transactions to import');
        batch.totalTransactions = 0;
        batch.status = 'completed';
        await this.updateBatch(batch);
        return batch;
      }

      batch.totalTransactions = plaidTransactions.length;

      // Stage 2: Normalize to internal format
      this.updateProgress(
        'normalizing',
        25,
        `Normalizing ${plaidTransactions.length} transactions...`
      );
      const normalized = this.normalizeTransactions(plaidTransactions);

      // Stage 3: Deduplicate
      this.updateProgress('deduplicating', 40, 'Detecting duplicates...');
      const unique = await this.duplicates.filterDuplicates(
        plaidTransactions
      );
      batch.duplicateTransactions = normalized.length - unique.length;
      batch.newTransactions = unique.length;

      // Stage 4: Create staged transactions
      this.updateProgress('staging', 60, `Creating ${unique.length} staged transactions...`);
      const staged = await this.stageTransactions(unique, batch);
      batch.stagedTransactionIds = staged.map(s => s.id);

      // Stage 5: Categorize (async, non-blocking)
      if (request.aiCategorizationEnabled !== false) {
        this.updateProgress(
          'categorizing',
          70,
          'Starting AI categorization (background)'
        );
        this.categorizeTransactionsAsync(staged, batch);
      }

      // Update batch status
      batch.status = 'staged';
      batch.aiCategorizationEnabled = request.aiCategorizationEnabled !== false;
      await this.updateBatch(batch);

      this.updateProgress('reviewing', 85, `Ready for review: ${staged.length} transactions`);
      console.log('[TransactionImport] Pipeline complete:', {
        batchId: batch.id,
        staged: staged.length,
        duplicates: batch.duplicateTransactions,
      });

      return batch;
    } catch (error) {
      console.error('[TransactionImport] Pipeline failed', error);
      this.errors$.next({ stage: 'pipeline', error: String(error) });
      throw error;
    }
  }

  /**
   * User approves a single staged transaction
   *
   * Stages 7-8: Approve → Create EconomicEvent + Reconcile
   */
  async approveTransaction(stagedId: string): Promise<void> {
    try {
      const staged = this.stagedTransactions.get(stagedId);
      if (!staged) {
        throw new Error(`Staged transaction ${stagedId} not found`);
      }

      if (staged.reviewStatus === 'approved') {
        console.log(`[TransactionImport] Transaction already approved: ${stagedId}`);
        return;
      }

      this.updateProgress('approving', 90, `Approving transaction ${stagedId}...`);

      // Create immutable EconomicEvent
      const event = await this.eventFactory.createFromStaged(staged);

      // Update staged transaction
      staged.reviewStatus = 'approved';
      staged.economicEventId = event.id;
      this.stagedTransactions.set(stagedId, staged);

      // Reconcile budget if linked
      if (staged.budgetId) {
        await this.budgetReconciliation.reconcileBudget(staged, event.id);
      }

      console.log('[TransactionImport] Transaction approved:', {
        stagedId,
        eventId: event.id,
        category: staged.category,
      });
    } catch (error) {
      console.error('[TransactionImport] Approval failed', error);
      this.errors$.next({
        stage: 'approval',
        error: `Failed to approve transaction: ${String(error)}`,
      });
      throw error;
    }
  }

  /**
   * User rejects a staged transaction
   */
  async rejectTransaction(stagedId: string, reason?: string): Promise<void> {
    const staged = this.stagedTransactions.get(stagedId);
    if (!staged) {
      throw new Error(`Staged transaction ${stagedId} not found`);
    }

    staged.reviewStatus = 'rejected';
    this.stagedTransactions.set(stagedId, staged);

    console.log('[TransactionImport] Transaction rejected:', {
      stagedId,
      reason,
    });
  }

  /**
   * User bulk-approves multiple transactions
   */
  async approveBatch(stagedIds: string[]): Promise<void> {
    const errors: Array<{ id: string; error: string }> = [];

    for (const id of stagedIds) {
      try {
        await this.approveTransaction(id);
      } catch (error) {
        errors.push({ id, error: String(error) });
      }
    }

    if (errors.length > 0) {
      console.warn('[TransactionImport] Some approvals failed:', errors);
      this.errors$.next({
        stage: 'bulk-approval',
        error: `${errors.length} transactions failed to approve`,
      });
    }
  }

  // ============================================================================
  // STAGE 2: NORMALIZATION
  // ============================================================================

  /**
   * Normalizes Plaid transactions to internal format
   */
  private normalizeTransactions(
    plaidTransactions: PlaidTransaction[]
  ): NormalizedTransaction[] {
    return plaidTransactions.map(txn => ({
      plaidTransactionId: txn.transaction_id,
      plaidAccountId: txn.account_id,
      timestamp: `${txn.date}T00:00:00Z`, // Plaid gives date only
      type: this.determineTransactionType(txn),
      amount: Math.abs(txn.amount),
      currency: txn.iso_currency_code || 'USD',
      description: txn.name,
      merchantName: txn.merchant_name,
      raw: txn,
    }));
  }

  /**
   * Determines transaction type (debit/credit/fee/transfer)
   */
  private determineTransactionType(
    txn: PlaidTransaction
  ): 'debit' | 'credit' | 'transfer' | 'fee' {
    const description = txn.name.toLowerCase();

    // Check for fee
    if (
      description.includes('fee') ||
      description.includes('charge') ||
      description.includes('interest')
    ) {
      return 'fee';
    }

    // Check for transfer
    if (
      description.includes('transfer') ||
      description.includes('move') ||
      description.includes('xfer')
    ) {
      return 'transfer';
    }

    // Amount sign indicates direction
    // (Plaid convention: negative = debit/expense, positive = credit/income)
    return txn.amount < 0 ? 'debit' : 'credit';
  }

  // ============================================================================
  // STAGE 4: CREATE STAGED TRANSACTIONS
  // ============================================================================

  /**
   * Creates StagedTransaction records for new transactions
   */
  private async stageTransactions(
    plaidTransactions: PlaidTransaction[],
    batch: ImportBatch
  ): Promise<StagedTransaction[]> {
    const staged: StagedTransaction[] = [];

    for (const plaidTxn of plaidTransactions) {
      const normalized = this.normalizeTransactions([plaidTxn])[0];

      const stagedTxn: StagedTransaction = {
        id: `staged-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
        batchId: batch.id,
        stewardId: batch.stewardId,

        plaidTransactionId: normalized.plaidTransactionId,
        plaidAccountId: normalized.plaidAccountId,
        financialAssetId: '', // Will be set by user

        timestamp: normalized.timestamp,
        type: normalized.type,
        amount: { value: normalized.amount, unit: normalized.currency },
        description: normalized.description,
        merchantName: normalized.merchantName,

        category: 'Uncategorized',
        categoryConfidence: 0,
        categorySource: 'manual',

        isDuplicate: false,
        reviewStatus: 'pending',

        plaidRawData: plaidTxn,
        createdAt: new Date().toISOString(),
      };

      staged.push(stagedTxn);
      this.stagedTransactions.set(stagedTxn.id, stagedTxn);
    }

    console.log(`[TransactionImport] Staged ${staged.length} transactions`);
    return staged;
  }

  // ============================================================================
  // STAGE 5: CATEGORIZATION (ASYNC)
  // ============================================================================

  /**
   * Categorizes transactions asynchronously (doesn't block pipeline)
   * Results are stored back in stagedTransactions when complete
   */
  private async categorizeTransactionsAsync(
    staged: StagedTransaction[],
    batch: ImportBatch
  ): Promise<void> {
    try {
      // TODO: Get actual budget categories from BudgetService
      const mockCategories = [
        {
          name: 'Groceries',
          description: 'Supermarkets, farmers markets',
        },
        {
          name: 'Dining',
          description: 'Restaurants, cafes',
        },
        {
          name: 'Shopping',
          description: 'Retail stores, online shopping',
        },
        {
          name: 'Transportation',
          description: 'Gas, rideshare, public transit',
        },
      ];

      // Batch categorization (max 50 per request)
      const batchSize = 50;
      for (let i = 0; i < staged.length; i += batchSize) {
        const batch = staged.slice(i, i + batchSize);

        try {
          const result = await this.aiCategorization.categorizeBatch(
            batch,
            mockCategories,
            batch[0].stewardId
          );

          // Store categorization results
          for (const catResult of result.results) {
            const stagedTxn = this.stagedTransactions.get(
              catResult.transactionId
            );
            if (stagedTxn) {
              stagedTxn.category = catResult.category;
              stagedTxn.categoryConfidence = catResult.confidence;
              stagedTxn.categorySource = 'ai';
              stagedTxn.suggestedCategories = catResult.alternatives;
              this.stagedTransactions.set(stagedTxn.id, stagedTxn);
            }
          }

          this.updateProgress(
            'categorizing',
            70 + (i / staged.length) * 15,
            `Categorizing: ${Math.min(i + batchSize, staged.length)}/${staged.length}`
          );
        } catch (error) {
          console.error('[TransactionImport] Categorization batch failed:', error);
          this.errors$.next({
            stage: 'categorization',
            error: `Failed to categorize batch ${Math.floor(i / batchSize)}: ${String(error)}`,
          });
        }
      }

      // Mark batch as categorized
      batch.aiCategorizationCompletedAt = new Date().toISOString();
      await this.updateBatch(batch);

      console.log('[TransactionImport] Categorization complete');
      this.updateProgress('reviewing', 85, 'Categorization complete - ready for review');
    } catch (error) {
      console.error('[TransactionImport] Categorization failed', error);
      this.errors$.next({
        stage: 'categorization',
        error: String(error),
      });
    }
  }

  // ============================================================================
  // BATCH MANAGEMENT
  // ============================================================================

  /**
   * Creates a new import batch
   */
  private async createBatch(request: ImportRequest): Promise<ImportBatch> {
    const batch: ImportBatch = {
      id: `batch-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
      batchNumber: `IB-${this.generateSequentialId()}`,
      stewardId: '', // Will be set by caller
      connectionId: request.connectionId,
      accountIds: request.accountIds || [],
      dateRange: request.dateRange,
      totalTransactions: 0,
      newTransactions: 0,
      duplicateTransactions: 0,
      errorTransactions: 0,
      status: 'fetching',
      stagedTransactionIds: [],
      aiCategorizationEnabled: request.aiCategorizationEnabled !== false,
      createdAt: new Date().toISOString(),
    };

    this.batches.set(batch.id, batch);
    return batch;
  }

  /**
   * Retrieves a batch
   */
  getBatch(batchId: string): ImportBatch | undefined {
    return this.batches.get(batchId);
  }

  /**
   * Gets all batches for a steward
   */
  getBatchesForSteward(stewardId: string): ImportBatch[] {
    return Array.from(this.batches.values()).filter(
      b => b.stewardId === stewardId
    );
  }

  /**
   * Updates a batch
   */
  private async updateBatch(batch: ImportBatch): Promise<void> {
    batch.updatedAt = new Date().toISOString();
    this.batches.set(batch.id, batch);
    // TODO: Persist to Holochain DHT
  }

  // ============================================================================
  // STAGED TRANSACTION MANAGEMENT
  // ============================================================================

  /**
   * Gets a staged transaction
   */
  getStagedTransaction(stagedId: string): StagedTransaction | undefined {
    return this.stagedTransactions.get(stagedId);
  }

  /**
   * Gets all staged transactions for a batch
   */
  getStagedTransactionsForBatch(batchId: string): StagedTransaction[] {
    return Array.from(this.stagedTransactions.values()).filter(
      s => s.batchId === batchId
    );
  }

  /**
   * Updates a categorization for a staged transaction
   */
  async updateStagedTransactionCategory(
    stagedId: string,
    category: string,
    budgetId?: string,
    budgetCategoryId?: string
  ): Promise<void> {
    const staged = this.stagedTransactions.get(stagedId);
    if (!staged) {
      throw new Error(`Staged transaction ${stagedId} not found`);
    }

    staged.category = category;
    staged.categorySource = 'manual';
    staged.budgetId = budgetId;
    staged.budgetCategoryId = budgetCategoryId;

    this.stagedTransactions.set(stagedId, staged);

    // Notify learning service
    if (staged.categoryConfidence < 80) {
      console.log(`[TransactionImport] Learning from correction: ${category}`);
      // TODO: await this.aiCategorization.learnFromCorrection(staged, category);
    }
  }

  // ============================================================================
  // PROGRESS TRACKING
  // ============================================================================

  /**
   * Updates progress for UI
   */
  private updateProgress(
    stage: PipelineProgress['stage'],
    progress: number,
    message: string,
    itemsProcessed?: number,
    itemsTotal?: number
  ): void {
    const current = this.progress$.value;
    this.progress$.next({
      stage,
      progress: Math.min(100, progress),
      message,
      itemsProcessed: itemsProcessed ?? current.itemsProcessed,
      itemsTotal: itemsTotal ?? current.itemsTotal,
    });
  }

  // ============================================================================
  // UTILITY METHODS
  // ============================================================================

  /**
   * Gets a connection (mock - would query DHT)
   */
  private async getConnection(connectionId: string): Promise<PlaidConnection> {
    // TODO: Query DHT for actual connection
    // For now, return a mock connection
    return {
      id: connectionId,
      connectionNumber: 'PC-MOCK001',
      stewardId: 'steward-123',
      plaidItemId: 'item-123',
      plaidAccessToken: 'encrypted-token',
      plaidInstitutionId: 'ins-3',
      institutionName: 'Chase Bank',
      linkedAccounts: [],
      status: 'active',
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    };
  }

  /**
   * Generates sequential-looking ID
   */
  private generateSequentialId(): string {
    const chars = '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ';
    let result = '';
    for (let i = 0; i < 8; i++) {
      result += chars.charAt(Math.floor(Math.random() * chars.length));
    }
    return result;
  }
}
