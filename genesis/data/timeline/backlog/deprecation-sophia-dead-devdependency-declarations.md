---
id: "backlog-deprecation-sophia-dead-devdependency-declarations"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Three dead devDependency declarations in sophia (intersection-observer, typescript-coverage-report, rollup-plugin-preserve-shebangs) — knip-confirmed; one commit clears popper.js@1, sourcemap-codec, and a whole React-16 island"
slug: "deprecation-sophia-dead-devdependency-declarations"
written: "2026-07-30"
author: "deprecation-triage"
status: "backlog"
priority: "high"
deprecation_status: open
severity: low
fingerprints: ["50aa3734f6b0", "010ff5a7bfb5", "ce0de21b8053"]
relatedNodeIds:
  - "backlog-deprecation-sophia-rollup-filesize-npm-internals-subtree"
  - "backlog-deprecation-sophia-legacy-transitive-orphans"
  - "backlog-security-jquery-2-1-1-shipped-in-sophia-umd-bundle"
tags: [deprecation, sophia, dead-declaration, knip, intersection-observer, typescript-coverage-report, rollup-plugin-preserve-shebangs, popper.js, sourcemap-codec, react-16, bounded-fix, submodule]
cites:
  - https://knip.dev/
  - https://www.npmjs.com/package/intersection-observer
  - https://developer.mozilla.org/en-US/docs/Web/API/IntersectionObserver
  - sophia/package.json
  - sophia/packages/sophia/package.json
  - sophia/knip.config.ts
  - sophia/utils/test-with-coverage.sh
  - genesis/data/timeline/backlog/security-jquery-2-1-1-shipped-in-sophia-umd-bundle.md
---

## What is deprecated

Three **dead manifest declarations** in the sophia submodule — packages that are
declared but never imported, `require`d, or referenced by any script or config.
They are not broken dependencies; they are dependencies that do no work, and each
drags deprecated packages into the install banner for nothing.

Verbatim lockfile `deprecated:` fields for what they carry:

```
intersection-observer@0.12.2  The Intersection Observer polyfill is no longer needed and can
                              safely be removed. Intersection Observer has been Baseline since 2019.
popper.js@1.16.1              You can find the new Popper v2 at @popperjs/core, this package is
                              dedicated to the legacy v1
sourcemap-codec@1.4.8         Please use @jridgewell/sourcemap-codec instead
```

This entry was originally scoped to `intersection-observer` alone (fingerprints
`50aa3734f6b0` / `010ff5a7bfb5`). Triage of the aggregate sophia install banner
`ce0de21b8053` ("25 deprecated subdependencies found") found **two more
declarations of exactly the same shape** in `sophia/package.json` — same fix, same
verification, same blocker — so they are canonicalized here as one concern rather
than forked into duplicates. One commit closes all three.

## Usage inventory

### 1. `intersection-observer@^0.12.0` — `sophia/packages/sophia/package.json`

| Line | Section | Value |
|---|---|---|
| 87 | `devDependencies` (starts line 59) | `"intersection-observer": "^0.12.0"` |
| 126 | `peerDependencies` (starts line 98) | `"intersection-observer": "^0.12.0"` |

Not in `dependencies` (lines 42–58) — never a runtime dependency of the published
package, only a dev install plus a peer request on consumers. Full-tree grep for
both the package name and the `IntersectionObserver` identifier returns exactly
those two hits; zero imports, zero global usages.

### 2. `typescript-coverage-report@^0.7.0` — `sophia/package.json:132`

**The highest-value line of the three.** Declared once, referenced nowhere.
Repo-wide grep for `typescript-coverage-report` (all file types, excluding
`node_modules` / `pnpm-lock.yaml`) returns **one** hit: the declaration itself.

