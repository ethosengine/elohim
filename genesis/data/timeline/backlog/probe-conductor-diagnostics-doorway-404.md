---
id: "backlog-probe-conductor-diagnostics-doorway-404"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "GET /db/p2p/conductor-diagnostics 404s through the doorway — seam-smoke peer-store leg reads the 404 as total=0"
slug: "probe-conductor-diagnostics-doorway-404"
written: "2026-08-07"
author: "hoot-owl integrator shift"
status: "open"
priority: "medium"
area: "dataplane"
domain: "code"
tags: [probes, doorway, seam-smoke, trust-contract, untrustworthy-zeros]
---

# The peer-store probe's zeros are 404s in disguise

Live-verified 2026-08-07 ~04:1xZ on BOTH alpha doorways (doorway build
1.0.0-dev-3e2ed345): `GET /db/p2p/conductor-diagnostics` returns the doorway's own
`not_found_response` (hint "Use WebSocket connection to /admin or /app/:port") — even
though `routes/catching_up.rs::is_diagnostic_probe` explicitly names the path as a
breaker-bypass probe, `/db/` is in `is_service_path`, and elohim-storage registers the
route (http.rs:13073). The proxy dispatch between service-gate and storage forward is
dropping it.

Consequence: `scripts/ci/substrate-seam-smoke.sh` seam 3 parses the 404 body with a
`|| echo "0 0"` fallback, so a PLUMBING failure is indistinguishable from an EMPTY PEER
STORE. This contaminated the 2026-08-07 partition evidence ("peer-store FAIL total=0")
— an instance of the untrustworthy-zeros class the trust-contract runbook warns about.

Fix shape when picked up: (1) find why the doorway's route registry misses the path
(registry defaults vs storage build-manifest declaration) and wire the forward; (2) make
seam-smoke's parser fail-closed on non-200 (print PROBE-BROKEN, never "0 0"); (3) the
runbook names these probes as authority — a probe that can silently lie must not remain
in the authority set unfixed.
