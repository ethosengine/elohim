# Qahal (Mishpat DNA) Wire Types Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create `qahal-types` crate at `elohim/sdk/domains/qahal/types/` with wire types for the mishpat DNA coordinator, then wire it into the mishpat zome.

**Architecture:** Same pattern as imagodei-types (see `elohim/sdk/domains/CLAUDE.md`). The mishpat DNA handles governance: challenges, proposals, precedents, discussions, voting, reactions, and graduated feedback. No doorway consumer exists — these types flow through elohim-storage's route registry, not hand-copied in doorway.

**Tech Stack:** Rust, serde, holo_hash (=0.6.0), rmp-serde

**Parallel Safety:** This plan touches only the mishpat DNA. Safe to run in parallel with all other domain type plans.

---

## File Map

| File | Action | Purpose |
|------|--------|---------|
| `elohim/sdk/domains/qahal/types/Cargo.toml` | Create | Crate manifest |
| `elohim/sdk/domains/qahal/types/src/lib.rs` | Create | Wire type definitions |
| `elohim/holochain/dna/mishpat/zomes/mishpat/Cargo.toml` | Modify | Depend on qahal-types |
| `elohim/holochain/dna/mishpat/zomes/mishpat/src/lib.rs` | Modify | Replace local types with re-exports |

---

### Task 1: Create qahal-types crate

**Files:**
- Create: `elohim/sdk/domains/qahal/types/Cargo.toml`
- Create: `elohim/sdk/domains/qahal/types/src/lib.rs`

- [ ] **Step 1: Create Cargo.toml**

Create `elohim/sdk/domains/qahal/types/Cargo.toml`:

```toml
[package]
name = "qahal-types"
version = "0.1.0"
edition = "2021"
description = "Wire types for qahal (governance) domain coordinator functions"

[dependencies]
holo_hash = { version = "=0.6.0", features = ["encoding"] }
serde = { version = "1", features = ["derive"] }

[dev-dependencies]
rmp-serde = "1"

[features]
default = []
ts = ["dep:ts-rs"]

[dependencies.ts-rs]
version = "10"
optional = true
```

- [ ] **Step 2: Create src/lib.rs with wire types**

Read the mishpat coordinator zome at `elohim/holochain/dna/mishpat/zomes/mishpat/src/lib.rs` and the integrity zome at `elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs`.

Extract ALL input/output struct definitions. The mishpat DNA has these type groups:

**Governance entities** (integrity entry mirrors + coordinator I/O):
- Challenge: `CreateChallengeInput`, `ChallengeOutput`, `QueryChallengesInput`, plus wire `Challenge`
- Proposal: `CreateProposalInput`, `ProposalOutput`, `QueryProposalsInput`, plus wire `Proposal`
- Precedent: `CreatePrecedentInput`, `PrecedentOutput`, `QueryPrecedentsInput`, plus wire `Precedent`
- Discussion: `CreateDiscussionInput`, `DiscussionOutput`, `QueryDiscussionsInput`, plus wire `Discussion`
- GovernanceState: `CreateGovernanceStateInput`, `GovernanceStateOutput`, `GetGovernanceStateInput`, `QueryGovernanceStatesInput`, plus wire `GovernanceState`
- GovernanceReaction: `CreateGovernanceReactionInput`, `GovernanceReactionOutput`, `QueryGovernanceReactionsInput`, plus wire `GovernanceReaction`

**Feedback & Voting**:
- GraduatedFeedback: `CreateGraduatedFeedbackInput`, `GraduatedFeedbackOutput`, `QueryGraduatedFeedbackInput`, plus wire `GraduatedFeedback`
- ProposalVote: `CreateProposalVoteInput`, `ProposalVoteOutput`, `QueryProposalVotesInput`, plus wire `ProposalVote`
- OpinionStatement: `CreateOpinionStatementInput`, `OpinionStatementOutput`, `QueryOpinionStatementsInput`, plus wire `OpinionStatement`
- StatementVote: `CreateStatementVoteInput`, `StatementVoteOutput`, `QueryStatementVotesInput`, plus wire `StatementVote`

