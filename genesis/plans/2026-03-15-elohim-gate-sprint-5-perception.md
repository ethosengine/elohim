# ElohimGate Sprint 5: Perception — Angular Gate Client

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build the Angular service that interprets gate evaluation responses from mutation endpoints — handling PassThrough (transparent), Pause (409 confirm flow), and Settlement (403 boundary display). Give the frontend eyes to see what the gate is saying.

**Architecture:** A `GateService` in the elohim pillar that intercepts gate responses from mutation API calls, exposes reactive state (`latestEvaluation$`, `isPaused$`), and provides a `confirmPause(token)` method. Existing mutation services (`StorageApiService`, `RecognitionApiService`) get a shared HTTP interceptor or response handler that extracts the `gate` field from wrapped responses.

**Tech Stack:** Angular 19, RxJS, Vitest, HttpClient, generated types from `@elohim/storage-client`.

---

## Sprint 1–4 Feedback Incorporated

- **No new endpoints needed**: Backend already returns gate evaluation in every mutation response and has `POST /api/v1/gate/confirm`.
- **Types already generated**: `GateEvaluationView.ts` and `TrustContextView.ts` exist in `@elohim/storage-client/generated/`.
- **Response envelope**: Mutations return `{ data: T, gate: GateEvaluationView }`. Pause returns 409 with `{ gate, pausePrompt, confirmToken }`. Settlement returns 403 with `{ gate, boundary, appealPath }`.
- **No SSE this sprint**: Real-time streaming deferred to Sprint 6. This sprint handles request/response only.
- **Use inject() pattern**: All services use `inject()` not constructor DI (esbuild compatibility, see CLAUDE.md known issues).

---

## Task 1: GateService — Core Service with Reactive State

**Files:**
- Create: `app/elohim-app/src/app/elohim/services/gate.service.ts`
- Create: `app/elohim-app/src/app/elohim/services/gate.service.spec.ts`
- Modify: `app/elohim-app/src/app/elohim/services/index.ts` (add barrel export)

**Step 1: Write the test file**

