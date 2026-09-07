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

**Also needed (2026-09-08, sprint):** schema widening for `run.cargo.env` — the pinned
`elohim/rakia` submodule's build-manifest schema currently refuses `GateProject.run.cargo.env`
(`/gate/projects/elohim-storage/run/cargo must NOT have additional properties`), which is why a
per-project cargo resource cap (T8's storage gate `CARGO_BUILD_JOBS=1`) had to move to
`genesis/agentic/pool-policy.json`'s `cargo_env_overrides` instead of living on the manifest
project itself. The mirror shape to copy is already in
`genesis/orchestrator/manifest.schema.json:161-166`. See
`genesis/data/timeline/backlog/cargo-test-memory-shed-storage-gate.md` (2026-09-08 amendment)
for the full account.
