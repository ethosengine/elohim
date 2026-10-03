---
id: "backlog-security-angular-22-patch-line-coordinated-bump"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Angular 22.1.0 carries 6 open advisories (platform-server, router, common) — coordinated family bump to 22.2.x with the @angular/build patch re-cut"
slug: "security-angular-22-patch-line-coordinated-bump"
written: "2026-10-03"
author: "deprecation-triage"
status: "backlog"
priority: "high"
deprecation_status: open
severity: security
fingerprints: []
relatedNodeIds:
  - "backlog-dependabot-triage"
  - "backlog-security-npm-transitive-override-refresh"
tags: [security, angular, npm, dependabot, ssr, patched-dependency]
cites:
  - https://github.com/advisories/GHSA-ff3f-86qr-9cv3
  - https://github.com/advisories/GHSA-f67j-2jqw-jpq7
  - https://github.com/advisories/GHSA-j3r3-mxqp-r2p4
  - https://github.com/advisories/GHSA-v3p8-whq6-r5jg
  - https://github.com/advisories/GHSA-f6mr-pjwc-34m4
  - https://github.com/advisories/GHSA-p297-fm68-3q8c
  - pnpm-workspace.yaml
  - pnpm-lock.yaml
  - patches/@angular__build@22.1.0.patch
  - genesis/data/timeline/backlog/dependabot-triage.md
---

# Angular 22.1.0 advisory set

## What is flagged

Six open Dependabot alerts on the root `pnpm-lock.yaml`, part of the 2026-10-03 push banner
(ledger fingerprint `c81302c029ff`, carried by `dependabot-triage`):

| Alert | Package | Severity | Advisory | First patched |
|---|---|---|---|---|
| #909 | `@angular/router` | high | GHSA-ff3f-86qr-9cv3 | 22.2.0 |
| #872 | `@angular/platform-server` | high | GHSA-f67j-2jqw-jpq7 | 22.1.6 |
| #871 | `@angular/platform-server` | high | GHSA-j3r3-mxqp-r2p4 | 22.1.4 |
| #863 | `@angular/platform-server` | high | GHSA-v3p8-whq6-r5jg | 22.1.4 |
| #862 | `@angular/platform-server` | high | GHSA-f6mr-pjwc-34m4 | 22.1.4 |
| #864 | `@angular/common` | medium | GHSA-p297-fm68-3q8c | 22.1.1 |

## Usage inventory

The whole `@angular/*` family resolves at 22.1.0 in `pnpm-lock.yaml` (animations, build, cli,
common, compiler, compiler-cli, core, forms, platform-browser, platform-server, router, ssr).
Consumers: `app/elohim-app`, `app/lamad`, `app/imagodei-portal`, `doorway/doorway-app`,
`app/elohim-library` and its three projects.

## Migration path

The router fix needs 22.2.0, so the target is the 22.2.x line for the whole family, moved
together. This is a minor bump, not a major. Two constraints:

- `pnpm-workspace.yaml` `patchedDependencies` pins `'@angular/build@22.1.0'`. The patch must
  be re-cut for the new `@angular/build` version and the JIT resource-transformer fallback
  confirmed present after install, as the comment beside it requires.
- Angular packages peer-pin each other exactly. A partial move produces a mixed family: the
  override proof run in `security-npm-transitive-override-refresh` produced `common` and
  `router` at both 22.1.0 and 22.2.1 with `platform-server` at 22.1.8.

## Current decision

**Open, queued.** Not attempted on 2026-10-03: it needs `pnpm install` in the shared tree and
the app gates while an alpha fleet roll was in flight there. Sequence it before, or in the
same change as, the override refresh.

Next step: bump every `@angular/*` specifier to the 22.2.x line across the consuming
`package.json` files, re-cut `patches/@angular__build@<version>.patch`, `pnpm install`, then
`just gate` for the Angular projects and `just test app`.

## Verification

None yet. Closure requires the Angular project gates green and alerts #862, #863, #864, #871,
#872, #909 no longer open.
