# Elohim Token Sprint 1: Core Minting Loop

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire the micro-mint step into the existing recognition pipeline so that EPR content delivery produces elohim-token mint events with balance tracking.

**Architecture:** Extend the 5-stage recognition pipeline with a 6th "mint" stage injected after "settle." New protocol schemas define the token primitives. A database migration adds token tables. A mint service calculates amounts from recognition policy weights and a network mint rate. A ledger service tracks balances. An API exposes balance queries and mint history.

**Tech Stack:** Rust (elohim-storage), Diesel ORM, SQLite, JSON Schema, ts-rs codegen

**Design spec:** `genesis/plans/2026-04-01-elohim-token-epr-native-minting-design.md`

**Sprint scope:** Micro-mint only (Tier 1). No discernment mints, no ResponsibilityDemandParam curve enforcement, no decay, no bridge — those are Sprints 2-4.

---

## File Map

### New Files

| File | Responsibility |
|------|---------------|
| `elohim/sdk/schemas/v1/objects/token-mint-event.schema.json` | Protocol schema for mint events |
| `elohim/sdk/schemas/v1/objects/token-transfer.schema.json` | Protocol schema for transfers |
| `elohim/sdk/schemas/v1/objects/token-balance.schema.json` | Protocol schema for balances |
| `elohim/sdk/schemas/v1/inputs/create-token-transfer-input.schema.json` | Input schema for transfer creation |
| `elohim/elohim-storage/migrations/2026-04-01-000000_token_tables/up.sql` | Token tables migration |
| `elohim/elohim-storage/migrations/2026-04-01-000000_token_tables/down.sql` | Token tables rollback |
| `elohim/elohim-storage/src/db/token_mint_events.rs` | Diesel CRUD for mint events |
| `elohim/elohim-storage/src/db/token_balances.rs` | Diesel CRUD for balances |
| `elohim/elohim-storage/src/db/token_transfers.rs` | Diesel CRUD for transfers |
| `elohim/elohim-storage/src/services/token_mint_service.rs` | Mint calculation + creation |
| `elohim/elohim-storage/src/services/token_ledger_service.rs` | Balance tracking + transfer validation |
| `elohim/elohim-storage/src/api/token.rs` | HTTP routes for token queries |

### Modified Files

| File | Change |
|------|--------|
| `elohim/elohim-storage/src/db/mod.rs` | Add `pub mod token_mint_events; pub mod token_balances; pub mod token_transfers;` |
| `elohim/elohim-storage/src/db/diesel_schema.rs` | Add Diesel table macros for new tables |
| `elohim/elohim-storage/src/db/models.rs` | Add Queryable/Insertable structs for token tables |
| `elohim/elohim-storage/src/views.rs` | Add TokenMintEventView, TokenBalanceView, TokenTransferView, CreateTokenTransferInputView |
| `elohim/elohim-storage/src/services/mod.rs` | Add `pub mod token_mint_service; pub mod token_ledger_service;` |
| `elohim/elohim-storage/src/services/recognition_pipeline_service.rs` | Inject mint step after settle stage |
| `elohim/elohim-storage/src/api/mod.rs` | Add token route dispatcher |

---

## Task 1: Protocol Schemas

**Files:**
- Create: `elohim/sdk/schemas/v1/objects/token-mint-event.schema.json`
- Create: `elohim/sdk/schemas/v1/objects/token-balance.schema.json`
- Create: `elohim/sdk/schemas/v1/objects/token-transfer.schema.json`
- Create: `elohim/sdk/schemas/v1/inputs/create-token-transfer-input.schema.json`

- [ ] **Step 1: Create token-mint-event schema**

```json
{
  "$id": "epr:schema:object:token-mint-event",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "TokenMintEvent",
  "description": "Immutable record of elohim-token minting. Every mint is coupled to a witnessed REA economic event — there is no free-floating token creation.",
  "type": "object",
  "required": ["id", "amount", "provenanceEventId", "mintTier", "sourceEprId", "agentId"],
  "properties": {
    "id": { "type": "string", "description": "Unique mint event ID" },
    "amount": { "type": "number", "description": "Amount of elohim-token minted" },
    "provenanceEventId": { "type": "string", "description": "ID of the REA economic event that triggered this mint" },
    "mintTier": { "type": "string", "enum": ["micro", "discernment"], "description": "Whether this was a deterministic micro-mint or an elohim discernment mint" },
    "sourceEprId": { "type": "string", "description": "Content ID of the EPR that this mint is coupled to" },
    "agentId": { "type": "string", "description": "Agent receiving the minted tokens" },
    "constitutionalContext": { "type": "string", "description": "Governance layer under which this mint occurred" },
    "elohimAttestation": { "type": "string", "description": "Elohim attestation hash (Tier 2 discernment mints only)" },
    "reasoningTrace": { "type": "string", "description": "Explainability trace (Tier 2 discernment mints only)" }
  },
  "_source": {
    "file": "elohim/elohim-storage/src/db/models.rs",
    "type": "TokenMintEvent"
  }
}
```

Save to `elohim/sdk/schemas/v1/objects/token-mint-event.schema.json`.

- [ ] **Step 2: Create token-balance schema**

```json
{
  "$id": "epr:schema:object:token-balance",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "TokenBalance",
  "description": "Current elohim-token balance for an agent within a governance layer.",
  "type": "object",
  "required": ["agentId", "balance", "governanceLayer"],
  "properties": {
    "agentId": { "type": "string" },
    "balance": { "type": "number" },
    "governanceLayer": { "type": "string" },
    "totalMinted": { "type": "number", "description": "Lifetime minted for this agent" },
    "totalTransferredIn": { "type": "number" },
    "totalTransferredOut": { "type": "number" },
    "lastActivityAt": { "type": "string", "format": "date-time" }
  },
  "_source": {
    "file": "elohim/elohim-storage/src/db/models.rs",
    "type": "TokenBalance"
  }
}
```

