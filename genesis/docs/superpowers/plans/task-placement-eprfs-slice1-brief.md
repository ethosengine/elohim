---
id: placement-eprfs-slice1-brief
title: eprfs slice 1 — the view-scoped walk and its three queries over any EprfsStorage
status: Active
class: architecture
gap: plans__2026-10-20-placement-contract-architecture-plan#5
actor: agent:implementer@claude-opus-5-5
habits: [materialize-what-is-read]
cites:
  - "placement-contract-architecture-plan | The placement contract's architecture | sha256:d2a76864bcbbebd1 | path: genesis/docs/superpowers/plans/2026-10-20-placement-contract-architecture-plan.md"
  - "what-every-app-inherits-placement-contract-design | What every app inherits | sha256:2ae965e98bf63d04 | path: genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md"
  - elohim/eprfs/eprfs-core/.epr-meta
---

One seat implements plan items E1, E2 and E3 (gaps `#5`, `#6`, `#7`) from this brief and writes one report per
item: `task-placement-eprfs-slice1-e1-report.md`, `-e2-`, `-e3-`, each with its own `gap:`. E4 and E5 are
separate seats.

Rulings in force beyond the plan's: the orchestrator's rulings R1–R6 recorded on the plan (`epr flow note`).
R6 changes the plan below in one place, stated first.

## The one change from the drafted plan (ruling R6)

A node the store does not hold under the view is **not** an error. The walk yields
`WalkItem::Absent { view_path, reason: AbsentReason::NotHeld { missing_at, cid } }` and continues with every
other prefix of the view. Under `FetchPolicy::LocalOnly` it never waits. A node that is held but fails its
address check is still `TreeError::Integrity`, and a decode failure is still an error: integrity is never
degraded to absence. `read_path` and `list_dir` over a path whose spine is not held return
`TreeError::Incomplete` naming the path, because a single answer cannot be partial. Replace drafted tests 17
and 28's "fails promptly with Incomplete" for the walk with: a walk under LocalOnly over a tree with one
subtree removed yields the held paths and one named `NotHeld` absence, and returns without error; keep the
`Incomplete` expectation for `read_path`.

## Ruling 7 in slice 1 (added mid-lane)

Spec ruling 7: every unanswered outcome carries a typed reason, when to come back, and what can be done
locally meanwhile. In slice 1 the walk's `Absent` item already is the reason (`NotHeld`, `NotFound`,
`ThroughLeaf`) and the walk's held paths are the local alternative. Give `AbsentReason::NotHeld` the field a
later tier needs to say when to come back without a breaking change: an `#[non_exhaustive]` enum, so slice 2
can add the tiers tried and the retry time. Do not invent a shared cross-crate "unanswered" type here: the
interfaces lane names it, and slice 2 adopts it.

## Boundaries (from the eprfs .epr-meta rules; read them)

- `eprfs-core` stays pure and small: no new dependencies, no `futures`; a pull cursor, not a `Stream`.
- Reuse `tree.rs` verification (`fetch_checked`, `decode_tree`, `valid_name`) by widening to `pub(crate)`;
  never re-implement a hash or a codec check.
- No change to the `EprfsStorage` trait in slice 1.
- No git semantics in eprfs-core.

## Gate and environment

Claim the cargo berth first: `berth claim cargo --class verify --ttl 3600` (release it after). Fast loop and
gate as in the drafted plan §5; echo `EXIT=$?` on its own line after every cargo run and never judge a run
from piped output. No staging, commits or pushes.

## The drafted plan (adopted, with the R6 change above)

### Slice 1 implementation plan: view-scoped walk and query in eprfs-core

Slice 1 adds `walk`, `read_path`, `list_dir` and `diff_trees` to eprfs-core with no new dependencies, and makes no change to the `EprfsStorage` trait. It runs locally only, against `DirectoryStorage` with `FetchPolicy::LocalOnly`. I made no edits and ran no cargo.

## 0. Where things go

