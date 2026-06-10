---
title: Welcome page full-page render is ~14,000px of mostly empty gradient
created: 2026-06-10
domain: app-shell (UI surface; no substrate D#)
source: che-live-peer-dev-loop spike (look renders, 2026-06-10)
severity: low
---

`pnpm look https://doorway-alpha.elohim.host/` and the same render via a local dev
server both produce a 1280x13989 full-page screenshot: hero + CTA at top, footer
("The time to build technology organized around love is now") at bottom, and
~12,000px of empty blue gradient between them. Identical local vs deployed, so it
is an app layout property (min-height inflation or an empty stretched section),
not a deploy artifact. Evidence: `genesis/a2o/reports/look/eyes-probe-alpha/` and
`eyes-probe-local-x-alpha/`. Worth a look-driven polish pass on the Welcome
surface; also distorts any full-page visual judgment (agents read mostly gradient).
