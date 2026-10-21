---
id: placement-eprfs-slice1-e1-report
title: eprfs slice 1, E1 — View and the view-scoped walk over any EprfsStorage
status: DONE_WITH_CONCERNS
class: architecture
gap: plans__2026-10-20-placement-contract-architecture-plan#5
actor: agent:implementer@claude-opus-5-5
habits: [materialize-what-is-read]
commits: []
cites:
  - "placement-eprfs-slice1-brief | eprfs slice 1 | sha256:46be8b35cc71693b | path: genesis/docs/superpowers/plans/task-placement-eprfs-slice1-brief.md"
  - "placement-contract-architecture-plan | The placement contract's architecture | sha256:2148e5a4de372761 | path: genesis/docs/superpowers/plans/2026-10-20-placement-contract-architecture-plan.md"
  - elohim/eprfs/eprfs-core/.epr-meta
---

# E1 — View and the view-scoped walk

Item E1, gap `#5`. Built from the brief's drafted plan, with ruling R6 applied. Nothing is staged,
committed or pushed: the brief rules architecture before landing.

**Gate evidence.** `just gate eprfs` (run from `/projects/elohim`, cargo berth claimed with
`berth claim cargo --class verify --ttl 3600` and released after) ran `cargo fmt --check`,
`cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`: 1,234 tests passed
and 0 failed, across 76 suites. The run printed `EXIT=0`. The first gate run printed `EXIT=1` because of
three `clippy::explicit_auto_deref` errors in `walk.rs`. They were fixed and the gate was run again.

## What was built

- **`elohim/eprfs/eprfs-core/src/walk.rs`** (new):
  - **`TreePath`** is a path of byte components, so it handles non-UTF-8 names. It also has
    `TryFrom<&ProjectionPath>` and a lossy `Display` that prints the root as `<root>`.
  - **`View`** is a normalized set in which no path is a prefix of another. It has `placement()`, which
    returns `InView`, `Spine` or `Outside`. The doc comment carries the prefix rules.
  - **`walk()`** returns a `TreeWalk` pull cursor with `async fn next()`, plus `stats()`. There is no
    `futures` dependency and no `Stream`.
  - **`walk_collect()`** returns a `WalkedView`. Its `leaves_in_view()` gives the set of leaves that a
    sparse materializer would fetch.
  - **Item types:** `WalkItem` is `Node`, `Boundary` or `Absent`. `AbsentReason` is `NotFound`,
    `ThroughLeaf` or `NotHeld { missing_at, cid }`. `NotHeld` is the R6 variant.
- **`elohim/eprfs/eprfs-core/src/tree.rs`**:
  - `valid_name` and `fetch_checked` are widened to `pub(crate)`.
  - The new `pub(crate)` items are `Budget`, `fetch_node` and `check_root_codec`. `load_verified_tree` is
    refactored onto them, keeping the same order of checks and the same `&'static str` labels. All of
    the existing loader tests pass unchanged.
  - Five `TreeError` variants are added: `PathNotFound`, `NotADirectory`, `NotALeaf`, `ExternalBoundary`
    and `InvalidView`.
- **`elohim/eprfs/eprfs-core/src/lib.rs`**: `pub mod walk;`, `#[cfg(test)] mod test_support;` and the
  re-exports listed in §1.7 of the brief.

## What held

- No new dependencies were added, and nothing in any `Cargo.toml` changed.
- The `EprfsStorage` trait is unchanged.
- There is no git meaning in eprfs-core. External entries stay opaque boundaries.
- No hash or codec check was written again. Every node goes through `fetch_checked` and then
  `decode_tree`, using the CID from the link in its parent.
- **R6:**
  - A node the store does not hold yields `Absent { NotHeld }`, and the walk continues. The cursor never
    waits or retries.
  - A held node whose bytes fail their address check is still `Integrity`. A node that fails to decode is
    still `Invalid`. A storage-side corruption refusal is still `Storage`.
  - The walk turns only `TreeError::Incomplete` into an absence. `fetch_checked` raises it only for
    `BlobNotFound` or `BlobNotLocal`.
- **TreeLimits apply to visited nodes only.** Depth and entries are counted for visited nodes. Path bytes
  are checked for each view path before any fetch, and for each in-view or spine child. A node reached
  again through another path is charged its bytes again, as `"expanded bytes"`. The frontier length is
  checked as `"pending entries"`.

## Concerns

1. **R6 says "local nodes stream first".** Slice 1 has only one tier, so under `LocalOnly` there is
   nothing to put in order. The cursor yields in pre-order. When slice 2 adds a remote tier, it has to
   decide which held nodes go out ahead of the remote fetches, for example with a held-first pass over the
   frontier.
2. **Memory is not bounded by the frontier alone.** The cursor keeps a cache of the nodes it has visited,
   so that a shared subtree is fetched once. The cache is bounded by `max_total_bytes` through the
   budget. The doc comment says so.
3. **The spec and the code still disagree in the places listed in §6 of the brief:**
   - The walk is a free function, not a trait method.
   - It is a pull cursor, not a `Stream`.
   - The view is matched on byte `TreePath`s.
   - The eprfs build-manifest describes its test step as "nextest", but the gate runs `cargo test`.

   All of these are left as they are for the reviewer seat.
4. **Paths left for other seats.** The habit flip to red and its `checks:` line belong to the E4 seat. The
   brit-sealed probe, `eprfs-storage/examples/walk_view.rs`, belongs to the E5 seat. Neither was touched
   here.

No rule floor seemed warranted; the work stayed inside the brief's boundaries.
