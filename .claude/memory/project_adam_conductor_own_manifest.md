---
name: project-adam-conductor-own-manifest
title: Adam's alpha conductor has its own manifest
description: adam-firstman-conductor.yaml, not the shared template, renders adam's conductor — template-only edits never reach adam
metadata:
  type: project
---

Adam (alpha's first genesis peer) renders its conductor StatefulSet from `genesis/orchestrator/manifests/humans/adam-firstman-conductor.yaml`; every other alpha human renders from `_edgenode-conductor.template.yaml`. An env/resource change made only in the template silently skips adam.

**Why:** 2026-11-07 re-genesis 1 — a `TOKIO_WORKER_THREADS` band-aid edited only the template; five template conductors installed, adam kept deadlocking, and I wrongly concluded thread count could not explain adam before reading the edge render log (`sed … adam-firstman-conductor.yaml`).

**How to apply:** for any conductor-manifest change, grep both files (`grep -l <VAR> genesis/orchestrator/manifests/humans/*conductor*.yaml`) and confirm in the edge console which file each human was rendered from before reasoning about a per-peer difference. Related: [[project_conductor_arc_resources]].
