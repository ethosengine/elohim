# Monolithic Code Decomposition Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Sibling-module decomposition of the four monolithic files in dependency-respecting order: views.rs convert helpers (Phase A), http.rs (Phase B), p2p/mod.rs (Phase C), content_store/lib.rs (Phase D). Each file ends <500 LOC or removed; DNA hash unchanged; ts-rs output byte-identical; CI green throughout.

**Architecture:** The `graph_views/{lamad,shefa}/` sibling-module pattern from the 2026-05-16 graph-native landing is the exemplar. Each monolith decomposes into a `<file>/` directory with `mod.rs` plus per-concern sibling files. No new crate boundaries; just module-level reasoning scope. DNA hash on `elohim/holochain/dna/elohim/elohim.dna` is the load-bearing invariant for Phase D — verified byte-identical per task.

**Tech Stack:** Rust 2021, hdk macros, axum router, ts-rs codegen, holochain `hc dna hash` CLI for hash verification.

---

## Pre-execution gate

- [ ] Plans 1 (manifests) and 2 (SDK boundary) should be complete before Phase A. If Plan 2 hasn't run, Phase A's work is much larger — note that adjustment in Phase A's intro.
- [ ] Capture the baseline DNA hash: `cd /projects/elohim/elohim/holochain/dna/elohim && hc dna hash elohim.dna > /tmp/dna-hash-baseline.txt` (requires `hc` CLI; if not installed, install per the Holochain docs)
- [ ] Capture the ts-rs baseline: `find /projects/elohim/elohim/sdk/storage-client-ts/src/generated -name '*.ts' | sort | xargs sha256sum | sort > /tmp/ts-rs-baseline.sha256`
- [ ] Capture the LOC baseline:
  ```bash
  wc -l /projects/elohim/elohim/elohim-storage/src/{views.rs,http.rs,p2p/mod.rs} /projects/elohim/elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
  ```

---

# Phase A — views.rs convert-helper cleanup

**Goal:** After Plan 2 lands, `elohim-storage/src/views.rs` is mostly re-exports plus Wire→View conversion functions that need DB-type access. Move those conversion functions into a dedicated `views_convert/` sibling-module tree so views.rs becomes pure re-export. If Plan 2 has NOT landed, this phase is much larger — STOP and run Plan 2 first.

**Files to be created:**
```
elohim/elohim-storage/src/views_convert/
├── mod.rs       # Re-export hub
├── lamad.rs     # Wire→ContentView, Wire→PathView, etc.
├── shefa.rs
├── qahal.rs
├── imagodei.rs
├── infrastructure.rs
├── epr.rs
└── inputs.rs    # Wire ← InputView reverse direction
```

## Task A.1: Verify Plan 2 baseline

- [ ] **Step 1: Confirm views.rs is now a shim**

```bash
wc -l /projects/elohim/elohim/elohim-storage/src/views.rs
```

Expected: <2,000 LOC if Plan 2 landed, <8,500 LOC if not. If still ~8,200 LOC, STOP — run Plan 2 first. The full Plan 2 decomposition is unsafe to attempt inline as part of this phase.

- [ ] **Step 2: Survey what remains**

```bash
grep -nE "^(pub fn|fn|impl)" /projects/elohim/elohim/elohim-storage/src/views.rs | head -40
```

These are the conversion helpers. Note their structure — they likely have signatures like `fn content_to_view(wire: &Content, ctx: &Ctx) -> ContentView`.

- [ ] **Step 3: Classify each conversion function by domain**

Same domain classification as Plan 2 T1. Annotate in `/tmp/views-convert-inventory.md`.

## Task A.2: Create views_convert/ skeleton + move shared helpers

**Files:**
- Create: `elohim-storage/src/views_convert/{mod.rs,shared.rs}`
- Modify: `elohim-storage/src/lib.rs` (add `pub mod views_convert;`)

- [ ] **Step 1: Create the skeleton**

```bash
mkdir -p /projects/elohim/elohim/elohim-storage/src/views_convert
```

Write `/projects/elohim/elohim/elohim-storage/src/views_convert/mod.rs`:

```rust
//! Wire → View conversion helpers.
//!
//! Storage-internal conversion functions live here, organized by domain.
//! These functions touch DB types (Diesel models) and therefore cannot
//! live in elohim-sdk. They produce View types (defined in elohim-sdk)
//! at the HTTP API boundary.

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

Add empty stubs for each domain file:

```bash
for dom in shared lamad shefa qahal imagodei infrastructure epr inputs; do
  cat > /projects/elohim/elohim/elohim-storage/src/views_convert/$dom.rs <<EOF
//! Conversion helpers for $dom — populated in subsequent tasks.
EOF
done
```

- [ ] **Step 2: Register the module in lib.rs**

Read `/projects/elohim/elohim/elohim-storage/src/lib.rs` and find an appropriate place near other `pub mod` declarations. Add:

```rust
pub mod views_convert;
```

- [ ] **Step 3: Build**

```bash
cd /projects/elohim/elohim && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo build -p elohim-storage 2>&1 | tail -10
```

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/views_convert/ elohim/elohim-storage/src/lib.rs
git commit -m "feat(elohim-storage): scaffold views_convert/ sibling module tree

Per-domain stubs for Wire→View conversion helpers. Tasks A.3-A.9 move
the existing helpers from views.rs into these stubs.

Refs: genesis/docs/plans/2026-05-18-monolithic-code-decomposition.md (A.2)"
```

## Tasks A.3 – A.9: Move conversion helpers per domain

**Pattern (apply to each domain — lamad, shefa, qahal, imagodei, infrastructure, epr, inputs):**

