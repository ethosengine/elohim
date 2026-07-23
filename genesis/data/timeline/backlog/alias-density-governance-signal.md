---
id: "backlog-alias-density-governance-signal"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Alias density is a drift precursor — classify + stack-rank alias clusters as an epr-meta governance signal"
slug: "alias-density-governance-signal"
written: "2026-07-23"
author: "claude (reach-vocab slice-3 arc, operator-seeded)"
status: "backlog"
priority: "medium"
tags: [governance, epr-meta, vocabulary-drift, aliasing, code-smell, sentinel, sense-respond]
cites:
  - genesis/docs/superpowers/specs/2026-07-22-reach-ontology-vocabulary-split-spec.md
  - genesis/data/timeline/backlog/reach-vocabulary-frontend-strand.md
  - elohim/sdk/storage-client-ts/src/protocol-core.model.ts
  - app/elohim-library/projects/elohim-service/src/cache/types.ts
---

# Alias density is a drift precursor — govern it, don't just clean it

**Operator observation (2026-07-23, mid slice-3):** large piles of aliasing are a code smell; turn the observation into an epr-meta governance signal that stack-ranks instances so alias *clusters* can be inspected — instead of each reconciliation arc rediscovering them by hand.

## Evidence from the reach arc (why this earns a signal)

The 5-way reach-vocabulary drift was not five independent mistakes — it was **unmeasured aliasing accumulating past the point of inspection**. The slice-3 scouts found, in one afternoon: six same-named-but-unrelated `ReachLevel` symbols across four workspaces; a hand-mirrored vocabulary (`cache/types.ts` re-declaring the SDK's `LocalityLevel` rather than re-exporting it); alias chains three re-exports deep (`SDK → app protocol-core → barrel → lamad`); and homonym traps (`ContentReach` defined independently in elohim-library AND elohim-app with different value sets). Every one survived because nothing counted them.

## The classifier (drift-prevention law made inspectable)

The spec's drift-prevention law — exactly ONE generative source-of-record per vocabulary; every other appearance is a generated projection or an explicit re-export — gives the signal its taxonomy. Every alias instance classifies as:

- **(a) Declared projection** — a re-export or codegen output that names its source-of-record. Healthy. Signal value: link it (cite/`derived_from`), don't rank it.
- **(b) Deprecation bridge** — a time-boxed compatibility alias carrying a `@deprecated` tag and a sentinel fingerprint with an owning burn-down arc. Healthy *while owned*; ranks UP as it ages past its arc.
- **(c) Unmanaged mirror/homonym** — a hand-copied vocabulary, a same-named independent symbol, or a re-export chain with no declared source. **The smell.** Ranks by cluster size × consumer count × cross-workspace span.

Stack-rank output: clusters (grouped by symbol-family + value-set fingerprint), class (c) first, then aging (b), with per-cluster consumer counts — the queue a reconciliation arc plans against, replacing the ad-hoc scout sweeps this arc needed.

## Mechanism sketch (compose from the sentinel pattern — do not invent new machinery)

Follow the flag→agent→canon→stasis pattern already live for deprecations: (1) a **deterministic scanner** (AST-level: `export type X = Y` / `export const X = Y` re-declarations, duplicate string-literal-union value-sets, same-name symbols across tsconfig roots; Rust: duplicate enum value-sets) emitting fingerprinted instances to a ledger; (2) **clustering + stack-rank** as a `placement-audit`-style report line and an epr-meta surface (directory-local manifests can declare their vocabulary's source-of-record, making class-(a) machine-checkable); (3) **agent dispatch only on new class-(c) clusters**, canonicalize-then-fix, suppression on triaged. This is also a natural early instance of the eprfs sense-respond loop pointed at the codebase itself: the repo is already the deterministic floor of reach-earned attestation; alias density becomes one of its witnessed signals.

## Disposition

NOT in slice 3 (vocabulary burn-down only). Candidate follow-on arc after the reach reconciliation completes — the reach clusters it would have ranked are being drained by slices 1–3, which makes the reach family the natural fixture/regression corpus for the scanner's first run.
