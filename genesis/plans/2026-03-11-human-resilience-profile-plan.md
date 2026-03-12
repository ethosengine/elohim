# Human Resilience Profile Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build the ResilienceProfile shefa projection — types, pure computation functions, a2o scenarios through genesis humans, and an icon design story skeleton in Storybook.

**Architecture:** ResilienceProfile is a shefa projection computed from existing protocol primitives (ShardManifest, MutualAidContext, Commitment, CustodianNode, trust topology). No new on-chain types. The model lives in shefa/models, the computation service implements an IResilience interface with InjectionToken (matching existing shefa patterns), and the a2o scenarios drive the design through graduated genesis human stories. A Storybook MDX story captures the icon design brief for future visual work.

**Tech Stack:** Angular 19, TypeScript (strict), Vitest, Storybook 8, Cucumber/Gherkin (a2o)

---

### Task 1: ResilienceProfile Model

Create the core type definitions for the resilience projection.

**Files:**
- Create: `app/elohim-app/src/app/shefa/models/resilience-profile.model.ts`
- Modify: `app/elohim-app/src/app/shefa/models/index.ts`

**Context:**
- Follows existing shefa model patterns (see `stewarded-resources.model.ts` for style)
- Imports `ReachLevel` from `@app/elohim/models/protocol-core.model`
- Imports `AgentRef` from `@app/elohim/models/coordination-envelope.model`
- All imports use `import type` for type-only imports
- The file uses JSDoc comments in the style of other shefa models

**Step 1: Create the model file**

```typescript
/**
 * Resilience Profile Model — Human Data Protection at a Glance
 *
 * A shefa projection that composes existing protocol primitives (shard manifests,
 * mutual aid contexts, custodian commitments, trust topology) into a single
 * answer: "Am I safe? And if not, what should I do?"
 *
 * This is NOT a new protocol primitive — it's a view over what already exists.
 * The protocol computes the mechanical score; elohim assess whether it's adequate
 * given what the data is and where it lives.
 *
 * Design notes:
 * - Protection is per-content, shaped by reach and sensitivity. Medical records
 *   need institutional-grade attestation. A shared movie just needs a friendly peer.
 * - Attestation happens through use, not ceremony. Every shard fetch is a heartbeat.
 * - Data has a lifecycle. The right to be forgotten is a resilience concern.
 * - Elohim are LLMs — they need narrative memory alongside quantitative scores.
 */

import type { ReachLevel } from '@app/elohim/models/protocol-core.model';
import type { AgentRef } from '@app/elohim/models/coordination-envelope.model';

// =============================================================================
// CORE PROFILE
// =============================================================================

/**
 * ProtectionStatus — the human-facing state.
 *
 * - 'at-risk': data cannot survive loss of a single peer
 * - 'partial': some content protected, some vulnerable
 * - 'protected': data survives any single peer loss, commitments reciprocated
 */
export type ProtectionStatus = 'at-risk' | 'partial' | 'protected';

/**
 * ResilienceProfile — the top-level projection.
 *
 * Computed from ShardManifest + MutualAidContext + Commitment + CustodianNode +
 * trust topology. Not stored on-chain — assembled on demand.
 */
export interface ResilienceProfile {
  humanId: string;
  overallScore: number; // 0-1 normalized
  protectionStatus: ProtectionStatus;

  // What feeds the score
  shardHealth: ShardHealthSummary;
  commitmentHealth: CommitmentHealthSummary;
  trustCircleDepth: TrustCircleDepth;

  // Per-content risk (not all data needs the same protection)
  contentRiskBreakdown: ContentRiskBucket[];

  // What to do about it
  nextAction?: ResilienceAction;

  // Elohim assessment (discernment layer)
  elohimAssessment?: ElohimResilienceAssessment;

  lastComputedAt: string; // ISO 8601
}

// =============================================================================
// SHARD HEALTH
// =============================================================================

/**
 * ShardHealthSummary — mechanical distribution metrics.
 *
 * Derived from ShardManifest records and peer topology. Answers:
 * "How many peers hold my shards, and what encoding protects them?"
 */
export interface ShardHealthSummary {
  totalBlobs: number;
  totalShards: number;
  distinctPeers: number;
  averageShardsPerBlob: number;
  encodingBreakdown: {
    single: number; // No redundancy
    chunked: number; // Sequential chunks
    reedSolomon: number; // RS 4-of-7 (recoverable from any 4)
  };
  singlePointOfFailureCount: number; // Blobs on only one peer
  lastAccessVerifiedAt: string; // Most recent implicit heartbeat
}

// =============================================================================
// COMMITMENT HEALTH
// =============================================================================

/**
 * CommitmentHealthSummary — mutual aid agreement status.
 *
 * Derived from MutualAidContext and Commitment records. Answers:
 * "Do I have people who've committed to backing me up, and am I reciprocating?"
 */
export interface CommitmentHealthSummary {
  activeCommitments: number;
  reciprocatedCommitments: number; // Both sides contribute
  expiringSoon: number; // Commitments nearing expiry
  totalPeersCommitted: number; // Distinct humans backing your data
  commitmentCoverage: number; // 0-1, fraction of data commitment-backed
}

// =============================================================================
// TRUST CIRCLE DEPTH
// =============================================================================

/**
 * TrustCircleDepth — which relationship circles contribute stewardship.
 *
 * Different circles provide different protection qualities:
 * household is always-on but geographically concentrated;
 * institutional is jurisdictionally diverse but less intimate.
 */
export interface TrustCircleDepth {
  householdPeers: number;
  friendPeers: number;
  communityPeers: number;
  institutionalPeers: number;
  totalCircles: number; // How many distinct trust levels contribute
}

// =============================================================================
// CONTENT RISK BUCKETS
// =============================================================================

/**
 * ContentRiskBucket — groups content by appropriate protection level.
 *
 * Not all data needs the same distribution. Medical records at personal reach
 * need institutional attestation; a shared movie at community reach just needs
 * a friendly peer. The adequacy score reflects whether protection matches need.
 */
export interface ContentRiskBucket {
  reach: ReachLevel;
  contentCount: number;
  shardDistribution: number; // Avg distinct peers per content
  adequacy: number; // 0-1, distribution appropriate FOR this reach
  exemplar?: string; // Human-readable: "medical records", "shared media"
}

// =============================================================================
// RESILIENCE ACTIONS
// =============================================================================

/**
 * ResilienceActionType — categories of improvement.
 *
 * - connect: make new mutual aid connections
 * - diversify: spread shards across more trust circles / jurisdictions
 * - renew: renew expiring commitments
 * - review: assess whether current protection matches content sensitivity
 * - release: let go of data that no longer needs protecting
 */
export type ResilienceActionType = 'connect' | 'diversify' | 'renew' | 'review' | 'release';

/**
 * ResilienceAction — the single most impactful thing a human can do.
 */
export interface ResilienceAction {
  type: ResilienceActionType;
  description: string; // Human-readable: "Connect with a community custodian"
  suggestedPeerIds?: string[];
  urgency: 'whenever' | 'soon' | 'now';
}

// =============================================================================
// ELOHIM ASSESSMENT (DISCERNMENT LAYER)
// =============================================================================

/**
 * ElohimResilienceAssessment — the discernment layer.
 *
 * The protocol computes the mechanical score. Elohim assess whether it's
 * *adequate* given what the data is and where it lives. Elohim can also help
 * set the score at higher abstract levels requiring discernment —
 * institutional/political risk indicators, boundary-based attestations.
 *
 * Elohim are LLMs — they need narrative memory alongside quantitative scores.
 * A concern isn't just a flag; it's a remembered observation that might matter
 * later: "Timothy mentioned he's moving countries — his jurisdictional diversity
 * is about to change."
 */
export interface ElohimResilienceAssessment {
  assessedAt: string;
  assessedBy: AgentRef;
  overallAdequacy: number; // Elohim's judgment of the mechanical score

  // Narrative memory — contextual understanding
  narrative: string; // Current assessment in natural language
  memories: ResilienceMemory[];

  concerns: ResilienceConcern[];
  attestations: string[]; // EPR refs to boundary/institutional attestations
  constitutionalBasis?: string;
}

/**
 * ResilienceMemory — accumulated context across assessments.
 *
 * Memories persist, get updated, and can be resolved. They give the elohim
 * continuity of care — not just a snapshot score.
 *
 * Memories also degrade. Data shouldn't last forever unless it has a purpose.
 * The 'resolved' state and supersededBy chain support graceful forgetting.
 */
export interface ResilienceMemory {
  id: string;
  recordedAt: string;
  updatedAt: string;
  content: string; // Free text — the elohim's observation
  relevance: 'active' | 'background' | 'resolved';
  relatedContentIds?: string[];
  relatedHumanIds?: string[];
  supersededBy?: string; // Memory ID if updated/corrected
}

/**
 * ResilienceConcern — a specific issue the elohim has identified.
 */
export interface ResilienceConcern {
  severity: 'informational' | 'concerning' | 'critical';
  description: string; // Natural language
  affectedContentIds?: string[];
  suggestedAction?: string;
}
```

