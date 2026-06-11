---
title: Jenkins Seed Bearer-Gate — Stage A authorization for CI seeding (closes perimeter hole #2)
id: jenkins-seed-bearer-gate-plan
status: landed
class: protocol-canonical
domain: D8
sprint: unranked — born 2026-06-11; Stage A of the CI-substrate-authorization ladder
cites:
  - genesis/data/timeline/backlog/security-ci-substrate-authorization-grant-coherence.md
  - che-network-agency-arc-design | the agency ladder this borrows — jenkins is the same non-human actor shape as the Che agent; same DoorwaySessionClient identity-through-doorway pattern | sha256:d73e30ea0a205c13 | path: genesis/docs/superpowers/specs/2026-06-10-che-network-agency-arc-design.md
  - genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-23-doorway-access-tier-patterns.md
---

# Jenkins Seed Bearer-Gate — Implementation Plan (Stage A)

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development, task-by-task,
> two-stage review per task. Native Rust gates use `RUSTFLAGS=""` + the doorway cargo-pool slot.

**Goal:** Close perimeter hole #2 from the CI-substrate-authorization concern — doorway
`/admin/seed/*` (and the mutating `/admin/cache/*` routes) accept writes by **network position
alone, no credential**. Give CI seeding **identity + audit** (Stage A of the arc ladder): a
registered `jenkins-ci` account authenticates, the seeder sends a bearer, the doorway fails closed
without one. Local dev seeding stays unbroken (the dev-mode gate). **Out of scope:** perimeter hole
#1 (jenkins→conductor-WS 8444/8445 netpol — the brokered-conductor-surface task), and bounded
standing (Stage C delegates-compute).

## Why this is the honest Stage A, settled (not X-API-Key)

`doorway/doorway-service/src/routes/elohim_agent.rs::extract_http_permission` is the established
gate: **JWT (Authorization: Bearer) → X-API-Key → dev-mode fallback → Public**. The seed routes
call it nowhere today (`http.rs:2338-2358` dispatch straight to handlers; `handle_seed_blob` only
reads `X-Blob-*` headers). Reuse the helper — but the **credential choice is JWT, deliberately**:
an X-API-Key is a shared secret with *no identity and no audit of who acted* — it is the exact
"X-API-Key admin grant" the REA compute-commitment primitive exists to displace (gospel:
`project_rea_compute_commitment_primitive`). A JWT from a `jenkins-ci` account carries a login
event and an actor — which is the entire point of Stage A ("identity + audit, not yet bounded
standing"). The gate is **named for the authority it requires**, resolved today via
`PermissionLevel::Admin` (the transition-window fallback `auth/operator.rs` explicitly sanctions),
so Stage C swaps the resolution to the `operate-doorway`/`seed-content` operator capability in one
internal change without touching call sites or the seeder.

## P2P gate (no new entity)

The jenkins-ci authority is, at Stage C, an `operate-doorway` `Mishpat::Commitment` (Notarized,
Category A — exists, `auth/operator.rs`). At Stage A it is a registered account + Admin permission
(existing). **Zero new DHT entry types, zero new tables.** Session/permission are operational
(Category C). This slice adds an auth *check* and a seeder credential, not a data entity.

---

## Task 1: Rust — gate the seed + cache-mutation routes (TDD)

**Files:** `doorway/doorway-service/src/routes/seed.rs` (or a small `seed_auth` helper module),
`doorway/doorway-service/src/server/http.rs` (dispatch sites ~2338-2358), tests in the doorway crate.

- [x] **Investigate first (journal one line):** confirm `extract_http_permission` is reusable as-is
      from `http.rs` dispatch (it lives in `elohim_agent.rs` — promote it to a shared `auth` helper
      if cleaner, or call through; do NOT duplicate the JWT/api-key/dev-mode ladder). Decide the
      required level: **`Admin`** (seeding mutates the projection — elevated, matching the legacy
      god-flag; named `require_seed_authority` with a doc-comment that this resolves to the
      `seed-content` operator capability at Stage C).
- [x] `require_seed_authority(state, req) -> Result<(), Response>`: returns 401 (no/invalid bearer)
      or 403 (authenticated but under-privileged) on failure; Ok when level ≥ Admin OR dev_mode.
      **Dev-mode MUST pass** (local hc-start seeding unbroken — assert this in a test).
- [x] Call it at the top of `handle_seed_blob` and the mutating cache routes
      (`/admin/cache/disable|enable|clear/*`); leave read-only `/admin/cache/stats` and the
      `HEAD /admin/seed/blob/{hash}` existence check at their current posture unless the concern
      names them (it names the write path — gate writes, keep idempotent existence-probes open;
      journal the decision).
- [x] Tests: (a) dev-mode → blob upload 200 without auth (local-stack safety, prove FIRST);
      (b) non-dev + no bearer → 401, body never reaches storage; (c) non-dev + valid Admin JWT →
      200; (d) non-dev + valid non-admin JWT → 403. Use the crate's existing JWT test helpers
      (`auth/jwt.rs` test constructors). `RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/jenkins-seed-target
      cargo test -p doorway-service seed` (the /tmp slot per the container fingerprint quirk);
      `cargo clippy --tests -- -D warnings`, `cargo fmt --check`.
- [x] Commit.

## Task 2: Seeder — authenticate and send the bearer

**Files:** `genesis/seeder/src/doorway-client.ts` (the seed call ~:468), seeder config/env wiring.

- [x] **Decision (journal):** consume `@elohim/identity` `DoorwaySessionClient` (the next honest
      consumer of this week's SDK work — retires the seeder's hand-rolled auth too) OR add a minimal
      `login()` to the seeder's existing client. Prefer the SDK client IF the seeder's build
      (tsx/Node) consumes it cleanly via the proven `/core` path + workspace dep (a2o precedent);
      if the seeder's module setup makes it heavy, add the minimal login and FILE the full migration
      as backlog (`backlog/seeder-onto-doorway-session-client.md`) — do not over-widen this slice.
- [x] On seed start (non-dev target): authenticate with `SEED_DOORWAY_IDENTIFIER` /
      `SEED_DOORWAY_PASSWORD` (env; Jenkins secrets) → hold the JWT → add
      `Authorization: Bearer <jwt>` to the `/admin/seed/blob` PUT (and any other gated admin call
      the seeder makes — grep the client for `/admin/`). **Dev/local path:** if no creds are set,
      proceed without a bearer (dev-mode doorway accepts it) — local `pnpm hc:start:seed` unchanged.
- [x] Handle 401/403 from the gate with an actionable error (creds missing/expired/under-privileged
      → name the env vars), not a raw fetch failure.
- [x] Seeder unit/typecheck/lint green; a dry local seed against the dev stack still works
      (no creds, dev-mode). Commit.

## Task 3: Provision the jenkins-ci account + Jenkins wiring

**Files:** account provisioning (how alpha humans/admins are seeded — `genesis/seeder/src/seed-humans.ts`
/ `seed-test-admin.ts`; **note the known `admin_bootstrap_key` snake_case bug** — memory
`project_sprint_branch_not_orchestrator_indexed` / the seed-test-admin backlog: the wire field is
`adminBootstrapKey`), `genesis/Jenkinsfile` (the seed stages + blob upload ~:1829).

- [x] Provision a `jenkins-ci` doorway account with Admin permission on alpha — via the admin
      bootstrap path (camelCase `adminBootstrapKey` — verify the fix landed or fix it here) or the
      humans-seed path; identifier e.g. `jenkins-ci@alpha.elohim.host`. Document where the password
      lives (Jenkins credential store id).
- [x] `genesis/Jenkinsfile`: inject `SEED_DOORWAY_IDENTIFIER`/`SEED_DOORWAY_PASSWORD` into the seed
      stages via `withCredentials` (NEVER argv — the Jenkinsfile heredoc/secret discipline; secrets
      via `withEnv`/`withCredentials`). Update the stage comment that documents the now-authenticated
      seed path.
- [x] **Manifest:** ensure the alpha doorway has `API_KEY_ADMIN`/JWT secret configured so non-dev
      bearer validation works (it must already, for login to function — verify, don't assume; the
      gate needs `jwt_secret` set, which `config.rs:287` already requires in non-dev).
- [x] Verify reachable end-to-end where possible without a live deploy: the genesis pipeline is
      operator-merge-triggered; this plan lands the code + Jenkinsfile change, and the FIRST
      authenticated seed run is observed on the next genesis build (note it as the live-verification
      gate — CLAIMED until that build is green). Commit.

### Task 3 — LANDED 2026-06-11 (repo side). What shipped + operator-activation block.

**What the repo now contains (no live infra touched):**

- **Single gated call-site identified + wired.** The genesis pipeline hits the bearer-gated doorway
  route (`PUT /admin/seed/blob`) in **exactly one** place: `substrate-verify.sh upload_one()`. Every
  TS seeder stage in `genesis/Jenkinsfile` was audited — `seed-sqlite.ts` (the genesis content seed)
  PUTs blobs to **elohim-storage** `/blob/{hash}` (:8090), NOT the doorway admin route; the other TS
  stages (`seed-humans/presences/accounts/commitments/operator-bindings/projections/stewardship`)
  call ungated REA/commitment/human routes only. `seed.ts` (the one `readSeederCredentials()`/`login()`
  consumer, Task 2) is **not invoked** anywhere in the genesis pipeline — it is the local-dev
  `hc:seed` path. **Conclusion: no TS seeder stage needed credential wiring; adding it would have
  bloated the size-limited Jenkinsfile for a route those stages never touch.**

- **`substrate-verify.sh`** `upload_one()` now sends `Authorization: Bearer ${SEED_DOORWAY_TOKEN}`
  **only when the token is non-empty** (conditional curl-header array). Dev/local runs leave it empty
  → no header → dev-mode doorway accepts (`require_seed_authority` dev-mode pass, Task 1).

- **`doorway-seed-ensure.sh`** (new; supersedes the short-lived `doorway-seed-login.sh`):
  **self-provisions** the seed actor in-pipeline, then mints its JWT. It reads `ADMIN_KEY` (the existing
  `doorway-admin-bootstrap-key` = doorway `API_KEY_ADMIN`), **derives the account password from it**
  (`sha256sum`, dependency-free — no openssl, matching substrate-verify.sh's toolset), idempotently
  **registers** the actor (`POST /auth/register` with `adminBootstrapKey` = ADMIN_KEY → Admin in one
  call; **200/201 created AND 409 USER_EXISTS both succeed**, mirroring `seed-humans.ts`'s 409-via-login
  precedent), then **logs in** and prints ONLY the JWT. Empty/unset `ADMIN_KEY` → empty stdout, exit 0
  (unauthenticated dev path); register/login rejected or doorway unreachable → exit 1 (hard CI fail,
  with a diagnostic that distinguishes the two). PW/ADMIN_KEY are never echoed. Decision:
  **pre-minted-token, not in-script login** in `substrate-verify.sh` — keeps that careful script
  untouched except for the conditional header, and isolates the secret-bearing provision+login into a
  tiny dedicated helper.

- **`genesis/Jenkinsfile`** `uploadBlobContentStage()` mints the token in `resolveSeedDoorwayToken()`
  via `withCredentials([string(credentialsId: 'doorway-admin-bootstrap-key', variable: 'ADMIN_KEY')])`
  (the **same credential the seed-admin stages at ~1583/1621 already bind** — no new credential) → runs
  the ensure helper with `ADMIN_KEY` in env (`withEnv`, never argv) → passes `SEED_DOORWAY_TOKEN` to
  `substrate-verify.sh` via `withEnv`. Heredoc-free (both `sh` calls invoke external scripts); both
  helpers are top-level `def`s (own CPS methods — no pressure on the 64KB pipeline-block limit).

**OPERATOR ACTIVATION — now IN-PIPELINE and self-healing. Zero new operator steps.**

Provisioning the `jenkins-ci` seed actor is no longer a manual one-shot curl, and there is **no new
Jenkins credential to create**. On every genesis run `doorway-seed-ensure.sh` registers the actor (if
absent) from the existing `doorway-admin-bootstrap-key`, promotes it to Admin via `adminBootstrapKey`,
and logs in with a password **derived** from that same key — so the account self-heals if the doorway
DB is ever wiped. The register-on-conflict path (409 USER_EXISTS) makes re-runs idempotent.

The **only** remaining precondition (already satisfied — the seed-admin stages at ~1583/1621 depend on
it): the `doorway-admin-bootstrap-key` Jenkins credential exists and equals the **alpha doorway's
`API_KEY_ADMIN`** (the `api-key-admin` Secret). `config.rs:287` already requires `jwt_secret` in non-dev
(boot fails otherwise), so login/JWT-validation works by construction on a healthy alpha doorway.

**Password-derivation rationale:** the actor password is `sha256(ADMIN_KEY + ":jenkins-ci-seed-actor")`,
truncated to 48 chars. No password is stored anywhere — only an `ADMIN_KEY` holder can derive it, and
`ADMIN_KEY` is already a total-admin secret, so the seed actor adds **zero marginal exposure**.

**Admin-key-rotation caveat:** doorway `register` does NOT reset a password on a 409 conflict, so after
rotating `ADMIN_KEY` the derived password changes but the stored hash does not → login would 401. The
one-time fix: **delete the `jenkins-ci@alpha.elohim.host` account once** after rotating; the next genesis
build re-provisions it under the new derivation. (This is the entire rotation runbook.)

**Live-verification gate:** CLAIMED until the next operator-merged genesis build's *Upload Blob-Backed
Content* stage uploads with a bearer and returns 200 (the substrate-verify-upload.json artifact shows
`upload.content`/`upload.probe` pass). If `doorway-admin-bootstrap-key` does not match the alpha
doorway's `API_KEY_ADMIN`, expect a register `ADMIN_KEY_REJECTED`/403 with the actionable
`doorway-seed-ensure` diagnostic.

## Task 4: Close the loop — concern status + netpol-revert note

**Files:** the concern backlog entry, a note on `network-policies.yaml` posture.

- [x] Update `backlog/security-ci-substrate-authorization-grant-coherence.md`: hole #2 → Stage A
      LANDED (identity + audit); hole #1 + bounded standing remain. Cross-link this plan.
- [x] Note (do NOT apply — operator/cluster-owned): hole #2's bearer-gate does **not** revert the
      8444/8445 netpol (that's hole #1, conductor-WS, the brokered-surface task). Record that the
      netpol revert is gated on the brokered-conductor-surface work, so no one mistakes this slice
      for licensing a netpol rollback.
- [x] Commit.

## Out of scope

Brokered conductor-WS seeding surface (hole #1 — closes the netpol + the ipBlock VXLAN drift);
Stage C delegates-compute bounded grant (rails-gated); the seeder's full SDK migration if Task 2
took the minimal-login path; any cluster/kubectl action.

## Self-Review

Composes onto the canonical concern (Stage A of its named ladder, not a parallel fix); reuses the
existing `extract_http_permission` ladder (no duplicate auth code); dev-mode safety proven first
(local seeding unbroken); JWT-not-X-API-Key is the deliberate identity-bearing choice the ladder
requires; the gate is named for its Stage-C capability so graduation is a one-line swap; no new DHT
entity (p2p-gate clean); live-verification honestly deferred to the next genesis build (CLAIMED,
not asserted-done).
