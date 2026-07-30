---
id: "backlog-deprecation-sophia-rollup-filesize-npm-internals-subtree"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "rollup-plugin-filesize@10 drags npm's whole internal fetcher stack into sophia — 6 deprecated packages (npmlog/gauge/are-we-there-yet/read-package-json/@npmcli/move-file/tar@6) from one cosmetic build-log plugin at its newest release"
slug: "deprecation-sophia-rollup-filesize-npm-internals-subtree"
written: "2026-07-30"
author: "deprecation-triage"
status: "backlog"
priority: "medium"
deprecation_status: open
severity: low
fingerprints: ["ce0de21b8053"]
relatedNodeIds:
  - "backlog-deprecation-glob-support-window-upgrade-unit"
tags: [deprecation, sophia, rollup, rollup-plugin-filesize, pacote, node-gyp, cacache, npmlog, tar, upstream-stale, bounded-fix]
cites:
  - https://github.com/ConradIrwin/rollup-plugin-filesize
  - https://www.npmjs.com/package/rollup-plugin-filesize
  - sophia/package.json
  - sophia/config/build/rollup.config.js
  - sophia/packages/sophia-element/rollup.config.mjs
  - sophia/packages/psephos-element/rollup.config.mjs
  - genesis/data/timeline/backlog/deprecation-sophia-dead-devdependency-declarations.md
---

## What is deprecated

Six of the 25 packages in the sophia install banner (fingerprint `ce0de21b8053`)
enter the tree through **one cosmetic build-log plugin**:
`rollup-plugin-filesize@10.0.0`, which prints bundle sizes after a rollup build.
It depends on `pacote` — npm's package fetcher — purely to look up the *published*
size of the package for comparison, and `pacote` drags in npm's entire internal
fetcher/installer stack.

Verbatim lockfile `deprecated:` fields:

```
npmlog@6.0.2             This package is no longer supported.
gauge@4.0.4              This package is no longer supported.
are-we-there-yet@3.0.1   This package is no longer supported.
read-package-json@6.0.4  This package is no longer supported. Please use @npmcli/package-json instead.
@npmcli/move-file@2.0.1  This functionality has been moved to @npmcli/fs
tar@6.2.1                Old versions of tar are not supported, and contain widely publicized
                         security vulnerabilities, which have been fixed in the current version.
                         Please update. …
```

It additionally supplies carrier edges for three packages canonicalized
elsewhere: `glob@8.1.0` and `inflight@1.0.6` (via `cacache@16.1.3`) belong to the
glob support-window entry, and `rimraf@3.0.2` is multi-carrier.

## Usage inventory

Reverse-dep trace over `sophia/pnpm-lock.yaml`'s `snapshots:` section
(parent-edge index, peer-suffix normalised). The entire subtree hangs off a
**single root importer**:

```
rollup-plugin-filesize@10.0.0            (sophia/package.json:121 — sole parent of pacote@15 tree-wide)
└── pacote@15.2.0
    ├── read-package-json@6.0.4          ← DEPRECATED (sole parent: pacote@15)
    ├── tar@6.2.1                        ← DEPRECATED (also via cacache/node-gyp below)
    └── @npmcli/run-script@6.0.2
        └── node-gyp@9.4.1
            ├── npmlog@6.0.2             ← DEPRECATED (sole parent: node-gyp@9)
            │   ├── gauge@4.0.4          ← DEPRECATED (sole parent: npmlog@6)
            │   └── are-we-there-yet@3.0.1 ← DEPRECATED (sole parent: npmlog@6)
            ├── glob@7.2.3 · rimraf@3.0.2        (shared carriers)
            └── make-fetch-happen@10.2.1
                └── cacache@16.1.3
                    ├── @npmcli/move-file@2.0.1  ← DEPRECATED (sole parent: cacache@16)
                    ├── glob@8.1.0 → inflight@1.0.6   (shared carriers)
                    ├── rimraf@3.0.2 · tar@6.2.1      (shared carriers)
```

`cacache@17.1.4` also carries a `tar@6.2.1` edge, likewise under `pacote@15.2.0`.
Every one of these paths terminates at `rollup-plugin-filesize` — there is no
second first-party carrier for `pacote` anywhere in sophia's graph.

