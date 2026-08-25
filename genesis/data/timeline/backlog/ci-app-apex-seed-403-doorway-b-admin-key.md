---
id: "backlog-ci-app-apex-seed-403-doorway-b-admin-key"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The apex could not be deployed to: PUT /admin/seed/blob on doorway-B 403'd because one deploy pipeline held one key and each doorway checked its own admin identity — resolved by deriving seed authority from the DECLARED network stage plus a fleet seed authority (API_KEY_SEED)"
slug: "ci-app-apex-seed-403-doorway-b-admin-key"
written: "2026-08-25"
author: "epr-card-nav shift (integrator)"
status: "resolved-pending-deploy"
priority: "high"
ci_status: fixed-locally
fingerprints: ["58a6beed3932", "7607fa701e4d", "c6aa7163ed2b", "e3f618a889d1"]
jobs: [elohim]
relatedNodeIds: []
tags: [ci, elohim-app, deploy, apex, doorway-b, seed-gate, network-stakes, simulacra, api-key-seed, two-premises, least-privilege, stale-but-200]
cites:
  - Jenkinsfile
  - scripts/ci/stage-spa-blob.sh
  - doorway/doorway-service/src/routes/seed.rs
  - doorway/doorway-service/src/config.rs
  - doorway/doorway-service/src/routes/freshness.rs
  - elohim/elohim-storage/src/trust/stage.rs
  - genesis/orchestrator/manifests/doorway/alpha.yaml
  - genesis/orchestrator/manifests/doorway/alpha-b.yaml
  - genesis/a2o/LAYERS.md
  - https://jenkins.ethosengine.com/job/elohim/job/dev/1672/
  - https://jenkins.ethosengine.com/job/elohim/job/dev/1673/
---

## Symptom

`elohim/dev` #1672 and #1673: every `seed elohim.host … (browser|server)` leg failed with
`403` ×3 → "host left STALE". The alpha legs passed. The app bundle landed on
alpha.elohim.host only; elohim.host kept serving its last-reconciled bundle
(`x-elohim-freshness: amber`) — so the EPR-card navigation fix (54dedb119) went live on alpha
and NOT on the apex.

## Cause — and the correction to the first diagnosis

The first read of this ("a credential decision for the second premises") was **wrong**, and the
wrong part mattered: it would have punted an in-tree fix to an operator. There is no hidden
second-premises secret. Every doorway "admin key" in this repo is a **plaintext dev fixture in
the manifest's own `Secret.stringData`**, applied verbatim by `kubectl apply` — there is no
sealed-secret controller, no `kubectl create secret`, and no `withCredentials` on any
provisioning path in the repo. `alpha`/`staging` carry `dev-elohim-admin-2024`; `alpha-b`
carries `dev-elohim-admin-b-2024`.

The real defect is a **conflation**: `require_seed_authority` asked "is this caller MY admin?"
when the question the deploy path actually poses is "may this caller seed?". One App pipeline
drives several doorways whose own operator identities are *deliberately* distinct
(`alpha-b.yaml`: "distinct keys so cross-doorway auth exercises real JWT-validation"). With
those two questions collapsed into one key, the fleet was undeployable the moment the gate
genuinely authenticated.

It did not fail earlier because `alpha-b.yaml` set `DEV_MODE: "true"` **specifically to bypass
this gate** — the manifest said so, and carried its own TODO:

> `Opens the seed/admin gate like doorway-A so the CI admin key lands the SPA blob without a
> per-host credential. Remove when doorway-B federation auth hardens.`

