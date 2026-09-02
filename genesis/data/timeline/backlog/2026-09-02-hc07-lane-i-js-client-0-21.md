---
id: "backlog-hc07-lane-i-js-client-0-21"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Holochain 0.7 — Lane I: @holochain/client 0.21 across the pnpm workspace, single copy"
slug: "hc07-lane-i-js-client-0-21"
written: "2026-09-02"
author: "holochain 0.7 upgrade guide (Lane I)"
status: "open"
priority: "medium"
tags: [holochain-0.7, javascript, holochain-client, pnpm, codex-claimable, lane-i]
cites:
  - genesis/docs/superpowers/plans/2026-09-02-holochain-0-7-upgrade-guide.md
---

# Lane I — JavaScript clients (claimable by any agent; no session context assumed)

Part of the Holochain 0.7 upgrade guide (see cite; **§Global Constraints** and **§0.7 code-migration
patterns** govern). Holochain 0.7.0 changed the app/admin websocket wire protocol; `@holochain/client`
0.21 is the matching JS client. Our exposure is small: the tree imports only `AdminWebsocket`,
`AppWebsocket`, `encodeHashToBase64`, `CellId`, `ActionHash`, `AgentPubKey`, `EntryHash`, `AppInfo`.

## Write-set (nothing else)

`app/elohim-app/package.json:63`, `app/elohim-library/package.json:30`,
`app/elohim-library/projects/elohim-service/package.json:22`, `elohim/holochain/edgenode/scripts/package.json:10`,
`elohim/holochain/rna/typescript/package.json:25` (currently `^0.19.2`), `elohim/sdk/package.json:25,36`,
`genesis/a2o/package.json:61`, `genesis/seeder/package.json:64`, root `package.json` (`pnpm.overrides`),
`pnpm-lock.yaml`, and any `*.ts` the sweep in step 2 flags under `app/`, `elohim/sdk/`, `genesis/a2o/`,
`genesis/seeder/`, `elohim/holochain/`.

## Steps

1. Set every `@holochain/client` pin above to `"^0.21.0"`. Add `"@holochain/client": "^0.21.0"` under
   `pnpm.overrides` in the root `package.json` (a second copy breaks `instanceof` checks — upstream's
   documented trap). `pnpm install`. Verify one copy: `pnpm ls @holochain/client -r 2>/dev/null | grep -c '0.21'`
   equals the number of consumers and no `0.20`/`0.19` remains.
2. Sweep and fix per the guide's pattern table (common action fields moved under `header`;
   `is_webrtc` → `is_direct`; `signalingServerUrl` → `relayServerUrl`; `dumpNetworkStats` stats nested
   under `transport_stats`):
   `grep -rnE '\.(action|content)\.(author|timestamp|action_seq|prev_action)\b|is_webrtc|signalingServerUrl|dumpNetworkStats' app elohim/sdk genesis/a2o genesis/seeder elohim/holochain --include='*.ts' --exclude-dir=node_modules`.
   Sweep ALL `.ts` (tests, scripts), not only `src/` — untypechecked accesses fail at runtime, not build.
3. Gates: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts 2>&1 | tail -15; echo EXIT=$?` →
   0; `just gate app` → green; `cd genesis/a2o && pnpm exec tsc --noEmit; echo EXIT=$?` → 0;
   `cd genesis/seeder && pnpm exec tsc --noEmit; echo EXIT=$?` → 0 (if the package has no tsc script,
   say so).

## DoD

Gates EXIT=0 pasted in the report; one path-limited commit:
`git commit -m "chore(js): @holochain/client 0.21 across the workspace, single copy via pnpm override" -- <the files above> pnpm-lock.yaml`.
**Commit-only; never push.** Do not run against the deployed alpha doorway — the fleet is still 0.6
until Lane F; local vitest + tsc are the proof for this lane.
