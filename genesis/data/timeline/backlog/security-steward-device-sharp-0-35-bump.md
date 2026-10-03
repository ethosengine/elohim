---
id: "backlog-security-steward-device-sharp-0-35-bump"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "steward/device declares sharp ^0.34.5 — two high advisories (4 alerts), patched only on the 0.35 line"
slug: "security-steward-device-sharp-0-35-bump"
written: "2026-10-03"
author: "deprecation-triage"
status: "backlog"
priority: "medium"
deprecation_status: blocked
severity: high
fingerprints: []
relatedNodeIds:
  - "backlog-dependabot-triage"
tags: [security, sharp, npm, dependabot, steward, tauri, native-addon]
cites:
  - https://github.com/advisories/GHSA-rgj7-g3m4-5g8c
  - https://github.com/advisories/GHSA-f88m-g3jw-g9cj
  - https://sharp.pixelplumbing.com/changelog/
  - steward/device/package.json
  - pnpm-workspace.yaml
  - genesis/data/timeline/backlog/dependabot-triage.md
---

# sharp 0.34.5 in steward/device

## What is flagged

Two high-severity advisories, four open alerts, part of the 2026-10-03 push banner (ledger
fingerprint `c81302c029ff`, carried by `dependabot-triage`):

| Alerts | Advisory | Vulnerable | First patched |
|---|---|---|---|
| #861 (`steward/device/package.json`), #860 (`pnpm-lock.yaml`) | GHSA-rgj7-g3m4-5g8c | `< 0.35.4` | 0.35.4 |
| #739 (`steward/device/package.json`), #732 (`pnpm-lock.yaml`) | GHSA-f88m-g3jw-g9cj | `< 0.35.0` | 0.35.0 |

## Usage inventory

- `steward/device/package.json:14` — `"sharp": "^0.34.5"`, the only direct declaration in the
  workspace; the lock resolves `sharp@0.34.5` once.
- No `import`/`require` of `sharp` was found in tracked `steward/device` JS/TS sources by a
  grep on 2026-10-03, so the consumer is most likely a build or icon-generation step. Confirm
  the consumer before bumping; if there is none, removing the dependency closes all four.
- `sharp` is listed in `pnpm-workspace.yaml` `onlyBuiltDependencies` (native addon with a
  prebuilt libvips binary).

## Migration path

`^0.34.5` cannot reach 0.35.x, and for a `0.x` package a minor is the breaking boundary. Change
the specifier to `^0.35.4`, `pnpm install`, confirm the prebuilt binary installs in the Che
container and in the steward CI image, then run whatever consumes it.

## Current decision

**Blocked on scale rule, with a plan.** The bump crosses sharp's breaking boundary (0.34 to
0.35), which this agent does not land in the background, and it needs a native binary install
verified in two environments. Next step for an operator-initiated pass: identify the consumer,
then either remove the dependency or move the specifier to `^0.35.4` and run `just gate` for
`steward/device`.

## Verification

None yet. Closure requires the steward/device gate green and alerts #732, #739, #860, #861 no
longer open.
