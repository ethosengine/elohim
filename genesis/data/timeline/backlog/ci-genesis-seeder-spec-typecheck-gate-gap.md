---
id: "backlog-ci-genesis-seeder-spec-typecheck-gate-gap"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Genesis Validate Constants fails — strict-TS error in seed-commitments.spec.ts; local `just gate` omitted the type-check CI runs"
slug: "ci-genesis-seeder-spec-typecheck-gate-gap"
written: "2026-07-31"
author: "ci-failure-triage"
status: "wip"
priority: "high"
ci_status: in-progress
fingerprints: [5801b320ae13, 946a9f514597]
jobs: [elohim-genesis]
relatedNodeIds: []
tags: [ci, elohim-genesis, typescript, ts2352, ts2493, validate-constants, seeder, prepush-gate, host-green-not-ci-green]
cites:
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1400/
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1401/
  - genesis/seeder/src/__tests__/seed-commitments.spec.ts
  - genesis/seeder/justfile
  - genesis/scripts/ci/typecheck-seeder.sh
  - .husky/pre-push.bash
  - genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md
  - genesis/data/timeline/backlog/ci-genesis-projectionspec-ts2739.md
---

# Genesis `Validate Constants` aborts — strict-TS error in a seeder SPEC file, invisible to the local gate

## The failure

`elohim-genesis/dev` builds **#1400 and #1401** (both `FAILURE`), stage
**Validate Constants**, running
`bash genesis/scripts/ci/typecheck-seeder.sh` (quoted verbatim from #1401 log,
lines 1222–1226):

```
+ bash /home/jenkins/agent/workspace/elohim-genesis_dev/genesis/scripts/ci/typecheck-seeder.sh
Type-checking seeder (compile-time enum enforcement)...
src/__tests__/seed-commitments.spec.ts(683,22): error TS2352: Conversion of type 'undefined' to type '{ variant: string; commonsBytes: number; bounds: { ratePerMinute: number; reachCeiling: string; }; ratioAttestation: CapacityRatioAttestation; }' may be a mistake because neither type sufficiently overlaps with the other. If this was intentional, convert the expression to 'unknown' first.
src/__tests__/seed-commitments.spec.ts(683,57): error TS2493: Tuple type '[]' of length '0' has no element at index '0'.
❌ TypeScript type errors found — fix before seeding
```

**Two fingerprints, ONE concern** — `5801b320ae13` (TS2352) and `946a9f514597`
(TS2493) are the two compiler diagnostics emitted for the *same expression* on
the *same line* (`683,22` and `683,57`), from the same root cause.

Occurrence evidence: both fps `seen: 2`, `first_build: 1400`,
`last_build: 1401`.

**Regime note (worth reading before the verdict):** genesis had been chronically
`UNSTABLE` for the preceding window (#1382–#1399 all UNSTABLE/ABORTED — the
degraded-substrate condition tracked in
`ci-alpha-cluster-degraded-substrate.md`). #1400/#1401 are the only `FAILURE`s:
this type-check gate `exit 1`s *before* seeding, so the whole downstream
catchError-wrapped E2E surface never runs. UNSTABLE→FAILURE is the state change
that marks this as a fresh regression, not more substrate noise.

## Verdict

**real — a strict-TypeScript regression in test code, not a flake and not
infra.** Deterministic (2/2 builds), compiler-diagnosed, and reproducible on the
host with the exact CI command. Not a museum trap #1 lossy-measure artifact:
these are genuine FAILUREs from a gate that hard-exits.

The *secondary* verdict is the more valuable one: **the local pre-push gate was
not a superset of the CI gate**, which is why the break reached `dev` at all.
That is museum-cluster #3/#5/#6 (host-green ≠ CI-green) wearing a new costume —
see "The recurrence" below.

## Root cause

