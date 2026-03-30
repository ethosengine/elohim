# Replication Protocol: Peer-Attested Observations

**Status:** Research
**Date:** 2026-03-30
**Depends on:** [execution-model.md](execution-model.md)
**Connects to:** [Feedback Information Flows Design](../../plans/2026-03-28-feedback-information-flows-design.md)

---

## The Scientific Model

Observation science requires replication. A single observation is an anecdote. Replicated observations become evidence. Failed replications are discoveries, not failures.

Traditional CI inverts this: a passing test is expected, a failing test is a problem. The replication protocol restores the scientific framing:

| Scientific Concept | Scenario Equivalent |
|-------------------|---------------------|
| Hypothesis | Authored scenario (Given/When/Then claim) |
| Experiment | First execution by any agent |
| Replication | Independent execution by a different agent |
| Confirmation | Independent execution passes |
| Disconfirmation | Independent execution fails — a **discovery** |
| Retraction | Scenario validity drops below threshold |
| Peer review | Steward evaluation of observation quality |

The critical inversion: **disconfirmation is rewarded**. In peer-reviewed science, the researcher who demonstrates a result doesn't replicate receives credit for advancing knowledge. In the observation protocol, the agent who discovers that a previously-valid scenario no longer holds receives economic credit for surfacing behavioral drift.

---

## Observation Accumulation

Each scenario CID accumulates a **replication history** — a sequence of observations from diverse agents over time. This history determines the scenario's **validity score**.

### The Accumulation Cycle

This directly implements the feedback design's accumulation cycle (`genesis/plans/2026-03-28-feedback-information-flows-design.md`, lines 270-285):

```
Agent executes scenario
        ↓
  pass → positive observation → extends validity horizon
  fail → negative observation → shortens validity horizon
        ↓
  validity horizon expires without fresh evidence
        ↓
  scenario enters "review" state
        ↓
  review produces one of:
    • revalidation (someone re-executes, passes → horizon resets)
    • correction (scenario updated → new CID, old one deprecated)
    • escalation (governance deliberation required)
```

### Positive Observations (Scenario Passes)

Each passing execution by a new agent is a positive-polarity observation:

```json
{
  "action": "observe",
  "provider": "{executing-agent}",
  "receiver": "{scenario-cid}",
  "resourceConformsTo": "scenario-validity",
  "resourceQuantityValue": 1.0,
  "metadata": {
    "polarity": "positive",
    "observation": "behavioral-claim-holds",
    "replicationOf": "{original-observation-cid}",
    "replicatorDiversity": {
      "agentIsNew": true,
      "doorwayIsNew": true,
      "deviceTypeIsNew": false,
      "daysSinceLastReplication": 14
    }
  }
}
```

Multiple positive observations from diverse agents extend the validity horizon. The extension amount depends on replicator diversity (see below).

### Negative Observations (Scenario Fails)

Each failing execution is a negative-polarity observation:

```json
{
  "action": "observe",
  "provider": "{discovering-agent}",
  "receiver": "{scenario-cid}",
  "resourceConformsTo": "scenario-validity",
  "resourceQuantityValue": -1.0,
  "metadata": {
    "polarity": "negative",
    "observation": "behavioral-claim-fails",
    "replicationOf": "{original-observation-cid}",
    "failureContext": {
      "failingStep": "Then the content should be retrievable by ID",
      "stepCid": "{step-cid}",
      "errorSummary": "404 Not Found after 30s timeout"
    }
  }
}
```

Negative observations are **economically valuable**. They shorten the validity horizon and may generate review obligations. The discovering agent receives credit proportional to the observation's information value — finding the first failure is worth more than confirming an already-known problem.

---

## Freshness Decay

### Connection to Existing Mastery Decay

The lamad domain already implements freshness decay for content mastery: `ContentMastery` tracks `freshness_score` with configurable decay rates. Observation validity uses the **same mathematical model**:

```
validity(t) = validity(t₀) × e^(-λ × (t - t₀))
```

Where:
- `validity(t₀)` = validity score at last observation
- `λ` = decay rate (configurable per scenario type)
- `t - t₀` = time since last observation

A positive replication resets `validity(t₀)` to 1.0 and restarts the clock. A negative observation reduces `validity(t₀)` by its weight and restarts the clock at the lower value.

### Validity Horizon by Observation Type

Different scenarios have different natural validity periods, reflecting how quickly the underlying behavior is likely to change:

