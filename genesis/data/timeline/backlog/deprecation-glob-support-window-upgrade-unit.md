---
id: "backlog-deprecation-glob-support-window-upgrade-unit"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "glob@10-and-below support window — two first-party direct deps (elohim-service, elohim-content MCP) plus transitive 7.x carriers"
slug: "deprecation-glob-support-window-upgrade-unit"
written: "2026-07-30"
author: "deprecation-triage"
status: "backlog"
priority: "medium"
deprecation_status: blocked
severity: low
fingerprints: ["c8e23effd89f", "9d796424bd3a"]
relatedNodeIds:
  - "backlog-deprecation-uuid-support-window-upgrade-unit"
  - "backlog-dependabot-triage"
tags: [deprecation, glob, minimatch, elohim-service, elohim-content-mcp, jest, karma, nyc, rimraf, transitive, write-lock-blocked]
cites:
  - https://github.com/isaacs/node-glob/blob/main/changelog.md
  - app/elohim-library/projects/elohim-service/package.json
  - app/elohim-library/projects/elohim-service/src/services/import-pipeline.service.ts
  - elohim/elohim-agent/mcp-servers/elohim-content/package.json
  - genesis/data/timeline/backlog/deprecation-uuid-support-window-upgrade-unit.md
  - VULNERABILITY_CLUSTER_04_NODE_TOOLING_AND_ARTIFACTS.md
---

## What is deprecated

Two ledger fingerprints, one upstream banner. `glob` marks **every version below
12.0.0 deprecated** — including the whole 10.x line and, notably, 11.x too:

```
deprecated: Old versions of glob are not supported, and contain widely publicized
security vulnerabilities, which have been fixed in the current version. Please
update. Support for old versions may be purchased (at exorbitant rates) by
contacting i@izs.me
```

- `c8e23effd89f` — captured from a `pnpm` resolution banner during the in-flight
  vite security bump: `.../projects/elohim-service │ WARN deprecated glob@10.5.0`.
  pnpm attributes it to the **first-party importer**, which is what makes this
  actionable rather than carrier-pinned.
- `9d796424bd3a` — **self-minted echo**: emitted by this triage agent's own scoping
  command (a `python3`/`re` read of the `glob@10.5.0` snapshot block in
  `pnpm-lock.yaml`), which printed the `deprecated:` line already resident in the
  lockfile. Same banner text, no new debt. Like the uuid entry's echoes, this line
  will be re-minted by *any* future grep or diff of `pnpm-lock.yaml`, which is
  precisely why both fingerprints must stay present-and-`blocked` rather than be
  deleted.

This is a **support-window notice, not a live advisory**. Verified this run: the
npm registry carries **no deprecation on 12.x or 13.x**, and the "widely publicized
security vulnerabilities" the banner alludes to are the `minimatch` ReDoS family
(GHSA-23c5-xmqv-rm74 and relatives) reached *through* glob — and the root lockfile
already resolves `minimatch` at `3.1.5 / 9.0.9 / 10.2.3 / 10.2.4`, every one at or
above its line's fix floor (the vulnerability campaign's cluster-04/06 overrides did
that work). So there is **no reachable vulnerability behind this banner today**;
hence `severity: low` despite the alarming wording. What remains is genuine
first-party dependency debt: two production manifests pinned to an unsupported line.

## Usage inventory

### First-party direct declarations — 2 manifests, 6 call sites

| Manifest | Declared | Resolved |
|---|---|---|
| `app/elohim-library/projects/elohim-service/package.json:25` (`dependencies`) | `glob: ^10.3.0` | `10.5.0` (`pnpm-lock.yaml:782-784`) |
| `elohim/elohim-agent/mcp-servers/elohim-content/package.json:15` (`dependencies`) | `glob: ^10.3.0` | `10.5.0` (`pnpm-lock.yaml:1072-1074`) |

Both are `dependencies` (production), not dev. Repo-wide manifest scan confirms
these are the **only** two declarations; no other workspace package and no
`pnpm-workspace.yaml` override touches `glob`.

Every call site uses the same two-argument async form and only the `cwd` / `nodir`
options:

- `app/elohim-library/projects/elohim-service/src/services/import-pipeline.service.ts:15,282`
  — `import { glob } from 'glob'` / `await glob(pattern, { nodir: true })`
- `elohim/elohim-agent/mcp-servers/elohim-content/src/tools/assessment-tools.ts:9,59`
  — `await glob('**/*.json', { cwd: conceptsPath, nodir: true })`
- `elohim/elohim-agent/mcp-servers/elohim-content/src/tools/graph-tools.ts:9,43` — same shape
- `elohim/elohim-agent/mcp-servers/elohim-content/src/tools/path-tools.ts:10,72` — same shape
- `elohim/elohim-agent/mcp-servers/elohim-content/src/tools/seed-tools.ts:9,45` — `{ cwd: dataDir, nodir: true }`
- `elohim/elohim-agent/mcp-servers/elohim-content/src/tools/source-tools.ts:10,70` — `{ cwd: docsDir, nodir: true }`

