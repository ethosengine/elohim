---
id: "backlog-deprecation-analogjs-vite-optimizedeps-esbuildoptions"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "@analogjs/vite-plugin-angular sets optimizeDeps.esbuildOptions; Vite 7.3 deprecates it in favour of optimizeDeps.rolldownOptions"
slug: "deprecation-analogjs-vite-optimizedeps-esbuildoptions"
written: "2026-09-24"
author: "deprecation-triage"
status: "backlog"
priority: "low"
deprecation_status: blocked
severity: low
fingerprints: ["612e34199acb"]
relatedNodeIds: []
tags: [deprecation, vite, analogjs, vite-plugin-angular, lamad, vitest]
cites:
  - app/lamad/vite.config.ts
  - app/lamad/package.json
  - pnpm-lock.yaml
---

## What is deprecated

```
[vite] warning: `optimizeDeps.esbuildOptions` option was specified by "@analogjs/vite-plugin-angular" plugin. This option is deprecated, please use `optimizeDeps.rolldownOptions` instead.
```

Captured from `pnpm exec vitest run --config vite.config.ts` in `app/lamad`
(`fp 612e34199acb`).

## Usage inventory

`app/lamad/vite.config.ts` does **not** set `optimizeDeps.esbuildOptions`
itself — the app config only passes `{ tsconfig: 'tsconfig.spec.json' }` to
the `angular()` plugin factory. The option is set **inside the plugin**:

```
node_modules/.pnpm/@analogjs+vite-plugin-angular@2.6.4.../src/lib/utils/plugin-config.js:90
            ...(vite.rolldownVersion ? { rolldownOptions } : { esbuildOptions }),
```

`@analogjs/vite-plugin-angular@2.6.4` already branches on
`vite.rolldownVersion` — it only emits the legacy `esbuildOptions` key when
the installed Vite is the classic (non-rolldown) build, which is exactly the
resolved version here (`vite@7.3.1`, no `rolldownVersion`). So the plugin's
own conditional is doing the "right" thing for a pre-rolldown Vite, and Vite
7.3.1 is nonetheless emitting the option as deprecated on that branch —
Vite's own migration timeline has moved ahead of what 2.6.4's branch expects.

This is not our config to change: no other Angular workspace in the
monorepo (`app/elohim-app`, `app/elohim-library`) sets
`optimizeDeps.esbuildOptions` either, and all three resolve the same
`@analogjs/vite-plugin-angular@2.6.4` via `pnpm-lock.yaml`.

## Migration path

Track `@analogjs/vite-plugin-angular` releases past `2.6.4` for a build that
either stops setting `esbuildOptions` on the classic Vite branch or adopts
`rolldownOptions` unconditionally. No changelog entry confirming a fix has
been located yet (bounded search, 2026-09-24); re-check on the next
dependency-bump pass.

## Current decision

**Blocked**, for two independent reasons:

1. **Upstream, not ours.** The deprecated option is set inside the
   `@analogjs/vite-plugin-angular` package itself; there is no local config
   to migrate. A fix requires an upstream release.
2. **This run's constraint.** A household serving-receipt proof was live in
   this workspace for the triage session that canonicalized this entry, and
   `pnpm-lock.yaml` / `pnpm-workspace.yaml` were explicitly off-limits for
   that session (a dependency-version change would invalidate the receipt's
   source identity mid-run). Any real fix here is a version bump, so it
   waits for a session where the lockfile is writable.

Cosmetic and harmless in the meantime — it is a dev-time Vite warning about
its own internal option migration, not a runtime behavior change; the vitest
suite it was captured from runs and passes regardless.

The sentinel will suppress further dispatch on `612e34199acb` (ledger status:
blocked). Re-check when `@analogjs/vite-plugin-angular` is next bumped: if the
new version stops emitting the warning, delete the ledger fingerprint and
this entry; if it still does, re-confirm the same upstream-only disposition
and fold the new fingerprint in here.

## Verification

N/A — not fixed (blocked on an upstream release + this session's lockfile
constraint).
