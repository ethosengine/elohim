---
id: "backlog-deprecation-sophia-jsdom20-jest29-test-stack"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "sophia's jest@29 / jsdom@20 test stack carries abab, domexception, whatwg-encoding — clears only on a jest 29→30 migration across 15 workspace packages"
slug: "deprecation-sophia-jsdom20-jest29-test-stack"
written: "2026-07-30"
author: "deprecation-triage"
status: "backlog"
priority: "low"
deprecation_status: blocked
severity: low
fingerprints: ["ce0de21b8053"]
relatedNodeIds:
  - "backlog-deprecation-sophia-jest-jsdom-punycode-builtin-tr46"
  - "backlog-deprecation-glob-support-window-upgrade-unit"
  - "backlog-deprecation-sophia-eslint-8-eol-flat-config-migration"
tags: [deprecation, sophia, jest, jsdom, abab, domexception, whatwg-encoding, test-stack, major-upgrade, transitive]
cites:
  - https://jestjs.io/docs/upgrading-to-jest30
  - https://github.com/jsdom/jsdom/blob/main/Changelog.md
  - sophia/package.json
  - sophia/config/test/test.config.js
---

## What is deprecated

Three of the 25 packages in the sophia install banner (fingerprint
`ce0de21b8053`) are internals of **jsdom@20.0.3**, which enters solely through
`jest-environment-jsdom@29.7.0`. All three are "the platform has this natively
now" retirements, not vulnerabilities:

```
abab@2.0.6              Use your platform's native atob() and btoa() methods instead
domexception@4.0.0      Use your platform's native DOMException instead
whatwg-encoding@2.0.0   Use @exodus/bytes instead for a more spec-conformant and faster
                        implementation
```

None is first-party. Nothing in sophia imports any of them; they are jsdom's own
shims for Node versions that predate native `atob`/`btoa`/`DOMException`.

## Usage inventory

Reverse-dep trace over `sophia/pnpm-lock.yaml` `snapshots:`:

| Deprecated package | Parents |
|---|---|
| `abab@2.0.6` | `jsdom@20.0.3`, `data-urls@3.0.2` (itself a jsdom dep) |
| `domexception@4.0.0` | `jsdom@20.0.3` — sole parent tree-wide |
| `whatwg-encoding@2.0.0` | `jsdom@20.0.3`, `html-encoding-sniffer@3.0.0` (itself a jsdom dep) |

Single carrier chain, one root importer:

```
jest-environment-jsdom@29.7.0   (sophia/package.json — root devDependency)
└── jsdom@20.0.3                (sole parent of all three)
```

The wider jest@29 unit also supplies `glob@7.2.3` carrier edges via
`jest-config@29.7.0`, `jest-runtime@29.7.0`, `@jest/reporters@29.7.0`, and
`test-exclude@6.0.0` — those belong to the glob support-window entry, not here.

**Scale of the carrier.** `jest` is sophia's entire unit-test runner
(`"test": "jest"`, plus `utils/test-with-coverage.sh`). The workspace declares
jest-family packages at 29.x in the root manifest (`jest`, `@jest/globals`,
`jest-environment-jsdom`, `@types/jest`, `jest-css-modules-transform`,
`jest-serializer-html`, `jest-specific-snapshot`, `@swc/jest`) and the runner
executes across all 15 workspace packages.

## Migration path

Upstream has moved: `jest-environment-jsdom` latest is **30.4.1**, `jsdom` latest
is **30.0.1**. Jest 30's jsdom environment ships jsdom 26+, which dropped all
three shims in favour of the native platform APIs — so the deprecations clear as
a side effect of the runner bump rather than needing individual attention.

The unit of work is **jest 29 → 30 across the whole workspace**, per the official
upgrade guide:

1. Bump the jest family together (`jest`, `jest-environment-jsdom`,
   `@jest/globals`, `@types/jest`, and the transform/serializer plugins that
   declare jest peers: `@swc/jest`, `jest-css-modules-transform`,
   `jest-serializer-html`, `jest-specific-snapshot`).
2. Jest 30 raises its Node floor and removes several long-deprecated config keys
   and matcher aliases; `config/test/test.config.js` and every
   `packages/*/jest.config.js` need an audit against the upgrade guide.
3. jsdom 26 is materially stricter than jsdom 20 (navigation, CSS parsing, and
   `structuredClone`/`fetch` presence all changed). Expect real test churn — this
   is the part that cannot be estimated from the manifest alone.
