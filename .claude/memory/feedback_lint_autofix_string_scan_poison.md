---
id: feedback-lint-autofix-string-scan-poison
name: lint-autofix-string-scan-poison
title: Lint autofix string-scan poison
description: "eslint --fix rewrites runtime-critical code (prefer-set-has on strings; prefer-global-this on typeof-window SSR guards; promise-function-async retiming; Array<T> in generated files) — a green suite is NOT enough, run the AOT build too."
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 4dc49cfc-da6d-46ba-83cb-79c5c0613b7a
cites:
  - app/elohim-elements/elohim-core
---

During the 2026-06-05 theme-gate lint pass, `eslint --fix` rewrote `const after = cssText.slice(i)` + `after.includes('ButtonFace')` into `new Set(cssText.slice(i))` + `.has('ButtonFace')` (unicorn/prefer-set-has assumes an array; a Set of a string is a set of CHARACTERS, so `.has('ButtonFace')` is always false). 8 ua-prefs forced-colors gates in elohim-core went green→red purely from the "safe" fix pass.

**Why:** Autofixes are only mechanically safe for formatting rules (Prettier). Type-blind rules like prefer-set-has change runtime semantics; on test files full of string scans this class silently inverts assertions in the *false-negative-prone* direction (assertion still runs, always passes/fails wrong).

**2026-08-29, elohim-app 590→0 pass — the same class, three more rules, and one the tests could not see.** `unicorn/prefer-global-this` rewrote `typeof window !== 'undefined'` into `globalThis.window !== undefined` in BOTH of the app's SSR guards; TypeScript types that as statically always-true, and in login.component.ts it also folded a defensive clause into `?.` while leaving four comments still describing "the preceding typeof check" that no longer existed. `@typescript-eslint/promise-function-async` added `async` to two promise-returning functions, inserting a microtask tick — one broke a spec (`whenStable()` resolved before the value landed), the other silently retimed `main.server.ts`, the SSR entry that has its own lint rail and NO test coverage. `eslint --fix` also rewrote six `/generated/` files whose header says DO NOT EDIT, and would have stripped codegen's own disable banner from 99 more — an oscillation, since the next codegen run writes it back.

**The sharpened rule:** the dangerous autofixes are the ones on rules that are *warnings*, in files with no test. A style warning must never be able to retime or re-guard an SSR entry point. Both rules are now `off` in `app/elohim-app/eslint.config.js` with the reasoning at the site; generated dirs are scoped there too.

**And the tests are not the whole gate.** In the same pass, hand-extracting constants produced six self-referential `const EVENTS_URL = EVENTS_URL;` (the literal replacement rewrote the declaration it had just inserted) and hoisted an import into the middle of a doc comment, splitting a sentence. ESLint reported ZERO errors and the unit suite passed; only `ng build` (AOT) caught them.

**How to apply:** After ANY `eslint --fix` (especially mass passes by lint-fixer/quality-sweep), run the full test suite before committing — a green suite is the only proof the fix pass was behavior-neutral. Where a string scan must stay a string, leave `// eslint-disable-next-line unicorn/prefer-set-has -- string scan, not membership lookup` (pattern now in elohim-core spec files). Then run the AOT build (`just gate <project>`, or `ng build` directly) — the suite proves behaviour, the build proves the code is even well-formed. Before a mass `--fix`, check what it would touch: `eslint -f json` and group by `ruleId` where `m.fix` exists; anything under a `generated/` path or in an SSR/service-worker entry gets reverted and the rule scoped instead. Related: [[concurrent-sessions-shared-worktree]], [[project_elohim_app_local_build_verification_gaps]].
