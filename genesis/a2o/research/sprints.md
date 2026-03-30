# Implementation Roadmap: Sprints 0-5

**Status:** Research
**Date:** 2026-03-30
**Depends on:** All research documents in this directory

---

## Staging Pattern

Following the P2P Build System Roadmap's four-stage arc (`genesis/plans/2026-03-20-p2p-native-build-system-roadmap.md`):

| Stage | Name | Sprints | What Changes |
|-------|------|---------|-------------|
| **Seed** | Current state | — | Existing Cucumber harness, single trusted executor |
| **Root** | Content addressing + observation minting | 0, 1 | Scenarios get CIDs, execution produces EconomicEvents |
| **Canopy** | Interpretability surface + peer replication | 2, 3 | Grammar tags, multi-agent execution, replication tracking |
| **Forest** | Self-sustaining economics + observer integration | 4, 5 | Royalties, pool distribution, observation surface expansion |

Each sprint builds on the prior. No sprint requires changes to existing `.feature` files or step definitions unless explicitly noted as optional.

---

## Sprint 0: "Content Addressing the Existing"

**Stage:** Root
**Goal:** Make existing `.feature` files addressable as ContentNodes without changing any execution behavior. Existing cucumber runs are unchanged. Scenarios are now in the content graph.

### Deliverables

**1. CID computation module** — `src/framework/cid-resolver.ts`

Compute deterministic CIDs from scenario content using DAG-CBOR + SHA-256 (same algorithm as EPR content addressing):
- `computeFeatureCid(featureFile)` → CID from full feature file content
- `computeScenarioCid(pickle)` → CID from scenario name + ordered step texts
- `computeStepCid(step)` → CID from step type + text

**2. Scenario index generator** — `scripts/index-scenarios.ts`

Parse all 30 `.feature` files and generate a `reports/scenario-index.json`:
```json
{
  "generated": "2026-03-30T...",
  "features": [
    {
      "file": "features/content/content-lifecycle.feature",
      "cid": "bafyrei...",
      "scenarios": [
        {
          "name": "Create content and verify retrieval",
          "cid": "bafyrei...",
          "tags": ["@e2e", "@content"],
          "stepCount": 4,
          "steps": [
            { "type": "Context", "text": "human \"Matthew\" is logged in...", "cid": "bafyrei..." }
          ],
          "inferredRelationships": {
            "VALIDATES": ["content-lifecycle"],
            "DEPENDS_ON": []
          }
        }
      ]
    }
  ]
}
```

**3. Coverage scanner integration** — modify `scripts/scan-coverage.ts`

Extend the existing gap report to include scenario CIDs. Each conceptual→executable mapping gains a CID column. No behavioral change to scanning.

### Entry Points

| Action | File | Change |
|--------|------|--------|
| New | `src/framework/cid-resolver.ts` | CID computation functions |
| New | `scripts/index-scenarios.ts` | Feature file parser → index generator |
| Modify | `scripts/scan-coverage.ts` | Add CID column to gap report |
| Modify | `package.json` | Add `multiformats`, `@ipld/dag-cbor` dependencies |

### What Does NOT Change

All 30 `.feature` files, all 24 step definition files, `cucumber.mjs`, `world.ts`, all device/client code. Sprint 0 is purely additive — read-only operations on existing files, new tooling alongside.

### Upgrade Path for Existing Scenarios

Zero changes required. CID indexing is a read-only operation.

### Verification

```bash
npx tsx scripts/index-scenarios.ts
# → reports/scenario-index.json exists with CIDs for all scenarios
# → CIDs are deterministic (running twice produces identical output)
```

---

## Sprint 1: "Observation Minting"

**Stage:** Root
**Goal:** Execution produces EconomicEvents. Wire the signal harness pattern so scenario completion flows through the existing REA pipeline.

### Deliverables

**1. ObservationWorld extension** — modify `src/framework/world.ts`

Add `ObservationContext` to E2EWorld:
- `mode: 'local' | 'attestation'` (from env var `E2E_OBSERVATION_MODE`)
- `scenarioCid`, `stepCids`, `pendingObservations`
- All observation methods are no-ops in local mode (zero overhead)

**2. Observation hooks** — `src/framework/observation-hooks.ts`

Cucumber Before/After hooks for observation lifecycle:
- `BeforeScenario`: resolve scenario CID, set observation context
- `AfterScenario`: compute observation result, mint EconomicEvent if attestation mode
- Hooks are no-ops in local mode

**3. Observation minter** — `src/framework/observation-minter.ts`

