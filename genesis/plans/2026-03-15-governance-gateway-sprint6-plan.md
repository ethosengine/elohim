# Sprint 6: Signal Accumulation — Aggregation, Thresholds & Sensemaking Prep (v2)

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build the signal aggregation and threshold detection that feeds Sprint 7's Polis sensemaking. Most of the signal collection infrastructure (reactions, graduated feedback, REA events) was already built in Sprint 4. This sprint adds the server-side aggregation, client-side visualization, and threshold service that detects when an entity is "ready for sensemaking."

**Already built (Sprint 4):**
- ReactionBarComponent wired to governance signals API (mechanism_level 1)
- GraduatedFeedbackComponent wired to governance signals API (mechanism_level 2)
- GovernanceRecognitionService for REA economic events on participation
- FeedbackMechanismGateway renders reactions at level 1, graduated feedback at level 2
- governance_signals table + CRUD + GET/POST routes (Sprint 3)

**What remains:**
- Backend signal aggregation query (avoid transferring raw signals to client)
- FeedbackAggregateComponent (distribution visualization)
- SignalAccumulationService (threshold detection for sensemaking readiness)
- Tests and A2O scenarios

**Tech Stack:** Rust (Diesel, SQLite), Angular 19, TypeScript

**Depends on:** Sprint 4 (signal collection), Sprint 3 (signals backend)

---

### Task 1: Backend — signal aggregation query and view

**Files:**
- Modify: `elohim/elohim-storage/src/db/governance.rs`
- Modify: `elohim/elohim-storage/src/api/governance.rs`
- Modify: `elohim/elohim-storage/src/views.rs`
- Modify: `elohim/elohim-storage/src/http.rs`

Add a new view type:

```rust
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct SignalAggregateView {
    pub entity_type: String,
    pub entity_id: String,
    pub total_signals: i64,
    pub by_type: HashMap<String, i64>,       // {"reaction": 25, "graduated": 12, "vote": 5}
    pub by_value: HashMap<String, i64>,      // {"accurate": 8, "inspired": 15, ...}
    pub unique_participants: i64,
    pub consensus_strength: f64,             // 0.0-1.0, higher = more agreement
}
```

Add CRUD function `aggregate_signals(conn, entity_type, entity_id) -> SignalAggregateView` that:
1. Counts total signals
2. Groups by signal_type
3. Groups by signal_value
4. Counts distinct human_ids
5. Computes consensus_strength: if all signals agree on one value → 1.0, evenly distributed → 0.0. Use normalized entropy: `1.0 - (entropy / max_entropy)`.

Add HTTP route: `GET /signals/aggregate?entityType=X&entityId=Y`

Run `cargo test export_bindings` to generate `SignalAggregateView.ts`.

**Commit:** `feat(storage): add signal aggregation query with consensus strength`

---

### Task 2: GovernanceApiService — aggregation method

**Files:**
- Modify: `app/elohim-app/src/app/elohim/services/governance-api.service.ts`

Add:
```typescript
getSignalAggregate(entityType: string, entityId: string): Promise<SignalAggregateView>
```

Import `SignalAggregateView` from `@elohim/storage-client/generated`.

**Commit:** `feat(qahal): add signal aggregation API method`

---

### Task 3: FeedbackAggregateComponent — distribution visualization

**Files:**
- Create: `app/elohim-app/src/app/qahal/components/feedback-aggregate/feedback-aggregate.component.ts`

Standalone, inline template, signal-based.
Input: `entityType`, `entityId` (both required).

On init, load aggregate via `GovernanceApiService.getSignalAggregate()`.

Template shows:
- Total signal count and unique participants
- **By type breakdown:** "25 reactions, 12 feedback, 5 votes" as a horizontal stacked bar
- **By value distribution:** For each signal value, a horizontal bar showing count + percentage
  - Color-coded: positive values (green), neutral (gray), negative (amber)
- **Consensus strength meter:** 0-100% arc or bar with label "Low agreement" / "Moderate consensus" / "Strong consensus"
- **Sensemaking readiness:** If totalSignals > threshold (e.g. 20) and consensus_strength < 0.7, show "Diverse perspectives — ready for sensemaking"