Save to `elohim/sdk/schemas/v1/objects/token-balance.schema.json`.

- [ ] **Step 3: Create token-transfer schema**

```json
{
  "$id": "epr:schema:object:token-transfer",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "TokenTransfer",
  "description": "Witnessed transfer of elohim-tokens between agents.",
  "type": "object",
  "required": ["id", "fromAgent", "toAgent", "amount"],
  "properties": {
    "id": { "type": "string" },
    "fromAgent": { "type": "string" },
    "toAgent": { "type": "string" },
    "amount": { "type": "number" },
    "governanceLayer": { "type": "string" },
    "note": { "type": "string" }
  },
  "_source": {
    "file": "elohim/elohim-storage/src/db/models.rs",
    "type": "TokenTransfer"
  }
}
```

Save to `elohim/sdk/schemas/v1/objects/token-transfer.schema.json`.

- [ ] **Step 4: Create transfer input schema**

```json
{
  "$id": "epr:schema:input:create-token-transfer",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "CreateTokenTransferInput",
  "description": "Input for creating a token transfer between agents.",
  "type": "object",
  "required": ["fromAgent", "toAgent", "amount"],
  "properties": {
    "fromAgent": { "type": "string" },
    "toAgent": { "type": "string" },
    "amount": { "type": "number", "exclusiveMinimum": 0 },
    "governanceLayer": { "type": "string" },
    "note": { "type": "string" }
  }
}
```

Save to `elohim/sdk/schemas/v1/inputs/create-token-transfer-input.schema.json`.

- [ ] **Step 5: Verify schemas parse**

Run: `cd /projects/elohim && pnpm run schema:validate`
Expected: All existing validations pass. New schemas are structurally valid.

- [ ] **Step 6: Commit**

```bash
git add elohim/sdk/schemas/v1/objects/token-mint-event.schema.json \
        elohim/sdk/schemas/v1/objects/token-balance.schema.json \
        elohim/sdk/schemas/v1/objects/token-transfer.schema.json \
        elohim/sdk/schemas/v1/inputs/create-token-transfer-input.schema.json
git commit -m "feat(schema): add elohim-token protocol schemas (mint, balance, transfer)"
```

---