- [ ] **Step 1: Identify the helper functions for this domain in views.rs**

Use the inventory from A.1.

- [ ] **Step 2: Move them to `views_convert/<domain>.rs`**

Carry `use` imports. Internal helpers (`fn helper(...)`) move with their primary caller.

- [ ] **Step 3: Update views.rs to remove the moved functions; add a `pub use views_convert::<domain>::*;` re-export if any consumers import via `crate::views::...`**

- [ ] **Step 4: Build + ts-rs**

```bash
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo test export_bindings 2>&1 | tail -5
diff /tmp/ts-rs-baseline.sha256 <(find /projects/elohim/elohim/sdk/storage-client-ts/src/generated -name '*.ts' | sort | xargs sha256sum | sort)
cd /projects/elohim/elohim && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo build --workspace 2>&1 | tail -10
```

Expected: no ts-rs diff (conversion functions are not ts-rs anchored); workspace builds.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/views_convert/<domain>.rs elohim/elohim-storage/src/views.rs
git commit -m "refactor(elohim-storage): move <domain> Wire→View converters to views_convert/<domain>.rs

Refs: genesis/docs/plans/2026-05-18-monolithic-code-decomposition.md (A.<N>)"
```

## Task A.10: Final views.rs reduction

- [ ] **Step 1: Check final views.rs LOC**

```bash
wc -l /projects/elohim/elohim/elohim-storage/src/views.rs
```

Expected: <500 LOC. Just re-exports.

- [ ] **Step 2: If <50 LOC of only re-exports, consider deleting**

Run: `grep -vcE "^$|^//|^pub use" /projects/elohim/elohim/elohim-storage/src/views.rs`

If the count is 0, views.rs is pure boilerplate. Decision: keep as a stable consumer-import anchor (`crate::views::*` still works), OR delete and update consumers to use `crate::views_convert::*` and `elohim_sdk::views::*`. Default: keep the file.

- [ ] **Step 3: Commit any final cleanup**

```bash
git add elohim/elohim-storage/src/views.rs
git commit -m "refactor(elohim-storage): finalize views.rs as thin re-export anchor

Refs: genesis/docs/plans/2026-05-18-monolithic-code-decomposition.md (A.10)"
```

---

# Phase B — http.rs decomposition

**Goal:** Break `elohim-storage/src/http.rs` (10,199 LOC, single `impl HttpServer` block from line 280) into per-domain sibling modules. The natural seam is the per-route grouping visible at line 8145+ where `build_manifest()` registers ~250 routes via `.route(...)` calls.

**Files to be created:**
```
elohim/elohim-storage/src/http/
├── mod.rs               # Re-export hub + HttpServer constructor
├── routes.rs            # build_manifest() — route registrations
├── extractors.rs        # Custom axum extractors (auth, idempotency, etc.)
├── error.rs             # HTTP-layer error conversions
├── handlers/
│   ├── mod.rs
│   ├── lamad.rs         # Content, Path, Mastery handlers
│   ├── shefa.rs         # EconomicEvent, Stewardship handlers
│   ├── qahal.rs         # GovernanceAction, Vote handlers
│   ├── imagodei.rs      # Human, Relationship, Account handlers
│   ├── infrastructure.rs # P2P, Peer, Federation handlers
│   ├── epr.rs           # Epr* endpoints
│   ├── blob.rs          # Blob + shard endpoints
│   ├── identity.rs      # Auth, identity handshake
│   └── admin.rs         # Internal admin endpoints
```

**Files to be modified:**
- `elohim-storage/src/http.rs` — replace with module declaration + temporary shim
- Any consumer in elohim-storage that imports from `crate::http::*`

## Task B.1: Survey http.rs structure

- [ ] **Step 1: Capture the route inventory**

```bash
grep -nE '\.route\(' /projects/elohim/elohim/elohim-storage/src/http.rs > /tmp/http-routes.txt
wc -l /tmp/http-routes.txt
head -50 /tmp/http-routes.txt
```

- [ ] **Step 2: Identify section boundaries**

The `impl HttpServer` block runs from line 280 to ~8000+. Look for handler function definitions:

```bash
grep -nE "^(    async fn|    pub async fn)" /projects/elohim/elohim/elohim-storage/src/http.rs | head -80
```

Each `async fn handler_<name>` is a handler. Domain-classify based on the route path it serves.

- [ ] **Step 3: Write the seam map to /tmp/http-seam-map.md**

For each handler function, record:
- Function name
- Domain (lamad/shefa/qahal/imagodei/infrastructure/epr/blob/identity/admin)
- Approximate LOC

Don't commit this file. It's a working document.

## Task B.2: Create http/ skeleton

- [ ] **Step 1: Create directory structure**

```bash
mkdir -p /projects/elohim/elohim/elohim-storage/src/http/handlers
```

- [ ] **Step 2: Write http/mod.rs**

```rust
//! HTTP API layer.
//!
//! Decomposed per-domain. See submodules:
//! - routes: route registrations (axum Router building)
//! - extractors: custom axum extractors
//! - error: HTTP-layer error conversions
//! - handlers/: per-domain request handlers

pub mod error;
pub mod extractors;
pub mod handlers;
pub mod routes;

pub use routes::build_manifest;
// HttpServer struct and constructor — populated in B.3
```

- [ ] **Step 3: Write http/handlers/mod.rs**

```rust
//! Per-domain HTTP request handlers.

