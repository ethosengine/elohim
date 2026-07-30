---
id: "backlog-deprecation-angular19-toolchain-legacy-builder-transitives"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Angular 19 toolchain legacy-builder transitives (karma/webpack/pacote) — glob@7, inflight, rimraf@3, tar@6, uuid@8.3.2"
slug: "deprecation-angular19-toolchain-legacy-builder-transitives"
written: "2026-07-30"
author: "deprecation-triage"
status: "backlog"
priority: "low"
deprecation_status: blocked
severity: low
fingerprints: ["e83cd3f2d7e3"]
relatedNodeIds:
  - "backlog-deprecation-storybook-test-runner-jest-island-retire"
  - "backlog-deprecation-uuid-support-window-upgrade-unit"
tags: [deprecation, angular, angular-devkit, build-angular, angular-cli, karma, webpack-dev-server, pacote, tar, glob, inflight, rimraf, ng-packagr, major-upgrade]
cites:
  - https://angular.dev/tools/cli/build-system-migration
  - https://github.com/angular/angular-cli/releases
  - app/elohim-app/angular.json
  - app/elohim-library/angular.json
  - app/lamad/angular.json
  - app/imagodei-portal/angular.json
  - doorway/doorway-app/angular.json
---

## What is deprecated

Five of the eleven packages in the root-workspace install banner (fingerprint
`e83cd3f2d7e3`) enter the tree through the **Angular 19 build toolchain** —
`@angular-devkit/build-angular@19.2.22` (legacy Karma/Webpack builder set) and
`@angular/cli@19.2.22` (npm fetcher stack). Verbatim lockfile `deprecated:`
fields:

```
glob@7.2.3     Old versions of glob are not supported, and contain widely
               publicized security vulnerabilities, which have been fixed in the
               current version. Please update. …
inflight@1.0.6 This module is not supported, and leaks memory. Do not use it.
               Check out lru-cache …
rimraf@3.0.2   Rimraf versions prior to v4 are no longer supported
tar@6.2.1      Old versions of tar are not supported, and contain widely
               publicized security vulnerabilities … Please update. …
uuid@8.3.2     uuid@10 and below is no longer supported. …
```

