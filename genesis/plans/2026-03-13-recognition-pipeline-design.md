# Recognition Pipeline Service — Design

**Date**: 2026-03-13
**Status**: Approved
**Scope**: Rust service in elohim-storage for end-to-end recognition distribution

---

## Problem

`StewardshipAllocationService` (Angular) calculates proportional recognition distribution but never creates the economic events that record those distributions. REA API clients (economic-events, exchange, flow-planning) exist as independent thin clients with no unified coordinator. The allocation model is rich but untested (14.8% coverage).

The recognition flow is broken: content events happen, but stewards never receive recorded recognition.

## Decision

Build a `RecognitionPipelineService` in the Rust service layer (`elohim-storage/src/services/`) that orchestrates recognition distribution through composable, testable stages. Angular is narrowly scoped to interactive concerns — all economic logic lives in Rust.

## Architecture

```
POST /db/recognition/distribute
  { contentId, eventType, rawAmount }
         |
  recognition_pipeline_service.rs
         |
  Stage 1: Normalize  (eventType -> weighted amount)
  Stage 2: Resolve    (allocations + steward affinities)
  Stage 3: Weight     (ratio x affinity coefficient)
  Stage 4: Limit      (constitutional floor/ceiling checks)
  Stage 5: Settle     (create economic events per steward)
         |
  Returns: RecognitionDistributionResult
  (per-steward breakdown with full reasoning trace)
```

Each stage is a pure function. The pipeline function chains them. No shared mutable state between stages.

### Angular Side

A thin `RecognitionApiService` in the elohim pillar that calls `POST /db/recognition/distribute` and renders the result. Zero logic.

---

## Stage Details

### Stage 1: Normalize

`fn normalize_trigger(content_id, event_type, raw_amount) -> NormalizedTrigger`

Maps event types to recognition weights via a lookup table:

| event_type | weight | rationale |
|------------|--------|-----------|
| `mastery_completion` | 1.0 | Full learning signal |
| `content_access` | 0.01 | Micro-recognition per view |
| `content_citation` | 0.5 | Referenced by another piece |
| `assessment_attempt` | 0.1 | Attempted but didn't master |

`weighted_amount = raw_amount * weight`

The table is a `HashMap` constant — easy to extend, easy to explain.

### Stage 2: Resolve

`fn resolve_stewards(conn, content_id) -> Vec<ResolvedSteward>`

Queries `stewardship_allocations` for content, resolves two affinity signals per steward:

- **stored_affinity**: From `node_stewardship.affinity_score` if the steward also stewards infrastructure for this content. Default 1.0.
- **derived_affinity**: From the human's profile affinities matched against the content's domain/tags. Default 1.0.

### Stage 3: Weight

`fn apply_weights(stewards, weighted_amount) -> Vec<WeightedShare>`

```
effective_ratio = allocation_ratio * stored_affinity * derived_affinity
// Re-normalize so effective ratios sum to 1.0
share = weighted_amount * (effective_ratio / sum_all_effective_ratios)
```

Re-normalization ensures total distributed equals total available. Affinity shifts distribution between stewards but doesn't create or destroy value.

### Stage 4: Limit

`fn apply_limits(shares) -> Vec<LimitedShare>`

Constitutional bounds per steward:

- **Floor**: If steward is below dignity floor, minimum allocation guaranteed. (v0: passthrough — stage exists as checkpoint.)
- **Ceiling**: If recognition would push steward above accumulation ceiling, cap and redistribute excess.

Returns shares plus `Vec<LimitReason>` for explainability.

### Stage 5: Settle

`fn settle(conn, shares, trigger) -> Vec<EconomicEvent>`

For each steward share, creates an `EconomicEvent`:

- `action`: "produce" (recognition produced for steward)
- `provider`: content ID (source of value)
- `receiver`: steward presence ID
- `resource_classified_as`: ["recognition-distribution"]
- `lamad_event_type`: mirrors original trigger event type
- `content_id`: the content that generated recognition
- `triggered_by`: original trigger event ID
- `metadata`: full pipeline trace (stage outputs, affinity scores, limit reasons)

Updates `recognition_accumulated` on each allocation.

---

## Data Types

### Input

```rust
struct RecognitionTrigger {
    content_id: String,
    event_type: String,       // mastery_completion, content_access, etc.
    raw_amount: f64,
    triggered_by: Option<String>,  // originating event ID
}
```

### Pipeline Intermediates

