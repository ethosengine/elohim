---
title: jenkins-seed bearer-gate — two residuals for the DEV_MODE=false enforcement cutover
created: 2026-06-11
status: OPEN
domain: D-identity
source: pre-push end-to-end auth review of the jenkins-seed bearer-gate component
severity: medium
blocked-by-env: dev-mode-off
---

**Context.** The jenkins-seed bearer-gate (Stage A CI auth) is LANDED and self-provisioning:
`doorway-seed-ensure.sh` provisions the `jenkins-ci` actor in-pipeline from the existing
`doorway-admin-bootstrap-key` credential, mints a real JWT, and the doorway
`require_seed_authority` gate (`routes/seed.rs`) accepts `PermissionLevel::Admin`.

**It is INERT in every deployed environment today.** `DEV_MODE="true"` in all four doorway
manifests (`alpha/prod/staging/staging-read.yaml`), and the gate short-circuits
`if state.args.dev_mode { return Ok(()) }` (`seed.rs:63`) BEFORE any auth — so seeding works
unauthenticated and nothing enforces. The bearer machinery is correct groundwork that only engages
once `DEV_MODE=false` (itself a larger hardening: needs `jwt_secret` provisioning per
`config.rs:287`, and changes doorway auth behavior beyond seeding).

**Two residuals to close BEFORE / AS PART OF the `DEV_MODE=false` cutover** (both are moot under
dev-mode, neither blocks the dev push that landed this):

1. **Seeder-TS path runs bearerless in the Seed Database stage.** The self-provisioned token
   (`SEED_DOORWAY_TOKEN`) is fed only into `uploadBlobContentStage` (substrate-verify.sh upload).
   The `Seed Database` stage (`genesis/Jenkinsfile` ~1583–1640) binds `ADMIN_KEY` but NOT
   `SEED_DOORWAY_IDENTIFIER/PASSWORD`, so the seeder TS (`seed.ts`, `setBearerToken(null)` at ~1017)
   sends no bearer. If the seeder TS hits a gated route (`pushBlob` → `PUT /admin/seed/blob`,
   `POST /admin/cache/*`) when `dev_mode` is off, it 401s. Fix: feed the same self-provisioned
   bearer (or bind the derived creds) into the Seed Database stage's seeder invocation, OR confirm
   the seeder TS no longer touches gated routes (blob upload routed entirely through the upload
   stage).

2. **`resolveSeedDoorwayToken` soft-fails on a real auth error.** `doorway-seed-ensure.sh`
   correctly `exit 1`s on register/login failure (vs `exit 0` empty-token for creds-absent), but
   the Groovy caller (`genesis/Jenkinsfile:1013`) echoes and continues with an empty token →
   `uploadBlobContentStage`'s `catchError(UNSTABLE)` → the build goes UNSTABLE (yellow), not FAILURE
   (red). Fail-closed still holds (no data written under bad auth — it 401s), so this is an
   alerting gap: a rotated/rejected admin key looks like the normal degraded-substrate yellow. Fix:
   when `status != 0` from `ensure.sh`, `error()` / set `FAILURE` instead of continuing.

**Verification when the cutover happens:** flip `DEV_MODE=false` on alpha (repo manifest), confirm
`jwt_secret` is set, re-run the end-to-end auth review, and assert an authenticated seed run writes
content (no 401) while an unauthenticated/absent-creds run 401s loudly (FAILURE, not UNSTABLE).
