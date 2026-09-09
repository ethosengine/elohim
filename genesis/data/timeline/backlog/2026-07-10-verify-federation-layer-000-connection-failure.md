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

## 2026-09-09 recurrence: eventual 200 exceeds every five-second reader

Fresh public verification after doorway runtime `9b55e82f` exposed a more precise form of the
same `000` symptom. A ten-second curl budget received zero bytes from both public origins and
reported `HTTP=000`. Longer read-only probes eventually returned the unchanged membership-list
response with HTTP 200, but apex took 9.533 seconds (`x-elohim-hop-serve-ms: 9485.761`) and alpha
took 34.068 seconds (`x-elohim-hop-serve-ms: 33988.660`). This is a server-side latency failure,
not DNS, TLS, route shadowing, or a permanently missing handler.

The first responsible production seam is
`doorway/doorway-service/src/routes/federation.rs::handle_federation_doorways`. The handler awaits
`services::federation::get_all_doorways` before it reads and merges the existing `PeerCache`.
That DHT query calls `ZomeCaller::call_failover`; one conductor operation has a ten-second
deadline and fallback conductors are tried sequentially. The same route is consumed by two
five-second readers:

- `app/elohim-app/src/app/imagodei/services/doorway-registry.service.ts` uses
  `AbortSignal.timeout(5000)` and catches failure as an empty optional federation result.
- `services::federation::refresh_peer_cache` builds a reqwest client with a five-second timeout
  and queries sibling doorways through this route. A slow DHT-first response therefore also
  prevents the HTTP federation cache that the route intends to merge from refreshing.

The browser timeout is doing its job: optional federation discovery cannot hold the page open
for a 10–34 second backend traversal. Extending that timeout would move the delay into the UI and
would leave the five-second background reader broken.

**Bounded repair option.** Give only the DHT enrichment in `handle_federation_doorways` a
documented route budget below five seconds. On DHT error or elapsed budget, build the existing
self-only `Vec<DoorwaySummary>`, merge the existing `PeerCache`, and serialize the same
`FederationDoorwaysResponse`. A focused regression should use a never-settling DHT future to
prove a bounded HTTP 200 self-plus-cached response, and a ready DHT result to prove that admitted
identity, signature, endpoint, and capability fields remain intact. This is the smallest change,
but cancellation before `ZomeCaller`'s ten-second deadline means that particular call cannot mark
the primary unhealthy or advance its cooldown. Background DHT/JWKS work remains independent; the
tradeoff must be measured rather than assumed harmless.

**Stale-while-revalidate option.** Keep the latest DHT-admitted doorway registrations in shared
state, refresh that snapshot outside the request path, and have the HTTP handler read the snapshot
before merging `PeerCache`. This preserves DHT-derived identity and signature fields while making
request latency independent of conductor failover. It is the stronger design, but requires an
explicit shared-cache field and refresh wiring across `server/http.rs`, `main.rs`, and
`services/federation.rs`; it should be a dedicated bounded station with source tests and fresh
live latency proof. Lowering the global zome deadline or loosening the browser/capture oracle does
not address the ownership problem.

No federation source, client timeout, fixture, runner, or assertion was changed or shipped during
this verification pass.

### Evidence boundary: asset continuity passed; visual capture rail remained red

Station 8's doorway asset repair passed independently on both public origins: the declared browser
head and root served the same `version.json`; root `main-2O7U4VGN.js` returned 200 with the same
1,702,032 bytes as its immutable-head path; served-shell and SSR checks exited 0; and the frozen
browser slice passed 3 scenarios / 24 steps. HEAD behavior was unchanged. This proves the asset
binding repair against the then-declared app bundle; it does not prove that federation discovery
meets its latency contract.

The two light/dark visual passes rendered complete, styled landing pages on both origins, with no
page errors or HTTP errors. All eight unchanged `pnpm look` runs nevertheless exited 1 because
each captured the same-origin federation request as `net::ERR_ABORTED` (plus varying third-party
YouTube/Google aborts). No `visualValidation` bucket was emitted because this scenario slice had no
`@elohim-visually-validated` scenario. Each capture names its sibling `shot.png`:

- `genesis/a2o/reports/look/fresh-9b55-pass1-alpha-light/capture.json`
- `genesis/a2o/reports/look/fresh-9b55-pass1-alpha-dark/capture.json`
- `genesis/a2o/reports/look/fresh-9b55-pass1-apex-light/capture.json`
- `genesis/a2o/reports/look/fresh-9b55-pass1-apex-dark/capture.json`
- `genesis/a2o/reports/look/fresh-9b55-pass2-alpha-light/capture.json`
- `genesis/a2o/reports/look/fresh-9b55-pass2-alpha-dark/capture.json`
- `genesis/a2o/reports/look/fresh-9b55-pass2-apex-light/capture.json`
- `genesis/a2o/reports/look/fresh-9b55-pass2-apex-dark/capture.json`