## Task 2: Database Migration

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-04-01-000000_token_tables/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-04-01-000000_token_tables/down.sql`

- [ ] **Step 1: Create migration directory**

```bash
mkdir -p elohim/elohim-storage/migrations/2026-04-01-000000_token_tables
```

- [ ] **Step 2: Write up.sql**

```sql
-- Token mint events: immutable record of every token minted
-- Category A (notarized) — every mint is coupled to a witnessed REA event
CREATE TABLE token_mint_events (
    id TEXT PRIMARY KEY NOT NULL,
    h_app_id TEXT NOT NULL DEFAULT 'shefa',
    amount REAL NOT NULL,
    provenance_event_id TEXT NOT NULL,
    mint_tier TEXT NOT NULL DEFAULT 'micro',
    source_epr_id TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    constitutional_context TEXT,
    elohim_attestation TEXT,
    reasoning_trace TEXT,
    dht_anchor_hash TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_token_mint_events_h_app_id ON token_mint_events(h_app_id);
CREATE INDEX idx_token_mint_events_agent_id ON token_mint_events(agent_id);
CREATE INDEX idx_token_mint_events_provenance ON token_mint_events(provenance_event_id);
CREATE INDEX idx_token_mint_events_source_epr ON token_mint_events(source_epr_id);
CREATE INDEX idx_token_mint_events_tier ON token_mint_events(mint_tier);
CREATE INDEX idx_token_mint_events_created ON token_mint_events(created_at);

-- Token balances: current holdings per agent per governance layer
-- Category B (agent-scoped)
CREATE TABLE token_balances (
    agent_id TEXT NOT NULL,
    h_app_id TEXT NOT NULL DEFAULT 'shefa',
    governance_layer TEXT NOT NULL DEFAULT 'individual',
    balance REAL NOT NULL DEFAULT 0.0,
    total_minted REAL NOT NULL DEFAULT 0.0,
    total_transferred_in REAL NOT NULL DEFAULT 0.0,
    total_transferred_out REAL NOT NULL DEFAULT 0.0,
    last_activity_at TEXT NOT NULL DEFAULT (datetime('now')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (agent_id, h_app_id, governance_layer)
);

CREATE INDEX idx_token_balances_h_app_id ON token_balances(h_app_id);
CREATE INDEX idx_token_balances_balance ON token_balances(balance);

-- Token transfers: witnessed exchanges between agents
-- Category A (notarized)
CREATE TABLE token_transfers (
    id TEXT PRIMARY KEY NOT NULL,
    h_app_id TEXT NOT NULL DEFAULT 'shefa',
    from_agent TEXT NOT NULL,
    to_agent TEXT NOT NULL,
    amount REAL NOT NULL,
    governance_layer TEXT NOT NULL DEFAULT 'individual',
    note TEXT,
    dht_anchor_hash TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_token_transfers_h_app_id ON token_transfers(h_app_id);
CREATE INDEX idx_token_transfers_from ON token_transfers(from_agent);
CREATE INDEX idx_token_transfers_to ON token_transfers(to_agent);
CREATE INDEX idx_token_transfers_created ON token_transfers(created_at);
```

Save to `elohim/elohim-storage/migrations/2026-04-01-000000_token_tables/up.sql`.

- [ ] **Step 3: Write down.sql**

```sql
DROP TABLE IF EXISTS token_transfers;
DROP TABLE IF EXISTS token_balances;
DROP TABLE IF EXISTS token_mint_events;
```

Save to `elohim/elohim-storage/migrations/2026-04-01-000000_token_tables/down.sql`.

- [ ] **Step 4: Run migration to verify**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS="" cargo build --release 2>&1 | head -20
```

Expected: Compiles. Diesel will pick up the migration on next DB init.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/migrations/2026-04-01-000000_token_tables/
git commit -m "feat(storage): add token_mint_events, token_balances, token_transfers tables"
```

---

## Task 3: Diesel Schema + Models

**Files:**
- Modify: `elohim/elohim-storage/src/db/diesel_schema.rs`
- Modify: `elohim/elohim-storage/src/db/models.rs`
- Modify: `elohim/elohim-storage/src/db/mod.rs`

- [ ] **Step 1: Add Diesel table macros to diesel_schema.rs**

Append to the file:

```rust
diesel::table! {
    token_mint_events (id) {
        id -> Text,
        h_app_id -> Text,
        amount -> Float,
        provenance_event_id -> Text,
        mint_tier -> Text,
        source_epr_id -> Text,
        agent_id -> Text,
        constitutional_context -> Nullable<Text>,
        elohim_attestation -> Nullable<Text>,
        reasoning_trace -> Nullable<Text>,
        dht_anchor_hash -> Nullable<Text>,
        created_at -> Text,
    }
}

diesel::table! {
    token_balances (agent_id, h_app_id, governance_layer) {
        agent_id -> Text,
        h_app_id -> Text,
        governance_layer -> Text,
        balance -> Float,
        total_minted -> Float,
        total_transferred_in -> Float,
        total_transferred_out -> Float,
        last_activity_at -> Text,
        created_at -> Text,
    }
}

diesel::table! {
    token_transfers (id) {
        id -> Text,
        h_app_id -> Text,
        from_agent -> Text,
        to_agent -> Text,
        amount -> Float,
        governance_layer -> Text,
        note -> Nullable<Text>,
        dht_anchor_hash -> Nullable<Text>,
        created_at -> Text,
    }
}
```

- [ ] **Step 2: Add Queryable + Insertable models to models.rs**

Append to the file:

```rust
// ── Token Mint Events ──

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = token_mint_events)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TokenMintEvent {
    pub id: String,
    pub h_app_id: String,
    pub amount: f32,
    pub provenance_event_id: String,
    pub mint_tier: String,
    pub source_epr_id: String,
    pub agent_id: String,
    pub constitutional_context: Option<String>,
    pub elohim_attestation: Option<String>,
    pub reasoning_trace: Option<String>,
    pub dht_anchor_hash: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = token_mint_events)]
pub struct NewTokenMintEvent<'a> {
    pub id: &'a str,
    pub h_app_id: &'a str,
    pub amount: f32,
    pub provenance_event_id: &'a str,
    pub mint_tier: &'a str,
    pub source_epr_id: &'a str,
    pub agent_id: &'a str,
    pub constitutional_context: Option<&'a str>,
    pub elohim_attestation: Option<&'a str>,
    pub reasoning_trace: Option<&'a str>,
}

// ── Token Balances ──

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = token_balances)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TokenBalance {
    pub agent_id: String,
    pub h_app_id: String,
    pub governance_layer: String,
    pub balance: f32,
    pub total_minted: f32,
    pub total_transferred_in: f32,
    pub total_transferred_out: f32,
    pub last_activity_at: String,
    pub created_at: String,
}

// ── Token Transfers ──

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = token_transfers)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TokenTransfer {
    pub id: String,
    pub h_app_id: String,
    pub from_agent: String,
    pub to_agent: String,
    pub amount: f32,
    pub governance_layer: String,
    pub note: Option<String>,
    pub dht_anchor_hash: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = token_transfers)]
pub struct NewTokenTransfer<'a> {
    pub id: &'a str,
    pub h_app_id: &'a str,
    pub from_agent: &'a str,
    pub to_agent: &'a str,
    pub amount: f32,
    pub governance_layer: &'a str,
    pub note: Option<&'a str>,
}
```

- [ ] **Step 3: Add db module declarations to mod.rs**

Add to `elohim/elohim-storage/src/db/mod.rs`:

```rust
pub mod token_mint_events;
pub mod token_balances;
pub mod token_transfers;
```

- [ ] **Step 4: Verify compilation**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS="" cargo check 2>&1 | tail -5
```

Expected: Compiles with no errors.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/db/diesel_schema.rs \
        elohim/elohim-storage/src/db/models.rs \
        elohim/elohim-storage/src/db/mod.rs
git commit -m "feat(storage): add Diesel schema and models for token tables"
```

---

## Task 4: DB CRUD Modules

**Files:**
- Create: `elohim/elohim-storage/src/db/token_mint_events.rs`
- Create: `elohim/elohim-storage/src/db/token_balances.rs`
- Create: `elohim/elohim-storage/src/db/token_transfers.rs`

- [ ] **Step 1: Write token_mint_events.rs**

```rust
use diesel::prelude::*;
use crate::db::context::AppContext;
use crate::db::diesel_schema::token_mint_events;
use crate::db::models::{TokenMintEvent, NewTokenMintEvent};
use crate::errors::StorageError;