No `globSync`, no `Glob` class, no `hasMagic`, no CLI invocation (`glob` as a shell
command appears nowhere in `scripts`, `*.sh`, or `*.mjs`), no `--shell` usage.

### Dead-weight companion

`app/elohim-library/projects/elohim-service/package.json:50` declares
`@types/glob: ^8.1.0` in `devDependencies`. It is **inert**: glob ships its own
declarations with a root `types` field (`./dist/commonjs/index.d.ts`, present on
both 10.5.0 and 13.0.6), so TypeScript's `moduleResolution: node` resolver — the
one this package uses (`tsconfig.json`) — consults `node_modules/glob` and never
falls through to `@types/`. Upstream agrees: `@types/glob@9.0.0` is itself
deprecated as *"a stub types definition. glob provides its own type definitions,
so you do not need this installed."* It should be dropped in the same commit.

### Transitive resolutions — separate upgrade unit, carrier-pinned

`pnpm-lock.yaml` holds three glob resolutions. Reverse-dep trace over the snapshots
section:

| Resolution | Banner? | Carriers |
|---|---|---|
| `glob@10.5.0` | **yes** | the 2 first-party importers above, **plus** `@cucumber/cucumber@11.3.0`, `@jest/reporters@30.4.1`, `jest-config@30.4.2`, `jest-runtime@30.4.2`, `@npmcli/package-json@6.2.0`, `cacache@19.0.1` |
| `glob@7.2.3` | **yes** (worst line) | `@jest/reporters@29.7.0`, `jest-config@29.7.0` (×2 peer variants), `jest-runtime@29.7.0`, `karma@6.4.4`, `nyc@15.1.0`, `rimraf@3.0.2`, `test-exclude@6.0.0` — all dev tooling |
| `glob@13.0.6` | no | `npm-groovy-lint@16.2.0` (already on the supported line) |

The first-party bump therefore clears the *attributed* banner line but does **not**
remove `glob@10.5.0` from the tree — six transitive carriers pin it, and eight more
pin `7.2.3`. Those are a distinct upgrade unit (carrier-pinned; the only lever is a
`pnpm-workspace.yaml` override, campaign-owned) and are explicitly out of scope for
the bounded first-party fix. Cluster 04 already tracks these same packages as
`minimatch` carriers (`VULNERABILITY_CLUSTER_04_NODE_TOOLING_AND_ARTIFACTS.md:105,183`)
— this entry does not fork that ownership.

`sophia/pnpm-lock.yaml` (separate resolution surface) carries `glob@10.5.0`,
`8.1.0`, `7.2.3` with **zero direct declarations** in any `sophia/packages/*/package.json`
— entirely transitive, and owned by cluster 11's mirror-blocked set.

## Migration path

**Target `glob@13.0.6`** (current latest; `12.x` and `13.x` are the only
non-deprecated lines).

API delta 10 → 13 is **nil for this repo**, verified two ways:

1. Upstream changelog: v11 dropped Node <20; v12 removed the `--shell` CLI option;
   v13 moved the CLI out to a separate `glob-bin` package. The async
   `glob(pattern, options)` named export and the `cwd`/`nodir`/`ignore`/`absolute`
   options are unchanged across 11–13.
2. Against the **locally installed** `glob@13.0.6` declarations
   (`node_modules/.pnpm/glob@13.0.6/node_modules/glob/dist/commonjs/`): `glob` is
   still a named export (`export declare const glob: typeof glob_ & {…}`) whose
   base overload is `(pattern: string | string[], options?: GlobOptionsWithFileTypesUnset) => Promise<string[]>`,
   and `nodir` / `cwd` / `ignore` / `absolute` / `realpath` / `dot` all remain on
   `GlobOptions`. All six call sites type-check against that shape unchanged.

Module-shape and engine checks also clear:

- v13 is **dual CJS/ESM** (`exports` carries a `require` condition; `main` →
  `dist/commonjs/index.min.js`) and keeps a root `types` field — so it works for
  `elohim-service` (`"module": "commonjs"`, legacy `moduleResolution: node`) *and*
  for `elohim-content-mcp` (`"type": "module"`).
- v13 engines are `18 || 20 || >=22`; the repo declares `node >=20.20` and the dev
  container runs v22.22.2.
