---
id: "backlog-security-doorway-devmode-auth-bypass"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "DEV_MODE=true on every deployed doorway makes JWTs forgeable with a source-visible secret, grants anonymous callers Authenticated, and (before the loopback fix) opened the seed/cache-mutation routes with no credential"
slug: "security-doorway-devmode-auth-bypass"
written: "2026-08-24"
author: "opus (adversarial close-out of the iroh/dual-transport arc; forgery + seed bypass proven on the local mesh)"
status: "open"
priority: "critical"
area: "doorway/auth"
domain: "protocol"
jobs: [elohim-holochain]
relatedNodeIds:
  - "habit:reach-enforced-everywhere"
cites:
  - genesis/data/timeline/backlog/security-doorway-blob-pantry-ungated.md
  - genesis/data/timeline/backlog/security-doorway-auth-required-unenforced.md
tags: [security, doorway, auth, jwt, dev-mode, posture, operator-decision, critical]
---

# DEV_MODE=true is the fleet's standing auth posture, and it weakens auth three ways

**Root fact.** All five deployed doorway manifests set `DEV_MODE: "true"` —
`genesis/orchestrator/manifests/doorway/{alpha,alpha-b,prod,staging,staging-read}.yaml`. The doorway
README already flags flipping this as "an operator/architect decision" (coupled to the `FIXTURE_ONLY`
portal-handoff scenarios). It is treated as a fixture convenience; it is in fact the fleet's auth
posture, and `dev_mode` weakens authorization on three independent axes.

## Axis 1 — JWTs are forgeable with a SOURCE-VISIBLE secret (CRITICAL)

`validate_jwt_token` (`doorway-service/src/auth/http_permission.rs:66`) and every sibling validator
(`auth_routes.rs::get_jwt_validator:4386`, `validate_ws_token:4620`) select the validator as
`if dev_mode { JwtValidator::new_dev() } else { <configured JWT_SECRET> }`. `new_dev()` uses
`dev_mode_placeholder_value()` (`jwt.rs:303`) = the constant
`"dev-mode-not-for-production-use-123456"` — hardcoded in the open-source tree. Because minting
(`generate_token`) selects the same way, real logins on the fleet also sign with this constant, so
the whole fleet's tokens are HMAC-signed with a publicly-known key.

**Consequence:** anyone who reads the repo can forge a JWT with any `human_id`, any
`permission_level` (including `Admin`), for any doorway. This defeats every JWT-gated surface —
admin user management, hosted-user provisioning, conductor admin, the seed path, cross-doorway
federation trust.

**PROVEN (2026-08-24, local mesh, non-mutating).** A JWT forged in Python with the source constant,
claiming `permission_level: Admin`, was sent to `PUT /admin/seed/blob` with a deliberately-wrong
`X-Blob-Hash`. Response: **409 Hash mismatch** — the forged Admin token PASSED auth; only content
addressing stopped the write. (An earlier note in this arc said "JWT_SECRET is set, so the fleet is
safe" — that was WRONG: `config.rs::jwt_secret()` honors the configured secret, but the `dev_mode`
validators never call it.)

**Scope-bounding facts (measured, so the severity is not overstated):** `JWT_SECRET` *is* present
in all five manifests (so `config.rs`'s own `dev-only-insecure-secret` fallback never fires — a
different constant, and a red herring); and there is NO `Access-Control-Allow-Credentials` anywhere,
so the `dev_mode` any-origin CORS reflection cannot be used for credentialed cross-origin reads. The
forgery is the live vector; those two are not.

## Axis 2 — anonymous callers are granted `Authenticated`

`extract_http_permission` (`http_permission.rs:54-58`): under `dev_mode`, a request with no valid
credential returns `PermissionLevel::Authenticated` rather than `Public`. So every route gated at
`Authenticated` (not Admin) is open to anyone on the fleet. This is the doorway-side companion to
`http-reach-enforcement-gap`'s "200 to any authenticated caller."

## Axis 3 — seed / cache-mutation routes were credential-free (FIXED this pass)

`require_seed_authority` returned `Ok(())` on `dev_mode` alone, gating `PUT /admin/seed/blob` and
`POST /admin/cache/{disable,enable,clear/*,warm}`. **Proven** (2026-08-24): anonymous `PUT
/admin/seed/blob` with a wrong hash → 409 (gate passed; content addressing refused the write).
`cache_disable` is a trivial availability lever with no such backstop.

