---
title: Verification-Result Index — the system→state store that closes the stasis loop
id: verification-result-index-design
status: Draft
created: 2026-06-01
topic: [verification, state-index, ci, regression, done-gate, memory-stasis]
cites:
  - placement | the contract whose four verification states this index records rather than forks | sha256:f84d7cb16bea9379
  - che-browser-completion-oracle-design | the L2 done-gate oracle this composes with to resolve claimed→done/regressed | sha256:355cc8523a03f33b
# Produced via /brainstorm (coherence-wrapped): composed from the prior art the pre-step surfaced;
# born auditable (status + topic + cites) so it never enters the budget as no-status/orphan debt.
---

# Verification-Result Index

**Composed from** [`genesis/docs/PLACEMENT.md`](../../PLACEMENT.md) (the four verification states) + the L2
done-gate oracle ([`2026-05-30-che-browser-completion-oracle-design.md`](2026-05-30-che-browser-completion-oracle-design.md)).
This does **not** fork a new state model — it *records* the states PLACEMENT already defines.

## Problem

`placement-audit.py` can derive `BLOCKED-BY-ENV` (from cluster-state) and `CLAIMED` (status without evidence),
but it cannot deterministically tell `CLAIMED → DONE` or `→ REGRESSED` — there is no machine-readable store of
*verification results* per system. So the loop's back half (a claim auto-resolving to done/regression) is still
manual. This is the last gap before the memory-stasis loop fully closes.

## Requirements

- A `system-state-index.json` keyed by system / gap-id → `{state: done|regression|claimed|blocked, evidence, checked_at_commit}`.
- ci-investigator (and CI) WRITE verification results into it — the grader, never the claimant.
- `placement-audit.py` READS it: a `CLAIMED` gap with a passing result becomes `DONE`; with a failing result becomes `REGRESSION`.
- A regression result must cascade-warm the cited docs (the feedback graph) per `PLACEMENT.md`.
- The index is append/update-only with provenance; it is never hand-edited to assert done (a claim cannot grade itself).

## Non-goals

- Not a new state vocabulary (reuse PLACEMENT's four).
- Not a CI system (it consumes CI / ci-investigator output).
