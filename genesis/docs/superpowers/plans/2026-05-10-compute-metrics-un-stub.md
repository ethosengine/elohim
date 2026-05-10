# Compute-metrics un-stub: live ComputeMetricsView + SSR concurrency derivation

**Status**: in flight (2026-05-10)
**Owner**: dev
**Predecessor**: `2026-05-08-ssr-capability-implementation.md` (Known follow-up section)
**Memory**: `project_storage_as_pod_operator_sets_virtual_limits`

## Why

The SSR sprint shipped `DEFAULT_MAX_CONCURRENT = 8` as a static fallback in `doorway/doorway-service/src/render/capability.rs` because the per-node compute surface in elohim-storage is typed but stubbed. SSR's plan footnote queues the un-stub as its own sprint, since correctness for any per-node compute feature (transcode budget, indexer parallelism, future things) requires it — not just SSR.

The architectural shape is fixed (per memory): elohim-storage is a pod, elohim-operator (today: env vars; later: DHT-attested policy) imposes virtual limits via `AllocationBlockView` + `CeilingLimitView`, and consumers compute `min(probes, allocation, ceiling)`.

This sprint un-stubs the surface and wires the first consumer (SSR's `max_concurrent_renders`) to it.

## Scope

| In | Out |
|---|---|
| CPU count + load-average probes (`services/system_metrics.rs`) | Network bandwidth probes (no foundation today) |
| Live `ComputeMetricsView` from existing + new probes (`api/compute.rs`) | History series (`cpu_usage_history`, etc.) — separate sampler sprint |
| Env-driven `AllocationBlockView` + `CeilingLimitView` defaults | DHT-attested operator policy (Stage-3 elohim-defender) |
| Doorway subscribes to compute view; derives SSR max_concurrent from it | Other consumers (transcode, indexer) — done when those land |
| Cross-stack integration test (storage → doorway → SSR cap) | Angular dashboard polish (SheafaDashboardStateView consumers) |

## Phases

### Phase 1 — Probes (storage)

`elohim/elohim-storage/src/services/system_metrics.rs`:

- `pub fn cpu_count() -> Option<u32>` via `std::thread::available_parallelism()` (matches `doorway/orchestrator/node_bootstrap.rs:144` precedent).
- `pub fn load_average() -> Option<(f64, f64, f64)>` via `libc::getloadavg` on Linux/macOS/BSD; returns `None` on syscall failure or non-POSIX. (Linux-target builds always have this; document the cfg gate.)
- Tests mirror existing pattern (assert nonzero on real host).

### Phase 2 — Live ComputeMetricsView (storage)

`elohim/elohim-storage/src/api/compute.rs`:

- Rename `default_compute_metrics()` → `current_compute_metrics(ctx: &AppContext)`.
- Populate from probes:
  - `cpu_total_cores` ← `system_metrics::cpu_count()` (fallback 0 = "unknown")
  - `cpu_available` ← same as total (no per-process partitioning yet)
  - `memory_total_gb` ← `total_memory_bytes() / 1e9`
  - `memory_used_gb` ← `process_memory_bytes() / 1e9`
  - `memory_available_gb` ← `total - used` (or 0 if either unknown)
  - `storage_total_gb` ← `filesystem_capacity_bytes(blob_store_path) / 1e9`
  - `storage_breakdown_cache_gb` ← `directory_size(blob_store_path) / 1e9`
  - `storage_used_gb` ← same (best signal we have for "this pod's slice")
  - `load_average_one_minute` / `_five_minutes` / `_fifteen_minutes` ← from `load_average()`
- Network metrics + history series: stay zero with a `// TODO: network probe sprint` comment. Document in module header.
- Drop the "stubbed until sysinfo integration" comment in module header.

### Phase 3 — Env-driven operator limits

`elohim/elohim-storage/src/api/compute.rs`:

- `current_ceiling_limit()` reads:
  - `ELOHIM_OPERATOR_CEILING_MAX_CORES` (f64, default = `cpu_count()` — i.e., no virtual cap)
  - `ELOHIM_OPERATOR_CEILING_MAX_MEMORY_GB` (f64, default = total_memory_bytes/1e9)
  - `ELOHIM_OPERATOR_CEILING_MAX_STORAGE_GB`, `..._BANDWIDTH_MBPS` (default 0 = unknown)
- `current_allocation_block()` reads:
  - `ELOHIM_OPERATOR_ALLOCATION_CPU_CORES` (f64, default = `cpu_count()`)
  - `ELOHIM_OPERATOR_ALLOCATION_MEMORY_GB` (f64, default = total memory)
- Tests: serialize env mutations behind a `static OPERATOR_LIMITS_ENV_LOCK: Mutex<()>` per `feedback_env_var_test_flakiness`.

### Phase 4 — Doorway subscribes

`doorway/doorway-service/src/render/capability.rs`:

- New env var: `STORAGE_COMPUTE_URL` (e.g., `http://localhost:8090/api/v1/compute/dashboard`).
- New helper: `fetch_compute_budget(url) -> Option<ComputeBudget>` returning `{cpu_total_cores, ceiling_max_cores, allocation_cpu_cores}`.
- `derive_capability` signature gains an optional `compute_budget: Option<ComputeBudget>`:
  - Default `max_concurrent_renders = min(cpu_total_cores, ceiling_max_cores, allocation_cpu_cores)`, falling back to `DEFAULT_MAX_CONCURRENT` only when probe returned None.
  - Operator override (`override.toml#max_concurrent`) still wins (and may only reduce — already enforced).
- Drop the long TODO comment above `DEFAULT_MAX_CONCURRENT`.
- Caller (`render/mod.rs` or wherever `derive_capability` is invoked) fetches the budget at startup and passes it.

### Phase 5 — Cross-stack integration test

`elohim/elohim-storage/tests/` or `doorway/doorway-service/tests/`:

- Storage end: assert `/api/v1/compute/dashboard` returns non-zero `cpuTotalCores` and `memoryTotalGb` on a real test host.
- Doorway end: with a `wiremock` storage stub serving a known compute view (cpu=4, ceiling=2, allocation=8), assert `derive_capability` returns `max_concurrent = 2`.
- SSR layer: parametrized test that the semaphore is sized from the derived value (already covered by `requests_beyond_max_concurrent_get_csr_fallback`; no change needed).

### Phase 6 — Verify

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --workspace -- --test-threads=1   # storage (env-mutating tests)
RUSTFLAGS="" cargo test --lib --bins                                                       # doorway
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings                   # storage
RUSTFLAGS="" cargo clippy -- -D warnings                                                   # doorway
cargo fmt --check
```

### Phase 7 — Memory + comment cleanup

- Update `project_storage_as_pod_operator_sets_virtual_limits` to mark probes + ComputeMetricsView **live**; allocation/ceiling **env-driven** (DHT-attested still pending).
- Drop "stubbed until sysinfo integration" / "Compute metrics (system-level, stubbed...)" comments in `api/compute.rs`.
- Update SSR plan footnote: "follow-up shipped in 2026-05-10 sprint."

## Anti-patterns to avoid

1. **Hard-coding per-feature defaults** (memory: `feedback_check_existing_compute_foundation`). Always derive from `current_compute_metrics()` + operator limits.
2. **Filesystem path proliferation** (memory: `feedback_check_existing_compute_foundation`). Keep `fs4` + `directory_size` as the single foundation.
3. **Cross-node aggregation in this layer** (memory: `project_node_metrics_vs_hub_aggregation_boundary`). All probes are per-node; sums belong in the household-hub surface.
4. **Network probes in this sprint**. No foundation today, no requirement from current consumers — explicit "TODO: network probe sprint" comment is correct.
5. **Subagent dispatch on signature changes** without explicit forbid (memory: `feedback_subagent_dep_conflict_supervision`, `feedback_signature_changes_grep_callers`). The `derive_capability` signature change must grep callers in the storage + doorway tree before committing.

## Out of scope (queued for follow-ups)

- History series sampler (`cpu_usage_history`, `memory_usage_history` rolling buffers).
- Network bandwidth probe (would need either NetlinkSock or a sampling foundation).
- Storage breakdown beyond `cache_gb` (holochain_gb / custodian_data_gb / user_applications_gb belong to other pods in the household — surface them from the household-hub aggregator).
- DHT-attested operator policy (Stage-3 elohim-defender enforcement of `AllocationBlockView` + `CeilingLimitView`).
- Angular consumer polish for `SheafaDashboardStateView`.

## Verification gates

| Gate | Command | Expected |
|---|---|---|
| storage unit/integration | `cargo test --workspace -- --test-threads=1` | green |
| doorway unit/bin | `RUSTFLAGS="" cargo test --lib --bins` | green |
| storage clippy | `cargo clippy -- -D warnings` | clean |
| doorway clippy | `RUSTFLAGS="" cargo clippy -- -D warnings` | clean |
| fmt | `cargo fmt --check` (root) | clean |
| Cross-stack integration | new test in Phase 5 | green |
| Live host smoke | `curl /api/v1/compute/dashboard \| jq .computeMetrics.cpuTotalCores` | nonzero |
