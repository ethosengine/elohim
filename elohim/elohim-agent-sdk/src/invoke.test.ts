import { describe, it, expect } from 'vitest';
import Anthropic from '@anthropic-ai/sdk';
import { BudgetEnforcer, handleInvoke } from './invoke.js';
import type { InvokeRequest } from './types.js';

// ============================================================================
// BudgetEnforcer
// ============================================================================

describe('BudgetEnforcer', () => {
  it('starts at configured limit', () => {
    const budget = new BudgetEnforcer(10);
    expect(budget.remaining()).toBe(10);
  });

  it('decrements on consume', () => {
    const budget = new BudgetEnforcer(10);
    budget.consume();
    expect(budget.remaining()).toBe(9);
  });

  it('returns true when budget available', () => {
    const budget = new BudgetEnforcer(5);
    expect(budget.consume()).toBe(true);
  });

  it('returns false when exhausted', () => {
    const budget = new BudgetEnforcer(10);
    for (let i = 0; i < 10; i++) {
      expect(budget.consume()).toBe(true);
    }
    expect(budget.consume()).toBe(false);
  });

  it('remaining never goes below 0', () => {
    const budget = new BudgetEnforcer(2);
    budget.consume();
    budget.consume();
    budget.consume(); // past zero
    budget.consume(); // way past zero
    expect(budget.remaining()).toBe(0);
  });
});

// ============================================================================
// handleInvoke — budget exhaustion
// ============================================================================

describe('handleInvoke', () => {
  it('declines immediately when budget is exhausted', async () => {
    const budget = new BudgetEnforcer(0);

    const request: InvokeRequest = {
      requestId: 'test-req-1',
      elohimId: 'test-elohim',
      capability: 'path-recommendation',
      params: { query: 'mathematics' },
      requesterId: 'agent-1',
      priority: 'normal',
    };

    // SDK should never be called — pass an empty object
    const mockSdk = {} as unknown as Anthropic;

    const response = await handleInvoke(request, mockSdk, budget);

    expect(response.status).toBe('declined');
    expect(response.declineReason).toBe('Budget exhausted');
    expect(response.requestId).toBe('test-req-1');
    expect(response.elohimId).toBe('test-elohim');
    expect(response.constitutionalReasoning.primaryPrinciple).toBe(
      'Resource stewardship',
    );
    expect(response.respondedAt).toBeTruthy();
  });

  it('uses elohimId from request when provided', async () => {
    const budget = new BudgetEnforcer(0);

    const request: InvokeRequest = {
      requestId: 'test-req-2',
      elohimId: 'custom-elohim-id',
      capability: 'content-safety-review',
      params: {},
      requesterId: 'agent-2',
      priority: 'high',
    };

    const mockSdk = {} as unknown as Anthropic;
    const response = await handleInvoke(request, mockSdk, budget);

    expect(response.elohimId).toBe('custom-elohim-id');
    expect(response.status).toBe('declined');
  });
});
