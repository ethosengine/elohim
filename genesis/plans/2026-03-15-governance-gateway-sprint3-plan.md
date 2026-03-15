# Sprint 3: Governance Gateway Foundation — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add ranked-choice, approval, score, dot, consent, and conviction voting infrastructure to elohim-storage — the multi-mechanism backend that Layer A of the Governance Feedback Mechanism Gateway requires.

**Architecture:** Three new tables (`proposal_options`, `ranked_votes`, `governance_signals`), a `TallyStrategy` trait with 6 implementations, new Rust View types with TypeScript generation, and HTTP routes for multi-mechanism voting. Extends the existing governance module — same patterns as Sprint 2's votes table.

**Tech Stack:** Rust (Diesel ORM, SQLite), ts-rs for TypeScript generation, hyper HTTP handlers

**Design doc:** `genesis/plans/2026-03-15-governance-feedback-mechanism-gateway-design.md`

---

### Task 1: Migration — proposal_options table

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-03-15-000003_add_proposal_options/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-03-15-000003_add_proposal_options/down.sql`

**Step 1: Create migration directory and up.sql**

```sql
CREATE TABLE IF NOT EXISTS proposal_options (
    id TEXT PRIMARY KEY NOT NULL,
    proposal_id TEXT NOT NULL,
    label TEXT NOT NULL,
    description TEXT NOT NULL,
    position INTEGER NOT NULL,
    source TEXT,
    source_justification TEXT,
    created_at TEXT NOT NULL
);

ALTER TABLE proposals ADD COLUMN voting_mechanism TEXT NOT NULL DEFAULT 'consent';
ALTER TABLE proposals ADD COLUMN score_min INTEGER;
ALTER TABLE proposals ADD COLUMN score_max INTEGER;
ALTER TABLE proposals ADD COLUMN dots_per_voter INTEGER;
ALTER TABLE proposals ADD COLUMN quorum_percentage REAL;
ALTER TABLE proposals ADD COLUMN passage_threshold REAL;
```

**Step 2: Create down.sql**

```sql
DROP TABLE IF EXISTS proposal_options;
-- SQLite < 3.35.0 doesn't support DROP COLUMN; best-effort
```

**Step 3: Run migration**

Run: `cd elohim/elohim-storage && diesel migration run`
Expected: Migration applied successfully. `diesel_schema.rs` updated with `proposal_options` table and new `proposals` columns.

**Step 4: Verify diesel_schema.rs was updated**

Check that `proposal_options` table block and new `proposals` columns appear in `src/db/diesel_schema.rs`. Add `proposal_options` to the `allow_tables_to_appear_in_same_query!` macro.

**Step 5: Commit**

```bash
git add elohim/elohim-storage/migrations/2026-03-15-000003_add_proposal_options/
git add elohim/elohim-storage/src/db/diesel_schema.rs
git commit -m "feat(storage): add proposal_options table and voting_mechanism column"
```

---

### Task 2: Migration — ranked_votes and governance_signals tables

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-03-15-000004_add_ranked_votes_and_signals/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-03-15-000004_add_ranked_votes_and_signals/down.sql`

**Step 1: Create up.sql**

```sql
CREATE TABLE IF NOT EXISTS ranked_votes (
    id TEXT PRIMARY KEY NOT NULL,
    proposal_id TEXT NOT NULL,
    human_id TEXT NOT NULL,
    option_id TEXT NOT NULL,
    rank INTEGER,
    score INTEGER,
    dots INTEGER,
    approved INTEGER,
    reasoning TEXT,
    proxy_elohim_id TEXT,
    proxy_justification TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(proposal_id, human_id, option_id)
);

CREATE TABLE IF NOT EXISTS governance_signals (
    id TEXT PRIMARY KEY NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    human_id TEXT NOT NULL,
    signal_type TEXT NOT NULL,
    signal_value TEXT NOT NULL,
    mechanism_level INTEGER NOT NULL,
    proxy_elohim_id TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_governance_signals_entity
    ON governance_signals(entity_type, entity_id);

CREATE INDEX IF NOT EXISTS idx_ranked_votes_proposal
    ON ranked_votes(proposal_id);
```

**Step 2: Create down.sql**

```sql
DROP TABLE IF EXISTS ranked_votes;
DROP TABLE IF EXISTS governance_signals;
```

**Step 3: Run migration and verify schema**

Run: `cd elohim/elohim-storage && diesel migration run`
Expected: Both tables appear in `diesel_schema.rs`. Add both to `allow_tables_to_appear_in_same_query!`.

**Step 4: Commit**

```bash
git add elohim/elohim-storage/migrations/2026-03-15-000004_add_ranked_votes_and_signals/
git add elohim/elohim-storage/src/db/diesel_schema.rs
git commit -m "feat(storage): add ranked_votes and governance_signals tables"
```

---

### Task 3: Diesel models — ProposalOption, RankedVote, GovernanceSignal

**Files:**
- Modify: `elohim/elohim-storage/src/db/models.rs`

**Step 1: Add ProposalOption model structs**

Add after the existing `Vote`/`NewVote` structs (around line 1570):

```rust
/// Option within a multi-mechanism proposal
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = proposal_options)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ProposalOption {
    pub id: String,
    pub proposal_id: String,
    pub label: String,
    pub description: String,
    pub position: i32,
    pub source: Option<String>,
    pub source_justification: Option<String>,
    pub created_at: String,
}

/// New proposal option for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = proposal_options)]
pub struct NewProposalOption<'a> {
    pub id: &'a str,
    pub proposal_id: &'a str,
    pub label: &'a str,
    pub description: &'a str,
    pub position: i32,
    pub source: Option<&'a str>,
    pub source_justification: Option<&'a str>,
    pub created_at: &'a str,
}
```

**Step 2: Add RankedVote model structs**

```rust
/// Vote in a multi-mechanism proposal (ranked-choice, score, dot, approval)
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = ranked_votes)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct RankedVote {
    pub id: String,
    pub proposal_id: String,
    pub human_id: String,
    pub option_id: String,
    pub rank: Option<i32>,
    pub score: Option<i32>,
    pub dots: Option<i32>,
    pub approved: Option<i32>,
    pub reasoning: Option<String>,
    pub proxy_elohim_id: Option<String>,
    pub proxy_justification: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// New ranked vote for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = ranked_votes)]
pub struct NewRankedVote<'a> {
    pub id: &'a str,
    pub proposal_id: &'a str,
    pub human_id: &'a str,
    pub option_id: &'a str,
    pub rank: Option<i32>,
    pub score: Option<i32>,
    pub dots: Option<i32>,
    pub approved: Option<i32>,
    pub reasoning: Option<&'a str>,
    pub proxy_elohim_id: Option<&'a str>,
    pub proxy_justification: Option<&'a str>,
    pub created_at: &'a str,
    pub updated_at: &'a str,
}
```

**Step 3: Add GovernanceSignal model structs**

```rust
/// Normalized governance signal from any feedback mechanism
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = governance_signals)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct GovernanceSignal {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub human_id: String,
    pub signal_type: String,
    pub signal_value: String,
    pub mechanism_level: i32,
    pub proxy_elohim_id: Option<String>,
    pub created_at: String,
}

/// New governance signal for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = governance_signals)]
pub struct NewGovernanceSignal<'a> {
    pub id: &'a str,
    pub entity_type: &'a str,
    pub entity_id: &'a str,
    pub human_id: &'a str,
    pub signal_type: &'a str,
    pub signal_value: &'a str,
    pub mechanism_level: i32,
    pub proxy_elohim_id: Option<&'a str>,
    pub created_at: &'a str,
}
```

**Step 4: Add table imports to the diesel_schema use block**

In models.rs, find the `use super::diesel_schema::*` or individual table imports and add `proposal_options`, `ranked_votes`, `governance_signals`.

**Step 5: Verify compilation**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -5`
Expected: Compiles with no errors.

**Step 6: Commit**

```bash
git add elohim/elohim-storage/src/db/models.rs
git commit -m "feat(storage): add ProposalOption, RankedVote, GovernanceSignal models"
```

---

### Task 4: Update Proposal model for new columns

**Files:**
- Modify: `elohim/elohim-storage/src/db/models.rs` (Proposal and NewProposal structs)
- Modify: `elohim/elohim-storage/src/views.rs` (ProposalView and CreateProposalInputView)

**Step 1: Add new fields to Proposal struct**

Find the existing `Proposal` struct and add after `voting_anonymous`:

```rust
    pub voting_mechanism: String,
    pub score_min: Option<i32>,
    pub score_max: Option<i32>,
    pub dots_per_voter: Option<i32>,
    pub quorum_percentage: Option<f64>,
    pub passage_threshold: Option<f64>,
```

**Step 2: Add new fields to NewProposal struct**

Add corresponding fields with lifetime references where applicable:

```rust
    pub voting_mechanism: &'a str,
    pub score_min: Option<i32>,
    pub score_max: Option<i32>,
    pub dots_per_voter: Option<i32>,
    pub quorum_percentage: Option<f64>,
    pub passage_threshold: Option<f64>,
