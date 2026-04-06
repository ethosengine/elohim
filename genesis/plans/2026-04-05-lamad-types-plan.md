# Lamad Wire Types Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create `lamad-types` crate at `elohim/sdk/domains/lamad/types/` with wire types for the learning/content subset of the elohim DNA's content_store coordinator.

**Architecture:** Same pattern as imagodei-types (see `elohim/sdk/domains/CLAUDE.md`). The content_store coordinator in the elohim DNA serves multiple domains. This crate extracts the lamad (learning) types: content CRUD, paths, steps, chapters, relationships, progress, mastery, knowledge maps, and contributor operations. No doorway consumer — these flow through elohim-storage's route registry.

**Tech Stack:** Rust, serde, holo_hash (=0.6.0), rmp-serde

**Parallel Safety:** This plan touches the elohim DNA's content_store zome Cargo.toml (adds a dependency). If running in parallel with shefa-types or avodah-types plans, the Cargo.toml edits must be coordinated — each plan adds its own dependency line. The src/lib.rs changes are to different struct definitions and don't conflict.

---

## File Map

| File | Action | Purpose |
|------|--------|---------|
| `elohim/sdk/domains/lamad/types/Cargo.toml` | Create | Crate manifest |
| `elohim/sdk/domains/lamad/types/src/lib.rs` | Create | Wire type definitions |
| `elohim/holochain/dna/elohim/zomes/content_store/Cargo.toml` | Modify | Depend on lamad-types |
| `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` | Modify | Replace local types with re-exports |

---

### Task 1: Create lamad-types crate

**Files:**
- Create: `elohim/sdk/domains/lamad/types/Cargo.toml`
- Create: `elohim/sdk/domains/lamad/types/src/lib.rs`

- [ ] **Step 1: Create Cargo.toml**

Create `elohim/sdk/domains/lamad/types/Cargo.toml`:

```toml
[package]
name = "lamad-types"
version = "0.1.0"
edition = "2021"
description = "Wire types for lamad (learning) domain coordinator functions"

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

Read the content_store coordinator zome at `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` and the integrity zome at `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs`.

Extract ONLY the lamad-relevant types. These are the content and learning types:

**Content CRUD:**
- `CreateContentInput`, `ContentOutput`, `BulkCreateContentInput`, `BulkCreateContentOutput`
- `QueryByIdInput`, `QueryByTypeInput`, `CheckIdsExistInput`, `CheckIdsExistOutput`
- `BatchGetContentInput`, `BatchGetContentOutput`
- `PaginatedByTypeInput`, `PaginatedByTagInput`, `PaginatedContentOutput`, `ContentStats`
- Wire `Content` (mirrors integrity entry)

**Content Relationships:**
- `CreateRelationshipInput`, `RelationshipOutput`, `GetRelationshipsInput`
- `QueryRelatedContentInput`, `ContentGraphNode`, `ContentGraph`
- Wire `Relationship` (mirrors integrity entry)

**Learning Paths:**
- `CreatePathInput`, `PathImportInput`, `PathImportStepInput`
- `AddPathStepInput`, `BatchAddPathStepsInput`
- `UpdatePathInput`, `PathWithStepsOutput`, `PathOverviewOutput`
- Wire `LearningPath`, `PathStep` (mirrors integrity entries)

**Chapters:**
- `CreateChapterInput`, `ChapterOutput`, `UpdateChapterInput`
- Wire `Chapter` (mirrors integrity entry)

**Progress & Mastery:**
- `StartPathProgressInput`, `CompleteStepInput`, `CompletePathInput`
- `PathProgressOutput`, `QueryProgressInput`
- `GrantAttestationInput`, `CheckAttestationInput`, `AttestationOutput`
- Wire `PathProgress`, `ContentAttestation` (mirrors integrity entries)

**Knowledge Maps:**
- `CreateKnowledgeMapInput`, `KnowledgeMapOutput`, `QueryKnowledgeMapInput`
- Wire `KnowledgeMap` (mirrors integrity entry)

**Path Extensions:**
- `CreatePathExtensionInput`, `PathExtensionOutput`, `QueryPathExtensionInput`
- Wire `PathExtension` (mirrors integrity entry)

**Import:**
- `QueueImportInput`, `ImportChunkInput`, `ImportStatusOutput`

For each type, follow the standard pattern:
- `#[derive(Debug, Clone, Serialize, Deserialize)]`
- `#[cfg_attr(feature = "ts", derive(ts_rs::TS))]`
- `#[serde(default, skip_serializing_if = "Option::is_none")]` on `Option<T>` fields
- `#[serde(default)]` on `Vec<T>` fields that use `#[serde(default)]` in the zome