| Domain | Example Scenario | Default `@freshness` | Rationale |
|--------|-----------------|---------------------|-----------|
| **auth** | Auth lifecycle (register, login, logout) | `P7D` | Auth APIs change frequently |
| **content** | Content CRUD operations | `P14D` | Content model evolves moderately |
| **federation** | Cross-doorway content sync | `P30D` | Federation protocol is stable |
| **lamad** | Learning journey progression | `P30D` | Learning path structure is stable |
| **qahal** | Governance voting mechanics | `P60D` | Governance rules change slowly |
| **shefa** | Economic event settlement | `P90D` | REA patterns are foundational |
| **deployment** | P2P peer connectivity | `P7D` | Infrastructure changes frequently |
| **elohim** | Compute allocation | `P30D` | Compute model evolves moderately |

These are defaults. Stewards can override per-scenario via `@freshness:{duration}` tags.

### Decay Is Not Punishment

This mirrors the feedback design's therapeutic model: "Mastery decay isn't punishment for forgetting." Observation decay isn't punishment for not re-running tests. It's an honest reflection: **we haven't checked this recently, so our confidence has naturally diminished**. Re-running restores confidence. Not re-running doesn't mean the behavior broke — it means we don't know.

---

## Diversity of Observers

A single agent replicating its own scenario 100 times adds less trust than 3 independent agents each replicating once. The protocol weights observations by **replicator diversity** across four dimensions:

### Diversity Dimensions

| Dimension | What It Measures | Weight Factor |
|-----------|-----------------|---------------|
| **Agent identity** | Different pub keys | Highest — independent agents are the core of replication |
| **Doorway** | Different deployment environments | High — confirms behavior isn't environment-specific |
| **Device type** | HTTP vs Playwright vs Steward | Medium — confirms behavior isn't transport-specific |
| **Temporal spread** | Replications spread over days vs. minutes | Medium — confirms behavior persists over time |

### Diversity Score Computation

```
diversity_score = (
  unique_agents / total_replications × 0.4 +
  unique_doorways / total_replications × 0.3 +
  unique_device_types / 3 × 0.15 +           // max 3 device types
  temporal_spread_factor × 0.15              // 0..1 based on time distribution
)
```

A scenario with 10 replications from 1 agent on 1 doorway has a low diversity score. A scenario with 3 replications from 3 agents on 2 doorways across 3 weeks has a high diversity score. **The protocol values diverse, independent confirmation over concentrated repetition.**

---

## Reach and Replication

Observation reach follows the same progression as content reach (`ContentAttestation` in `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs`):

| Attestation Level | Content Parallel | Observation Equivalent | Trust Weight |
|-------------------|-----------------|----------------------|-------------|
| **Self-attested** | `author-verified` | Author executes own scenario | Low — "I tested my own code" |
| **Single-peer** | `steward-approved` | One independent agent replicates | Moderate — "someone else confirmed" |
| **Multi-peer** | `community-endorsed` | 3+ diverse agents replicate | High — "the community confirms" |
| **Governance-ratified** | `governance-ratified` | Elohim deliberation validates | Highest — "the protocol confirms" |

Self-attestation is valid but has limited reach. An agent can run its own scenarios in its own small pond — but why would you? The economic incentive is for **independent replication**, which carries more trust weight and higher residual payments.

This maps directly to the user's insight: "You could validate your own scenarios in your small pond, but why would you... not worth it." The economics make self-validation a poor investment compared to seeking independent replication.

---

## Adversarial Resilience

### What Prevents Fake Replications?

The same governance mechanisms that protect all protocol content:

**1. Steward standing.** Agents who produce false observations — claiming a scenario passes when it doesn't, or failing to replicate what others can confirm — lose standing in the steward system. Standing loss is economic: lower standing means lower allocation, less recognition, reduced reach for future observations.

**2. Distribution health observation.** The feedback design includes a `distribution-health` instrument archetype. If one agent produces >40% of observations for a domain, the `concentration-above-threshold` negative observation fires automatically. This is a protocol-level Sybil defense: concentration itself is observable and penalized.

**3. Diversity weighting.** Even if an adversary creates multiple agent identities, the diversity score weights temporal spread and doorway diversity. Spinning up 10 agents on the same machine in the same minute produces low-diversity observations with minimal trust weight.

**4. Replication reproducibility.** Observations include execution metadata (doorway ID, device type, step results, duration). An observation that claims a 50-step scenario passed in 1 millisecond is structurally implausible. Elohim agents can flag anomalous observation patterns.

