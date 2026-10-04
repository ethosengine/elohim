---
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The identity-recovery gate on hosted registration is a stop-gap — four things it does not close"
id: doorway-identity-recovery-gate-residuals-2026-10-04
slug: "doorway-identity-recovery-gate-residuals-2026-10-04"
written: "2026-10-04"
author: "rust-architect (hosted-provisioning stop-damage, branch fix/hosted-provisioning-stop-damage)"
status: "backlog"
priority: "high"
tags: [doorway-service, hosted-provisioning, auth-posture, identity, registration, C9, C12, risk]
relatedNodeIds: []
cites:
  - doorway/doorway-service/src/routes/auth_routes.rs
  - doorway/doorway-service/src/routes/admin_conductors.rs
  - doorway/doorway-service/src/server/http.rs
  - doorway/doorway-service/src/conductor/provisioner.rs
  - doorway/doorway-service/src/conductor/provision_intent.rs
  - doorway/doorway-service/.epr-meta/hosted-cell-promised.habit.md
  - genesis/orchestrator/manifests/doorway/alpha.yaml
  - genesis/orchestrator/manifests/doorway/alpha-b.yaml
  - "doorway-auth-posture-declared-stage | Doorway Auth Posture | sha256:97ba36fbe79abc18 | path: genesis/docs/content/elohim-protocol/architecture/2026-08-25-doorway-auth-posture-declared-stage.md"
  - "plane-separation-design | Plane separation | sha256:ccc2bf039ff28cf0 | path: genesis/docs/superpowers/specs/2026-10-03-plane-separation-design.md"
---

# What the identity-recovery gate does not close

The branch `fix/hosted-provisioning-stop-damage` added two refusals to `POST /auth/register` and a proof
requirement to hosted provisioning. They close one class. This entry records what is still open, so the gate is
not read as "registration identity is done". Line numbers are as of the commit that adds this file.

## What was reachable before the branch (the class, now closed)

An unauthenticated registration could be bound to an identity that already existed. Two routes led there.

- **Through the app id.** A hosted person's app id carried three bytes of a hash of their identifier, and the
  identifier is the local part of whatever the registration body says. Several provisioning paths read "an app
  exists under this id" as "this is that person's app" and returned its agent. The registration route's
  "Agent already has a Human profile" branch then attached that agent's existing Human to the new account row
  and session.
- **Through the doorway-phase recovery.** A registration naming the doorway agency phase authors its Human on
  the doorway's own conductor agent. Once that Human exists, every later registration on that path was answered
  with the existing Human, and the account row and session were issued with its `human_id` and its profile.

Fix commits: `71ad3524e` (typed faults; an install of unknown outcome is never re-offered), `39d444a87` (the
provisioning intent log), `e06b89eb8` (full-width app ids; adoption only where the conductor's agent key equals
the key recorded for the identifier; both recovery branches gated), and the commit that adds this entry (the
refusal is `403`, not `409`).

## Still open

### 1. The FIRST doorway-phase registration is ungated

`agency_phase` is read from the unauthenticated request body (`auth_routes.rs:1218`). In the `"doorway"` arm
(`auth_routes.rs:1343`) `call_create_human` runs on the doorway's own conductor agent (`:1376`) with the
caller's display name and, if supplied, the caller's `human_id`. Only the RECOVERY — the branch taken when that
agent already has a Human — is gated (`operator_identity_recovery_authorized`, `auth_routes.rs:1024`). On a
fresh doorway, whoever sends the first doorway-phase registration becomes the Human on the operator's agent.
The same holds for a `hosted` registration on a doorway with no conductor pool, which falls back to the same
singleton call (`:1621`).

Exit: the agency phase must not be a caller's choice. Either the doorway-phase arm requires the operator
credential outright, or the operator's Human is created at bootstrap by the operator's own runtime and the arm
is removed from the public route.

### 2. The gate's evidence is a shared secret, not the person's key