| File | Change |
|---|---|
| `/projects/elohim/elohim/eprfs/eprfs-core/src/walk.rs` | **New.** `TreePath`, `View`, `Placement`, the walk cursor, read, list and diff. |
| `/projects/elohim/elohim/eprfs/eprfs-core/src/tree.rs` | Make `valid_name` and `fetch_checked` `pub(crate)`. Add the new `TreeError` variants. Move the per-node budget accounting into a `pub(crate) struct Budget`. |
| `/projects/elohim/elohim/eprfs/eprfs-core/src/test_support.rs` | **New**, `#[cfg(test)] pub(crate)`. A counting storage double and tree builders. |
| `/projects/elohim/elohim/eprfs/eprfs-core/src/lib.rs` | Add `pub mod walk;`, `#[cfg(test)] mod test_support;` and the re-exports in §1.7. |
| `/projects/elohim/elohim/eprfs/eprfs-storage/tests/walk_directory_storage.rs` | **New.** Integration test against the real `DirectoryStorage`. |
| `/projects/elohim/elohim/eprfs/eprfs-storage/examples/walk_view.rs` | **New.** The out-of-gate probe over a brit-sealed store. It is compiled by the gate's `clippy --all-targets` and `cargo test`, but run by hand. |
| `/projects/elohim/elohim/eprfs/.epr-meta/materialize-what-is-read.habit.md` | Add `checks:`, flip `unwired` to `red` with evidence, add a DELTA line, then run `.claude/scripts/habits-project.py`. |

`walk.rs` is a new module rather than more code in `tree.rs`, which is already 685 lines and holds the codec and the closure loader. The functions sit "beside `load_verified_tree`" at module level, as the spec asks.

There are no Cargo.toml changes. eprfs-core already has `async-trait`, `bytes` and the `tokio` dev-dependency. eprfs-storage already has `tempfile` and `tokio` (macros, rt-multi-thread, sync) for its tests and the example.

## 1. Types and signatures (`walk.rs`)

### 1.1 TreePath
Tree names are raw bytes and can be non-UTF-8 (see the test `raw_file_with_non_utf8_name_has_frozen_tag_42_tree_bytes_and_cid`). `ProjectionPath` is OS-encoded and cannot name the root. So matching runs on byte components.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct TreePath(Vec<Vec<u8>>);          // empty = the root
impl TreePath {
    pub fn root() -> Self;
    pub fn from_components(c: Vec<Vec<u8>>) -> TreeResult<Self>;   // each passes tree::valid_name, else TreeError::InvalidView
    pub fn from_slash_bytes(b: &[u8]) -> TreeResult<Self>;          // b"" -> root; split on b'/'
    pub fn components(&self) -> &[Vec<u8>];
    pub fn is_root(&self) -> bool;
    pub fn join(&self, name: &[u8]) -> Self;
    pub fn starts_with(&self, prefix: &TreePath) -> bool;           // component-wise, never byte-prefix
    pub fn byte_len(&self) -> usize;                                // sum of name lengths, the same measure as max_path_bytes today
}
impl TryFrom<&ProjectionPath> for TreePath { type Error = TreeError; }
impl fmt::Display for TreePath { /* lossy UTF-8 joined by '/', root shown as "<root>" */ }
```

`TryFrom<&ProjectionPath>`:
- On Unix it uses `std::os::unix::ffi::OsStrExt::as_bytes` per `Component::Normal`.
- On other platforms it uses `to_str()`, and returns `InvalidView` when that fails.
- `ProjectionPath` already rejects empty, absolute, `..` and leading `.` paths. `Path::components` normalizes `a/./b` and a trailing `/`.

### 1.2 View: the prefix semantics
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct View { paths: BTreeSet<TreePath> }   // normalized antichain: no member is a prefix of another
impl View {
    pub fn whole() -> Self;                                                     // {root}: covers everything
    pub fn new(paths: impl IntoIterator<Item = ProjectionPath>) -> TreeResult<Self>;   // the spec's form
    pub fn from_tree_paths(paths: impl IntoIterator<Item = TreePath>) -> Self;
    pub fn paths(&self) -> impl Iterator<Item = &TreePath>;
    pub fn is_empty(&self) -> bool;
    pub fn placement(&self, path: &TreePath) -> Placement;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement { InView, Spine, Outside }
```