```typescript
// gate.service.spec.ts
import { TestBed } from '@angular/core/testing';
import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { of, throwError } from 'rxjs';
import { GateService } from './gate.service';

describe('GateService', () => {
  let service: GateService;
  let httpMock: { post: ReturnType<typeof vi.fn> };

  beforeEach(() => {
    httpMock = { post: vi.fn() };

    TestBed.configureTestingModule({
      providers: [
        GateService,
        { provide: HttpClient, useValue: httpMock },
      ],
    });

    service = TestBed.inject(GateService);
  });

  describe('initial state', () => {
    it('should start with no evaluation', () => {
      expect(service.latestEvaluation()).toBeNull();
    });

    it('should not be paused', () => {
      expect(service.isPaused()).toBe(false);
    });
  });

  describe('handleGateResponse', () => {
    it('should update latestEvaluation on passthrough', () => {
      const gate = {
        tier: 'Light',
        trustContext: {
          compositeTrust: 0.8,
          masteryDepth: 0.6,
          stewardStanding: 0.7,
          relationshipDensity: 0.5,
          governanceHealth: 0.9,
          behavioralTrust: 0.8,
          intentDivergence: 0.1,
          declaredIntent: null,
        },
        pausePrompt: null,
        confirmToken: null,
        settlementBoundary: null,
        appealPath: null,
      };

      service.handleGateResponse(gate);

      expect(service.latestEvaluation()).toEqual(gate);
      expect(service.isPaused()).toBe(false);
    });

    it('should set paused state when pausePrompt is present', () => {
      const gate = {
        tier: 'Deep',
        trustContext: {
          compositeTrust: 0.3,
          masteryDepth: 0.2,
          stewardStanding: 0.1,
          relationshipDensity: 0.3,
          governanceHealth: 0.4,
          behavioralTrust: 0.3,
          intentDivergence: 0.6,
          declaredIntent: null,
        },
        pausePrompt: 'This action affects multiple stewards.',
        confirmToken: 'token-abc123',
        settlementBoundary: null,
        appealPath: null,
      };

      service.handleGateResponse(gate);

      expect(service.isPaused()).toBe(true);
      expect(service.latestEvaluation()?.pausePrompt).toBe(
        'This action affects multiple stewards.',
      );
    });

    it('should set settled state when settlementBoundary is present', () => {
      const gate = {
        tier: 'Constitutional',
        trustContext: {
          compositeTrust: 0.1,
          masteryDepth: 0.1,
          stewardStanding: 0.1,
          relationshipDensity: 0.1,
          governanceHealth: 0.1,
          behavioralTrust: 0.1,
          intentDivergence: 0.9,
          declaredIntent: null,
        },
        pausePrompt: null,
        confirmToken: null,
        settlementBoundary: 'Constitutional: steward holding > 50%',
        appealPath: '/governance/appeal/123',
      };

      service.handleGateResponse(gate);

      expect(service.isSettled()).toBe(true);
      expect(service.isPaused()).toBe(false);
    });
  });

  describe('confirmPause', () => {
    it('should POST to gate confirm endpoint', () => {
      httpMock.post.mockReturnValue(of({ success: true }));

      service.confirmPause('token-abc123').subscribe();

      expect(httpMock.post).toHaveBeenCalledWith(
        '/api/v1/gate/confirm',
        { confirmToken: 'token-abc123' },
      );
    });

    it('should clear paused state on success', () => {
      httpMock.post.mockReturnValue(of({ success: true }));

      // Set paused state first
      service.handleGateResponse({
        tier: 'Deep',
        trustContext: {
          compositeTrust: 0.3, masteryDepth: 0.2, stewardStanding: 0.1,
          relationshipDensity: 0.3, governanceHealth: 0.4, behavioralTrust: 0.3,
          intentDivergence: 0.6, declaredIntent: null,
        },
        pausePrompt: 'Review needed',
        confirmToken: 'token-abc123',
        settlementBoundary: null,
        appealPath: null,
      });

      expect(service.isPaused()).toBe(true);

      service.confirmPause('token-abc123').subscribe();

      expect(service.isPaused()).toBe(false);
    });
  });

  describe('clearState', () => {
    it('should reset all state', () => {
      service.handleGateResponse({
        tier: 'Deep',
        trustContext: {
          compositeTrust: 0.3, masteryDepth: 0.2, stewardStanding: 0.1,
          relationshipDensity: 0.3, governanceHealth: 0.4, behavioralTrust: 0.3,
          intentDivergence: 0.6, declaredIntent: null,
        },
        pausePrompt: 'Review needed',
        confirmToken: 'token-abc',
        settlementBoundary: null,
        appealPath: null,
      });

      service.clearState();

      expect(service.latestEvaluation()).toBeNull();
      expect(service.isPaused()).toBe(false);
      expect(service.isSettled()).toBe(false);
    });
  });
});
```

**Step 2: Run test to verify it fails**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "gate.service" --reporter=verbose`
Expected: FAIL (service doesn't exist)

**Step 3: Write minimal implementation**

```typescript
// gate.service.ts
import { Injectable, inject, signal, computed } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { Observable, tap } from 'rxjs';
import type { GateEvaluationView } from '@elohim/storage-client';

@Injectable({ providedIn: 'root' })
export class GateService {
  private readonly http = inject(HttpClient);

  private readonly _latestEvaluation = signal<GateEvaluationView | null>(null);

  readonly latestEvaluation = this._latestEvaluation.asReadonly();

  readonly isPaused = computed(
    () => this._latestEvaluation()?.pausePrompt != null,
  );

  readonly isSettled = computed(
    () => this._latestEvaluation()?.settlementBoundary != null,
  );

  readonly trustContext = computed(
    () => this._latestEvaluation()?.trustContext ?? null,
  );

  handleGateResponse(gate: GateEvaluationView): void {
    this._latestEvaluation.set(gate);
  }

  confirmPause(confirmToken: string): Observable<unknown> {
    return this.http
      .post('/api/v1/gate/confirm', { confirmToken })
      .pipe(tap(() => this._latestEvaluation.set(null)));
  }

  clearState(): void {
    this._latestEvaluation.set(null);
  }
}
```

**Step 4: Run test to verify it passes**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "gate.service" --reporter=verbose`
Expected: All 7 tests PASS

**Step 5: Add barrel export**

Add to `app/elohim-app/src/app/elohim/services/index.ts`:

```typescript
export { GateService } from './gate.service';
```

**Step 6: Run lint**