Introduced by `30ba2413e` ("feat(capacity): consent-legibility ratios read +
seeder-default capacity pledge + custody-cancellation predicate tests"). The
spec declared its mock with a **zero-argument** async arrow:

```ts
const createCapacityPledge = vi.fn(async () => new Response('{}', { status: 200 }));
```

`vi.fn()` infers its call-tuple from the implementation's signature. A zero-arg
implementation gives `mock.calls: []` — a tuple of length 0. So at line 683,
`createCapacityPledge.mock.calls[0][0]`:

- `[0][0]` indexes past the end of a `[]` tuple → **TS2493**, and the element
  type resolves to `undefined`;
- the trailing `as { variant; commonsBytes; bounds; ratioAttestation }` is then
  a cast from `undefined` to an object type with no overlap → **TS2352**.

Both diagnostics, one defect. Note the test *passes at runtime* — the mock does
receive a body argument; only the static type of the call tuple is wrong.

## The recurrence — why it escaped the local gate

`vitest run` does **not** type-check (vite strips types), so `just gate`'s
`test` step is blind to a strict-TS error that lives only in a spec file.
CI's `typecheck-seeder.sh` runs `tsc --noEmit` over the **whole** seeder tree,
`src/` and `src/__tests__/` alike.

The pre-push hook *appeared* to cover this. Its `genesis)` fallback arm runs
`pnpm run typecheck` and carries a `DEFERRED:` note saying so. **That arm is
dead code.** `run_gate` branches:

```
elif command -v just >/dev/null 2>&1 && [ -f justfile ]; then   # → just gate
else                                                            # → case fallback
```

`just` is installed in the dev image (`/usr/local/bin/just`, 1.57.0) and
`genesis/seeder/justfile` exists, so the justfile branch **always** wins and the
fallback `case` is unreachable. The justfile's gate was
`gate: install validate test` — no type-check. The DEFERRED comment therefore
described coverage nobody was getting.

This is the **second** time the gap broke genesis at this exact stage:

| Build | Fingerprint | Error | Concern |
|---|---|---|---|
| #1101 (2026-06-06) | `0a93d2d79477` | TS2739 — `ProjectionSpec` literal missing two required fields | `ci-genesis-projectionspec-ts2739.md` |
| #1400–#1401 (2026-07-31) | `5801b320ae13`, `946a9f514597` | TS2352/TS2493 — zero-arg `vi.fn` tuple index | this entry |

Both are strict-TS errors in seeder **test** files. Two independent authors, two
months apart, same blind spot — the gate, not the authors, is the defect.

**Same-class siblings (not fixed here, deliberately — different blast radius):**
two other `DEFERRED:` fallback arms make claims their justfile gates do not
honour, and are equally unreachable:

- `elohim-app` — fallback runs `pnpm exec eslint src --ext .ts,.html`;
  `app/elohim-app/justfile` has `gate: lint-routes lint-a11y deps build test`
  (no `lint`). Full-tree eslint is a large surface: enabling it may abort pushes
  on pre-existing debt, so it needs its own bounded run.
- `orchestrator` — fallback runs `node --test graph-walker.test.mjs
  orchestrator-integration.test.mjs jenkinsfile-cps-scope.test.mjs`;
  `genesis/orchestrator/justfile` has `gate: lint test-jenkinsfile-lints`, and
  `test-jenkinsfile-lints` is explicitly scoped to `jenkinsfile-cps-scope.test.mjs`
  only. `graph-walker` + `orchestrator-integration` tests are uncovered locally.
  (Its DEFERRED comment's `(gate: lint)` parenthetical is also stale.)

## Current decision

**Fixed and locally verified; awaiting CI disappearance confirmation.** Ledger
entries stamped `status: triaged`, `triaged_at_build: 1401` — a later
`last_build > 1401` means the fix did not take. The next genesis build confirms
by passing `Validate Constants`; it may still go UNSTABLE downstream on the
degraded substrate (`ci-alpha-cluster-degraded-substrate.md`) — these two
concerns close on different signals, exactly as #1101 did.

**Not** marked `decompose_on_confirm`: the lesson is museum-worthy and has been
graduated (below), and the two same-class sibling gates above remain open work
this entry is the only record of.

## Fix trail

**1 — the immediate defect** (`00459a090`, "fix(seeder): type the capacity-pledge
mock arg — heal genesis typecheck red"), `genesis/seeder/src/__tests__/seed-commitments.spec.ts:673`:

```diff
-    const createCapacityPledge = vi.fn(async () => new Response('{}', { status: 200 }));
+    const createCapacityPledge = vi.fn(async (_body: unknown) => new Response('{}', { status: 200 }));
```

Typing the parameter gives the mock a 1-arg call tuple, so `mock.calls[0][0]` is
legally indexable and its type is `unknown` — a legal cast source. Runtime
behaviour unchanged.

**2 — the gate gap** (this triage run):

- `genesis/seeder/justfile` — `gate: install validate test` →
  `gate: install validate check test`, with a comment recording *why* `check`
  is load-bearing (vitest does not type-check; CI does) and both build numbers.
- `.husky/pre-push.bash` — the `genesis)` fallback arm's DEFERRED note replaced
  with a REACHABILITY WARNING recording that the whole `case` block is the
  `just not found` branch and therefore dead in this image, so no future step is
  parked there again. The residual real gap (`validate-human-ids.py`) is kept
  named.

**Local verification (this run, in-container):**

- `pnpm exec tsc --noEmit` in `genesis/seeder` → exit **0**, no `error TS` lines
  (the exact predicate `typecheck-seeder.sh` grep-tests).
- `pnpm exec vitest run src/__tests__/seed-commitments.spec.ts` → **44/44 pass**.
- `just gate` (with the new `check` step) → exit **0**, **410 passed / 9 skipped
  across 30 files**, type-check clean. The corrected local gate now reproduces
  the CI gate's verdict.

**3 — lesson graduated to the museum**: new row #13 ("A `DEFERRED` fallback arm
in the pre-push hook is DEAD CODE") in
`genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md`,
extending the existing record per the one-lessons-doc rule.