Rules (put them in the doc comment):
- An entry at path Q is **InView** if some view path P is a prefix of Q or equal to it. A directory named in the view brings in its whole subtree; a file brings in itself.
- Q is **Spine** if Q is a strict prefix of some P. Ancestors are on the spine, and the root is on the spine of every non-empty view.
- Otherwise Q is **Outside**: never fetched and never descended. Its name is still visible, because the spine node holding it was fetched and verified whole. That is how depth 1 "shows it as absent".
- Matching is component-wise: `src` does not cover `src2`.
- Normalization removes duplicates, and a path covered by a shorter view path is dropped (`{a, a/b}` becomes `{a}`).
- An empty view fetches nothing and yields nothing. The root codec is still checked, with no fetch.

Implementation:
- A frontier item at a Spine node carries the view tails still under it: `Vec<&'a [Vec<u8>]>`, borrowed from the `View`. For each entry in the node, match tails whose first component equals `entry.name`. A tail of length 1 makes that child InView; longer tails keep it on the Spine.
- Tails whose first component matches no entry produce `Absent`.
- An InView node carries no tails; everything below it is InView.
- Both entries and tails are byte-sorted, so a merge pass is O(entries + tails).

### 1.3 The walk: a cursor rather than `futures::Stream`
eprfs-core has no `futures` dependency, and its CLAUDE.md says the core stays small. The cursor is a pull-based async stream with memory bounded by the frontier. A real `Stream` adaptor (`futures_util::stream::unfold`) can wait for slice 2, where a remote tier needs it.

```rust
pub fn walk<'a, S: EprfsStorage>(
    storage: &'a S, root: &BlobCid, view: &'a View, policy: FetchPolicy, limits: &'a TreeLimits,
) -> TreeResult<TreeWalk<'a, S>>;          // root codec != 0x71 -> TreeError::Invalid. Every view path's
                                           // byte_len > max_path_bytes -> Budget("path bytes") BEFORE any fetch

#[must_use]
pub struct TreeWalk<'a, S> { /* stack frontier, out queue VecDeque<WalkItem>, Budget, node cache, stats */ }
impl<'a, S: EprfsStorage> TreeWalk<'a, S> {
    pub async fn next(&mut self) -> Option<TreeResult<WalkItem>>;   // after an Err, returns None
    pub fn stats(&self) -> &WalkStats;
}
pub async fn walk_collect<S: EprfsStorage>(
    storage: &S, root: &BlobCid, view: &View, policy: FetchPolicy, limits: &TreeLimits,
) -> TreeResult<WalkedView>;

pub enum WalkItem {
    Node(WalkedNode),
    Boundary { path: TreePath, kind: String, reference: Vec<u8>, placement: Placement }, // Spine = a view path went through it
    Absent { view_path: TreePath, reason: AbsentReason },
}
pub enum AbsentReason { NotFound { missing_at: TreePath }, ThroughLeaf { leaf: TreePath } }
pub struct WalkedNode {
    pub path: TreePath, pub cid: BlobCid, pub node: TreeNode,
    pub placement: Placement,              // Spine or InView (never Outside)
    pub entry_placements: Vec<Placement>,  // parallel to node.entries
}
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct WalkStats { pub nodes_fetched: usize, pub nodes_reused: usize, pub node_bytes: usize, pub entries_visited: usize }
pub struct WalkedView { pub root: BlobCid, pub nodes: Vec<WalkedNode>, pub boundaries: Vec<WalkItem>, pub absent: Vec<WalkItem>, pub stats: WalkStats }
impl WalkedView {
    pub fn leaves_in_view(&self) -> impl Iterator<Item = (TreePath, &TreeEntryKind)>;  // what a sparse materializer or slice 2's fetch_blobs would fetch
}
```

Behaviour:
- **Tree nodes only.** The walk never fetches a leaf blob, unlike `load_verified_tree`. This matches the spec's "a stream of verified tree nodes".
- **Order.** Depth-first pre-order in byte-sorted name order. Push children onto the stack in reverse, as the existing loader does. A node's Boundary and Absent items are queued right after its Node item.
- **Verification.** Each node goes through `fetch_checked(storage, &link_cid, policy)`, with the CID taken from the parent's link, then `decode_tree`. That gives "verified against the link in its parent" and the canonical bytes check.
- **Incomplete nodes.** When a node is missing, the walk wraps `TreeError::Incomplete` with the path appended to `reason` (`"… at a1/b2"`). That meets the "fails promptly, naming the path" rule in §12.4 without changing the variant's shape.
- **External entries** are emitted as `Boundary` and never followed. This includes a view path that goes through one (`vendor/x` where `vendor` is External).
- **Symlinks** are leaves and never resolved. A view path through a file or symlink gives `Absent{ThroughLeaf}`.
- **Cycle check.** Keep the lineage/ancestors check from the loader.

