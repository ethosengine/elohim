---
id: "backlog-deprecation-uuid-support-window-upgrade-unit"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "uuid@10-and-below support window — transitive dev tooling (root) + first-party production dep (sophia, campaign-uncovered)"
slug: "deprecation-uuid-support-window-upgrade-unit"
written: "2026-07-30"
author: "deprecation-triage"
status: "backlog"
priority: "low"
deprecation_status: blocked
severity: low
fingerprints: ["9b2c18a09eb7", "a200f917e702", "72a88a3bbff4", "e83cd3f2d7e3"]
relatedNodeIds:
  - "backlog-dependabot-triage"
  - "backlog-deprecation-storybook-test-runner-jest-island-retire"
  - "backlog-deprecation-angular19-toolchain-legacy-builder-transitives"
tags: [deprecation, uuid, cucumber, storybook-test-runner, nyc, jest-junit, sockjs, npm-groovy-lint, sophia, mirror-blocked, transitive]
cites:
  - https://github.com/uuidjs/uuid/blob/main/CHANGELOG.md
  - https://github.com/advisories/GHSA-w5hq-g745-h8pq
  - genesis/data/timeline/backlog/dependabot-triage.md
  - VULNERABILITY_CLUSTER_04_NODE_TOOLING_AND_ARTIFACTS.md
  - VULNERABILITY_CLUSTER_06_PNPM_LOCK_INTEGRATION.md
  - sophia/packages/sophia/package.json
  - sophia/packages/sophia/src/components/math-input.tsx
---

## What is deprecated

Four ledger fingerprints, all the **same upstream banner** emitted while resolving
the npm tree:

```
deprecated: uuid@10 and below is no longer supported.  For ESM codebases, update
to uuid@latest.  For CommonJS codebases, use uuid@11 (but be aware this version
will likely be deprecated in 2028).
```

- `9b2c18a09eb7` — captured from a `git diff pnpm-lock.yaml` during the in-flight
  npm vulnerability-remediation campaign (cluster 06 work).
- `a200f917e702`, `72a88a3bbff4` — **self-minted echoes**: emitted by this triage
  agent's own scoping command (`grep -n "uuid" pnpm-lock.yaml`), which printed the
  two `deprecated:` lines already resident in the lockfile at lines 12314 (the
  `uuid@10.0.0` block) and 12327 (the `uuid@8.3.2` block). Identical warning text,
  line-number-prefixed. They are not new debt — they are the same two lockfile
  lines the sentinel will re-mint for *any* future grep of `pnpm-lock.yaml`. This
  is precisely why the three lines must stay present-and-`blocked` rather than be
  deleted (see Current decision).
- `e83cd3f2d7e3` — **added 2026-07-30.** The root-workspace `pnpm install
  --lockfile-only` aggregate banner: `WARN 11 deprecated subdependencies found:
  expect-playwright@0.8.0, glob@7.2.3, inflight@1.0.6, jest-process-manager@0.4.0,
  node-domexception@1.0.0, prebuild-install@7.1.3, rimraf@3.0.2, tar@6.2.1,
  uuid@10.0.0, uuid@8.3.2, whatwg-encoding@2.0.0`. It names **both** uuid
  resolutions this entry owns, so it is co-canonicalized here — but it is a
  *multi-concern aggregate*, decomposed by upgrade unit across six entries (this
  one plus the five named under Current decision). Its value to this entry: it
  re-confirms, independently of any lockfile grep, that `uuid@10.0.0` and
  `uuid@8.3.2` are still resolved and still emitting.

This is a **support-window notice, not a vulnerability** — the banner says the
8.x/10.x lines are out of upstream maintenance. A *separate but co-located*
advisory exists and is handled as the same upgrade unit (below).

## Usage inventory

### Root workspace (`pnpm-lock.yaml`) — zero first-party importers

Every resolution is transitive dev tooling. Reverse-dep trace over the lockfile
snapshot section:

| Resolution | Banner? | Carrier chain |
|---|---|---|
| `uuid@10.0.0` (line 12312) | **yes** | `@cucumber/gherkin@30.0.4` / `@31.0.0` → `@cucumber/messages@26.0.1` → `uuid@10.0.0`; cucumber enters via `@cucumber/cucumber: ^11.2.0`, a `dependencies` entry of `@elohim/a2o` (`genesis/a2o/package.json`) |
| `uuid@8.3.2` (line 12325) | **yes** | four independent carriers: `@storybook/test-runner@0.24.4` (declared by `app/elohim-library/package.json`); `nyc@15.1.0` → `istanbul-lib-processinfo@2.0.3`; `jest-junit@16.0.0`; `webpack-dev-server@5.2.2` → `sockjs@0.3.24` |
| `uuid@11.0.5` | no | `@cucumber/messages@27.2.0` — already past the support window |
| `uuid@13.0.0` | no | `npm-groovy-lint@16.2.0` (`elohim-orchestrator` devDependency) |

