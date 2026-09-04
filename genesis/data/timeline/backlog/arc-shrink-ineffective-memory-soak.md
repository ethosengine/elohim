# Backlog: conductor `target_arc_factor=0` does NOT bound memory — instrument the real driver

> ## ✅ RESOLVED — 2026-06-19 — this falsification holds; the "real driver" was the glibc allocator, cured by jemalloc
> This record's finding stands: arc=0 does NOT bound the conductor's memory (arc-independent leak). The "real
> driver" it called for instrumenting was found via the smaps sampler: **glibc-malloc secondary-arena
> retention** in the conductor child (Rust/C allocations; Go heap flat ~52MB; tx5/go-pion exonerated). CURED
> by swapping the global allocator glibc→jemalloc — flat past the old OOM cadence, DNA hash unchanged. The
> "memory ∝ corpus × arc" theory remains falsified for this OOM.
> Truth: genesis/docs/content/elohim-protocol/history/2026-06-19-conductor-leak-jemalloc-cure-verdict.md · genesis/docs/content/elohim-protocol/history/2026-06-18-conductor-leak-rca-native-heap-reframe.md


**Status:** open · **Captured:** 2026-06-16 (shift `alpha-conductor-oom-arc-leecher`) · **Class:** self-heal / runtime memory · **Env:** alpha-cluster (observed live)

## Finding (decisive, soak-evidenced)

The conductor authority-arc lever `network.target_arc_factor=0` (leecher) is **honored** by the deployed kitsune2/holochain_p2p (boot logs show `HolochainP2pConfig { target_arc_factor: 0 }`) but **does NOT bound the node's working-set memory**. Controlled proof: **jessica**, a confirmed leecher (`arc=0` across 5+ boots 11:14–13:53Z on 2026-06-16), still **sawtooths 1.3 GB → 4.29 GB (her 4Gi OOM ceiling) → restart every ~40 min**. A full-arc node (matthew) sawtooths to 8 GB ~every 3h; the *shape is identical*, only the ceiling differs (= the cgroup limit, not the arc).

This **falsifies** the arc-shrink memory strategy in `genesis/docs/superpowers/specs/2026-06-13-conductor-authority-arc-memory-scaling.md` and the per-node leecher rationale in `deployments.json` `$arcFactorComment` (jessica) — `arc=0` is the wrong lever for OOM. The 2026-06-15 RCA fanout theory 1 ("memory ∝ corpus × arc") is not supported by the leecher soak.

## The real lever (next work — RCA §4.2, the highest-value cheapest add)

The OOM driver is **arc-independent** and hidden by the **fused `elohim-node` cgroup** (conductor child + elohim-storage parent in one container, one `container_memory_working_set_bytes`). Cannot attribute the climb without splitting it. Implement the §4 instrumentation in `elohim/elohim-storage/src/conductor/process_manager.rs`:
- per-process RSS + `RssAnon` vs `RssFile` split (read `/proc/<child_pid>/status` every 60s) → `elohim_node_child_rss_bytes{proc="holochain"|"elohim-storage"}`, anon-vs-file.
- boot-time log of effective `db_max_readers` + `num_cpus` + cgroup cpu quota.

This settles leak (anon-heap, growing) vs bounded (file-mmap SQLite page cache) and tells us whether the climb is the conductor or the storage parent — which is what actually sizes matthew's RAM honestly and finds the real fix.

## Open follow-ups
- **Revert jessica + james to default `arc=1`?** Leecher gives no memory benefit and slightly reduces DHT coverage participation — but reverting churns a disruptive image-roll deploy. Low priority; operator call. (Coverage is currently safe: 2 leechers, 12 full holders.)
- matthew's 503 (doorway OOM-stall co-location park, 100 restarts) is unchanged by arc — needs the operator levers (RAM 8→16 / podAntiAffinity / route alpha→adam), staged separately.

## 2026-09-04 update — local mesh runs STOCK holochain 0.7.0, not the jemalloc fork

The fleet cure above (glibc→jemalloc) lives in the fleet's conductor fork/image; the **local
household mesh does not carry it** — it runs stock holochain 0.7.0 conductors. Measured overnight
2026-09-04 (rung-5 c3 shift, journal gitignored): the three mesh conductors grew from ~2.0 GB RSS
at 01:24Z to ~4.3 GB each by 08:10Z (12.6 GB total) across ~5 a2o runs + 3 rung-5 ceremonies —
sawtooth-free, monotonic growth over the session, not the fleet's arc-independent OOM-cycle shape
this record falsified, but growth all the same. This left `elohim-storage` unbuildable under the
RAM guard for the rest of the night; cured only for the session by
`--config profile.dev.package.elohim-storage.debug=0` (debug-info stripping, not a memory fix).
Resource finding for the conductor-arc habit / `conductor-image` line: the fork's jemalloc cure
has no local-mesh equivalent, so local-mesh sessions accumulate conductor RSS the fleet no longer
does — worth either backporting the fork to the local dev conductor slot or budgeting mesh-session
length/restarts against it. No RCA attempted this shift; capturing the measurement only.

## Links
- Soak evidence + judgment: shift journal `.claude/shifts/2026-06-16T0357-alpha-conductor-oom-arc-leecher.journal.md` (iteration 2)
- RCA: `genesis/docs/content/elohim-protocol/history/2026-06-15-matthew-edge-resiliency-rca-fanout-synthesis.md` (§4 instrumentation, §5 staged experiments)
- Falsified spec: `genesis/docs/superpowers/specs/2026-06-13-conductor-authority-arc-memory-scaling.md`