Critically, sophia's coverage pipeline does **not** use it.
`utils/test-with-coverage.sh` — the body of the `coverage` script — runs Jest with
`--coverage`, Cypress with `CYPRESS_COVERAGE=1`, then merges via `nyc merge` /
`nyc report`. `nyc` is the live coverage tool; `typescript-coverage-report` is an
unrelated *type*-coverage HTML generator that nothing invokes.

What it drags in — a complete **React 16 island inside a React 18 repo**:

```
typescript-coverage-report@0.7.0        (sophia/package.json:132, sole parent tree-wide)
├── react@16.14.0
├── react-dom@16.14.0
├── rimraf@3.0.2                        ← deprecated (shared carrier; see rollup-filesize entry)
├── type-coverage-core@2.29.7 · ncp@2.0.0 · cli-table3@0.6.5
└── semantic-ui-react@0.88.2            (an entire UI framework; sole parent: this package)
    └── react-popper@1.3.11             (sole parent: semantic-ui-react@0.88.2)
        └── popper.js@1.16.1            ← DEPRECATED, sole parent tree-wide
```

**Blast-radius note that matters:** sophia *does* use `react-popper` first-party —
`packages/sophia/src/widgets/label-image/answer-pill.tsx:6` imports `{Popper}` —
but that resolves against the **`^2.2.5`** declarations in `sophia/package.json:113`
and `packages/sophia/package.json:95,132`, which depend on `@popperjs/core@2`
(pinned in the catalog), **not** on `popper.js@1`. The `react-popper@1.3.11`
resolution is a wholly separate branch reachable only through `semantic-ui-react`.
Removing `typescript-coverage-report` therefore cannot affect first-party Popper
usage.

### 3. `rollup-plugin-preserve-shebangs@^0.2.0` — `sophia/package.json:123`

Declared once, imported nowhere. Sophia has three rollup configs
(`config/build/rollup.config.js`, `packages/sophia-element/rollup.config.mjs`,
`packages/psephos-element/rollup.config.mjs`); grep for `preserve-shebangs` /
`preserveShebangs` across all `.js/.mjs/.ts/.cjs` returns **zero** import sites.
The root config's plugin imports are `@rollup/plugin-alias`, `-commonjs`,
`-node-resolve`, `-replace`, `-swc`, `ancesdir`, `postcss-import`, `postcss-url`,
`rollup-plugin-auto-external`, `rollup-plugin-filesize`, `rollup-plugin-postcss` —
no shebang plugin. (Shebang preservation for the CLI bins is handled by
`rollup-plugin-executable-output`, which is separately declared and live.)

Chain:

```
rollup-plugin-preserve-shebangs@0.2.0   (sophia/package.json:123, sole parent tree-wide)
└── magic-string@0.25.9                 (sole parent: this package)
    └── sourcemap-codec@1.4.8           ← DEPRECATED, sole parent tree-wide
```

### Independent confirmation — the repo's own detector agrees

Sophia ships `knip` (`sophia/knip.config.ts`, `pnpm knip`). Run this pass,
`knip --dependencies` independently flags all three under **Unused
devDependencies**:

```
rollup-plugin-preserve-shebangs    package.json:123:6
typescript-coverage-report         package.json:132:6
intersection-observer              packages/sophia/package.json:87:10
```

**One knip caveat, checked rather than assumed:** `knip.config.ts` excludes
`!utils/**` from its project scan, so anything used *only* by a helper in `utils/`
reads as unused. That is exactly why knip also lists `nyc` — which is **live**
(driven by `utils/test-with-coverage.sh` and the `nyc` key in `package.json` for
Cypress coverage). Each of the three above was therefore cross-checked with a
repo-wide grep *including* `utils/` and all shell scripts, and all three come back
genuinely unreferenced. Do not treat knip's list as directly actionable without
that cross-check.

## Migration path

No migration — these are **deletions**, not replacements. Nothing needs to adopt a
successor API because nothing was using the deprecated one.

Exact patch:

1. `sophia/packages/sophia/package.json` — delete line 87 (`devDependencies`) and
   line 126 (`peerDependencies`), both `"intersection-observer": "^0.12.0"`.
