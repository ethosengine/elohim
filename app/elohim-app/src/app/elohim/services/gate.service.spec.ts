import { TestBed } from '@angular/core/testing';
import { HttpClient } from '@angular/common/http';
import { of } from 'rxjs';
import { vi } from 'vitest';

import type { GateEvaluationView } from '@elohim/storage-client';

import { GateService } from './gate.service';

function makeEvaluation(overrides: Partial<GateEvaluationView> = {}): GateEvaluationView {
  return {
    tier: 'standard',
    trustContext: {
      compositeTrust: 0.8,
      masteryDepth: 0.5,
      stewardStanding: 0.7,
      relationshipDensity: 0.3,
      governanceHealth: 0.9,
      behavioralTrust: 0.85,
      intentDivergence: 0.1,
      declaredIntent: null,
    },
    pausePrompt: null,
    confirmToken: null,
    settlementBoundary: null,
    appealPath: null,
    ...overrides,
  };
}

describe('GateService', () => {
  let service: GateService;
  let httpMock: { post: ReturnType<typeof vi.fn> };

  beforeEach(() => {
    httpMock = { post: vi.fn() };

    TestBed.configureTestingModule({
      providers: [{ provide: HttpClient, useValue: httpMock }],
    });
    service = TestBed.inject(GateService);
  });

  it('should start with no evaluation', () => {
    expect(service.latestEvaluation()).toBeNull();
  });

  it('should not be paused initially', () => {
    expect(service.isPaused()).toBe(false);
  });

  it('should update latestEvaluation on passthrough', () => {
    const gate = makeEvaluation();
    service.handleGateResponse(gate);

    expect(service.latestEvaluation()).toEqual(gate);
    expect(service.isPaused()).toBe(false);
  });

  it('should set paused state when pausePrompt is present', () => {
    const gate = makeEvaluation({
      pausePrompt: 'Are you sure you want to proceed?',
      confirmToken: 'tok-123',
    });
    service.handleGateResponse(gate);

    expect(service.isPaused()).toBe(true);
  });

  it('should set settled state when settlementBoundary is present', () => {
    const gate = makeEvaluation({
      settlementBoundary: 'constitutional-limit',
      appealPath: '/appeal/123',
    });
    service.handleGateResponse(gate);

    expect(service.isSettled()).toBe(true);
  });

  it('should POST to gate confirm endpoint', () => {
    httpMock.post.mockReturnValue(of({}));

    service.confirmPause('tok-abc');

    expect(httpMock.post).toHaveBeenCalledWith('/api/v1/gate/confirm', {
      confirmToken: 'tok-abc',
    });
  });

  it('should clear paused state on confirm success', () => {
    const gate = makeEvaluation({
      pausePrompt: 'Pause prompt',
      confirmToken: 'tok-abc',
    });
    service.handleGateResponse(gate);
    expect(service.isPaused()).toBe(true);

    httpMock.post.mockReturnValue(of({}));
    service.confirmPause('tok-abc').subscribe();

    expect(service.isPaused()).toBe(false);
    expect(service.latestEvaluation()).toBeNull();
  });

  it('should reset all state on clearState', () => {
    const gate = makeEvaluation({
      pausePrompt: 'Pause',
      confirmToken: 'tok',
      settlementBoundary: 'boundary',
    });
    service.handleGateResponse(gate);
    expect(service.isPaused()).toBe(true);
    expect(service.isSettled()).toBe(true);

    service.clearState();

    expect(service.latestEvaluation()).toBeNull();
    expect(service.isPaused()).toBe(false);
    expect(service.isSettled()).toBe(false);
    expect(service.trustContext()).toBeNull();
  });
});