Run: `cd app/elohim-app && pnpm run lint`
Expected: Clean

**Step 7: Commit**

```bash
git add app/elohim-app/src/app/elohim/services/gate.service.ts app/elohim-app/src/app/elohim/services/gate.service.spec.ts app/elohim-app/src/app/elohim/services/index.ts
git commit -m "feat(gate): add GateService with reactive evaluation state and pause confirm"
```

---

## Task 2: GatedResponse Type and Response Extractor

**Files:**
- Create: `app/elohim-app/src/app/elohim/models/gated-response.model.ts`
- Create: `app/elohim-app/src/app/elohim/models/gated-response.model.spec.ts`
- Modify: `app/elohim-app/src/app/elohim/models/index.ts` (add barrel export)

**Step 1: Write the test file**

```typescript
// gated-response.model.spec.ts
import { extractGateFromResponse, isGatedResponse } from './gated-response.model';

describe('GatedResponse utilities', () => {
  const mockGate = {
    tier: 'Light',
    trustContext: {
      compositeTrust: 0.8, masteryDepth: 0.6, stewardStanding: 0.7,
      relationshipDensity: 0.5, governanceHealth: 0.9, behavioralTrust: 0.8,
      intentDivergence: 0.1, declaredIntent: null,
    },
    pausePrompt: null,
    confirmToken: null,
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
      const result = extractGateFromResponse({ data: { id: '1' }, gate: mockGate });
      expect(result).toEqual(mockGate);
    });

    it('should return null for non-gated response', () => {
      const result = extractGateFromResponse({ data: { id: '1' } });
      expect(result).toBeNull();
    });
  });
});
```

**Step 2: Run test to verify it fails**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "gated-response" --reporter=verbose`
Expected: FAIL

**Step 3: Write implementation**

```typescript
// gated-response.model.ts
import type { GateEvaluationView } from '@elohim/storage-client';

/**
 * Response envelope for gated mutations.
 * Every mutation endpoint returns { data: T, gate: GateEvaluationView }.
 */
export interface GatedResponse<T> {
  data: T;
  gate: GateEvaluationView;
}

/**
 * Type guard: does this response include gate evaluation?
 */
export function isGatedResponse(response: unknown): response is GatedResponse<unknown> {
  return (
    response != null &&
    typeof response === 'object' &&
    'gate' in response &&
    response.gate != null
  );
}

/**
 * Extract gate evaluation from a response, or null if not present.
 */
export function extractGateFromResponse(response: unknown): GateEvaluationView | null {
  if (isGatedResponse(response)) {
    return response.gate;
  }
  return null;
}
```

**Step 4: Run test to verify it passes**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "gated-response" --reporter=verbose`
Expected: All 5 tests PASS

**Step 5: Add barrel export**

Add to `app/elohim-app/src/app/elohim/models/index.ts`:

```typescript
export { GatedResponse, isGatedResponse, extractGateFromResponse } from './gated-response.model';
```

**Step 6: Run lint**

Run: `cd app/elohim-app && pnpm run lint`

**Step 7: Commit**

```bash
git add app/elohim-app/src/app/elohim/models/gated-response.model.ts app/elohim-app/src/app/elohim/models/gated-response.model.spec.ts app/elohim-app/src/app/elohim/models/index.ts
git commit -m "feat(gate): add GatedResponse type and extraction utilities"
```

---

## Task 3: Gate HTTP Error Handler

**Files:**
- Create: `app/elohim-app/src/app/elohim/services/gate-error.handler.ts`
- Create: `app/elohim-app/src/app/elohim/services/gate-error.handler.spec.ts`

**Step 1: Write the test file**

