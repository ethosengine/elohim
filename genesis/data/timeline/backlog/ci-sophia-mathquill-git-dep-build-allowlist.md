---
id: "backlog-ci-sophia-mathquill-git-dep-build-allowlist"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "elohim-sophia Install hard-fails — mathquill git dep lost its onlyBuiltDependencies entry in the pnpm-10 package.json→workspace migration"
slug: "ci-sophia-mathquill-git-dep-build-allowlist"
written: "2026-07-31"
author: "ci-failure-triage"
status: "wip"
priority: "high"
ci_status: in-progress
fingerprints: [88e5f1135510]
jobs: [elohim-sophia]
relatedNodeIds: []
tags: [ci, elohim-sophia, pnpm, pnpm10, git-dependency, onlybuiltdependencies, mathquill, install-stage, host-green-not-ci-green]
cites:
  - https://jenkins.ethosengine.com/job/elohim-sophia/job/dev/145/
  - https://jenkins.ethosengine.com/job/elohim-sophia/job/dev/146/
  - sophia/pnpm-workspace.yaml
  - sophia/packages/math-input/package.json
  - genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md
---

# `elohim-sophia` Install aborts — a git-hosted dep needs a build script it is no longer allowed to run

## The failure

`elohim-sophia/dev` build **#145** (`FAILURE`, 62s, stage **Install**), started by
upstream `elohim-orchestrator/dev` #1578. Quoted verbatim from the #145 console
(line 218):

```
 ERR_PNPM_GIT_DEP_PREPARE_NOT_ALLOWED  Failed to prepare git-hosted package fetched from
 "https://codeload.github.com/Khan/mathquill/tar.gz/c9e4329b0bc5d9b4c21d765b5768e4e7693515b3":
 The git-hosted package "mathquill@1.0.3" needs to execute build scripts but is not in the
 "onlyBuiltDependencies" allowlist.
```

Occurrence evidence (ledger `88e5f1135510`): **seen 1**, `first_build` 145,
`last_build` 145. The harvester classified it `UNCLASSIFIED` with the fallback
line `red build, stage:Install` — the real signature is the `ERR_PNPM_*` line
above, which sits in the console tail but matched no taxonomy rule.

#145 was the **first orchestrator-dispatched** `elohim-sophia` run — the job had
just been wired in by sophia `676cfc71b3` ("declare jenkinsPath in
build-manifest — make elohim-sophia orchestrator-dispatchable"). The newly-wired
pipeline immediately surfaced a latent config loss on its first execution.

## Verdict — **real**, deterministic, not a flake

Not a flake, not infra. Given the missing allowlist entry, #145 fails every time
the git dep must be *prepared*. The `seen: 1` is not weak evidence of a flake —
it is the fix landing before a second occurrence could accrue.

## Root cause

`mathquill` is a **git-hosted dependency** —
`"mathquill": "github:Khan/mathquill#v1.0.3"` in
`sophia/packages/math-input/package.json`. A git dep has no published `dist`, so
pnpm must run its `prepare` script at fetch-into-store time. Under pnpm 10 that
requires an explicit `onlyBuiltDependencies` entry.

On **2026-07-30** sophia's root pnpm settings were migrated out of `package.json`'s
`pnpm` key into `pnpm-workspace.yaml`, because pnpm 10 no longer reads that key
(pnpm's own warning: *"The pnpm field in package.json is no longer read by pnpm"*).
The migration was a manual transcription of the allowlist and **`mathquill` was
not carried over** — `cypress`, `esbuild` and `@swc/core` were.

The deprecation warning names the dead *field*; it does not enumerate what the
field contained. So a partial transcription is silent, and the loss stays
invisible until something actually needs the dropped entry.

**Why nobody saw it locally** — this is the museum's *host-green ≠ CI-green*
class (see the museum record, "Host-green ≠ CI-green; the gap is the environment,
not your code"). pnpm runs a git dep's `prepare` only when fetching it **into the
store**. A developer with a warm pnpm store already holds a built `mathquill`,
so `pnpm install` reuses it and the allowlist gate never fires. CI runs cold —
#145's progress line reads `resolved 1668, reused 0, downloaded 1666`, a
fully-cold store — so CI is the *only* environment that exercises the gate.
No new museum trap: this is an instance of an already-catalogued class.

## Current decision

**Fixed and confirmed green in CI.** Ledger `88e5f1135510` is `status: triaged`,
`triaged_at_build: 145`, `decompose_on_confirm: true` — the concern carries no
lesson the in-place config comment does not carry better (the restored entry
names the mechanism and cites #145 at exactly the line a future editor edits), so
the harvester should delete this entry and the ledger line once sophia's green
streak confirms disappearance (≥3; at time of writing the streak is 1).

Recurrence would mean the allowlist regressed again — reopen rather than re-derive.

## Fix trail

- **sophia `c9de47ae4d`** — *"fix(ci): restore mathquill to onlyBuiltDependencies
  — git dep needs prepare script"*. Four lines added to
  `sophia/pnpm-workspace.yaml`: the `mathquill` entry plus a comment recording
  that it fell off in the 2026-07-30 migration and naming the CI symptom.
- **parent `0ac6fdaa3d`** — *"chore(sophia): bump submodule — mathquill
  build-script allowlist fix"*. The parent submodule pointer now reads
  `c9de47ae4d5e502d3c5562e96d7e8995643bbe8f`, matching sophia's `main` HEAD.

**Verification — CI, not host** (the failure is cold-store-only, so a host run
proves nothing): `elohim-sophia/dev` **#146** is `SUCCESS` (19m36s), also on a
fully-cold store (`resolved 1668, reused 0`), and its console shows the prepare
step now running rather than being refused:

```
..._158_8fcea4789973e7c87a85fa4bb36d7935 npm-install: > mathquill@1.0.3 prepare
```

The dependent suites pass in the same build —
`packages/math-input/src/components/input/__tests__/mathquill.test.ts`,
`.../keypad/__tests__/keypad-v2-mathquill.test.tsx`, and
`.../input/__tests__/mathquill-helpers.test.ts` are all `PASS`.

## Adjacent concern (not folded here)

The same Install stage emits `WARN Unsupported engine: wanted: {"node":"^24.18.0"}
(current: {"node":"v20.20.2"})` on every run. That is a **separate** concern with
its own trajectory and its own file —
`genesis/data/timeline/backlog/ci-sophia-node-engine-skew-ci-pod.md`. It is
recorded there precisely because this entry is marked for decomposition.
