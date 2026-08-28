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

## 2026-08-28 — the opposite polarity, and why it must NOT be "fixed"

The defect above is over-COARSE (one string, many concerns). The mirror case is
over-FINE: an a2o assertion renders the OBSERVED value into its headline, so one
scenario mints a fresh fingerprint — and a fresh background triage dispatch —
per value the substrate happens to report:

```
4b6fe47bfdb3  alpha-A /health: p2p.caughtUp is false (expected true)       builds 1499..1513
a672ee4586c6  alpha-A /health: p2p.caughtUp is undefined (expected true)   build  1514
```

Same feature (`dataplane/inventory-convergence.feature:42`), same step
(`steps/dataplane.steps.ts:451`), same peer. The message is
`` `${peer} /health: p2p.caughtUp is ${JSON.stringify(body.p2p?.caughtUp)} (expected true)` ``,
so the interpolated value is load-bearing in the fingerprint.

The obvious fix — a `normalize()` rule collapsing `is <observed> (expected <x>)`
to `is # (expected <x>)`, in the same family as the existing agent-pod-name rule
(which exists verbatim to stop "one background triage dispatch per pod name") —
was **evaluated and REJECTED on 2026-08-28.** The two values are two different
code paths with two different root causes (`routes/health.rs`):

- `caughtUp: false` → a p2p snapshot IS cached, carrying `Some(false)`:
  storage's projection-reconcile ran and reports *behind*.
- `caughtUp` absent → `#[serde(skip_serializing_if = "Option::is_none")]` on
  `Option<bool>` omits the key entirely: the doorway has **no** snapshot
  (`P2PHealth::default()`) — too young, or unable to reach storage at all.

Collapsing them would have destroyed the field that distinguishes "the
substrate is behind" from "the doorway was just restarted", which is exactly
the distinction that root-caused genesis #1514
(`backlog/ci-genesis-doorway-503-seed-phase-wedge.md`). So the ledger's
2-fingerprint cost here bought a diagnosis and was worth paying.

**The rule this yields:** normalize what is INCIDENTAL to the concern (pod
names, build ids, timestamps, ports); never normalize a value the assertion
was written to REPORT. When in doubt, read the producing code — if the
observed value selects a code path, it is signal, and the right remedy for
the extra dispatch is a backlog entry that names both fingerprints as one
concern (which is what the sentinel is for), not a coarser measure.
