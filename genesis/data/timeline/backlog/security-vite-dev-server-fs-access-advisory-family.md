---
id: "backlog-security-vite-dev-server-fs-access-advisory-family"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Vite dev-server filesystem-access advisory family (5 GHSAs) — complete remediation recipe proven, blocked only on Nexus tarball availability"
slug: "security-vite-dev-server-fs-access-advisory-family"
written: "2026-07-30"
author: "deprecation-triage"
status: "backlog"
priority: "high"
deprecation_status: blocked
severity: security
fingerprints: [8388b106c43a, 66610afd3d65, bde5206463f0, 2503c85aa1e8, bc915e697a99]
relatedNodeIds: []
tags: [security, vite, npm, dev-server, path-traversal, dependabot, nexus, sophia]
cites:
  - https://github.com/advisories/GHSA-4r4m-qw57-chr8
  - https://github.com/advisories/GHSA-356w-63v5-8wf4
  - https://github.com/advisories/GHSA-859w-5945-r5v3
  - https://github.com/advisories/GHSA-93m4-6634-74q7
  - https://github.com/advisories/GHSA-4w7w-66w2-5vf9
  - pnpm-workspace.yaml
  - pnpm-lock.yaml
  - app/elohim-app/package.json
  - app/elohim-library/package.json
  - doorway/doorway-app/package.json
  - app/elohim-elements/elohim-core/package.json
  - app/elohim-elements/elohim-imagodei/package.json
  - app/elohim-elements/elohim-qahal/package.json
  - genesis/landing/package.json
  - sophia/pnpm-workspace.yaml
  - VULNERABILITY_CLUSTER_02_JS_BUILD_AND_TEST.md
  - VULNERABILITY_CLUSTER_06_PNPM_LOCK_INTEGRATION.md
  - VULNERABILITY_CLUSTER_11_SOPHIA_SUBMODULE.md
  - genesis/data/timeline/backlog/dependabot-triage.md
---

## What is deprecated

Five GitHub advisories in one family: bypasses of Vite's dev-server filesystem
guard (`server.fs.deny` / `server.fs.strict`), each allowing an attacker to read
files the guard was supposed to withhold (`.env`, certs, source maps outside the
project root). All five are **dev-server-only** and all require that the dev
server be *explicitly exposed to the network* (`--host` / `server.host`) — which
is exactly how the Eclipse Che devspace serves `:4200`, so this is not a purely
theoretical surface here.

Verbatim ledger lines:

```
GHSA-4r4m-qw57-chr8 - Vite has a `server.fs.deny` bypassed for `inline` and `raw` with `?import` query
GHSA-356w-63v5-8wf4 - Vite has an `server.fs.deny` bypass with an invalid `request-target`
GHSA-859w-5945-r5v3 - Vite's server.fs.deny bypassed with /. for files under project root
GHSA-93m4-6634-74q7 - vite allows server.fs.deny bypass via backslash on Windows
GHSA-4w7w-66w2-5vf9 - Vite Vulnerable to Path Traversal in Optimized Deps `.map` Handling
```

**Exposure matrix** (verified against each advisory 2026-07-30). The repo holds
three distinct resolved Vite lines; no single advisory hits all three:

| Advisory | CVE | Patched at | sophia `5.4.11` | root `6.4.1` | root `7.3.1` |
|---|---|---|---|---|---|
| GHSA-4r4m-qw57-chr8 | CVE-2025-31125 | 5.4.16 · 6.0.13 · 6.1.3 · 6.2.4 | **affected** | ok | ok |
| GHSA-356w-63v5-8wf4 | CVE-2025-32395 | 5.4.18 · 6.0.15 · 6.1.5 · 6.2.6 | **affected** | ok | ok |
| GHSA-859w-5945-r5v3 | CVE-2025-46565 | 5.4.19 · 6.1.6 · 6.2.7 · 6.3.4 | **affected** | ok | ok |
| GHSA-93m4-6634-74q7 | CVE-2025-62522 | 5.4.21 · 6.4.1 · 7.0.8 · 7.1.11 | **affected** | ok | ok |
| GHSA-4w7w-66w2-5vf9 | CVE-2026-39365 | 6.4.2 · 7.3.2 · 8.0.5 | not affected | **affected** | **affected** |

Two remediation units follow from that split, and they can land independently:

- **Unit R (root workspace)** — one advisory, GHSA-4w7w-66w2-5vf9, hitting *both*
  root Vite lines. Fingerprint `bde5206463f0`.
