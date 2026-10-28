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

## CORRECTION 2026-10-04 (later the same day) — the gate did not fire against the real DNA

A live household run (`genesis/a2o/reports/recovery/recovery-gate-proof-20261004/`) showed that the
recovery gate added in `e06b89eb8` NEVER RAN, and that the doorway-phase route was still open on the
success path.

**What this entry and the round-3 report claimed, and why it was wrong.** They said the doorway-phase and
hosted recoveries were reached through an `"Agent already has a Human profile"` error from `create_human`, and
that gating that error branch closed the class. `imagodei::create_human` raises no such error. It is
idempotent (`elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs`, the "Idempotency gate" in
`create_human`): when the calling agent already has a Human it returns that Human's hash and succeeds. The
string exists only in a doc comment. The trace was reasoned from the doorway's own error handling, not from the
zome, and the tests exercised the predicates without ever driving the branch — so nothing checked that the
branch could be taken.

**What was actually reachable, with the round-3 commits applied.** A registration naming the doorway agency
phase, sent to a doorway whose own agent already had a Human, was answered `201`: an account row and a session
token carrying the existing Human's id, and that Human's profile in the body. No credential was asked for.
Supplying the existing Human's id in the request changed nothing. The same held for a hosted registration with
no conductor pool (the singleton fallback), and for a hosted registration whose provisioned agent already had
a Human.

**The fix** (the commit that adds this correction). The registration ASKS before it creates:
`routes::human_binding::bind_human` (`human_binding.rs:184`), called from every arm that creates or attaches a
Human through `bind_registration_human` (`auth_routes.rs:1077`), reads the agent's Human under a per-agent
lock before `create_human`. If one exists, attaching it is a recovery and needs the evidence already defined;
without it the answer is `403 IDENTITY_RECOVERY_REFUSED` and `create_human` is not called. If none exists it
creates, reads again, and treats a Human that is not the one it just sent as a recovery too. The dead
error-string branches are removed. The test fake's `create_human` is idempotent like the zome's, and one test
reads the zome source and fails if that stops being true.

So the gate now fires on EVERY registration after the first on a given agent — which is what "recovery" means
once `create_human` is idempotent. Item 1 below is unchanged: the first one is still ungated.

Not yet confirmed by a live run: that the rebuilt doorway refuses the three requests the run sent.

## UPDATE 2026-10-19 — landed on dev beside the node/device and operator-actuator fixes

- **Item 3, mostly closed by dev.** `POST /admin/hosted-users` and `DELETE /admin/hosted-users/{agent}` pass
  the operator-actuator gate (`auth::is_operator_actuator` + `require_admin_permission`, fb3c12867 / 9facfba30),
  and the merge put `POST /admin/provisioning/recover` behind the same gate as well as its handler check.
  `GET /admin/hosted-users` is still ungated (the gate leaves every GET public) and still lists every hosted
  identifier with its agent key and conductor.
- **The node/device arms no longer scan the pool.** Dev's `AgentProvisioner::find_agent` matched the
  deterministic app id on every conductor and handed a found hosted agent (key, conductor, routing claims) to
  the unauthenticated registration body. The merge replaces it with `AgentProvisioner::hosted_here`, which asks
  only this doorway's record: nobody → the caller's own key; a live record → 403 `IDENTITY_RECOVERY_REFUSED`;
  unknown (log unreadable, store verdict wiped or unknown) → 503 with the provisioning code. Nothing is
  installed or adopted.
- **Node-key bind forgets the record only when the deprovision succeeded** (`routes/node_key_bind.rs`). On a
  failed uninstall the record is kept: it truthfully names the live cell.
