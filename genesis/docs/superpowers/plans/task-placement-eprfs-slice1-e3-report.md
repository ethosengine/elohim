---
id: placement-eprfs-slice1-e3-report
title: eprfs slice 1, E3 — the spine proof and its refusals, just gate eprfs green
status: DONE_WITH_CONCERNS
class: architecture
gap: plans__2026-10-20-placement-contract-architecture-plan#7
actor: agent:implementer@claude-opus-5-5
habits: [materialize-what-is-read]
commits: []
cites:
  - "placement-eprfs-slice1-brief | eprfs slice 1 | sha256:46be8b35cc71693b | path: genesis/docs/superpowers/plans/task-placement-eprfs-slice1-brief.md"
  - "placement-contract-architecture-plan | The placement contract's architecture | sha256:2148e5a4de372761 | path: genesis/docs/superpowers/plans/2026-10-20-placement-contract-architecture-plan.md"
---

# E3 — tests, and `just gate eprfs` green

Item E3, gap `#7`. Nothing is staged, committed or pushed.

**Gate evidence.** `just gate eprfs`, run from `/projects/elohim` with the cargo berth claimed:

- fmt `--check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed 1,234 tests with 0 failed, across 76 suites.
- The run printed `EXIT=0`.

The first gate run printed `EXIT=1` because of three `clippy::explicit_auto_deref` errors in `walk.rs`.
Those were fixed and the gate was run again. In the fast loop,
`cargo test -p eprfs-core -p eprfs-storage` printed `EXIT=0`: eprfs-core has 67 lib tests (38 before
this work, 29 new in `walk::tests`), eprfs-storage has 3 lib tests, and `walk_directory_storage` has 2.
The cargo berth was released after the gate.

## The headline: the spine proof

`walk::tests::walk_ten_thousand_entries_three_path_view_fetches_only_spine` builds a tree of 111 nodes
holding 10,000 files, 10,110 entries in all. Under the view `{a1/b2/f003, a4/b4/f099, a9/b0}`:

- The counting storage records exactly 7 fetches, each once, in pre-order: the root, `a1`, `a1/b2`, `a4`,
  `a4/b4`, `a9` and `a9/b0`. No leaf is fetched.
- Placements are `Spine` six times, then `InView` for `a9/b0`.
- `entries_visited == 340`.
- `leaves_in_view()` gives 102.
- Over the same store, `load_verified_tree` fetches 10,111 objects.

This test is the check line for the `materialize-what-is-read` habit. The E4 seat records it.

## Tests added

### `eprfs-core/src/walk.rs` (29)

**Coverage and budget**

- The headline test, above.
- A store that holds only the 7 spine nodes passes under `LocalOnly`.
- View prefixes match component by component: `src` does not cover `src2`.
- The view is normalized. It can be built from a `ProjectionPath`, including non-UTF-8 bytes on Unix.
- An empty view fetches nothing.
- An oversized view path is refused with `Budget("path bytes")` before any fetch.
- The depth budget applies to the spine.
- **The entry budget counts only visited nodes.** A limit of 340 passes and 339 refuses, while
  `load_verified_tree` refuses at 340.

**Integrity and absence**

- A spine node that is tampered with gives `Integrity(cid)`.
- **A decode failure is an error, not an absence.** This test is new beyond the drafted plan, for R6.
- External boundaries are never followed. One is reported as `InView` and one through the spine.
- A missing name is reported as an absence `NotFound`, with `missing_at`.
- A view path that goes through a file is reported as `ThroughLeaf`.
- A shared subtree is fetched once and charged once for each path that reaches it (`"expanded bytes"`).
- The whole view visits the same set of nodes as `load_verified_tree`, with 111 fetches and none of them
  a leaf.
- A root that is not DAG-CBOR is refused.
- **R6 replacement for drafted test 17:**
  `walk_missing_subtree_is_named_not_held_and_walk_continues`.
  - With `a4/b4` removed, the walk returns `Ok` with the 6 nodes that are held, plus one named
    `NotHeld { missing_at: a4/b4, cid }` for `a4/b4/f099`.
  - Under the whole view, with `a3` removed as well, 99 nodes still come through, along with two
    absences in pre-order.
- `read_path_over_a_spine_not_held_is_incomplete_naming_path`: the `Incomplete` expectation is kept for
  `read_path` and `list_dir`.

**Read, list and diff**

- `read_path` fetches the spine plus one leaf, and a tampered leaf gives `Integrity`.
- A missing path gives `PathNotFound`, and the error's `Display` names `a1/b2/nope`.
- The refusals `NotALeaf`, `ExternalBoundary` and `NotADirectory` are each tested.
- `list_dir` on the root fetches only the root. On `a4/b4` it fetches 3 nodes and none of the children.
- `list_dir` on a file gives `NotADirectory`.
- A diff of two identical roots fetches nothing.
- A diff reports what was added, removed and modified, including an executable-bit flip, both under the
  whole view and under a view of `e` only.
- **A diff skips shared subtrees by CID.** With one file changed in the 10,000-entry tree, the diff
  fetches exactly 6 nodes: the root, `a3` and `a3/b7` on each side.
- A diff ignores changes outside its view.
- `TreePath`'s `Display` is lossy and names the root.

### `eprfs-storage/tests/walk_directory_storage.rs` (2, against the real `DirectoryStorage`, `LocalOnly`)

**`directory_storage_walk_three_path_view_over_sparse_store`**

- All 111 tree nodes are written with `put_blob_verified`, and the 104 nodes outside the view are removed
  with `std::fs::remove_file`.
- Only the 102 leaves in view are written. That leaves the store in the same state as writing every leaf
  and removing the rest, and it avoids 10,000 fsyncs.
- The walk, `read_path` and `list_dir` pass, and the walk visits 340 entries.
- `read_path` of `a2/b0/f000` gives `Incomplete` naming `at a2`.
- A walk with the view `{a1/b2/f003, a2/b0}` returns `Ok` with one named `NotHeld` absence.

**`directory_storage_corrupt_spine_object_refuses`**

`a1` is overwritten with garbage and keeps its 0600 mode. The walk gives `TreeError::Storage` naming
`a1`'s CID, from `read_object`'s corruption check. Corruption is never reported as an absence.

## Also changed

- **Test support.** `eprfs-core/src/test_support.rs` is new and test-only. It holds `CountingStorage`,
  the builders `file`, `dir`, `external`, `tp`, `ten_thousand` and `ten_thousand_content`.
- **Existing loader tests.** The three `TestStorage`/`RootOnly` doubles inline in `tree.rs` were moved
  onto `CountingStorage` mechanically. The `long_directory_frontier` test now also asserts that only the
  root was fetched.

## Fix round 1

Two review findings were checked against the code. Both held. One part of the second was not done, and the
reason is given below.

**Gate evidence (fix round 1).** `just gate eprfs`, run from `/projects/elohim` with the cargo berth claimed
and released afterwards, printed `EXIT=0`: fmt `--check` and `clippy --workspace --all-targets -D warnings`
passed, and `cargo test --workspace` passed 1,236 tests with 0 failed, across 76 suites. The fast loop,
`cargo test -p eprfs-core -p eprfs-storage`, printed `EXIT=0`: 69 eprfs-core lib tests, 3 eprfs-storage lib
tests and 2 `walk_directory_storage` tests.

### Finding 1 (major, correctness): a Spine ancestor was reported as a change. Fixed.

The finding held. With the view `{a/b}`, an old root with no `a` and a new root where `a` is `dir{c}`
produced `TreeChange { path: a }`. The `_` arm of `diff_trees` emitted every change that was not
Dir to Dir, whatever its scope was.

The fix is in `walk.rs` `diff_trees`:

- Each side of `DiffFrame` is now an `Option<BlobCid>`. `None` stands for an empty directory, which is
  never fetched (`diff_side`).
- When an entry's scope is `Spine`, a side that is not a directory counts as empty. The diff descends
  when either side is a directory, and the ancestor itself is never reported.
- When the scope is `InView`, the diff behaves as before: it descends only when both sides are
  directories, and otherwise reports the change once at its path.
- The doc comment states the Spine rule.

Two tests were added:

- `diff_reports_only_view_tails_below_a_spine_ancestor`, with the view `{a/b}`. It covers:
  - an ancestor added or removed with nothing at `a/b`, which gives no change;
  - an ancestor added or removed that holds `a/b`, which reports only `a/b`;
  - an in-view directory `a/b` added under a new ancestor, which is reported once;
  - the ancestor changing between file and directory in both directions, with and without `b`;
  - a file changed at the ancestor, which gives no change;
  - directories on both sides, which still descend to the tail.
- `diff_in_view_kind_change_is_reported_at_its_path`. Under the whole view, the same add, remove and
  file-to-directory change are still reported once at `a`.

### Finding 2 (major, design): the new public types were exhaustive. Fixed in the code; the version is left to the release.

The finding held, and `#[non_exhaustive]` was added to these types:

