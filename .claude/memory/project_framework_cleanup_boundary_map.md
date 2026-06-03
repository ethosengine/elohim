---
id: project-framework-cleanup-boundary-map
name: project_framework_cleanup_boundary_map
description: "Sprint 1 Classify&Map delivered the canonical framework boundary map + core↔reference split decision + reliability backlog (genesis/docs/architecture/framework-cleanup/, committed aeec00029). FOUR-bucket taxonomy: SDK / elohim-core / app-private / dev-tooling-&-delivery-harness — CI+seeder are the harness, NOT app-layers; their hand-coded delivery knowledge IS the symptom. Manifest has no delivery block = cascade root; doorway warm cache bypassed on / and /lamad routes."
metadata: 
  node_type: memory
  type: project
  originSessionId: e045cbeb-1783-45fc-974b-1a8fc8ae5de3
cites:
  - elohim-sdk-epr-app-boundaries-sprint-kickoff | the kickoff brief whose three-bucket framing this Sprint-1 map corrected to four and grounded in evidence | sha256:9776d193efcabc84
---

Sprint 1 ("Classify & Map", 2026-05-30) of the Elohim framework-cleanup sequence produced three
committed docs (read-only analysis, no code) at `genesis/docs/architecture/framework-cleanup/`:
- `2026-05-30-boundary-map.md` — every component across the 8-tier inventory classified, an 11-row
  leak/duplication register, coverage table, key findings.
- `2026-05-30-core-vs-reference-split-decision.md` — split YES but execute LAST (Sprint 5); 11 ranked
  blockers; independent-SemVer-per-repo versioning.
- `2026-05-30-reliability-backlog.md` — 22 prioritized defects (P0–P2) scoping Sprint 2 + the
  render-verification SLO harness + operator dependencies.
Committed `aeec00029` on `sprint/cross-pillar-cleanup`. Produced by an 8-agent read-only fan-out;
load-bearing claims spot-verified against source.

**FOUR-bucket taxonomy (operator correction to the kickoff's three).** SDK (third-party contract) /
elohim-core (runtime substrate, reached via SDK/HTTP) / app-private (one reference app) / **dev-tooling
& delivery harness**. The 4th bucket fixes a category error: **CI/CD + the seeder are NOT app-layers —
they are the packaging/delivery harness, and the thickness of the hand-coded delivery knowledge inside
them IS the symptom this cleanup addresses.** The cure isn't "tidy the harness"; it's "mature the
app-manifest so the harness becomes a thin manifest-driven consumer." Goal: harness gets *thinner* as
Sprints 3–4 land. See [[project_doorway_manifest_driven_routes]], [[project_elohim_dna_as_sdk_boundary]].

**Top structural findings (scope Sprints 2–5):**
- **app-manifest.schema.json has NO delivery block** (keys: id/name/version/description/vocabulary/
  rendering/projections/writeThrough/signalKinds/graduation/constitutionalRatios/graph). epr_id/slug/
  url_path/entry_file/base_href/cache+SW policy live hand-coded in `seed-projections.ts`. This is the
  cascade ROOT and why the harness is thick → Sprint 4.
- **Doorway warm cache is bypassed on the primary routes.** `dispatch_to_projected_epr`
  (doorway server/http.rs:1144) serves `/` and `/lamad` via a throwaway reqwest straight to storage
  ("doorway proxies, not owns"); the doorway's own MongoDB AppFileCacheService only serves direct
  `/apps/` fetches. The warm fast path the reliability story is about isn't in the path that matters.
  Reliability backlog R1 (P0).
- **Duplicate ELOHIM_CLIENT token** (SDK angular-provider.ts:16 vs elohim-app-local
  elohim-client.provider.ts:19) — Sprint 3 must collapse at source; the lamad alias is only a patch.
- **Projection subscribers bound to registry.infrastructure** (elohim-storage main.rs:597) but
  project-epr signals fire from the lamad cell → SSE repopulation dead; router heals only via 30s poll.
- **elohim DNA is physically the `lamad` role** (dna.yaml name: lamad); no role named `elohim`.
- **app-manifest delivery / apps-sw / DI bootstrap** are the white-page class → Sprint 3 SDK-owned
  `provideElohimApp(manifest)`.

Sequence: Map(1)→Reliability(2)→SDK-bootstrap(3)→manifest-conformance(4)→split(5). Split is blocked by
app↔app coupling (duplicate token, lamad importing 16 @app/elohim classes, 174 pillar violations) +
harness coupling — all cleared by Sprints 3–4 before the Sprint 5 execution.
See [[project_epr_projection_serving_chain]], [[project_alpha_edge_deploy_debugging_landmarks]],
[[project_pillar_boundary_violations_backlog]].
