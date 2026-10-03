---
id: "backlog-security-sdk-vitest-3-to-4-bump"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Three SDK packages still declare vitest ^3.2.6 (GHSA-82fw-gwwq-j7x9, 5 alerts) — move to the 4.1 line the apps already use"
slug: "security-sdk-vitest-3-to-4-bump"
written: "2026-10-03"
author: "deprecation-triage"
status: "backlog"
priority: "medium"
deprecation_status: blocked
severity: medium
fingerprints: []
relatedNodeIds:
  - "backlog-dependabot-triage"
tags: [security, vitest, npm, dependabot, sdk, dev-dependency]
cites:
  - https://github.com/advisories/GHSA-82fw-gwwq-j7x9
  - https://vitest.dev/guide/migration
  - elohim/sdk/storage-client-ts/package.json
  - elohim/sdk/epr-ts/package.json
  - elohim/elohim-agent/elohim-agent-sdk/package.json
  - genesis/data/timeline/backlog/dependabot-triage.md
---

# vitest 3.2.7 in the SDK packages

## What is flagged

GHSA-82fw-gwwq-j7x9 (medium), vulnerable range `>= 2.1.0, < 4.1.11`, first patched 4.1.11.
Five open alerts, part of the 2026-10-03 push banner (ledger fingerprint `c81302c029ff`,
carried by `dependabot-triage`): #834, #835, #836 on the three `package.json` files, and
#841 (`@vitest/mocker`), #842 (`vitest`) on `pnpm-lock.yaml`.

## Usage inventory

- `elohim/sdk/storage-client-ts/package.json:61` — `"vitest": "^3.2.6"`
- `elohim/sdk/epr-ts/package.json:28` — `"vitest": "^3.2.6"`
- `elohim/elohim-agent/elohim-agent-sdk/package.json:31` — `"vitest": "^3.2.6"`

All three resolve `vitest@3.2.7`. The rest of the workspace is already on `vitest@4.1.11`
(commit 707c34fda, 2026-09-23). The 3.x line also holds `vite@7.3.1` and `esbuild@0.27.3` in
the lock.

## Migration path

Change the three specifiers to `^4.1.11`, `pnpm install`, run each package's test script.
There is no patched 3.x release, so this is a dependency major for these three packages.
Follow the vitest 4 migration guide for config changes; the app workspaces already made this
move and are the local reference.

## Current decision

**Blocked on scale rule, with a plan.** The fix changes a dependency major version, which this
agent does not land in the background. It is small (three one-line specifier changes) and
dev-only: vitest is a test runner, not shipped. Next step for an operator-initiated pass:
apply the three specifier changes, `pnpm install`, then `just gate` for
`elohim/sdk/storage-client-ts`, `elohim/sdk/epr-ts` and `elohim/elohim-agent/elohim-agent-sdk`.

## Verification

None yet. Closure requires the three packages' test suites green on vitest 4.1.11 and alerts
#834, #835, #836, #841, #842 no longer open.
