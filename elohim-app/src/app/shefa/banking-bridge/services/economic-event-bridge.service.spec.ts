/**
 * Economic Event Bridge Service Tests
 *
 * Tests the translation layer between banking-bridge and Holochain:
 * - Transforming StagedTransactions to EconomicEvents
 * - Committing individual and batch transactions
 * - Retrieving committed events
 * - Preventing double-commits
 */

import { TestBed } from '@angular/core/testing';

import { EconomicEventBridgeService, EconomicEventPayload, CommitResult, BatchCommitResult } from './economic-event-bridge.service';
import { HolochainClientService } from '@app/elohim/services/holochain-client.service';
import { bankingStore, StagedTransactionLocal } from '../stores/banking-store';

describe('EconomicEventBridgeService', () => {
  let service: EconomicEventBridgeService;
  let mockHolochain: jasmine.SpyObj<HolochainClientService>;

  beforeEach(() => {
    mockHolochain = jasmine.createSpyObj('HolochainClientService', ['callZome']);

    TestBed.configureTestingModule({
      providers: [
        EconomicEventBridgeService,
        { provide: HolochainClientService, useValue: mockHolochain }
      ],
    });
    service = TestBed.inject(EconomicEventBridgeService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should have service instance', () => {
    expect(service).toBeDefined();
  });

  // =========================================================================
  // Transaction Commitment
  // =========================================================================

  describe('commitStagedTransaction', () => {
    it('should commit approved staged transaction to Holochain', async () => {
      const mockStaged = createMockStagedTransaction('staged-1', 'approved', 100);
      spyOn(bankingStore, 'getStaged').and.returnValue(Promise.resolve(mockStaged));
      spyOn(bankingStore, 'saveStaged').and.returnValue(Promise.resolve());

      mockHolochain.callZome.and.returnValue(Promise.resolve({
        success: true,
        data: 'action-hash-123'
      }));

      const result = await service.commitStagedTransaction('staged-1');

      expect(result.success).toBe(true);
      expect(result.economicEventId).toBeDefined();
      expect(mockHolochain.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          zomeName: 'content_store',
          fnName: 'create_economic_event'
        })
      );
    });

    it('should return error for non-existent staged transaction', async () => {
      spyOn(bankingStore, 'getStaged').and.returnValue(Promise.resolve(undefined));

      const result = await service.commitStagedTransaction('non-existent');

      expect(result.success).toBe(false);
      expect(result.error).toContain('not found');
    });

    it('should reject non-approved transactions', async () => {
      const mockStaged = createMockStagedTransaction('staged-1', 'pending', 100);
      spyOn(bankingStore, 'getStaged').and.returnValue(Promise.resolve(mockStaged));

      const result = await service.commitStagedTransaction('staged-1');

      expect(result.success).toBe(false);
      expect(result.error).toContain('not approved');
    });

    it('should handle already committed transactions', async () => {
      const mockStaged = createMockStagedTransaction('staged-1', 'approved', 100, 'existing-event-id');
      spyOn(bankingStore, 'getStaged').and.returnValue(Promise.resolve(mockStaged));

      const result = await service.commitStagedTransaction('staged-1');

      expect(result.success).toBe(true);
      expect(result.error).toContain('Already committed');
    });

    it('should handle Holochain zome call failure', async () => {
      const mockStaged = createMockStagedTransaction('staged-1', 'approved', 100);
      spyOn(bankingStore, 'getStaged').and.returnValue(Promise.resolve(mockStaged));

      mockHolochain.callZome.and.returnValue(Promise.resolve({
        success: false,
        error: 'DHT error'
      }));

      const result = await service.commitStagedTransaction('staged-1');

      expect(result.success).toBe(false);
      expect(result.error).toContain('DHT error');
    });

    it('should catch and return errors', async () => {
      spyOn(bankingStore, 'getStaged').and.returnValue(Promise.reject(new Error('Store error')));

      const result = await service.commitStagedTransaction('staged-1');

      expect(result.success).toBe(false);
      expect(result.error).toContain('Store error');
    });

    it('should update staged transaction with event ID after successful commit', async () => {
      const mockStaged = createMockStagedTransaction('staged-1', 'approved', 100);
      spyOn(bankingStore, 'getStaged').and.returnValue(Promise.resolve(mockStaged));
      const saveSpy = spyOn(bankingStore, 'saveStaged').and.returnValue(Promise.resolve());

      mockHolochain.callZome.and.returnValue(Promise.resolve({
        success: true,
        data: 'action-hash-abc'
      }));

      await service.commitStagedTransaction('staged-1');

      expect(saveSpy).toHaveBeenCalled();
      const savedTx = saveSpy.calls.mostRecent().args[0];
      expect(savedTx.economicEventId).toBeDefined();
    });
  });

  // =========================================================================
  // Transaction Transformation
  // =========================================================================

  describe('transaction transformation', () => {
    it('should transform debit transaction to consume action', async () => {
      const mockStaged = createMockStagedTransaction('staged-1', 'approved', -50, undefined, 'debit');
      spyOn(bankingStore, 'getStaged').and.returnValue(Promise.resolve(mockStaged));
      spyOn(bankingStore, 'saveStaged').and.returnValue(Promise.resolve());

      mockHolochain.callZome.and.returnValue(Promise.resolve({
        success: true,
        data: 'hash'
      }));

      const result = await service.commitStagedTransaction('staged-1');

      expect(result.success).toBe(true);
      // Verify callZome was called with consume action
      const payload = mockHolochain.callZome.calls.mostRecent().args[0].payload as EconomicEventPayload;
      expect(payload.action).toBe('consume');
    });

    it('should transform credit transaction to produce action', async () => {
      const mockStaged = createMockStagedTransaction('staged-1', 'approved', 500, undefined, 'credit');
      spyOn(bankingStore, 'getStaged').and.returnValue(Promise.resolve(mockStaged));
      spyOn(bankingStore, 'saveStaged').and.returnValue(Promise.resolve());

      mockHolochain.callZome.and.returnValue(Promise.resolve({
        success: true,
        data: 'hash'
      }));

      await service.commitStagedTransaction('staged-1');

      const payload = mockHolochain.callZome.calls.mostRecent().args[0].payload as EconomicEventPayload;
      expect(payload.action).toBe('produce');
    });

    it('should transform transfer transaction correctly', async () => {
      const mockStaged = createMockStagedTransaction('staged-1', 'approved', 200, undefined, 'transfer');
      spyOn(bankingStore, 'getStaged').and.returnValue(Promise.resolve(mockStaged));
      spyOn(bankingStore, 'saveStaged').and.returnValue(Promise.resolve());

      mockHolochain.callZome.and.returnValue(Promise.resolve({
        success: true,
        data: 'hash'
      }));

      await service.commitStagedTransaction('staged-1');

      const payload = mockHolochain.callZome.calls.mostRecent().args[0].payload as EconomicEventPayload;
      expect(payload.action).toBe('transfer');
    });

    it('should transform fee transaction to consume action', async () => {
      const mockStaged = createMockStagedTransaction('staged-1', 'approved', -5, undefined, 'fee');
      spyOn(bankingStore, 'getStaged').and.returnValue(Promise.resolve(mockStaged));
      spyOn(bankingStore, 'saveStaged').and.returnValue(Promise.resolve());

      mockHolochain.callZome.and.returnValue(Promise.resolve({
        success: true,
        data: 'hash'
      }));

      await service.commitStagedTransaction('staged-1');

      const payload = mockHolochain.callZome.calls.mostRecent().args[0].payload as EconomicEventPayload;
      expect(payload.action).toBe('consume');
    });

    it('should set provider and receiver based on transaction type', async () => {
      const mockStaged = createMockStagedTransaction('staged-1', 'approved', -50, undefined, 'debit');
      mockStaged.stewardId = 'steward-123';
      mockStaged.merchantName = 'Store ABC';
      spyOn(bankingStore, 'getStaged').and.returnValue(Promise.resolve(mockStaged));
      spyOn(bankingStore, 'saveStaged').and.returnValue(Promise.resolve());

      mockHolochain.callZome.and.returnValue(Promise.resolve({
        success: true,
        data: 'hash'
      }));

      await service.commitStagedTransaction('staged-1');

      const payload = mockHolochain.callZome.calls.mostRecent().args[0].payload as EconomicEventPayload;
      expect(payload.provider).toBe('steward-123'); // Provider for debit
      expect(payload.receiver).toBe('Store ABC'); // Receiver is merchant
    });

    it('should include resource classifications', async () => {
      const mockStaged = createMockStagedTransaction('staged-1', 'approved', -50, undefined, 'debit');
      mockStaged.category = 'Groceries';
      spyOn(bankingStore, 'getStaged').and.returnValue(Promise.resolve(mockStaged));
      spyOn(bankingStore, 'saveStaged').and.returnValue(Promise.resolve());

      mockHolochain.callZome.and.returnValue(Promise.resolve({
        success: true,
        data: 'hash'
      }));

      await service.commitStagedTransaction('staged-1');

      const payload = mockHolochain.callZome.calls.mostRecent().args[0].payload as EconomicEventPayload;
      expect(payload.resourceClassifiedAs).toContain('Groceries');
      expect(payload.resourceClassifiedAs).toContain('debit');
      expect(payload.resourceClassifiedAs).toContain('bank-import');
    });

    it('should preserve metadata with Plaid provenance', async () => {
      const mockStaged = createMockStagedTransaction('staged-1', 'approved', -50, undefined, 'debit');
      mockStaged.plaidTransactionId = 'plaid-tx-123';
      mockStaged.plaidAccountId = 'plaid-acct-456';
      spyOn(bankingStore, 'getStaged').and.returnValue(Promise.resolve(mockStaged));
      spyOn(bankingStore, 'saveStaged').and.returnValue(Promise.resolve());

      mockHolochain.callZome.and.returnValue(Promise.resolve({
        success: true,
        data: 'hash'
      }));

      await service.commitStagedTransaction('staged-1');

      const payload = mockHolochain.callZome.calls.mostRecent().args[0].payload as EconomicEventPayload;
      expect(payload.metadata['plaid_transaction_id']).toBe('plaid-tx-123');
      expect(payload.metadata['plaid_account_id']).toBe('plaid-acct-456');
      expect(payload.metadata['source']).toBe('plaid-import');
    });

    it('should set event state to completed', async () => {
      const mockStaged = createMockStagedTransaction('staged-1', 'approved', -50);
      spyOn(bankingStore, 'getStaged').and.returnValue(Promise.resolve(mockStaged));
      spyOn(bankingStore, 'saveStaged').and.returnValue(Promise.resolve());

      mockHolochain.callZome.and.returnValue(Promise.resolve({
        success: true,
        data: 'hash'
      }));

      await service.commitStagedTransaction('staged-1');

      const payload = mockHolochain.callZome.calls.mostRecent().args[0].payload as EconomicEventPayload;
      expect(payload.state).toBe('completed');
    });
  });

  // =========================================================================
  // Batch Commitment
  // =========================================================================

  describe('commitBatch', () => {
    it('should commit all approved transactions in batch', async () => {
      const mockBatch: any = {
        id: 'batch-1',
        status: 'pending',
        batchNumber: 'IB-001',
        stewardId: 'steward-123',
        connectionId: 'conn-123',
        accountIds: [],
        dateRange: { start: '2024-01-01', end: '2024-01-31' },
        totalTransactions: 2,
        newTransactions: 2,
        duplicateTransactions: 0,
        errorTransactions: 0,
        stagedTransactionIds: [],
        aiCategorizationEnabled: false,
        createdAt: new Date().toISOString()
      };
      const stagedTxs = [
        createMockStagedTransaction('staged-1', 'approved', 100),
        createMockStagedTransaction('staged-2', 'approved', 200),
      ];

      spyOn(bankingStore, 'getBatch').and.returnValue(Promise.resolve(mockBatch));
      spyOn(bankingStore, 'getStagedByBatch').and.returnValue(Promise.resolve(stagedTxs));
      spyOn(bankingStore, 'getStaged').and.callFake((id: string) => {
        return Promise.resolve(stagedTxs.find(tx => tx.id === id));
      });
      spyOn(bankingStore, 'saveStaged').and.returnValue(Promise.resolve());
      spyOn(bankingStore, 'saveBatch').and.returnValue(Promise.resolve());

      mockHolochain.callZome.and.returnValue(Promise.resolve({
        success: true,
        data: 'hash'
      }));

      const result = await service.commitBatch('batch-1');

      expect(result.successCount).toBe(2);
      expect(result.totalAttempted).toBe(2);
    });

    it('should handle batch with non-existent batch ID', async () => {
      spyOn(bankingStore, 'getBatch').and.returnValue(Promise.resolve(undefined));

      const result = await service.commitBatch('non-existent');

      expect(result.successCount).toBe(0);
      expect(result.totalAttempted).toBe(0);
      expect(result.results.length).toBe(0);
    });

    it('should skip already committed transactions in batch', async () => {
      const mockBatch: any = {
        id: 'batch-1',
        status: 'pending',
        batchNumber: 'IB-001',
        stewardId: 'steward-123',
        connectionId: 'conn-123',
        accountIds: [],
        dateRange: { start: '2024-01-01', end: '2024-01-31' },
        totalTransactions: 2,
        newTransactions: 2,
        duplicateTransactions: 0,
        errorTransactions: 0,
        stagedTransactionIds: [],
        aiCategorizationEnabled: false,
        createdAt: new Date().toISOString()
      };
      const stagedTxs = [
        createMockStagedTransaction('staged-1', 'approved', 100),
        createMockStagedTransaction('staged-2', 'approved', 200, 'existing-event-id'),
      ];

      spyOn(bankingStore, 'getBatch').and.returnValue(Promise.resolve(mockBatch));
      spyOn(bankingStore, 'getStagedByBatch').and.returnValue(Promise.resolve(stagedTxs));
      spyOn(bankingStore, 'getStaged').and.callFake((id: string) => {
        return Promise.resolve(stagedTxs.find(tx => tx.id === id));
      });
      spyOn(bankingStore, 'saveStaged').and.returnValue(Promise.resolve());
      spyOn(bankingStore, 'saveBatch').and.returnValue(Promise.resolve());

      mockHolochain.callZome.and.returnValue(Promise.resolve({
        success: true,
        data: 'hash'
      }));

      const result = await service.commitBatch('batch-1');

      // Only 1 new transaction, 1 already committed
      expect(result.totalAttempted).toBe(1);
    });

    it('should continue processing after individual failure', async () => {
      const mockBatch: any = {
        id: 'batch-1',
        status: 'pending',
        batchNumber: 'IB-001',
        stewardId: 'steward-123',
        connectionId: 'conn-123',
        accountIds: [],
        dateRange: { start: '2024-01-01', end: '2024-01-31' },
        totalTransactions: 3,
        newTransactions: 3,
        duplicateTransactions: 0,
        errorTransactions: 0,
        stagedTransactionIds: [],
        aiCategorizationEnabled: false,
        createdAt: new Date().toISOString()
      };
      const stagedTxs = [
        createMockStagedTransaction('staged-1', 'approved', 100),
        createMockStagedTransaction('staged-2', 'approved', 200),
        createMockStagedTransaction('staged-3', 'approved', 300),
      ];

      spyOn(bankingStore, 'getBatch').and.returnValue(Promise.resolve(mockBatch));
      spyOn(bankingStore, 'getStagedByBatch').and.returnValue(Promise.resolve(stagedTxs));
      spyOn(bankingStore, 'getStaged').and.callFake((id: string) => {
        return Promise.resolve(stagedTxs.find(tx => tx.id === id));
      });
      spyOn(bankingStore, 'saveStaged').and.returnValue(Promise.resolve());
      spyOn(bankingStore, 'saveBatch').and.returnValue(Promise.resolve());

      mockHolochain.callZome.and.returnValues(
        Promise.resolve({ success: true, data: 'hash' }),
        Promise.resolve({ success: false, error: 'Failed' }),
        Promise.resolve({ success: true, data: 'hash' })
      );

      const result = await service.commitBatch('batch-1');

      expect(result.successCount).toBe(2);
      expect(result.failureCount).toBe(1);
      expect(result.totalAttempted).toBe(3);
    });

    it('should update batch status to completed when all succeed', async () => {
      const mockBatch: any = {
        id: 'batch-1',
        status: 'pending',
        batchNumber: 'IB-001',
        stewardId: 'steward-123',
        connectionId: 'conn-123',
        accountIds: [],
        dateRange: { start: '2024-01-01', end: '2024-01-31' },
        totalTransactions: 1,
        newTransactions: 1,
        duplicateTransactions: 0,
        errorTransactions: 0,
        stagedTransactionIds: [],
        aiCategorizationEnabled: false,
        createdAt: new Date().toISOString()
      };
      const stagedTxs = [
        createMockStagedTransaction('staged-1', 'approved', 100),
      ];

      spyOn(bankingStore, 'getBatch').and.returnValue(Promise.resolve(mockBatch));
      spyOn(bankingStore, 'getStagedByBatch').and.returnValue(Promise.resolve(stagedTxs));
      spyOn(bankingStore, 'getStaged').and.returnValue(Promise.resolve(stagedTxs[0]));
      spyOn(bankingStore, 'saveStaged').and.returnValue(Promise.resolve());
      const saveBatchSpy = spyOn(bankingStore, 'saveBatch').and.returnValue(Promise.resolve());

      mockHolochain.callZome.and.returnValue(Promise.resolve({
        success: true,
        data: 'hash'
      }));

      await service.commitBatch('batch-1');

      expect(saveBatchSpy).toHaveBeenCalled();
      const savedBatch = saveBatchSpy.calls.mostRecent().args[0];
      expect(savedBatch.status).toBe('completed');
      expect(savedBatch.completedAt).toBeDefined();
    });
  });

  // =========================================================================
  // Retrieving Committed Events
  // =========================================================================

  describe('getCommittedEvent', () => {
    it('should retrieve committed event from Holochain', async () => {
      const mockStaged = createMockStagedTransaction('staged-1', 'approved', 100, 'event-123');
      spyOn(bankingStore, 'getStaged').and.returnValue(Promise.resolve(mockStaged));

      mockHolochain.callZome.and.returnValue(Promise.resolve({
        success: true,
        data: { id: 'event-123', action: 'consume' }
      }));

      const result = await service.getCommittedEvent('staged-1');

      expect(result).toBeDefined();
      expect(result?.eventId).toBe('event-123');
      expect(result?.event).toBeDefined();
    });

    it('should return null for uncommitted transaction', async () => {
      const mockStaged = createMockStagedTransaction('staged-1', 'approved', 100);
      spyOn(bankingStore, 'getStaged').and.returnValue(Promise.resolve(mockStaged));

      const result = await service.getCommittedEvent('staged-1');

      expect(result).toBeNull();
    });

    it('should return null if Holochain lookup fails', async () => {
      const mockStaged = createMockStagedTransaction('staged-1', 'approved', 100, 'event-123');
      spyOn(bankingStore, 'getStaged').and.returnValue(Promise.resolve(mockStaged));

      mockHolochain.callZome.and.returnValue(Promise.resolve({
        success: false,
        error: 'Not found'
      }));

      const result = await service.getCommittedEvent('staged-1');

      expect(result).toBeNull();
    });

    it('should return null if staged transaction not found', async () => {
      spyOn(bankingStore, 'getStaged').and.returnValue(Promise.resolve(undefined));

      const result = await service.getCommittedEvent('non-existent');

      expect(result).toBeNull();
    });
  });

  // =========================================================================
  // Duplicate Detection
  // =========================================================================

  describe('isAlreadyCommitted', () => {
    it('should return true if already committed locally', async () => {
      const mockStaged = createMockStagedTransaction('staged-1', 'approved', 100, 'event-123');
      spyOn(bankingStore, 'checkDuplicate').and.returnValue(Promise.resolve(mockStaged));

      const result = await service.isAlreadyCommitted('plaid-tx-123');

      expect(result).toBe(true);
    });

    it('should return false if not committed locally', async () => {
      spyOn(bankingStore, 'checkDuplicate').and.returnValue(Promise.resolve(undefined));

      mockHolochain.callZome.and.returnValue(Promise.resolve({
        success: false,
        data: null
      }));

      const result = await service.isAlreadyCommitted('plaid-tx-123');

      expect(result).toBe(false);
    });

    it('should check Holochain if not found locally', async () => {
      spyOn(bankingStore, 'checkDuplicate').and.returnValue(Promise.resolve(undefined));

      mockHolochain.callZome.and.returnValue(Promise.resolve({
        success: true,
        data: { id: 'event-123' }
      }));

      const result = await service.isAlreadyCommitted('plaid-tx-123');

      expect(result).toBe(true);
      expect(mockHolochain.callZome).toHaveBeenCalled();
    });

    it('should query with correct event ID format', async () => {
      spyOn(bankingStore, 'checkDuplicate').and.returnValue(Promise.resolve(undefined));

      mockHolochain.callZome.and.returnValue(Promise.resolve({
        success: true,
        data: null
      }));

      await service.isAlreadyCommitted('plaid-tx-abc123');

      const zomeCall = mockHolochain.callZome.calls.mostRecent().args[0];
      expect((zomeCall.payload as any).id).toBe('ee-plaid-tx-abc123');
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
  economicEventId?: string,
  type: 'debit' | 'credit' | 'fee' | 'transfer' = 'debit'
): StagedTransactionLocal {
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
    stewardId: 'steward-123',
    batchId: 'batch-123',
    plaidTransactionId: `plaid-tx-${id}`,
    plaidAccountId: 'plaid-acct-123',
    financialAssetId: 'asset-123',
    merchantName: 'Test Merchant',
    plaidRawData: {},
    isDuplicate: false,
    createdAt: new Date().toISOString()
  };
}
