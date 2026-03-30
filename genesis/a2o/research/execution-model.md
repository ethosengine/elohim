# From Test Runner to Observation Minter

**Status:** Research
**Date:** 2026-03-30
**Depends on:** [grammar-spec.md](grammar-spec.md)

---

## Current Executor Architecture

### Components

All infrastructure lives in `genesis/a2o/src/framework/`:

| Component | File | Role |
|-----------|------|------|
| **E2EWorld** | `world.ts` | Cucumber World — shared state: doorways, humans, content IDs, device mode, cleanup callbacks |
| **DoorwayClient** | `api/doorway-client.ts` | Typed HTTP client for doorway API (auth, content CRUD, health, allocations) |
| **BrowserDevice** | `devices/browser-device.ts` | HTTP-only device via undici (no browser) |
| **PlaywrightDevice** | `devices/playwright-device.ts` | Real browser device — console/error/network capture, screenshots |
| **StewardDevice** | `devices/steward-device.ts` | Direct to elohim-storage at :8090 (Tauri desktop path) |
| **Human** | `human.ts` | Persona model with devices, auth tokens, agent pub key |
| **Reporter** | `reporters/console-report.ts` | Browser artifact JSON output |
| **Config** | `cucumber.mjs` | 6 profiles (default, alpha, local, browser, genesis, testnet) |

### Execution Flow

```
1. Cucumber reads .feature files per profile (paths + tag expressions)
2. Step definitions match Given/When/Then text patterns (regex/expression)
3. E2EWorld provides shared context (doorway clients, humans, content IDs)
4. Steps call DoorwayClient HTTP methods or PlaywrightDevice browser methods
5. Assertions verify responses (expect/assert)
6. Reporter outputs HTML/JSON to reports/
7. Process exits with pass/fail code
```

### What the Executor Knows

- Which doorway to talk to (env vars: `E2E_DOORWAY_ALPHA`, `E2E_DOORWAY_BETA`)
- Which humans exist (fixture credentials in `fixtures/humans.ts`)
- Which device mode to use (`E2E_DEVICE_MODE` env var: `http` or `playwright`)
- Test results (pass/fail/pending per scenario)

### What the Executor Does NOT Know

- Content addressing (no CIDs, no content graph awareness)
- Economic events (no REA pipeline connection)
- Observation history (no replication tracking)
- Governance context (no reach, no steward standing)
- Other agents' executions (no peer awareness)

---

## The Transformation

The transformation is **additive, not replacing**. Every existing component continues to work unchanged. New components layer observation capabilities on top.

### World → ObservationWorld

E2EWorld gains an observation dimension:

```typescript
interface ObservationContext {
  mode: 'local' | 'attestation';
  agentPubKey?: string;           // executing agent's identity
  scenarioCid?: string;           // current scenario's content address
  stepCids: Map<string, string>;  // step text → CID
  observations: PendingObservation[];  // buffered until scenario completes
  replicationOf?: string;         // CID of observation being replicated (null for original)
}

interface PendingObservation {
  scenarioCid: string;
  stepResults: StepResult[];
  startTime: number;
  endTime?: number;
  polarity?: 'positive' | 'negative';
  deviceType: string;
  doorwayId: string;
}

class ObservationWorld extends E2EWorld {
  // Inherits: doorways, humans, contentIds, deviceMode, cleanup
  readonly observationContext: ObservationContext;

  constructor(options: IWorldOptions) {
    super(options);
    this.observationContext = {
      mode: process.env.E2E_OBSERVATION_MODE === 'attestation'
        ? 'attestation' : 'local',
      stepCids: new Map(),
      observations: [],
    };
  }
}
```

In **local mode**, the observation context is inert — all observation methods are no-ops. Zero overhead on existing test runs.

In **attestation mode**, the context tracks CIDs, buffers observations, and mints economic events on scenario completion.

### Hooks → Observation Signals

Cucumber `Before`/`After` hooks gain observation lifecycle:

