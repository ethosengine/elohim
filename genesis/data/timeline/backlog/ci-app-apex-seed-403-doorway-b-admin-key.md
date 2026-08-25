---
id: "backlog-ci-app-apex-seed-403-doorway-b-admin-key"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "App pipeline cannot land bundles on elohim.host since the doorway loopback seed gate (edge #1380): PUT /admin/seed/blob on doorway-B returns 403 — doorway-B's API_KEY_ADMIN is a separate secret the Jenkins admin credential does not match"
slug: "ci-app-apex-seed-403-doorway-b-admin-key"
written: "2026-08-25"
author: "epr-card-nav shift (integrator)"
status: "backlog"
priority: "high"
ci_status: blocked
fingerprints: []
jobs: [elohim]
relatedNodeIds: []
tags: [ci, elohim-app, deploy, apex, doorway-b, seed-gate, api-key-admin, credentials, two-premises, operator-ceiling, stale-but-200]
cites:
  - Jenkinsfile
  - scripts/ci/stage-spa-blob.sh
  - doorway/doorway-service/src/routes/seed.rs
  - genesis/orchestrator/manifests/doorway/alpha-b.yaml
  - genesis/orchestrator/manifests/doorway/alpha.yaml
  - genesis/Jenkinsfile
  - genesis/scripts/ci/doorway-seed-ensure.sh
  - https://jenkins.ethosengine.com/job/elohim/job/dev/1672/
  - https://jenkins.ethosengine.com/job/elohim/job/dev/1673/
---

## Symptom

`elohim/dev` #1672 and #1673 (2026-08-25): every `seed elohim.host … (browser|server)` leg fails with
`curl: (22) The requested URL returned error: 403` ×3 → "host left STALE"; the `author … (browser)`
and `projected-head elohim.host …` legs fail downstream of it. The alpha legs (alpha.elohim.host,
doorway-A) pass. Net effect: the app bundle lands on alpha.elohim.host only; elohim.host keeps
serving its last-reconciled bundle (`x-elohim-freshness: amber`), i.e. the EPR-card navigation fix
(54dedb119) is live on alpha and NOT on the apex.

## Cause

`routes/seed.rs::require_seed_authority`: `dev_mode && peer_is_loopback` OR resolved level ≥ Admin.
Edge #1380 (2026-08-25) added the loopback conjunct, closing the hole through which every remote
caller had seeded doorway-B unauthenticated. `stage-spa-blob.sh` sends `X-API-Key:
$STORAGE_API_KEY_ADMIN` (Jenkins `storage-api-key-admin` / `doorway-admin-bootstrap-key`). Doorway-A
binds `API_KEY_ADMIN` ← Secret `elohim-doorway-alpha-secrets/api-key-admin` (matches → Admin →
200); doorway-B binds `elohim-doorway-alpha-b-secrets/api-key-admin` — a different value — so the
key resolves to Authenticated-not-Admin → **403** (not 401: dev_mode still grants Authenticated).
Genesis' bearer-minting fix (47fb60f58, `doorway-seed-ensure.sh`) does not help here: register-
promotion on B also needs B's own admin key.

## Fix shape (operator decision — credentials on the second premises)

Either (a) set doorway-B's `api-key-admin` secret to the same value as the Jenkins credential, or
(b) provision a second Jenkins credential holding B's key and thread a per-host `adminKey` through
`stageSpaBlobs(doorwayEprUrl, bundles, adminKey, outcomes)` (the call site already iterates hosts;
the change is a `[host → credentialId]` map plus one `withCredentials` per host). Then re-run the App
pipeline; expected: the four `seed elohim.host` legs + `projected-head elohim.host` go green and the
apex converges on the current head. Until then, every App build is UNSTABLE on these legs and the
apex is a stale-but-200 host by construction.