**Step 2: Wire into shefa models barrel**

In `app/elohim-app/src/app/shefa/models/index.ts`, add after the resource-explorer export:

```typescript
// Resilience profile (P2P data protection projection)
export * from './resilience-profile.model';
```

**Step 3: Run type check**

Run: `cd app/elohim-app && pnpm exec tsc --noEmit --project tsconfig.json 2>&1 | head -20`

Expected: No NEW errors from resilience-profile.model.ts (existing errors are pre-existing — see MEMORY.md)

**Step 4: Commit**

```bash
git add app/elohim-app/src/app/shefa/models/resilience-profile.model.ts app/elohim-app/src/app/shefa/models/index.ts
git commit -m "feat(shefa): add ResilienceProfile model for P2P data protection projection"
```

---

### Task 2: IResilience Interface + InjectionToken

Create the abstract interface and injection token for resilience computation, following the existing shefa pattern (see `data-protection.interface.ts` for the exact pattern).

**Files:**
- Create: `app/elohim-app/src/app/shefa/interfaces/resilience.interface.ts`
- Modify: `app/elohim-app/src/app/shefa/interfaces/index.ts`

**Context:**
- Every shefa service has: interface file with types + `InjectionToken` → API service implements interface → consumers inject token
- See `data-protection.interface.ts` for the canonical example
- The interface defines what the resilience service can do; the implementation (Task 3) provides the computation

**Step 1: Create the interface file**

