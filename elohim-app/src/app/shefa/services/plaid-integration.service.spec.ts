/**
 * Plaid Integration Service Tests
 *
 * Coverage focus:
 * - Constructor and initialization
 * - OAuth flow methods
 * - Transaction fetching
 * - Account management
 * - Webhook handling
 * - Utility methods
 */

import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import { PlaidIntegrationService } from './plaid-integration.service';
import { PlaidConnection, PlaidWebhookPayload } from '../models/transaction-import.model';

describe('PlaidIntegrationService', () => {
  let service: PlaidIntegrationService;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [
        PlaidIntegrationService,
        provideHttpClient(),
        provideHttpClientTesting(),
      ],
    });
    service = TestBed.inject(PlaidIntegrationService);
  });

  // ==========================================================================
  // Service Creation Tests
  // ==========================================================================

  describe('service creation', () => {
    it('should be created', () => {
      expect(service).toBeTruthy();
    });

    it('should have service instance', () => {
      expect(service).toBeDefined();
    });
  });

  // ==========================================================================
  // OAuth Flow Tests
  // ==========================================================================

  describe('OAuth flow', () => {
    it('should have initiatePlaidLink method', () => {
      expect(typeof service.initiatePlaidLink).toBe('function');
    });

    it('should have handlePlaidCallback method', () => {
      expect(typeof service.handlePlaidCallback).toBe('function');
    });

    it('should initiatePlaidLink return Promise', () => {
      const stewardId = 'steward-123';
      const promise = service.initiatePlaidLink(stewardId);
      expect(promise instanceof Promise).toBe(true);
    });

    it('should handlePlaidCallback return Promise<PlaidConnection>', () => {
      const promise = service.handlePlaidCallback('public-token-123');
      expect(promise instanceof Promise).toBe(true);
    });
  });

  // ==========================================================================
  // Transaction Fetching Tests
  // ==========================================================================

  describe('transaction fetching', () => {
    it('should have fetchTransactions method', () => {
      expect(typeof service.fetchTransactions).toBe('function');
    });

    it('should have syncRecentTransactions method', () => {
      expect(typeof service.syncRecentTransactions).toBe('function');
    });

    it('should fetchTransactions return Promise', () => {
      const mockConnection: PlaidConnection = {
        id: 'conn-123',
        connectionNumber: 'PC-001',
        stewardId: 'steward-123',
        plaidItemId: 'item-123',
        plaidAccessToken: 'encrypted-token',
        plaidInstitutionId: 'inst-123',
        institutionName: 'Test Bank',
        linkedAccounts: [],
        status: 'active',
        lastSyncedAt: new Date().toISOString(),
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };

      const dateRange = {
        start: '2024-01-01',
        end: '2024-01-31',
      };

      const promise = service.fetchTransactions(mockConnection, dateRange);
      expect(promise instanceof Promise).toBe(true);
    });

    it('should syncRecentTransactions return Promise', () => {
      const mockConnection: PlaidConnection = {
        id: 'conn-123',
        connectionNumber: 'PC-001',
        stewardId: 'steward-123',
        plaidItemId: 'item-123',
        plaidAccessToken: 'encrypted-token',
        plaidInstitutionId: 'inst-123',
        institutionName: 'Test Bank',
        linkedAccounts: [],
        status: 'active',
        lastSyncedAt: new Date().toISOString(),
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };

      const promise = service.syncRecentTransactions(mockConnection);
      expect(promise instanceof Promise).toBe(true);
    });
  });

  // ==========================================================================
  // Account Management Tests
  // ==========================================================================

  describe('account management', () => {
    it('should have getAccounts method', () => {
      expect(typeof service.getAccounts).toBe('function');
    });

    it('should have refreshConnection method', () => {
      expect(typeof service.refreshConnection).toBe('function');
    });

    it('should getAccounts return Promise', () => {
      const promise = service.getAccounts('encrypted-token');
      expect(promise instanceof Promise).toBe(true);
    });

    it('should refreshConnection return Promise', () => {
      const mockConnection: PlaidConnection = {
        id: 'conn-123',
        connectionNumber: 'PC-001',
        stewardId: 'steward-123',
        plaidItemId: 'item-123',
        plaidAccessToken: 'encrypted-token',
        plaidInstitutionId: 'inst-123',
        institutionName: 'Test Bank',
        linkedAccounts: [],
        status: 'active',
        lastSyncedAt: new Date().toISOString(),
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };

      const promise = service.refreshConnection(mockConnection);
      expect(promise instanceof Promise).toBe(true);
    });
  });

  // ==========================================================================
  // Webhook Handling Tests
  // ==========================================================================

  describe('webhook handling', () => {
    it('should have handleWebhook method', () => {
      expect(typeof service.handleWebhook).toBe('function');
    });

    it('should have onWebhookReceived method', () => {
      expect(typeof service.onWebhookReceived).toBe('function');
    });

    it('should handleWebhook accept PlaidWebhookPayload', () => {
      const payload: PlaidWebhookPayload = {
        webhook_type: 'TRANSACTIONS',
        webhook_code: 'TRANSACTIONS_UPDATES_AVAILABLE',
        item_id: 'item-123',
        new_transactions: 5
      };

      expect(() => service.handleWebhook(payload)).not.toThrow();
    });

    it('should onWebhookReceived return Observable', () => {
      const observable = service.onWebhookReceived();
      expect(observable).toBeDefined();
      expect(observable.subscribe).toBeDefined();
    });

    it('should emit webhook events on handleWebhook', (done) => {
      const payload: PlaidWebhookPayload = {
        webhook_type: 'ITEM',
        webhook_code: 'LOGIN_REQUIRED',
        item_id: 'item-123',
        new_transactions: 0
      };

      service.onWebhookReceived().subscribe(received => {
        expect(received).toEqual(payload);
        done();
      });

      service.handleWebhook(payload);
    });
  });

  // ==========================================================================
  // Method Existence Tests
  // ==========================================================================

  describe('method existence', () => {
    it('should have initiatePlaidLink', () => {
      expect(typeof service.initiatePlaidLink).toBe('function');
    });

    it('should have handlePlaidCallback', () => {
      expect(typeof service.handlePlaidCallback).toBe('function');
    });

    it('should have fetchTransactions', () => {
      expect(typeof service.fetchTransactions).toBe('function');
    });

    it('should have syncRecentTransactions', () => {
      expect(typeof service.syncRecentTransactions).toBe('function');
    });

    it('should have getAccounts', () => {
      expect(typeof service.getAccounts).toBe('function');
    });

    it('should have refreshConnection', () => {
      expect(typeof service.refreshConnection).toBe('function');
    });

    it('should have handleWebhook', () => {
      expect(typeof service.handleWebhook).toBe('function');
    });

    it('should have onWebhookReceived', () => {
      expect(typeof service.onWebhookReceived).toBe('function');
    });
  });

  // ==========================================================================
  // Parameter Acceptance Tests
  // ==========================================================================

  describe('parameter acceptance', () => {
    it('should accept stewardId for initiatePlaidLink', () => {
      const stewardId = 'steward-123';
      expect(() => service.initiatePlaidLink(stewardId)).not.toThrow();
    });

    it('should accept publicToken for handlePlaidCallback', () => {
      const publicToken = 'public-token-123';
      expect(() => service.handlePlaidCallback(publicToken)).not.toThrow();
    });

    it('should accept PlaidConnection and dateRange for fetchTransactions', () => {
      const mockConnection: PlaidConnection = {
        id: 'conn-123',
        connectionNumber: 'PC-001',
        stewardId: 'steward-123',
        plaidItemId: 'item-123',
        plaidAccessToken: 'encrypted-token',
        plaidInstitutionId: 'inst-123',
        institutionName: 'Test Bank',
        linkedAccounts: [],
        status: 'active',
        lastSyncedAt: new Date().toISOString(),
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };

      const dateRange = {
        start: '2024-01-01',
        end: '2024-01-31',
      };

      expect(() => service.fetchTransactions(mockConnection, dateRange)).not.toThrow();
    });

    it('should accept PlaidConnection for syncRecentTransactions', () => {
      const mockConnection: PlaidConnection = {
        id: 'conn-123',
        connectionNumber: 'PC-001',
        stewardId: 'steward-123',
        plaidItemId: 'item-123',
        plaidAccessToken: 'encrypted-token',
        plaidInstitutionId: 'inst-123',
        institutionName: 'Test Bank',
        linkedAccounts: [],
        status: 'active',
        lastSyncedAt: new Date().toISOString(),
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };

      expect(() => service.syncRecentTransactions(mockConnection)).not.toThrow();
    });

    it('should accept accessToken for getAccounts', () => {
      const accessToken = 'access-token-123';
      expect(() => service.getAccounts(accessToken)).not.toThrow();
    });

    it('should accept PlaidConnection for refreshConnection', () => {
      const mockConnection: PlaidConnection = {
        id: 'conn-123',
        connectionNumber: 'PC-001',
        stewardId: 'steward-123',
        plaidItemId: 'item-123',
        plaidAccessToken: 'encrypted-token',
        plaidInstitutionId: 'inst-123',
        institutionName: 'Test Bank',
        linkedAccounts: [],
        status: 'active',
        lastSyncedAt: new Date().toISOString(),
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };

      expect(() => service.refreshConnection(mockConnection)).not.toThrow();
    });

    it('should accept PlaidWebhookPayload for handleWebhook', () => {
      const payload: PlaidWebhookPayload = {
        webhook_type: 'TRANSACTIONS',
        webhook_code: 'TRANSACTIONS_UPDATES_AVAILABLE',
        item_id: 'item-123',
        new_transactions: 5
      };

      expect(() => service.handleWebhook(payload)).not.toThrow();
    });
  });

  // ==========================================================================
  // Observable Return Type Tests
  // ==========================================================================

  describe('return type verification', () => {
    it('onWebhookReceived should return Observable', () => {
      const result = service.onWebhookReceived();
      expect(result).toBeDefined();
      expect(typeof result.subscribe).toBe('function');
    });

    it('initiatePlaidLink should return Promise', () => {
      const result = service.initiatePlaidLink('steward-123');
      expect(result instanceof Promise).toBe(true);
    });

    it('handlePlaidCallback should return Promise', () => {
      const result = service.handlePlaidCallback('public-token');
      expect(result instanceof Promise).toBe(true);
    });

    it('fetchTransactions should return Promise', () => {
      const mockConnection: PlaidConnection = {
        id: 'conn-123',
        connectionNumber: 'PC-001',
        stewardId: 'steward-123',
        plaidItemId: 'item-123',
        plaidAccessToken: 'encrypted-token',
        plaidInstitutionId: 'inst-123',
        institutionName: 'Test Bank',
        linkedAccounts: [],
        status: 'active',
        lastSyncedAt: new Date().toISOString(),
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };

      const result = service.fetchTransactions(mockConnection, {
        start: '2024-01-01',
        end: '2024-01-31',
      });

      expect(result instanceof Promise).toBe(true);
    });

    it('syncRecentTransactions should return Promise', () => {
      const mockConnection: PlaidConnection = {
        id: 'conn-123',
        connectionNumber: 'PC-001',
        stewardId: 'steward-123',
        plaidItemId: 'item-123',
        plaidAccessToken: 'encrypted-token',
        plaidInstitutionId: 'inst-123',
        institutionName: 'Test Bank',
        linkedAccounts: [],
        status: 'active',
        lastSyncedAt: new Date().toISOString(),
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };

      const result = service.syncRecentTransactions(mockConnection);
      expect(result instanceof Promise).toBe(true);
    });

    it('getAccounts should return Promise', () => {
      const result = service.getAccounts('encrypted-token');
      expect(result instanceof Promise).toBe(true);
    });

    it('refreshConnection should return Promise', () => {
      const mockConnection: PlaidConnection = {
        id: 'conn-123',
        connectionNumber: 'PC-001',
        stewardId: 'steward-123',
        plaidItemId: 'item-123',
        plaidAccessToken: 'encrypted-token',
        plaidInstitutionId: 'inst-123',
        institutionName: 'Test Bank',
        linkedAccounts: [],
        status: 'active',
        lastSyncedAt: new Date().toISOString(),
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };

      const result = service.refreshConnection(mockConnection);
      expect(result instanceof Promise).toBe(true);
    });
  });

  // ==========================================================================
  // Edge Cases Tests
  // ==========================================================================

  describe('edge cases', () => {
    it('should handle multiple webhook subscriptions', (done) => {
      let count = 0;
      const payload: PlaidWebhookPayload = {
        webhook_type: 'TRANSACTIONS',
        webhook_code: 'TRANSACTIONS_UPDATES_AVAILABLE',
        item_id: 'item-123',
        new_transactions: 5
      };

      const sub1 = service.onWebhookReceived().subscribe(() => {
        count++;
      });

      const sub2 = service.onWebhookReceived().subscribe(() => {
        count++;
        sub1.unsubscribe();
        sub2.unsubscribe();
        expect(count).toBe(2);
        done();
      });

      service.handleWebhook(payload);
    });

    it('should accept empty linked accounts array', () => {
      const mockConnection: PlaidConnection = {
        id: 'conn-123',
        connectionNumber: 'PC-001',
        stewardId: 'steward-123',
        plaidItemId: 'item-123',
        plaidAccessToken: 'encrypted-token',
        plaidInstitutionId: 'inst-123',
        institutionName: 'Test Bank',
        linkedAccounts: [],
        status: 'active',
        lastSyncedAt: new Date().toISOString(),
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };

      expect(() => service.refreshConnection(mockConnection)).not.toThrow();
    });
  });
});
