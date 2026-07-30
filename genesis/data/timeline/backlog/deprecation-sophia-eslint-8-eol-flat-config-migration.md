---
id: "backlog-deprecation-sophia-eslint-8-eol-flat-config-migration"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "sophia pins EOL eslint 8.57.1 — 13 .eslintrc.js files need the flat-config migration (target 10.x, not 9.x)"
slug: "deprecation-sophia-eslint-8-eol-flat-config-migration"
written: "2026-07-30"
author: "deprecation-triage"
status: "backlog"
priority: "medium"
deprecation_status: blocked
severity: medium
fingerprints: ["819aa7c6f6bd", "bba59aabdf63", "b1561f3d429d", "ce0de21b8053"]
relatedNodeIds: []
tags: [deprecation, eslint, sophia, flat-config, toolchain, eol, submodule]
cites:
  - https://eslint.org/version-support
  - https://eslint.org/docs/latest/use/configure/migration-guide
  - sophia/package.json
  - sophia/.eslintrc.js
  - elohim/sdk/package.json
---

## What is deprecated

`pnpm install` in the sophia submodule warns that its pinned ESLint is
end-of-life:

```
WARN  deprecated eslint@8.57.1: This version is no longer supported.
      Please see https://eslint.org/version-support for other options.
```

Per eslint.org/version-support (fetched 2026-07-30): **ESLint 8.0.0–8.57.1
reached end-of-life on 2024-10-05** — no updates, including no security fixes,
except via HeroDevs' commercial NES program. sophia is therefore running a lint
toolchain that has been unsupported for ~21 months.

### The migration target is 10.x, and this is time-boxed

The support state at triage time makes the naive "bump 8 → 9" the wrong move:

| Line | Status (2026-07-30) | Note |
|---|---|---|
| **10.x** (`latest` = 10.8.0) | **Current** since 2026-02-06 | the only actively supported line |
| 9.x (`maintenance` = 9.39.5) | Maintenance | critical/security fixes only, **EOL 2026-08-06** |
| 8.x (8.57.1) | **EOL** since 2024-10-05 | what sophia pins |

Migrating sophia 8 → 9 would land it on a line that goes EOL **within a week**
of this entry. Target **10.x** directly.

### Related, and deliberately not covered by this entry

Two adjacent facts found while scoping, recorded so they are not re-derived —
each is a *different upgrade unit*, so neither belongs in this file:

1. **The parent repo's ESLint fleet is on 9.x** — `app/elohim-app`,
   `app/elohim-library`, `app/elohim-elements/elohim-{core,qahal,imagodei}`, and
   `genesis/a2o` all pin `eslint: ^9.39.2`; `doorway/doorway-app` pins `^9.0.0`.
   These are already flat-config and are *supported today*, but 9.x hits EOL
   **2026-08-06**. That is a real, dated, fleet-wide trajectory needing its own
   concern — it did not fire a fingerprint, so no entry was invented for it here.
2. **`elohim/sdk/package.json:31` pins `eslint: ^8.55.0`** — also EOL, and in the
   parent repo (editable). It is inert, not debt worth churning: `elohim/sdk`
   (`@elohim/holochain-sdk`) is **not** in the root `pnpm-workspace.yaml` (only
   `elohim/sdk/storage-client-ts` and `elohim/sdk/epr-ts` are), it has **no**
   ESLint config file of any kind, and its `"lint": "eslint src/**/*.ts"` script
   is wired into **no** gate — so it is never installed and never runs. Bumping
   it would be unverifiable churn (no config, no gate, nothing to prove green).
   Noted, intentionally untouched.

## Usage inventory

**The pin** — `sophia/package.json:78`: `"eslint": "^8.57.1"`.

**Legacy eslintrc configs — 13 files**, all `.eslintrc.js` (the format ESLint 9
demoted and 10 requires opt-in flags to read):

```
sophia/.eslintrc.js                          (root config)
sophia/packages/sophia/.eslintrc.js
sophia/packages/sophia-editor/.eslintrc.js
sophia/packages/sophia-linter/.eslintrc.js
sophia/packages/sophia-utils/.eslintrc.js
sophia/packages/perseus-core/.eslintrc.js
sophia/packages/perseus-score/.eslintrc.js
sophia/packages/simple-markdown/.eslintrc.js
sophia/packages/pure-markdown/.eslintrc.js
sophia/packages/keypad-context/.eslintrc.js
sophia/packages/kas/.eslintrc.js
sophia/packages/kmath/.eslintrc.js
sophia/packages/math-input/.eslintrc.js
```