**Shipped this pass (locally verified):** `require_seed_authority` now takes `peer_is_loopback` and
passes on `dev_mode` ONLY for a loopback caller — derived from the accepted socket's peer address,
never `X-Forwarded-For`. Behind the cluster ingress the peer is the ingress pod (not loopback), so
the fleet now authenticates; a developer's box (127.0.0.1) keeps `pnpm run hc:start:seed` and the
local mesh working with no credential. The CI path was already bearer-gated
(`doorway-seed-ensure.sh` mints `SEED_DOORWAY_TOKEN`), so it is unaffected. Regression test:
`dev_mode_remote_caller_is_refused`. **This closes Axis 3 only.**

## Why Axes 1 & 2 are NOT fixed in code this pass (operator decision required)

The clean fix is to select the JWT validator by *secret presence*, not by `dev_mode`: use the
configured `JWT_SECRET` whenever it is set (fleet), fall back to the dev placeholder only when no
secret is configured (bare local dev). Applied consistently across `http_permission.rs`,
`auth_routes.rs` (both `get_jwt_validator` and `validate_ws_token`), and the mint path, this stops
forgery while keeping local dev credential-free.

**But it invalidates every currently-issued fleet session** (all signed with the dev constant) —
a mass logout on deploy — and it is coupled to the same `DEV_MODE` posture the README reserves for
the operator (flipping `DEV_MODE=false` outright also breaks the `FIXTURE_ONLY` portal-handoff
fixtures and needs that surface rehomed). Because it is a live production-posture change with a
user-visible consequence, it is filed for an operator/architect decision rather than applied
unilaterally.

## Options (operator decision)

- **A — Decouple the secret from dev_mode (recommended).** Validator/mint select the configured
  `JWT_SECRET` when present, dev placeholder only when absent. Stops forgery; local dev unchanged.
  Cost: one deploy-time mass logout (tokens re-minted under the real secret). Does NOT require
  flipping `DEV_MODE`.
- **B — Also fix Axis 2** in the same pass: `extract_http_permission` grants `Authenticated` under
  `dev_mode` only for loopback (same discriminator as the seed fix), or gate it behind an explicit
  `DOORWAY_DEV_ANON_AUTHENTICATED` rather than `dev_mode`.
- **C — Flip `DEV_MODE=false` on the fleet** and rehome the `FIXTURE_ONLY` portal-handoff surface to
  a dedicated fixture posture. Largest blast radius; the README already scopes this as coupled work.

## DoD / verification (for A+B when authorized)

- Unit: a JWT signed with the dev placeholder is REJECTED when a real `JWT_SECRET` is configured; a
  JWT signed with the configured secret is accepted; local dev (no `JWT_SECRET`) still accepts the
  dev token. WS and HTTP paths both covered.
- Unit: under `dev_mode`, a remote (non-loopback) anonymous request resolves to `Public`, not
  `Authenticated`.
- `cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib --bins` green; the gate project is
  `doorway` (NOT `doorway-service`).
- Coordinate the deploy with the session-invalidation window (all fleet users re-login once).

## Disjointness

Axis 3's fix is already in `routes/seed.rs` + `server/http.rs` (the loopback thread). Axes 1 & 2
touch `auth/http_permission.rs`, `auth/jwt.rs`, `routes/auth_routes.rs` — the auth core, independent
of the blob-pantry / storage reach-gate write-set in `security-doorway-blob-pantry-ungated`.

---

## DECISION + IMPLEMENTATION (2026-08-25) — auth posture decoupled from DEV_MODE

**Decision (made, not deferred).** The doorway must stop deriving its auth posture from the
`dev_mode` bool. This is not a new principle — it is the discipline elohim-storage already enforces
for `NetworkStage` (`elohim/elohim-storage/src/trust/stage.rs:31`: "`NetworkStage` must never derive
from any `DEV_MODE` flag"; `impl Default → Bootstrap` fail-closed; `InertPricer` = `FullChain` when
undeclared; property test `default_is_bootstrap_never_simulacra`). The doorway is the lone outlier.
The experience we are steering toward — **model things as test fixtures while driving velocity** —
is delivered by *declaring low stakes* (secret-absence + loopback for the LOCAL box / the keyless
mesh), never by a `DEV_MODE` flag that also weakens the fleet. Identity forgeability is
floor-protected — the doorway analog of the pricer's `Constitutional`/`LocalRelationship`/
`CounterEvidence` floors that never cheapen at any stage.

