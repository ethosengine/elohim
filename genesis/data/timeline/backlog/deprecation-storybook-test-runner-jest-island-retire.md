---
id: "backlog-deprecation-storybook-test-runner-jest-island-retire"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "@storybook/test-runner — the last Jest island in a Vitest repo (7 of 11 root banner deprecations)"
slug: "deprecation-storybook-test-runner-jest-island-retire"
written: "2026-07-30"
author: "deprecation-triage"
status: "backlog"
priority: "medium"
deprecation_status: blocked
severity: low
fingerprints: ["e83cd3f2d7e3"]
relatedNodeIds:
  - "backlog-deprecation-uuid-support-window-upgrade-unit"
  - "backlog-ci-storybook-smoke-test-timeout-flake"
tags: [deprecation, storybook, test-runner, jest, expect-playwright, jest-process-manager, nyc, http-server, whatwg-encoding, elohim-library, mirror-blocked]
cites:
  - https://storybook.js.org/docs/writing-tests/integrations/test-runner
  - https://playwright.dev/docs/test-assertions
  - app/elohim-library/package.json
  - app/elohim-library/scripts/test-storybook-ci.sh
  - .husky/pre-push
  - genesis/data/timeline/backlog/deprecation-uuid-support-window-upgrade-unit.md
---

## What is deprecated

Seven of the eleven packages named in the root-workspace install banner
(fingerprint `e83cd3f2d7e3`) are reachable through **one** first-party
devDependency: `@storybook/test-runner@^0.24.4` in `app/elohim-library`. Three of
them are reachable through *nothing else* in the tree.

Verbatim from the lockfile's `deprecated:` fields:

```
expect-playwright@0.8.0
    ⚠️ The 'expect-playwright' package is deprecated. The Playwright core
    assertions (via @playwright/test) now cover the same functionality. Please
    migrate to built-in expect.

jest-process-manager@0.4.0
    ⚠️ The 'jest-process-manager' package is deprecated. Please migrate to
    Playwright's built-in test runner (@playwright/test) which now includes full
    Jest-style features and parallel testing.

whatwg-encoding@2.0.0
    Use @exodus/bytes instead for a more spec-conformant and faster implementation

rimraf@3.0.2      Rimraf versions prior to v4 are no longer supported
glob@7.2.3        Old versions of glob are not supported, and contain widely
                  publicized security vulnerabilities …
inflight@1.0.6    This module is not supported, and leaks memory. Do not use it.
uuid@8.3.2        uuid@10 and below is no longer supported.
```

The banner is an aggregate; the decomposition of `e83cd3f2d7e3` across upgrade
units lives in six sibling entries (see **Current decision**).

## Usage inventory

`@storybook/test-runner@0.24.4` is declared once —
`app/elohim-library/package.json:56` (`"@storybook/test-runner": "^0.24.4"`) —
and consumed by two scripts in the same manifest: `test-storybook`
(`test-storybook`) and `test-storybook:ci`
(`sh scripts/test-storybook-ci.sh`). `scripts/test-storybook-ci.sh` is shared by
the `.husky/pre-push` `elohim-storybook` gate and
`app/elohim-library/Jenkinsfile`.

Its **direct** dependency set (lockfile snapshot,
`@storybook/test-runner@0.24.4(…)`) is where the deprecations enter:

| Deprecated package | How test-runner reaches it | Exclusive to this unit? |
|---|---|---|
| `expect-playwright@0.8.0` | direct dependency | **yes** — 1 chain, no other parent |
| `jest-process-manager@0.4.0` | direct dependency | **yes** — 1 chain, no other parent |
| `whatwg-encoding@2.0.0` | `http-server@14.1.1` → `html-encoding-sniffer@3.0.0` (the harness that serves `dist/storybook`) | **yes** — `http-server` is declared only by `app/elohim-library/package.json:67` and invoked only at `scripts/test-storybook-ci.sh:40` |
| `rimraf@3.0.2` | direct dependency; also `nyc@15.1.0` and `nyc → spawn-wrap@2.0.0` | no — also `karma@6.4.4`, `chromium-edge-launcher@0.2.0` |
| `glob@7.2.3` | `nyc@15.1.0` → `glob@7.2.3`; `nyc → test-exclude@6.0.0` → `glob@7.2.3` | no — also `karma@6.4.4`, `jest@29.7.0` internals |
| `inflight@1.0.6` | `glob@7.2.3` → `inflight` (its only parent, tree-wide) | no — inherits every `glob@7` parent |
| `uuid@8.3.2` | direct dependency; also `jest-junit@16.0.0`, `nyc → istanbul-lib-processinfo@2.0.3` | no — also `webpack-dev-server@5.2.2` → `sockjs@0.3.24` |

