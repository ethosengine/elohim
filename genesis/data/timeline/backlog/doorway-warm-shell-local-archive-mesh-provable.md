---
id: "backlog-doorway-warm-shell-local-archive-mesh-provable"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Warm-shell archive is Mongo-only, so the 18a65fd0d wiring cure is fleet-only evidence — run a real mongod in the local mesh so the production wiring path is desk-provable"
slug: "doorway-warm-shell-local-archive-mesh-provable"
written: "2026-08-21"
author: "claude (local-pair validation session, /btw fork analysis)"
status: "wip"
priority: "medium"
jobs: [elohim-edge]
nodes: [elohim-doorway-alpha, elohim-doorway-alpha-b]
relatedNodeIds:
  - "memory:project_local_pair_failover_validation_rail"
  - "memory:project_freshness_graded_by_declared_stakes"
tags: [doorway, warm-shell, local-mesh, mongodb, dataplane, doorway-failover, desk-proven-is-not-wired, che-devworkspaces]
cites:
  - doorway/doorway-service/src/render/warm_shell.rs
  - doorway/doorway-service/src/cache/app_file_cache.rs
  - doorway/doorway-service/src/server/http.rs
  - app/elohim-app/scripts/hc-mesh.sh
  - doorway/doorway-service/src/config.rs
  - genesis/data/timeline/backlog/doorway-breaker-trial-theft-fleet-verification.md
  - genesis/data/timeline/backlog/doorway-boot-self-heal-family-mesh-repro.md
---

# Warm-shell archive is Mongo-only — the local mesh cannot prove the wiring cure

## The gap

`18a65fd0d` fixed the warm-boot shell cache never being wired in production (every constructor
built `WarmShellStore::inert()`; `init_projection` installed the Mongo `app_file_cache` but never
rebuilt the store). The cure is host-green, but the **local mesh runs no MongoDB**, so on the desk the
store is inert *by design* and the cure is unobservable — the exact "desk-proven is not wired" lesson
written into the `doorway-failover` habit. Today it is fleet-only evidence (`[build:edge] [edge:validate-only]`).

MongoDB has no embedded/in-process mode (no "H2 for Mongo"). FerretDB is the nearest wire-compatible
stand-in but is still a separate server, absent from this container, with an unverified driver-compat surface.

## The seam already exists

`ShellArchive` (`doorway/doorway-service/src/render/warm_shell.rs:68`) — four async methods
(`declared_blob_hash`, `load`, `load_latest`, `store`). Impls today: the Mongo-backed
`AppFileCacheService` and a test-only `FakeArchive`. Archive entries are content-addressed
(`{slug}:{file_path}:{blob_hash}`); the Mongo features used are an upsert and a `last_accessed` touch.

## Landed 2026-08-21 (same day)

- che-devworkspaces `9409696`: `containers/udi-plus/Dockerfile` installs `mongod` 8.0.12 (pinned rhel93 tarball, sha256-gated); runs on the UBI10 lineage (glibc 2.39, OpenSSL 3.5). Until the image rebuilds the binary lives at `/home/user/bin/mongod` (ephemeral, same class as the gh CLI).
- elohim `cb0e182b4`: `just mesh start` brings up a loopback `mongod` (:27017, dbpath `$MESH_DIR/mongo`) before the doorways; A/B get `doorway-a`/`doorway-b`; `mesh stop`/`status` cover it; no binary ⇒ previous behaviour, said aloud.
- elohim `1f81a3cc1`: doorways receive the mesh's `ELOHIM_NETWORK_STAKES=simulacra`.
- Desk proof: both doorways boot `archive_backed=true`, `hydrated=1`; with matthew's storage killed, `/` → 200 in 0.9 ms with `x-elohim-bundle: last-reconciled`. Remaining: the image rebuild (so a fresh workspace has `mongod` without the ephemeral binary) and the a2o scenario asserting the marker through an outage on the local pair.

## Decision 2026-08-21 (operator, via /btw thread): real `mongod` in the mesh, NOT a SQLite shim

A second `ShellArchive` impl plus a selector would prove only a convenience path; the in-process wiring
invariant is already pinned by `binding_the_archive_lights_the_warm_shell_and_the_invariant_holds`. What
the desk cannot yet prove is the PRODUCTION path — Mongo-backed `app_file_cache` → `init_projection` →
archive — and only a real `mongod` proves that.

What it costs, from the code:

- **Doorway side: nothing.** `MONGODB_URI` defaults to `mongodb://localhost:27017`, `MONGODB_DB` to
  `doorway` (`config.rs:97,101`); with nothing listening it logs "MongoDB connection failed (dev mode,
  continuing without)" and goes inert. The moment a `mongod` is on 27017 both mesh doorways wire the
  archive through the same `init_projection` path production uses.
- **Mesh side: one leg** in `hc-mesh.sh` before the doorways start —
  `mongod --dbpath $MESH_DIR/mongo --bind_ip 127.0.0.1 --port 27017 --fork --logpath $MESH_DIR/logs/mongod.log`
  — plus stop/status lines. Give doorway B its own database (`MONGODB_DB=doorway-b`): a shared archive
  between A and B would hide exactly the per-doorway boot-order class this exists to reproduce.
- **Image side: the real cost.** No `mongod`/`mongosh`/`ferretdb` binary exists in this container. Added in
  the che-devworkspaces submodule (`containers/` for the image; `devfile.yaml` under `commands:` — exec on
  demand, never postStart). Watch-out: the dev image is UBI10 lineage and MongoDB's RPMs target RHEL 8/9 —
  confirm a mongod build installs on UBI10 before committing; if it does not, FerretDB (single static Go
  binary, Mongo wire protocol) is the fallback — it proves the driver path, not `mongod` itself.

Mesh proof once landed: `hydrated: N>0` in both doorway boot logs; `/` cache-first; a2o scenario on the
local pair asserting `x-elohim-bundle: last-reconciled` (and `x-elohim-freshness: amber` once the
freshness verdict lands) through a storage outage (`fuser -k 8090/tcp` drill per the memory rail). With
Mongo in the mesh the SSR "boot-once" gap in `doorway-boot-self-heal-family-mesh-repro.md` also becomes
fully reproducible locally — the same absence was blocking it.

Sequence: the mesh-script leg is disjoint from the freshness work; the image change lives in the
submodule (pushes straight to che-devworkspaces main, inert-by-default; force the image with `[build:conductor]`
is NOT the right tag — it is the devspace image, not the conductor image).
