---
name: project_storage_build_under_ram_guard_debuginfo_off
description: "elohim-storage cargo test/build gets shed by the 80% RAM guard while the mesh is up; cure in-tree since 2026-09-05 — line-tables-only debuginfo + cargo-jobs-flag -j1; just gate picks it up"
metadata: 
  node_type: memory
  title: Storage builds under the RAM guard
  type: project
  originSessionId: f84d8f67-f26c-4b08-8a6f-6cfd7e6cf12d
  modified: 2026-09-05T20:07:31.913Z
---

**Trap (2026-09-04 overnight):** with the 3-peer 0.7 mesh running, every plain `cargo build --features "p2p p2p-iroh"`
and `cargo test --lib …` of `elohim/elohim-storage` was shed by the RAM guard at 80% (6.2–6.6 GB rustc peak; the
guard left ~5.5 GB), at jobs=4, jobs=2, jobs=1 and with `CARGO_INCREMENTAL=0`. The stock 0.7 conductors had grown
from ~2 GB to ~4.3 GB RSS each over 7 h (12.6 GB total). `conductors-restart` (which would free ~6 GB) is refused by the
auto-mode classifier.

**Cure that worked (EXIT=0, 3m12s, one job):**
```
CARGO_TARGET_DIR=<pool slot> CARGO_BUILD_JOBS=1 cargo build --features "p2p p2p-iroh" \
  --config "profile.dev.package.elohim-storage.debug=0"
```
Only the top crate recompiles without debuginfo (deps stay cached); the binary is ~90 MB smaller and runs the mesh fine
(`storage-restart` onto it 08:19Z). The same flag should let the `--lib` test target fit.

**2026-09-05 (integration push #4):** line-tables-only debuginfo alone was NOT enough — `cargo test` compiles the lib
and the test harness as TWO ~5.5 GB rustc processes and the guard shed it at 80.5% (13 GB). Durable cure now in-tree:
`elohim/elohim-storage/.cargo/config.toml` (line-tables-only for the top crate) + the storage justfile's `jobs_flag`,
computed by `genesis/agentic/bin/cargo-jobs-flag` (`-j1` whenever `ram-guard status` shows < 14 GB headroom; CI has
no guard → cargo's default). `just gate elohim-storage` picks both up; nothing to type.

**Why:** debuginfo generation is the marginal memory of the top-crate codegen; the guard's ceiling is a policy
(memory.high is 88%), not a kernel limit, but building past it risks an OOM of a 4 GB conductor (oom.group=0 kills the
largest process). **How to apply:** reach for the flag first on any mesh-up storage build; note conductor RSS growth as a
resource finding for the conductor-arc habit (see [[project_conductor_arc_resources]]); the conductors-restart arm needs
the operator's hand or an allowlist rule. Related: [[project_local_mesh_binary_slot_and_restart]], [[project_ram_guard_oom_group_kill]].