### 1.4 How TreeLimits applies to the spine
Move the loader's accounting into `pub(crate) struct Budget { total_entries, total_bytes, pending_path_bytes, … }` and use it from the walk, read, list and diff. Keep the same `&'static str` labels so tests can match on them. Optionally, refactor `load_verified_tree` onto `Budget` as well; it must keep identical behaviour, and the existing tests must still pass.

| Field | Applied to |
|---|---|
| `max_node_bytes`, `max_entries_per_node` | Each fetched node, through `decode_tree`, unchanged. |
| `max_depth` | The depth of visited nodes only (`"tree depth"`). |
| `max_total_entries` | The sum of `entries.len()` over visited nodes only (`"total entries"`). A spine node with 100k siblings still counts all 100k, because its bytes were fetched and decoded. |
| `max_total_bytes` | Fetched node bytes (`"total bytes"`). A node reached again by another path is charged again (`"expanded bytes"`), as today. `read_path` also charges its one leaf. |
| `max_path_bytes` | View paths at walk start, and the child paths of InView or Spine entries only. Outside siblings are never checked. |
| `max_pending_entries` | Frontier length (`"pending entries"`). |

### 1.5 Read, list and diff
```rust
pub async fn read_path<S: EprfsStorage>(storage: &S, root: &BlobCid, path: &TreePath, policy: FetchPolicy, limits: &TreeLimits) -> TreeResult<ReadLeaf>;
pub struct ReadLeaf { pub path: TreePath, pub cid: BlobCid, pub kind: LeafKind, pub bytes: Bytes }
pub enum LeafKind { File { executable: bool }, Symlink }

pub async fn list_dir<S: EprfsStorage>(storage: &S, root: &BlobCid, path: &TreePath, policy: FetchPolicy, limits: &TreeLimits) -> TreeResult<DirListing>;
pub struct DirListing { pub path: TreePath, pub cid: BlobCid, pub entries: Vec<TreeEntry> }   // children are never fetched

pub async fn diff_trees<S: EprfsStorage>(storage: &S, old: &BlobCid, new: &BlobCid, view: &View, policy: FetchPolicy, limits: &TreeLimits) -> TreeResult<Vec<TreeChange>>;
pub struct TreeChange { pub path: TreePath, pub old: Option<TreeEntryKind>, pub new: Option<TreeEntryKind> } // None/Some = added; Some/None = removed
```

**Read and list.** Both use a shared `pub(crate) async fn resolve_entry(...)`, a single-path spine walk that does not descend into the target.
- A missing name gives `PathNotFound`.
- A file or symlink in the middle of the path gives `NotADirectory`.
- An External entry on the path gives `ExternalBoundary`.
- Read on a directory gives `NotALeaf`. List on a file gives `NotADirectory`. List on the root fetches only the root node.

**Diff** is a co-walk, not two full walks.
- When `old == new`, return with zero fetches.
- At each pair of nodes, merge entries by name, considering only entries that are InView or Spine.
- Two directories with equal CIDs are skipped without a fetch, which is what keeps the cost to the changed paths (§12.1 inversion 5). With different CIDs, the diff descends. A directory-to-directory change is never emitted as a change itself.
- A directory added or removed is emitted once, at its path, and not expanded.
- A kind change, an executable-bit change or a changed External reference shows up as Some/Some with differing kinds.
- One `Budget` covers both sides. Output is pre-order, byte-sorted.

### 1.6 Error variants to add to `TreeError` in `tree.rs`
```rust
#[error("path not found under root: {path}")]               PathNotFound { path: TreePath },
#[error("path is not a directory: {path}")]                  NotADirectory { path: TreePath },
#[error("path is not a file or symlink: {path}")]            NotALeaf { path: TreePath },
#[error("path crosses an external boundary at {path} ({kind})")] ExternalBoundary { path: TreePath, kind: String },
#[error("invalid view: {0}")]                                InvalidView(String),
```
Adding variants is safe. Outside tree.rs, nothing in the eprfs workspace, brit-bridge, brit-cli or elohim-storage matches on a `TreeError::` variant; brit uses `?` into anyhow, and eprfs-local uses `Box<TreeError>`. `TreePath` is about 24 bytes, so `clippy::result_large_err` is not affected. If `clippy::large_enum_variant` fires on `WalkItem`, box the `WalkedNode`.

### 1.7 lib.rs exports
```rust
pub mod walk;
#[cfg(test)] mod test_support;
pub use walk::{walk, walk_collect, read_path, list_dir, diff_trees, AbsentReason, DirListing, LeafKind,
    Placement, ReadLeaf, TreeChange, TreePath, TreeWalk, View, WalkItem, WalkStats, WalkedNode, WalkedView};
