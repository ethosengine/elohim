---
id: "backlog-ci-genesis-steward-portal-handoff-regression"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Steward portal-handoff E2E regression — /auth/login drops portalHostUrl, SPA never navigates to portal host"
slug: "ci-genesis-steward-portal-handoff-regression"
written: "2026-06-07"
author: "ci-failure-triage"
status: "wip"
priority: "high"
ci_status: in-progress
fingerprints: [ce5f4e6a4d1f, a9a433e7b31c]
jobs: [elohim-genesis]
relatedNodeIds: []
tags: [ci, genesis, e2e, auth, doorway, portal-handoff, recovery-m5, auth-portal-convergence]
cites:
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1105/
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1104/
  - genesis/a2o/features/auth/steward-login-portal-handoff.feature
  - genesis/a2o/steps/ui/steward-login-portal-handoff.steps.ts
  - doorway/doorway-service/src/routes/auth_routes.rs
  - doorway/doorway-service/src/server/http.rs
  - elohim/elohim-storage/src/services/session_exchange.rs
  - app/elohim-app/src/app/imagodei/services/providers/oauth-auth.provider.ts
  - app/elohim-app/src/app/account/services/handoff.service.ts
  - app/elohim-app/src/app/account/services/portal-host-discovery.service.ts
---

# Steward portal-handoff E2E regression (elohim-genesis)

The `@auth-portal-convergence @recovery-m5` E2E scenarios in
`genesis/a2o/features/auth/steward-login-portal-handoff.feature` regressed: the
doorway's `/auth/login` no longer carries `portalHostUrl` for a registered
steward, and the SPA consequently never navigates to the steward's portal host.
The portal-unreachable fall-through scenario also lands on the wrong origin.

## The failure

Two fingerprints, one root concern — same feature file, same auth/portal surface.

`ce5f4e6a4d1f` — Scenario "Matthew's login response carries his portal host URL"
(NOT shem-gated; `@elohim-visually-validated`):

```
✔ Then the /auth/login response includes "isSteward": true
✖ And the /auth/login response includes a "portalHostUrl" matching his registered host
AssertionError [ERR_ASSERTION]: Expected /auth/login "portalHostUrl" to equal
  registered host "https://matthew.steward.example/account", got undefined
```

The sibling scenario "Doorway redirects Matthew to his portal host after auth"
fails downstream of the same cause (steps.ts:527):

```
✖ Then the browser is redirected to "https://matthew.steward.example/account"
Error: No client-driven navigation toward the portal host was observed within 15s.
  Expected the SPA to read portalHostUrl from /auth/login and navigate; the
  page.route interceptor never fired (REDIRECT_ATTEMPT_KEY unset).
```

`a9a433e7b31c` — Scenario "Portal host unreachable — fall through to local auth"
(NOT shem-gated). The fall-through itself is correct (response correctly omits
`portalHostUrl`), but the browser ends at the wrong place:

```
✔ And the /auth/login response does NOT include "portalHostUrl"
✖ And the browser completes the OAuth dance at the doorway as relying-party-and-identity-provider
AssertionError [ERR_ASSERTION]: Expected the OAuth dance to finish at the doorway
  origin "https://doorway-alpha.elohim.host" (no portal hand-off) but the browser
  is at: https://alpha.elohim.host/threshold/login
```

The genuinely shem-gated scenario ("Hosted visitor receives no portalHostUrl",
line 43 `@requires:shem`) is correctly HELD/skipped, not failed — these two are
real `@requires:doorway` failures (doorway is available in the run).

**Occurrence evidence:** ledger `seen: 1`, `first_build: 1105`, `last_build: 1105`
— but cross-build correlation shows BOTH signatures are also present in build
**#1104** with byte-identical assertion text and step line numbers
(`:449`, `:620`). The harvester's window opened at 1105; the failure is older
than its first fingerprint. Onset is bounded below: build **#1102**
(2026-06-06 19:13Z) has ZERO of these signatures; #1104 (2026-06-07 14:38Z) has
both. (#1103 was ABORTED — a lossy 0-failure measure, museum trap #1; it carries
no signal.)

## Verdict

**Real regression** (not a flake, not infra). Stable and reproducing across
consecutive builds (#1104, #1105), deterministic assertion, two correlated
scenarios sharing one root surface. The clean-then-failing transition between
#1102 and #1104 pins it to a code change, not a substrate or flake event.

## Root cause (suspected — onset window)

The onset window (#1102 clean → #1104 failing) contains the dense doorway/EPR
Slice-2/Slice-3 landing on dev (Jun 5–6):

- `bcf1b7a28` feat(doorway): universal `/epr/{id}` address — reserved prefix + root-bundle dispatch
- `385a7485a` feat(doorway): Slice-3 dispatch — alias 302 at B13, claims-aware `/epr/{id}` resolver, `/sitemap.xml`
- `28773e763` fix(doorway): anon-reach coherence — {commons,public} readable set, single authority
- `26ee27eb4` feat(app): EPR-native link sweep — programmatic nav via `EprNavService`, lamad navigator wiring

The `portalHostUrl` producer surface is `doorway/doorway-service/src/routes/auth_routes.rs`
+ `server/http.rs` + `elohim/elohim-storage/src/services/session_exchange.rs`; the
app consumer/navigation surface is `oauth-auth.provider.ts` + `handoff.service.ts`
+ `portal-host-discovery.service.ts`. The likeliest mechanisms (for the
foreground driver to confirm): (a) the doorway auth-login path's dispatch/reach
reshuffle drops `portalHostUrl` from the response body, and (b) the EPR-native
nav sweep changed where the SPA lands so the fall-through ends at
`/threshold/login` rather than the doorway origin. NOT verified to a single
commit this run — left to the foreground fix owner who has the live repro.

This is a product-behavior regression; it matches NO entry in the CI/orchestrator
anti-patterns museum (the four traps are infra/measure-class). No museum citation.

## Current decision

`ci_status: in-progress`. **The foreground (main) session is actively driving the
fix for this exact routing/portal-handoff regression as of 2026-06-07** (operator
reported it directly). This triage run, by coordination, canonicalizes + supplies
the evidence base (onset bounding, cross-build correlation, suspect window,
producer/consumer surfaces) and does NOT land code — to avoid racing the
foreground owner on shared auth/doorway files. Ledger left `status: open` (no
`triaged_at_build` stamp) because no fix landed here; the foreground commit will
carry the actual fix and the integrator/sweep will confirm by disappearance
(genesis green-streak ≥3 with no recurrence of either fingerprint).

## Fix trail

- None landed in this triage run (foreground-owned by coordination).
- Recurrence reference for the sweep once a fix lands: `last_build` at triage = 1105.
