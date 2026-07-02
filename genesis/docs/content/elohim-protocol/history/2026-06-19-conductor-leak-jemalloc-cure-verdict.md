---
title: "Conductor leak — CURED by the jemalloc allocator swap (verdict, 2026-06-19)"
id: conductor-leak-jemalloc-cure-verdict
type: history-gotcha
status: noted
tier: history
created: 2026-06-19
topic: [conductor-leak, oom, jemalloc, glibc-arena, cure, alpha]
---

# Conductor leak — CURED by the jemalloc allocator swap (verdict, 2026-06-19)

*Live-telemetry verdict closing the RCA in `2026-06-18-conductor-leak-rca-diverse-eyes-synthesis.md`
+ `2026-06-18-conductor-leak-rca-native-heap-reframe.md` + `2026-06-18-conductor-leak-canary-runbook.md`. The leak is the native glibc-malloc anon heap
leak in the embedded `holochain` child (layer-confirmed ~88%; Layer-C call site was still open).*

## TL;DR
**The OOM leak is CURED on alpha right now — by the jemalloc allocator that rode in on the
profiler build, exactly the runbook's predicted "allocator IS the fix" outcome.** The `b8481f090`
"TEMP fleet jemalloc-prof conductor" (Canary B) went fleet-wide ~2026-06-18 and the monotonic-to-OOM
anon climb **stopped dead**. The cure is attributable to the allocator (glibc→jemalloc) — the ONLY
binary difference between the leaking and flat images — because jemalloc's aggressive decay/`munmap`
returns the freed-but-pinned memory glibc was hoarding in chained 64 MB secondary sub-heaps. This
also settles the synthesis's open Layer-B question: the bytes were **freed-but-glibc-pinned
retention (reclaimable), not a never-freed true leak** (a true never-freed leak would climb under
jemalloc too; it went flat).

## The decisive evidence — image-attributed cadvisor working_set (the number the OOM-killer acts on)
`container_memory_working_set_bytes{container="elohim-node"}`, carrying the running `image` label,
on the two former worst amplifiers:

| image (allocator) | matthew / james working_set behavior |
|---|---|
| `1.0.0-dev-b33ff524`, `-2af2607e`, `-d433d085` (**glibc**) | monotonic climb to **8.0–8.5 GB → OOM**, sawtooth every ~5 h |
| **`1.0.0-dev-b8481f09`** (**jemalloc-prof**) | **flat ~2.7 GB, oscillating, 0 restarts, ~10.5 h** |

App metric agrees: `elohim_node_conductor_smaps_anon_bytes{class="other"}` flat ~2.2 GB on
matthew/james/jessica over the same window (was 2.8→6.95 GB monotonic pre-cutover). cadvisor
(kernel cgroup) + smaps (app) concur → not a metric artifact.

Restarts corroborate: matthew/james/jessica `kube_pod_container_status_restarts_total` = **0** across
the whole ~10.5 h jemalloc window (were OOM-restarting ~1–2×/11 h under glibc). 10.5 h ≈ 2 old
OOM-cycles, so this is a real cure, not "too early."

## Deploy provenance
- Commit `b8481f090` (elohim monorepo), authored **2026-06-18 15:38 UTC**, body: *"Repoints the
  storage build at the profiling conductor elohim-edgenode-prof:e87a680 and adds `_RJEM_MALLOC_CONF`
  to the elohim-node container … auto-dumps jeprof heap profiles to the data-dir PVC (~1/GiB). … ⚠
  TEMP — REVERT after the leak site is named: restore CONDUCTOR_SOURCE_IMAGE to elohim-edgenode:latest
  and drop the `_RJEM_MALLOC_CONF` env."* Changed only `elohim/elohim-storage/Dockerfile` +
  `…/_edgenode-consolidated.template.yaml`.
- All 14 `elohim-node` containers currently run `harbor.ethosengine.com/ethosengine/elohim-storage:1.0.0-dev-b8481f09`
  (`kube_pod_container_info`). che-devworkspaces submodule pins `e87a680` (the prof-default pipeline).
