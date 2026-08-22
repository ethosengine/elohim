---
index: false
name: project_track4_projected_head_coherence
title: Track-4 projected-head coherence arc
description: "Declared-head rails already existed (Content head + StampMode + authorHeadOnce); real gap was boot-frozen doorway SSR + no served-vs-declared probe; T4-1/T4-2 landed 2026-07-22."
metadata: 
  node_type: memory
  title: Track-4 projected-head coherence arc
  type: project
  originSessionId: fa5ce6f4-de74-4690-b832-4cb9191e4cc8
---

# Track-4 projected-artifact coherence (2026-07-22 arc)

The "CDN coherence gap" complaint (stale SPA per-host = cache-invalidation failure wearing a
deploy costume) was grounded via 4-seam atlas fan-out. **Key discovery: the declared-head
discipline already existed** — do NOT re-design it:
- Declared head = `serverBlobHash`/`blobHash` on the Content row (DHT-witnessed);
  `declare_content_head` zome fn + head-DAG (`supersedes`) + `StampMode {Declare, HealCanonical,
  GapFill}` (`content_diesel.rs:971`) + projection_reconcile content arm + per-deploy
  "canonical head propagated" probe.
- App pipeline already two-phase: `stageSpaBlobs` (bytes to ALL backends) + `authorHeadOnce`
  (head authored ONCE via live conductor, failover; `Jenkinsfile:306`). The 2-week stale-host
  incident root cause (doorway-B seed 401, DEV_MODE auth asymmetry) was already patched.
- Browser bundle is proxied per-request from each doorway's OWN storage (no doorway cache);
  storage serving path has heal-on-read (`heal_on_read_budget_ms`, `http.rs:2849-2975`) — so
  per-host byte-seeding is an optimization, not the coherence mechanism.

**The real gaps + what landed (integ/dev-merge):**
- T4-1 `0c785b5ef` (+review-fix commit): SSR bundle was resolved/materialized ONLY at boot
  (`registry.rs`, restart = only refresh). Now: `servedBundleHeads` attestation on
  `/health/startup` (slug/serverBlobHash/materializedAt/status current|stale|refreshing|failed/
  declaredServerBlobHash — exact camelCase contract consumed by CI probe + a2o), reconcile tick
  (`DOORWAY_BUNDLE_RECONCILE_SECS`, default 300, 0=off), live hot-swap (per-slug V8 isolate swap),
  `POST /admin/ssr-bundle/refresh` (steward-peers/refresh pattern). Safe-degrade: unverifiable
  head → keep serving current. Review found 3 should-fixes (last-Arc drop joining V8 on tokio
  worker → graveyard reaper; attestation TOCTOU double-resolve; tick-vs-refresh race → per-slug
  in-progress guard) — fixed in follow-up commit.
- T4-2 `5fec4f025`+`31128b8c8`: `scripts/ci/verify-projected-head.sh` served-vs-declared per
  host (4th catchError leg, hash-precise JUnit; FIELD-ABSENT = skip until T4-1 deploys); a2o
  `served-projected-head.feature`. Also fixed: `emitAppDeployJunit` fired before verifyEprMounts
  (mount outcomes never reached the report). Jenkinsfile now 1766 lines — watch
  MethodTooLargeException on first post-push build (heredoc-free rule followed; known fix =
  extract to scripts/ci).

**T4-4 deferred (design decision, don't build casually):** reach-governed serving = new
`projects-artifact`-style action on the EXISTING Mishpat Commitment (free-string action + schema +
integrity match arm — moves the DNA hash) via wired `bounds_validator` 7-check; doorway reach code
(`access_control.rs can_serve_at_reach`, `reach_aware_serving.rs`) is ORPHANED (zero consumers);
HTTP reach enforcement is a documented HIGH open gap; reach vocabulary drifted FIVE ways
(schema 8 / epr_kind 8 / resilience 5 / TS-geographic 8 / library 6 — see
`genesis/data/timeline/backlog/reach-vocabulary-frontend-strand.md`). Sequence: vocab
reconciliation FIRST, then the commitment action. Serving accounting rides
[[project-eprfs-witnessed-interaction-primitive]] once past Draft.

Related: [[project_prod_main_lag_vs_alpha_dev]] (the incident), [[project_ssr_first_deploy_seed_then_restart]]
(ordering contract T4-1 dissolves), [[project_resilience_card_data_plumbing]] (same-day identity arc).
