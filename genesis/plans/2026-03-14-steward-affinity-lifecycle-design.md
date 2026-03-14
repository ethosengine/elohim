# Steward Affinity Lifecycle Design

_2026-03-14_

## Problem

The recognition pipeline (`recognition_pipeline_service.rs`) hardcodes `stored_affinity: 1.0` and `derived_affinity: 1.0` in Stage 2 (resolve). This means recognition distribution is purely proportional to allocation ratios — affinity has no influence. The pipeline's affinity weighting mechanism exists but receives dummy data.

More fundamentally, there's no lifecycle for steward affinity. Affinity should be earned through mastery and sustained curation work, creating content that is self-governing through the people who genuinely know and care for it.

## Motivation: Anti-Capture Through Earned Standing

Steward affinity is an anti-capture mechanism. On centralized platforms, "ownership" is a single credential — steal the credential, steal the page (e.g., Sheila Wray Gregoire's Facebook Business Page hacked, 90K followers reporting into a void at Meta for months).

In this model, content becomes increasingly hard to capture because:

- **Mastery gate**: You can't build steward affinity without proving deep content understanding
- **Affinity accrual**: Standing is earned through sustained curation work, not granted by a credential
- **Community resistance**: Other stewards have governance standing (qahal) to resist hostile changes
- **No single point of capture**: Stewardship is a web of demonstrated relationships

**Critical distinction**: Learner engagement (attention) does NOT increase steward affinity. That would recreate the attention economy. Affinity only grows through active curation work. Learner attention flows through REA recognition as reciprocal value (tokens/energy) to stewards, but doesn't inflate their governance standing.

## Data Model

New `steward_affinity` table in elohim-storage:

| Column | Type | Purpose |
|--------|------|---------|
| `id` | TEXT PK | UUID |
| `steward_id` | TEXT NOT NULL | Human/presence ID |
| `content_id` | TEXT NOT NULL | ContentNode ID |
| `affinity_score` | REAL NOT NULL | 0.0–1.0 |
| `source` | TEXT NOT NULL | Origin: `genesis_seed`, `mastery_gate`, `curation_edit`, `curation_review`, `dispute_resolution` |
| `created_at` | TEXT NOT NULL | ISO timestamp |
| `updated_at` | TEXT NOT NULL | ISO timestamp |

Unique constraint on `(steward_id, content_id)` — one affinity record per steward-content pair, updated over time.

## Pipeline Integration

### Stage 2 (Resolve) Changes

Currently in `resolve_from_allocations()`:
```rust
stored_affinity: 1.0,  // v0 hardcoded
derived_affinity: 1.0, // v0 hardcoded
```

After wiring:
- `stored_affinity` = `steward_affinity.affinity_score` looked up by `(steward_id, content_id)`. Falls back to **0.0** if no record exists — no affinity means no recognition share. You have to earn it.
- `derived_affinity` stays 1.0. Future slot for computed affinity (network effects, community signals).

### Fallback Behavior

If a steward has an allocation but no affinity record, `stored_affinity = 0.0` zeros out their share. This is intentional — allocation alone doesn't earn recognition. Genesis-seeded stewards always have affinity records, so day-one behavior is correct.

### Test Impact

Existing pipeline tests construct `ResolvedSteward` structs directly with explicit affinity values — they won't break. New tests exercise the DB lookup path.

## Mastery Gate

A learner becomes eligible for stewardship on content X when they achieve mastery on practicable content within X's scope.

```
can_steward(human_id, content_id) -> bool:
  1. Find practicable content nodes within content_id's scope
  2. Check if human_id has mastery-level assessment results on any of them
  3. Return true if mastery exists, false otherwise
```

"Within scope": If the content is an epic-level node, mastery on any child practicable node qualifies. If the content is itself practicable, mastery on that specific node.

The gate is checked before any curation activity can create or update affinity.

## Curation Mutations

### Endpoint

```
POST /api/v1/steward-affinity/curation-event
{
  "steward_id": "...",
  "content_id": "...",
  "activity_type": "edit" | "review" | "dispute_resolution"
}

201: Updated affinity record
403: "mastery gate not met"
```

### Affinity Deltas

| Activity | Delta | Rationale |
|----------|-------|-----------|
| Content edit | +0.10 | Direct curation work |
| Content review | +0.05 | Lighter touch, still valuable |
| Dispute resolution | +0.15 | High-effort governance work |

All capped at 1.0.

## Genesis Seeding

Seed data matching a2o scenarios in `stewardship-allocation.feature`:

| Steward | Content Domain | Seeded Affinity | Rationale |
|---------|---------------|-----------------|-----------|
| Eve | public-observer | 0.85 | Highest affinity per scenario |
| Pete | faith/pastoral | 0.50 | Pastoral affinity per scenario |
| Matthew | fallback/bootstrap | 0.70 | Bootstrap steward, broad coverage |

Values tweakable via genesis vars.

## Constitutional Limits (Stage 4)

Currently a pure passthrough. Implementation:

- **Floor**: Minimum recognition amount (prevents dust distributions)
- **Ceiling**: Maximum share any single steward can receive (prevents concentration)
- **Excess redistribution**: Amounts above ceiling redistributed proportionally to remaining stewards
- **Limit reasons**: Populate existing `LimitReason` enum (`FloorApplied`, `CeilingApplied`, `ExcessRedistributed`) in trace data

## Three Increments

### Increment 1: Storage + Pipeline Wiring
- `steward_affinity` table + diesel migration
- CRUD functions (create, get_by_steward, get_by_content, get_by_steward_and_content, update_score)
- Genesis seed data
- Wire Stage 2 to query real affinity
- Unit tests

### Increment 2: Mastery Gate + Curation Mutations
- Mastery gate check (query assessment results)
- `POST /api/v1/steward-affinity/curation-event` endpoint
- Gate validation + affinity delta application
- Unit + integration tests

### Increment 3: Constitutional Limits (Stage 4)
- Floor/ceiling enforcement in `apply_limits()`
- Excess redistribution
- LimitReason trace population
- Tests for limit scenarios

## Explicitly Not In Scope

- **Decay**: No time-based affinity reduction yet
- **derived_affinity**: Stays 1.0; future network/community signals
- **Angular UI for curation tracking**: Backend only
- **REA coordinator unification**: Existing API clients stay as-is
- **Multi-swimlane distribution**: Future research per distribution models doc
