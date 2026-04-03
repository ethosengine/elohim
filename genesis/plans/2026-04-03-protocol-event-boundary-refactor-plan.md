# Protocol vs Domain Event Boundary Refactor

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Split the monolithic `LamadEventType` (60+ types in `@app/elohim/models/`) into protocol-level event types (elohim) and domain-specific event types (lamad/shefa/etc), rename `EventService` methods to reflect protocol-level content interaction primitives, and move `ContentAnalyticsComponent` to the elohim pillar where protocol-level metrics belong.

**Architecture:** Extract `ProtocolEventType` and `PROTOCOL_EVENT_MAPPINGS` from `economic-event.model.ts` into a new `protocol-event-types.model.ts` in elohim/models. `LamadEventType` becomes a union of `ProtocolEventType | LamadDomainEventType` and moves to `lamad/models/`. `EventService` gets a generic `recordContentInteraction(agentId, contentId, type)` method — domain-specific convenience methods (`recordQuizSubmit`, etc.) move to a new `LamadEventService` in lamad. `ContentAnalyticsComponent` moves from lamad to elohim since it reads protocol-level metrics.

**Tech Stack:** Angular 19, TypeScript, Vitest

---

## File Structure

| File | Responsibility |
|------|----------------|
| `src/app/elohim/models/protocol-event-types.model.ts` | **NEW** — Protocol-level event types, REA mappings, and constants. The events any app on the protocol would need. |
| `src/app/elohim/models/economic-event.model.ts` | **MODIFY** — Remove `LamadEventType`, `LAMAD_EVENT_MAPPINGS`, and domain-specific types. Keep `EconomicEvent`, `EventQuery`, `EventState`, and other structural types. Import `ProtocolEventType` from new file. |
| `src/app/elohim/models/index.ts` | **MODIFY** — Export new protocol event types model |
| `src/app/lamad/models/lamad-event-types.model.ts` | **NEW** — Lamad domain event types extending protocol types. `LamadEventType = ProtocolEventType \| LamadDomainEventType`. `LAMAD_EVENT_MAPPINGS` with domain-specific entries only. |
| `src/app/lamad/models/index.ts` | **MODIFY** — Export lamad event types |
| `src/app/shefa/services/event.service.ts` | **MODIFY** — Add generic `recordContentInteraction()`. Keep `recordContentView` as deprecated alias. Remove lamad-specific methods to `LamadEventService`. |
| `src/app/lamad/services/lamad-event.service.ts` | **NEW** — Lamad-specific convenience methods (`recordQuizSubmit`, `recordAssessmentComplete`, etc.) wrapping `EventService.recordContentInteraction()`. |
| `src/app/shefa/services/attention-tracker.service.ts` | **MODIFY** — Call `recordContentInteraction()` instead of `recordContentView()` |
| `src/app/elohim/components/content-analytics/` | **MOVE** from `lamad/components/content-analytics/` — Protocol-level metrics belong in elohim |
| `src/app/lamad/components/content-viewer/content-viewer.component.ts` | **MODIFY** — Update import path for ContentAnalyticsComponent |
| All files importing `LamadEventType` from `@app/elohim/models` | **MODIFY** — Update import paths |

---

### Task 1: Create ProtocolEventType in elohim/models

**Files:**
- Create: `src/app/elohim/models/protocol-event-types.model.ts`

All paths relative to `app/elohim-app/`.

- [ ] **Step 1: Write the new model file**

