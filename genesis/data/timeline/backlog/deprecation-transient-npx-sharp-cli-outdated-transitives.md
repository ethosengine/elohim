---
id: "backlog-deprecation-transient-npx-sharp-cli-outdated-transitives"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Transient npx sharp-cli outdated transitives (sliced, lodash.pick) — not repo debt"
slug: "deprecation-transient-npx-sharp-cli-outdated-transitives"
written: "2026-07-03"
author: "deprecation-triage"
status: "backlog"
priority: "low"
deprecation_status: blocked
severity: low
fingerprints: ["9cce60d22e4f", "95578a6fc2b8"]
relatedNodeIds: []
tags: [deprecation, npx, sharp-cli, sliced, lodash-pick, transient, scan-noise]
cites:
  - https://www.npmjs.com/package/sharp-cli
  - https://sharp.pixelplumbing.com/
  - steward/device/package.json
  - genesis/a2o/scripts/look.ts
  - .claude/hooks/deprecation-sentinel.py
---

## What is deprecated

Two `npm warn deprecated` lines emitted while an **ephemeral `npx --yes
sharp-cli`** invocation resolved its dependency tree during an agent session
(cropping the header of a `pnpm look` report screenshot). The two lines:

```
npm warn deprecated sliced@1.0.1: Unsupported                                    (fp 9cce60d22e4f)
npm warn deprecated lodash.pick@3.1.0: This package is deprecated.               (fp 95578a6fc2b8)
                    Use destructuring assignment syntax instead.
```

Both are **transitive dependencies of the `sharp-cli` CLI wrapper**, an
unmaintained package pulled transiently into the npx cache — never installed by,
declared in, or resolved into this repository. The triggering command was an
image-crop step (`… && (command -v convert || command -v magick) && convert
shot.png -crop 1280x220+0+0 header-crop.png`); when ImageMagick was absent the
session reached for `npx --yes sharp-cli` as a fallback, and npm surfaced the
wrapper's stale transitives while resolving it.

These are **not elohim dependency debt**: no repo lockfile contains `sharp-cli`,
`sliced`, or `lodash.pick`. The warnings describe the internals of an external
tool an agent ran once, not the state of the tree.

## Usage inventory

Zero repo usage. Confirmed absence across every lockfile and every workspace
manifest:

- `sharp-cli`, `sliced@`, `lodash.pick` → **zero** matches in `pnpm-lock.yaml`,
  `che-devworkspaces/package-lock.json`, `sophia/pnpm-lock.yaml`,
  `elohim/kitsune2/docs-site/yarn.lock`, and the `.claude/worktrees/*` locks.
- No `package.json` in the workspace declares `sharp-cli`.
- The a2o look pipeline (`genesis/a2o/scripts/look.ts`, `graphos.ts`) codifies
  **no** image-crop tooling — Playwright captures the screenshot; the crop was
  an ad-hoc, per-session agent step, not a repo script.
- The one repo-wide `sliced` grep hit is the English word in a spec comment
  (`app/lamad/src/app/services/related-concepts.service.spec.ts:586` — "caches
  the sliced …"), not the package.

Separately, the **modern `sharp` library** (`sharp@0.34.5`) *is* in the tree as
a direct dependency of `steward/device/package.json`. It ships as prebuilt
`@img/sharp-*` binaries and does **not** depend on `sliced` or `lodash.pick`.
`sharp` (the library) and `sharp-cli` (the wrapper that emitted these warnings)
are unrelated as far as this transitive tree is concerned — the in-tree `sharp`
is clean.

## Migration path

None inside this repository — there is nothing here to migrate. The remedy is
purely at the point of use: an agent needing to crop an image should use, in
preference order, (1) ImageMagick `convert`/`magick` (the command already probes
for it first), (2) the modern in-tree `sharp` library (no deprecated
transitives), or (3) any maintained CLI — and should **not** reach for
`npx --yes sharp-cli`, whose stale transitive tree is the sole source of these
warnings. `sharp-cli` itself would only stop emitting these lines if its
upstream refreshed `sliced`/`lodash.pick` — an external event irrelevant to
elohim's correctness.

## Current decision

**Blocked (terminal for automation) — transient npx emission, not repo debt.**
No repo lockfile references any of the three packages, so there is nothing in
this repository to fix. Deleting the ledger lines would be wrong: `sharp-cli`'s
transitive tree is stable, so the next `npx --yes sharp-cli` (e.g. cropping
another look-report screenshot on a host without ImageMagick) would re-mint
these exact two fingerprints as NEW and needlessly re-dispatch a triage agent.
Keeping the two lines present with `status: blocked` makes the sentinel cite
this decision deterministically on every re-encounter and never re-fire; the
deprecation-stasis sweep owns the (no-op) re-check.

**Live trajectory — behavioral, not code.** The permanent-clean resolution is to
not invoke `npx --yes sharp-cli` at all (use ImageMagick or the in-tree `sharp`
library, per Migration path above). A structural option — a
`deprecation-sentinel.py` guard suppressing `npm warn deprecated` lines whose
package is unresolvable in any repo lockfile AND whose triggering command is a
transient `npx`/`npx --yes` — is deferred for the same reason the lit-context
node_modules guard was: a single occurrence does not justify editing a shared,
safety-critical hook, and a naive gate risks over-suppressing real warnings from
`npx`-invoked repo-adjacent tooling. If this transient-npx class recurs, promote
the guard (key on: warning package ∉ any lockfile ∧ command matches
`npx( --yes| -y)? <pkg>`), then decompose this entry and its two fingerprints.

## Verification

- Lockfile scan (all repo lockfiles, `node_modules` excluded): `sharp-cli`,
  `sliced@`, `lodash.pick` → **zero** matches — the packages are not in the
  resolved tree.
- Manifest scan: no workspace `package.json` declares `sharp-cli`; `sharp` is
  declared only by `steward/device/package.json` (`^0.34.5`, prebuilt
  `@img/sharp-*`, no `sliced`/`lodash.pick`).
- Pipeline scan: `genesis/a2o/scripts/` codifies no image-crop step — the
  invocation was ad-hoc, so there is no repo script to correct.

No code change was needed or made; this entry records the disposition so the
deterministic layers answer every re-encounter without another agent dispatch.
