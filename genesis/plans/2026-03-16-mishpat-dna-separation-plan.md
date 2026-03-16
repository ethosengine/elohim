# Mishpat DNA Separation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create a new Holochain DNA named Mishpat for formal governance, moving 10 entry types, ~33 link types, and ~15 coordinator functions from the lamad DNA. Lamad drops from 83→73 entry types.

**Architecture:** Create mishpat integrity + coordinator zomes. Copy governance entry types, link types, constants, and coordinator functions from lamad. Remove them from lamad. Update hApp manifest and CI build. Update documentation with new DNA capacity numbers.

**Tech Stack:** Holochain HDK/HDI 0.6/0.7, Rust WASM (wasm32-unknown-unknown), hc CLI

---

### Task 1: Create Mishpat DNA Directory Structure + Cargo.toml files

**Files:**
- Create: `elohim/holochain/dna/mishpat/dna.yaml`
- Create: `elohim/holochain/dna/mishpat/Cargo.toml`
- Create: `elohim/holochain/dna/mishpat/zomes/mishpat_integrity/Cargo.toml`
- Create: `elohim/holochain/dna/mishpat/zomes/mishpat/Cargo.toml`

**Step 1: Create directory structure**

```bash
mkdir -p elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src
mkdir -p elohim/holochain/dna/mishpat/zomes/mishpat/src
mkdir -p elohim/holochain/dna/mishpat/workdir
```

**Step 2: Write dna.yaml**

```yaml
---
manifest_version: "0"
name: mishpat
integrity:
  network_seed: ~
  properties: ~
  zomes:
    - name: mishpat_integrity
      path: "target/wasm32-unknown-unknown/release/mishpat_integrity.wasm"

coordinator:
  zomes:
    - name: mishpat
      path: "target/wasm32-unknown-unknown/release/mishpat.wasm"
      dependencies:
        - name: mishpat_integrity
```

**Step 3: Write workspace Cargo.toml**

```toml
[workspace]
members = ["zomes/mishpat_integrity", "zomes/mishpat"]
resolver = "2"

[workspace.dependencies]
hdi = "0.7"
hdk = "0.6"
holochain_serialized_bytes = "0.0.55"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

Note: Check the existing elohim DNA `Cargo.toml` for exact version pins. If it uses workspace inheritance from a parent, match that pattern instead.

**Step 4: Write integrity zome Cargo.toml**

```toml
[package]
name = "mishpat_integrity"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]
name = "mishpat_integrity"

[dependencies]
hdi.workspace = true
holochain_serialized_bytes.workspace = true
serde.workspace = true
```

**Step 5: Write coordinator zome Cargo.toml**

```toml
[package]
name = "mishpat"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]
name = "mishpat"

[dependencies]
hdk.workspace = true
holochain_serialized_bytes.workspace = true
serde.workspace = true
serde_json.workspace = true
mishpat_integrity = { path = "../mishpat_integrity" }
```

**Step 6: Commit**

```bash
git add elohim/holochain/dna/mishpat/
git commit -m "feat(mishpat): create DNA directory structure and Cargo.toml files"
```

---

### Task 2: Write Mishpat Integrity Zome (entry types + link types)

**Files:**
- Create: `elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs`

**Step 1: Write the integrity zome**

This file contains all 10 governance entry types, their constants, and all governance link types. Copy the exact struct definitions from the lamad DNA (content_store_integrity/src/lib.rs) — they use `#[hdk_entry_helper]` and `#[derive(Clone, PartialEq)]`.

The file structure should be:

```rust
use hdi::prelude::*;

// ============================================================
// ENTRY TYPES — Formal Governance (Mishpat)
// ============================================================

// --- Challenge ---
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Challenge { /* exact copy from lamad lines 2915-2935 */ }

pub const CHALLENGE_GROUNDS: [&str; 5] = [/* exact copy */];
pub const CHALLENGE_STATUS: [&str; 5] = [/* exact copy */];

// --- Proposal ---
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Proposal { /* exact copy from lamad lines 2960-2979 */ }

pub const PROPOSAL_TYPES: [&str; 4] = [/* exact copy */];
pub const PROPOSAL_STATUS: [&str; 5] = [/* exact copy */];

// --- Precedent ---
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Precedent { /* exact copy from lamad lines 3003-3019 */ }

pub const PRECEDENT_BINDING: [&str; 4] = [/* exact copy */];

// --- Discussion ---
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Discussion { /* exact copy from lamad lines 3033-3048 */ }

pub const DISCUSSION_CATEGORIES: [&str; 4] = [/* exact copy */];

// --- GovernanceState ---
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct GovernanceState { /* exact copy from lamad lines 3061-3078 */ }

pub const GOVERNANCE_STATUS: [&str; 4] = [/* exact copy */];

// --- GovernanceReaction ---
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct GovernanceReaction { /* exact copy from lamad lines 3102-3117 */ }

pub const REACTION_TYPES: [&str; 6] = [/* exact copy */];

// --- GraduatedFeedback ---
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct GraduatedFeedback { /* exact copy from lamad lines 3122-3137 */ }

pub const FEEDBACK_CONTEXTS: [&str; 5] = [/* exact copy */];

// --- ProposalVote ---
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ProposalVote { /* exact copy from lamad lines 3142-3156 */ }

pub const VOTE_POSITIONS: [&str; 4] = [/* exact copy */];

// --- OpinionStatement ---
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct OpinionStatement { /* exact copy from lamad lines 3161-3176 */ }

// --- StatementVote ---
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct StatementVote { /* exact copy from lamad lines 3181-3190 */ }

pub const STATEMENT_VOTES: [&str; 3] = [/* exact copy */];


// ============================================================
// ENTRY TYPES ENUM
// ============================================================

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    Challenge(Challenge),
    Proposal(Proposal),
    Precedent(Precedent),
    Discussion(Discussion),
    GovernanceState(GovernanceState),
    GovernanceReaction(GovernanceReaction),
    GraduatedFeedback(GraduatedFeedback),
    ProposalVote(ProposalVote),
    OpinionStatement(OpinionStatement),
    StatementVote(StatementVote),
}


// ============================================================
// LINK TYPES
// ============================================================

#[hdk_link_types]
pub enum LinkTypes {
    // Governance Signals (Loomio/Forby/Polis patterns)
    ContentToReactions,
    AgentToReactions,
    ReactionByType,
    ContentToFeedback,
    AgentToFeedback,
    FeedbackByContext,
    ProposalToVotes,
    AgentToVotes,
    VoteByPosition,
    ContextToStatements,
    AgentToStatements,
    StatementToVotes,
    AgentToStatementVotes,
    ReactionToMediation,
    AgentToMediations,

    // Challenge
    IdToChallenge,
    EntityToChallenge,
    ChallengerToChallenge,
    ChallengeByStatus,

    // Proposal
    IdToProposal,
    ProposalByType,
    ProposerToProposal,
    ProposalByStatus,

    // Precedent
    IdToPrecedent,
    PrecedentByScope,
    PrecedentByStatus,

    // Discussion
    IdToDiscussion,
    EntityToDiscussion,
    DiscussionByCategory,
    DiscussionByStatus,

    // GovernanceState
    IdToGovernanceState,
    GovernanceStateByStatus,
}
```

**CRITICAL:** Copy the EXACT struct definitions from the lamad DNA. Every field, every type, every attribute must match exactly. Read the source file to get the precise content — do not paraphrase.

**Step 2: Verify compilation**

```bash
cd elohim/holochain/dna/mishpat
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check
```

If workspace dependencies don't resolve, check how the lamad DNA resolves them and match the pattern.

**Step 3: Commit**

```bash
git add elohim/holochain/dna/mishpat/zomes/mishpat_integrity/
git commit -m "feat(mishpat): integrity zome with 10 governance entry types and 33 link types"
```

---

### Task 3: Write Mishpat Coordinator Zome (CRUD functions + bridges)