Translates `PendingObservation` → `CreateEconomicEventInput` following the signal harness pattern (`signal-harness.service.ts`):
- `action: "observe"`
- `provider: {executing-agent}`
- `receiver: {scenario-cid}`
- `resourceConformsTo: "scenario-validity"`

**4. Observation profile** — modify `cucumber.mjs`

New profile `observation` with attestation mode enabled. Existing profiles unchanged.

**5. Observation reporter** — `src/framework/reporters/observation-report.ts`

JSON output of all observations minted during a run. Parallel to existing `console-report.ts`.

### Entry Points

| Action | File | Change |
|--------|------|--------|
| Modify | `src/framework/world.ts` | Add ObservationContext to E2EWorld |
| Modify | `cucumber.mjs` | Add `observation` profile |
| New | `src/framework/observation-hooks.ts` | Before/After observation lifecycle |
| New | `src/framework/observation-minter.ts` | EconomicEvent creation |
| New | `src/framework/reporters/observation-report.ts` | JSON observation log |

### What Does NOT Change

All `.feature` files, all step definitions, all device/client code. The observation layer is a side-channel — tests run identically, observations are a side effect.

### Upgrade Path for Existing Scenarios

Zero changes to feature files or step definitions. The `observation` profile runs the same scenarios with economic event side effects.

### Verification

```bash
# Local mode — existing behavior, no observations
npm test

# Attestation mode — same tests, plus observation output
E2E_OBSERVATION_MODE=attestation npx cucumber-js --profile observation
# → reports/observations.json exists with EconomicEvent records
# → All existing tests still pass
```

---

## Sprint 2: "The Interpretability Surface"

**Stage:** Canopy
**Goal:** Add optional protocol metadata to feature files via tag vocabulary. Agent tooling resolves tags to protocol metadata. Human-readable surface unchanged.

### Deliverables

**1. Tag resolver** — `src/framework/tag-resolver.ts`

Parse protocol-aware tags (`@validates:`, `@depends-on:`, `@mints:`, `@freshness:`, `@reach:`, `@replication:`). Resolve slug references to CIDs via scenario index. Generate three-leg coupling metadata from tag combinations.

**2. a2o domain manifest** — `elohim/sdk/domains/a2o/manifest.json`

Define `observation-scenario` and `observation-step` content types with three-leg coupling. Follow the exact structure of `elohim/sdk/domains/lamad/manifest.json`. Include observation vocabulary (`behavioral-claim-holds`, `behavioral-claim-fails`).

**3. Sidecar metadata generator** — `scripts/generate-sidecar-meta.ts`

For each `.feature` file, generate `.meta.json` with resolved protocol metadata: CID, coupling, relationships, claims, freshness. Sidecars are agent-generated, never human-authored.

**4. Feature file tag enrichment** (optional, incremental)

Add protocol tags to highest-value existing features first:
- `features/content/content-lifecycle.feature` — `@validates:content-crud`
- `features/content/epr-content-addressing.feature` — `@validates:epr-resolution`
- `features/elohim/compute-allocation.feature` — `@validates:rea-settlement @mints:compute-observation`

### Entry Points

| Action | File | Change |
|--------|------|--------|
| New | `elohim/sdk/domains/a2o/manifest.json` | a2o domain manifest |
| New | `src/framework/tag-resolver.ts` | Protocol tag parsing and resolution |
| New | `scripts/generate-sidecar-meta.ts` | Sidecar metadata generation |
| Optional | Existing `.feature` files | Add `@validates:`, `@freshness:` tags |

### What Does NOT Change

Step definitions, device code, world, minter, cucumber profiles. Tags are additive and ignored by Cucumber itself.

### Upgrade Path for Existing Scenarios

Optional. Scenarios without protocol tags continue to work. Tags are added incrementally by stewards as they curate domains. No migration required.

### Verification

```bash
npx tsx scripts/generate-sidecar-meta.ts
# → .meta.json sidecars generated for all features
# → Sidecars include CIDs, resolved relationships, coupling metadata
# → Schema validation passes against a2o manifest
```

---

## Sprint 3: "Replication Protocol"

**Stage:** Canopy
**Goal:** Independent agents can re-execute scenarios. Replication history is tracked. Invalidation is rewarded. Connect to feedback information flows design.

### Deliverables

**1. Replication tracker** — `src/framework/replication-tracker.ts`

Record which agents have executed which scenarios:
- Track diversity metrics (agent, doorway, device, time)
- Compute diversity score (diverse replications weighted higher)
- Persist replication history to `reports/replication-history.json`

**2. Observation accumulator** — `src/framework/observation-accumulator.ts`

