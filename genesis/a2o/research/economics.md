# Observation Royalties: The Economics of Distributed Observation

**Status:** Research
**Date:** 2026-03-30
**Depends on:** [replication-protocol.md](replication-protocol.md)
**Connects to:** [REA Economics Skill](../../.claude/skills/rea-economics/SKILL.md)

---

## The Economic Problem

Traditional testing has no economics. Tests are written because process requires them. Tests are maintained because CI breaks when they're not. Tests are never rewarded for discovering that something previously true is no longer true.

This creates three pathologies:

1. **Testing is a cost center.** Developers write the minimum tests needed to satisfy coverage thresholds. Nobody invests in scenario quality because there's no return on that investment.

2. **Maintenance is uncompensated.** When a test breaks, someone fixes it as a cost of doing business. The person who maintains 200 scenarios for a year receives the same recognition as someone who wrote 0.

3. **Invalidation is punished.** A failing test blocks the build. The person who discovers the failure is associated with the disruption, not credited with the discovery. The incentive is to make tests pass, not to make tests honest.

The observation protocol solves all three by treating observations as **economic events** with value that flows to creators, executors, stewards, and discoverers.

---

## The Four-Phase Royalty Model

Like book royalties: a signing bonus for creation, a delivery advance for first publication, and ongoing residuals for as long as the work generates value. Plus a fourth phase unique to observation: **invalidation rewards** for discovering that a claim no longer holds.

### Phase 1: Bounty (Scenario Creation)

An agent authors a new `observation-scenario` ContentNode.

```json
{
  "action": "produce",
  "provider": "protocol:observation-pool",
  "receiver": "{author-agent}",
  "resourceConformsTo": "observation-bounty",
  "resourceQuantityValue": 1.0,
  "lamadEventType": "observation-bounty",
  "metadata": {
    "scenarioCid": "{scenario-cid}",
    "domain": "content",
    "coverageGap": "high"
  }
}
```

**Bounty sizing:** Not all scenarios are equally valuable. The gap analysis tool (`genesis/a2o/scripts/scan-coverage.ts`) already identifies uncovered domains and content. Bounties are weighted by coverage gap — scenarios that validate uncovered content command higher bounties than scenarios duplicating already-observed behavior.

**Activation gate:** Bounties don't activate on creation alone. The scenario must be accepted into a domain by a steward (`steward-consent` governance model). This prevents bounty farming — you can't mint value by writing trivial scenarios that no steward endorses.

### Phase 2: Delivery (First Passing Execution)

The scenario passes for the first time — the hypothesis has evidence.

```json
{
  "action": "produce",
  "provider": "protocol:observation-pool",
  "receiver": "{executor-agent}",
  "resourceConformsTo": "observation-delivery",
  "resourceQuantityValue": 1.0,
  "lamadEventType": "observation-delivery",
  "metadata": {
    "scenarioCid": "{scenario-cid}",
    "executionCid": "{observation-cid}",
    "firstPass": true
  }
}
```

**Who receives delivery:** The agent who first executes the scenario and it passes. This may be the author (combined bounty + delivery for self-validation) or a different agent (delivery goes to executor, bounty stays with author). Separating creation from execution incentivizes **independent validation** — the delivery is worth more when it comes from someone other than the author.

**Complexity weighting:** A 2-step scenario on HTTP produces less delivery value than a 15-step browser-mode federation scenario. Delivery amount factors in:
- Step count
- Device type (Playwright > HTTP > mock)
- Multi-doorway involvement
- Prerequisite chain depth

### Phase 3: Residual (Ongoing Stewardship)

Each re-execution that confirms validity generates a small residual payment. **This phase never ends.** As long as someone is stewarding the scenario — re-running it, maintaining it against code changes, keeping it relevant — value flows.

```json
{
  "action": "transfer",
  "provider": "protocol:observation-pool",
  "receiver": "{split-recipients}",
  "resourceConformsTo": "observation-residual",
  "resourceQuantityValue": 0.1,
  "lamadEventType": "observation-residual",
  "metadata": {
    "scenarioCid": "{scenario-cid}",
    "replicationNumber": 7,
    "split": {
      "author": 0.25,
      "executor": 0.25,
      "stewardPool": 0.30,
      "elohimPool": 0.20
    }
  }
}
```

**Residual splits:** Each residual payment is divided among four recipients:

| Recipient | Share | Rationale |
|-----------|-------|-----------|
| **Author** | 25% | Created the behavioral claim — ongoing credit for the intellectual contribution |
| **Executor** | 25% | Performed the labor of replication — computation, environment setup, time |
| **Steward pool** | 30% | Domain health fund — pays for curation, quality review, infrastructure |
| **Elohim pool** | 20% | Protocol governance — sustains the observation network itself |