```typescript
/**
 * IResilience — Abstract interface for human resilience profile computation.
 *
 * Composes existing protocol primitives (shard manifests, mutual aid contexts,
 * custodian commitments, trust topology) into a ResilienceProfile projection.
 *
 * Consumers inject the RESILIENCE token; the default factory resolves to
 * ResilienceApiService.
 *
 * @example
 * ```typescript
 * @Injectable({ providedIn: 'root' })
 * export class ProtectionIndicator {
 *   private readonly resilience = inject(RESILIENCE);
 *
 *   profile$ = this.resilience.getProfile$('human-matthew-manager');
 * }
 * ```
 */

import { InjectionToken, inject } from '@angular/core';

import type { Observable } from 'rxjs';

import type {
  ResilienceProfile,
  ProtectionStatus,
  ResilienceAction,
  ContentRiskBucket,
  ElohimResilienceAssessment,
} from '../models/resilience-profile.model';

// =============================================================================
// INTERFACE
// =============================================================================

/**
 * Abstract resilience manager — computes and monitors human data protection
 * status from protocol primitives.
 */
export interface IResilience {
  /**
   * Compute a resilience profile for a human.
   *
   * Assembles shard health, commitment health, trust circle depth, and
   * content risk breakdown from existing protocol data.
   *
   * @param humanId - Agent public key or genesis human ID
   * @returns Promise resolving to computed profile
   */
  computeProfile(humanId: string): Promise<ResilienceProfile>;

  /**
   * Get a live-updating resilience profile stream.
   *
   * Recomputes on the given interval, reflecting changes as peers come/go,
   * commitments expire, and shards redistribute.
   *
   * @param humanId - Agent public key or genesis human ID
   * @param refreshInterval - Milliseconds between recomputation (default 60000)
   * @returns Observable stream of profile updates
   */
  getProfile$(humanId: string, refreshInterval?: number): Observable<ResilienceProfile>;

  /**
   * Get current cached profile snapshot.
   *
   * @returns Current profile or null if not yet computed
   */
  getProfile(): ResilienceProfile | null;

  /**
   * Get the current protection status.
   *
   * @returns Status or null if not yet computed
   */
  getProtectionStatus(): ProtectionStatus | null;

  /**
   * Get the most impactful next action to improve resilience.
   *
   * @returns Action or null if fully protected
   */
  getNextAction(): ResilienceAction | null;

  /**
   * Get content risk breakdown by reach level.
   *
   * @returns Array of content risk buckets, or empty if not yet computed
   */
  getContentRiskBreakdown(): ContentRiskBucket[];

  /**
   * Get the most recent elohim assessment.
   *
   * @returns Assessment or null if elohim hasn't assessed yet
   */
  getElohimAssessment(): ElohimResilienceAssessment | null;
}

// =============================================================================
// INJECTION TOKEN
// =============================================================================

/**
 * Injection token for resilience profile computation.
 *
 * Default factory resolves to ResilienceApiService which provides:
 * - Profile computation from shard + commitment + trust primitives
 * - Live-updating Observable stream
 * - Per-content risk breakdown by reach level
 * - Elohim assessment with narrative memory
 *
 * Override in tests:
 * ```typescript
 * { provide: RESILIENCE, useValue: mockResilience }
 * ```
 */
export const RESILIENCE = new InjectionToken<IResilience>('Resilience', {
  providedIn: 'root',
  factory: () => {
    // Lazy import to avoid circular dependency
    const { ResilienceApiService } = require('../services/resilience-api.service');
    return inject(ResilienceApiService);
  },
});
```

**Step 2: Wire into interfaces barrel**

In `app/elohim-app/src/app/shefa/interfaces/index.ts`, add at the end:

```typescript
export type {
  IResilience,
} from './resilience.interface';
export { RESILIENCE } from './resilience.interface';
```

**Step 3: Run type check**

Run: `cd app/elohim-app && pnpm exec tsc --noEmit --project tsconfig.json 2>&1 | grep -i resilience`

Expected: No errors from resilience files (the lazy `require` for the not-yet-created service is fine at type level)

**Step 4: Commit**

```bash
git add app/elohim-app/src/app/shefa/interfaces/resilience.interface.ts app/elohim-app/src/app/shefa/interfaces/index.ts
git commit -m "feat(shefa): add IResilience interface and RESILIENCE injection token"
```

---

### Task 3: ResilienceApiService — Thin HTTP Client

Create the service that implements IResilience, following the exact pattern of `DataProtectionApiService`.

**Files:**
- Create: `app/elohim-app/src/app/shefa/services/resilience-api.service.ts`
- Create: `app/elohim-app/src/app/shefa/services/resilience-api.service.spec.ts`
- Modify: `app/elohim-app/src/app/shefa/services/index.ts`

