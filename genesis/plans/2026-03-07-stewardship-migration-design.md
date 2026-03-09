# Stewardship Fat Service Migration Design

**Date**: 2026-03-07
**Status**: Approved
**Scope**: Delete Angular `StewardshipService` (932 lines), add Rust storage API endpoints

## Problem

The Angular `StewardshipService` is a fat service that calls Holochain zome functions directly from the browser. The v0 migration pattern moves business logic to elohim-storage behind the `/api/v1/*` boundary. A thin `StewardshipApiService` already exists but is incomplete — it implements 11 of the 18 methods needed by consumers.

### Consumer Analysis

| Component | Methods Used | On Interface? |
|-----------|-------------|---------------|
| appeal-wizard | `getMyStewards()`, `fileAppeal()` | Yes |
| capabilities-dashboard | `getMyPolicy()`, `getMyStewards()`, `checkTimeAccess()` | Missing 1 |
| policy-console | `getGrantForSubject()`, `getPolicyChain()`, `getSubjectPolicy()`, `getParentPolicy()`, `getMyPolicy()`, `getMyPolicyChain()`, `upsertPolicy()` | Missing 5 |
| community-intervention | None (dead injection) | N/A |

### Missing Methods (7)

1. `checkTimeAccess()` — time-based access with remaining session/daily
2. `getGrantForSubject(subjectId)` — computable client-side from `getMySubjects()`
3. `getSubjectPolicy(subjectId)` — get active device policy for subject
4. `getParentPolicy(subjectId)` — get parent's computed policy
5. `getPolicyChain(subjectId)` — full policy inheritance chain
6. `getMyPolicyChain()` — my own policy chain
7. `upsertPolicy(input)` — create/update device policy

## Design

### 1. SQLite Schema (v4→v5 Migration)

New `device_policies` table in `schema.rs`:

```sql
CREATE TABLE IF NOT EXISTS device_policies (
    id TEXT PRIMARY KEY NOT NULL,
    subject_id TEXT NOT NULL,
    device_id TEXT,
    author_id TEXT NOT NULL,
    author_tier TEXT NOT NULL DEFAULT 'self',
    inherits_from TEXT,
    blocked_categories_json TEXT NOT NULL DEFAULT '[]',
    blocked_hashes_json TEXT NOT NULL DEFAULT '[]',
    age_rating_max TEXT,
    reach_level_max INTEGER,
    session_max_minutes INTEGER,
    daily_max_minutes INTEGER,
    time_windows_json TEXT NOT NULL DEFAULT '[]',
    cooldown_minutes INTEGER,
    disabled_features_json TEXT NOT NULL DEFAULT '[]',
    disabled_routes_json TEXT NOT NULL DEFAULT '[]',
    require_approval_json TEXT NOT NULL DEFAULT '[]',
    log_sessions INTEGER NOT NULL DEFAULT 0,
    log_categories INTEGER NOT NULL DEFAULT 0,
    log_policy_events INTEGER NOT NULL DEFAULT 1,
    retention_days INTEGER NOT NULL DEFAULT 30,
    subject_can_view INTEGER NOT NULL DEFAULT 1,
    effective_from TEXT NOT NULL,
    effective_until TEXT,
    version INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_device_policies_subject ON device_policies(subject_id);
CREATE INDEX IF NOT EXISTS idx_device_policies_author ON device_policies(author_id);
```

The `inherits_from` column creates a linked list for policy chain traversal. Each link has an `author_tier` (self, guide, guardian, coordinator, constitutional) and a `layerOrder` derived from tier position.

### 2. Rust API Endpoints (7 new routes)

| Method | Route | Purpose | Data Source |
|--------|-------|---------|-------------|
| POST | `/policies` | Upsert device policy | `device_policies` table |
| GET | `/policies?subjectId=X` | List policies for subject | device_policies query |
| GET | `/policies/{subjectId}` | Get subject's active policy | Most recent for subject |
| GET | `/policies/{subjectId}/parent` | Get parent's computed policy | Walk `inherits_from` |
| GET | `/policies/{subjectId}/chain` | Full policy chain | Walk chain → `PolicyChainLink[]` |
| GET | `/policies/me/chain` | My policy chain | Same, using `?agentId=` |
| GET | `/access/time` | Time access check | Existing `PolicyEnforcement::check_time_access()` |