- **Unit S (sophia submodule)** — the other four advisories, all against the
  single catalog pin `5.4.11`. Fingerprints `8388b106c43a`, `66610afd3d65`,
  `2503c85aa1e8`, `bc915e697a99`.

## Usage inventory

**Unit R — root workspace.** Seven importers declare Vite directly:

| Declaration | Files |
|---|---|
| `"vite": "^6.0.0"` → resolves `6.4.1` | `genesis/landing/package.json:13`, `app/elohim-elements/elohim-core/package.json:89`, `app/elohim-elements/elohim-imagodei/package.json:80`, `app/elohim-elements/elohim-qahal/package.json:76` |
| `"vite": "^7.3.1"` → resolves `7.3.1` | `app/elohim-app/package.json:140`, `app/elohim-library/package.json:75`, `doorway/doorway-app/package.json:40` |

**The eighth carrier is the one the existing campaign plan misses, and it is the
most important one.** `@angular/build@19.2.22` declares `vite` as a **regular
dependency** (not a peer — its `peerDependencies` block in `pnpm-lock.yaml`
lists `@angular/compiler`, `less`, `postcss`, `typescript`, … and *no* `vite`),
pinned at `6.4.1` across five peer-context variations. That instance is the one
`ng serve` actually runs — i.e. the dev server Che exposes. It is reached by
`app/elohim-app`, `doorway/doorway-app`, `app/lamad`, and
`app/imagodei-portal`; the last two declare no Vite of their own at all, so a
declaration-only edit cannot move them.