2. `sophia/package.json` — delete line 132 (`"typescript-coverage-report": "^0.7.0"`).
3. `sophia/package.json` — delete line 123 (`"rollup-plugin-preserve-shebangs": "^0.2.0"`).
4. `cd sophia && pnpm install` to refresh `pnpm-lock.yaml`.

If a future sophia surface needs viewport observation, use the native
`IntersectionObserver` — no polyfill. If type-coverage reporting is ever actually
wanted, wire it into `utils/test-with-coverage.sh` deliberately and re-declare it
then; a declaration nothing invokes is not a capability.

## Current decision

**Fix is bounded, exact, knip-confirmed, and ready — deliberately NOT landed this
pass, because a jQuery-3 migration sprint is running live in the same shared
worktree and owns `sophia/pnpm-lock.yaml` right now.**

This is not the indefinite "submodule freeze" this entry recorded at first triage
(that one cleared). It is a specific, short-lived, *observed* race:

- `sophia` is on branch `feat/jquery-3` with uncommitted changes to
  `pnpm-workspace.yaml` (catalog `jquery` `2.1.1` → `3.7.1`), `pnpm-lock.yaml`
  (6 insertions / 12 deletions, all jQuery), and
  `packages/sophia/src/jquery.mobile.vmouse.js`.
- `sophia/pnpm-lock.yaml`'s mtime is **14:58:03Z** — byte-identical to the
  timestamp of ledger fingerprint `ce0de21b8053`, the entry that dispatched this
  triage. The `pnpm install` that emitted the 25-package banner **is** that
  sprint's install. It ran minutes before this pass.

Two reasons not to proceed anyway, both hard rules rather than caution:

1. **The lockfile cannot be staged selectively.** Applying step 4 regenerates
   `sophia/pnpm-lock.yaml` on top of the sprint's uncommitted jQuery edits.
   Committing the result would sweep their jQuery lockfile hunks into a
   deprecation commit *without* the `pnpm-workspace.yaml` catalog change that
   justifies them — producing a lockfile that references a catalog spec which is
   not committed, i.e. a broken `--frozen-lockfile` for CI, the pre-push gates,
   and every other session sharing this worktree.
2. **`pnpm install` rewrites `node_modules` under a live test run.** The sprint is
   actively verifying 22 jQuery import sites against the installed tree.

So this holds `deprecation_status: open` — fix specified, no technical unknown —
rather than `blocked`. Ledger fingerprints stay present with `status: triaged`.

### Correction to the recorded blocker — the npm mirror is no longer in the way

Sibling entries across this backlog record dependency work as blocked because
"the Nexus npm mirror serves cached artifacts only; uncached tarballs 404."
**That constraint no longer applies to this repo**, and the reason is worth
stating precisely so it is not mis-generalised:

Nexus was not repaired. The repo **stopped using it for public packages.**
Commit `ecc65384f` *"fix(registry): reserve Nexus for first-party components;
consume crates.io + npmjs direct"* (2026-07-30 03:18Z, in HEAD) sets
`/projects/elohim/.npmrc` to `registry=https://registry.npmjs.org/`, scoping
Nexus to `@elohim:` first-party publishing only. Sophia has no `.npmrc` of its own
and inherits that ancestor config. Probed this pass against the effective
registry: uncached `rollup-plugin-filesize-10.0.0.tgz` → **200**, control
`left-pad-1.2.0.tgz` → **200**.

Consequence: artifact availability is **not** a blocker for any sophia dependency
work, and the several sibling entries still citing mirror 404s should have that
blocker **re-tested rather than inherited**. Deliberately not edited here — those
are other concerns and each re-test is its own trajectory. Flagged for the stasis
sweep.

### Live trajectory — one commit, unblocking on a named event

When the `feat/jquery-3` sprint commits and `git -C sophia status --porcelain` is
clean:

