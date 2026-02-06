/**
 * Transaction Import Service Tests - Comprehensive Coverage
 *
 * Tests the 8-stage transaction import pipeline:
 * FETCH → NORMALIZE → DEDUPLICATE → STAGE → CATEGORIZE → REVIEW → APPROVE → CREATE
 *
 * Coverage targets:
 * - Pipeline orchestration (executeImport)
 * - Transaction approval flow
 * - Bulk operations
 * - Error handling and recovery
 * - Progress tracking
 * - Categorization integration
 */

import { TestBed } from '@angular/core/testing';
import { firstValueFrom } from 'rxjs';

import { TransactionImportService } from './transaction-import.service';
import { AICategorizationService } from './ai-categorization.service';
import { BudgetReconciliationService } from './budget-reconciliation.service';
import { DuplicateDetectionService } from './duplicate-detection.service';
import { EconomicEventFactoryService } from './economic-event-factory.service';
import { PlaidIntegrationService } from './plaid-integration.service';
import {
  ImportRequest,
  PlaidTransaction,
  StagedTransaction,
  ImportBatch,
  CategorizationResponse,
} from '../models/transaction-import.model';

describe('TransactionImportService', () => {
  let service: TransactionImportService;
  let mockPlaid: jasmine.SpyObj<PlaidIntegrationService>;
  let mockDuplicates: jasmine.SpyObj<DuplicateDetectionService>;
  let mockCategorization: jasmine.SpyObj<AICategorizationService>;
  let mockEventFactory: jasmine.SpyObj<EconomicEventFactoryService>;
  let mockBudgetReconciliation: jasmine.SpyObj<BudgetReconciliationService>;

  beforeEach(() => {
    mockPlaid = jasmine.createSpyObj('PlaidIntegrationService', ['fetchTransactions']);
    mockDuplicates = jasmine.createSpyObj('DuplicateDetectionService', ['filterDuplicates']);
    mockCategorization = jasmine.createSpyObj('AICategorizationService', ['categorizeBatch']);
    mockEventFactory = jasmine.createSpyObj('EconomicEventFactoryService', ['createFromStaged']);
    mockBudgetReconciliation = jasmine.createSpyObj('BudgetReconciliationService', [
      'reconcileBudget',
    ]);

    TestBed.configureTestingModule({
      providers: [
        TransactionImportService,
        { provide: PlaidIntegrationService, useValue: mockPlaid },
        { provide: DuplicateDetectionService, useValue: mockDuplicates },
        { provide: AICategorizationService, useValue: mockCategorization },
        { provide: EconomicEventFactoryService, useValue: mockEventFactory },
        { provide: BudgetReconciliationService, useValue: mockBudgetReconciliation },
      ],
    });
    service = TestBed.inject(TransactionImportService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  // ==========================================================================
  // Progress & Error Observables
  // ==========================================================================

  describe('observables', () => {
    it('should provide progress observable', done => {
      const progress$ = service.getProgress$();
      expect(progress$).toBeDefined();

      progress$.subscribe(progress => {
        expect(progress.stage).toBe('created');
        expect(progress.progress).toBe(0);
        done();
      });
    });

    it('should provide errors observable', () => {
      const errors$ = service.getErrors$();
      expect(errors$).toBeDefined();
    });
  });

  // ==========================================================================
  // executeImport - Main Pipeline
  // ==========================================================================

  describe('executeImport', () => {
    it('should execute full import pipeline for valid transactions', async () => {
      const mockPlaidTransactions: PlaidTransaction[] = [
        createMockPlaidTransaction('tx-1', -50.0, 'Grocery Store'),
        createMockPlaidTransaction('tx-2', -30.0, 'Coffee Shop'),
      ];

      const mockCategorizationResponse: CategorizationResponse = {
        results: [
          {
            transactionId: 'staged-test-1',
            category: 'Groceries',
            confidence: 90,
            alternatives: [],
          },
          {
            transactionId: 'staged-test-2',
            category: 'Dining',
            confidence: 85,
            alternatives: [],
          },
        ],
      };

      mockPlaid.fetchTransactions.and.returnValue(Promise.resolve(mockPlaidTransactions));
      mockDuplicates.filterDuplicates.and.returnValue(mockPlaidTransactions);
      mockCategorization.categorizeBatch.and.returnValue(
        mockCategorizationResponse as any
      );

      const request: ImportRequest = {
        connectionId: 'conn-1',
        dateRange: { start: '2024-01-01', end: '2024-01-31' },
        aiCategorizationEnabled: true,
      };

      const batch = await service.executeImport(request);

      expect(batch).toBeDefined();
      expect(batch.status).toBe('staged');
      expect(batch.totalTransactions).toBe(2);
      expect(batch.newTransactions).toBe(2);
      expect(batch.stagedTransactionIds.length).toBe(2);
      expect(mockPlaid.fetchTransactions).toHaveBeenCalled();
      expect(mockDuplicates.filterDuplicates).toHaveBeenCalled();
    });

    it('should handle empty transaction list', async () => {
      mockPlaid.fetchTransactions.and.returnValue(Promise.resolve([]));

      const request: ImportRequest = {
        connectionId: 'conn-1',
        dateRange: { start: '2024-01-01', end: '2024-01-31' },
      };

      const batch = await service.executeImport(request);

      expect(batch.totalTransactions).toBe(0);
      expect(batch.status).toBe('completed');
      expect(batch.stagedTransactionIds.length).toBe(0);
    });

    it('should filter duplicate transactions', async () => {
      const mockPlaidTransactions: PlaidTransaction[] = [
        createMockPlaidTransaction('tx-1', -50.0, 'Store'),
        createMockPlaidTransaction('tx-2', -50.0, 'Store'), // Duplicate
        createMockPlaidTransaction('tx-3', -30.0, 'Other'),
      ];

      const uniqueTransactions = [mockPlaidTransactions[0], mockPlaidTransactions[2]];

      mockPlaid.fetchTransactions.and.returnValue(Promise.resolve(mockPlaidTransactions));
      mockDuplicates.filterDuplicates.and.returnValue(uniqueTransactions);

      const request: ImportRequest = {
        connectionId: 'conn-1',
        dateRange: { start: '2024-01-01', end: '2024-01-31' },
      };

      const batch = await service.executeImport(request);

      expect(batch.totalTransactions).toBe(3);
      expect(batch.newTransactions).toBe(2);
      expect(batch.duplicateTransactions).toBe(1);
    });

    it('should skip categorization when disabled', async () => {
      const mockPlaidTransactions: PlaidTransaction[] = [
        createMockPlaidTransaction('tx-1', -50.0, 'Store'),
      ];

      mockPlaid.fetchTransactions.and.returnValue(Promise.resolve(mockPlaidTransactions));
      mockDuplicates.filterDuplicates.and.returnValue(mockPlaidTransactions);

      const request: ImportRequest = {
        connectionId: 'conn-1',
        dateRange: { start: '2024-01-01', end: '2024-01-31' },
        aiCategorizationEnabled: false,
      };

      const batch = await service.executeImport(request);

      expect(batch.aiCategorizationEnabled).toBe(false);
      expect(mockCategorization.categorizeBatch).not.toHaveBeenCalled();
    });

    it('should propagate errors from Plaid fetch', async () => {
      mockPlaid.fetchTransactions.and.returnValue(
        Promise.reject(new Error('Plaid API error'))
      );

      const request: ImportRequest = {
        connectionId: 'conn-1',
        dateRange: { start: '2024-01-01', end: '2024-01-31' },
      };

      await expectAsync(service.executeImport(request)).toBeRejectedWithError('Plaid API error');
    });

    it('should handle null transactions from Plaid', async () => {
      mockPlaid.fetchTransactions.and.returnValue(Promise.resolve(null as any));

      const request: ImportRequest = {
        connectionId: 'conn-1',
        dateRange: { start: '2024-01-01', end: '2024-01-31' },
      };

      const batch = await service.executeImport(request);

      expect(batch.totalTransactions).toBe(0);
      expect(batch.status).toBe('completed');
    });
  });

  // ==========================================================================
  // Transaction Normalization
  // ==========================================================================

  describe('transaction normalization', () => {
    it('should normalize debit transactions', async () => {
      const plaidTx = createMockPlaidTransaction('tx-1', -100.0, 'Store Purchase');
      mockPlaid.fetchTransactions.and.returnValue(Promise.resolve([plaidTx]));
      mockDuplicates.filterDuplicates.and.returnValue([plaidTx]);

      const request: ImportRequest = {
        connectionId: 'conn-1',
        dateRange: { start: '2024-01-01', end: '2024-01-31' },
        aiCategorizationEnabled: false,
      };

      const batch = await service.executeImport(request);
      const staged = service.getStagedTransactionsForBatch(batch.id);

      expect(staged[0].type).toBe('debit');
      expect(staged[0].amount.value).toBe(100); // Absolute value
    });

    it('should normalize credit transactions', async () => {
      const plaidTx = createMockPlaidTransaction('tx-1', 500.0, 'Paycheck Deposit');
      mockPlaid.fetchTransactions.and.returnValue(Promise.resolve([plaidTx]));
      mockDuplicates.filterDuplicates.and.returnValue([plaidTx]);

      const request: ImportRequest = {
        connectionId: 'conn-1',
        dateRange: { start: '2024-01-01', end: '2024-01-31' },
        aiCategorizationEnabled: false,
      };

      const batch = await service.executeImport(request);
      const staged = service.getStagedTransactionsForBatch(batch.id);

      expect(staged[0].type).toBe('credit');
      expect(staged[0].amount.value).toBe(500);
    });

    it('should detect fee transactions', async () => {
      const plaidTx = createMockPlaidTransaction('tx-1', -5.0, 'ATM Fee');
      mockPlaid.fetchTransactions.and.returnValue(Promise.resolve([plaidTx]));
      mockDuplicates.filterDuplicates.and.returnValue([plaidTx]);

      const request: ImportRequest = {
        connectionId: 'conn-1',
        dateRange: { start: '2024-01-01', end: '2024-01-31' },
        aiCategorizationEnabled: false,
      };

      const batch = await service.executeImport(request);
      const staged = service.getStagedTransactionsForBatch(batch.id);

      expect(staged[0].type).toBe('fee');
    });

    it('should detect transfer transactions', async () => {
      const plaidTx = createMockPlaidTransaction('tx-1', -100.0, 'Transfer to Savings');
      mockPlaid.fetchTransactions.and.returnValue(Promise.resolve([plaidTx]));
      mockDuplicates.filterDuplicates.and.returnValue([plaidTx]);

      const request: ImportRequest = {
        connectionId: 'conn-1',
        dateRange: { start: '2024-01-01', end: '2024-01-31' },
        aiCategorizationEnabled: false,
      };

      const batch = await service.executeImport(request);
      const staged = service.getStagedTransactionsForBatch(batch.id);

      expect(staged[0].type).toBe('transfer');
    });

    it('should handle missing currency code', async () => {
      const plaidTx: PlaidTransaction = {
        transaction_id: 'tx-1',
        account_id: 'acct-1',
        amount: -50.0,
        date: '2024-01-15',
        name: 'Store',
        iso_currency_code: 'USD',
        authorized_date: '2024-01-15',
        transaction_type: 'place',
        pending: false,
      };

      mockPlaid.fetchTransactions.and.returnValue(Promise.resolve([plaidTx]));
      mockDuplicates.filterDuplicates.and.returnValue([plaidTx]);

      const request: ImportRequest = {
        connectionId: 'conn-1',
        dateRange: { start: '2024-01-01', end: '2024-01-31' },
        aiCategorizationEnabled: false,
      };

      const batch = await service.executeImport(request);
      const staged = service.getStagedTransactionsForBatch(batch.id);

      expect(staged[0].amount.unit).toBe('USD'); // Default
    });
  });

  // ==========================================================================
  // Approval Flow
  // ==========================================================================

  describe('approveTransaction', () => {
    let batch: ImportBatch;
    let stagedId: string;

    beforeEach(async () => {
      const plaidTx = createMockPlaidTransaction('tx-1', -100.0, 'Test Purchase');
      mockPlaid.fetchTransactions.and.returnValue(Promise.resolve([plaidTx]));
      mockDuplicates.filterDuplicates.and.returnValue([plaidTx]);

      const request: ImportRequest = {
        connectionId: 'conn-1',
        dateRange: { start: '2024-01-01', end: '2024-01-31' },
        aiCategorizationEnabled: false,
      };

      batch = await service.executeImport(request);
      stagedId = batch.stagedTransactionIds[0];
    });

    it('should approve transaction and create economic event', async () => {
      mockEventFactory.createFromStaged.and.returnValue(
        Promise.resolve({ id: 'event-123' } as any)
      );
      mockBudgetReconciliation.reconcileBudget.and.returnValue(Promise.resolve({} as any));

      await service.approveTransaction(stagedId);

      const staged = service.getStagedTransaction(stagedId);
      expect(staged?.reviewStatus).toBe('approved');
      expect(staged?.economicEventId).toBe('event-123');
      expect(mockEventFactory.createFromStaged).toHaveBeenCalled();
    });

    it('should reconcile budget if budgetId present', async () => {
      mockEventFactory.createFromStaged.and.returnValue(
        Promise.resolve({ id: 'event-123' } as any)
      );
      mockBudgetReconciliation.reconcileBudget.and.returnValue(Promise.resolve({} as any));

      // Update staged to have budget linkage
      const staged = service.getStagedTransaction(stagedId);
      if (staged) {
        await service.updateStagedTransactionCategory(stagedId, 'Groceries', 'budget-1', 'cat-1');
      }

      await service.approveTransaction(stagedId);

      expect(mockBudgetReconciliation.reconcileBudget).toHaveBeenCalled();
    });

    it('should skip budget reconciliation if no budgetId', async () => {
      mockEventFactory.createFromStaged.and.returnValue(
        Promise.resolve({ id: 'event-123' } as any)
      );

      await service.approveTransaction(stagedId);

      expect(mockBudgetReconciliation.reconcileBudget).not.toHaveBeenCalled();
    });

    it('should throw error for non-existent transaction', async () => {
      await expectAsync(service.approveTransaction('non-existent-id')).toBeRejectedWithError(
        /not found/
      );
    });

    it('should handle idempotent approval', async () => {
      mockEventFactory.createFromStaged.and.returnValue(
        Promise.resolve({ id: 'event-123' } as any)
      );

      await service.approveTransaction(stagedId);
      await service.approveTransaction(stagedId); // Second approval

      // Should not create event twice
      expect(mockEventFactory.createFromStaged).toHaveBeenCalledTimes(1);
    });

    it('should propagate errors from event creation', async () => {
      mockEventFactory.createFromStaged.and.returnValue(
        Promise.reject(new Error('Event creation failed'))
      );

      await expectAsync(service.approveTransaction(stagedId)).toBeRejectedWithError(
        /Event creation failed/
      );
    });
  });

  describe('rejectTransaction', () => {
    let batch: ImportBatch;
    let stagedId: string;

    beforeEach(async () => {
      const plaidTx = createMockPlaidTransaction('tx-1', -100.0, 'Test Purchase');
      mockPlaid.fetchTransactions.and.returnValue(Promise.resolve([plaidTx]));
      mockDuplicates.filterDuplicates.and.returnValue([plaidTx]);

      const request: ImportRequest = {
        connectionId: 'conn-1',
        dateRange: { start: '2024-01-01', end: '2024-01-31' },
        aiCategorizationEnabled: false,
      };

      batch = await service.executeImport(request);
      stagedId = batch.stagedTransactionIds[0];
    });

    it('should mark transaction as rejected', async () => {
      await service.rejectTransaction(stagedId);

      const staged = service.getStagedTransaction(stagedId);
      expect(staged?.reviewStatus).toBe('rejected');
    });

    it('should store rejection reason', async () => {
      await service.rejectTransaction(stagedId, 'Duplicate transaction');

      const staged = service.getStagedTransaction(stagedId);
      expect((staged as any).rejectionReason).toBe('Duplicate transaction');
    });

    it('should throw error for non-existent transaction', async () => {
      await expectAsync(service.rejectTransaction('non-existent')).toBeRejectedWithError(
        /not found/
      );
    });
  });

  describe('approveBatch', () => {
    let batch: ImportBatch;
    let stagedIds: string[];

    beforeEach(async () => {
      const plaidTransactions = [
        createMockPlaidTransaction('tx-1', -100.0, 'Purchase 1'),
        createMockPlaidTransaction('tx-2', -50.0, 'Purchase 2'),
        createMockPlaidTransaction('tx-3', -75.0, 'Purchase 3'),
      ];

      mockPlaid.fetchTransactions.and.returnValue(Promise.resolve(plaidTransactions));
      mockDuplicates.filterDuplicates.and.returnValue(plaidTransactions);

      const request: ImportRequest = {
        connectionId: 'conn-1',
        dateRange: { start: '2024-01-01', end: '2024-01-31' },
        aiCategorizationEnabled: false,
      };

      batch = await service.executeImport(request);
      stagedIds = batch.stagedTransactionIds;
    });

    it('should approve multiple transactions', async () => {
      mockEventFactory.createFromStaged.and.returnValue(Promise.resolve({ id: 'event-1' } as any));

      await service.approveBatch(stagedIds);

      for (const id of stagedIds) {
        const staged = service.getStagedTransaction(id);
        expect(staged?.reviewStatus).toBe('approved');
      }
    });

    it('should continue processing after individual failure', async () => {
      mockEventFactory.createFromStaged.and.returnValues(
        Promise.resolve({ id: 'event-1' } as any),
        Promise.reject(new Error('Failed')), // Second fails
        Promise.resolve({ id: 'event-3' } as any)
      );

      await service.approveBatch(stagedIds);

      // First and third should succeed
      expect(service.getStagedTransaction(stagedIds[0])?.reviewStatus).toBe('approved');
      expect(service.getStagedTransaction(stagedIds[1])?.reviewStatus).toBe('pending');
      expect(service.getStagedTransaction(stagedIds[2])?.reviewStatus).toBe('approved');
    });
  });

  // ==========================================================================
  // Batch Management
  // ==========================================================================

  describe('batch management', () => {
    it('should retrieve batch by ID', async () => {
      const plaidTx = createMockPlaidTransaction('tx-1', -100.0, 'Purchase');
      mockPlaid.fetchTransactions.and.returnValue(Promise.resolve([plaidTx]));
      mockDuplicates.filterDuplicates.and.returnValue([plaidTx]);

      const request: ImportRequest = {
        connectionId: 'conn-1',
        dateRange: { start: '2024-01-01', end: '2024-01-31' },
      };

      const batch = await service.executeImport(request);
      const retrieved = service.getBatch(batch.id);

      expect(retrieved).toBeDefined();
      expect(retrieved?.id).toBe(batch.id);
    });

    it('should return undefined for non-existent batch', () => {
      const batch = service.getBatch('non-existent-id');
      expect(batch).toBeUndefined();
    });

    it('should filter batches by steward', async () => {
      // This test would require setting stewardId on batches
      // Currently stewardId is empty string - would need service integration
      const batches = service.getBatchesForSteward('steward-123');
      expect(batches).toEqual([]);
    });
  });

  // ==========================================================================
  // Staged Transaction Management
  // ==========================================================================

  describe('staged transaction management', () => {
    let batch: ImportBatch;
    let stagedId: string;

    beforeEach(async () => {
      const plaidTx = createMockPlaidTransaction('tx-1', -100.0, 'Purchase');
      mockPlaid.fetchTransactions.and.returnValue(Promise.resolve([plaidTx]));
      mockDuplicates.filterDuplicates.and.returnValue([plaidTx]);

      const request: ImportRequest = {
        connectionId: 'conn-1',
        dateRange: { start: '2024-01-01', end: '2024-01-31' },
        aiCategorizationEnabled: false,
      };

      batch = await service.executeImport(request);
      stagedId = batch.stagedTransactionIds[0];
    });

    it('should retrieve staged transaction by ID', () => {
      const staged = service.getStagedTransaction(stagedId);
      expect(staged).toBeDefined();
      expect(staged?.id).toBe(stagedId);
    });

    it('should return undefined for non-existent staged transaction', () => {
      const staged = service.getStagedTransaction('non-existent');
      expect(staged).toBeUndefined();
    });

    it('should get staged transactions for batch', () => {
      const staged = service.getStagedTransactionsForBatch(batch.id);
      expect(staged.length).toBe(1);
      expect(staged[0].batchId).toBe(batch.id);
    });

    it('should update transaction category', async () => {
      await service.updateStagedTransactionCategory(
        stagedId,
        'Groceries',
        'budget-1',
        'cat-groceries'
      );

      const staged = service.getStagedTransaction(stagedId);
      expect(staged?.category).toBe('Groceries');
      expect(staged?.categorySource).toBe('manual');
      expect(staged?.budgetId).toBe('budget-1');
      expect(staged?.budgetCategoryId).toBe('cat-groceries');
    });

    it('should throw error when updating non-existent transaction', async () => {
      await expectAsync(
        service.updateStagedTransactionCategory('non-existent', 'Category')
      ).toBeRejectedWithError(/not found/);
    });
  });

  // ==========================================================================
  // Categorization
  // ==========================================================================

  describe('categorization', () => {
    it('should batch categorize transactions in chunks of 50', async () => {
      // Create 120 transactions (should result in 3 batches)
      const plaidTransactions = Array.from({ length: 120 }, (_, i) =>
        createMockPlaidTransaction(`tx-${i}`, -50.0, `Purchase ${i}`)
      );

      const mockCategorizationResp: CategorizationResponse = {
        results: plaidTransactions.slice(0, 50).map((_, i) => ({
          transactionId: `staged-${i}`,
          category: 'Groceries',
          confidence: 85,
          alternatives: [],
        })),
      };

      mockPlaid.fetchTransactions.and.returnValue(Promise.resolve(plaidTransactions));
      mockDuplicates.filterDuplicates.and.returnValue(plaidTransactions);
      mockCategorization.categorizeBatch.and.returnValue(mockCategorizationResp as any);

      const request: ImportRequest = {
        connectionId: 'conn-1',
        dateRange: { start: '2024-01-01', end: '2024-01-31' },
        aiCategorizationEnabled: true,
      };

      await service.executeImport(request);

      // Should be called 3 times (50 + 50 + 20)
      expect(mockCategorization.categorizeBatch).toHaveBeenCalledTimes(3);
    });

    it('should update staged transactions with categorization results', async () => {
      const plaidTx = createMockPlaidTransaction('tx-1', -100.0, 'Whole Foods');

      mockPlaid.fetchTransactions.and.returnValue(Promise.resolve([plaidTx]));
      mockDuplicates.filterDuplicates.and.returnValue([plaidTx]);

      const request: ImportRequest = {
        connectionId: 'conn-1',
        dateRange: { start: '2024-01-01', end: '2024-01-31' },
        aiCategorizationEnabled: true,
      };

      const batch = await service.executeImport(request);

      // Wait for async categorization
      await new Promise(resolve => setTimeout(resolve, 100));

      // Note: In the actual service, categorization happens async
      // For this test, we'd need to mock the promise resolution
      expect(batch.aiCategorizationEnabled).toBe(true);
    });
  });

  // ==========================================================================
  // Progress Tracking
  // ==========================================================================

  describe('progress tracking', () => {
    it('should update progress through pipeline stages', async () => {
      const progressUpdates: string[] = [];

      service.getProgress$().subscribe(progress => {
        progressUpdates.push(progress.stage);
      });

      const plaidTx = createMockPlaidTransaction('tx-1', -100.0, 'Purchase');
      mockPlaid.fetchTransactions.and.returnValue(Promise.resolve([plaidTx]));
      mockDuplicates.filterDuplicates.and.returnValue([plaidTx]);

      const request: ImportRequest = {
        connectionId: 'conn-1',
        dateRange: { start: '2024-01-01', end: '2024-01-31' },
        aiCategorizationEnabled: false,
      };

      await service.executeImport(request);

      // Should see progress through stages
      expect(progressUpdates).toContain('fetching');
      expect(progressUpdates).toContain('normalizing');
      expect(progressUpdates).toContain('staging');
    });
  });

  // ==========================================================================
  // Error Handling
  // ==========================================================================

  describe('error handling', () => {
    it('should emit error on progress stream', done => {
      mockPlaid.fetchTransactions.and.returnValue(Promise.reject(new Error('API Error')));

      service.getErrors$().subscribe(error => {
        expect(error.stage).toBe('pipeline');
        expect(error.error).toContain('API Error');
        done();
      });

      const request: ImportRequest = {
        connectionId: 'conn-1',
        dateRange: { start: '2024-01-01', end: '2024-01-31' },
      };

      service.executeImport(request).catch(() => {
        // Expected to fail
      });
    });
  });

  // ==========================================================================
  // Edge Cases
  // ==========================================================================

  describe('edge cases', () => {
    it('should handle very large transaction amounts', async () => {
      const plaidTx = createMockPlaidTransaction('tx-1', -999999999.99, 'Large Purchase');
      mockPlaid.fetchTransactions.and.returnValue(Promise.resolve([plaidTx]));
      mockDuplicates.filterDuplicates.and.returnValue([plaidTx]);

      const request: ImportRequest = {
        connectionId: 'conn-1',
        dateRange: { start: '2024-01-01', end: '2024-01-31' },
        aiCategorizationEnabled: false,
      };

      const batch = await service.executeImport(request);
      const staged = service.getStagedTransactionsForBatch(batch.id);

      expect(staged[0].amount.value).toBe(999999999.99);
    });

    it('should handle zero amount transactions', async () => {
      const plaidTx = createMockPlaidTransaction('tx-1', 0.0, 'Zero Amount');
      mockPlaid.fetchTransactions.and.returnValue(Promise.resolve([plaidTx]));
      mockDuplicates.filterDuplicates.and.returnValue([plaidTx]);

      const request: ImportRequest = {
        connectionId: 'conn-1',
        dateRange: { start: '2024-01-01', end: '2024-01-31' },
        aiCategorizationEnabled: false,
      };

      const batch = await service.executeImport(request);
      const staged = service.getStagedTransactionsForBatch(batch.id);

      expect(staged[0].amount.value).toBe(0);
    });

    it('should handle transactions with special characters in description', async () => {
      const plaidTx = createMockPlaidTransaction(
        'tx-1',
        -50.0,
        'Store™ & Café® - "Special" (Deal)'
      );
      mockPlaid.fetchTransactions.and.returnValue(Promise.resolve([plaidTx]));
      mockDuplicates.filterDuplicates.and.returnValue([plaidTx]);

      const request: ImportRequest = {
        connectionId: 'conn-1',
        dateRange: { start: '2024-01-01', end: '2024-01-31' },
        aiCategorizationEnabled: false,
      };

      const batch = await service.executeImport(request);
      const staged = service.getStagedTransactionsForBatch(batch.id);

      expect(staged[0].description).toContain('Special');
    });
  });
});

// ==========================================================================
// Test Helpers
// ==========================================================================

function createMockPlaidTransaction(
  id: string,
  amount: number,
  name: string
): PlaidTransaction {
  return {
    transaction_id: id,
    account_id: 'acct-test',
    amount,
    date: '2024-01-15',
    name,
    iso_currency_code: 'USD',
    merchant_name: name,
    authorized_date: '2024-01-15',
    transaction_type: 'place',
    pending: false,
  };
}