```

**Step 3: Update ProposalView**

Add to `ProposalView` struct:

```rust
    pub voting_mechanism: String,
    pub score_min: Option<i32>,
    pub score_max: Option<i32>,
    pub dots_per_voter: Option<i32>,
    pub quorum_percentage: Option<f64>,
    pub passage_threshold: Option<f64>,
```

Update the `From<Proposal>` impl to pass through the new fields.

**Step 4: Update CreateProposalInputView**

Add:

```rust
    #[serde(default = "default_voting_mechanism")]
    pub voting_mechanism: String,
    pub score_min: Option<i32>,
    pub score_max: Option<i32>,
    pub dots_per_voter: Option<i32>,
    pub quorum_percentage: Option<f64>,
    pub passage_threshold: Option<f64>,
```

Add helper:

```rust
fn default_voting_mechanism() -> String {
    "consent".to_string()
}
```

**Step 5: Update create_proposal handler in api/governance.rs**

Update the `NewProposal` construction in the POST `/proposals` handler to pass through the new fields from `CreateProposalInputView`.

**Step 6: Verify compilation**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -5`
Expected: Compiles with no errors.

**Step 7: Commit**

```bash
git add elohim/elohim-storage/src/db/models.rs elohim/elohim-storage/src/views.rs elohim/elohim-storage/src/api/governance.rs
git commit -m "feat(storage): add voting_mechanism and config fields to proposals"
```

---

### Task 5: View types — ProposalOptionView, RankedVoteView, GovernanceSignalView

**Files:**
- Modify: `elohim/elohim-storage/src/views.rs`

**Step 1: Add ProposalOptionView**

```rust
/// Proposal option — API response
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ProposalOptionView {
    pub id: String,
    pub proposal_id: String,
    pub label: String,
    pub description: String,
    pub position: i32,
    pub source: Option<String>,
    pub source_justification: Option<String>,
    pub created_at: String,
}

impl From<ProposalOption> for ProposalOptionView {
    fn from(o: ProposalOption) -> Self {
        Self {
            id: o.id,
            proposal_id: o.proposal_id,
            label: o.label,
            description: o.description,
            position: o.position,
            source: o.source,
            source_justification: o.source_justification,
            created_at: o.created_at,
        }
    }
}
```

**Step 2: Add RankedVoteView**

```rust
/// Ranked/scored/dot vote — API response
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RankedVoteView {
    pub id: String,
    pub proposal_id: String,
    pub human_id: Option<String>,
    pub option_id: String,
    pub rank: Option<i32>,
    pub score: Option<i32>,
    pub dots: Option<i32>,
    pub approved: Option<bool>,
    pub reasoning: Option<String>,
    pub proxy_elohim_id: Option<String>,
    pub created_at: String,
}

impl RankedVoteView {
    pub fn from_ranked_vote(v: RankedVote, hide_identity: bool) -> Self {
        Self {
            id: v.id,
            proposal_id: v.proposal_id,
            human_id: if hide_identity { None } else { Some(v.human_id) },
            option_id: v.option_id,
            rank: v.rank,
            score: v.score,
            dots: v.dots,
            approved: v.approved.map(|a| a == 1),
            reasoning: v.reasoning,
            proxy_elohim_id: v.proxy_elohim_id,
            created_at: v.created_at,
        }
    }
}
```

**Step 3: Add GovernanceSignalView**

```rust
/// Governance signal — API response
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct GovernanceSignalView {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub human_id: String,
    pub signal_type: String,
    pub signal_value: String,
    pub mechanism_level: i32,
    pub proxy_elohim_id: Option<String>,
    pub created_at: String,
}

impl From<GovernanceSignal> for GovernanceSignalView {
    fn from(s: GovernanceSignal) -> Self {
        Self {
            id: s.id,
            entity_type: s.entity_type,
            entity_id: s.entity_id,
            human_id: s.human_id,
            signal_type: s.signal_type,
            signal_value: s.signal_value,
            mechanism_level: s.mechanism_level,
            proxy_elohim_id: s.proxy_elohim_id,
            created_at: s.created_at,
        }
    }
}
```

**Step 4: Add input views**

```rust
/// Create proposal options — API request
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateProposalOptionInputView {
    pub id: String,
    pub label: String,
    pub description: String,
    pub position: i32,
    pub source: Option<String>,
    pub source_justification: Option<String>,
}

/// Cast a ranked/scored/dot/approval vote — API request
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CastRankedVoteInputView {
    pub human_id: String,
    pub ballots: Vec<BallotEntry>,
    #[serde(default)]
    pub reasoning: Option<String>,
    #[serde(default)]
    pub proxy_elohim_id: Option<String>,
    #[serde(default)]
    pub proxy_justification: Option<String>,
}

/// Single entry in a ranked/scored/dot ballot
#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct BallotEntry {
    pub option_id: String,
    pub rank: Option<i32>,
    pub score: Option<i32>,
    pub dots: Option<i32>,
    pub approved: Option<bool>,
}

/// Record a governance signal — API request
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RecordSignalInputView {
    pub entity_type: String,
    pub entity_id: String,
    pub human_id: String,
    pub signal_type: String,
    pub signal_value: String,
    pub mechanism_level: i32,
    #[serde(default)]
    pub proxy_elohim_id: Option<String>,
}
```

**Step 5: Add model imports at top of views.rs**

Add `ProposalOption`, `RankedVote`, `GovernanceSignal` to the `use crate::db::models::` import block.

**Step 6: Verify compilation**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -5`

**Step 7: Commit**

```bash
git add elohim/elohim-storage/src/views.rs
git commit -m "feat(storage): add View types for proposal options, ranked votes, governance signals"
```

---

### Task 6: CRUD functions — proposal_options, ranked_votes, governance_signals

**Files:**
- Modify: `elohim/elohim-storage/src/db/governance.rs`

**Step 1: Add proposal_options CRUD**

Add after the existing votes section:

```rust
// =========================================================================
// Proposal Options
// =========================================================================

