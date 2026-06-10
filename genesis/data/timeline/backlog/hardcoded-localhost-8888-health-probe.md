---
title: Client-side code requests absolute http://localhost:8888/health, bypassing the same-origin proxy strategy
created: 2026-06-10
domain: D8 (doorway projection / connection strategy seam)
source: che-live-peer-dev-loop spike (look capture, 2026-06-10)
severity: low
---

Rendering the local dev server (`look http://localhost:4201/` with the alpha-target
proxy) captures a failed request: `http://localhost:8888/health net::ERR_CONNECTION_REFUSED`.
The dev proxy covers `/health` same-origin, so some client code constructs an
ABSOLUTE doorway URL instead of going through `window.location.origin` per
`doorway-connection-strategy.ts`. Harmless when a local doorway runs at :8888;
breaks the connection-strategy contract everywhere else (and produces console
noise that pollutes agent captures). Find the absolute-URL construction site
(likely a health/availability probe) and route it through the strategy.
Evidence: `genesis/a2o/reports/look/eyes-probe-local-x-alpha/capture.json`.
