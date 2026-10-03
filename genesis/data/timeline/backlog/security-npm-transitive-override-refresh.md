---
id: "backlog-security-npm-transitive-override-refresh"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "npm transitive advisories: 226 open Dependabot alerts close with 50 range-scoped, same-major pnpm overrides (resolution proven, install and gates not yet run)"
slug: "security-npm-transitive-override-refresh"
written: "2026-10-03"
author: "deprecation-triage"
status: "backlog"
priority: "high"
deprecation_status: open
severity: security
fingerprints: []
relatedNodeIds:
  - "backlog-dependabot-triage"
  - "backlog-security-vite-dev-server-fs-access-advisory-family"
tags: [security, npm, pnpm, overrides, dependabot, hono, axios, undici, brace-expansion, fastify]
cites:
  - https://github.com/ethosengine/elohim/security/dependabot
  - https://pnpm.io/settings#overrides
  - package.json
  - pnpm-workspace.yaml
  - pnpm-lock.yaml
  - .npmrc
  - genesis/data/timeline/backlog/dependabot-triage.md
---

# npm transitive advisories closable by same-major overrides

## What is flagged

> remote: GitHub found 302 vulnerabilities on ethosengine/elohim's default branch
> (135 high, 140 moderate, 27 low).

This entry holds the largest single unit inside that banner (ledger fingerprint
`c81302c029ff`, carried by the umbrella `dependabot-triage`). Of the 252 open alerts whose
manifest is the root `pnpm-lock.yaml`, **226 have a patched version on the same major line
as the version installed** (same minor for `0.x`). 101 are high, 113 medium, 12 low.

Read on 2026-10-03 with
`gh api --paginate '/repos/ethosengine/elohim/dependabot/alerts?state=open&per_page=100'`,
which now works from the dev environment.

## Usage inventory

Every one is transitive or a direct dependency of a tooling package. Heaviest packages, with
the parent that pulls the vulnerable version:

| Package (installed) | Alerts | Pulled by |
|---|---|---|
| `hono` 4.12.3 | 35 | `@modelcontextprotocol/sdk` 1.27.1 / 1.29.0, `@hono/node-server` |
| `axios` 1.13.2, 1.13.5 | 35 | `sonarqube-scanner`, `acme-client`, `npm-groovy-lint`, `wait-on` |
| `undici` 7.22.0 | 24 | `jsdom` 28.1.0, `@libp2p/http`; direct in `genesis/a2o` |
| `brace-expansion` 1.1.12, 2.0.2, 5.0.3 | 22 | the `minimatch` lines |
| `@xmldom/xmldom` 0.8.13 | 10 | `@lit/localize-tools` |
| `vite` 6.4.1, 7.3.1 | 9 | `@angular/build`, vitest 3 line |
| `fastify` 5.8.1 | 8 | direct in `elohim/elohim-agent/elohim-agent-sdk` |
| `fast-uri` 3.1.0, `js-yaml` 3.14.2 / 4.1.1 / 4.3.0 | 8 each | ajv, eslint and cucumber chains |

The remaining 67 alerts spread over 36 packages at one to five alerts each; the override list
below names all of them.

## Migration path

Two facts changed since the 2026-07-30 npm campaign recorded these as `MIRROR-RETRY` and
`NOT-REACHABLE`:

1. **The mirror constraint is gone.** `.npmrc` now sets `registry=https://registry.npmjs.org/`
   (registry split, 2026-07-30). Every target version below resolved from npmjs.
2. **`pnpm update` does not move them.** `pnpm -r update --lockfile-only --depth 99 <packages>`
   closed 32 alerts; the rest stay pinned by the existing lock. Overrides are required.

**Where the overrides must go.** On pnpm 10.30.3 the lockfile's `overrides:` block records the
16 entries of root `package.json` `pnpm.overrides`, and the same 50 entries placed in
`pnpm-workspace.yaml` `overrides:` had no effect on resolution. The committed lock also lacks
the `'@holochain/client': ^0.21.0` override that exists only in `pnpm-workspace.yaml`. The
comment in `pnpm-workspace.yaml` ("pnpm >=10 ignores the package.json pnpm field") does not
match this behaviour. Settle the single home for overrides as part of this work; until then,
add these to `package.json` `pnpm.overrides`.

**Proof run (isolated copy, not the repo tree).** Copied the root and 24 workspace
`package.json` files, `pnpm-workspace.yaml`, `.npmrc`, `patches/`, the lock and
`elohim/elohim-cache-core/pkg` to a scratch directory, added the 50 entries to
`package.json` `pnpm.overrides`, ran `pnpm install --lockfile-only`: exit 0, lock diff
1523 insertions / 1479 deletions. Re-checking all 252 alerts' vulnerable ranges against the
resulting lock left 22 live, all owned by sibling entries.

