# Elohim Token Sprint 2: ResponsibilityDemandParam Curve Enforcement

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the ResponsibilityDemandParam curve — the structural mechanism that couples power with responsibility. As agents accumulate tokens, obligations increase. The curve is context-aware per governance layer (Robeyns insight: different social contracts produce different limits).

**Architecture:** A `responsibility_demand_configs` table stores curve parameters per governance layer. A `ResponsibilityDemandService` evaluates an agent's position on the curve and returns obligation requirements. The `TokenLedgerService.transfer()` method checks the curve before allowing transfers. Curve configs are qahal-governed (created/updated through governance, not arbitrary API calls).

**Tech Stack:** Rust (elohim-storage), Diesel ORM, SQLite

**Design spec:** `genesis/plans/2026-04-01-elohim-token-epr-native-minting-design.md`

---

## File Map

### New Files

| File | Responsibility |
|------|---------------|
| `elohim/elohim-storage/migrations/2026-04-01-100000_responsibility_demand_configs/up.sql` | Config table migration |
| `elohim/elohim-storage/migrations/2026-04-01-100000_responsibility_demand_configs/down.sql` | Rollback |
| `elohim/elohim-storage/src/db/responsibility_demand_configs.rs` | CRUD for config table |
| `elohim/elohim-storage/src/services/responsibility_demand_service.rs` | Curve evaluation logic |

### Modified Files

| File | Change |
|------|--------|
| `elohim/elohim-storage/src/db/diesel_schema.rs` | Add responsibility_demand_configs table macro |
| `elohim/elohim-storage/src/db/models.rs` | Add ResponsibilityDemandConfig + NewResponsibilityDemandConfig models |
| `elohim/elohim-storage/src/db/mod.rs` | Add `pub mod responsibility_demand_configs;` |
| `elohim/elohim-storage/src/views.rs` | Add ResponsibilityDemandConfigView + input views |
| `elohim/elohim-storage/src/services/mod.rs` | Add `pub mod responsibility_demand_service;` |
| `elohim/elohim-storage/src/services/token_ledger_service.rs` | Add curve check before transfers |
| `elohim/elohim-storage/src/api/token.rs` | Add config query/create routes |

---

## Task 1: Migration + Diesel Schema + Models

Create the responsibility_demand_configs table and Rust types. This is the foundation — everything else depends on it.

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-04-01-100000_responsibility_demand_configs/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-04-01-100000_responsibility_demand_configs/down.sql`
- Modify: `elohim/elohim-storage/src/db/diesel_schema.rs`
- Modify: `elohim/elohim-storage/src/db/models.rs`
- Modify: `elohim/elohim-storage/src/db/mod.rs`

- [ ] **Step 1: Create migration up.sql**

```sql
-- ResponsibilityDemandConfig: curve parameters per governance layer
-- Category A (constitutional) — qahal-governed, changes require consent process
CREATE TABLE responsibility_demand_configs (
    id TEXT PRIMARY KEY NOT NULL,
    h_app_id TEXT NOT NULL DEFAULT 'shefa',
    governance_layer TEXT NOT NULL,
    dignity_floor REAL NOT NULL DEFAULT 100.0,
    median_estimate REAL NOT NULL DEFAULT 1000.0,
    soft_ceiling_multiplier REAL NOT NULL DEFAULT 10.0,
    hard_ceiling_multiplier REAL NOT NULL DEFAULT 20.0,
    social_contract_health REAL NOT NULL DEFAULT 0.5,
    enforcement_active INTEGER NOT NULL DEFAULT 1,
    ratified_by TEXT,
    ratified_at TEXT,
    dht_anchor_hash TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(h_app_id, governance_layer)
);