**5. Reach governance.** Observations start with limited reach (self or community). Reaching `commons` requires governance ratification. An adversary can pollute their local observation space, but cannot inflate their observations to protocol-wide reach without governance approval.

### The Cost-Benefit of Adversarial Attestation

The key insight: adversarial attestation in a small pond has **no audience and no economic return**. The observation is only valuable when it reaches steward pools and elohim pools — and reaching those pools requires the diversity and governance validation that adversarial behavior can't fake.

This is fundamentally different from blockchain proof-of-work, where the computation itself is the value. Here, the **independent replication** is the value. You can't replicate independently by yourself.

---

## The Observer Protocol Connection

The Observer Protocol (`genesis/docs/content/elohim-protocol/observer-protocol.md`) defines an **ephemeral witness** architecture:

1. Capture (camera frame, encrypted at sensor)
2. Process (pattern recognition, local only)
3. Generate (REA story elements)
4. Destroy (visual data cryptographically destroyed)
5. Only the structured story remains

The replication protocol applies the same pattern:

1. **Capture**: Execute scenario (run steps, collect results)
2. **Process**: Determine pass/fail, compute CIDs, assess diversity
3. **Generate**: Mint observation EconomicEvent with structured metadata
4. **Destroy**: Discard execution artifacts (logs, screenshots, traces, browser state)
5. **Only the observation persists**: CID, polarity, agent, timestamp, metadata

This is **observation without surveillance**. You know the behavioral claim was tested. You have the structured evidence (who ran it, when, what happened). You don't have a recording of the test execution — just as the Observer Protocol doesn't retain video, only stories.

The parallel goes deeper: the Observer Protocol's privacy switch (hardware-enforced, three positions: active/blind/audio-only) has an analog in the dual-mode executor. `local` mode is the switch in the "blind" position — no observations leave the machine. `attestation` mode is the switch in the "active" position — observations are published to the network.

---

## Observation Lifecycle

### Birth

1. Scenario is authored (`.feature` file created or modified)
2. CID computed from content → scenario enters content graph as `observation-scenario`
3. If `@wip`: scenario is an Intent (unfulfilled observation request)
4. If executable: scenario is available for observation

### Life

1. First execution → initial observation minted (self-attestation)
2. Independent replications → diversity score grows, validity extends
3. Freshness decay → validity slowly diminishes without fresh evidence
4. Periodic re-execution by stewards → validity maintained
5. Invalidation discovered → negative observation minted, discoverer credited

### Death (and Rebirth)

1. Validity drops below threshold → review obligation generated
2. Steward investigates → three paths:
   - **Revalidate**: Fresh positive evidence arrives, obligation dissolves
   - **Correct**: Scenario is updated (new CID), old scenario deprecated via `DERIVED_FROM` link
   - **Retire**: Scenario is no longer relevant, explicitly deprecated
3. Deprecated scenarios retain their observation history — they're not deleted, just no longer active. The history is itself evidence.

---

## Open Questions

1. **Environment prerequisites**: Replication requires equivalent environments. How precisely must a scenario specify its prerequisites? Too precise (exact Docker image hash, database state) and no one can replicate. Too loose (any doorway, any version) and environment differences cause false invalidations. The scenario's `@depends-on:` tags and `Background` steps implicitly define prerequisites, but is that sufficient?

2. **Grace period for failures**: Should a failed replication immediately shorten validity, or should there be a grace period where the protocol asks "is this an environment issue or a real failure?" A single failed replication from a new agent might reflect misconfiguration, not behavioral drift. Possible approach: first negative observation from a new agent is flagged for review rather than immediately weighted.

3. **Observation vs. instrument quality**: The feedback design distinguishes between "the claim is invalid" and "the instrument is bad." How does the replication protocol distinguish "this scenario genuinely fails" from "this agent's test environment is broken"? The diversity score partially addresses this (if 3 agents fail, it's probably real; if 1 of 4 fails, maybe environment), but the heuristic needs refinement.

4. **Cross-version replication**: A scenario that passes at commit A and fails at commit B — is the failure an invalidation of the scenario, or evidence that commit B introduced a regression? Both interpretations are valid. The observation metadata should include a code version reference, but how this factors into validity computation needs design.

5. **Replication scheduling**: Who decides which scenarios to replicate and when? Stewards manually? Automated by freshness decay (re-run whatever's closest to expiring)? Bounty-driven (highest bounty = most replicated)? Probably a combination, with the economics guiding natural allocation.
