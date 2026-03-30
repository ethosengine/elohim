# The Distributed Observation Protocol

**Status:** Research
**Date:** 2026-03-30
**Predecessor:** [Feedback Information Flows Design](../../plans/2026-03-28-feedback-information-flows-design.md)

---

## The Insight

Given/When/Then is not a testing pattern. It is an **observation pattern**.

- **Given** describes a precondition — a claim about the state of the world before intervention
- **When** describes an action — an agent's intervention in that world
- **Then** describes a postcondition — a claim about how the world changed

This is the structure of any observation: precondition, action, postcondition. Cucumber happened to apply it to software testing. The grammar is more general than its current use. It describes how an observer witnesses a state transition — in code, in a classroom, in a board meeting, in a household.

The Elohim Protocol already has the infrastructure for content-addressed observations (EPR), economic events (REA), peer validation (DHT), and obligation accumulation (feedback design). What it lacks is a **human-readable grammar for structured behavioral claims** that agents can resolve, peers can replicate, and economics can sustain.

Gherkin is that grammar. The fork makes it protocol-native.

---

## The Organizing Concept: Interpretability

The central problem this protocol solves is **interpretability** — but not in the way AI research usually means that word.

**Mechanistic interpretability** looks inside the model to understand its reasoning. Hard, expensive, incomplete. You get circuit-level explanations no human can act on.

**Chain-of-thought interpretability** asks the model to narrate its reasoning. Easily confabulated. The model can say "I did X because Y" while actually doing X because Z. There is no verification.

**Behavioral interpretability through peer-attested observation** is a third path. The agent doesn't need to explain its internal reasoning. It needs to produce a **scenario** — a structured claim about what it did — that other agents can independently replicate. The interpretability isn't in the model's internals. It's in the **observation protocol** that surrounds its outputs.

A Gherkin scenario is already a minimal interpretable unit of behavioral evidence:
- A human reads it and understands what was claimed
- An agent resolves the CIDs and verifies whether the claim replicates
- The economic layer makes honest observation sustainable

The grammar fork exists because **interpretability requires a shared language between humans and agents** — structured enough for machines to verify, natural enough for humans to read. Gherkin is already that language. The protocol makes it verifiable.

---

## What Exists Today

### The a2o Harness

The existing `genesis/a2o/` directory contains a mature BDD test harness:

- **30 feature files** across 9 domain directories (auth, browser, content, deployment, elohim, federation, lamad, qahal, shefa)
- **100+ scenarios** covering auth lifecycle, learning journeys, content lifecycle, EPR content addressing, federation, governance, human resilience, compute allocation
- **24 step definition files** — 14 API-level, 10 browser-level (Playwright)
- **Framework infrastructure**: E2EWorld (`src/framework/world.ts`), DoorwayClient (`src/framework/api/doorway-client.ts`), BrowserDevice, PlaywrightDevice, StewardDevice, 10+ page objects, 120+ selectors
- **Coverage tools**: gap analysis scanner (`scripts/scan-coverage.ts`), step skeleton generator (`scripts/generate-step-skeletons.ts`)
- **Close-loop workflow**: dev-intent.jsonl captures implementation intent; `/close-loop` generates scenarios from intent; `/gap-analysis` identifies coverage holes; `/generate-scenarios` fills them
- **6 Cucumber profiles**: default, alpha, local, browser, genesis, testnet

### What It Cannot Do

| Limitation | Consequence |
|-----------|-------------|
| Scenarios are files in git | Not addressable as protocol content. Cannot be referenced by learning paths, governance proposals, or economic events. |
| Execution produces test reports | HTML/JSON reports die in CI artifacts. No economic event. No value flow. No feedback loop. |
| Single attester (CI runner) | Results trusted because Matthew's machine ran them. No independent replication. No diversity of observation. |
| No connection to feedback design | The approved claims/observations/obligations model has no scenario instrument. |
| No economic incentive | No reward for scenario creation, maintenance, or invalidation discovery. Testing is a cost center, not a value generator. |
| Developer-only audience | Non-developers cannot see, understand, or participate in behavioral verification. |

---

## The Protocol Vision

