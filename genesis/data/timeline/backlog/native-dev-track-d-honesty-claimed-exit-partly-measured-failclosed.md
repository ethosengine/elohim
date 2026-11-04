---
id: "backlog-native-dev-track-d-honesty-claimed-exit-partly-measured-failclosed"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Track D [integrity]: three honesty gaps - claimed gets its own exit code, partly-measured habits, integrity validators fail closed"
slug: "native-dev-track-d-honesty-claimed-exit-partly-measured-failclosed"
written: "2026-11-04"
author: "claude (design pass, native-dev-plane spec; steward human:matthew)"
status: "backlog"
priority: "high"
relatedNodeIds:
  - "native-dev-plane-one-derivation-earned-reach-design"
tags: [native-dev-plane, integrity, honesty, unmeasured-vs-zero, gate-attest, habits, epr-meta, track-d]
---

Source spec: `native-dev-plane-one-derivation-earned-reach-design` (genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md).

## Item (spec §7 rules 1-3, §11 Track D) [integrity]

Write-set: `genesis/orchestrator/gate-attest.mjs`, `.claude/scripts/habits-status.py`, `elohim/eprfs/eprfs-meta`.

**D1. `claimed` exits distinctly.** Verified: `gate-attest.mjs:108-110` returns `outcome: 'pass', tier: 'claimed'` when `read.read !== 'ok'`. Give claimed its own exit code so exit-code-only consumers stop reading it as green. Unit test.

**D2. Partly-measured.** Verified in part: `habits-status.py` `observe_concern` (about :494-505) already returns NOT MEASURED when passed==0, and a label "N of M scenarios never ran" exists (:565-569). The remaining gap is passed>0 with some skipped or pending: the docstring says passed>0 with no failure is green. Add a `partly-measured` state for that case. Unit test.

**D3. Integrity-class validators fail closed.** Correction to the spec's wording: the Rust evaluator (`eprfs-meta/src/evaluation.rs:~767-805`) already clamps an unresolvable validator to the rule's declared class: deny/ask routes to `ask`, and inject stays inject by design. So the gap is narrower: an integrity-class rule (signing, attestation, evidence admission) declared `inject` still degrades to advisory. Cure: an integrity marker on the rule (or a lint that integrity-class rules cannot be declared inject) so unresolvable resolves to ask or deny regardless. Check the Python hook `.claude/hooks/epr-meta-resolver.py` agrees (it defers to `epr` on decision).

**Related.** `quiesce-preflight-absent-unmeasured-reads-as-pass` is the same family on a different gate.

**retire-when.** Each gap has a unit test and no consumer maps `claimed` or partial skips to green.

## 2026-11-04 (adversarial review): the claimed ladder (spec §7 rule 2, §11)

An exit code alone is not the whole cure. Attested projects have no local recipe (`genesis/orchestrator/gate-runner.mjs:309`), so an unreachable forge read leaves nothing to fall back to. The honest consequence is a ladder:

1. A `DerivationRecord` for the pinned commit's check, from this device or a sibling, admits it.
2. Failing that, `claimed` is admitted **for landing only**, as a recorded debt shown in the watch view, settled by the next reachable read; if that read is red the landing reopens.
3. `claimed` is never admitted for release adoption or reach graduation.

A forge outage then costs neither landing nor honesty. Receipt additions: unit test that `claimed` exits with its own code and is refused by the release and graduation consumers; test that a red follow-up read reopens a claimed landing.

**Gating.** D now gates joins F1r, F3, F5 and F6 (F3 so `claimed` never lands silently).

**2026-11-04 (confirmation review).** Debt is inherited through closures. Unsettled `claimed` debt is
keyed to the pinned gitlink. Every derivation whose input root includes that gitlink inherits it, and
so does every release closure built on such a derivation. Release adoption and graduation check the
whole closure and refuse while any inherited debt is unsettled. A red settling read writes contest
records against every dependent derivation and halts and reverts adoption of affected releases. See
the spec §7 rule 2. Receipt: A lands with debt, B builds on A, and B's release is refused until A
settles.
