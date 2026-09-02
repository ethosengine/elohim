---
id: "backlog-deprecation-storage-client-ts-moduleresolution-node10"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "@elohim/storage-client tsconfig is the monorepo's last moduleResolution=node10 — TS 6.0.3 turned it into a build-stopping error, fix is `bundler` (verified emit-identical)"
slug: "deprecation-storage-client-ts-moduleresolution-node10"
written: "2026-09-02"
author: "deprecation-triage"
status: "wip"
priority: "high"
deprecation_status: in-progress
severity: high
fingerprints: ["9ab4dfa901ea", "dc758c9c3d3f", "6ffe65fc044a", "c8babb72ae2c"]
relatedNodeIds:
  - "backlog-deprecation-sentinel-redundant-capture-surfaces"
tags: [deprecation, typescript, typescript6, typescript7, moduleResolution, node10, storage-client, sdk, gate-deps]
cites:
  - https://aka.ms/ts6
  - https://www.typescriptlang.org/tsconfig/#moduleResolution
  - elohim/sdk/storage-client-ts/tsconfig.json
  - elohim/sdk/storage-client-ts/package.json
  - app/elohim-app/justfile
  - genesis/scripts/ci/install-substrate-runner.sh
---

## What is deprecated

`moduleResolution: "node"` (canonicalized by TypeScript as `node10`). TypeScript
6.0 promoted it from a silent legacy mode to a **reported error**; TypeScript 7.0
removes it. Verbatim, from `pnpm --filter @elohim/storage-client build`:

```
> @elohim/storage-client@0.1.0 build /projects/elohim/elohim/sdk/storage-client-ts
> tsc

tsconfig.json(25,25): error TS5107: Option 'moduleResolution=node10' is deprecated and will stop functioning in TypeScript 7.0. Specify compilerOption '"ignoreDeprecations": "6.0"' to silence this error.
  Visit https://aka.ms/ts6 for migration information.
/projects/elohim/elohim/sdk/storage-client-ts:
 ERR_PNPM_RECURSIVE_RUN_FIRST_FAIL  @elohim/storage-client@0.1.0 build: `tsc`
Exit status 2
error: recipe `deps` failed on line 70 with exit code 2
```

This is a **config-parse error**: `tsc` stops before type-checking, so it fails
the recipe outright rather than degrading to a warning.

## Why it became an error now

Three facts, dated from the repo, not inferred:

1. `elohim/sdk/storage-client-ts/tsconfig.json` has declared
   `"moduleResolution": "node"` since **2026-03-10** (`48b807551`, the
   `refactor: move sdk and rust-ipfs into elohim/` relocation). The line is old
   and never changed.