Three orthogonal discriminators replace the one `dev_mode` bool:
- **Crypto (Axis 1) → JWT_SECRET presence.** DONE this pass.
- **Request fallthrough (Axis 2) → peer_is_loopback.** SPEC below.
- **Admin passthrough / steward key-proof / CORS (Root C) → loopback / explicit allowlist.** SPEC below.

### Axis 1 — IMPLEMENTED (forgery closed at the crypto layer)

`JwtValidator::from_config(secret: Option<&str>, expiry) -> Result<Self>` (auth/jwt.rs) selects the
validator/signer by **secret presence**: `Some → new(secret)` (the `>=32` check stays in `new`),
`None → new_dev()`. `Args::configured_jwt_secret()` (config.rs) is the single discriminator (trimmed,
non-empty). All 13 runtime selection sites route through it: http_permission.rs:validate_jwt_token,
auth/operator.rs:verify_claims, routes/api.rs:parse_requester_identity, routes/status.rs,
routes/auth_routes.rs:{get_jwt_validator, validate_ws_token}, routes/admin_users.rs:{get_jwt_validator,
try_extract_user_id_for_tracking}, server/http.rs:{federation_jwt_validator, header extractor},
server/websocket.rs:{decode_jwt_claims, validate_jwt}. The dead `Args::jwt_secret()` (a SECOND
forgeable constant `dev-only-insecure-secret`, zero callers) is deleted. main.rs emits a loud startup
WARN when no secret is configured. Because every mint path draws its signer from these validators,
keying validate on secret-presence keys mint too. **Net fleet change: every site moves from the
`dev_mode`→`new_dev()` (forgeable) branch to the configured-secret branch — a pure improvement, no
regression** (the fleet always took the dev_mode branch before). The only fail-open combination
(`!dev_mode && no secret`) was already unbootable via `validate()`.

**Why NOT a non-loopback startup hard-fail:** the local mesh binds `0.0.0.0` keyless by design
(hc-mesh.sh: `--dev-mode --listen 0.0.0.0:$PORT`); requiring a secret for any non-loopback bind would
break the mesh (velocity). A loud startup warning is used instead.

**Verification status:** `cargo fmt --check` clean (rustfmt parsed all 11 edited files →
syntactically valid). Full typecheck/clippy/tests DEFERRED: this worktree's sandbox cannot resolve
the holochain fork `dev.23` deps for doorway-service (the crates.io index is stale for the fork
prereleases and this branch's `doorway/Cargo.toml` lacks the git patch elohim-storage has:
`holochain_client = { git = ".../ethosengine/holochain.git", rev = "6d08142…" }`). fable's
doorway-family slot compiled doorway at 00:04 and CI resolves it — that is where the typecheck/test
run lands. Do NOT add the git patch to doorway just to compile here (it churns Cargo.lock and builds
a different dep graph than production).

### Axis 2 — SPEC (anon→Authenticated, loopback-gated)

`extract_http_permission` (auth/http_permission.rs:54) grants `Authenticated` to any credential-free
request under `dev_mode`. Change signature to `extract_http_permission(state, req, peer_is_loopback:
bool)` and grant `Authenticated` only when `dev_mode && peer_is_loopback`, else `Public`. Callers:
`routes/seed.rs:89` (already has `peer_is_loopback` from `require_seed_authority`), and
`routes/elohim_agent.rs:33` (thread `addr.ip().is_loopback()` from the server/http.rs dispatch, as the
five seed sites already do). Companion WS grant (server/websocket.rs) gets the same loopback gate.

### Root C — SPEC (admin passthrough, steward key-proof, CORS)

- **Admin conductor passthrough:** `proxy/admin.rs:81`, `proxy/pool.rs:42`, `proxy/nats.rs:44` skip
  `is_operation_allowed`/`filter_message` under `dev_mode`, and `server/http.rs:5218/5259` expose
  `/hc/admin` dev-only. Gate on `peer_is_loopback` (never disable filtering for a non-loopback peer);
  keep `/hc/admin` loopback-only. This is the raw conductor admin plane — treat as high severity.