1. Re-run the zero-usage greps for all three packages (cheap; the sprint touches
   `packages/sophia/package.json`, so re-confirm rather than assume). If that
   sprint already removed any of the lines, drop it from the patch.
2. Apply the 4-step patch, then run sophia's gates: `pnpm install`, `pnpm build`,
   `pnpm test`, `pnpm typecheck`, `pnpm lint`.
3. Confirm the banner arithmetic: `popper.js@1.16.1`, `sourcemap-codec@1.4.8`, and
   `intersection-observer@0.12.2` absent from `sophia/pnpm-lock.yaml`; the install
   banner drops from **25** deprecated subdependencies to **23**.
4. On green, **close out with full decomposition**: delete ledger lines
   `50aa3734f6b0` and `010ff5a7bfb5`, delete this entry, and quote the
   verification in the commit message. Do **not** delete `ce0de21b8053` — that
   fingerprint is the shared aggregate banner, owned jointly with the sibling
   entries listed in `relatedNodeIds`, and clears only when the banner does.

There is no lesson here worth graduating to a chronicle. Three dead declarations
removed is exactly the case that should decompose to nothing but a commit.

## Verification

No fix landed this pass; the following is scoping evidence, not fix verification.

- **Zero-usage, full-tree, per package.** `intersection-observer`: grep for the
  package name *and* the `IntersectionObserver` identifier across the submodule
  (excluding `node_modules`, `.git`, `pnpm-lock.yaml`), all file types → 2 hits,
  both the declaration. `typescript-coverage-report`: repo-wide grep over
  `*.json`/`*.sh`/`*.yml`/`*.yaml` plus all sources → 1 hit, the declaration.
  `rollup-plugin-preserve-shebangs`: grep for both the package name and
  `preserveShebangs` over `*.js/*.mjs/*.ts/*.cjs` → 0 import sites; the three
  rollup configs' import lists were read directly.
- **knip agreement.** `pnpm knip --dependencies` (exit 0) lists all three under
  *Unused devDependencies*. The `!utils/**` false-positive class was identified
  via its `nyc` entry and cross-checked out for all three.
- **Sole-parent reverse-dep proof**, from a parent-edge index over
  `sophia/pnpm-lock.yaml`'s `snapshots:` section (peer-suffix normalised):
  `popper.js@1.16.1` ← `react-popper@1.3.11` ← `semantic-ui-react@0.88.2` ←
  `typescript-coverage-report@0.7.0` — one parent at every hop, terminating at a
  root importer. `sourcemap-codec@1.4.8` ← `magic-string@0.25.9` ←
  `rollup-plugin-preserve-shebangs@0.2.0` — likewise.
- **First-party Popper is a different resolution.** `react-popper` is declared
  `^2.2.5` at `sophia/package.json:113` and `packages/sophia/package.json:95,132`;
  the sole first-party import site is
  `packages/sophia/src/widgets/label-image/answer-pill.tsx:6`. Neither reaches
  `popper.js@1`.
- **Live coverage tool is `nyc`, not `typescript-coverage-report`** — read from
  `utils/test-with-coverage.sh` (`nyc merge` / `nyc report`).
- **Contention, observed:** `git -C sophia status --porcelain` non-empty on
  `feat/jquery-3`; `sophia/pnpm-lock.yaml` mtime `2026-07-30 14:58:03Z` equals the
  dispatching fingerprint's timestamp.
- **Registry probe:** uncached `rollup-plugin-filesize-10.0.0.tgz` → 200, control
  `left-pad-1.2.0.tgz` → 200 against the effective registry
  (`registry.npmjs.org`, set by `.npmrc` in commit `ecc65384f`).

Closure requires: all three declarations deleted, the three packages absent from
`sophia/pnpm-lock.yaml`, the banner at 23, and sophia's gates green — then this
entry and ledger lines `50aa3734f6b0` / `010ff5a7bfb5` are deleted, not parked.
