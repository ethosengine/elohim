---
name: project_local_mesh_binary_slot_and_restart
title: Local mesh binary slot, restart & prologue traps
description: Local mesh runs the doorway-family DEBUG elohim-storage; rebuild into that slot + `hc-mesh.sh storage-restart` (exe record, loud failure, profile overlay); procfs copyFile is 0 bytes
metadata: 
  node_type: memory
  type: project
  originSessionId: 73728c2e-2397-427f-b5b0-9718f01568dd
  modified: 2026-08-22T20:19:51.232Z
---

The household mesh (`just mesh start`) runs elohim-storage from
`/projects/.cargo-target-pool/family/doorway/elohim__elohim-storage/dev/debug/elohim-storage`
(verified 2026-08-22 via /proc/<pid>/exe) — NOT the `family/dev/.../release` path hc-mesh.sh
defaults to (that file does not exist). Since 2026-08-22 `storage-restart` records each live
peer's exe beside its environ (`$MESH_DIR/storage-restart/<name>.exe`) and restores a DEAD peer
from that record → a running sibling's exe → `STORAGE_BIN`, so the default no longer has to exist;
it returns 1 (`storage-restart FAILED for: …`) when any requested peer has no usable capture or
doesn't answer `/health` by port afterwards. A running peer whose exe shows `(deleted)` is simply
older than the last build.

**Why:** a Rust cure is not on the mesh until the binary is rebuilt INTO that slot and the
storage processes are re-exec'd; `just mesh start/stop` restarts conductors too (DHT churn,
minutes). The restart re-execs from the CAPTURED environ (keeps a chaos-re-keyed AGENT_PUBKEY),
so a profile knob added after boot does not reach a running mesh unless you overlay it:
`MESH_RESTART_APPLY_PROFILE=1` re-applies the dev-tier pacing knobs (never AGENT_PUBKEY);
`MESH_RESTART_ENV_OVERLAY="K=V K=V"` for ad-hoc keys. The 2026-08-22 cascade (~21 ECONNREFUSED
reds from one dead jessica): the a2o drill captured environ with `fs.copyFile` — procfs reports
st_size 0, so the capture was EMPTY — and the script exited 0 on "no captured environment".
Read procfs to EOF (`process-control.ts writeRestartCapture`); never copyFile it.

**How to apply:** `CARGO_TARGET_DIR=<slot> cargo build --bin elohim-storage` →
`MESH_RESTART_APPLY_PROFILE=1 STORAGE_BIN=<slot>/debug/elohim-storage hc-mesh.sh storage-restart`
→ re-run the scoped feature with `CUCUMBER_JSON_REPORT=<own path> just test mesh <feature>` (a
scoped run otherwise overwrites the full-lane cucumber JSON; each run still mints its own
run-identified sprint report; one feature path per scope argument — two paths in one string
resolve to 0 scenarios). `hc-mesh.sh fixture-refresh` re-stamps the household fixture's pids AND
`agentPubKey` (custody commitments name providers by agent key; drills match peerId OR agent key).
Destructive steps (kill/restart/pin/delete) are ONE gate — `substrate-scope.ts
destructiveAllowed()` — the lane's declared `owned-substrate` cap (act1-household contract) with
`A2O_ALLOW_DESTRUCTIVE=1|0` as operator override, never fail-open. `pkill -f <pattern>` matches
the shell that issued it — bracket a char (`cucumber-j[s]`). Mesh pacing: `ACQUISITION_RECONCILE_SECS`
(profile 10, prod 60) bounds saga ch11's exhaustion wait (290 s → 50 s). See
[[feedback_local_mesh_first_cadence]].

**2026-08-23 additions (verified live):**
- `storage-restart` PRESERVES the live exe by design (`resolve_exe` prefers `/proc/<pid>/exe`),
  so rebuilding the pool slot does NOT reach a mesh whose peers were started from a different
  binary (found one running an ad-hoc `/tmp/elohim-storage-release-target/release` build — /tmp
  dies with the container). Cure: full `stop` + `start` with `STORAGE_BIN=`/`DOORWAY_BIN=`
  pinned to the family/doorway slots.
- `hc-mesh.sh:144` hardcodes `POOL=/projects/.cargo-target-pool/family/dev` — doorway/storage
  DEFAULTS point at the dev family regardless of worktree. Pin both env vars when the arc's
  binaries live in another family.
- **Agent-DoD trap:** a DoD whose LAST build is `cargo build --features "p2p"` overwrites the
  slot with a p2p-only binary (dual/iroh restart then refuses or misbehaves). Feature-full
  build (`p2p p2p-iroh`) must be the FINAL build in any storage DoD.
- `hc-mesh.sh stop` kills its own process group — a backgrounded compound command containing
  `stop` dies at exit 144 with empty output. Run stop foreground, tolerate 144, verify with ps.

**2026-08-25 additions (verified live):**
- **Prologue `stage-landing-server-A` OOM trap:** `scripts/ci/stage-spa-blob.sh` zips `dist/elohim-app/server`
  VERBATIM (no build, no `*.map` exclusion). A `development`-config `ng build` (sourceMap: true) that
  accumulates across days left 13,376 `.mjs` + 9,728 `.map` (4.5 GB) → a 1.1 GB zip → `curl: option
  --data-binary: out of memory` ×3 → leg red + `stamp-server-projection-peers` red (no CONTENT_BLOB_HASH).
  Cure: a production `pnpm exec ng build` in app/elohim-app (cleans the output path; 15 MB server / 22 MB
  browser, 0 maps), then re-run the prologue. Check `du -sh dist/elohim-app/server` BEFORE a prologue.
