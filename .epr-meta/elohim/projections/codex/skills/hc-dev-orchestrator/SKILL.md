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

The alpha-shaped local topology is one doorway plus N conductor/storage peers
(default matthew, jessica, james), all on loopback:

```bash
just mesh start
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

`just mesh monitor` (hc-mesh-monitor.py, port 4210 via the `mesh-monitor`
devfile endpoint; honors `MESH_MONITOR_PORT`) serves the one-page live
dashboard: component liveness, per-peer convergence gauges, a gate-legs
panel mirroring `fleet-quiesce-gate.sh`'s exact PASS predicate, log tails,
and a phase/progress status bar.

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
| Conductor app | 4445 | WebSocket |
| Conductor admin | dynamic | `elohim/holochain/local-dev/.hc_ports` |
| Storage | 8090 | `/health`, `/db/stats` |

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
| `genesis/orchestrator/gate-runner.mjs` | manifest gate selection/execution |
| `genesis/agentic/bin/pool-lib.sh` | cargo-pool family and slot authority |
| `elohim/holochain/local-dev/.hc_ports` | local conductor ports |