- Profiling is configured ACTIVE (`_RJEM_MALLOC_CONF` set) → jeprof `.heap` dumps SHOULD be
  accumulating on each PVC. **UNVERIFIED from the dev container** (no kubectl/exec) — operator must
  confirm `.heap` files exist (and contain C frames — the unprefixed-malloc interposition trap) before
  assuming the site is nameable.

## Scope / caveats (honest bounds)
1. **Cured everywhere a container lives long enough to show the curve.** Confirmed flat under
   b8481f09: matthew (8 GB limit), james (8 GB), jessica (4 GB), terrance (6 GB, ~2.5 GB flat),
   gertrude (3 GB, current container flat ~2.1 GB for ~7.7 h). The cure is NOT anchor-only — it holds
   on 3 GB-limited shem pods too.
2. **eve is the one residual churner — but it is NOT the anon leak.** eve (3 GB limit) restarts every
   ~1.5–2 h at **~2.0–2.27 GB working_set, well below its 3 GB ceiling** → not a cgroup OOM. Likely a
   liveness/crash failure or node-level eviction on the over-subscribed shem node (shem carries
   8+6+9×3 = 41 GB of limits). **Investigate separately; do not read as "leak uncured."**
3. **Flat band ≈ 400 MB wide over 10.5 h** kills the fast leak (was 1.16 GB/h) decisively, but cannot
   rule out a ~10–20× slower residual. Cheap settle: a 24–48 h slope check on one anchor.
4. **The cure may have partially defeated the site-namer.** The runbook's `jeprof --base T0 T1` diff
   works best on a *growing* target; jemalloc flattened it, so cumulative-growth attribution is now
   weak. Naming the Layer-C site may need comparing churn against the historical glibc behavior — and
   the cure does NOT depend on naming it.

## What to actually do (reframes the stale A/B merge question)
The pasted "A (deploy elohim-edgenode:latest tx5-fix) vs B (keep profiler)" framing is SUPERSEDED:
- **A is a trap.** `:latest` is the **glibc** conductor (+ the tx5 zombie-fix #194/#199 already proven
  not to touch this leak — deployed fleet-wide, leak persisted). Reverting to `:latest` re-introduces
  the OOM. The "revert once named" instruction in b8481f090's body assumed reverting to glibc; that's
  now wrong because the temp build turned out to be the fix.
- **B (keep jemalloc) is correct as a bridge** — it's what's keeping the fleet alive — but it's a
  TEMP debug build with profiling overhead; don't leave it forever.
- **The real deliverable: build a jemalloc-PRODUCTION `:latest`** — jemalloc as the allocator,
  profiling OFF (drop `_RJEM_MALLOC_CONF` / `prof:active`; keep the `tikv-jemallocator` global
  allocator, no `jemalloc-prof` feature). That captures the allocator cure as the production fix and
  replaces BOTH the leaking glibc `:latest` and the temp profiler. **Ship this independent of whether
  site-naming succeeds.**
- **Optional, while the instrument is live:** pull two jeprof dumps from one anchor's PVC and diff to
  name the Layer-C site for the record (operator-run; cheap insurance), THEN stand the profiler down.
- **Separately:** investigate eve's sub-limit restart-churn (node pressure / liveness), distinct from
  the anon leak.

## Evidence sources (all live, queried this session 2026-06-19)
- Prometheus (`prometheus`): `container_memory_working_set_bytes` (image-labelled),
  `elohim_node_conductor_smaps_anon_bytes{class="other"}`, `kube_pod_container_status_restarts_total`,
  `kube_pod_container_resource_limits{resource="memory"}`, `kube_pod_container_info`.
- Git: `b8481f090` body + stat; che-devworkspaces @ `e87a680`.
- Prior RCA chain: `2026-06-18-conductor-leak-rca-diverse-eyes-synthesis.md`,
  `2026-06-18-conductor-leak-rca-native-heap-reframe.md`, `2026-06-18-conductor-leak-canary-runbook.md`,
  `2026-06-17-conductor-leak-tx5-zombie-fix-deploy-recipe.md`.
</content>
</invoke>
