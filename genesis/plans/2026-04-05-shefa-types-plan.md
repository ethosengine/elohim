# Shefa Wire Types Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create `shefa-types` crate at `elohim/sdk/domains/shefa/types/` with wire types for the economics subset of the elohim DNA's content_store coordinator.

**Architecture:** Same pattern as imagodei-types (see `elohim/sdk/domains/CLAUDE.md`). The content_store coordinator in the elohim DNA includes REA (Resource-Event-Agent) economic types. This crate extracts them: agreements, commitments, economic events, premium gating, steward credentials, and custodian commitments.

**Tech Stack:** Rust, serde, holo_hash (=0.6.0), rmp-serde

**Parallel Safety:** This plan adds a dependency to the elohim DNA's content_store zome Cargo.toml. If lamad-types or avodah-types plans run in parallel, the Cargo.toml edits must be coordinated. The src/lib.rs changes target different structs and don't conflict.

---

## File Map

| File | Action | Purpose |
|------|--------|---------|
| `elohim/sdk/domains/shefa/types/Cargo.toml` | Create | Crate manifest |
| `elohim/sdk/domains/shefa/types/src/lib.rs` | Create | Wire type definitions |
| `elohim/holochain/dna/elohim/zomes/content_store/Cargo.toml` | Modify | Depend on shefa-types |
| `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` | Modify | Replace local types with re-exports |

---

### Task 1: Create shefa-types crate

**Files:**
- Create: `elohim/sdk/domains/shefa/types/Cargo.toml`
- Create: `elohim/sdk/domains/shefa/types/src/lib.rs`

- [ ] **Step 1: Create Cargo.toml**

Create `elohim/sdk/domains/shefa/types/Cargo.toml`:

```toml
[package]
name = "shefa-types"
version = "0.1.0"
edition = "2021"
description = "Wire types for shefa (economics) domain coordinator functions"

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

Read the content_store coordinator zome at `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` and the integrity zome.

Extract ONLY the shefa-relevant types:

**REA Economics:**
- `CreateAgreementInput`, `AgreementOutput` + wire `Agreement`
- `CreateReaCommitmentInput`, `ReaCommitmentOutput` + wire `ReaCommitment`
- `CreateReaEconomicEventInput`, `ReaEconomicEventOutput` + wire `ReaEconomicEvent`
- Query types for each

**Premium Gating:**
- `CreatePremiumGateInput`, `PremiumGateOutput` + wire `PremiumGate`
- `GrantAccessInput`, `CheckAccessInput`
- Query types

**Steward Credentials:**
- `CreateStewardCredentialInput`, `StewardCredentialOutput` + wire `StewardCredential`
- `ContributorDashboardOutput`
- Revenue summary types

**Custodian Commitments:**
- `CreateCustodianCommitmentInput`, `CustodianCommitmentOutput` + wire `CustodianCommitment`
- `AcceptCustodianCommitmentInput`, `BatchAcceptCommitmentsInput`, `BatchUpdateCommitmentsInput`
- Query types

Follow the standard pattern for derives, serde attributes, and ts-rs.

Add one MessagePack roundtrip test per major Create*Input type.

**DO NOT include** lamad types (Content, Path, Step) or avodah types (ServiceRequest, FlowPlan).

- [ ] **Step 3: Verify the crate builds and tests pass**

```bash
cd elohim/sdk/domains/shefa/types && cargo check && cargo test
```

- [ ] **Step 4: Commit**

```bash
git add elohim/sdk/domains/shefa/types/
git commit -m "feat(shefa): create wire types crate in sdk/domains/shefa/types/

Wire types for REA economics: agreements, commitments, economic events,
premium gating, steward credentials, custodian commitments. Zero HDK deps."
```

---

### Task 2: Wire content_store coordinator zome to use shefa-types

**Files:**
- Modify: `elohim/holochain/dna/elohim/zomes/content_store/Cargo.toml`
- Modify: `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs`

- [ ] **Step 1: Add shefa-types dependency**

In `elohim/holochain/dna/elohim/zomes/content_store/Cargo.toml`, add:

```toml
shefa-types = { path = "../../../../../sdk/domains/shefa/types" }
```

- [ ] **Step 2: Replace local type definitions with re-exports**

Replace locally-defined shefa input/output/query structs with re-exports from `shefa_types`. Leave lamad and avodah types untouched.

- [ ] **Step 3: Fix construction sites**

Convert integrity entry types to wire types at each construction site.

- [ ] **Step 4: Verify zome builds**

```bash
cd elohim/holochain/dna/elohim
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check --target wasm32-unknown-unknown -p content_store 2>&1 | tail -10
```

- [ ] **Step 5: cargo fmt and commit**

```bash
cd elohim/holochain/dna/elohim/zomes/content_store && cargo fmt
git add elohim/holochain/dna/elohim/zomes/content_store/
git commit -m "refactor(content_store): use sdk/domains/shefa/types for economics wire types

Shefa input/output types re-exported from shefa-types crate."
```

---

### Task 3: Final verification

- [ ] **Step 1: Build and test**

```bash
cd elohim/sdk/domains/shefa/types && cargo test
cd elohim/holochain/dna/elohim && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check --target wasm32-unknown-unknown
```
