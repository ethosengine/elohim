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

## Root cause (VERIFIED — foreground session, 2026-06-07)

NOT a regression in the `portalHostUrl` producer, and NOT the Slice-2/3 commit
cluster. The two fingerprints have DIFFERENT, separable root causes:

### F1 (`ce5f4e6a4d1f`) — a pre-existing TEST-PRECONDITION fiction, not a product regression

`probe_first_portal_host` (`auth_routes.rs:3890`) is the producer. It (1) GETs the
human's registered hosts from `/api/v1/account/portal-hosts`, then (2) does a
**REAL server-side HEAD to `{host_url}/healthz`** with a 1 s timeout
(`auth_routes.rs:3908-3917`) and returns the host ONLY on HTTP success. The
test's registered host is `https://matthew.steward.example/account` — a
**non-resolving `.example` domain**. The HEAD can never succeed, so the probe
correctly returns `None` and `/auth/login` correctly omits `portalHostUrl`.

The feature's `Background` step **"the portal host responds to /healthz with 200"**
is a **no-op comment stub** (`genesis/a2o/steps/ui/account-m5.steps.ts:198-201`)
— it stands up no reachable host. So the precondition the assertion depends on is
fiction; the assertion was **never satisfiable in CI**, independent of any deploy.

**Why it "appeared" at #1104 (NOT a regression onset):** at #1102 alpha ran
`mode:production`, so the fixture grant `PUT /admin/users/{id}/steward` 403'd
(`FIXTURE_ONLY`) and the scenario failed EARLY, never reaching the portalHostUrl
assertion. The rebuild-all enabled `DEV_MODE` on alpha, the grant then PASSED, and
the scenario progressed FURTHER to finally hit the long-standing unsatisfiable
assertion. Enabling dev_mode REVEALED a pre-existing test fiction; it did not
regress the product. (Confirmed: `account-m5.steps.ts:198` is comment-only;
`auth_routes.rs:3908` is a live `client.head(&probe_url)`.)

### F2 (`a9a433e7b31c`) — most likely downstream of the empty EprRouter (Concern B)

The fall-through scenario correctly omits `portalHostUrl` (portal unreachable);
the failure is the browser landing at `https://alpha.elohim.host/threshold/login`
instead of finishing at the doorway origin. **This is the same empty-EprRouter
symptom as Concern B** (`ci-genesis-lamad-shell-routing-regression.md`): with the
router empty, the doorway's no-root-projection fallback 302s to `/threshold`, and
the OAuth fall-through is caught by that same fallback. Strong hypothesis that the
Concern B storage fix (heal legacy scope shapes + per-row degradation in
`find_active_projections`) ALSO clears F2 — to be re-verified against a deployed
fix. NOT independently confirmed this run.

Neither matches a CI/orchestrator anti-patterns museum entry (the four traps are
infra/measure-class). No museum citation.

## Current decision

`ci_status: in-progress`.

- **F2:** expected to clear with the **Concern B storage fix** (LANDED — see that
  backlog). Re-verify the portal-unreachable terminal origin after the fixed
  storage binary deploys to alpha; if it then completes at the doorway origin,
  close `a9a433e7b31c` by disappearance.
- **F1:** a **test-precondition gap, not a product regression** — DECISION
  PENDING with the operator. The doorway's `/healthz` reachability probe is
  server-side, so a Playwright `page.route` interceptor cannot satisfy it. Three
  options: (a) a `DEV_MODE`-gated health-probe override on the steward-grant
  fixture surface so `probe_first_portal_host` trusts a registered host without a
  live HEAD (makes BOTH the reachable + unreachable scenarios testable);
  (b) point the test's registered host at a genuinely reachable CI stub;
  (c) park as a documented test-infra blocker and re-tag the scenario. No F1 code
  landed this session.

Ledger left `status: open` for both fingerprints (no `triaged_at_build` stamp) —
F2 closes by disappearance once the Concern B fix deploys; F1 awaits the operator
decision.

## Fix trail

- F2: no separate fix — rides the Concern B storage change (`db/rea_commitments.rs`
  scope healing + per-row degradation).
- F1: none landed (decision pending — see Current decision).
- Recurrence reference for the sweep: `last_build` at triage = 1105.