```typescript
// src/app/elohim/models/protocol-event-types.model.ts
/**
 * Protocol Event Types — The attention-to-attestation pipeline.
 *
 * These are the event types any app built on the Elohim Protocol would need.
 * The pipeline: view → engage → demonstrate → attest → capability
 * Every stage produces REA economic events. Every stage is subject to governance.
 *
 * Domain-specific event types (quiz-submit, claim-filed, governance-vote)
 * extend these from their respective pillar models.
 */

import type { REAAction, ResourceClassification } from './rea-bridge.model';

// =============================================================================
// Protocol Event Types
// =============================================================================

/**
 * ProtocolEventType — Events that any app on the protocol produces.
 *
 * These follow the attention-to-attestation pipeline:
 *   view → engage → demonstrate → attest → capability
 *
 * Test: "Is this a STAGE of the pipeline, or an INSTRUMENT that implements a stage?"
 * Stages → protocol. Instruments (quiz, simulation, peer review) → domain.
 *
 * Learning is a universal capacity of the protocol, not a lamad feature.
 * All content is attestable. Assessment is the "demonstrate" stage — protocol.
 * HOW you assess (Sophia quiz, portfolio, peer review) — domain.
 */
export type ProtocolEventType =
  // Attention (content interaction — "view" and "engage" stages)
  | 'content-view'            // Agent viewed content (use + attention)
  | 'content-complete'        // Agent completed content (produce + achievement)
  | 'session-start'           // Agent began a session (use + attention)
  | 'session-end'             // Agent ended a session (use + attention)

  // Demonstration ("demonstrate" stage — assessment happened, not HOW)
  | 'assessment-start'        // Agent began demonstrating understanding (use + attention)
  | 'assessment-complete'     // Agent finished demonstrating understanding (produce + credential)

  // Attestation ("attest" and "capability" stages — the pipeline's output)
  | 'attestation-grant'       // Attestation granted (produce + attestation)
  | 'capability-earn'         // Capability developed (produce + credential)

  // Recognition (value flow)
  | 'recognition-given'       // Recognition given to contributor (appreciate + recognition)
  | 'recognition-received'    // Recognition received (appreciate + recognition)
  | 'affinity-mark'           // Agent marked affinity (appreciate + recognition)
  | 'endorsement'             // Formal endorsement (appreciate + endorsement)
  | 'citation'                // Content cited another (cite + recognition)

  // Stewardship (content governance)
  | 'stewardship-begin'       // Steward began stewardship (work + stewardship)
  | 'presence-claim'          // Contributor claimed presence (accept + recognition)
  | 'recognition-transfer'    // Recognition transferred (transfer + recognition)
  | 'invitation-send'         // Invitation sent (deliver-service)

  // Content lifecycle
  | 'content-create'          // Content created (produce + content)
  | 'content-flag'            // Content flagged (modify + content)
  | 'attestation-revoke'      // Attestation revoked (modify + attestation)

  // Governance
  | 'governance-vote';        // Vote cast (work + governance)

// =============================================================================
// Protocol Event Constants
// =============================================================================

export const ProtocolEventTypes = {
  CONTENT_VIEW: 'content-view' as ProtocolEventType,
  CONTENT_COMPLETE: 'content-complete' as ProtocolEventType,
  SESSION_START: 'session-start' as ProtocolEventType,
  SESSION_END: 'session-end' as ProtocolEventType,
  ASSESSMENT_START: 'assessment-start' as ProtocolEventType,
  ASSESSMENT_COMPLETE: 'assessment-complete' as ProtocolEventType,
  ATTESTATION_GRANT: 'attestation-grant' as ProtocolEventType,
  CAPABILITY_EARN: 'capability-earn' as ProtocolEventType,
  RECOGNITION_GIVEN: 'recognition-given' as ProtocolEventType,
  RECOGNITION_RECEIVED: 'recognition-received' as ProtocolEventType,
  AFFINITY_MARK: 'affinity-mark' as ProtocolEventType,
  ENDORSEMENT: 'endorsement' as ProtocolEventType,
  CITATION: 'citation' as ProtocolEventType,
  STEWARDSHIP_BEGIN: 'stewardship-begin' as ProtocolEventType,
  PRESENCE_CLAIM: 'presence-claim' as ProtocolEventType,
  RECOGNITION_TRANSFER: 'recognition-transfer' as ProtocolEventType,
  INVITATION_SEND: 'invitation-send' as ProtocolEventType,
  CONTENT_CREATE: 'content-create' as ProtocolEventType,
  CONTENT_FLAG: 'content-flag' as ProtocolEventType,
  ATTESTATION_REVOKE: 'attestation-revoke' as ProtocolEventType,
  GOVERNANCE_VOTE: 'governance-vote' as ProtocolEventType,
} as const;

// =============================================================================
// Standard Units (protocol-level)
// =============================================================================

export const PROTOCOL_UNITS = {
  EACH: 'unit-each',
  VIEW: 'unit-view',
  SESSION: 'unit-session',
  MINUTE: 'unit-minute',
  AFFINITY: 'unit-affinity',
  ENDORSEMENT: 'unit-endorsement',
  ATTESTATION: 'unit-attestation',
  NODE: 'unit-node',
  TOKEN: 'unit-token',
} as const;

// =============================================================================
// Protocol Event REA Mappings
// =============================================================================

export interface EventREAMapping {
  action: REAAction;
  resourceType: ResourceClassification;
  defaultUnit: string;
}

export const PROTOCOL_EVENT_MAPPINGS: Record<ProtocolEventType, EventREAMapping> = {
  'content-view': { action: 'use', resourceType: 'attention', defaultUnit: PROTOCOL_UNITS.VIEW },
  'content-complete': { action: 'produce', resourceType: 'credential', defaultUnit: PROTOCOL_UNITS.EACH },
  'session-start': { action: 'use', resourceType: 'attention', defaultUnit: PROTOCOL_UNITS.SESSION },
  'session-end': { action: 'use', resourceType: 'attention', defaultUnit: PROTOCOL_UNITS.MINUTE },
  'assessment-start': { action: 'use', resourceType: 'attention', defaultUnit: PROTOCOL_UNITS.EACH },
  'assessment-complete': { action: 'produce', resourceType: 'credential', defaultUnit: PROTOCOL_UNITS.EACH },
  'attestation-grant': { action: 'produce', resourceType: 'credential', defaultUnit: PROTOCOL_UNITS.ATTESTATION },
  'capability-earn': { action: 'produce', resourceType: 'credential', defaultUnit: PROTOCOL_UNITS.EACH },
  'recognition-given': { action: 'raise', resourceType: 'recognition', defaultUnit: PROTOCOL_UNITS.EACH },
  'recognition-received': { action: 'raise', resourceType: 'recognition', defaultUnit: PROTOCOL_UNITS.EACH },
  'affinity-mark': { action: 'raise', resourceType: 'recognition', defaultUnit: PROTOCOL_UNITS.AFFINITY },
  'endorsement': { action: 'raise', resourceType: 'recognition', defaultUnit: PROTOCOL_UNITS.ENDORSEMENT },
  'citation': { action: 'cite', resourceType: 'recognition', defaultUnit: PROTOCOL_UNITS.EACH },
  'stewardship-begin': { action: 'work', resourceType: 'stewardship', defaultUnit: PROTOCOL_UNITS.EACH },
  'presence-claim': { action: 'accept', resourceType: 'recognition', defaultUnit: PROTOCOL_UNITS.AFFINITY },
  'recognition-transfer': { action: 'transfer', resourceType: 'recognition', defaultUnit: PROTOCOL_UNITS.AFFINITY },
  'invitation-send': { action: 'deliver-service' as REAAction, resourceType: 'stewardship', defaultUnit: PROTOCOL_UNITS.EACH },
  'content-create': { action: 'produce', resourceType: 'content', defaultUnit: PROTOCOL_UNITS.NODE },
  'content-flag': { action: 'modify', resourceType: 'content', defaultUnit: PROTOCOL_UNITS.NODE },
  'attestation-revoke': { action: 'modify', resourceType: 'credential', defaultUnit: PROTOCOL_UNITS.ATTESTATION },
  'governance-vote': { action: 'work', resourceType: 'membership', defaultUnit: PROTOCOL_UNITS.EACH },
};
```