```rust
struct NormalizedTrigger {
    content_id: String,
    event_type: String,
    raw_amount: f64,
    weight: f64,
    weighted_amount: f64,
}

struct ResolvedSteward {
    steward_presence_id: String,
    allocation_ratio: f32,
    stored_affinity: f64,
    derived_affinity: f64,
    contribution_type: String,
}

struct WeightedShare {
    steward_presence_id: String,
    effective_ratio: f64,
    share_amount: f64,
}

struct LimitedShare {
    steward_presence_id: String,
    pre_limit_amount: f64,
    final_amount: f64,
    limit_reasons: Vec<LimitReason>,
}

enum LimitReason {
    None,
    FloorApplied { floor: f64, original: f64 },
    CeilingApplied { ceiling: f64, excess: f64 },
    ExcessRedistributed { from_steward: String, amount: f64 },
}
```

### Output

```rust
struct RecognitionDistributionResult {
    content_id: String,
    trigger_event_type: String,
    raw_amount: f64,
    weighted_amount: f64,
    distributions: Vec<StageTrace>,
    economic_event_ids: Vec<String>,
    limits_applied: Vec<LimitReason>,
}

struct StageTrace {
    steward_presence_id: String,
    allocation_ratio: f32,
    stored_affinity: f64,
    derived_affinity: f64,
    effective_ratio: f64,
    pre_limit_share: f64,
    final_share: f64,
    limit_reasons: Vec<LimitReason>,
    economic_event_id: String,
}
```

---

## HTTP Endpoint

```
POST /db/recognition/distribute
Content-Type: application/json

{
  "contentId": "epr:abc123",
  "eventType": "mastery_completion",
  "rawAmount": 10.0,
  "triggeredBy": "event-1710345600000-a1b2"
}
```

Response (200):

```json
{
  "contentId": "epr:abc123",
  "triggerEventType": "mastery_completion",
  "rawAmount": 10.0,
  "weightedAmount": 10.0,
  "distributions": [
    {
      "stewardPresenceId": "presence-matthew",
      "allocationRatio": 0.6,
      "storedAffinity": 1.0,
      "derivedAffinity": 0.9,
      "effectiveRatio": 0.574,
      "preLimitShare": 5.74,
      "finalShare": 5.74,
      "limitReasons": [],
      "economicEventId": "event-1710345600001-c3d4"
    }
  ],
  "economicEventIds": ["event-1710345600001-c3d4", "..."],
  "limitsApplied": []
}
```

---

## View Types

Following elohim-storage boundary conventions:

- `RecognitionTriggerInputView` — camelCase input, `Into<RecognitionTrigger>`
- `RecognitionDistributionResultView` — camelCase output, `From<RecognitionDistributionResult>`
- `StageTraceView` — camelCase, `From<StageTrace>`
- ts-rs exports to `sdk/storage-client-ts/src/generated/`

---

## What This Does NOT Include (v0)

- Drips-style cascading through content dependency graphs (future: attribution graph)
- Demurrage or time-based decay of recognition
- Multi-dimensional contribution weighting by context
- Swimlane-specific distribution (all recognition is generic v0)
- Constitutional limit enforcement (stage 4 is passthrough, structure only)

These are documented in `genesis/research/economic/` for future work.

---

## Files Changed

### New Files
- `elohim-storage/src/services/recognition_pipeline_service.rs` — Pipeline service
- `elohim-storage/src/services/recognition_pipeline_service/stages.rs` — Stage functions (optional split)

### Modified Files
- `elohim-storage/src/services/mod.rs` — Add module + re-export
- `elohim-storage/src/http.rs` — Add `/db/recognition/distribute` route
- `elohim-storage/src/views.rs` — Add input/output view types + ts-rs exports

### Generated
- `sdk/storage-client-ts/src/generated/RecognitionTriggerInputView.ts`
- `sdk/storage-client-ts/src/generated/RecognitionDistributionResultView.ts`
- `sdk/storage-client-ts/src/generated/StageTraceView.ts`

---

## Testing Strategy

Unit tests for each stage function (pure functions, easy to test):
- Normalize: weight lookup, unknown event type fallback
- Resolve: single steward, multiple stewards, no allocations
- Weight: re-normalization math, affinity coefficients
- Limit: passthrough (v0), structure validation
- Settle: economic event creation, recognition accumulation

Integration test: full pipeline from trigger to economic events in SQLite.
