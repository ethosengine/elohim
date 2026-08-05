---
id: "backlog-angular-native-vitest-builder-adoption"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Adopt @angular/build:unit-test (vitest runner) — retire the Analog shim + the hand-carried @angular/build patch"
slug: "angular-native-vitest-builder-adoption"
written: "2026-07-30"
author: "angular22-campaign"
status: "backlog"
priority: "medium"
tags: [angular, vitest, analog, patch-retirement, test-infra]
cites:
  - patches/@angular__build@22.1.0.patch
  - doorway/doorway-app/angular.json
  - pnpm-workspace.yaml
---

## What

Angular 22's `@angular/build:unit-test` builder (EXPERIMENTAL at 22.1) with `runner: "vitest"`
can replace `@analogjs/vitest-angular:test` — and with it, the pnpm patch
`patches/@angular__build@22.1.0.patch` that must be re-cut on every Angular major.

## Spiked 2026-07-30 (doorway-app, report-only, clean revert)

- Ran 7 files / 32 tests at full green parity with the Analog baseline; comparable speed.
- **The patch's defect is structurally moot on the native path**: the builder compiles specs
  with the full AOT compiler by default (`aot: true` flows from the build target;
  `JitCompilation`/`jit-resource-transformer.js` — the patched file — is never entered).
  Confirmed empirically: external `templateUrl`/`styleUrl` resolved, patched file untouched.
- `setupFiles` honored (matchMedia stub worked) but not auto-added to the TS program —
  needs `tsconfig.spec.json` include. Zone + TestBed init are auto-injected by the builder;
  the hand-rolled init in `test-setup.ts` must be trimmed or it double-inits.
- Path aliases resolve from tsconfig `paths` directly — no vite-tsconfig-paths plugin needed.
- Gaps: relative-path `vi.mock()` throws (package-path mocks fine; no current doorway spec
  uses relative); `runnerConfig` overrides `test.projects`/`test.include`; fine-grained
  vite.config.ts settings (pool/forks) need `runnerConfig` to reproduce.

## Why not adopted during the v22 campaign (decided)

Adopting one app retires nothing (the patch stays while ANY app uses the Analog JIT path),
and it would put an experimental builder in the CI gate path. The larger suites
(elohim-app, lamad, elohim-library) need their own validation for relative `vi.mock` and
custom vitest config usage first.

## Adoption checklist (when the builder drops EXPERIMENTAL)

1. Audit elohim-app/lamad/elohim-library specs for relative-path `vi.mock` + vite.config.ts
   features without a `runnerConfig` equivalent.
2. Per app: wire `unit-test` target (runner vitest), trim `test-setup.ts` to stubs only,
   add setup file to `tsconfig.spec.json` include, retarget npm `test*` scripts from
   `vitest run --config vite.config.ts` to `ng test` (Jenkinsfile + pre-push gate clauses
   reference these scripts — update `build-manifest.json` gate wiring in the same pass).
3. When ALL apps are migrated: drop `@analogjs/vite-plugin-angular` + `@analogjs/vitest-angular`
   from every manifest and DELETE the `patchedDependencies` entry + patch file — ending the
   re-cut-every-major maintenance burden.
4. Re-check at each Angular 22.x minor whether `unit-test` is still experimental.