pub mod admin;
pub mod blob;
pub mod epr;
pub mod identity;
pub mod imagodei;
pub mod infrastructure;
pub mod lamad;
pub mod qahal;
pub mod shefa;
```

- [ ] **Step 4: Create empty stubs for each handler file + extractors.rs + error.rs + routes.rs**

```bash
for f in admin blob epr identity imagodei infrastructure lamad qahal shefa; do
  cat > /projects/elohim/elohim/elohim-storage/src/http/handlers/$f.rs <<EOF
//! HTTP handlers for $f — populated in subsequent tasks.
EOF
done
for f in extractors error routes; do
  cat > /projects/elohim/elohim/elohim-storage/src/http/$f.rs <<EOF
//! $f — populated in subsequent tasks.
EOF
done
```

- [ ] **Step 5: Rename existing http.rs to http_old.rs (transient)**

```bash
cd /projects/elohim/elohim/elohim-storage/src && git mv http.rs http_old.rs
```

This is transient — http_old.rs holds the original content during migration and is deleted at end. Update the `pub mod http` declaration in lib.rs to point at both:

```rust
pub mod http;
mod http_old;  // transitional — removed in B.10
```

- [ ] **Step 6: Make http/mod.rs re-export from http_old during migration**

In `http/mod.rs`, add a temporary re-export:

```rust
// Transitional during decomposition — handlers + HttpServer still live in http_old.rs
pub use crate::http_old::*;
```

- [ ] **Step 7: Build**

```bash
cd /projects/elohim/elohim && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo build -p elohim-storage 2>&1 | tail -10
```

Expected: builds clean. The decomposition is a no-op so far.

- [ ] **Step 8: Commit**

```bash
git add elohim/elohim-storage/src/http/ elohim/elohim-storage/src/http_old.rs elohim/elohim-storage/src/lib.rs
git commit -m "feat(elohim-storage/http): scaffold http/ sibling-module skeleton

http.rs renamed to http_old.rs transitionally; http/ re-exports from it.
Subsequent tasks move handlers domain-by-domain into http/handlers/<dom>.rs.

Refs: genesis/docs/plans/2026-05-18-monolithic-code-decomposition.md (B.2)"
```

## Task B.3: Extract HttpServer struct + constructors to http/mod.rs

- [ ] **Step 1: Find HttpServer struct + impl in http_old.rs**

```bash
grep -nE "^(pub struct HttpServer|impl HttpServer)" /projects/elohim/elohim/elohim-storage/src/http_old.rs
```

The struct is likely near the top of the file. The impl block at line 280 (in the original) contains the handlers — those move later, not here.

- [ ] **Step 2: Move the struct definition + non-handler constructors to http/mod.rs**

Identify the constructor `pub fn new(...)`, `pub async fn run(...)`, and similar non-handler infrastructure methods. Move those.

Leave the handler `async fn` methods in `http_old.rs` for B.4-B.8.

- [ ] **Step 3: Update http/mod.rs to remove the transitional re-export of HttpServer (now defined here)**

- [ ] **Step 4: Build**

```bash
cd /projects/elohim/elohim && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo build -p elohim-storage 2>&1 | tail -10
```

Expected: builds. May fail with `cannot find function ...` if a constructor references handlers that still live in http_old — in that case, leave the constructor as a thin wrapper in http/mod.rs that calls `crate::http_old::HttpServerOld::run()` for now, and re-attach in B.10.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/http/mod.rs elohim/elohim-storage/src/http_old.rs
git commit -m "refactor(elohim-storage/http): move HttpServer struct + constructors to http/mod.rs

Refs: genesis/docs/plans/2026-05-18-monolithic-code-decomposition.md (B.3)"
```

## Task B.4: Move blob + shard handlers to http/handlers/blob.rs

The blob/shard endpoints (`/shard/{hash}`, `/blob/{hash}`, `/manifest/{hash}`) are the most self-contained domain. Start here as proof-of-concept.

- [ ] **Step 1: Identify blob/shard handlers**

```bash
grep -nE "async fn .*shard|async fn .*blob|async fn .*manifest" /projects/elohim/elohim/elohim-storage/src/http_old.rs | head -20
```

- [ ] **Step 2: Move each blob/shard handler to http/handlers/blob.rs**

A handler that currently looks like:

```rust
impl HttpServer {
    async fn get_shard(&self, ...) -> ... { ... }
}
```

Becomes (in http/handlers/blob.rs):

```rust
use crate::http::HttpServer;
use axum::{...};

impl HttpServer {
    pub(crate) async fn get_shard(&self, ...) -> ... { ... }
}
```

Note: split impl blocks across files. Rust allows this for inherent impls of structs from the SAME crate as the impl. Since http/mod.rs defines `HttpServer` and http/handlers/blob.rs is in the same crate, this works.

Carry necessary `use` imports.

- [ ] **Step 3: Build**

```bash
cd /projects/elohim/elohim && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo build -p elohim-storage 2>&1 | tail -10
```

Expected: builds. If `cannot find method` errors appear, a handler is referenced from another handler that's still in http_old — temporarily make the moved handler `pub(crate)` and call sites use `HttpServer::get_shard(self, ...)` syntax.

- [ ] **Step 4: Run blob-related tests if any exist**

```bash
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo test --lib http 2>&1 | tail -15
```

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/http/handlers/blob.rs elohim/elohim-storage/src/http_old.rs
git commit -m "refactor(elohim-storage/http): move blob + shard handlers to http/handlers/blob.rs