### 1. Scenarios as ContentNodes

Each `.feature` file becomes a ContentNode with `contentType: "observation-scenario"`. Each scenario within it gets a CID computed from its content. Each step (Given/When/Then) gets a CID.

The scenario's three-leg coupling:

- **Knowledge**: `VALIDATES` relationships to the content nodes it tests. `DEPENDS_ON` relationships to prerequisite scenarios. `DERIVED_FROM` for forked/evolved scenarios.
- **Value**: `onExecute` produces `EconomicEvent` with `action="observe"`. Observations are resource flows — provider is the executing agent, receiver is the scenario CID, resource is `scenario-validity`.
- **Governance**: `reach` determines who can see and replicate the observation. `governanceModel: "steward-consent"` means stewards curate which scenarios enter their domain.

This makes scenarios first-class citizens of the content graph. A learning path can reference the scenarios that validate its content. A governance proposal can require observation coverage before ratification. An elohim agent can trace a learner's mastery to the behavioral evidence that grounds it.

### 2. Execution as Economic Event

The existing signal harness (`signal-harness.service.ts`) translates `RendererCompletionEvent` → `CreateEconomicEventInput`. The observation minter applies the same pattern:

```
ScenarioCompletionEvent → CreateEconomicEventInput
```

The resulting economic event:

```json
{
  "action": "observe",
  "providerId": "{executing-agent-pub-key}",
  "receiverId": "{scenario-cid}",
  "resourceConformsTo": "scenario-validity",
  "resourceQuantityValue": 1.0,
  "lamadEventType": "scenario-observation",
  "metadata": {
    "polarity": "positive",
    "observation": "behavioral-claim-holds",
    "executionDuration": 4200,
    "stepsExecuted": 5,
    "stepsPassed": 5,
    "replicationOf": null
  }
}
```

This is not a new pipeline. It flows through the existing REA infrastructure — `EconomicEventsApiService`, DHT notarization, storage projection. The observation is just another economic event, denominated in a new resource type.

### 3. Replication as Scientific Method

The scientific replication model redefines what "testing" means:

| Scientific Concept | Scenario Equivalent |
|-------------------|---------------------|
| Hypothesis | Authored scenario (Given/When/Then claim) |
| Experiment | First execution by author |
| Replication | Independent execution by different agent |
| Confirmation | Independent execution passes |
| Disconfirmation | Independent execution fails — a discovery |
| Retraction | Scenario validity drops below threshold |

This connects directly to the feedback design's obligation accumulation:

- **Positive observations** (scenario passes) extend the validity horizon
- **Negative observations** (scenario fails) shorten the validity horizon
- When validity drops below threshold → **review obligation** generated
- Three escalation paths: automated revalidation, steward correction, elohim escalation

The critical inversion: **finding that a scenario no longer holds is rewarded, not punished**. In traditional CI, a failing test is a problem to fix. In the observation protocol, a failure-as-discovery is a contribution — it surfaces real information about behavioral drift.

### 4. Agent Complexity Absorption

The dual surface:

**What humans see**: Pure Gherkin. No CIDs, no economic bindings, no governance metadata in the feature file.

```gherkin
@e2e @content
Feature: Content Lifecycle

  Scenario: Create and retrieve content
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And human "Matthew" is logged in on doorway "alpha"
    When Matthew creates content titled "Governance Basics"
    Then the content should be created successfully
```

**What agents resolve**: A ContentNode with CID, three-leg coupling, relationship edges, economic bindings, replication history, governance metadata. All computed from the feature file content and protocol context — never authored by humans.

This parallels the EPR model: the Head (~500 bytes, gossipped) is the human-readable summary. The Document and Bytes are the protocol infrastructure that makes it work. The interpretability contract: **the human-readable surface IS the interpretable claim**. Everything else is agent-resolved substrate.

---

## The Observer Protocol Connection

The Observer Protocol (`genesis/docs/content/elohim-protocol/observer-protocol.md`) describes ephemeral witness for physical reality:

1. Camera captures frame (encrypted at sensor)
2. Streams to local node (never leaves your network)
3. Elohim processes for patterns
4. Generates REA story elements
5. Visual data cryptographically destroyed
6. Only structured story remains

