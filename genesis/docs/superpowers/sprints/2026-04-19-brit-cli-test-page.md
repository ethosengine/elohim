# Sprint Result: Brit CLI Test Page

**Date:** 2026-04-19 (sprint close)
**Spec:** `docs/superpowers/specs/2026-04-19-brit-cli-test-page-design.md`
**Plan:** `docs/superpowers/plans/2026-04-19-brit-cli-test-page.md`
**Branch:** `feat/brit-cli-test-page` (brit submodule)

## What changed

Hybrid CLI test infrastructure shipped end-to-end. Two layers, one runner, one committed artifact:

- **`cli-journey`** crate (test infrastructure): `TestRepo` + `MockRemote` + `Normalizer` + `BritInvocation` helpers. 15 self-tests covering the helpers themselves; 23 integration tests covering rakia, brit-verify, and brit-build-ref (38 cli-journey tests total, all passing).
- **`cli-test-page`** crate (runner binary `brit-test-page`): subcommand discovery via recursive `--help` parsing, coverage computation, markdown formatter, similar-crate diff, three modes (`--check`/`--update`/`--candidate`). 11 self-tests, all passing.
- **`tests/baseline.md`** committed: 31 covered subcommands across 4 binaries, with full per-subcommand captured output + auto-discovered uncovered list.

Test count delta:
- Before sprint: 0 cli-test infrastructure tests
- After sprint: 49 (15 cli-journey support + 23 cli-journey per-binary + 11 cli-test-page)

## Coverage status (initial baseline)

| Binary | Covered | Total | % |
|---|---|---|---|
| brit | 12 | 70 | 17% |
| rakia | 8 | 8 | 100% |
| brit-verify | 1 capture | 0 leaves discovered | n/a (single-binary) |
| brit-build-ref | 11 | 11 | 100% |
| **Combined** | **31** | **89** | **34%** |

Three of four binaries fully covered. The remaining 58 brit subcommands are listed explicitly in `baseline.md`'s Uncovered section — they're the carry-over for follow-up sprints (each is ~10 lines: TestRepo + invocation + staging dump).

## Verified properties

- **TDD discipline**: every helper landed via failing-test-first → implementation → green
- **Determinism**: `--update` followed by `--check` shows ~no diff (one known edge case in `brit cat`, see carry-overs)
- **Round-trip workflow**: `--candidate /tmp/desired.md` → edit → iterate brit code → `--check` goes green (the TDD-redesign loop sketched in the spec)
- **Mock remotes work**: `brit clone file://<MockRemote>` succeeds in tests; no daemon, no network
- **Hybrid layers compose**: shell journey tests (rakia.sh, brit-verify.sh, brit-build-ref.sh) and Rust integration tests (rakia.rs, brit_verify.rs, brit_build_ref.rs, brit.rs) feed the same staging directory; runner picks up both

## Discoveries during dogfood-prep

These came out of writing actual coverage tests for brit's subcommands and become real UX work for the future "redesign brit for elohim-protocol" sprint:

1. **`brit push` is not implemented in gitoxide upstream.** Returns "error: unrecognized subcommand 'push'". Daily-driver brit users can't push. Either gitoxide gets a push implementation (upstream contribution), or brit ships its own push wrapper. Captured staging slot has a documented gap notice.

2. **`brit commit` is not what `git commit` is.** Subcommands are `verify`, `sign`, `describe` — not "make a new commit." How does a brit user MAKE a commit today? Via `git commit` (since brit doesn't have one) or via direct API. This is a fundamental gap for the daily-driver story.

3. **`brit branch` and `brit tag` only have `list` subcommands.** No `create` or `delete`. Users still need `git` for branch/tag mutations.

4. **`brit diff` has its own subcommand tree** (`tree <OLD> <NEW>`, `file <OLD_REVSPEC> <NEW_REVSPEC>`) — not a flat `brit diff HEAD HEAD~1` like git. Different invocation syntax may be friction.

5. **gitoxide progress output is a normalization minefield.** Wall-clock times, throughput rates (objects/s, MB/s), durations all vary per run. Normalizer was extended to redact `<CLOCK>`, `<RATE>`, `<DUR>` patterns, but other variable bytes may surface as more subcommands get coverage.

6. **`brit-verify` uses a hand-rolled arg parser, not clap.** `brit-verify --help` errors with "failed to resolve rev --help" because --help is interpreted as a positional rev. Discovery walking via `--help` doesn't enumerate it as a subcommand-tree binary.

## Carry-overs

| Item | Why deferred |
|---|---|
| **Cover the remaining 58 brit subcommands** | Daily-driver subset (12) is enough to validate the pattern; rest fills in incrementally as we dogfood and hit them |
| **CI integration** (`brit-test-page --check` in a Jenkins stage) | brit submodule has no Jenkinsfile yet; needs a new pipeline registered with the orchestrator. Separate concern. |
| **Resolve `brit cat` non-determinism** (5 lines of diff between consecutive runs) | Likely root cause: brit cat on a commit OID sometimes resolves to commit text, sometimes to underlying blob — possibly a gitoxide quirk |
| **Implement or wrap `brit push`** | Upstream gap; needs design decision (contribute to gitoxide vs ship a brit-specific wrapper) |
| **Resolve `brit commit` semantics for daily-driver use** | Either rakia/brit-cli provides a `commit` wrapper, or gitoxide's `commit` subcommand grows a "make a commit" leaf |
| **Wire `dump-to-staging` into existing journey shells** | Helper exists in utilities.sh; not yet called from rakia.sh/brit-verify.sh/brit-build-ref.sh because Rust integration tests cover the same surface more richly. Useful for future shell-only test scenarios. |
| **Snapshot test for `brit-test-page` itself** | The runner has good unit tests but no full end-to-end fixture-based test. Could add `tests/test-page-end-to-end.rs` that runs the runner against a synthetic small repo and asserts on the produced markdown. |

## Sprint statistics

- **Tasks completed:** 22 / 22 (Task 17 recast from "extend gix.sh" to "new brit.rs Rust integration tests" — see commit `6795db17`)
- **Submodule commits:** 19 in brit (one per task plus the small fixes)
- **Lines added (brit submodule):** ~3,500 across the cli-journey crate, cli-test-page crate, journey/*.sh files, and baseline.md
- **Test count delta:** 0 → 49 (38 cli-journey + 11 cli-test-page)
- **Binaries covered:** 4 (brit, rakia, brit-verify, brit-build-ref)
- **Coverage achieved:** 34% combined; 100% on the 3 small binaries; 17% on brit (daily-driver subset)
- **Initial `baseline.md` size:** 21KB, 986 lines

## What's next

Three threads are now naturally enabled:

1. **Continue dogfooding `brit` for daily git ops.** Each friction point becomes a captured staging entry + (eventually) a code fix. The TDD loop (`--candidate desired.md` → edit → iterate) is now the design tool for command-by-command redesign.

2. **Chip away at the brit coverage gap.** 58 uncovered subcommands. Pattern is established; each new test is ~10 lines. Could fold into the daily-dogfooding loop ("hit a subcommand that wasn't covered → add a test").

3. **The bigger thread A consolidation** (deferred): fold brit-verify and brit-build-ref into `brit verify` / `brit build-ref` subcommands of the unified binary. Test infrastructure already in place to catch regressions during that refactor.

Bigger design horizon stays the same: redesign brit subcommands command-by-command for elohim-protocol semantics. The test suite is now the harness — write desired output, code to match, lock in via baseline update.
