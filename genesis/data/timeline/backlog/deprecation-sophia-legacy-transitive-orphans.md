---
id: "backlog-deprecation-sophia-legacy-transitive-orphans"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Five deep-transitive orphans in sophia (urix, resolve-url, source-map-url, source-map-resolve, stable) — no first-party lever; they clear only when cypress-jest-adapter, sloc, and cssnano's svgo advance"
slug: "deprecation-sophia-legacy-transitive-orphans"
written: "2026-07-30"
author: "deprecation-triage"
status: "backlog"
priority: "low"
deprecation_status: blocked
severity: low
fingerprints: ["ce0de21b8053"]
relatedNodeIds:
  - "backlog-deprecation-sophia-rollup-filesize-npm-internals-subtree"
tags: [deprecation, sophia, transitive, orphan, snapdragon, micromatch, cypress-jest-adapter, sloc, svgo, cssnano, rollup-plugin-postcss, abandoned-upstream]
cites:
  - https://github.com/lydell/urix#deprecated
  - https://github.com/lydell/resolve-url#deprecated
  - https://github.com/lydell/source-map-resolve#deprecated
  - https://github.com/lydell/source-map-url#deprecated
  - https://www.npmjs.com/package/stable
  - sophia/package.json
  - sophia/config/cypress/support.ts
---

## What is deprecated

Five of the 25 packages in the sophia install banner (fingerprint
`ce0de21b8053`) are **deep-transitive orphans**: author-retired micro-packages
buried three to six hops below a first-party declaration, with no direct
declaration, no first-party import, and no override that would be safe.

```
urix@0.1.0               Please see https://github.com/lydell/urix#deprecated
resolve-url@0.2.1        https://github.com/lydell/resolve-url#deprecated
source-map-url@0.4.1     See https://github.com/lydell/source-map-resolve#deprecated
source-map-resolve@0.5.3 See https://github.com/lydell/source-map-resolve#deprecated
stable@0.1.8             'Modern JS already guarantees Array#sort() is a stable sort, so this
                         library is deprecated. …'
```

The four `lydell/*` packages are one cluster — the author deprecated the whole
family at once when the ecosystem moved to `@jridgewell/*`. `stable@0.1.8` is the
same shape: the language absorbed the capability (stable `Array#sort` since ES2019).

**Severity note.** These are the *least* consequential lines in the banner. All
five are dev-time-only, none is a vulnerability, and each is deprecated because
the platform grew the feature — not because the code is unsafe. They are recorded
so the banner is fully explained and never re-triaged, not because they warrant
work.

## Usage inventory

Reverse-dep trace over `sophia/pnpm-lock.yaml` `snapshots:`. Two independent
carrier trees.

### Tree 1 — the `snapdragon` cluster (four packages)

```
source-map-resolve@0.5.3            ← DEPRECATED (sole parent: snapdragon@0.8.2)
├── resolve-url@0.2.1               ← DEPRECATED (sole parent: source-map-resolve)
├── source-map-url@0.4.1            ← DEPRECATED (sole parent: source-map-resolve)
└── urix@0.1.0                      ← DEPRECATED (sole parent: source-map-resolve)

snapdragon@0.8.2 ← braces@2.3.2, expand-brackets@2.1.4, extglob@2.0.4,
                   micromatch@3.1.10, nanomatch@1.2.13   (all one micromatch@3 cluster)
micromatch@3.1.10 ← jest-message-util@24.9.0, readdirp@2.2.1
```

Two root importers reach it, both ancient dev-only declarations in
`sophia/package.json`:

| Root importer | Path |
|---|---|
| `cypress-jest-adapter@0.1.1` | → `expect@24.9.0` → `jest-message-util@24.9.0` → `micromatch@3.1.10` → `snapdragon@0.8.2` |
| `sloc@0.2.1` | → `readdirp@2.2.1` → `micromatch@3.1.10` → `snapdragon@0.8.2` |

`cypress-jest-adapter` drags an entire **jest 24 island** (`expect@24.9.0` and
friends) into a workspace whose real jest is 29 — it exists to expose Jest's
`expect` API inside Cypress component tests. It is **live**, not dead:
`config/cypress/support.ts:2` does `import "cypress-jest-adapter";`. Upstream is
abandoned at `0.1.1`.

`sloc` is live too, behind the `sloc` npm script (`sloc packages --exclude
node_modules`) — a line-counting utility.

### Tree 2 — `stable` under the CSS pipeline (one package)

```
stable@0.1.8                        ← DEPRECATED (sole parent: svgo@2.8.0)
└── svgo@2.8.0 ← postcss-svgo@5.1.0 ← cssnano-preset-default@5.2.14
                ← cssnano@5.1.15 ← rollup-plugin-postcss@4.0.2   (root importer)
```