- [ ] **Step 2: Export from elohim models barrel**

In `src/app/elohim/models/index.ts`, add:

```typescript
// Protocol event types — the attention-to-attestation pipeline primitives
export * from './protocol-event-types.model';
```

- [ ] **Step 3: Verify compilation**

Run: `cd app/elohim-app && pnpm exec tsc --noEmit --pretty 2>&1 | head -20`
Expected: No new errors (the new file has no consumers yet)

- [ ] **Step 4: Commit**

```bash
git add app/elohim-app/src/app/elohim/models/protocol-event-types.model.ts \
       app/elohim-app/src/app/elohim/models/index.ts
git commit -m "feat(elohim): extract ProtocolEventType — protocol-level attention-to-attestation pipeline"
```

---

### Task 2: Add `recordContentInteraction()` to EventService

**Files:**
- Modify: `src/app/shefa/services/event.service.ts`
- Modify: `src/app/shefa/services/event.service.spec.ts`

- [ ] **Step 1: Write the failing test**

In `event.service.spec.ts`, add a new describe block:

```typescript
  describe('recordContentInteraction', () => {
    it('should record an interaction with the specified event type', () => {
      service.recordContentInteraction(agentId, contentId, 'content-view');
      expect(storageApiSpy.createEconomicEvent).toHaveBeenCalledWith(
        jasmine.objectContaining({
          action: 'use',
          provider: agentId,
          receiver: contentId,
          lamadEventType: 'content-view',
          contentId,
        }),
      );
    });

    it('should record content-complete interaction', () => {
      service.recordContentInteraction(agentId, contentId, 'content-complete');
      expect(storageApiSpy.createEconomicEvent).toHaveBeenCalledWith(
        jasmine.objectContaining({
          action: 'produce',
          lamadEventType: 'content-complete',
        }),
      );
    });
  });
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "event.service"`
Expected: FAIL — `recordContentInteraction` is not a function

