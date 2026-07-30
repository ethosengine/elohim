---
id: "backlog-deprecation-sophia-khan-math-parser-legacy"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "sophia's math layer carries two upstream-pinned deprecations — mathjax-full@3 (superseded by @mathjax/src at v4) via @khanacademy/mathjax-renderer, and nomnom via the kas jison parser generator"
slug: "deprecation-sophia-khan-math-parser-legacy"
written: "2026-07-30"
author: "deprecation-triage"
status: "backlog"
priority: "low"
deprecation_status: blocked
severity: low
fingerprints: ["ce0de21b8053"]
relatedNodeIds:
  - "backlog-deprecation-sophia-rollup-filesize-npm-internals-subtree"
tags: [deprecation, sophia, mathjax, mathjax-full, khanacademy, nomnom, jison, kas, parser-generator, upstream-pinned, fork-boundary]
cites:
  - https://www.npmjs.com/package/mathjax-full
  - https://www.npmjs.com/package/@mathjax/src
  - https://www.npmjs.com/package/@khanacademy/mathjax-renderer
  - https://www.npmjs.com/package/jison
  - sophia/pnpm-workspace.yaml
  - sophia/packages/kas/package.json
  - sophia/packages/sophia-editor/src/widgets/interactive-graph-editor/locked-figures/util.ts
  - sophia/packages/math-input/src/components/input/mathquill-instance.ts
---

## What is deprecated

Two of the 25 packages in the sophia install banner (fingerprint
`ce0de21b8053`) sit in sophia's **math layer** — the part of the Perseus fork
that renders equations and parses mathematical expressions. Both are pinned by an
upstream package rather than by sophia directly.

```
mathjax-full@3.2.2   Version 4 replaces this package with the scoped package @mathjax/src
nomnom@1.5.2         Package no longer supported. Contact support@npmjs.com for more info.
```

They are grouped as one concern because they share a shape: each is held in place
by a dependency sophia does not control (Khan's renderer package; an abandoned
parser-generator), and each would move only on someone else's release.

## Usage inventory

### `mathjax-full@3.2.2` — via `@khanacademy/mathjax-renderer`

```
mathjax-full@3.2.2                    ← DEPRECATED
└── @khanacademy/mathjax-renderer@3.0.0   (sole parent tree-wide; a root importer)
```

`@khanacademy/mathjax-renderer` is **catalog-pinned** in
`sophia/pnpm-workspace.yaml` — `devDeps: 3.0.0`, `peerDeps: ^3.0.0` — and consumed
by two workspace packages, each declaring it `catalog:devDeps` + `catalog:peerDeps`:

| Package | Manifest lines | First-party import site |
|---|---|---|
| `packages/sophia-editor` | 52 (dev), 84 (peer) | `src/widgets/interactive-graph-editor/locked-figures/util.ts:9` — `import {SpeechRuleEngine} from "@khanacademy/mathjax-renderer"` |
| `packages/math-input` | 48 (dev), 68 (peer) | `src/components/input/mathquill-instance.ts:1` — same import |

Both use exactly one export: `SpeechRuleEngine` (accessibility — spoken math for
screen readers). Note the catalog header comment: the `devDeps`/`peerDeps`
catalogs are **generated from Khan's `frontend` repo** via
`utils/sync-dependencies.ts`, so this pin is inherited from upstream Khan, not
chosen locally.

### `nomnom@1.5.2` — via the `kas` parser generator

```
nomnom@1.5.2                          ← DEPRECATED
├── jison@0.4.15                      (root importer: packages/kas)
├── jison-lex@0.3.4                   (under jison)
└── jsonlint@1.6.0 ← cjson@0.3.0      (under jison)
```

`jison` is declared at `sophia/packages/kas/package.json:32` (pinned exactly,
`"jison": "0.4.15"`) and drives `gen:parsers` (line 26,
`node src/parser-generator.ts`). That script is **in the build path** — the root
`prebuild` runs `pnpm gen:parsers` before every `pnpm build`. `nomnom` is jison's
CLI argument parser, reached three ways inside the jison tree.

`kas` is the Khan Academy expression-parsing library (the math-answer comparison
engine); its grammar is compiled to a parser at build time.

## Migration path

Neither has a lever sophia can pull alone.

**`mathjax-full` → `@mathjax/src` (MathJax v4).** The rename is upstream's; the
consumer that must adopt it is `@khanacademy/mathjax-renderer`. Latest published
is **3.1.4** — still a 3.x line, still on `mathjax-full`. So:

- Bumping the catalog `3.0.0 → 3.1.4` is cheap and probably worth doing on
  general currency grounds, but it does **not** clear this deprecation.
- Clearing it requires a `@khanacademy/mathjax-renderer` **v4** that depends on
  `@mathjax/src`. That release does not exist. This is a genuine
  wait-on-upstream.