`rollup-plugin-postcss@4.0.2` is live in all three rollup configs
(`config/build/rollup.config.js:16`, `packages/sophia-element/rollup.config.mjs:18`,
`packages/psephos-element/rollup.config.mjs:15`) — it is the CSS pipeline for the
UMD bundles, not removable.

## Migration path

There is **no first-party lever** for any of the five. Nothing declares them,
nothing imports them, and each is pinned by an intermediate package's own
manifest. They clear only when a carrier advances:

| Carrier | What would clear it | Realistic? |
|---|---|---|
| `cypress-jest-adapter@0.1.1` | Upstream is abandoned; the fix is to **drop it** and use Cypress's built-in `expect` (Chai) or `@testing-library/cypress` in the component tests | Plausible, but requires rewriting assertion style across the Cypress component suite — its own sprint |
| `sloc@0.2.1` | Drop it (a line-count script is not load-bearing) or replace with `cloc`/`tokei` | Cheap, but clears nothing on its own — `cypress-jest-adapter` also carries `snapdragon`; **both** must go |
| `rollup-plugin-postcss@4.0.2` → `cssnano@5` → `svgo@2` | `cssnano@7` uses `svgo@3`, which dropped `stable` | Rides a `rollup-plugin-postcss`/`cssnano` major bump |

**Do not reach for a pnpm override here.** Overriding `source-map-resolve` or
`stable` to a nonexistent "fixed" version is not possible (there are no
non-deprecated releases — the packages were retired, not patched), and overriding
`micromatch@3` → `4` inside `jest-message-util@24` would break a package that
declares the v3 API.

The honest sequencing: **tree 1 clears only if `cypress-jest-adapter` and `sloc`
are both removed** — a partial removal leaves `snapdragon` in place through the
other carrier. That mutual dependency is the single most useful fact in this
entry.

## Current decision

**Blocked — no first-party lever, and the only real path (dropping
`cypress-jest-adapter`) is an assertion-style rewrite across the Cypress component
suite that exceeds the bounded-fix envelope.**

Deliberately recorded as a low-priority standing explanation rather than queued
work. The value of this entry is that five banner lines are *explained* and never
re-triaged — not that anyone should act on them.

Explicitly **not** the blocker (recorded so a future pass does not re-derive):
artifact availability. The repo switched public-package resolution to
`registry.npmjs.org` in commit `ecc65384f` (2026-07-30); uncached tarballs probed
200 this pass. The "Nexus mirror serves cached artifacts only" constraint recorded
across sibling entries does not apply.

The ledger fingerprint stays present so the sentinel cites this decision
deterministically and never re-dispatches; the stasis sweep owns the re-check.

### Live trajectory

Two independent, opportunistic ride-alongs — neither worth scheduling on its own:

1. **When the Cypress component suite is next touched substantively**, evaluate
   dropping `cypress-jest-adapter` (abandoned upstream at 0.1.1, drags a jest-24
   island into a jest-29 workspace — that alone is a maintenance argument
   independent of the deprecations). Remove `sloc` in the same commit, or tree 1
   does not clear.
2. **When `rollup-plugin-postcss`/`cssnano` are next bumped**, confirm
   `stable@0.1.8` leaves the lockfile as `svgo` advances to 3.x.

Do not delete this entry until all five are absent from `sophia/pnpm-lock.yaml`.
If only tree 2 clears, narrow this entry to the four `lydell` packages rather than
deleting it.

## Verification

No fix landed this pass; scoping evidence only.

- **Sole-parent chains**, from a parent-edge index over `sophia/pnpm-lock.yaml`
  `snapshots:` (peer-suffix normalised): `resolve-url@0.2.1`,
  `source-map-url@0.4.1`, and `urix@0.1.0` each have exactly one parent tree-wide
  (`source-map-resolve@0.5.3`), which itself has exactly one
  (`snapdragon@0.8.2`). `stable@0.1.8` has exactly one parent tree-wide
  (`svgo@2.8.0`).
- **Two-carrier finding for tree 1:** `micromatch@3.1.10` has exactly two parents
  (`jest-message-util@24.9.0`, `readdirp@2.2.1`), tracing to two distinct root
  importers (`cypress-jest-adapter@0.1.1`, `sloc@0.2.1`). This is why a
  single-package removal does not clear the cluster.
- **Carriers are live, not dead declarations:** `cypress-jest-adapter` imported at
  `config/cypress/support.ts:2`; `sloc` invoked by the `sloc` npm script;
  `rollup-plugin-postcss` imported in all three rollup configs (line numbers
  above). Independently corroborated — `knip --dependencies` lists none of the
  three as unused, while it does flag the two packages in the dead-declarations
  sibling entry.
- **No first-party declaration or import** of any of the five packages anywhere in
  the submodule.

Closure requires all five absent from `sophia/pnpm-lock.yaml`; partial clearance
narrows this entry rather than closing it.
