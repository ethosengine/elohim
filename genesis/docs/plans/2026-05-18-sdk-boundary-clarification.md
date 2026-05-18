# SDK Boundary Clarification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move ts-rs-anchored View types from `elohim-storage/src/views.rs` into the dedicated `crates/elohim-sdk/` crate so consumers depend on a small, reviewable type surface; storage internals refactor freely behind the boundary. Add a `cargo-deny` rule that fails the build if `elohim-storage` becomes a direct dep of any non-server consumer.

**Architecture:** Today `crates/elohim-sdk/src/lib.rs` is 50 lines of skeleton with empty `client/cache/sync/traits/reach` subdirectories. Real SDK-surface types live in `elohim-storage/src/views.rs` (8,208 LOC), pulled in by ts-rs codegen for TypeScript consumers but Rust-visible to anyone who depends on `elohim-storage`. This plan inverts the dependency: `elohim-sdk` owns the View types (and the ts-rs anchor); `elohim-storage` becomes a `dep:elohim-sdk` consumer that implements them. Consumers (`doorway-service`, `steward/node`, `crates/elohim-storage-client`) depend on `elohim-sdk` only.

**Tech Stack:** Rust 2021 edition, ts-rs codegen, cargo-deny dependency-bans, the existing `cargo test export_bindings` workflow.

---

## Pre-execution gate (do once before Task 1)

- [ ] Confirm `crates/elohim-sdk/src/lib.rs` is still ~50 LOC: `wc -l /projects/elohim/crates/elohim-sdk/src/lib.rs`
- [ ] Confirm `elohim-storage/src/views.rs` is still ~8,208 LOC: `wc -l /projects/elohim/elohim/elohim-storage/src/views.rs`
- [ ] Confirm ts-rs export landing zone exists: `ls /projects/elohim/elohim/sdk/storage-client-ts/src/generated/ | head -20`
- [ ] Capture the ts-rs output baseline: `find /projects/elohim/elohim/sdk/storage-client-ts/src/generated -name '*.ts' | sort | xargs sha256sum | sort > /tmp/ts-rs-baseline.sha256`
- [ ] Note: the cargo registry T9 publish is still BLOCKED on Nexus auth (see `.claude/memory/feedback_nexus_cargo_publish_basic_auth.md`). This plan uses path-dependencies throughout. The publish step is a follow-up.

## File Structure

**Files to be created:**
```
crates/elohim-sdk/src/
├── views/
│   ├── mod.rs                # Re-export hub for all View types
│   ├── shared.rs             # Primitives used by 2+ domain modules (ViewSlice, Freshness, JsonValue wrapper, etc.)
│   ├── lamad.rs              # Content/Path/Mastery view types
│   ├── shefa.rs              # EconomicEvent/Stewardship/Collective view types
│   ├── qahal.rs              # GovernanceAction/Vote/Affinity view types
│   ├── imagodei.rs           # Human/Relationship/AgentPeerBinding view types
│   ├── infrastructure.rs     # P2P/Peer/Federation/Resilience view types
│   ├── epr.rs                # EprView/EprEnvelopeView/EprListView/EprProvidersView
│   └── inputs.rs             # *InputView types (write-path views)
└── ts_rs_anchor.rs           # Single file that re-imports everything for ts-rs export discovery
```