pub fn create_mint_event(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    new: NewTokenMintEvent,
) -> Result<TokenMintEvent, StorageError> {
    diesel::insert_into(token_mint_events::table)
        .values(&new)
        .execute(conn)?;

    token_mint_events::table
        .filter(token_mint_events::id.eq(new.id))
        .filter(token_mint_events::h_app_id.eq(&ctx.h_app_id))
        .first::<TokenMintEvent>(conn)
        .map_err(StorageError::from)
}

pub fn get_mints_for_agent(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    agent_id: &str,
) -> Result<Vec<TokenMintEvent>, StorageError> {
    token_mint_events::table
        .filter(token_mint_events::h_app_id.eq(&ctx.h_app_id))
        .filter(token_mint_events::agent_id.eq(agent_id))
        .order(token_mint_events::created_at.desc())
        .load::<TokenMintEvent>(conn)
        .map_err(StorageError::from)
}

pub fn get_mints_for_event(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    provenance_event_id: &str,
) -> Result<Vec<TokenMintEvent>, StorageError> {
    token_mint_events::table
        .filter(token_mint_events::h_app_id.eq(&ctx.h_app_id))
        .filter(token_mint_events::provenance_event_id.eq(provenance_event_id))
        .load::<TokenMintEvent>(conn)
        .map_err(StorageError::from)
}
```

- [ ] **Step 2: Write token_balances.rs**

```rust
use diesel::prelude::*;
use crate::db::context::AppContext;
use crate::db::diesel_schema::token_balances;
use crate::db::models::TokenBalance;
use crate::errors::StorageError;

pub fn get_balance(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    agent_id: &str,
    governance_layer: &str,
) -> Result<Option<TokenBalance>, StorageError> {
    token_balances::table
        .filter(token_balances::h_app_id.eq(&ctx.h_app_id))
        .filter(token_balances::agent_id.eq(agent_id))
        .filter(token_balances::governance_layer.eq(governance_layer))
        .first::<TokenBalance>(conn)
        .optional()
        .map_err(StorageError::from)
}

pub fn get_all_balances_for_agent(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    agent_id: &str,
) -> Result<Vec<TokenBalance>, StorageError> {
    token_balances::table
        .filter(token_balances::h_app_id.eq(&ctx.h_app_id))
        .filter(token_balances::agent_id.eq(agent_id))
        .load::<TokenBalance>(conn)
        .map_err(StorageError::from)
}

/// Credit tokens to an agent's balance (upsert pattern).
/// Called after a mint event or incoming transfer.
pub fn credit_balance(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    agent_id: &str,
    governance_layer: &str,
    amount: f32,
    source: CreditSource,
) -> Result<TokenBalance, StorageError> {
    let now = chrono::Utc::now().to_rfc3339();
    let existing = get_balance(conn, ctx, agent_id, governance_layer)?;

    match existing {
        Some(bal) => {
            let (new_minted, new_in) = match source {
                CreditSource::Mint => (bal.total_minted + amount, bal.total_transferred_in),
                CreditSource::TransferIn => (bal.total_minted, bal.total_transferred_in + amount),
            };
            diesel::update(
                token_balances::table
                    .filter(token_balances::agent_id.eq(agent_id))
                    .filter(token_balances::h_app_id.eq(&ctx.h_app_id))
                    .filter(token_balances::governance_layer.eq(governance_layer)),
            )
            .set((
                token_balances::balance.eq(bal.balance + amount),
                token_balances::total_minted.eq(new_minted),
                token_balances::total_transferred_in.eq(new_in),
                token_balances::last_activity_at.eq(&now),
            ))
            .execute(conn)?;

            get_balance(conn, ctx, agent_id, governance_layer)?
                .ok_or_else(|| StorageError::NotFound("balance after update".into()))
        }
        None => {
            let (minted, transferred_in) = match source {
                CreditSource::Mint => (amount, 0.0),
                CreditSource::TransferIn => (0.0, amount),
            };
            diesel::insert_into(token_balances::table)
                .values((
                    token_balances::agent_id.eq(agent_id),
                    token_balances::h_app_id.eq(&ctx.h_app_id),
                    token_balances::governance_layer.eq(governance_layer),
                    token_balances::balance.eq(amount),
                    token_balances::total_minted.eq(minted),
                    token_balances::total_transferred_in.eq(transferred_in),
                    token_balances::total_transferred_out.eq(0.0_f32),
                    token_balances::last_activity_at.eq(&now),
                ))
                .execute(conn)?;

            get_balance(conn, ctx, agent_id, governance_layer)?
                .ok_or_else(|| StorageError::NotFound("balance after insert".into()))
        }
    }
}

/// Debit tokens from an agent's balance.
/// Called on outgoing transfer. Returns error if insufficient balance.
pub fn debit_balance(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    agent_id: &str,
    governance_layer: &str,
    amount: f32,
) -> Result<TokenBalance, StorageError> {
    let bal = get_balance(conn, ctx, agent_id, governance_layer)?
        .ok_or_else(|| StorageError::NotFound("no balance to debit".into()))?;

    if bal.balance < amount {
        return Err(StorageError::Validation(format!(
            "insufficient balance: have {}, need {}", bal.balance, amount
        )));
    }

    let now = chrono::Utc::now().to_rfc3339();
    diesel::update(
        token_balances::table
            .filter(token_balances::agent_id.eq(agent_id))
            .filter(token_balances::h_app_id.eq(&ctx.h_app_id))
            .filter(token_balances::governance_layer.eq(governance_layer)),
    )
    .set((
        token_balances::balance.eq(bal.balance - amount),
        token_balances::total_transferred_out.eq(bal.total_transferred_out + amount),
        token_balances::last_activity_at.eq(&now),
    ))
    .execute(conn)?;

    get_balance(conn, ctx, agent_id, governance_layer)?
        .ok_or_else(|| StorageError::NotFound("balance after debit".into()))
}

