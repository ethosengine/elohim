---
name: hc-dev-orchestrator
description: START and manage the Elohim P2P Framework local DEVELOPMENT STACK — runs conductor (identity/provenance), storage (content), doorway (unified API) as a coordinated service trio. Use when "start the local dev stack", "spin up holochain locally", "why isn't the conductor reachable", "is the doorway alive?", or checking service health during development. NOT for desktop Tauri shell knowledge (use tauri-desktop).
metadata:
  runtime: codex
  sourceRuntime: claude
  master: package
  sourcePath: .epr-meta/elohim/packages/skills/hc-dev-orchestrator.json
  packageKind: SkillPackage
governance: "epr:elohim-agent/skills/hc-dev-orchestrator"
---

# Elohim Local Development Orchestrator

The root `justfile` is the public developer interface. It coordinates the
Holochain conductor (identity/provenance), `elohim-storage` (content and blob
projection), and doorway (HTTP/WS projection) without exposing crate-specific
paths, `RUSTFLAGS`, or cargo-target placement.

## Quick start

Run from the repository root:

```bash
just --list
just dev start                 # isolated single-peer stack
just dev start isolated true   # start and seed
just dev status
just dev stop
```

The positional `dev` parameters are `action profile seed build`. Profiles:

- `isolated` — local island DHT; the safe default.
- `alpha` — joins alpha via its bootstrap/signal endpoints and deployed hApp.

Use `just dev start isolated false true` to force component rebuilds. Native
storage and doorway builds go to their explicit cargo-pool release slots;
DNA/WASM builds remain in-tree because `hc dna pack` requires `./target`.

## Multi-peer mesh

The alpha-shaped local topology is two doorways (A `:8888` alpha stand-in,
bootstrap+signal owner; B `:8889` apex/elohim.host stand-in, jessica-primary)
plus N conductor/storage peers (default matthew, jessica, james), all on
loopback — fronted by a loopback `mongod` (`:27017`, dbpath `$MESH_DIR/mongo`)
so both doorways boot **archive-backed** (Mongo-side projection archive:
`app_file_cache` / warm-shell `ShellArchive` / resolver store, one database per
doorway: `doorway-a`, `doorway-b`). Without the binary (`MONGOD_BIN` unset and
no `mongod` on `$PATH`/`~/bin`) the doorways run archive-less with an INERT
warm-shell store — the production shape 18a65fd0d found un-wired — and
`mesh status` says so. Both doorways launch with `--dev-mode
--dev-signal-subscriber` (env twin `DEV_SIGNAL_SUBSCRIBER`): dev mode alone
skips the multi-peer signal subscriber (most dev contexts have no conductor),
but the mesh fronts real conductors, so the opt-in lights the subscriber and
`status.json` `compute.peers[]` populates — the surface the
peer-conductor-resilience a2o reads. The image ships `mongod` (che-devworkspaces udi-plus);
a2o resolves `alpha-A`/`elohim.host` to `E2E_DOORWAY_ALPHA`/`E2E_DOORWAY_B`,
so the failover feature runs against the local pair unchanged:

```bash
just mesh start                # mongod → doorway A → doorway B → conductors → storage peers
just mesh status
just mesh probe
just mesh quiesce
just mesh stop
```

`mesh quiesce` measures an already-running mesh and records its bounded result
(one line per run, including the wall-clock, verdict, knobs and an io_baseline
write-throughput probe) under `${MESH_DIR:-/tmp/elohim-local-mesh}`. It never
starts or stops peers. The underlying maintained interfaces are
`hc-mesh.sh start|stop|status|probe` and `hc-mesh-quiesce.sh`; do not copy
their pacing environment into new npm aliases.

### Act I Prologue cast (`hc-mesh-prologue.sh`)