```typescript
// observation-hooks.ts

Before(async function (this: ObservationWorld, { pickle }) {
  if (this.observationContext.mode !== 'attestation') return;

  // Resolve scenario CID from pickle (scenario) content
  this.observationContext.scenarioCid = await computeScenarioCid(pickle);

  // Check prerequisites (DEPENDS_ON edges)
  const prereqs = resolvePrerequisites(pickle.tags);
  for (const prereq of prereqs) {
    const validity = await checkObservationValidity(prereq);
    if (validity < MINIMUM_PREREQ_VALIDITY) {
      throw new Error(`Prerequisite scenario ${prereq} has insufficient validity (${validity})`);
    }
  }

  // Record observation start
  this.observationContext.observations.push({
    scenarioCid: this.observationContext.scenarioCid,
    stepResults: [],
    startTime: Date.now(),
    deviceType: this.deviceMode,
    doorwayId: this.currentDoorwayId,
  });
});

After(async function (this: ObservationWorld, { pickle, result }) {
  if (this.observationContext.mode !== 'attestation') return;

  const observation = this.observationContext.observations.at(-1);
  if (!observation) return;

  observation.endTime = Date.now();
  observation.polarity = result.status === 'PASSED' ? 'positive' : 'negative';

  // Mint the observation as an EconomicEvent
  await this.observationMinter.mint(observation);
});
```

### Reporter → Minter

The existing reporter writes HTML/JSON artifacts. The observation minter writes EconomicEvents:

```typescript
// observation-minter.ts

class ObservationMinter {
  constructor(
    private readonly client: DoorwayClient,
    private readonly agentPubKey: string,
  ) {}

  async mint(observation: PendingObservation): Promise<void> {
    const event: CreateEconomicEventInput = {
      action: 'observe',
      provider: this.agentPubKey,
      receiver: observation.scenarioCid,
      resourceConformsTo: 'scenario-validity',
      resourceQuantityValue: observation.polarity === 'positive' ? 1.0 : -1.0,
      lamadEventType: 'scenario-observation',
      metadata: {
        polarity: observation.polarity,
        observation: observation.polarity === 'positive'
          ? 'behavioral-claim-holds'
          : 'behavioral-claim-fails',
        executionDuration: observation.endTime - observation.startTime,
        stepsExecuted: observation.stepResults.length,
        stepsPassed: observation.stepResults.filter(s => s.passed).length,
        replicationOf: observation.replicationOf ?? null,
        deviceType: observation.deviceType,
        doorwayId: observation.doorwayId,
      },
    };

    await this.client.createEconomicEvent(event);
  }
}
```

This pattern is **identical** to the signal harness (`signal-harness.service.ts`):

| Signal Harness | Observation Minter |
|---------------|-------------------|
| Input: `RendererCompletionEvent` | Input: `PendingObservation` |
| Reads manifest coupling for content type | Reads a2o manifest coupling for scenario type |
| Produces: `CreateEconomicEventInput` | Produces: `CreateEconomicEventInput` |
| Sends to: `EconomicEventsApiService` | Sends to: `DoorwayClient.createEconomicEvent()` |
| Action: varies by coupling | Action: `observe` |

Same pattern. Same economic event infrastructure. Different source event.

---

## The Dual-Mode Executor

### Mode: Local (default — existing behavior preserved)

```
E2E_OBSERVATION_MODE=local  (or unset — local is default)
```

- Cucumber runs exactly as it does today
- No CID resolution, no economic events, no DHT interaction
- ObservationContext exists but all methods are no-ops
- Step definitions call DoorwayClient HTTP endpoints
- Reports go to `reports/` directory
- All existing profiles (alpha, local, browser, testnet) work unchanged
- **Zero performance overhead** — observation code paths are guarded by mode check

### Mode: Attestation (new)

```
E2E_OBSERVATION_MODE=attestation
```

- Activated by env var or dedicated cucumber profile
- Before execution: resolve scenario CID, verify agent identity, check prerequisites
- During execution: record step-level results alongside normal test execution
- After execution: mint observation EconomicEvent via minter
- Observation flows through existing REA pipeline (`EconomicEventsApiService`)
- Observation reporter writes JSON log of all minted observations

### New Cucumber Profile

Added to `cucumber.mjs`:

```javascript
observation: {
  ...defaultConfig,
  worldParameters: {
    env: 'alpha',
    observationMode: 'attestation',
  },
  format: [
    'progress-bar',
    ['./src/framework/reporters/observation-report.ts', 'reports/observations.json'],
  ],
}
```

This profile runs the same scenarios as `alpha` but with observation minting enabled. The observation reporter outputs a structured JSON file alongside the existing HTML report.

---

## CID Computation

Content identifiers for scenarios are computed deterministically from content:

