---
id: "backlog-elohim-host-deploy-401-operator-secret-alignment"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "elohim.host deploy BLOCKED — doorway-B 401s the SPA-blob PUT: Jenkins storage-api-key-admin value != doorway-B API_KEY_ADMIN (OPERATOR secret-alignment)"
slug: "elohim-host-deploy-401-operator-secret-alignment"
written: "2026-06-27"
author: "overnight doorway-deploy + genesis fan-out shift (2026-06-27T03)"
status: "backlog"
priority: "high"
jobs: [elohim]
---

## The block (OPERATOR action required — repo side is DONE)

`elohim.host` has been serving a stale bundle (blobHash `sha256-1a7617c8…`, updatedAt
2026-06-18) for ~2 weeks. Root-caused + driven this shift to a single remaining operator
action.

App build `elohim/dev #1566` (built from dev HEAD, which carries the auth fix `6dd2ea5dd`)
ran the Upload SPA Blob stage. The **elohim.host** PUT `/admin/seed/blob` returned **401 on
all 3 retries** — and this time the `X-API-Key` header WAS sent (confirmed: stage-spa-blob.sh
line 87 sends `${STORAGE_API_KEY_ADMIN}`, Jenkins log "stageSpaBlobs auth: using credential
'storage-api-key-admin'"). So:

- It is NOT a missing-header 401 (that was #1563, fixed by `6dd2ea5dd`).
- It is a **rejected-VALUE 401**: doorway-B's `require_seed_authority` gate rejects the
  credential value CI sends. **The Jenkins `storage-api-key-admin` credential value does not
  match doorway-B's `API_KEY_ADMIN` secret** (`elohim-doorway-alpha-b-secrets`, set in
  `genesis/orchestrator/manifests/doorway/alpha-b.yaml`).

Why it only bites elohim.host: `alpha.elohim.host` (doorway-A) runs `DEV_MODE=true` so its
gate is bypassed (auth ignored); `elohim.host` (doorway-B / alpha-b) does NOT set DEV_MODE,
so it enforces the gate. A single CI admin key works only if it equals doorway-B's secret.

## Operator action (pick one)

1. Set the Jenkins `storage-api-key-admin` credential value to doorway-B's `API_KEY_ADMIN`
   (from `elohim-doorway-alpha-b-secrets`); or
2. Set doorway-B's `API_KEY_ADMIN` (`elohim-doorway-alpha-b-secrets`) to the value CI holds
   in `storage-api-key-admin`; or
3. If the two gated backends must hold DIFFERENT admin keys long-term, provision a per-host
   Jenkins credential (e.g. `storage-api-key-admin-alpha-b`) and map elohim.host → it in
   `Jenkinsfile` `stageSpaBlobs` (repo change — but needs the credential provisioned first).
   (Not needed today: only ONE gated host exists, so a single matching key suffices.)

Then re-run the App pipeline (`git commit --allow-empty -m "ci: redeploy [build:app]"` or any
app-path change). Verify: `elohim.host/db/content/elohim-host-landing` blobHash flips off
`sha256-1a7617c8…`, and the `elohim-app.deploy.dev / elohim.host …` JUnit cases go green.

## Repo side — COMPLETE this shift (nothing more to do here)

- `6dd2ea5dd` — seed-PUT now sends `X-API-Key`.
- `01518fc83` — App manifest watches `scripts/ci/stage-spa-blob.sh` so deploy-script fixes
  trigger the App pipeline (was the reason the auth fix initially didn't build).
- `adcb695d4` — bounded retry + `emitAppDeployJunit` so a stale host is a NAMED
  `spa-blob-stale` JUnit failure, not a buried UNSTABLE. **This is how the 401 is now visible.**

## Sibling

- `alpha-spa-blob-patch-503.md` — alpha.elohim.host's distinct 503 (PATCH/verify transient;
  re-run when the storage backend stabilizes). Different host, different cause.
- Memory: `project_prod_main_lag_vs_alpha_dev` (the per-host deploy-lag class, leg 2 = SPA-blob).

## Evidence / refs

- ci-observer (#1566): elohim.host legs = 401 ×3 (header sent); alpha legs = 503 (upload ok, PATCH fails).
- `Jenkinsfile:296` withEnv STORAGE_API_KEY_ADMIN; `:371` withCredentials storage-api-key-admin.
- `doorway/doorway-service/src/routes/seed.rs` `require_seed_authority`.
- Shift journal: `.claude/shifts/2026-06-27T03-overnight-doorway-deploy-genesis-fanout.journal.md` (iter-6/7).
