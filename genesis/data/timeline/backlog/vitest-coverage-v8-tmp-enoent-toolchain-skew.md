---
id: "backlog-vitest-coverage-v8-tmp-enoent-toolchain-skew"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "vitest dual-major skew → v8 coverage `.tmp` ENOENT fails app Unit Test (workaround landed; realign toolchain to one major)"
slug: "vitest-coverage-v8-tmp-enoent-toolchain-skew"
written: "2026-06-29"
author: "pipeline-shakeout shift"
status: "wip"
priority: "medium"
ci_status: in-progress
jobs: [elohim]
tags: [ci, vitest, coverage, pnpm, dual-major, version-skew, lockfile, workaround, elohim-app]
cites:
  - app/elohim-app/vite.config.ts
  - app/elohim-app/vitest.global-setup.ts
  - elohim/sdk/epr-ts/package.json
  - elohim/sdk/storage-client-ts/package.json
  - elohim/elohim-agent/elohim-agent-sdk/package.json
---

# vitest dual-major skew → v8 coverage `.tmp` ENOENT

## Symptom (app pipeline `elohim`, build #1571, 2026-06-29)

`Unit Test` stage (`cd app/elohim-app && pnpm exec vitest run --config vite.config.ts --coverage`)
exits 1 though **every test passes**. The failure is an unhandled rejection from the
v8 coverage provider:

```
⎯ Unhandled Rejection ⎯
Error: ENOENT: no such file or directory,
  open '…/app/elohim-app/coverage/vitest/.tmp/coverage-0.json'
```

Reproduced locally (deterministic; `--no-file-parallelism` does NOT help → not a race).

## Root cause

The workspace carries **two vitest majors**:

- vitest **4.0.18** — `app/elohim-app`, `app/lamad`, `app/elohim-library` (+ `elohim-service`), `doorway/doorway-app`, `genesis/seeder`.
- vitest **^3.0.0 → 3.2.4** — `elohim/sdk/epr-ts`, `elohim/sdk/storage-client-ts`, `elohim/elohim-agent/elohim-agent-sdk`. (Plus `elohim/sdk` on `vitest ^1.0.0`.)

pnpm hoists `@vitest/runner@3.2.4` to the **workspace root** (`node_modules/@vitest/runner` → 3.2.4
while `vitest` core is 4.0.18). The 4.0.18 v8 coverage provider's per-fork temp-dir setup
hook does not fire against that skewed runner, so `coverage/vitest/.tmp/` is never created and
the first per-fork coverage write throws ENOENT.

**Trigger / timing:** latent until 2026-06-28's `pnpm-lock` regen (`d5b16758a` / `0cd103b06`,
the @elohim/storage-client peerDep fix) re-hoisted `@vitest/runner` to 3.2.4. It only became
*visible* after the app pipeline's two upstream FAILUREs (install-deps lockfile, then Build
Lamad Bundle WASM) were fixed, letting the run reach `Unit Test` for the first time in a while —
classic "green = deferred, not passed" debt surfacing as each upstream layer is cleared.

## Workarounds attempted and REJECTED (this shift — do not retry)

The `.tmp`-creation angle is a **dead end** — the skew breaks the provider's lifecycle
deterministically, not the dir's existence. Tried, all ENOENT (8 of 9 runs failed; the lone
early pass was stale-`.tmp` luck and did NOT reproduce):

- pre-`mkdir coverage/vitest/.tmp` (shell) + `coverage.clean=false` — CLI flag AND config form.
- `globalSetup` `.ts` — mangled by the @analogjs Angular vite plugin (transforms every `.ts`
  via tsconfig.spec.json/src-only), stripping the default export → "invalid globalSetup file".
- `globalSetup` `.mjs` — loaded fine, ran, still ENOENT.
- a `configResolved` vite plugin doing the mkdir (earliest hook, cwd-correct) — still ENOENT.

The provider re-empties/needs `.tmp` at a point the pre-create cannot survive under the skewed
runner. **vite.config.ts was reverted to original** — no shim is checked in.

## Reliable fix — OPERATOR DECISION (3 options, escalated 2026-06-29)

App is **tests-green** (every run's log: all test files ✓); only coverage-report generation is
broken. All three reliable fixes touch the lockfile (which just caused this cascade) or CI
coverage policy — hence escalated:

1. **istanbul provider** *(recommended — keeps coverage, contained)*: add
   `@vitest/coverage-istanbul@^4.0.18` devDep to elohim-app + `provider: 'istanbul'`. Istanbul
   instruments at transform time and does NOT use the v8 `.tmp/coverage-N.json` dump, so the
   runner skew shouldn't reach it. Cost: one additive devDep + `pnpm install` (lockfile) + local
   verify. Keeps Sonar coverage data.
2. **Drop `--coverage` in the CI Unit Test stage** *(fastest, reversible)*: tests still gate;
   coverage report is temporarily lost (no coverage threshold gates today, but SonarQube coverage
   metrics would read 0 — confirm Sonar doesn't hard-fail the build first). 1-line Jenkinsfile
   change, no lockfile touch.
3. **Realign the vitest toolchain to one major** *(root fix, highest risk)*: bump
   `elohim/sdk/epr-ts`, `elohim/sdk/storage-client-ts`, `elohim/elohim-agent/elohim-agent-sdk`
   (and `elohim/sdk` ^1) to `vitest ^4.0.18`, regen lockfile, verify those packages' own tests on
   vitest 4. Removes the dual major permanently but risks the SDK consumers + another lockfile regen.

## Confirms by disappearance

Once fixed: `pnpm exec vitest run --config vite.config.ts --coverage` (CI-exact, fresh) exits 0
with a report, and (for option 3) `@vitest/runner` resolves 4.0.18 at repo root.