None of them is first-party. All five are internal dependencies of build tooling
this repo does not actually exercise: a **Karma** runner, a **Webpack**
dev-server, and **pacote** (npm's package fetcher, used by `ng add`/`ng update`).

## Usage inventory

Reverse-dep trace over `pnpm-lock.yaml` `snapshots:` (peer-suffix-normalised
parent-edge index). `@angular-devkit/build-angular@19.2.22` is a devDependency of
**five** workspaces; `@angular/cli@19.2.22` of the same five:

| Deprecated package | Carrier chain inside the Angular unit |
|---|---|
| `glob@7.2.3` | `karma@6.4.4` → `glob@7.2.3`; `jest@29.7.0` internals (`jest-config@29.7.0`, `jest-runtime@29.7.0`, `@jest/reporters@29.7.0`) → `glob@7.2.3` — both under `@angular-devkit/build-angular@19.2.22` |
| `inflight@1.0.6` | `glob@7.2.3` → `inflight@1.0.6` (its **only** parent tree-wide) |
| `rimraf@3.0.2` | `karma@6.4.4` → `rimraf@3.0.2` |
| `uuid@8.3.2` | `webpack-dev-server@5.2.2` → `sockjs@0.3.24` → `uuid@8.3.2` (also reached via `@angular-devkit/build-webpack@0.1902.22`) |
| `tar@6.2.1` | `@angular/cli@19.2.22` → `pacote@20.0.0` → `tar@6.2.1` — **5 chains, one per importer**, no other parent tree-wide |

Importers (identical set for both carriers): `app/elohim-app`,
`app/elohim-library`, `app/imagodei-portal`, `app/lamad`,
`doorway/doorway-app`. `@angular-devkit/build-angular` is additionally recorded
against `@analogjs/vite-plugin-angular@2.3.0` in the snapshot graph — but only as
a **satisfied optional peer** (upstream declares both `@angular/build` and
`@angular-devkit/build-angular` in `peerDependencies` with
`peerDependenciesMeta.optional: true`), so Analog does not force its presence.

**Builder audit — the shape of the trap.** Every Angular target in all five
`angular.json` files uses a `@angular-devkit/build-angular:*` builder:

| Workspace | Builders in use |
|---|---|
| `app/elohim-app` | `:application`, `:dev-server`, `:extract-i18n` (test: `@analogjs/vitest-angular:test`) |
| `app/lamad` | `:application`, `:dev-server`, `:extract-i18n` |
| `app/imagodei-portal` | `:application`, `:dev-server` |
| `doorway/doorway-app` | `:application`, `:dev-server`, `:extract-i18n` |
| `app/elohim-library` | `:application`, `:dev-server`, `:extract-i18n`, **`:ng-packagr`** (×4 libraries: `elohim-identity`, `elohim-rea-runtime`, `graphos`, `lamad-ui`) |

No first-party target uses Karma or the Webpack dev-server — the whole repo tests
on Vitest via `@analogjs/vitest-angular:test`. The legacy builder set is pure dead
weight that `@angular-devkit/build-angular` ships as **regular** dependencies, so
it cannot be pruned by configuration.

## Migration path

Two steps, and the second is the one that gates everything.

1. **`use-application-builder` migration for the four app workspaces**
   (`app/elohim-app`, `app/lamad`, `app/imagodei-portal`,
   `doorway/doorway-app`): repoint `:application` / `:dev-server` /
   `:extract-i18n` at `@angular/build:*` — already resolved in the tree at
   `@angular/build@19.2.22`, so it needs **no new tarball** — then drop
   `@angular-devkit/build-angular` from those four manifests. Analog's optional
   peer then resolves against `@angular/build`.
2. **`app/elohim-library` is the blocker.** Its four library targets need the
   `ng-packagr` builder, which in Angular **19** exists only as
   `@angular-devkit/build-angular:ng-packagr`. Until elohim-library can use
   `@angular/build:ng-packagr` (Angular 20+), `@angular-devkit/build-angular`
   stays in the root workspace — and because pnpm resolves one
   `karma@6.4.4`/`webpack-dev-server@5.2.2`/`jest@29.7.0` for the whole
   workspace, **`glob@7.2.3`, `inflight@1.0.6`, `rimraf@3.0.2`, and
   `uuid@8.3.2` do not leave the banner** even after step 1.

`tar@6.2.1` is on its own clock: it clears when `@angular/cli` advances to a
major whose `pacote` uses `tar@7`. There is no first-party lever — a
`pnpm-workspace.yaml` override to `tar@7` would break `pacote@20`
(tar v7 changed its API surface), and `tar-7.5.13.tgz` is **404 on the mirror**
anyway.

## Current decision

**Blocked — clearing this unit requires an Angular major upgrade across five
workspaces. That exceeds background-agent scope by the hard rule (major
dependency version, >20 files).**

Plan sketch for the operator-initiated sprint, in order:

1. `ng update @angular/core@20 @angular/cli@20` per workspace, five workspaces,
   plus `@angular-eslint/*@20`, `@angular/compiler-cli@20`, `ng-packagr@20`, and
   the `@analogjs/*@2.x` pair (already declares `^20.0.0 || ^21.0.0` peer
   support, so it does not block).
2. Migrate `app/elohim-library`'s four `ng-packagr` targets to
   `@angular/build:ng-packagr`; run `use-application-builder` across all five
   workspaces; drop `@angular-devkit/build-angular` from every manifest.
3. Re-run the root install and confirm the five packages are gone from the
   banner. `tar@6.2.1` should clear with the CLI 20 `pacote` bump in the same
   pass — verify, don't assume.
4. Blast radius to budget for: the patched dependency
   `patches/@angular__build@19.2.22.patch` (declared in `pnpm-lock.yaml`
   `patchedDependencies`) must be re-cut against the new `@angular/build`
   version, and the `app/elohim-app` production build + AOT strict-template gate
   is the known-fragile surface (see the elohim-app local-build-verification-gaps
   memory: in-container `tsc`/JIT misses `strictTemplates` AOT errors — verify via
   a direct `ng build`).

Secondary blockers that would bite even at smaller scope: `pnpm-lock.yaml` /
`pnpm-workspace.yaml` / workspace `package.json`s are write-locked by concurrent
in-flight runs this session (untouched by this triage), and the Nexus npm mirror
serves **cached artifacts only** — probed this run, `tar-7.5.13.tgz` and
`rimraf-6.1.3.tgz` both `404` while an already-cached
`@anthropic-ai/sdk-0.39.0.tgz` returns `200`.

Fingerprint `e83cd3f2d7e3` stays **present with `status: blocked`**. It is a
**shared aggregate banner fingerprint** decomposed across six sibling entries in
`genesis/data/timeline/backlog/`: this entry,
`deprecation-storybook-test-runner-jest-island-retire.md`,
`deprecation-helia-webrtc-native-addon-react-native-subtree.md`,
`deprecation-anthropic-agent-sdk-legacy-http-stack-bump.md`,
`deprecation-first-party-glob-v10-declarations-bump.md`, and
`deprecation-uuid-support-window-upgrade-unit.md`.

### Live trajectory

This entry's realistic next move is **not** its own sprint — it is to ride the
next Angular major upgrade, which the repo will want for other reasons. Until
then it is a `low`-priority standing record whose value is that the five banner
lines are *explained* and never re-triaged. Concretely: when an Angular 20/21
upgrade is scheduled, attach steps 1–4 above to it as acceptance criteria, and do
**not** delete this entry until the root install banner has actually lost
`glob@7.2.3`, `inflight@1.0.6`, `rimraf@3.0.2`, `tar@6.2.1`, and the
`sockjs`-carried `uuid@8.3.2`.

## 2026-07-30 — operator-requested 19 → 21 attempt: gates cleared, artifact layer blocked

An operator-initiated attempt (target chosen as **21.x**, not 20.x, because v19 is EOL
and 20's LTS window is short) got as far as a full manifest bump and died at
`pnpm install` resolution. Every *code and toolchain* gate cleared; the blocker is
purely artifact availability. Recording the cleared gates so the next attempt is
one-shot rather than a re-derivation.

**Cleared (probed against real 20.3.x / 21.2.x package contents, not release notes):**

| Gate | Finding |
|---|---|
| Node floor | `@angular/core@21` engines `^20.19.0 \|\| ^22.12.0 \|\| >=24.0.0`; local 22.22, root `engines >=20.20`, containers `node:20-alpine` / `node:22-alpine` — all satisfy. No container rebuild needed. |
| TypeScript | `compiler-cli@21` peer `>=5.9 <6.1`, `@angular/build@21` peer `>=5.9 <6.0` → `~5.9.3`. Note `elohim-service` and `perseus-plugin` separately pin `~5.4.0` and must move too. |
| Vitest | `@angular/build@21` peers **`vitest ^4.0.8`** — matches the workspace's vitest 4 exactly (the 20.x line wanted `^3.1.1`, so 21 is the *better* target for this repo). |
| Vite | `@angular/build@21` depends on `vite 7.3.2`; workspace declares `^7.3.1`. This closes the recorded `'@angular/build>vite'` override gap and the `@vitejs/plugin-basic-ssl` unmet-peer warning without an override. |
| Builders | `@angular-devkit/build-angular@21.2.x` still publishes, so the five `angular.json` files keep resolving unchanged — the `use-application-builder` migration (step 1 above) is decoupled from the version bump and can land after. `@angular/build:ng-packagr` exists at 21, unblocking step 2. |
| Analog | `@analogjs/vite-plugin-angular@2.3.0` (installed) already declares `^20.0.0 \|\| ^21.0.0` peers — no bump strictly required; `2.3.1` is a patch. |
| SSR private-API survival | The elohim-app / lamad SSR path leans on three non-public symbols. All three survive at **21.2.14**: `ɵNoopNgZone` (`@angular/core`), `ɵREQUESTS_CONTRIBUTE_TO_STABILITY` (`@angular/common/http`), and `renderApplication` + `provideServerRendering` (`@angular/platform-server`). This was the largest silent-break candidate; it is not one. |
| Forced source migrations | v21 ships six `ng-update` migrations. Four are no-ops here (no `Router.getCurrentNavigation`, no `Router.lastSuccessfulNavigation`, no `ApplicationConfig` imported from `platform-browser`, no deprecated bootstrap options). `add-bootstrap-context-to-server-main` is already hand-implemented in both `main.server.ts` files — with `BootstrapContext` public at 21, the `any` cast + eslint-disable can be dropped. `control-flow-migration` is opt-in, not forced. |
| TestBed strictness | `THROW_ON_UNKNOWN_ELEMENTS_DEFAULT = false` at 21 — the Lit `<elohim-*>` custom elements rendered in Angular specs do **not** start erroring. |
| `@angular/build` patch | The Vitest-JIT resource-transformer defect is **not fixed upstream in 20.x or 21.x** (verified byte-identical `visitEachChild` logic in both). The patch must be re-cut, not dropped. The hunk applies at the identical anchor — replace the single line `const updatedSourceFile = ts.visitEachChild(sourceFile, visitNode, context);` in `src/tools/angular/transformers/jit-resource-transformer.js` with the `let` + manual-statement-iteration fallback from `patches/@angular__build@19.2.22.patch`; `git apply --check` confirmed clean against pristine 21.2.14. |

**The blocker — the Nexus npm mirror serves cached artifacts only.**

`pnpm install` fails at *version resolution*, before any tarball fetch, because the
mirror's cached metadata lags upstream (`@angular/cli` latest reads `21.2.12`, not
`21.2.14`; `@angular-eslint/*` reads `21.3.1`, not `21.4.0`). Pinning to the exact
versions the mirror's metadata *does* know still fails, because no Angular 20/21
tarball is cached:

```
authenticated GET https://nexus.ethosengine.com/repository/npm/…
  @angular/core@21.2.14        404      @angular/core@21.2.12        404
  @angular/core@20.3.27        404      @angular/build@21.2.12       404
  @angular/cli@21.2.12         404      @angular/ssr@21.2.12         404
  @angular-devkit/build-angular@21.2.12 404
  @angular/core@19.2.19        200   ← control: cached, serves
```

Anonymous and token-authenticated requests behave identically, so this is not an
auth gap. Uncached non-Angular probes 404 the same way (`left-pad@1.2.0`,
`is-odd@3.0.1`), while every artifact that *does* serve turns out to be already in
the cache — so the proxy's on-demand upstream fetch is broken repo-wide, not
Angular-specific. `registry.npmjs.org` is directly reachable from the build
container (`@angular/core@21.2.14` → `200`), so the wall is the mirror, not egress.

**Beware one measurement artifact** that produces false "the mirror has it" reads:
the Nexus npm endpoint sometimes returns upstream `dist.tarball` URLs
(`https://registry.npmjs.org/…`) in its metadata and sometimes its own
(`https://nexus.ethosengine.com/repository/npm/…`). Probing whatever
`npm view <pkg> dist.tarball` hands back therefore flips between `200` (npmjs
path, irrelevant to what pnpm will fetch) and `404` (the mirror path, the real
answer). Always probe the **`repository/npm/` URL explicitly** before concluding
an artifact is available. Same caution applies to the `vite@6.4.2` /
`glob@13.0.6` / `qs@6.15.2` "fetchable" reads recorded in
`VULNERABILITY_CLUSTER_00_RETRY_QUEUE.md`.

**Two unblock paths, both operator/infra decisions:**

1. **Repair the mirror's proxy remote** (root fix). A `repository/npm-proxy/`
   endpoint exists and returns metadata `200` but tarball `403`, while the
   configured `repository/npm/` group returns metadata `200` and tarball `404`
   for anything uncached — consistent with the group's proxy member being
   unable to reach or unauthorized against upstream. Fixing this unblocks the
   whole 228-finding dependency campaign, not just Angular.
2. **Point the install at `registry.npmjs.org`** — works from this container, but
   `.npmrc` `registry=` is what CI uses too, so CI would fetch through the same
   broken mirror unless it is repointed as well. The lockfile itself is
   host-agnostic (integrity hashes only, zero recorded hosts), so a lockfile
   generated against npmjs is *portable* — it just cannot be **installed**
   through a mirror that lacks the artifacts.

Until one of those lands, the manifests stay at 19.2.x. The bump was reverted
rather than left in the tree: `package.json` at 21 with `pnpm-lock.yaml` at 19
breaks `--frozen-lockfile` for CI, the pre-push gates, and every concurrent
session sharing this worktree.

**Landed from the attempt (not blocked):** the six test-setup files migrated off
the deprecated `@angular/platform-browser-dynamic/testing` entry point to
`platformBrowserTesting` / `BrowserTestingModule` from
`@angular/platform-browser/testing`. Those symbols already exist at 19.2.19, so
this is forward-pay for the upgrade that is verifiable *today*, and it closes
ledger fingerprints `7bddeaf8d471` / `80b6044081e1`. One non-obvious catch:
`platformBrowserTesting()` does **not** pull in the JIT compiler the way
`platformBrowserDynamicTesting()` did — every migrated setup needs an explicit
`import '@angular/compiler';` or specs fail with *"needs to be compiled using the
JIT compiler"* (doorway-app's 7 spec files failed exactly this way before the
import was added).

