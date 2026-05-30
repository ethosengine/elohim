---
name: project_alpha_edge_deploy_debugging_landmarks
description: "Landmarks for debugging alpha edge deploys via Jenkins+Loki — log labels, force-edge pattern, cascade-halt, and the pre-existing conductor IPv6-bind crashloop on shem."
metadata: 
  node_type: memory
  type: project
  originSessionId: 21dcbb18-990e-405a-93d6-2beb9577827a
---

Hard-won landmarks from a JSON-logging delivery + crashloop shake-out (2026-05-29/30):

- **Loki labels:** alpha elohim-storage logs are under `container="elohim-node"`
  (the consolidated edgenode container), NOT `container="elohim-storage"` (that
  label doesn't exist on alpha). doorway is `container="doorway"`. Datasource uid
  `P8E80F9AEF21F6940` (Loki). Retention ~7 days.
- **Force a single pipeline standalone:** an EMPTY commit with `[build:edge]`
  (`git commit --allow-empty -m "ci: ... [build:edge]"`) → empty changeset → the
  orchestrator dispatches ONLY the forced pipeline (no app, no cascade). A
  storage-only changeset + [build:edge] also dispatches edge-only (+ genesis as
  edge's downstream dependent). A root `Jenkinsfile` change → graph-walker
  `{"projects":[]}` → does NOT auto-dispatch; needs a force tag to validate.
- **Cascade-halt (FIXED for the app case):** the orchestrator runs `elohim/dev`
  (app) wait-for-result at Level 0; a FAILURE in the post-build "Upload SPA Blob"
  deploy-seed step (transient doorway 503) aborted the WHOLE dependency graph and
  blocked edge. Fix landed: `catchError(buildResult:'UNSTABLE')` around the
  `stageSpaBlobs()` call in root `Jenkinsfile` (orchestrator treats UNSTABLE as
  success). See [[project_pre_dispatch_hard_fail_post_dispatch_unstable]].
- **Storage SQLite crashloop (FIXED):** `SqlitePragmas::on_acquire` set
  `busy_timeout` AFTER `journal_mode=WAL`; under concurrent r2d2 warm-up a
  contended connection hit immediate SQLITE_BUSY ("database is locked") → pool
  init fail → crashloop. Fix: set busy_timeout FIRST.
- **OPEN next-blocker — conductor IPv6 EADDRINUSE on shem (PRE-EXISTING):**
  Holochain conductor binds dual-stack `[::]:4444/4445` by default; on `shem`
  (multi-tenant: 9 edgenode pods pinned via node-type=remote, podManagementPolicy
  Parallel, CPU-contended) restarting pods can't rebind the IPv6 socket → AddrInUse
  → storage exits after 60 conductor-readiness attempts → crashloop. 10/14 alpha
  pods affected; 4 family nodes (ethosengine) Ready. Fix levers: conductor
  `host: 127.0.0.1` (drop IPv6 bind — verify HC config schema + ws-proxy/doorway
  connectivity); podManagementPolicy Parallel→OrderedReady (IMMUTABLE sts field →
  operator recreate); shem CPU capacity (operator). File:
  genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml.
  Needs rust-architect + operator. See [[project_alpha_topology_bootstrap_pair]].
- **Cascade-unmasking confirmed (2026-05-30):** the SQLite fix (bd083b4b6) is
  PROVEN correct — the 4 pods that received image `1.0.0-dev-bd083b4b` (adam,
  matthew, jessica, terrance = family nodes) show **0 restarts**. The same fix
  *unmasked* the conductor-IPv6 crashloop on the 10 shem pods (it was buried under
  the SQLite crash). Classic [[feedback_cascade_halt_masks_failures]]: one fix
  surfaces the next buried failure. "Images Pushed: Skipped" in the edge build
  summary is a dedup label, NOT a missing artifact — the 4 healthy pods are
  running that exact tag, so the image IS in Harbor.
- **WHY landing pages 404 — the EPR-router population chain (durable):** `/` and
  `/lamad` are served by the doorway **EprRouter**, which is populated ONLY two
  ways: (1) boot-fetch B12 = a SINGLE non-retrying `GET /db/rea_commitments?action=
  project-epr&doorwayId=<id>` with a 10s timeout (doorway-service/src/main.rs
  ~562-605); (2) SSE `projection.registered` re-sync (storage_events_subscriber.rs
  ~145-233). BOTH go through storage's diesel pool. When storage's SQLite is jammed
  every `/db/*` hangs → boot-fetch times out → router stays EMPTY → terminal 404
  with the `"Use WebSocket connection to /admin or /app/:port"` hint (the
  not_found fall-through). The landing-page bug is DOWNSTREAM of the storage DB
  hang, not a frontend bug.
- **GOTCHA — storage /health does NOT touch diesel:** elohim-storage `/health`
  reads `blob_store.stats()` + atomics only (http.rs ~1309-1358); doorway `/health`
  reports its conductor WS worker pool, not storage's SQLite pool. So a pod can be
  **Ready / "healthy" while its diesel pool is jammed.** Never trust /health to
  prove DB-serving — probe `/db/rea_commitments?action=project-epr` directly.
- **GOTCHA — DOORWAY_ID silent-empty-router:** boot-fetch uses
  `args.doorway_id.unwrap_or(node_id)`. Seeded projection rows are scoped
  `doorway:elohim-host` / `doorway:alpha-elohim-host`; the query filters
  `in_scope_of LIKE '%doorway:{id}|%'`. If `DOORWAY_ID` env is unset the doorway
  uses its node UUID → finds ZERO rows even on a healthy DB → empty router, no
  error. Verify `DOORWAY_ID` on doorway pods.
- **Theory B — projection subscriber bound to wrong cell (NOT fixed):** in
  elohim-storage/src/main.rs ~597 the projection-signal subscribers bind to
  `registry.infrastructure`, but the projection signals fire from the **lamad**
  cell — so SSE repopulation never fires and the router only ever populates via
  boot-fetch (i.e. a doorway restart). Fix = move
  subscribe_rea_projection_signals + subscribe_elohim_content_signals onto
  `registry.lamad`. Boot-fetch still works without this, so a doorway restart
  after a good seed is the operational workaround.
- **Seed state (genesis #1061, 2026-05-29):** content IS seeded — matthew=3465,
  adam=3465, jessica=3465 content rows verified; 6 project-epr commitments created
  (elohim-host-landing@/ + lamad-spa@/lamad on both elohim-host and
  alpha-elohim-host). The camelCase serde fix (57bf7d672) is in the live seeder
  path (created=6 succeeded). apex elohim.host = adam backend; alpha doorway =
  matthew backend (separate storage, content must project on both).
