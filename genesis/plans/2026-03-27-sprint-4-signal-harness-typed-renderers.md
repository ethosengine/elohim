# Sprint 4: Signal Harness + Typed Renderers

**Parent design:** `2026-03-27-typed-content-pipeline-design.md`
**Depends on:** Sprint 3 (codegen helper + alignment)
**Goal:** Renderers emit typed completion events. The signal harness translates them to REA economic events as declared in the lamad manifest. Renderer registry auto-wired from manifest. The full loop closes: render → signal → economic event → aggregation.

> **P2P note:** Economic events are Category A (notarized). This sprint wires the existing `CreateEconomicEventInput` (schema from sprint 1) to the existing `/db/events/bulk` endpoint. No new storage entities. The signal harness reads manifest coupling declarations and produces correctly typed economic events.

## Tasks

### 1. Create signal harness service

**File:** `app/elohim-app/src/app/lamad/services/signal-harness.service.ts`

The harness is the bridge between renderer output and protocol input. It reads the manifest and translates.

```typescript
import manifest from '@app/lamad/manifest.json';
import type { CreateEconomicEventInput } from '@app/generated/create-economic-event-input';
import type { RendererCompletionEvent } from '../renderers/renderer-registry.service';
import type { TypedContentNode } from '@app/lamad/generated/content-node-types';

@Injectable({ providedIn: 'root' })
export class SignalHarnessService {
  private readonly economicEventsApi = inject(EconomicEventsApiService);
  private readonly agentService = inject(AgentService);

  /**
   * Called when a renderer emits a completion event.
   * Looks up the manifest coupling for the content type,
   * translates to REA economic event, dispatches.
   */
  async onRendererComplete(
    node: TypedContentNode,
    event: RendererCompletionEvent
  ): Promise<void> {
    const agentId = this.agentService.getCurrentAgentId();
    const coupling = this.getCoupling(node.contentType);
    if (!coupling?.value) return;

    // Determine which lifecycle event fired
    const lifecycle = event.passed ? 'onComplete' : 'onConsume';
    const valueFlow = coupling.value[lifecycle];
    if (!valueFlow) return;

    // Build typed economic event from manifest declaration
    const economicEvent: CreateEconomicEventInput = {
      action: valueFlow.action,
      provider: agentId,
      receiver: node.id,
      resourceConformsTo: valueFlow.resourceConformsTo,
      lamadEventType: this.inferLamadEventType(node, event),
      contentId: node.id,
      metadata: {
        contentType: node.contentType,
        contentFormat: node.contentFormat,
        score: event.score,
        signal: this.getSignalType(coupling, lifecycle),
      },
    };

    await this.economicEventsApi.createEconomicEvent(economicEvent);
  }

  private getCoupling(contentType: string): ThreeLegCoupling | undefined {
    return manifest.vocabulary.contentTypes[contentType]?.coupling;
  }

  private getSignalType(coupling: ThreeLegCoupling, lifecycle: string): string | undefined {
    // Map lifecycle to signal: onComplete → mastery-achieved, onConsume → learning-signal
    const signalTypes = coupling.governance?.signalTypes ?? [];
    if (lifecycle === 'onComplete') return signalTypes.find(s => s.includes('mastery') || s.includes('completed'));
    return signalTypes.find(s => s.includes('learning') || s.includes('engagement'));
  }

  private inferLamadEventType(node: TypedContentNode, event: RendererCompletionEvent): string {
    if (event.type === 'quiz' && event.passed) return 'assessment-complete';
    if (event.type === 'quiz') return 'assessment-attempt';
    if (event.type === 'simulation') return 'simulation-complete';
    return 'content-complete';
  }
}
```

### 2. Wire signal harness into content viewer

**File:** `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.ts`

Replace direct service calls with harness:

```typescript
// Before (hand-wired, no manifest coupling):
this.agentService.markContentSeen(nodeId);

// After (manifest-driven):
private readonly signalHarness = inject(SignalHarnessService);

// On renderer completion:
if (instance['complete'] instanceof Object && 'subscribe' in instance['complete']) {
  this.rendererSubscription = instance['complete'].subscribe(
    (event: RendererCompletionEvent) => {
      this.signalHarness.onRendererComplete(this.node, event);
      this.onRendererComplete(event); // existing UI handling
    }
  );
}

// On content view (attention signal):
this.signalHarness.onRendererComplete(this.node, {
  type: 'view',
  passed: true,
  score: 0,
});
```

### 3. Auto-wire renderer registry from manifest

**File:** `app/elohim-app/src/app/lamad/renderers/renderer-initializer.service.ts`

Replace hard-coded registration:

```typescript
// Before (hard-coded):
registry.register(['markdown'], MarkdownRendererComponent, 10);
registry.register(['html5-app', 'video-embed'], IframeRendererComponent, 10);
registry.register(['gherkin'], GherkinRendererComponent, 5);
registry.register(['perseus-quiz-json', 'perseus', 'sophia', 'sophia-quiz-json'], SophiaRendererComponent, 15);

// After (manifest-driven):
import manifest from '@app/lamad/manifest.json';
import { RENDERER_COMPONENTS } from './renderer-components';

// RENDERER_COMPONENTS maps manifest renderer names to Angular components
const RENDERER_COMPONENTS: Record<string, Type<ContentRenderer>> = {
  'markdown-renderer': MarkdownRendererComponent,
  'gherkin-renderer': GherkinRendererComponent,
  'sophia-renderer': SophiaRendererComponent,
  'iframe-renderer': IframeRendererComponent,
  'path-renderer': PathViewerComponent,
};

for (const [rendererName, registration] of Object.entries(manifest.rendering)) {
  const component = RENDERER_COMPONENTS[rendererName];
  if (component) {
    registry.register(registration.formats, component);
  }
}
```