**Context:**
- Follow `data-protection-api.service.ts` exactly: HttpClient, BehaviorSubject cache, timer polling
- The service reads from doorway projection endpoints (not yet implemented in Rust — that's future work)
- The spec follows `data-protection-api.service.spec.ts`: TestBed + HttpTestingController
- All Angular DI uses `inject()` function, NOT constructor injection (see MEMORY.md — esbuild strips constructor metadata)

**Step 1: Write the spec file**

```typescript
import { TestBed } from '@angular/core/testing';
import {
  HttpTestingController,
  provideHttpClientTesting,
} from '@angular/common/http/testing';
import { provideHttpClient } from '@angular/common/http';

import { ResilienceApiService } from './resilience-api.service';
import type { ResilienceProfile } from '../models/resilience-profile.model';

describe('ResilienceApiService', () => {
  let service: ResilienceApiService;
  let httpMock: HttpTestingController;

  const stubProfile: ResilienceProfile = {
    humanId: 'human-matthew-manager',
    overallScore: 0.65,
    protectionStatus: 'partial',
    shardHealth: {
      totalBlobs: 42,
      totalShards: 120,
      distinctPeers: 3,
      averageShardsPerBlob: 2.86,
      encodingBreakdown: { single: 10, chunked: 12, reedSolomon: 20 },
      singlePointOfFailureCount: 10,
      lastAccessVerifiedAt: '2026-03-11T10:00:00Z',
    },
    commitmentHealth: {
      activeCommitments: 2,
      reciprocatedCommitments: 1,
      expiringSoon: 0,
      totalPeersCommitted: 2,
      commitmentCoverage: 0.6,
    },
    trustCircleDepth: {
      householdPeers: 1,
      friendPeers: 0,
      communityPeers: 1,
      institutionalPeers: 0,
      totalCircles: 2,
    },
    contentRiskBreakdown: [
      {
        reach: 'personal',
        contentCount: 5,
        shardDistribution: 1,
        adequacy: 0.4,
        exemplar: 'medical records',
      },
      {
        reach: 'community',
        contentCount: 30,
        shardDistribution: 3,
        adequacy: 0.8,
        exemplar: 'faith community content',
      },
    ],
    nextAction: {
      type: 'connect',
      description: 'Connect with a friend or community peer to diversify personal-reach backup',
      urgency: 'soon',
    },
    lastComputedAt: '2026-03-11T10:00:00Z',
  };

  async function fetchAndFlush(
    humanId = 'human-matthew-manager',
    data: ResilienceProfile = stubProfile
  ): Promise<ResilienceProfile> {
    const promise = service.computeProfile(humanId);
    httpMock
      .expectOne(`/api/v1/resilience/${humanId}/profile`)
      .flush(data);
    return promise;
  }

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [
        ResilienceApiService,
        provideHttpClient(),
        provideHttpClientTesting(),
      ],
    });
    service = TestBed.inject(ResilienceApiService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => httpMock.verify());

  it('computeProfile calls GET profile endpoint', async () => {
    const promise = service.computeProfile('human-matthew-manager');
    const req = httpMock.expectOne('/api/v1/resilience/human-matthew-manager/profile');
    expect(req.request.method).toBe('GET');
    req.flush(stubProfile);
    expect(await promise).toEqual(stubProfile);
  });

  it('getProfile returns null before computation', () => {
    expect(service.getProfile()).toBeNull();
  });

  it('getProfile returns cached profile after computation', async () => {
    await fetchAndFlush();
    expect(service.getProfile()).toEqual(stubProfile);
  });

  it('getProtectionStatus returns status from cached profile', async () => {
    expect(service.getProtectionStatus()).toBeNull();
    await fetchAndFlush();
    expect(service.getProtectionStatus()).toBe('partial');
  });

  it('getNextAction returns action from cached profile', async () => {
    expect(service.getNextAction()).toBeNull();
    await fetchAndFlush();
    expect(service.getNextAction()?.type).toBe('connect');
  });

  it('getContentRiskBreakdown returns buckets from cached profile', async () => {
    expect(service.getContentRiskBreakdown()).toEqual([]);
    await fetchAndFlush();
    expect(service.getContentRiskBreakdown().length).toBe(2);
    expect(service.getContentRiskBreakdown()[0].reach).toBe('personal');
  });

  it('getElohimAssessment returns null when no assessment', async () => {
    await fetchAndFlush();
    expect(service.getElohimAssessment()).toBeNull();
  });

  it('getElohimAssessment returns assessment when present', async () => {
    const withAssessment: ResilienceProfile = {
      ...stubProfile,
      elohimAssessment: {
        assessedAt: '2026-03-11T10:00:00Z',
        assessedBy: { id: 'elohim-1', name: 'Guardian', type: 'elohim' },
        overallAdequacy: 0.5,
        narrative: 'Personal-reach data is under-protected for its sensitivity.',
        memories: [
          {
            id: 'mem-1',
            recordedAt: '2026-03-10T10:00:00Z',
            updatedAt: '2026-03-10T10:00:00Z',
            content: 'Both conductors are in the same household.',
            relevance: 'active',
          },
        ],
        concerns: [
          {
            severity: 'concerning',
            description: 'Medical records on household peers only — single infrastructure failure risk.',
            suggestedAction: 'Diversify personal-reach backup to a trusted friend.',
          },
        ],
        attestations: [],
      },
    };
    await fetchAndFlush('human-matthew-manager', withAssessment);
    expect(service.getElohimAssessment()?.overallAdequacy).toBe(0.5);
    expect(service.getElohimAssessment()?.memories.length).toBe(1);
  });
});
```

**Step 2: Run the spec to verify it fails**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "resilience-api" 2>&1 | tail -10`

Expected: FAIL — `Cannot find module './resilience-api.service'`

**Step 3: Write the service implementation**

```typescript
/**
 * ResilienceApiService — Thin HTTP client for resilience profile computation.
 *
 * Calls doorway `/api/v1/resilience/*` endpoints, implementing IResilience.
 * Maintains a local cache of the profile for synchronous accessor methods.
 */

import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import {
  BehaviorSubject,
  Observable,
  Subscription,
  firstValueFrom,
  switchMap,
  tap,
  timer,
} from 'rxjs';

import type {
  ResilienceProfile,
  ProtectionStatus,
  ResilienceAction,
  ContentRiskBucket,
  ElohimResilienceAssessment,
} from '../models/resilience-profile.model';
import type { IResilience } from '../interfaces/resilience.interface';

@Injectable({ providedIn: 'root' })
export class ResilienceApiService implements IResilience {
  private readonly http = inject(HttpClient);
  private readonly profile$ = new BehaviorSubject<ResilienceProfile | null>(null);
  private pollingSubscription: Subscription | null = null;

  async computeProfile(humanId: string): Promise<ResilienceProfile> {
    const profile = await firstValueFrom(
      this.http.get<ResilienceProfile>(`/api/v1/resilience/${humanId}/profile`)
    );
    this.profile$.next(profile);
    return profile;
  }

  getProfile$(humanId: string, refreshInterval = 60000): Observable<ResilienceProfile> {
    this.pollingSubscription?.unsubscribe();

    const poll$ = timer(0, refreshInterval).pipe(
      switchMap(() =>
        this.http.get<ResilienceProfile>(`/api/v1/resilience/${humanId}/profile`)
      ),
      tap((profile) => this.profile$.next(profile))
    );

    this.pollingSubscription = poll$.subscribe();

    return poll$;
  }

  getProfile(): ResilienceProfile | null {
    return this.profile$.getValue();
  }

  getProtectionStatus(): ProtectionStatus | null {
    return this.profile$.getValue()?.protectionStatus ?? null;
  }

  getNextAction(): ResilienceAction | null {
    return this.profile$.getValue()?.nextAction ?? null;
  }

  getContentRiskBreakdown(): ContentRiskBucket[] {
    return this.profile$.getValue()?.contentRiskBreakdown ?? [];
  }

  getElohimAssessment(): ElohimResilienceAssessment | null {
    return this.profile$.getValue()?.elohimAssessment ?? null;
  }
}
```

**Step 4: Run the spec to verify it passes**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "resilience-api" 2>&1 | tail -15`

Expected: All 8 tests PASS

**Step 5: Wire into services barrel**

In `app/elohim-app/src/app/shefa/services/index.ts`, add after the flow-planning export:

```typescript
// Resilience API (P2P data protection profile)
export { ResilienceApiService } from './resilience-api.service';
```

**Step 6: Update the RESILIENCE token factory**

The lazy `require` in the interface file needs to be replaced with a proper import now that the service exists. In `app/elohim-app/src/app/shefa/interfaces/resilience.interface.ts`, replace the factory:

```typescript
import { ResilienceApiService } from '../services/resilience-api.service';

// ... (in the InjectionToken definition)
export const RESILIENCE = new InjectionToken<IResilience>('Resilience', {
  providedIn: 'root',
  factory: () => inject(ResilienceApiService),
});
```

And add the import at the top of the file (alongside the existing `@angular/core` imports):

```typescript
import { InjectionToken, inject } from '@angular/core';
import { ResilienceApiService } from '../services/resilience-api.service';
```

**Step 7: Run tests again after wiring**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "resilience-api" 2>&1 | tail -10`

Expected: All 8 tests still PASS

**Step 8: Commit**

```bash
git add app/elohim-app/src/app/shefa/services/resilience-api.service.ts app/elohim-app/src/app/shefa/services/resilience-api.service.spec.ts app/elohim-app/src/app/shefa/services/index.ts app/elohim-app/src/app/shefa/interfaces/resilience.interface.ts
git commit -m "feat(shefa): add ResilienceApiService with IResilience interface and tests"
```

---

### Task 4: Compile-Time Type Verification

Create a `.typetest.ts` file that exercises all discriminated unions and type compositions at compile time, following the pattern from `coordination-envelope.model.typetest.ts`.

**Files:**
- Create: `app/elohim-app/src/app/shefa/models/resilience-profile.model.typetest.ts`

**Context:**
- This file is NOT a runtime test — the `.typetest.ts` naming excludes it from Vitest
- It verifies that types compose correctly: ResilienceProfile uses ReachLevel from protocol-core, AgentRef from coordination-envelope, etc.
- If this file compiles, the type design is sound

**Step 1: Create the type test file**

```typescript
/**
 * Compile-time type verification for ResilienceProfile model.
 *
 * This file is NOT a runtime test. It exercises type composition to catch
 * structural errors at compile time. If it compiles, the types are sound.
 */

import type { ReachLevel } from '@app/elohim/models/protocol-core.model';
import type { AgentRef } from '@app/elohim/models/coordination-envelope.model';
import type {
  ProtectionStatus,
  ResilienceProfile,
  ShardHealthSummary,
  CommitmentHealthSummary,
  TrustCircleDepth,
  ContentRiskBucket,
  ResilienceAction,
  ResilienceActionType,
  ElohimResilienceAssessment,
  ResilienceMemory,
  ResilienceConcern,
} from './resilience-profile.model';

// --- ProtectionStatus exhaustiveness ---
function handleStatus(s: ProtectionStatus): string {
  switch (s) {
    case 'at-risk': return 'danger';
    case 'partial': return 'warning';
    case 'protected': return 'safe';
  }
}

// --- ResilienceActionType exhaustiveness ---
function handleAction(t: ResilienceActionType): string {
  switch (t) {
    case 'connect': return 'find peers';
    case 'diversify': return 'spread shards';
    case 'renew': return 'extend commitments';
    case 'review': return 'assess adequacy';
    case 'release': return 'let go gracefully';
  }
}

// --- ContentRiskBucket uses ReachLevel from protocol-core ---
const bucket: ContentRiskBucket = {
  reach: 'personal' satisfies ReachLevel,
  contentCount: 5,
  shardDistribution: 1,
  adequacy: 0.4,
  exemplar: 'medical records',
};

// --- ElohimResilienceAssessment uses AgentRef from coordination-envelope ---
const agent: AgentRef = { id: 'elohim-1', name: 'Guardian', type: 'elohim' };
const assessment: ElohimResilienceAssessment = {
  assessedAt: '2026-03-11T10:00:00Z',
  assessedBy: agent,
  overallAdequacy: 0.5,
  narrative: 'Under-protected for sensitivity level.',
  memories: [{
    id: 'mem-1',
    recordedAt: '2026-03-11T10:00:00Z',
    updatedAt: '2026-03-11T10:00:00Z',
    content: 'Both nodes in same household.',
    relevance: 'active',
  }],
  concerns: [{
    severity: 'concerning',
    description: 'Medical records household-only.',
  }],
  attestations: ['epr:attestation-jurisdiction-diversity'],
};

// --- Full profile composition ---
const profile: ResilienceProfile = {
  humanId: 'human-matthew-manager',
  overallScore: 0.65,
  protectionStatus: 'partial',
  shardHealth: {
    totalBlobs: 42,
    totalShards: 120,
    distinctPeers: 3,
    averageShardsPerBlob: 2.86,
    encodingBreakdown: { single: 10, chunked: 12, reedSolomon: 20 },
    singlePointOfFailureCount: 10,
    lastAccessVerifiedAt: '2026-03-11T10:00:00Z',
  },
  commitmentHealth: {
    activeCommitments: 2,
    reciprocatedCommitments: 1,
    expiringSoon: 0,
    totalPeersCommitted: 2,
    commitmentCoverage: 0.6,
  },
  trustCircleDepth: {
    householdPeers: 1,
    friendPeers: 0,
    communityPeers: 1,
    institutionalPeers: 0,
    totalCircles: 2,
  },
  contentRiskBreakdown: [bucket],
  nextAction: {
    type: 'connect',
    description: 'Connect with a community custodian.',
    urgency: 'soon',
  },
  elohimAssessment: assessment,
  lastComputedAt: '2026-03-11T10:00:00Z',
};

// --- Memory lifecycle ---
const resolvedMemory: ResilienceMemory = {
  id: 'mem-2',
  recordedAt: '2026-03-01T10:00:00Z',
  updatedAt: '2026-03-11T10:00:00Z',
  content: 'Timothy concentrated in one region. Resolved via learning community mutual aid.',
  relevance: 'resolved',
  relatedHumanIds: ['human-timothy-learner'],
  supersededBy: 'mem-3',
};

// --- Graceful release action ---
const releaseAction: ResilienceAction = {
  type: 'release',
  description: '47 expired community announcements still replicated. Release them?',
  urgency: 'whenever',
};

// Suppress unused warnings
void handleStatus;
void handleAction;
void profile;
void resolvedMemory;
void releaseAction;
```

**Step 2: Verify it compiles**

Run: `cd app/elohim-app && pnpm exec tsc --noEmit --project tsconfig.json 2>&1 | grep -c "resilience-profile.model.typetest"`

Expected: 0 (no errors from this file)

**Step 3: Commit**

```bash
git add app/elohim-app/src/app/shefa/models/resilience-profile.model.typetest.ts
git commit -m "test(shefa): add compile-time type verification for ResilienceProfile"
```

---

### Task 5: A2O Scenarios — Graduated Resilience Through Genesis Humans

Create the acceptance scenarios that drive the resilience projection through our genesis human trust topology.

**Files:**
- Create: `genesis/a2o/features/shefa/human-resilience.feature`

**Context:**
- Follow existing feature file conventions from `genesis/a2o/CLAUDE.md`
- Tags: `@e2e @shefa` on first line; `@wip` on all scenarios (step defs not yet implemented)
- Background: `Given doorway "alpha" at "E2E_DOORWAY_ALPHA"`
- Uses named personas from genesis humans
- Graduated through trust topology: Matthew → +Susan → +Pete → +Timothy+Frank → Maria cold start → degradation → graceful release
- These scenarios match the design doc stories exactly

**Step 1: Create the feature file**

```gherkin
@e2e @shefa @resilience
Feature: Human Resilience — P2P Data Protection at a Glance
  As a human in the Elohim Protocol,
  I want to know at a glance whether my data is protected
  so that I can act when I need to and rest when I don't.

  The resilience profile is a shefa projection — computed from existing
  protocol primitives (shard manifests, mutual aid contexts, custodian
  commitments, trust topology). It answers: "Am I safe? And if not,
  what should I do?"

  Protection is per-content. Medical records need institutional attestation.
  A shared movie just needs a friendly peer. Attestation happens through
  use, not ceremony — every shard fetch is a heartbeat.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  # ─── Graduated Resilience Through Trust Topology ─────────────────────────

  @wip @scaling
  Scenario: Matthew alone — single conductor, at risk
    Given human "Matthew" is logged in on doorway "alpha" with device
    And Matthew has 1 conductor with all his stewardship content
    And no mutual aid commitments exist
    When Matthew views his resilience profile
    Then his protection status is "at-risk"
    And his shard health shows 0 distinct peers beyond self
    And his trust circle depth shows 0 total circles
    And his next action is to "connect" with urgency "now"

  @wip @scaling
  Scenario: Matthew + Susan — household reciprocation, partial
    Given human "Matthew" is logged in on doorway "alpha" with device
    And human "Susan" has a conductor in the same household
    And household content replicates via spouse relationship
    And a mutual aid commitment exists between Matthew and Susan
    When Matthew views his resilience profile
    Then his protection status is "partial"
    And his shard health shows 1 distinct peer beyond self
    And his trust circle depth shows 1 total circle
    And his commitment health shows 1 active commitment
    And the elohim assessment narrative mentions "same household"
    And the elohim has a memory about infrastructure concentration risk

  @wip @scaling
  Scenario: Matthew + Susan + Pastor Pete — community depth
    Given human "Matthew" is logged in on doorway "alpha" with device
    And human "Susan" has a conductor in the same household
    And human "Pastor Pete" has a conductor at the congregation
    And community-reach content replicates to Pete via congregation relationship
    When Matthew views his resilience profile
    Then his protection status is "partial"
    And his trust circle depth shows 2 total circles
    And his content risk breakdown shows:
      | reach     | adequacy | exemplar                |
      | personal  | high     | medical records         |
      | community | high     | faith community content |
    And the elohim assessment confirms personal-reach content is appropriately household-only

  @wip @scaling
  Scenario: Full network — Matthew + Susan + Pete + Timothy + Frank
    Given 5 conductors are running for Matthew, Susan, Pete, Timothy, and Frank
    And content is distributed by stewardship affinity
    And mutual aid commitments are reciprocated across trust circles
    When Matthew views his resilience profile
    Then his protection status is "protected"
    And his trust circle depth shows at least 3 total circles
    And his commitment health shows at least 3 reciprocated commitments
    And no content risk bucket has adequacy below 0.7
    And the elohim assessment overall adequacy is above 0.8

  # ─── Cold Start ──────────────────────────────────────────────────────────

  @wip
  Scenario: Maria — cold start with zero peers
    Given human "Maria" is logged in on doorway "alpha" with device
    And Maria has no existing peer connections
    And no mutual aid commitments exist for Maria
    When Maria views her resilience profile
    Then her protection status is "at-risk"
    And her shard health shows 0 distinct peers
    And her next action is to "connect" with urgency "now"
    And the next action suggests peers from her potential trust circles

  @wip
  Scenario: Maria builds resilience through connection
    Given human "Maria" is logged in on doorway "alpha" with device
    And Maria has connected with Susan through the learning community
    And a mutual aid commitment exists between Maria and Susan
    When Maria views her resilience profile
    Then her protection status is "partial"
    And her commitment health shows 1 active commitment
    And the elohim has a memory about Maria's first mutual aid connection

  # ─── Degradation ─────────────────────────────────────────────────────────

  @wip
  Scenario: Degradation — Matthew's conductor goes offline
    Given human "Susan" is logged in on doorway "alpha" with device
    And Susan's resilience was previously "protected"
    And Matthew's conductor has gone offline
    When Susan views her resilience profile
    Then her protection status is "partial"
    And the elohim assessment narrative mentions conductor offline
    And a mutual aid context has been activated in emergency mode
    And the next action is to "diversify" with urgency "soon"

  @wip
  Scenario: Recovery — after-action review when Matthew comes back
    Given human "Susan" is logged in on doorway "alpha" with device
    And Matthew's conductor has come back online after an outage
    And the emergency mutual aid context has been closed
    When Susan views her resilience profile
    Then her protection status is "protected"
    And the elohim has a memory about the outage with relevance "resolved"
    And the elohim assessment suggests diversifying personal-reach backup

  # ─── Graceful Release ───────────────────────────────────────────────────

  @wip
  Scenario: The right to be forgotten — releasing content that no longer serves
    Given human "Matthew" is logged in on doorway "alpha" with device
    And Matthew has content that has expired or been superseded
    When Matthew views his resilience profile
    Then the next action type is "release"
    And the release action describes content that no longer needs protecting
    And releasing the content updates the profile accordingly

  # ─── Per-Content Sensitivity ─────────────────────────────────────────────

  @wip
  Scenario: Medical data needs institutional attestation, media does not
    Given human "Matthew" is logged in on doorway "alpha" with device
    And Matthew has personal-reach medical records
    And Matthew has community-reach shared media
    When Matthew views his content risk breakdown
    Then the personal-reach bucket references "medical records"
    And the community-reach bucket references "shared media"
    And the elohim assessment distinguishes protection adequacy by content type
    And medical records with only household peers shows lower adequacy
    And shared media with community peers shows higher adequacy
```

**Step 2: Verify the feature file parses**

Run: `cd genesis/a2o && npx gherkin-lint features/shefa/human-resilience.feature 2>&1 || echo "gherkin parsed (lint may not be configured)"`

Expected: File parses without syntax errors (lint rules may flag @wip but that's expected)

**Step 3: Commit**

```bash
git add genesis/a2o/features/shefa/human-resilience.feature
git commit -m "feat(a2o): add graduated resilience scenarios through genesis humans"
```

---

### Task 6: Storybook Icon Design Story Skeleton (Story-Only)

Create a Storybook MDX story that captures the icon design brief — no component implementation, just the design exploration narrative and placeholder for future visual work.

**Files:**
- Create: `app/elohim-library/projects/lamad-ui/src/lib/components/resilience-indicator/__docs__/resilience-indicator.stories.mdx`

**Context:**
- Storybook config picks up `../projects/**/__docs__/**/*.@(stories.ts|mdx)` — MDX files are supported
- This is story-only: a design brief, not a component. No Angular component needed.
- Captures the icon design direction from brainstorming: wifi bars, chevrons, dense visual communication
- Storybook MDX docs: use `<Meta>` for title, prose for design narrative

**Step 1: Create the directory**

Run: `mkdir -p app/elohim-library/projects/lamad-ui/src/lib/components/resilience-indicator/__docs__`

**Step 2: Create the MDX story**

```mdx
import { Meta } from '@storybook/blocks';

<Meta title="Design/Resilience Indicator" />

# Resilience Indicator — Design Brief

> **Status:** Story-only skeleton. No component implementation yet.
> This captures the design direction for future visual work.

## What It Represents

The Resilience Indicator is a small, ambient icon that answers one question:
**"Am I safe? And if not, what should I do?"**

It's a visual projection of the `ResilienceProfile` — a shefa computation
that composes shard distribution, mutual aid commitments, trust circle depth,
and elohim assessment into a single at-a-glance signal.

## Design Principles

### Ambient When Protected
When everything is healthy, the icon fades to background — like full WiFi bars,
you stop noticing it. It should work at very small sizes (16x16, favicon, mobile
status bar) and never demand attention when there's nothing to do.

### Actionable When At Risk
When protection degrades, the icon should communicate:
1. **Something changed** (not alarming — a shift, not an alert)
2. **What to do about it** (connect, diversify, renew, review, release)
3. **How urgent it is** (whenever, soon, now)

### Dense Information in Tiny Space
A small visual element can carry layered meaning. Design inspiration:

- **WiFi bars** — graduated signal strength, universally understood, tiny footprint.
  Everyone knows what 2-of-4 bars means without a tooltip.

- **Military chevrons** — rank, branch, specialty, and time-in-service encoded in
  a few stripes on a sleeve. A sergeant's chevrons communicate years of context
  at a glance. The resilience icon should aspire to this density.

- **Protocol identity logos** — Hylo, Collaborative Technology Alliance — icons
  that embody a protocol's values while functioning as status indicators. The
  Elohim resilience indicator should feel like it belongs to this family.

### Continuous Gradient, Not Discrete States
The model has three named states (at-risk, partial, protected) but the underlying
score is continuous (0-1). The icon should reflect the gradient — partial protection
at 0.4 looks different from partial at 0.7.

## What the Icon Must Convey

| Dimension | At Risk | Partial | Protected |
|-----------|---------|---------|-----------|
| Overall safety | Vulnerable | Improving | Ambient |
| Trust circles | None | Some | Deep |
| Shard distribution | Concentrated | Spreading | Well-distributed |
| Commitments | Missing | Building | Reciprocated |

## Open Design Questions

1. **Aggregate vs. contextual** — does the icon represent overall resilience, or
   the resilience of the content you're currently viewing? (Both are computable
   from the model.)

