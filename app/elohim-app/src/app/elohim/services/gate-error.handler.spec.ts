import { TestBed } from '@angular/core/testing';
import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { firstValueFrom } from 'rxjs';
import { vi } from 'vitest';

import type { GateEvaluationView } from '@elohim/storage-client';

import { GateService } from './gate.service';
import { handleGateError } from './gate-error.handler';

function makeGate(overrides: Partial<GateEvaluationView> = {}): GateEvaluationView {
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

describe('handleGateError', () => {
  let gateService: GateService;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [{ provide: HttpClient, useValue: { post: vi.fn() } }],
    });
    gateService = TestBed.inject(GateService);
  });

  it('should handle 409 pause by updating gate service', async () => {
    const gate = makeGate({
      pausePrompt: 'Are you sure?',
      confirmToken: 'tok-pause-1',
    });
    const error = new HttpErrorResponse({
      status: 409,
      error: { gate },
    });

    const result$ = handleGateError(error, gateService);

    await expect(firstValueFrom(result$)).rejects.toBeTruthy();
    expect(gateService.isPaused()).toBe(true);
    expect(gateService.latestEvaluation()?.pausePrompt).toBe('Are you sure?');
  });

  it('should handle 403 settlement by updating gate service', async () => {
    const gate = makeGate({
      settlementBoundary: 'constitutional-limit',
      appealPath: '/appeal/456',
    });
    const error = new HttpErrorResponse({
      status: 403,
      error: { gate },
    });

    const result$ = handleGateError(error, gateService);

    await expect(firstValueFrom(result$)).rejects.toBeTruthy();
    expect(gateService.isSettled()).toBe(true);
    expect(gateService.latestEvaluation()?.settlementBoundary).toBe('constitutional-limit');
  });

  it('should rethrow non-gate errors unchanged', async () => {
    const error = new HttpErrorResponse({
      status: 500,
      error: { message: 'Internal Server Error' },
    });

    const result$ = handleGateError(error, gateService);

    await expect(firstValueFrom(result$)).rejects.toBeTruthy();
    expect(gateService.isPaused()).toBe(false);
    expect(gateService.isSettled()).toBe(false);
    expect(gateService.latestEvaluation()).toBeNull();
  });

  it('should rethrow 409 without gate field', async () => {
    const error = new HttpErrorResponse({
      status: 409,
      error: { message: 'Conflict' },
    });

    const result$ = handleGateError(error, gateService);

    await expect(firstValueFrom(result$)).rejects.toBeTruthy();
    expect(gateService.isPaused()).toBe(false);
    expect(gateService.latestEvaluation()).toBeNull();
  });
});