Consequence: the remediation recorded in
`VULNERABILITY_CLUSTER_02_JS_BUILD_AND_TEST.md` ("change only the four `^6.0.0`
declarations to `^6.4.2`" / "only the three `^7.3.1` declarations to `^7.3.2`")
is **incomplete** — on success it would still leave `vite@6.4.1` resolved under
`@angular/build`, and the Vite 6 alerts would have been marked resolved against
a still-vulnerable dev server. Verified by doing it: after bumping all seven
declarations, `pnpm-lock.yaml` still held 13 references to `vite@6.4.1`.

**Unit S — sophia submodule.** One catalog pin, `sophia/pnpm-workspace.yaml:97`
(`vite: 5.4.11` under `catalogs.devDeps`), consumed by `sophia/package.json:134`
(`"vite": "catalog:devDeps"`). Sophia is a git submodule with its own pnpm
workspace and its own `sophia/pnpm-lock.yaml`; it is *not* a root-workspace
importer, so root lockfile work can never move it and must never try.

## Migration path

Both units are patch-level moves inside the existing major line — no API change,
no major upgrade, no source edits.

**Unit R.** Do *not* use `pnpm update vite -r`: the wide re-resolution it forces
walks `react-native@0.84.1 → babel-jest → @babel/core@7.29.7`, which requires
`@babel/helpers@^7.29.7` while the mirror tops out at `7.29.2`, aborting with
`ERR_PNPM_NO_MATCHING_VERSION`. (This is the same class of false blocker recorded
in cluster 02 for `@types/estree@1.0.9` — the wide re-resolve is the cause, not
the target version.) Instead, edit declarations and let plain `pnpm install`
re-resolve only the Vite subtree:

1. Bump the four `^6.0.0` declarations to `^6.4.2` and the three `^7.3.1`
   declarations to `^7.3.2` (patched floors; `^` lets the lock float to the
   current heads).
2. Add the dependency-path-scoped override to `pnpm-workspace.yaml` `overrides:`,
   matching the existing `'@angular/build>picomatch': ^4.0.4` precedent — this is
   what moves the `ng serve` instance, and it is scoped so the `^7` surfaces
   (vitest, storybook) are untouched:

   ```yaml
   '@angular/build>vite': ^6.4.2
   ```
3. `pnpm install --lockfile-only`, then `pnpm install`.

**This recipe is proven to resolve cleanly and completely** (run 2026-07-30):
`pnpm install --lockfile-only` exits 0 in ~33s, the lock lands `vite@6.4.3` and
`vite@7.3.6`, and references to the vulnerable versions drop to **zero** —
`grep -c 'vite@6\.4\.1' pnpm-lock.yaml` → `0`, `grep -c 'vite@7\.3\.1'` → `0`.
The resulting diff is surgical: 285 lock lines, entirely Vite specifiers plus the
peer-context hashes that embed Vite's version; no unrelated package moves.

**Unit S.** Change `sophia/pnpm-workspace.yaml` `catalogs.devDeps.vite` from
`5.4.11` to `5.4.21` (the single 5.x release that clears all four advisories —
the highest patched floor in the family is `5.4.21` for CVE-2025-62522), then
regenerate `sophia/pnpm-lock.yaml` from inside `sophia/`. Never fold this into
root lockfile work.

## Current decision

**Blocked — on artifact availability in the Nexus npm proxy, not on code.** The
remediation above is complete, correct, and proven to resolve; it cannot be
*installed*. `pnpm install` fails at fetch time:

```
ERR_PNPM_FETCH_404  GET https://nexus.ethosengine.com/repository/npm/vite/-/vite-7.3.6.tgz: Not Found - 404
```

Independently confirmed by direct tarball probes (anonymous `curl`, which is the
working path — the `_authToken` in `.npmrc` is stale and returns 401):

| Tarball | Result |
|---|---|
| `vite-6.4.1.tgz`, `vite-7.3.1.tgz` (the **vulnerable** versions, in the lock) | **200** |
| `vite-6.4.2`, `6.4.3`, `7.3.2`, `7.3.3`, `7.3.5`, `7.3.6`, `5.4.19`, `5.4.21` | **404** |

The controlling generalization — every version already resident in
`pnpm-lock.yaml` fetches, every version not resident 404s regardless of what the
packument advertises:

| Probe | Result |
|---|---|
| `esbuild-0.25.4`, `esbuild-0.25.12`, `esbuild-0.27.3`, `rollup-4.59.0` (all in lock) | **200** |
| `esbuild-0.28.0`, `rollup-4.61.0` (not in lock, both published) | **404** |

So Nexus's npm proxy still proxies *metadata* (`npm view vite@7.3.2` resolves,
which is why resolution succeeds) but no longer fetches *tarballs* from upstream
— it serves its existing blob cache only. That independently reproduces the
critical finding already recorded in
`VULNERABILITY_CLUSTER_06_PNPM_LOCK_INTEGRATION.md` ("Nexus's npm-proxy cannot
fetch new tarballs right now"), and it supersedes the "mirror hasn't synced past
version X" readings in cluster 02 — the mirror's *metadata* is current; its
*blob store* is frozen. This blocker is operator-side on Nexus and is not
fixable from any manifest, lockfile, or override in this repo. It gates every
npm security bump in the monorepo, not just Vite.

The provisional edits were therefore **reverted** rather than committed: landing
a lockfile that pins un-fetchable tarballs would break `pnpm install` for every
developer and every CI pipeline. `pnpm install` was re-run after the revert and
is green (exit 0, "Lockfile is up to date"), so the worktree is left healthy.
`sophia/pnpm-lock.yaml` was never touched (owned by the concurrent cluster-11
session), and Unit S carries a second hold on that coordination besides the
mirror.

**Unblock condition:** operator restores upstream tarball fetch on Nexus's
`npm-proxy` repository (or a temporary registry override is approved). Re-check
by probing `https://nexus.ethosengine.com/repository/npm/vite/-/vite-6.4.3.tgz`
for a 200; the moment that flips, Unit R is a ~10-minute landing with the recipe
above verbatim.

## Verification

Not yet fixed, so nothing is claimed closed. What *was* proven this run:

- Advisory-to-version exposure matrix checked against all five GHSA pages
  individually — this is what split the family into Units R and S and showed
  sophia's `5.4.11` is **not** affected by CVE-2026-39365.
- `@angular/build@19.2.22` confirmed to carry `vite` as a regular dependency at
  `6.4.1` (five peer-context variations) by reading its `pnpm-lock.yaml`
  `peerDependencies` block and its snapshot dependency entries — the gap in the
  cluster-02 plan.
- Unit R recipe resolves clean: `pnpm install --lockfile-only` exit 0; lock holds
  only `vite@6.4.3` / `7.3.6`; zero references to `vite@6.4.1` or `vite@7.3.1`.
- Blocker reproduced two ways (pnpm `ERR_PNPM_FETCH_404`, and direct tarball
  probes with the in-lock/not-in-lock control pair above).
- Revert verified byte-clean against `HEAD` (`git status --short` empty for all
  nine touched files) and `pnpm install` green afterward.

**Gates still owed when Unit R lands** (none were run — the install could not
complete, so running them would have proved nothing): `pnpm --filter elohim-app
test`, `pnpm --filter elohim-app run build`, `pnpm --filter elohim-library test`,
and one Vite-6 surface smoke (`pnpm --filter @elohim/protocol-landing run
build`) to cover both major lines. Quote their results in the closing commit.
