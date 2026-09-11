---
id: "backlog-ssr-connection-mode-conflates-node-with-native"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "SSR connection-mode detection conflates 'rendering on Node' with 'native sidecar' — bakes localhost:8090 into served HTML and drifts transfer-state keys on every hydrated page"
slug: "ssr-connection-mode-conflates-node-with-native-2026-09-11"
written: "2026-09-11"
author: "doorway-federation sprint — Task 18 measure on alpha (face02de)"
status: "open"
priority: "high"
tags: [ssr, elohim-service, connection-strategy, lamad, elohim-app, transfer-state, doorway-failover, D8]
relatedNodeIds:
  - arch-frontend-bundle-seams-backlog
cites:
  - app/elohim-library/projects/elohim-service/src/connection/connection-strategy-factory.ts
  - app/elohim-library/projects/elohim-service/src/connection/direct-connection-strategy.ts
  - app/lamad/src/app/utils/blob-url.ts
  - genesis/docs/superpowers/plans/2026-09-10-doorway-federation-three-reds-to-green-plan.md
---

# SSR connection-mode detection conflates Node with native

## What was observed (2026-09-11, alpha at app build face02de)

`curl https://alpha.elohim.host/lamad/path/foundations-christian-technology` returns server-rendered HTML
carrying `<img src="http://localhost:8090/blob/sha256-c5dcc24c…">`. Every browser visitor gets
`net::ERR_CONNECTION_REFUSED` for that blob. Found by the a2o `@concern:epr-atom-home` After-hook
("The learning app is one lens away"), reproduced three runs, and by `pnpm look` (one failedRequest).

## Cause

`connection-strategy-factory.ts:51` — `detectConnectionMode()`:
`if (typeof process !== 'undefined' && process.versions?.node !== undefined) return 'direct';`
In the Angular SSR process that is always true, so `direct-connection-strategy.ts:126-128` returns
`config.storageUrl || http://localhost:8090` as the storage base for HTML that a *browser* will consume.
The browser path resolves `doorway` → `location.origin` and was never wrong, which is why the defect was
invisible in local dev and in every browser-side test.

## Second symptom of the same cause (not yet cured)

Angular transfer-state keys are built from the request URL. SSR caches under
`http://localhost:8090/api/v1/resilience/…` (verified in the deployed shell HTML); the hydrated browser
looks up `https://alpha.elohim.host/api/…`. Every hydrated page misses the transfer state and silently
refetches — the SSR compute buys nothing on those calls. Same bug, different surface.

## What is cured, and where the cure belongs

Cured at lamad's composition root only: `app/lamad/src/app/utils/blob-url.ts` (`withOriginRelativeBlobUrls`)
claims `direct` only for a genuine native runtime (`__TAURI__` / `__env.connectionMode === 'direct'`),
and emits origin-relative `/blob/{hash}` for browser AND SSR. Commit e09eec608, tests red→green.

The general cure belongs in `@elohim/service` `detectConnectionMode()`: "I am rendering on Node" must be
distinguished from "the client I render for talks to a local sidecar" — the SSR renderer is rendering FOR
a doorway-served browser, so its storage base for anything the browser will dereference is the serving
origin (doorway-failover: the same bundle must serve from both doorways). Touches both bundles plus the
native/Tauri case; needs its own gate run and an a2o clause on served-shell-boots (no `localhost` in any
served document). `TODO(ssr-mode-detection)` in blob-url.ts points here.
