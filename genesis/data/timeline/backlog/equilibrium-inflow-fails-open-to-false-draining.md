---
id: "backlog-equilibrium-inflow-fails-open-to-false-draining"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "equilibrium check FAILS OPEN: an unreadable git zeroes the inflow arm, so `outflow >= inflow` reports DRAINING — a fabricated equilibrium exactly when the check is blind"
slug: "equilibrium-inflow-fails-open-to-false-draining"
written: "2026-08-14"
author: "claude (saga leg-2 shift 2026-08-14T02-42, reproduced)"
status: "open"
priority: "high"
tags: [eprfs, epr-flow, equilibrium, stocks, stasis-termination, fail-closed, run-projection, instrument]
cites:
  - elohim/eprfs/epr-cli/src/flow/stocks.rs
  - .claude/hooks/run-projection.py
  - genesis/docs/superpowers/specs/2026-08-13-dev-system-equilibrium-stocks-design.md
---

# A blind inflow arm reports equilibrium instead of refusing

The equilibrium verdict is the proposed **termination criterion** for the stasis loops
(delivery-stasis, memory-stasis-loop): stasis is declared when `outflow >= inflow`
sustained. This defect makes that criterion assert its strongest claim — *drained* —
precisely in the state where it can see nothing.

## Reproduction (decisive, same window / stock / log bytes / binary)

```
# git unreadable (the `dubious ownership` state a fresh container hits
# before safe.directory is set):
HOME=/nonexistent epr flow stocks --window 2026-08-08..2026-08-15 --per week \
  --stock commitments --json --root /projects/elohim
  → inflow: 0.0   outflow: 2.0   level: 559.0   verdict: DRAINING

# control, identical arguments:
epr flow stocks --window 2026-08-08..2026-08-15 --per week \
  --stock commitments --json --root /projects/elohim
  → inflow: 23.0  outflow: 2.0   level: 559.0   verdict: FILLING
```

`.eprfs/status/flows.jsonl` byte-identical across both runs (2502991 bytes), same
git HEAD, same binary. **Observed in the wild, not synthetic:** the SessionStart fold
at 2026-08-14T17:05Z ran while git was in exactly that state and cached
`inflow 0.0 · verdict draining`; the run-plane headline then displayed
`outflow 2.00/wk · DRAINING` for a stock genuinely filling at 23/wk.

## Why it is fail-OPEN, and why that is the wrong direction

The level arm survives (reads the log) and the outflow arm survives — only inflow
collapses to 0. Since the predicate is `outflow >= inflow`, a zeroed inflow makes
**any** non-negative outflow satisfy equilibrium. The failure mode is silent and
maximally confident: no refusal, no marker, a clean `DRAINING`.

`stocks.rs` already has the correct machinery — `EquilibriumVerdict::Refused` with a
typed `RefusalReason` (`NoObservations`, `NotComparable`, `FoldRefused`) and the
discipline that "a weekly equilibrium claim that hides its window is not a claim."
The inflow path simply does not route this error class into it: an unreadable source
degrades to an empty event set rather than a `FoldRefused`.

## Fix direction (eprfs — outside the leg-2 shift's scope, hence filed)

1. In the inflow/outflow fold, distinguish **"the source said zero"** from
   **"the source could not be read"**; the latter must produce
   `RefusalReason::FoldRefused { kind: "inflow", error }`, never a 0.0 measure.
   The asymmetry is the tell: any state where inflow and outflow arms disagree about
   whether the source was READABLE is a refusal, not a verdict.
2. Add a table-test alongside the existing verdict tests: source-unreadable ⇒
   `Refused`, and specifically NOT `Draining`.
3. Audit any other arm that maps an I/O error to an empty collection on this path.

## Already mitigated (in-scope, landed this shift)

`.claude/hooks/run-projection.py` now gates the SessionStart fold on `git_readable()`
and records an honest absence instead of a verdict when git cannot read the repo
(fail-closed). That keeps the *headline* honest; it does **not** fix `epr` itself, so
any other caller of `epr flow stocks` — a stasis loop calling it directly — is still
exposed. This item stays open until the typed refusal lands in the Rust verdict arm.

## Verification hook

`HOME=/nonexistent epr flow stocks … --json` exits non-zero (or emits
`verdict: refused` with `reason: fold-refused`) instead of `draining`.
