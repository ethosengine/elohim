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
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1108/
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1105/
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1104/
  - https://jenkins.ethosengine.com/job/elohim/job/dev/1522/
  - genesis/a2o/features/auth/steward-login-portal-handoff.feature
  - genesis/a2o/steps/ui/steward-login-portal-handoff.steps.ts
  - genesis/a2o/steps/ui/account-m5.steps.ts
  - genesis/data/timeline/backlog/steward-grant-fixture-surface.md
  - doorway/doorway-service/src/routes/auth_routes.rs
  - doorway/doorway-service/src/routes/admin_dev.rs
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

### F2 (`a9a433e7b31c`) — a TEST bug (same class as F1), NOT downstream of the router

**Earlier hypothesis DISPROVEN.** I first guessed F2 was the empty-EprRouter
`/threshold` fallback and would clear with the Concern B storage fix. Build
**#1106** (storage fix deployed, router repopulated, sitemap populated) shows F2
**STILL FAILING** — so it is not the router.

**Verified root cause (2026-06-07):** F2's terminal assertion
(`steward-login-portal-handoff.steps.ts`, "completes the OAuth dance at the
doorway") required `page.url().origin === doorwayOrigin`. But the shared When
step navigates the PlaywrightDevice — whose base is `appUrl` (`doorwayToAppUrl`,
i.e. `alpha.elohim.host`) — to `/threshold/login`, so the browser sits on the
**app** origin throughout. And the threshold IdP surface is served on **both**
origins: the doorway forwards `/threshold/*` to doorway-app
(`doorway-service/src/routes/threshold.rs`) and the app ingress mirrors it (both
return 200 "Doorway Operator Dashboard"; verified live on alpha). So the
origin-equality check tested the device base, not the product — **never
satisfiable** for this app-initiated flow. Like F1, it only became reachable
(and thus visibly red) once `DEV_MODE` let the scenario progress past the
fixture-grant gate at #1104. **Not a product regression.**

**Fix (landed):** replaced the brittle origin-equality with the genuine
fall-through invariant — **no portal hand-off** (the clean inverse of the
sibling "the browser is redirected to {portal}"): assert the portal-host
interceptor never fired (`REDIRECT_ATTEMPT_KEY` unset) and the browser did not
land on the portal-host origin. Origin-agnostic; real signal (a wrong hand-off
sets the key → fails). `tsc`/`eslint` clean.

Neither matches a CI/orchestrator anti-patterns museum entry (the four traps are
infra/measure-class). No museum citation.

## Current decision

`ci_status: in-progress` — BOTH fingerprints have a landed fix that is now
**merged to `dev` AND deployed to alpha** (no longer unpushed). Neither was a
product regression; both were never-satisfiable test assertions revealed when
`DEV_MODE` let the scenarios progress past the fixture-grant gate at #1104.

- **F1** (`ce5f4e6a4d1f`): operator chose option (a) — a `DEV_MODE`-gated
  portal-health override. Landed `0ce5c23e0` (doorway `routes/admin_dev.rs` +
  `probe_first_portal_host` decision split + the two a2o step rewrites).
- **F2** (`a9a433e7b31c`): assertion corrected to the no-portal-hand-off
  invariant (origin-agnostic). Landed `f242caad9` (a2o step-only change).

**Deploy/confirmation status (reconciled 2026-06-09):** all three fix commits
landed on `dev` 2026-06-07 ~19:00–23:58Z. Genesis **#1108** (2026-06-08 21:00Z)
still ran against an alpha whose doorway binary PREDATED `0ce5c23e0` — the
rewired a2o step (`account-m5.steps.ts:223` "portal host responds to /healthz
with 200") shows `✔` because the step now POSTs the dev override, but the OLD
binary had no override endpoint so the POST was a no-op and `portalHostUrl`
stayed `undefined` (F1 still red at #1108; this is consistent with the fix, not
a refutation of it). The binary fix only reached alpha via the
**elohim#1522 redeploy (2026-06-09 12:04Z)** — AFTER #1108 last tested. Since
then NO genesis build has re-run these E2E scenarios against the redeployed
alpha: **#1109 ABORTED** (museum trap #1 — lossy 0-failure measure, no signal)
and **#1110 ran 0 a2o scenarios** (`Findings: 0 (scenarios: 0)` for both the
api and browser cucumber profiles — the E2E executor produced no scenario
output; also no signal, NOT a green confirmation). So the failures' *absence*
since #1108 is not yet a confirmed disappearance — it is the museum-trap-#1 "no
signal" gap.

Ledger stamped `status: triaged` + `triaged_at_build` (F1=1108, F2=1106 — each
fingerprint's own `last_build` at triage, the sweep's recurrence reference). The
harvester's disappearance-sweep is now armed: it confirms by a genesis
green-streak ≥3 with no recurrence of either fingerprint once a real E2E run
lands against the #1522-deployed alpha. NOT stamped `decompose_on_confirm` —
the F1/F2 lesson (`DEV_MODE` reveals a pre-existing never-satisfiable test
fiction; the fixture-grant gate is the upstream cause — see
`steward-grant-fixture-surface.md`) is a recurring-class insight worth
graduating, so this graduates-then-decomposes rather than auto-deleting.

## Fix trail

- F1: `0ce5c23e0` (DEV_MODE portal-health override — doorway `admin_dev.rs` +
  `auth_routes.rs` `portal_probe_decision` split + `account-m5.steps.ts` and
  `steward-login-portal-handoff.steps.ts` rewrites). Verified in-tree: under
  `dev_mode` + a doorway-local health override, `portal_probe_decision` returns
  `Return` (short-circuits the live HEAD to the non-resolving `.example` host)
  so `/auth/login` carries `portalHostUrl`; with `dev_mode` off, byte-for-byte
  unchanged. On `dev` and on the working HEAD.
- F2: `f242caad9` — a2o step assertion fix in
  `steward-login-portal-handoff.steps.ts` (origin-equality → no-hand-off
  invariant); `tsc`/`eslint` clean. Verified live that `/threshold/login` is
  served on BOTH origins (the fact that breaks the old assertion). On `dev`.
- Recurrence reference for the sweep: `triaged_at_build` = 1108 (F1) / 1106
  (F2). The disproven "rides storage fix" hypothesis was settled by #1106 (F2
  still red with the router populated).
- Final proof = the next genesis build that actually runs the auth E2E suite
  against the #1522-deployed alpha (cannot run live a2o E2E locally).
