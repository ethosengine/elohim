# SDK Boundary Clarification Implementation Plan (Revised — elohim-views)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create lightweight `crates/elohim-views` crate holding ALL ts-rs-anchored View + InputView types in one atomic migration. `elohim-storage` depends on it; `elohim-sdk` re-exports it as the consumer-friendly facade; `cargo-deny` enforces no-direct-`elohim-storage`-deps for non-server consumers. Consumers (Tauri client, future third-party Rust apps) get the small type surface without pulling diesel/axum/libp2p/conductor transitively.

**Architecture:** The original plan's incremental per-domain type moves failed in T4 PILOT because ts-rs computes cross-crate import paths in generated TypeScript using the source crate's file path, not the `export_to` directory — so partial moves break 70+ downstream `.ts` files (verified 2026-05-18; see `[[feedback_ts_rs_cross_crate_import_paths]]`). The fix is structural: ALL ts-rs-anchored types live in one crate. We move ~261 View types from `elohim/elohim-storage/src/views.rs` (8,208 LOC) to `crates/elohim-views/src/{lamad,shefa,qahal,imagodei,infrastructure,epr,inputs,shared}.rs` in a single atomic commit, retarget every `export_to` path once, relocate the `cargo test export_bindings` invocation, and verify byte-identical generated TypeScript with sha256 against baseline.

**Tech Stack:** Rust 2021, ts-rs 10.1, cargo-deny dependency-bans, the existing `cargo test export_bindings` workflow.

---

## Pre-execution gate (do once before Task 1)