CREATE INDEX idx_rdc_h_app_id ON responsibility_demand_configs(h_app_id);
CREATE INDEX idx_rdc_governance_layer ON responsibility_demand_configs(governance_layer);
```

- [ ] **Step 2: Create migration down.sql**

```sql
DROP TABLE IF EXISTS responsibility_demand_configs;
```

- [ ] **Step 3: Add Diesel table macro to diesel_schema.rs**

Append after existing token table macros:

```rust
diesel::table! {
    responsibility_demand_configs (id) {
        id -> Text,
        h_app_id -> Text,
        governance_layer -> Text,
        dignity_floor -> Float,
        median_estimate -> Float,
        soft_ceiling_multiplier -> Float,
        hard_ceiling_multiplier -> Float,
        social_contract_health -> Float,
        enforcement_active -> Integer,
        ratified_by -> Nullable<Text>,
        ratified_at -> Nullable<Text>,
        dht_anchor_hash -> Nullable<Text>,
        created_at -> Text,
        updated_at -> Text,
    }
}
```

Also add `responsibility_demand_configs` to the `allow_tables_to_appear_in_same_query!` macro.

- [ ] **Step 4: Add models to models.rs**

Append:

```rust
// ── Responsibility Demand Config ──

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = responsibility_demand_configs)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ResponsibilityDemandConfig {
    pub id: String,
    pub h_app_id: String,
    pub governance_layer: String,
    pub dignity_floor: f32,
    pub median_estimate: f32,
    pub soft_ceiling_multiplier: f32,
    pub hard_ceiling_multiplier: f32,
    pub social_contract_health: f32,
    pub enforcement_active: i32,
    pub ratified_by: Option<String>,
    pub ratified_at: Option<String>,
    pub dht_anchor_hash: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = responsibility_demand_configs)]
pub struct NewResponsibilityDemandConfig<'a> {
    pub id: &'a str,
    pub h_app_id: &'a str,
    pub governance_layer: &'a str,
    pub dignity_floor: f32,
    pub median_estimate: f32,
    pub soft_ceiling_multiplier: f32,
    pub hard_ceiling_multiplier: f32,
    pub social_contract_health: f32,
    pub enforcement_active: i32,
    pub ratified_by: Option<&'a str>,
    pub ratified_at: Option<&'a str>,
    pub dht_anchor_hash: Option<&'a str>,
}
```

- [ ] **Step 5: Add module to mod.rs**

```rust
pub mod responsibility_demand_configs;
```

- [ ] **Step 6: Verify compilation**

```bash
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS="" cargo check
```

- [ ] **Step 7: Commit**

```bash
git add elohim/elohim-storage/migrations/2026-04-01-100000_responsibility_demand_configs/ \
        elohim/elohim-storage/src/db/diesel_schema.rs \
        elohim/elohim-storage/src/db/models.rs \
        elohim/elohim-storage/src/db/mod.rs
git commit -m "feat(storage): add responsibility_demand_configs table and models

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: CRUD Module + Views

**Files:**
- Create: `elohim/elohim-storage/src/db/responsibility_demand_configs.rs`
- Modify: `elohim/elohim-storage/src/views.rs`

- [ ] **Step 1: Write responsibility_demand_configs.rs CRUD**

Functions needed:
- `get_config_for_layer(conn, ctx, governance_layer) -> Result<Option<ResponsibilityDemandConfig>>` — query by governance_layer
- `get_all_configs(conn, ctx) -> Result<Vec<ResponsibilityDemandConfig>>` — list all
- `create_config(conn, ctx, new: NewResponsibilityDemandConfig) -> Result<ResponsibilityDemandConfig>` — insert
- `update_config(conn, ctx, id, updates) -> Result<ResponsibilityDemandConfig>` — update params

Follow the exact pattern from `db/token_mint_events.rs` — filter by `h_app_id` from ctx.

- [ ] **Step 2: Add View types to views.rs**

