---
id: "backlog-portal-login-step-domain-scoped-identifier"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The deployed doorway names a signed-in hosted human with its gateway domain (`portal-a2o-<uuid>@alpha.elohim.host`) while the portal-login step asserts the bare registered name — the Act II sign-in scenario fails on an identifier convention, not on sign-in (the session was real)"
slug: "portal-login-step-domain-scoped-identifier"
written: "2026-08-29"
author: "m4-fleet-confirm shift (close)"
status: "triaged"
priority: "medium"
jobs: [elohim-genesis]
cluster: "arch-frontend-bundle-seams-backlog"
tags: [a2o, portal, auth, doorway, domain-scoping, act-ii]
---

genesis #1519 (fleet, playwright leg): `doorway-portal-login-neighbourhood.feature` "The deployed portal renders and
the doorway signs the human in" → `AssertionError: the doorway named "portal-a2o-<uuid>@alpha.elohim.host" for the
portal's token, not the human who signed in ("portal-a2o-<uuid>")`. The wrong-password twin PASSED, and the four
`auth-discovery-neighbourhood` scenarios PASSED (the API leg now reports the portal contract on every genesis run).
The deployed doorway scopes identifiers to its domain (see `threshold-login-domain-scoping.feature`); the household
mesh doorway does not, so the step `the doorway confirms a session for that human`
(`genesis/a2o/steps/ui/doorway-portal-login.steps.ts`) compares the bare name. Cure: the step accepts
`<name>` or `<name>@<doorway host>` (derive the host from `E2E_DOORWAY_ALPHA`), or the registration step records the
identifier the doorway itself returns. Steps are judge surface — not edited inside the shift that found this.

## 2026-08-29 cure in flight (owned by a sibling session)

Recurred as genesis #1520/#1521 (ci fingerprints d06665d8c323, 45e6118a0ea2). Two sessions reached for it at once
in the shared worktree; the cure that stands is the row's SECOND shape — the registration step reads back the
identifier the doorway itself returns (`canonicalIdentifier`) and the session assertion compares against that, with
`src/framework/doorway-identity.ts` (`namesHuman` / `expectedIdentifiersFor`) carrying the same rule for the
session-handoff and auth-lifecycle steps, and `MESH_DOORWAY_GATEWAY_SCOPING` letting the household mesh doorway
scope identifiers the way the fleet does. Nothing is derived, so nothing can drift from the doorway. The derivation
variant (`gatewayDomainOf` mirroring `auth_routes.rs`) was written and superseded the same hour. Landing commit and
fleet read: that session's delta below. Separately, the a2o gate's one standing error (`auth-discovery.steps.ts`
slow-regex) is cleared in the dataplane ratchet commit so this push is not refused on lint.

## 2026-08-29 landed — measured on the household mesh

Seven sites, one class. The instance the fleet caught is the portal step; the same naive
`assert.strictEqual(<what the doorway answered>, <what the scenario typed>)` sat at six more:
`auth-lifecycle.steps.ts` (`/auth/me`), `session-handoff.steps.ts` x3, `user-management.steps.ts`
(admin details for a THIRD-PARTY human, where the doorway's answer arrives via the admin row
rather than a session), and `oauth-code-flow.steps.ts`, which had registered under a synthetic
`@local.mesh`. `session-handoff.feature` additionally hardcoded `matthew.dowell@alpha.elohim.host`
— a fleet hostname pinned into a scenario that also runs on the mesh, so it could only ever be
green in one deployment. Replaced by the property it was reaching for: the handoff must not
RENAME the human, so the doorway-app account must be EXACTLY the identifier the doorway issued at
sign-in (strictly stronger, and deployment-independent).

**Why the fleet found this and the mesh could not.** Every deployed doorway runs with
`DOORWAY_URL` set; `hc-mesh.sh` launched both mesh doorways WITHOUT it, so `gateway_domain`
returned `None` and identifiers were stored verbatim. The household mesh was structurally
incapable of expressing the convention it was being asked to assert.
`MESH_DOORWAY_GATEWAY_SCOPING` (default **1**) closes that. Measured after the change: register
bare `probe-<uuid>` -> stored `probe-<uuid>@localhost`; login bare -> 200 naming
`probe-<uuid>@localhost`.

**Detector control.** Flipping the portal assertion back to the bare name fails on the
now-scoped mesh with genesis #1519's exact shape (`"...@localhost"` vs `"..."`). Reverted; green
after. The mesh can now pre-empt this class instead of the fleet discovering it.

**Evidence (household mesh, scoping on):** `@auth` lane **37 passed / 0 failed** / 9 held / 1
pending — the pending is `human "Susan" is logged in`, a household-formation precondition that
predates this work. Portal browser lane 2/2 Act I passed (Act II twins correctly held). a2o
`tsc --noEmit` and `eslint` clean on every touched file. Fleet read arrives with the next
genesis run; that is the CONFIRM, not the discovery.