Refs: genesis/docs/plans/2026-05-18-monolithic-code-decomposition.md (B.4)"
```

## Tasks B.5 – B.9: Move remaining domain handlers

Apply the B.4 recipe to each remaining domain:

- **B.5: identity** — auth, identity handshake, account package endpoints
- **B.6: lamad** — content, path, mastery endpoints
- **B.7: shefa** — economic events, stewardship, collective endpoints
- **B.8: qahal + imagodei** — governance actions, votes, human relationships
- **B.9: infrastructure + epr + admin** — P2P, peer status, EPR endpoints, internal admin

Each follows the same pattern: move handlers, build, test (where coverage exists), commit.

Commit message template:

```
refactor(elohim-storage/http): move <domain> handlers to http/handlers/<domain>.rs

Refs: genesis/docs/plans/2026-05-18-monolithic-code-decomposition.md (B.<N>)
```

## Task B.10: Move routes.rs + retire http_old.rs

- [ ] **Step 1: Move `build_manifest()` (lines 8145+ in original) to http/routes.rs**

```bash
grep -n "pub fn build_manifest" /projects/elohim/elohim/elohim-storage/src/http_old.rs
```

Move the function. It references handlers via `HttpServer::<handler_name>` — those references work transparently since the inherent impls still attach to the same struct.

- [ ] **Step 2: Verify http_old.rs is now empty (or near-empty)**

```bash
wc -l /projects/elohim/elohim/elohim-storage/src/http_old.rs
```

If <100 LOC, it's mostly leftover comments/imports. Delete the file.

```bash
git rm /projects/elohim/elohim/elohim-storage/src/http_old.rs
```

If there's substantive remaining code, it wasn't classified into any domain — flag in `/tmp/http-leftover.md`, add a "Phase B leftovers" section to this plan, and move them in an additional B.11 task before retiring http_old.rs.

- [ ] **Step 3: Remove the `mod http_old;` declaration from lib.rs**

- [ ] **Step 4: Final LOC check**

```bash
wc -l /projects/elohim/elohim/elohim-storage/src/http/*.rs /projects/elohim/elohim/elohim-storage/src/http/handlers/*.rs
```

Expected: each file <2,000 LOC, most <1,000.

- [ ] **Step 5: Full workspace build + tests**

```bash
cd /projects/elohim/elohim && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo build --workspace 2>&1 | tail -10
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo test --lib 2>&1 | tail -20
```

Expected: builds and tests green.

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/http/routes.rs elohim/elohim-storage/src/lib.rs
git rm elohim/elohim-storage/src/http_old.rs
git commit -m "refactor(elohim-storage/http): retire http_old.rs; routes.rs holds build_manifest()

http.rs decomp complete. http/handlers/ holds per-domain handlers,
http/routes.rs holds the route registration table, http/mod.rs holds
HttpServer and lifecycle.

Refs: genesis/docs/plans/2026-05-18-monolithic-code-decomposition.md (B.10)"
```

---

# Phase C — p2p/mod.rs decomposition

**Goal:** Break `elohim-storage/src/p2p/mod.rs` (6,279 LOC). 31 sub-modules are already declared (lines 31-61); the heavy lifting is in the `impl P2PNode` block starting at line 1501 (~4,700 LOC). Split that impl by protocol concern.

**Files to be created:**
```
elohim/elohim-storage/src/p2p/
├── mod.rs                     # Reduce to type definitions + module declarations + handle types
├── node/                      # NEW subdirectory
│   ├── mod.rs                 # impl P2PNode partial — constructor + run loop
│   ├── command_handler.rs     # impl P2PNode — handle_command
│   ├── event_loop.rs          # impl P2PNode — event_loop body
│   ├── reconciliation.rs      # impl P2PNode — reconciliation/drain methods
│   ├── peer_management.rs     # impl P2PNode — peer add/remove/query
│   └── reach_gate.rs          # pub fn reach_gate_allows + tests (move from end of mod.rs)
```

**Note:** the 31 existing submodules (`adapters`, `attention_tending`, `behaviour`, `blob_fetch`, etc.) STAY. This phase only splits the giant `impl P2PNode` block.

## Task C.1: Survey impl P2PNode

- [ ] **Step 1: Map the impl block contents**

```bash
awk 'NR>=1501 && NR<=6230' /projects/elohim/elohim/elohim-storage/src/p2p/mod.rs | grep -nE "^    (pub )?(async )?fn" | head -60
```

Each function in the impl block is a candidate for grouping.

- [ ] **Step 2: Group by concern**

Common groupings in P2PNode impl blocks:
- **command_handler**: `handle_command`, dispatch matchers for P2PCommand variants
- **event_loop**: the main `event_loop` async fn + helpers that drive libp2p events
- **reconciliation**: drain/sync/reconcile methods
- **peer_management**: add_peer, remove_peer, peer queries
- **reach_gate**: the `reach_gate_allows` function near line 6215 + its tests

Write the grouping to `/tmp/p2p-impl-grouping.md`.

- [ ] **Step 3: No commit — research only**

## Task C.2: Create p2p/node/ skeleton + move reach_gate

**Files:**
- Create: `elohim-storage/src/p2p/node/{mod.rs,reach_gate.rs}` (start small)
- Modify: `elohim-storage/src/p2p/mod.rs`

reach_gate is the smallest, most self-contained candidate. Start there as proof-of-concept.

- [ ] **Step 1: Create the directory**

```bash
mkdir -p /projects/elohim/elohim/elohim-storage/src/p2p/node
```

- [ ] **Step 2: Write p2p/node/mod.rs (initially just declaring submodules)**

```rust
//! P2PNode impl decomposed by protocol concern.

mod reach_gate;
pub use reach_gate::reach_gate_allows;
```

- [ ] **Step 3: Move `pub fn reach_gate_allows` + `mod reach_gate_tests` to p2p/node/reach_gate.rs**

From p2p/mod.rs lines 6215-6275 (approximately):

```rust
//! Reach-gate decision function.

use ...; // copy necessary imports from p2p/mod.rs

pub fn reach_gate_allows(...) -> ReachDecision {
    ...
}

#[cfg(test)]
mod tests {
    use super::*;
    // tests body
}
```

Remove the corresponding lines from p2p/mod.rs.

- [ ] **Step 4: Wire node/ into p2p/mod.rs**

Add to the top-level `pub mod` declarations in p2p/mod.rs:

```rust
pub mod node;
```

- [ ] **Step 5: Update any consumer that calls `crate::p2p::reach_gate_allows` to use `crate::p2p::node::reach_gate_allows` OR add a re-export in p2p/mod.rs**

The cleanest fix is a re-export at the top of p2p/mod.rs:

```rust
pub use node::reach_gate_allows;
```

- [ ] **Step 6: Build**

```bash
cd /projects/elohim/elohim && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo build -p elohim-storage 2>&1 | tail -10
```

- [ ] **Step 7: Run p2p tests**

```bash
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo test --lib p2p 2>&1 | tail -15
```

- [ ] **Step 8: Commit**

```bash
git add elohim/elohim-storage/src/p2p/node/ elohim/elohim-storage/src/p2p/mod.rs
git commit -m "refactor(elohim-storage/p2p): create node/ subtree; move reach_gate

reach_gate_allows + tests now live in p2p/node/reach_gate.rs. Subsequent
tasks decompose the impl P2PNode block by concern (command_handler,
event_loop, reconciliation, peer_management).

Refs: genesis/docs/plans/2026-05-18-monolithic-code-decomposition.md (C.2)"
```

## Tasks C.3 – C.6: Move impl P2PNode methods by concern

Apply the same pattern as Phase B's handler moves. For each grouping (command_handler, event_loop, reconciliation, peer_management):

- [ ] **Step 1: Identify the methods in the grouping**

From the inventory in C.1.

- [ ] **Step 2: Move them to `p2p/node/<group>.rs`**

The pattern for split inherent impls works the same way as B.4. The receiving file:

```rust
//! <group> methods on P2PNode.

use super::super::*;  // pull in types from p2p/mod.rs
// or specifically: use crate::p2p::{P2PNode, P2PCommand, ...};

impl P2PNode {
    pub(crate) async fn handle_command(&mut self, cmd: P2PCommand) { ... }
    pub(crate) fn dispatch_inner(...) { ... }
}
```

- [ ] **Step 3: Build + test**

```bash
cd /projects/elohim/elohim && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo build -p elohim-storage 2>&1 | tail -10
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo test --lib p2p 2>&1 | tail -10
```

- [ ] **Step 4: Commit**

```
refactor(elohim-storage/p2p/node): move <group> methods to <group>.rs

Refs: genesis/docs/plans/2026-05-18-monolithic-code-decomposition.md (C.<N>)
```

## Task C.7: Final p2p/mod.rs reduction

- [ ] **Step 1: Check p2p/mod.rs LOC**

```bash
wc -l /projects/elohim/elohim/elohim-storage/src/p2p/mod.rs
```

Expected: <2,000 LOC. mod.rs now contains: the existing 31 `pub mod` declarations, the type definitions (DeliveryPeer, P2PConfig, P2PNode struct, PeerMetrics, ReconciliationMetrics, etc.), and the new `pub mod node;` line.

- [ ] **Step 2: If any type definitions are heavy (DeliveryPeer + P2PConfig + P2PNode + PeerMetrics combined >1,500 LOC), consider extracting types to `p2p/types.rs`**

Decision: defer this unless someone has a specific reason to act now. The type defs are stable; moving them is low-value churn.

- [ ] **Step 3: Run full p2p test suite**

```bash
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo test --lib p2p 2>&1 | tail -20
```

- [ ] **Step 4: Commit any final cleanup**

```bash
# (Likely no changes; this is a verification task.)
```

---

# Phase D — content_store/lib.rs decomposition

**Goal:** Break `holochain/dna/elohim/zomes/content_store/src/lib.rs` (12,197 LOC, largest single file) into sibling modules. **DNA hash MUST remain unchanged** — this is the load-bearing invariant. Verified byte-identical per task.

**Why DNA hash matters:** changing module layout inside a Holochain zome can affect the compiled WASM bytecode if `pub use` re-exports drift, which changes the DNA hash, which orphans every existing cell. The HC 0.6 unstable-migration path is the only clean way to handle a hash change; we avoid that path here by re-exporting carefully.

**Files to be created:**
```
holochain/dna/elohim/zomes/content_store/src/
├── lib.rs                      # Reduced to module declarations + re-exports
├── conversions/                # Wire-type conversion helpers (lines ~200-859 of original lib.rs)
│   ├── mod.rs
│   ├── lamad.rs                # Lamad wire conversion helpers
│   └── avodah.rs               # Avodah wire conversion helpers
├── bridges/                    # Cross-DNA calls (lines ~860-1112)
│   ├── mod.rs
│   └── imagodei.rs
├── lifecycle/                  # init + cache config + import config (lines ~1113-1779)
│   ├── mod.rs
│   ├── init.rs
│   ├── doorway_cache.rs
│   └── doorway_import.rs
├── io_types/                   # Input/Output types (lines ~1780-2346)
│   ├── mod.rs
│   ├── content.rs
│   ├── blob.rs
│   ├── relationship.rs
│   ├── human.rs
│   ├── agent.rs
│   ├── attestation.rs
│   ├── path.rs
│   ├── progress.rs
│   ├── shard.rs
│   ├── emergency.rs
│   ├── category.rs
│   └── mastery.rs
├── content_crud/               # CRUD operations (lines ~2347-2462)
│   ├── mod.rs
│   └── (one file per major operation or grouped reasonably)
├── import_batch/               # Import batch processing (lines ~2463-2993)
│   ├── mod.rs
│   └── ...
├── queries/                    # Content queries (lines ~2994-3162, 3220-3414)
│   ├── mod.rs
│   ├── by_id.rs
│   ├── by_type.rs
│   ├── by_tag.rs
│   ├── by_human.rs
│   └── stats.rs
├── recovery/                   # Recovery M4 cross-DNA queries (lines ~3163-3434)
│   ├── mod.rs
│   └── ...
└── blob_ops/                   # Blob operations Phase 1 (lines ~3435+)
    ├── mod.rs
    └── ...
```

**Files to be modified:**
- `holochain/dna/elohim/zomes/content_store/src/lib.rs` — reduced to <500 LOC of `pub mod` declarations and selective `pub use` re-exports

## Task D.1: Establish DNA-hash verification harness

**Files:**
- Create: `/projects/elohim/genesis/scripts/verify-dna-hash.sh` (helper script — committed)

- [ ] **Step 1: Capture baseline DNA hash**

```bash
cd /projects/elohim/elohim/holochain/dna/elohim
hc dna pack . 2>&1 | tail -5  # Ensure elohim.dna is built
hc dna hash elohim.dna > /tmp/dna-hash-baseline.txt
cat /tmp/dna-hash-baseline.txt
```

If `hc` is not installed: `cargo install holochain_cli --features unstable-functions` (or use the version your project pins via nix shell).

- [ ] **Step 2: Write the verification helper**

Create `/projects/elohim/genesis/scripts/verify-dna-hash.sh`:

```bash
#!/usr/bin/env bash
# Verify that the elohim DNA hash matches the captured baseline.
# Used per-task during the content_store decomposition.
set -euo pipefail

BASELINE="${1:-/tmp/dna-hash-baseline.txt}"
DNA_DIR="/projects/elohim/elohim/holochain/dna/elohim"

if [ ! -f "$BASELINE" ]; then
  echo "✗ Baseline file not found: $BASELINE"
  echo "  Capture it first: cd $DNA_DIR && hc dna pack . && hc dna hash elohim.dna > $BASELINE"
  exit 1
fi

cd "$DNA_DIR"
hc dna pack . 2>&1 | tail -3
CURRENT=$(hc dna hash elohim.dna)
EXPECTED=$(cat "$BASELINE")

if [ "$CURRENT" = "$EXPECTED" ]; then
  echo "✓ DNA hash unchanged: $CURRENT"
  exit 0
else
  echo "✗ DNA hash CHANGED!"
  echo "  Expected: $EXPECTED"
  echo "  Got:      $CURRENT"
  echo ""
  echo "  This is a HARD STOP. Module re-exports must be reviewed —"
  echo "  Rust struct ordering or pub use drift has affected WASM bytecode."
  exit 1
fi
```

```bash
chmod +x /projects/elohim/genesis/scripts/verify-dna-hash.sh
```

- [ ] **Step 3: Verify the helper works against the baseline**

```bash
bash /projects/elohim/genesis/scripts/verify-dna-hash.sh
```

Expected: `✓ DNA hash unchanged`.

- [ ] **Step 4: Commit**

```bash
git add genesis/scripts/verify-dna-hash.sh
git commit -m "feat(genesis/scripts): DNA hash verification harness

Used per-task during Phase D content_store decomposition to ensure
module layout changes don't drift the elohim DNA hash.

Refs: genesis/docs/plans/2026-05-18-monolithic-code-decomposition.md (D.1)"
```

## Task D.2: Move conversion helpers (lamad + avodah) to conversions/

**Files:**
- Create: `holochain/dna/elohim/zomes/content_store/src/conversions/{mod.rs,lamad.rs,avodah.rs}`
- Modify: `holochain/dna/elohim/zomes/content_store/src/lib.rs`

Lines ~200-859 contain Wire conversion helpers grouped by source domain (lamad lines 202-645, avodah lines 647-859 per the section markers found earlier).

- [ ] **Step 1: Read the exact line ranges**

```bash
grep -n "^// ==" /projects/elohim/elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs | head -8
```

Confirm the section boundaries against the plan's references.

- [ ] **Step 2: Create the conversions/ subdirectory**

```bash
mkdir -p /projects/elohim/elohim/holochain/dna/elohim/zomes/content_store/src/conversions
```

- [ ] **Step 3: Write conversions/mod.rs**

```rust
//! Wire-type conversion helpers between integrity entries and shared crates.

pub mod lamad;
pub mod avodah;
```

- [ ] **Step 4: Move lamad conversion helpers to conversions/lamad.rs**

Cut lines 202-645 (the lamad conversion section per the earlier survey) and paste into `conversions/lamad.rs`. Add module-level doc comment + necessary `use` statements at top.

Critical: the moved functions need to retain their EXACT signatures and visibility. If a function was `pub fn`, keep it `pub fn`. If `pub(crate) fn`, keep it. If implicit `fn` (private), keep it. The DNA hash is sensitive to the public function signatures exported from the zome.

- [ ] **Step 5: Re-export from lib.rs to preserve the public surface**

In lib.rs, replace the deleted section with:

```rust
pub mod conversions;
pub use conversions::lamad::*;  // preserve all the conversion helpers as crate-level visible
pub use conversions::avodah::*;
```

This ensures `content_store::<helper_name>` still resolves the same way.

- [ ] **Step 6: Move avodah conversion helpers to conversions/avodah.rs**

Same recipe.

- [ ] **Step 7: Pack DNA + verify hash**

```bash
bash /projects/elohim/genesis/scripts/verify-dna-hash.sh
```

Expected: `✓ DNA hash unchanged`. If hash CHANGED, STOP. The re-exports may have dropped or reordered a function. Re-inspect lib.rs for missing `pub use` lines.

- [ ] **Step 8: Run sweettest if available**

```bash
cd /projects/elohim/elohim/holochain/tests/sweettest && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__holochain__tests__sweettest/dev cargo test --test content_store 2>&1 | tail -15
```

(Substitute the correct test target if `content_store` isn't the right one — check `holochain/tests/sweettest/Cargo.toml` for `[[test]]` entries.)

- [ ] **Step 9: Commit**

```bash
git add elohim/holochain/dna/elohim/zomes/content_store/src/conversions/ \
        elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
git commit -m "refactor(content_store): move Wire conversion helpers to conversions/

Lamad and Avodah Wire→integrity conversion functions now live in
conversions/{lamad,avodah}.rs. lib.rs re-exports them with pub use to
preserve the public function set; DNA hash verified byte-identical.

Refs: genesis/docs/plans/2026-05-18-monolithic-code-decomposition.md (D.2)"
```

## Task D.3: Move cross-DNA bridges to bridges/imagodei.rs

**Files:**
- Create: `content_store/src/bridges/{mod.rs,imagodei.rs}`
- Modify: `content_store/src/lib.rs`

Lines ~860-1112: "Cross-DNA Bridge Calls to Imagodei".

- [ ] **Step 1: Read the section**

- [ ] **Step 2: Create bridges/ subdirectory + move**

```bash
mkdir -p /projects/elohim/elohim/holochain/dna/elohim/zomes/content_store/src/bridges
```

Apply the D.2 recipe with the bridge functions.

- [ ] **Step 3: Verify DNA hash + commit**

```bash
bash /projects/elohim/genesis/scripts/verify-dna-hash.sh
git add elohim/holochain/dna/elohim/zomes/content_store/src/bridges/ \
        elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
git commit -m "refactor(content_store): move cross-DNA bridges to bridges/imagodei.rs

DNA hash verified unchanged.

Refs: genesis/docs/plans/2026-05-18-monolithic-code-decomposition.md (D.3)"
```

## Task D.4: Move lifecycle (init + doorway cache + doorway import) to lifecycle/

**Files:**
- Create: `content_store/src/lifecycle/{mod.rs,init.rs,doorway_cache.rs,doorway_import.rs}`
- Modify: `content_store/src/lib.rs`

Lines ~1113-1779: DNA Initialization, Doorway Cache Configuration, Doorway Import Config.

- [ ] **Step 1: Identify the `#[hdk_extern]` functions in this range**

```bash
grep -nE "^#\[hdk_extern\]" /projects/elohim/elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs | head -10
```

The `#[hdk_extern]` attribute is what defines the WASM-exported function set — these are EXTRA-sensitive to layout changes.

- [ ] **Step 2: Move each section to its file**

Carry `#[hdk_extern]` attributes with the function. Preserve the function names exactly.

- [ ] **Step 3: Re-export from lib.rs**

```rust
pub mod lifecycle;
pub use lifecycle::init::init;
pub use lifecycle::doorway_cache::__doorway_cache_rules;
pub use lifecycle::doorway_import::__doorway_import_config;
```

The `pub use` re-exports are what preserve the WASM export set. Verify against the original `#[hdk_extern]` list.

- [ ] **Step 4: Verify DNA hash + sweettest + commit**

```bash
bash /projects/elohim/genesis/scripts/verify-dna-hash.sh
git add elohim/holochain/dna/elohim/zomes/content_store/src/lifecycle/ \
        elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
git commit -m "refactor(content_store): move lifecycle (init/cache/import config) to lifecycle/

DNA hash verified unchanged.

Refs: genesis/docs/plans/2026-05-18-monolithic-code-decomposition.md (D.4)"
```

## Tasks D.5 – D.10: Move io_types, content_crud, import_batch, queries, recovery, blob_ops

Apply the same recipe to each section. For each:

- [ ] **Step 1: Identify the line range from the section-marker grep at plan-start**
- [ ] **Step 2: Move to the appropriate sibling-module directory**
- [ ] **Step 3: Re-export from lib.rs**
- [ ] **Step 4: Verify DNA hash byte-identical**
- [ ] **Step 5: Run relevant sweettest scenarios**
- [ ] **Step 6: Commit**

Mapping:

| Task | Section | Original lines (approx) | Sibling module |
|---|---|---|---|
| D.5 | Input/Output types — Content + Blob + Relationship + Human + Agent + Progress + Attestation + Path + Progress + Shard + Emergency + Category + Mastery | 1780-2346 | `io_types/{content,blob,relationship,human,agent,attestation,path,progress,shard,emergency,category,mastery}.rs` |
| D.6 | Content CRUD operations | 2347-2462 | `content_crud/` |
| D.7 | Import batch processing | 2463-2993 | `import_batch/` |
| D.8 | Content query operations (by_type, by_tag, by_human, paginated, stats) | 2994-3162, 3311-3414 | `queries/` |
| D.9 | Recovery M4 cross-DNA queries | 3163-3310 | `recovery/` |
| D.10 | Blob operations Phase 1 | 3435+ | `blob_ops/` |

Each task gets a commit, each task verifies DNA hash byte-identical.

## Task D.11: Final lib.rs reduction

- [ ] **Step 1: Check current lib.rs LOC**

```bash
wc -l /projects/elohim/elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
```

Expected: <500 LOC of `pub mod` and `pub use` declarations plus crate-level docs.

- [ ] **Step 2: If anything substantive remains, classify it**

`grep -nE "^(pub fn|fn|impl|pub struct|pub enum)" /projects/elohim/elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs`

Anything ungrouped is "orphan" code — discuss with operator whether it belongs in a new module or stays as a top-level convenience.

- [ ] **Step 3: Final DNA hash verification + full sweettest**

```bash
bash /projects/elohim/genesis/scripts/verify-dna-hash.sh
cd /projects/elohim/elohim/holochain/tests/sweettest && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__holochain__tests__sweettest/dev cargo test 2>&1 | tail -30
```

Expected: hash unchanged; sweettest all green.

- [ ] **Step 4: Commit any final cleanup**

```bash
git add elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
git commit -m "refactor(content_store): finalize lib.rs as module-declaration shell

content_store decomp complete. lib.rs is <500 LOC of module declarations
+ public re-exports; per-concern code lives in sibling modules. DNA hash
verified byte-identical against pre-decomp baseline.

Refs: genesis/docs/plans/2026-05-18-monolithic-code-decomposition.md (D.11)"
```

---

## Final verification (end of Plan 3)

- [ ] **Step 1: LOC report**

```bash
echo "=== Final monolith sizes ==="
wc -l /projects/elohim/elohim/elohim-storage/src/{views.rs,http.rs,p2p/mod.rs} 2>/dev/null
wc -l /projects/elohim/elohim/elohim-storage/src/http/*.rs /projects/elohim/elohim/elohim-storage/src/http/handlers/*.rs
wc -l /projects/elohim/elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
echo "=== Per-domain sibling-module files ==="
find /projects/elohim/elohim/elohim-storage/src/{http,p2p/node,views_convert} -name "*.rs" -exec wc -l {} \;
find /projects/elohim/elohim/holochain/dna/elohim/zomes/content_store/src -name "*.rs" -not -path "*/target/*" -exec wc -l {} \;
```

Expected:
- views.rs: <500 LOC (re-export anchor)
- http.rs: removed (replaced by http/ subtree)
- p2p/mod.rs: <2,000 LOC (type defs + module decls)
- content_store/lib.rs: <500 LOC (module decls + re-exports)
- Each per-domain sibling file: <2,000 LOC

- [ ] **Step 2: Full workspace build + test**

```bash
cd /projects/elohim/elohim && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo build --workspace --release 2>&1 | tail -10
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo test --lib 2>&1 | tail -30
```

- [ ] **Step 3: DNA hash final verification**

```bash
bash /projects/elohim/genesis/scripts/verify-dna-hash.sh
```

- [ ] **Step 4: ts-rs output final verification**

```bash
diff /tmp/ts-rs-baseline.sha256 <(find /projects/elohim/elohim/sdk/storage-client-ts/src/generated -name '*.ts' | sort | xargs sha256sum | sort)
```

- [ ] **Step 5: Re-measure compilation baseline**

```bash
rm -rf /projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev
cd /projects/elohim/elohim && (time RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo build --workspace --release 2>&1) 2>&1 | tail -5
```

Compare against the 2026-05-17 baseline (25.6s cold). Expectation: similar or marginally better — sibling-module reorg doesn't directly change compile time, but the smaller per-file context can enable rustc incremental optimizations.

- [ ] **Step 6: Update measurements doc**

Append a "Post-decomp" section to `genesis/docs/measurements/2026-05-17-compilation-load-baseline.md` capturing the post-Plan-3 numbers.

```bash
git add genesis/docs/measurements/2026-05-17-compilation-load-baseline.md
git commit -m "docs(measurements): append post-decomp build measurements

Plan 3 (monolithic code decomposition) closure measurements.

Refs: genesis/docs/plans/2026-05-18-monolithic-code-decomposition.md"
```

---

## Self-Review (already performed by plan author)

**Spec coverage:**
- views.rs convert-helper cleanup — Phase A (Tasks A.1-A.10)
- http.rs decomposition — Phase B (Tasks B.1-B.10)
- p2p/mod.rs decomposition — Phase C (Tasks C.1-C.7)
- content_store/lib.rs decomposition — Phase D (Tasks D.1-D.11)
- DNA hash stability verified per task — Phase D throughout
- ts-rs output byte-identical verified per task — Phase A throughout, final verification in closure

**Placeholder scan:** No `TBD`, `TODO`, `FILL IN`. Tasks D.5-D.10 use a tabular form (each section + line range + target module) which is the same pattern, deliberately compact. Each row IS a task; each row gets a commit; the pattern (Steps 1-6) is identical across them.

**Type consistency:**
- `HttpServer` referenced in B.3 (move), B.4 (split impl), B.10 (final) — same name throughout
- `P2PNode` referenced in C.1 (survey), C.2-C.6 (move), C.7 (final) — same name throughout
- `verify-dna-hash.sh` helper created in D.1, used in D.2-D.11 — single canonical path
- Sibling-module directory names match across Phase D table and "Files to be created" header

**Risks documented in-plan:**
- DNA hash sensitivity to module layout — verification harness in D.1, called per Phase D task
- ts-rs path-dependent ordering — baseline captured at plan-start, diffed per task in Phase A
- Split inherent impls only work within the same crate — B.4 and C.3 acknowledge this
- Phase A presumes Plan 2 landed — A.1 STOP-rule if views.rs is still ~8,200 LOC

**Execution handoff:** Ready for superpowers:subagent-driven-development. Recommended phase ordering: A → B → C → D. Phases B and C can run in parallel (different files). Phase D is last (highest risk).