2. **Degradation visual** — loss (bars disappearing) or change (color shift)?
   Loss implies permanent damage; color shift implies temporary state. The protocol
   supports recovery, so color shift may be more honest.

3. **Maslow graduation** — as base-level protection is satisfied, the icon could
   shift to represent higher-order concerns: community governance participation,
   stewardship reach, contribution to others' resilience. The same icon space,
   but the meaning graduates from "am I safe?" to "am I contributing to safety?"

4. **Graceful release** — how do you represent "you have data that no longer needs
   protecting"? This is not degradation — it's maturity. A visual language for
   intentional forgetting.

5. **The logo itself** — could the resilience indicator double as the Elohim
   Protocol's identity mark? A living logo that reflects the actual state of the
   human's participation in the network.

## Data Model (for reference)

The icon is powered by `ResilienceProfile` from `@app/shefa/models`:

```typescript
interface ResilienceProfile {
  overallScore: number;          // 0-1
  protectionStatus: ProtectionStatus;
  shardHealth: ShardHealthSummary;
  commitmentHealth: CommitmentHealthSummary;
  trustCircleDepth: TrustCircleDepth;
  contentRiskBreakdown: ContentRiskBucket[];
  nextAction?: ResilienceAction;
  elohimAssessment?: ElohimResilienceAssessment;
}
```

