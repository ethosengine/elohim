---
title: look capture.json shows 404 console errors without URLs — capture 4xx/5xx response URLs
created: 2026-06-10
domain: process-meta (build-and-test; a2o tooling)
source: che-live-peer-dev-loop spike (look captures, 2026-06-10)
severity: low
---

`look` renders of `/` on alpha show `console: "Failed to load resource: the server
responded with a status of 404 ()"` ×3 with no URL, and `failedRequests` only
carries network-level failures (`requestfailed`), not HTTP error responses. An
agent reading `capture.json` cannot tell WHAT 404'd. Small fix in
`PlaywrightDevice` (or `look.ts`): subscribe to `page.on('response')` and record
`{url, status}` for status >= 400 into a `httpErrors` array in the capture.
Keeps the L1 contract additive (new field, nothing removed).
