# Testnet Lifecycle Design: Protocol-Native Compute Allocation

**Date**: 2026-03-09
**Status**: Approved
**Sprint target**: Proof of concept (5 conductors, full lifecycle)
**Next sprint**: Full REA cycle, 20 conductors, dual-xeon box

## Context

We need conductors spawned for a2o test runs to scale down after tests finish. But rather than building throwaway ops scripts, every lifecycle event emits CoordinationEnvelopes using the vocabulary a protocol-native researcher would use to request community compute. The ops cleanup is a side-effect of economic settlement.

## Design Decisions

1. **Parallel approach (C)**: Ops cleanup is immediate (shell scripts), but every spawn/stop emits CoordinationEnvelopes and EconomicEvents using the same verbs the protocol would use.
2. **Session-scoped with TTL (C)**: Conductors spawn on first `@testnet` scenario, stay alive across features, hard TTL + budget ceiling as safety net. Cucumber `AfterAll` is the graceful path.
3. **Coordination-envelope-wrapped (B→C)**: Each lifecycle phase emits envelopes with REA-shaped payloads. Not full REA cycle yet (no shefa service layer), but payloads are structured so next sprint pipes them into the DHT without restructuring.
4. **Matthew's story**: The researcher is Matthew-the-fixture-human. Same admin credentials, cluster primary, stewardship annotations.
5. **Cucumber report attachment (B)**: Compute telemetry appears alongside scenario pass/fail in the HTML report. Next sprint: seed to DHT for shefa UX rendering.

## Lifecycle Architecture

```
Scenario start (first @testnet tag)
  → ServiceRequest envelope (provision verb)
  → spawn-persona-testnet.sh start N
  → compute-budget.sh watch (background, 10s interval)

Scenario execution
  → sense envelopes emitted per sample
  → budget check per sample (soft warn at 80%, hard kill at 100%)

Graceful shutdown (cucumber AfterAll)
  → compute-budget.sh settle → EconomicEvent envelopes
  → spawn-persona-testnet.sh stop
  → archive ledger + attach summary to cucumber report

Forced shutdown (TTL or budget exceeded)
  → budget circuit breaker SIGTERMs offending node
  → emits EconomicEvent with action: 'budget-exceeded'
  → remaining nodes continue (partial degradation, not full abort)
  → settle on remaining nodes at AfterAll
```

## Matthew's Story Scenario

```gherkin
@testnet @compute-allocation
Feature: Community compute allocation
  As Matthew, I have a distributed app to test.
  I request compute from my community, peers provision
  capacity, my test runs, and settlement happens.

  Background:
    Given doorway "alpha" is healthy at env "E2E_DOORWAY_ALPHA"
    And human "Matthew" is logged in on doorway "alpha"

  @e2e
  Scenario: Matthew requests compute from 5 community peers
    Given Matthew has a simulation requiring 5 peer nodes
    When he submits a ServiceRequest with budget 1800 cpu-seconds
    Then a provision envelope is emitted for each persona
    And 5 conductors are running within 30 seconds
    And compute-budget tracking is active

  @e2e
  Scenario: Compute settles after simulation completes
    Given 5 conductors are running for Matthew's simulation
    When the simulation workload completes
    Then a settle envelope is emitted for each persona
    And each EconomicEvent contains cpu-seconds and memory-mb
    And the total spend is within the 1800 cpu-second budget
    And the compute summary appears in the test report

  @e2e @circuit-breaker
  Scenario: Budget exceeded triggers graceful degradation
    Given 5 conductors are running for Matthew's simulation
    And one persona is configured with a 60 cpu-second budget
    When that persona exceeds its budget
    Then it receives SIGTERM with a budget-exceeded envelope
    And the remaining 4 conductors continue
    And settlement records the partial delivery
```

## Coordination Envelope Payloads

### Provision (spawn)

```json
{
  "verb": "provision",
  "scope": { "agents": ["human-matthew-manager", "...4 peers"] },
  "routing": { "urgency": "near-realtime", "fallback": "queue" },
  "payload": {
    "serviceRequest": {
      "resourceQuantity": { "value": 1800, "unit": "cpu-second" },
      "duration": { "value": 30, "unit": "minute" },
      "trustFloor": "Community"
    }
  },
  "sender": { "agentId": "matthew", "delegationChain": [] }
}
```

### Sense (sample)

```json
{
  "verb": "sense",
  "payload": {
    "computeMetrics": {
      "persona": "human-susan-household",
      "cpuSeconds": 42.3,
      "memoryMb": 87,
      "budgetRemaining": { "cpu": 317.7, "memory": 63 }
    }
  }
}
```

### Settle (shutdown — two variants)

Graceful:
```json
{
  "verb": "settle",
  "payload": {
    "economicEvent": {
      "action": "deliver-service",
      "provider": "human-susan-household",
      "resourceQuantity": { "value": 180, "unit": "cpu-second" },
      "settlement": "pending"
    }
  }
}
```

Budget exceeded:
```json
{
  "verb": "settle",
  "payload": {
    "economicEvent": {
      "action": "budget-exceeded",
      "provider": "human-pete-pastor",
      "resourceQuantity": { "value": 365, "unit": "cpu-second" },
      "budgetLimit": 360,
      "settlement": "partial"
    }
  }
}
```

## Implementation Scope

### This Sprint

| Component | Work | Location |
|-----------|------|----------|
| `compute-budget.sh` | TTL watchdog, budget circuit breaker (SIGTERM), envelope JSON emission | `elohim-node/simulation/` |
| `spawn-persona-testnet.sh` | `spawn-subset` command (5 of 20), provision/settle envelopes on start/stop | `elohim-node/simulation/` |
| Cucumber testnet hooks | Session-scoped spawn in `BeforeAll` for `@testnet`, teardown + settle in `AfterAll`, report attachment | `genesis/a2o/src/framework/` |
| Step definitions | 3 Matthew scenarios wired to shell scripts | `genesis/a2o/src/steps/` |
| Feature file | `compute-allocation.feature` | `genesis/a2o/features/elohim/` |
| Envelope adapter | Validates compute-budget.sh JSON output against CoordinationEnvelope interfaces | `genesis/a2o/src/framework/` |
| Report attachment | Post-settle summary in cucumber JSON/HTML report | `genesis/a2o/src/framework/` |

### Next Sprint (Dual-Xeon Box)

- Shefa service layer processing real ServiceRequests
- DHT seeding of economic events
- Full REA cycle (Request → Commitment → Event → Settlement)
- Scale to 20 conductors
- Phase 2 full Holochain conductors (~1.2GB each)
- Mutual credit settlement

### YAGNI

- No k8s integration — bare processes
- No compute allocation UI — envelopes are data, UX later
- No mutual credit — `settlement: 'pending'` placeholder