```

## 2. The test double: `test_support.rs`
Model it on `TestStorage` in `tree.rs` (lines 525–554). That type is copied inline three times, at lines 525, 588 and 639. Reuse the `file()` helper from `tree.rs` (line 376) by moving it here.

```rust
pub(crate) struct CountingStorage { pub blobs: HashMap<BlobCid, Bytes>, fetched: std::sync::Mutex<Vec<BlobCid>> }
impl CountingStorage {
    pub fn new() -> Self;
    pub fn put_leaf(&mut self, bytes: &[u8]) -> BlobCid;                 // compute_raw
    pub fn put_tree(&mut self, entries: Vec<TreeEntry>) -> BlobCid;      // TreeNode::new + encode_tree
    pub fn remove(&mut self, cid: &BlobCid);
    pub fn tamper(&mut self, cid: &BlobCid, bytes: Bytes);
    pub fn fetched(&self) -> Vec<BlobCid>;
    pub fn reset(&self);
}
// EprfsStorage impl: fetch_blob logs the cid, then returns BlobNotFound when missing; resolve, put and publish are unreachable!()
pub(crate) fn file(name: &[u8], content: &[u8]) -> TreeEntry;
pub(crate) fn dir(name: &[u8], tree: BlobCid) -> TreeEntry;
pub(crate) fn external(name: &[u8]) -> TreeEntry;
/// root -> a0..a9 -> b0..b9 -> f000..f099: 10,000 file entries (10,110 entries in all), 111 tree nodes.
pub(crate) fn ten_thousand(store: &mut CountingStorage, with_leaves: bool) -> (BlobCid, HashMap<TreePath, BlobCid>);
```
Moving the three inline doubles onto `CountingStorage` is optional and outside the slice; do it only if it stays mechanical.

## 3. Tests

**`eprfs-core/src/walk.rs`, `mod tests`**
1. `walk_ten_thousand_entries_three_path_view_fetches_only_spine` — **the headline test.**
   - View: `{a1/b2/f003, a4/b4/f099, a9/b0}`.
   - Fetched set equals exactly {root, a1, a1/b2, a4, a4/b4, a9, a9/b0}: 7 of 111 nodes, each once, and no leaf CID.
   - Node items come in pre-order, with placements Spine×6 and InView for a9/b0.
   - `stats.entries_visited == 340`.
   - In the same test, `load_verified_tree` over the same store fetches 10,111 objects. This is the contrast.
2. `walk_over_store_holding_only_the_spine_succeeds_local_only` — only the 7 nodes are inserted, and the walk passes under `LocalOnly`.
3. `view_prefix_is_component_wise` — `src` covers `src` and `src/x`, but not `src2`.
4. `view_normalizes_duplicates_and_covered_paths`
5. `view_from_projection_paths_and_whole_view`
6. `empty_view_fetches_nothing`
7. `walk_refuses_oversized_view_path_before_any_fetch` — `Budget("path bytes")` with `fetched()` empty.
8. `walk_depth_budget_applies_to_spine` — `max_depth: 1` against a depth-3 view path gives `Budget("tree depth")`.
9. `walk_entry_budget_counts_only_visited_nodes` — 340 passes and 339 refuses, while `load_verified_tree` refuses at 340.
10. `walk_verifies_each_node_against_parent_link` — a tampered spine node gives `Integrity(cid)`.
11. `walk_emits_external_boundary_and_does_not_follow` — once with InView placement, once through the Spine.
12. `walk_reports_absent_view_path_by_name` — `NotFound`, with `missing_at`.
13. `walk_view_through_a_file_is_absent_through_leaf`
14. `walk_shared_subtree_fetched_once_charged_per_path` — `"expanded bytes"`.
15. `walk_whole_view_visits_same_node_set_as_load_verified_tree`
16. `walk_rejects_non_dag_cbor_root`
17. `walk_missing_spine_node_is_incomplete_naming_path`
18. `read_path_fetches_spine_plus_one_leaf_and_verifies` — 3 nodes and 1 leaf.
19. `read_path_missing_is_path_not_found_named` — `Display` contains `a1/b2/nope`.
20. `read_path_on_directory_is_not_a_leaf_and_through_external_refuses`
21. `list_dir_root_and_nested_fetch_no_children`
22. `list_dir_on_file_is_not_a_directory`
23. `diff_identical_roots_fetch_nothing`
24. `diff_reports_added_removed_modified_under_view`
25. `diff_skips_shared_subtrees_by_cid` — change one file in a3/b7 of the 10k tree, use `View::whole()`, and the fetches are exactly root, a3 and a3/b7 on each side (6).
26. `diff_ignores_changes_outside_view`
27. `tree_path_display_is_lossy_and_names_root`

**`eprfs-storage/tests/walk_directory_storage.rs`** (real adapter, `LocalOnly`, tempdir via `DirectoryStorage::create`)
28. `directory_storage_walk_three_path_view_over_sparse_store` — write the tree with `put_blob_verified`, then `std::fs::remove_file` every object outside the view. The walk, `read_path` and `list_dir` still pass, and reading a removed path fails promptly with `Incomplete`, naming the path.
29. `directory_storage_corrupt_spine_object_refuses` — expects `TreeError::Storage`, from `read_object`'s corruption check.

## 4. The brit-sealed tree proof
It is **not feasible inside the eprfs gate.** There are three reasons:
- brit consumes eprfs-core 0.1.2 from the Nexus `elohim` registry, with no path or `[patch]`. It cannot see the new API until eprfs publishes a bump and brit's lock moves, and per memory an attested pin move rolls the fleet.
- eprfs cannot depend on brit.
- An env-gated or `#[ignore]` test passes trivially in the gate, which is the hollow check CLAUDE.md warns about.

