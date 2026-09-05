---
name: project_doorway_ops_incidents
title: Doorway ops incidents (umbrella)
description: "Doorway ops incidents: A/B edges are islands (no coherence); kitsune2 bootstrap gap made conductors DHT islands; blocking getaddrinfo parks tokio workers."
metadata:
  type: project
---

# Doorway ops incidents (umbrella)

Folds the doorway operational-incident cluster. Members:

- [[project_doorway_ab_edge_islanding]] — two independent doorway edges over matthew/adam with no cross-edge coherence or divergence detection; e0352a7/8a2c65e glyphs were buildIds, not content CIDs.
- [[project_doorway_kitsune2_bootstrap_protocol]] — HC 0.6 conductors speak kitsune2 bootstrap (PUT /bootstrap/{space}/{agent}); doorway served only kitsune1 until 2026-06-12, leaving conductors DHT islands.
- [[project_doorway_wedge_unbounded_mongo_await]] — doorway-alpha SIGKILL crashloop root = blocking getaddrinfo in holochain_client connect parking tokio workers during DNS flaps; fixed via async DNS + watchdog.
- **Conductor split left the doorway admin URL on the storage Service (2026-08-31 → 09-03).** The split (9c9f9fc65) moved the socat 8444/8445 bridge into `<prefix>-conductor-0` with its own Services (`elohim-<human>-alpha-conductor`); the Jenkinsfiles render `CONDUCTOR_URLS` against them, but every doorway manifest's hand-written `CONDUCTOR_ADMIN_URL` still named `elohim-<human>-alpha:4444` → each doorway start died minting its app auth token (one WARN, probe kill after a 130 s connect timeout; no ERROR). doorway-B had no Ready pod for 3 days; doorway-A lived on one pre-split pod. Fix 2d356dbc2 (all five doorway manifests → `<prefix>-conductor:4444`). Rule: cross-workload Service names in doorway manifests are rendered or linted, never hand-written. Atom: `genesis/data/timeline/backlog/conductor-split-left-doorway-admin-url-on-the-storage-service.md`.
- **Hostnames:** doorway-A = `https://doorway-alpha.elohim.host` (also `doorways.elohim.host`); doorway-B = `https://elohim.host` (ingress `elohim-doorway-alpha-b`; E2E_DOORWAY_B default). `doorway.elohim.host` is a STALE ingress in namespace `elohim-prod` (`elohim-edgenode-prod`, no pods) — permanently nginx 503; probing it as "the apex doorway" cost an hour on 2026-09-03. Retire/re-point via `genesis/orchestrator/manifests/doorway/prod.yaml`.
- **New path-dep → Dockerfile COPY (edge #1421, 2026-09-03):** `elohim-ark-core` as a storage path-dep aborted the image build at dep resolve (`failed to read /ark/core/Cargo.toml`); the local gate cannot see it. Verify Dockerfile edits without Docker by replaying its COPY + `RUN sed` steps into a scratch layout and running `cargo metadata` there (846 packages resolved for 3c29b39d7).