```rust
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ResponsibilityDemandConfigView {
    pub id: String,
    pub governance_layer: String,
    pub dignity_floor: f32,
    pub median_estimate: f32,
    pub soft_ceiling_multiplier: f32,
    pub hard_ceiling_multiplier: f32,
    pub social_contract_health: f32,
    pub enforcement_active: bool,
    pub ratified_by: Option<String>,
    pub ratified_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ResponsibilityDemandConfig> for ResponsibilityDemandConfigView {
    fn from(c: ResponsibilityDemandConfig) -> Self {
        Self {
            id: c.id,
            governance_layer: c.governance_layer,
            dignity_floor: c.dignity_floor,
            median_estimate: c.median_estimate,
            soft_ceiling_multiplier: c.soft_ceiling_multiplier,
            hard_ceiling_multiplier: c.hard_ceiling_multiplier,
            social_contract_health: c.social_contract_health,
            enforcement_active: c.enforcement_active == 1,
            ratified_by: c.ratified_by,
            ratified_at: c.ratified_at,
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateResponsibilityDemandConfigInputView {
    pub governance_layer: String,
    pub dignity_floor: Option<f32>,
    pub median_estimate: Option<f32>,
    pub soft_ceiling_multiplier: Option<f32>,
    pub hard_ceiling_multiplier: Option<f32>,
    pub social_contract_health: Option<f32>,
    pub enforcement_active: Option<bool>,
}
```

- [ ] **Step 3: Verify and commit**

```bash
RUSTFLAGS="" cargo check
git add src/db/responsibility_demand_configs.rs src/views.rs
git commit -m "feat(storage): add ResponsibilityDemandConfig CRUD and views

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: ResponsibilityDemandService — The Curve

This is the core of Sprint 2 — the curve evaluation logic.

**Files:**
- Create: `elohim/elohim-storage/src/services/responsibility_demand_service.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`

- [ ] **Step 1: Write the service**

The service evaluates an agent's position on the curve and returns an `ObligationLevel`:

```rust
use diesel::SqliteConnection;
use crate::db::context::AppContext;
use crate::db::responsibility_demand_configs;
use crate::db::token_balances;
use crate::error::StorageError;

/// The agent's position on the responsibility demand curve
#[derive(Debug, Clone, PartialEq)]
pub enum ObligationLevel {
    /// Below dignity floor — no demands, supported by commons
    Supported,
    /// Floor to median — normal circulation, minimal obligations  
    Normal,
    /// Median to soft ceiling — increasing governance visibility
    Elevated { visibility_required: bool },
    /// Soft to hard ceiling — significant responsibility, stewardship required
    High { stewardship_required: bool, justification_required: bool },
    /// Above hard ceiling — extreme responsibility, elohim scrutiny
    Extreme { elohim_review_required: bool, constitutional_justification: bool },
}

pub struct ResponsibilityDemandService;

impl ResponsibilityDemandService {
    /// Evaluate an agent's obligation level based on their current balance
    /// and the governance layer's curve configuration.
    pub fn evaluate(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        agent_id: &str,
        governance_layer: &str,
    ) -> Result<ObligationLevel, StorageError> {
        let config = responsibility_demand_configs::get_config_for_layer(
            conn, ctx, governance_layer,
        )?;

        // If no config exists, default to Normal (no curve enforcement)
        let config = match config {
            Some(c) if c.enforcement_active == 1 => c,
            _ => return Ok(ObligationLevel::Normal),
        };

        let balance = token_balances::get_balance(conn, ctx, agent_id, governance_layer)?
            .map(|b| b.balance)
            .unwrap_or(0.0);

        Ok(Self::evaluate_position(balance, &config))
    }

    /// Pure function: given a balance and config, return the obligation level.
    /// This is the curve itself.
    fn evaluate_position(
        balance: f32,
        config: &crate::db::models::ResponsibilityDemandConfig,
    ) -> ObligationLevel {
        let floor = config.dignity_floor;
        let median = config.median_estimate;
        let soft_ceiling = median * config.soft_ceiling_multiplier;
        let hard_ceiling = median * config.hard_ceiling_multiplier;

        if balance < floor {
            ObligationLevel::Supported
        } else if balance < median {
            ObligationLevel::Normal
        } else if balance < soft_ceiling {
            ObligationLevel::Elevated {
                visibility_required: true,
            }
        } else if balance < hard_ceiling {
            ObligationLevel::High {
                stewardship_required: true,
                justification_required: true,
            }
        } else {
            ObligationLevel::Extreme {
                elohim_review_required: true,
                constitutional_justification: true,
            }
        }
    }

