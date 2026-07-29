---
id: "backlog-ci-storage-conductor-pin-retention"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Storage Dockerfile's edgenode digest pin ages out under Harbor retention — pass the same-run image instead"
slug: "ci-storage-conductor-pin-retention"
written: "2026-07-29"
author: "deliver-the-saga morning sprint"
status: "open"
priority: "medium"
ci_status: open
jobs: [elohim-edge]
tags: [ci, harbor, retention, docker, edge, delegable]
---

# Conductor-source digest pin vs Harbor retention

`elohim/elohim-storage/Dockerfile:28`'s `ARG CONDUCTOR_SOURCE_IMAGE` pins the
jemalloc edgenode by digest for rollback safety, but Harbor retention keeps only
the most recent edgenode artifacts — a busy multi-push day (2026-07-29, three
waves) evicted the pinned digest and edge #1261 died at
`load metadata … not found`. The pin was repointed to the then-current
dev-latest digest as the unblock; that recreates the same time bomb.

## Scope (disjoint, delegable)

Structural options, pick one:
1. **Same-run coherence (preferred):** `scripts/ci/build-storage-image.sh`
   passes `--build-arg CONDUCTOR_SOURCE_IMAGE=harbor…/elohim-edgenode:1.0.0-dev-<commit>`
   for the edgenode image built earlier in the same edge run (EDGENODE_TAG is
   already in the pipeline env); the Dockerfile default stays as documented
   fallback for local builds. Requires the edgenode build/push stage to be
   ordered before the storage image build — verify in
   `elohim/holochain/Jenkinsfile` and reorder if needed.
2. **Retention-protected pin:** give the pinned digest a tag Harbor retention
   excludes (e.g. `conductor-pin-jemalloc`) and update the retention rule.

Keep the Dockerfile's jemalloc rollback warning comment either way.

## DoD

- An empty-commit `[build:edge]` run builds the storage image without touching
  the Dockerfile pin, on a day with 3+ prior edgenode pushes.
- Local `docker build -f elohim/elohim-storage/Dockerfile .` still resolves.
