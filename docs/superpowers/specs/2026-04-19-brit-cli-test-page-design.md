# Brit CLI Test Page — Design

**Date:** 2026-04-19
**Status:** Approved (brainstorming complete, awaiting spec review)
**Author:** Matthew Dowell + Claude Opus 4.7
**Predecessor sprints:**
- `docs/superpowers/sprint-results/2026-04-19-rakia-describable.md`
- `docs/superpowers/sprint-results/2026-04-19-brit-content-fingerprint.md`

## TL;DR

Build a comprehensive CLI test suite + single-command "test page" runner for the four user-facing brit-workspace binaries (`brit`, `rakia`, `brit-verify`, `brit-build-ref`). Hybrid architecture: extend gitoxide's existing shell-based journey-test framework where it already covers brit (was gix); add a new Rust integration-test crate for the structured-output binaries (`rakia`, `brit-verify`, `brit-build-ref`); unify both behind a single Rust runner that produces a markdown "test page" (`baseline.md`) capturing every CLI subcommand's invocation + actual output.

The runner supports a baseline/candidate workflow that turns CLI redesign into TDD: hand-edit a candidate file to define desired output, then iterate on brit code until actual matches desired. Once aligned, copy candidate over baseline to lock in the new behavior. The same workflow is what protects against accidental output regressions in CI (`--check` mode).

100% subcommand coverage of the four binaries is the milestone. Coverage tracking (auto-discovered by recursively parsing `--help`) lives in the test page itself, so coverage gaps are visible at a glance.

## Problem

We have four CLI binaries shipped from the brit workspace:

| Binary | Surface | Current test coverage |
|---|---|---|
| `brit` (gitoxide-derived git client) | ~35 top-level subcommands, recursive deeper | Partial — gitoxide's `tests/journey/gix.sh` (705 lines) covers some |
| `rakia` (build/CI orchestrator) | 6 top-level subcommands | Minimal — 2 tests in `brit-cli/tests/cli_smoke.rs` |
| `brit-verify` (trailer verification) | ~1 main mode | None |
| `brit-build-ref` (attestation refs) | ~3 subcommands | None |

We don't know which subcommands work, which are broken, or what their output looks like. We're about to dogfood `brit` daily; the moment we hit a UX gap we want to capture it as a test (not just remember it). And we want to redesign `brit` command-by-command to align with the elohim-protocol — which requires being able to define desired output, then refactor to it.

A static markdown inventory rots; a one-shot manual audit captures a snapshot in time but doesn't catch tomorrow's regressions. The fix is an executable specification: every CLI invocation lives in a test, every actual output is captured, the diff against committed baseline IS the regression detector.

## Architecture

### Two layers, one runner, one output

```
+-----------------------------------------------------------+
|                     brit-test-page                        |
|     (Rust runner — invokes both layers, formats MD)       |
+-----------------------------------------------------------+
            ↓ shell layer            ↓ Rust layer
+--------------------------+    +-----------------------+
| journey.sh (extended)    |    | cli-journey crate     |
| - gix.sh (extended)      |    | - rakia.rs            |
| - rakia.sh, verify.sh,   |    | - verify.rs           |
|   build-ref.sh (new)     |    | - build_ref.rs        |
| Bash BDD                 |    | assert_cmd + insta    |
| Snapshot files in        |    | + custom normalization|
| tests/snapshots/         |    |                       |
+--------------------------+    +-----------------------+
            ↓                            ↓
                  Output captures
                       ↓
              tests/baseline.md (committed artifact)
```

**Why hybrid:**

