import { describe, expect, it } from 'vitest';

import type { GateEvaluationView } from '@elohim/storage-client';

import { extractGateFromResponse, isGatedResponse } from './gated-response.model';

const mockGate: GateEvaluationView = {
  tier: 'pause',
  trustContext: {
    compositeTrust: 0.72,
    masteryDepth: 0.6,
    stewardStanding: 0.8,
    relationshipDensity: 0.5,
    governanceHealth: 0.9,
    behavioralTrust: 0.85,
    intentDivergence: 0.1,
    declaredIntent: 'edit',
  },
  pausePrompt: 'Are you sure you want to proceed?',
  confirmToken: 'tok_abc123',
  settlementBoundary: null,
  appealPath: null,
};

describe('isGatedResponse', () => {
  it('should return true when response has gate field', () => {
    expect(isGatedResponse({ data: {}, gate: mockGate })).toBe(true);
  });

  it('should return false when response has no gate field', () => {
    expect(isGatedResponse({ data: {} })).toBe(false);
  });

  it('should return false for null/undefined', () => {
    expect(isGatedResponse(null)).toBe(false);
    expect(isGatedResponse(undefined)).toBe(false);
  });
});

describe('extractGateFromResponse', () => {
  it('should extract gate from gated response', () => {
    const response = { data: { id: '123' }, gate: mockGate };
    expect(extractGateFromResponse(response)).toBe(mockGate);
  });

  it('should return null for non-gated response', () => {
    expect(extractGateFromResponse({ data: {} })).toBeNull();
  });
});