Format compatibility is already guaranteed by construction, because brit-bridge seals through eprfs-core's `encode_tree` and `DirectoryStorage`. What the monorepo run adds is scale.

**Out-of-gate proof:**
1. Build brit with `RUSTUP_TOOLCHAIN=1.96.1 RUSTFLAGS= CARGO_TARGET_DIR=<pool slot> cargo build --manifest-path /projects/elohim/elohim/brit/Cargo.toml -p brit-cli --bin brit`.
   - Never cd into the submodule.
   - This needs the Nexus registry, or a warm cache.
   - The submodule checkout (7310df6e3) is an **ancestor of**, i.e. behind, the committed pin 76ba95d8a. The pin adds `MutationGuards` to `snapshot seal`, which takes a writer guard on the repo that contains `--store`. Build at the pin, from a worktree under `/projects/elohim/.worktrees/`, referenced by `--manifest-path`.
2. Run `brit snapshot seal --repo /projects/elohim --revision HEAD --store /projects/elohim/genesis/local-dev/eprfs-slice1/store`. That directory is gitignored and survives restarts. The output JSON's `root` is the CID.
   - Expected size, measured with `git ls-tree` on HEAD: 20,117 blobs (~239 MB), 2,129 subdirectories plus the root, 5 gitlinks that become External.
   - That fits the default `TreeLimits` and is under `DirectoryStorage`'s 64 MiB per-blob cap. Watch the PVC watermark.
3. Run `cargo run -p eprfs-storage --example walk_view -- --store … --root <cid> --view elohim/eprfs/eprfs-core/src/tree.rs --view genesis/docs/superpowers/specs --view elohim/brit/brit-bridge/src/lib.rs [--baseline]`.
   - The third path crosses a gitlink, so it exercises Boundary through the Spine.
   - The example wraps `DirectoryStorage` in a private counting wrapper, as an independent witness alongside `WalkStats`. It prints JSON: `nodesFetched`, `treeNodesInStore`, `entriesVisited`, `boundaries`, `absent`, and with `--baseline` the `load_verified_tree` fetch count.