**Files to be modified:**
- `crates/elohim-sdk/Cargo.toml` — add `ts-rs`, `serde`, `serde_json`, `chrono`, `serde_bytes` deps
- `crates/elohim-sdk/src/lib.rs` — add `pub mod views;` and re-export
- `elohim/elohim-storage/Cargo.toml` — add `elohim-sdk = { path = "../../crates/elohim-sdk" }` and switch types over
- `elohim/elohim-storage/src/views.rs` — becomes a thin re-export shim during migration; deleted at end
- `elohim/elohim-storage/src/http.rs` — update imports from `crate::views::*` to `elohim_sdk::views::*`
- `crates/elohim-storage-client/Cargo.toml` — depend on `elohim-sdk` instead of (or alongside) `elohim-storage` for type definitions
- `deny.toml` (NEW at repo root, NOT the brit submodule's deny.toml)
- `.husky/pre-push` — add `cargo deny check bans` smoke test

**Files NOT touched:**
- The TypeScript SDK at `elohim/sdk/` — separate concern, doc-only changes in T13
- Schema files at `elohim/sdk/schemas/v1/` — already per-view; this plan moves Rust to match
- Storage internals (Diesel models at `elohim-storage/src/db/models.rs`, internal error types, P2P transport details) — these STAY in elohim-storage and must NOT leak across the boundary

---

## Task 1: Inventory SDK-surface types in views.rs

**Files:**
- Create: `/tmp/sdk-boundary-inventory.md` (transient — not committed)

Before moving anything, catalog every type currently in `views.rs` and classify each as SDK-surface (move to elohim-sdk) or implementation-internal (stays).

- [ ] **Step 1: Extract type declarations from views.rs**

```bash
grep -nE "^pub (struct|enum) [A-Z][a-zA-Z]*View" /projects/elohim/elohim/elohim-storage/src/views.rs > /tmp/view-types.txt
wc -l /tmp/view-types.txt
head -30 /tmp/view-types.txt
```

Expected: 100+ type declarations.

- [ ] **Step 2: Classify each type**

For each type in /tmp/view-types.txt, determine its domain by looking at the type name prefix or context:
- `Content*View`, `Path*View`, `Mastery*View` → lamad
- `EconomicEvent*View`, `Stewardship*View`, `Collective*View`, `Reciprocity*View` → shefa
- `GovernanceAction*View`, `Vote*View`, `Affinity*View`, `Challenge*View` → qahal
- `Human*View`, `Relationship*View`, `AgentPeerBinding*View`, `Account*View`, `Identity*View`, `Recovery*View` → imagodei
- `P2P*View`, `Peer*View`, `Federation*View`, `Resilience*View`, `Topology*View`, `Cluster*View`, `Drain*View` → infrastructure
- `Epr*View`, `EprEnvelope*View`, `EprList*View` → epr
- `*InputView` (any suffix) → inputs

Write the classification to `/tmp/sdk-boundary-inventory.md`. Schema:

```markdown
# SDK Boundary Inventory — 2026-05-18

## lamad (N types)
- ContentView (line 142)
- ContentWithTagsView (line 178)
- ...

## shefa (M types)
- EconomicEventView (line 1248)
- ...

## qahal (K types)
...

## imagodei (...)
## infrastructure (...)
## epr (...)
## inputs (...)

## Storage-internal (must NOT move to SDK)
- (any type that's NOT a `*View` and lives in views.rs by accident — flag with grep -nE "^pub (struct|enum) [A-Z][a-zA-Z]+" views.rs and diff against the View list)
```

- [ ] **Step 3: Sanity check — any types missing a domain?**

```bash
# Confirm every View type matches one of the domain prefixes
grep -cE "^pub (struct|enum) (Content|Path|Mastery|EconomicEvent|Stewardship|Collective|Reciprocity|GovernanceAction|Vote|Affinity|Challenge|Human|Relationship|AgentPeerBinding|Account|Identity|Recovery|P2P|Peer|Federation|Resilience|Topology|Cluster|Drain|Epr)" /projects/elohim/elohim/elohim-storage/src/views.rs
```

The count should be close to the total. If not, the missed types are domain-ambiguous — list them at the bottom of `/tmp/sdk-boundary-inventory.md` under "Domain TBD — operator decides" with file paths and types.

- [ ] **Step 4: No commit — inventory is transient.**

The inventory at `/tmp/sdk-boundary-inventory.md` informs Tasks 2-9 but isn't committed itself. It's a working document.

---

## Task 2: Add elohim-sdk deps + create views/mod.rs skeleton

**Files:**
- Modify: `crates/elohim-sdk/Cargo.toml`
- Create: `crates/elohim-sdk/src/views/mod.rs`
- Modify: `crates/elohim-sdk/src/lib.rs`

Set up the destination structure before any types move.

- [ ] **Step 1: Read current elohim-sdk Cargo.toml**

```bash
cat /projects/elohim/crates/elohim-sdk/Cargo.toml
```

Note the existing `[dependencies]` block.

- [ ] **Step 2: Add ts-rs and friends to elohim-sdk/Cargo.toml**

Append to the `[dependencies]` section (alphabetically among existing entries):

```toml
chrono = { workspace = true }
serde_bytes = "0.11"
ts-rs = { workspace = true, features = ["chrono-impl", "serde-compat"] }
```

If any of these are already there with different settings, reconcile manually — read the file first, then edit.

- [ ] **Step 3: Create views/mod.rs**

Create `/projects/elohim/crates/elohim-sdk/src/views/mod.rs`:

```rust
//! View types exposed at the elohim-sdk boundary.
//!
//! These are the wire-shape (camelCase, JSON-parsed) types that HTTP
//! consumers and SDK clients depend on. The ts-rs codegen pipeline reads
//! the per-domain modules and produces TypeScript at
//! `elohim/sdk/storage-client-ts/src/generated/`.
//!
//! Boundary rule: types declared in this module MUST NOT pull in
//! `elohim-storage` types as fields. Anything that needs a storage-internal
//! type lives in `elohim-storage` and is converted at the API edge.

pub mod shared;
pub mod lamad;
pub mod shefa;
pub mod qahal;
pub mod imagodei;
pub mod infrastructure;
pub mod epr;
pub mod inputs;

// Convenience re-exports of the most commonly used types.
pub use shared::*;
```

- [ ] **Step 4: Create empty per-domain stubs**

For each of the seven domain modules, create a stub file with just a doc comment so cargo can build the empty tree:

```bash
for dom in shared lamad shefa qahal imagodei infrastructure epr inputs; do
  cat > /projects/elohim/crates/elohim-sdk/src/views/$dom.rs <<EOF
//! $dom domain view types — populated in subsequent tasks.
EOF
done
```

- [ ] **Step 5: Wire views/ into lib.rs**

Read `/projects/elohim/crates/elohim-sdk/src/lib.rs` and add the `pub mod views;` declaration near the other top-level `pub mod` lines:

```rust
pub mod views;
```

- [ ] **Step 6: Verify the crate builds**

```bash
cd /projects/elohim && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo build -p elohim-sdk 2>&1 | tail -10
```

Expected: builds clean. Empty modules compile.

- [ ] **Step 7: Commit**

```bash
git add crates/elohim-sdk/
git commit -m "feat(elohim-sdk): scaffold views/ module with per-domain stubs

Empty per-domain stubs so subsequent tasks can move View types one
domain at a time without each task touching mod.rs.

Refs: genesis/docs/plans/2026-05-18-sdk-boundary-clarification.md (T2)"
```

---

## Task 3: Wire elohim-storage to depend on elohim-sdk

**Files:**
- Modify: `elohim/elohim-storage/Cargo.toml`

Add the dependency edge so future tasks can incrementally re-export types from elohim-sdk back through elohim-storage for compat.

- [ ] **Step 1: Read elohim-storage Cargo.toml**

```bash
cat /projects/elohim/elohim/elohim-storage/Cargo.toml | head -80
```

Find the `[dependencies]` block.

- [ ] **Step 2: Add the elohim-sdk dependency**

Add alphabetically among the existing path-deps:

```toml
elohim-sdk = { path = "../../crates/elohim-sdk" }
```

- [ ] **Step 3: Verify the workspace still builds**

```bash
cd /projects/elohim/elohim && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo build -p elohim-storage 2>&1 | tail -10
```

Expected: builds clean.

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/Cargo.toml
git commit -m "feat(elohim-storage): depend on elohim-sdk (path dep)

Sets up the dependency edge for incremental View type migration.
elohim-storage will re-export from elohim-sdk during the transition.

Refs: genesis/docs/plans/2026-05-18-sdk-boundary-clarification.md (T3)"
```

---

## Task 4: Move shared primitives to elohim-sdk

**Files:**
- Modify: `crates/elohim-sdk/src/views/shared.rs`
- Modify: `elohim/elohim-storage/src/views.rs`

Move the primitives used across multiple domains first: `ViewSlice`, `Freshness`, `JsonValue` (the ts-rs wrapper at top of views.rs), and any other cross-cutting types.

- [ ] **Step 1: Locate the shared primitives in views.rs**

```bash
grep -nE "^pub (struct|enum) (ViewSlice|Freshness|JsonValue|DataValue)" /projects/elohim/elohim/elohim-storage/src/views.rs
```

Read each declaration's full block (including derive macros, doc comments, and impl blocks).

- [ ] **Step 2: Move each shared type into crates/elohim-sdk/src/views/shared.rs**

For each shared type, copy the FULL declaration (doc comments, derives, struct/enum body, any impls) into `shared.rs`. Preserve the `#[derive(TS)]` annotations and `#[ts(export, export_to = "...")]` attributes EXACTLY — these drive the codegen output paths.

Replace the original in views.rs with a re-export:

```rust
pub use elohim_sdk::views::shared::ViewSlice;
```

Repeat for each moved type.

Maintain `use` imports as needed. The shared types likely use `serde`, `serde_json::Value`, `ts_rs::TS`, possibly `chrono::DateTime<Utc>`.

- [ ] **Step 3: Run ts-rs export from elohim-storage**

```bash
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo test export_bindings 2>&1 | tail -15
```

Expected: ts-rs writes its outputs. May take a moment.

- [ ] **Step 4: Verify ts-rs output is byte-identical to baseline**

```bash
find /projects/elohim/elohim/sdk/storage-client-ts/src/generated -name '*.ts' | sort | xargs sha256sum | sort > /tmp/ts-rs-after-shared.sha256
diff /tmp/ts-rs-baseline.sha256 /tmp/ts-rs-after-shared.sha256
```

Expected: no diff. If there IS a diff, the ts-rs export path changed because the type now lives in a different crate. STOP and investigate:
1. Is the `#[ts(export_to = "...")]` attribute pointing to the same final path?
2. Is the codegen invocation from the right crate? If `elohim-sdk` also has a `cargo test export_bindings` target, the export paths may need adjustment.

If the diff is COSMETIC only (whitespace, comment ordering), document the diff in `/tmp/ts-rs-cosmetic-diff.md` and proceed; otherwise treat as a blocker.

- [ ] **Step 5: Build the workspace**

```bash
cd /projects/elohim/elohim && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo build --workspace 2>&1 | tail -10
```

Expected: builds clean.

- [ ] **Step 6: Commit**

```bash
git add crates/elohim-sdk/src/views/shared.rs elohim/elohim-storage/src/views.rs
git commit -m "refactor(elohim-sdk): move shared View primitives from elohim-storage

ViewSlice, Freshness, JsonValue wrapper, and other cross-cutting primitives
now live in elohim-sdk. elohim-storage re-exports them for compat during
the migration; ts-rs output unchanged.

Refs: genesis/docs/plans/2026-05-18-sdk-boundary-clarification.md (T4)"
```

---

## Task 5: Move lamad View types

**Files:**
- Modify: `crates/elohim-sdk/src/views/lamad.rs`
- Modify: `elohim/elohim-storage/src/views.rs`

Same recipe as T4, applied to lamad. Use the `/tmp/sdk-boundary-inventory.md` from T1 to identify the lamad types.

- [ ] **Step 1: For each lamad-classified type in the inventory**

Read the type's full declaration from views.rs (including ts-rs attributes), move it to `crates/elohim-sdk/src/views/lamad.rs`, replace the original with `pub use elohim_sdk::views::lamad::TypeName;`.

Carry over necessary `use` imports — typically `serde::{Deserialize, Serialize}`, `serde_json::Value`, `ts_rs::TS`, and references to shared primitives via `super::shared::*` or `crate::views::shared::*`.

- [ ] **Step 2: Re-run ts-rs and check baseline**

```bash
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo test export_bindings 2>&1 | tail -15
find /projects/elohim/elohim/sdk/storage-client-ts/src/generated -name '*.ts' | sort | xargs sha256sum | sort > /tmp/ts-rs-after-lamad.sha256
diff /tmp/ts-rs-baseline.sha256 /tmp/ts-rs-after-lamad.sha256
```

Expected: no diff (or only cosmetic, documented).

- [ ] **Step 3: Build the workspace**

```bash
cd /projects/elohim/elohim && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo build --workspace 2>&1 | tail -10
```

- [ ] **Step 4: Commit**

```bash
git add crates/elohim-sdk/src/views/lamad.rs elohim/elohim-storage/src/views.rs
git commit -m "refactor(elohim-sdk): move lamad View types from elohim-storage

ts-rs output unchanged.

Refs: genesis/docs/plans/2026-05-18-sdk-boundary-clarification.md (T5)"
```

---

## Task 6: Move shefa View types

**Files:**
- Modify: `crates/elohim-sdk/src/views/shefa.rs`
- Modify: `elohim/elohim-storage/src/views.rs`

Same recipe as T5 with shefa types.

- [ ] **Step 1-4: Apply the T5 recipe with shefa paths and types from the inventory**

- [ ] **Step 5: Commit**

```bash
git add crates/elohim-sdk/src/views/shefa.rs elohim/elohim-storage/src/views.rs
git commit -m "refactor(elohim-sdk): move shefa View types from elohim-storage

Refs: genesis/docs/plans/2026-05-18-sdk-boundary-clarification.md (T6)"
```

---

## Task 7: Move qahal View types

**Files:**
- Modify: `crates/elohim-sdk/src/views/qahal.rs`
- Modify: `elohim/elohim-storage/src/views.rs`

- [ ] **Step 1-4: Apply the T5 recipe with qahal paths and types**

- [ ] **Step 5: Commit**

```bash
git add crates/elohim-sdk/src/views/qahal.rs elohim/elohim-storage/src/views.rs
git commit -m "refactor(elohim-sdk): move qahal View types from elohim-storage

Refs: genesis/docs/plans/2026-05-18-sdk-boundary-clarification.md (T7)"
```

---

## Task 8: Move imagodei View types

**Files:**
- Modify: `crates/elohim-sdk/src/views/imagodei.rs`
- Modify: `elohim/elohim-storage/src/views.rs`

- [ ] **Step 1-4: Apply the T5 recipe with imagodei paths and types**

- [ ] **Step 5: Commit**

```bash
git add crates/elohim-sdk/src/views/imagodei.rs elohim/elohim-storage/src/views.rs
git commit -m "refactor(elohim-sdk): move imagodei View types from elohim-storage

Refs: genesis/docs/plans/2026-05-18-sdk-boundary-clarification.md (T8)"
```

---

## Task 9: Move infrastructure View types

**Files:**
- Modify: `crates/elohim-sdk/src/views/infrastructure.rs`
- Modify: `elohim/elohim-storage/src/views.rs`

- [ ] **Step 1-4: Apply the T5 recipe with infrastructure paths and types**

- [ ] **Step 5: Commit**

```bash
git add crates/elohim-sdk/src/views/infrastructure.rs elohim/elohim-storage/src/views.rs
git commit -m "refactor(elohim-sdk): move infrastructure View types from elohim-storage

Refs: genesis/docs/plans/2026-05-18-sdk-boundary-clarification.md (T9)"
```

---

## Task 10: Move epr View types + *InputView types

**Files:**
- Modify: `crates/elohim-sdk/src/views/epr.rs`
- Modify: `crates/elohim-sdk/src/views/inputs.rs`
- Modify: `elohim/elohim-storage/src/views.rs`

- [ ] **Step 1: Move all `Epr*View` types to epr.rs (per the T5 recipe)**

- [ ] **Step 2: Move all `*InputView` types to inputs.rs (per the T5 recipe)**

- [ ] **Step 3: Re-run ts-rs + build + verify**

```bash
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo test export_bindings 2>&1 | tail -15
find /projects/elohim/elohim/sdk/storage-client-ts/src/generated -name '*.ts' | sort | xargs sha256sum | sort > /tmp/ts-rs-after-epr-inputs.sha256
diff /tmp/ts-rs-baseline.sha256 /tmp/ts-rs-after-epr-inputs.sha256
cd /projects/elohim/elohim && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo build --workspace 2>&1 | tail -10
```

- [ ] **Step 4: Commit**

```bash
git add crates/elohim-sdk/src/views/epr.rs crates/elohim-sdk/src/views/inputs.rs elohim/elohim-storage/src/views.rs
git commit -m "refactor(elohim-sdk): move EPR + Input View types from elohim-storage

Refs: genesis/docs/plans/2026-05-18-sdk-boundary-clarification.md (T10)"
```

---

## Task 11: Reduce views.rs to a thin re-export shim

**Files:**
- Modify: `elohim/elohim-storage/src/views.rs`

After Tasks 4-10, views.rs should be mostly `pub use elohim_sdk::views::*::*` lines. Trim to just the re-exports and any helper functions that legitimately need to live next to the storage internals (e.g. conversion functions Wire→View that import from `crate::db::*`).

- [ ] **Step 1: Check current LOC**

```bash
wc -l /projects/elohim/elohim/elohim-storage/src/views.rs
```

Expected: significantly smaller than 8,208 (most types moved). Probably 500-1,500 LOC depending on how much glue stayed.

- [ ] **Step 2: Identify what remains**

```bash
grep -nE "^(pub fn|fn|impl|pub use|//) " /projects/elohim/elohim/elohim-storage/src/views.rs | head -40
```

Anything that is `pub use elohim_sdk::views::...` is fine — that's the re-export layer.

Anything that is `pub fn` or `impl` is conversion logic and either:
(a) Stays in views.rs because it touches `crate::db::*` types
(b) Should be moved into `elohim-storage/src/views_convert.rs` (new file) for separation
(c) Belongs in elohim-sdk if it's domain logic with no storage dep

Apply the appropriate move for each remaining function. Default: leave as-is unless cleanup is obvious.

- [ ] **Step 3: Verify build + ts-rs unchanged**

```bash
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo test export_bindings 2>&1 | tail -5
cd /projects/elohim/elohim && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo build --workspace 2>&1 | tail -10
```

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/views.rs
git commit -m "refactor(elohim-storage): reduce views.rs to thin re-export shim

views.rs now only contains pub use lines re-exporting from elohim-sdk
plus the storage-coupled conversion helpers that legitimately need DB
type access.

Refs: genesis/docs/plans/2026-05-18-sdk-boundary-clarification.md (T11)"
```

---

## Task 12: Switch consumers to depend on elohim-sdk directly

**Files:**
- Modify: `doorway/doorway-service/Cargo.toml`
- Modify: `crates/elohim-storage-client/Cargo.toml`
- Modify: `steward/node/Cargo.toml` (if it currently depends on elohim-storage for types)
- Source files in consumers that import from `elohim_storage::views::*`

- [ ] **Step 1: Find consumers that import View types from elohim-storage**

```bash
grep -rln "elohim_storage::views" /projects/elohim/doorway /projects/elohim/steward /projects/elohim/crates 2>/dev/null
```

Capture the list.

- [ ] **Step 2: For each consumer, add elohim-sdk as a dep**

In the consumer's `Cargo.toml`, add (alphabetically among existing deps):

```toml
elohim-sdk = { path = "../../crates/elohim-sdk" }
```

Adjust the path depth based on the consumer's location.

- [ ] **Step 3: Update imports**

Use sed (carefully, scoped):

```bash
for f in <list of files from Step 1>; do
  sed -i 's/use elohim_storage::views::/use elohim_sdk::views::/g' "$f"
done
```

Better: do it by hand with Edit tool, one consumer at a time, so you catch any non-trivial usage.

- [ ] **Step 4: Build each consumer**

```bash
cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/doorway__doorway-service/dev cargo build 2>&1 | tail -10
cd /projects/elohim/steward/node && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/steward__node/dev cargo build 2>&1 | tail -10
cd /projects/elohim && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/crates/dev cargo build -p elohim-storage-client 2>&1 | tail -10
```

- [ ] **Step 5: Commit**

```bash
git add doorway/doorway-service/Cargo.toml steward/node/Cargo.toml crates/elohim-storage-client/Cargo.toml \
        <any consumer source files modified>
git commit -m "refactor(consumers): depend on elohim-sdk for View types

doorway-service, steward/node, and elohim-storage-client now pull View
type definitions from elohim-sdk directly instead of routing through
elohim-storage. elohim-storage retains the re-export shim for any
in-tree consumer that hasn't migrated yet.

Refs: genesis/docs/plans/2026-05-18-sdk-boundary-clarification.md (T12)"
```

---

## Task 13: Add cargo-deny boundary rule

**Files:**
- Create: `/projects/elohim/deny.toml` (repo root — NOT elohim/brit/deny.toml which is a submodule's)
- Modify: `.husky/pre-push` (add cargo-deny smoke)

A `cargo deny check bans` rule that fails the build if `elohim-storage` becomes a direct dep of any non-server consumer.

- [ ] **Step 1: Install cargo-deny if not present**

```bash
cargo deny --version 2>&1 || cargo install cargo-deny --locked
```

Expected: cargo-deny is available.

- [ ] **Step 2: Create deny.toml at repo root**

Write to `/projects/elohim/deny.toml`:

```toml
# cargo-deny configuration for the Elohim Protocol repo.
#
# Primary purpose: enforce the elohim-sdk boundary.
# - elohim-storage is the implementation; only server-side consumers may
#   depend on it directly. Client-side or external consumers depend on
#   elohim-sdk and let elohim-storage refactor freely behind the boundary.
#
# To check locally:  cargo deny --manifest-path elohim/Cargo.toml check bans

[graph]
all-features = false
no-default-features = false

[output]
feature-depth = 1

[bans]
multiple-versions = "warn"
wildcards = "deny"

# elohim-storage is the implementation; non-server consumers must NOT depend
# on it directly. They should depend on elohim-sdk instead.
[[bans.deny]]
name = "elohim-storage"
wrappers = [
    "elohim-storage-client",   # the client adapter that wraps storage for non-server callers
    "doorway-service",          # server-side gateway, allowed
    "steward-node",             # server-side node runtime, allowed
    "elohim-node",              # server-side node, allowed
    "elohim-storage",           # self-reference for dev-deps
]

# Add similar rules here as more boundaries get established.

[licenses]
allow = ["MIT", "Apache-2.0", "BSD-3-Clause", "BSD-2-Clause", "ISC", "Unicode-DFS-2016", "CAL-1.0", "MPL-2.0", "Zlib", "OpenSSL"]
exceptions = []
unlicensed = "warn"

[sources]
unknown-registry = "warn"
unknown-git = "warn"
allow-registry = ["https://github.com/rust-lang/crates.io-index", "sparse+https://nexus.ethosengine.com/repository/cargo-internal/", "sparse+https://nexus.ethosengine.com/repository/cargo/"]
```

- [ ] **Step 3: Run cargo-deny against the elohim workspace**

```bash
cd /projects/elohim && RUSTFLAGS="" RUSTC_WRAPPER="" cargo deny --manifest-path elohim/Cargo.toml check bans 2>&1 | tail -20
```

Expected: passes. If it fails because a current consumer still depends on elohim-storage directly, that consumer wasn't migrated in T12 — go back and migrate it OR add it to the `wrappers` list with a comment explaining why.

- [ ] **Step 4: Add a cargo-deny smoke to pre-push**

Read `/projects/elohim/.husky/pre-push` and find the gate-detection block. Add (in the project-conditional section that runs Rust gates):

```bash
# cargo-deny bans check — enforces the elohim-sdk boundary
if command -v cargo-deny >/dev/null 2>&1; then
  echo "→ cargo deny check bans (elohim workspace)"
  if ! (cd "$REPO_ROOT" && cargo deny --manifest-path elohim/Cargo.toml check bans 2>&1 | tail -20); then
    echo "✗ cargo-deny failed — boundary violation. See deny.toml for rules."
    exit 1
  fi
fi
```

Place the snippet in a sensible location (near other workspace-level checks). If pre-push has a structure that splits per-project gates differently, adapt — the goal is "ensure cargo-deny runs at least once per push."

- [ ] **Step 5: Commit**

```bash
git add deny.toml .husky/pre-push
git commit -m "feat(deny): enforce elohim-sdk boundary via cargo-deny bans

deny.toml at repo root declares elohim-storage as a banned direct dep
except for the explicitly-listed server-side wrappers. Pre-push runs
\`cargo deny check bans\` against the elohim workspace.

Refs: genesis/docs/plans/2026-05-18-sdk-boundary-clarification.md (T13)"
```

---

## Task 14: Document the TypeScript SDK boundary

**Files:**
- Modify: `elohim/sdk/CLAUDE.md`
- Modify: `elohim/sdk/storage-client-ts/README.md` (create if missing)

The TS SDK at `elohim/sdk/` is healthier than the Rust side was — but its boundary is implicit. Document it.

- [ ] **Step 1: Read current elohim/sdk/CLAUDE.md**

```bash
cat /projects/elohim/elohim/sdk/CLAUDE.md
```

Note the current sections.

- [ ] **Step 2: Add or update a "SDK Boundary" section**

Add after the existing top-level intro:

```markdown
## SDK Boundary

The `elohim/sdk/` tree is the **TypeScript SDK** distributed to consumers (browser, doorway clients, future external integrators). Its boundary is:

| Path | Role |
|---|---|
| `elohim/sdk/storage-client-ts/` | Generated wire types + HTTP client; pulled from `elohim-sdk::views` via ts-rs |
| `elohim/sdk/epr-ts/` | Generated EPR codec types; pulled from `elohim-epr` via ts-rs |
| `elohim/sdk/schemas/v1/` | JSON Schemas — the authoritative wire contract (drives both Rust and TS codegen) |
| `elohim/sdk/domains/<app>/` | App manifests + companion schemas + per-app codegen scripts |
| `elohim/sdk/src/` | Hand-written SDK helpers (connection management, type guards) |

**Rule:** consumers import from `@elohim/storage-client` and `@elohim/epr-ts`. They do NOT reach into `elohim-storage` internal Rust types, and they do NOT bypass the generated types by reading raw JSON. The generated TS is the boundary.

For the Rust side of the SDK boundary, see `crates/elohim-sdk/` and the `deny.toml` rules at the repo root.
```

- [ ] **Step 3: Add a README to storage-client-ts if missing**

If `elohim/sdk/storage-client-ts/README.md` is missing, create it with:

```markdown
# @elohim/storage-client

TypeScript client for the Elohim Protocol storage layer.

## Generated types

Types under `src/generated/` are auto-generated from the Rust `crates/elohim-sdk` crate via ts-rs. DO NOT edit them by hand — regenerate via:

```bash
cd /path/to/elohim/elohim/elohim-storage
cargo test export_bindings
```

## Hand-written

`src/client.ts`, `src/sync.ts`, `src/types.ts`, `src/index.ts` are hand-written wrappers around the generated types. They live here for ergonomic API surface and stable consumer imports.

## Stability

Wire shapes are governed by `elohim/sdk/schemas/v1/views/*.schema.json`. Changes to wire types should land schema-first, then propagate to Rust (View struct in elohim-sdk) → ts-rs generation → consumer code.
```

- [ ] **Step 4: Commit**

```bash
git add elohim/sdk/CLAUDE.md elohim/sdk/storage-client-ts/README.md
git commit -m "docs(sdk): document the TypeScript SDK boundary

Adds an SDK Boundary section to elohim/sdk/CLAUDE.md naming the canonical
paths and rules. Adds a README to storage-client-ts explaining the
generated-vs-hand-written split and the schema-first stability rule.

Refs: genesis/docs/plans/2026-05-18-sdk-boundary-clarification.md (T14)"
```

---

## Self-Review (already performed by plan author)

**Spec coverage:**
- Flesh out crates/elohim-sdk/ — Tasks 2, 4-10
- Pull SDK-surface types from elohim-storage — Tasks 4-11
- elohim-storage depends on elohim-sdk — Task 3
- Consumers depend on elohim-sdk — Task 12
- cargo-deny boundary check — Task 13
- TypeScript SDK boundary doc — Task 14

**Placeholder scan:** No `TBD`, `TODO`, `FILL IN`. Every task has concrete files, exact commands, expected outputs.

**Type consistency:** `elohim-sdk::views::shared::ViewSlice` used in T4; same path referenced in T11. `crates/elohim-sdk/src/views/<domain>.rs` naming consistent across T2 (stubs), T5-10 (population), T11 (shim reduction). cargo-deny `wrappers` list in T13 enumerates the consumer crates that legitimately depend on `elohim-storage` directly — must match the workspace member names.

**Risks documented in-plan:**
- ts-rs path-dependent export ordering — every type-move task verifies byte-identical TS output
- Some View types may legitimately need `crate::db::*` access — T11 Step 2 handles this case
- cargo-deny might fail T13 Step 3 if a consumer wasn't migrated — recovery path in same step

**Execution handoff:** Ready for superpowers:subagent-driven-development.
