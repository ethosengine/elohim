---
id: "backlog-security-jquery-2-1-1-shipped-in-sophia-umd-bundle"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "jQuery 2.1.1 ships to browsers in the sophia-element UMD bundle (4 XSS/prototype-pollution advisories)"
slug: "security-jquery-2-1-1-shipped-in-sophia-umd-bundle"
written: "2026-07-30"
author: "deprecation-triage"
status: "backlog"
priority: "high"
deprecation_status: blocked
severity: security
fingerprints: ["011f5406331d", "313c6eac27c1", "93e83acd4b96", "d5f606fc5fa4", "4ee2a842e119", "01b4b9157783", "9d31ba938515"]
relatedNodeIds: []
tags: [deprecation, security, jquery, sophia, perseus, umd-bundle, xss, prototype-pollution, submodule]
cites:
  - https://github.com/advisories/GHSA-gxr4-xjj5-5px2
  - https://github.com/advisories/GHSA-jpcq-cgw6-v4j6
  - https://github.com/advisories/GHSA-6c3j-c64m-qhgq
  - https://github.com/advisories/GHSA-rmxg-73gg-4p98
  - https://jquery.com/upgrade-guide/3.0/
  - https://www.herodevs.com/support/jquery-nes
  - sophia/pnpm-workspace.yaml
  - sophia/packages/sophia-element/rollup.config.mjs
  - app/elohim-app/scripts/check-sophia.sh
  - app/elohim-library/projects/perseus-plugin/package.json
---

## What is deprecated

`pnpm install` in the sophia submodule warns that the resolved jQuery is an
end-of-life 2.x release:

```
packages/kmath                           |  WARN  deprecated jquery@2.1.1     (fp 011f5406331d)
```

and the lockfile carries upstream's own notice (self-captured while scoping —
fp `313c6eac27c1`):

```
jquery@2.1.1:
  deprecated: This version is deprecated. Please upgrade to the latest version
              or find support at https://www.herodevs.com/support/jquery-nes.
```

jQuery 2.1.1 was released in **2014**. An OSV query for `npm:jquery@2.1.1`
returns **four** advisories, all rated MODERATE (each self-captured as its own
fingerprint while scoping):

| Advisory | Alias | Class | Fixed in | fp |
|---|---|---|---|---|
| GHSA-gxr4-xjj5-5px2 | CVE-2020-11022 | XSS via DOM manipulation with untrusted HTML | **3.5.0** | `d5f606fc5fa4` |
| GHSA-jpcq-cgw6-v4j6 | CVE-2020-11023 | XSS via DOM manipulation with untrusted HTML | **3.5.0** | `4ee2a842e119` |
| GHSA-6c3j-c64m-qhgq | CVE-2019-11358 | Prototype pollution via `$.extend` | 3.4.0 (also 2.1.9 / 2.2.2) | `93e83acd4b96` |
| GHSA-rmxg-73gg-4p98 | CVE-2015-9251 | XSS via cross-domain AJAX response | 3.0.0 | `01b4b9157783` |

**No 2.x release clears all four** — CVE-2020-11022/11023 are first fixed in
**3.5.0**, so 3.5.0 is the minimum clearing floor. (Only CVE-2019-11358 has a
2.x fix, at 2.1.9/2.2.2.)

### Why this is `severity: security`, not routine deprecation

The deprecated copy is **executed in learners' browsers**, verified empirically
rather than inferred. The shipped asset
`app/elohim-app/src/assets/sophia-plugin/sophia-element.umd.js` inlines jQuery,
and the bundled version resolves to 2.1.1 — jQuery's own `version` variable is
minified to `p` and assigned literally:

```js
var r=[],n=r.slice,…,u={},d=e.document,p="2.1.1",h=function(e,t){…}
                                       ^^^^^^^^^
```