pub enum CreditSource {
    Mint,
    TransferIn,
}
```

- [ ] **Step 3: Write token_transfers.rs**

```rust
use diesel::prelude::*;
use crate::db::context::AppContext;
use crate::db::diesel_schema::token_transfers;
use crate::db::models::{TokenTransfer, NewTokenTransfer};
use crate::errors::StorageError;

pub fn create_transfer(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    new: NewTokenTransfer,
) -> Result<TokenTransfer, StorageError> {
    diesel::insert_into(token_transfers::table)
        .values(&new)
        .execute(conn)?;

    token_transfers::table
        .filter(token_transfers::id.eq(new.id))
        .filter(token_transfers::h_app_id.eq(&ctx.h_app_id))
        .first::<TokenTransfer>(conn)
        .map_err(StorageError::from)
}

pub fn get_transfers_for_agent(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    agent_id: &str,
) -> Result<Vec<TokenTransfer>, StorageError> {
    token_transfers::table
        .filter(token_transfers::h_app_id.eq(&ctx.h_app_id))
        .filter(
            token_transfers::from_agent.eq(agent_id)
                .or(token_transfers::to_agent.eq(agent_id))
        )
        .order(token_transfers::created_at.desc())
        .load::<TokenTransfer>(conn)
        .map_err(StorageError::from)
}
```

- [ ] **Step 4: Verify compilation**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS="" cargo check 2>&1 | tail -5
```

Expected: Compiles.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/db/token_mint_events.rs \
        elohim/elohim-storage/src/db/token_balances.rs \
        elohim/elohim-storage/src/db/token_transfers.rs
git commit -m "feat(storage): add CRUD modules for token mint events, balances, transfers"
```

---

## Task 5: View Types (API Boundary)

**Files:**
- Modify: `elohim/elohim-storage/src/views.rs`

- [ ] **Step 1: Add TokenMintEventView**

Append to `views.rs`:

```rust
// ── Token Views ──

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct TokenMintEventView {
    pub id: String,
    pub amount: f32,
    pub provenance_event_id: String,
    pub mint_tier: String,
    pub source_epr_id: String,
    pub agent_id: String,
    pub constitutional_context: Option<String>,
    pub elohim_attestation: Option<String>,
    pub reasoning_trace: Option<String>,
    pub created_at: String,
}

