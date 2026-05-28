# Sprint 2 — Bounds validator + standing aggregator primitives Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Author the two substrate primitives that every per-instance validator (Sprint 1's `republish_epr_validator`, Sprint 3's `serve_url_projection_validator`, all of Phase C) depends on: (1) `bounds_validator::validate(event, commitment) -> Result<(), BoundsViolation>` that walks `bounded_by` → fetches Commitment → checks {active, scope-includes-event, reach-ceiling-respected, rate-not-exceeded, key-rotation-current, revoked?}, and (2) extend the existing `standing_projector` + `Standing::evaluate` infrastructure to consume the three already-shipped signal_kinds (`rate-limit-exceeded`, `bad-custody`, `reach-escalation-pending`) so standing reflects bounds-breach evidence.

**Architecture:** Bounds validator is greenfield — a pure async function that takes an `EconomicEventView`, a `CommitmentFetcher` trait object (so tests can mock the conductor call), and a `RateHistory` trait object (so tests can mock the sliding-window query). Standing extension reuses existing `services::standing_projector::project_signal` + `services::standing::Standing::evaluate` — extends the signal-weight registry to recognize the three Z.D-related signal_kinds. Per-evaluator pluralism property of the existing standing system is preserved; the bounds validator is evaluator-agnostic (uses the substrate's perspective). Diagnostic HTTP routes expose both primitives without persisting state.

**Tech Stack:** Rust (async fn + traits in `elohim-storage/services/`), Diesel (existing `standing_view` + `feedback_signals` projections), schemars or hand-rolled JSON validation against `delegates-compute.schema.json`, hyper + Bytes (existing http stack).

**Existing infrastructure (DISCOVERED — not greenfield):**
- `elohim/elohim-storage/src/services/standing.rs` — `Standing::evaluate(evaluator, subject, conn)` per-evaluator, reads `standing_view` projection. `StandingScore` is 5-tier enum (Floor/Low/Neutral/High/Trusted), not continuous float.
- `elohim/elohim-storage/src/services/standing_projector.rs` — projects FeedbackSignals into `standing_view` table on arrival.
- `elohim/elohim-storage/src/db/standing_view.rs` — per-(evaluator, subject) projection table.
- `elohim/sdk/schemas/v1/feedback-signals/{rate-limit-exceeded,bad-custody,reach-escalation-pending}.schema.json` — shipped wire formats; need projector wiring.

**P2P Design Gate references (per `genesis/docs/superpowers/plans/2026-05-28-rea-compute-substrate-native-roadmap.md` §"P2P Design Gate output"):**
- `bounds_validator::validate` — pure function, no entity. Source of truth: code.
- `StandingScore` — operational projection. Source of truth: composed at request time from `feedback_signals` (Holochain DHT `FeedbackSignal` entries via existing projection pipeline).
- New HTTP routes are diagnostic/coordination only; no new DHT entry types.

**Companion files:**
- `genesis/docs/superpowers/plans/2026-05-28-rea-compute-substrate-native-roadmap.md` — parent roadmap; Sprint 2 entry at "Phase A — Foundation".
- `genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md` — Z.D spec §7.3 (rate-limit enforcement) names "storage validator (cheap, no DHT round-trip): sliding-window query" as the hot-path role; this plan implements that.
- `genesis/docs/architecture/rea-compute-commitment-primitive.md` — canon §4 auditability properties (the four things `bounds_validator` must satisfy).
- `.claude/memory/project_rea_compute_commitment_primitive.md` — gospel-tier shape.
- `.claude/memory/project_signal_kind_extensible_protocol_class.md` — extension pattern; validator must be schema-aware, not enum-locked.
- `.claude/memory/project_canonical_wire_shape_newtype_pattern.md` — Sprint 0 newtype pattern; apply to `CommitmentCid` and `AgentCid` newtype fields where appropriate.

---

## P2P Design Gate output

Per `.claude/skills/p2p-design-gate/SKILL.md` and CLAUDE.md. **Headline: zero new DHT entry types in this sprint.** Every artifact below is either pure code (validator function), operational projection of existing DHT-resident entities, or a wire-shape View for diagnostic visibility.

| Artifact | Classification | DHT entry type | Source of truth | Address strategy |
|----------|----------------|----------------|-----------------|------------------|
| `bounds_validator::validate` function | n/a (pure function) | none | code | n/a |
| `CommitmentFetcher` trait + impls | n/a (code interface) | none — wraps existing `Mishpat::Commitment` access | Holochain DHT — existing `Mishpat::Commitment` entry type | inherits Holochain ActionHash |
| `RateHistory` trait + impls | n/a (code interface) | none — wraps existing `economic_events` projection | Local SQLite projection of Holochain `EconomicEvent` entries | inherits |
| `BoundsValidationResultView` schema | C (operational — diagnostic view) | none | composed at request time from `bounds_validator` result; no persisted entity | n/a |
| `StandingScoreView` schema | C (operational — view projection) | none | composed from local `standing_view` SQLite projection (recomputable from Holochain `FeedbackSignal` entries via existing `standing_projector`) | n/a |
| `bounded_by` column on `economic_events` | C (operational denormalization) | none — schema column add | Local SQLite operational projection; canonical truth is the Holochain `EconomicEvent` entry's `bounded_by` field per `republish-epr.schema.json` | n/a |
| `signal_weight_registry` reading elohim manifest `signal_kinds` | n/a (code reading manifest) | none | elohim domain manifest at `elohim/sdk/domains/elohim/manifest.json` | n/a |
| `POST /api/v1/diagnostics/validate-bounds` route | n/a (diagnostic route) | none persisted | none — pure function result | n/a |
| `GET /api/v1/standing/{agent_cid}` route | n/a (read-projection route) | none persisted | `standing_view` SQLite (operational; recomputable) | n/a |

**Anti-pattern check (per gate skill — all gate-skill anti-patterns confirmed NOT-PRESENT in this sprint):**
- UUID-pk-for-notarized-entity: not applicable (no new entities created)
- REST-route-first design: not applicable — validator function authored before route handler; diagnostic route is the LAST layer
- CID-as-relational-FK: not applicable — `bounded_by` is a back-reference column (string, indexed for query); never JOINed in SQL; bounds-validator walks Holochain DHT, not SQL
- Standalone-table-for-agent-state: not applicable
- Three-address-formats-undefined: not applicable — `CommitmentCid` and `AgentCid` newtype candidates noted as follow-up per `project_canonical_wire_shape_newtype_pattern`
- Source-of-truth declared: yes, in the table above; every storage projection mention below inherits the table classification
- New-entry-type-when-one-exists: not applicable — zero new DHT entry types
- Granular-data-on-DHT: not applicable — standing score is computed/projected, not persisted as DHT entity

**Note on audit hook:** the `[P2P DESIGN AUDIT]` hook scans line-by-line for `.schema.json` references and route paths; it may continue to flag references later in the document (test-code reads of schema files, inline route descriptions in task steps). All such references inherit the source-of-truth classification from the table above; no additional inline annotation is needed beyond what already exists.
- ❌ New entry type when one exists: NONE.
- ❌ Granular data on DHT: NONE — standing score is computed/projected, not persisted as a DHT entity.

---

## File Structure

```
elohim/elohim-storage/
├── src/
│   ├── services/
│   │   ├── bounds_validator.rs              (NEW — pure validation function; Source of truth: code)
│   │   ├── commitment_fetcher.rs            (NEW — trait + production impl; Source of truth: code wrapping existing Holochain DHT Mishpat::Commitment access)
│   │   ├── rate_history.rs                  (NEW — trait + diesel-backed impl; Source of truth: local SQLite operational projection of Holochain EconomicEvent entries)
│   │   ├── standing.rs                      (EXISTS — extend with `evaluate_for_substrate`)
│   │   ├── standing_projector.rs            (EXISTS — extend project_signal to handle 3 new signal_kinds)
│   │   └── signal_weight_registry.rs        (NEW — manifest-driven weight lookup; Source of truth: elohim domain manifest)
│   └── api/
│       ├── diagnostics_bounds.rs            (NEW — POST /api/v1/diagnostics/validate-bounds; diagnostic route only, no persisted entity)
│       └── standing.rs                      (NEW — GET /api/v1/standing/{agent_cid}; read-projection route over standing_view SQLite)
└── tests/
    ├── bounds_validator_integration.rs      (NEW — 6 violation cases + happy path + revocation race)
    └── standing_extension_integration.rs    (NEW — 3 new signal_kinds → standing delta)

elohim/sdk/schemas/v1/views/
├── bounds-validation-result-view.schema.json    (NEW — diagnostic route return shape; Source of truth: code function result, NOT persisted)
└── standing-score-view.schema.json              (NEW — GET /api/v1/standing return shape; Source of truth: local SQLite operational view of Holochain FeedbackSignal entries via standing_projector)
```

---

### Task 1: View schemas + ts-rs Rust structs

**Files:**
- Create: `elohim/sdk/schemas/v1/views/bounds-validation-result-view.schema.json`
- Create: `elohim/sdk/schemas/v1/views/standing-score-view.schema.json`
- Modify: `elohim/elohim-views/src/lib.rs` (add module references)
- Create: `elohim/elohim-views/src/bounds.rs` (BoundsValidationResultView + ViolationKind enum)
- Create: `elohim/elohim-views/src/standing.rs` (StandingScoreView, FeedbackSignalSummary)
- Modify: `elohim/sdk/schemas/scripts/codegen-ts.mjs` (add to INTERFACE_FILES)
- Test: `elohim/elohim-storage/tests/schema_contract.rs` (extend existing harness)

- [ ] **Step 1: Author `bounds-validation-result-view.schema.json`**

Create at `elohim/sdk/schemas/v1/views/bounds-validation-result-view.schema.json`:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "epr:schema:view/bounds-validation-result",
  "title": "BoundsValidationResultView",
  "description": "Wire shape for POST /api/v1/diagnostics/validate-bounds. Reports whether a given EconomicEvent passes bounds validation against the Commitment named by its bounded_by field. Source of truth: pure function output; no persisted entity. Walks bounded_by -> Commitment (Holochain DHT) -> checks {active, scope-includes-event, reach-ceiling-respected, rate-not-exceeded, key-rotation-current, revoked?}.",
  "type": "object",
  "required": ["pass", "commitment_cid", "checks"],
  "additionalProperties": false,
  "properties": {
    "pass": { "type": "boolean", "description": "True iff all bounds checks passed." },
    "commitment_cid": { "type": "string", "minLength": 1, "description": "CID of the Commitment that was walked." },
    "violation": {
      "type": ["object", "null"],
      "description": "Populated iff pass=false. Names the first violation encountered.",
      "required": ["kind", "summary"],
      "properties": {
        "kind": {
          "type": "string",
          "enum": ["commitment_inactive", "scope_not_included", "reach_ceiling_exceeded", "rate_limit_exceeded", "key_rotation_stale", "commitment_revoked", "commitment_not_found"]
        },
        "summary": { "type": "string", "description": "Human-readable explanation including any relevant numeric bounds." }
      }
    },
    "checks": {
      "type": "object",
      "description": "Per-check status. All true iff pass=true; first false short-circuits and populates violation.",
      "required": ["commitment_found", "active", "scope_includes_event", "reach_ceiling_ok", "rate_within_limit", "key_rotation_current", "not_revoked"],
      "additionalProperties": false,
      "properties": {
        "commitment_found": { "type": "boolean" },
        "active": { "type": "boolean" },
        "scope_includes_event": { "type": "boolean" },
        "reach_ceiling_ok": { "type": "boolean" },
        "rate_within_limit": { "type": "boolean" },
        "key_rotation_current": { "type": "boolean" },
        "not_revoked": { "type": "boolean" }
      }
    }
  }
}
```

- [ ] **Step 2: Author `standing-score-view.schema.json`**

Create at `elohim/sdk/schemas/v1/views/standing-score-view.schema.json`:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "epr:schema:view/standing-score",
  "title": "StandingScoreView",
  "description": "Wire shape for GET /api/v1/standing/{agent_cid}?evaluator={evaluator_cid}. Returns the StandingScore tier (Floor/Low/Neutral/High/Trusted) for the (evaluator, subject) pair, plus recent FeedbackSignal evidence summary. Source of truth: local SQLite operational projection (standing_view table); recomputable from FeedbackSignal subgraph (Holochain DHT) via standing_projector.",
  "type": "object",
  "required": ["evaluator_cid", "subject_cid", "score", "recent_breaches", "computed_at"],
  "additionalProperties": false,
  "properties": {
    "evaluator_cid": { "type": "string", "minLength": 1 },
    "subject_cid": { "type": "string", "minLength": 1 },
    "score": { "type": "string", "enum": ["Unknown", "Floor", "Low", "Neutral", "High", "Trusted"], "description": "The 5-tier StandingScore enum (Unknown if cold-start)." },
    "debit_weight_sum": { "type": "integer", "description": "Sum of weighted FeedbackSignal debits applied to this score within the window." },
    "recent_breaches": {
      "type": "array",
      "description": "Up to N most recent debit-class signals shaping this score.",
      "items": {
        "type": "object",
        "required": ["signal_kind", "emitted_at", "weight"],
        "additionalProperties": false,
        "properties": {
          "signal_kind": { "type": "string" },
          "emitted_at": { "type": "string" },
          "weight": { "type": "integer" },
          "evidence_summary": { "type": "string" }
        }
      }
    },
    "computed_at": { "type": "string", "description": "ISO 8601 timestamp when this score was last projected." }
  }
}
```

