---
id: "backlog-a2o-conductor-rail-shared-typed-module"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "a2o mesh-proof scripts copy the frozen oracle's any-shaped conductor rail twelve times — extract one typed rail module and lift the scripts/ lint relaxation"
slug: "a2o-conductor-rail-shared-typed-module"
written: "2026-09-02"
author: "session-2026-09-01-adoption-ceremony"
status: "open"
priority: "medium"
jobs: [elohim-genesis]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-task-runtime-upgrade-a2o-receipt"
  - "backlog-task-release-channel-ceremony-driver"
tags: [a2o, lint-debt, conductor-rail, delegable]
---

## The debt (measured 2026-09-02, `pnpm exec eslint .` in genesis/a2o)

274 errors across twelve `scripts/*.ts` mesh-proof / ceremony drivers (release-ceremony,
release-lineage-probe, release-attestation-probe, device-ceremony, manifesto-native-declare,
fleet-ratify, delegation-live-check, rootauthor, w2-election-evidence-check,
w2-update-coordinators, coordswap-probe-build-info, carried-election-mesh-proof). 86 are
`no-explicit-any`: each script re-implements the FROZEN oracle's conductor rail
(`AdminWebsocket.connect → listApps → provisioned cell → authorizeSigningCredentials →
issueAppAuthenticationToken → AppWebsocket.connect → callZome`) with the oracle's `any`
shapes, because the oracle has no exports by rule and each driver was told to copy its SHAPE.

Interim (2026-09-02): the `scripts/**/*.ts` eslint block relaxes the `any` family and four
sonar structure rules with the reason inline, so the a2o gate is green again without touching
runtime behaviour of scripts that had just passed a live receipt.

## The fix

One typed module `genesis/a2o/scripts/lib/conductor-rail.ts` (name TBD) exporting
`connectPeer(name, adminPort, appPort, timeoutMs) → { agent, call<T>(zome, fn, payload) }`
with `@holochain/client` types, plus the port-scheme helper (`admin 4444+10i / app 4445+10i /
http 8090+i`). Migrate the eleven non-frozen scripts to it (the oracle stays frozen and
exempted). Then delete the relaxation block. DoD: `pnpm exec eslint .` exit 0 with the block
removed; each migrated script's `--help`/usage path runs; `release-ceremony.ts status` and
`release-attestation-probe.ts` pass on the household mesh.