```typescript
// cid-resolver.ts

import { CID } from 'multiformats/cid';
import * as dagCBOR from '@ipld/dag-cbor';
import { sha256 } from 'multiformats/hashes/sha2';

async function computeScenarioCid(pickle: Pickle): Promise<string> {
  // Normalize: scenario name + step texts (order-preserving)
  const content = {
    name: pickle.name,
    steps: pickle.steps.map(s => ({
      type: s.type,     // Context (Given), Action (When), Outcome (Then)
      text: s.text,     // The step text
    })),
  };

  const bytes = dagCBOR.encode(content);
  const hash = await sha256.digest(bytes);
  return CID.create(1, dagCBOR.code, hash).toString();
}

async function computeStepCid(step: PickleStep): Promise<string> {
  const content = { type: step.type, text: step.text };
  const bytes = dagCBOR.encode(content);
  const hash = await sha256.digest(bytes);
  return CID.create(1, dagCBOR.code, hash).toString();
}
```

This uses the same CID algorithm as EPR content addressing — DAG-CBOR encoding with SHA-256 hash, CIDv1. Scenarios with identical content produce identical CIDs regardless of file location.

---

## Entry Points for Implementation

### Files That Change

| File | Change | Sprint |
|------|--------|--------|
| `src/framework/world.ts` | Extend E2EWorld with ObservationContext | Sprint 0 |
| `cucumber.mjs` | Add `observation` profile | Sprint 1 |
| `package.json` | Add `multiformats`, `@ipld/dag-cbor` dependencies | Sprint 0 |
| `scripts/scan-coverage.ts` | Add CID column to gap report | Sprint 0 |

### New Files

| File | Purpose | Sprint |
|------|---------|--------|
| `src/framework/cid-resolver.ts` | CID computation for scenarios and steps | Sprint 0 |
| `src/framework/observation-hooks.ts` | Before/After observation lifecycle | Sprint 1 |
| `src/framework/observation-minter.ts` | EconomicEvent creation from observations | Sprint 1 |
| `src/framework/reporters/observation-report.ts` | JSON output of minted observations | Sprint 1 |
| `scripts/index-scenarios.ts` | Parse all features → scenario index with CIDs | Sprint 0 |

### Files That Do NOT Change

| File | Reason |
|------|--------|
| All 30 `.feature` files | Grammar is unchanged |
| All 24 step definition files | They remain pure test/instrument logic |
| `api/doorway-client.ts` | HTTP transport is unchanged |
| `devices/*.ts` | Device abstraction is orthogonal to observation |
| `fixtures/humans.ts` | Fixture data unchanged |
| `src/framework/pages/*.ts` | Page objects unchanged |

---

## Authentication in Attestation Mode

The observation minter needs an agent identity to sign observations. Options:

**Option A: Reuse test human identity.** The test logs in as "Matthew" — observations are attributed to Matthew's agent pub key. Simple, but conflates the test persona with the observer.

**Option B: Dedicated observer agent.** The executor authenticates as a separate "observer" agent with its own pub key. Observations are clearly attributed to the CI/agent that ran them, not to a fixture human. This is cleaner and maps to the Observer Protocol's design: the observer is a distinct agent from the observed.

**Recommendation:** Option B. The observer agent's pub key becomes part of the observation metadata, enabling replication tracking per-agent.

---

## Failure Modes

### DHT Unreachable in Attestation Mode

If the DHT/doorway is unreachable when the minter tries to publish:

1. **Buffer locally.** Write observation to a local JSON file (`reports/pending-observations.json`)
2. **Retry on next run.** Before starting new observations, attempt to publish buffered ones
3. **Never fail the test run.** The test passed or failed independently of observation publishing. The observation is a side effect, not a gate.

This follows the existing Observer Protocol principle: observation is valuable but non-blocking. A failed publish doesn't mean the observation didn't happen.

### Environment Mismatch

A scenario that passes in one environment but fails in another is not necessarily an invalidation — it might be an environment configuration difference. The observation metadata includes `doorwayId` and `deviceType` to distinguish:

- Same scenario, same doorway, different result → behavioral drift (meaningful)
- Same scenario, different doorway, different result → environment divergence (investigate, don't auto-invalidate)

---

## Open Questions

1. **Formatter vs hooks**: Should the minter be a Cucumber custom formatter (has cleaner lifecycle integration, sees the full test run) or a hook-based system (more explicit, easier to reason about)? The prototype uses hooks; production may benefit from a formatter.

2. **Step-level observation**: Should each step produce its own observation, or only the scenario as a whole? Per-step observations enable "this Given passed but that Then failed" granularity, but multiply DHT entries by 3-5x per scenario.

3. **Parallel execution**: Cucumber supports parallel workers (`--parallel`). How does the observation context handle parallel scenario execution? Each worker needs its own ObservationContext instance — the World is already per-worker, so this should be natural.

4. **Observation deduplication**: If the same agent runs the same scenario twice in a row, should both observations be published? Or should the minter deduplicate within a validity window?
