---
id: "backlog-doorway-warm-shell-local-archive-mesh-provable"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Warm-shell archive is Mongo-only, so the 18a65fd0d wiring cure is fleet-only evidence — add a SQLite/directory ShellArchive so the local mesh can prove it"
slug: "doorway-warm-shell-local-archive-mesh-provable"
written: "2026-08-21"
author: "claude (local-pair validation session, /btw fork analysis)"
status: "backlog"
priority: "medium"
jobs: [elohim-edge]
nodes: [elohim-doorway-alpha, elohim-doorway-alpha-b]
relatedNodeIds:
  - "memory:project_local_pair_failover_validation_rail"
  - "memory:project_freshness_graded_by_declared_stakes"
tags: [doorway, warm-shell, local-mesh, dataplane, doorway-failover, desk-proven-is-not-wired]
cites:
  - doorway/doorway-service/src/render/warm_shell.rs
  - doorway/doorway-service/src/cache/app_file_cache.rs
  - doorway/doorway-service/src/server/http.rs
  - app/elohim-app/scripts/hc-mesh.sh
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

## Task (well-specified, disjoint — any agent may claim)

1. Add a `SqliteShellArchive` (or a plain directory store under `$MESH_DIR`) implementing `ShellArchive`;
   select it when Mongo is unconfigured (`--dev-mode` mesh), keep Mongo authoritative when configured.
2. Route it through the SAME `bind_warm_shell_to_archive()` path as Mongo — the invariant under test is
   "every archive-installing path rebuilds the store"; do not add a second wiring path.
3. Keep the inert-store tests (`an_inert_store_hydrates_nothing`); the degrade path is a contract.
4. Mesh proof: `hydrated: N>0` in the boot log; `/` cache-first; a2o scenario on the local pair asserting
   `x-elohim-bundle: last-reconciled` (and `x-elohim-freshness: amber` once the freshness verdict lands)
   through a storage outage (`fuser -k 8090/tcp` drill per the memory rail).
5. Sequence AFTER the freshness-verdict work lands (it edits `AppState` construction in
   `server/http.rs` — same write-set).

## Parity leg (separate, image change)

A real `mongod` as one more loopback process in `hc-mesh.sh` is the only thing that proves the exact
production path; needs a devfile/image change and must live under `commands:` (exec on demand), never
postStart. Mongo's absence is also why the SSR "boot-once" gap in
`doorway-boot-self-heal-family-mesh-repro.md` cannot be fully reproduced locally.