The **shape** of the debt is the point: `app/elohim-library` runs Storybook 10
(`storybook@^10.3.6`, `@storybook/web-components@^10.3.6`) and Vitest 4
(`vitest@^4.0.18`, `@vitest/coverage-v8@^4.0.18`, `@analogjs/vitest-angular`),
yet `@storybook/test-runner` drags in an entire **parallel Jest 30 + nyc +
Istanbul + jest-junit** test stack purely to run story smoke tests. Every
Angular target in every workspace already uses `@analogjs/vitest-angular:test`
(builder audit across all five `angular.json` files) — this is the only
Jest-based island left, and it is the single largest deprecated-transitive
carrier in the root workspace.

## Migration path

Retire `@storybook/test-runner` in favour of Storybook's Vitest-based story
testing (`storybook test` / the Vitest addon), which runs stories as Vitest
browser-mode tests using the Vitest + Playwright already in this manifest. The
end state removes four manifest entries at once — `@storybook/test-runner`,
`http-server`, and (transitively) the Jest 30/nyc/jest-junit stack — and deletes
`scripts/test-storybook-ci.sh` in favour of a `vitest run` invocation, keeping
the `.husky/pre-push` `elohim-storybook` gate and
`app/elohim-library/Jenkinsfile` pointed at the new command.

**A version bump is not available.** `@storybook/test-runner`'s `latest`
dist-tag on the configured registry is **`0.24.4` — exactly what is installed**.
The seven deprecated packages are pinned inside the runner's own dependency set,
so no `pnpm update` can clear them. Retirement is the *only* lever; an override
would fork upstream's Jest stack.

Story-parity work the migration owes: `test-storybook-ci.sh` currently
(1) requires `dist/storybook/index.json`, (2) `pnpm exec playwright install
chromium`, (3) serves the static bundle on `:6006` via `http-server`, (4) polls
`/index.json` for up to `STORYBOOK_WAIT_SECONDS` (30), then (5) runs
`test-storybook`. Steps 3–4 disappear under the Vitest addon (it drives the
built stories directly), which also removes the `:6006` port race that
`ci-storybook-smoke-test-timeout-flake` tracks.

## Current decision

**Blocked — the replacement artifact is not fetchable from the configured
registry, and the lockfile is write-locked.**

1. **Mirror-blocked, probed this run.** The Nexus npm mirror
   (`https://nexus.ethosengine.com/repository/npm/`) returns **HTTP 404 for the
   `@storybook/addon-vitest` packument itself** (`{"success":false,"error":"Not
   found"}`) — the replacement package cannot even be resolved, let alone
   fetched. Independently confirmed that the mirror is **cached-artifact-only**:
   `@anthropic-ai/sdk-0.39.0.tgz` (already in the tree) → `200`, while
   `rimraf-6.1.3.tgz`, `tar-7.5.13.tgz`, `uuid-11.1.1.tgz` → `404` on two
   consecutive passes. Clearing this is a Nexus proxy/remote-cache operator
   action.
2. **Write-lock.** `pnpm-lock.yaml`, `pnpm-workspace.yaml`, and the workspace
   `package.json`s (including `app/elohim-library/package.json`) are owned by
   concurrent in-flight runs this session; this triage was explicitly scoped to
   touch none of them, and did not. A manifest edit without a matching lockfile
   re-resolution would strand CI on `--frozen-lockfile` — partial work, not a
   fix.
