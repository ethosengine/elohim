---
title: Alpha serves 403 on /db/content/manifesto during anonymous Welcome render
created: 2026-06-10
domain: D8 (doorway projection; reach-gating verdict on a content read)
source: papercut sweep Item 5 — look httpErrors capture (commit 67000fbf9), probe 2026-06-10
severity: medium
---

The new `httpErrors` capture in `look` (records HTTP >=400 responses that neither
console-error text nor `requestfailed` surface) caught an anonymous render of
`https://doorway-alpha.elohim.host/` requesting `/db/content/manifesto` and
receiving **403** — alongside the known 404s (`version.json`,
`/api/v1/epr/elohim-host-landing/nav-context`, `/wasm/elohim-cache-core/...`).
A 403 on a content read is a reach/authz verdict, not a missing row: either the
manifesto's reach should permit anonymous public read (seed/reach config wrong on
alpha), or the Welcome surface should not request it unauthenticated (client
fetch wrong). Decide which side is lying. Evidence:
`genesis/a2o/reports/look/papercut-httperrors-probe/capture.json`. Related-but-
distinct: the 404 trio above; the EprRouter poisoned-scope 404 lesson
(memory `project_epr_router_empties_on_poisoned_scope`) is a different shape.