## Verification

No fix was applied this run; nothing is claimed fixed. Verified:

- **Reverse-dep trace** over `pnpm-lock.yaml` `snapshots:` — chains in the table
  above. `tar@6.2.1` has exactly one parent tree-wide (`pacote@20.0.0`) and five
  importer chains; `inflight@1.0.6` has exactly one parent tree-wide
  (`glob@7.2.3`).
- **Builder audit** of all five `angular.json` files (table above) — zero
  first-party Karma/Webpack test targets; `app/elohim-library` is the only
  `ng-packagr` consumer, ×4 library projects.
- **Upstream peer check**: `@analogjs/vite-plugin-angular@2.3.0` (and `2.3.1`,
  current `latest`) declare `@angular/build` and `@angular-devkit/build-angular`
  as **optional** peers — Analog does not pin the legacy builder set, so the
  first-party devDependency is the sole reason it is installed.
- **Registry probes, this session**: `tar` `dist-tags.latest = 7.5.13`,
  `tar-7.5.13.tgz` → `404`; `rimraf` `latest = 6.1.3`, `rimraf-6.1.3.tgz` →
  `404`; control `@anthropic-ai/sdk-0.39.0.tgz` → `200`. Two consecutive passes,
  same result — persistent, not transient.
- **Files touched this run**: this entry (new), five sibling entries, and one
  `.claude/data/deprecations.jsonl` status transition. No lockfile, no
  `angular.json`, no `package.json`, no `pnpm install`.