    /// Check if a transfer is allowed given the sender's obligation level.
    /// Returns Ok(()) if allowed, Err with reason if blocked.
    pub fn check_transfer_allowed(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        from_agent: &str,
        amount: f32,
        governance_layer: &str,
    ) -> Result<(), StorageError> {
        let level = Self::evaluate(conn, ctx, from_agent, governance_layer)?;

        match level {
            ObligationLevel::Supported | ObligationLevel::Normal => Ok(()),
            ObligationLevel::Elevated { .. } => {
                // Elevated: transfers allowed but logged with visibility
                // Sprint 2: allow, Sprint 3+ may add logging
                Ok(())
            }
            ObligationLevel::High { .. } => {
                // High: large transfers require justification
                // For now, allow but could gate on note/justification field
                Ok(())
            }
            ObligationLevel::Extreme { .. } => {
                // Extreme: large transfers blocked pending elohim review
                // For Sprint 2: warn but allow (full enforcement in Sprint 3 with discernment)
                eprintln!(
                    "[responsibility-demand] agent {} at Extreme level, transfer of {} proceeding with warning",
                    from_agent, amount
                );
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::ResponsibilityDemandConfig;

    fn test_config() -> ResponsibilityDemandConfig {
        ResponsibilityDemandConfig {
            id: "test".into(),
            h_app_id: "shefa".into(),
            governance_layer: "individual".into(),
            dignity_floor: 100.0,
            median_estimate: 1000.0,
            soft_ceiling_multiplier: 10.0,  // soft ceiling at 10,000
            hard_ceiling_multiplier: 20.0,  // hard ceiling at 20,000
            social_contract_health: 0.5,
            enforcement_active: 1,
            ratified_by: None,
            ratified_at: None,
            dht_anchor_hash: None,
            created_at: "2026-04-01".into(),
            updated_at: "2026-04-01".into(),
        }
    }

    #[test]
    fn test_below_dignity_floor() {
        let config = test_config();
        assert_eq!(
            ResponsibilityDemandService::evaluate_position(50.0, &config),
            ObligationLevel::Supported
        );
    }

    #[test]
    fn test_normal_range() {
        let config = test_config();
        assert_eq!(
            ResponsibilityDemandService::evaluate_position(500.0, &config),
            ObligationLevel::Normal
        );
    }

    #[test]
    fn test_elevated_range() {
        let config = test_config();
        let level = ResponsibilityDemandService::evaluate_position(5000.0, &config);
        assert!(matches!(level, ObligationLevel::Elevated { visibility_required: true }));
    }

    #[test]
    fn test_high_range() {
        let config = test_config();
        let level = ResponsibilityDemandService::evaluate_position(15000.0, &config);
        assert!(matches!(level, ObligationLevel::High { .. }));
    }

    #[test]
    fn test_extreme_range() {
        let config = test_config();
        let level = ResponsibilityDemandService::evaluate_position(25000.0, &config);
        assert!(matches!(level, ObligationLevel::Extreme { .. }));
    }

    #[test]
    fn test_at_boundaries() {
        let config = test_config();
        // At exactly the floor
        assert_eq!(
            ResponsibilityDemandService::evaluate_position(100.0, &config),
            ObligationLevel::Normal
        );
        // At exactly the median
        assert_eq!(
            ResponsibilityDemandService::evaluate_position(1000.0, &config),
            ObligationLevel::Normal
        );
    }
}
```

- [ ] **Step 2: Add module to services/mod.rs**

```rust
pub mod responsibility_demand_service;
```

- [ ] **Step 3: Run tests**

```bash
RUSTFLAGS="" cargo test responsibility_demand -- --nocapture
```

Expected: 6 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/services/responsibility_demand_service.rs src/services/mod.rs
git commit -m "feat(token): add ResponsibilityDemandService with curve evaluation

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

## Task 4: Integrate Curve into Token Ledger

Wire the curve check into `TokenLedgerService::transfer()`.

**Files:**
- Modify: `elohim/elohim-storage/src/services/token_ledger_service.rs`

- [ ] **Step 1: Add import**

```rust
use crate::services::responsibility_demand_service::ResponsibilityDemandService;
```

- [ ] **Step 2: Add curve check to transfer()**

After the validation checks (amount > 0, not self-transfer) and BEFORE the debit call, add:

```rust
        // Check responsibility demand curve
        ResponsibilityDemandService::check_transfer_allowed(
            conn, ctx, from_agent, amount, governance_layer,
        )?;
```

- [ ] **Step 3: Verify and commit**

```bash
RUSTFLAGS="" cargo check
RUSTFLAGS="" cargo test --lib --bins 2>&1 | tail -5
git add src/services/token_ledger_service.rs
git commit -m "feat(token): integrate ResponsibilityDemandParam curve check into transfers

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

## Task 5: Config API Routes

Add endpoints for querying and creating curve configurations.

**Files:**
- Modify: `elohim/elohim-storage/src/api/token.rs`

- [ ] **Step 1: Add config routes**

Add to the match in handle():
- `GET /api/v1/token/config/{governance_layer}` — returns config for a layer
- `GET /api/v1/token/config` — returns all configs
- `POST /api/v1/token/config` — creates a config (qahal-governed in production, direct for now)
- `GET /api/v1/token/obligation/{agent_id}/{governance_layer}` — evaluates and returns an agent's current obligation level

The obligation endpoint is the key consumer-facing query — "where am I on the curve?"

- [ ] **Step 2: Add handler functions**

Follow the existing handler pattern in token.rs for the new routes.

- [ ] **Step 3: Verify and commit**

```bash
RUSTFLAGS="" cargo check
git add src/api/token.rs
git commit -m "feat(api): add responsibility demand config and obligation query routes

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

## Task 6: Integration Verification

- [ ] **Step 1: Full build**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS="" cargo build --release 2>&1 | tail -5
```

- [ ] **Step 2: All tests**

```bash
RUSTFLAGS="" cargo test --lib --bins 2>&1 | tail -10
```

- [ ] **Step 3: Verify new tests pass**

```bash
RUSTFLAGS="" cargo test responsibility_demand -- --nocapture 2>&1 | tail -15
```

- [ ] **Step 4: Commit any generated files**

```bash
RUSTFLAGS="" cargo test export_bindings 2>&1 | tail -5
git add elohim/sdk/storage-client-ts/src/generated/ResponsibilityDemandConfigView.ts \
        elohim/sdk/storage-client-ts/src/generated/CreateResponsibilityDemandConfigInputView.ts
git commit -m "chore: add generated TypeScript types for responsibility demand config

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

## Sprint 2 Deliverables

1. **ResponsibilityDemandConfig** table and models — stores curve parameters per governance layer
2. **ResponsibilityDemandService** — evaluates agent position on curve, returns obligation level
3. **Curve integration** — transfers check the curve before proceeding
4. **5 obligation levels** — Supported, Normal, Elevated, High, Extreme
5. **Context-aware** — each governance layer can have different curve parameters (Robeyns insight)
6. **API routes** — query configs, evaluate obligation level
7. **6+ unit tests** — curve evaluation at boundaries and ranges

## What Sprint 2 does NOT do (deferred to Sprint 3+)

- Hard blocking of transfers at Extreme level (needs elohim discernment infrastructure)
- Automatic social_contract_health sensing (needs elohim observation)
- Story-memory decay (Sprint 3)
- Governance process for config changes (Sprint 5+, needs qahal infrastructure)
