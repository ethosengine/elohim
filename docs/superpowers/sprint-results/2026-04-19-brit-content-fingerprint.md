# Sprint Result: Brit Content Fingerprint

**Date:** 2026-04-19 (sprint close)
**Plan:** `docs/superpowers/plans/2026-04-19-brit-content-fingerprint.md`
**Branches:** `feat/brit-content-fingerprint` (brit submodule), `feat/build-plan-fingerprints-param` (rakia submodule)

## What changed

`brit-graph` gained `ContentFingerprint::from_repo_globs(repo, commit_id, patterns)` behind a new `repo` feature flag. It walks the git tree at a specific commit (NOT the working tree), matches paths against globs, reads blob bytes, and feeds them into the existing pure `ContentFingerprint::compute`. Same commit + same patterns yields identical fingerprints across machines.

`brit fingerprint <manifest> [--commit <ref>]` now uses it. Output fingerprints are 64-char blake3 hex strings derived from actual file contents — not glob pattern strings as before. Output JSON gains a `commit` field (the resolved 40-char SHA).

`brit plan` does the same: for each step in the plan, compute the fingerprint from the head commit's tree, pass to `to_build_plan` via the new `fingerprints: BTreeMap<String, String>` parameter. In `--files` mode (no commit context), fingerprints are empty strings — the documented sentinel.

`rakia-core::build_plan::to_build_plan` no longer computes fingerprints internally. The placeholder `compute_fingerprint` (a `DefaultHasher` over qualified-name + patterns) is deleted. Callers that have repo access (brit-cli) compute fingerprints and pass them in; callers that don't (test fixtures, schema-contract tests) pass `&BTreeMap::new()`. The schema accepts empty fingerprints as the no-repo-context sentinel (no `pattern` or `minLength` constraint on the field).

## Why

The previous fingerprint hashed glob pattern STRINGS, not file contents. Two repos with identical patterns and entirely different file contents produced identical fingerprints — useless for the "artifact X verified by N peers" attestation flow planned in future rakia sprints. This sprint fixes the primitive before downstream work depends on it.

The placeholder in `rakia-core::build_plan::compute_fingerprint` was an explicit anti-pattern in a content-addressed system: stable hash that wasn't content-derived. Callers might have treated its output as content identity when it wasn't. Deleting it removes that footgun.

## Verified properties

Asserted in `brit-graph/tests/repo_fingerprint.rs` (7 cases, all passing under `--features repo`):

- **Determinism**: same repo + same commit + same patterns → identical fingerprint
- **Content sensitivity**: same patterns, different file contents → different fingerprints (the property the OLD pattern-string hashing did NOT have)
- **Reproducibility**: reads from git tree, not working tree (skips uncommitted changes)
- **Empty case**: no patterns OR no matching files → stable empty-input fingerprint
- **Multi-pattern combination**: distinct patterns combine into one input map
- **Error path**: invalid glob produces typed `FingerprintError::InvalidGlob`

Validated end-to-end on the live elohim repo (`brit fingerprint app/elohim-app/build-manifest.json --step build-angular`):
- HEAD: fingerprint `9f42a992…`, 2040 input files
- HEAD~50: fingerprint `db7635d8…`, 2012 input files
- Different content (28 files added in those 50 commits) → different fingerprint ✓

`brit plan --since` output validates against `build-plan.schema.json` with real fingerprints in the `fingerprint` field.

## Discoveries

**The breadth-first tree visitor needed real implementation in path-tracking methods.** Initial scaffold left `push_back_tracked_path_component`, `pop_back_tracked_path_and_set_current`, `pop_front_tracked_path_and_set_current` as no-ops. Without them, gix's breadthfirst traversal lost the directory prefix when descending — `src/foo.ts` was visited as just `foo.ts`, so globs like `src/**/*.ts` matched nothing. Symptom: every fingerprint came back identical (empty inputs). Caught by Task 2's `single_pattern_matches_one_file` test. Fixed by mirroring gix's own `Recorder` pattern with a `VecDeque<BString>` snapshot queue. Lesson: gix's `Visit` trait has SEPARATE methods for `push_path_component` (your own bookkeeping) and the `_tracked_path_` family (used by the traversal engine to remember subtree contexts). Both must be implemented for breadthfirst to behave correctly.

**`gix::traverse::tree::visit::Action` in gix 0.81 is a type alias for `std::ops::ControlFlow<(), bool>`**, not the `Action::Continue` enum I'd assumed. `Continue(true)` = descend; `Continue(false)` = skip subtree; `Break(())` = halt. Caught at compile time, easy fix.

**No schema constraint on `fingerprint` field.** The `build-plan.schema.json` declares `fingerprint: { type: string }` with no `pattern` or `minLength`, so empty strings are accepted natively. The "no repo context" sentinel works without schema changes. If we ever want to enforce real-fingerprint-or-explicit-null, that's a future schema refinement (could be `pattern: "^([0-9a-f]{64})?$"` to allow 64-char hex or empty).

## Carry-overs

| Item | Why deferred |
|---|---|
| Submodule + symlink tree entries | Skipped today — only regular blobs. Defer until a manifest cares. |
| Working-tree mode (`--working-tree` flag) | Out of scope — must be tree-at-commit for reproducibility. Could add as explicit opt-out later. |
| Caching | Bounded today (2040 files takes <1s). Defer until profiling shows it matters. |
| Including executor declaration in fingerprint | Stage 2 work — requires BuildExecutor identity contract. |
| `planFingerprint: BritCid` field on BuildPlan | Composite-of-step-fingerprints for plan-level identity. Stage 2 dispatch primitive. |
| Schema constraint `pattern: "^([0-9a-f]{64})?$"` on fingerprint field | Optional tightening; not load-bearing today. |

## Sprint statistics

- **Tasks**: 11 / 11
- **Submodule commits**: 7 (brit) + 1 (rakia) = 8
- **Parent commits**: 2 (plan + sprint-result)
- **Lines added**: brit ~400, rakia ~80
- **Test count delta**:
  - brit-graph default features: unchanged (23 tests)
  - brit-graph `--features repo`: +7 (30 tests total)
  - brit-cli: +1 smoke test (2 total in cli_smoke.rs)
  - rakia-core: +1 unit test in build_plan.rs (3 total)
- **Eliminated anti-patterns**:
  - `DefaultHasher` placeholder in `rakia-core::build_plan::compute_fingerprint` (stable hash that wasn't content-addressed)
  - Pattern-string hashing in `brit-cli::fingerprint` (claimed content-addressing but produced pattern-addressing)

## What's next

The fingerprint primitive is ready for the rakia-runnable sprint to use as the work-unit identity for content-addressed dispatch and per-peer Build attestation anchoring. Each step in a `BuildPlan` now has a stable, content-derived BritCid hex that two peers computing from the same `(commit, patterns)` will agree on — that's the dispatch coordination primitive.

The next sprint scope (carried from the prior sprint's open questions): rakia-executor + `rakia ci` wrapper + ExecutionEvent + BuildAttestation. Build attestations will reference `(planFingerprint, stepFingerprint, outcome)` as the witnessed unit.