**Files:**
- Create: `elohim/holochain/dna/mishpat/zomes/mishpat/src/lib.rs`

**Step 1: Write the coordinator zome**

Copy the governance coordinator functions from the lamad coordinator (`content_store/src/lib.rs`). The functions to copy are:

- Challenge: `create_challenge`, `get_challenge_by_id`, `query_challenges` (lines ~7633-7770)
- Proposal: `create_proposal`, `get_proposal_by_id`, `query_proposals` (lines ~7814-7960)
- Precedent: `create_precedent`, `get_precedent_by_id`, `query_precedents` (lines ~7990-8100)
- Discussion: `create_discussion`, `get_discussion_by_id`, `query_discussions` (lines ~8131-8280)
- GovernanceState: `set_governance_state`, `get_governance_state`, `query_governance_states` (lines ~8304-8430)

Also copy any governance signal functions (create_reaction, create_vote, etc.) from the same coordinator.

Update imports to use `mishpat_integrity::*` instead of `content_store_integrity::*`.

The file structure:

```rust
use hdk::prelude::*;
use mishpat_integrity::*;

// Input/Output types for coordinator functions
// (copy from lamad coordinator, these are the Create*Input and *Output types)

// ============================================================
// CHALLENGE FUNCTIONS
// ============================================================

#[hdk_extern]
pub fn create_challenge(input: CreateChallengeInput) -> ExternResult<ChallengeOutput> {
    // exact copy from lamad coordinator
}

// ... etc for all governance functions

// ============================================================
// CROSS-DNA BRIDGES
// ============================================================

// Bridge to imagodei for identity verification
fn verify_human(human_id: &str) -> ExternResult<bool> {
    let response: ZomeCallResponse = call(
        CallTargetCell::OtherRole("imagodei".into()),
        "imagodei",
        "get_human_by_id".into(),
        None,
        human_id.to_string(),
    )?;
    match response {
        ZomeCallResponse::Ok(_) => Ok(true),
        _ => Ok(false),
    }
}
```

**CRITICAL:** Read the exact coordinator functions from lamad before writing. The input/output types, link creation patterns, and error handling must match exactly.

**Step 2: Verify compilation**

```bash
cd elohim/holochain/dna/mishpat
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check
```

**Step 3: Commit**

```bash
git add elohim/holochain/dna/mishpat/zomes/mishpat/
git commit -m "feat(mishpat): coordinator zome with governance CRUD + cross-DNA bridges"
```

---

### Task 4: Remove Governance from Lamad DNA

**Files:**
- Modify: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs`
- Modify: `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs`

**Step 1: Remove governance entry types from lamad integrity zome**

In `content_store_integrity/src/lib.rs`:

1. Remove the struct definitions for: Challenge, Proposal, Precedent, Discussion, GovernanceState, GovernanceReaction, GraduatedFeedback, ProposalVote, OpinionStatement, StatementVote
2. Remove the associated constants (CHALLENGE_GROUNDS, CHALLENGE_STATUS, PROPOSAL_TYPES, etc.)
3. Remove these variants from the `EntryTypes` enum
4. Remove governance link types from the `LinkTypes` enum

**DO NOT REMOVE:** MasteryChallenge (learning construct), LearningSignal (lamad concern), MediationLog (may be shared — check references).

**Step 2: Remove governance coordinator functions from lamad coordinator**

In `content_store/src/lib.rs`:

Remove the `#[hdk_extern]` functions for: create_challenge, get_challenge_by_id, query_challenges, create_proposal, get_proposal_by_id, query_proposals, create_precedent, get_precedent_by_id, query_precedents, create_discussion, get_discussion_by_id, query_discussions, set_governance_state, get_governance_state, query_governance_states.

Also remove any governance signal coordinator functions (create_reaction, cast_vote, etc.).

Remove associated input/output types that were only used by these functions.

**Step 3: Verify lamad still compiles**

```bash
cd elohim/holochain/dna/elohim
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check
```

This is the critical step — removing entry types may break references in remaining lamad code. Fix any compilation errors.

