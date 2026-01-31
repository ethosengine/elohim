/**
 * Economic-event-factory Service Tests
 *
 * Tests transformation of StagedTransactions into immutable EconomicEvents:
 * - Transaction type to event type mapping
 * - Provider/receiver determination
 * - Metadata preservation and Plaid reconciliation
 * - Batch event creation
 * - Correction event generation
 */

import { TestBed } from '@angular/core/testing';

import { EconomicEventFactoryService } from './economic-event-factory.service';
import { StagedTransaction } from '../models/transaction-import.model';

describe('EconomicEventFactoryService', () => {
  let service: EconomicEventFactoryService;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [EconomicEventFactoryService],
    });
    service = TestBed.inject(EconomicEventFactoryService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should have service instance', () => {
    expect(service).toBeDefined();
  });

  // =========================================================================
  // Single Transaction Conversion
  // =========================================================================

  describe('createFromStaged', () => {
    it('should create event from approved debit transaction', async () => {
      const staged = createMockStagedTransaction('tx-1', 'approved', -100, 'debit');

      const event = await service.createFromStaged(staged);

      expect(event).toBeDefined();
      expect(event.id).toBeDefined();
      expect(event.providerId).toBe(staged.stewardId);
      expect(event.receiverId).toBe('Test Merchant'); // Uses merchantName from staged transaction
    });

    it('should create event from approved credit transaction', async () => {
      const staged = createMockStagedTransaction('tx-1', 'approved', 500, 'credit');

      const event = await service.createFromStaged(staged);

      expect(event).toBeDefined();
      expect(event.providerId).toBe('external-party');
      expect(event.receiverId).toBe(staged.stewardId);
    });

    it('should create event from approved fee transaction', async () => {
      const staged = createMockStagedTransaction('tx-1', 'approved', -5, 'fee');

      const event = await service.createFromStaged(staged);

      expect(event).toBeDefined();
      expect(event.providerId).toBe(staged.stewardId);
      expect(event.receiverId).toBeDefined(); // Fee collector
    });

    it('should create event from approved transfer transaction', async () => {
      const staged = createMockStagedTransaction('tx-1', 'approved', -200, 'transfer');

      const event = await service.createFromStaged(staged);

      expect(event).toBeDefined();
      expect(event.providerId).toBe(staged.stewardId);
      expect(event.receiverId).toBeDefined();
    });

    it('should reject non-approved transactions', async () => {
      const staged = createMockStagedTransaction('tx-1', 'pending', -100, 'debit');

      await expectAsync(service.createFromStaged(staged)).toBeRejectedWithError(
        /non-approved/
      );
    });

    it('should reject already-converted transactions', async () => {
      const staged = createMockStagedTransaction('tx-1', 'approved', -100, 'debit', 'event-123');

      await expectAsync(service.createFromStaged(staged)).toBeRejectedWithError(
        /already created/
      );
    });

    it('should preserve transaction quantity and unit', async () => {
      const staged = createMockStagedTransaction('tx-1', 'approved', -100, 'debit');
      staged.amount = { value: 100, unit: 'USD' };

      const event = await service.createFromStaged(staged);

      expect(event.quantity).toBe(100);
      expect(event.unit).toBe('USD');
    });

    it('should set event state to validated', async () => {
      const staged = createMockStagedTransaction('tx-1', 'approved', -100, 'debit');

      const event = await service.createFromStaged(staged);

      expect(event.state.status).toBe('validated');
    });

    it('should set event created timestamp', async () => {
      const staged = createMockStagedTransaction('tx-1', 'approved', -100, 'debit');

      const event = await service.createFromStaged(staged);

      expect(event.createdAt).toBeDefined();
      expect(event.createdBy).toBe(staged.stewardId);
    });
  });

  // =========================================================================
  // Event Type Mapping
  // =========================================================================

  describe('event type mapping', () => {
    it('should map debit to credit-transfer', async () => {
      const staged = createMockStagedTransaction('tx-1', 'approved', -100, 'debit');

      const event = await service.createFromStaged(staged);

      expect(event.eventType).toBe('credit-transfer');
    });

    it('should map credit to credit-transfer', async () => {
      const staged = createMockStagedTransaction('tx-1', 'approved', 500, 'credit');

      const event = await service.createFromStaged(staged);

      expect(event.eventType).toBe('credit-transfer');
    });

    it('should map fee to credit-retire', async () => {
      const staged = createMockStagedTransaction('tx-1', 'approved', -5, 'fee');

      const event = await service.createFromStaged(staged);

      expect(event.eventType).toBe('credit-retire');
    });

    it('should map transfer to credit-transfer', async () => {
      const staged = createMockStagedTransaction('tx-1', 'approved', 200, 'transfer');

      const event = await service.createFromStaged(staged);

      expect(event.eventType).toBe('credit-transfer');
    });
  });

  // =========================================================================
  // Action Determination
  // =========================================================================

  describe('action determination', () => {
    it('should determine action for credit-transfer event', async () => {
      const staged = createMockStagedTransaction('tx-1', 'approved', -100, 'debit');

      const event = await service.createFromStaged(staged);

      expect(event.action).toBe('transfer');
    });

    it('should determine action for credit-retire event', async () => {
      const staged = createMockStagedTransaction('tx-1', 'approved', -5, 'fee');

      const event = await service.createFromStaged(staged);

      expect(event.action).toBe('consume');
    });
  });

  // =========================================================================
  // Metadata Preservation
  // =========================================================================

  describe('metadata preservation', () => {
    it('should preserve Plaid transaction ID for reconciliation', async () => {
      const staged = createMockStagedTransaction('tx-1', 'approved', -100, 'debit');
      staged.plaidTransactionId = 'plaid-tx-12345';

      const event = await service.createFromStaged(staged);

      expect(event.metadata['plaidTransactionId']).toBe('plaid-tx-12345');
    });

    it('should preserve Plaid account ID', async () => {
      const staged = createMockStagedTransaction('tx-1', 'approved', -100, 'debit');
      staged.plaidAccountId = 'plaid-acct-abc';

      const event = await service.createFromStaged(staged);

      expect(event.metadata['plaidAccountId']).toBe('plaid-acct-abc');
    });

    it('should preserve categorization info', async () => {
      const staged = createMockStagedTransaction('tx-1', 'approved', -100, 'debit');
      staged.category = 'Groceries';
      staged.categoryConfidence = 95;
      staged.categorySource = 'ai';

      const event = await service.createFromStaged(staged);

      expect(event.metadata['category']).toBe('Groceries');
      expect(event.metadata['categoryConfidence']).toBe(95);
      expect(event.metadata['categorySource']).toBe('ai');
    });

    it('should preserve budget linkage', async () => {
      const staged = createMockStagedTransaction('tx-1', 'approved', -100, 'debit');
      staged.budgetId = 'budget-1';
      staged.budgetCategoryId = 'cat-groceries';

      const event = await service.createFromStaged(staged);

      expect(event.metadata['budgetId']).toBe('budget-1');
      expect(event.metadata['budgetCategoryId']).toBe('cat-groceries');
    });

    it('should preserve import batch ID', async () => {
      const staged = createMockStagedTransaction('tx-1', 'approved', -100, 'debit');
      staged.batchId = 'batch-import-1';

      const event = await service.createFromStaged(staged);

      expect(event.metadata['importBatchId']).toBe('batch-import-1');
    });

    it('should preserve original Plaid raw data', async () => {
      const staged = createMockStagedTransaction('tx-1', 'approved', -100, 'debit');
      staged.plaidRawData = { raw_field: 'value' };

      const event = await service.createFromStaged(staged);

      expect(event.metadata['plaidRawData']).toBeDefined();
    });

    it('should include source indicator', async () => {
      const staged = createMockStagedTransaction('tx-1', 'approved', -100, 'debit');

      const event = await service.createFromStaged(staged);

      expect(event.metadata['source']).toBe('plaid-import');
    });

    it('should include audit trail info', async () => {
      const staged = createMockStagedTransaction('tx-1', 'approved', -100, 'debit');
      staged.id = 'staged-123';

      const event = await service.createFromStaged(staged);

      expect(event.metadata['eventFactory']).toBe('economic-event-factory-service');
      expect(event.metadata['stagedTransactionId']).toBe('staged-123');
    });
  });

  // =========================================================================
  // Note Generation
  // =========================================================================

  describe('note generation', () => {
    it('should include merchant name in note', async () => {
      const staged = createMockStagedTransaction('tx-1', 'approved', -100, 'debit');
      staged.merchantName = 'Whole Foods';

      const event = await service.createFromStaged(staged);

      expect(event.note).toContain('Whole Foods');
    });

    it('should include description in note', async () => {
      const staged = createMockStagedTransaction('tx-1', 'approved', -100, 'debit');
      staged.merchantName = 'Store';
      staged.description = 'Weekly groceries';

      const event = await service.createFromStaged(staged);

      expect(event.note).toContain('Weekly groceries');
    });

    it('should include account source in note', async () => {
      const staged = createMockStagedTransaction('tx-1', 'approved', -100, 'debit');
      staged.plaidAccountId = 'checking-123';

      const event = await service.createFromStaged(staged);

      expect(event.note).toContain('checking-123');
    });

    it('should avoid duplicating merchant and description', async () => {
      const staged = createMockStagedTransaction('tx-1', 'approved', -100, 'debit');
      staged.merchantName = 'Store';
      staged.description = 'Store'; // Same as merchant

      const event = await service.createFromStaged(staged);

      const storeCount = ((event.note || '').match(/Store/g) || []).length;
      expect(storeCount).toBeLessThanOrEqual(2); // Only in prefix and source
    });
  });

  // =========================================================================
  // Batch Creation
  // =========================================================================

  describe('createMultipleFromStaged', () => {
    it('should create events from multiple staged transactions', async () => {
      const stagedList = [
        createMockStagedTransaction('tx-1', 'approved', -100, 'debit'),
        createMockStagedTransaction('tx-2', 'approved', 500, 'credit'),
        createMockStagedTransaction('tx-3', 'approved', -50, 'debit'),
      ];

      const events = await service.createMultipleFromStaged(stagedList);

      expect(events.length).toBe(3);
      expect(events[0].id).toBeDefined();
      expect(events[1].id).toBeDefined();
      expect(events[2].id).toBeDefined();
    });

    it('should skip non-approved transactions in batch', async () => {
      const stagedList = [
        createMockStagedTransaction('tx-1', 'approved', -100, 'debit'),
        createMockStagedTransaction('tx-2', 'pending', 500, 'credit'),
        createMockStagedTransaction('tx-3', 'approved', -50, 'debit'),
      ];

      const events = await service.createMultipleFromStaged(stagedList);

      expect(events.length).toBe(2); // Only approved ones
    });

    it('should continue processing after individual failure', async () => {
      const stagedList = [
        createMockStagedTransaction('tx-1', 'approved', -100, 'debit'),
        createMockStagedTransaction('tx-2', 'approved', 500, 'credit', 'event-existing'),
        createMockStagedTransaction('tx-3', 'approved', -50, 'debit'),
      ];

      const events = await service.createMultipleFromStaged(stagedList);

      // Should skip the one with already-created event, but continue
      expect(events.length).toBe(2);
      expect(events[0].id).toBeDefined();
      expect(events[1].id).toBeDefined();
    });

    it('should handle empty batch', async () => {
      const events = await service.createMultipleFromStaged([]);

      expect(events.length).toBe(0);
    });

    it('should handle batch with all rejected transactions', async () => {
      const stagedList = [
        createMockStagedTransaction('tx-1', 'rejected', -100, 'debit'),
        createMockStagedTransaction('tx-2', 'pending', 500, 'credit'),
      ];

      const events = await service.createMultipleFromStaged(stagedList);

      expect(events.length).toBe(0);
    });
  });

  // =========================================================================
  // Correction Events
  // =========================================================================

  describe('correction events', () => {
    it('should have createCorrectionEvent method', () => {
      expect(service.createCorrectionEvent).toBeDefined();
      expect(typeof service.createCorrectionEvent).toBe('function');
    });

    it('should reject correction event creation when not implemented', async () => {
      await expectAsync(
        service.createCorrectionEvent('event-1', {}, 'Incorrect amount')
      ).toBeRejectedWithError(/not yet implemented/i);
    });
  });

  // =========================================================================
  // Provider/Receiver Determination
  // =========================================================================

  describe('provider/receiver determination', () => {
    it('should set steward as provider for debit', async () => {
      const staged = createMockStagedTransaction('tx-1', 'approved', -100, 'debit');
      staged.stewardId = 'steward-alice';

      const event = await service.createFromStaged(staged);

      expect(event.providerId).toBe('steward-alice');
    });

    it('should set steward as receiver for credit', async () => {
      const staged = createMockStagedTransaction('tx-1', 'approved', 500, 'credit');
      staged.stewardId = 'steward-bob';

      const event = await service.createFromStaged(staged);

      expect(event.receiverId).toBe('steward-bob');
    });

    it('should use merchant name as receiver for debit', async () => {
      const staged = createMockStagedTransaction('tx-1', 'approved', -100, 'debit');
      staged.merchantName = 'Best Store';

      const event = await service.createFromStaged(staged);

      expect(event.receiverId).toBe('Best Store');
    });

    it('should use fee collector for fee transaction', async () => {
      const staged = createMockStagedTransaction('tx-1', 'approved', -5, 'fee');
      staged.merchantName = 'Bank of Trust';

      const event = await service.createFromStaged(staged);

      expect(event.receiverId).toBe('Bank of Trust');
    });

    it('should fallback to external-party when no merchant', async () => {
      const staged = createMockStagedTransaction('tx-1', 'approved', -100, 'debit');
      staged.merchantName = undefined;

      const event = await service.createFromStaged(staged);

      expect(event.receiverId).toBe('external-party');
    });
  });
});

// =========================================================================
// Test Helpers
// =========================================================================

function createMockStagedTransaction(
  id: string,
  reviewStatus: 'pending' | 'approved' | 'rejected',
  amount: number,
  type: 'debit' | 'credit' | 'fee' | 'transfer',
  economicEventId?: string
): StagedTransaction {
  return {
    id,
    reviewStatus,
    economicEventId,
    type,
    amount: {
      value: Math.abs(amount),
      unit: 'USD'
    },
    timestamp: new Date().toISOString(),
    description: 'Test transaction',
    category: 'Test',
    categorySource: 'manual',
    categoryConfidence: 100,
    stewardId: 'steward-test',
    batchId: 'batch-test',
    plaidTransactionId: `plaid-tx-${id}`,
    plaidAccountId: 'plaid-acct-test',
    financialAssetId: 'asset-test',
    merchantName: 'Test Merchant',
    plaidRawData: {},
    budgetId: undefined,
    budgetCategoryId: undefined,
    isDuplicate: false,
    createdAt: new Date().toISOString()
  };
}
