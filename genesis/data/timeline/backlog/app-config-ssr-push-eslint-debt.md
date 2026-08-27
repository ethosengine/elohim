---
id: "backlog-app-config-ssr-push-eslint-debt"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "app.config.ts (+ SSR+P2P-push files) carry accrued eslint debt from the --no-verify be757cb0b integration — App lint gate red"
slug: "app-config-ssr-push-eslint-debt"
written: "2026-06-27"
author: "overnight doorway-deploy + genesis fan-out shift (2026-06-27T03)"
status: "backlog"
priority: "medium"
jobs: [elohim]
---

## The debt

`app/elohim-app/src/app/app.config.ts` carries ~45 eslint errors that PREDATE this
shift — introduced by the SSR+P2P integration push `be757cb0b` (landed `--no-verify`,
so the local husky app gate never ran and CI is the only backstop). Linting the file
directly surfaces three classes:

1. **`import/order`** — the many new service imports (economic-events-api,
   custodian-commitment, helia-fetch, epr-resolver, governance-signal, …) were added
   without group ordering. Mostly `--fix`-able.
2. **`import/no-extraneous-dependencies` 'lamad'** — `@app/lamad/*` imports resolve via
   tsconfig path alias, not an `elohim-app` package.json dependency, so the rule flags
   every `@app/lamad/*` import. Pre-existing (the file imported `@app/lamad/*` before
   this shift); needs a config/dep decision, NOT auto-fixable.
3. **`unicorn/prefer-global-this`** at `resolveDoorwayUrl` (`typeof window !== 'undefined'
   && window.location.hostname`). This `window` guard is **load-bearing for SSR** —
   `globalThis.location` is undefined in the Node SSR render, so the rule's auto-fix
   would CRASH SSR. The correct resolution is an `eslint-disable-next-line` (the rule is
   a false-positive here), NOT the auto-fix. Do not let `eslint --fix` touch it.

This is the gate-debt class predicted by memory `feedback_pvc_deferral_hides_gate_debt`
("elohim-app gate never defers → --no-verify is the only frontend-touching dev-push
path"). The App pipeline goes UNSTABLE on lint but still DEPLOYS (deploy stage runs on
UNSTABLE), so it has not blocked delivery — but it masks new frontend lint regressions.

## Proposed fix

A dedicated lint-debt cleanup pass on the SSR+P2P-push frontend files (not just
app.config.ts — audit the other `be757cb0b`-touched `app/elohim-app/src/**` files for
the same): (a) `eslint --fix` for `import/order` + `prettier`; (b) decide the
`@app/lamad` dependency story (add to package.json deps OR scope the rule off for path
aliases in the flat config); (c) add a scoped `eslint-disable-next-line
unicorn/prefer-global-this` at the SSR `window` guards with a `// SSR: window guard is
intentional` comment. Verify the App lint stage returns to green.

## Evidence / refs

- This shift's t1 fix (`a185da8f8`) added only the `LEARNER_BACKEND` provider + 2 imports
  (clean); the surrounding ~45 errors are pre-existing and were NOT introduced by it.
- Shift journal: `.claude/shifts/2026-06-27T03-overnight-doorway-deploy-genesis-fanout.journal.md` (iter-4).
- Memory: `feedback_pvc_deferral_hides_gate_debt`.

**Measured 2026-08-27** (`just gate` on dev at `00b21f834`): `pnpm run lint` in app/elohim-app reports **607 errors / 190 warnings across 156 files** (top: `data-loader.service.ts` 64, `insurance-mutual.service.ts` 43, `app.config.ts` 42). The root app pipeline does not run `pnpm lint`, so CI is NOT a backstop for this class — the pre-push hook is the only gate, and it now fails on every push that touches `app/elohim-app`, which is why integration pushes keep landing `--no-verify`. The `@workspace/runtime` fence (2026-08-27) added zero new errors (`import/internal-regex` + a pathGroup). Ratchet candidate: an eslint baseline file like `lint-workspace-imports` uses, so NEW errors block while the tail drains.