No `package.json` in the root workspace declares `uuid`; no first-party `.ts`/`.js`
file imports it. Confirmed by manifest scan and import grep (`node_modules`,
`dist/`, and Angular `.angular/cache/` vite-dep bundles excluded — the only
in-cache hits are the string `"uuid"` inside highlight.js's N1QL builtin-function
list, not an import).

### sophia submodule (`sophia/pnpm-lock.yaml`) — one first-party **production** dep

Sophia is excluded from the root pnpm workspace (own lockfile, own `pnpm install`),
so it is a **separate resolution surface**:

- `sophia/packages/sophia/package.json:57` declares `uuid: "^10.0.0"` in
  `dependencies` (production, not dev) → resolved `10.0.0` at
  `sophia/pnpm-lock.yaml:838` under the `packages/sophia:` importer block.
- Single call site: `sophia/packages/sophia/src/components/math-input.tsx:21` —
  `import {v4 as uuid} from "uuid";`. **`v4` only**; no `v3`/`v5`/`v6` anywhere in
  sophia.
- Transitive too, all pre-window: `uuid@8.3.2` via `@cypress/request@3.0.10` and
  `istanbul-lib-processinfo@2.0.3`; `uuid@9.0.1` via
  `@storybook/addon-actions@8.6.15`.

Because sophia builds the `sophia-element` UMD bundle that ships into elohim-app,
`uuid@10.0.0` is bundled into shipped browser product code — this is genuine
first-party dependency debt, unlike the root's dev-tooling-only picture.

### Campaign-coverage gap (the finding worth escalating)

`VULNERABILITY_CLUSTER_04_NODE_TOOLING_AND_ARTIFACTS.md:65-69` records, for its
2026-07-30 handoff to cluster 06:

> `sophia/` was checked separately for all 21 families (`cd sophia && pnpm why
> --recursive <pkg>`) and **none of them appear anywhere in sophia's dependency
> graph** — sophia's own lockfile has no resolution for any cluster-04 package, so
> sophia needs no handoff entries here.

