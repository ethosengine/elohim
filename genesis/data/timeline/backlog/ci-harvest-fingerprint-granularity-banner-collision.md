# ci-harvest fingerprint granularity — banner/aggregate lines collide across concerns

- **status:** open
- **class:** ci-tooling
- **filed:** 2026-07-12 (convergence-bank shift; observed live on elohim-holochain/dev #1357)
- **owner-discipline:** ci-harvest maintainer (deterministic layer, `.claude/scripts/ci-harvest.py`)

## The defect

`ci-harvest.py` fingerprints non-specific aggregate/banner log lines
independently of the test-name line. Two live examples:

- `b3696104006e` = `"test result: FAILED. 0 passed; 1 failed; …"` (nextest
  per-shard aggregate — emitted by EVERY single-test sweettest failure)
- `56ec1c027ba6` = `"DNA BUILD FAILED"` (pipeline post-action banner)

Because these strings recur for *any* failing test, the harvester reopens
whichever backlog first claimed them and mis-attributes the recurrence. On
2026-07-12 both were "reopened" against the (genuinely cured, #1355/#1356
green) partition-isolation concern when the UNRELATED convergence-test red
at #1357 re-emitted the same banner strings — a false reopen that cost a
triage dispatch.

## The fix shape

Suppress fingerprinting of known aggregate/banner patterns (shard summaries,
post-action banners) OR key them compositely with the nearest preceding
test-name line so the fingerprint is per-concern, not per-banner. The
specific-test fingerprints (e.g. `b195ea5b6587`, `e14ce1a7e360` for the
convergence tests) already behave correctly — only the banner class needs
the change.

## Evidence trail

- ci-failure-triage run 2026-07-12 (this shift) — verdict "fingerprint
  collision, not genuine recurrence", no ledger mutation made.
- `.claude/data/ci-findings.jsonl` lines carrying the two fingerprints above.