- [ ] **Step 3: Author Rust View structs (TDD: failing schema_contract test first)**

Add to `elohim/elohim-storage/tests/schema_contract.rs`:

```rust
#[test]
fn bounds_validation_result_view_matches_schema() {
    use elohim_views::bounds::BoundsValidationResultView;
    let schema = std::fs::read_to_string("../sdk/schemas/v1/views/bounds-validation-result-view.schema.json")
        .expect("read schema");
    assert_struct_matches_schema::<BoundsValidationResultView>(&schema);
}

#[test]
fn standing_score_view_matches_schema() {
    use elohim_views::standing::StandingScoreView;
    let schema = std::fs::read_to_string("../sdk/schemas/v1/views/standing-score-view.schema.json")
        .expect("read schema");
    assert_struct_matches_schema::<StandingScoreView>(&schema);
}
```

(Adapt `assert_struct_matches_schema` to whatever harness already exists — see existing tests in `schema_contract.rs` for the pattern.)

- [ ] **Step 4: Run; expect FAIL (red)**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev cargo test --manifest-path elohim/elohim-storage/Cargo.toml --test schema_contract bounds_validation_result_view 2>&1 | tail -10
```

Expected: compile error (modules don't exist yet) or test failure (`BoundsValidationResultView` not found).

- [ ] **Step 5: Author `elohim/elohim-views/src/bounds.rs`**

```rust
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct BoundsValidationResultView {
    pub pass: bool,
    pub commitment_cid: String,
    pub violation: Option<BoundsViolationView>,
    pub checks: BoundsChecksView,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct BoundsViolationView {
    pub kind: ViolationKind,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum ViolationKind {
    CommitmentInactive,
    ScopeNotIncluded,
    ReachCeilingExceeded,
    RateLimitExceeded,
    KeyRotationStale,
    CommitmentRevoked,
    CommitmentNotFound,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct BoundsChecksView {
    pub commitment_found: bool,
    pub active: bool,
    pub scope_includes_event: bool,
    pub reach_ceiling_ok: bool,
    pub rate_within_limit: bool,
    pub key_rotation_current: bool,
    pub not_revoked: bool,
}
```

- [ ] **Step 6: Author `elohim/elohim-views/src/standing.rs`**

```rust
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct StandingScoreView {
    pub evaluator_cid: String,
    pub subject_cid: String,
    pub score: StandingScoreTier,
    pub debit_weight_sum: i32,
    pub recent_breaches: Vec<FeedbackSignalSummary>,
    pub computed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum StandingScoreTier {
    Unknown,
    Floor,
    Low,
    Neutral,
    High,
    Trusted,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct FeedbackSignalSummary {
    pub signal_kind: String,
    pub emitted_at: String,
    pub weight: i32,
    pub evidence_summary: Option<String>,
}
```

Register both modules in `elohim/elohim-views/src/lib.rs`.

- [ ] **Step 7: Add to codegen-ts INTERFACE_FILES**

In `elohim/sdk/schemas/scripts/codegen-ts.mjs` find the `INTERFACE_FILES` array. Add:

```js
'bounds-validation-result-view.schema.json',
'standing-score-view.schema.json',
```

- [ ] **Step 8: Run schema_contract + ts-rs export**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev cargo test --manifest-path elohim/elohim-storage/Cargo.toml --test schema_contract 2>&1 | tail -20
cargo test --manifest-path elohim/elohim-views/Cargo.toml export_bindings 2>&1 | tail -10
pnpm run schema:codegen:ts 2>&1 | tail -10
```

Expected: all pass; TS types generated; schema_contract green.

- [ ] **Step 9: Commit**

```bash
git add elohim/sdk/schemas/v1/views/bounds-validation-result-view.schema.json \
        elohim/sdk/schemas/v1/views/standing-score-view.schema.json \
        elohim/elohim-views/src/bounds.rs elohim/elohim-views/src/standing.rs elohim/elohim-views/src/lib.rs \
        elohim/elohim-storage/tests/schema_contract.rs \
        elohim/sdk/schemas/scripts/codegen-ts.mjs \
        elohim/sdk/storage-client-ts/src/generated/ \
        app/elohim-app/src/app/generated/ \
        app/elohim-library/projects/elohim-service/src/generated/ \
        genesis/seeder/src/generated/
git commit -m "feat(views): BoundsValidationResultView + StandingScoreView schemas + ts-rs"
```

---

### Task 2: CommitmentFetcher trait + production impl

**Files:**
- Create: `elohim/elohim-storage/src/services/commitment_fetcher.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs` (declare module)
- Test: inline `#[cfg(test)] mod tests` in commitment_fetcher.rs

- [ ] **Step 1: Write the failing test**

```rust
// elohim/elohim-storage/src/services/commitment_fetcher.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_fetcher_returns_seeded_commitment() {
        let mock = MockCommitmentFetcher::new();
        mock.seed("commitment-cid-abc", sample_active_commitment());
        let result = mock.fetch("commitment-cid-abc").await.unwrap();
        assert_eq!(result.action, "delegates-compute");
    }

    #[tokio::test]
    async fn mock_fetcher_returns_none_for_unknown_cid() {
        let mock = MockCommitmentFetcher::new();
        let result = mock.fetch("commitment-cid-unknown").await.unwrap();
        assert!(result.is_none());
    }

    fn sample_active_commitment() -> CommitmentRecord {
        CommitmentRecord {
            cid: "commitment-cid-abc".into(),
            action: "delegates-compute".into(),
            scope: "republish-epr".into(),
            provider: "agent:matthew-steward".into(),
            recipient: "agent:deploy-svc-matthew".into(),
            bounds: serde_json::json!({
                "epr_scope": ["epr:lamad-spa"],
                "reach_ceiling": "commons",
                "rate_per_hour": 30,
                "rotation_ttl_days": 90
            }),
            valid_from: "2026-05-01T00:00:00Z".into(),
            valid_until: "2026-08-01T00:00:00Z".into(),
            revoked_at: None,
        }
    }
}
```

- [ ] **Step 2: Implement trait + production impl + mock**

```rust
// elohim/elohim-storage/src/services/commitment_fetcher.rs
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommitmentRecord {
    pub cid: String,
    pub action: String,
    pub scope: String,
    pub provider: String,
    pub recipient: String,
    pub bounds: serde_json::Value,
    pub valid_from: String,
    pub valid_until: String,
    pub revoked_at: Option<String>,
}

#[async_trait]
pub trait CommitmentFetcher: Send + Sync {
    async fn fetch(&self, cid: &str) -> Result<Option<CommitmentRecord>, FetchError>;
}

#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("conductor unreachable: {0}")]
    ConductorUnreachable(String),
    #[error("malformed commitment record: {0}")]
    MalformedRecord(String),
}

/// Production impl — fetches from the local Mishpat zome via the conductor.
/// In Sprint 1 implementation, the deploy-svc-agent's events are validated
/// against Commitments fetched here.
pub struct ConductorCommitmentFetcher {
    pub hc_client: Arc<crate::hc_client::HcClient>,
}

#[async_trait]
impl CommitmentFetcher for ConductorCommitmentFetcher {
    async fn fetch(&self, cid: &str) -> Result<Option<CommitmentRecord>, FetchError> {
        self.hc_client
            .call_mishpat_get_commitment(cid)
            .await
            .map_err(|e| FetchError::ConductorUnreachable(e.to_string()))
    }
}

/// Test mock — supports seed() + fetch() with a HashMap.
pub struct MockCommitmentFetcher {
    inner: Arc<Mutex<HashMap<String, CommitmentRecord>>>,
}

impl MockCommitmentFetcher {
    pub fn new() -> Self {
        Self { inner: Arc::new(Mutex::new(HashMap::new())) }
    }
    pub fn seed(&self, cid: &str, record: CommitmentRecord) {
        self.inner.lock().unwrap().insert(cid.to_string(), record);
    }
}

#[async_trait]
impl CommitmentFetcher for MockCommitmentFetcher {
    async fn fetch(&self, cid: &str) -> Result<Option<CommitmentRecord>, FetchError> {
        Ok(self.inner.lock().unwrap().get(cid).cloned())
    }
}
```

If `hc_client::HcClient::call_mishpat_get_commitment` doesn't exist yet, leave a `todo!()` body in the production impl + a `TODO(Sprint 1)` comment. Mock + tests still verify the trait surface. Sprint 1 wires the production path.

- [ ] **Step 3: Run; expect PASS**

```bash
cargo test --manifest-path elohim/elohim-storage/Cargo.toml --lib services::commitment_fetcher:: 2>&1 | tail -15
```

Expected: 2 tests pass.

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/services/commitment_fetcher.rs elohim/elohim-storage/src/services/mod.rs
git commit -m "feat(storage): CommitmentFetcher trait + mock + conductor impl skeleton"
```

---

### Task 3: RateHistory trait + diesel-backed production impl

**Files:**
- Create: `elohim/elohim-storage/src/services/rate_history.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`

- [ ] **Step 1: Write failing test**

```rust
// elohim/elohim-storage/src/services/rate_history.rs tests
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_returns_seeded_count() {
        let mock = MockRateHistory::new();
        mock.seed("commitment-cid-abc", "2026-05-28T12:00:00Z", 7);
        let count = mock.count_in_window("commitment-cid-abc", "2026-05-28T12:00:00Z", 60).await.unwrap();
        assert_eq!(count, 7);
    }

    #[tokio::test]
    async fn mock_returns_zero_for_unseeded() {
        let mock = MockRateHistory::new();
        let count = mock.count_in_window("commitment-cid-xyz", "2026-05-28T12:00:00Z", 60).await.unwrap();
        assert_eq!(count, 0);
    }
}
```

- [ ] **Step 2: Implement trait + diesel impl + mock**

```rust
// elohim/elohim-storage/src/services/rate_history.rs
use async_trait::async_trait;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::db::DbPool;
use crate::StorageError;

#[async_trait]
pub trait RateHistory: Send + Sync {
    /// Count events bounded by `commitment_cid` with `signed_at` in the trailing
    /// `window_minutes` from `now_iso`. Used for sliding-window rate-limit check.
    async fn count_in_window(
        &self,
        commitment_cid: &str,
        now_iso: &str,
        window_minutes: u32,
    ) -> Result<u32, StorageError>;
}

/// Production impl — queries the local `economic_events` SQLite projection.
pub struct DieselRateHistory {
    pub pool: DbPool,
}

#[async_trait]
impl RateHistory for DieselRateHistory {
    async fn count_in_window(
        &self,
        commitment_cid: &str,
        now_iso: &str,
        window_minutes: u32,
    ) -> Result<u32, StorageError> {
        use crate::db::diesel_schema::economic_events::dsl;
        let pool = self.pool.clone();
        let cid_owned = commitment_cid.to_string();
        let now_owned = now_iso.to_string();
        tokio::task::spawn_blocking(move || -> Result<u32, StorageError> {
            let mut conn = pool.get().map_err(|e| StorageError::Database(e.to_string()))?;
            let cutoff = chrono::DateTime::parse_from_rfc3339(&now_owned)
                .map_err(|e| StorageError::InvalidInput(format!("bad now_iso: {e}")))?
                - chrono::Duration::minutes(window_minutes as i64);
            let cutoff_iso = cutoff.to_rfc3339();
            let n: i64 = dsl::economic_events
                .filter(dsl::bounded_by.eq(&cid_owned))
                .filter(dsl::signed_at.ge(&cutoff_iso))
                .count()
                .get_result(&mut conn)
                .map_err(|e| StorageError::Database(e.to_string()))?;
            Ok(n as u32)
        })
        .await
        .map_err(|e| StorageError::Internal(e.to_string()))?
    }
}

/// Test mock.
pub struct MockRateHistory {
    inner: Arc<Mutex<HashMap<String, u32>>>,
}

impl MockRateHistory {
    pub fn new() -> Self {
        Self { inner: Arc::new(Mutex::new(HashMap::new())) }
    }
    pub fn seed(&self, cid: &str, _at_iso: &str, count: u32) {
        self.inner.lock().unwrap().insert(cid.to_string(), count);
    }
}

#[async_trait]
impl RateHistory for MockRateHistory {
    async fn count_in_window(
        &self,
        commitment_cid: &str,
        _now_iso: &str,
        _window_minutes: u32,
    ) -> Result<u32, StorageError> {
        Ok(self.inner.lock().unwrap().get(commitment_cid).copied().unwrap_or(0))
    }
}
```

Important: if the `economic_events` diesel schema does NOT yet have a `bounded_by` column, this is a separate migration that Sprint 1 should ship before its put_epr-handler validator call. For Sprint 2's purposes, write the diesel query assuming the column exists; sequence the migration via blocking note below.

- [ ] **Step 3: Add `bounded_by` column to `economic_events` (migration)**

Create `elohim/elohim-storage/migrations/2026-05-28-add-bounded-by-to-economic-events/up.sql`:

```sql
-- Add bounded_by column to economic_events for REA compute-commitment back-reference.
-- Nullable for backward compatibility with pre-roadmap events; new emits required to populate.
-- Source of truth: Holochain DHT (the EconomicEvent entry's bounded_by field); this column
-- is the operational projection used by bounds_validator and rate_history for fast queries.
ALTER TABLE economic_events ADD COLUMN bounded_by TEXT;
CREATE INDEX idx_economic_events_bounded_by_signed_at ON economic_events(bounded_by, signed_at);
```

And `down.sql`:

```sql
DROP INDEX IF EXISTS idx_economic_events_bounded_by_signed_at;
ALTER TABLE economic_events DROP COLUMN bounded_by;
```

Update the diesel `schema.rs` if it's generated (typically via `diesel print-schema` or by hand-editing in `elohim/elohim-storage/src/db/diesel_schema.rs`).

- [ ] **Step 4: Run tests + migration**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev cargo test --manifest-path elohim/elohim-storage/Cargo.toml --lib services::rate_history:: 2>&1 | tail -15
```

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/services/rate_history.rs elohim/elohim-storage/src/services/mod.rs \
        elohim/elohim-storage/src/db/diesel_schema.rs \
        elohim/elohim-storage/migrations/2026-05-28-add-bounded-by-to-economic-events/
git commit -m "feat(storage): RateHistory trait + diesel impl; bounded_by column on economic_events"
```

---

### Task 4: bounds_validator::validate function — happy path

**Files:**
- Create: `elohim/elohim-storage/src/services/bounds_validator.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`

- [ ] **Step 1: Write failing happy-path test**

```rust
// elohim/elohim-storage/src/services/bounds_validator.rs tests
#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::commitment_fetcher::{CommitmentRecord, MockCommitmentFetcher};
    use crate::services::rate_history::MockRateHistory;

    fn sample_active_commitment() -> CommitmentRecord {
        CommitmentRecord {
            cid: "commitment-cid-abc".into(),
            action: "delegates-compute".into(),
            scope: "republish-epr".into(),
            provider: "agent:matthew-steward".into(),
            recipient: "agent:deploy-svc-matthew".into(),
            bounds: serde_json::json!({
                "epr_scope": ["epr:lamad-spa"],
                "reach_ceiling": "commons",
                "rate_per_hour": 30,
                "rotation_ttl_days": 90
            }),
            valid_from: "2026-05-01T00:00:00Z".into(),
            valid_until: "2026-08-01T00:00:00Z".into(),
            revoked_at: None,
        }
    }

    fn sample_event() -> EventForValidation {
        EventForValidation {
            action: "republish-epr".into(),
            performer: "agent:deploy-svc-matthew".into(),
            bounded_by: "commitment-cid-abc".into(),
            target_epr_id: "epr:lamad-spa".into(),
            reach: "commons".into(),
            signed_at: "2026-05-28T12:00:00Z".into(),
        }
    }

    #[tokio::test]
    async fn validate_passes_when_all_checks_satisfied() {
        let fetcher = MockCommitmentFetcher::new();
        fetcher.seed("commitment-cid-abc", sample_active_commitment());
        let rate = MockRateHistory::new();  // empty — count == 0

        let result = validate(&sample_event(), &fetcher, &rate).await;
        assert!(matches!(result, Ok(())));
    }
}
```

- [ ] **Step 2: Implement validate + EventForValidation**

```rust
// elohim/elohim-storage/src/services/bounds_validator.rs
use crate::services::commitment_fetcher::{CommitmentFetcher, FetchError};
use crate::services::rate_history::RateHistory;
use elohim_views::bounds::{BoundsChecksView, ViolationKind};

/// Subset of EconomicEventView that bounds_validator needs. Per-instance
/// validators (Sprint 1 republish_epr_validator etc.) convert from their
/// schema-specific view to this projection before calling validate.
#[derive(Debug, Clone)]
pub struct EventForValidation {
    pub action: String,
    pub performer: String,
    pub bounded_by: String,
    pub target_epr_id: String,
    pub reach: String,
    pub signed_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundsViolation {
    pub kind: ViolationKind,
    pub commitment_cid: String,
    pub summary: String,
    pub checks: BoundsChecksView,
}

pub async fn validate<F: CommitmentFetcher, R: RateHistory>(
    event: &EventForValidation,
    fetcher: &F,
    rate_history: &R,
) -> Result<(), BoundsViolation> {
    let mut checks = BoundsChecksView::default();

    // 1. Fetch the Commitment
    let commitment = match fetcher.fetch(&event.bounded_by).await {
        Ok(Some(c)) => {
            checks.commitment_found = true;
            c
        }
        Ok(None) => {
            return Err(BoundsViolation {
                kind: ViolationKind::CommitmentNotFound,
                commitment_cid: event.bounded_by.clone(),
                summary: format!("no Commitment found for cid {}", event.bounded_by),
                checks,
            });
        }
        Err(FetchError::ConductorUnreachable(msg)) => {
            return Err(BoundsViolation {
                kind: ViolationKind::CommitmentNotFound,
                commitment_cid: event.bounded_by.clone(),
                summary: format!("conductor unreachable: {msg}"),
                checks,
            });
        }
        Err(FetchError::MalformedRecord(msg)) => {
            return Err(BoundsViolation {
                kind: ViolationKind::CommitmentNotFound,
                commitment_cid: event.bounded_by.clone(),
                summary: format!("malformed record: {msg}"),
                checks,
            });
        }
    };

    // 2. Revocation check (most likely to fail; cheap)
    if commitment.revoked_at.is_some() {
        return Err(BoundsViolation {
            kind: ViolationKind::CommitmentRevoked,
            commitment_cid: commitment.cid.clone(),
            summary: format!("commitment revoked at {}", commitment.revoked_at.as_deref().unwrap_or("unknown")),
            checks,
        });
    }
    checks.not_revoked = true;

    // 3. Active window check
    let now = &event.signed_at;
    if now < &commitment.valid_from || now > &commitment.valid_until {
        return Err(BoundsViolation {
            kind: ViolationKind::CommitmentInactive,
            commitment_cid: commitment.cid.clone(),
            summary: format!("event signed at {now} outside commitment window [{}, {}]", commitment.valid_from, commitment.valid_until),
            checks,
        });
    }
    checks.active = true;

    // 4. Scope check (event.action must equal commitment.scope)
    if event.action != commitment.scope {
        return Err(BoundsViolation {
            kind: ViolationKind::ScopeNotIncluded,
            commitment_cid: commitment.cid.clone(),
            summary: format!("event.action='{}' does not match commitment.scope='{}'", event.action, commitment.scope),
            checks,
        });
    }
    // Plus: target_epr_id must be in bounds.epr_scope OR scope contains "*"
    let epr_scope = commitment.bounds.get("epr_scope")
        .and_then(|v| v.as_array())
        .ok_or_else(|| BoundsViolation {
            kind: ViolationKind::ScopeNotIncluded,
            commitment_cid: commitment.cid.clone(),
            summary: "commitment.bounds.epr_scope missing or not an array".into(),
            checks: checks.clone(),
        })?;
    let scope_matches = epr_scope.iter().any(|v| {
        v.as_str().map(|s| s == "*" || s == event.target_epr_id).unwrap_or(false)
    });
    if !scope_matches {
        return Err(BoundsViolation {
            kind: ViolationKind::ScopeNotIncluded,
            commitment_cid: commitment.cid.clone(),
            summary: format!("event.target_epr_id='{}' not in commitment.bounds.epr_scope", event.target_epr_id),
            checks,
        });
    }
    checks.scope_includes_event = true;

    // 5. Reach ceiling check
    let reach_rank = reach_rank(&event.reach).ok_or_else(|| BoundsViolation {
        kind: ViolationKind::ReachCeilingExceeded,
        commitment_cid: commitment.cid.clone(),
        summary: format!("unknown reach value '{}'", event.reach),
        checks: checks.clone(),
    })?;
    let ceiling = commitment.bounds.get("reach_ceiling")
        .and_then(|v| v.as_str())
        .ok_or_else(|| BoundsViolation {
            kind: ViolationKind::ReachCeilingExceeded,
            commitment_cid: commitment.cid.clone(),
            summary: "commitment.bounds.reach_ceiling missing".into(),
            checks: checks.clone(),
        })?;
    let ceiling_rank = reach_rank(ceiling).ok_or_else(|| BoundsViolation {
        kind: ViolationKind::ReachCeilingExceeded,
        commitment_cid: commitment.cid.clone(),
        summary: format!("unknown ceiling value '{}'", ceiling),
        checks: checks.clone(),
    })?;
    if reach_rank > ceiling_rank {
        return Err(BoundsViolation {
            kind: ViolationKind::ReachCeilingExceeded,
            commitment_cid: commitment.cid.clone(),
            summary: format!("event.reach='{}' exceeds commitment.reach_ceiling='{}'", event.reach, ceiling),
            checks,
        });
    }
    checks.reach_ceiling_ok = true;

    // 6. Rate limit check (sliding window)
    let rate_per_hour = commitment.bounds.get("rate_per_hour")
        .and_then(|v| v.as_u64())
        .unwrap_or(u64::MAX);
    let recent = rate_history.count_in_window(&commitment.cid, &event.signed_at, 60).await
        .map_err(|e| BoundsViolation {
            kind: ViolationKind::RateLimitExceeded,
            commitment_cid: commitment.cid.clone(),
            summary: format!("rate-history query failed: {e}"),
            checks: checks.clone(),
        })?;
    if (recent as u64) >= rate_per_hour {
        return Err(BoundsViolation {
            kind: ViolationKind::RateLimitExceeded,
            commitment_cid: commitment.cid.clone(),
            summary: format!("recent count {recent} >= rate_per_hour {rate_per_hour} in 60min window"),
            checks,
        });
    }
    checks.rate_within_limit = true;

    // 7. Key rotation check
    let rotation_ttl_days = commitment.bounds.get("rotation_ttl_days")
        .and_then(|v| v.as_u64())
        .unwrap_or(u64::MAX);
    let valid_from = chrono::DateTime::parse_from_rfc3339(&commitment.valid_from)
        .map_err(|e| BoundsViolation {
            kind: ViolationKind::KeyRotationStale,
            commitment_cid: commitment.cid.clone(),
            summary: format!("bad valid_from: {e}"),
            checks: checks.clone(),
        })?;
    let signed_at = chrono::DateTime::parse_from_rfc3339(&event.signed_at)
        .map_err(|e| BoundsViolation {
            kind: ViolationKind::KeyRotationStale,
            commitment_cid: commitment.cid.clone(),
            summary: format!("bad signed_at: {e}"),
            checks: checks.clone(),
        })?;
    let age_days = (signed_at - valid_from).num_days() as u64;
    if age_days > rotation_ttl_days {
        return Err(BoundsViolation {
            kind: ViolationKind::KeyRotationStale,
            commitment_cid: commitment.cid.clone(),
            summary: format!("commitment age {age_days}d exceeds rotation_ttl_days {rotation_ttl_days}"),
            checks,
        });
    }
    checks.key_rotation_current = true;

    Ok(())
}

fn reach_rank(s: &str) -> Option<u8> {
    match s {
        "private" => Some(0),
        "self" => Some(1),
        "intimate" => Some(2),
        "trusted" => Some(3),
        "familiar" => Some(4),
        "community" => Some(5),
        "public" => Some(6),
        "commons" => Some(7),
        _ => None,
    }
}
```

- [ ] **Step 3: Run; expect PASS for happy path**

```bash
cargo test --manifest-path elohim/elohim-storage/Cargo.toml --lib services::bounds_validator::tests::validate_passes_when_all_checks_satisfied 2>&1 | tail -10
```

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/services/bounds_validator.rs elohim/elohim-storage/src/services/mod.rs
git commit -m "feat(storage): bounds_validator::validate happy path + EventForValidation projection"
```

---

### Task 5: bounds_validator — 6 violation-case tests + red-team adversarial

**Files:**
- Modify: `elohim/elohim-storage/src/services/bounds_validator.rs` (tests module)
- Create: `elohim/elohim-storage/tests/bounds_validator_integration.rs`

- [ ] **Step 1: Author 6 unit tests, one per ViolationKind**

Append to `bounds_validator.rs::tests`:

```rust
#[tokio::test]
async fn validate_rejects_commitment_not_found() {
    let fetcher = MockCommitmentFetcher::new();  // empty
    let rate = MockRateHistory::new();
    let result = validate(&sample_event(), &fetcher, &rate).await;
    assert!(matches!(result, Err(BoundsViolation { kind: ViolationKind::CommitmentNotFound, .. })));
}

#[tokio::test]
async fn validate_rejects_revoked_commitment() {
    let mut c = sample_active_commitment();
    c.revoked_at = Some("2026-05-15T00:00:00Z".into());
    let fetcher = MockCommitmentFetcher::new();
    fetcher.seed(&c.cid.clone(), c);
    let rate = MockRateHistory::new();
    let result = validate(&sample_event(), &fetcher, &rate).await;
    assert!(matches!(result, Err(BoundsViolation { kind: ViolationKind::CommitmentRevoked, .. })));
}

#[tokio::test]
async fn validate_rejects_inactive_window() {
    let mut c = sample_active_commitment();
    c.valid_until = "2026-05-15T00:00:00Z".into();  // event is 2026-05-28
    let fetcher = MockCommitmentFetcher::new();
    fetcher.seed(&c.cid.clone(), c);
    let rate = MockRateHistory::new();
    let result = validate(&sample_event(), &fetcher, &rate).await;
    assert!(matches!(result, Err(BoundsViolation { kind: ViolationKind::CommitmentInactive, .. })));
}

#[tokio::test]
async fn validate_rejects_out_of_scope_action() {
    let mut c = sample_active_commitment();
    c.scope = "serve-url-projection".into();  // event.action is republish-epr
    let fetcher = MockCommitmentFetcher::new();
    fetcher.seed(&c.cid.clone(), c);
    let rate = MockRateHistory::new();
    let result = validate(&sample_event(), &fetcher, &rate).await;
    assert!(matches!(result, Err(BoundsViolation { kind: ViolationKind::ScopeNotIncluded, .. })));
}

#[tokio::test]
async fn validate_rejects_reach_ceiling_exceeded() {
    let mut e = sample_event();
    e.reach = "public".into();  // commitment ceiling is commons
    let fetcher = MockCommitmentFetcher::new();
    fetcher.seed("commitment-cid-abc", sample_active_commitment());
    let rate = MockRateHistory::new();
    let result = validate(&e, &fetcher, &rate).await;
    assert!(matches!(result, Err(BoundsViolation { kind: ViolationKind::ReachCeilingExceeded, .. })));
}

#[tokio::test]
async fn validate_rejects_rate_limit_exceeded() {
    let fetcher = MockCommitmentFetcher::new();
    fetcher.seed("commitment-cid-abc", sample_active_commitment());
    let rate = MockRateHistory::new();
    rate.seed("commitment-cid-abc", "2026-05-28T12:00:00Z", 30);  // rate_per_hour limit
    let result = validate(&sample_event(), &fetcher, &rate).await;
    assert!(matches!(result, Err(BoundsViolation { kind: ViolationKind::RateLimitExceeded, .. })));
}

#[tokio::test]
async fn validate_rejects_key_rotation_stale() {
    let mut c = sample_active_commitment();
    c.valid_from = "2026-01-01T00:00:00Z".into();  // 148 days before signed_at
    c.bounds = serde_json::json!({
        "epr_scope": ["epr:lamad-spa"],
        "reach_ceiling": "commons",
        "rate_per_hour": 30,
        "rotation_ttl_days": 90  // exceeded
    });
    let fetcher = MockCommitmentFetcher::new();
    fetcher.seed(&c.cid.clone(), c);
    let rate = MockRateHistory::new();
    let result = validate(&sample_event(), &fetcher, &rate).await;
    assert!(matches!(result, Err(BoundsViolation { kind: ViolationKind::KeyRotationStale, .. })));
}
```

- [ ] **Step 2: Author red-team adversarial integration test file**

Create `elohim/elohim-storage/tests/bounds_validator_integration.rs`:

```rust
//! Adversarial integration tests for bounds_validator.
//!
//! Tests attack surface that unit-test mocks don't catch — race conditions
//! between fetch+validate, malformed CommitmentRecords, sliding-window edges,
//! and the trust assumption that fetcher returns accurate revoked state.
//!
//! See `genesis/docs/superpowers/plans/2026-05-28-sprint2-bounds-validator-standing-aggregator.md`.

use elohim_storage::services::bounds_validator::{validate, EventForValidation};
use elohim_storage::services::commitment_fetcher::{MockCommitmentFetcher, CommitmentRecord};
use elohim_storage::services::rate_history::MockRateHistory;
use elohim_views::bounds::ViolationKind;

fn deploy_svc_commitment(revoked: bool) -> CommitmentRecord {
    CommitmentRecord {
        cid: "comm-deploy-svc".into(),
        action: "delegates-compute".into(),
        scope: "republish-epr".into(),
        provider: "agent:matthew-steward".into(),
        recipient: "agent:deploy-svc-matthew".into(),
        bounds: serde_json::json!({
            "epr_scope": ["epr:lamad-spa", "epr:elohim-host-landing"],
            "reach_ceiling": "commons",
            "rate_per_hour": 30,
            "rotation_ttl_days": 90
        }),
        valid_from: "2026-05-01T00:00:00Z".into(),
        valid_until: "2026-08-01T00:00:00Z".into(),
        revoked_at: if revoked { Some("2026-05-15T00:00:00Z".into()) } else { None },
    }
}

fn event_against(commitment_cid: &str, action: &str, target: &str, reach: &str) -> EventForValidation {
    EventForValidation {
        action: action.into(),
        performer: "agent:deploy-svc-matthew".into(),
        bounded_by: commitment_cid.into(),
        target_epr_id: target.into(),
        reach: reach.into(),
        signed_at: "2026-05-28T12:00:00Z".into(),
    }
}

#[tokio::test]
async fn adversarial_revocation_race_blocks_immediately() {
    // Operator just revoked at 2026-05-15; event was signed earlier at 2026-05-28
    // but fetcher returns the current (revoked) state. Validator must reject.
    let fetcher = MockCommitmentFetcher::new();
    fetcher.seed("comm-deploy-svc", deploy_svc_commitment(true));
    let rate = MockRateHistory::new();

    let event = event_against("comm-deploy-svc", "republish-epr", "epr:lamad-spa", "commons");
    let result = validate(&event, &fetcher, &rate).await;
    assert!(matches!(result, Err(v) if v.kind == ViolationKind::CommitmentRevoked));
}

#[tokio::test]
async fn adversarial_forged_bounded_by_pointing_to_unrelated_scope() {
    // Attacker emits republish-epr but bounded_by points to a serve-url-projection Commitment.
    let mut c = deploy_svc_commitment(false);
    c.scope = "serve-url-projection".into();
    let fetcher = MockCommitmentFetcher::new();
    fetcher.seed("comm-deploy-svc", c);
    let rate = MockRateHistory::new();

    let event = event_against("comm-deploy-svc", "republish-epr", "epr:lamad-spa", "commons");
    let result = validate(&event, &fetcher, &rate).await;
    assert!(matches!(result, Err(v) if v.kind == ViolationKind::ScopeNotIncluded));
}

#[tokio::test]
async fn adversarial_silent_reach_escalation_blocked() {
    let fetcher = MockCommitmentFetcher::new();
    fetcher.seed("comm-deploy-svc", deploy_svc_commitment(false));
    let rate = MockRateHistory::new();

    let event = event_against("comm-deploy-svc", "republish-epr", "epr:lamad-spa", "private");
    // private rank=0 < commons rank=7 — should this PASS or FAIL?
    // Decision per delegates-compute schema description: reach_ceiling is the MAX, so lower reaches
    // (more restrictive) are allowed by default. Confirm: validate passes for reach<=ceiling.
    let result = validate(&event, &fetcher, &rate).await;
    assert!(matches!(result, Ok(())), "lower reach within ceiling must pass");

    // But upward escalation must fail.
    let event_up = event_against("comm-deploy-svc", "republish-epr", "epr:lamad-spa", "public");
    let result_up = validate(&event_up, &fetcher, &rate).await;
    assert!(matches!(result_up, Err(v) if v.kind == ViolationKind::ReachCeilingExceeded));
}

#[tokio::test]
async fn adversarial_rate_limit_exact_boundary() {
    let fetcher = MockCommitmentFetcher::new();
    fetcher.seed("comm-deploy-svc", deploy_svc_commitment(false));
    let rate = MockRateHistory::new();
    rate.seed("comm-deploy-svc", "2026-05-28T12:00:00Z", 29);  // 29 < 30 ceiling

    let event = event_against("comm-deploy-svc", "republish-epr", "epr:lamad-spa", "commons");
    assert!(matches!(validate(&event, &fetcher, &rate).await, Ok(())), "29 events in window must pass");

    rate.seed("comm-deploy-svc", "2026-05-28T12:00:00Z", 30);  // exactly at limit
    assert!(matches!(validate(&event, &fetcher, &rate).await, Err(v) if v.kind == ViolationKind::RateLimitExceeded));
}

#[tokio::test]
async fn adversarial_out_of_epr_scope_rejected() {
    let fetcher = MockCommitmentFetcher::new();
    fetcher.seed("comm-deploy-svc", deploy_svc_commitment(false));
    let rate = MockRateHistory::new();

    let event = event_against("comm-deploy-svc", "republish-epr", "epr:not-in-scope", "commons");
    assert!(matches!(validate(&event, &fetcher, &rate).await, Err(v) if v.kind == ViolationKind::ScopeNotIncluded));
}

#[tokio::test]
async fn adversarial_wildcard_epr_scope_allows_unknown_target() {
    let mut c = deploy_svc_commitment(false);
    c.bounds["epr_scope"] = serde_json::json!(["*"]);
    let fetcher = MockCommitmentFetcher::new();
    fetcher.seed("comm-deploy-svc", c);
    let rate = MockRateHistory::new();

    let event = event_against("comm-deploy-svc", "republish-epr", "epr:any-future-bundle", "commons");
    assert!(matches!(validate(&event, &fetcher, &rate).await, Ok(())));
}
```

- [ ] **Step 3: Run all bounds_validator tests**

```bash
cargo test --manifest-path elohim/elohim-storage/Cargo.toml --lib services::bounds_validator:: 2>&1 | tail -20
cargo test --manifest-path elohim/elohim-storage/Cargo.toml --test bounds_validator_integration 2>&1 | tail -20
```

Expected: 7 unit tests pass; 6 integration tests pass.

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/services/bounds_validator.rs \
        elohim/elohim-storage/tests/bounds_validator_integration.rs
git commit -m "test(storage): bounds_validator 6 violation-kind cases + red-team adversarial"
```

---

### Task 6: Diagnostic HTTP route — POST /api/v1/diagnostics/validate-bounds

**Files:**
- Create: `elohim/elohim-storage/src/api/diagnostics_bounds.rs`
- Modify: `elohim/elohim-storage/src/api/mod.rs`
- Modify: `elohim/elohim-storage/src/http.rs` (mount route)

- [ ] **Step 1: Author handler**

```rust
// elohim/elohim-storage/src/api/diagnostics_bounds.rs
use hyper::{Request, Response, StatusCode};
use hyper::body::{Bytes, Incoming};
use http_body_util::Full;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::AppContext;
use crate::StorageError;
use crate::services::bounds_validator::{validate, EventForValidation};
use elohim_views::bounds::{BoundsValidationResultView, BoundsViolationView, ViolationKind};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateBoundsRequest {
    pub event: EventForValidation,
}

pub async fn handle_validate_bounds(
    req: Request<Incoming>,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let body: ValidateBoundsRequest = super::parse_body(req).await?;
    let fetcher = ctx.commitment_fetcher.as_ref().ok_or_else(|| {
        StorageError::Internal("CommitmentFetcher not wired in AppContext".into())
    })?;
    let rate = ctx.rate_history.as_ref().ok_or_else(|| {
        StorageError::Internal("RateHistory not wired in AppContext".into())
    })?;

    let result = validate(&body.event, fetcher.as_ref(), rate.as_ref()).await;
    let view = match result {
        Ok(()) => BoundsValidationResultView {
            pass: true,
            commitment_cid: body.event.bounded_by.clone(),
            violation: None,
            checks: Default::default(),  // TODO: populate from validate's side-channel (refactor: return Ok((checks, ()))
        },
        Err(v) => BoundsValidationResultView {
            pass: false,
            commitment_cid: v.commitment_cid,
            violation: Some(BoundsViolationView { kind: v.kind, summary: v.summary }),
            checks: v.checks,
        },
    };

    let body = serde_json::to_vec(&view)
        .map_err(|e| StorageError::Internal(format!("serialize: {e}")))?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap())
}
```

Note the TODO: the current `validate` signature returns `Result<(), BoundsViolation>` — on success, the diagnostic view has no `checks` populated. Refactor `validate` to return `Result<BoundsChecksView, BoundsViolation>` so the success path can return the full check trail (every check that passed). Do this refactor in this task; update existing tests to ignore the new return value where they only care about Ok/Err shape.

- [ ] **Step 2: Refactor validate signature**

Change `validate` return type from `Result<(), BoundsViolation>` to `Result<BoundsChecksView, BoundsViolation>`. Update the existing happy-path test to discard via `.is_ok()` or `let _ = result.unwrap()`. The 6 violation tests already use `matches!(result, Err(...))` so they don't break.

- [ ] **Step 3: Wire route in http.rs**

In `elohim/elohim-storage/src/http.rs`, find the routing dispatch (likely a match on `req.uri().path()`). Add:

```rust
(POST, "/api/v1/diagnostics/validate-bounds") => {
    crate::api::diagnostics_bounds::handle_validate_bounds(req, &ctx).await
}
```

- [ ] **Step 4: Integration test via reqwest or hyper client**

Add to `bounds_validator_integration.rs`:

```rust
#[tokio::test]
async fn diagnostics_route_returns_correct_violation_view() {
    // Spin up a minimal AppContext with MockCommitmentFetcher seeded with a revoked Commitment.
    // POST /api/v1/diagnostics/validate-bounds with an event referencing it.
    // Assert response JSON has pass: false, violation.kind: "commitment_revoked".
    // Implementation depends on existing test-harness pattern in elohim-storage.
    // If existing tests use a TestHarness struct, follow that; otherwise build the
    // smallest possible AppContext stub.
    //
    // Skip body if existing test-harness pattern is unclear — note as DONE_WITH_CONCERNS
    // and the implementer of Sprint 1 fills it in when wiring put_epr through this same route.
}
```

- [ ] **Step 5: Run + commit**

```bash
cargo test --manifest-path elohim/elohim-storage/Cargo.toml --lib api::diagnostics_bounds 2>&1 | tail -10
cargo test --manifest-path elohim/elohim-storage/Cargo.toml --lib services::bounds_validator:: 2>&1 | tail -10  # ensure refactor didn't break
git add elohim/elohim-storage/src/api/diagnostics_bounds.rs elohim/elohim-storage/src/api/mod.rs elohim/elohim-storage/src/http.rs elohim/elohim-storage/src/services/bounds_validator.rs
git commit -m "feat(storage): POST /api/v1/diagnostics/validate-bounds; validate returns checks on success"
```

---

### Task 7: Signal-weight registry (manifest-driven)

**Files:**
- Create: `elohim/elohim-storage/src/services/signal_weight_registry.rs`
- Modify: `elohim/sdk/domains/elohim/manifest.json` (declare new signal_kind weights)
- Modify: `elohim/elohim-storage/src/services/mod.rs`

- [ ] **Step 1: Declare weights in elohim manifest**

Edit `elohim/sdk/domains/elohim/manifest.json`. Find the section that declares signal_kinds (or a feedback-signal vocabulary if it exists). Add weight metadata:

```json
{
  "signal_kinds": {
    "rate-limit-exceeded": { "debit_weight": 5, "decay_days": 30 },
    "bad-custody": { "debit_weight": 20, "decay_days": 90 },
    "reach-escalation-pending": { "debit_weight": 3, "decay_days": 14 },
    "compute-breach": { "debit_weight": 10, "decay_days": 60 }
  }
}
```

(If the manifest doesn't have a `signal_kinds` top-level yet, the manifest's existing schema lets you extend; coordinate with the manifest schema in `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json` if validation fails.)

- [ ] **Step 2: Implement registry loader**

```rust
// elohim/elohim-storage/src/services/signal_weight_registry.rs
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, Clone, Deserialize)]
pub struct SignalWeight {
    pub debit_weight: i32,
    pub decay_days: u32,
}

#[derive(Debug, Clone, Deserialize)]
struct ElohimManifest {
    signal_kinds: HashMap<String, SignalWeight>,
}

static REGISTRY: OnceLock<HashMap<String, SignalWeight>> = OnceLock::new();

pub fn weight_for(signal_kind: &str) -> Option<SignalWeight> {
    REGISTRY.get_or_init(|| {
        let manifest_path = std::env::var("ELOHIM_MANIFEST_PATH")
            .unwrap_or_else(|_| "elohim/sdk/domains/elohim/manifest.json".into());
        let bytes = std::fs::read(&manifest_path)
            .unwrap_or_else(|e| panic!("read manifest at {manifest_path}: {e}"));
        let manifest: ElohimManifest = serde_json::from_slice(&bytes)
            .unwrap_or_else(|e| panic!("parse manifest: {e}"));
        manifest.signal_kinds
    }).get(signal_kind).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_returns_weight_for_known_signal() {
        let w = weight_for("rate-limit-exceeded").expect("known signal_kind has weight");
        assert!(w.debit_weight > 0);
    }

    #[test]
    fn registry_returns_none_for_unknown() {
        assert!(weight_for("nonexistent-signal").is_none());
    }
}
```

- [ ] **Step 3: Run tests + commit**

```bash
cargo test --manifest-path elohim/elohim-storage/Cargo.toml --lib services::signal_weight_registry 2>&1 | tail -10
git add elohim/elohim-storage/src/services/signal_weight_registry.rs elohim/elohim-storage/src/services/mod.rs elohim/sdk/domains/elohim/manifest.json
git commit -m "feat(storage): manifest-driven signal_weight_registry; declare 4 new signal_kinds"
```

---

### Task 8: Extend standing_projector to consume new signal_kinds

**Files:**
- Modify: `elohim/elohim-storage/src/services/standing_projector.rs`
- Test: append to existing tests module

- [ ] **Step 1: Find existing project_signal function**

Read `elohim/elohim-storage/src/services/standing_projector.rs`. It has a `project_signal` function that takes a FeedbackSignal and updates `standing_view`. Identify the switch/match that maps signal_kind → debit_weight.

- [ ] **Step 2: Replace hardcoded weight lookup with registry call**

If the function currently hardcodes signal_kind→weight, replace with `signal_weight_registry::weight_for(signal_kind)`. If a signal_kind has no entry in the registry, log a warning and apply weight 0 (no-op rather than panic — graceful degradation).

```rust
let weight = match signal_weight_registry::weight_for(&signal.signal_kind) {
    Some(w) => w.debit_weight,
    None => {
        tracing::warn!(
            target: "elohim_storage::standing",
            signal_kind = %signal.signal_kind,
            "no weight registered for signal_kind; applying 0 (no standing impact)"
        );
        0
    }
};
```

- [ ] **Step 3: Add integration test for each of the 3 new signal_kinds**

Create `elohim/elohim-storage/tests/standing_extension_integration.rs`:

```rust
//! Integration: projecting the 3 Z.D-related signal_kinds into standing_view
//! moves StandingScore in the expected direction.

use elohim_storage::services::standing::{Standing, StandingScore};
use elohim_storage::services::standing_projector::project_signal;
// ... harness imports

fn signal(kind: &str, target: &[u8], declarer: &[u8]) -> FeedbackSignalLike { /* ... */ }

#[test]
fn rate_limit_exceeded_signal_debits_target_standing() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    let evaluator = b"evaluator-pubkey";
    let target = b"deploy-svc-pubkey";

    // Start: Standing::Unknown
    assert_eq!(Standing::evaluate(evaluator, target, &mut conn), Standing::Unknown);

    // Project 10 rate-limit-exceeded signals (weight 5 each per manifest)
    for _ in 0..10 {
        let sig = signal("rate-limit-exceeded", target, evaluator);
        project_signal(&mut conn, &sig).unwrap();
    }

    // Standing should now be Low or Floor (depends on threshold mapping)
    let s = Standing::evaluate(evaluator, target, &mut conn);
    match s {
        Standing::Computed { score } => assert!(score == StandingScore::Low || score == StandingScore::Floor),
        Standing::Unknown => panic!("expected Computed after projecting 10 debit signals"),
    }
}

#[test]
fn bad_custody_signal_higher_weight() {
    // bad-custody weight=20 should debit harder than rate-limit-exceeded weight=5
    // 1 bad-custody >= 4 rate-limit-exceeded in debit impact
    // ...
}

#[test]
fn reach_escalation_pending_signal_debits_lightly() {
    // weight=3 — lighter than rate-limit-exceeded
    // ...
}
```

- [ ] **Step 4: Run + commit**

```bash
cargo test --manifest-path elohim/elohim-storage/Cargo.toml --test standing_extension_integration 2>&1 | tail -20
cargo test --manifest-path elohim/elohim-storage/Cargo.toml --lib services::standing 2>&1 | tail -10
git add elohim/elohim-storage/src/services/standing_projector.rs elohim/elohim-storage/tests/standing_extension_integration.rs
git commit -m "feat(storage): standing_projector reads signal weights from manifest registry"
```

---

### Task 9: GET /api/v1/standing/{agent_cid} route

**Files:**
- Create: `elohim/elohim-storage/src/api/standing.rs`
- Modify: `elohim/elohim-storage/src/api/mod.rs`
- Modify: `elohim/elohim-storage/src/http.rs`

- [ ] **Step 1: Implement handler**

```rust
// elohim/elohim-storage/src/api/standing.rs
use hyper::{Request, Response, StatusCode};
use hyper::body::{Bytes, Incoming};
use http_body_util::Full;

use crate::AppContext;
use crate::StorageError;
use crate::db::standing_view;
use crate::services::standing::{Standing, StandingScore};
use elohim_views::standing::{StandingScoreView, StandingScoreTier, FeedbackSignalSummary};

pub async fn handle_get_standing(
    _req: Request<Incoming>,
    agent_cid: &str,
    evaluator_cid: Option<&str>,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let evaluator_bytes = evaluator_cid
        .ok_or_else(|| StorageError::InvalidInput("?evaluator=<cid> query param required".into()))?
        .as_bytes()
        .to_vec();
    let subject_bytes = agent_cid.as_bytes().to_vec();

    let pool = ctx.db_pool.clone();
    let view = tokio::task::spawn_blocking(move || -> Result<StandingScoreView, StorageError> {
        let mut conn = pool.get().map_err(|e| StorageError::Database(e.to_string()))?;
        let standing = Standing::evaluate(&evaluator_bytes, &subject_bytes, &mut conn);
        let row = standing_view::fetch(&mut conn, &evaluator_bytes, &subject_bytes)
            .map_err(|e| StorageError::Database(e.to_string()))?;

        let tier = match standing {
            Standing::Unknown => StandingScoreTier::Unknown,
            Standing::Computed { score } => match score {
                StandingScore::Floor => StandingScoreTier::Floor,
                StandingScore::Low => StandingScoreTier::Low,
                StandingScore::Neutral => StandingScoreTier::Neutral,
                StandingScore::High => StandingScoreTier::High,
                StandingScore::Trusted => StandingScoreTier::Trusted,
            },
        };

        Ok(StandingScoreView {
            evaluator_cid: String::from_utf8(evaluator_bytes.clone()).unwrap_or_default(),
            subject_cid: String::from_utf8(subject_bytes.clone()).unwrap_or_default(),
            score: tier,
            debit_weight_sum: row.as_ref().map(|r| r.debit_weight_sum).unwrap_or(0),
            recent_breaches: vec![],  // TODO: implement recent-breaches join in follow-up
            computed_at: row.as_ref().map(|r| r.last_signal_at.clone()).unwrap_or_default(),
        })
    })
    .await
    .map_err(|e| StorageError::Internal(e.to_string()))??;

    let body = serde_json::to_vec(&view).map_err(|e| StorageError::Internal(e.to_string()))?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap())
}
```

The `recent_breaches` field is left as `vec![]` for this sprint — populating it requires a JOIN against the `feedback_signals` table by (subject, signal_kind, in last N days) which is a follow-up enhancement. Sprint 8's audit-trail UI will need this; capture as a follow-up.

- [ ] **Step 2: Wire route + parse path/query**

In `http.rs`, parse the URI `/api/v1/standing/{agent_cid}?evaluator=...`. Dispatch to `handle_get_standing`.

- [ ] **Step 3: Smoke test + commit**

```bash
cargo test --manifest-path elohim/elohim-storage/Cargo.toml --lib api::standing 2>&1 | tail -10
git add elohim/elohim-storage/src/api/standing.rs elohim/elohim-storage/src/api/mod.rs elohim/elohim-storage/src/http.rs
git commit -m "feat(storage): GET /api/v1/standing/{agent_cid}?evaluator=...; pluralism-aware"
```

---

### Task 10: Close-out + memory updates

**Files:**
- Append: `genesis/docs/superpowers/plans/2026-05-28-sprint2-bounds-validator-standing-aggregator.md` (close-out section, this file)
- Update: `genesis/docs/superpowers/plans/2026-05-28-rea-compute-substrate-native-roadmap.md` (mark S2 complete)
- Create: `.claude/memory/project_bounds_validator_pattern.md`
- Update: `.claude/memory/MEMORY.md`

- [ ] **Step 1: Append close-out section to this plan**

```markdown
## Close-out — Sprint 2 landed