- **`seed-stewardship-fixtures-via-A` post-flight red is the seed.ts false-red class**, not reach: rows
  insert (3/3) but doorway A's read-back misses them while its conductor workers flap (hundreds of
  `Reconnecting to conductor` lines); minutes later all three read 200 anon on A, B and direct storage.
- **Recovery evidence is NOT durable:** `$MESH_DIR/recovery-timeline*.jsonl` lives in /tmp and died with
  the 2026-08-25 container restart (the 13-row live series is gone; only the 8-row a2o fixture survives
  in-repo). Same defect the a2o reports were moved under `genesis/a2o/reports/` for — move the JSONL too.
- **2026-08-29 traps:** `just mesh recovery warm <peer>` WIPES the peer's content db + DocStore + blobs (only
  identity survives) — "warm" is a content-cold restart, so run it only against CONVERGED survivors or the
  recovering peer re-acquires half records. `just mesh storage-restart a b` from a cwd inside
  `elohim/elohim-storage` fails with "justfile does not contain recipe `mesh`" and a multi-peer form printed
  nothing once — restart peers one at a time from `/projects/elohim` and confirm via `P2P node started` stamps.
  Background tool tasks running cargo were killed twice mid-chain today; build in the foreground (≤10 min) when
  a chain keeps dying. The recovery script overruns the 10-min tool ceiling — launch it `setsid nohup … &` from a
  foreground call and wait on its log with a background `until grep` loop.
- **Running the mesh from a git WORKTREE (2026-09-03, 0.7 cutover):** hc-mesh.sh + the a2o lane read
  `$REPO_ROOT` = the worktree, so stage what the tree does not carry: `elohim/holochain/local-dev` (symlink
  to the main tree or a fresh dir), `app/elohim-app/dist` + `app/lamad/dist` (SSR bundles), the packed
  `elohim/holochain/dna/elohim/workdir/{elohim.happ,*.dna}` AND `dna/node-registry/node-registry.dna`
  (the rung-5 candidate minter copies them — `MESH_HAPP_PATH` alone is not enough), `pnpm install`, and
  submodules the gates read (`sophia` missing ⇒ orchestrator + pipeline-list-fresh gates false-red). Scope a
  cucumber run with an a2o-RELATIVE path (`features/...`), not repo-relative (0 scenarios, exit 0). Put the
  matching `hc` FIRST on PATH for the run (`/opt/holochain/bin/hc` is 0.6.0). The 0.7 line adds a required
  local iroh-relay (`MESH_RELAY_BIN`, :3340) and every storage peer now gets
  `ELOHIM_RUNTIME_CONFIG_PATH=<mesh>/<peer>/runtime-config.toml` from hc-mesh.sh (rung-4 watcher; without
  it `/admin/adoption` reads `sweeps:0` forever and station 1 times out). See [[project_devspace_recovery]]
  for the disk-write budget (sample cgroup io.stat; never seed beside a cargo build).
**2026-09-05 — coordswap bundle discipline (verified live):** `hc-mesh.sh coordswap` (POST `/admin/coordinators/sync`) applies EVERY drifted role and has no role filter, and a long-lived mesh's installed coordinators drift from the tree in both directions (a peer's adopted candidate zome, e.g. lamad on the c3 candidate; integrity hashes on dev moving past the installed bundle). A hApp packed from the tree would have rolled lamad BACK. Cure: build the bundle from the mesh's own DNAs (the installed bundle's `.dna` files, or `hc app unpack` of the latest adopted candidate under `/tmp/elohim-local-mesh/<peer>/release-adoption/<cid>/`) plus ONLY the coordinators you mean to move; dry-run (`apply=false`) per role and apply only when the drifted set equals the intended set with no `error`. Installed DNA hashes (`/version` passport) fold the happ role modifiers and never equal `hc dna hash <packed>`.
**2026-09-04 conductors-restart no-op:** after an ark respawn the mesh pid file goes stale; `conductors-restart` kills the stale pids (nothing) and refuses to launch beside the arks, printing "conductors up" for the OLD processes. Verify with `ps -eo pid,etimes,rss,comm | grep holochain` (etimes must reset); cure = `kill -TERM` the real conductor pids and let the arks respawn them, then `fixture-refresh` + `storage-restart`. Also: `storage-restart` refuses a slot binary lacking the p2p-iroh marker and leaves the live peers running — rebuild feature-full first.

**2026-09-04:** `storage-restart` also goes stale on its pid file — it spawned peers that could not bind while the old ones (unknown to it) held the ports; kill the real storage pids by NUMBER first (a `pkill -f "elohim-storage --http-port"` killed my OWN shell — argv self-match; bracket a char: `elohim-storag[e]`). The new binary boots slower than the restart waits: peers report DOWN then come up ~1 min later; re-probe before declaring failure.

**2026-09-05 a2o trap:** a scoped `just test mesh`/cucumber run of a `@requires:household-nodes` feature needs `ELOHIM_CLUSTER_STATE_PATH_OVERRIDE=<…>/act1-household.yaml` AND `A2O_RUN_WIP=1` (for @wip); without them the whole feature is HELD and the run exits 0 having measured nothing — check the summary counts, never the exit code.

**Scoped a2o runs need the recipe env (2026-09-05):** a bare `cucumber-js --config <scoped.mjs> --name …` dies in the Before hook with `E2E_STORAGE_<PEER> is not set`; the peer URLs, doorway URLs, fixture path and cluster-state override are exported by the `just test mesh` recipe (`source hc-mesh.sh; mesh_seed_env; export E2E_…`). Run scoped stations through `just test mesh <feature>` (whole feature) or reproduce that env block first; a relative `ELOHIM_CLUSTER_STATE_PATH_OVERRIDE` resolves against genesis/a2o, not the repo root.