```typescript
// gate-error.handler.spec.ts
import { TestBed } from '@angular/core/testing';
import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { GateService } from './gate.service';
import { handleGateError } from './gate-error.handler';
import { firstValueFrom } from 'rxjs';

describe('handleGateError', () => {
  let gateService: GateService;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [
        GateService,
        { provide: HttpClient, useValue: { post: vi.fn() } },
      ],
    });

    gateService = TestBed.inject(GateService);
  });

  it('should handle 409 pause by updating gate service', () => {
    const errorBody = {
      gate: {
        tier: 'Deep',
        trustContext: {
          compositeTrust: 0.3, masteryDepth: 0.2, stewardStanding: 0.1,
          relationshipDensity: 0.3, governanceHealth: 0.4, behavioralTrust: 0.3,
          intentDivergence: 0.6, declaredIntent: null,
        },
        pausePrompt: 'Review needed',
        confirmToken: 'token-abc',
        settlementBoundary: null,
        appealPath: null,
      },
      pausePrompt: 'Review needed',
      confirmToken: 'token-abc',
    };

    const error = new HttpErrorResponse({ status: 409, error: errorBody });
    const result$ = handleGateError(error, gateService);

    expect(() => firstValueFrom(result$)).rejects.toBeTruthy();
    expect(gateService.isPaused()).toBe(true);
  });

  it('should handle 403 settlement by updating gate service', () => {
    const errorBody = {
      gate: {
        tier: 'Constitutional',
        trustContext: {
          compositeTrust: 0.1, masteryDepth: 0.1, stewardStanding: 0.1,
          relationshipDensity: 0.1, governanceHealth: 0.1, behavioralTrust: 0.1,
          intentDivergence: 0.9, declaredIntent: null,
        },
        pausePrompt: null,
        confirmToken: null,
        settlementBoundary: 'Constitutional boundary',
        appealPath: '/appeal/123',
      },
      boundary: 'Constitutional boundary',
      appealPath: '/appeal/123',
    };

    const error = new HttpErrorResponse({ status: 403, error: errorBody });
    const result$ = handleGateError(error, gateService);

    expect(() => firstValueFrom(result$)).rejects.toBeTruthy();
    expect(gateService.isSettled()).toBe(true);
  });

  it('should rethrow non-gate errors unchanged', async () => {
    const error = new HttpErrorResponse({ status: 500, error: 'Server error' });
    const result$ = handleGateError(error, gateService);

    await expect(firstValueFrom(result$)).rejects.toThrow();
    expect(gateService.latestEvaluation()).toBeNull();
  });

  it('should rethrow 409 without gate field', async () => {
    const error = new HttpErrorResponse({ status: 409, error: { message: 'conflict' } });
    const result$ = handleGateError(error, gateService);

    await expect(firstValueFrom(result$)).rejects.toThrow();
    expect(gateService.isPaused()).toBe(false);
  });
});
```

**Step 2: Run test to verify it fails**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "gate-error" --reporter=verbose`
Expected: FAIL

**Step 3: Write implementation**

```typescript
// gate-error.handler.ts
import { HttpErrorResponse } from '@angular/common/http';
import { Observable, throwError } from 'rxjs';
import type { GateService } from './gate.service';
import { isGatedResponse } from '../models/gated-response.model';

/**
 * Handle gate-specific HTTP errors (409 Pause, 403 Settlement).
 * Updates GateService state, then rethrows so callers can react.
 */
export function handleGateError(
  error: HttpErrorResponse,
  gateService: GateService,
): Observable<never> {
  if ((error.status === 409 || error.status === 403) && isGatedResponse(error.error)) {
    gateService.handleGateResponse(error.error.gate);
  }
  return throwError(() => error);
}
```

**Step 4: Run test to verify it passes**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "gate-error" --reporter=verbose`
Expected: All 4 tests PASS

**Step 5: Run lint**

Run: `cd app/elohim-app && pnpm run lint`

**Step 6: Commit**

```bash
git add app/elohim-app/src/app/elohim/services/gate-error.handler.ts app/elohim-app/src/app/elohim/services/gate-error.handler.spec.ts
git commit -m "feat(gate): add gate error handler for 409 pause and 403 settlement"
```

---

## Task 4: Wire GateService into StorageApiService Mutation Methods

**Files:**
- Modify: `app/elohim-app/src/app/elohim/services/storage-api.service.ts`
- Modify: `app/elohim-app/src/app/elohim/services/storage-api.service.spec.ts` (if exists, add gate tests)

**Context:** StorageApiService already has mutation methods (createAllocation, updateAllocation, deleteAllocation, etc.). These now return `{ data: T, gate: GateEvaluationView }`. We need to:
1. Inject GateService
2. After each mutation response, extract the gate field and pass to `gateService.handleGateResponse()`
3. On error, use `handleGateError` for 409/403

**Step 1: Read the current StorageApiService to understand its mutation methods**