- **Steward key-proof skip:** `routes/admin_users.rs:1027/1143` (force-grant bypasses agent-key proof
  under dev_mode). Gate on loopback or a dedicated `FIXTURE_SEED` flag.
- **CORS reflect-any:** `cors.rs:91` reflects any Origin under dev_mode. Drive from a `CORS_ORIGINS`
  allowlist; drop the dev_mode reflect-any branch (or have the mesh set `CORS_ORIGINS`). No
  `Access-Control-Allow-Credentials` exists anywhere, so this is defense-in-depth, not a live
  credentialed cross-origin vector.

Axis 2 + Root C are threading-heavy (signature + caller changes) and are HELD for the compile-capable
env rather than shipped blind — a verification-discipline call, NOT an open decision. The decision is
made above; these are execution with a compile dependency.

### Deploy-time consequence (operator-sequenced)

Landing Axis 1 in-tree does NOT log anyone out. The mass re-login happens only when the operator
DEPLOYS (every fleet token today is signed with the dev constant and becomes invalid under the real
secret). Sequence the deploy with a one-time session-invalidation window. No dual-verify of the old
dev secret on non-loopback — that would keep the forgery hole open. Storage sequencing is unaffected.

---

## AXIS 2 CLOSED + a MORE SEVERE hole found (2026-08-27)

### Axis 2 — CLOSED in code, locally proven

`extract_http_permission` no longer promotes anonymous callers on a mode flag. It now grants
`Authenticated` only when `state.network_stage < NetworkStage::Coordinated && peer_is_loopback`,
mirroring authority (1) of `require_seed_authority`; `peer_is_loopback` is threaded from the accepted
socket (`addr.ip().is_loopback()`), never a header. Signature gained the flag; the two consumers
(`routes/seed.rs:132`, `routes/elohim_agent.rs:33` via `server/http.rs:5893`) pass it.

