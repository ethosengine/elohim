---
name: project_workspace_to_fleet_first_crossing_2026_09_06
title: Workspace→fleet first crossing 2026-09-06
description: "Rung 5 left the house 2026-09-06: 4 coordinator releases 5/5 on alpha's long-lived channel; T3 peer bring-up recipe + traps — read before any fleet rung-5 measure"
metadata:
  type: project
---

**What happened (2026-09-06 04:39–07:1xZ):** `workspace-to-fleet-release.feature` Stations 1–5 passed 5/5 four times on
live alpha (receipts `genesis/a2o/reports/workspace-release/2026-09-06/shift-it{2,6,7}.json`; it5 was the
`lineage_parent_mismatch` refusal that produced the driver cure 9f2856b48). ~60–80 s per crossing, no pipeline in the
path, nobody applied (all alpha humans `=observe`). Habit `runtime-upgrade-propagation` carries the DELTA + STABILITY.

**How the T3 peer was brought up (reuse this, it is not in a script yet — backlog
`runtime-workspace-stack-idempotent-live-conductor.md`):** `just mesh stop` (8090/8888 contested); fleet-parity
conductor = `/projects/.claude-config/tools/hc-fork-25dd2d0be144/bin` (see [[project_sovereign_peer_t3_rung_traps]]);
`NETWORK_PROFILE=join-alpha CONDUCTOR_RELEASE_CHANNELS=runtime:coordinators:elohim:workspace=observe
MESH_FORK_BIN_DIRS=<that dir> bash app/elohim-app/scripts/hc-start.sh` brings the conductor up (admin random, app
4485) but then cold-builds a RELEASE storage binary — kill it, symlink `$slot/release/elohim-storage` → the pool's
`dev/debug/elohim-storage`, and start storage by hand:
`HOLOCHAIN_ADMIN_URL=ws://localhost:<admin> ENABLE_IMPORT_API=true ENABLE_CONTENT_DB=true STORAGE_DIR=<fresh>
ELOHIM_RELEASE_CHANNELS=<channel>=observe <debug elohim-storage> --http-port 8090`. A second `hc-start.sh` run against
the live conductor FAILS and deletes `.hc_ports` — recreate it (`admin_port=…\napp_port=4485`) or the story's
Background goes PENDING. Scoped run: `--config reports/cucumber-delivery-scoped.mjs -p delivery <feature>` (profile
paths merge with positionals — [[feedback_cucumber_profile_paths_merge_trap]]).

**Why:** this is the receipt the operator's north star ([[feedback_upgrade_propagation_north_star_wall_clock]]) was
waiting on, and the apparatus traps cost ~40 min of a 3-hour shift.
**How to apply:** for any fleet-facing rung-5 measure, bring the peer up as above, check DHT parity via
`/db/p2p/conductor-diagnostics` spaces (must equal the manifest's appliesTo dnaHashes), never run the fleet WRITE while
an edge deploy is rolling, and remember the fleet controllers' own adoption rows are unobservable from outside
(backlog `runtime-fleet-adoption-observability-doorway-projection.md`). Next fleet move is the operator's
`=observe→canary` flip (james first).