BYO renderer: add format to manifest + add component to `RENDERER_COMPONENTS`. The registry reads the manifest — no code changes to the registry itself.

### 4. Type renderer inputs

**SophiaRendererComponent:**
```typescript
// Before:
@Input() node: ContentNode;  // generic, 410-line interface

// After:
@Input() node: TypedContentNode;  // discriminated union
// Inside component, narrow:
if (isAssessmentNode(this.node)) {
  // this.node.metadata is AssessmentMetadata — typed
  this.assessmentMode = this.node.metadata.mode ?? 'mastery';
}
```

**MarkdownRendererComponent:**
```typescript
@Input() node: TypedContentNode;
// node.content is string (markdown) — the discriminated union ensures this
```

**PathViewerComponent:**
```typescript
@Input() node: TypedContentNode & { contentType: 'path' };
// node.metadata is PathMetadata — typed
// JSON.parse(node.contentBody) is EprCompositeBody — typed
```

### 5. Scaffold aggregation instruments (pseudo-code, not delivered)

Document the pattern for sprint 5+. Aggregation instruments consume economic events and produce state changes:

```typescript
// PSEUDO-CODE — sprint 5+ scaffolding

interface AggregationInstrument<TInput, TOutput> {
  /** Which economic event types this instrument processes */
  readonly eventFilter: { action: string; resourceConformsTo: string };

  /** Aggregate an event into the instrument's state */
  aggregate(event: EconomicEventView, currentState: TOutput): TOutput;

  /** Produce a state change from accumulated signals */
  evaluate(state: TOutput): StateChange[];
}

// Example: mastery instrument
class MasteryInstrument implements AggregationInstrument<EconomicEventView, MasteryState> {
  readonly eventFilter = { action: 'produce', resourceConformsTo: 'mastery-attestation' };

  aggregate(event, state) {
    // Quiz score → mastery level progression
    // Uses sophia psychometric model for scoring
    return { ...state, attempts: state.attempts + 1, lastScore: event.metadata.score };
  }

  evaluate(state) {
    // Enough evidence to level up?
    if (state.lastScore >= 0.8 && state.attempts >= 2) {
      return [{ type: 'mastery-level-up', contentId: state.contentId, newLevel: 'understand' }];
    }
    return [];
  }
}

// Example: stewardship instrument
class StewardshipInstrument implements AggregationInstrument<EconomicEventView, StewardshipState> {
  readonly eventFilter = { action: 'produce', resourceConformsTo: 'contribution' };

  aggregate(event, state) {
    // Contribution → stewardship standing
    return { ...state, contributions: state.contributions + 1 };
  }

  evaluate(state) {
    // Enough contributions to increase affinity?
    // Only curation acts build standing — attention doesn't (anti-attention-economy)
  }
}
```

### 6. Scaffold manifest governance lifecycle (pseudo-code, not delivered)

Document the pattern for sprint 5+:

```typescript
// PSEUDO-CODE — manifest as governed EPR

interface ManifestGovernance {
  /** Validate manifest coupling at registration time */
  validateManifest(manifest: AppManifest): ValidationResult;

  /** Challenge a manifest's coupling declarations */
  challengeManifest(manifestCid: string, reason: string): Challenge;

  /** Governance process reviews and decides */
  resolveChallenge(challengeId: string, decision: 'uphold' | 'revoke' | 'require-update'): Resolution;

  /** Force minimum version — nodes must upgrade or stop serving */
  requireMinimumVersion(manifestName: string, minimumVersion: string): Decree;
}

// Protocol enforces:
// - Every content type has value + governance legs (structural validation)
// - Signal declarations map to valid substrate signals
// - Economic actions reference valid ResourceSpecifications
// - Revoked manifests are not served by doorway

// Governance enforces:
// - Coupling declarations match observed behavior
// - Assessment instruments are meaningful (not trivial mastery farming)
// - Economic flows are balanced (not extractive)
```

## Verification

```bash
# App builds with signal harness
cd app/elohim-app && pnpm run build

# Renderer registry reads from manifest (no hard-coded format lists)
grep -r "registry.register\[" src/app/lamad/renderers/renderer-initializer.service.ts  # only manifest-driven

# Signal harness wired in content viewer
grep "signalHarness" src/app/lamad/components/content-viewer/content-viewer.component.ts  # present

# Deploy to alpha, then:
# 1. Open /lamad — path thumbnails load ✓
# 2. Click path — chapter overview with correct counts ✓
# 3. Click Start Chapter — navigates to first step ✓
# 4. Complete sophia quiz — network tab shows POST to /db/events/bulk ✓
# 5. Economic event contains:
#    - action: "produce" (from manifest.concept.coupling.value.onComplete.action)
#    - resourceConformsTo: "mastery-attestation" (from manifest)
#    - lamadEventType: "assessment-complete"
#    - metadata.signal: "mastery-achieved" (from manifest governance signalTypes)
```

## What This Sprint Does NOT Deliver

- Aggregation instruments (pseudo-code only — sprint 5+)
- Manifest governance lifecycle (pseudo-code only — sprint 5+)
- Rust-side manifest validation (requires DNA changes)
- Manifest revocation enforcement in doorway
- Mastery level progression from accumulated economic events

These are the immune system. Sprint 4 builds the nervous system (signal flow). The immune system follows.
