---
id: "backlog-elohim-app-lint-debt-gate-scope"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "elohim-app lint gate carries 593 pre-existing errors on dev — any wave that touches the app tree inherits a red gate it didn't cause"
slug: "elohim-app-lint-debt-gate-scope"
written: "2026-08-22"
author: "orchestrator (wave-4 landing)"
status: "open"
priority: "medium"
tags: [frontend, lint, gate-debt, pvc-deferral-class, bounded-code-fix]
---

# elohim-app lint debt: 593 errors, pre-existing on dev

Measured 2026-08-22 during the wave-4 push gate: `just gate app/elohim-app`
fails with **783 problems (593 errors, 190 warnings)** — SSR-unsafe globals in
`main.ts`, `import/no-extraneous-dependencies` in `test-setup.ts`,
promise-function-async across services, etc. Byte-identity check: the wave's
entire `app/elohim-app/src` delta vs `origin/dev` is four additive GENERATED
codegen files — every linted error lives in files identical to dev. This is
the documented PVC-deferral class (dev "green" = deferred, not passed): the
lint gate had not actually run on this tree in some time, and any push whose
changeset touches `app/elohim-app` (even scripts or codegen) drags the whole
red gate into scope.

162 errors auto-fixable per eslint. Beware the two known traps before a bulk
fix: `--fix` on prefer-set-has string-scans inverts assertions (run the full
suite after), and the codegen Prettier oscillation (18 generated files flip
formatting every run — exclude generated dirs from any hand-fix pass).

## Done when

`just gate app/elohim-app` passes from a clean tree, generated dirs excluded
or conformant, and the fix commit is verified by the full vitest suite (not
just lint).
