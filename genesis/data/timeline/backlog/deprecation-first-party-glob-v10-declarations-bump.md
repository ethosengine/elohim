---
id: "backlog-deprecation-first-party-glob-v10-declarations-bump"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "First-party glob@^10.3.0 declarations (elohim-service, elohim-content MCP) — deprecated major, fetchable API-compatible target at glob@13"
slug: "deprecation-first-party-glob-v10-declarations-bump"
written: "2026-07-30"
author: "deprecation-triage"
status: "backlog"
priority: "high"
deprecation_status: open
severity: low
fingerprints: ["e83cd3f2d7e3", "45025802800f"]
relatedNodeIds:
  - "backlog-deprecation-anthropic-agent-sdk-legacy-http-stack-bump"
  - "backlog-deprecation-angular19-toolchain-legacy-builder-transitives"
tags: [deprecation, glob, types-glob, elohim-service, elohim-content-mcp, first-party, bounded-fix, write-locked]
cites:
  - https://github.com/isaacs/node-glob/blob/main/changelog.md
  - app/elohim-library/projects/elohim-service/package.json
  - app/elohim-library/projects/elohim-service/src/services/import-pipeline.service.ts
  - elohim/elohim-agent/mcp-servers/elohim-content/package.json
  - elohim/elohim-agent/mcp-servers/elohim-content/src/tools/seed-tools.ts
---

## What is deprecated

`glob@10.5.0`, resolved from **two first-party `^10.3.0` declarations**. Verbatim
registry/lockfile `deprecated:` field (identical text to the `glob@7.2.3` line in
the banner — it is izs's blanket notice for every pre-current glob major):

```
glob@10.5.0
    Old versions of glob are not supported, and contain widely publicized
    security vulnerabilities, which have been fixed in the current version.
    Please update. Support for old versions may be purchased (at exorbitant
    rates) by contacting i@izs.me
```

`glob@11.1.0` — the newest 11.x — carries the **same** deprecation notice, so the
support window has moved past the 11 line as well.

**Provenance note.** This is an **adjacent finding surfaced while scoping
fingerprint `e83cd3f2d7e3`**, not one of that banner's eleven lines (the banner
names `glob@7.2.3`, a transitive of the Angular/Storybook units). The lockfile
carries a `deprecated:` field for `glob@10.5.0` that the captured banner did not
list. Its own banner line will mint a distinct fingerprint on the next root
install; add it to this entry's `fingerprints` when it does. The aggregate
fingerprint is listed here so the sentinel's deterministic citation for the banner
reaches the full six-entry decomposition.

The message wording ("widely publicized security vulnerabilities") is blanket
boilerplate applied by the maintainer to every superseded major — **no specific
advisory against `glob@10.5.0` was identified this run**, which is why this entry
is `severity: low` despite `priority: high`. Do not escalate it to the security
class on the strength of the notice text alone.

## Usage inventory

Two first-party manifests declare `glob` directly — the *only* direct `glob`
declarations in the root workspace:

| Manifest | Declaration | Module system |
|---|---|---|
| `app/elohim-library/projects/elohim-service/package.json` | `dependencies: { "glob": "^10.3.0" }` + `devDependencies: { "@types/glob": "^8.1.0" }` | CommonJS (`tsconfig.json`: `"module": "commonjs"`, `"moduleResolution": "node"`; no `"type"` field; `main: dist/index.js`, `bin.elohim: ./dist/cli/index.js`) |
| `elohim/elohim-agent/mcp-servers/elohim-content/package.json` | `dependencies: { "glob": "^10.3.0" }` | ESM (`"type": "module"`, run via `tsx`) |

Call sites — **eight, all using the same modern named async API** (`import
{ glob } from 'glob'`, i.e. the v9+ promise API; zero callback-style or
`new Glob()` use, zero default-import use):

- `app/elohim-library/projects/elohim-service/src/services/import-pipeline.service.ts:15`
- `app/elohim-library/projects/elohim-service/src/services/import-pipeline.service.spec.ts:9`
- `elohim/elohim-agent/mcp-servers/elohim-content/src/tools/assessment-tools.ts:9`
- `elohim/elohim-agent/mcp-servers/elohim-content/src/tools/path-tools.ts:10`
- `elohim/elohim-agent/mcp-servers/elohim-content/src/tools/seed-tools.ts:9`
- `elohim/elohim-agent/mcp-servers/elohim-content/src/tools/graph-tools.ts:9`
- `elohim/elohim-agent/mcp-servers/elohim-content/src/tools/source-tools.ts:10`
- `elohim/elohim-agent/mcp-servers/elohim-content/src/tools/graph-tools.ts` (the
  `dist/` hit at `elohim-service/dist/services/import-pipeline.service.js:39`,
  `require("glob")`, is build output — evidence of the CJS emit, not a source
  site)

`glob@10.5.0` is *also* reached transitively (`@cucumber/cucumber@11.3.0`,
`@jest/reporters@30.4.1`, `jest-config@30.4.2`, `jest-runtime@30.4.2`,
`@npmcli/package-json@6.2.0`, `cacache@19.0.1`) — so the resolution will not
vanish from the tree when the two manifests move. What clears is the
**first-party declaration of a deprecated major**, which is the part this repo
actually owns.