- v13's dep set shrinks to `minipass ^7.1.3 / minimatch ^10.2.2 / path-scurry ^2.0.2`
  (the CLI's `jackspeak` / `foreground-child` / `package-json-from-dist` are gone),
  and `minimatch ^10.2.2` dedupes onto the already-resolved, already-patched
  `10.2.4` — a small line-consolidation win, retiring one `minimatch@9.x` carrier.

### The exact change

1. `app/elohim-library/projects/elohim-service/package.json` — `"glob": "^10.3.0"` → `"^13.0.6"`; **delete** `"@types/glob": "^8.1.0"` from `devDependencies`.
2. `elohim/elohim-agent/mcp-servers/elohim-content/package.json` — `"glob": "^10.3.0"` → `"^13.0.6"`.
3. `pnpm install` at the repo root (lockfile update; no override needed).
4. No source edits.

**Registry caveat that constrains the range.** The configured registry is the Nexus
mirror `https://nexus.ethosengine.com/repository/npm/`, which serves only artifacts
it has already cached. Probed this run: `glob-13.0.6.tgz` → **HTTP 200** (it is
cached — `npm-groovy-lint` already pulled it, and it is present in
`node_modules/.pnpm`), while `glob-12.0.0.tgz` → **HTTP 404**. So `13.0.6` is the
one installable non-deprecated target; a future `13.0.7` would 404 until cached.
The committed lockfile pins the resolution, so CI's `--frozen-lockfile` path is
unaffected — but re-resolving with a floating range on a cold mirror can fail. This
is the *opposite* of the uuid entry's situation, where the patched tarball was
unfetchable and no manifest edit could resolve: **here the fix is fully fetchable.**

## Current decision

**Blocked (terminal for automation) — dependency major-version bump, plus the
lockfile writer is held by concurrent runs.** Two independent blockers:

1. **Scale gate.** `glob@10 → 13` is a dependency *major*-version change across two
   production manifests. Per the triage agent's hard rules, that class stops at
   `blocked` with a written plan sketch rather than being landed by a background
   agent — it wants an operator-initiated, gate-verified change even though the API
   analysis above says the blast radius is nil.
2. **Write-lock.** `pnpm-lock.yaml` and `pnpm-workspace.yaml` were owned by a
   concurrent deprecation-triage run (in-flight vite security bump) and
   `sophia/pnpm-lock.yaml` by a third session at the time of triage; this run was
   explicitly scoped to touch none of them and to run no `pnpm install`/`pnpm update`
   against the root workspace, and did not. The manifest edit is worthless without
   the matching lock update — a manifest-only bump would leave the lock out of sync
   and break CI's `pnpm install --frozen-lockfile`, which is the "half-applied
   migration" anti-pattern.

Both fingerprints stay **present with `status: blocked`** so the sentinel cites this
decision deterministically and never re-dispatches. Deleting them would be actively
harmful: `9d796424bd3a` was minted by nothing more than reading `pnpm-lock.yaml`, so
the next agent to grep or diff that file would re-mint it as NEW and burn another
triage dispatch on an unchanged concern. The deprecation-stasis sweep owns re-checks.

### Live trajectory — one bounded commit, ready to execute

Unlike the uuid entry, nothing external gates this: the target tarball is fetchable,
the API is verified compatible, no source changes are needed, and no override is
required. It needs only the lockfile write-lock to clear.

1. **Wait for the lock owner to land** (the vite bump / cluster-06 campaign). Confirm
   with `git status --short pnpm-lock.yaml pnpm-workspace.yaml`.
2. **Apply the four steps** under "The exact change" above.
3. **Verify — all four must be green, and quoted in the closing commit message:**
   - `pnpm --filter @elohim/service test` (vitest; note
     `src/services/import-pipeline.service.spec.ts:22` does `vi.mock('glob')`, so
     the unit suite proves the *import shape* resolves but not real traversal)
   - `pnpm --filter @elohim/service build` (tsc under `moduleResolution: node` —
     this is the real type-resolution proof, and the check that catches a missing
     root `types` field)
   - `pnpm --filter elohim-content-mcp build` (tsc, ESM side)
   - one **real traversal** smoke, since the only unit test mocks glob: run an
     import/scan against `genesis/docs` (e.g. the `import` CLI script or the MCP
     `list_docs` tool) and confirm a non-empty, `nodir`-respecting file list —
     `nodir: true` behavior is what the code depends on
   - banner-gone proof: `pnpm install` output carries no `deprecated glob@` line
     attributed to either first-party importer, and `grep -c 'glob@10.5.0'` on the
     lockfile drops by the two importer blocks
4. **Then close out**: delete both ledger lines and delete this entry — but only
   once the banner is actually gone for the first-party importers. Do **not** delete
   on a manifest-only edit.
5. **Do not** attempt the transitive `glob@7.2.3` / `glob@10.5.0` carrier removal in
   the same commit — that is override work on eight jest-29/karma/nyc/rimraf
   carriers and belongs to the campaign's cluster-04 surface.

## Verification

Nothing fixed yet — no verification to record. Evidence gathered this run (all
read-only; no workspace file was modified):

- Registry deprecation surface: `glob` `dist-tags` = `latest: 13.0.6`,
  `legacy-v10: 10.5.0`; `10.3.10` / `10.4.5` / `10.5.0` / `11.0.0` / `11.0.3` all
  carry the banner; `12.0.0` and `13.0.0`–`13.0.6` carry none.
- Mirror fetchability: `glob-13.0.6.tgz` → 200, `glob-12.0.0.tgz` → 404 against
  `https://nexus.ethosengine.com/repository/npm/`.
- API compatibility read directly off the installed
  `node_modules/.pnpm/glob@13.0.6/.../dist/commonjs/{index,glob}.d.ts`.
- Lockfile reverse-dep trace for all three glob resolutions, and confirmation that
  every resolved `minimatch` (`3.1.5`, `9.0.9`, `10.2.3`, `10.2.4`) is at or above
  its fix floor — the reason this entry is `severity: low`.