2. `app/elohim-app/justfile` gained the `deps` leg — `pnpm --filter
   @elohim/storage-client build` — on **2026-05-20** (`c8fa188ae`, "rebuild
   @elohim/storage-client dist as a gate prereq"). `deps` is wired into
   `gate: lint lint-routes lint-a11y deps build test`.
3. The workspace TypeScript moved **5.x → 6.0.3 on 2026-07-30** in `7663db487`
   ("build(deps): Angular 19→22.1.0, TS 6.0.3, drop @angular-devkit/build-angular").

The trap is the **declared-vs-resolved TypeScript split**.
`elohim/sdk/storage-client-ts/package.json` still declares
`"typescript": "^5.3.0"` in `devDependencies`, but pnpm's `.bin` resolution for
`tsc` reaches the hoisted workspace root, where `node_modules/typescript` is
**6.0.3**. The package's own declaration is not the compiler that runs it. So the
error arrived with the 2026-07-30 root bump, entirely without touching this
package, and stayed latent for a month because the `elohim-app` gate's `deps` leg
was not exercised on a full run until the **2026-09-01 release-ceremony push
wave** (`genesis/a2o/reports/release-ceremony/2026-09-01/push-wave4.log`,
captured 2026-09-02 07:38Z).

Read that as a gate-coverage fact, not just a config fact: `just gate elohim-app`
has been latently red on `deps` for ~34 days.

## Usage inventory

Full repo sweep for `moduleResolution` in every `*.json`/`*.jsonc` outside
`node_modules/` and `dist/` (includes the `sophia` submodule):

| Config | moduleResolution | Status |
|---|---|---|
| **`elohim/sdk/storage-client-ts/tsconfig.json:25`** | **`node`** | **the only offender, repo-wide** |
| `app/elohim-app/tsconfig.json:61` | `bundler` | clean |
| `app/elohim-elements/{elohim-core,elohim-imagodei,elohim-qahal}/tsconfig.json:5` | `Bundler` | clean |
| `app/elohim-library/tsconfig.json:34`, `projects/perseus-plugin/tsconfig.json:5` | `bundler` | clean |
| `app/elohim-library/projects/elohim-service/tsconfig.json:20` | `nodenext` | clean |
| `app/imagodei-portal/tsconfig.json:16`, `app/lamad/tsconfig.json:77` | `bundler` | clean |
| `doorway/doorway-app/tsconfig.json:21` | `bundler` | clean |
| `elohim/elohim-agent/elohim-agent-sdk/tsconfig.json:5`, `mcp-servers/elohim-content/tsconfig.json:5` | `NodeNext` | clean |
| `elohim/holochain/rna/typescript/tsconfig.json:5` | `NodeNext` | clean |
| `elohim/sdk/tsconfig.json:5`, `genesis/seeder/tsconfig.json:5` | `NodeNext` | clean |
| `elohim/sdk/epr-ts/tsconfig.json:5` | `Bundler` | clean |
| `genesis/a2o/tsconfig.json:5` | `Node16` | clean |
| `genesis/landing/tsconfig.json:5` | `bundler` | clean |
| `sophia/tsconfig-common.json:17` | `bundler` | clean (and sophia resolves TS 5.7.3, not 6.x) |

**Zero** configs set `"module": "commonjs"` (which would imply `node10` without
declaring it). The concern is genuinely one file.

**One TS-7.0 horizon residual, not in scope for this entry:**
`sophia/config/cypress/tsconfig.json` declares neither `module` nor
`moduleResolution` nor `extends`, so it inherits the compiler default — today
`node10` by implication. It does **not** emit TS5107 (the deprecation check only
fires on explicitly-written options) and sophia is pinned to TypeScript 5.7.3,
so it is inert. It lives in the `sophia` submodule, so fixing it is a submodule
commit and a separate unit of work. Re-check it when sophia takes TS 6.
(`elohim/kitsune2/docs-site/tsconfig.json` is likewise submodule-owned and
`update = none` — never built by our CI.)

## Migration path

Per https://aka.ms/ts6 the successors are `bundler`, `node16`, `nodenext`, or
`node20`. Two candidates were measured against this package, both from inside
`elohim/sdk/storage-client-ts` (the cwd `pnpm --filter … build` uses):

**`nodenext` — REJECTED, not bounded.** `pnpm exec tsc --noEmit
--moduleResolution nodenext --module nodenext` → **exit 2, 645 errors**, nearly
all `TS2835: Relative import paths need explicit file extensions in ECMAScript
imports`. The package has **238 extensionless relative imports vs 10 with `.js`**,
and the overwhelming majority sit in `src/generated/` — ts-rs output from
`elohim-storage/src/views.rs`. Fixing them means changing what ts-rs *emits*
across ~230 generated files, not editing this package. That is a cross-crate
codegen change, well past this concern's boundary.

**`bundler` — ADOPTED.** It is the mode that matches how this package is actually
consumed: Angular/vite (`app/elohim-app`, `app/lamad`), and tsx under Node
(`genesis/seeder`, `genesis/a2o`) — all of which resolve extensionless
specifiers. It also preserves today's semantics exactly, because `bundler` and
`node10` agree on extensionless relative resolution and neither rewrites import
specifiers on emit.

```json
"moduleResolution": "bundler"
```

`"module": "ESNext"` (already set) is a valid pairing for `bundler`; no other
compilerOption changes are needed.

## Current decision

**Fix in flight, applied by the orchestrator on `dev` (2026-09-02) to unblock the
release-ceremony push.** The one-line change is
`elohim/sdk/storage-client-ts/tsconfig.json:25` → `"moduleResolution": "bundler"`.
This triage run did not edit that file (it was concurrently owned); it
established the scope, dated the cause, rejected `nodenext` with evidence, and
pre-verified `bundler` so the landing change is a known-good one-liner.

The four ledger fingerprints are **one warning**, and only the first
(`9ab4dfa901ea`) is a distinct real capture. See the sentinel note below.

**Close-out (delete this entry, delete the four ledger rows) the moment
`cd elohim/sdk/storage-client-ts && pnpm exec tsc --noEmit` exits 0 with the
`bundler` value committed** — quote that in the closing commit message. Nothing
here needs to outlive the fix; there is no chronicle-grade lesson in a one-line
config migration. The two facts that *are* worth carrying — the
declared-vs-resolved TypeScript split, and the month of latent `deps` redness —
belong in that closing commit message, not in a parked file.

## Verification

Measured 2026-09-02 against TypeScript **6.0.3** (workspace-hoisted; confirmed
via `pnpm exec tsc --version` inside the package), from
`/projects/elohim/elohim/sdk/storage-client-ts`:

| Run | Exit | Diagnostics |
|---|---|---|
| `pnpm exec tsc --noEmit --moduleResolution bundler` | **0** | **0** |
| `pnpm exec tsc --noEmit --moduleResolution nodenext --module nodenext` | 2 | 645 (`TS2835`) |

**Emit-equivalence proven, not assumed.** Both modes were compiled to separate
scratch `--outDir`s and compared:

```
tsc --outDir <scratch>/emit-node10  --moduleResolution node10   → exit 0
tsc --outDir <scratch>/emit-bundler --moduleResolution bundler  → exit 0
diff -rq <scratch>/emit-node10 <scratch>/emit-bundler           → exit 0, 0 differences
```

**1372 emitted files, byte-identical.** The migration is a compiler-front-end
change with zero effect on the published `dist/` — no consumer of
`@elohim/storage-client` can observe it.

Measurement trap recorded so the next run does not repeat it: invoking
`pnpm exec tsc --noEmit -p elohim/sdk/storage-client-ts` **from the repo root**
produces 36 spurious `TS2304: Cannot find name 'fetch' / 'AbortController' /
'setTimeout'` errors, because the automatic `@types` inclusion walk starts from
the invoking directory and never reaches this package's own
`node_modules/@types/node@20.19.35`. Those errors are an artifact of the invoking
cwd, not of the package or of the resolution mode — they appear identically
under `node10`. Always verify this package from inside its own directory, which
is what `pnpm --filter … build` does.

## Sentinel note — one warning, four fingerprints

`9ab4dfa901ea` and `dc758c9c3d3f` are the same log line captured twice from the
same file under different `grep -n` prefixes (`1799:` and `526:`) — Class 3 of
`deprecation-sentinel-redundant-capture-surfaces.md`, exactly as predicted there.

`6ffe65fc044a` and `c8babb72ae2c` were minted **by this triage run**:
`6ffe65fc044a` is the bare-text variant (the same warning read out of the log
with `sed` instead of `grep -n`, so no line prefix), and `c8babb72ae2c` is the
same `tsc` diagnostic carrying a `elohim/sdk/storage-client-ts/` path prefix
because the compiler was invoked from the repo root rather than from the package
directory. Each requested a fresh background Opus dispatch for a concern already
under triage; both were declined. Evidence appended to the sentinel entry.