## Migration path

**`^10.3.0` → `^13.0.0` in both manifests, and delete
`@types/glob` from `elohim-service`'s devDependencies. No source change.**

Every claim below was probed against the configured registry this run, not
assumed:

- **Target is fetchable.** `glob` `dist-tags.latest = 13.0.6`, not deprecated, and
  `…/glob/-/glob-13.0.6.tgz` → **HTTP 200** on the Nexus mirror. This is the
  **only** unit in the entire `e83cd3f2d7e3` decomposition whose remediation
  artifact is actually available (`uuid-11.1.1`, `rimraf-6.1.3`, `tar-7.5.13`,
  `sdk-0.80.0`, `glob-11.1.1`, `glob-12.0.0` are all `404`).
- **Both consumers can consume it.** `glob@13.0.6` is `"type": "module"` but
  **dual-published**: its `.` export declares both `import` *and* `require`
  conditions (`./dist/esm/index.min.js` / `./dist/commonjs/index.min.js`), and it
  still ships classic `main: ./dist/commonjs/index.min.js` + `types:
  ./dist/commonjs/index.d.ts` fields — so `elohim-service`'s CJS emit with
  `moduleResolution: "node"` resolves both the runtime and the types. **Do not
  target `glob@11` or `glob@12`**: 11.1.0 is itself deprecated and neither
  tarball is fetchable.
- **API is unchanged for these call sites.** The named async `glob()` export is
  stable from v9 through v13; all eight sites use exactly that.
- **Engines fit.** `glob@13.0.6` requires node `18 || 20 || >=22`; the repo
  declares `engines.node: ">=20.20"` and the container runs `v22.22.2`.
- **Dependency surface shrinks.** `glob@11.1.0` needs six deps (`minipass`,
  `jackspeak`, `minimatch`, `path-scurry`, `foreground-child`,
  `package-json-from-dist`); `glob@13.0.6` needs three (`minimatch@^10.2.2`,
  `minipass@^7.1.3`, `path-scurry@^2.0.2`). `minimatch@10` is already in the tree
  (the root `overrides` pin `eslint-plugin-sonarjs>minimatch: ^10.2.4`), so this
  adds nothing new.
- **`@types/glob` must go.** `@types/glob`'s own `latest` (9.0.0) is deprecated
  with *"This is a stub types definition. glob provides its own type definitions,
  so you do not need this installed."* The pinned `^8.1.0` is not itself flagged,
  but it describes glob **v8**'s API while the code uses the v9+ promise API — a
  stale ambient typing shadowing the real one. Removing it is part of the fix, not
  a separate cleanup.

## Current decision

**Blocked on the lockfile write-lock alone — no upstream blocker, no mirror
blocker. This is the first thing to land when the lock clears.**
*(Re-verified 2026-08-06 and still exactly true — see the dated update below.
`deprecation_status` moved `blocked` → `open`: the only gate is a transient
write-lock held by another agent, which is not an external blocker.)*

`pnpm-lock.yaml` and `pnpm-workspace.yaml` are owned exclusively by concurrent
in-flight runs this session, and this triage was explicitly scoped to touch
neither (and to run no `pnpm install`/`pnpm update` against the root workspace).
Editing the two `package.json` files without the matching lockfile re-resolution
would strand CI on `--frozen-lockfile` — a half-applied migration, which the
close-out discipline forbids. So the work is *ready*, not *done*, and it is
recorded here rather than half-landed.

Note the ambient constraint that does **not** block this fix but shapes how to
apply it: a wide `pnpm update` cannot run in this repo at all — the
`react-native@0.84.1` → `babel-jest` → `@babel/core@^7.29.7` requirement exceeds
the mirror's `@babel/helpers@7.29.2` ceiling, so any broad re-resolution dies with
`ERR_PNPM_NO_MATCHING_VERSION` (see
`deprecation-helia-webrtc-native-addon-react-native-subtree.md`). **Use a targeted
install, never `pnpm update`.**

Fingerprint `e83cd3f2d7e3` stays **present with `status: blocked`**. It is a
**shared aggregate banner fingerprint** decomposed across six sibling entries in
`genesis/data/timeline/backlog/`: this entry,
`deprecation-storybook-test-runner-jest-island-retire.md`,
`deprecation-angular19-toolchain-legacy-builder-transitives.md`,
`deprecation-helia-webrtc-native-addon-react-native-subtree.md`,
`deprecation-anthropic-agent-sdk-legacy-http-stack-bump.md`, and
`deprecation-uuid-support-window-upgrade-unit.md`.

### Live trajectory — ready-to-execute, one sitting

1. **Wait for the lockfile write-lock to clear** (the concurrent runs' campaign
   commits land). Confirm with `git status -- pnpm-lock.yaml pnpm-workspace.yaml`
   showing clean, or coordinate with the owning session.
2. **Edit two manifests**: `glob: "^10.3.0"` → `"^13.0.0"` in
   `app/elohim-library/projects/elohim-service/package.json` and
   `elohim/elohim-agent/mcp-servers/elohim-content/package.json`; delete
   `"@types/glob": "^8.1.0"` from the former's devDependencies.
