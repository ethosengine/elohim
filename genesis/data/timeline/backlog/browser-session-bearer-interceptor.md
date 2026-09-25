---
id: "backlog-browser-session-bearer-interceptor"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "One browser session bearer for both bundles — an auth interceptor that replaces three hand-attached Authorization headers and deletes the lamad → imagodei edge"
slug: "browser-session-bearer-interceptor"
written: "2026-09-25"
author: "agent:implementer@claude-opus-5-5-1m (post-station-4 sprint, Lane A, rulings R-A12/R-A13)"
status: "backlog"
priority: "medium"
area: "app/elohim-app, app/lamad, app/elohim-library/projects/elohim-identity"
domain: "code"
relatedNodeIds:
  - "backlog-arch-frontend-bundle-seams-backlog"
  - "genesis/a2o/features/lms/attention-witnessed-privately.feature"
  - "genesis/a2o/reports/post-station-4-2026-09-25/rulings.md#R-A12"
  - "genesis/a2o/reports/post-station-4-2026-09-25/rulings.md#R-A13"
  - "genesis/a2o/reports/post-station-4-2026-09-25/rulings.md#R-S11"
tags: [frontend, auth, bearer, interceptor, bundles, lamad, imagodei, identity, attention, search]
shift_objective: |
  Give the browser ONE way to send the signed-in person's session bearer to the doorway, so that
  a route the node attributes to a person is never reached anonymous by accident. Today neither
  EPR bundle (elohim-app, lamad) registers an HTTP interceptor that attaches `Authorization`, and
  the doorway resolves the caller ONLY from a verified bearer (it injects the `X-Agent-Cid` the
  node requires). Each person-attributed call therefore attaches the bearer by hand, and every
  one that forgot has reached the node anonymous and been refused.

  Done means: (1) the browser session bearer (read from the shared session token store) is
  exposed from `@elohim/identity`, where `StoredSession` / `SessionTokenStore` already live;
  (2) one functional `HttpInterceptorFn` built on it is registered in BOTH composition roots
  (`app/elohim-app/src/app/app.config.ts`, `app/lamad/src/app/app.config.ts`) and attaches the
  bearer only to requests aimed at this doorway's storage base (never a third-party origin, never
  a sibling doorway reached under `ANONYMOUS_CONTENT_READ`); (3) the three hand-attached call sites
  below are removed in favour of it, their specs moved to the interceptor; (4) the lamad → imagodei
  edge `@app/imagodei/services/auth.service` is deleted from lamad's app.config and from
  `app/scripts/workspace-import-baseline.json` (the ratchet then shrinks); (5) the attention
  scenario and the content-search private-row scenario stay green on the household mesh.
---

# One browser session bearer for both bundles

## Why this exists

The post-station-4 mesh proof reddened the attention scenario with
`POST /api/v1/observations answered 500: … requires the X-Agent-Cid header`. The cause
(ruling R-A12) was not the node and not the doorway: **no bundle attaches the session bearer
automatically.** The doorway resolves who is calling only from a verified bearer, so a call
without one arrives with no `X-Agent-Cid`, and the node refuses anything attributed to a person.

The sprint cured it the bounded way, one call site at a time. That was the right move under a
deadline, and it leaves a pattern that will recur: every future person-attributed route has to
remember to attach the header, and the failure mode for forgetting is a refusal on the mesh rather
than a red unit test.

## The three call sites an interceptor would subsume

| # | Call site | Hand-attached by | Ruling |
|---|---|---|---|
| 1 | `ObservationEmitterService.end()` and its `pagehide` keepalive flush — `app/elohim-library/projects/elohim-rea-runtime/src/lib/observation-emitter.service.ts`, fed by the `OBSERVATION_BEARER` token provided in both app configs | commit `bab140f95` | R-A12 |
| 2 | `StorageClientService.getObservationStream` — `app/elohim-app/src/app/elohim/services/storage-client.service.ts`, resolving `AuthService` on demand through `Injector` | the R-A13 commit | R-A13 |
| 3 | The R-S11 content-search reads (`GET /db/content/search`), whose provenance line currently records that browser searches rely on the doorway injecting `X-Agent-Cid` from the session | not yet hand-attached | R-S11 |

**Caution on #1.** The keepalive `fetch` on `pagehide` does not go through Angular's `HttpClient`,
so an `HttpInterceptorFn` cannot reach it. The emitter keeps a way to read the bearer for that one
path. The move is to feed `OBSERVATION_BEARER` from the new `@elohim/identity` bearer rather than
from `AuthService`, not to delete the token. The in-app `end()` post does go through `HttpClient`,
so the interceptor subsumes it.

## The edge it deletes

`lamad → @app/imagodei/services/auth.service` was added to lamad's composition root by R-A12 so
that lamad could provide `OBSERVATION_BEARER`. It is recorded in
`app/scripts/workspace-import-baseline.json` with this exit condition: the edge goes away once the
browser session bearer moves to `@elohim/identity`. The alternative rejected at the time, hard-coding
the shell's `elohim-auth-token` localStorage key inside lamad, would have been the same contract left
out of the ratchet's count. The library is the counted home that both bundles already consume.

## Readiness

- **Ready:** the three call sites are known and each carries a spec asserting the bearer, which
  moves to the interceptor's spec. `@elohim/identity` already defines `StoredSession`,
  `SessionTokenStore` and `InMemorySessionTokenStore`. `BrowserSessionTokenStore` (imagodei) is the
  writer, and its localStorage key names are the part that graduates.
- **Decide first:** the origin rule (which requests get the bearer). A bearer sent to the wrong
  origin leaks the session. The storage base from `CONNECTION_STRATEGY` is the natural allow-list.
  Direct/Tauri mode calls the person's own sidecar and needs no bearer. That exemption is recorded
  on the R-S11 provenance line.
- **Not this atom:** token refresh, 401-driven re-auth, and the two-portal SSO flow
  (`app/CLAUDE.md` §"Apps sign humans in through a portal"). The interceptor only attaches what the
  session already holds.
