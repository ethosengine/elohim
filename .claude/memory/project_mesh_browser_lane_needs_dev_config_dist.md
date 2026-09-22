---
index: false
name: project-mesh-browser-lane-needs-dev-config-dist
title: "Mesh-browser lanes need a development-configuration dist"
id: project-mesh-browser-lane-needs-dev-config-dist
description: Household mesh-browser lanes need elohim-app built with --configuration development; production/alpha builds bake doorway-alpha and hosted-steward scenarios 401 there
metadata: 
  node_type: memory
  title: Mesh-browser lanes need a development-configuration dist
  type: project
  originSessionId: dcddc033-024b-4dfa-8d13-39fa5f75b9ef
  modified: 2026-09-13T04:01:41.832Z
cites:
  - app/elohim-app/scripts/hc-mesh.sh
  - app/elohim-app/angular.json
---

On the household mesh, `just mesh prologue` stages whatever sits in `app/elohim-app/dist/elohim-app/{browser,server}`. A `pnpm build` (default configuration = production, and the alpha/staging configurations too) bakes `doorway-alpha.elohim.host` into the bundle; only `ng build --configuration development` carries `localhost:8888`. With a production dist staged, OAuth succeeds at the local doorway but the app then dials the fleet doorway with a localhost-minted JWT → `Chaperone failed (401): Invalid or expired token`, `IdentityService` never reaches network mode, and the agency badge can only read "Hosted Visitor" (discovered 2026-09-13 in the pipeline shift; fixes ec96b713d/43f5f49ed made the Chaperone follow the session's issuer, but the staged dist must still be a development build for the mesh).

**Why:** the prologue's own error text says "cd app/elohim-app && pnpm build", which produces the wrong bundle for the household; the lane's failure surfaces two steps later as a profile-bubble timeout with no hint of the address.

**How to apply:** before `just mesh prologue` on the household, build with `pnpm exec ng build --configuration development` (it emits both browser and server dists; the prologue stamps version.json); check `curl localhost:8888/version.json` shows `environment: local` and the served chunks mention `localhost:8888`. On the mesh the app and the doorway share an origin, so any Playwright `page.route('**/auth/account*')` hold registered before login also strangles the portal's own refresh. Related: [[feedback_frontend_review_eyes_first]], [[project_local_mesh_binary_slot_and_restart]].

**Build CLEAN (2026-09-18).** A non-clean `ng build` left two build generations in `dist/elohim-app/server` (600 files); `main.server.mjs` still imported the OLD home chunk, so the doorway's SSR HTML carried raw YouTube iframes while the browser bundle and `version.json` showed the new click-to-load facade — a lane failed on a fix that had "landed". `rm -rf dist/elohim-app` before building. Probe the SSR body (`curl localhost:8889/ | grep '<iframe'`) after staging has settled — a CSR-shell grep taken right after the prologue passes falsely.

**Seed by id, never the whole corpus, on the household (2026-09-18).** `just seed apply mesh content` (~4,000 nodes) grows the lamad DHT to ~115k ops; after a restart full-arc gossip stops completing rounds (0–2 in 2.5 h, `NoPeersForLocation` on every get) and lanes refuse at readiness. The landing probes ten slugs and the Lamad home four more — seed exactly those: `cd genesis/seeder && pnpm exec tsx src/seed.ts --ids=a,b,c` with the mesh env (`source app/elohim-app/scripts/hc-mesh.sh; mesh_seed_env`). Backlog: household-lamad-gossip-wedge-large-dht, household-mesh-harness-honest-readiness. Per-DNA gossip health: `hc client call --port <admin> dump-network-metrics` → `completed_rounds` vs `peer_timeouts`.