**Is the plugin live?** Yes — this is *not* a dead declaration (contrast the
sibling entry, where two others were). It is imported and invoked in all three
rollup configs:

| Config | Import | Invocation |
|---|---|---|
| `config/build/rollup.config.js` | line 15 | `plugins: [filesize()]` at lines 286, 313 |
| `packages/sophia-element/rollup.config.mjs` | line 19 | in the plugin list |
| `packages/psephos-element/rollup.config.mjs` | line 16 | in the plugin list |

It is also separately declared by `packages/sophia-element/package.json:67` and
`packages/psephos-element/package.json:63`.

**What it actually does:** prints a size summary line to the build log. It emits
no artifact, transforms no module, and participates in no output. Removing it
cannot change bundle bytes — only build-log verbosity.

## Migration path

**There is no version escape.** `rollup-plugin-filesize@10.0.0` is simultaneously
the *installed* version and the *latest published* version (`npm view
rollup-plugin-filesize version` → `10.0.0`), and its own manifest declares
`"pacote": "^15.1.1"`. Upstream is stale; waiting does nothing. Three real
options:

1. **Drop the plugin (recommended).** Remove the import and the `filesize()`
   plugin entries from all three rollup configs, and the three manifest
   declarations. Clears all six deprecated packages plus the `glob@8`/`inflight`
   and part of the `rimraf@3`/`tar@6` carrier load in one move. Cost: the build
   log stops printing bundle sizes. Sophia already has a dedicated size-check
   path — `pnpm build:prodsizecheck` — and `build-storybook --stats-json`, so
   observability of bundle size does not depend on this plugin.
2. **Replace with a pacote-free size reporter.** Any plugin (or a ~20-line
   `generateBundle` hook) that reports `Buffer.byteLength` plus `gzip-size`
   /`brotli-size` gives the same log line with none of the fetcher stack. Both
   `gzip-size` and `brotli-size` are already in the tree as filesize's own deps.
   Choose this if the comparison-against-published number is genuinely used.
3. **`pnpm` override on the npm-internals chain.** Sophia already uses
   `overrides` in `pnpm-workspace.yaml` (`qs`, `flatted`, `picomatch`,
   `minimatch`). Overriding `pacote`/`node-gyp`/`cacache` to current majors is
   *not* recommended: `pacote@21` and `cacache@19` changed API surface, and this
   would be pinning npm internals for a plugin that only wants a number.

Option 1 is the bounded one, and it is behaviour-neutral for artifacts.

## Current decision

**Open — bounded fix identified (option 1), not landed this pass, blocked on the
same live worktree race as the sibling dead-declarations entry.**

`sophia` is on branch `feat/jquery-3` with an in-flight jQuery 2→3 migration
holding uncommitted edits to `pnpm-workspace.yaml`, `pnpm-lock.yaml`, and
`packages/sophia/src/jquery.mobile.vmouse.js`. `sophia/pnpm-lock.yaml`'s mtime
(14:58:03Z) is identical to the timestamp of the dispatching ledger fingerprint —
the `pnpm install` that produced this banner *is* that sprint's install. Any
dependency removal here requires regenerating that same lockfile, which cannot be
staged selectively and would rewrite `node_modules` under a live test run. See the
sibling entry's *Current decision* for the full reasoning; it applies verbatim.

**Artifact availability is NOT a blocker.** The repo switched public-package
resolution to `registry.npmjs.org` in commit `ecc65384f` (2026-07-30), so the
"Nexus mirror serves cached artifacts only" constraint recorded across sibling
entries does not apply. Probed this pass: uncached
`rollup-plugin-filesize-10.0.0.tgz` → 200.

This is deliberately **not** marked `blocked`: there is no technical unknown and
no upstream dependency. It is a queued, specified fix waiting on a named,
short-lived event.