Keep styling simple — CSS bars, no charting library needed.

Add to qahal barrel exports.

**Commit:** `feat(qahal): add feedback aggregate visualization component`

---

### Task 4: SignalAccumulationService — threshold detection

**Files:**
- Create: `app/elohim-app/src/app/qahal/services/signal-accumulation.service.ts`

```typescript
export interface AccumulationStatus {
  totalSignals: number;
  uniqueParticipants: number;
  consensusStrength: number;
  readyForSensemaking: boolean;    // enough signals + enough diversity
  controversyDetected: boolean;    // low consensus with many signals
  settled: boolean;                // high consensus, stable over time
}

@Injectable({ providedIn: 'root' })
export class SignalAccumulationService {
  private readonly governanceApi = inject(GovernanceApiService);

  async getAccumulationStatus(entityType: string, entityId: string): Promise<AccumulationStatus>
}
```

Thresholds (configurable via constants):
- `SENSEMAKING_MIN_SIGNALS = 20` — minimum signals before sensemaking is meaningful
- `SENSEMAKING_MAX_CONSENSUS = 0.7` — if consensus is too high, sensemaking isn't needed
- `CONTROVERSY_MIN_SIGNALS = 10` — minimum signals to detect controversy
- `CONTROVERSY_MAX_CONSENSUS = 0.3` — low consensus = controversy
- `SETTLED_MIN_SIGNALS = 30` — minimum signals for settled
- `SETTLED_MIN_CONSENSUS = 0.85` — high consensus = settled

These thresholds prepare the ground for Sprint 7. The sensemaking layer activates when `readyForSensemaking` is true.

Add to qahal barrel exports.

**Commit:** `feat(qahal): add signal accumulation threshold service`

---

### Task 5: Integrate aggregate into gateway

**Files:**
- Modify: `app/elohim-app/src/app/qahal/components/feedback-mechanism-gateway/feedback-mechanism-gateway.component.ts`

Add `FeedbackAggregateComponent` below the level 1-2 feedback components. It shows "what others thought" — the aggregate distribution of signals for this entity.

Also load `AccumulationStatus` and show a subtle indicator when sensemaking is ready:
- If `readyForSensemaking`: show "Diverse perspectives on this content — sensemaking available" link
- If `controversyDetected`: show "Active discussion" badge
- If `settled`: show "Community consensus" badge

**Commit:** `feat(qahal): integrate signal aggregate and accumulation status into gateway`

---

### Task 6: Tests

**Files:**
- Create: `app/elohim-app/src/app/qahal/services/signal-accumulation.service.spec.ts`

Test SignalAccumulationService threshold logic:
- Below min signals → not ready, not controversial, not settled
- 25 signals, consensus 0.5 → readyForSensemaking true
- 15 signals, consensus 0.2 → controversyDetected true
- 35 signals, consensus 0.9 → settled true
- Edge cases: exactly at thresholds

Also test consensus_strength computation in the Rust service if time permits.

Run: `pnpm exec vitest run --config vite.config.ts "signal-accumulation"`

**Commit:** `test(qahal): add signal accumulation threshold tests`

---

### Task 7: A2O scenarios

**Files:**
- Modify: `genesis/a2o/features/qahal/collective-governance.feature`

Scenarios:
- "Signal aggregate shows community feedback distribution" — byType and byValue breakdown displayed
- "Consensus strength indicator reflects agreement level" — high consensus = strong indicator
- "Signal accumulation triggers sensemaking readiness" — 20+ signals with diverse opinions
- "Controversy detected on divisive content" — low consensus badge appears
- "Content reaches settled status through consensus" — high consensus, stable

**Commit:** `feat(a2o): add signal accumulation and aggregate scenarios`

---

## Summary

| Task | What | Layer |
|------|------|-------|
| 1 | Backend signal aggregation + consensus strength | Rust |
| 2 | GovernanceApiService aggregation method | Angular service |
| 3 | FeedbackAggregateComponent | Angular component |
| 4 | SignalAccumulationService | Angular service |
| 5 | Integrate into gateway | Integration |
| 6 | Tests | Testing |
| 7 | A2O scenarios | Scenarios |
