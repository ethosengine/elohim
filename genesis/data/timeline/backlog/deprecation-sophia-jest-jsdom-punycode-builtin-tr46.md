---
id: "backlog-deprecation-sophia-jest-jsdom-punycode-builtin-tr46"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "sophia's jest 29 / jsdom 20 stack emits Node DEP0040 on every test run — tr46@3 bare-requires the builtin punycode (parent repo already clean at jsdom 28 / tr46 5)"
slug: "deprecation-sophia-jest-jsdom-punycode-builtin-tr46"
written: "2026-07-30"
author: "deprecation-triage"
status: "backlog"
priority: "low"
deprecation_status: blocked
severity: low
fingerprints: ["534a561884b4", "e0db70ae40c7", "9f2941b72728", "d4b8d4668bd1", "7b068a5072a3"]
relatedNodeIds: []
tags: [deprecation, node, punycode, DEP0040, jest, jsdom, tr46, whatwg-url, sophia, transitive, test-only]
cites:
  - https://nodejs.org/api/deprecations.html#DEP0040
  - https://github.com/jsdom/tr46
  - https://github.com/jestjs/jest/blob/main/CHANGELOG.md
  - sophia/package.json
  - sophia/pnpm-lock.yaml
  - genesis/data/timeline/backlog/security-jquery-2-1-1-shipped-in-sophia-umd-bundle.md
---

## What is deprecated

```
(node:810849) [DEP0040] DeprecationWarning: The `punycode` module is deprecated.
Please use a userland alternative instead.
```

Node's **built-in `punycode` module** (runtime-deprecated since Node 21, DEP0040).
It still functions on Node 24 but emits a process-level warning on first load.
The published replacement is the identical-source userland package
`punycode@2.x`, reached via the trailing-slash specifier `require("punycode/")`
— the bare specifier `require("punycode")` **always** resolves to the builtin in
CJS regardless of what sits in `node_modules`, which is exactly the trap here.

No security exposure and no runtime exposure: the emitter is reachable only from
sophia's **test environment** (jsdom under jest). Nothing in the shipped
`sophia-element` UMD bundle or any browser/Node production path loads it. The
cost is (a) stderr noise on every jest invocation and (b) a deprecation-sentinel
fingerprint storm — four identical fingerprints were captured within one hour on
2026-07-30 purely from repeated jest runs during unrelated work.

## Usage inventory

**Zero first-party usage.** No `.ts`/`.js`/`.tsx` file in the monorepo requires
`punycode` directly (verified by grep across `app/`, `sophia/packages/`,
`doorway/`, `genesis/`). The emitter is a single transitive file.

Emitter isolated by `node --trace-deprecation --stack-trace-limit=40`; the
innermost non-`node:` frame is unambiguous:

```
at Object.<anonymous> (sophia/node_modules/.pnpm/tr46@3.0.0/node_modules/tr46/index.js:3:18)
at Object.<anonymous> (…/whatwg-url@11.0.0/node_modules/whatwg-url/lib/url-state-machine.js:2:14)
at Object.<anonymous> (…/whatwg-url@11.0.0/node_modules/whatwg-url/lib/URL-impl.js:2:13)
at Object.<anonymous> (…/whatwg-url@11.0.0/node_modules/whatwg-url/lib/URL.js:442:14)
```

`tr46@3.0.0/index.js:3` is literally `const punycode = require("punycode");`.

Resolved chain in `sophia/pnpm-lock.yaml`:

```
sophia (devDependency)
└─ jest-environment-jsdom@29.7.0        (pnpm-lock.yaml:6438, 15445)
   └─ jsdom@20.0.3                      (pnpm-lock.yaml:6643, 15836)
      ├─ whatwg-url@11.0.0              (pnpm-lock.yaml:15861)
      │  └─ tr46@3.0.0  ← EMITTER       (pnpm-lock.yaml:8761, 18255)
      └─ data-urls@3.0.2
         └─ whatwg-url@11.0.0           (pnpm-lock.yaml:13604)  → same tr46@3.0.0
```

**Sole emitter in the jest path — proven, not assumed.** Node dedupes DEP0040 to
one emission per process, so a second requirer would be masked. Verified
empirically by patching the installed `tr46@3.0.0/index.js` to `require("punycode/")`
and re-running: the warning disappeared entirely from both captured commands
(see Verification). The other bare-`require("punycode")` carriers in the sophia
tree — `tr46@0.0.3` and `whatwg-url@5.0.0` (both reached only via `node-fetch@2`),
and `checksync` (bundled inside `@khanacademy/eslint-plugin`) — are **not on the
jest/jsdom load path** and did not fire once tr46@3 was patched. `tough-cookie@4.1.4`,
`uri-js@4.4.1`, `psl@1.15.0` and the `regenerate` family are **false positives**:
their only `require('punycode')` hits are inside a vendored `punycode/README.md`.

**The parent repo is already clean.** `/projects/elohim/pnpm-lock.yaml` resolves
`jsdom@28.1.0` with `tr46@5.1.1` and `tr46@6.0.0` — both use `require("punycode/")`.
The parent carries no `tr46@3.0.0` at all. This concern is **sophia-submodule-local**
and is a straggler of the same migration the parent already completed.

