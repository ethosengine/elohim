---
id: backlog-verify-federation-layer-000-connection-failure
kind: backlog
title: Verify Federation Layer — GET /api/v1/federation/doorways returns 000 (connection failure)
created: 2026-07-10
status: OPEN
domain: D-federation
source: genesis #1272 stage evidence (ci-observer)
severity: medium
tags: [federation, genesis-pipeline, verify-stage]
---

**Context.** genesis #1272 `Verify Federation Layer` went UNSTABLE on:

```
❌ federation.doorways — GET /api/v1/federation/doorways → 000
```

HTTP status `000` = the curl never completed a request (connection refused / DNS / TLS / parse
failure), NOT the `catching-up` projector shed that the seed stages hit. So it is a **separate
root cause** from the seeder catching-up fix (`ec5f0f522`) and from the read-path shed twin
(`2026-07-10-server-side-epr-read-path-catching-up-shed.md`).

**To scope.** Which endpoint was probed (internal svc DNS vs external `https://…`), and whether the
`/api/v1/federation/doorways` route was reachable at all at that moment (doorway restarting from an
earlier stage's `Restart Doorway POD` recovery? route shadowed? netpol?). `000` during a window
where the doorway pod was mid-restart (Seed Projections triggered a pod restart in the same build)
is the leading hypothesis — a timing/ordering artifact of the restart-recovery, which the seeder
catching-up fix should reduce the need for. Re-check after `ec5f0f522` lands: if the pod no longer
needs restarting mid-seed, this `000` may disappear on its own.

**Acceptance.** `Verify Federation Layer` federation.doorways probe returns a real HTTP status
(200 with the doorway membership list), or the stage's own retry/readiness gate tolerates a
transient restart window.