4. Record the numbers as MEASURED evidence in the habit's `checks:` entry and its DELTA, as other habits do.

## 5. Gate and environment
- **Gate:** `just gate eprfs`. gate-runner reads `/projects/elohim/elohim/eprfs/build-manifest.json`, `gate.projects.eprfs`, of kind root-just. The recipe is `_gate-eprfs` in `/projects/elohim/justfile:646`: `cargo fmt --check`, then `cargo clippy --workspace --all-targets -- -D warnings`, then `cargo test --workspace`, all in `elohim/eprfs`.
- **Environment the gate injects:**
  - `CARGO_TARGET_DIR=/tmp/eprfs-gate-target`, `RUSTFLAGS=""`, dev profile (from the manifest).
  - `RUSTUP_TOOLCHAIN=1.96.1` (pool-policy `"*"`; installed at `/opt/rust/rustup/toolchains/1.96.1-x86_64-unknown-linux-gnu`).
  - `CARGO_BUILD_JOBS=1`, `RUST_TEST_THREADS=1` (pool-policy `cargo_env_overrides.eprfs`).
- **Fast loop:** `cd /projects/elohim/elohim/eprfs && RUSTUP_TOOLCHAIN=1.96.1 RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-gate-target CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 cargo test -p eprfs-core -p eprfs-storage; echo EXIT=$?`
  - The workspace gate also builds epr-cli, which is heavy.
  - nextest is not installed.
  - The PreToolUse hook denies native cargo without `CARGO_TARGET_DIR`.
  - rust-analyzer may hold the build slot after `.rs` edits.
- **Habit check line:** `cargo test -p eprfs-core --lib walk::tests::walk_ten_thousand_entries_three_path_view_fetches_only_spine` (elohim/eprfs), plus the example run above. The status stays `red` after slice 1, because tiers and reach are not met.

## 6. Where the spec's slice-1 text and the code disagree
1. **Trait method or free function.** §12.3 says "every tier implements `EprfsStorage` … the design adds two calls", which reads as trait methods. Slice 1 says "beside `load_verified_tree`". The plan uses free functions generic over `S: EprfsStorage`, so no adapter or registry consumer breaks. Whether a remote tier answers `walk` server-side, as a trait method with a default implementation, is slice 2's call.
2. **Stream.** §12.3 says "a stream", but eprfs-core has no futures dependency. The plan uses an async pull cursor.
3. **ProjectionPath cannot name the root, and is OS-encoded.** Tree names are raw bytes, including non-UTF-8. The plan keeps `View::new(ProjectionPath…)` as the spec's form and matches on a byte `TreePath` internally, with `View::whole()` for the root.
4. **TreeLimits fields.** The spec lists "depth, entries, bytes, pending". `TreeLimits` also has `max_node_bytes`, `max_entries_per_node` and `max_path_bytes`; all of them apply (§1.4).
5. **Leaves.** `load_verified_tree` fetches every leaf. The walk fetches none; `read_path` fetches one. `fetch_blobs(cids, policy)` from §12.3 is not in slice 1's list, and the plan leaves it to slice 2. `leaves_in_view()` exposes the set it would fetch.
6. **Diff cost.** "Diff two roots under a view … over a verified walk" would be quadratic in waste if done as two walks. A co-walk with CID short-circuit is what §12.1 inversion 5 ("cost follows the change") requires.
7. **The brit-sealed proof cannot run in the slice-1 gate.** brit pins eprfs-core 0.1.2 from the registry (§4), so slice 1's proof has a gate half and an out-of-gate half. Native brit consumption is slice 7.
8. **The habit has no `checks:` field yet.** The flip to red needs one added, with evidence, before re-projection. `genesis/manifests/habits.yaml` is already modified in the working tree.
9. **Small drift.** The eprfs build-manifest step description says "nextest", but the recipe runs `cargo test`.
10. **"Agent's read set discovered as it reads".** §12.2 implies a view that grows during a walk. Slice 1's view is static; a `TreeWalk::extend_view` hook belongs to slice 3, the on-read depth.

No rule floor seemed to be needed for this read-only work.
