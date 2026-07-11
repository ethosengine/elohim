---
title: "Admin Key Lifecycle — Dev to Production"
id: admin-key-lifecycle-dev-to-production
status: active
created: 2026-06-03
tier: design-spec
topic: [admin, api-key, x-api-key, jwt, bootstrap-key, doorway, storage, rea-compute-commitment, delegates-compute, revocation, standing, production-readiness, secrets, threat-model]
cites:
  - genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md
  - rea-compute-commitment-primitive | rea-compute-commitment-primitive | sha256:3ea123e3a9796449 | path: genesis/docs/architecture/rea-compute-commitment-primitive.md
  - rea-compute-substrate-native-roadmap | 2026-05-28-rea-compute-substrate-native-roadmap | sha256:64e5ffe3b8756e6e | path: genesis/docs/superpowers/plans/2026-05-28-rea-compute-substrate-native-roadmap.md
  - doorway/doorway-service/src/auth/api_key.rs
  - doorway/doorway-service/src/routes/auth_routes.rs
  - doorway/doorway-service/src/routes/admin_users.rs
  - genesis/orchestrator/manifests/doorway/prod.yaml
depends_on:
  - genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md  # §1 names the REA compute-commitment primitive that displaces the static admin key
---

# Admin Key Lifecycle — Dev to Production

What happens to the admin keys when this substrate goes to production. This is the canonical
articulation: the dev posture is correct *for dev*, the static-bearer model is unacceptable *for
prod*, and the production path is composition with the REA compute-commitment primitive — not key
rotation.

## 1. Dev posture today

In the simulated dev environment there is **one consistent admin secret used everywhere**: the
doorway process secret (`api_key_admin`, env `API_KEY_ADMIN`, config.rs:74), its k8s Secret field
`api-key-admin` (`genesis/orchestrator/manifests/doorway/{alpha,alpha-b,staging,staging-read}.yaml`),
the seeder copy presented as `adminBootstrapKey` at `/auth/register`, and the E2E copy
(`E2E_ADMIN_BOOTSTRAP_KEY`, fallback the well-known dev string) are **the same value**.

**Consistency-over-rotation is the correct dev invariant** (operator directive 2026-06-03). The dev
substrate is simulated; there is no adversary to rotate against, and divergence is the real hazard —
two Jenkins credential IDs (`storage-api-key-admin`, `doorway-admin-bootstrap-key`) feeding the same
logical secret will silently 403 (`ADMIN_KEY_REJECTED`) the moment they disagree. So: one value,
everywhere, deterministically. Rotation buys nothing here and costs reproducibility.

**Handling rules (in force):**
- **No-echo.** The admin/auth/jwt values are never echoed to logs, console, or test output. Scripts
  that previously printed them (the echo pattern) are forbidden. Refer to secrets by `file:line` or
  credential/Secret **name** only — never by value.
- **`withCredentials`.** Browser-origin admin calls do not carry the static key; the admin surface
  is reached via the JWT minted at bootstrap (Authorization: Bearer), with credentialed CORS scoped
  to the operator origin. The static key is a server-side / CI-side secret, not a browser secret.

**Leak-and-fix history.** An echo pattern in a seeder script leaked the dev admin value to build
logs; fixed 2026-06-03 by removing the echo and codifying the no-echo rule above. (Dev value only;
no prod secret was ever in a value-printing path.)

## 2. Inventory — names and gates (no values)

| Key NAME | Definition (file:line / Secret) | Presented as | Gates | Trust model |
|---|---|---|---|---|
| `API_KEY_ADMIN` / `api_key_admin` | config.rs:74; Secret `api-key-admin`; Jenkins `doorway-admin-bootstrap-key` | env (server); wire `adminBootstrapKey`; `X-API-Key` (WS/agent) | Admin grant at `/auth/register` + WS/agent Admin level | Static process-wide bearer; no scope/expiry/rotation |
| `API_KEY_AUTHENTICATED` / `api_key_authenticated` | config.rs:70; Secret `api-key-authenticated` | `X-API-Key` | Authenticated (non-admin) on WS + agent proxy | Static shared bearer |
| `X-API-Key` (header) | auth/api_key.rs:4; read websocket.rs:411, elohim_agent.rs:68 | `X-API-Key` | maps value → Admin/Authenticated/Public | Static bearer match (constant-time) |
| `adminBootstrapKey` (wire) / `admin_bootstrap_key` (Rust) | auth_routes.rs:141 | `/auth/register` body field | one-time Admin promotion at registration | Per-registration capability grant (== `API_KEY_ADMIN`) |
| `E2E_ADMIN_BOOTSTRAP_KEY` | a2o operator-onboarding.steps.ts; Jenkinsfile | `adminBootstrapKey` body | mints Admin JWT for E2E operator persona | Static bearer (copy of admin key) |
| `JWT_SECRET` / `jwt-secret` | config.rs:62; Secret `jwt-secret` | server-internal | signs/verifies every session JWT incl. the Admin claim `require_admin` checks | The real session-auth root |

Enforcement surface today: `/admin/users/*` is JWT-Admin-gated (`require_admin`, admin_users.rs:597).
The other operational `/admin/*` routes (federation, pipeline, cache, conductors, capabilities,
seed/blob) and **all** of elohim-storage's `/admin/*` (http.rs:1149) have **no presented-credential
gate** — they trust network/operator-locality. (`admin_url`/`HOLOCHAIN_ADMIN_URL` is the conductor
admin socket, not an API key — listed only to disambiguate.)

## 3. Threat-model delta: dev → prod

The dev invariant (one omnipotent static bearer, everywhere) becomes **unacceptable in production**:

- **Omnipotence.** A single process-wide secret = full Admin, no per-caller scoping. Possession is
  authority; there is no "this CI agent may only republish *these* EPRs at *this* reach."
- **No expiry / no rotation propagation.** Compromise requires a "rotate the key everywhere"
  scramble across doorway pods, Jenkins creds, seeders, and E2E — racy and lossy.
- **No on-chain standing or audit trail.** The substrate cannot witness who acted with admin
  authority or whether they were entitled to. The doorway edge holds the trust; the substrate is blind.
- **Locality-only gating is a prod gap.** Ungated `/admin/*` routes (storage *and* most of doorway)
  relying on network isolation are a real prod exposure once the network perimeter is the internet.
- **Committed placeholders.** prod.yaml carries `CHANGE-ME-...` placeholders for `api-key-admin`/
  `api-key-authenticated`/`jwt-secret`; these must never be the live prod values, and the prod
  secret material must not live in repo `stringData` at all (see §6).

## 4. Production path — composition with the REA compute-commitment primitive

The static admin key is **explicitly slated for displacement**, not hardening. The replacement is one
substrate primitive: a **`Mishpat::Commitment` with action `delegates-compute`** — a bounded,
reciprocal, on-chain, revocable delegation from an operator-steward (provider) to a named service
agent (recipient), with every privileged action carrying `bounded_by: <Commitment CID>` the substrate
validates (`genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md` §1;
canon `genesis/docs/architecture/rea-compute-commitment-primitive.md` §6 names the X-API-Key admin
grant as the anti-pattern this displaces).

Each current usage maps to a commitment-bounded replacement:

| Current usage | What it gates | Commitment replacement | Event class / scope |
|---|---|---|---|
| `api_key.rs` `admin_key` → `Admin` | destructive admin/dashboard ops | `delegates-compute`, operator-steward → operator/dashboard agent | admin scopes; revocation via Mishpat |
| Deploy/CI mutation w/ admin key | republishing bundles | `delegates-compute`, steward → `deploy-svc-agent` | `republish-epr`, `bounded_by` checked at publish (seed §2) |
| `api_key.rs` `authenticated_key` → `Authenticated` | normal-user API access | JWT agent identity (already the primary path) | not a delegation — collapses into agent-key/JWT auth |
| `adminBootstrapKey` bootstrap | minting the first admin | bootstrap Commitment with `epr_scope: ["*"]` | even first-publish carries a `bounded_by` (canon §6) |

**Enforcement points become substrate, not edge.** Doorway stops being the authority and returns to
its documented role — a CDN edge projecting DHT truth (`doorway/CLAUDE.md` Trust Model). The authority
decision moves onto the notarized substrate: given any privileged `EconomicEvent`, walk `bounded_by`
→ Commitment, verify it is active, in-scope, within reach-ceiling and rate, key-rotation-current, and
not-revoked (the 7-check `bounds_validator`, roadmap Phase A, landed 2026-05-28). Storage's `/admin/*`
operator-local endpoints likewise gate on a presented `bounded_by` rather than network locality.
**Revocation is real:** the steward revokes one Commitment and every subsequent referencing event
fails validation — no rotate-everywhere scramble. **Standing compounds:** bounds violations emit
weighted `FeedbackSignal`s aggregated into a per-agent `StandingScore`, and future delegation is gated
on prior standing — authority from a notarized track record, not possession of a secret.

## 5. Migration stages (and the gate between each)

1. **Dev key (now).** One consistent static `API_KEY_ADMIN` everywhere; no-echo + withCredentials in
   force. **Gate to advance:** the silent-drop / dead-endpoint inconsistencies in the usage map are
   fixed (camelCase `adminBootstrapKey` everywhere; non-existent `/auth/api-keys` removed; one
   canonical Jenkins credential ID).
2. **Scoped keys per surface.** Split the omnipotent admin key into per-surface server-side secrets;
   gate the currently-ungated `/admin/*` routes (doorway operational + storage) on a presented
   credential, not network locality. **Gate to advance:** no route grants more authority than its
   surface needs; prod secrets sourced from sealed-secrets, never repo `stringData`.
3. **Commitment-backed delegation.** Stand up named service-agent keypairs; operator-steward authors
   one `delegates-compute` Commitment per (provider, recipient, scope) with explicit bounds; callers
   sign + emit `EconomicEvent`s referencing it; `bounds_validator` enforces at the publish/admin
   boundary. **Gate to advance:** every privileged action carries a validated `bounded_by`; revocation
   and standing exercised end-to-end in E2E.
4. **Key retirement.** Delete the `X-API-Key` → `Admin`/`Authenticated` paths and the
   `adminBootstrapKey` register promotion; `JWT_SECRET` remains (session-auth root), admin authority
   is entirely commitment-bounded. **Gate:** no live caller depends on the static key; the
   pattern-hunter absorption backlog (roadmap Sprint 2) is empty for admin surfaces.

## 6. What never ships to production

- The single omnipotent static admin bearer as the *authority* mechanism (it may survive only as a
  retired/disabled code path until §5.4).
- Any committed key/jwt **value** in repo `stringData` — prod `api-key-admin`/`api-key-authenticated`/
  `jwt-secret` come from sealed-secrets only; the `CHANGE-ME-...` placeholders must never be live.
- The E2E fallback dev key (`'test-admin-key'`) — never a prod default.
- Ungated `/admin/*` routes relying on network locality as their only gate.
- Any echo of a secret value to logs/console/test output (no-echo rule, §1).
- Two diverging Jenkins credential IDs for one logical secret (divergence → silent admin 403).