Organize with section comments matching the groups above.

Add one MessagePack roundtrip test per major Create*Input type.

**DO NOT include** shefa types (Agreement, Commitment, EconomicEvent), avodah types (ServiceRequest, FlowPlan, Insurance), or infrastructure types (Shard, ContentSuccession, CategoryOverride). Those belong in their respective domain crates.

- [ ] **Step 3: Verify the crate builds and tests pass**

```bash
cd elohim/sdk/domains/lamad/types && cargo check && cargo test
```

- [ ] **Step 4: Commit**

```bash
git add elohim/sdk/domains/lamad/types/
git commit -m "feat(lamad): create wire types crate in sdk/domains/lamad/types/

Wire types for content CRUD, paths, steps, chapters, relationships,
progress, mastery, knowledge maps, and import. Zero HDK deps."
```

---

### Task 2: Wire content_store coordinator zome to use lamad-types

**Files:**
- Modify: `elohim/holochain/dna/elohim/zomes/content_store/Cargo.toml`
- Modify: `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs`

- [ ] **Step 1: Read the zome's current lib.rs in full**

This file is very large (~3000+ lines). Read it in chunks. Identify ALL lamad-relevant struct definitions and their construction sites.

- [ ] **Step 2: Add lamad-types dependency**

In `elohim/holochain/dna/elohim/zomes/content_store/Cargo.toml`, add:

```toml
lamad-types = { path = "../../../../../sdk/domains/lamad/types" }
```

Path: `zomes/content_store/` → `dna/elohim/` → `holochain/` → `elohim/` → `sdk/domains/lamad/types/`

- [ ] **Step 3: Replace local type definitions with re-exports**

Replace locally-defined lamad input/output/query structs with re-exports from `lamad_types`. Leave shefa, avodah, and infrastructure types untouched.

- [ ] **Step 4: Fix construction sites**

At each site where an output struct wraps an integrity entry type (Content, LearningPath, PathStep, etc.), convert field-by-field to the wire type from lamad-types.

This zome has many construction sites due to its size. Use grep to find them all:

```bash
grep -n 'ContentOutput {' elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs | wc -l
grep -n 'PathWithStepsOutput {' elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs | wc -l
```

- [ ] **Step 5: Verify zome builds for WASM target**

```bash
cd elohim/holochain/dna/elohim
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check --target wasm32-unknown-unknown -p content_store 2>&1 | tail -10
```

- [ ] **Step 6: cargo fmt**

```bash
cd elohim/holochain/dna/elohim/zomes/content_store && cargo fmt
```

- [ ] **Step 7: Commit**

```bash
git add elohim/holochain/dna/elohim/zomes/content_store/
git commit -m "refactor(content_store): use sdk/domains/lamad/types for learning wire types

Lamad input/output types re-exported from lamad-types crate.
Zome converts between integrity and wire types at construction sites."
```

---

### Task 3: Final verification

- [ ] **Step 1: Build and test**

```bash
cd elohim/sdk/domains/lamad/types && cargo test
cd elohim/holochain/dna/elohim && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check --target wasm32-unknown-unknown
```

- [ ] **Step 2: cargo fmt check**

```bash
cd elohim/sdk/domains/lamad/types && cargo fmt --check
cd elohim/holochain/dna/elohim/zomes/content_store && cargo fmt --check
```