**Step 4: Verify mishpat still compiles**

```bash
cd elohim/holochain/dna/mishpat
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check
```

**Step 5: Commit**

```bash
git add elohim/holochain/dna/elohim/
git commit -m "refactor(lamad): remove 10 governance entry types and ~33 link types — moved to mishpat DNA

Lamad drops from 83 to ~73 entry types. MasteryChallenge stays (learning construct)."
```

---

### Task 5: Update hApp Manifest + CI Build

**Files:**
- Modify: `elohim/holochain/dna/elohim/workdir/happ.yaml` (or wherever the hApp manifest lives)
- Modify: `elohim/holochain/dna/Jenkinsfile`

**Step 1: Find and update hApp manifest**

Search for the hApp manifest:
```bash
find elohim/holochain -name "happ.yaml" -o -name "happ.yml"
```

Add a mishpat role after imagodei:
```yaml
  - name: mishpat
    provisioning:
      strategy: create
      deferred: false
    dna:
      path: "mishpat.dna"
```

**Step 2: Update DNA Jenkinsfile**

In `elohim/holochain/dna/Jenkinsfile`, find where the existing DNAs are built and add mishpat. Follow the exact same pattern as imagodei/infrastructure builds:

```groovy
// Mishpat (governance)
dir('elohim/holochain/dna/mishpat') {
    sh 'cargo build --release --target wasm32-unknown-unknown'
    sh 'hc dna pack . -o ../elohim/workdir/mishpat.dna'
}
```

**Step 3: Update orchestrator changeset patterns**

In `genesis/orchestrator/Jenkinsfile`, find the DNA pipeline's change patterns and add `elohim/holochain/dna/mishpat/`.

**Step 4: Commit**

```bash
git add elohim/holochain/ genesis/orchestrator/Jenkinsfile
git commit -m "ci(mishpat): add mishpat DNA to hApp manifest and CI build pipeline"
```

---

### Task 6: Update Documentation + Enforcement Infrastructure

**Files:**
- Modify: `CLAUDE.md`
- Modify: `.claude/skills/p2p-design-gate/SKILL.md`
- Modify: `.claude/agents/rust-architect.md`

**Step 1: Update CLAUDE.md architecture section**

Add mishpat to the DNA list in the Architecture section. Update entry type counts.

**Step 2: Update p2p-design-gate skill capacity table**

Change the DHT Capacity Constraints table:
```
| Lamad | 83 → ~73 | Comfortable (freed by mishpat split) |
| Mishpat | 10 entry types, ~33 link types | Wide open |
```

**Step 3: Update rust-architect agent**

Add mishpat DNA to the DNA listing and the decision tree.

**Step 4: Update memory**

Update `project-dht-capacity-constraints.md` with new numbers.

**Step 5: Update Sprint 4 migration comments**

The governance migration `2026-03-16-300000_qahal_provenance/up.sql` references "lamad DNA" for governance entries. Update to reference "mishpat DNA".

**Step 6: Commit**

```bash
git add CLAUDE.md .claude/ genesis/plans/
git commit -m "docs: update capacity tables and architecture docs for mishpat DNA separation"
```

---

### Task 7: Build Verification

**Step 1: Build mishpat WASM**

```bash
cd elohim/holochain/dna/mishpat
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release --target wasm32-unknown-unknown
```

Expected: Two WASM files in `target/wasm32-unknown-unknown/release/`:
- `mishpat_integrity.wasm`
- `mishpat.wasm`

**Step 2: Build lamad WASM (verify nothing broke)**

```bash
cd elohim/holochain/dna/elohim
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release --target wasm32-unknown-unknown
```

**Step 3: Build imagodei WASM (verify no regressions)**

```bash
cd elohim/holochain/dna/imagodei
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release --target wasm32-unknown-unknown
```

**Step 4: Storage compilation check**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check
```

**Step 5: Storage tests**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib
```

**Step 6: Commit build outputs if needed**

```bash
git add -A
git commit -m "chore(mishpat): build verification — all DNAs compile, storage tests pass"
```