- `TreeError`.
- `AbsentReason` and each of its variants, so a `tiers` field can be added to `NotHeld`.
- `WalkItem` and its `Boundary` and `Absent` variants.
- The structs `WalkStats`, `WalkedNode`, `WalkedView`, `ReadLeaf`, `DirListing` and `TreeChange`.

Code inside the crate is unaffected. The `eprfs-storage` integration test now matches the absence with
`..` instead of building it, which is the pattern a downstream caller has to use. A search of the
workspace and `bridges/jenkins` found no other code outside the crate that builds these types or matches
them exhaustively.

`Placement` (`InView`, `Spine`, `Outside`) and `LeafKind` (`File`, `Symlink`) are left exhaustive. Each is
a closed partition by definition, and callers need to match them completely.

**Not done: the version bump.** The workspace version is still `0.1.2`. The reason:

- `[workspace.package] version` is shared by every crate in the eprfs workspace.
- `eprfs-local`, `eprfs-meta` and `eprfs-storage` pin `eprfs-core` at `version = "0.1.2"` on the `elohim`
  registry.
- Publication is outside this slice, and the session covenant is "no landing".

Instead, the requirement is recorded twice: in the `TreeError` doc comment, and as a concern below.

## Fix round 2

Three review findings were checked against the code. All three held, and all three were fixed. None was
refused.