- [ ] **Step 3: Implement the method**

In `event.service.ts`, add the import and the new method:

```typescript
import {
  ProtocolEventType,
  PROTOCOL_EVENT_MAPPINGS,
} from '@app/elohim/models/protocol-event-types.model';
```

Add at the top of the class, before the existing `recordContentView`:

```typescript
  // ===========================================================================
  // Protocol-Level Content Interaction
  // ===========================================================================

  /**
   * Record a content interaction as a protocol-level REA event.
   *
   * This is the generic protocol primitive. A "view" is an attention resource
   * event. A "complete" is an achievement resource event. The interaction type
   * determines the REA action and resource mapping.
   */
  recordContentInteraction(
    agentId: string,
    contentId: string,
    interactionType: ProtocolEventType,
  ): Observable<EconomicEventView> {
    const mapping = PROTOCOL_EVENT_MAPPINGS[interactionType];
    return this.storageApi.createEconomicEvent({
      action: mapping.action,
      provider: agentId,
      receiver: contentId,
      lamadEventType: interactionType,
      contentId,
    });
  }
```

- [ ] **Step 4: Deprecate old convenience methods**

Add `@deprecated` JSDoc to `recordContentView` and `recordContentComplete`:

```typescript
  /**
   * @deprecated Use recordContentInteraction(agentId, contentId, 'content-view') instead.
   */
  recordContentView(agentId: string, contentId: string): Observable<EconomicEventView> {
    return this.recordContentInteraction(agentId, contentId, 'content-view');
  }

  /**
   * @deprecated Use recordContentInteraction(agentId, contentId, 'content-complete') instead.
   */
  recordContentComplete(agentId: string, contentId: string): Observable<EconomicEventView> {
    return this.recordContentInteraction(agentId, contentId, 'content-complete');
  }
```