Zero `eslint.config.*` files exist in the submodule — nothing is migrated yet.

**Plugin ecosystem that must move in lockstep — 16 eslint-scoped packages** in
`sophia/package.json`:

- `@typescript-eslint/eslint-plugin` + `parser` `^8.18.0` (already v8 = flat-ready)
- `eslint-config-prettier@^9.1.0`, `eslint-plugin-prettier@^5.1.3`
- `eslint-plugin-import@^2.29.1`, `eslint-import-resolver-alias@^1.1.2`,
  `eslint-import-resolver-typescript@^3.5.5`
- `eslint-plugin-react@^7.34.1`, `eslint-plugin-react-hooks@^4.6.0`,
  `eslint-plugin-react-native@^4.1.0`, `eslint-plugin-jsx-a11y@^6.10.2`
- `eslint-plugin-jest@28.9.0`, `eslint-plugin-testing-library@^6.2.2`,
  `eslint-plugin-cypress@^2.15.1`, `eslint-plugin-storybook@^0.11.0`
- `eslint-plugin-jsdoc@^48.2.1`, `eslint-plugin-promise@^6.1.1`,
  `eslint-plugin-eslint-comments@^3.2.0`, `eslint-plugin-disable@^2.0.3`

Two are the blockers, both verified against the registry on 2026-07-30:

- **`eslint-plugin-eslint-comments`** — latest published is still **3.2.0** (the
  pinned version) with peer `eslint: >=4.19.1`, i.e. stalled since before flat
  config. The maintained community fork
  **`@eslint-community/eslint-plugin-eslint-comments@4.7.2`** declares peer
  `eslint: ^6 || ^7 || ^8 || ^9 || ^10` — an eslint-10-ready drop-in successor.
  This one is a clean swap.
- **`eslint-plugin-disable`** — latest published is still **2.0.3** (the pinned
  version) with peer `eslint: >=0.16.0`, and no successor fork was found. Its
  whole purpose is manipulating the legacy directory cascade, which flat config
  removes. Treat it as unsupported and **re-express** its effect as explicit
  flat-config `files`/`ignores` scoping rather than attempting a port. (Stated
  from release staleness + mechanism, not from a tested flat-config failure —
  confirm behaviour during the sprint.)

## Migration path

Per the ESLint flat-config migration guide, targeting 10.x:

1. Bump `eslint` to `^10`, `@typescript-eslint/*` to a 10-compatible v8.x line,
   and every plugin to its flat-config-capable release.
2. Replace `eslint-plugin-eslint-comments` with
   `@eslint-community/eslint-plugin-eslint-comments`.
3. Retire `eslint-plugin-disable` — re-express whatever per-directory disabling
   it provided as explicit `files`/`ignores` entries in the flat config.
4. Collapse the 13-file `.eslintrc.js` cascade into flat config. The cascade is
   the real work: flat config has **no directory-based inheritance**, so the
   root `eslint.config.js` must reconstruct each package's effective ruleset as
   explicit `files:`-scoped objects. Read each of the 13 files for the deltas it
   contributes before writing the flat equivalent — a mechanical concat will
   silently widen or narrow rule coverage.
5. `pnpm lint` (and `pnpm typecheck`) green, with a **rule-coverage diff** against
   the 8.x baseline — capture `eslint --print-config` for one representative file
   per package before and after, so a rule silently dropped by the cascade
   collapse is caught. Green-with-fewer-rules is the failure mode to guard.

Useful precedent in-tree: the parent repo's `app/elohim-app` already runs
ESLint 9 flat config with SonarQube-parity rules — its `eslint.config.*` is the
closest working reference for the plugin set and TS integration, though its rule
philosophy differs from Perseus-derived sophia.

## Current decision

**Blocked — two-major toolchain migration inside a submodule frozen by
concurrent work.**

1. **Scale.** 8 → 10 is a **two-major** jump requiring a **13-file legacy-cascade
   collapse** into flat config, coordinated bumps of **16** eslint-scoped
   packages, and **2** plugins with no direct flat-config successor. Both the
   "dependency major version" and ">20 files" stop conditions in the
   deprecation-triage envelope are tripped (13 configs + `package.json` + the
   plugin replacements, before counting any lint-fix churn the new rules
   surface). This wants an operator-initiated toolchain sprint.