Sprint 2 commits on `sprint/<branch>`:
- <SHA list with one-line subjects>

Total: <N> commits over <span>.

**What landed:** bounds_validator with 6 violation kinds + happy path; CommitmentFetcher + RateHistory trait surfaces with diesel and mock impls; new bounded_by column on economic_events with indexed lookup; signal_weight_registry reading from elohim manifest; standing_projector extended for 4 signal_kinds; two diagnostic HTTP routes (POST /api/v1/diagnostics/validate-bounds, GET /api/v1/standing/{agent_cid}); two new View schemas (BoundsValidationResultView, StandingScoreView) with ts-rs export and contract tests.

**Unblocked:** Sprint 1 republish_epr_validator (instance of bounds_validator); Sprints 3 + 5a-e (each authors its instance validator that delegates to this primitive); Sprint 6 matchmaking gate (consumes standing).

**Follow-up captured (not blocking):**
- `recent_breaches` field on StandingScoreView is empty in this sprint; populating requires a feedback_signals JOIN — Sprint 8 needs it for the audit UI.
- Pattern-hunter scan for existing X-API-Key / `if admin:` / ad-hoc auth (Sprint 2 roadmap entry referenced this) is captured here for explicit follow-up sprint; not yet executed.
```

- [ ] **Step 2: Update roadmap S2 marker**

In `genesis/docs/superpowers/plans/2026-05-28-rea-compute-substrate-native-roadmap.md` Phase A table, change `S2 ▢` → `S2 ✓`.

- [ ] **Step 3: Create pattern memory**

`.claude/memory/project_bounds_validator_pattern.md`:

```markdown
---
name: bounds-validator-pattern
description: "Single substrate-side bounds_validator::validate function that every per-instance per-row-of-the-table validator delegates to. Walks bounded_by → Commitment → 7 checks. CommitmentFetcher + RateHistory traits enable mocking without conductor."
metadata:
  node_type: memory
  type: project