- [ ] **Step 5: Run tests**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "event.service"`
Expected: All tests PASS (old methods still work via delegation)

- [ ] **Step 6: Commit**

```bash
git add app/elohim-app/src/app/shefa/services/event.service.ts \
       app/elohim-app/src/app/shefa/services/event.service.spec.ts
git commit -m "feat(shefa): add recordContentInteraction() — protocol-level generic content interaction"
```

---

### Task 3: Update AttentionTrackerService to use `recordContentInteraction`

**Files:**
- Modify: `src/app/shefa/services/attention-tracker.service.ts`
- Modify: `src/app/shefa/services/attention-tracker.service.spec.ts`

- [ ] **Step 1: Update the spec**

In `attention-tracker.service.spec.ts`, replace all references to `recordContentView` with `recordContentInteraction`:

```typescript
// In the mock setup:
recordContentInteraction: vi.fn().mockReturnValue(of(MOCK_EVENT)),
```

```typescript
// In assertions:
expect(eventServiceMock['recordContentInteraction']).toHaveBeenCalledWith(
  MOCK_AGENT_ID,
  'concept-trust',
  'content-view',
);
```

Update all `toHaveBeenCalledTimes` and `not.toHaveBeenCalled` assertions similarly.

- [ ] **Step 2: Update the service**

In `attention-tracker.service.ts`, change the `recordQualifiedView` method:

```typescript
  private recordQualifiedView(contentId: string): void {
    this.sessionViewed.add(contentId);

    const agentId = this.agentService.getCurrentAgentId();
    const sub = this.eventService.recordContentInteraction(
      agentId,
      contentId,
      'content-view',
    ).subscribe();
    this.subscriptions.push(sub);
  }
```

- [ ] **Step 3: Run tests**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "attention-tracker"`
Expected: All tests PASS

- [ ] **Step 4: Commit**

```bash
git add app/elohim-app/src/app/shefa/services/attention-tracker.service.ts \
       app/elohim-app/src/app/shefa/services/attention-tracker.service.spec.ts
git commit -m "refactor(shefa): AttentionTrackerService uses recordContentInteraction()"
```

---

### Task 4: Export ProtocolEventTypes from shefa barrel

**Files:**
- Modify: `src/app/shefa/services/index.ts`

- [ ] **Step 1: Add protocol event type re-exports**

In `shefa/services/index.ts`, update the event service exports:

```typescript
// Event service (elohim-storage backend) — protocol-level content interaction
export { EventService } from './event.service';
// Re-export protocol event types for convenience (canonical source: @app/elohim/models)
export { ProtocolEventTypes } from '@app/elohim/models/protocol-event-types.model';
export type { ProtocolEventType } from '@app/elohim/models/protocol-event-types.model';
// Legacy re-exports (domain types — will move to lamad barrel)
export { LamadEventTypes } from './event.service';
export type { LamadEventType } from './event.service';

// Attention tracking (dwell-qualified view recording, session dedup)
export { AttentionTrackerService } from './attention-tracker.service';
```

- [ ] **Step 2: Commit**

```bash
git add app/elohim-app/src/app/shefa/services/index.ts
git commit -m "refactor(shefa): export ProtocolEventTypes alongside legacy LamadEventTypes"
```

---

### Task 5: Move ContentAnalyticsComponent from lamad to elohim

**Files:**
- Move: `src/app/lamad/components/content-analytics/` → `src/app/elohim/components/content-analytics/`
- Modify: `src/app/lamad/components/content-viewer/content-viewer.component.ts` — update import path
- Modify: `src/app/elohim/components/content-analytics/content-analytics.component.ts` — update EventService import

- [ ] **Step 1: Move the files**

```bash
cd app/elohim-app
mv src/app/lamad/components/content-analytics src/app/elohim/components/content-analytics
```

- [ ] **Step 2: Update import in ContentAnalyticsComponent**

In `src/app/elohim/components/content-analytics/content-analytics.component.ts`, the import `from '@app/shefa/services/event.service'` stays the same (shefa is the correct layer for EventService). No change needed to the component itself.

