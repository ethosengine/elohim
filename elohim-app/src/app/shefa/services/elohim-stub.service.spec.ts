/**
 * Elohim-stub Service Tests
 */

import { TestBed } from '@angular/core/testing';

import { ElohimStubService } from './elohim-stub.service';
import { StagedTransaction } from '../models/transaction-import.model';

describe('ElohimStubService', () => {
  let service: ElohimStubService;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [ElohimStubService],
    });
    service = TestBed.inject(ElohimStubService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should have service instance', () => {
    expect(service).toBeDefined();
  });

  describe('categorizeTransactions', () => {
    it('should have categorizeTransactions method', () => {
      expect(service.categorizeTransactions).toBeDefined();
      expect(typeof service.categorizeTransactions).toBe('function');
    });

    it('should return observable', (done) => {
      const transactions: StagedTransaction[] = [
        {
          id: 'txn-1',
          plaidTransactionId: 'plaid-1',
          plaidAccountId: 'acc-1',
          amount: { value: 100, currency: 'USD' },
          timestamp: new Date().toISOString(),
          description: 'Amazon Purchase',
          merchantName: 'Amazon',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        } as any,
      ];

      const result = service.categorizeTransactions({
        transactions,
        categories: ['Shopping', 'Groceries', 'Other'],
        stewardId: 'steward-1',
      });

      result.subscribe((categorized) => {
        expect(categorized).toBeDefined();
        expect(categorized.length).toBeGreaterThan(0);
        done();
      });
    });

    it('should categorize amazon purchases as Shopping', (done) => {
      const transactions: StagedTransaction[] = [
        {
          id: 'txn-1',
          plaidTransactionId: 'plaid-1',
          plaidAccountId: 'acc-1',
          amount: { value: 100, currency: 'USD' },
          timestamp: new Date().toISOString(),
          description: 'Amazon Purchase',
          merchantName: 'Amazon',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        } as any,
      ];

      const result = service.categorizeTransactions({
        transactions,
        categories: ['Shopping', 'Groceries', 'Other'],
        stewardId: 'steward-1',
      });

      result.subscribe((categorized) => {
        expect(categorized[0].category).toBe('Shopping');
        expect(categorized[0].confidence).toBeGreaterThan(0);
        done();
      });
    });

    it('should generate alternatives for uncertain matches', (done) => {
      const transactions: StagedTransaction[] = [
        {
          id: 'txn-1',
          plaidTransactionId: 'plaid-1',
          plaidAccountId: 'acc-1',
          amount: { value: 100, currency: 'USD' },
          timestamp: new Date().toISOString(),
          description: 'Unknown Store',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        } as any,
      ];

      const result = service.categorizeTransactions({
        transactions,
        categories: ['Shopping', 'Groceries', 'Other'],
        stewardId: 'steward-1',
      });

      result.subscribe((categorized) => {
        expect(categorized[0]?.alternatives).toBeDefined();
        expect(categorized[0]?.alternatives?.length).toBeGreaterThan(0);
        done();
      });
    });
  });

  describe('adjudicateClaim', () => {
    it('should have adjudicateClaim method', () => {
      expect(service.adjudicateClaim).toBeDefined();
      expect(typeof service.adjudicateClaim).toBe('function');
    });

    it('should return observable', (done) => {
      const result = service.adjudicateClaim({
        claimId: 'claim-1',
        claimType: 'health',
        amount: 5000,
        evidence: ['Doctor letter', 'Receipt'],
        memberHistory: {
          claimsCount: 2,
          riskScore: 50,
          memberSince: '2025-01-01',
        },
      });

      result.subscribe((decision) => {
        expect(decision).toBeDefined();
        expect(decision.decision).toBeDefined();
        done();
      });
    });

    it('should approve low-amount claims with evidence', (done) => {
      const result = service.adjudicateClaim({
        claimId: 'claim-1',
        claimType: 'health',
        amount: 500,
        evidence: ['Doctor letter'],
        memberHistory: {
          claimsCount: 1,
          riskScore: 40,
          memberSince: '2025-01-01',
        },
      });

      result.subscribe((decision) => {
        expect(decision.decision).toBe('approve');
        expect(decision.confidence).toBeGreaterThan(0);
        done();
      });
    });

    it('should flag high-risk members for review', (done) => {
      const result = service.adjudicateClaim({
        claimId: 'claim-1',
        claimType: 'health',
        amount: 5000,
        evidence: ['Doctor letter'],
        memberHistory: {
          claimsCount: 10,
          riskScore: 85,
          memberSince: '2025-01-01',
        },
      });

      result.subscribe((decision) => {
        expect(decision.decision).toBe('review');
        done();
      });
    });

    it('should flag claims without evidence for review', (done) => {
      const result = service.adjudicateClaim({
        claimId: 'claim-1',
        claimType: 'health',
        amount: 5000,
        evidence: [],
        memberHistory: {
          claimsCount: 1,
          riskScore: 50,
          memberSince: '2025-01-01',
        },
      });

      result.subscribe((decision) => {
        expect(decision.decision).toBe('review');
        done();
      });
    });
  });

  describe('getCallLogs', () => {
    it('should have getCallLogs method', () => {
      expect(service.getCallLogs).toBeDefined();
      expect(typeof service.getCallLogs).toBe('function');
    });

    it('should return array of call logs', () => {
      const result = service.getCallLogs();
      expect(result).toEqual(jasmine.any(Array));
    });

    it('should initially be empty', () => {
      service.clearLogs();
      const result = service.getCallLogs();
      expect(result.length).toBe(0);
    });

    it('should record call logs after categorization', (done) => {
      service.clearLogs();
      const transactions: StagedTransaction[] = [
        {
          id: 'txn-1',
          plaidTransactionId: 'plaid-1',
          plaidAccountId: 'acc-1',
          amount: { value: 100, currency: 'USD' },
          timestamp: new Date().toISOString(),
          description: 'Amazon',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        } as any,
      ];

      const result = service.categorizeTransactions({
        transactions,
        categories: ['Shopping', 'Other'],
        stewardId: 'steward-1',
      });

      result.subscribe(() => {
        const logs = service.getCallLogs();
        expect(logs.length).toBeGreaterThan(0);
        expect(logs[0].agentType).toBe('categorizer');
        done();
      });
    });
  });

  describe('getCallsByAgent', () => {
    it('should have getCallsByAgent method', () => {
      expect(service.getCallsByAgent).toBeDefined();
      expect(typeof service.getCallsByAgent).toBe('function');
    });

    it('should return empty array initially', () => {
      service.clearLogs();
      const result = service.getCallsByAgent('categorizer');
      expect(result.length).toBe(0);
    });

    it('should filter logs by agent type', (done) => {
      service.clearLogs();
      const transactions: StagedTransaction[] = [
        {
          id: 'txn-1',
          plaidTransactionId: 'plaid-1',
          plaidAccountId: 'acc-1',
          amount: { value: 100, currency: 'USD' },
          timestamp: new Date().toISOString(),
          description: 'Amazon',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        } as any,
      ];

      const result = service.categorizeTransactions({
        transactions,
        categories: ['Shopping', 'Other'],
        stewardId: 'steward-1',
      });

      result.subscribe(() => {
        const logs = service.getCallsByAgent('categorizer');
        expect(logs.length).toBeGreaterThan(0);
        expect(logs.every(log => log.agentType === 'categorizer')).toBeTrue();
        done();
      });
    });
  });

  describe('clearLogs', () => {
    it('should have clearLogs method', () => {
      expect(service.clearLogs).toBeDefined();
      expect(typeof service.clearLogs).toBe('function');
    });

    it('should clear all logs', (done) => {
      const transactions: StagedTransaction[] = [
        {
          id: 'txn-1',
          plaidTransactionId: 'plaid-1',
          plaidAccountId: 'acc-1',
          amount: { value: 100, currency: 'USD' },
          timestamp: new Date().toISOString(),
          description: 'Amazon',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        } as any,
      ];

      const result = service.categorizeTransactions({
        transactions,
        categories: ['Shopping', 'Other'],
        stewardId: 'steward-1',
      });

      result.subscribe(() => {
        expect(service.getCallLogs().length).toBeGreaterThan(0);
        service.clearLogs();
        expect(service.getCallLogs().length).toBe(0);
        done();
      });
    });
  });

  describe('exportLogs', () => {
    it('should have exportLogs method', () => {
      expect(service.exportLogs).toBeDefined();
      expect(typeof service.exportLogs).toBe('function');
    });

    it('should return JSON string', () => {
      const result = service.exportLogs();
      expect(typeof result).toBe('string');
      expect(() => JSON.parse(result)).not.toThrow();
    });

    it('should contain empty array when no logs', () => {
      service.clearLogs();
      const result = service.exportLogs();
      const parsed = JSON.parse(result);
      expect(parsed).toEqual([]);
    });

    it('should export logs as valid JSON', (done) => {
      service.clearLogs();
      const transactions: StagedTransaction[] = [
        {
          id: 'txn-1',
          plaidTransactionId: 'plaid-1',
          plaidAccountId: 'acc-1',
          amount: { value: 100, currency: 'USD' },
          timestamp: new Date().toISOString(),
          description: 'Amazon',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        } as any,
      ];

      const result = service.categorizeTransactions({
        transactions,
        categories: ['Shopping', 'Other'],
        stewardId: 'steward-1',
      });

      result.subscribe(() => {
        const exported = service.exportLogs();
        const parsed = JSON.parse(exported);
        expect(parsed).toEqual(jasmine.any(Array));
        expect(parsed.length).toBeGreaterThan(0);
        done();
      });
    });
  });
});