**Update, same day — the worktree race cleared and the sibling fix landed.** The
`feat/jquery-3` sprint committed (`3adf1d493c`), and the sibling dead-declaration
removals were applied and verified in sophia commit `a4d931cca1` (483 test suites
/ 6453 tests, plus build, typecheck, and lint all green). That entry has been
decomposed — closure is recorded in the commit, not in the backlog. So the
lockfile is no longer contended and **this fix is now landable**; it was left for
a separate pass only because removing a live build plugin needs its own
byte-identity verification, not because anything blocks it.

### Carried forward from the closed sibling — the knip trap

Both entries lean on `pnpm knip --dependencies` as corroborating evidence. Record
the caveat here, since the entry that documented it is gone:

`sophia/knip.config.ts` excludes `!utils/**` from its project scan, so any
dependency used *only* by a helper in `utils/` reads as **unused**. Concretely,
knip reports `nyc` as an unused devDependency — and `nyc` is **live**, driven by
`utils/test-with-coverage.sh` and the `nyc` key in `package.json`. Never act on a
knip "unused dependency" line without a cross-checking grep that includes
`utils/` and all shell scripts. `rollup-plugin-filesize` is *not* flagged by knip
at all, which is independent confirmation that it is genuinely in use — the fix
for this entry is a deliberate removal of a live plugin, not a dead-declaration
cleanup, and the two must not be conflated.

### Live trajectory

Once `git -C sophia status --porcelain` is clean:

1. Decide option 1 vs option 2 — a one-line judgement call about whether anyone
   reads the published-size comparison. Default to option 1; the repo has
   `build:prodsizecheck` for the real question.
2. Remove the `filesize` import + `filesize()` plugin entries from
   `config/build/rollup.config.js` (lines 15, 286, 313),
   `packages/sophia-element/rollup.config.mjs` (line 19), and
   `packages/psephos-element/rollup.config.mjs` (line 16); drop the three
   manifest declarations (`package.json:121`,
   `packages/sophia-element/package.json:67`,
   `packages/psephos-element/package.json:63`).
3. `pnpm install && pnpm build && pnpm test` — and specifically diff the built
   UMD bundle against a pre-change build to prove byte-identity. That is the
   verification that matters: a build-log plugin must not change output.
4. Confirm the six packages are absent from `sophia/pnpm-lock.yaml` and the
   banner count drops accordingly (25 → 19 for this entry alone; → 17 combined
   with the dead-declarations entry).
5. On green, close out with full decomposition — delete this entry, quote the
   byte-identity proof in the commit message. Do not delete fingerprint
   `ce0de21b8053`; it is the shared aggregate banner.

Best landed **in the same commit** as the sibling dead-declarations entry: one
`pnpm install`, one lockfile regeneration, one verification run.

## Verification

No fix landed this pass; scoping evidence only.

- **Sole-carrier proof.** Parent-edge index over `sophia/pnpm-lock.yaml`
  `snapshots:` — `pacote@15.2.0` has exactly one parent tree-wide
  (`rollup-plugin-filesize@10.0.0`), which is itself a root importer. Sole-parent
  chains confirmed for `read-package-json@6.0.4` ← `pacote@15`;
  `npmlog@6.0.2` ← `node-gyp@9.4.1`; `gauge@4.0.4` and `are-we-there-yet@3.0.1`
  ← `npmlog@6.0.2`; `@npmcli/move-file@2.0.1` ← `cacache@16.1.3`.
- **No version escape.** `npm view rollup-plugin-filesize version` → `10.0.0`,
  equal to the installed version; the installed
  `node_modules/rollup-plugin-filesize/package.json` declares
  `"pacote": "^15.1.1"`. So the newest release still carries the whole stack.
- **Plugin is live, not dead.** Import + invocation lines read directly from all
  three rollup configs (table above). Contrast: `knip --dependencies` does **not**
  list `rollup-plugin-filesize` as unused, while it does list the two packages in
  the sibling entry — an independent cross-check that this one is genuinely in use.
- **Contention, observed:** `sophia` dirty on `feat/jquery-3`;
  `sophia/pnpm-lock.yaml` mtime `2026-07-30 14:58:03Z` equals the dispatching
  fingerprint's timestamp.

Closure requires: plugin removed, six packages absent from the lockfile, and a
**byte-identical UMD bundle diff** plus green `pnpm build`/`pnpm test` — then this
entry is deleted, not parked.