Split ratios are governed by the domain's governance model. Different communities may weight differently — a research community might give more to authors, a production community might give more to executors.

**The book royalty parallel:** An author writes a book (bounty). It gets published (delivery). Every time someone reads it, a small royalty flows back (residual). The publisher (steward pool) and the industry (elohim pool) also receive shares. The book generates value for as long as people read it. The scenario generates value for as long as agents replicate it.

### Phase 4: Invalidation (Discovery Reward)

The most important economic innovation. An agent discovers that a previously-valid scenario no longer holds.

```json
{
  "action": "produce",
  "provider": "protocol:observation-pool",
  "receiver": "{discovering-agent}",
  "resourceConformsTo": "observation-invalidation",
  "resourceQuantityValue": 2.0,
  "lamadEventType": "observation-invalidation",
  "metadata": {
    "scenarioCid": "{scenario-cid}",
    "previousValidity": 0.85,
    "failingStep": "Then the content should be retrievable by ID",
    "failingStepCid": "{step-cid}",
    "priorPositiveObservations": 12,
    "isFirstNegative": true
  }
}
```

**Why invalidation is the highest-value phase:**

- A positive observation confirms what we already believed — informational value is low
- A negative observation reveals something **new** — informational value is high
- The first negative observation after a string of positives is the highest-value observation of all — it discovered behavioral drift that no one else caught

**Invalidation bounty scaling:** The more confident the protocol was in the scenario's validity (high validity score, many prior positive observations), the more valuable the invalidation discovery. Finding a crack in a well-trusted scenario is worth more than finding a crack in a scenario that was already shaky.

This directly inverts the traditional incentive: in CI, a failing test is a problem to fix. In the observation protocol, a failing replication is a **contribution to knowledge** — rewarded, not punished.

---

## Currency Swimlane

Observation economics flow through the existing currency swimlanes defined in the REA economics model:

| Swimlane | Observation Application |
|----------|----------------------|
| **Time** | Execution time (cpu-seconds, wall-clock) consumed by observation |
| **Care** | Scenario authoring — human attention invested in expressing behavioral claims |
| **Infrastructure** | Compute resources for execution (doorway, browser, network) |
| **Learning** | Observations that improve understanding of system behavior |
| **Creator** | Original scenario authorship — the intellectual contribution |

A single observation may generate events across multiple swimlanes. A browser-mode scenario execution consumes Infrastructure (compute) and produces Learning (observation), with the author receiving Creator recognition and the executor receiving Time compensation.

---

## The Commons Default

Following the established commons pool pattern in the REA economics model:

**Scenarios authored without explicit steward assignment flow to the community pool by default.** This is critical for cold-start economics:

1. **Anyone can write a scenario** → it belongs to the commons immediately
2. **Stewards curate domains** → they claim scenarios relevant to their expertise
3. **Claimed scenarios redirect value** → from commons pool to steward + author
4. **Unclaimed scenarios still generate value** → residuals flow to elohim pool

This means the observation network starts generating value from day one, before any steward has claimed any scenario. The commons default eliminates the bootstrapping problem: you don't need governance to be fully operational before observations produce economic events.

The pattern mirrors the existing protocol design: "Unattributed content flows to commons pool (community-stewarded) by default. When steward attests ('I wrote/maintain this'), recognition transfers to them." The incentive is to find your scenarios, claim them, and receive the value flow.

---

## Coverage-Driven Bounty Pricing

The existing gap analysis tool (`genesis/a2o/scripts/scan-coverage.ts`) produces a coverage gap report identifying which content domains lack behavioral observation. This report directly drives bounty pricing:

| Coverage Level | Bounty Multiplier | Rationale |
|---------------|-------------------|-----------|
| **Uncovered** (0 scenarios for content domain) | 3x | Highest priority — no behavioral evidence exists |
| **Sparse** (1-2 scenarios, all `@wip`) | 2x | Hypotheses exist but no instruments |
| **Partial** (some executable, gaps remain) | 1.5x | Incremental coverage valuable |
| **Well-covered** (>5 passing scenarios) | 1x | Baseline bounty |
| **Saturated** (>15 scenarios, high diversity) | 0.5x | Diminishing returns — marginal scenario adds little trust |

This creates a **market signal** for observation investment: agents are economically guided toward the domains with the least behavioral evidence. The gap analysis becomes a bounty board.

---

