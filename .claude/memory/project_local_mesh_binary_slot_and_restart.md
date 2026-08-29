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
