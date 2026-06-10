---
title: rea-runtime specs silently report "No test suite found" under the elohim-library root vitest config
created: 2026-06-10
domain: process-meta (build-and-test; silent test gap)
source: sdk-core plan Task 4 (commit 44c06a3d7) — stash/baseline-verified pre-existing
severity: medium
---

`app/elohim-library/vite.config.ts` `test.include` lists rea-runtime specs, but
the root `tsconfig.spec.json` never included `projects/elohim-rea-runtime/**`,
so the Analog Angular plugin doesn't transform them — all 4 rea-runtime spec
files report "No test suite found" under the root-config run path (before AND
after the /core pre-pave; identity passes 70/70 under the same root config).
The per-project `projects/elohim-rea-runtime/vite.config.ts` is the working
path (69/69 green, matches the @analogjs/vitest-angular:test builder). Risk:
any CI/gate invoking the ROOT config believes rea-runtime is tested when zero
suites execute. Fix: add the include to root tsconfig.spec.json (and audit the
other projects' inclusion while there), or remove rea-runtime from the root
include so the gap is at least loud. Matters more once arc Phase 4 lands
CommitmentService + its tests into this library.