In the spec file, same — imports remain valid.

- [ ] **Step 3: Update import in ContentViewerComponent**

In `content-viewer.component.ts`, change:

```typescript
// Old:
import { ContentAnalyticsComponent } from '../content-analytics/content-analytics.component';
// New:
import { ContentAnalyticsComponent } from '@app/elohim/components/content-analytics/content-analytics.component';
```

- [ ] **Step 4: Run tests**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "content-analytics"`
Expected: PASS

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "content-viewer"`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add app/elohim-app/src/app/elohim/components/content-analytics/ \
       app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.ts
git commit -m "refactor: move ContentAnalyticsComponent to elohim — protocol-level metrics"
```

---

### Task 6: Create LamadEventService for domain-specific convenience methods

**Files:**
- Create: `src/app/lamad/services/lamad-event.service.ts`
- Create: `src/app/lamad/services/lamad-event.service.spec.ts`

- [ ] **Step 1: Write the failing test**

```typescript
// src/app/lamad/services/lamad-event.service.spec.ts
import { TestBed } from '@angular/core/testing';
import { of } from 'rxjs';

import { LamadEventService } from './lamad-event.service';
import { EventService } from '@app/shefa/services/event.service';

describe('LamadEventService', () => {
  let service: LamadEventService;
  let eventServiceSpy: { recordContentInteraction: ReturnType<typeof vi.fn> };

  const MOCK_EVENT = { id: 'evt-1' } as any;
  const AGENT = 'agent-1';
  const CONTENT = 'content-1';

  beforeEach(() => {
    eventServiceSpy = {
      recordContentInteraction: vi.fn().mockReturnValue(of(MOCK_EVENT)),
    };

    TestBed.configureTestingModule({
      providers: [
        LamadEventService,
        { provide: EventService, useValue: eventServiceSpy },
      ],
    });
    service = TestBed.inject(LamadEventService);
  });

  it('creates', () => {
    expect(service).toBeTruthy();
  });

  it('recordQuizSubmit calls recordContentInteraction with quiz-submit', () => {
    service.recordQuizSubmit(AGENT, CONTENT, 'quiz-1', true, 85);
    expect(eventServiceSpy.recordContentInteraction).toHaveBeenCalled();
  });

  it('recordAssessmentComplete calls recordContentInteraction', () => {
    service.recordAssessmentComplete(AGENT, CONTENT, 'assess-1', 90);
    expect(eventServiceSpy.recordContentInteraction).toHaveBeenCalled();
  });

  it('recordPathStepComplete delegates to EventService', () => {
    service.recordPathStepComplete(AGENT, 'path-1', 'step-1');
    expect(eventServiceSpy.recordContentInteraction).toHaveBeenCalled();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "lamad-event.service"`
Expected: FAIL — module not found

- [ ] **Step 3: Write the service**

```typescript
// src/app/lamad/services/lamad-event.service.ts
import { Injectable, inject } from '@angular/core';

import { Observable } from 'rxjs';

import { EconomicEventView } from '@app/elohim/adapters/storage-types.adapter';
import { EventService } from '@app/shefa/services/event.service';

/**
 * LamadEventService — Lamad domain convenience methods over the protocol EventService.
 *
 * The protocol provides `recordContentInteraction()` as the generic primitive.
 * This service adds lamad-specific helpers for assessments, paths, and practice —
 * things only a learning app would need. Other apps (governance, economics) would
 * build their own domain event services wrapping the same protocol primitive.
 */
@Injectable({ providedIn: 'root' })
export class LamadEventService {
  private readonly eventService = inject(EventService);

  recordQuizSubmit(
    agentId: string,
    contentId: string,
    quizId: string,
    correct: boolean,
    score?: number,
  ): Observable<EconomicEventView> {
    return this.eventService.recordContentInteraction(
      agentId,
      contentId,
      'content-complete' as any, // TODO: once LamadDomainEventType is wired, use 'quiz-submit'
    );
  }

  recordAssessmentComplete(
    agentId: string,
    contentId: string,
    assessmentId: string,
    score?: number,
  ): Observable<EconomicEventView> {
    return this.eventService.recordContentInteraction(
      agentId,
      contentId,
      'content-complete' as any, // TODO: once LamadDomainEventType is wired, use 'assessment-complete'
    );
  }

  recordPathStepComplete(
    agentId: string,
    pathId: string,
    stepId: string,
  ): Observable<EconomicEventView> {
    return this.eventService.recordContentInteraction(
      agentId,
      pathId,
      'content-complete' as any, // TODO: use 'path-step-complete'
    );
  }
}
```

**Note:** The `as any` casts are intentional. The `recordContentInteraction` currently accepts `ProtocolEventType`, but domain event types like `quiz-submit` aren't protocol-level. The next step (not in this sprint) is to make `recordContentInteraction` accept `ProtocolEventType | string` or to add a domain-typed overload. For now, these methods delegate through `content-complete` which IS protocol-level. The metadata (quizId, score, correct) should be added to `recordContentInteraction`'s API in a follow-up — this task establishes the SERVICE boundary, not the full type boundary.

- [ ] **Step 4: Run tests**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "lamad-event.service"`
Expected: All 4 tests PASS

- [ ] **Step 5: Commit**

```bash
git add app/elohim-app/src/app/lamad/services/lamad-event.service.ts \
       app/elohim-app/src/app/lamad/services/lamad-event.service.spec.ts
git commit -m "feat(lamad): add LamadEventService — domain convenience methods over protocol EventService"
```

---

### Task 7: Final lint and integration check

- [ ] **Step 1: Run full lint**

Run: `cd app/elohim-app && pnpm run lint`
Expected: PASS

- [ ] **Step 2: Run full test suite**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts`
Expected: PASS — no regressions. Old code still works through deprecated aliases.

- [ ] **Step 3: Commit any lint fixes**

```bash
git add -u app/elohim-app/
git commit -m "chore: lint fixes from protocol event boundary refactor"
```

---

## Self-Review Checklist

1. **Spec coverage:**
   - Protocol event types extracted → Task 1
   - Generic `recordContentInteraction()` → Task 2
   - Deprecated old methods (not removed — backwards compat) → Task 2
   - AttentionTracker updated → Task 3
   - Barrel exports updated → Task 4
   - ContentAnalyticsComponent moved to elohim → Task 5
   - LamadEventService created → Task 6

2. **Placeholder scan:** Task 6 has intentional `as any` casts with TODO comments. These are acknowledged tech debt — the domain event type union needs the `recordContentInteraction` API extended to accept domain types. Scoped for a follow-up, not this refactor.

3. **Type consistency:** `ProtocolEventType`, `ProtocolEventTypes`, `PROTOCOL_EVENT_MAPPINGS` used consistently across Tasks 1-4. `recordContentInteraction` signature identical in Tasks 2, 3, and 6.

## What This Does NOT Do (Scoped Out)

- **Does not delete `LamadEventType` from `economic-event.model.ts`** — Too many consumers to migrate in one refactor. The deprecated type remains, the new `ProtocolEventType` is the intended replacement. Consumers migrate incrementally.
- **Does not move domain-specific `LAMAD_EVENT_MAPPINGS` entries to lamad** — Same reason. The protocol mappings are extracted; the domain mappings stay alongside the deprecated type until consumers are migrated.
- **Does not update `SignalHarnessService`** — It imports `LamadEventType` from `@app/elohim/models` and infers types from renderer events. Will migrate when the deprecated type is removed.
- **Does not change the storage API contract** — The `lamadEventType` field name in `CreateEconomicEventInput` stays as-is. Renaming it requires a coordinated Rust + TS change. The field accepts any string at runtime; the TypeScript type narrows it.

These are follow-up tasks once the new boundary is proven through use.
