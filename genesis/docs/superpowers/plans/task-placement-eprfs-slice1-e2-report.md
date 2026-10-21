---
id: placement-eprfs-slice1-e2-report
title: eprfs slice 1, E2 — read one path, list one directory, diff two roots under a view
status: DONE
class: architecture
gap: plans__2026-10-20-placement-contract-architecture-plan#6
actor: agent:implementer@claude-opus-5-5
habits: [materialize-what-is-read]
commits: []
cites:
  - "placement-eprfs-slice1-brief | eprfs slice 1 | sha256:46be8b35cc71693b | path: genesis/docs/superpowers/plans/task-placement-eprfs-slice1-brief.md"
  - "placement-contract-architecture-plan | The placement contract's architecture | sha256:2148e5a4de372761 | path: genesis/docs/superpowers/plans/2026-10-20-placement-contract-architecture-plan.md"
---

# E2 — the three queries

Item E2, gap `#6`. Built in `elohim/eprfs/eprfs-core/src/walk.rs` beside the walk from E1, as free
functions generic over `S: EprfsStorage`. Nothing is staged, committed or pushed.

**Gate evidence.** `just gate eprfs` ran fmt `--check`, clippy `-D warnings` and `cargo test --workspace`.
1,234 tests passed and 0 failed, and the run printed `EXIT=0`. The cargo berth was claimed before the run
and released after it.

## What was built

All three queries check the root codec before fetching anything.

### `read_path`

- Returns a `ReadLeaf { path, cid, kind: LeafKind, bytes }`.
- It goes through `resolve_entry`, which walks the single spine to the target without descending into the
  target. Each spine node is verified with `fetch_node`, the same check the walk uses.
- The one leaf is then fetched with `fetch_checked` and charged to the same `Budget`.
- A path longer than `max_path_bytes` is refused before any fetch.

### `list_dir`

- Returns a `DirListing { path, cid, entries }`.
- It fetches the spine plus the directory node itself, and never a child. Listing the root fetches only
  the root node.

### `diff_trees`

- Returns `Vec<TreeChange { path, old, new }>`.
- It walks both trees together: one pass, one `Budget` for both sides.
- **It returns early, with no fetch, in two cases:** when the two roots are equal, and when the view is
  empty.
- **Equal directories cost nothing.** A directory entry that is equal by CID on both sides is skipped
  without a fetch, so the cost follows the change.
- **A directory that differs on both sides** is descended, and is never reported itself.
- **Everything else that differs** is reported once at its path, without being expanded:
  - a directory added or removed;
  - a change of kind;
  - a flipped executable bit;
  - a changed leaf or External reference.
- Only entries in view or on the spine are considered.
- The output is sorted by component-wise `TreePath`, which is pre-order in byte-sorted name order.

### Refusals

| Case | Error |
|---|---|
| A missing name | `PathNotFound`, naming the full requested path |
| A file or symlink in the middle of the path | `NotADirectory`, at that prefix |
| An External entry on the path, or as the target | `ExternalBoundary`, at the boundary |
| `read_path` on a directory or on the root | `NotALeaf` |
| `list_dir` on a file | `NotADirectory` |

### R6: a single answer cannot be partial

A node or leaf the store does not hold is `TreeError::Incomplete`. Its reason gains `" at <path>"`, so the
error names where the store fell short, and the variant keeps its shape. This holds for `read_path`,
`list_dir` and `diff_trees` alike. A tampered leaf is `Integrity`.

## Concerns

None for this item. The reviewer seat may want to look at two choices:

- **Spine entries in the diff.** An entry on the spine that changed in some way other than
  directory-to-directory, such as a directory replaced by a file under a view path, is reported at the
  spine entry's path. It is not dropped, and the view path is not expanded.
- **Matching names to view paths.** In the diff this is a linear filter over the view paths for each name
  (`scope_for`). The walk uses a merge pass. Both share `group_scope`, and view paths are few.