3. **Targeted re-resolve** (not `pnpm update`): `pnpm install --lockfile-only`
   scoped to the change, then `pnpm install`.
4. **Verify**: `cd app/elohim-library/projects/elohim-service && pnpm build`
   (`tsc` — this is the load-bearing check, since it proves the CJS + node10
   type resolution against glob@13 and catches any `@types/glob` removal
   fallout) `&& pnpm test` (Vitest, covers
   `import-pipeline.service.spec.ts`); then `cd
   elohim/elohim-agent/mcp-servers/elohim-content && pnpm exec tsc --noEmit`.
   Spot-check one MCP glob tool actually enumerates files (`seed-tools` or
   `source-tools`).
5. **Close out with full decomposition**: on green, delete this entry and quote
   the verification in the `chore(deprecation):` commit message. Do **not** delete
   `e83cd3f2d7e3` from the ledger — the aggregate banner still carries its other
   packages; the fingerprint retires only when the last sibling closes.

## 2026-08-06 — re-triage on fingerprint `45025802800f`: still ready, two ambient constraints now void

A fresh workspace install re-emitted the same warning against the MCP-server
importer (`.../mcp-servers/elohim-content | WARN deprecated glob@10.5.0`), minting
fingerprint `45025802800f`. Re-scoped rather than re-derived. **The plan above is
unchanged and still correct** — but two of its recorded ambient constraints have
expired, and one new one applies:

| Recorded constraint | Status on 2026-08-06 |
|---|---|
| "the mirror's `@babel/helpers@7.29.2` ceiling makes a wide `pnpm update` die with `ERR_PNPM_NO_MATCHING_VERSION`" | **Void.** That ceiling was a *Nexus mirror* artifact. Commit `ecc65384f` (2026-07-30, "reserve Nexus for first-party components; consume crates.io + npmjs direct") repointed `.npmrc` `registry=` to `https://registry.npmjs.org/`. The targeted-install advice is still *good hygiene*, but it is no longer *forced*. |
| container `node -v` → `v22.22.2` | Now **`v24.18.1`**. Still inside `glob@13.0.6` engines (`18 \|\| 20 \|\| >=22`). |
| — | **New:** at this triage `pnpm-lock.yaml` was dirty with a hand-patched `@automerge/automerge` bump owned by a concurrent lane. `pnpm install` would have normalised that hand-patch away. Untouched. |

Re-probed this run against the **now-current** registry (`registry.npmjs.org`):
`glob@13.0.6` `dist.tarball` → **HTTP 200**, and `npm view glob@13.0.6 deprecated`
returns **empty** (13 carries no notice) while `glob@11.1.0` still returns izs's
blanket notice. Call-sites re-confirmed unchanged: all ten first-party import
sites are `import { glob } from 'glob'` with `await glob(pattern, {cwd, nodir})` —
the v9+ signature, stable through v13.

`45025802800f` joins `e83cd3f2d7e3` on this entry with ledger `status: triaged`.

## Verification

No fix was applied this run; nothing is claimed fixed. Verified:

- **Registry probes, this session** (`https://nexus.ethosengine.com/repository/npm/`):
  `glob` `dist-tags.latest = 13.0.6`; `versions["13.0.6"].deprecated` absent;
  `versions["11.1.0"].deprecated` **present** (same blanket notice);
  `versions["10.5.0"].deprecated` present. Tarballs: `glob-13.0.6.tgz` →
  **`200`**; `glob-11.1.1.tgz`, `glob-11.1.2.tgz`, `glob-12.0.0.tgz` → `404`.
- **Module-shape probe**: `versions["13.0.6"].exports["."]` contains **both**
  `import` and `require` conditions, plus top-level `main`/`types`/`module`
  fields — the fact that makes the CJS `moduleResolution: "node"` consumer safe.
  Dependency sets for 11.1.0 (six) vs 13.0.6 (three) read from the same packument.
- **`@types/glob` probe**: `dist-tags.latest = 9.0.0`, `deprecated = "This is a
  stub types definition. glob provides its own type definitions, so you do not
  need this installed."`; `8.1.0` itself not flagged.
- **Declaration + call-site scan** across the root workspace (excluding
  `node_modules`): exactly two direct `glob` declarations (both `^10.3.0`);
  exactly eight source import sites, all `import { glob } from 'glob'`; one
  `require("glob")` hit in `elohim-service/dist/` build output.
- **Module-system confirmation**: `elohim-service` `tsconfig.json` →
  `"module": "commonjs"`, `"moduleResolution": "node"`, no `"type"` in its
  `package.json`; `elohim-content` `package.json` → `"type": "module"`.
- **Engines**: container `node -v` → `v22.22.2`; root `package.json`
  `engines.node` → `">=20.20"`; `glob@13.0.6` engines → `18 || 20 || >=22`.
- **Files touched this run**: this entry (new), five sibling entries, and one
  `.claude/data/deprecations.jsonl` status transition. No lockfile, no
  `package.json`, no `pnpm install`.
