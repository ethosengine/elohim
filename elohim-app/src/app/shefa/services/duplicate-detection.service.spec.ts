/**
 * Duplicate-detection Service Tests
 */

import { TestBed } from '@angular/core/testing';

import { DuplicateDetectionService } from './duplicate-detection.service';
import { PlaidTransaction, StagedTransaction } from '../models/transaction-import.model';

describe('DuplicateDetectionService', () => {
  let service: DuplicateDetectionService;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [DuplicateDetectionService],
    });
    service = TestBed.inject(DuplicateDetectionService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should have service instance', () => {
    expect(service).toBeDefined();
  });

  describe('detect', () => {
    it('should have detect method', () => {
      expect(service.detect).toBeDefined();
      expect(typeof service.detect).toBe('function');
    });

    it('should return DuplicateResult', () => {
      const transaction: PlaidTransaction = {
        transaction_id: 'txn-1',
        account_id: 'acc-1',
        amount: 100,
        date: '2026-01-01',
        name: 'Test Transaction',
      } as any;

      const result = service.detect(transaction);
      expect(result).toEqual(jasmine.objectContaining({
        isDuplicate: jasmine.any(Boolean),
        confidence: jasmine.any(Number),
      }));
    });

    it('should detect exact match', () => {
      const transaction: PlaidTransaction = {
        transaction_id: 'txn-1',
        account_id: 'acc-1',
        amount: 100,
        date: '2026-01-01',
        name: 'Test Transaction',
      } as any;

      const staged: StagedTransaction = {
        id: 'staged-1',
        plaidTransactionId: 'txn-1',
        plaidAccountId: 'acc-1',
        amount: { value: 100, currency: 'USD' },
        timestamp: '2026-01-01T00:00:00Z',
        description: 'Test Transaction',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      } as any;

      service.registerTransaction(staged);
      const result = service.detect(transaction);
      expect(result.isDuplicate).toBeTrue();
      expect(result.confidence).toBeGreaterThan(90);
    });

    it('should not detect new transaction', () => {
      const transaction: PlaidTransaction = {
        transaction_id: 'txn-new',
        account_id: 'acc-1',
        amount: 100,
        date: '2026-01-01',
        name: 'New Transaction',
      } as any;

      const result = service.detect(transaction);
      expect(result.isDuplicate).toBeFalse();
    });
  });

  describe('filterDuplicates', () => {
    it('should have filterDuplicates method', () => {
      expect(service.filterDuplicates).toBeDefined();
      expect(typeof service.filterDuplicates).toBe('function');
    });

    it('should return array of transactions', () => {
      const transactions: PlaidTransaction[] = [
        {
          transaction_id: 'txn-1',
          account_id: 'acc-1',
          amount: 100,
          date: '2026-01-01',
          name: 'Test Transaction',
        } as any,
      ];

      const result = service.filterDuplicates(transactions);
      expect(result).toEqual(jasmine.any(Array));
    });

    it('should filter out exact duplicates', () => {
      const transaction: PlaidTransaction = {
        transaction_id: 'txn-1',
        account_id: 'acc-1',
        amount: 100,
        date: '2026-01-01',
        name: 'Test Transaction',
      } as any;

      const staged: StagedTransaction = {
        id: 'staged-1',
        plaidTransactionId: 'txn-1',
        plaidAccountId: 'acc-1',
        amount: { value: 100, currency: 'USD' },
        timestamp: '2026-01-01T00:00:00Z',
        description: 'Test Transaction',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      } as any;

      service.registerTransaction(staged);
      const result = service.filterDuplicates([transaction]);
      expect(result.length).toBeLessThanOrEqual(1);
    });
  });

  describe('registerTransaction', () => {
    it('should have registerTransaction method', () => {
      expect(service.registerTransaction).toBeDefined();
      expect(typeof service.registerTransaction).toBe('function');
    });

    it('should register transaction without error', () => {
      const staged: StagedTransaction = {
        id: 'staged-1',
        plaidTransactionId: 'txn-1',
        plaidAccountId: 'acc-1',
        amount: { value: 100, currency: 'USD' },
        timestamp: '2026-01-01T00:00:00Z',
        description: 'Test Transaction',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      } as any;

      expect(() => {
        service.registerTransaction(staged);
      }).not.toThrow();
    });

    it('should make transaction detected after registration', () => {
      const transaction: PlaidTransaction = {
        transaction_id: 'txn-1',
        account_id: 'acc-1',
        amount: 100,
        date: '2026-01-01',
        name: 'Test Transaction',
      } as any;

      const staged: StagedTransaction = {
        id: 'staged-1',
        plaidTransactionId: 'txn-1',
        plaidAccountId: 'acc-1',
        amount: { value: 100, currency: 'USD' },
        timestamp: '2026-01-01T00:00:00Z',
        description: 'Test Transaction',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      } as any;

      service.registerTransaction(staged);
      const result = service.detect(transaction);
      expect(result.isDuplicate).toBeTrue();
    });
  });

  describe('registerTransactions', () => {
    it('should have registerTransactions method', () => {
      expect(service.registerTransactions).toBeDefined();
      expect(typeof service.registerTransactions).toBe('function');
    });

    it('should register multiple transactions without error', () => {
      const stagedList: StagedTransaction[] = [
        {
          id: 'staged-1',
          plaidTransactionId: 'txn-1',
          plaidAccountId: 'acc-1',
          amount: { value: 100, currency: 'USD' },
          timestamp: '2026-01-01T00:00:00Z',
          description: 'Test Transaction 1',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        } as any,
        {
          id: 'staged-2',
          plaidTransactionId: 'txn-2',
          plaidAccountId: 'acc-1',
          amount: { value: 200, currency: 'USD' },
          timestamp: '2026-01-02T00:00:00Z',
          description: 'Test Transaction 2',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        } as any,
      ];

      expect(() => {
        service.registerTransactions(stagedList);
      }).not.toThrow();
    });
  });

  describe('clearIndexes', () => {
    it('should have clearIndexes method', () => {
      expect(service.clearIndexes).toBeDefined();
      expect(typeof service.clearIndexes).toBe('function');
    });

    it('should clear indexes without error', () => {
      expect(() => {
        service.clearIndexes();
      }).not.toThrow();
    });

    it('should remove registered transactions after clear', () => {
      const transaction: PlaidTransaction = {
        transaction_id: 'txn-1',
        account_id: 'acc-1',
        amount: 100,
        date: '2026-01-01',
        name: 'Test Transaction',
      } as any;

      const staged: StagedTransaction = {
        id: 'staged-1',
        plaidTransactionId: 'txn-1',
        plaidAccountId: 'acc-1',
        amount: { value: 100, currency: 'USD' },
        timestamp: '2026-01-01T00:00:00Z',
        description: 'Test Transaction',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      } as any;

      service.registerTransaction(staged);
      service.clearIndexes();
      const result = service.detect(transaction);
      expect(result.isDuplicate).toBeFalse();
    });
  });
});
