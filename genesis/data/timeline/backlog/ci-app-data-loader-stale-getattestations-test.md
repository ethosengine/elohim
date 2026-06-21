---
id: "backlog-ci-app-data-loader-stale-getattestations-test"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "elohim-app unit test asserts retired DataLoaderService.getAttestations method (Phase-2a cleanup left a stale test)"
slug: "ci-app-data-loader-stale-getattestations-test"
written: "2026-06-21"
author: "ci-failure-triage"
status: "wip"
priority: "high"
ci_status: in-progress
fingerprints: [e6177d581f7b]
jobs: [elohim]
relatedNodeIds: []
tags: [ci, elohim-app, vitest, attestation, phase-2a, stale-test, dead-code-cleanup]
cites:
  - https://jenkins.ethosengine.com/job/elohim/job/dev/1546/
  - https://jenkins.ethosengine.com/job/elohim/job/dev/1545/
  - app/elohim-app/src/app/elohim/services/data-loader.service.spec.ts
  - app/elohim-app/src/app/elohim/services/data-loader.service.ts
  - genesis/data/timeline/backlog/phase2a-attestation-cleanup-remaining-surface.md
---

# elohim-app unit test asserts a retired method (stale getAttestations existence test)

## The failure

`elohim` (the elohim-app pipeline, multibranch `elohim/dev`) builds **#1545 and
#1546**, **Unit Test** stage (Vitest). Occurrence evidence: seen 2,
first_build 1545, last_build 1546.

```
 FAIL  src/app/elohim/services/data-loader.service.spec.ts > DataLoaderService > should have getAttestations method
AssertionError: expected undefined to be defined
  ❯ src/app/elohim/services/data-loader.service.spec.ts:177:37

176|   it('should have getAttestations method', () => {
177|     expect(service.getAttestations).toBeDefined();
   |                                    ^
178|     expect(typeof service.getAttestations).toBe('function');
```

The single failing assertion across the whole Vitest run — build #1546 summary:
`Test Files 1 failed | 218 passed (219)` / `Tests 1 failed | 4596 passed (4597)`.
The Unit Test stage is the stage of record that turned the build red; every
downstream stage (SonarQube → Upload SPA Blob → Build Image → Deploy → E2E) was
"skipped due to earlier failure(s)" and the pipeline ended **FAILURE**.

(An earlier ORAS wasm-cache-core pull miss appears in the #1546 log —
`elohim-wasm-cache-core:1.0.0-dev-3ec9c300: not found` — but the pipeline
continued past it into Unit Test, so it is not the FAILURE-of-record. That
wasm-cache SHA-pin behavior is its own canonicalized concern:
`ci-app-wasm-cache-core-sha-pin-blocks-nondna-deploys`.)

## Verdict — real (stale existence test), bounded

Not a flake, not infra. The identical assertion (same file, same test name, same
line 177, same column 37) appears byte-for-byte in BOTH #1545 and #1546 —
deterministic, cross-build-consistent. It is host-reproducible: the test
asserts the existence of a `getAttestations` method that no longer exists on
`DataLoaderService` at test time, so the binding evaluates to `undefined`.

Museum gate: none of the 11 CI/orchestrator traps apply. This is an app-layer
Vitest existence test that outlived the method it guarded — narration (a forgotten
test left behind by a code-retirement commit), not a structural recurring trap.
No new museum lesson.

## Root cause

The attestation-consolidation **Phase-2a frontend cleanup** retired the
content-quality attestation reads (`getAttestations` / `getAttestationsForContent`
/ `getActiveAttestations`) from the elohim-app `DataLoaderService` — that twin had
no rendering consumer (the elohim-app `TrustBadgeService` twin was dead; the LIVE
trust-badge read lives in the **lamad** `DataLoaderService`, repointed onto the
unified attestation surface via `AttestationApiService.listBySubject()`). The
retirement commit (`e9a920b5d`, in build #1546's ancestry) removed the method and
left an explanatory comment block at `data-loader.service.ts:932-942`, but did
**not** remove the corresponding `should have getAttestations method` existence
test in the spec. The test then asserted `toBeDefined()` on a now-absent method.

This is the standard dead-code-cleanup tail: the implementation was retired in one
commit, the guarding test in the next. Sibling-in-spirit to
`ci-app-routes-count-canary-stale-debug-shell` (a test/structure-narration drift
that the test correctly caught), and downstream of the Phase-2a cleanup surface
tracked in `phase2a-attestation-cleanup-remaining-surface.md`.

## Current decision

**Bounded fix ALREADY LANDED on the branch (commit `e66ce1685`), verified
locally — awaiting CI disappearance-confirmation.** Build #1546 was built from
`3ec9c300` (HEAD~1), which carried the method retirement but NOT the stale-test
removal; `e66ce1685` is the single commit between #1546's SHA and current HEAD and
is exactly the test deletion. The ledger stamp (`status: triaged`,
`triaged_at_build: 1546`, `decompose_on_confirm: true`) lets the harvester confirm
by green streak (elohim job ≥3 green, no recurrence of fp `e6177d581f7b`) once a
build > 1546 runs, then auto-clean ledger line + this backlog entry — a forgotten
existence test carries no museum-worthy lesson. Re-trigger rides the integrator's
next feat→dev / orchestrator push.

## Fix trail

- Fix commit (already on the branch): `e66ce1685` — *"test(elohim-app): remove
  stale getAttestations test (method retired in attestation cleanup) [build:app]"*.
  Removed the `should have getAttestations method` test from
  `app/elohim-app/src/app/elohim/services/data-loader.service.spec.ts`.
- Upstream cause: `e9a920b5d` — *"refactor(attestation): repoint trust-badge read
  onto unified attestation surface + retire legacy content-attestation code
  (Phase-2a frontend cleanup)"* retired `DataLoaderService.getAttestations`.
- Local verification (against HEAD = `e66ce1685`):
  `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts
  src/app/elohim/services/data-loader.service.spec.ts` →
  **31/31 passed, 1 file passed** (no failing assertion; the stale existence test
  is gone, every other DataLoaderService test green).
- No integrator action needed for this concern beyond the next `elohim` pipeline
  run, which confirms by green streak.
