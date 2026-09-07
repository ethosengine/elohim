---
id: "backlog-rakia-executor-untracked-in-submodule-pin"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "elohim/rakia/rakia-executor (the compute-executor binary the delegated-compute leg runs) is UNTRACKED inside the rakia submodule — pin 13cb31d does not contain it, so CI and any fresh clone cannot build the executor the operations guide and worker image depend on"
slug: "rakia-executor-untracked-in-submodule-pin"
written: "2026-09-07"
author: "overnight shift 2026-09-07"
status: "open"
priority: "high"
jobs: [rakia, elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
tags: [rakia, submodule, delegated-compute, ci]
---

**Evidence (2026-09-07 05:4xZ, compute-leg prep):** `git -C elohim/rakia status` shows `rakia-executor/` untracked; the superproject pin is `13cb31dddb…` (heads/main). The binary at `/projects/.cargo-target-pool/family/dev/elohim__rakia/dev/debug/compute-executor` was built from that uncommitted work. `scripts/ci/build-compute-worker.sh` and `genesis/agentic/compute/operations.md` both require it. **Cure (operator-owned, the submodule is theirs):** commit `rakia-executor/` in the rakia repo, push, bump the pin in elohim. **Done when:** a fresh `git submodule update --init elohim/rakia` contains `rakia-executor/Cargo.toml` and `just gate rakia` builds `compute-executor`.