`62b658784` (edge #1380) correctly removed that bypass by adding the loopback conjunct, but
nothing replaced the per-host credential it was standing in for. **This is that unfinished
business**, and it is a recurrence: `stage-spa-blob.sh:266-276` records the same class in
2026-06-27, when doorway-B stranded the apex for ~2 weeks on a 401.

## Resolution (landed in-tree, locally verified)

1. **Seed authority derives from the DECLARED network stage, never from `DEV_MODE`.**
   The doorway already resolves `ELOHIM_NETWORK_STAKES` once at boot into
   `AppState::network_stage` (`routes/freshness.rs`), sharing
   `seam_contracts::freshness::NetworkStage` with elohim-storage's `trust/stage.rs` — which
   names this very file's neighbour: *"NetworkStage must never derive from any DEV_MODE flag —
   neither elohim-storage's inert one nor doorway's live, auth-permissive one (config.rs)."*
   That vocabulary was wired for freshness pricing and had **zero auth consumers**. It has one
   now. No new word, no new enum, no new env var was invented for the posture.

   Not a widening: every deployed manifest sets `DEV_MODE: "true"` and none declares a stage,
   so all resolve to `Bootstrap`; `Bootstrap < Coordinated` holds exactly where `dev_mode` held.

2. **A fleet seed authority, `API_KEY_SEED`** — the deploy pipeline's own credential, separate
   from any doorway's admin identity, wired on `alpha` and `alpha-b`. Scoped to the seed +
   admin-cache routes only; it never enters `extract_http_permission`'s ladder, so it is
   strictly *narrower* than the admin key it replaces on that path (it cannot read a user, mint
   a token, promote an account, or reach the conductor). Presence-keyed, so `prod.yaml` —
   declaring none — has no such authority at all.

3. **A designed expiry.** Both pre-coordination affordances (loopback, fleet key) switch off by
   themselves once a doorway declares `coordinated`, which is where `genesis/a2o/LAYERS.md` puts
   Act II. No flag to remember to unset.

Verified locally: `cargo check --offline --all-targets` clean; `cargo test --lib routes::seed`
20/20 including 7 new gate tests (`fleet_seed_key_admits_the_remote_deploy_caller`,
`fleet_seed_key_never_grants_general_admin`,
`coordinated_stage_retires_both_pre_coordination_affordances`, blank/absent/wrong-key refusals,
`admin_identity_still_seeds_at_coordinated_stage`); `cargo fmt --check` clean.

Needs an edge deploy (doorway image + manifests) before the App pipeline's apex legs go green.

## Two findings this surfaced — filed, not fixed here

**(a) The declare leg is not seed-gated.** `authorHeadOnce`'s `DECLARE_ONLY` fan-out
(`Jenkinsfile:342-357`) POSTs `/db/content/{slug}/canonical-head` to doorway-B, and that route
does NOT call `require_seed_authority`. Through #1672-#1673 doorway-B therefore **accepted a
canonical head for bytes whose PUT it had just refused** — it declared a head it could not
materialize. That is a head-plane/byte-plane split worth its own scenario: a declare should not
outrun the bytes it points at.

**(b) The fleet's credentials are public.** `api-key-admin`, `api-key-authenticated` and
`jwt-secret` for alpha, alpha-b and staging are committed in plaintext and applied verbatim to
internet-facing hosts. `API_KEY_SEED` does not worsen this (it is narrower than the admin key
already published), but the underlying exposure is real and independent of this fix. The
durable move is deploy-time injection from Jenkins credentials — no such machinery exists in
the repo today.

## Trajectory — what replaces `API_KEY_SEED`

A standing key is web2 plumbing for a web2 ceremony (pushing SPA bytes to a DNS-fronted host).
The doorway should otherwise derive authority from the p2p plane; the exception is the
**chaperone pattern for hosted humans**, a transitional flywheel as people graduate from users
back to stewards. A deploy pipeline is not a hosted human, so it is not under that exception.
Two successors, in order of reach:

1. **Chaperoned seed actor.** `genesis/scripts/ci/doorway-seed-ensure.sh` already registers
   `jenkins-ci@…` as a doorway-hosted account and seeds under its **bearer** (identity + audit,
   not a shared secret). `API_KEY_SEED` is exactly the fleet-uniform bootstrap that lets the App
   pipeline do the same on any doorway — after which seeding rides the actor's JWT.
2. **Bounded, revocable authority from the authoritative layer.** The REA compute-commitment
   path (`Mishpat::Commitment` + delegates-compute) already shadow-runs behind
   `DELEGATES_COMPUTE_OP_GATE` on `POST /db/content(/bulk)` and is the canonical displacement
   for X-API-Key grants. Routing the seed gate through the same `authorize-operation` call makes
   the migration a config flip rather than a rewrite — and is what the `Coordinated` expiry in
   (3) above is waiting for.
3. **Self-authorizing bytes.** Strongest form: a PUT whose `X-Blob-Hash` is already referenced
   by a notarized head needs no credential at all — content addressing plus the notarized head
   IS the authority (verify-locally-then-serve). Needs the pipeline to author/declare before it
   seeds; today it seeds first.