3. **Gate risk is real, not theoretical.** `test-storybook` backs a pre-push
   gate and a Jenkins stage. Swapping the runner is a test-infrastructure
   migration with its own verification surface (every indexed story must still
   assert), which is operator-sprint scale rather than background-agent scale.

Fingerprint `e83cd3f2d7e3` stays **present with `status: blocked`** so the
sentinel cites this decision deterministically and never re-dispatches. It is a
**shared aggregate banner fingerprint**, decomposed across six sibling entries in
`genesis/data/timeline/backlog/`:
`deprecation-storybook-test-runner-jest-island-retire.md` (this entry),
`deprecation-angular19-toolchain-legacy-builder-transitives.md`,
`deprecation-helia-webrtc-native-addon-react-native-subtree.md`,
`deprecation-anthropic-agent-sdk-legacy-http-stack-bump.md`,
`deprecation-first-party-glob-v10-declarations-bump.md`, and
`deprecation-uuid-support-window-upgrade-unit.md`. Do not fold them back into one
file — they have different owners, different blockers, and different unblock
dates.

### Live trajectory

1. **Operator: make the Nexus npm proxy fetch uncached artifacts.** This single
   blocker gates this entry, all five siblings, and (per the uuid entry) ~47 of
   the vulnerability campaign's npm targets. Re-probe:
   `curl -o /dev/null -w "%{http_code}" https://nexus.ethosengine.com/repository/npm/@storybook%2faddon-vitest`
   — a `200` unblocks step 2.
2. **Scoped sprint: Storybook story-test migration in `app/elohim-library`.**
   Add the Vitest-based story testing config, port `test-storybook:ci` to
   `vitest run`, repoint the `.husky/pre-push` `elohim-storybook` gate and
   `app/elohim-library/Jenkinsfile`, then drop `@storybook/test-runner` **and**
   `http-server` from the manifest and delete
   `scripts/test-storybook-ci.sh`. Verification surface: every story that passes
   today must still pass, plus the pre-push gate green.
3. On green, this entry decomposes to nothing — but only after re-running the
   root install and confirming `expect-playwright`, `jest-process-manager`, and
   `whatwg-encoding` are **gone from the banner** (they are exclusive to this
   unit, so their disappearance is the proof); `glob@7`/`inflight`/`rimraf@3`/
   `uuid@8.3.2` will remain until the Angular unit clears.

## Verification

No fix was applied this run; nothing is claimed fixed. Verified:

- **Registry probes (the load-bearing blocker), this session:**
  `@storybook/test-runner` packument → `200`, `dist-tags.latest = 0.24.4`
  (**already installed** — no bump available);
  `@storybook/addon-vitest` packument → **`404`**;
  mirror cached-only behaviour confirmed by `sdk-0.39.0.tgz` → `200` vs
  `rimraf-6.1.3.tgz` / `tar-7.5.13.tgz` / `uuid-11.1.1.tgz` → `404` (×2 passes).
- **Reverse-dep trace** over `pnpm-lock.yaml` `snapshots:` (parent-edge index,
  peer-suffix-normalised) mapping each of the seven banner packages to its
  carrier chains — table above. `expect-playwright` and `jest-process-manager`
  each resolve to exactly **one** chain, terminating at
  `IMPORTER:app/elohim-library`.
- **`http-server` exclusivity scan**: declared only at
  `app/elohim-library/package.json:67`; the only invocation in tracked files is
  `app/elohim-library/scripts/test-storybook-ci.sh:40`.
- **Builder audit** across `app/elohim-app`, `app/elohim-library`,
  `app/imagodei-portal`, `app/lamad`, `doorway/doorway-app` `angular.json`: every
  `test` target is `@analogjs/vitest-angular:test` — no Karma/Jest first-party
  test target anywhere, which is what makes this a stranded island.
- **Files touched this run**: this entry (new), five sibling entries, and one
  `.claude/data/deprecations.jsonl` status transition. No lockfile, no
  `pnpm-workspace.yaml`, no `package.json`, no `pnpm install`.
