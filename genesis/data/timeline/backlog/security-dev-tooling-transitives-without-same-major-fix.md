---
id: "backlog-security-dev-tooling-transitives-without-same-major-fix"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Dev-tooling transitives with no same-major fix: adm-zip, image-size, extract-zip, serialize-javascript, esbuild, uuid (16 alerts) — each needs a parent bump, a cross-major override, or a recorded dismissal"
slug: "security-dev-tooling-transitives-without-same-major-fix"
written: "2026-10-03"
author: "deprecation-triage"
status: "backlog"
priority: "medium"
deprecation_status: blocked
severity: high
fingerprints: []
relatedNodeIds:
  - "backlog-dependabot-triage"
  - "backlog-security-npm-transitive-override-refresh"
tags: [security, npm, dependabot, adm-zip, sonarqube-scanner, image-size, metro, extract-zip, puppeteer, serialize-javascript, esbuild, uuid]
cites:
  - https://github.com/ethosengine/elohim/security/dependabot
  - app/elohim-app/package.json
  - pnpm-lock.yaml
  - genesis/data/timeline/backlog/dependabot-triage.md
  - genesis/data/timeline/backlog/deprecation-uuid-support-window-upgrade-unit.md
  - genesis/data/timeline/backlog/deprecation-helia-webrtc-native-addon-react-native-subtree.md
  - genesis/data/timeline/backlog/security-sdk-vitest-3-to-4-bump.md
---

# Dev-tooling transitives with no same-major fix

## What is flagged

Sixteen open alerts on the root `pnpm-lock.yaml`, part of the 2026-10-03 push banner (ledger
fingerprint `c81302c029ff`, carried by `dependabot-triage`). They are the residue left after
the same-major override set in `security-npm-transitive-override-refresh`: the installed
version has no patched release on its own major line, or no patched release at all.

## Usage inventory

Parents read from the `snapshots:` section of `pnpm-lock.yaml` on 2026-10-03.

| Package (installed) | Alerts | Severity | Pulled by | First patched |
|---|---|---|---|---|
| `adm-zip` 0.5.16 | #703, #854, #868, #885, #886, #887, #888, #889 | 5 high, 3 medium | `sonarqube-scanner` 4.3.4 (declared at `app/elohim-app/package.json:112`) | 0.6.0 / 0.6.1; one advisory has none |
| `image-size` 1.2.1 | #869, #870 | 2 high | `metro` 0.83.7 (React Native subtree) | 2.0.3 |
| `extract-zip` 2.0.1 | #814, #840 | 2 high | `@puppeteer/browsers` 2.13.1 | none published |
| `serialize-javascript` 6.0.2 | #554, #641 | 1 high, 1 medium | `copy-webpack-plugin` 12.0.2 | 7.0.3 / 7.0.5 |
| `esbuild` 0.27.3 | #659 | low | `vite` 7.3.1, `tsx` 4.21.0, `storybook` 10.3.6, `@web/dev-server-esbuild` 1.0.5 | 0.28.1 |
| `uuid` 8.3.2, 10.0.0 | #640 | medium | `@storybook/test-runner`, `jest-junit`, `sockjs`, `istanbul-lib-processinfo`, `@cucumber/messages` | 11.1.1 |

## Migration path

Per row, in order of preference: bump the parent to a release that requires the patched line;
else a cross-major override after reading the child's changelog for API breaks the parent
depends on; else a dismissal on GitHub with the reachability evidence.

- **adm-zip**: check for a `sonarqube-scanner` release on `adm-zip` 0.6. The scanner runs in CI
  and local analysis only and unpacks an archive it downloads itself.
- **image-size**: rides the React Native subtree already canonicalized in
  `deprecation-helia-webrtc-native-addon-react-native-subtree`; removing that subtree removes
  the alert.
- **extract-zip**: no fix exists. Dismissal with evidence, or drop the puppeteer parent, are
  the only moves.
- **serialize-javascript**: check for a `copy-webpack-plugin` release on 7.x.
- **esbuild**: `vite` 7.3.1 leaves the lock when the SDK packages move to vitest 4
  (`security-sdk-vitest-3-to-4-bump`); re-check the remaining three parents after that.
- **uuid**: owned by `deprecation-uuid-support-window-upgrade-unit`; listed here only so the
  alert number has a home.

The 2026-07-30 npm campaign disposed most of these packages' earlier alerts as NOT-REACHABLE
in commit 12d01341b. Those dispositions were written to the repo and never recorded as
dismissals on GitHub, so the alerts still count in the banner.

## Current decision

**Blocked: each row needs a dependency major or a risk-acceptance call.** A background agent
lands neither. Nothing here ships in a production bundle as far as the parent chain shows, but
that has not been re-verified per alert on 2026-10-03. Next step for an operator-initiated
pass: walk the table top to bottom, choosing parent bump, cross-major override or GitHub
dismissal per row; adm-zip first (8 alerts, 5 high).

## Verification

None yet. Closure per row is the alert no longer open, by fix or by a dismissal that carries
its reason on GitHub.