---

When implementing a per-instance validator (Sprint 1's republish_epr_validator, Sprint 3's serve_url_projection_validator, Sprints 5a-e's per-row validators), DELEGATE to `services::bounds_validator::validate` for the substrate-wide concerns. Per-instance validators only handle (1) schema validation of the action's specific payload, (2) action-discriminator check, and (3) construction of an `EventForValidation` projection. The substrate-wide concerns — Commitment fetch, active-window, scope-includes-event, reach-ceiling, rate-limit, key-rotation, revoked — all live in one function.

**Why:** revocation propagation and rate-limit discipline must be uniform across all 7 rows of the gospel-tier generalization table. One implementation; one place to fix bugs; one place to audit. Per `[[project_rea_compute_commitment_primitive]]` §4 auditability properties.

**How to apply:**
1. Build your per-instance validator at `services/<instance>_validator.rs`.
2. Schema-validate the event payload against `elohim/sdk/schemas/v1/economic-events/<instance>.schema.json`.
3. Convert your view to `EventForValidation { action, performer, bounded_by, target_epr_id, reach, signed_at }`.
4. Call `bounds_validator::validate(&event, fetcher, rate_history).await`.
5. On `BoundsViolation`, emit the appropriate FeedbackSignal — `rate-limit-exceeded` for that kind, `bad-custody` for revoked/expired, `reach-escalation-pending` for ReachCeilingExceeded, etc.

**Reference:** `elohim/elohim-storage/src/services/bounds_validator.rs`. First instance: Sprint 1's `republish_epr_validator`.

**Related:** `[[project_rea_compute_commitment_primitive]]`, `[[project_signal_kind_extensible_protocol_class]]` (the signal_weight_registry uses this extension pattern), `[[project_canonical_wire_shape_newtype_pattern]]` (CommitmentCid and AgentCid are candidates for newtype hardening in a follow-up).
```

- [ ] **Step 4: Add MEMORY.md entry**

```markdown
- [Bounds-validator pattern](project_bounds_validator_pattern.md) — single substrate-side validate function that all per-instance validators delegate to; 7 checks; CommitmentFetcher + RateHistory traits enable mocking.
```

- [ ] **Step 5: Commit + close**

```bash
git add genesis/docs/superpowers/plans/2026-05-28-sprint2-bounds-validator-standing-aggregator.md \
        genesis/docs/superpowers/plans/2026-05-28-rea-compute-substrate-native-roadmap.md \
        .claude/memory/project_bounds_validator_pattern.md \
        .claude/memory/MEMORY.md
git commit -m "docs(memory): Sprint 2 close-out + bounds-validator pattern memory"
```

---

# Self-Review

**Spec coverage:** every Sprint 2 deliverable from the roadmap is decomposed into a task above — bounds_validator (Tasks 2-5), standing extension (Tasks 7-8), diagnostic routes (Tasks 6 + 9), schemas + views (Task 1), close-out (Task 10). The pattern-hunter pass for X-API-Key absorption is deferred to a separate follow-up sprint per the Task 10 close-out.

**Placeholder scan:** the only "TODO" comments are explicit follow-ups (recent_breaches join in StandingScoreView; conductor production impl of CommitmentFetcher pending Sprint 1's hc_client extension). Both are flagged in their task descriptions as deliberate Sprint-2-scope decisions.

**Type consistency:** `EventForValidation` is introduced in Task 4 and consumed in Task 6's diagnostic route. `CommitmentRecord` from Task 2 is used in Task 4's tests. `BoundsViolation` is the validate return-error type throughout. `StandingScoreView` / `StandingScoreTier` from Task 1 used in Task 9. `ViolationKind` is consistent between schema (snake_case enum) and Rust (PascalCase enum with `serde(rename_all = "snake_case")`). The `bounded_by` column added in Task 3 is queried in Task 3 itself.

# Execution Handoff

**Plan saved to** `genesis/docs/superpowers/plans/2026-05-28-sprint2-bounds-validator-standing-aggregator.md`. **Two execution paths per writing-plans skill:**

1. **Subagent-Driven (recommended for Sprint 2)** — dispatch one implementer per task; spec compliance + code-quality reviews after each. Sonnet for Tasks 1-5 (judgment + multi-file), Haiku for Tasks 6-9 (mechanical). Tasks 4-5 are the substrate keystone — operator-watch those reviews.

2. **Inline batch execution** — execute in this session using executing-plans. Suitable if operator wants the foundation primitive landed quickly before Sprint 1 picks up.

**Recommended pairing:** Sprint 1 implementation can start in parallel with Sprint 2 Task 5 completion — Sprint 1's `republish_epr_validator` constructs an `EventForValidation` and calls `bounds_validator::validate`. As long as Task 4 has landed (validate API surface stable), Sprint 1 can proceed against the trait surface even while Tasks 5-9 finish.