## The Self-Sustainability Question

For the observation economics to sustain themselves without external subsidy, observation creation must generate more value than observation execution costs. Is this realistic?

### The Value Proposition

The value is **not in the test passing** — it's in the **trust generated by independent replication**:

- **Avoided regressions**: An observation that catches a behavioral drift before it reaches production saves orders of magnitude more than the observation cost
- **Governance confidence**: Proposals backed by diverse observation coverage are more trustworthy — governance participants invest more confidence in evidence-backed decisions
- **Learning grounding**: Content backed by behavioral observations has pedagogical value — "this concept is verified by 7 passing scenarios" is meaningful to learners
- **Agent accountability**: LLM agents whose claims are observation-backed are more trustworthy — the observation network is infrastructure for AI interpretability

### The Proof-of-Work Parallel

This mirrors proof-of-work economics: the computation itself is "wasted" (electricity burned), but the trust it generates is the product. In the observation protocol:

- The execution itself is "wasted" (compute consumed running scenarios)
- The **independent replication** is the product (distributed trust)
- The trust is valuable because it's expensive to fake (requires real diversity)

The difference from blockchain PoW: observation work produces **useful side effects** (bug discovery, regression detection, coverage data) in addition to trust. The observation isn't purely artificial scarcity — it's real information.

### Sustainability Conditions

The economics are self-sustaining when:

1. **Pool inflow > pool outflow**: Observation events generate fees (from content consumption, governance actions, or protocol operations) that replenish the observation pool
2. **Trust value > execution cost**: The trust generated by observations is worth more to the network than the compute consumed producing them
3. **Invalidation prevents losses**: Early detection of behavioral drift saves more than the invalidation reward costs

Condition 1 requires the observation pool to be funded — by governance allocation, protocol fees, or value capture from the trust substrate. This is a design decision for Sprint 4.

---

## Shefa Integration

Observation economics are **accounted for by the shefa pillar**, not by a2o. a2o produces observations; shefa manages the economic events. This follows the existing separation of concerns: lamad produces learning signals, shefa accounts for the value.

### New Resource Types in Shefa Vocabulary

Added to the shefa domain manifest:

| Resource Type | Phase | Description |
|--------------|-------|-------------|
| `observation-bounty` | Creation | One-time payment for scenario authorship |
| `observation-delivery` | First pass | One-time payment for first successful execution |
| `observation-residual` | Ongoing | Recurring micro-payment per replication |
| `observation-invalidation` | Discovery | Variable payment for discovering behavioral drift |
| `scenario-validity` | Accumulation | The observation itself — validity as a measurable resource |

### Integration with Existing Services

The shefa pillar already has:

- `EconomicEventsApiService` — routes all economic events
- `PoolManagementService` (stub) — manages steward and community pools
- `CoreContributorService` (stub) — tracks contributor presence and attribution

The observation minter calls `EconomicEventsApiService` to publish observation events. Pool distribution uses `PoolManagementService` to split residuals. Author attribution uses `CoreContributorService` to track who created and maintains each scenario.

These services are currently stubs defining API surfaces. The observation protocol provides a concrete use case for implementing them — observations are a well-defined, measurable flow of economic events with clear pool distribution rules.

---

## Open Questions

1. **Pool funding**: How is the observation pool funded? Options: protocol reserve allocation (governance-decided), transaction fee on other economic events (sustainable but adds friction), value capture from trust substrate (the trust itself is the funding). This is the most critical design decision for Sprint 4.

2. **Residual decay**: Should residual payments decrease over time (like diminishing book royalties) or remain constant? Constant residuals incentivize long-term stewardship. Decaying residuals incentivize fresh scenario creation. A hybrid model (constant for active scenarios, decaying for stale ones) might be optimal.

3. **Split governance**: Who decides the residual split ratios? Protocol-level defaults with per-domain governance overrides seems right. But how do split changes apply to existing scenarios? Retroactively (disrupting established flows) or only to new scenarios (creating split version fragmentation)?

4. **Invalidation gaming**: Could an agent deliberately break a system, then "discover" the failure to collect invalidation rewards? The observation metadata includes agent identity — if the agent who introduced the regression is the same agent who discovers the failure, the protocol should flag this as suspicious, not reward it.

5. **Cross-domain observation value**: A scenario in the auth domain that validates auth lifecycle has direct value. A scenario in the shefa domain that validates economic settlement has direct value. But an auth scenario also has *indirect* value to shefa (shefa depends on auth). How does cross-domain observation value flow? This is the cascading attribution problem from the value distribution research (`genesis/research/README.md`).