- [ ] Workspace clean: `git status --short` shows nothing uncommitted
- [ ] Baseline captured: `/tmp/ts-rs-baseline-plan2.sha256` (356 .ts files; captured at session start)
- [ ] T4 PILOT learnings landed in memory (see commit history for `feedback_ts_rs_cross_crate_import_paths.md`)
- [ ] Disk healthy: `df -h /projects` shows ≥30 GB free (cargo cold builds need headroom)
- [ ] crates/ exclusion confirmed: `grep -A5 'exclude' /projects/elohim/elohim/Cargo.toml` — crates/* is excluded from the elohim workspace; this is deliberate (each crates/* crate uses direct version pins, not `workspace = true`)

## File Structure

**Files to be created:**
```
crates/elohim-views/
├── Cargo.toml                 # Lightweight deps: serde, serde_json, chrono, serde_bytes, ts-rs (no diesel, no axum, no libp2p)
└── src/
    ├── lib.rs                 # Re-export hub
    ├── shared.rs              # ViewSlice, Freshness, JsonValue wrapper, DataValue — cross-domain primitives
    ├── lamad.rs               # Content/Path/Mastery/Knowledge view types
    ├── shefa.rs               # EconomicEvent/Stewardship/Collective/Reciprocity view types
    ├── qahal.rs               # GovernanceAction/Vote/Affinity/Challenge view types
    ├── imagodei.rs            # Human/Relationship/AgentPeerBinding/Account/Identity/Recovery view types
    ├── infrastructure.rs      # P2P/Peer/Federation/Resilience/Topology/Cluster/Drain view types
    ├── epr.rs                 # Epr/EprEnvelope/EprList/EprProviders view types
    └── inputs.rs              # All *InputView types
```

**Files to be modified:**
- `crates/elohim-sdk/Cargo.toml` — depend on elohim-views (REMOVE the ts-rs/chrono/serde_bytes added in the previous SDK.T2; those move to elohim-views)
- `crates/elohim-sdk/src/views/mod.rs` — change from local modules to `pub use elohim_views::*;` facade
- `crates/elohim-sdk/src/views/{shared,lamad,shefa,qahal,imagodei,infrastructure,epr,inputs}.rs` — DELETE (no longer needed; facade re-exports from elohim-views)
- `elohim/elohim-storage/Cargo.toml` — add elohim-views path-dep, KEEP elohim-sdk path-dep
- `elohim/elohim-storage/src/views.rs` — reduce from 8,208 LOC to a re-export shim
- `elohim/elohim-storage/Cargo.toml` — keep ts-rs in dev-dependencies (no longer needed for the export pass, but other tests may use it)
- `crates/elohim-storage-client/Cargo.toml` — depend on elohim-views directly (lightweight; no more transitive elohim-storage)
- `doorway/doorway-service/Cargo.toml` — add elohim-views (keeps elohim-storage too — server-side)
- `steward/node/Cargo.toml` — add elohim-views (keeps elohim-storage too — server-side)
- `deny.toml` (new, at repo root) — cargo-deny boundary rules
- `.husky/pre-push` — add cargo-deny check
- `elohim/sdk/CLAUDE.md` + `crates/elohim-sdk/README.md` — document the boundary

**Files NOT touched:**
- The generated TypeScript at `elohim/sdk/storage-client-ts/src/generated/` — output paths unchanged
- Schema files at `elohim/sdk/schemas/v1/views/` — already per-view; this plan moves Rust to match
- Diesel models at `elohim/elohim-storage/src/db/models.rs` — these STAY in elohim-storage (storage-internal)
- Wire conversion helpers in views.rs (Wire→View From impls touching `crate::db::*`) — these STAY in elohim-storage too

---

## Task 1: Revert prior SDK.T2 scaffolding + create elohim-views skeleton

**Files:**
- Modify: `crates/elohim-sdk/Cargo.toml` (remove ts-rs, chrono, serde_bytes added in prior SDK.T2)
- Delete: `crates/elohim-sdk/src/views/` (8 stub files added in prior SDK.T2 — wrong place)
- Modify: `crates/elohim-sdk/src/lib.rs` (remove `pub mod views;`)
- Create: `crates/elohim-views/Cargo.toml`
- Create: `crates/elohim-views/src/{lib.rs,shared.rs,lamad.rs,shefa.rs,qahal.rs,imagodei.rs,infrastructure.rs,epr.rs,inputs.rs}`

The previous SDK.T2 put ts-rs deps + view stubs into elohim-sdk; the PILOT showed that was the wrong architecture. We revert that scaffolding and put ts-rs in the new elohim-views crate where it belongs.

- [ ] **Step 1: Verify prior SDK.T2 state**

```bash
grep -E "ts-rs|chrono|serde_bytes" /projects/elohim/crates/elohim-sdk/Cargo.toml
ls /projects/elohim/crates/elohim-sdk/src/views/ 2>/dev/null
```

You should see ts-rs/chrono/serde_bytes in elohim-sdk's Cargo.toml and ~8 stub files in src/views/.

- [ ] **Step 2: Revert elohim-sdk Cargo.toml**

Use Edit to remove the ts-rs, chrono, serde_bytes entries added in SDK.T2. The file should be back to its pre-SDK.T2 state (with only the original serde/serde_json/thiserror/tracing/async-trait deps).

- [ ] **Step 3: Delete the elohim-sdk views/ stubs**

```bash
rm -rf /projects/elohim/crates/elohim-sdk/src/views
```

- [ ] **Step 4: Remove `pub mod views;` from elohim-sdk/src/lib.rs**

Use Edit. Remove the single line added in SDK.T2.

- [ ] **Step 5: Verify elohim-sdk builds clean post-revert**

```bash
cd /projects/elohim && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo build -p elohim-sdk 2>&1 | tail -10
```

Expected: clean build.

- [ ] **Step 6: Create elohim-views crate**

```bash
mkdir -p /projects/elohim/crates/elohim-views/src
```

Write `/projects/elohim/crates/elohim-views/Cargo.toml`:

```toml
[package]
name = "elohim-views"
version = "0.1.0"
edition = "2021"
description = "Wire-shape Rust types for the Elohim Protocol storage API — ts-rs-anchored View + InputView types used at the HTTP boundary"
license = "CAL-1.0"
repository = "https://github.com/ethosengine/elohim"
readme = "README.md"
keywords = ["elohim", "views", "ts-rs", "wire-types"]
categories = ["data-structures"]
publish = ["elohim"]

[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_bytes = "0.11"
chrono = { version = "0.4", features = ["serde"] }
ts-rs = { version = "10.1", features = ["chrono-impl", "serde-compat", "no-serde-warnings"] }
```

Write `/projects/elohim/crates/elohim-views/src/lib.rs`:

```rust
//! Wire-shape Rust types for the Elohim Protocol storage API.
//!
//! These types use `#[derive(TS)]` + `#[ts(export, export_to = "...")]` to
//! generate camelCase TypeScript interfaces at
//! `elohim/sdk/storage-client-ts/src/generated/`. Consumers depend on this
//! crate (directly or via the `elohim-sdk` facade) to get a stable wire
//! contract without pulling the heavy `elohim-storage` implementation
//! (diesel, axum, libp2p, conductor).
//!
//! # Boundary rules
//!
//! - Types here MUST be wire-shape (camelCase via `#[serde(rename_all = "camelCase")]`)
//! - Types here MUST NOT depend on storage-internal types (Diesel models,
//!   internal error types, P2P transport details)
//! - Conversion functions (Wire→View `From` impls touching DB types) live in
//!   `elohim-storage`, NOT here
//!
//! See `genesis/docs/plans/2026-05-18-sdk-boundary-clarification.md` for the
//! migration history.

pub mod shared;
pub mod lamad;
pub mod shefa;
pub mod qahal;
pub mod imagodei;
pub mod infrastructure;
pub mod epr;
pub mod inputs;

pub use shared::*;
```

Write empty stubs for each domain module:

```bash
for dom in shared lamad shefa qahal imagodei infrastructure epr inputs; do
  cat > /projects/elohim/crates/elohim-views/src/$dom.rs <<EOF
//! $dom view types — populated in Task 2 atomic migration.
EOF
done
```

Write `/projects/elohim/crates/elohim-views/README.md`:

```markdown
# elohim-views

Wire-shape Rust types for the Elohim Protocol storage API.

This crate holds the ts-rs-anchored View + InputView types that define the HTTP wire contract between elohim-storage and its clients. It is intentionally lightweight — only serde, serde_json, chrono, serde_bytes, and ts-rs — so consumers (Tauri desktop, third-party Rust SDKs) can depend on the type surface without pulling the full storage implementation.

## Generated TypeScript

Running `cargo test export_bindings` in this crate produces TypeScript types at `elohim/sdk/storage-client-ts/src/generated/`, consumed by `@elohim/storage-client`.

## Boundary

Per `deny.toml` at the repo root, only the server-side wrappers (doorway-service, steward-node, elohim-node, elohim-storage-client) may depend on `elohim-storage` directly. All other consumers depend on `elohim-views` (or `elohim-sdk` which re-exports it).
```

- [ ] **Step 7: Verify elohim-views builds**

```bash
cd /projects/elohim && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo build -p elohim-views 2>&1 | tail -10
```

Expected: builds clean (empty modules compile).

- [ ] **Step 8: Commit**

```bash
git add -u  # Pick up the SDK.T2 reverts
git add crates/elohim-views/
git commit -m "feat(elohim-views): create lightweight Wire-shape types crate + revert SDK.T2 ts-rs into wrong crate

Reverts the prior SDK.T2 attempt that put ts-rs deps + view stubs in
crates/elohim-sdk. Per the T4 PILOT finding ([[ts-rs-cross-crate-import-paths]]),
ts-rs cross-crate import paths break when types are split across crates.
The architecturally correct fix is a single lightweight types crate;
elohim-sdk becomes a thin re-export facade in a later task.

Refs: genesis/docs/plans/2026-05-18-sdk-boundary-clarification.md (T1)
Refs: .claude/memory/feedback_ts_rs_cross_crate_import_paths.md"
```

---

## Task 2: ATOMIC MIGRATION — move all 261 View types from elohim-storage to elohim-views

**Files:**
- Modify: `crates/elohim-views/src/{shared,lamad,shefa,qahal,imagodei,infrastructure,epr,inputs}.rs` (populated)
- Modify: `elohim/elohim-storage/Cargo.toml` (add elohim-views path-dep)
- Modify: `elohim/elohim-storage/src/views.rs` (reduce to re-export shim)

This is the big one. ALL ts-rs-anchored types move together in a single atomic commit so there are no cross-crate type references at any point. ts-rs sees ONE source tree when generating; all import paths in generated .ts files become `./TypeName` form, matching the flat output directory. Byte-identical against baseline.

### Step 1: Wire elohim-storage to depend on elohim-views

In `elohim/elohim-storage/Cargo.toml`, add (alphabetically among path-deps):

```toml
elohim-views = { path = "../../crates/elohim-views" }
```

(Keep the existing `elohim-sdk` path-dep added in prior SDK.T3.)

### Step 2: Plan the migration script

This task is too large for hand-editing. Write a Python script to /tmp that:

1. Parses `elohim/elohim-storage/src/views.rs` to find every `pub struct/enum` declaration that has `#[derive(...TS...)]` and `#[ts(export, ...)]`
2. Reads the inventory from prior `/tmp/sdk-boundary-inventory.md` (or re-classifies inline) to map each type to a domain
3. For each type:
   a. Extracts the full declaration block (doc comments + derives + ts-rs attribs + struct/enum body + any inherent impl blocks)
   b. Rewrites the `#[ts(export_to = "../../sdk/storage-client-ts/src/generated/")]` to `#[ts(export_to = "../../elohim/sdk/storage-client-ts/src/generated/")]` (adjusts for crates/elohim-views/ location: from elohim/elohim-storage/Cargo.toml the path was `../../sdk/...`; from crates/elohim-views/Cargo.toml it's `../../elohim/sdk/...`)
   c. Writes the rewritten block to the appropriate `crates/elohim-views/src/<domain>.rs`
   d. Removes the block from `views.rs`, replacing with a `pub use elohim_views::{lamad,shefa,...}::TypeName;` re-export so any in-tree consumer's `crate::views::TypeName` still resolves
4. Collects necessary `use` imports per domain file (serde, serde_json, ts_rs::TS, chrono, serde_bytes — preserve what each domain needs)

Do NOT commit the script; it's transient.

### Step 3: Run the migration script

```bash
python3 /tmp/migrate-views.py 2>&1 | tail -30
```

Expected: script reports types moved per domain, total count matches the ~261 from the inventory.

### Step 4: Verify both crates compile

```bash
cd /projects/elohim && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo build -p elohim-views -p elohim-storage 2>&1 | tail -20
```

If errors:
- "cannot find type X": a type wasn't moved or the re-export path is wrong
- "duplicate definition": views.rs still has the original (script didn't fully remove)
- "no field X on type Y": dependency between types that the script didn't fully route through re-exports
- Path issues: re-export `pub use elohim_views::<domain>::X;` might be wrong — check the actual module location

Fix any errors. The script may need refinement passes.

### Step 5: Relocate the export_bindings test target

Currently `cargo test export_bindings` runs from elohim-storage. After migration, ts-rs's auto-generated tests for the moved types live in elohim-views. Verify:

```bash
echo "=== From elohim-views (should produce all generated TS) ==="
cd /projects/elohim/crates/elohim-views && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo test export_bindings 2>&1 | tail -10

echo ""
echo "=== From elohim-storage (should produce 0 generated TS — all types moved) ==="
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo test export_bindings 2>&1 | tail -10
```

Expected: elohim-views test count matches the type count (~261); elohim-storage test count is 0 (or close to 0 — any remaining ts-rs types that didn't move would show up here; investigate any).

### Step 6: Byte-identical verification

```bash
find /projects/elohim/elohim/sdk/storage-client-ts/src/generated -name '*.ts' | sort | xargs sha256sum | sort > /tmp/ts-rs-after-elohim-views.sha256
diff /tmp/ts-rs-baseline-plan2.sha256 /tmp/ts-rs-after-elohim-views.sha256
echo "diff exit: $?"
```

Expected: exit 0 (empty diff).

Three failure modes to handle:

**(A) Missing files** — a type didn't get its ts-rs attribute preserved. Check the moved type for `#[ts(export, export_to = "...")]`; if missing, restore it.

**(B) Extra files** — types got duplicated; some are in both views.rs AND elohim-views. Remove from views.rs.

**(C) Content diffs** — a type's TypeScript interface has different fields. This means the moved Rust struct lost a field or the migration script mis-parsed something. Inspect the diff to identify which type, then fix.

**(D) Import path diffs** — should NOT happen now that all types are in one crate. If it does, some type wasn't moved (still in elohim-storage) and references one that did move. Resolve by moving the lagging type too.

### Step 7: views.rs final state check

```bash
wc -l /projects/elohim/elohim/elohim-storage/src/views.rs
```

Expected: <1,500 LOC. The file should now contain:
- Module-level docs
- `pub use elohim_views::*;` blanket re-export (or per-domain `pub use elohim_views::lamad::*;` etc.)
- The Wire→View conversion `From` impls (touching `crate::db::*` — must stay here)
- Any other storage-coupled helpers

If views.rs is still >2,000 LOC, the migration left more behind than it should have — go back to Step 2 and adjust the script.

### Step 8: Run the workspace tests (sanity check)

```bash
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo test --lib --no-fail-fast 2>&1 | tail -20
```

Expected: tests pass (or fail in known-pre-existing ways not introduced by this migration). Schema-contract tests should still pass since the wire shape is unchanged.

### Step 9: Commit

```bash
git add crates/elohim-views/src/ elohim/elohim-storage/Cargo.toml elohim/elohim-storage/src/views.rs
git commit -m "refactor(elohim-views): atomic migration of ~261 ts-rs-anchored View types from elohim-storage

All Views + InputViews moved to crates/elohim-views in a single atomic
migration to preserve ts-rs import-path semantics. ALL types in one crate
means generated .ts files use './TypeName' imports (flat output dir),
verified byte-identical against pre-move baseline.

elohim-storage/src/views.rs reduced to a thin re-export shim + the
Wire→View From impls that touch storage-internal Diesel types. The
cargo test export_bindings target moves to elohim-views.

Refs: genesis/docs/plans/2026-05-18-sdk-boundary-clarification.md (T2)
Refs: .claude/memory/feedback_ts_rs_cross_crate_import_paths.md"
```

---

## Task 3: Make elohim-sdk a facade over elohim-views

**Files:**
- Modify: `crates/elohim-sdk/Cargo.toml` (add elohim-views path-dep)
- Modify: `crates/elohim-sdk/src/lib.rs` (re-export from elohim-views)

elohim-sdk becomes the consumer-friendly entry point. Per-module organization is preserved by re-exporting from elohim-views — consumers can write `use elohim_sdk::views::lamad::ContentView` or the convenience `use elohim_sdk::views::ContentView`.

- [ ] **Step 1: Add elohim-views to elohim-sdk Cargo.toml**

In `[dependencies]`:

```toml
elohim-views = { path = "../elohim-views" }
```

- [ ] **Step 2: Add views re-export module to elohim-sdk/src/lib.rs**

Edit `crates/elohim-sdk/src/lib.rs`. Add near the existing module declarations:

```rust
/// View types (re-exported from `elohim-views` for the consumer-friendly facade).
///
/// Consumers should prefer `elohim_sdk::views::*` over depending on
/// `elohim-views` directly — this insulates them from any future
/// reorganization of the underlying types crate.
pub mod views {
    pub use elohim_views::*;
}
```

- [ ] **Step 3: Verify elohim-sdk builds**

```bash
cd /projects/elohim && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo build -p elohim-sdk 2>&1 | tail -10
```

- [ ] **Step 4: Commit**

```bash
git add crates/elohim-sdk/Cargo.toml crates/elohim-sdk/src/lib.rs
git commit -m "feat(elohim-sdk): re-export elohim-views as the consumer-friendly facade

Consumers write 'use elohim_sdk::views::ContentView' to get the stable
type surface; the underlying definitions live in elohim-views.

Refs: genesis/docs/plans/2026-05-18-sdk-boundary-clarification.md (T3)"
```

---

## Task 4: Switch elohim-storage-client to elohim-views

**Files:**
- Modify: `crates/elohim-storage-client/Cargo.toml`
- Modify: Source files in elohim-storage-client that import from elohim_storage::views

The Tauri desktop sidecar consumer. After this, elohim-storage-client no longer pulls elohim-storage transitively — its dep tree should be small.

- [ ] **Step 1: Inspect current elohim-storage-client deps**

```bash
cat /projects/elohim/crates/elohim-storage-client/Cargo.toml
grep -rln "elohim_storage::views" /projects/elohim/crates/elohim-storage-client/src 2>/dev/null
```

- [ ] **Step 2: Add elohim-views, REMOVE elohim-storage (if present)**

In `[dependencies]`, add:

```toml
elohim-views = { path = "../elohim-views" }
```

If elohim-storage is currently listed as a dependency, REMOVE it. The crate should depend on elohim-views (types) but NOT on elohim-storage (implementation).

- [ ] **Step 3: Update imports in source files**

For each file matched by the grep in Step 1, change `use elohim_storage::views::*` to `use elohim_views::*` (or `use elohim_sdk::views::*` if the project convention prefers the facade).

Use Edit per file with the exact line.

- [ ] **Step 4: Build**

```bash
cd /projects/elohim && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo build -p elohim-storage-client 2>&1 | tail -10
```

Expected: builds. Compile time should DECREASE (fewer transitive deps).

- [ ] **Step 5: Commit**

```bash
git add crates/elohim-storage-client/
git commit -m "refactor(elohim-storage-client): depend on elohim-views for types (drop transitive elohim-storage)

The desktop sidecar client now pulls only the lightweight wire-types
crate, not the full elohim-storage implementation. Validates the new
boundary's compile-cost claim.

Refs: genesis/docs/plans/2026-05-18-sdk-boundary-clarification.md (T4)"
```

---

## Task 5: Server-side consumers add elohim-views (keep elohim-storage)

**Files:**
- Modify: `doorway/doorway-service/Cargo.toml`
- Modify: `steward/node/Cargo.toml`

These consumers ARE the server — they need elohim-storage's implementation. They also get an explicit elohim-views path-dep so their View imports stay clean.

- [ ] **Step 1: Inspect each Cargo.toml**

```bash
grep -E "elohim-storage|elohim-views|elohim-sdk" /projects/elohim/doorway/doorway-service/Cargo.toml
grep -E "elohim-storage|elohim-views|elohim-sdk" /projects/elohim/steward/node/Cargo.toml
```

- [ ] **Step 2: Add elohim-views path-dep to each**

For doorway-service (path: `../../crates/elohim-views`):

```toml
elohim-views = { path = "../../crates/elohim-views" }
```

For steward/node (path adjusts based on depth):

```toml
elohim-views = { path = "../../crates/elohim-views" }
```

- [ ] **Step 3: Update imports**

```bash
grep -rln "elohim_storage::views" /projects/elohim/doorway/doorway-service/src 2>/dev/null
grep -rln "elohim_storage::views" /projects/elohim/steward/node/src 2>/dev/null
```

For each file, change `use elohim_storage::views::*` to `use elohim_views::*`.

- [ ] **Step 4: Build both**

```bash
cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/doorway__doorway-service/dev cargo build 2>&1 | tail -10
cd /projects/elohim/steward/node && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/steward__node/dev cargo build 2>&1 | tail -10
```

- [ ] **Step 5: Commit**

```bash
git add doorway/doorway-service/Cargo.toml steward/node/Cargo.toml \
        <any consumer source files modified>
git commit -m "refactor(doorway,steward): depend on elohim-views for View types

Server-side consumers keep elohim-storage (they ARE the server), but
their View imports now route through elohim-views — clarifies the
boundary and prepares for cargo-deny enforcement.

Refs: genesis/docs/plans/2026-05-18-sdk-boundary-clarification.md (T5)"
```

---

## Task 6: Add cargo-deny boundary rule

**Files:**
- Create: `/projects/elohim/deny.toml`
- Modify: `.husky/pre-push`

- [ ] **Step 1: Install cargo-deny if not present**

```bash
cargo deny --version 2>&1 || cargo install cargo-deny --locked
```

- [ ] **Step 2: Create deny.toml at repo root**

Write to `/projects/elohim/deny.toml`:

```toml
# cargo-deny configuration for the Elohim Protocol repo.
#
# Primary purpose: enforce the elohim-views / elohim-sdk boundary.
# - elohim-views: lightweight Wire-shape types crate (consumers depend here)
# - elohim-sdk: re-export facade with client helpers (consumers depend here)
# - elohim-storage: server-side implementation (only server crates depend here)
#
# Non-server consumers (elohim-storage-client, future third-party SDKs,
# elohim-app build deps) must NOT depend on elohim-storage directly. They
# depend on elohim-views or elohim-sdk instead.
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

# elohim-storage is the implementation; only the server-side wrappers may
# depend on it directly. Everything else uses elohim-views (or the elohim-sdk
# facade that re-exports it).
[[bans.deny]]
name = "elohim-storage"
wrappers = [
    "doorway-service",      # server-side gateway
    "steward-node",         # server-side node runtime
    "elohim-node",          # server-side node deployment wrapper
    "elohim-storage",       # self-reference (dev-deps, etc.)
]

[licenses]
allow = [
    "MIT", "Apache-2.0", "BSD-3-Clause", "BSD-2-Clause", "ISC",
    "Unicode-DFS-2016", "CAL-1.0", "MPL-2.0", "Zlib", "OpenSSL",
]
exceptions = []
unlicensed = "warn"

[sources]
unknown-registry = "warn"
unknown-git = "warn"
allow-registry = [
    "https://github.com/rust-lang/crates.io-index",
    "sparse+https://nexus.ethosengine.com/repository/cargo-internal/",
    "sparse+https://nexus.ethosengine.com/repository/cargo/",
]
```

- [ ] **Step 3: Run cargo-deny against the elohim workspace**

```bash
cd /projects/elohim && RUSTFLAGS="" RUSTC_WRAPPER="" cargo deny --manifest-path elohim/Cargo.toml check bans 2>&1 | tail -30
```

Expected: passes. If it fails because a crate still depends on elohim-storage that shouldn't, either migrate that crate to elohim-views OR add it to the `wrappers` list with a comment explaining why.

- [ ] **Step 4: Add cargo-deny smoke to pre-push**

Read `/projects/elohim/.husky/pre-push`. Find an appropriate location among the project-level checks. Add:

```bash
# cargo-deny bans check — enforces the elohim-views/elohim-sdk boundary
if command -v cargo-deny >/dev/null 2>&1; then
  echo "→ cargo deny check bans (elohim workspace)"
  if ! (cd "$REPO_ROOT" && cargo deny --manifest-path elohim/Cargo.toml check bans 2>&1 | tail -20); then
    echo "✗ cargo-deny failed — boundary violation. See deny.toml for rules."
    exit 1
  fi
fi
```

- [ ] **Step 5: Commit**

```bash
git add deny.toml .husky/pre-push
git commit -m "feat(deny): enforce elohim-views/elohim-sdk boundary via cargo-deny bans

deny.toml at repo root declares elohim-storage as a banned direct dep
except for the explicitly-listed server-side wrappers. Pre-push runs
'cargo deny check bans' against the elohim workspace.

Refs: genesis/docs/plans/2026-05-18-sdk-boundary-clarification.md (T6)"
```

---

## Task 7: Document the TypeScript SDK boundary + the elohim-views story

**Files:**
- Modify: `elohim/sdk/CLAUDE.md`
- Modify: `elohim/elohim-storage/CLAUDE.md`
- Create: `crates/elohim-views/README.md` (already created in T1, possibly extend)

- [ ] **Step 1: Update elohim/sdk/CLAUDE.md with the SDK Boundary section**

Add after the existing top-level intro:

```markdown
## SDK Boundary

The `elohim/sdk/` tree is the **TypeScript SDK** distributed to consumers (browser, doorway clients, future external integrators). Its boundary is:

| Path | Role |
|---|---|
| `elohim/sdk/storage-client-ts/` | Generated wire types + HTTP client; pulled from `crates/elohim-views` via ts-rs |
| `elohim/sdk/epr-ts/` | Generated EPR codec types; pulled from `elohim-epr` via ts-rs |
| `elohim/sdk/schemas/v1/` | JSON Schemas — the authoritative wire contract (drives both Rust and TS codegen) |
| `elohim/sdk/domains/<app>/` | App manifests + companion schemas + per-app codegen scripts |
| `elohim/sdk/src/` | Hand-written SDK helpers (connection management, type guards) |

**Rule:** consumers import from `@elohim/storage-client` and `@elohim/epr-ts`. They do NOT reach into `elohim-storage` internal Rust types, and they do NOT bypass the generated types by reading raw JSON.

For the Rust side of the SDK boundary, see `crates/elohim-views/` (lightweight wire types) and `crates/elohim-sdk/` (consumer-friendly facade). The boundary is enforced by `deny.toml` at the repo root.
```

- [ ] **Step 2: Update elohim/elohim-storage/CLAUDE.md to reflect the new architecture**

Find the "## File Reference" section and update:

```markdown
| File | Purpose |
|------|---------|
| `crates/elohim-views/src/*.rs` | **Wire-shape View types** — ts-rs-anchored, per-domain modules |
| `elohim/elohim-storage/src/views.rs` | Re-export shim + Wire→View `From` impls that touch DB types |
| `elohim/elohim-storage/src/http.rs` | HTTP routes — uses View types via `use elohim_views::...` |
| `elohim/elohim-storage/src/db/models.rs` | Diesel models — internal snake_case |
| `elohim/elohim-storage/src/db/*_diesel.rs` | CRUD operations — internal only |
```

Find "## Adding New Entities" and update step 2:

```markdown
2. **crates/elohim-views/src/<domain>.rs** - Add View type with `#[derive(TS)]` + ts-rs export attribute (camelCase, Value fields)
```

- [ ] **Step 3: Commit**

```bash
git add elohim/sdk/CLAUDE.md elohim/elohim-storage/CLAUDE.md
git commit -m "docs: document elohim-views as the SDK wire-type boundary

Updates the SDK + storage CLAUDE.md files to point at crates/elohim-views
as the canonical home for ts-rs-anchored Wire-shape types.

Refs: genesis/docs/plans/2026-05-18-sdk-boundary-clarification.md (T7)"
```

---

## Self-Review

**Spec coverage:**
- Lightweight wire-types crate created — Task 1
- Atomic migration of all 261 View types — Task 2
- elohim-sdk facade — Task 3
- Lightweight consumer (elohim-storage-client) detransitives — Task 4
- Server-side consumers re-routed — Task 5
- cargo-deny boundary enforcement — Task 6
- Documentation — Task 7

**Placeholder scan:** No TBD/TODO/FILL IN. The big task (T2) does have a "write a script to /tmp" instruction but that's the right pattern — the migration is too large for hand-editing and the script is transient.

**Type consistency:**
- `elohim-views` (singular crate name, plural concept) used consistently
- Path `crates/elohim-views` used consistently throughout
- ts-rs `export_to` retargeted from `../../sdk/...` to `../../elohim/sdk/...` consistently
- Module structure (shared/lamad/shefa/qahal/imagodei/infrastructure/epr/inputs) consistent T1→T2→T7

**Risks documented:**
- ts-rs cross-crate import paths — addressed via single-crate atomic migration; the T4 PILOT failure is the canonical example, memorialized at `[[ts-rs-cross-crate-import-paths]]`
- Migration script complexity — Task 2 acknowledges and budgets for script refinement passes
- Some consumers may have `use elohim_storage::views` references the migration missed — Tasks 4 and 5 grep for this explicitly

**What this plan does NOT cover** (deferred):
- Move `elohim-storage-client` itself to elohim-sdk-client or rename — separate concern
- Migrate `*InputView` to a separate `inputs` crate — combined here in `crates/elohim-views/src/inputs.rs`
- Publish elohim-views to Nexus — blocked behind the Nexus Cargo Basic-auth issue (see `[[nexus-cargo-publish-basic-auth]]`)

**Execution handoff:** Ready for superpowers:subagent-driven-development. Recommended approach: dispatch each task as a separate subagent (T2 needs Sonnet+ for script writing; others can use Sonnet or Haiku).
