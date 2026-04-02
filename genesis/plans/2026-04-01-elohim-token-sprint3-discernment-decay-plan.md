# Elohim Token Sprint 3: Discernment Minting & Story-Memory Decay

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Tier 2 minting (elohim discernment — periodic pattern recognition with attestation and reasoning traces) and story-memory decay (tokens backed by dormant stories decay gently, dignity floor protected).

**Architecture:** Discernment minting reuses the existing `NewTokenMintEvent` struct (which already has `elohim_attestation` and `reasoning_trace` fields). A new `discernment_mint()` function accepts explicit amounts with elohim attestation. Decay uses a new `token_decay_events` audit table and a service that evaluates obligation levels to set decay rates, respecting the dignity floor. Decay is triggered via API (future: scheduled by elohim agents).

**Tech Stack:** Rust (elohim-storage), Diesel ORM, SQLite

---

## Task 1: Decay Events Migration + Models

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-04-01-200000_token_decay_events/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-04-01-200000_token_decay_events/down.sql`
- Modify: `elohim/elohim-storage/src/db/diesel_schema.rs`
- Modify: `elohim/elohim-storage/src/db/models.rs`
- Modify: `elohim/elohim-storage/src/db/mod.rs`

### up.sql
```sql
CREATE TABLE token_decay_events (
    id TEXT PRIMARY KEY NOT NULL,
    h_app_id TEXT NOT NULL DEFAULT 'shefa',
    agent_id TEXT NOT NULL,
    governance_layer TEXT NOT NULL,
    balance_before REAL NOT NULL,
    balance_after REAL NOT NULL,
    decay_amount REAL NOT NULL,
    obligation_level TEXT NOT NULL,
    dignity_floor REAL NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_token_decay_agent ON token_decay_events(agent_id);
CREATE INDEX idx_token_decay_h_app ON token_decay_events(h_app_id);
CREATE INDEX idx_token_decay_created ON token_decay_events(created_at);
```

### down.sql
```sql
DROP TABLE IF EXISTS token_decay_events;
```

### Diesel + Models
- Add `diesel::table!` macro for `token_decay_events`
- Add to `allow_tables_to_appear_in_same_query!`
- Add `TokenDecayEvent` (Queryable) and `NewTokenDecayEvent<'a>` (Insertable) to models.rs
- Add `pub mod token_decay_events;` to db/mod.rs

---

## Task 2: Decay Events CRUD + Views

**Files:**
- Create: `elohim/elohim-storage/src/db/token_decay_events.rs`
- Modify: `elohim/elohim-storage/src/views.rs`

### CRUD functions
- `create_decay_event(conn, ctx, new: NewTokenDecayEvent) -> Result<TokenDecayEvent>`
- `get_decay_events_for_agent(conn, ctx, agent_id) -> Result<Vec<TokenDecayEvent>>`

### View types
- `TokenDecayEventView` (Serialize + TS) with `From<TokenDecayEvent>` — convert `obligation_level` string directly
- `DiscernmentMintInputView` (Deserialize + TS) — input for discernment mint API:
  - `agent_id: String` (required)
  - `governance_layer: Option<String>` (default "individual")
  - `amount: f32` (required)
  - `elohim_attestation: String` (required)
  - `reasoning_trace: String` (required)
  - `source_epr_id: Option<String>` (optional — discernment may reference content or be agent-level)

---

## Task 3: Discernment Mint Function

**Files:**
- Modify: `elohim/elohim-storage/src/services/token_mint_service.rs`

Add `discernment_mint()` to `TokenMintService`:

```rust
pub fn discernment_mint(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    agent_id: &str,
    governance_layer: &str,
    amount: f32,
    elohim_attestation: &str,
    reasoning_trace: &str,
    source_epr_id: Option<&str>,
) -> Result<TokenMintEventView, StorageError>
```

Key differences from `mint_for_recognition`:
- Amount is explicit (set by elohim evaluation), not calculated from weights
- `mint_tier` = `"discernment"` (not `"micro"`)
- `elohim_attestation` and `reasoning_trace` are populated (not None)
- `provenance_event_id` generated internally (discernment is self-referencing — the mint IS the provenance)
- `source_epr_id` is optional (discernment may be about an agent's pattern, not a specific content)
- Mint ID: `"mint-discern-" + SHA256(agent_id + governance_layer + timestamp)[..8]`

### Unit tests
- `test_discernment_mint_sets_tier` — verify mint_tier is "discernment"
- `test_discernment_mint_requires_attestation` — non-empty attestation required
- `test_discernment_mint_requires_positive_amount` — amount must be > 0

---

## Task 4: Story-Memory Decay Service

**Files:**
- Create: `elohim/elohim-storage/src/services/token_decay_service.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`

### Core functions

**calculate_decay_rate(obligation_level) -> f32:**
Pure function mapping obligation level to per-period decay rate:
- Supported: 0.0 (no decay — protected)
- Normal: 0.001 (0.1% per period)
- Elevated: 0.005 (0.5% per period)
- High: 0.02 (2% per period)
- Extreme: 0.05 (5% per period)

**apply_decay(conn, ctx, agent_id, governance_layer) -> Result<DecayResult>:**
1. Fetch config via `responsibility_demand_configs::get_config_for_layer()`
2. If no config or enforcement_active == 0, return DecayResult with no decay
3. Fetch balance via `token_balances::get_balance()`
4. If balance is None or 0, return no decay
5. Evaluate obligation level via `ResponsibilityDemandService::evaluate_position()`
6. Calculate decay: `balance * calculate_decay_rate(level)`
7. Clamp: `new_balance = max(balance - decay, config.dignity_floor)` — never below floor
8. Actual decay = `balance - new_balance` (may be less than calculated if floor hit)
9. If actual_decay > 0: call `token_balances::debit_balance()` and record `TokenDecayEvent`
10. Return `DecayResult`

**DecayResult struct:**
```rust
pub struct DecayResult {
    pub decay_applied: bool,
    pub amount: f32,
    pub balance_before: f32,
    pub balance_after: f32,
    pub obligation_level: String,
    pub dignity_floor_protected: bool,
}
```

### Unit tests (pure function only)
- `test_decay_rate_supported_is_zero` — no decay at supported level
- `test_decay_rate_normal` — 0.1% decay
- `test_decay_rate_extreme` — 5% decay
- `test_decay_respects_dignity_floor` — decay clamps to floor
- `test_no_decay_below_floor` — balance already below floor → 0 decay

---

## Task 5: API Routes

**Files:**
- Modify: `elohim/elohim-storage/src/api/token.rs`

Add routes:

### POST /api/v1/token/discernment-mint
Accepts `DiscernmentMintInputView`. Calls `TokenMintService::discernment_mint()`. Returns `TokenMintEventView`.

### POST /api/v1/token/apply-decay/{agent_id}/{governance_layer}
Triggers decay evaluation for an agent. Calls `TokenDecayService::apply_decay()`. Returns `DecayResult` as JSON. In production, this is called by elohim agents on a schedule; the API enables testing and manual invocation.

### GET /api/v1/token/decay-history/{agent_id}
Returns `Vec<TokenDecayEventView>` — the immutable audit trail of all decay events for an agent.

---

## Task 6: Integration Verification

- Full build: `RUSTFLAGS="" cargo build --release`
- All tests: `RUSTFLAGS="" cargo test --lib --bins`
- TypeScript codegen: `RUSTFLAGS="" cargo test export_bindings`
- Verify no "amplification" references: `grep -rn "amplification" src/services/token_*`

---

## Sprint 3 Deliverables

1. **Discernment minting** — elohim agents can mint tokens with attestation + reasoning traces
2. **Story-memory decay** — dormant tokens decay based on obligation level
3. **Dignity floor protection** — balance never drops below floor
4. **Decay audit trail** — immutable record of every decay event
5. **3 new API routes** — discernment-mint, apply-decay, decay-history
6. **8+ unit tests** — decay rates, floor protection, discernment mint validation
