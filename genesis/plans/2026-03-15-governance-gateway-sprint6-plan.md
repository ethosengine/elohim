# Sprint 6: Signal Accumulation — Graduated Feedback, Reactions & REA Flow

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build the low-friction governance signals (emotional reactions, graduated feedback scales) that accumulate over time and feed into sensemaking. Wire the REA pipeline so governance participation generates economic events and builds steward affinity.

**Architecture:** Signals flow: User interaction → governance_signals table → REA economic event → recognition pipeline → steward affinity. Accumulated signals become the input for Sprint 7's Polis sensemaking layer.

**Tech Stack:** Angular 19, TypeScript, elohim-storage Rust backend

**Depends on:** Sprint 4 (gateway component), Sprint 3 (signals backend)

---

### Task 1: EmotionalReactionComponent — rich reactions

**Files:**
- Create: `app/elohim-app/src/app/qahal/components/emotional-reaction/emotional-reaction.component.ts`

Reaction types from governance-deliberation.model.ts: moved, grateful, inspired, hopeful, challenged, concerned, uncomfortable. Each maps to a `governance_signal` with `signal_type: "reaction"` and `signal_value: "{reaction_type}"`.

UI: Small icon bar, expandable. Click to react. Second click to remove. Show aggregate counts.

**Commit:** `feat(qahal): add emotional reaction component`

---

### Task 2: GraduatedFeedbackSelectorComponent — context-aware scales

**Files:**
- Create: `app/elohim-app/src/app/qahal/components/graduated-feedback-selector/graduated-feedback-selector.component.ts`

Renders the appropriate scale from governance-deliberation.model.ts based on FeedbackContext:
- Accuracy: {inaccurate, mostly-inaccurate, uncertain, mostly-accurate, accurate}
- Usefulness: {not-useful, slightly, moderately, very, essential}
- Proposal position: {strongly-disagree, disagree, abstain, agree, strongly-agree}
- Label agreement: {disagree, uncertain, agree}

Negative feedback (bottom 2 options) requires reasoning text.

Submits as `governance_signal` with `signal_type: "graduated"`, `signal_value: "{scale}:{value}"`, `mechanism_level: 2`.

**Commit:** `feat(qahal): add graduated feedback selector component`

---

### Task 3: FeedbackAggregateComponent — display signal distribution

**Files:**
- Create: `app/elohim-app/src/app/qahal/components/feedback-aggregate/feedback-aggregate.component.ts`

Shows aggregate distribution of graduated feedback for an entity:
- Horizontal bar chart per scale option
- Total respondents count
- Consensus strength indicator (standard deviation of responses)
- "N people found this accurate" summary

Loads via GovernanceApiService.getSignals() and aggregates client-side.

**Commit:** `feat(qahal): add feedback aggregate visualization component`

---

### Task 4: Signal-to-REA pipeline integration

**Files:**
- Modify: voting components and reaction/feedback components
- Modify or use: `app/elohim-app/src/app/elohim/services/recognition.service.ts` (or create if missing)

After each governance signal submission (vote, reaction, feedback), fire an REA economic event:
- Resource: the signal itself
- Event type: "governance-participation"
- Agent: the human who submitted
- Trigger: POST to `/api/v1/recognition/distribute`

Map mechanism levels to recognition weight:
- Level 0-1 (reactions): low weight
- Level 2-3 (feedback, approval): medium weight
- Level 4-6 (ranked-choice, score, consent): high weight
- Level 7 (deliberation): highest weight

This makes governance participation a first-class stewardship activity.

**Commit:** `feat(qahal): wire governance signals to REA recognition pipeline`

---

### Task 5: Signal accumulation threshold detection

**Files:**
- Create: `app/elohim-app/src/app/qahal/services/signal-accumulation.service.ts`

This service monitors signal counts for entities and detects when thresholds are crossed:
- N signals received → "ready for sensemaking" flag
- Divergent signals detected → "controversy detected" flag
- High consensus → "settled" indicator

These flags prepare the ground for Sprint 7's Polis sensemaking. For now, they surface as indicators on the governance summary.

Uses `GovernanceApiService.getSignals()` and `count_signals()` backend function.

**Commit:** `feat(qahal): add signal accumulation threshold service`

---

### Task 6: Integrate reactions and feedback into gateway

**Files:**
- Modify: `FeedbackMechanismGatewayComponent` (Sprint 4 Task 3)

Replace stubs at levels 1 and 2:
- Level 1: render `EmotionalReactionComponent`
- Level 2: render `GraduatedFeedbackSelectorComponent` + `EmotionalReactionComponent`

Add `FeedbackAggregateComponent` below the feedback selector (shows what others thought).

**Commit:** `feat(qahal): integrate reactions and graduated feedback into gateway`

---

### Task 7: Backend — signal aggregation query

**Files:**
- Modify: `elohim/elohim-storage/src/db/governance.rs`
- Modify: `elohim/elohim-storage/src/api/governance.rs`

Add `GET /signals/aggregate?entityType=X&entityId=Y` route that returns grouped signal counts:
```json
{
  "totalSignals": 42,
  "byType": { "reaction": 25, "graduated": 12, "vote": 5 },
  "byValue": { "accurate": 8, "mostly-accurate": 3, "inspired": 15, ... },
  "consensusStrength": 0.72
}
```

This avoids transferring all raw signals to the client for aggregation.

**Commit:** `feat(storage): add signal aggregation query route`

---

### Task 8: Tests

- EmotionalReactionComponent: renders reactions, click toggles, shows counts
- GraduatedFeedbackSelectorComponent: renders correct scale per context, validates reasoning
- FeedbackAggregateComponent: renders bar chart from signal data
- SignalAccumulationService: threshold detection logic
- Signal-to-REA integration: verify economic event created after signal

**Commit:** `test(qahal): add signal accumulation and feedback tests`

---

### Task 9: A2O scenarios

- "Learner reacts to content with emotional response" — reaction recorded as signal
- "Learner rates content accuracy with graduated feedback" — feedback with reasoning
- "Feedback aggregate shows community consensus" — distribution visualization
- "Governance participation earns recognition" — REA event → affinity delta
- "Signal accumulation triggers sensemaking readiness" — threshold crossed

**Commit:** `feat(a2o): add signal accumulation and feedback scenarios`

---

## Summary

| Task | What | Layer |
|------|------|-------|
| 1 | EmotionalReactionComponent | Component |
| 2 | GraduatedFeedbackSelectorComponent | Component |
| 3 | FeedbackAggregateComponent | Component |
| 4 | Signal-to-REA pipeline | Integration |
| 5 | Signal accumulation threshold service | Service |
| 6 | Integrate into gateway | Integration |
| 7 | Backend signal aggregation | Rust |
| 8 | Tests | Testing |
| 9 | A2O scenarios | Scenarios |