2. **Worktree contention (transient).** All files live in the `sophia`
   submodule, which at triage time had **uncommitted changes on branch
   `feat/node24`** (`package.json` — the exact file holding the eslint pin —
   plus `packages/sophia/package.json` and `pnpm-workspace.yaml`), from a
   concurrent Node 24 + dependency-security upgrade. **No sophia file was
   modified by this triage pass.**

Ledger fingerprint `819aa7c6f6bd` stays present with `status: blocked` so the
sentinel cites this decision deterministically and never re-dispatches; the
deprecation-stasis sweep owns the re-check.

**Live trajectory.** Sequence after `feat/node24` lands: (a) confirm that branch
did not already move the eslint pin; (b) schedule the flat-config sprint against
**10.x** — and note the sprint has a natural pairing, since the parent fleet's
9.x line EOLs 2026-08-06, so a single toolchain push could take sophia 8→10 and
the parent 9→10 together with one shared plugin-compatibility investigation.
This is a lint-toolchain concern only: no runtime or shipped-artifact exposure,
which is why it sits at `medium`/`medium` rather than tracking with the jQuery
entry.

## Verification

What this pass proved (no fix landed — scoping evidence, not fix verification):

- **EOL status:** eslint.org/version-support — 8.0.0–8.57.1 EOL 2024-10-05;
  9.x Maintenance until 2026-08-06; 10.x Current since 2026-02-06.
- **Registry dist-tags:** `npm view eslint dist-tags` → `latest: 10.8.0`,
  `maintenance: 9.39.5` — confirms 10.x is the live target.
- **Config surface:** 13 `.eslintrc.js` files, **0** `eslint.config.*` files in
  the submodule — the migration is entirely un-started.
- **Plugin surface:** 16 eslint-scoped devDependencies in `sophia/package.json`.
- **Parent-fleet scan:** `eslint ^9.39.2` in 6 parent packages, `^9.0.0` in
  `doorway/doorway-app`, `^8.55.0` in the un-workspaced `elohim/sdk`.
- **`elohim/sdk` inertness:** absent from root `pnpm-workspace.yaml`; no
  `.eslintrc*` or `eslint.config.*`; `lint` script referenced by no gate.
- **Contention:** `git -C sophia status --porcelain` showed 3 modified files on
  `feat/node24` at triage time.

Closure requires: `eslint` at `^10`, flat config replacing all 13 `.eslintrc.js`
files, sophia `pnpm lint` + `pnpm typecheck` green, and a `--print-config`
rule-coverage diff showing no silent rule loss.

## 2026-07-30 — the eslint-8 subtree, from aggregate banner `ce0de21b8053`

Triage of sophia's aggregate install banner ("25 deprecated subdependencies
found") assigned **two further deprecated packages to this concern**. They are
not new work — they clear automatically when this entry's migration lands — but
they are recorded here so the banner is fully accounted for and nobody re-triages
them as a separate concern.

```
@humanwhocodes/config-array@0.13.0   Use @eslint/config-array instead
@humanwhocodes/object-schema@2.0.3   Use @eslint/object-schema instead
```

Reverse-dep trace over `sophia/pnpm-lock.yaml` `snapshots:` — a two-hop chain
under the pinned runner, sole parent at each hop:

```
eslint@8.57.1                          (the pin this entry retires)
└── @humanwhocodes/config-array@0.13.0 ← DEPRECATED (sole parent: eslint@8.57.1)
    └── @humanwhocodes/object-schema@2.0.3 ← DEPRECATED (sole parent: config-array)
```

These two are the clearest possible signal that the pin is the problem:
`@humanwhocodes/config-array` **is** the eslintrc-era config loader, renamed to
`@eslint/config-array` when flat config became the default. They exist in the tree
only because eslint 8 still loads `.eslintrc.js`. Migrating to flat config removes
the loader, and both packages leave with it — no separate action, no override.

The same banner also traces a `glob@7.2.3` carrier edge into this unit via
`eslint-plugin-monorepo@0.3.2` → `globby@7.1.1` → `glob@7.2.3`. That plugin is
**live** (`.eslintrc.js:67` registers it; line 313 enables
`monorepo/no-internal-import`), so it must be re-declared for flat config rather
than dropped — worth flagging as a migration task, since `eslint-plugin-monorepo`
is an eslintrc-era plugin whose flat-config support needs checking before the bump
is scheduled. The `glob@7.2.3` line itself is owned by
`deprecation-glob-support-window-upgrade-unit.md`, not here.

Add to this entry's closure check: after the migration, confirm
`@humanwhocodes/config-array` and `@humanwhocodes/object-schema` are absent from
`sophia/pnpm-lock.yaml`.