```yaml
'fastify@>=5.0.0 <5.12.5': ^5.12.5
'hono@>=4.0.0 <4.13.7': ^4.13.7
'axios@>=1.0.0 <1.20.0': ^1.20.0
'ip-address@>=10.0.0 <10.7.1': ^10.7.1
'brace-expansion@>=1.0.0 <1.1.21': ^1.1.21
'brace-expansion@>=2.0.0 <2.1.7': ^2.1.7
'brace-expansion@>=5.0.0 <5.0.12': ^5.0.12
'engine.io@>=6.0.0 <6.6.10': ^6.6.10
'undici@>=7.0.0 <7.29.1': ^7.29.1
'joi@>=17.0.0 <17.13.7': ^17.13.7
'webpack-dev-middleware@>=7.0.0 <7.4.6': ^7.4.6
'fast-uri@>=3.0.0 <3.1.7': ^3.1.7
'@libp2p/peer-store@>=12.0.0 <12.0.24': ^12.0.24
'js-yaml@>=3.0.0 <3.15.2': ^3.15.2
'js-yaml@>=4.0.0 <4.3.2': ^4.3.2
'@xmldom/xmldom@>=0.8.0 <0.8.15': ^0.8.15
'colord@>=2.0.0 <2.9.4': ^2.9.4
'baseline-browser-mapping@>=2.0.0 <2.11.0': ^2.11.0
'qs@>=6.0.0 <6.16.0': ^6.16.0
'@humanfs/node@>=0.16.0 <0.16.8': ^0.16.8
'nanoid@>=3.0.0 <3.3.18': ^3.3.18
'nanoid@>=5.0.0 <5.1.16': ^5.1.16
'browserslist@>=4.0.0 <4.28.7': ^4.28.7
'postcss-selector-parser@>=7.0.0 <7.1.3': ^7.1.3
'@hono/node-server@>=1.0.0 <1.19.15': ^1.19.15
'postcss@>=8.0.0 <8.5.23': ^8.5.23
'find-my-way@>=9.0.0 <9.7.0': ^9.7.0
'immutable@>=5.0.0 <5.1.8': ^5.1.8
'body-parser@>=2.0.0 <2.3.0': ^2.3.0
'webpack-dev-server@>=5.0.0 <5.2.6': ^5.2.6
'http-proxy-middleware@>=3.0.0 <3.0.7': ^3.0.7
'piscina@>=4.0.0 <4.9.3': ^4.9.3
'form-data@>=4.0.0 <4.0.6': ^4.0.6
'vite@>=6.0.0 <6.4.3': ^6.4.3
'vite@>=7.0.0 <7.3.5': ^7.3.5
'@babel/core@>=7.0.0 <7.29.6': ^7.29.6
'ws@>=7.0.0 <7.5.11': ^7.5.11
'ws@>=8.0.0 <8.21.0': ^8.21.0
'uuid@>=11.0.0 <11.1.1': ^11.1.1
'uuid@>=13.0.0 <13.0.1': ^13.0.1
'@libp2p/kad-dht@>=16.0.0 <16.2.6': ^16.2.6
'protocol-buffers-schema@>=3.0.0 <3.6.1': ^3.6.1
'follow-redirects@>=1.0.0 <1.16.0': ^1.16.0
'lodash-es@>=4.0.0 <4.18.0': ^4.18.0
'lodash@>=4.0.0 <4.18.0': ^4.18.0
'path-to-regexp@>=8.0.0 <8.4.0': ^8.4.0
'node-forge@>=1.0.0 <1.4.0': ^1.4.0
'yauzl@>=3.0.0 <3.2.1': ^3.2.1
'file-type@>=21.0.0 <21.3.2': ^21.3.2
'express-rate-limit@>=8.0.0 <8.2.2': ^8.2.2
```

**Known hazard from the proof run.** The re-resolve let Angular float: the scratch lock
carried `@angular/common` 22.1.0 and 22.2.1, `@angular/router` 22.1.0 and 22.2.1, and
`@angular/platform-server` 22.1.8 next to a 22.1.0 core. A mixed Angular family must not be
committed, and `patchedDependencies` pins `@angular/build@22.1.0`. Either land
`security-angular-22-patch-line-coordinated-bump` first, or confirm the Angular entries in
the lock diff are unchanged before committing this one.

## Current decision

**Open, ready to run; not landed on 2026-10-03.** The override set is resolution-proven only.
Landing needs `pnpm install` in the shared working tree and the JS gates for all 25 workspace
projects; the dispatching session had an alpha fleet roll in flight in that tree and
`.worktrees/` was off limits, so no install was run. Nothing blocks it upstream.

Next step, in a quiet tree or a worktree:

1. Add the 50 entries to `package.json` `pnpm.overrides`; `pnpm install`.
2. Check the lock diff for `@angular/*` drift (see hazard above).
3. `just gate` for the changed projects, plus `just test app`; `genesis/a2o` lint and unit tests.
4. Re-read the alerts API after the push; expect the open count to fall by about 226.

Overrides that pin a transitive line are debt: each one should be dropped when its parent
releases a version that requires the patched range.

## Verification

None yet. Closure requires a green gate run plus the alerts API showing the count drop.