For the `uuid` family (cluster-04 alerts #640, #625) that blanket claim is
**false against the lockfile**: `sophia/pnpm-lock.yaml` carries three uuid
resolutions (`10.0.0` direct-production, `8.3.2`, `9.0.1`), all `<11.1.1` and
therefore all inside GHSA-w5hq-g745-h8pq's affected range. The likely cause is a
`pnpm why` run against an uninstalled/partial sophia `node_modules` returning empty
rather than erroring — the lockfile is the ground truth. Cluster 04's per-package
`uuid` section separately and correctly states "Direct importer(s): none", but that
holds only for the **root** workspace; sophia has one.

## Migration path

Target floor **`uuid@11.1.1`** (or `13.0.1`), which clears both the deprecation
banner and the advisory in one bump.

- **API compatibility: clean.** Per the upstream changelog, `import { v4 } from
  'uuid'` is unchanged across v11 and v13. v11 was a TypeScript port + internal v1/v7
  state refactor; v13's headline change was making browser exports the default.
  Neither breaks the named-import pattern, so sophia's single `v4` call site needs
  **no code change** — only the manifest range and lockfile move.
- **Root workspace**: no first-party manifest to edit. The four carriers pin uuid
  internally, so the only lever is a `pnpm-workspace.yaml` override (the mechanism
  cluster 06 already uses for `minimatch`/`picomatch`/`qs`). Note the 8.3.2→11.x
  hop is a major bump for those carriers; cluster 04 assessed the behavioral risk
  as nil because every carrier calls parameterless v4-style generation internally.
- **sophia**: `packages/sophia/package.json` `^10.0.0` → `^11.1.1`, then
  `pnpm install` in the submodule, then `pnpm build && pnpm build:umd` (the UMD
  bundle is a prebuild dependency of elohim-app).

### Co-located advisory (same upgrade unit, not reachable here)

GHSA-w5hq-g745-h8pq / CVE-2026-41907 — Moderate, CVSS v4.0 **6.3**. `v3()`, `v5()`,
`v6()` omit buffer-bounds validation when the caller supplies an external
`buf`/`offset`, permitting silent partial out-of-bounds writes (`v4()`/`v1()`/`v7()`
correctly throw `RangeError`). Affected `<11.1.1`, `>=12.0.0 <12.0.1`,
`>=13.0.0 <13.0.1`; patched `11.1.1` / `12.0.1` / `13.0.1` / `14.0.0`.

**Reachability: none, in either surface.** Triggering requires calling v3/v5/v6
with an attacker-influenced external buffer and offset. Repo-wide grep finds zero
first-party v3/v5/v6 call sites; sophia's lone site is `v4`; and all transitive
carriers use internal parameterless generation. This keeps `severity: low` on this
entry even though the advisory itself is Moderate. Security-class ownership of
#640/#625 stays with the campaign — this entry does not fork it.

## Current decision

**Blocked (terminal for automation) — the patched artifact is not fetchable, and
the one permitted lockfile writer is another agent.** Three independent blockers,
each sufficient on its own:

1. **Mirror-blocked — independently re-verified this run, not inherited.** Against
   `pnpm config get registry` → `https://nexus.ethosengine.com/repository/npm/`:
   the `uuid` packument returns **200**, but
   `…/uuid/-/uuid-11.1.1.tgz` → **HTTP 404** and `…/uuid-13.0.1.tgz` → **HTTP 404**.
   The mirror serves metadata it cannot serve artifacts for, so *no* manifest or
   override edit can resolve — `pnpm install` would fail at tarball fetch. This
   matches cluster 06's probe table (`uuid | 11.1.1 / 13.0.1 | 404 |
   mirror-blocked`, one of 47 mirror-blocked targets out of 49 probed). Clearing it
   is a Nexus proxy/remote-cache operator action, not a repo change.

   **Refinement 2026-07-30 — the mirror is cached-artifact-only, per-artifact, and
   `uuid@14.0.1` IS fetchable.** Re-probed on two consecutive passes:
   `uuid-11.1.1.tgz` and `uuid-13.0.1.tgz` → `404` (persistent, not transient),
   but `uuid-14.0.1.tgz` (`dist-tags.latest`) → **`200`**, and an
   already-installed control (`@anthropic-ai/sdk-0.39.0.tgz`) → `200` while its
   `latest` (`sdk-0.80.0.tgz`) → `404`. So the mirror is not uniformly broken: it
   serves whatever happens to be cached and 404s the rest. That does **not**
   unblock this entry, because the one fetchable target is not a drop-in:
   `uuid@14.0.1` is `"type": "module"` and its `exports["."].node` condition has
   **no `require` branch** (`{"node": {"types": …, "default": "./dist-node/index.js"}}`)
   — ESM-only under Node. Every carrier here is CommonJS (`@cucumber/messages`,
   `jest-junit`, `nyc → istanbul-lib-processinfo`, `sockjs`,
   `@storybook/test-runner`; and on the sophia side, a Jest CJS test path), so an
   override to `14.0.1` would break them at `require()`. The **dual-published**
   version that would actually work — `uuid@11.1.1`, whose `exports["."].node`
   carries both `import` and `require` — is precisely the one that 404s. Blocker
   stands; it is now sharper, not gone. Re-probe both on every sweep, since
   availability is per-artifact and can change without any repo action.
2. **Write-lock**: `pnpm-lock.yaml`, `pnpm-workspace.yaml`, and workspace
   `package.json`s are owned exclusively by the in-flight cluster-06 campaign owner
   (mid-edit). This triage run was explicitly scoped to touch none of them, and did
   not.
3. **Submodule boundary** (sophia only): the fix requires a commit inside the
   `sophia` git submodule, whose worktree this environment cannot even `git status`
   (`fatal: detected dubious ownership`). That is a deliberate operator-scoped
   surface, not a background-agent one.

The four ledger fingerprints stay **present with `status: blocked`** so the
sentinel cites this decision deterministically and never re-dispatches. Deleting
them would be actively harmful: two of the four were minted by nothing more than a
`grep` of the lockfile, so the next agent that greps `pnpm-lock.yaml` — or diffs it
— would re-mint them as NEW and burn another triage dispatch on an unchanged,
externally-blocked concern. The deprecation-stasis sweep owns the re-check.

`e83cd3f2d7e3` additionally requires care on close-out: it is a **shared aggregate
banner fingerprint** whose eleven packages are decomposed by upgrade unit across
six entries in `genesis/data/timeline/backlog/` — this entry plus
`deprecation-storybook-test-runner-jest-island-retire.md`,
`deprecation-angular19-toolchain-legacy-builder-transitives.md`,
`deprecation-helia-webrtc-native-addon-react-native-subtree.md`,
`deprecation-anthropic-agent-sdk-legacy-http-stack-bump.md`, and
`deprecation-first-party-glob-v10-declarations-bump.md`. Fixing the uuid unit
therefore does **not** license deleting `e83cd3f2d7e3` from the ledger; that
fingerprint retires only when the last of the six closes and the banner is
actually gone. The other three fingerprints are uuid-only and retire with this
entry.

### Live trajectory — what unblocks this, in order

1. **Nexus mirror fetch for `uuid@11.1.1`** (operator). Re-probe with the two
   `curl` calls in Verification; a 200 unblocks every lever below. This single
   blocker gates ~47 of the campaign's npm targets, so it is worth escalating as
   campaign-wide infrastructure rather than a uuid-specific errand.
2. **Hand the sophia surface to the campaign** (bounded, do-able the moment 1
   clears). Cluster 04's "sophia is clean for all 21 families" note needs
   correcting for at least `uuid`; the other 20 families should be re-checked
   against `sophia/pnpm-lock.yaml` directly rather than via `pnpm why`, since the
   same empty-result artifact would have hidden them too. **This is the highest-value
   next action in this entry** — it is a coverage hole in an in-flight campaign, not
   new work.
3. **sophia bump** `^10.0.0` → `^11.1.1` + `pnpm build && pnpm build:umd`;
   verification surface is sophia's own suite plus a math-input render (the single
   `v4` call site feeds input-field IDs).
4. **Root overrides** via `pnpm-workspace.yaml`, cluster-06-owned, validated by
   `cd genesis/a2o && pnpm test` (cucumber message-ID generation) and
   `elohim-library`'s storybook test-runner path.

Do **not** delete this entry or its fingerprints until the banner is actually gone
from both lockfiles — a lockfile-only resolution with an unfetchable tarball is the
"resolution ≠ resolved" trap cluster 06 already flagged on `#642`.

## Verification

No fix was applied this run, so nothing is claimed fixed. What *was* verified:

- **Registry probe (the load-bearing blocker), run this session:**
  `curl -o /dev/null -w "%{http_code}" https://nexus.ethosengine.com/repository/npm/uuid/-/uuid-11.1.1.tgz`
  → `404`; same for `uuid-13.0.1.tgz` → `404`; packument `/uuid` → `200`.
- **Registry re-probe, 2026-07-30** (two consecutive passes, same results):
  `uuid-11.1.1.tgz` → `404`, `uuid-13.0.1.tgz` → `404`, **`uuid-14.0.1.tgz` →
  `200`**; `uuid` `dist-tags.latest = 14.0.1`. Module-shape probe from the same
  packument: `versions["14.0.1"].exports["."] = {"node": {"types":
  "./dist/index.d.ts", "default": "./dist-node/index.js"}, "default":
  "./dist/index.js"}` with `"type": "module"` — **no `require` condition**, i.e.
  ESM-only under Node; whereas `versions["11.1.1"].exports["."].node =
  {"import": …, "require": …}` — dual. Control probes establishing the mirror's
  cached-only behaviour: `@anthropic-ai/sdk-0.39.0.tgz` (installed) → `200`,
  `@anthropic-ai/sdk-0.80.0.tgz` (latest) → `404`, `rimraf-6.1.3.tgz` → `404`,
  `tar-7.5.13.tgz` → `404`, `glob-13.0.6.tgz` → `200`.
- **Reverse-dep trace**: lockfile snapshot walk mapping all four root `uuid:`
  resolutions to owning packages (table above) — no first-party importer.
- **Manifest scan**: the only `uuid` declaration in any tracked `package.json` is
  `sophia/packages/sophia/package.json:57` (`^10.0.0`, production `dependencies`).
- **Import scan**: exactly one first-party import repo-wide —
  `sophia/packages/sophia/src/components/math-input.tsx:21`, `v4` only; zero
  `v3`/`v5`/`v6` call sites, which is what makes the co-located advisory
  unreachable.
- **Overrides scan**: root `package.json` has empty `pnpm.overrides`, `overrides`,
  and `resolutions` — no pre-existing uuid pin to reconcile.
- **Files touched this run**: this entry and three `.claude/data/deprecations.jsonl`
  status transitions. No lockfile, no `pnpm-workspace.yaml`, no `package.json`, no
  submodule content.