impl From<crate::db::models::TokenMintEvent> for TokenMintEventView {
    fn from(m: crate::db::models::TokenMintEvent) -> Self {
        Self {
            id: m.id,
            amount: m.amount,
            provenance_event_id: m.provenance_event_id,
            mint_tier: m.mint_tier,
            source_epr_id: m.source_epr_id,
            agent_id: m.agent_id,
            constitutional_context: m.constitutional_context,
            elohim_attestation: m.elohim_attestation,
            reasoning_trace: m.reasoning_trace,
            created_at: m.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct TokenBalanceView {
    pub agent_id: String,
    pub governance_layer: String,
    pub balance: f32,
    pub total_minted: f32,
    pub total_transferred_in: f32,
    pub total_transferred_out: f32,
    pub last_activity_at: String,
}

impl From<crate::db::models::TokenBalance> for TokenBalanceView {
    fn from(b: crate::db::models::TokenBalance) -> Self {
        Self {
            agent_id: b.agent_id,
            governance_layer: b.governance_layer,
            balance: b.balance,
            total_minted: b.total_minted,
            total_transferred_in: b.total_transferred_in,
            total_transferred_out: b.total_transferred_out,
            last_activity_at: b.last_activity_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct TokenTransferView {
    pub id: String,
    pub from_agent: String,
    pub to_agent: String,
    pub amount: f32,
    pub governance_layer: String,
    pub note: Option<String>,
    pub created_at: String,
}

impl From<crate::db::models::TokenTransfer> for TokenTransferView {
    fn from(t: crate::db::models::TokenTransfer) -> Self {
        Self {
            id: t.id,
            from_agent: t.from_agent,
            to_agent: t.to_agent,
            amount: t.amount,
            governance_layer: t.governance_layer,
            note: t.note,
            created_at: t.created_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateTokenTransferInputView {
    pub from_agent: String,
    pub to_agent: String,
    pub amount: f32,
    pub governance_layer: Option<String>,
    pub note: Option<String>,
}
```

- [ ] **Step 2: Verify compilation and generate TypeScript types**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS="" cargo check 2>&1 | tail -5
RUSTFLAGS="" cargo test export_bindings 2>&1 | tail -10
```

Expected: Compiles. TypeScript types generated to `sdk/storage-client-ts/src/generated/`.

- [ ] **Step 3: Commit**

```bash
git add elohim/elohim-storage/src/views.rs \
        elohim/sdk/storage-client-ts/src/generated/TokenMintEventView.ts \
        elohim/sdk/storage-client-ts/src/generated/TokenBalanceView.ts \
        elohim/sdk/storage-client-ts/src/generated/TokenTransferView.ts \
        elohim/sdk/storage-client-ts/src/generated/CreateTokenTransferInputView.ts
git commit -m "feat(storage): add token View types with TypeScript generation"
```

---

## Task 6: Token Mint Service

**Files:**
- Create: `elohim/elohim-storage/src/services/token_mint_service.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`

- [ ] **Step 1: Write token_mint_service.rs**

```rust
use diesel::SqliteConnection;
use sha2::{Digest, Sha256};

use crate::db::context::AppContext;
use crate::db::models::NewTokenMintEvent;
use crate::db::{token_mint_events, token_balances};
use crate::db::token_balances::CreditSource;
use crate::errors::StorageError;
use crate::views::TokenMintEventView;

/// Default mint rate — tokens minted per unit of weighted recognition.
/// Qahal-governed; this is the initial value.
const DEFAULT_MINT_RATE: f32 = 1.0;

/// Recognition policy weights (mirrored from recognition_pipeline_service.rs).
/// These map event types to relative mint amounts.
const WEIGHT_MASTERY_COMPLETION: f32 = 1.0;
const WEIGHT_CONTENT_CITATION: f32 = 0.5;
const WEIGHT_ASSESSMENT_ATTEMPT: f32 = 0.1;
const WEIGHT_CONTENT_ACCESS: f32 = 0.01;

pub struct TokenMintService;

impl TokenMintService {
    /// Calculate the micro-mint amount for a recognition event.
    /// amount = event_weight * allocation_ratio * mint_rate
    pub fn calculate_micro_mint(
        event_type: &str,
        allocation_ratio: f32,
        mint_rate: Option<f32>,
    ) -> f32 {
        let rate = mint_rate.unwrap_or(DEFAULT_MINT_RATE);
        let weight = Self::event_weight(event_type);
        weight * allocation_ratio * rate
    }

    /// Map event types to recognition weights.
    fn event_weight(event_type: &str) -> f32 {
        match event_type {
            "mastery-advance" | "mastery-completion" | "path-completion" => WEIGHT_MASTERY_COMPLETION,
            "citation" | "content-citation" => WEIGHT_CONTENT_CITATION,
            "assessment-attempt" | "quiz-attempt" => WEIGHT_ASSESSMENT_ATTEMPT,
            "content-view" | "content-access" => WEIGHT_CONTENT_ACCESS,
            _ => WEIGHT_CONTENT_ACCESS,
        }
    }

    /// Create a micro-mint event and credit the agent's balance.
    /// Called from the recognition pipeline settle stage.
    pub fn mint_for_recognition(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        agent_id: &str,
        provenance_event_id: &str,
        source_epr_id: &str,
        event_type: &str,
        allocation_ratio: f32,
        governance_layer: &str,
    ) -> Result<TokenMintEventView, StorageError> {
        let amount = Self::calculate_micro_mint(event_type, allocation_ratio, None);

        if amount <= 0.0 {
            return Err(StorageError::Validation("mint amount must be positive".into()));
        }

        let mint_id = Self::generate_mint_id(provenance_event_id, agent_id);

        let new_mint = NewTokenMintEvent {
            id: &mint_id,
            h_app_id: &ctx.h_app_id,
            amount,
            provenance_event_id,
            mint_tier: "micro",
            source_epr_id,
            agent_id,
            constitutional_context: Some(governance_layer),
            elohim_attestation: None,
            reasoning_trace: None,
        };

        let mint = token_mint_events::create_mint_event(conn, ctx, new_mint)?;

        // Credit the agent's balance
        token_balances::credit_balance(
            conn, ctx, agent_id, governance_layer, amount, CreditSource::Mint,
        )?;

        Ok(TokenMintEventView::from(mint))
    }

    fn generate_mint_id(provenance_event_id: &str, agent_id: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(provenance_event_id.as_bytes());
        hasher.update(agent_id.as_bytes());
        let hash = hasher.finalize();
        format!("mint-{}", hex::encode(&hash[..8]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_micro_mint_calculation() {
        // mastery completion at full allocation
        let amount = TokenMintService::calculate_micro_mint("mastery-advance", 1.0, Some(1.0));
        assert!((amount - 1.0).abs() < f32::EPSILON);

        // content view at 50% allocation
        let amount = TokenMintService::calculate_micro_mint("content-view", 0.5, Some(1.0));
        assert!((amount - 0.005).abs() < f32::EPSILON);

        // citation at 30% allocation with 2x mint rate
        let amount = TokenMintService::calculate_micro_mint("citation", 0.3, Some(2.0));
        assert!((amount - 0.3).abs() < f32::EPSILON);
    }

    #[test]
    fn test_event_weight_defaults() {
        assert!((TokenMintService::event_weight("unknown-type") - 0.01).abs() < f32::EPSILON);
    }

    #[test]
    fn test_mint_id_deterministic() {
        let id1 = TokenMintService::generate_mint_id("event-123", "agent-abc");
        let id2 = TokenMintService::generate_mint_id("event-123", "agent-abc");
        assert_eq!(id1, id2);

        let id3 = TokenMintService::generate_mint_id("event-456", "agent-abc");
        assert_ne!(id1, id3);
    }
}
```

- [ ] **Step 2: Add module declaration to services/mod.rs**

Add to `elohim/elohim-storage/src/services/mod.rs`:

```rust
pub mod token_mint_service;
pub mod token_ledger_service;
```

- [ ] **Step 3: Run unit tests**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS="" cargo test token_mint_service -- --nocapture 2>&1 | tail -15
```

Expected: 3 tests pass.

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/services/token_mint_service.rs \
        elohim/elohim-storage/src/services/mod.rs
git commit -m "feat(storage): add TokenMintService with micro-mint calculation"
```

---

## Task 7: Token Ledger Service

**Files:**
- Create: `elohim/elohim-storage/src/services/token_ledger_service.rs`

- [ ] **Step 1: Write token_ledger_service.rs**

```rust
use diesel::SqliteConnection;
use sha2::{Digest, Sha256};

use crate::db::context::AppContext;
use crate::db::models::NewTokenTransfer;
use crate::db::{token_balances, token_transfers};
use crate::db::token_balances::CreditSource;
use crate::errors::StorageError;
use crate::views::{TokenBalanceView, TokenTransferView};

pub struct TokenLedgerService;

impl TokenLedgerService {
    /// Get an agent's balance for a governance layer.
    pub fn get_balance(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        agent_id: &str,
        governance_layer: &str,
    ) -> Result<TokenBalanceView, StorageError> {
        match token_balances::get_balance(conn, ctx, agent_id, governance_layer)? {
            Some(bal) => Ok(TokenBalanceView::from(bal)),
            None => Ok(TokenBalanceView {
                agent_id: agent_id.to_string(),
                governance_layer: governance_layer.to_string(),
                balance: 0.0,
                total_minted: 0.0,
                total_transferred_in: 0.0,
                total_transferred_out: 0.0,
                last_activity_at: chrono::Utc::now().to_rfc3339(),
            }),
        }
    }

    /// Get all balances for an agent across governance layers.
    pub fn get_all_balances(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        agent_id: &str,
    ) -> Result<Vec<TokenBalanceView>, StorageError> {
        let balances = token_balances::get_all_balances_for_agent(conn, ctx, agent_id)?;
        Ok(balances.into_iter().map(TokenBalanceView::from).collect())
    }

    /// Transfer tokens between agents.
    /// Sprint 1: basic transfer with insufficient-balance check.
    /// Sprint 2 will add ResponsibilityDemandParam curve enforcement.
    pub fn transfer(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        from_agent: &str,
        to_agent: &str,
        amount: f32,
        governance_layer: &str,
        note: Option<&str>,
    ) -> Result<TokenTransferView, StorageError> {
        if amount <= 0.0 {
            return Err(StorageError::Validation("transfer amount must be positive".into()));
        }
        if from_agent == to_agent {
            return Err(StorageError::Validation("cannot transfer to self".into()));
        }

        // Debit sender
        token_balances::debit_balance(conn, ctx, from_agent, governance_layer, amount)?;

        // Credit receiver
        token_balances::credit_balance(
            conn, ctx, to_agent, governance_layer, amount, CreditSource::TransferIn,
        )?;

        // Record transfer
        let transfer_id = format!(
            "txfr-{}-{}",
            chrono::Utc::now().timestamp_millis(),
            &sha2::Sha256::digest(format!("{}{}{}", from_agent, to_agent, amount).as_bytes())
                .iter()
                .take(4)
                .map(|b| format!("{:02x}", b))
                .collect::<String>()
        );

        let new_transfer = NewTokenTransfer {
            id: &transfer_id,
            h_app_id: &ctx.h_app_id,
            from_agent,
            to_agent,
            amount,
            governance_layer,
            note,
        };

        let transfer = token_transfers::create_transfer(conn, ctx, new_transfer)?;
        Ok(TokenTransferView::from(transfer))
    }

    /// Get transfer history for an agent.
    pub fn get_transfers(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        agent_id: &str,
    ) -> Result<Vec<TokenTransferView>, StorageError> {
        let transfers = token_transfers::get_transfers_for_agent(conn, ctx, agent_id)?;
        Ok(transfers.into_iter().map(TokenTransferView::from).collect())
    }
}
```

- [ ] **Step 2: Verify compilation**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS="" cargo check 2>&1 | tail -5
```

Expected: Compiles.

- [ ] **Step 3: Commit**

```bash
git add elohim/elohim-storage/src/services/token_ledger_service.rs
git commit -m "feat(storage): add TokenLedgerService with balance tracking and transfers"
```

---

## Task 8: Inject Mint Step into Recognition Pipeline

**Files:**
- Modify: `elohim/elohim-storage/src/services/recognition_pipeline_service.rs`

This is the key integration — connecting the existing recognition pipeline to the new minting infrastructure.

- [ ] **Step 1: Add import for TokenMintService**

At the top of `recognition_pipeline_service.rs`, add:

```rust
use crate::services::token_mint_service::TokenMintService;
```

- [ ] **Step 2: Inject mint step in settle() function**

In the `settle()` function, after the line that calls `record_event` (approximately line 464) and before the line that calls `accumulate_recognition` (approximately line 466), inject the mint step:

```rust
        // ── Mint elohim-token for this recognition share ──
        let content_id_for_mint = trigger.content_id.clone().unwrap_or_default();
        let event_type = trigger.event_type.as_deref().unwrap_or("content-access");
        if let Err(e) = TokenMintService::mint_for_recognition(
            conn,
            ctx,
            &share.steward_id,
            &event_id,
            &content_id_for_mint,
            event_type,
            share.allocation_ratio,
            "individual", // governance layer — Sprint 2 will make this context-aware
        ) {
            // Log mint failure but don't fail the recognition pipeline
            eprintln!("[token-mint] failed to mint for {}: {}", share.steward_id, e);
        }
```

This is inserted inside the existing `for share in shares` loop, after the economic event is recorded but before recognition is accumulated.

- [ ] **Step 3: Verify the pipeline still compiles**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS="" cargo check 2>&1 | tail -5
```

Expected: Compiles with no errors.

- [ ] **Step 4: Run existing recognition pipeline tests**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS="" cargo test recognition_pipeline -- --nocapture 2>&1 | tail -20
```

Expected: Existing tests still pass. Mint step may log errors in test context (no DB tables yet in test fixtures) but should not break the pipeline.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/services/recognition_pipeline_service.rs
git commit -m "feat(token): inject micro-mint step into recognition pipeline settle stage"
```

---

## Task 9: Token API Routes

**Files:**
- Create: `elohim/elohim-storage/src/api/token.rs`
- Modify: `elohim/elohim-storage/src/api/mod.rs`

- [ ] **Step 1: Write token.rs API handler**

```rust
use bytes::Bytes;
use http::{Method, Response, StatusCode};
use http_body_util::Full;
use hyper::Request;
use hyper::body::Incoming;

use crate::api::{get_conn, json_response, parse_body, from_create_result};
use crate::db::context::AppContext;
use crate::db::token_mint_events;
use crate::services::token_ledger_service::TokenLedgerService;
use crate::services::token_mint_service::TokenMintService;
use crate::views::{TokenMintEventView, TokenBalanceView, TokenTransferView, CreateTokenTransferInputView};
use crate::errors::StorageError;

type DbPool = deadpool_diesel::sqlite::Pool;

pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    match (method, resource_path) {
        // GET /api/v1/token/balance/{agent_id}
        (Method::GET, p) if p.starts_with("balance/") => {
            let agent_id = &p["balance/".len()..];
            let mut conn = get_conn(pool)?;
            let balances = TokenLedgerService::get_all_balances(&mut conn, ctx, agent_id)?;
            json_response(&balances)
        }

        // GET /api/v1/token/mints/{agent_id}
        (Method::GET, p) if p.starts_with("mints/") => {
            let agent_id = &p["mints/".len()..];
            let mut conn = get_conn(pool)?;
            let mints = token_mint_events::get_mints_for_agent(&mut conn, ctx, agent_id)?;
            let views: Vec<TokenMintEventView> = mints.into_iter().map(TokenMintEventView::from).collect();
            json_response(&views)
        }

        // GET /api/v1/token/transfers/{agent_id}
        (Method::GET, p) if p.starts_with("transfers/") => {
            let agent_id = &p["transfers/".len()..];
            let mut conn = get_conn(pool)?;
            let transfers = TokenLedgerService::get_transfers(&mut conn, ctx, agent_id)?;
            json_response(&transfers)
        }

        // POST /api/v1/token/transfer
        (Method::POST, "transfer") => {
            let input: CreateTokenTransferInputView = parse_body(req).await?;
            let mut conn = get_conn(pool)?;
            let governance_layer = input.governance_layer.as_deref().unwrap_or("individual");
            let transfer = TokenLedgerService::transfer(
                &mut conn,
                ctx,
                &input.from_agent,
                &input.to_agent,
                input.amount,
                governance_layer,
                input.note.as_deref(),
            )?;
            from_create_result(Ok(transfer))
        }

        _ => {
            let mut resp = Response::new(Full::new(Bytes::from("{\"error\":\"not found\"}")));
            *resp.status_mut() = StatusCode::NOT_FOUND;
            Ok(resp)
        }
    }
}
```

- [ ] **Step 2: Register token routes in api/mod.rs**

Add the route dispatcher in `handle_api_request()`. Find the section with `if sub_path.starts_with("economic-events")` and add after it:

```rust
        } else if sub_path.starts_with("token") {
            let resource_path = sub_path.strip_prefix("token").unwrap_or("").trim_start_matches('/');
            token::handle(req, method, resource_path, pool, ctx).await
```

Also add the module declaration at the top of `api/mod.rs`:

```rust
pub mod token;
```

- [ ] **Step 3: Verify compilation**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS="" cargo check 2>&1 | tail -5
```

Expected: Compiles.

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/api/token.rs \
        elohim/elohim-storage/src/api/mod.rs
git commit -m "feat(api): add token routes for balance, mints, transfers"
```

---

## Task 10: Integration Verification

- [ ] **Step 1: Full build**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS="" cargo build --release 2>&1 | tail -10
```

Expected: Clean build.

- [ ] **Step 2: Run all unit tests**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS="" cargo test --lib --bins 2>&1 | tail -20
```

Expected: All tests pass including new token_mint_service tests.

- [ ] **Step 3: Generate TypeScript types**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS="" cargo test export_bindings 2>&1 | tail -10
```

Expected: TokenMintEventView.ts, TokenBalanceView.ts, TokenTransferView.ts, CreateTokenTransferInputView.ts generated.

- [ ] **Step 4: Verify schema validation**

```bash
cd /projects/elohim
pnpm run schema:validate 2>&1 | tail -10
```

Expected: All validations pass.

- [ ] **Step 5: Verify no "amplification" references in token context**

```bash
grep -rn "amplification" elohim/elohim-token/ elohim/elohim-storage/src/services/token_*
```

Expected: Zero matches.

- [ ] **Step 6: Commit any generated files**

```bash
git add elohim/sdk/storage-client-ts/src/generated/Token*.ts \
        elohim/sdk/storage-client-ts/src/generated/CreateTokenTransferInputView.ts
git commit -m "chore: add generated TypeScript types for token views"
```

---

## Sprint 1 Deliverables

After completing all tasks, Sprint 1 delivers:

1. **Protocol schemas** for token mint events, balances, and transfers
2. **Database tables** for persistent token storage
3. **Mint service** that calculates deterministic micro-mint amounts from recognition policy weights
4. **Ledger service** that tracks balances and validates transfers
5. **Pipeline integration** — the recognition pipeline now mints elohim-tokens on every content delivery event
6. **API routes** for querying balances, mint history, and creating transfers
7. **TypeScript types** generated for frontend consumption

## What Sprint 2 Adds

- ResponsibilityDemandParam curve enforcement on transfers and accumulation
- Context-aware curves per governance layer (social contract health sensing)
- Curve configuration API (qahal-governed parameter management)

## What Sprint 3 Adds

- Elohim discernment minting (periodic pattern evaluation)
- Story-memory decay (provenance vitality scanning)

## What Sprint 4 Adds

- Settlement bridge interface (chain-agnostic trait)
- Provenance hash generation for bridge crossing