/// Get options for a proposal
pub fn query_proposal_options(
    conn: &mut SqliteConnection,
    proposal_id: &str,
) -> Result<Vec<ProposalOption>, StorageError> {
    proposal_options::table
        .filter(proposal_options::proposal_id.eq(proposal_id))
        .order(proposal_options::position.asc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Create a proposal option
pub fn create_proposal_option(
    conn: &mut SqliteConnection,
    new: &NewProposalOption,
) -> Result<ProposalOption, StorageError> {
    diesel::insert_into(proposal_options::table)
        .values(new)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;

    proposal_options::table
        .filter(proposal_options::id.eq(new.id))
        .first(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Batch-create proposal options
pub fn create_proposal_options(
    conn: &mut SqliteConnection,
    options: &[NewProposalOption],
) -> Result<Vec<ProposalOption>, StorageError> {
    for opt in options {
        diesel::insert_into(proposal_options::table)
            .values(opt)
            .execute(conn)
            .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;
    }

    let ids: Vec<&str> = options.iter().map(|o| o.proposal_id).collect();
    let pid = ids.first().copied().unwrap_or("");

    proposal_options::table
        .filter(proposal_options::proposal_id.eq(pid))
        .order(proposal_options::position.asc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}
```

**Step 2: Add ranked_votes CRUD**

```rust
// =========================================================================
// Ranked Votes (multi-mechanism)
// =========================================================================

/// Get all ranked votes for a proposal
pub fn query_ranked_votes(
    conn: &mut SqliteConnection,
    proposal_id: &str,
) -> Result<Vec<RankedVote>, StorageError> {
    ranked_votes::table
        .filter(ranked_votes::proposal_id.eq(proposal_id))
        .order(ranked_votes::created_at.asc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Get a specific human's ranked votes on a proposal
pub fn get_ranked_votes_for_human(
    conn: &mut SqliteConnection,
    proposal_id: &str,
    human_id: &str,
) -> Result<Vec<RankedVote>, StorageError> {
    ranked_votes::table
        .filter(ranked_votes::proposal_id.eq(proposal_id))
        .filter(ranked_votes::human_id.eq(human_id))
        .order(ranked_votes::rank.asc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Cast ranked votes (upsert: delete existing for this human, then insert batch)
pub fn cast_ranked_votes(
    conn: &mut SqliteConnection,
    proposal_id: &str,
    human_id: &str,
    votes: &[NewRankedVote],
) -> Result<Vec<RankedVote>, StorageError> {
    // Delete existing votes for this human on this proposal
    diesel::delete(
        ranked_votes::table
            .filter(ranked_votes::proposal_id.eq(proposal_id))
            .filter(ranked_votes::human_id.eq(human_id)),
    )
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Delete failed: {}", e)))?;

    // Insert new votes
    for vote in votes {
        diesel::insert_into(ranked_votes::table)
            .values(vote)
            .execute(conn)
            .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;
    }

    get_ranked_votes_for_human(conn, proposal_id, human_id)
}
```

**Step 3: Add governance_signals CRUD**

```rust
// =========================================================================
// Governance Signals
// =========================================================================

/// Record a governance signal
pub fn record_signal(
    conn: &mut SqliteConnection,
    new: &NewGovernanceSignal,
) -> Result<GovernanceSignal, StorageError> {
    diesel::insert_into(governance_signals::table)
        .values(new)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;

    governance_signals::table
        .filter(governance_signals::id.eq(new.id))
        .first(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Query signals for an entity
pub fn query_signals(
    conn: &mut SqliteConnection,
    entity_type: &str,
    entity_id: &str,
) -> Result<Vec<GovernanceSignal>, StorageError> {
    governance_signals::table
        .filter(governance_signals::entity_type.eq(entity_type))
        .filter(governance_signals::entity_id.eq(entity_id))
        .order(governance_signals::created_at.desc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Count signals for an entity (sensemaking readiness check)
pub fn count_signals(
    conn: &mut SqliteConnection,
    entity_type: &str,
    entity_id: &str,
) -> Result<i64, StorageError> {
    governance_signals::table
        .filter(governance_signals::entity_type.eq(entity_type))
        .filter(governance_signals::entity_id.eq(entity_id))
        .count()
        .get_result(conn)
        .map_err(|e| StorageError::Internal(format!("Count failed: {}", e)))
}
```

**Step 4: Add table imports at top of governance.rs**

Add `proposal_options`, `ranked_votes`, `governance_signals` to the `use crate::db::diesel_schema::` block. Add new model imports: `NewProposalOption`, `ProposalOption`, `NewRankedVote`, `RankedVote`, `NewGovernanceSignal`, `GovernanceSignal`.

**Step 5: Verify compilation**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -5`

**Step 6: Commit**

```bash
git add elohim/elohim-storage/src/db/governance.rs
git commit -m "feat(storage): add CRUD for proposal options, ranked votes, governance signals"
```

---

### Task 7: TallyStrategy trait and implementations

**Files:**
- Create: `elohim/elohim-storage/src/tally/mod.rs`
- Create: `elohim/elohim-storage/src/tally/ranked_choice.rs`
- Create: `elohim/elohim-storage/src/tally/approval.rs`
- Create: `elohim/elohim-storage/src/tally/score.rs`
- Create: `elohim/elohim-storage/src/tally/dot.rs`
- Create: `elohim/elohim-storage/src/tally/consent.rs`
- Create: `elohim/elohim-storage/src/tally/conviction.rs`
- Modify: `elohim/elohim-storage/src/lib.rs` (add `pub mod tally;`)

**Step 1: Create tally/mod.rs with trait and registry**

```rust
//! Tally strategies for multi-mechanism voting.
//!
//! Each voting mechanism implements TallyStrategy. New mechanisms are
//! added by creating a struct, implementing the trait, and registering
//! it in `get_strategy()`.

pub mod approval;
pub mod consent;
pub mod conviction;
pub mod dot;
pub mod ranked_choice;
pub mod score;

use crate::db::models::{ProposalOption, RankedVote};
use serde::Serialize;
use ts_rs::TS;

/// Configuration passed to tally strategies
#[derive(Debug, Clone)]
pub struct VotingConfig {
    pub score_min: Option<i32>,
    pub score_max: Option<i32>,
    pub dots_per_voter: Option<i32>,
    pub quorum_percentage: Option<f64>,
    pub passage_threshold: Option<f64>,
}

/// Result of tallying votes
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct TallyResult {
    pub mechanism: String,
    pub total_voters: usize,
    pub quorum_met: bool,
    pub option_results: Vec<OptionResult>,
    pub recommendation: String, // "pass", "fail", "unclear", "blocked"
    pub rounds: Option<Vec<TallyRound>>, // for ranked-choice IRV rounds
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct OptionResult {
    pub option_id: String,
    pub label: String,
    pub votes: f64,
    pub percentage: f64,
    pub rank: Option<i32>,
    pub eliminated: bool,
}

/// Round details for ranked-choice IRV
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct TallyRound {
    pub round_number: i32,
    pub eliminated_option_id: Option<String>,
    pub standings: Vec<OptionResult>,
}

/// Ballot validation error
#[derive(Debug, Clone)]
pub enum BallotError {
    MissingOptions(Vec<String>),
    DuplicateRanks,
    ScoreOutOfRange { option_id: String, score: i32 },
    DotsExceedBudget { used: i32, budget: i32 },
    InvalidOptionId(String),
}

impl std::fmt::Display for BallotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingOptions(ids) => write!(f, "Missing options: {}", ids.join(", ")),
            Self::DuplicateRanks => write!(f, "Duplicate ranks in ballot"),
            Self::ScoreOutOfRange { option_id, score } => {
                write!(f, "Score {} out of range for option {}", score, option_id)
            }
            Self::DotsExceedBudget { used, budget } => {
                write!(f, "Dots used ({}) exceeds budget ({})", used, budget)
            }
            Self::InvalidOptionId(id) => write!(f, "Invalid option ID: {}", id),
        }
    }
}

/// The core trait. Each voting mechanism implements this.
pub trait TallyStrategy: Send + Sync {
    fn tally(
        &self,
        votes: &[RankedVote],
        options: &[ProposalOption],
        config: &VotingConfig,
    ) -> TallyResult;

    fn validate_ballot(
        &self,
        votes: &[RankedVote],
        options: &[ProposalOption],
        config: &VotingConfig,
    ) -> Result<(), BallotError>;
}

/// Look up a tally strategy by mechanism name.
/// Returns None for unknown mechanisms — extensible by adding new match arms.
pub fn get_strategy(mechanism: &str) -> Option<Box<dyn TallyStrategy>> {
    match mechanism {
        "ranked-choice" => Some(Box::new(ranked_choice::RankedChoiceTally)),
        "approval" => Some(Box::new(approval::ApprovalTally)),
        "score-vote" => Some(Box::new(score::ScoreTally)),
        "dot-vote" => Some(Box::new(dot::DotTally)),
        "consent" => Some(Box::new(consent::ConsentTally)),
        "conviction" => Some(Box::new(conviction::ConvictionTally)),
        _ => None,
    }
}
```

**Step 2: Create tally/ranked_choice.rs**

Instant-runoff voting: eliminate the option with fewest first-preference votes, redistribute to next preference, repeat until a majority.

```rust
use super::*;
use std::collections::{HashMap, HashSet};

pub struct RankedChoiceTally;

impl TallyStrategy for RankedChoiceTally {
    fn tally(
        &self,
        votes: &[RankedVote],
        options: &[ProposalOption],
        config: &VotingConfig,
    ) -> TallyResult {
        let option_labels: HashMap<&str, &str> = options.iter()
            .map(|o| (o.id.as_str(), o.label.as_str()))
            .collect();
        let option_ids: Vec<&str> = options.iter().map(|o| o.id.as_str()).collect();

        // Group votes by voter, sorted by rank
        let mut voter_ballots: HashMap<&str, Vec<&RankedVote>> = HashMap::new();
        for v in votes {
            voter_ballots.entry(&v.human_id).or_default().push(v);
        }
        for ballot in voter_ballots.values_mut() {
            ballot.sort_by_key(|v| v.rank.unwrap_or(999));
        }

        let total_voters = voter_ballots.len();
        let quorum_met = check_quorum(total_voters, config);
        let mut eliminated: HashSet<&str> = HashSet::new();
        let mut rounds: Vec<TallyRound> = Vec::new();

        loop {
            let active_options: Vec<&str> = option_ids.iter()
                .filter(|id| !eliminated.contains(**id))
                .copied()
                .collect();

            if active_options.is_empty() {
                break;
            }

            // Count first-valid-preference for each voter
            let mut counts: HashMap<&str, f64> = active_options.iter()
                .map(|id| (*id, 0.0))
                .collect();

            for ballot in voter_ballots.values() {
                for v in ballot {
                    if active_options.contains(&v.option_id.as_str()) {
                        *counts.entry(&v.option_id).or_default() += 1.0;
                        break;
                    }
                }
            }

            let total_counted: f64 = counts.values().sum();
            let standings: Vec<OptionResult> = {
                let mut s: Vec<_> = counts.iter().map(|(&id, &ct)| {
                    OptionResult {
                        option_id: id.to_string(),
                        label: option_labels.get(id).unwrap_or(&"").to_string(),
                        votes: ct,
                        percentage: if total_counted > 0.0 { ct / total_counted * 100.0 } else { 0.0 },
                        rank: None,
                        eliminated: false,
                    }
                }).collect();
                s.sort_by(|a, b| b.votes.partial_cmp(&a.votes).unwrap_or(std::cmp::Ordering::Equal));
                s
            };

            // Check if anyone has majority
            if let Some(winner) = standings.first() {
                if winner.percentage > 50.0 {
                    rounds.push(TallyRound {
                        round_number: rounds.len() as i32 + 1,
                        eliminated_option_id: None,
                        standings,
                    });
                    break;
                }
            }

            // Eliminate lowest
            let lowest = standings.last().map(|s| s.option_id.clone());
            if let Some(ref elim_id) = lowest {
                eliminated.insert(
                    option_ids.iter().find(|id| **id == elim_id.as_str()).copied().unwrap_or("")
                );
            }

            rounds.push(TallyRound {
                round_number: rounds.len() as i32 + 1,
                eliminated_option_id: lowest,
                standings,
            });

            // Safety: if only one option left, stop
            if active_options.len() <= 2 {
                break;
            }
        }

        // Build final results from last round
        let final_results: Vec<OptionResult> = if let Some(last_round) = rounds.last() {
            let mut results = last_round.standings.clone();
            // Mark eliminated options
            for r in &mut results {
                if eliminated.contains(r.option_id.as_str()) {
                    r.eliminated = true;
                }
            }
            // Add back eliminated options not in final round
            for &id in &option_ids {
                if !results.iter().any(|r| r.option_id == id) {
                    results.push(OptionResult {
                        option_id: id.to_string(),
                        label: option_labels.get(id).unwrap_or(&"").to_string(),
                        votes: 0.0,
                        percentage: 0.0,
                        rank: None,
                        eliminated: true,
                    });
                }
            }
            // Assign ranks
            for (i, r) in results.iter_mut().enumerate() {
                r.rank = Some(i as i32 + 1);
            }
            results
        } else {
            Vec::new()
        };

        let recommendation = if !quorum_met {
            "unclear".to_string()
        } else if final_results.first().map(|r| r.percentage > 50.0).unwrap_or(false) {
            "pass".to_string()
        } else {
            "unclear".to_string()
        };

        TallyResult {
            mechanism: "ranked-choice".to_string(),
            total_voters,
            quorum_met,
            option_results: final_results,
            recommendation,
            rounds: Some(rounds),
        }
    }

    fn validate_ballot(
        &self,
        votes: &[RankedVote],
        options: &[ProposalOption],
        _config: &VotingConfig,
    ) -> Result<(), BallotError> {
        let valid_ids: HashSet<&str> = options.iter().map(|o| o.id.as_str()).collect();
        let mut ranks_seen = HashSet::new();

        for v in votes {
            if !valid_ids.contains(v.option_id.as_str()) {
                return Err(BallotError::InvalidOptionId(v.option_id.clone()));
            }
            if let Some(rank) = v.rank {
                if !ranks_seen.insert(rank) {
                    return Err(BallotError::DuplicateRanks);
                }
            }
        }
        Ok(())
    }
}

fn check_quorum(total_voters: usize, config: &VotingConfig) -> bool {
    match config.quorum_percentage {
        Some(q) if q > 0.0 => {
            // For now, quorum is checked against total_voters vs some external eligible count.
            // Without eligible count, assume quorum is met if anyone voted.
            total_voters > 0
        }
        _ => true,
    }
}
```

**Step 3: Create tally/approval.rs**

```rust
use super::*;
use std::collections::HashMap;

pub struct ApprovalTally;

impl TallyStrategy for ApprovalTally {
    fn tally(
        &self,
        votes: &[RankedVote],
        options: &[ProposalOption],
        config: &VotingConfig,
    ) -> TallyResult {
        let option_labels: HashMap<&str, &str> = options.iter()
            .map(|o| (o.id.as_str(), o.label.as_str()))
            .collect();

        let voter_ids: std::collections::HashSet<&str> = votes.iter()
            .map(|v| v.human_id.as_str())
            .collect();
        let total_voters = voter_ids.len();
        let quorum_met = check_quorum(total_voters, config);

        let mut counts: HashMap<&str, f64> = HashMap::new();
        for v in votes {
            if v.approved == Some(1) {
                *counts.entry(&v.option_id).or_default() += 1.0;
            }
        }

        let mut results: Vec<OptionResult> = options.iter().map(|o| {
            let ct = counts.get(o.id.as_str()).copied().unwrap_or(0.0);
            OptionResult {
                option_id: o.id.clone(),
                label: o.label.clone(),
                votes: ct,
                percentage: if total_voters > 0 { ct / total_voters as f64 * 100.0 } else { 0.0 },
                rank: None,
                eliminated: false,
            }
        }).collect();

        results.sort_by(|a, b| b.votes.partial_cmp(&a.votes).unwrap_or(std::cmp::Ordering::Equal));
        for (i, r) in results.iter_mut().enumerate() {
            r.rank = Some(i as i32 + 1);
        }

        let threshold = config.passage_threshold.unwrap_or(0.5);
        let recommendation = if !quorum_met {
            "unclear"
        } else if results.first().map(|r| r.percentage / 100.0 >= threshold).unwrap_or(false) {
            "pass"
        } else {
            "fail"
        }.to_string();

        TallyResult {
            mechanism: "approval".to_string(),
            total_voters,
            quorum_met,
            option_results: results,
            recommendation,
            rounds: None,
        }
    }

    fn validate_ballot(
        &self,
        votes: &[RankedVote],
        options: &[ProposalOption],
        _config: &VotingConfig,
    ) -> Result<(), BallotError> {
        let valid_ids: std::collections::HashSet<&str> = options.iter().map(|o| o.id.as_str()).collect();
        for v in votes {
            if !valid_ids.contains(v.option_id.as_str()) {
                return Err(BallotError::InvalidOptionId(v.option_id.clone()));
            }
        }
        Ok(())
    }
}

fn check_quorum(total_voters: usize, config: &VotingConfig) -> bool {
    config.quorum_percentage.map_or(true, |q| q <= 0.0 || total_voters > 0)
}
```

**Step 4: Create tally/score.rs**

```rust
use super::*;
use std::collections::HashMap;

pub struct ScoreTally;

impl TallyStrategy for ScoreTally {
    fn tally(
        &self,
        votes: &[RankedVote],
        options: &[ProposalOption],
        config: &VotingConfig,
    ) -> TallyResult {
        let voter_ids: std::collections::HashSet<&str> = votes.iter()
            .map(|v| v.human_id.as_str())
            .collect();
        let total_voters = voter_ids.len();
        let quorum_met = config.quorum_percentage.map_or(true, |q| q <= 0.0 || total_voters > 0);

        let mut totals: HashMap<&str, f64> = HashMap::new();
        for v in votes {
            if let Some(s) = v.score {
                *totals.entry(&v.option_id).or_default() += s as f64;
            }
        }

        let mut results: Vec<OptionResult> = options.iter().map(|o| {
            let total = totals.get(o.id.as_str()).copied().unwrap_or(0.0);
            let max_possible = total_voters as f64 * config.score_max.unwrap_or(10) as f64;
            OptionResult {
                option_id: o.id.clone(),
                label: o.label.clone(),
                votes: total,
                percentage: if max_possible > 0.0 { total / max_possible * 100.0 } else { 0.0 },
                rank: None,
                eliminated: false,
            }
        }).collect();

        results.sort_by(|a, b| b.votes.partial_cmp(&a.votes).unwrap_or(std::cmp::Ordering::Equal));
        for (i, r) in results.iter_mut().enumerate() {
            r.rank = Some(i as i32 + 1);
        }

        TallyResult {
            mechanism: "score-vote".to_string(),
            total_voters,
            quorum_met,
            option_results: results,
            recommendation: if quorum_met { "pass" } else { "unclear" }.to_string(),
            rounds: None,
        }
    }

    fn validate_ballot(
        &self,
        votes: &[RankedVote],
        options: &[ProposalOption],
        config: &VotingConfig,
    ) -> Result<(), BallotError> {
        let valid_ids: std::collections::HashSet<&str> = options.iter().map(|o| o.id.as_str()).collect();
        let min = config.score_min.unwrap_or(0);
        let max = config.score_max.unwrap_or(10);

        for v in votes {
            if !valid_ids.contains(v.option_id.as_str()) {
                return Err(BallotError::InvalidOptionId(v.option_id.clone()));
            }
            if let Some(s) = v.score {
                if s < min || s > max {
                    return Err(BallotError::ScoreOutOfRange {
                        option_id: v.option_id.clone(),
                        score: s,
                    });
                }
            }
        }
        Ok(())
    }
}
```

**Step 5: Create tally/dot.rs**

```rust
use super::*;
use std::collections::HashMap;

pub struct DotTally;

impl TallyStrategy for DotTally {
    fn tally(
        &self,
        votes: &[RankedVote],
        options: &[ProposalOption],
        config: &VotingConfig,
    ) -> TallyResult {
        let voter_ids: std::collections::HashSet<&str> = votes.iter()
            .map(|v| v.human_id.as_str())
            .collect();
        let total_voters = voter_ids.len();
        let quorum_met = config.quorum_percentage.map_or(true, |q| q <= 0.0 || total_voters > 0);

        let mut totals: HashMap<&str, f64> = HashMap::new();
        for v in votes {
            if let Some(d) = v.dots {
                *totals.entry(&v.option_id).or_default() += d as f64;
            }
        }

        let total_dots: f64 = totals.values().sum();
        let mut results: Vec<OptionResult> = options.iter().map(|o| {
            let ct = totals.get(o.id.as_str()).copied().unwrap_or(0.0);
            OptionResult {
                option_id: o.id.clone(),
                label: o.label.clone(),
                votes: ct,
                percentage: if total_dots > 0.0 { ct / total_dots * 100.0 } else { 0.0 },
                rank: None,
                eliminated: false,
            }
        }).collect();

        results.sort_by(|a, b| b.votes.partial_cmp(&a.votes).unwrap_or(std::cmp::Ordering::Equal));
        for (i, r) in results.iter_mut().enumerate() {
            r.rank = Some(i as i32 + 1);
        }

        TallyResult {
            mechanism: "dot-vote".to_string(),
            total_voters,
            quorum_met,
            option_results: results,
            recommendation: if quorum_met { "pass" } else { "unclear" }.to_string(),
            rounds: None,
        }
    }

    fn validate_ballot(
        &self,
        votes: &[RankedVote],
        options: &[ProposalOption],
        config: &VotingConfig,
    ) -> Result<(), BallotError> {
        let valid_ids: std::collections::HashSet<&str> = options.iter().map(|o| o.id.as_str()).collect();
        let budget = config.dots_per_voter.unwrap_or(10);
        let mut total_dots = 0;

        for v in votes {
            if !valid_ids.contains(v.option_id.as_str()) {
                return Err(BallotError::InvalidOptionId(v.option_id.clone()));
            }
            total_dots += v.dots.unwrap_or(0);
        }

        if total_dots > budget {
            return Err(BallotError::DotsExceedBudget { used: total_dots, budget });
        }
        Ok(())
    }
}
```

**Step 6: Create tally/consent.rs**

Consent: passes unless a block exists. Blocks trigger escalation, not veto.

```rust
use super::*;

pub struct ConsentTally;

impl TallyStrategy for ConsentTally {
    fn tally(
        &self,
        votes: &[RankedVote],
        options: &[ProposalOption],
        config: &VotingConfig,
    ) -> TallyResult {
        // In consent, there's typically one option (the proposal itself).
        // Voters mark approved=1 (consent) or approved=0 (block).
        // The `reasoning` field is required for blocks.
        let voter_ids: std::collections::HashSet<&str> = votes.iter()
            .map(|v| v.human_id.as_str())
            .collect();
        let total_voters = voter_ids.len();
        let quorum_met = config.quorum_percentage.map_or(true, |q| q <= 0.0 || total_voters > 0);

        let consents = votes.iter().filter(|v| v.approved == Some(1)).count();
        let blocks = votes.iter().filter(|v| v.approved == Some(0)).count();
        let abstains = total_voters.saturating_sub(consents + blocks);

        let results = vec![
            OptionResult {
                option_id: "consent".to_string(),
                label: "Consent".to_string(),
                votes: consents as f64,
                percentage: if total_voters > 0 { consents as f64 / total_voters as f64 * 100.0 } else { 0.0 },
                rank: Some(1),
                eliminated: false,
            },
            OptionResult {
                option_id: "abstain".to_string(),
                label: "Abstain".to_string(),
                votes: abstains as f64,
                percentage: if total_voters > 0 { abstains as f64 / total_voters as f64 * 100.0 } else { 0.0 },
                rank: Some(2),
                eliminated: false,
            },
            OptionResult {
                option_id: "block".to_string(),
                label: "Block".to_string(),
                votes: blocks as f64,
                percentage: if total_voters > 0 { blocks as f64 / total_voters as f64 * 100.0 } else { 0.0 },
                rank: Some(3),
                eliminated: false,
            },
        ];

        // Blocks don't veto — they trigger escalation. But we signal it.
        let recommendation = if !quorum_met {
            "unclear"
        } else if blocks > 0 {
            "blocked" // signals escalation needed, not rejection
        } else {
            "pass"
        }.to_string();

        TallyResult {
            mechanism: "consent".to_string(),
            total_voters,
            quorum_met,
            option_results: results,
            recommendation,
            rounds: None,
        }
    }

    fn validate_ballot(
        &self,
        _votes: &[RankedVote],
        _options: &[ProposalOption],
        _config: &VotingConfig,
    ) -> Result<(), BallotError> {
        // Consent ballots are simple: approved=1 or approved=0
        Ok(())
    }
}
```

**Step 7: Create tally/conviction.rs**

Conviction voting: votes accumulate weight over time. Weight = 1 + (days_held * decay_factor). This rewards sustained conviction.

```rust
use super::*;
use std::collections::HashMap;

pub struct ConvictionTally;

const CONVICTION_HALF_LIFE_DAYS: f64 = 7.0;

impl TallyStrategy for ConvictionTally {
    fn tally(
        &self,
        votes: &[RankedVote],
        options: &[ProposalOption],
        config: &VotingConfig,
    ) -> TallyResult {
        let now = chrono::Utc::now();
        let voter_ids: std::collections::HashSet<&str> = votes.iter()
            .map(|v| v.human_id.as_str())
            .collect();
        let total_voters = voter_ids.len();
        let quorum_met = config.quorum_percentage.map_or(true, |q| q <= 0.0 || total_voters > 0);

        let mut weighted: HashMap<&str, f64> = HashMap::new();
        for v in votes {
            if v.approved != Some(1) {
                continue;
            }
            let created = chrono::NaiveDateTime::parse_from_str(&v.created_at, "%Y-%m-%dT%H:%M:%SZ")
                .unwrap_or_else(|_| now.naive_utc());
            let days_held = (now.naive_utc() - created).num_hours() as f64 / 24.0;
            // Conviction grows with time: weight = 1 - 0.5^(days/half_life)
            // Approaches 1.0 asymptotically, starts near 0
            let weight = 1.0 - (0.5_f64).powf(days_held / CONVICTION_HALF_LIFE_DAYS);
            *weighted.entry(&v.option_id).or_default() += weight.max(0.01); // minimum weight
        }

        let total_weight: f64 = weighted.values().sum();
        let mut results: Vec<OptionResult> = options.iter().map(|o| {
            let w = weighted.get(o.id.as_str()).copied().unwrap_or(0.0);
            OptionResult {
                option_id: o.id.clone(),
                label: o.label.clone(),
                votes: w,
                percentage: if total_weight > 0.0 { w / total_weight * 100.0 } else { 0.0 },
                rank: None,
                eliminated: false,
            }
        }).collect();

        results.sort_by(|a, b| b.votes.partial_cmp(&a.votes).unwrap_or(std::cmp::Ordering::Equal));
        for (i, r) in results.iter_mut().enumerate() {
            r.rank = Some(i as i32 + 1);
        }

        let threshold = config.passage_threshold.unwrap_or(0.5);
        let recommendation = if !quorum_met {
            "unclear"
        } else if results.first().map(|r| r.percentage / 100.0 >= threshold).unwrap_or(false) {
            "pass"
        } else {
            "unclear"
        }.to_string();

        TallyResult {
            mechanism: "conviction".to_string(),
            total_voters,
            quorum_met,
            option_results: results,
            recommendation,
            rounds: None,
        }
    }

    fn validate_ballot(
        &self,
        votes: &[RankedVote],
        options: &[ProposalOption],
        _config: &VotingConfig,
    ) -> Result<(), BallotError> {
        let valid_ids: std::collections::HashSet<&str> = options.iter().map(|o| o.id.as_str()).collect();
        for v in votes {
            if !valid_ids.contains(v.option_id.as_str()) {
                return Err(BallotError::InvalidOptionId(v.option_id.clone()));
            }
        }
        Ok(())
    }
}
```

**Step 8: Add `pub mod tally;` to lib.rs**

Find `elohim/elohim-storage/src/lib.rs` and add `pub mod tally;` alongside other module declarations.

**Step 9: Verify compilation**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -5`
Expected: Compiles with no errors.

**Step 10: Commit**

```bash
git add elohim/elohim-storage/src/tally/ elohim/elohim-storage/src/lib.rs
git commit -m "feat(storage): add TallyStrategy trait with 6 voting mechanism implementations"
```

---

### Task 8: Tally strategy unit tests

**Files:**
- Create: `elohim/elohim-storage/src/tally/tests.rs`
- Modify: `elohim/elohim-storage/src/tally/mod.rs` (add `#[cfg(test)] mod tests;`)

**Step 1: Create test helpers and ranked-choice tests**

Create `tally/tests.rs` with helpers that build `ProposalOption` and `RankedVote` structs for testing:

```rust
use super::*;
use crate::db::models::{ProposalOption, RankedVote};

fn make_option(id: &str, label: &str, pos: i32) -> ProposalOption {
    ProposalOption {
        id: id.to_string(),
        proposal_id: "prop-1".to_string(),
        label: label.to_string(),
        description: String::new(),
        position: pos,
        source: None,
        source_justification: None,
        created_at: "2026-01-01T00:00:00Z".to_string(),
    }
}

fn make_ranked_vote(human: &str, option: &str, rank: i32) -> RankedVote {
    RankedVote {
        id: format!("rv-{}-{}", human, option),
        proposal_id: "prop-1".to_string(),
        human_id: human.to_string(),
        option_id: option.to_string(),
        rank: Some(rank),
        score: None,
        dots: None,
        approved: None,
        reasoning: None,
        proxy_elohim_id: None,
        proxy_justification: None,
        created_at: "2026-01-01T00:00:00Z".to_string(),
        updated_at: "2026-01-01T00:00:00Z".to_string(),
    }
}

fn make_approval_vote(human: &str, option: &str, approved: bool) -> RankedVote {
    RankedVote {
        id: format!("av-{}-{}", human, option),
        proposal_id: "prop-1".to_string(),
        human_id: human.to_string(),
        option_id: option.to_string(),
        rank: None,
        score: None,
        dots: None,
        approved: Some(if approved { 1 } else { 0 }),
        reasoning: None,
        proxy_elohim_id: None,
        proxy_justification: None,
        created_at: "2026-01-01T00:00:00Z".to_string(),
        updated_at: "2026-01-01T00:00:00Z".to_string(),
    }
}

fn make_score_vote(human: &str, option: &str, score: i32) -> RankedVote {
    RankedVote {
        id: format!("sv-{}-{}", human, option),
        proposal_id: "prop-1".to_string(),
        human_id: human.to_string(),
        option_id: option.to_string(),
        rank: None,
        score: Some(score),
        dots: None,
        approved: None,
        reasoning: None,
        proxy_elohim_id: None,
        proxy_justification: None,
        created_at: "2026-01-01T00:00:00Z".to_string(),
        updated_at: "2026-01-01T00:00:00Z".to_string(),
    }
}

fn make_dot_vote(human: &str, option: &str, dots: i32) -> RankedVote {
    RankedVote {
        id: format!("dv-{}-{}", human, option),
        proposal_id: "prop-1".to_string(),
        human_id: human.to_string(),
        option_id: option.to_string(),
        rank: None,
        score: None,
        dots: Some(dots),
        approved: None,
        reasoning: None,
        proxy_elohim_id: None,
        proxy_justification: None,
        created_at: "2026-01-01T00:00:00Z".to_string(),
        updated_at: "2026-01-01T00:00:00Z".to_string(),
    }
}

fn default_config() -> VotingConfig {
    VotingConfig {
        score_min: None,
        score_max: None,
        dots_per_voter: None,
        quorum_percentage: None,
        passage_threshold: None,
    }
}

// =========================================================================
// Ranked-Choice (IRV) Tests
// =========================================================================

#[test]
fn ranked_choice_clear_majority_first_round() {
    let options = vec![
        make_option("a", "Option A", 1),
        make_option("b", "Option B", 2),
        make_option("c", "Option C", 3),
    ];
    // 3 voters prefer A, 1 prefers B, 1 prefers C → A wins round 1
    let votes = vec![
        make_ranked_vote("h1", "a", 1), make_ranked_vote("h1", "b", 2), make_ranked_vote("h1", "c", 3),
        make_ranked_vote("h2", "a", 1), make_ranked_vote("h2", "c", 2), make_ranked_vote("h2", "b", 3),
        make_ranked_vote("h3", "a", 1), make_ranked_vote("h3", "b", 2), make_ranked_vote("h3", "c", 3),
        make_ranked_vote("h4", "b", 1), make_ranked_vote("h4", "a", 2), make_ranked_vote("h4", "c", 3),
        make_ranked_vote("h5", "c", 1), make_ranked_vote("h5", "b", 2), make_ranked_vote("h5", "a", 3),
    ];

    let strategy = ranked_choice::RankedChoiceTally;
    let result = strategy.tally(&votes, &options, &default_config());

    assert_eq!(result.mechanism, "ranked-choice");
    assert_eq!(result.total_voters, 5);
    assert_eq!(result.recommendation, "pass");
    assert_eq!(result.option_results[0].option_id, "a");
    assert!(result.option_results[0].percentage > 50.0);
}

#[test]
fn ranked_choice_elimination_redistributes() {
    let options = vec![
        make_option("a", "Option A", 1),
        make_option("b", "Option B", 2),
        make_option("c", "Option C", 3),
    ];
    // No first-round majority: A=2, B=2, C=1. C eliminated, C's voter preferred B.
    let votes = vec![
        make_ranked_vote("h1", "a", 1), make_ranked_vote("h1", "b", 2), make_ranked_vote("h1", "c", 3),
        make_ranked_vote("h2", "a", 1), make_ranked_vote("h2", "c", 2), make_ranked_vote("h2", "b", 3),
        make_ranked_vote("h3", "b", 1), make_ranked_vote("h3", "a", 2), make_ranked_vote("h3", "c", 3),
        make_ranked_vote("h4", "b", 1), make_ranked_vote("h4", "c", 2), make_ranked_vote("h4", "a", 3),
        make_ranked_vote("h5", "c", 1), make_ranked_vote("h5", "b", 2), make_ranked_vote("h5", "a", 3),
    ];

    let strategy = ranked_choice::RankedChoiceTally;
    let result = strategy.tally(&votes, &options, &default_config());

    assert!(result.rounds.as_ref().unwrap().len() >= 2, "Should take multiple rounds");
    // B should win after redistribution (B gets C's voter)
    assert_eq!(result.option_results[0].option_id, "b");
    assert_eq!(result.recommendation, "pass");
}

#[test]
fn ranked_choice_empty_votes() {
    let options = vec![make_option("a", "A", 1)];
    let strategy = ranked_choice::RankedChoiceTally;
    let result = strategy.tally(&[], &options, &default_config());
    assert_eq!(result.total_voters, 0);
}

// =========================================================================
// Approval Tests
// =========================================================================

#[test]
fn approval_highest_approved_wins() {
    let options = vec![
        make_option("a", "A", 1),
        make_option("b", "B", 2),
        make_option("c", "C", 3),
    ];
    let votes = vec![
        make_approval_vote("h1", "a", true), make_approval_vote("h1", "b", true),
        make_approval_vote("h2", "b", true), make_approval_vote("h2", "c", true),
        make_approval_vote("h3", "b", true),
    ];

    let strategy = approval::ApprovalTally;
    let result = strategy.tally(&votes, &options, &default_config());

    assert_eq!(result.option_results[0].option_id, "b"); // 3 approvals
    assert_eq!(result.option_results[0].votes, 3.0);
}

// =========================================================================
// Score Tests
// =========================================================================

#[test]
fn score_highest_total_wins() {
    let options = vec![
        make_option("a", "A", 1),
        make_option("b", "B", 2),
    ];
    let config = VotingConfig { score_max: Some(10), ..default_config() };
    let votes = vec![
        make_score_vote("h1", "a", 8), make_score_vote("h1", "b", 6),
        make_score_vote("h2", "a", 3), make_score_vote("h2", "b", 9),
    ];

    let strategy = score::ScoreTally;
    let result = strategy.tally(&votes, &options, &config);

    // A=11, B=15 → B wins
    assert_eq!(result.option_results[0].option_id, "b");
    assert_eq!(result.option_results[0].votes, 15.0);
}

#[test]
fn score_validates_range() {
    let options = vec![make_option("a", "A", 1)];
    let config = VotingConfig { score_min: Some(1), score_max: Some(5), ..default_config() };
    let votes = vec![make_score_vote("h1", "a", 7)]; // out of range

    let strategy = score::ScoreTally;
    assert!(strategy.validate_ballot(&votes, &options, &config).is_err());
}

// =========================================================================
// Dot Vote Tests
// =========================================================================

#[test]
fn dot_distributes_budget() {
    let options = vec![
        make_option("a", "A", 1),
        make_option("b", "B", 2),
    ];
    let config = VotingConfig { dots_per_voter: Some(10), ..default_config() };
    let votes = vec![
        make_dot_vote("h1", "a", 7), make_dot_vote("h1", "b", 3),
        make_dot_vote("h2", "a", 2), make_dot_vote("h2", "b", 8),
    ];

    let strategy = dot::DotTally;
    let result = strategy.tally(&votes, &options, &config);

    // A=9, B=11
    assert_eq!(result.option_results[0].option_id, "b");
}

#[test]
fn dot_validates_budget() {
    let options = vec![make_option("a", "A", 1), make_option("b", "B", 2)];
    let config = VotingConfig { dots_per_voter: Some(5), ..default_config() };
    let votes = vec![make_dot_vote("h1", "a", 3), make_dot_vote("h1", "b", 4)]; // 7 > 5

    let strategy = dot::DotTally;
    assert!(strategy.validate_ballot(&votes, &options, &config).is_err());
}

// =========================================================================
// Consent Tests
// =========================================================================

#[test]
fn consent_passes_without_blocks() {
    let options = vec![make_option("a", "Proposal", 1)];
    let votes = vec![
        make_approval_vote("h1", "a", true),
        make_approval_vote("h2", "a", true),
    ];

    let strategy = consent::ConsentTally;
    let result = strategy.tally(&votes, &options, &default_config());

    assert_eq!(result.recommendation, "pass");
}

#[test]
fn consent_blocked_triggers_escalation() {
    let options = vec![make_option("a", "Proposal", 1)];
    let votes = vec![
        make_approval_vote("h1", "a", true),
        make_approval_vote("h2", "a", false), // block
    ];

    let strategy = consent::ConsentTally;
    let result = strategy.tally(&votes, &options, &default_config());

    assert_eq!(result.recommendation, "blocked");
}

// =========================================================================
// Strategy Registry Tests
// =========================================================================

#[test]
fn get_strategy_returns_all_known_mechanisms() {
    assert!(get_strategy("ranked-choice").is_some());
    assert!(get_strategy("approval").is_some());
    assert!(get_strategy("score-vote").is_some());
    assert!(get_strategy("dot-vote").is_some());
    assert!(get_strategy("consent").is_some());
    assert!(get_strategy("conviction").is_some());
    assert!(get_strategy("unknown").is_none());
}
```

**Step 2: Add test module to mod.rs**

Add at bottom of `tally/mod.rs`:

```rust
#[cfg(test)]
mod tests;
```

**Step 3: Run tests**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test tally -- --nocapture 2>&1 | tail -20`
Expected: All tally tests pass.

**Step 4: Commit**

```bash
git add elohim/elohim-storage/src/tally/
git commit -m "test(storage): add tally strategy unit tests — ranked-choice IRV, approval, score, dot, consent"
```

---

### Task 9: HTTP routes — multi-mechanism voting endpoints

**Files:**
- Modify: `elohim/elohim-storage/src/api/governance.rs`
- Modify: `elohim/elohim-storage/src/http.rs`

**Step 1: Add imports to api/governance.rs**

Add to the import block:

```rust
use crate::db::models::{NewProposalOption, NewRankedVote, NewGovernanceSignal};
use crate::views::{
    ProposalOptionView, RankedVoteView, GovernanceSignalView,
    CreateProposalOptionInputView, CastRankedVoteInputView, RecordSignalInputView,
};
use crate::tally;
```

**Step 2: Add proposal options routes**

Add to the match block in `handle()`, before the general `GET /proposals/{id}` pattern:

```rust
// POST /api/v1/governance/proposals/{id}/options — Add options to a proposal
(&Method::POST, p) if p.starts_with("/proposals/") && p.ends_with("/options") => {
    let id = p.strip_prefix("/proposals/")
        .and_then(|s| s.strip_suffix("/options"))
        .ok_or_else(|| StorageError::InvalidInput("Proposal ID required".to_string()))?;

    let body = req.collect().await
        .map_err(|e| StorageError::Internal(format!("Body read failed: {}", e)))?;
    let inputs: Vec<CreateProposalOptionInputView> = serde_json::from_slice(&body.to_bytes())
        .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

    let mut conn = get_conn(pool)?;
    let now = crate::db::models::current_timestamp();
    let new_options: Vec<NewProposalOption> = inputs.iter().map(|i| {
        NewProposalOption {
            id: &i.id,
            proposal_id: id,
            label: &i.label,
            description: &i.description,
            position: i.position,
            source: i.source.as_deref(),
            source_justification: i.source_justification.as_deref(),
            created_at: &now,
        }
    }).collect();

    let results = governance::create_proposal_options(&mut conn, &new_options)?;
    let views: Vec<ProposalOptionView> = results.into_iter().map(Into::into).collect();
    Ok(response::created(&views))
}

// GET /api/v1/governance/proposals/{id}/options — List options
(&Method::GET, p) if p.starts_with("/proposals/") && p.ends_with("/options") => {
    let id = p.strip_prefix("/proposals/")
        .and_then(|s| s.strip_suffix("/options"))
        .ok_or_else(|| StorageError::InvalidInput("Proposal ID required".to_string()))?;

    let mut conn = get_conn(pool)?;
    let results = governance::query_proposal_options(&mut conn, id)?;
    let views: Vec<ProposalOptionView> = results.into_iter().map(Into::into).collect();
    Ok(response::ok(&views))
}
```

**Step 3: Add ranked voting routes**

```rust
// POST /api/v1/governance/proposals/{id}/ranked-votes — Cast a multi-mechanism ballot
(&Method::POST, p) if p.starts_with("/proposals/") && p.ends_with("/ranked-votes") => {
    let id = p.strip_prefix("/proposals/")
        .and_then(|s| s.strip_suffix("/ranked-votes"))
        .ok_or_else(|| StorageError::InvalidInput("Proposal ID required".to_string()))?;

    let body = req.collect().await
        .map_err(|e| StorageError::Internal(format!("Body read failed: {}", e)))?;
    let input: CastRankedVoteInputView = serde_json::from_slice(&body.to_bytes())
        .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

    let mut conn = get_conn(pool)?;
    let proposal = governance::get_proposal(&mut conn, id)?
        .ok_or_else(|| StorageError::NotFound(format!("Proposal {} not found", id)))?;

    // Validate ballot against mechanism
    let options = governance::query_proposal_options(&mut conn, id)?;
    let config = tally::VotingConfig {
        score_min: proposal.score_min,
        score_max: proposal.score_max,
        dots_per_voter: proposal.dots_per_voter,
        quorum_percentage: proposal.quorum_percentage.map(|v| v as f64),
        passage_threshold: proposal.passage_threshold.map(|v| v as f64),
    };

    let now = crate::db::models::current_timestamp();
    let new_votes: Vec<NewRankedVote> = input.ballots.iter().enumerate().map(|(i, b)| {
        NewRankedVote {
            id: &format!("rv-{}-{}-{}", id, input.human_id, i),
            proposal_id: id,
            human_id: &input.human_id,
            option_id: &b.option_id,
            rank: b.rank,
            score: b.score,
            dots: b.dots,
            approved: b.approved.map(|a| if a { 1 } else { 0 }),
            reasoning: input.reasoning.as_deref(),
            proxy_elohim_id: input.proxy_elohim_id.as_deref(),
            proxy_justification: input.proxy_justification.as_deref(),
            created_at: &now,
            updated_at: &now,
        }
    }).collect();

    // Validate before writing
    if let Some(strategy) = tally::get_strategy(&proposal.voting_mechanism) {
        // Build temporary RankedVote structs for validation
        let temp_votes: Vec<crate::db::models::RankedVote> = new_votes.iter().map(|nv| {
            crate::db::models::RankedVote {
                id: nv.id.to_string(),
                proposal_id: nv.proposal_id.to_string(),
                human_id: nv.human_id.to_string(),
                option_id: nv.option_id.to_string(),
                rank: nv.rank,
                score: nv.score,
                dots: nv.dots,
                approved: nv.approved,
                reasoning: nv.reasoning.map(|s| s.to_string()),
                proxy_elohim_id: nv.proxy_elohim_id.map(|s| s.to_string()),
                proxy_justification: nv.proxy_justification.map(|s| s.to_string()),
                created_at: nv.created_at.to_string(),
                updated_at: nv.updated_at.to_string(),
            }
        }).collect();
        if let Err(e) = strategy.validate_ballot(&temp_votes, &options, &config) {
            return Err(StorageError::InvalidInput(format!("Invalid ballot: {}", e)));
        }
    }

    let results = governance::cast_ranked_votes(&mut conn, id, &input.human_id, &new_votes)?;
    let hide = proposal.voting_anonymous == 1;
    let views: Vec<RankedVoteView> = results.into_iter()
        .map(|v| RankedVoteView::from_ranked_vote(v, hide))
        .collect();
    Ok(response::created(&views))
}

// GET /api/v1/governance/proposals/{id}/ranked-votes — List ranked votes
(&Method::GET, p) if p.starts_with("/proposals/") && p.ends_with("/ranked-votes") => {
    let id = p.strip_prefix("/proposals/")
        .and_then(|s| s.strip_suffix("/ranked-votes"))
        .ok_or_else(|| StorageError::InvalidInput("Proposal ID required".to_string()))?;

    let mut conn = get_conn(pool)?;
    let proposal = governance::get_proposal(&mut conn, id)?
        .ok_or_else(|| StorageError::NotFound(format!("Proposal {} not found", id)))?;
    let hide = proposal.voting_anonymous == 1;
    let results = governance::query_ranked_votes(&mut conn, id)?;
    let views: Vec<RankedVoteView> = results.into_iter()
        .map(|v| RankedVoteView::from_ranked_vote(v, hide))
        .collect();
    Ok(response::ok(&views))
}

// GET /api/v1/governance/proposals/{id}/tally — Compute tally results
(&Method::GET, p) if p.starts_with("/proposals/") && p.ends_with("/tally") => {
    let id = p.strip_prefix("/proposals/")
        .and_then(|s| s.strip_suffix("/tally"))
        .ok_or_else(|| StorageError::InvalidInput("Proposal ID required".to_string()))?;

    let mut conn = get_conn(pool)?;
    let proposal = governance::get_proposal(&mut conn, id)?
        .ok_or_else(|| StorageError::NotFound(format!("Proposal {} not found", id)))?;
    let options = governance::query_proposal_options(&mut conn, id)?;
    let votes = governance::query_ranked_votes(&mut conn, id)?;

    let config = tally::VotingConfig {
        score_min: proposal.score_min,
        score_max: proposal.score_max,
        dots_per_voter: proposal.dots_per_voter,
        quorum_percentage: proposal.quorum_percentage.map(|v| v as f64),
        passage_threshold: proposal.passage_threshold.map(|v| v as f64),
    };

    let strategy = tally::get_strategy(&proposal.voting_mechanism)
        .ok_or_else(|| StorageError::InvalidInput(
            format!("Unknown voting mechanism: {}", proposal.voting_mechanism)
        ))?;

    let result = strategy.tally(&votes, &options, &config);
    Ok(response::ok(&result))
}
```

**Step 4: Add governance signals route**

```rust
// POST /api/v1/governance/signals — Record a governance signal
(&Method::POST, "/signals") => {
    let body = req.collect().await
        .map_err(|e| StorageError::Internal(format!("Body read failed: {}", e)))?;
    let input: RecordSignalInputView = serde_json::from_slice(&body.to_bytes())
        .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

    let mut conn = get_conn(pool)?;
    let now = crate::db::models::current_timestamp();
    let signal_id = format!("sig-{}-{}", input.entity_id, now.replace([':', '-', 'T', 'Z'], ""));
    let new_signal = NewGovernanceSignal {
        id: &signal_id,
        entity_type: &input.entity_type,
        entity_id: &input.entity_id,
        human_id: &input.human_id,
        signal_type: &input.signal_type,
        signal_value: &input.signal_value,
        mechanism_level: input.mechanism_level,
        proxy_elohim_id: input.proxy_elohim_id.as_deref(),
        created_at: &now,
    };

    let result = governance::record_signal(&mut conn, &new_signal)?;
    Ok(response::created(&GovernanceSignalView::from(result)))
}

// GET /api/v1/governance/signals?entityType=X&entityId=Y — Query signals
(&Method::GET, "/signals") => {
    #[derive(Deserialize, Default)]
    #[serde(rename_all = "camelCase")]
    struct SignalQuery {
        entity_type: Option<String>,
        entity_id: Option<String>,
    }
    let params: SignalQuery = serde_urlencoded::from_str(query_str).unwrap_or_default();

    let entity_type = params.entity_type
        .ok_or_else(|| StorageError::InvalidInput("entityType required".to_string()))?;
    let entity_id = params.entity_id
        .ok_or_else(|| StorageError::InvalidInput("entityId required".to_string()))?;

    let mut conn = get_conn(pool)?;
    let results = governance::query_signals(&mut conn, &entity_type, &entity_id)?;
    let views: Vec<GovernanceSignalView> = results.into_iter().map(Into::into).collect();
    Ok(response::ok(&views))
}
```

**Step 5: Add route declarations to http.rs**

Add to the governance route block in `http.rs`:

```rust
.route(
    Route::post("/api/v1/governance/proposals/{id}/options")
        .handler("create_proposal_options")
        .build(),
)
.route(
    Route::get("/api/v1/governance/proposals/{id}/options")
        .handler("list_proposal_options")
        .cache_ttl(60)
        .build(),
)
.route(
    Route::post("/api/v1/governance/proposals/{id}/ranked-votes")
        .handler("cast_ranked_votes")
        .build(),
)
.route(
    Route::get("/api/v1/governance/proposals/{id}/ranked-votes")
        .handler("list_ranked_votes")
        .cache_ttl(30)
        .build(),
)
.route(
    Route::get("/api/v1/governance/proposals/{id}/tally")
        .handler("compute_tally")
        .cache_ttl(10)
        .build(),
)
.route(
    Route::post("/api/v1/governance/signals")
        .handler("record_governance_signal")
        .build(),
)
.route(
    Route::get("/api/v1/governance/signals")
        .handler("query_governance_signals")
        .cache_ttl(60)
        .build(),
)
```

**Step 6: Verify compilation**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -5`

**Step 7: Run all tests**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test 2>&1 | tail -10`

**Step 8: Commit**

```bash
git add elohim/elohim-storage/src/api/governance.rs elohim/elohim-storage/src/http.rs
git commit -m "feat(storage): add HTTP routes for multi-mechanism voting, tally, and governance signals"
```

---

### Task 10: Generate TypeScript types

**Files:**
- Modify: `elohim/sdk/storage-client-ts/src/generated/index.ts`
- Create: Multiple `.ts` files in `elohim/sdk/storage-client-ts/src/generated/`

**Step 1: Run ts-rs export**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test export_bindings 2>&1 | tail -5`
Expected: TypeScript types generated to `../../sdk/storage-client-ts/src/generated/`

**Step 2: Add new exports to index.ts**

Add to `elohim/sdk/storage-client-ts/src/generated/index.ts` in alphabetical order:

```typescript
export * from './BallotEntry';
export * from './CastRankedVoteInputView';
export * from './CreateProposalOptionInputView';
export * from './GovernanceSignalView';
export * from './OptionResult';
export * from './ProposalOptionView';
export * from './RankedVoteView';
export * from './RecordSignalInputView';
export * from './TallyResult';
export * from './TallyRound';
```

**Step 3: Verify TypeScript compilation**

Run: `cd elohim/sdk/storage-client-ts && npx tsc --noEmit 2>&1 | tail -5`

**Step 4: Commit**

```bash
git add elohim/sdk/storage-client-ts/src/generated/
git commit -m "chore: regenerate TypeScript types with multi-mechanism voting types"
```

---

### Task 11: A2O scenarios — multi-mechanism voting

**Files:**
- Modify: `genesis/a2o/features/qahal/collective-governance.feature`

**Step 1: Add scenarios**

Append to the existing feature file:

```gherkin
  Scenario: Community uses ranked-choice to pick a curriculum path
    Given a collective "homeschool-coop" has an active proposal "Which history curriculum?"
    And the proposal uses "ranked-choice" voting with 3 options
      | option            |
      | Story of the World |
      | History Odyssey    |
      | Classical Conversations |
    When member "sarah" ranks her preferences
      | rank | option            |
      | 1    | History Odyssey    |
      | 2    | Story of the World |
      | 3    | Classical Conversations |
    And member "james" ranks his preferences
      | rank | option            |
      | 1    | Story of the World |
      | 2    | History Odyssey    |
      | 3    | Classical Conversations |
    Then the tally shows round-by-round elimination results
    And the winning option is displayed with the elohim's justification

  Scenario: Stewards score competing content revisions
    Given content "intro-to-fractions" has 2 proposed revisions
    And the proposal uses "score-vote" voting with range 1 to 10
    When 3 stewards score each revision independently
    Then the revision with the highest total score is recommended
    And each steward's reasoning is visible to the others

  Scenario: Dot-voting allocates limited attention across proposals
    Given a collective has 5 pending proposals
    And each member gets 10 dots to distribute
    When member "maria" allocates dots across proposals
      | dots | proposal                    |
      | 5    | Add music theory path       |
      | 3    | Update science curriculum   |
      | 2    | Create art history module   |
    Then the proposals are ranked by total dots received
    And proposals with zero dots are deprioritized

  Scenario: Consent round with escalation on block
    Given a proposal "Restructure reading groups" is in consent round
    When all members consent except "david" who blocks
    And "david" provides justification "This eliminates the only group for struggling readers"
    Then the block triggers an elohim-facilitated conversation
    And the elohim engages with david's concern before proceeding
    And the block is recorded in the settlement log regardless of outcome

  Scenario: Elohim selects feedback mechanism based on content context
    Given content "manifesto-foundations" has governance state "constitutional"
    When a learner views the content
    Then only reasoned dissent is available via the context menu
    And no low-friction reactions are shown
    But the learner can still flag, challenge, or provide open feedback
```

**Step 2: Commit**

```bash
git add genesis/a2o/features/qahal/collective-governance.feature
git commit -m "feat(a2o): add multi-mechanism voting scenarios — ranked-choice, score, dot, consent, mechanism selection"
```

---

## Summary

| Task | What | Files |
|------|------|-------|
| 1 | proposal_options table + voting_mechanism column | migration up/down, diesel_schema |
| 2 | ranked_votes + governance_signals tables | migration up/down, diesel_schema |
| 3 | Diesel models (ProposalOption, RankedVote, GovernanceSignal) | models.rs |
| 4 | Update Proposal model + views for new columns | models.rs, views.rs, api/governance.rs |
| 5 | View types (ProposalOptionView, RankedVoteView, etc.) | views.rs |
| 6 | CRUD functions for all 3 new tables | db/governance.rs |
| 7 | TallyStrategy trait + 6 implementations | tally/ module (7 files) |
| 8 | Tally unit tests | tally/tests.rs |
| 9 | HTTP routes (options, ranked-votes, tally, signals) | api/governance.rs, http.rs |
| 10 | Generate TypeScript types | storage-client-ts/generated/ |
| 11 | A2O scenarios | collective-governance.feature |