- **Shell layer** preserves gitoxide's existing test investment and stays compatible if we ever rebase brit on a newer gitoxide. The shell BDD pattern (`title`/`when`/`it`/`expect_run`) is mature; the snapshots dir already has fixtures for many commands. We extend, don't replace.
- **Rust layer** handles structured JSON output (where shell's assertion tools struggle), per-test fixture isolation, and the rich error-path tests for `rakia`/`verify`/`build-ref`.
- **Single runner** in Rust unifies both layers' outputs into `baseline.md` so consumers see one artifact.

### Cross-layer interface

The shell journey tests already write deterministic snapshots to `tests/snapshots/plumbing/<command>/`. The Rust runner reads these snapshots (after invoking journey.sh) and includes them in the markdown. The Rust integration tests under `cli-journey/` write their captured outputs to a known intermediate location that the runner picks up.

A simple file convention bridges the two:

```
tests/.test-page-staging/
├── shell/        ← journey tests dump captured outputs here
│   └── gix/
│       └── log.txt
└── rust/         ← Rust tests dump here
    └── rakia/
        └── graph_discover.txt
```

The runner walks both subtrees, formats by binary + subcommand path, emits markdown.

## Components

### File structure (new + modified)

```
elohim/brit/
├── tests/
│   ├── journey.sh                                # MODIFY (add new sourced files)
│   ├── journey/
│   │   ├── gix.sh                                # MODIFY (extend coverage gaps for brit)
│   │   ├── ein.sh                                # NO CHANGE
│   │   ├── rakia.sh                              # NEW (smoke shell tests for rakia)
│   │   ├── brit-verify.sh                        # NEW
│   │   └── brit-build-ref.sh                     # NEW
│   ├── snapshots/                                # EXISTING — extended with new captures
│   ├── fixtures/                                 # EXISTING — extended where needed
│   ├── helpers.sh                                # NO CHANGE
│   ├── utilities.sh                              # NO CHANGE
│   ├── cli-journey/                              # NEW Rust integration test crate
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs                            # support module exports
│   │   │   └── support/
│   │   │       ├── mod.rs
│   │   │       ├── test_repo.rs                  # TestRepo helper (fixture-builder)
│   │   │       ├── mock_remote.rs                # bare-repo + file:// transport
│   │   │       ├── normalize.rs                  # tempdir/SHA/timestamp/ANSI redaction
│   │   │       ├── runner.rs                     # invoke binary + capture + dump to staging
│   │   │       └── fixtures/                     # static fixture data
│   │   └── tests/
│   │       ├── rakia.rs                          # rakia subcommands (graph/affected/plan/etc.)
│   │       ├── brit_verify.rs
│   │       └── brit_build_ref.rs
│   ├── cli-test-page/                            # NEW Rust runner crate
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs                           # CLI entry: --check / --update / --candidate
│   │       ├── discover.rs                       # recursive --help parsing per binary
│   │       ├── format.rs                         # markdown emission
│   │       ├── coverage.rs                       # coverage % computation
│   │       ├── diff.rs                           # baseline vs candidate diff (colored terminal)
│   │       └── normalize.rs                      # shared normalization (or re-exports cli-journey's)
│   └── baseline.md                               # NEW committed artifact
└── Cargo.toml                                    # MODIFY (add cli-journey + cli-test-page to workspace.members)
```

### Component responsibilities

- **`cli-journey/src/support/test_repo.rs`** — `TestRepo` struct that creates a temp git repo with a deterministic commit history (uses `set-static-git-environment` equivalent for stable SHAs). `Drop` cleans up.
- **`cli-journey/src/support/mock_remote.rs`** — `MockRemote` = bare repo at temp path. `file://` URL accessor. Pairs with TestRepo for clone/fetch/push tests.
- **`cli-journey/src/support/normalize.rs`** — `normalize(text, &NormalizationContext) -> String` applies regex-based replacements: tempdir paths, ANSI codes, variable timestamps, variable SHAs (the context tracks which SHAs are "stable" because they were generated from fixed-content commits and which need redaction).
- **`cli-journey/src/support/runner.rs`** — `BritInvocation` builder + `.run()` that captures stdout/stderr, applies normalization, writes to the staging directory. Each test gets ONE call per command being tested.
- **`cli-test-page/src/main.rs`** — `clap`-based entry point with three modes:
  - `--check` (default) — runs both layers, generates candidate in memory, diffs against `baseline.md`, exits 0/1
  - `--update` — runs and copies candidate over `baseline.md`
  - `--candidate <path>` — runs and writes candidate to the given path; doesn't touch `baseline.md`
- **`cli-test-page/src/discover.rs`** — for each of the 4 binaries, invoke `<bin> --help`, parse subcommand list, recurse into each subcommand's `--help`, build a tree of leaf subcommands. Output: `Vec<SubcommandPath>` like `["brit", "log"]`, `["brit", "branch", "list"]`, etc.
- **`cli-test-page/src/coverage.rs`** — match the discovered universe against the staging directory contents to compute X-of-Y coverage per binary.
- **`cli-test-page/src/format.rs`** — emits the markdown report (TOC, coverage summary, per-binary sections, per-subcommand blocks with help text + invocation + captured output).
- **`cli-test-page/src/diff.rs`** — colored terminal diff using `similar` crate (well-established in Rust ecosystem).

## The TDD baseline-candidate workflow

Three modes, one runner:

```bash
# Default: did we regress?
brit-test-page --check
# Generates candidate in memory, diffs against tests/baseline.md
# Exit 0: clean. Exit 1: diff printed to stderr; CI fails.

# Update baseline after review:
brit-test-page --check                  # see what changed
brit-test-page --update                 # accept changes (cp candidate -> baseline.md)
git diff brit/tests/baseline.md         # final review before commit
git add brit/tests/baseline.md && git commit ...

# TDD redesign workflow:
brit-test-page --candidate desired.md   # snapshot current state
$EDITOR desired.md                      # hand-edit to what brit SHOULD output
# ... iterate on brit code:
while ! diff -q desired.md <(brit-test-page --candidate /dev/stdout); do
    $EDITOR src/...                     # change brit
    cargo build -p gitoxide --bin brit
done
cp desired.md tests/baseline.md         # lock in new behavior
```

The `--candidate <path>` mode writes to an arbitrary path (could be `/tmp/desired.md`, could be `/dev/stdout` for piping, could be `tests/baseline.md` directly if you're feeling brave). Default `--check` and `--update` use `tests/.test-page-candidate.md` as the implicit candidate path.

## Output normalization

The baseline file must be byte-stable across runs and machines. Normalizations:

| Variable in raw output | Replaced with | Rationale |
|---|---|---|
| Temp dir paths (`/tmp/.brit-XXXX/`, `/var/folders/.../...`) | `<TMPDIR>/` | Path varies per run |
| ANSI escape sequences | stripped | Platform/terminal-dependent |
| Wall-clock RFC3339 timestamps | `<TIMESTAMP>` | E.g. `generated_at` field in BuildPlan |
| Process IDs | `<PID>` | Rare but possible |
| Random ports (test mock servers) | `<PORT>` | N/A in `file://` mock-remote design but reserved for future |
| **Variable** git SHAs (commits made with `now()` authorship) | `<SHA>` | Vary per run |
| **Stable** git SHAs (against fixed-content commits) | left as-is | Static via `set-static-git-environment` |
| Cargo target paths in error messages | `<CARGO_TARGET>` | E.g. linker errors that escape |

The runner constructs a `NormalizationContext` per test that tracks which SHAs are stable (computed from the fixture's known content) vs variable (anything generated during the test). Stable SHAs flow through verbatim — they ARE part of the contract being tested. Variable SHAs get redacted.

Reusing gitoxide's `set-static-git-environment` (deterministic GIT_AUTHOR_DATE/EMAIL/etc.) means most fixture commits produce stable SHAs naturally.

## Mock remotes

Network-bearing commands (`brit clone`, `brit fetch`, `brit push`, `brit remote`) need test coverage without external network. The pattern: bare git repo at a temp path, `file://` URL.

Shell:

```bash
function with-mock-remote() {
  local upstream="${TMPDIR:-/tmp}/test-upstream-$$.git"
  git init --bare "$upstream" >/dev/null
  echo "file://$upstream"
}

upstream_url="$(with-mock-remote)"
brit clone "$upstream_url" local
(cd local && touch x && git add x && brit commit -m one && brit push)
```

Rust:

```rust
let upstream = MockRemote::new();              // bare repo, file:// URL
let local = TestRepo::clone(&upstream).unwrap();
local.commit_file("x", "content")?;
local.push("origin", "main")?;
```

`file://` is universally supported, requires no daemon, deterministic. Covers all the network-flow test scenarios we need.

## Coverage tracking

The runner enumerates every leaf subcommand by recursively walking `--help`:

```
brit
├── archive
├── blame
├── branch
│   ├── list
│   ├── create
│   └── delete
├── ...

rakia
├── graph
│   ├── discover
│   └── show
├── affected
├── plan
├── fingerprint
└── baseline
    ├── read
    ├── write
    └── migrate

brit-verify (single command, no subcommands)
brit-build-ref
├── new
├── list
└── ...
```

Each test registers what it covers (filename convention: `tests/<binary>/<subcommand>/...` OR a metadata header `// COVERS: brit log`). The runner cross-references discovered universe vs registered coverage and emits the report header:

```markdown
## Coverage Summary

| Binary | Covered | Total | % |
|---|---|---|---|
| brit | 38 | 40 | 95% |
| rakia | 8 | 8 | 100% |
| brit-verify | 1 | 1 | 100% |
| brit-build-ref | 3 | 3 | 100% |
| **Total** | **50** | **52** | **96%** |

### Uncovered subcommands
- brit corpus
- brit free progress
```

100% across all four binaries is the "test page of a printer" milestone — the moment you can run one command and know every shipped CLI surface still works.

## CI integration

Add a Jenkins job stage (or extend an existing one) that runs:

```bash
brit-test-page --check
```

Two failure modes signal:

1. **Output diff** — actual output changed vs baseline. Could be intentional (somebody updated `brit log` formatting on purpose, needs corresponding `--update` commit) OR accidental (regression).
2. **New uncovered subcommand** — somebody added a CLI subcommand without adding a test. Coverage % drops below 100% (after we've reached it). Forces "test goes with the code that ships."

Both produce build failures with actionable output (the diff, the missing-coverage list).

## Acceptance Criteria

### Test infrastructure

- [ ] `cli-journey` crate compiles, integrated into brit workspace
- [ ] `cli-test-page` crate compiles, integrated into brit workspace
- [ ] `TestRepo` and `MockRemote` helpers usable from Rust tests
- [ ] Normalization correctly redacts tempdir paths, ANSI codes, variable timestamps, variable SHAs
- [ ] Static git environment produces stable SHAs across runs (verified by running the suite twice and diffing)

### Coverage

- [ ] 100% of `brit` subcommands have at least one test (test invokes the command, captures output)
- [ ] 100% of `rakia` subcommands covered
- [ ] 100% of `brit-verify` covered
- [ ] 100% of `brit-build-ref` covered
- [ ] Coverage % is computed and reported at the top of `baseline.md`

### Runner modes

- [ ] `brit-test-page --check` exits 0 when actual matches baseline; exits 1 with diff on mismatch
- [ ] `brit-test-page --update` overwrites `baseline.md` with current actual
- [ ] `brit-test-page --candidate <path>` writes candidate to arbitrary path

### TDD workflow validation

- [ ] Demo: hand-edit a section of `baseline.md`; running `--check` produces a diff for that section
- [ ] Demo: edit brit code to match the hand-edited baseline; `--check` returns to green

### CI

- [ ] `brit-test-page --check` runs in CI for any change to brit/rakia binaries
- [ ] Failure output includes the diff (so you know what broke)

### Artifact

- [ ] `tests/baseline.md` committed to repo with 100% coverage at sprint close

## Out of Scope

| Item | Why deferred |
|---|---|
| Redesigning command outputs to align with elohim-protocol | This sprint just captures CURRENT behavior. Redesign work happens command-by-command afterward, using the test suite as the TDD harness. |
| `ein` binary coverage | Less commonly used; gitoxide-derived; defer. The pattern is identical if/when added. |
| HTML/web rendering of the test page | YAGNI; markdown is enough today. |
| Performance benchmarks | Different concern; not output coverage. |
| Mock HTTP/SSH transports | `file://` covers the network test scenarios we need. |
| Parallel test execution | If suite gets slow, optimize then. Don't pre-optimize. |
| Snapshot review UI (`cargo insta review` equivalent) | The markdown baseline IS the review surface. Diff via standard tools. |

## P2P Design Gate Classification

This is pure developer tooling — test infrastructure for the CLI binaries. No protocol entities introduced.

| Entity | Classification | Justification |
|---|---|---|
| `TestRepo` (test fixture) | Operational (C) | Ephemeral temp directory; no persistence; reconstructable from scratch |
| `MockRemote` (test fixture) | Operational (C) | Same |
| `baseline.md` (committed artifact) | Operational (C) | A specification of CLI behavior, NOT a protocol entity. Source of truth: this file in git |
| Coverage statistics | Operational (C) | Computed at run time |
| Normalization context | Operational (C) | In-memory only |

No DHT, no attestations, no cross-peer communication. Test infrastructure operates entirely in the dev-loop substrate (local file system + git tree).

The `baseline.md` is a CONTRACT artifact in the same way that `build-plan.schema.json` is a contract artifact — it formalizes expected behavior. The diff against it IS the IoC enforcement. Same discipline, applied to CLI behavior instead of data shape.

## Key Design Decisions

1. **Hybrid (shell + Rust), not all-Rust.** Preserves gitoxide's existing 705 lines of journey tests; doesn't fight upstream. Rust layer handles only the structured-output binaries where shell assertion is painful.

2. **Single Rust runner produces one artifact (`baseline.md`).** Whatever layer captures the output, the format consumers see is unified.

3. **Baseline-candidate workflow modeled on `insta` snapshots, but at the human-readable markdown level.** Same discipline (commit baseline, diff candidate, accept after review), but the artifact is one file you can reason about as a whole — vs N `.snap` files you can't see together.

4. **`file://` mock remotes, no daemon.** Covers clone/fetch/push test scenarios without external network or test-server infrastructure.

5. **Auto-discovered coverage via recursive `--help` parsing.** No manual coverage-list maintenance; no risk of forgetting to add a new subcommand to the registry. Coverage gaps surface naturally.

6. **Static git environment from gitoxide's helpers.** Deterministic SHAs without test-side gymnastics.

7. **CI fails on output diff OR missing coverage.** Both signals matter; the build pressure forces tests-with-code.

8. **TDD-redesign workflow as first-class.** The `--candidate <path>` mode exists specifically so you can write desired output by hand and iterate code to match. This is the design's reason for being beyond regression detection.