Implement validity horizon tracking per scenario:
- Positive observations extend validity (weight by diversity)
- Negative observations shorten validity
- Generate review obligations when validity drops below threshold
- Use same mathematical model as mastery freshness decay

**3. Invalidation detection** — modify `observation-hooks.ts`

When a previously-passing scenario fails for a new agent:
- Detect this as invalidation (cross-reference replication history)
- Mint invalidation EconomicEvent with higher weight
- Credit the discovering agent

**4. Replication dashboard** — `scripts/replication-report.ts`

Per-scenario: replication count, diversity score, validity, last execution, agent list. Aggregate: domain-level replication health. Output compatible with existing `reports/` directory.

### Entry Points

| Action | File | Change |
|--------|------|--------|
| Modify | `src/framework/observation-hooks.ts` | Add invalidation detection |
| Modify | `src/framework/observation-minter.ts` | Add replication metadata |
| New | `src/framework/replication-tracker.ts` | Replication history tracking |
| New | `src/framework/observation-accumulator.ts` | Validity horizon computation |
| New | `scripts/replication-report.ts` | Replication dashboard data |

### What Does NOT Change

Feature files, step definitions, grammar surface. Replication tracking is transparent — existing scenarios gain tracking automatically when executed through the observation profile.

### Verification

```bash
# Agent A executes
E2E_OBSERVATION_MODE=attestation E2E_AGENT_ID=agent-a npx cucumber-js --profile observation

# Agent B replicates (different agent identity)
E2E_OBSERVATION_MODE=attestation E2E_AGENT_ID=agent-b npx cucumber-js --profile observation

# Replication report shows both agents, diversity score > 0
npx tsx scripts/replication-report.ts
# → Per-scenario diversity scores
# → Validity horizon status
# → Any invalidation discoveries flagged
```

---

## Sprint 4: "Observation Royalties"

**Stage:** Forest
**Goal:** Wire up the economics. Bounty/delivery/residual/invalidation model. Steward pool and elohim pool distribution.

### Deliverables

**1. Observation economics module** — `src/framework/observation-economics.ts`

Compute bounty, delivery, residual, and invalidation amounts:
- Bounty sizing from gap analysis (uncovered domains → higher bounties)
- Delivery weighting by complexity (step count, device type, multi-doorway)
- Residual splits: author (25%) / executor (25%) / steward pool (30%) / elohim pool (20%)
- Invalidation bounty scaling by prior validity score

**2. Shefa vocabulary additions** — modify `elohim/sdk/domains/shefa/manifest.json`

Add observation resource types:
- `observation-bounty`
- `observation-delivery`
- `observation-residual`
- `observation-invalidation`
- `scenario-validity`

**3. Pool distribution logic** — `src/framework/pool-distributor.ts`

Split residuals per governance-defined ratios. Commons default: unclaimed scenarios pool to community. Connect to existing commons pool pattern.

**4. Coverage-driven bounty pricing** — modify `scripts/scan-coverage.ts`

Gap report now includes bounty recommendations. Uncovered domains → 3x multiplier. Saturated domains → 0.5x. Gap analysis becomes a bounty board.

### Entry Points

| Action | File | Change |
|--------|------|--------|
| Modify | `elohim/sdk/domains/shefa/manifest.json` | Add observation resource types |
| Modify | `scripts/scan-coverage.ts` | Add bounty pricing column |
| Modify | `src/framework/observation-minter.ts` | Integrate economics module |
| New | `src/framework/observation-economics.ts` | Economic computation |
| New | `src/framework/pool-distributor.ts` | Pool distribution logic |

### Upgrade Path for Existing Scenarios

Economics activate only for scenarios with protocol tags and active steward claims. Untagged scenarios generate observations but not economic events.

### Verification

```bash
# Run observation with economics enabled
E2E_OBSERVATION_MODE=attestation npx cucumber-js --profile observation

# Verify economic events were generated
# → reports/observations.json includes bounty/delivery/residual events
# → Pool distribution totals are correct (sum of splits = total residual)
# → Gap analysis shows bounty recommendations
npx tsx scripts/scan-coverage.ts
```

---

## Sprint 5: "Observer Integration"

**Stage:** Forest
**Goal:** Expand observation surface beyond code behavior. Connect to Observer Protocol. Scenario grammar becomes general observation grammar.

**Gated on:** Sprint 3 stability (same gating pattern as build system roadmap Stage 3)

### Deliverables

**1. General observation grammar** (research + prototype)