**Credential verification**:
- `CredentialVerification`, `VerificationStatus`

For each type:
- Copy the struct definition from the zome
- Replace `#[derive(Serialize, Deserialize, Debug)]` with `#[derive(Debug, Clone, Serialize, Deserialize)]`
- Add `#[cfg_attr(feature = "ts", derive(ts_rs::TS))]`
- Add `#[serde(default, skip_serializing_if = "Option::is_none")]` on all `Option<T>` fields
- For wire mirrors of integrity entry types, copy all fields but do NOT include `#[hdk_entry_helper]`

Organize with module comments:

```rust
// =============================================================================
// Challenge Types
// =============================================================================

// ... CreateChallengeInput, Challenge, ChallengeOutput, QueryChallengesInput

// =============================================================================
// Proposal Types
// =============================================================================

// ... etc
```

Add one MessagePack roundtrip test per Create*Input type (10 tests total).

- [ ] **Step 3: Verify the crate builds and tests pass**

```bash
cd elohim/sdk/domains/qahal/types && cargo check && cargo test
```

- [ ] **Step 4: Commit**

```bash
git add elohim/sdk/domains/qahal/types/
git commit -m "feat(qahal): create wire types crate in sdk/domains/qahal/types/

Wire types for governance: challenges, proposals, precedents, discussions,
voting, reactions, and graduated feedback. Zero HDK deps."
```

---

### Task 2: Wire mishpat coordinator zome to use qahal-types

**Files:**
- Modify: `elohim/holochain/dna/mishpat/zomes/mishpat/Cargo.toml`
- Modify: `elohim/holochain/dna/mishpat/zomes/mishpat/src/lib.rs`

- [ ] **Step 1: Read the zome's current lib.rs in full**

- [ ] **Step 2: Add qahal-types dependency**

In `elohim/holochain/dna/mishpat/zomes/mishpat/Cargo.toml`, add:

```toml
qahal-types = { path = "../../../../../sdk/domains/qahal/types" }
```

Path: `zomes/mishpat/` → `dna/mishpat/` → `holochain/` → `elohim/` → `sdk/domains/qahal/types/`

- [ ] **Step 3: Replace local type definitions with re-exports**

Replace every locally-defined input/output/query struct with `pub use qahal_types::*;` or individual re-exports. Keep integrity entry types (they use `#[hdk_entry_helper]` via `mishpat_integrity`).

- [ ] **Step 4: Fix construction sites**

At each site where an output struct wraps an integrity entry type, convert field-by-field to the wire type from qahal-types.

```bash
grep -n 'Output {' elohim/holochain/dna/mishpat/zomes/mishpat/src/lib.rs
```

- [ ] **Step 5: Verify zome builds for WASM target**

```bash
cd elohim/holochain/dna/mishpat
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check --target wasm32-unknown-unknown -p mishpat 2>&1 | tail -10
```

- [ ] **Step 6: cargo fmt**

```bash
cd elohim/holochain/dna/mishpat/zomes/mishpat && cargo fmt
```

- [ ] **Step 7: Commit**

```bash
git add elohim/holochain/dna/mishpat/zomes/mishpat/
git commit -m "refactor(mishpat): use sdk/domains/qahal/types for wire types in zome

All input/output types re-exported from qahal-types crate.
Zome converts between integrity and wire types at construction sites."
```

---

### Task 3: Final verification

- [ ] **Step 1: Build and test**

```bash
cd elohim/sdk/domains/qahal/types && cargo test
cd elohim/holochain/dna/mishpat && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check --target wasm32-unknown-unknown
```

- [ ] **Step 2: cargo fmt check**

```bash
cd elohim/sdk/domains/qahal/types && cargo fmt --check
cd elohim/holochain/dna/mishpat/zomes/mishpat && cargo fmt --check
```