- **A node/device registration cannot claim a hosted or bound key** (codex review of 0a7c3f1a5). Registering a
  fresh identifier as `device` with a victim's hosted agent key, then logging in, used to mint a session onto the
  victim's cell: the key is stored with no proof of possession and login resolves it through the registry.
  Registration now refuses (403 `AGENT_KEY_ALREADY_BOUND`) a key the registry maps to an app this doorway hosts,
  or that another live account holds; login refuses (403 `ACCOUNT_KEY_HOSTED_HERE`) a node/device account whose
  key resolves to a hosted app, unless this doorway's record for exactly that identifier names that agent and app.
  The registration response and the stored row now carry the same key (the caller's).

### 5. Proof of possession for a node/device key — the real cure

The guards above refuse keys the doorway can SEE are taken. They cannot refuse a member's own pod key whose
holder has no account here yet: the first registrant to name it gets an account (and, at login, routing) onto
that pod agent. An already-minted JWT naming a key is also refreshed from its own claims and routed by the
chaperone without re-asking whose key it is.

Exit: a node/device registration (and `/auth/refresh`) carries a signature by the claimed agent key over a
doorway-issued challenge naming this doorway and identifier — the same shape `routes/node_key_bind.rs` already
verifies for a hosted person moving onto their node key — and the doorway refuses a key it has no signature for.

## UPDATE 2026-10-28 — ported onto dev, the refusing half dormant behind `DOORWAY_PROVISIONING_STORE_GATE`

The series (`land/hosted-provisioning-1019`, tip `e674f52e8`) was ported onto dev `084fdd742` on branch `port/c-hosted`. Two adaptations change what this page's readers can rely on:

- **The conductor mints the agent key.** Dev's `e8a337195` stopped handing the conductor a key (a supplied key lists every lair seed in one 8 KiB frame; a churned hosting conductor then refuses every install). The record can no longer hold the key before the install is sent. It now holds `absent_before_send`: the conductor answered "nothing under this id" immediately before the install was written. The key the conductor minted is recorded as soon as the install answers, and is the proof from then on. An install whose answer was lost is recognised by `absent_before_send` alone. **Residual:** another writer installing under the same full-hash id on the same conductor inside that in-flight window would be adopted. Only a doorway on a different database registering the same identifier at the same moment can do that, and dev today adopts by a 24-bit app id with no proof at all.
- **Everything that refuses on the store verdict is gated, default off.** With `DOORWAY_PROVISIONING_STORE_GATE` unset, a person with no record is resolved as dev resolved them (an app under the legacy short id is adopted, unreachable conductors skipped, no record written for it). No `PROVISIONING_NOT_READY` or `PROVISIONING_UNRESOLVED` comes from the verdict, and no node/device login is refused for want of a record. The verdict is still computed and published (`/health` `provisioning.storeGate`, `storeVerdict`).

With the gate ON, every boot backfills records from account rows whose agent key a conductor confirms (`AgentProvisioner::migrate_from_accounts`), whatever the verdict, inserting only where no record exists. The record-dependent node/device login check is enforced only once that backfill has completed since boot. The backfill clears `wiped` only when the walk is complete (every conductor answered, every record written) and proved at least one account. `report.complete` now requires the record writes, for the operator's recovery too.

Residuals the gate leaves:

1. **The Human recovery refusal is gated too.** The doorway always asks whether the agent already has a Human before creating one. With the gate off it re-attaches that Human as dev did ("conductor not reset but DB was cleared"), and the account-takeover path this page exists for stays open. With the gate on, the re-attach needs this doorway's record for exactly that identifier (or the operator credential for the doorway's own agent), else 403 `IDENTITY_RECOVERY_REFUSED`; after a database wipe, a person provisioned before the log then cannot recover by re-registering.
2. **Gate off keeps adoption by legacy app id**, the 24-bit collision this page names. It ends when the gate is turned on.
3. **A partial loss is not detected.** The migration clears `wiped` on one proven account; hosted apps no account claims are reported (`appsWithoutAnAccount`) and assigned to nobody, as in the operator's recovery.
4. **Historical node/device accounts.** Backfill writes a record for any account whose key a conductor confirms, including a node/device row that carries a hosted app's key, so with the gate on such a row keeps logging in (`records_agent_for`). Whether it should is the open login-policy question in `hosted-provisioning-stop-damage-held-for-operator`.
5. **Not yet measured on a household.** No live run has exercised a first boot with the gate on.

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
  (This was reachable as a plain SUCCESS, not through an error branch — see the correction above. It stayed
  open until the commit that adds that correction.)

Fix commits: `71ad3524e` (typed faults; an install of unknown outcome is never re-offered), `39d444a87` (the
provisioning intent log), `e06b89eb8` (full-width app ids; adoption only where the conductor's agent key equals
the key recorded for the identifier; both recovery branches gated), and the commit that adds this entry (the
refusal is `403`, not `409`).

## Still open

### 1. The FIRST doorway-phase registration is ungated

`agency_phase` is read from the unauthenticated request body (`auth_routes.rs:1314`). In the `"doorway"` arm
(`auth_routes.rs:1439`) the Human is created on the doorway's own conductor agent with the caller's display
name and, if supplied, the caller's `human_id`. Only the RECOVERY — a registration that finds a Human already
on that agent — is gated (`operator_identity_recovery_authorized`, `auth_routes.rs:1026`, applied in
`bind_registration_human`, `:1077`). On a fresh doorway, whoever sends the first doorway-phase registration
becomes the Human on the operator's agent. The same holds for a `hosted` registration on a doorway with no
conductor pool, which uses the same singleton agent. Every registration after that first one is gated.

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
- `bind_human` is tested against a fake held to the zome, and the predicates and the refusal are tested;
  `handle_register` itself is still not driven end to end by any test.
- Two doorway PROCESSES sharing one conductor agent could both create an identical Human in the same instant
  and neither could tell; the zome does not say whether a call created or found the Human. No deployed shape
  does this. The exit is a coordinator-only change that answers "created" or "existed".
- A `node` or `device` registration makes no conductor call for the Human and writes the caller-supplied
  `human_id` AND `agent_pub_key` straight into the account row and the session token. No evidence is asked for
  that either is the caller's (no signature by the key). With no key supplied it writes a placeholder derived
  from the identifier that no conductor ever authored.

## Readiness

Items 1 and 3 are bounded and doorway-only; each is a gate plus tests and needs the eight auth-posture
questions answered in the predicate. Item 2 waits on a recovery-by-proof design. Item 4 waits on the native
provisioning work the habit names.
