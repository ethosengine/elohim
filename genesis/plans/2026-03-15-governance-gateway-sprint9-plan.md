# Sprint 9: Elohim Deliberation — Layer B, Plug in Inference

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build Layer B — the elohim deliberation layer. Elohim agents carry governance dispositions into peer deliberation, traverse the governance hierarchy sensing for constraints and creative settlements, and report outcomes to their humans. This is where you plug in an API key or wire up native inference.

**Architecture:** Each human has an elohim. Each elohim carries its human's GovernanceDisposition. Elohim-to-elohim deliberation is the primary governance arena. Humans opt-in to override, not opt-in to participate. The elohim solves quorum by being a faithful proxy.

**Tech Stack:** Angular 19, TypeScript, elohim-storage Rust backend, Claude API (or local inference), elohim-agent-sdk

**Depends on:** Sprint 8 (dispositions + proxy voting), Sprint 7 (sensemaking data), all previous sprints

---

### Task 1: ElohimGovernanceAgent — the agent prompt template

**Files:**
- Create: `elohim/elohim-storage/src/agents/governance_agent.rs` (or appropriate location)
- Or: `app/elohim-app/src/app/elohim/agents/governance-agent.ts`

Define the governance agent's system prompt:
- Soul document context (from protocol specification)
- Human's GovernanceDisposition (loaded at deliberation time)
- Current proposal context (options, sensemaking results, signals)
- Governance hierarchy constraints (what higher levels have decided)
- Output format: { position: string, justification: string, confidence: float }

This is the soul document IS the base model principle. The prompt carries the whole context so the agent doesn't need hedging.

**Commit:** `feat(elohim): define governance agent prompt template`

---

### Task 2: Inference integration seam — API key or native

**Files:**
- Create: `elohim/elohim-storage/src/agents/inference_client.rs`
- Or create: TypeScript inference client in elohim services

Build an inference client that:
1. Accepts a prompt + context
2. Calls Claude API (or configurable endpoint)
3. Parses structured response
4. Returns typed GovernancePosition

Configuration: API key in environment variable, endpoint URL configurable. Support both:
- Remote API (Claude, etc.) — for hosted deployment
- Local inference endpoint — for Tauri desktop (future)

**Commit:** `feat(elohim): add inference client for governance deliberation`

---

### Task 3: Elohim-to-elohim deliberation protocol

**Files:**
- Create: `elohim/elohim-storage/src/agents/deliberation.rs`

The deliberation flow:
1. Proposal reaches voting phase
2. For each human in the collective who hasn't voted:
   a. Load their GovernanceDisposition
   b. Load sensemaking context (clusters, bridging statements)
   c. Load hierarchy constraints (what higher governance levels require)
   d. Run governance agent inference → get position + justification
