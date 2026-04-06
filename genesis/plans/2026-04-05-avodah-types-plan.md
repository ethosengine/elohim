# Avodah Wire Types Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create `avodah-types` crate at `elohim/sdk/domains/avodah/types/` with wire types for the work/services subset of the elohim DNA's content_store coordinator.

**Architecture:** Same pattern as imagodei-types (see `elohim/sdk/domains/CLAUDE.md`). The content_store coordinator includes service marketplace, flow planning, and insurance types. This is the smallest domain extraction.

**Tech Stack:** Rust, serde, holo_hash (=0.6.0), rmp-serde

**Parallel Safety:** This plan adds a dependency to the elohim DNA's content_store zome Cargo.toml. If lamad-types or shefa-types plans run in parallel, the Cargo.toml edits must be coordinated. The src/lib.rs changes target different structs.

---

## File Map

| File | Action | Purpose |
|------|--------|---------|
| `elohim/sdk/domains/avodah/types/Cargo.toml` | Create | Crate manifest |
| `elohim/sdk/domains/avodah/types/src/lib.rs` | Create | Wire type definitions |
| `elohim/holochain/dna/elohim/zomes/content_store/Cargo.toml` | Modify | Depend on avodah-types |
| `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` | Modify | Replace local types with re-exports |

---

### Task 1: Create avodah-types crate

**Files:**
- Create: `elohim/sdk/domains/avodah/types/Cargo.toml`
- Create: `elohim/sdk/domains/avodah/types/src/lib.rs`

- [ ] **Step 1: Create Cargo.toml**

Create `elohim/sdk/domains/avodah/types/Cargo.toml`:

```toml
[package]
name = "avodah-types"
version = "0.1.0"
edition = "2021"
description = "Wire types for avodah (work/services) domain coordinator functions"

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

Extract ONLY the avodah-relevant types:

**Service Marketplace:**
- `CreateServiceRequestInput`, `ServiceRequestOutput` + wire `ServiceRequest`
- `CreateServiceOfferInput`, `ServiceOfferOutput` + wire `ServiceOffer`
- `CreateServiceMatchInput`, `ServiceMatchOutput` + wire `ServiceMatch`
- Query types for each

**Flow Planning:**
- `CreateFlowPlanInput`, `FlowPlanOutput` + wire `FlowPlan`
- `CreateFlowBudgetInput`, `CreateFlowGoalInput`, `CreateFlowMilestoneInput`
- `CreateFlowScenarioInput`, `CreateFlowProjectionInput`, `CreateRecurringPatternInput`
- Query types

**Insurance:**
- `CreateMemberRiskProfileInput`, `MemberRiskProfileOutput` + wire `MemberRiskProfile`
- `CreateCoveragePolicyInput`, `CoveragePolicyOutput` + wire `CoveragePolicy`
- `CreateInsuranceClaimInput`, `InsuranceClaimOutput` + wire `InsuranceClaim`
- `CreateAdjustmentReasoningInput`, `AdjustmentReasoningOutput` + wire `AdjustmentReasoning`

Follow the standard pattern for derives, serde attributes, and ts-rs.

Add one MessagePack roundtrip test per major Create*Input type.

**DO NOT include** lamad types (Content, Path) or shefa types (Agreement, Commitment).

- [ ] **Step 3: Verify the crate builds and tests pass**

```bash
cd elohim/sdk/domains/avodah/types && cargo check && cargo test
```

- [ ] **Step 4: Commit**

```bash
git add elohim/sdk/domains/avodah/types/
git commit -m "feat(avodah): create wire types crate in sdk/domains/avodah/types/

Wire types for service marketplace, flow planning, and insurance.
Zero HDK deps."
```

---

### Task 2: Wire content_store coordinator zome to use avodah-types

**Files:**
- Modify: `elohim/holochain/dna/elohim/zomes/content_store/Cargo.toml`
- Modify: `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs`

- [ ] **Step 1: Add avodah-types dependency**

In `elohim/holochain/dna/elohim/zomes/content_store/Cargo.toml`, add:

```toml
avodah-types = { path = "../../../../../sdk/domains/avodah/types" }
```

- [ ] **Step 2: Replace local type definitions with re-exports**

Replace locally-defined avodah input/output/query structs with re-exports from `avodah_types`.

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
git commit -m "refactor(content_store): use sdk/domains/avodah/types for work wire types

Avodah input/output types re-exported from avodah-types crate."
```

---

### Task 3: Final verification

- [ ] **Step 1: Build and test**

```bash
cd elohim/sdk/domains/avodah/types && cargo test
cd elohim/holochain/dna/elohim && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check --target wasm32-unknown-unknown
```