- The fork-boundary caveat: because the catalog is generated from Khan's
  `frontend` repo, the clean way to take a renderer major is to re-run
  `utils/sync-dependencies.ts` against an updated upstream catalog rather than
  hand-editing the pin — otherwise the next sync reverts it.

**`nomnom` → nothing.** `jison@0.4.15` (2017) is the last release; the project is
abandoned. Options, none bounded:

1. **Leave it.** `nomnom` is a CLI arg parser inside a build-time code generator.
   It never reaches a bundle, never runs in a browser, and has no runtime
   exposure. This is the honest default.
2. **Migrate `kas` off jison** to a maintained generator (`peggy`, `nearley`) or a
   hand-written recursive-descent parser. That is a rewrite of the grammar
   pipeline for the math-answer engine — high risk, high effort, zero user-visible
   benefit, and it would diverge the fork further from Khan's upstream.
3. **Pre-generate and commit the parser**, dropping `jison` from the install
   entirely. Tempting, but it trades a dev-only deprecation for a generated
   artifact checked into source — worse for maintenance.

## Current decision

**Blocked — both are pinned by dependencies sophia does not control, and neither
has a non-deprecated target available today.**

- `mathjax-full@3.2.2`: blocked on an upstream `@khanacademy/mathjax-renderer`
  major that adopts `@mathjax/src`. Latest is 3.1.4; no v4 exists. Nothing to do
  but wait.
- `nomnom@1.5.2`: blocked on `jison` being abandoned since 0.4.15 (2017). The only
  escapes are a grammar-pipeline rewrite or committing generated parsers, both of
  which are worse than the deprecation. **Recommended posture: accept
  permanently** — a build-time-only CLI arg parser with no runtime reach.

Explicitly **not** the blocker (recorded so a future pass does not re-derive):
artifact availability. The repo switched public-package resolution to
`registry.npmjs.org` in commit `ecc65384f` (2026-07-30); uncached tarballs probed
200 this pass. The "Nexus mirror serves cached artifacts only" constraint recorded
across sibling entries does not apply.

The ledger fingerprint stays present so the sentinel cites this decision
deterministically and never re-dispatches; the stasis sweep owns the re-check.

### Live trajectory

Two different trajectories in one entry, deliberately:

1. **`mathjax-full` — a watch item with a concrete trigger.** Re-check
   `npm view @khanacademy/mathjax-renderer versions` on each stasis sweep; when a
   `4.x` appears, verify it depends on `@mathjax/src`, then take it via
   `utils/sync-dependencies.ts` against Khan's updated catalog (not a hand-edit)
   and confirm the two `SpeechRuleEngine` import sites still compile. Opportunistic
   ride-along in the meantime: bump the catalog to `3.1.4` next time the sophia
   dependency block is open.
2. **`nomnom` — accept and stop re-examining.** If a future sweep is tempted to
   act on it, the answer is in *Migration path* option 1: no runtime exposure, no
   maintained target, and every alternative is worse. This line exists so that
   conclusion is not re-derived every quarter.

Do not delete this entry while either package remains in `sophia/pnpm-lock.yaml`.
If `mathjax-full` clears and `nomnom` does not, narrow this entry to `nomnom`
alone and drop its priority further.

## Verification

No fix landed this pass; scoping evidence only.

- **Sole-parent proof**, from a parent-edge index over `sophia/pnpm-lock.yaml`
  `snapshots:` (peer-suffix normalised): `mathjax-full@3.2.2` has exactly one
  parent tree-wide, `@khanacademy/mathjax-renderer@3.0.0`, itself a root importer.
  `nomnom@1.5.2` has three parents (`jison@0.4.15`, `jison-lex@0.3.4`,
  `jsonlint@1.6.0`), all inside the single `jison` tree rooted at
  `packages/kas`.
- **First-party import sites** located by grep and read directly: two
  `SpeechRuleEngine` imports (`packages/sophia-editor/.../locked-figures/util.ts:9`
  and `packages/math-input/src/components/input/mathquill-instance.ts:1`). No
  other export of the renderer is used.
- **Catalog pinning confirmed** in `sophia/pnpm-workspace.yaml`
  (`peerDeps: ^3.0.0`, `devDeps: 3.0.0`), together with the file's own header
  comment stating the catalogs are generated from Khan's `frontend` repo via
  `utils/sync-dependencies.ts` — the fork-boundary fact that makes hand-editing
  the pin the wrong move.
- **`jison` is build-path, not optional:** `packages/kas/package.json:32` pins
  `0.4.15`; line 26 defines `gen:parsers`; the root `prebuild` script runs
  `pnpm gen:parsers` ahead of every build.
- **Upstream state probed this pass:** `npm view @khanacademy/mathjax-renderer
  version` → `3.1.4` (no 4.x). `jison` remains at `0.4.15`.

Closure requires both packages absent from `sophia/pnpm-lock.yaml`; partial
clearance narrows this entry rather than closing it.