3. All elohim positions are collected
4. If consensus: record as proxy votes with justifications
5. If disagreement: elohim "discuss" (multi-turn inference where each sees others' positions)
6. After N rounds or convergence: record final positions as proxy votes
7. If blocks: traverse hierarchy seeking settlement (escalation)

For Sprint 9 MVP: single-turn inference (no multi-turn deliberation). Just position + justification.

**Commit:** `feat(elohim): add elohim deliberation protocol (single-turn MVP)`

---

### Task 4: Hierarchy traversal for settlement

**Files:**
- Create: `elohim/elohim-storage/src/agents/hierarchy_traversal.rs`

When elohim deliberation produces a conflict (block or no consensus):
1. Identify the governance hierarchy level (family → community → bioregional → network)
2. Load constraints from the level above
3. Run inference with additional context: "The {level} governance has determined {constraint}. How does this affect the settlement?"
4. Produce a Settlement: the best outcome that respects higher-level constraints while honoring lower-level desires

```rust
pub struct Settlement {
    pub proposal_id: String,
    pub outcome: String,           // "approved", "modified", "deferred"
    pub justification: String,     // elohim's reasoning
    pub constraints_honored: Vec<String>,
    pub compromises: Vec<String>,
    pub hierarchy_levels_consulted: Vec<String>,
}
```

**Commit:** `feat(elohim): add hierarchy traversal for governance settlements`

---

### Task 5: Replace BracketSynthesisService stub (Sprint 7)

**Files:**
- Modify: `app/elohim-app/src/app/qahal/services/bracket-synthesis.service.ts`

Replace the stub with actual inference:
1. Load sensemaking result (clusters, bridging statements)
2. Run governance agent with bracket-synthesis prompt
3. Agent produces: ranked options with justifications for why this bracket structure best serves the community
4. Create proposal with agent-synthesized options

**Commit:** `feat(qahal): replace bracket synthesis stub with inference`

---

### Task 6: DeliberationVisualizationComponent

**Files:**
- Create: `app/elohim-app/src/app/qahal/components/deliberation-visualization/deliberation-visualization.component.ts`

Shows the deliberation process:
- Each elohim's position with justification
- Consensus/disagreement visualization
- Settlement path (if escalation occurred)
- "Your elohim said..." for the current user
- Override option for the current user

This makes governance transparent — you can see why decisions were made.

**Commit:** `feat(qahal): add deliberation visualization component`

---

### Task 7: Replace disposition computation stub (Sprint 8)

**Files:**
- Modify: `elohim/elohim-storage/src/services/disposition_service.rs`

Replace rule-based disposition computation with inference-based:
1. Load human's full governance history (votes, challenges, signals, sensemaking positions)
2. Run inference: "Based on this governance behavior, characterize this person's governance disposition"
3. Parse structured response into GovernanceDisposition fields

**Commit:** `feat(elohim): replace disposition computation with inference`

---

### Task 8: Configuration and deployment

**Files:**
- Modify: deployment configuration
- Add: environment variables for inference API

Configuration:
- `ELOHIM_INFERENCE_ENDPOINT` — API endpoint (default: Claude API)
- `ELOHIM_INFERENCE_API_KEY` — API key
- `ELOHIM_INFERENCE_MODEL` — model to use
- `ELOHIM_DELIBERATION_ENABLED` — feature flag (default: false)
- `ELOHIM_DELIBERATION_MAX_ROUNDS` — max multi-turn rounds (default: 1 for MVP)

**Commit:** `feat(elohim): add inference configuration and feature flags`

---

### Task 9: Tests

- Governance agent prompt template produces valid structured output (mock inference)
- Deliberation protocol: single-turn produces proxy votes
- Hierarchy traversal: produces settlement when constraints exist
- Bracket synthesis with inference (mock)
- Disposition computation with inference (mock)
- Feature flag disables deliberation when off

**Commit:** `test(elohim): add governance deliberation tests`

---

### Task 10: A2O scenarios

- "Elohim deliberates on proposal using human's disposition" — agent infers position from values
- "Elohim-to-elohim deliberation reaches consensus" — proxy votes recorded with justifications
- "Block triggers hierarchy traversal for settlement" — escalation produces settlement
- "Human reviews elohim's deliberation reasoning" — visualization shows position + justification
- "Governance proceeds without human participation" — elohim solves quorum
- "Human overrides after reviewing deliberation" — override with proportional weight

**Commit:** `feat(a2o): add elohim deliberation scenarios`

---

## Summary

| Task | What | Layer |
|------|------|-------|
| 1 | Governance agent prompt template | Agent |
| 2 | Inference client (API key seam) | Infrastructure |
| 3 | Elohim deliberation protocol | Agent |
| 4 | Hierarchy traversal for settlement | Agent |
| 5 | Replace bracket synthesis stub | Service |
| 6 | DeliberationVisualizationComponent | Component |
| 7 | Replace disposition computation stub | Service |
| 8 | Configuration and deployment | Infrastructure |
| 9 | Tests | Testing |
| 10 | A2O scenarios | Scenarios |

---

## Layer B Complete

After Sprint 9, the full three-layer architecture is wired:
- **Layer A** (Sprints 3-6): Feedback mechanisms at the content → signals accumulate
- **Layer C** (Sprint 7): Signals → opinion clusters → bridging statements → brackets
- **Layer B** (Sprints 8-9): Dispositions → elohim deliberation → settlements → humans opt-in to override

The medium IS the message. Governance is experienced, not abstracted.