**Gate evidence (fix round 2).** `just gate eprfs`, run from `/projects/elohim` with the cargo berth claimed
and released afterwards, printed `EXIT=0`: fmt `--check` and `clippy --workspace --all-targets -D warnings`
passed, and `cargo test --workspace` passed 1,239 tests with 0 failed, across 76 suites. The fast loop,
`cargo test -p eprfs-core -p eprfs-storage`, printed `EXIT=0`: 72 eprfs-core lib tests, 3 eprfs-storage lib
tests and 2 `walk_directory_storage` tests. The first fast-loop run printed `EXIT=101`: a new test moved a
`BlobCid` and then used it again, which failed to compile. That was fixed before the runs above.

### Finding 1 (major, correctness): an External on the spine was read as an empty directory. Fixed.

The finding held. Under the view `{vendor/x}`, round 1's Spine rule counted any side that was not a
directory as empty. So a changed External reference at `vendor` gave no change, and External to Directory
was reported as `vendor/x` being added from nothing. The walk reports the same path as a `Boundary` with
`Spine` placement, so the diff and the walk disagreed about the same view.

The fix is in `diff_trees`. In the Spine arm, when either side is External, the change is reported once at
the boundary path and not descended. A File or Symlink side still counts as empty. The doc comment states
the rule.

The test `diff_external_on_the_spine_is_reported_at_the_boundary` runs under both `{vendor/x}` and
`{vendor}`. It covers four cases, each reported at `vendor` with the exact old and new kinds:

- a changed reference;
- External to Directory;
- Directory to External;
- an External added.

In each case the directory below the boundary is never fetched. The test also checks that the walk over
the new root reports `Boundary { vendor, placement: Spine }`.

### Finding 2 (major, correctness): the paths kept in a gathered answer were not metered. Fixed.

The finding held. A shared subtree reached under many long paths costs its node bytes once for each path.
But `walk_collect` keeps a `WalkedNode.path` for every node, and `diff_trees` keeps a path for every
`TreeChange`, and neither path was charged. With the default limits, that allows about 4 GB of kept paths
against a budget of 1 GiB.

The fix:

- `Budget::charge_result` is new in `tree.rs`. It is a separate counter, bounded by `max_total_bytes`, with
  the label `"result path bytes"`.
- `walk_collect` charges the path of each node it keeps. It uses its own `Budget`, because the cursor keeps
  nothing.
- `diff_trees` charges each change it keeps: the path, plus the kind and reference bytes of any External it
  clones.
- The `TreeWalk` doc says that items a caller keeps are the caller's to bound. The `walk_collect` doc points
  a caller with a large tree to the cursor.

Two tests were added. Each builds a chain 10 levels deep with names of 100 bytes, so the paths kept grow
faster than the node bytes fetched:

- `walk_collect_meters_the_paths_it_keeps`. The paths kept come to 5,500 bytes. With `max_total_bytes` set
  to the node bytes fetched, the cursor still yields all 11 items. `walk_collect` is refused with
  `Budget("result path bytes")`.
- `diff_meters_the_paths_it_keeps`. Twenty changed files sit at the bottom of the chain. With
  `max_total_bytes` set to the bytes fetched, the diff is refused with `Budget("result path bytes")`.

### Finding 3 (major, correctness): the `Incomplete` path assertions could not tell the spine node from the requested leaf. Fixed.

The finding held. The text is `"… {reason} at {path}"`. The assertion `contains("at a1/b2")` would also pass
for `"at a1/b2/f003"`, the requested path.

Both assertions now use `ends_with`:

- `walk.rs`: `ends_with(" at a1/b2")`.
- `walk_directory_storage.rs`: `ends_with(" at a2")`.

`TreeError::Incomplete`'s `Display` ends with the reason, so `ends_with` names the missing node and nothing
longer. A structured `path` field was not added. The assertion now tells the two apart, and a new field
would widen the error's public shape beyond what slice 1 needs.

## Concerns

- **The next publication of eprfs-core must be 0.2.0, not 0.1.3.** Two changes break compatibility with
  the published 0.1.2:
  - `TreeError` gained five variants and is now `#[non_exhaustive]`.
  - Every new walk type is `#[non_exhaustive]`.

  Whoever publishes moves the workspace version and the three registry pins together. brit consumes 0.1.2
  from the registry and is unaffected until it chooses 0.2.0, which is slice 7.
- The brit-sealed acceptance probe (§4 of the brief) cannot run inside this gate. It belongs to the E5
  seat.
