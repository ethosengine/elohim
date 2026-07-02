# Verdict: the conductor OOM is an ANONYMOUS-HEAP leak — not page cache, not corpus, arc-independent

> ## ✅ RESOLVED — 2026-06-19 — attribution was RIGHT; cured by the jemalloc allocator swap
> This attribution stands: the OOM was an anonymous-heap leak in the holochain conductor child,
> arc-independent, not page cache / corpus / storage. What was still open here (the in-conductor mechanism +
> fix) is now closed: the anon was **glibc-malloc secondary-arena retention** (Rust/C allocations; Go heap
> flat ~52MB), CURED by swapping the conductor's global allocator glibc→jemalloc — flat past the old OOM
> cadence, DNA hash unchanged. NOT more RAM, NOT a SQLite cap, NOT arc — exactly as this verdict predicted.
> Truth: genesis/docs/content/elohim-protocol/history/2026-06-19-conductor-leak-jemalloc-cure-verdict.md · genesis/docs/content/elohim-protocol/history/2026-06-18-conductor-leak-rca-native-heap-reframe.md


**Status:** verdict CONFIRMED + attribution CONFIRMED (2026-06-17 ~00:52 UTC) · **Class:** runtime memory / self-heal · **Env:** alpha (observed live) · **Resolves:** the P-ARC §B "leak-vs-bounded-large" hard gate · **Plan:** `genesis/docs/superpowers/plans/2026-06-16-conductor-memory-attribution-instrument-plan.md`

> **TL;DR:** anonymous-heap leak (not page cache, not corpus, arc-independent) located in the **Holochain conductor child** (not the elohim-storage parent). The brake is a conductor/kitsune2 heap-leak hunt — NOT more RAM, NOT a SQLite cache cap, NOT arc sharding, and NOT in our storage code.

## The question
The fused `elohim-node` cgroup (conductor child + storage parent, one working-set number) hid whether the OOM climb is a heap leak, the SQLite page cache, or slab. The operator's directive: **"16GB shouldn't be the solution."** P-ARC's option-(iii) decision is gated on this discriminator.

## The verdict (decisive)
**The OOM climb is anonymous heap (`rss`), not page cache (`file`).** Measured at the cgroup/container level via cadvisor (`container_memory_rss` = cgroup `anon`; `container_memory_cache` = cgroup `file`) — the same split the deployed sampler reads from `memory.stat`, available in Prometheus *now* without waiting for the rollout or Loki (which was 502-storming).

| Pod | arc | anon (`rss`) | page cache | anon share |
|-----|-----|-------------|------------|------------|
| matthew | 1 (full) | 5.39 GB (mid-climb, post-restart) | 0.13 GB | **97.6%** |
| james | 0 (leecher) | 6.53 GB | 0.11 GB | **98.3%** |
| jessica | 0 (leecher) | 3.79 GB (mid-climb) | 0.24 GB | **94%** |

**The sawtooth lives entirely in anon.** 4-hour `container_memory_rss` range (step 5m):
- **matthew** (8Gi): anon `4.3 → 8.5 GB` monotonic over ~4h → OOM → new pod restarts at **1.57 GB** → climbs `1.6 → 5.4 GB` again. Page cache flat ~0.13 GB throughout.
- **jessica** (4Gi, arc=0): **6+ restart cycles** in 4h, each anon `~1.5 → ~4.2 GB` over ~40 min → OOM → restart. Identical shape, faster period (smaller ceiling).

## What this rules OUT
- **More RAM (8→16Gi): futile.** Anon grows without bound to whatever ceiling you set; a bigger ceiling just lengthens the OOM interval. Confirms the operator's instinct — the slope, not the intercept, is the problem.
- **Capping SQLite `cache_size`/`mmap_size`: futile.** The page cache is ~100–240 MB, flat. SQLite kernel page cache is NOT the driver. (The advisor's note that SQLite pread/pwrite cache would show in cgroup `file` was the right thing to check — and it's empty. The small, suppressed page cache is itself a symptom: under cgroup pressure from the growing anon, the kernel evicts clean file cache to make room.)
- **Corpus-held / arc: NOT the driver.** jessica (arc=0, holds ~no keyspace) leaks anon at the same shape; james (arc=0) has the *worst* anon (6.5 GB). This re-confirms the arc-falsification soak (`arc-shrink-ineffective-memory-soak.md`) at the memory-class level: the runaway is arc-independent. P-ARC option-(ii)/(iii) (arc sharding) would NOT bound this leak.