with the matching `jquery:p` version property on the prototype. Grepping the
same bundle for `"3.7.1"` returns **zero** hits. (The nearby `"2.2.4"` string is
unrelated — it is `@khanacademy/pure-markdown`'s `libVersion`.)

The chain that puts it there:

1. `sophia/packages/sophia-element/rollup.config.mjs` sets `external: []` with
   the explicit comment *"Don't externalize anything - bundle everything for
   UMD"* — so every import in the graph is inlined, jQuery included.
2. `umd-entry.ts` → `./register` → `./index` reaches
   `packages/sophia/src/renderer.tsx`, which does `import $ from "jquery"`.
3. jQuery is declared only as a **devDependency + peerDependency** (never a
   runtime `dependencies` entry), so rollup resolves it from the dev-installed
   copy — which the catalog pins to 2.x.
4. `app/elohim-app/scripts/check-sophia.sh` copies
   `packages/sophia-element/dist/sophia-element.umd.js` into
   `app/elohim-app/src/assets/sophia-plugin/`, and the app serves it.

**Reachability of the XSS pair is plausible but NOT established.** CVE-2020-11022/11023
require untrusted HTML reaching jQuery DOM-manipulation methods
(`.html()`, `.append()`, …). Sophia's Perseus-derived render path does pass
content-authored markup through jQuery (`renderer.tsx`, `util/graphie.ts`,
`widgets/passage`, `widgets/iframe`), so the sinks exist — but no
attacker-controlled-input-to-sink trace has been proven. Treat the upgrade as
warranted on advisory-count and EOL grounds; do **not** cite this entry as proof
of an exploitable XSS. Establishing or refuting reachability is the probe named
under Verification.

### Version drift between the two bundling paths

The repo has **two** independent bundlers that inline jQuery, at **different
versions**:

| Path | jQuery declared | Resolves to |
|---|---|---|
| `sophia/packages/sophia-element` (UMD, submodule) | catalog dev `^2.1.1` / peer `2.1.1` | **2.1.1** (vulnerable) |
| `app/elohim-library/projects/perseus-plugin` (parent repo) | `"jquery": "^3.7.1"` | 3.7.1 (clears all four) |

The parent repo already standardised on 3.7.1. The submodule's exact peer pin
`2.1.1` is not even satisfied by 3.7.1, so the two paths cannot currently agree.
This is useful evidence for the migration: **sophia's own sources are already
expected to work against jQuery 3.7.1** by the perseus-plugin path, which
materially de-risks the catalog bump.

## Usage inventory

**Catalog (single source of both pins)** — `sophia/pnpm-workspace.yaml`:

- line 75 — `devDeps` catalog: `jquery: ^2.1.1`
- line 114 — `peerDeps` catalog: `jquery: 2.1.1`  ← exact pin

**Declaring manifests** (all reference the catalog; all dev + peer, none runtime):

- `sophia/packages/sophia/package.json:88` (devDeps), `:127` (peerDeps)
- `sophia/packages/sophia-editor/package.json:75` (devDeps), `:107` (peerDeps)
- `sophia/packages/kmath/package.json:32` (devDeps), `:37` (peerDeps)
- `sophia/packages/math-input/package.json:59` (devDeps), `:79` (peerDeps)

**Source import sites** — `import $ from "jquery"`, **22 sites across 22 files**:

| Package | Files |
|---|---|
| `packages/sophia` | 19 — incl. `src/renderer.tsx`, `src/util/graphie.ts`, `src/util/tex.ts`, `src/util/interactive.ts`, `src/interactive2/movable.ts`, `src/interactive2/wrapped-drawing.ts`, `src/components/{graphie,sortable,math-input,text-list-editor}.tsx`, `src/widgets/{iframe,passage,orderer,measurer,plotter,cs-program}/*.tsx`, `src/jquery.mobile.vmouse.js` (vendored shim), plus tests |
| `packages/sophia-editor` | 2 — incl. `src/editor.tsx` |
| `packages/kmath` | 1 |

**Lockfile** — `sophia/pnpm-lock.yaml`: `jquery@2.1.1` (5 importers) and
`jquery@3.7.1` (only for `jest-jquery-matchers`, test-only). One transitive
declares a permissive `jquery: '>=2.0.0'`.

**Shipped artifact** — `app/elohim-app/src/assets/sophia-plugin/sophia-element.umd.js`
(3.6 MB, untracked build output) contains jQuery 2.1.1 inline.

## Migration path

Target **jQuery 3.7.1** — clears all four advisories (floor is 3.5.0) and
matches what `perseus-plugin` already resolves, collapsing the version drift.

1. In `sophia/pnpm-workspace.yaml`, set both catalog entries to `^3.7.1`
   (devDeps line 75, peerDeps line 114). The four declaring manifests need no
   edit — they all read the catalog.
2. Work the **jQuery 3 upgrade guide** across the 22 import sites. The removals
   and semantic changes that actually bite a Perseus-derived tree:
   - removed: `.andSelf()`, `.size()`, `.context`, `.selector`, `$.browser`
   - `.data()` key casing changed (hyphen → camelCase normalisation)
   - Deferred objects became Promises/A+ compliant — `.then()` error propagation
     and exception semantics changed (this is the highest-risk class here, given
     `interactive2/` and `util/interactive.ts`)
   - `:visible` / `:hidden` redefined; `.width()`/`.height()` now return
     fractional values
   - `$(html)` no longer executes scripts in some paths; `.load()`/`.unload()`/
     `.error()` shorthands gone
3. Special attention to `packages/sophia/src/jquery.mobile.vmouse.js` — a
   vendored jQuery Mobile vmouse shim written against jQuery 2 event internals.
   It is the single most likely hard break.
4. Re-run sophia's own suite (`pnpm test`) plus `pnpm build && pnpm build:umd`,
   then verify the rebuilt UMD: the `p="2.1.1"` marker must become `3.7.1`.
5. Re-render the assessment surfaces with eyes (`pnpm look`, and the Sophia
   widget paths in a2o) — jQuery 3 breakage in Perseus widgets is typically
   visual/interactive, not a test failure.

A cheaper **partial** exists and is worth naming so nobody mistakes it for the
fix: bumping only to `2.2.4` would clear CVE-2019-11358 alone and leave both XSS
advisories and CVE-2015-9251 live. Not worth a release.

## Current decision

**Blocked — exceeds the bounded-fix envelope on two independent counts, and the
target tree is frozen by concurrent work.**

1. **Scale.** This is a **major version bump (2.x → 3.x)** across **22 import
   sites in 22 files**, including a vendored jQuery-2-era event shim and the
   Deferred→Promise semantic change through the interactive widget layer. Both
   the "dependency major version" and the ">20 files" stop conditions in the
   deprecation-triage envelope are tripped. This needs an operator-initiated
   sprint against the jQuery 3 upgrade guide with eyes-on widget verification,
   not a background agent.
2. **Worktree contention (transient).** Every file needing edit lives inside the
   `sophia` git submodule, which at triage time had **uncommitted changes on
   branch `feat/node24`** (`package.json`, `packages/sophia/package.json`,
   `pnpm-workspace.yaml` — a concurrent Node 24 + dependency-security upgrade,
   itself mid-flight editing the very catalog file this fix must touch). Editing
   sophia now risks clobbering unlanded work. **No sophia file was modified by
   this triage pass.**

The ledger fingerprints stay present with `status: blocked` so the sentinel
cites this decision deterministically and never re-dispatches; the
deprecation-stasis sweep owns the re-check.

**Live trajectory — ordered, and the first step is cheap.** The concurrent
`feat/node24` agent is *already editing the catalog file* and is *already doing
dependency-security remediation* (its uncommitted diff adds an `overrides:`
block for `qs`/`flatted`/`picomatch`/`minimatch` and bumps `vite` 5.4.11→5.4.21).
The catalog one-liner in step 1 is a natural, near-zero-cost addition to that
work — **but only if paired with the step-2/4/5 verification**, because an
unverified catalog bump would silently swap the jQuery major under 22 call sites.
So:

- **Next step (small):** once `feat/node24` lands, re-check whether that agent's
  `overrides:` work already moved jQuery. If not, decide explicitly whether to
  fold the catalog bump into a follow-up commit *with* the suite + UMD-marker +
  eyes verification, or to schedule the sprint.
- **Then (sprint-sized):** the 22-site jQuery 3 migration per Migration path.
- **Escape hatch if the sprint cannot be scheduled:** because the vulnerable
  copy arrives *only* via the bundler, the exposure could in principle be cut by
  externalising jQuery in `sophia-element`'s rollup config and having the host
  app supply 3.7.1 (which `perseus-plugin` already has). This is **not**
  recommended blind — it converts a bundling concern into a runtime-peer
  concern, and sophia's sources would then face jQuery 3 semantics *without* the
  step-2 audit. Recorded as an option, not a plan.

## Verification

What this pass proved (no fix landed, so this is scoping evidence, not
fix verification):

- **Shipped-bundle version, empirical:** the committed-into-assets
  `sophia-element.umd.js` contains jQuery's own `version` assignment
  `p="2.1.1"` with the matching `jquery:p` property; `grep "3.7.1"` → **0**
  hits. jQuery 2.1.1 is what executes in browsers.
- **Bundling mechanism:** `rollup.config.mjs:109` — `external: []`, commented
  *"Don't externalize anything - bundle everything for UMD"*.
- **Advisory set:** OSV `POST /v1/query` for `npm:jquery@2.1.1` → 4 vulns
  (GHSA-gxr4-xjj5-5px2, GHSA-jpcq-cgw6-v4j6, GHSA-6c3j-c64m-qhgq,
  GHSA-rmxg-73gg-4p98), all MODERATE; earliest version clearing all four =
  3.5.0.
- **Blast radius:** 22 `import $ from "jquery"` sites in 22 files across 3
  packages; both pins originate from 2 catalog lines in `pnpm-workspace.yaml`.
- **Contention:** `git -C sophia status --porcelain` showed 3 modified files on
  `feat/node24` at triage time.

Outstanding probes (owed before the sprint is scoped, and the one honest gap in
the security framing above):

- **Reachability trace** for CVE-2020-11022/11023: does any
  content-authored-or-learner-supplied HTML reach a jQuery DOM-manipulation sink
  in `renderer.tsx` / `widgets/passage` / `widgets/iframe`? A proven trace
  raises priority to urgent; a proven refutation makes this a hygiene upgrade
  that can wait for the sprint.
- **Published-package check:** `check-sophia.sh` can source the UMD from Nexus
  (`npm pack @ethosengine/sophia-element`) or a GitHub Release instead of the
  local build. Confirm the published artifacts also carry 2.1.1, so the fix is
  known to require a republish and not just a local rebuild.

Closure requires: catalog at `^3.7.1`, sophia `pnpm test` green, rebuilt UMD
showing the 3.7.1 marker, and eyes-verified Sophia widget rendering.
