---
name: project_workspace_to_fleet_first_crossing_2026_09_06
title: Workspace→fleet first crossing 2026-09-06
description: "Rung 5 left the house 2026-09-06: three releases applied by the alpha canary by election; single-role candidates only; T3 peer bring-up recipe + traps — read before any fleet rung-5 measure"
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

**14:29Z — the fleet APPLIED (james, canary) and ATTESTED.** Two traps between "fleet verifies" and "canary applies", both cured at the driver:
- **Wire address form.** `artifact_pull` raced `ShardRequest::Get{hash: <bafkrei… CID>}`; every holder's `ShardService::handle_get` looked the key up literally against `sha256-<hex>` → honest NotFound on all peers (`elohim_iroh_blob_fetches_total{result="not_found"}`), while `/blob/<cid>` served. Local repro: `/shard/<cid>` 404 vs `/shard/sha256-…` 200. Cure: `shard_service::on_disk_key` + `artifact_pull::wire_address`. Reachability was fine (gossip up, board present); don't chase planes when the metric says `not_found`.
- **`appliesTo` = what the release applies ONTO, read from the BUILDER's passport.** For a workspace→fleet cut the builder's coordinators ≠ the target's (mishpat differed; 4 roles matched) → `coordinator_lineage_mismatch`, honest. Cut the release FOR the target: `--applies-to-from-adoption <doorway>/db/p2p/adoption?peer=<name>` reads the peer's `installedReality` (added 2026-09-06); bridge was a literal `--applies-to`.
- Loop timing: publish → verify 30 s → canary apply 59 s → attestation readable via the workspace conductor 5 min. Workspace storage restart recipe: scratchpad `start-workspace-storage.py` replays `/proc/<pid>/environ` (needs `--features "p2p p2p-iroh"` in the `dev` slot).


**19:2xZ — releases 2 and 3 reached the fleet (three by election today); the 5-role candidate was the trap.** Release 1's
`.happ` rebuilt all five DNAs from the workspace tree, and the workspace toolchain moves INTEGRITY bytes too, so james's apply
was `drifted=5, applied=1` (lamad swapped, four roles refused on DNA lineage) → a mixed peer that no 5-role release can verify
(`already_runs_target` is all-or-nothing, lineage refuses the rest). The adopted lamad survived the edge pod roll; the controller's
in-memory `appliedRelease` did not.
- **Cut single-role candidates only**: `pnpm exec tsx steps/delivery/coordinator-candidate.ts --baseline-happ <deployed .happ>
  --report-dir <dir> --role lamad [--rollback-to-baseline | --marker <text>]` with `ELOHIM_HC_BIN=<0.7 fork hc>` and the fork
  bin on PATH (default hc is 0.6). Package with `--applies-to-from-adoption <doorway>/db/p2p/adoption?peer=<name>
  --applies-to-role lamad` — the packager now refuses an unscoped multi-role coordinator appliesTo.
- **To bring a wandered canary back**: a rollback-shaped single-role release (target = baseline bytes) cut FOR that peer; observers
  read it already-current, the canary re-applies. Never a 5-role "revert".
- Timings tonight: publish → canary apply 54 s / 68 s (artifact peer-pull 640 ms), attestation +86 s. Six observers verify but never
  move (`observe`); promotion needs the =observe→apply data flip. gertrude/eve lagged the election read by >15 min (unexplained).
- Stale-snapshot trap: after a canary apply, `/admin/adoption` installedReality showed the PRE-apply reality for 10+ min (idempotence
  exit does zero conductor reads) — fixed in the same batch; until deployed, read the storage log line `coordinator hot-swap applied
  … drifted=N applied=M` as the truth.
