---
name: project_new_path_dep_needs_dockerfile_copy
description: "a new path-dep crate in doorway/storage Cargo.toml needs a COPY + sed path-rewrite in BOTH Dockerfiles, or the edge build floor breaks — uncaught until an edge build runs"
metadata: 
  node_type: memory
  type: project
  originSessionId: ca4a672a-d664-44dd-b21f-64d780015b5d
---

The edge image build flattens the workspace into a build context, so each path-dep crate referenced by `doorway/doorway-service/Cargo.toml` or `elohim/elohim-storage/Cargo.toml` needs TWO things in the respective Dockerfile: (1) a `COPY <crate-path> ./<flattened-path>` line, and (2) a `sed -i 's|path = "../../<crate>"|path = "<crate>"|' Cargo.toml` path-rewrite. The two Dockerfiles use DIFFERENT relative prefixes — `doorway/doorway-service/Dockerfile` rewrites `../../…`, `elohim/elohim-storage/Dockerfile` rewrites a single `../`.

When a NEW path-dep crate lands (e.g. `elohim-peer-fabric`, added 2026-06-20), BOTH Dockerfiles break with `failed to read /<crate>/Cargo.toml` and the edge build fails before any stage of value.

**Why:** it's caught LATE — the orchestrator only dispatches the edge pipeline on `dev` (and `claude/*`), so a `feat/*` or `sprint/*` push that adds the crate never triggers an edge build (see [[project_sprint_branch_not_orchestrator_indexed]]); the gap surfaces only when the change reaches dev. Recurring class: the same missing-COPY shape bit edge-#1007.

**How to apply:** whenever a Rust workspace change adds a path-dep to doorway-service or elohim-storage, grep both Dockerfiles for the existing crate COPY/sed pattern and add a matching pair before it reaches dev. When a shipped Rust workspace change won't build on edge, check Dockerfile COPY/sed coverage FIRST. Fix reference: commit 72a623439 (peer-fabric staged into both contexts; edge build went green after).
