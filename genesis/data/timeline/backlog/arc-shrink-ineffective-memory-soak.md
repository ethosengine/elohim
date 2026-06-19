# Backlog: conductor `target_arc_factor=0` does NOT bound memory — instrument the real driver

> ## ✅ RESOLVED — 2026-06-19 — this falsification holds; the "real driver" was the glibc allocator, cured by jemalloc
> This record's finding stands: arc=0 does NOT bound the conductor's memory (arc-independent leak). The "real
> driver" it called for instrumenting was found via the smaps sampler: **glibc-malloc secondary-arena
> retention** in the conductor child (Rust/C allocations; Go heap flat ~52MB; tx5/go-pion exonerated). CURED
> by swapping the global allocator glibc→jemalloc — flat past the old OOM cadence, DNA hash unchanged. The
> "memory ∝ corpus × arc" theory remains falsified for this OOM.
> Truth: .claude/data/conductor-leak-jemalloc-cure-verdict-2026-06-19.md · conductor-leak-rca-native-heap-reframe-2026-06-18.md


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

## Links
- Soak evidence + judgment: shift journal `.claude/shifts/2026-06-16T0357-alpha-conductor-oom-arc-leecher.journal.md` (iteration 2)
- RCA: `.claude/data/matthew-edge-resiliency-rca-fanout-2026-06-15.md` (§4 instrumentation, §5 staged experiments)
- Falsified spec: `genesis/docs/superpowers/specs/2026-06-13-conductor-authority-arc-memory-scaling.md`