The observation protocol applies the **same pattern** to behavioral claims:

1. Executor runs scenario (captures execution state)
2. Processes for pass/fail (pattern recognition)
3. Generates structured observation (EconomicEvent)
4. Execution artifacts destroyed (logs, screenshots, traces are ephemeral)
5. Only the observation persists

This is observation without surveillance. You know the behavioral claim was tested. You have the structured evidence. You don't have a recording of the test — just as the Observer Protocol doesn't retain video, only stories.

When wired to the Observer epic, the observation surface **expands from code behavior to any claim an agent makes about reality**. A scenario like:

```gherkin
Scenario: Community garden produces weekly harvest
  Given the Elm Street garden has 12 active plots
  When the weekly harvest is observed
  Then at least 8 plots report yield
  And the community food access score increases
```

...is the same grammar, the same protocol, the same economics. The executor is different (an Observer node instead of a CI runner), but the observation is the same: a structured claim, content-addressed, economically incentivized, peer-verifiable.

---

## The LLM Verification Layer

This is where the interpretability thesis has its most practical application.

Today, when an LLM agent says "I completed the task," the verification options are:

1. **CI** — centralized, binary, limited scope
2. **Human review** — doesn't scale
3. **Trust the agent** — catastrophically naive

With the observation protocol, an agent's claim becomes a **scenario execution** — a structured observation that other agents can replicate:

1. Agent claims "I implemented feature X"
2. The claim is anchored to a scenario CID: "here's the behavioral evidence"
3. Peer agents attempt replication: "can I independently reproduce this observation?"
4. Self-attestation has limited reach (parallels `ContentAttestation`: author-verified has lower reach than governance-ratified)
5. Trust weight comes from **independent replication** — diverse agents confirming the observation

This is **proof-of-work for behavioral claims**, but the "work" is observation, not computation. An agent that consistently self-attests accurately builds a verification reputation. An agent that self-attests falsely gets flagged by replication failures.

The scenario is the interpretability layer between what an agent did and what a human can verify. Not "trust me, I did it" — but "here's the structured claim, go reproduce it." **Peer-grading of one's own homework**, with economics that make honesty sustainable.

---

## What This Is Not

- **Not a replacement for unit tests.** Unit tests stay local, fast, developer-only. They don't need content addressing or economics. The observation protocol is for behavioral claims that matter beyond one developer's machine.

- **Not a blockchain for test results.** Observations are economic events in the existing REA pipeline, not a separate chain. They flow through the same infrastructure as all other value.

- **Not requiring all scenarios on-chain.** The dual-mode executor means local/mock mode for development (zero overhead) and attestation mode for peer-verified observations. Most development runs are local. Only published observations hit the DHT.

- **Not changing the Gherkin grammar.** Humans write the same Given/When/Then they always have. Protocol metadata is agent-resolved, never author-declared.

---

## Open Questions

1. **Observation validity and release cycles**: A scenario valid at v1.2 may be invalid at v1.3. How does the validity horizon interact with software release cycles? Should observations be version-scoped?

2. **Minimum viable observation**: What is the smallest useful observation for Sprint 0? Just CID addressing (scenarios in the content graph), or does even Sprint 0 need economic event wiring?

3. **Genesis profile composition**: The `genesis` cucumber profile already pulls conceptual scenarios from `genesis/docs/content/`. How do observation scenarios compose with these? Are genesis scenarios the "hypotheses" that a2o observations validate?

4. **Observation granularity**: Should observations be per-scenario or per-step? Per-scenario is simpler; per-step enables finer-grained claim tracking but increases DHT entry volume.

5. **Environment reproducibility**: Replication requires that independent agents can set up equivalent environments. The scenario's `DEPENDS_ON` edges must include environment prerequisites — but how precisely? Too precise and no one can replicate; too loose and replications are meaningless.

6. **The naming question**: This is no longer Cucumber. It's a distributed observation protocol that uses Gherkin as its human-readable surface. What is the right name? Working title: **a2o-protocol** (alpha-to-omega — acceptance to observation).
