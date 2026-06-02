---
id: "backlog-dockerfile-completeness-lint"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Dockerfile target-completeness lint — catch [[bin]]/[[bench]]/[[example]] + path-deps missing from the Docker context at PR time"
slug: "dockerfile-completeness-lint"
written: "2026-06-02"
author: "cartographer"
status: "proposed"
priority: "high"
area: "cargo"
recurrence: 1
source_shifts:
  - "2026-05-17"
domain: "code"
relatedNodeIds:
  - "memory:feedback_docker_include_str_path_mirroring"
  - "memory:feedback_signature_changes_grep_callers"
tags: [cargo, docker, lint, prepush, code-domain]
shift_objective: |
  A new Cargo target ([[bin]] / [[bench]] / [[example]]) or a new path-dependency crate breaks
  the Docker build context but passes host pre-push, because host `cargo build` sees the whole
  workspace while the Docker context only COPYs what the Dockerfile lists. The result is a
  Docker-only failure discovered ~30 min later in CI, not at PR time (observed 2026-05-17;
  see also the include_str! path-mirroring gotcha).
  Resolve it with a lint that diffs each crate's declared targets + path-deps against what the
  Dockerfile COPYs into the context (or against placeholder/COPY stubs), and fails at PR time
  when a target or path-dep input isn't represented in the build context. This is code-domain
  (a pre-push / lint script reading Cargo.toml manifests + the Dockerfile COPY set), NOT a
  Jenkinsfile edit. Done when adding a [[bin]] or a path-dep crate that the Dockerfile doesn't
  cover fails a committed completeness lint before it reaches CI.
---

# Dockerfile target-completeness lint

## Why this matters

Code-domain. The Docker-only failure mode is uniquely expensive — it passes every host gate
and only blows up in the containerized build, costing a full CI cycle to discover. Catching it
at PR time pays for itself the first time.

## The failure shape

- A new `[[bin]]`/`[[bench]]/[[example]]` target, or a new path-dependency crate, is added.
- Host `cargo build` sees the whole workspace and passes pre-push.
- The Docker context only COPYs the inputs the Dockerfile lists; the new target/dep isn't
  there → the Docker build fails ~30 min into CI.

## Shape of the fix (code-domain)

A lint that compares each crate's declared targets + path-deps (from Cargo.toml) against the
Dockerfile's COPY set / placeholder stubs, failing at PR time on any input not represented in
the build context. Mind `feedback_docker_include_str_path_mirroring` (include_str! resolves
relative to source in-container; parent-dir refs need COPY mirrors). Pre-push / lint script,
not a Jenkinsfile edit.

## Acceptance

Adding a `[[bin]]` or path-dep crate the Dockerfile doesn't cover fails a committed
completeness lint before it reaches CI.
