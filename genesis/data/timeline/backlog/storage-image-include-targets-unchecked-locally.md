---
id: "backlog-storage-image-include-targets-unchecked-locally"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A compile-time include outside src reaches the storage image build unchecked by the local gate"
slug: "storage-image-include-targets-unchecked-locally"
written: "2026-10-05"
author: "claude-opus-5-5 (shem, shift event-driven-head-delivery)"
status: "resolved"
priority: "medium"
jobs: [elohim-edge]
cluster: "ci-gates"
relatedNodeIds:
  - "habit:push-delivers-within-budget"
tags: [ci, docker, gate-gap, host-green-not-ci-green]
---

Edge #1541 (2026-10-05, commit `3e92ffe88`) failed at `Build Storage`: `PolicyConfig::builtin()`
made `include_str!("../../config/peer-policy.example.toml")` part of the release lib, and the
Dockerfile's `builder` stage did not copy `config/` (only the `check` stage did). The local gate
passed 4298 tests because the file is present in the tree. The fleet did not roll and an hour of
pipeline was spent. Edge #1137 failed the same way on a different file.

Owed: a pre-push check that every `include_str!` / `include_bytes!` target in
`elohim/elohim-storage/src` that resolves outside `src/` is covered by a `COPY` in the stage that
compiles it (`builder` for non-test code, `check` for test code). Text-level is enough: resolve
each path against the crate root and match it against the stage's COPY sources. It belongs on the
`elohim-storage` gate project so it runs wherever the tests do.

## Resolved, 2026-10-05

`scripts/ci/check-image-include-targets.py`, run by `just image-includes` and by the storage
`gate` recipe. It resolves every `include_str!` / `include_bytes!` target that leaves `src/`,
classes it as test or release code, and requires a `COPY` covering it in the `check` or `builder`
stage's chain. On the Dockerfile at `3e92ffe88` it names `config/peer-policy.example.toml` missing
from `builder` and exits 1; on the fixed Dockerfile it passes (11 targets). Text-level: no docker,
no cargo, under a second. Not covered: targets built from `concat!` or `env!`, and other crates'
Dockerfiles (pass `--crate`).