Read: `app/elohim-app/src/app/elohim/services/storage-api.service.ts`

Identify all mutation methods (POST, PUT, DELETE calls to `/api/v1/*` routes).

**Step 2: Add GateService injection and gate response handling**

Add to imports:
```typescript
import { GateService } from './gate.service';
import { handleGateError } from './gate-error.handler';
import { extractGateFromResponse } from '../models/gated-response.model';
```

Add to the service:
```typescript
private readonly gateService = inject(GateService);
```

For each mutation method, add a `tap` to extract gate and a `catchError` for gate errors:
```typescript
// Example pattern for mutation methods:
createAllocation(input: CreateAllocationInputView): Observable<StewardshipAllocationView> {
  return this.http
    .post<GatedResponse<StewardshipAllocationView>>(`${this.baseUrl}/api/v1/stewardship/allocations`, input)
    .pipe(
      tap(response => {
        const gate = extractGateFromResponse(response);
        if (gate) this.gateService.handleGateResponse(gate);
      }),
      map(response => response.data),
      catchError(error => handleGateError(error, this.gateService)),
    );
}
```

**Important**: Only modify methods that hit `/api/v1/*` mutation routes (the ones with gate evaluation). Do NOT modify read-only `/db/*` routes.

**Step 3: Run existing tests to verify no breakage**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "storage-api.service" --reporter=verbose`

**Step 4: Run lint**

Run: `cd app/elohim-app && pnpm run lint`

**Step 5: Commit**

```bash
git add app/elohim-app/src/app/elohim/services/storage-api.service.ts
git commit -m "feat(gate): wire GateService into StorageApiService mutation methods"
```

---

## Task 5: Wire GateService into RecognitionApiService

**Files:**
- Modify: `app/elohim-app/src/app/elohim/services/recognition-api.service.ts`

**Context:** RecognitionApiService has a `distribute()` method that hits `POST /api/v1/recognition/distribute`. This now returns a gated response. Same pattern as Task 4.

**Step 1: Read the current RecognitionApiService**

Read: `app/elohim-app/src/app/elohim/services/recognition-api.service.ts`

**Step 2: Add gate handling to distribute method**

Same pattern: inject GateService, tap to extract gate, catchError for 409/403.

**Step 3: Run tests**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "recognition-api" --reporter=verbose`

**Step 4: Run lint**

Run: `cd app/elohim-app && pnpm run lint`

**Step 5: Commit**

```bash
git add app/elohim-app/src/app/elohim/services/recognition-api.service.ts
git commit -m "feat(gate): wire GateService into RecognitionApiService"
```

---

## Task 6: Integration Verification

**Files:**
- No file changes — verification only

**Step 1: Run full Angular test suite**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts --reporter=verbose 2>&1 | tail -20`
Expected: All tests pass (existing + ~16 new gate tests)

**Step 2: Run lint**

Run: `cd app/elohim-app && pnpm run lint`
Expected: Clean

**Step 3: Verify barrel exports work**

Run: `grep -n "GateService\|GatedResponse\|extractGateFromResponse" app/elohim-app/src/app/elohim/services/index.ts app/elohim-app/src/app/elohim/models/index.ts`
Expected: All exports present

**Step 4: Verify imports compile**

Run: `cd app/elohim-app && pnpm run build 2>&1 | tail -10`
Expected: Build succeeds (or at minimum, no gate-related errors)

---

## Verification

### Tests
```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts --reporter=verbose
```

### Lint
```bash
cd app/elohim-app && pnpm run lint
```

### Build
```bash
cd app/elohim-app && pnpm run build
```

---

## What This Sprint Completes

After Sprint 5, the Angular frontend can:
- See gate evaluation results on every mutation response
- Detect pause (409) and settlement (403) gate responses
- Confirm paused mutations via `GateService.confirmPause(token)`
- Access trust context signals reactively via `GateService.trustContext()`

## What Remains (Future Sprints)

- **Sprint 6: SSE Streaming** — Push gate evaluations to Angular in real-time (no existing SSE patterns)
- **Sprint 7: Inference Sidecar** — Connect elohim-agent-sdk at :8095 for Deep/Constitutional tier
- **Sprint 8: Gate UI Components** — Trust context visualization, pause confirm dialog, settlement display
