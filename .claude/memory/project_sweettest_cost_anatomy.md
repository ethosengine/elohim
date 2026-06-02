---
name: project_sweettest_cost_anatomy
description: "Sweettest \"hour\" is ~72% Rust compile not test-run; local husky gate is check-only and was cold-evicted by legacy-targets clean"
metadata: 
  node_type: memory
  type: project
  originSessionId: c525e164-d9a4-4717-9a30-fcd1392710b6
cites:
  - elohim/holochain/tests/sweettest/src/common/conductors.rs
---

Investigated 2026-05-29 (Jenkins builds #1302/#1304, grounded). Where the sweettest cost lives:

**CI (`elohim-holochain/dev`, the only pipeline that runs sweettests):** of a ~52min build,
the `cargo nextest archive` **compile** is ~37min (~72%); 5-DNA WASM build ~3.7min; actual test
**execution** only ~3min (4 parallel shard pods, `--partition hash:N/4`, bottleneck shard-2 ~188s);
rest is Nix/checkout/Harbor/scheduling. So it is a COMPILE problem, not a test-run problem. The
crate statically links the entire Holochain conductor. Big architectural wins already shipped
(warm-PVC, compile-once+shard, per-DNA `@dna-scope` selectivity via build-nextest-filter.sh).
Biggest untaken CI win: add `[profile.release]` to `tests/sweettest/Cargo.toml` (it has NONE →
full opt+debuginfo); `debug="line-tables-only"` + dep `opt-level=1` + `codegen-units=256`. sccache
stays OFF for sweettest by design (spawn-ENOENT RCA). Held per operator on 2026-05-29.

**Value verdict:** keep them — they test what unit tests can't (cross-agent DHT consistency,
cross-agent validator rejection, coordinator→integrity round-trips, signal replay, wire-format
drift). ~530 fns / ~28 domain files; execution is cheap so value-per-runtime is high.

**Local husky pre-push gate (`sweettest-check`):** is `cargo check --tests` — COMPILE-CHECK ONLY,
does not run tests; "~30–90s warm". The cold-eviction root cause (non-obvious): the gate didn't
set `CARGO_TARGET_DIR`, so it built into in-tree `tests/sweettest/target/`, which is a
NATIVE_WORKSPACES member that `cargo-pool legacy-targets --clean` (the 89%-disk banner's own
suggestion) reclaims — wiping the warm cache → next push cold. The disk-hygiene loop and the
slow-push loop were the same loop. Fixed 2026-05-29 in `.husky/pre-push`: L1 `SKIP_SWEETTEST=1`
selective bypass, L2 scoped the over-broad fallback grep to `sweettest/(src/|Cargo.toml)`, L3
redirect the gate into the pooled slot (fail-open). See [[feedback_multi_agent_pvc_pacing]],
[[project_ci_storage_topology]]. Full report: /projects/research/sweettest-efficiency-2026-05/.