Define how Given/When/Then applies to non-software claims:
```gherkin
Scenario: Community garden produces weekly harvest
  Given the Elm Street garden has 12 active plots
  When the weekly harvest is observed
  Then at least 8 plots report yield
  And the community food access score increases
```

Specify which Observer Protocol outputs map to scenario structures. The observer captures physical reality; the grammar structures the observation.

**2. Observer-to-scenario bridge** — `src/framework/observer-bridge.ts`

Translate Observer Protocol REA story elements to observation events:
- Observer captures → processes patterns → generates observation
- Same flow as scenario execution → processes pass/fail → generates observation
- Unified observation entry point for both code and physical observations

**3. Unified observation vocabulary** — modify `elohim/sdk/domains/a2o/manifest.json`

Merge software-behavioral observations with physical-world observations. Shared instrument archetypes: `code-execution`, `visual-witness`, `behavioral-pattern`, `community-report`. These extend the feedback design's instrument archetypes.

**4. Observation surface documentation**

Map which observer protocol events produce which observation types. Map which scenario domains extend to physical-world observation. Define the boundary between "software behavior" and "general observation."

### Entry Points

| Action | File | Change |
|--------|------|--------|
| Modify | `elohim/sdk/domains/a2o/manifest.json` | Unified observation vocabulary |
| New | `src/framework/observer-bridge.ts` | Observer Protocol integration |
| New | Research documentation | Observation surface mapping |

### Upgrade Path

This sprint is aspirational. It will be designed when Sprint 3 is operational and the replication protocol is proven stable. The grammar, execution model, and economics from Sprints 0-4 are the foundation; Sprint 5 extends the surface, not the infrastructure.

---

## Dependencies

```
Sprint 0 (CID addressing)
    ↓ scenarios have identities
Sprint 1 (observation minting)
    ↓ execution produces economic events
Sprint 2 (interpretability surface)
    ↓ scenarios carry protocol metadata
Sprint 3 (replication protocol)
    ↓ independent agents can verify claims
Sprint 4 (observation royalties)
    ↓ economics sustain the network
Sprint 5 (observer integration)
    ↓ observation surface expands beyond code
```

- Sprints 0 and 1 can be done by a single developer with a2o framework knowledge
- Sprint 2 requires SDK schema knowledge (manifest authoring)
- Sprint 3 requires distributed systems experience (replication, diversity, freshness)
- Sprint 4 requires REA economics understanding (value flows, pool distribution)
- Sprint 5 requires Observer Protocol integration (cross-team coordination)

---

## Risk Register

| Risk | Impact | Likelihood | Mitigation |
|------|--------|-----------|------------|
| **CID instability** — content changes produce new CIDs, breaking observation history | High | Medium | Scenario versioning: `DERIVED_FROM` links create CID chains, not replacements. Old CIDs retain history. |
| **Economic gaming** — bounty farming via trivial scenarios | Medium | Medium | `steward-consent` gating: bounties activate only after steward endorsement |
| **Environment divergence** — replication fails due to config, not behavior | High | High | Environment prerequisites as `DEPENDS_ON` edges. Grace period for first-time failures from new agents. |
| **Complexity creep** — protocol metadata makes `.feature` files unreadable | High | Low | Agent complexity absorption principle: no protocol syntax in feature files. Tags are optional and minimal. |
| **DHT performance** — observation volume overwhelms the network | Medium | Low | Dual-mode: local is default, attestation is opt-in. Observation deduplication within validity windows. |
| **Pool funding** — observation economics require external subsidy | Medium | Medium | Start with governance-allocated pool. Transition to self-sustaining as trust substrate proves value. |

---

## Connection to Existing Roadmaps

| Roadmap | How This Connects |
|---------|-------------------|
| **P2P Build System** (`genesis/plans/2026-03-20-p2p-native-build-system-roadmap.md`) | Build attestations and observation attestations use the same ContentNode + EconomicEvent pattern. A build that passes its scenarios mints both a build-attestation and observation events. |
| **Feedback Information Flows** (`genesis/plans/2026-03-28-feedback-information-flows-design.md`) | Sprint 3 directly implements the claims/observations/obligation accumulation cycle for scenarios. Scenarios are a new instrument archetype alongside `retention-check`, `outcome-correlation`, etc. |
| **Governance Gateway** (sprints 5-9) | Governance proposals can require observation coverage. "This change affects auth — show me the auth scenario observations before we ratify." |
| **Recognition Pipeline** (`genesis/plans/2026-03-13-recognition-pipeline-design.md`) | Observation minting flows through the same recognition pipeline as all other economic events. The `StageTrace` for observations tells the story: "here's who observed, here's what they found, here's how value distributed." |