4. `nyc`'s coverage merge path (`utils/test-with-coverage.sh`) reads Jest's
   `coverage-final.json`; verify the merged report still produces after the bump.

### Sibling concern — same upgrade unit, different symptom

`deprecation-sophia-jest-jsdom-punycode-builtin-tr46.md` (written the same day by
a concurrent triage pass) covers a **different symptom of this identical carrier**:
Node's `DEP0040` runtime warning, emitted because `tr46@3.0.0` — reached via
`whatwg-url@11` under `jsdom@20` — bare-`require`s the builtin `punycode` module.

The two entries are deliberately kept separate because they are found by different
detectors and cite different evidence: this entry tracks packages carrying a
lockfile `deprecated:` field (visible in the `pnpm install` banner), that one
tracks a Node process warning (visible in jest stderr). But **they have one fix
between them.** jest 30's jsdom environment ships jsdom 26+, which pulls
`whatwg-url@14` / `tr46@5` — clearing `DEP0040` — *and* drops the `abab` /
`domexception` / `whatwg-encoding` shims tracked here.

Whoever schedules the jest 29 → 30 sprint should close **both** entries with it,
and use both closure checks: the three packages absent from
`sophia/pnpm-lock.yaml`, and a clean jest run with no `DEP0040` on stderr. Neither
entry should be deleted on the strength of the other's verification.

## Current decision

**Blocked — exceeds the bounded-fix envelope on scale.** This is a major version
bump of the test runner touching every one of sophia's 15 workspace packages plus
the shared test config, with an unbounded test-churn tail from the jsdom 20 → 26
strictness jump. Both the "dependency major version" and ">20 files" stop
conditions in the deprecation-triage envelope are tripped. It needs an
operator-initiated sprint with eyes on the failing specs, not a background agent.

Two things that are explicitly **not** the blocker, recorded so a future pass does
not re-derive them:

- **Not artifact availability.** The repo switched public-package resolution to
  `registry.npmjs.org` in commit `ecc65384f` (2026-07-30); the "Nexus mirror
  serves cached artifacts only" constraint recorded across sibling entries no
  longer applies. Uncached tarballs probed 200 this pass.
- **Not the worktree race.** The live `feat/jquery-3` sprint blocks the *bounded*
  sibling fixes because they need an immediate lockfile regeneration. This item
  is not close enough to landing for that to be its limiting factor.

The ledger fingerprint stays present so the sentinel cites this decision
deterministically and never re-dispatches; the stasis sweep owns the re-check.

### Live trajectory

Low priority, and honestly: this entry's realistic next move is **not its own
sprint**. Three "use the native API" shims in a dev-only test environment are the
cheapest possible deprecations — zero runtime exposure, zero security weight
(severity `low` deliberately, not `security`).

The right move is to **attach it to the next jest upgrade** the repo wants for
other reasons, as acceptance criteria: after the bump, confirm `abab`,
`domexception`, and `whatwg-encoding` are absent from `sophia/pnpm-lock.yaml`.
Do not delete this entry until that is true. Sequencing note: the sophia eslint
8→10 flat-config migration touches the same devDependency block and `eslint-plugin-jest`
declares a jest peer — if both are scheduled, land them in one dependency sprint.

## Verification

No fix landed this pass; scoping evidence only.

- **Reverse-dep trace** over `sophia/pnpm-lock.yaml` `snapshots:` (parent-edge
  index, peer-suffix normalised) — parents per package as tabulated above.
  `domexception@4.0.0` has exactly one parent tree-wide; `abab` and
  `whatwg-encoding` have two each, both of which are themselves jsdom
  dependencies, so `jsdom@20.0.3` is the sole effective carrier for all three.
- **Single root importer:** `jsdom@20.0.3` ← `jest-environment-jsdom@29.7.0`, a
  root devDependency of `sophia/package.json`. No second carrier tree-wide.
- **Upstream state probed this pass:** `npm view jest-environment-jsdom version`
  → `30.4.1`; `npm view jsdom version` → `30.0.1`. A current path exists; this is
  a scale block, not an availability block.
- **No first-party usage:** the three packages appear nowhere in sophia's sources
  — they are jsdom-internal shims.

Closure requires: jest/jsdom bumped, all three packages absent from
`sophia/pnpm-lock.yaml`, and sophia's `pnpm test` + coverage-merge path green —
then this entry and its share of the banner fingerprint are decomposed.