## The brake (verdict-chosen follow-on)
**Find and fix the anonymous-heap leak.** Growth is ~linear in wall-clock, independent of arc/corpus → a per-event/per-tick accumulation that's never freed (candidate classes: an unbounded in-memory cache/map, gossip/op-integration buffers, validation receipts, SSE/subscription accumulators, peer-tracking growth).

**Prime-suspect framing for the hunt** (to be settled by the per-process sampler): elohim-storage runs *identical* code on every node regardless of arc; if the leak is in the **storage parent**, it would leak the same on matthew and jessica — which matches the observed arc-independence. The **conductor child** leaking per-gossip/per-connection (which all nodes do) is the alternative. The fused cgroup can't separate them.

## ATTRIBUTION — CONFIRMED: the leak is the HOLOCHAIN CONDUCTOR CHILD (not storage)
The sampler (commit `fa9985436`) rolled to alpha as image `1.0.0-dev-ea494df1` and its per-process `RssAnon` split on **jessica** (arc=0 leecher) is decisive:

| time (UTC) | holochain conductor (pid 17) `rss_anon` | elohim-storage parent (pid 1) `rss_anon` |
|---|---|---|
| 00:49:21 | 3.365 GB · **391 threads** | 101.13 MB · 24 thr |
| 00:50:21 | 3.435 GB | 101.15 MB |
| 00:51:21 | 3.437 GB | 101.16 MB |
| 00:52:21 | **3.448 GB · 391 threads** | **101.16 MB · 23 thr** |

- **The conductor child carries ~97% of the anon and ALL the growth** (3.37→3.45 GB, monotonic). The **elohim-storage parent is dead-flat at ~101 MB** — it is NOT the leak.
- **Reconciliation holds** (the advisor's check): cgroup `anon` 3.55 GB ≈ conductor 3.45 + storage 0.10. cgroup `file` ~242–287 MB (small, ~flat → not page cache); `slab` ~12 MB; `swap` 0.
- **My prior hypothesis ("storage parent is the prime suspect, since it runs identically regardless of arc") was WRONG** — the instrument corrected the guess. The arc-independence is instead explained by the conductor leaking on work *every* node does regardless of arc (gossip/peer-resolution/receipts), not on held corpus.

### Live lead for the conductor heap-leak hunt
jessica's conductor spams `holochain::core::workflow::validation_receipt_workflow: send_validation_receipts could not find url for peer` (HolochainP2pError). Failed peer-URL resolution that retries/accumulates is an arc-independent anon-leak candidate; the **391-thread** count suggests per-connection/per-task accumulation in kitsune2/holochain_p2p. **Scope of the brake: the conductor (holochain 0.6 / kitsune2 0.3.2 / holochain_p2p), NOT elohim-storage.** We spawn but do not compile the conductor binary, so deeper attribution needs conductor-side tooling (see toolkit P2 note below), not jemalloc-on-storage.

### Toolkit implication (design-decision-toolkit plan P2)
The P2 heap-profiling instrument must target the **child holochain process**, not elohim-storage — `tikv-jemallocator` as elohim-storage's allocator would profile the wrong process. Options: a `/proc/<child_pid>/smaps_rollup`-by-mapping sampler, attach `heaptrack`/bpf to the child pid, or build/run the conductor with jemalloc prof. Recorded so P2 is scoped right.

## Evidence
- cadvisor instant + 4h range `container_memory_rss` / `container_memory_cache`, ns `elohim-alpha`, 2026-06-17 ~00:45 UTC (Prometheus uid `prometheus`).
- Loki was 502-storming at verdict time → the container-level split came from Prometheus cadvisor instead (per-process split needs the sampler + Loki).
- Lineage: `arc-shrink-ineffective-memory-soak.md` (arc falsified), `genesis/docs/content/elohim-protocol/history/2026-06-15-matthew-edge-resiliency-rca-fanout-synthesis.md` §4.2/§7 (leak-vs-bounded was "formally unconfirmed" — now confirmed: leak), `2026-06-14-dataplane-arc-plan.md` §B (gate resolved → leak branch).