The recovery is allowed when `adminBootstrapKey` equals `API_KEY_ADMIN`. That answers "is this caller my
admin?" (auth-posture question 1). It does not show that the caller controls the key the Human was authored
under. It is also the value the seed pipeline holds: `genesis/seeder/src/seed-humans.ts` reads it from
`API_KEY_ADMIN` and sends it for the admin human, and `genesis/scripts/ci/doorway-seed-ensure.sh` documents the
same value as its `ADMIN_KEY`. Anything that can read the deploy environment can pass the gate. The comparison
is plain equality, as the neighbouring admin-bootstrap check is.

The hosted branch's evidence is stronger but still doorway-local: a live intent record naming the same
conductor, app and agent key for exactly that identifier (`AgentProvisioner::provisioned_for`). It proves which
key this doorway generated, not who is asking.

Exit: recovery by proof of control — a signature from the agent key, or the substrate's own recovery flow
(imagodei recovery, `StewardshipGrant` where one person acts for another). The predicate's doc names this as
its successor; nothing enforces the graduation.

### 3. `/admin/hosted-users` has no in-process credential check

`POST /admin/hosted-users` (`http.rs:8579` → `handle_provision_user`, `admin_conductors.rs:355`),
`GET /admin/hosted-users` (`http.rs:8598` → `handle_list_hosted_users`, `admin_conductors.rs:431`) and
`DELETE /admin/hosted-users/{agent}` (`http.rs:8603` → `handle_deprovision_user`, `admin_conductors.rs:479`)
call neither `require_admin` nor any API-key check. Their siblings in the same file do (`handle_list_conductors`
and three others). The dispatch comments describe operator-only access as an ingress property.

Verified by reading: the three handlers from signature to first side effect, and a keyword scan of
`handle_request` between its entry and the route match for any admin gate applied before dispatch — none found.
NOT verified: the ingress and network policy that are supposed to keep these paths off the open web, any
middleware outside `server/http.rs`, and behaviour on a running doorway. Nothing was probed.

What they allow if reachable: provision an app for any identifier, list every hosted identifier with its agent
key and conductor, and uninstall any hosted person's app by agent key. The new
`/admin/provisioning/recover` route does check the admin credential in the handler.

Exit: `require_admin` (or the admin API key) in all three handlers, with a test per handler that an
unauthenticated request is refused before the body is read.

### 4. Two doorways on one conductor pool are not serialised

The intent log's compare-and-set holds within one database. `doorway-alpha` and `doorway-alpha-b` use the same
Mongo server and different databases (`alpha.yaml` `MONGODB_DB: doorway-alpha`; `alpha-b.yaml`
`MONGODB_DB: doorway-alpha-b`) while both receive the same conductor list from `computeConductorUrls` in
`elohim/holochain/Jenkinsfile`. The same identifier registered on both can be provisioned twice. If both pick
the same conductor the second install is refused and reported as a conflict; if they pick different conductors
the person has two agent keys.

Exit: the `hosted-cell-promised` habit's retire-when — hosted provisioning becomes a substrate act performed
by the pool peer, the doorway only relays it, and "one cell per person" is a fold over the DHT rather than a
row in any one doorway's database.

## Also not covered

- A doorway that lost SOME records (not all) is not detected; `store_looks_wiped` catches only an empty store
  over populated conductors.
- If the account rows are lost as well, the operator's recovery rebuilds agent mappings but no identifier
  records. An identifier typed into a form is not evidence of who is asking, so returning people are not
  re-bound to their apps.
- The recovery branches inside `handle_register` are covered by tests of their predicates and of the refusal,
  not by a test that drives the handler.

## Readiness

Items 1 and 3 are bounded and doorway-only; each is a gate plus tests and needs the eight auth-posture
questions answered in the predicate. Item 2 waits on a recovery-by-proof design. Item 4 waits on the native
provisioning work the habit names.
