/**
 * Budget Reconciliation Service Tests
 *
 * Tests the pure business logic for reconciling staged transactions
 * with flow budgets, calculating variance, and generating alerts.
 */

import { TestBed } from '@angular/core/testing';

import { BudgetReconciliationService } from './budget-reconciliation.service';
import { StagedTransaction, ReconciliationResult } from '../models/transaction-import.model';

describe('BudgetReconciliationService', () => {
  let service: BudgetReconciliationService;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [BudgetReconciliationService],
    });
    service = TestBed.inject(BudgetReconciliationService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  // ==========================================================================
  // reconcileBudget Tests
  // ==========================================================================

  describe('reconcileBudget', () => {
    it('should return not reconciled result when no budget linkage', async () => {
      const staged = createStagedTransaction({
        budgetId: undefined,
        budgetCategoryId: undefined,
      });

      const result = await service.reconcileBudget(staged, 'event-123');

      expect(result.reconciled).toBeFalse();
      expect(result.budgetId).toBe('');
      expect(result.newHealthStatus).toBe('healthy');
    });

    it('should return not reconciled when budgetId missing', async () => {
      const staged = createStagedTransaction({
        budgetId: undefined,
        budgetCategoryId: 'cat-1',
      });

      const result = await service.reconcileBudget(staged, 'event-123');

      expect(result.reconciled).toBeFalse();
    });

    it('should return not reconciled when budgetCategoryId missing', async () => {
      const staged = createStagedTransaction({
        budgetId: 'budget-1',
        budgetCategoryId: undefined,
      });

      const result = await service.reconcileBudget(staged, 'event-123');

      expect(result.reconciled).toBeFalse();
    });

    it('should reconcile transaction with valid budget linkage', async () => {
      const staged = createStagedTransaction({
        budgetId: 'budget-1',
        budgetCategoryId: 'cat-1', // Groceries category in mock budget
        amount: { value: 100, unit: 'USD' },
        stewardId: 'steward-1',
      });

      const result = await service.reconcileBudget(staged, 'event-123');

      expect(result.reconciled).toBeTrue();
      expect(result.budgetId).toBe('budget-1');
      expect(result.budgetCategoryId).toBe('cat-1');
      expect(result.amountAdded).toBe(100);
      expect(result.previousActualAmount).toBe(0);
      expect(result.newActualAmount).toBe(100);
    });

    it('should throw error when budget category not found', async () => {
      const staged = createStagedTransaction({
        budgetId: 'budget-1',
        budgetCategoryId: 'non-existent-category',
        stewardId: 'steward-1',
      });

      await expectAsync(service.reconcileBudget(staged, 'event-123')).toBeRejectedWithError(
        /Budget category non-existent-category not found/
      );
    });

    it('should calculate variance correctly', async () => {
      const staged = createStagedTransaction({
        budgetId: 'budget-1',
        budgetCategoryId: 'cat-1', // Planned: 500 USD
        amount: { value: 600, unit: 'USD' }, // 100 over budget
        stewardId: 'steward-1',
      });

      const result = await service.reconcileBudget(staged, 'event-123');

      expect(result.varianceAfterReconciliation).toBe(100); // actual - planned
    });

    it('should include timestamp in result', async () => {
      const staged = createStagedTransaction({
        budgetId: 'budget-1',
        budgetCategoryId: 'cat-1',
        stewardId: 'steward-1',
      });

      const result = await service.reconcileBudget(staged, 'event-123');

      expect(result.timestamp).toBeTruthy();
      // Verify it's a valid ISO date
      expect(new Date(result.timestamp).toISOString()).toBe(result.timestamp);
    });
  });

  // ==========================================================================
  // Health Status Tests
  // ==========================================================================

  describe('health status calculation', () => {
    it('should return healthy when under budget', async () => {
      const staged = createStagedTransaction({
        budgetId: 'budget-1',
        budgetCategoryId: 'cat-1', // Planned: 500 USD
        amount: { value: 100, unit: 'USD' }, // Well under budget
        stewardId: 'steward-1',
      });

      const result = await service.reconcileBudget(staged, 'event-123');

      expect(result.newHealthStatus).toBe('healthy');
    });

    it('should return warning when 10-20% over budget', async () => {
      // Mock budget total is 1000 USD
      // Need to trigger warning (>10% over) but not critical (>20% over)
      const staged = createStagedTransaction({
        budgetId: 'budget-1',
        budgetCategoryId: 'cat-1', // Planned: 500 USD
        amount: { value: 1150, unit: 'USD' }, // 15% over total budget (1000)
        stewardId: 'steward-1',
      });

      const result = await service.reconcileBudget(staged, 'event-123');

      expect(result.newHealthStatus).toBe('warning');
    });

    it('should return critical when more than 20% over budget', async () => {
      // Mock budget total is 1000 USD
      const staged = createStagedTransaction({
        budgetId: 'budget-1',
        budgetCategoryId: 'cat-1',
        amount: { value: 1250, unit: 'USD' }, // 25% over total budget
        stewardId: 'steward-1',
      });

      const result = await service.reconcileBudget(staged, 'event-123');

      expect(result.newHealthStatus).toBe('critical');
    });
  });

  // ==========================================================================
  // reconcileMultiple Tests
  // ==========================================================================

  describe('reconcileMultiple', () => {
    it('should reconcile multiple transactions', async () => {
      const transactions = [
        {
          staged: createStagedTransaction({
            id: 'tx-1',
            budgetId: 'budget-1',
            budgetCategoryId: 'cat-1',
            amount: { value: 50, unit: 'USD' },
            stewardId: 'steward-1',
          }),
          eventId: 'event-1',
        },
        {
          staged: createStagedTransaction({
            id: 'tx-2',
            budgetId: 'budget-1',
            budgetCategoryId: 'cat-2',
            amount: { value: 30, unit: 'USD' },
            stewardId: 'steward-1',
          }),
          eventId: 'event-2',
        },
      ];

      const results = await service.reconcileMultiple(transactions);

      expect(results.length).toBe(2);
      expect(results[0].reconciled).toBeTrue();
      expect(results[1].reconciled).toBeTrue();
    });

    it('should continue processing after individual transaction failure', async () => {
      const transactions = [
        {
          staged: createStagedTransaction({
            id: 'tx-1',
            budgetId: 'budget-1',
            budgetCategoryId: 'non-existent', // Will fail
            stewardId: 'steward-1',
          }),
          eventId: 'event-1',
        },
        {
          staged: createStagedTransaction({
            id: 'tx-2',
            budgetId: 'budget-1',
            budgetCategoryId: 'cat-1', // Will succeed
            amount: { value: 30, unit: 'USD' },
            stewardId: 'steward-1',
          }),
          eventId: 'event-2',
        },
      ];

      const results = await service.reconcileMultiple(transactions);

      // Only the successful one should be in results
      expect(results.length).toBe(1);
      expect(results[0].reconciled).toBeTrue();
    });

    it('should return empty array for empty input', async () => {
      const results = await service.reconcileMultiple([]);

      expect(results).toEqual([]);
    });

    it('should handle transactions without budget linkage', async () => {
      const transactions = [
        {
          staged: createStagedTransaction({
            id: 'tx-1',
            budgetId: undefined,
            budgetCategoryId: undefined,
          }),
          eventId: 'event-1',
        },
      ];

      const results = await service.reconcileMultiple(transactions);

      expect(results.length).toBe(1);
      expect(results[0].reconciled).toBeFalse();
    });
  });

  // ==========================================================================
  // Variance Tracking Tests
  // ==========================================================================

  describe('variance tracking', () => {
    it('should calculate variance before and after reconciliation', async () => {
      const staged = createStagedTransaction({
        budgetId: 'budget-1',
        budgetCategoryId: 'cat-1', // Planned: 500 USD
        amount: { value: 200, unit: 'USD' },
        stewardId: 'steward-1',
      });

      const result = await service.reconcileBudget(staged, 'event-123');

      expect(result.varianceBeforeReconciliation).toBe(-500); // 0 actual - 500 planned
      expect(result.varianceAfterReconciliation).toBe(-300); // 200 actual - 500 planned
    });

    it('should handle negative variance (under budget)', async () => {
      const staged = createStagedTransaction({
        budgetId: 'budget-1',
        budgetCategoryId: 'cat-1', // Planned: 500 USD
        amount: { value: 100, unit: 'USD' },
        stewardId: 'steward-1',
      });

      const result = await service.reconcileBudget(staged, 'event-123');

      expect(result.varianceAfterReconciliation).toBeLessThan(0);
    });

    it('should handle positive variance (over budget)', async () => {
      const staged = createStagedTransaction({
        budgetId: 'budget-1',
        budgetCategoryId: 'cat-1', // Planned: 500 USD
        amount: { value: 600, unit: 'USD' },
        stewardId: 'steward-1',
      });

      const result = await service.reconcileBudget(staged, 'event-123');

      expect(result.varianceAfterReconciliation).toBeGreaterThan(0);
    });
  });
});

// ==========================================================================
// Test Helpers
// ==========================================================================

/**
 * Creates a staged transaction for testing.
 * Matches the StagedTransaction interface from transaction-import.model.ts
 */
function createStagedTransaction(overrides: Partial<StagedTransaction> = {}): StagedTransaction {
  const now = new Date().toISOString();

  return {
    id: 'staged-' + Math.random().toString(36).substr(2, 9),
    batchId: 'batch-1',
    stewardId: 'steward-1',
    plaidTransactionId: 'plaid-tx-1',
    plaidAccountId: 'plaid-acct-1',
    financialAssetId: 'asset-1',
    timestamp: now,
    type: 'debit',
    amount: { value: 50, unit: 'USD' },
    description: 'Test transaction',
    merchantName: 'Test Merchant',
    category: 'Groceries',
    categoryConfidence: 85,
    categorySource: 'ai',
    isDuplicate: false,
    reviewStatus: 'pending',
    plaidRawData: {},
    createdAt: now,
    ...overrides,
  };
}
