import { TestBed } from '@angular/core/testing';
import { HttpClient } from '@angular/common/http';
import { of } from 'rxjs';
import { vi } from 'vitest';

import type { GateEvaluationView, TrustContextView } from '@elohim/storage-client';

import { GateService } from './gate.service';
import { GateInteractionService } from './gate-interaction.service';

function makeTrustContext(overrides: Partial<TrustContextView> = {}): TrustContextView {
  return {
    compositeTrust: 0.5,
    masteryDepth: 0.5,
    stewardStanding: 0.7,
    relationshipDensity: 0.3,
    governanceHealth: 0.9,
    behavioralTrust: 0.85,
    intentDivergence: 0.1,
    declaredIntent: null,
    ...overrides,
  };
}

function makeEvaluation(overrides: Partial<GateEvaluationView> = {}): GateEvaluationView {
  return {
    tier: 'standard',
    trustContext: makeTrustContext(),
    pausePrompt: null,
    confirmToken: null,
    settlementBoundary: null,
    appealPath: null,
    ...overrides,
  };
}

describe('GateInteractionService', () => {
  let service: GateInteractionService;
  let gateService: GateService;
  let httpMock: { post: ReturnType<typeof vi.fn> };

  beforeEach(() => {
    httpMock = { post: vi.fn().mockReturnValue(of({})) };

    TestBed.configureTestingModule({
      providers: [
        GateInteractionService,
        { provide: HttpClient, useValue: httpMock },
      ],
    });
    service = TestBed.inject(GateInteractionService);
    gateService = TestBed.inject(GateService);
  });

  // --- Initial state ---

  it('should start in draft state', () => {
    expect(service.state()).toBe('draft');
  });

  it('should start with empty draft text', () => {
    expect(service.draftText()).toBe('');
  });

  it('should start with no gate result', () => {
    expect(service.gateResult()).toBeNull();
  });

  // --- Reach tier computation ---

  it('should return private when settlementBoundary is present', () => {
    service.handleGateEvaluation(
      makeEvaluation({ settlementBoundary: 'constitutional-limit' }),
    );
    expect(service.reachTier()).toBe('private');
  });

  it('should return close for low trust (0.2)', () => {
    service.handleGateEvaluation(
      makeEvaluation({ trustContext: makeTrustContext({ compositeTrust: 0.2 }) }),
    );
    expect(service.reachTier()).toBe('close');
  });

  it('should return community for mid trust (0.45)', () => {
    service.handleGateEvaluation(
      makeEvaluation({ trustContext: makeTrustContext({ compositeTrust: 0.45 }) }),
    );
    expect(service.reachTier()).toBe('community');
  });

  it('should return network for high trust (0.75)', () => {
    service.handleGateEvaluation(
      makeEvaluation({ trustContext: makeTrustContext({ compositeTrust: 0.75 }) }),
    );
    expect(service.reachTier()).toBe('network');
  });

  it('should return constitutional for very high trust (0.9)', () => {
    service.handleGateEvaluation(
      makeEvaluation({ trustContext: makeTrustContext({ compositeTrust: 0.9 }) }),
    );
    expect(service.reachTier()).toBe('constitutional');
  });

  // --- Submit ---

  it('should transition to evaluating on submit', () => {
    service.submit('hello world', 'create', {});
    expect(service.state()).toBe('evaluating');
    expect(service.draftText()).toBe('hello world');
  });

  it('should transition to affirm when gate passes', () => {
    service.submit('text', 'create', {});
    service.handleGateEvaluation(makeEvaluation());
    expect(service.state()).toBe('affirm');
  });

  it('should transition to dialogue when gate pauses', () => {
    service.submit('text', 'create', {});
    service.handleGateEvaluation(
      makeEvaluation({
        pausePrompt: 'Are you sure?',
        confirmToken: 'tok-123',
      }),
    );
    expect(service.state()).toBe('dialogue');
  });

  it('should transition to settled when gate settles', () => {
    service.submit('text', 'create', {});
    service.handleGateEvaluation(
      makeEvaluation({
        settlementBoundary: 'constitutional-limit',
        appealPath: '/appeal/1',
      }),
    );
    expect(service.state()).toBe('settled');
  });

  // --- Affirm ---

  it('should transition to posted on affirm', () => {
    service.submit('text', 'create', {});
    service.handleGateEvaluation(makeEvaluation());
    expect(service.state()).toBe('affirm');

    service.affirm();
    expect(service.state()).toBe('posted');
  });

  // --- Revise ---

  it('should transition back to draft with new text on revise', () => {
    service.submit('original', 'create', {});
    service.handleGateEvaluation(
      makeEvaluation({ pausePrompt: 'Pause', confirmToken: 'tok' }),
    );
    expect(service.state()).toBe('dialogue');

    service.revise('revised text');
    expect(service.state()).toBe('draft');
    expect(service.draftText()).toBe('revised text');
  });

  // --- Resubmit ---

  it('should transition from draft to evaluating on resubmit', () => {
    service.submit('text', 'create', {});
    service.handleGateEvaluation(
      makeEvaluation({ pausePrompt: 'Pause', confirmToken: 'tok' }),
    );
    service.revise('new text');
    expect(service.state()).toBe('draft');

    service.resubmit();
    expect(service.state()).toBe('evaluating');
  });

  // --- Reset ---

  it('should clear all state on reset', () => {
    service.submit('text', 'create', {});
    service.handleGateEvaluation(makeEvaluation());

    const clearSpy = vi.spyOn(gateService, 'clearState');
    service.reset();

    expect(service.state()).toBe('draft');
    expect(service.draftText()).toBe('');
    expect(service.gateResult()).toBeNull();
    expect(clearSpy).toHaveBeenCalled();
  });

  // --- Edge cases ---

  it('should not double submit when already evaluating', () => {
    service.submit('first', 'create', {});
    expect(service.state()).toBe('evaluating');

    service.submit('second', 'create', {});
    expect(service.draftText()).toBe('first');
    expect(service.state()).toBe('evaluating');
  });

  it('should do nothing on affirm when not in affirm state', () => {
    expect(service.state()).toBe('draft');
    service.affirm();
    expect(service.state()).toBe('draft');
  });

  it('should do nothing on resubmit when not in draft state', () => {
    service.submit('text', 'create', {});
    expect(service.state()).toBe('evaluating');

    service.resubmit();
    expect(service.state()).toBe('evaluating');
  });
});
