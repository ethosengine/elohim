---
id: "backlog-task-conductor-image-tag-runtime-visibility"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Task: bake the conductor image tag (conductor-<hc12>-<tx512>) into the storage image as a runtime-readable env"
slug: "task-conductor-image-tag-runtime-visibility"
written: "2026-08-31"
author: "session-2026-08-31-velocity-snowball"
status: "open"
priority: "medium"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-task-runtime-passport-endpoint"
  - "backlog-upgrade-propagation-p2p-design-arc"
tags: [observability, ci, delegable]
---

**Claimable by any implementation agent. Small, fully disjoint.**

## Why

The conductor image tag `conductor-<hc12>-<tx512>` encodes the exact
holochain-fork and tx5-fork SHAs that determine the conductor binary — the
component with the most cross-peer behavioral variance. Today that
knowledge dies at build time: `CONDUCTOR_SOURCE_IMAGE` is a docker
build-arg (`scripts/ci/build-storage-image.sh` →
`elohim/elohim-storage/Dockerfile`) and nothing at runtime can answer
"which conductor build is this peer running." The runtime passport
(companion task) wants to serve it.

## Scope

1. In `elohim/elohim-storage/Dockerfile`: persist the existing
   `CONDUCTOR_SOURCE_IMAGE` build-arg (or just its tag suffix) as an
   `ENV CONDUCTOR_IMAGE_TAG=...` in the final image stage, so every
   container from this image carries it.
2. Verify read-only that `scripts/ci/build-storage-image.sh` already passes
   the build-arg. It does; do not edit or restructure the script.
3. Local-dev parity note in the Dockerfile comment: locally-built binaries
   won't have the env; consumers must treat absence as `"unknown"`.

## Disjointness contract

- The delegated implementation agent (Codex or equivalent) MAY edit
  `elohim/elohim-storage/Dockerfile` only.
- It MUST NOT edit `scripts/ci/build-storage-image.sh`; `http.rs` (including
  the `/version` match arm); `happ_manager.rs`; any Jenkinsfile; any
  deployment/orchestrator manifest; `hc-mesh.sh`;
  `src/p2p/view_federation.rs`; or any other Rust source. Those are the rung
  lane's surfaces this week.

## DoD + verification

- `docker build` is unavailable in the dev container — verification is
  static: show the Dockerfile stage where the ENV lands and confirm the
  build-arg name matches what build-storage-image.sh passes
  (`grep -nE 'ARG CONDUCTOR_SOURCE_IMAGE|ENV CONDUCTOR_IMAGE_TAG|--build-arg CONDUCTOR_SOURCE_IMAGE' elohim/elohim-storage/Dockerfile scripts/ci/build-storage-image.sh`).
- The named CI backstop: the next edge build's image should show the env
  via the passport endpoint once both tasks land.