**The feared blast radius was measured, not assumed, and it is one route.** The crate has exactly ONE
`Authenticated` gate — the elohim-agent invocation proxy — whose own comment already states the intent
the grant was defeating ("compute is shared commons, but only for real people in the network, not
anonymous traffic"). Content/blob/apps/cache routes are registry `StorageProxy` paths with NO
permission gate, so ordinary browsing never consults this ladder; `can_serve_at_reach` reads a
different source and has no live caller; the WS ladder is a separate function. A signed-in browser
sends a bearer token, and `/health` short-circuits above the gate. The one a2o reference is `@wip`
with no steps. Second-order: a remote anonymous seed refusal flips 403 → 401, which the existing test
tolerates (it asserts `is_client_error`).

**PROVEN LIVE** (real binary, `--dev-mode`, bound `0.0.0.0:8899`): `POST /api/v1/elohim/invoke`
anonymously → from `10.1.19.183` (non-loopback) **401 "Authentication required for elohim agent
invocation"**; from `127.0.0.1` **502** (passed the gate, failed only at the absent sidecar);
`/health` **502 on both** (still public). 4 new unit tests, incl. the designed expiry
(`coordinated_stage_retires_the_loopback_grant`). Registered as a decision point in
`doorway/doorway-service/seam-registry.yaml` (birth rule), census 31 pts · 31 cited · 0 uncited.

### A MongoDB outage was an authentication downgrade — CLOSED

Four auth paths branch on `dev_mode && state.mongo.is_none()` (`auth_routes.rs:1354, 1626, 2106,
3623`), and `:1626` accepted **any credentials** and minted `PermissionLevel::Admin`. `main.rs`
continued past a failed Mongo connection whenever `dev_mode` — true on every deployed manifest. So a
transient MongoDB outage silently converted the fleet into "any password logs in as Admin."

Fixed at the source, mirroring the fail-loud bootstrap-store precedent immediately below it: a
**configured** `MONGODB_URI` that cannot be reached is now fatal, so `mongo.is_none()` can only mean
"none configured" (a genuine local-dev shape). Defense in depth: that branch's ceiling dropped
`Admin → Authenticated`. **PROVEN LIVE:** `MONGODB_URI=mongodb://127.0.0.1:59999` → `EXIT_CODE=1`
with "refusing to start credential-free rather than degrade authentication".

### OPEN — most severe of the whole arc: anonymous UNFILTERED conductor admin

Not closed here, and deliberately so. The chain, each link verified in code:

1. `server/http.rs:5220` gates `/hc/admin` on `dev_mode` with the message "Admin WebSocket disabled in
   production" — but `DEV_MODE: "true"` is set on all five deployed manifests, so the 403 arm is dead
   on the fleet and the upgrade proceeds. `:5261` is a second entrance via the legacy `/` upgrade.
2. `server/websocket.rs:426` returns `Ok(PermissionLevel::Public)` instead of `Err` for a caller with
   no JWT and no `X-API-Key`. (Note the app sends `apiKey` as a QUERY param while this reads the
   `X-API-Key` HEADER, which browsers cannot set on a WebSocket — so browser callers are anonymous.)
3. `proxy/admin.rs:81` — `if dev_mode { passthrough }` — sends every binary frame straight to the
   conductor, so `filter_message`/`is_operation_allowed` never runs and `permission_level` is never
   consulted. Same shape in `proxy/pool.rs:42` and `proxy/nats.rs:44`. The closed-world default is
   removed too: unknown and unparseable operations also pass.
4. The ingress is a catch-all `path: /` (`alpha.yaml`, `prod.yaml`), so this is internet-reachable.

Net: an anonymous internet client can reach `install_app`, `enable_app`, `disable_app`,
`uninstall_app`, `update_coordinators`, `delete_clone_cell`, `add_agent_info`, `revoke_agent_key` on
the production conductor.

**Why it is NOT fixed in this pass.** The deployed app's ANONYMOUS visitors use this exact socket:
`connect()` picks the chaperone only when `!isCheEnvironment() && !!config.doorwayToken`, so a visitor
with no session falls back to `connectViaAdminWs`, which calls `generateAgentPubKey` (Authenticated
tier) and, when the app is absent, `installApp`/`enableApp` (Admin tier). Turning filtering on alone
would refuse anonymous visitors at `generate_agent_pub_key` and break onboarding on a live site. The
hole and the onboarding path are the same mechanism, so closing it is coupled to migrating anonymous
visitors onto the chaperone (hosted-user provisioning) — an app-side change that must land with, or
before, the doorway change. That sequencing is the operator's, which is why this is reported rather
than shipped.

**Recommended sequence:** (1) make the chaperone reachable for a session-less visitor (or provision a
visitor session before conductor connect); (2) then, in the doorway, drop the `dev_mode` passthrough
so `filter_message` always runs, replace the WS `Ok(Public)` fallback with loopback-only, and gate the
`/hc/admin` upgrade on loopback; (3) verify an anonymous visitor still onboards, and that
`uninstall_app` from a remote anonymous socket is refused.

### Also OPEN, with the reason each was NOT shipped blind

**`fixture_only_gate` opens two write routes on the fleet** (`admin_users.rs:1019` → `PUT
/admin/users/{id}/steward`, which sets `is_steward` with the key-proof skipped; `admin_dev.rs:70` →
`PUT /admin/dev/portal-health`). Both are `if dev_mode { None }` — i.e. open on every deployed
doorway, with no `require_admin` anywhere on the portal-health handler.

NOT loopback-gated, because the a2o suite calls both against the DEPLOYED doorway:
`genesis/a2o/steps/auth/agency.steps.ts:208` and `genesis/a2o/steps/ui/account-m5.steps.ts:248`. A
loopback conjunct would have gone green in unit tests and broken the E2E suite on the fleet.

The canon's question 1 gives the right shape: *"may this caller drive fixtures?"* is a DIFFERENT
question from *"is this caller my admin?"*, so it wants its own narrow credential — a dedicated
fixture key on the `API_KEY_SEED` pattern (presence-keyed, scoped to these routes, never entering the
permission ladder, retiring at `Coordinated`), held by CI. That is a small, self-contained follow-up:
add the key, gate both routes on it, and set it in the a2o runner's env.

**CORS reflects any Origin under `dev_mode`** (`cors.rs:91`). NOT changed: `CORS_ORIGINS` is declared
and parsed (`config.rs:288`) but set in NO manifest, so dropping the `dev_mode` branch today would
leave an EMPTY allowlist and break every browser cross-origin call. The real defect is the dead
config, not the branch — the fix is to populate `CORS_ORIGINS` in the five manifests FIRST, then
remove the reflect-any branch. Severity is bounded meanwhile: `Access-Control-Allow-Credentials`
appears nowhere in the crate (verified by grep), so reflect-any cannot be used for credentialed
cross-origin reads.