`hc-mesh.sh` brings the mesh's PROCESSES up; it does not cast the household.
Once `just mesh start` reports both doorways healthy, run the Prologue to
seed the Act I substrate a2o's `@act:i` scenarios need — named conductor
identities, the base corpus rows every later leg patches bytes onto
(`elohim-host-landing`, `lamad-spa`, `evolution-of-trust` — seed a row here or
its stage leg 404s and the scenarios that need it env-red on the
precondition), the landing + lamad-spa bundles (browser AND the landing's SSR
server bundle, whose `serverBlobHash` is then stamped on EVERY peer's row —
the field is a diesel-direct deploy-projection artifact no sync plane
carries, so without the per-peer stamp doorway B's declared read stays NULL
and resiliency-saga ch06's cross-doorway scenario pends forever), the full
CI-order seed chain (identities cast BEFORE `seed-humans` on the mesh — doorway A's hosted pool is matthew's conductor, so hosted registrations must not claim it first), and the household fixture
manifest `genesis/a2o/src/framework/fixtures/household-mesh.ts` resolves
against, and Act I's own cast — the drill fixtures two resilience features
name (`heal-target`, `chaos-ladder`) with their household custody promises,
and the co-steward agreement (`seed-household-costeward.ts`) the saga's last
chapters count. On alpha that agreement is authored at run time by chapter 5
with adam as co-steward; the household mesh has no such author, so the
Prologue casts jessica co-stewarding the landing EPR instead:

```bash
just mesh start                # bring the mesh up first — the Prologue never starts/stops it
./app/elohim-app/scripts/hc-mesh.sh prologue   # (or: bash hc-mesh-prologue.sh directly)
```

`just mesh prologue` is routed by the root `justfile` (`mesh` recipe whitelist); `hc-mesh.sh prologue` is the same entry point.

**The cast fix — named `CONDUCTOR_URLS`.** An unnamed loopback conductor URL
(`ws://localhost:4445,ws://localhost:4455,...`) resolves by first-reachable-
wins in `seed-conductor-identities.ts`, which can cast the wrong human onto
the wrong conductor (observed 2026-08-21: Adam cast onto james's conductor,
zeroing household participants downstream). `hc-mesh.sh`'s `conductor_csv()`
produces the named form instead — `matthew=ws://localhost:4445,jessica=ws://
localhost:4455,james=ws://localhost:4465` — and `mesh_seed_env()` (source
`hc-mesh.sh`, then call it) exports it as `CONDUCTOR_URLS` alongside
`HOLOCHAIN_ADMIN_URL` / `STORAGE_URL` / `DOORWAY_URL` / `PEER_STORAGE_URLS` /
`SEEDER_TARGET_PEERS` / `API_KEY_ADMIN` — one source of truth for both the
Prologue script and an operator's shell. `just mesh status` prints the same
named form on its `probe env:` line.

Backlog record: `genesis/data/timeline/backlog/mesh-prologue-cast-and-env-gaps.md`.

### Re-measuring a2o against the mesh — scoping trap

`cucumber-js -p local <files>` runs the WHOLE suite: a profile's `paths` in
`cucumber.mjs` MERGE with CLI positionals rather than being replaced by them,
so pointing the `local` profile at one feature directory still loads every
path the profile itself declares. To re-measure a narrow set of scenarios
after a fix, either:

```bash
# an EMPTY .mjs config file, path relative to the REPO ROOT, so no profile paths merge in
pnpm exec cucumber-js --config path/to/empty.mjs features/dataplane/one.feature

# or: keep the profile's env/worldParameters, narrow by scenario NAME instead of path
pnpm exec cucumber-js -p local --name '^exact scenario title$'
```

Never assume a directory argument alone narrows the run under a named
profile — it is additive, not a filter.

The mesh runs a declared dev-tier pacing profile (a preproduction-stakes
declaration, never a prod default) exported to the storage peers by
`hc-mesh.sh` — each knob overridable via its `MESH_*` twin:
`PROJECTION_RECONCILE_SECS=30`, `CONTEST_BACKOFF_SECONDS=120`,
`HEAL_MISSING_BACKOFF_SECONDS=60`, `ELOHIM_EVIDENCE_ABSENT_BACKOFF_SECS=600`,
`ELOHIM_HEAD_CORPUS_DIGEST=1`, `ELOHIM_ADOPT_BEFORE_AUTHOR=1` (cross-peer
head divergence has no adopt discharge without it), `ADOPT_CONTEST_FANOUT=1`
(concurrent declares race the conductor chain head; serialized lands
first-try), `ELOHIM_NETWORK_STAKES=simulacra` (the explicit Simulacra
declaration). The conductor side gets kitsune2 `k2Gossip` intervals patched
to 1000ms test cadence post-generate. The profile block in `hc-mesh.sh` is
the authoritative knob list.

### Which conductor the mesh runs (`HOLOCHAIN_BIN`) — and why the `hc` CLI is half of it

Alpha runs the conductor **fork**; a mesh on stock is not a proving ground for it, and
`hc-mesh.sh status` says so on both the `conductor RUNNING:` and `conductor NEXT LAUNCH:` lines
(`[FORK]` / `[STOCK — alpha runs the fork, so this mesh is NOT at parity]`).

The CLI is not a detail: **`hc sandbox` REWRITES `conductor-config.yaml` in its own version's
schema** (that is how `-f` pins admin ports), so a stock `hc` in front of a fork conductor hands the
conductor a file it refuses to parse. Therefore:

- `HOLOCHAIN_BIN` takes a **binary OR a directory** holding `holochain` + `hc`; the matching `hc`
  goes on PATH for **`generate` AND `run`** (the old code did `run` only — that asymmetry is how a
  fork conductor got 0.6.0-schema configs and three `hc sandbox run` panics).
- `start` and `conductors-restart` **refuse a mismatched pair** and print both versions.
  The check is on the FULL version, not the major.minor line — 0.6.0 and 0.6.3 agree on `0.6` and are
  still schema-incompatible. `MESH_ALLOW_TOOLCHAIN_SKEW=1` overrides for a deliberate experiment.
- The **generate transport subcommand differs by line**: stock 0.6.0 `network … webrtc <SIGNAL_URL>`,
  fork 0.6.3 `network … quic <RELAY_URL>` (iroh). `mesh_network_args()` reads the grammar from
  `hc sandbox generate network --help` rather than guessing; `MESH_FORK_RELAY_URL` (default: the
  doorway) fills the relay argument, which must parse as a URL.
- Auto-detect searches `$MESH_DIR/fork-bin`, `$REPO_ROOT/.fork-bin`, `/opt/elohim/fork-bin` — opt-in
  homes only. A local fork build in the cargo-pool slot is passed **explicitly**:
  `HOLOCHAIN_BIN=/projects/.cargo-target-pool/family/dev/crates/dev/release just mesh start`.
- Do **not** migrate an existing stock sandbox to the fork — measured twice, it fails on schema and
  lands `app_ports:[]`. `just mesh stop` then generate fresh with the fork's own `hc`.
  Background: backlog `mesh-prologue-cast-and-env-gaps.md` (parity attempts 1–3).

`conductors-restart` is still **half an operation**: it leaves each storage peer's app-websocket
handles pointing at a conductor that no longer honors them. Follow it with `storage-restart` and
confirm with `zome-probe`. The bridge supervisor re-mints the three supervised roles, but the
PeerStatus heartbeat holds a fourth, unsupervised client — so `/health conductor.zomePath` flaps
`live↔dead` on a ~60 s cycle until storage restarts (backlog
`storage-stale-app-interface-token-after-conductor-restart.md`).

`storage-restart [peer…]` re-execs each storage peer in place from its captured `/proc` environ
(conductors untouched; a chaos-re-keyed `AGENT_PUBKEY` survives). The mesh runs the
**doorway-family debug** binary (`/projects/.cargo-target-pool/family/doorway/elohim__elohim-storage/dev/debug/elohim-storage`),
not the release path the script defaults to — pass `STORAGE_BIN=<that path>` explicitly, and rebuild
into that slot first (`CARGO_TARGET_DIR=<slot> cargo build --bin elohim-storage`) when a Rust cure
must reach the mesh. The restart re-resolves every peer's pid into the household fixture: the a2o
chaos drills kill and verify peers BY THAT PID, so a stale fixture reads as `kill ESRCH`.

`just mesh monitor` (hc-mesh-monitor.py, port 4210 via the `mesh-monitor`
devfile endpoint; honors `MESH_MONITOR_PORT`) serves the one-page live
dashboard: component liveness, per-peer convergence gauges, a gate-legs
panel mirroring `fleet-quiesce-gate.sh`'s exact PASS predicate, log tails,
and a phase/progress status bar.

### Chaos and spin detection (`hc-mesh-spin-detector.sh`, `hc-mesh-chaos-rekey.sh`)

The alpha conductor spin (sys-validation retrying unfetchable dependencies every 10 s,
read-pool saturation logged at ~1000 lines/s — backlog
`alpha-conductor-sys-validation-spin-unfetchable-deps.md`) is measured and staged at the desk:

```bash
bash app/elohim-app/scripts/hc-mesh-spin-detector.sh --window 20 --cycles 9        # verdict SPIN|QUIET + JSON
bash app/elohim-app/scripts/hc-mesh-chaos-rekey.sh --peer james --tag act1 --phase author   # james-originated content, referenced by the others
bash app/elohim-app/scripts/hc-mesh-chaos-rekey.sh --peer james --tag act1 --phase rekey    # destroy james's chain (conductor + key), keep his storage DB
bash app/elohim-app/scripts/hc-mesh-chaos-rekey.sh --peer james --tag act1 --phase measure  # detector for 3+ min on the survivors
```

The detector reads per-conductor CPU from `/proc`, log-rate spectroscopy from `.sandbox_run_log`
(saturation / `No peers to fetch` / `missing dependencies` with the count extracted) and says `SPIN`
when the missing-dependency count is non-decreasing across ≥3 cycles with saturation above threshold.
It is validated against alpha-shaped synthetic logs both ways. The three conductors are children of
ONE `hc sandbox run` parent: a single-peer kill may drag the others down; the script restarts them
from their existing sandboxes (no regenerate, no re-key) and says so loudly, because it changes the
measure. Scenario: `genesis/a2o/features/resilience/conductor-validation-spin.feature`.

**Trap:** `just mesh stop` kills by `pgrep -f` patterns (`elohim-storag[e]`, `doorwa[y]`, `hc sandbox`) —
a shell whose command line contains those strings (an env assignment, a `pgrep -af …` argument) is
SIGTERM'd too (exit 144). Run `just mesh stop` alone in its own command.

## Running the Act I lane

```bash
just test mesh                                   # whole Act I lane (@e2e, not @wip/@browser) under cluster-state.act1-household.yaml
just test mesh features/dataplane/doorway-failover.feature
just test mesh '@act:i and @dataplane'
```

`just test mesh` sources `hc-mesh.sh`'s `mesh_seed_env` and exports the Prologue's a2o env block; a
scope argument makes it write a paths-less config so the run is actually scoped (cucumber merges a
profile's `paths` with positionals otherwise). `@act:<i|ii|iii|host>` resolves to the act's baseline caps;
an undeclared `@requires:` cap warns loudly once per run. Spec: `genesis/a2o/LAYERS.md`.

Destructive steps (kill/restart/pin/delete) ride ONE gate — `substrate-scope.ts destructiveAllowed()`:
the lane's declared `owned-substrate` cap (true only in `cluster-state.act1-household.yaml`), with
`A2O_ALLOW_DESTRUCTIVE=1|0` as the operator override, never fail-open. So on this lane they RUN; two
consequences are already handled by `hc-mesh.sh`: doorways launch with generous, overridable
`DOORWAY_MEMBRANE_{SHAPE,CHALLENGE,BAN}_THRESHOLD` (every a2o request is one loopback client and the
churn scenarios exceed the binary's 1200/min ban — a tripped membrane answers `403 x-membrane:deny`
to the rest of the lane), and `storage-restart` refreshes fixture pids. A scoped run overwrites the
full-lane cucumber JSON unless you pass your own `CUCUMBER_JSON_REPORT=<path>`; every run still mints
its own run-identified sprint report.

## Build and gate

```bash
just gate                     # changed projects vs origin/dev + worktree
just gate elohim-storage      # one manifest project
just gate doorway/doorway-service
just test app
```

`build-manifest.json gate.projects` owns both detection and typed local
execution. The shared runner resolves explicit cargo-pool workspaces and the
crate-specific `RUSTFLAGS`; direct native `cargo build/test/check/clippy`
without `CARGO_TARGET_DIR` is intentionally denied by the disk guard.

## Seed and inspect

```bash
just seed validate
just seed apply local
just seed stats
just seed diagnose
just look page http://localhost:4200/epr/elohim-protocol
just look graphos list
```

There is currently no content-seed dry-run. Historical `--dry-run` and
`--validate-only` flags were ignored by `seed.ts` and could perform real
writes; use `just seed validate` for the non-writing schema check.

## Health and ports

```bash
just status runtime
just status habits
just status saga
```

| Service | Default | Probe |
|---|---:|---|
| Angular | 4200 | page request |
| Doorway | 8888 | `/health`, `/status`, `/db/stats` |
| Doorway health watchdog | 8079 (A) / 8089 (B) | `/health`, `/ready`, `/health/serving` on their own OS-thread runtime (`DOORWAY_A_HEALTH_PORT`/`DOORWAY_B_HEALTH_PORT`; alpha runs 8079) |
| Conductor app | 4445 | WebSocket |
| Conductor admin | dynamic | `elohim/holochain/local-dev/.hc_ports` |
| Storage | 8090 | `/health`, `/db/stats` |
| Doorway B (mesh) | 8889 | `/health` |
| mongod (mesh) | 27017 | tcp open; `$MESH_DIR/logs/mongod.log` |

## Load-bearing runtime facts

- `app/elohim-app/scripts/hc-start.sh` is the single-peer owner. Its native
  builds are pool-aware; its DNA builds intentionally are not redirected.
- `hc sandbox --piped` reads the lair passphrase from stdin. `-f` pins admin
  ports, `-r` pins app ports, and `-n/-d` create named sandboxes.
- A sandbox with no network section reaches the public Holochain dev network;
  isolation requires the local doorway bootstrap and signal endpoints.
- tx5 signal URLs must be pathless (`ws://signal.localhost:8888`).
- Storage ignores `HTTP_PORT`; pass `--http-port`.
- Storage p2p needs `ENABLE_P2P=true`, `P2P_PORT`, and the conductor agent key.
- Doorway accepts one `--storage-url` plus comma-separated `--storage-urls`.
- Paths are content nodes. There is no `/db/paths` route.

## Focused troubleshooting

```bash
just status runtime
fuser 8888/tcp 8090/tcp 4445/tcp
cat elohim/holochain/local-dev/.hc_ports
curl -s http://localhost:8888/status | jq .
```

If a port is held by an old binary, stop the stack and restart it before
trusting wire shapes. If the shell's `prestart` needs unavailable `wasm-pack`
but the generated package already exists, the specialist escape hatch is:

```bash
cd app/elohim-app
pnpm exec ng serve --proxy-config proxy.conf.mjs --disable-host-check
```

## Canonical files

| File | Purpose |
|---|---|
| `justfile` | public eight-verb interface |
| `app/elohim-app/scripts/hc-start.sh` | single-peer stack |
| `app/elohim-app/scripts/hc-mesh.sh` | local multi-peer mesh |
| `app/elohim-app/scripts/hc-mesh-quiesce.sh` | bounded quiesce measure |
| `app/elohim-app/scripts/hc-mesh-prologue.sh` | Act I Prologue cast (seeds an already-running mesh) |
| `app/elohim-app/scripts/hc-mesh-spin-detector.sh` | conductor spin detector (CPU + log-rate spectroscopy → SPIN/QUIET + JSON) |
| `app/elohim-app/scripts/hc-mesh-chaos-rekey.sh` | stage the unfetchable-dependency class: author on one peer, re-key it, measure the survivors |
| `app/elohim-app/scripts/hc-mesh-perf-watch.sh` | continuous 15 s timing watch: per-service CPU, direct-vs-doorway latency, breaker state, storage zome path; writes `$MESH_DIR/perf/watch.jsonl` + SPIKE lines to `perf/watch.spikes`. Run it after `start` — it is how the doorway first-SSR-render stall was found |
| `genesis/orchestrator/gate-runner.mjs` | manifest gate selection/execution |
| `genesis/agentic/bin/pool-lib.sh` | cargo-pool family and slot authority |
| `elohim/holochain/local-dev/.hc_ports` | local conductor ports |
