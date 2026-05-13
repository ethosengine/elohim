---
name: elohim-storage as pod / elohim-operator as virtual-limit setter
description: Per-node compute features (SSR concurrency, transcode budget, etc.) must subscribe to ComputeMetricsView; elohim-operator imposes virtual limits via AllocationBlockView + CeilingLimitView; treating these as hard limits is the dwellinghub-orchestration model. Probes + view live (2026-05-10), allocation/ceiling env-driven until DHT-attested policy
type: project
originSessionId: f5eddd4d-c824-4266-b68e-fd61a7afac58
---
elohim-storage models itself as a pod that elohim-operator orchestrates as part of a household/dwellinghub fabric. Three layered surfaces:

| Layer | What it represents | Where | Status (2026-05-10) |
|---|---|---|---|
| Probes | What the hardware actually has | `services/system_metrics.rs` | **live**: filesystem, memory, CPU count (`std::thread::available_parallelism`), load average (`libc::getloadavg`); network bandwidth still TODO |
| Allocation | Operator's "I'm using N cores for this workload" virtual partitioning | `AllocationBlockView` (`api/compute.rs::current_allocation_block`) | **env-driven**: `ELOHIM_OPERATOR_ALLOCATION_CPU_CORES`, `_MEMORY_GB` (defaults to probes); DHT-attested when policy lands |
| Ceiling | Operator's hard "do not exceed M cores in this dwellinghub" | `CeilingLimitView` (`api/compute.rs::current_ceiling_limit`) | **env-driven**: `ELOHIM_OPERATOR_CEILING_MAX_CORES`, `_MEMORY_GB`, `_STORAGE_GB`, `_BANDWIDTH_MBPS` (defaults to probes / 0); DHT-attested when policy lands |
| View | Typed read surface every consumer subscribes to | `ComputeMetricsView` + `/api/v1/compute/dashboard` | **live**: `current_compute_metrics()` populated from probes; doorway subscribes via `STORAGE_COMPUTE_URL` |

**Why:** elohim-operator IS the virtual-limit setter for the household compute mesh. Even when the limits are virtual (k8s modeling, dev convenience), peers must treat them as authoritative — that's how the operator orchestrates compute across blades in a dwelling without each feature inventing its own throttle.

**How to apply:** When adding a per-node compute feature (SSR `max_concurrent_renders`, transcode budget, indexer parallelism), do not hard-code defaults. Subscribe to ComputeMetricsView and pick `min(probes, allocation, ceiling)`. Operator override (env vars, override.toml) is the debugging escape hatch, not the primary input.

**Reference implementation:** SSR's `max_concurrent_renders` derivation. Doorway `render::fetch_compute_budget` reads three pointer paths from `/api/v1/compute/dashboard`:
- `/computeMetrics/cpuTotalCores`
- `/constitutionalLimits/ceilingLimit/computeMaxCores`
- `/allocations/allocationBlocks/0/cpuCores`

Then `derive_capability` picks `min(non-zero values)`. Cross-stack contract test guards the JSON paths in `tests/render_capability_view.rs::storage_compute_view_satisfies_doorway_pointer_paths`.

**Anti-patterns:**

1. Hard-coded numeric defaults in feature deriver code (e.g. `DEFAULT_MAX_CONCURRENT = 8`). Allowed only as a last-resort fallback when the dashboard fetch fails entirely.
2. Per-feature env vars for compute limits. Always subscribe to the typed view; if you need an escape hatch, prefer extending `ELOHIM_OPERATOR_*` rather than inventing `ELOHIM_<FEATURE>_MAX_CORES`.
3. Reading `/proc/*` directly. Use `services::system_metrics` as the single foundation (`feedback_check_existing_compute_foundation`).

**Pending un-stub work:** network bandwidth probe (no foundation today), history series sampler (rolling buffers), storage breakdown beyond `cache_gb` (other pods own holochain/custodian/user-applications slices). Plan: `genesis/docs/superpowers/plans/2026-05-10-compute-metrics-un-stub.md`.
