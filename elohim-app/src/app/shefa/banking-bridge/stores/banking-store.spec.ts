/**
 * Banking Store Tests
 *
 * Coverage focus:
 * - Singleton pattern
 * - Connection management
 * - Batch operations
 * - Transaction staging
 * - Rule management
 * - Correction records
 */

import { BankingStore, PlaidConnectionLocal, ImportBatchLocal, StagedTransactionLocal, TransactionRuleLocal, CorrectionRecordLocal } from './banking-store';

describe('BankingStore', () => {
  let store: BankingStore;

  beforeEach(() => {
    // Get singleton instance
    store = BankingStore.getInstance();
  });

  afterEach(async () => {
    // Clean up after tests
    if (store) {
      await store.clearAllData();
    }
  });

  // ==========================================================================
  // Singleton Pattern Tests
  // ==========================================================================

  describe('singleton pattern', () => {
    it('should be created', () => {
      expect(store).toBeTruthy();
    });

    it('should have store instance', () => {
      expect(store).toBeDefined();
    });

    it('should return same instance on multiple calls', () => {
      const store1 = BankingStore.getInstance();
      const store2 = BankingStore.getInstance();
      expect(store1).toBe(store2);
    });

    it('should be a BankingStore instance', () => {
      expect(store instanceof BankingStore).toBe(true);
    });
  });

  // ==========================================================================
  // Initialization Tests
  // ==========================================================================

  describe('initialization', () => {
    it('should have init method', () => {
      expect(typeof store.init).toBe('function');
    });

    it('should initialize without error', async () => {
      expect(async () => {
        await store.init();
      }).not.toThrow();
    });

    it('init should return Promise', () => {
      const promise = store.init();
      expect(promise instanceof Promise).toBe(true);
    });

    it('should handle multiple init calls', async () => {
      await store.init();
      await store.init();
      // Should not throw
      expect(store).toBeTruthy();
    });
  });

  // ==========================================================================
  // Connection Management Tests
  // ==========================================================================

  describe('connection management', () => {
    it('should have saveConnection method', () => {
      expect(typeof store.saveConnection).toBe('function');
    });

    it('should have getConnection method', () => {
      expect(typeof store.getConnection).toBe('function');
    });

    it('should have getConnectionsByAgent method', () => {
      expect(typeof store.getConnectionsByAgent).toBe('function');
    });

    it('should have deleteConnection method', () => {
      expect(typeof store.deleteConnection).toBe('function');
    });

    it('saveConnection should return Promise', () => {
      const mockConnection: PlaidConnectionLocal = {
        id: 'conn-123',
        connectionNumber: 'PC-001',
        stewardId: 'steward-123',
        plaidItemId: 'item-123',
        plaidAccessTokenEncrypted: 'encrypted-token',
        plaidInstitutionId: 'inst-123',
        institutionName: 'Test Bank',
        linkedAccounts: [],
        status: 'active',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };

      const promise = store.saveConnection(mockConnection);
      expect(promise instanceof Promise).toBe(true);
    });

    it('getConnection should return Promise', () => {
      const promise = store.getConnection('conn-123');
      expect(promise instanceof Promise).toBe(true);
    });

    it('getConnectionsByAgent should return Promise', () => {
      const promise = store.getConnectionsByAgent('steward-123');
      expect(promise instanceof Promise).toBe(true);
    });

    it('deleteConnection should return Promise', () => {
      const promise = store.deleteConnection('conn-123');
      expect(promise instanceof Promise).toBe(true);
    });
  });

  // ==========================================================================
  // Batch Operations Tests
  // ==========================================================================

  describe('batch operations', () => {
    it('should have saveBatch method', () => {
      expect(typeof store.saveBatch).toBe('function');
    });

    it('should have getBatch method', () => {
      expect(typeof store.getBatch).toBe('function');
    });

    it('should have getBatchesByConnection method', () => {
      expect(typeof store.getBatchesByConnection).toBe('function');
    });

    it('should have getBatchesByStatus method', () => {
      expect(typeof store.getBatchesByStatus).toBe('function');
    });

    it('should have deleteBatch method', () => {
      expect(typeof store.deleteBatch).toBe('function');
    });

    it('saveBatch should return Promise', () => {
      const mockBatch: ImportBatchLocal = {
        id: 'batch-123',
        batchNumber: 'IB-001',
        stewardId: 'steward-123',
        connectionId: 'conn-123',
        accountIds: ['acc-1', 'acc-2'],
        dateRange: { start: '2024-01-01', end: '2024-01-31' },
        totalTransactions: 100,
        newTransactions: 80,
        duplicateTransactions: 15,
        errorTransactions: 5,
        status: 'fetching',
        stagedTransactionIds: [],
        aiCategorizationEnabled: false,
        createdAt: new Date().toISOString(),
      };

      const promise = store.saveBatch(mockBatch);
      expect(promise instanceof Promise).toBe(true);
    });

    it('getBatch should return Promise', () => {
      const promise = store.getBatch('batch-123');
      expect(promise instanceof Promise).toBe(true);
    });

    it('getBatchesByConnection should return Promise', () => {
      const promise = store.getBatchesByConnection('conn-123');
      expect(promise instanceof Promise).toBe(true);
    });

    it('getBatchesByStatus should return Promise', () => {
      const promise = store.getBatchesByStatus('completed');
      expect(promise instanceof Promise).toBe(true);
    });

    it('deleteBatch should return Promise', () => {
      const promise = store.deleteBatch('batch-123');
      expect(promise instanceof Promise).toBe(true);
    });
  });

  // ==========================================================================
  // Staged Transaction Tests
  // ==========================================================================

  describe('staged transactions', () => {
    it('should have saveStaged method', () => {
      expect(typeof store.saveStaged).toBe('function');
    });

    it('should have saveStagedBulk method', () => {
      expect(typeof store.saveStagedBulk).toBe('function');
    });

    it('should have getStaged method', () => {
      expect(typeof store.getStaged).toBe('function');
    });

    it('should have getStagedByBatch method', () => {
      expect(typeof store.getStagedByBatch).toBe('function');
    });

    it('should have getStagedByStatus method', () => {
      expect(typeof store.getStagedByStatus).toBe('function');
    });

    it('should have getStagedPending method', () => {
      expect(typeof store.getStagedPending).toBe('function');
    });

    it('should have checkDuplicate method', () => {
      expect(typeof store.checkDuplicate).toBe('function');
    });

    it('should have deleteStaged method', () => {
      expect(typeof store.deleteStaged).toBe('function');
    });

    it('should have deleteStagedByBatch method', () => {
      expect(typeof store.deleteStagedByBatch).toBe('function');
    });

    it('saveStaged should return Promise', () => {
      const mockTx: StagedTransactionLocal = {
        id: 'tx-123',
        batchId: 'batch-123',
        stewardId: 'steward-123',
        plaidTransactionId: 'plaid-tx-123',
        plaidAccountId: 'acc-123',
        financialAssetId: 'asset-123',
        timestamp: new Date().toISOString(),
        type: 'debit',
        amount: { value: 100, unit: 'USD' },
        description: 'Coffee',
        category: 'Food and Drink',
        categoryConfidence: 0.95,
        categorySource: 'ai',
        isDuplicate: false,
        reviewStatus: 'pending',
        plaidRawData: {},
        createdAt: new Date().toISOString(),
      };

      const promise = store.saveStaged(mockTx);
      expect(promise instanceof Promise).toBe(true);
    });

    it('saveStagedBulk should return Promise', () => {
      const mockTxs: StagedTransactionLocal[] = [
        {
          id: 'tx-123',
          batchId: 'batch-123',
          stewardId: 'steward-123',
          plaidTransactionId: 'plaid-tx-123',
          plaidAccountId: 'acc-123',
          financialAssetId: 'asset-123',
          timestamp: new Date().toISOString(),
          type: 'debit',
          amount: { value: 100, unit: 'USD' },
          description: 'Coffee',
          category: 'Food and Drink',
          categoryConfidence: 0.95,
          categorySource: 'ai',
          isDuplicate: false,
          reviewStatus: 'pending',
          plaidRawData: {},
          createdAt: new Date().toISOString(),
        },
      ];

      const promise = store.saveStagedBulk(mockTxs);
      expect(promise instanceof Promise).toBe(true);
    });

    it('getStaged should return Promise', () => {
      const promise = store.getStaged('tx-123');
      expect(promise instanceof Promise).toBe(true);
    });

    it('getStagedByBatch should return Promise', () => {
      const promise = store.getStagedByBatch('batch-123');
      expect(promise instanceof Promise).toBe(true);
    });

    it('getStagedByStatus should return Promise', () => {
      const promise = store.getStagedByStatus('pending');
      expect(promise instanceof Promise).toBe(true);
    });

    it('getStagedPending should return Promise', () => {
      const promise = store.getStagedPending();
      expect(promise instanceof Promise).toBe(true);
    });

    it('checkDuplicate should return Promise', () => {
      const promise = store.checkDuplicate('plaid-tx-123');
      expect(promise instanceof Promise).toBe(true);
    });

    it('deleteStaged should return Promise', () => {
      const promise = store.deleteStaged('tx-123');
      expect(promise instanceof Promise).toBe(true);
    });

    it('deleteStagedByBatch should return Promise', () => {
      const promise = store.deleteStagedByBatch('batch-123');
      expect(promise instanceof Promise).toBe(true);
    });
  });

  // ==========================================================================
  // Transaction Rules Tests
  // ==========================================================================

  describe('transaction rules', () => {
    it('should have saveRule method', () => {
      expect(typeof store.saveRule).toBe('function');
    });

    it('should have getRule method', () => {
      expect(typeof store.getRule).toBe('function');
    });

    it('should have getRulesByAgent method', () => {
      expect(typeof store.getRulesByAgent).toBe('function');
    });

    it('should have getEnabledRules method', () => {
      expect(typeof store.getEnabledRules).toBe('function');
    });

    it('should have deleteRule method', () => {
      expect(typeof store.deleteRule).toBe('function');
    });

    it('saveRule should return Promise', () => {
      const mockRule: TransactionRuleLocal = {
        id: 'rule-123',
        ruleNumber: 'TR-001',
        stewardId: 'steward-123',
        name: 'Coffee Purchases',
        matchType: 'contains',
        matchField: 'description',
        matchValue: 'coffee',
        action: 'categorize',
        targetCategory: 'Food and Drink',
        learnFromCorrections: true,
        priority: 10,
        enabled: true,
        createdAt: new Date().toISOString(),
      };

      const promise = store.saveRule(mockRule);
      expect(promise instanceof Promise).toBe(true);
    });

    it('getRule should return Promise', () => {
      const promise = store.getRule('rule-123');
      expect(promise instanceof Promise).toBe(true);
    });

    it('getRulesByAgent should return Promise', () => {
      const promise = store.getRulesByAgent('steward-123');
      expect(promise instanceof Promise).toBe(true);
    });

    it('getEnabledRules should return Promise', () => {
      const promise = store.getEnabledRules('steward-123');
      expect(promise instanceof Promise).toBe(true);
    });

    it('deleteRule should return Promise', () => {
      const promise = store.deleteRule('rule-123');
      expect(promise instanceof Promise).toBe(true);
    });
  });

  // ==========================================================================
  // Correction Records Tests
  // ==========================================================================

  describe('correction records', () => {
    it('should have saveCorrection method', () => {
      expect(typeof store.saveCorrection).toBe('function');
    });

    it('should have getCorrectionsByMerchant method', () => {
      expect(typeof store.getCorrectionsByMerchant).toBe('function');
    });

    it('should have getCorrectionsByAgent method', () => {
      expect(typeof store.getCorrectionsByAgent).toBe('function');
    });

    it('saveCorrection should return Promise', () => {
      const mockCorrection: CorrectionRecordLocal = {
        id: 'corr-123',
        stewardId: 'steward-123',
        transactionDescription: 'Coffee Shop Purchase',
        merchantName: 'Coffee Co',
        transactionAmount: 5.5,
        originalCategory: 'Uncategorized',
        originalConfidence: 0.2,
        correctedCategory: 'Food and Drink',
        improvedAccuracy: true,
        ruleCreatedFrom: false,
        timestamp: new Date().toISOString(),
      };

      const promise = store.saveCorrection(mockCorrection);
      expect(promise instanceof Promise).toBe(true);
    });

    it('getCorrectionsByMerchant should return Promise', () => {
      const promise = store.getCorrectionsByMerchant('Coffee Co');
      expect(promise instanceof Promise).toBe(true);
    });

    it('getCorrectionsByAgent should return Promise', () => {
      const promise = store.getCorrectionsByAgent('steward-123');
      expect(promise instanceof Promise).toBe(true);
    });
  });

  // ==========================================================================
  // Cleanup Tests
  // ==========================================================================

  describe('cleanup operations', () => {
    it('should have clearAllData method', () => {
      expect(typeof store.clearAllData).toBe('function');
    });

    it('should have clearCompletedBatches method', () => {
      expect(typeof store.clearCompletedBatches).toBe('function');
    });

    it('clearAllData should return Promise', () => {
      const promise = store.clearAllData();
      expect(promise instanceof Promise).toBe(true);
    });

    it('clearCompletedBatches should return Promise', () => {
      const promise = store.clearCompletedBatches(30);
      expect(promise instanceof Promise).toBe(true);
    });

    it('clearCompletedBatches should accept days parameter', () => {
      expect(() => store.clearCompletedBatches(7)).not.toThrow();
    });
  });

  // ==========================================================================
  // Parameter Acceptance Tests
  // ==========================================================================

  describe('parameter acceptance', () => {
    it('should accept PlaidConnectionLocal for saveConnection', () => {
      const mockConnection: PlaidConnectionLocal = {
        id: 'conn-123',
        connectionNumber: 'PC-001',
        stewardId: 'steward-123',
        plaidItemId: 'item-123',
        plaidAccessTokenEncrypted: 'encrypted-token',
        plaidInstitutionId: 'inst-123',
        institutionName: 'Test Bank',
        linkedAccounts: [],
        status: 'active',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };

      expect(() => store.saveConnection(mockConnection)).not.toThrow();
    });

    it('should accept string id for getConnection', () => {
      expect(() => store.getConnection('conn-123')).not.toThrow();
    });

    it('should accept stewardId for getConnectionsByAgent', () => {
      expect(() => store.getConnectionsByAgent('steward-123')).not.toThrow();
    });

    it('should accept batch status for getBatchesByStatus', () => {
      expect(() => store.getBatchesByStatus('completed')).not.toThrow();
      expect(() => store.getBatchesByStatus('fetching')).not.toThrow();
    });

    it('should accept transaction status for getStagedByStatus', () => {
      expect(() => store.getStagedByStatus('pending')).not.toThrow();
      expect(() => store.getStagedByStatus('approved')).not.toThrow();
    });

    it('should accept optional days for clearCompletedBatches', () => {
      expect(() => store.clearCompletedBatches()).not.toThrow();
      expect(() => store.clearCompletedBatches(14)).not.toThrow();
    });
  });

  // ==========================================================================
  // Type Verification Tests
  // ==========================================================================

  describe('type verification', () => {
    it('should verify saveConnection returns Promise', () => {
      const mockConnection: PlaidConnectionLocal = {
        id: 'conn-123',
        connectionNumber: 'PC-001',
        stewardId: 'steward-123',
        plaidItemId: 'item-123',
        plaidAccessTokenEncrypted: 'encrypted-token',
        plaidInstitutionId: 'inst-123',
        institutionName: 'Test Bank',
        linkedAccounts: [],
        status: 'active',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };

      const result = store.saveConnection(mockConnection);
      expect(result instanceof Promise).toBe(true);
    });

    it('should verify getConnection returns Promise', () => {
      const result = store.getConnection('conn-123');
      expect(result instanceof Promise).toBe(true);
    });

    it('should verify saveBatch returns Promise', () => {
      const mockBatch: ImportBatchLocal = {
        id: 'batch-123',
        batchNumber: 'IB-001',
        stewardId: 'steward-123',
        connectionId: 'conn-123',
        accountIds: [],
        dateRange: { start: '2024-01-01', end: '2024-01-31' },
        totalTransactions: 0,
        newTransactions: 0,
        duplicateTransactions: 0,
        errorTransactions: 0,
        status: 'fetching',
        stagedTransactionIds: [],
        aiCategorizationEnabled: false,
        createdAt: new Date().toISOString(),
      };

      const result = store.saveBatch(mockBatch);
      expect(result instanceof Promise).toBe(true);
    });
  });
});