## Migration path

Two routes, tactical and structural.

**1. Tactical — pnpm override (recipe proven, see Verification).** `tr46@4.1.1`
switched the specifier to `require("punycode/")` and declares `punycode: ^2.3.0`
as a real dependency. Its `toASCII` / `toUnicode` option surface is
byte-for-byte compatible with the `tr46@3.0.0` surface `whatwg-url@11` calls
(`beStrict`, `checkBidi`, `checkHyphens`, `checkJoiners`, `processingOption`,
`useSTD3ASCIIRules` — all still accepted; `processingOption` was **not** removed
in v4, the same `transitional`/`nontransitional` RangeError guard is present at
`index.js:233`). Only the Unicode/IDNA tables and the engine floor
(`node >=14`) moved. In `sophia/package.json`:

```json
"pnpm": {
  "overrides": {
    "whatwg-url@11>tr46": "^4.1.1"
  }
}
```

then `pnpm install` to regenerate `sophia/pnpm-lock.yaml`. Note `tr46@5`/`@6`
raise the engine floor and are the versions the newer jsdom line pairs with —
`^4.1.1` is the minimal in-range-behaviour step for `whatwg-url@11`.

**2. Structural — jest 29 → 30 (the override deletes itself).** `jest-environment-jsdom@30`
moves to `jsdom@^26`, which brings `whatwg-url@14` → `tr46@5`, dropping the
builtin require without any override. This is the route the parent repo
effectively took (vitest + jsdom 28). It is a **major bump of sophia's test
runner across a large React monorepo** — well past this agent's bounded-fix
ceiling — and belongs to an operator-initiated sprint, not a background run.

Overriding `punycode` itself is **not** a viable route: the bare specifier
resolves to the builtin no matter what version sits in `node_modules`. The
specifier is the bug, not the version.

## Current decision

**Blocked — write-set contention on the exact file the fix must regenerate.**

The tactical override is proven-correct and behaviour-neutral, but landing it
requires editing `sophia/package.json` **and** regenerating `sophia/pnpm-lock.yaml`.
At triage time the sophia submodule is checked out on branch **`feat/jquery-3`**
with `pnpm-lock.yaml` and `pnpm-workspace.yaml` dirty — a concurrent session is
mid-flight on the jQuery 2.1.1 → 3.7.1 upgrade, the highest-value item in the
security backlog (`security-jquery-2-1-1-shipped-in-sophia-umd-bundle.md`,
8 fingerprints, browser-reachable XSS). Regenerating the lockfile now would
clobber that in-flight work. Adding the `overrides` key *without* regenerating is
strictly worse: it desynchronizes the lockfile and breaks the next
`pnpm install --frozen-lockfile` in CI.

The blocker is transient and the next step is one command, so this is recorded
as a live trajectory rather than a deferred investigation:

> **Next step (unblocks the moment `feat/jquery-3` lands):** add the
> `whatwg-url@11>tr46: ^4.1.1` override to `sophia/package.json`, run
> `pnpm install` in `sophia/`, and verify per the Verification section below.
> Then delete the four fingerprints from `.claude/data/deprecations.jsonl` and
> delete this entry.

Severity/priority are `low`: test-only, no runtime or security reach, and the
structural fix (jest 30) retires it for free. The material cost being paid today
is sentinel noise — four identical fingerprints in one hour — which the `blocked`
ledger status now suppresses deterministically.

Ledger fingerprints `534a561884b4`, `e0db70ae40c7`, `9f2941b72728`, `d4b8d4668bd1`
set to `blocked` citing this entry. All four are the *same* warning re-emitted
from different jest invocations; they map N:1 onto this one concern.

## Verification

**Recipe proven, fix not yet landed.** Verified 2026-07-30 by temporarily
rewriting the installed `sophia/node_modules/.pnpm/tr46@3.0.0/node_modules/tr46/index.js`
line 3 from `require("punycode")` to `require("punycode/")` (node_modules only —
no tracked file touched; reverted immediately after):

| Command | Unpatched | Patched (tr46 → `punycode/`) |
|---|---|---|
| `jest packages/sophia/src/interactive2/movable.test.ts` | DEP0040 present · 38 passed, EXIT=0 | **DEP0040 gone** · 38 passed, EXIT=0 |
| `jest packages/sophia/src/widgets/passage/__tests__/passage.test.tsx` | DEP0040 present · 2 failed, 11 passed | **DEP0040 gone** · 2 failed, 11 passed |

Two conclusions: the warning is fully attributable to `tr46@3` (no second
emitter was hiding behind Node's per-process dedupe), and the swap is
behaviour-neutral. The 2 `passage.test.tsx` failures are **identical patched and
unpatched** — pre-existing, belonging to the in-flight jQuery 2→3 work on
`feat/jquery-3`, not caused by this change.

**Closing verification when the fix lands:** after `pnpm install` with the
override, `grep -c 'tr46@3.0.0' sophia/pnpm-lock.yaml` must return 0, and
`pnpm exec jest packages/sophia/src/interactive2/movable.test.ts 2>&1 | grep -c DEP0040`
must return 0 with the suite still green — then decompose (delete the four
ledger lines and this entry).