The `access/time` endpoint reuses the existing `PolicyEnforcement` engine in `policy_cache.rs` (already implemented, tested).

After policy upsert, the service recomputes the merged `CachedPolicy` and updates via `PolicyCache::save_policy()`.

### 3. Rust Layer Structure

Following the existing elohim-storage architecture (CLAUDE.md):

- **`db/device_policies.rs`** — Diesel schema, models, CRUD queries
- **`views.rs`** — `DevicePolicyView`, `UpsertPolicyInputView`, `PolicyChainLinkView`, `TimeAccessView` with `From` conversions
- **`services/stewardship_service.rs`** — New methods: `upsert_policy()`, `get_policies_for_subject()`, `get_policy_chain()`, `compute_merged_policy()`
- **`api/stewardship.rs`** — New handler functions wired in dispatcher

### 4. Policy Chain Computation

Merge algorithm for computing `CachedPolicy` from device policy chain:

- **Arrays** (blockedCategories, disabledFeatures, disabledRoutes): union across all tiers
- **Scalars** (sessionMaxMinutes, reachLevelMax): most restrictive wins (lowest number)
- **Booleans** (logSessions, logCategories): OR (if any tier enables, it's enabled)
- **Tier order**: constitutional (4) → coordinator (3) → guardian (2) → guide (1) → self (0)

Higher tiers can only ADD restrictions, never remove them. This matches the stewardship model: "power scales with responsibility."

### 5. Angular Changes

1. **Expand `IStewardshipPolicy`** — add 7 method signatures
2. **Expand `StewardshipApiService`** — 7 new HTTP methods (~100 lines added)
3. **Switch 3 consumers** to `inject(STEWARDSHIP_POLICY)` token:
   - `appeal-wizard.component.ts`
   - `policy-console.component.ts`
   - `capabilities-dashboard.component.ts`
4. **Remove dead injection** from `community-intervention.component.ts`
5. **Delete fat service**: `stewardship.service.ts` (932 lines) + `stewardship.service.spec.ts` (89 tests)
6. **Update 4 spec files** — swap providers from class to token
7. **`getGrantForSubject()`** computed client-side: filters `getMySubjects()` result

### 6. Files Modified/Created

**Rust (elohim-storage) — ~800 lines added:**
- `src/db/schema.rs` — v4→v5 migration
- `src/db/device_policies.rs` — NEW: Diesel schema + CRUD
- `src/db/mod.rs` — register module
- `src/services/stewardship_service.rs` — expand with policy methods
- `src/views.rs` — add DevicePolicy views
- `src/api/stewardship.rs` — 7 new route handlers

**Angular (elohim-app) — ~932 lines deleted:**
- `src/app/imagodei/interfaces/stewardship-policy.interface.ts` — expand
- `src/app/imagodei/services/stewardship-api.service.ts` — expand
- `src/app/imagodei/services/stewardship.service.ts` — DELETE
- `src/app/imagodei/services/stewardship.service.spec.ts` — DELETE
- `src/app/imagodei/components/appeal-wizard/appeal-wizard.component.ts` — switch injection
- `src/app/imagodei/components/policy-console/policy-console.component.ts` — switch injection
- `src/app/imagodei/components/capabilities-dashboard/capabilities-dashboard.component.ts` — switch injection
- `src/app/imagodei/components/community-intervention/community-intervention.component.ts` — remove dead injection
- 4 corresponding `.spec.ts` files — update providers

## Risks

- **Policy chain without grants**: Chain is built from `device_policies.inherits_from`, not from a `stewardship_grants` table. When Holochain DHT is available, grants will authorize policy creation. For v0, any authenticated user can upsert policies.
- **Merge algorithm**: The "most restrictive wins" merge is straightforward but needs unit tests for edge cases (null vs Some, empty arrays).
- **Recomputation**: After upsert, the cached policy must be recomputed. If this fails, the stale cache may not reflect the new policy. Mitigated by fail-open defaults.