## Next Steps

1. Sketch 3-5 icon concepts exploring the wifi-bar/chevron/logo design space
2. Test at multiple sizes (16x16, 24x24, 32x32, 48x48)
3. Prototype the gradient animation (at-risk → partial → protected)
4. Explore the Maslow graduation (safety → contribution)
5. Build the Angular component backed by `RESILIENCE` injection token
```

**Step 3: Verify Storybook can find the file**

Run: `ls app/elohim-library/projects/lamad-ui/src/lib/components/resilience-indicator/__docs__/resilience-indicator.stories.mdx`

Expected: File exists (Storybook will pick it up via the glob pattern `../projects/**/__docs__/**/*.@(stories.ts|mdx)`)

**Step 4: Commit**

```bash
git add app/elohim-library/projects/lamad-ui/src/lib/components/resilience-indicator/__docs__/resilience-indicator.stories.mdx
git commit -m "docs(storybook): add resilience indicator design brief as story skeleton"
```

---

## Summary

| Task | What | Files | Tests |
|------|------|-------|-------|
| 1 | ResilienceProfile model | `resilience-profile.model.ts`, `index.ts` | Type check |
| 2 | IResilience interface + token | `resilience.interface.ts`, `index.ts` | Type check |
| 3 | ResilienceApiService + spec | `resilience-api.service.ts`, `.spec.ts`, `index.ts` | 8 Vitest specs |
| 4 | Compile-time type verification | `.typetest.ts` | Compiles = passes |
| 5 | A2O scenarios (11 scenarios) | `human-resilience.feature` | All @wip (step defs future) |
| 6 | Storybook icon design brief | `.stories.mdx` | Story-only, no component |

Total: 6 commits, incremental, each independently verifiable.
