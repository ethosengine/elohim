---
name: lint-autofix-string-scan-poison
description: Mass eslint --fix can apply behavior-changing autofixes (unicorn/prefer-set-has on STRINGS) that silently break string-scan assertions — always run the full test suite after any --fix pass
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 4dc49cfc-da6d-46ba-83cb-79c5c0613b7a
---

During the 2026-06-05 theme-gate lint pass, `eslint --fix` rewrote `const after = cssText.slice(i)` + `after.includes('ButtonFace')` into `new Set(cssText.slice(i))` + `.has('ButtonFace')` (unicorn/prefer-set-has assumes an array; a Set of a string is a set of CHARACTERS, so `.has('ButtonFace')` is always false). 8 ua-prefs forced-colors gates in elohim-core went green→red purely from the "safe" fix pass.

**Why:** Autofixes are only mechanically safe for formatting rules (Prettier). Type-blind rules like prefer-set-has change runtime semantics; on test files full of string scans this class silently inverts assertions in the *false-negative-prone* direction (assertion still runs, always passes/fails wrong).

**How to apply:** After ANY `eslint --fix` (especially mass passes by lint-fixer/quality-sweep), run the full test suite before committing — a green suite is the only proof the fix pass was behavior-neutral. Where a string scan must stay a string, leave `// eslint-disable-next-line unicorn/prefer-set-has -- string scan, not membership lookup` (pattern now in elohim-core spec files). Related: [[concurrent-sessions-shared-worktree]].
