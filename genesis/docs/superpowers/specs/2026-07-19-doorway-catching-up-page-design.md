---
title: "Doorway catching-up shed page — content-negotiated 503 with staged, honest progress"
id: doorway-catching-up-page
tier: spec
status: Draft
created: 2026-07-19
maintainers: Matthew Dowell + Claude Fable 5
class: protocol-canonical
domain: doorway
topic: [doorway, shed, backpressure, circuit-breaker, status-page, self-healing, catching-up]
context-tier: disclosed
steward: cartographer
graduation-trigger: shed-page-landed-and-a2o-scenario-green
requires_env: household-nodes
cites:
---

# Doorway catching-up shed page

## Problem

Every doorway shed path — upstream circuit open, upstream backpressure honored, admission
ceiling, EPR dispatch fast-fail — answers with the same opaque body:

```json
{"status":"catching-up","retryAfter":30}
```

For an API client that is correct backpressure. For a person in a browser it is a dead end
that says nothing about *what* is catching up, *how far along* it is, or *when* to expect
recovery. The 2026-07-19 elohim.host incident made the gap concrete: doorway-alpha-b's
breaker to its storage peer flapped open/half-open for hours and the entire host — including
the diagnostic routes that would have explained the incident — served only that JSON line.

## What ships

### 1. Content-negotiated shed responses (all shed sites, one vocabulary)

A single helper (`routes/catching_up.rs`) replaces the three shed-body emitters
(`server::http::catching_up_response`, `server::http::epr_dispatch_shed_response`,
`storage_proxy::catching_up_proxy_response`):

- `Accept` contains `text/html` (a browser navigation) → **503 + HTML page** (below).
- Anything else (JSON clients, curl, SDKs, blob/image fetches) → the **existing JSON body,
  byte-shape unchanged**. The SDK contract is untouched.
- Both variants keep `503` + `Retry-After` (+ `Cache-Control: no-store` on the HTML) —
  crawlers and caches must keep seeing honest unavailability.

The helper takes a shed *cause* so the page can say why: `UpstreamOpen { endpoint }`
(circuit open / upstream backpressure) or `Admission` (doorway itself at ceiling).

### 2. The page: staged, honest progress (launcher-loader style)

Askama template `templates/catching_up.html`, compiled into the binary (zero runtime I/O,
same mechanism as `/status`). Design tokens and base styles are extracted from
`status.html` into a shared `templates/_theme.css` Askama include used by both pages — the
shed page inherits the doorway design language structurally, not by copy-paste.

Stages (dots + pulse on the active one; no fake percentages — only observable state):

1. **Doorway online** — green by definition (the page rendered).
2. **Storage peer** — the light health probe (`storage.reachable` / `storage.healthy`).
3. **Data circuit** — breaker state (`closed` / `half-open` / `open`), consecutive error
   streak, countdown to the next half-open retry trial.
4. **Serving** — reached when the circuit closes; the page then reloads the originally
   requested URL automatically.

~50 lines of dependency-free inline JS polls `/status.json` every 5s, updates the stages,
counts down `Retry-After`, and self-recovers. Poll failures degrade the page to "doorway
busy — retrying", never a broken UI.

### 3. `/status.json` gains the progress fields

`/status.json` is doorway-local and never proxied, so it stays answerable during a shed.
It gains two Cat-C blocks sourced from the accessors `/admin/self-healing` already uses
(that admin surface stays as-is):

- `upstreams`: `[{ endpoint, circuit, errorStreak, skipped }]` from
  `UpstreamBreakers::snapshot()` (read-only — never admits a trial by observation).
- `admission`: `{ maxInflight, available, shedTotal }`.

### 4. Diagnostic routes bypass the breaker shed (self-blinding fix)

`GET /p2p/status` and `GET /db/p2p/conductor-diagnostics` are read-only probes. They now
bypass the breaker's shed *and* do not record outcomes into it: they always attempt the
upstream with the normal client timeout. The platform must not blind its own probes during
exactly the incident they exist to explain (the trust-contract runbook names these probes
as the authority).

## Classification (p2p-design-gate)

No new data entities. Everything presented is **Category C operational** state
(breaker/admission snapshots, in-process; reconstructable by observation). No DHT entry,
no coordinator, no storage projection, no new address form. Doorway-resident by necessity
and by legitimacy: a doorway serving *its own* shed state passes the swap test — a sibling
doorway serves its own equivalent page.

## Non-goals

- No projector-lag deep detail on the page (requires the unavailable upstream; stage 4
  covers recovery honestly). Can be added when a cached last-known-good lands.
- No SSR / Angular involvement — the page must have zero dependencies on the upstream
  that is down.
- No change to storage, DNA, or the route registry.

## Testing

- Unit: Accept-negotiation matrix (browser string → HTML; `application/json`, absent,
  bare `*/*` → JSON); `Retry-After` preserved on both variants; template renders against
  open/half-open snapshots; diagnostic-bypass path list; `/status.json` new keys stable.
- a2o: visitor during a shed sees the staged page and the page recovers on its own
  (`genesis/a2o/features/dataplane/doorway-catching-up-page.feature`).
