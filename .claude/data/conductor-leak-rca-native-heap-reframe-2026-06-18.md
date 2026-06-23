# Conductor leak — NATIVE-HEAP reframe (glibc-arena fill, NOT go-pion), 2026-06-18

Supersedes the go-pion framing of `conductor-leak-rca-empirical-reframe-2026-06-18.md`.
This pass did the one thing every prior pass skipped: **read the live smaps localizer
that shipped in `aa9f97f09` and has been flowing to Loki/Prometheus on alpha unread.**
The data overturns two load-bearing claims of the previous RCA.

> **DOC OF RECORD = `conductor-leak-rca-diverse-eyes-synthesis-2026-06-18.md`** (6-lens
> adversarial fan-out that confirmed this reframe and tightened it). Two refinements it makes
> to THIS doc, honor them: (a) **`near=<DNA>-shm` is SPATIAL adjacency only — zero causal
> attribution** (the parser carries the nearest preceding *file-backed* basename forward); do
> NOT read it as "SQLite caused it." (b) **SQLite is not a config sink** (no `cache_size`/
> `mmap_size` pragma; SQLCipher forces `mmap_size=0`) — so H2, if real, is codec/page *churn*,
> not a pragma-sized cache. The `8×24=192` arena-cap match is a **red herring** (count climbs
> *through* 192 to OOM). Layer-confirmed verdict stands; the specific call site is still open.

> **TL;DR:** the leak is a **native (Rust/C) heap leak in the holochain conductor**,
> accumulating across **glibc secondary malloc arenas** (each a 64 MB anon mmap that
> faults in resident pages as it fills) in the `0x77xx` mmap region **adjacent to the
> per-DNA SQLite databases**. It is **NOT** the go-pion Go heap (which is flat at 52 MB),
> **NOT** go-pion/tx5/SCTP, and **NOT** "stable-population resident fill" — the per-band
> histogram shows mappings **migrating 1-8m → 8-64m as arenas fill**, plus net-new arenas.
> This is *why the tx5 zombie-teardown fix did nothing*: the leak was never in go-pion.

---

## 1. The live data nobody had read (matthew, the amplifier anchor)

`scope="smaps_hist"` (Loki, `target=elohim_storage::memory_attribution`), 09:20 UTC:
```
0-64k=5465/68MB  64k-1m=289/28MB  1m-8m=81/471MB  8m-64m=192/6754MB  64m-256m=5/445MB  256m+=0/0MB
```
The histogram is bucketed **by per-mapping RESIDENT anon bytes** (`anon_size_histogram`,
`system_metrics.rs:620`; test `anon_size_histogram_bins_by_resident_anon`). So the leak is
**6.75 GB held in 192 mappings of ~35 MB resident each** — the `8m-64m` band. Every other
band is small/flat.

### 1a. Trajectory of the `8m-64m` band (3 h, one conductor lifetime, Prometheus)
| metric | start | end | rate |
|---|---|---|---|
| `anon_bucket_bytes{bucket="8m-64m"}` | 3.61 GB | 7.08 GB | **+1.16 GB/h, monotonic** |
| `anon_bucket_count{bucket="8m-64m"}` | 109 | 192 | **+~28/h, monotonic** |
| avg bytes/mapping | 33.1 MB | 36.9 MB | ~stable |

### 1b. The MIGRATION signal (the discriminator the scalar gauges hid)
Over the same lifetime: `1m-8m` **count 134 → 85 (declining)**, `8m-64m` count 109 → 192
(climbing), `64m-256m` pinned at 5, `256m+` empty. So mappings **graduate upward** —
a roughly-bounded population of 64 MB-reserved regions, each progressively faulting in
resident pages and **crossing the 8 MB boundary as it fills**, plus net-new regions when
the existing ones top out. Total `other` anon climbed 5.0 → 7.74 GB then **collapsed to
2.07 GB = a live OOM-restart (the flap), captured in the range query.**

### 1c. Localizer: WHERE the growing mappings live (`scope="smaps_growth"`)
The large growers are **16 MB-aligned addresses in the glibc mmap region** (`0x779820000000`,
`0x779a30000000`, `0x7799f8000000`, `0x7796a4000000`…), 64 MB-reserved, resident climbing:
```
0x779820000000 +59.2MB (sz64MB, near=uhC0kK8…-shm)       # filled to ~full 64MB arena
0x779a30000000 +39.4MB (sz48MB, near=uhC0kTYRgnhf…-shm)  # filling
0x7796a4000000 +10.3MB (sz10MB, near=holochain, NEW)     # a fresh arena born this sample
0x7796d0000000 + 2.2MB (sz40MB, near=holochain)
0xc000000000   + 0.2MB (sz52MB)                          # the Go heap — FLAT, NOT the leak
```
`near=` is the nearest preceding **file-backed** mapping (`system_metrics.rs:528-536`). The
growers' neighbors are consistently SQLite **`-shm`** files (the per-DNA WAL shared-memory
index — several distinct `uhC0k…` DNA hashes) and the **`holochain`** binary. The Go runtime
heap sits separately at its canonical base `0xc000000000` and is **flat at ~52 MB**
(+0.2–0.8 MB/sample = negligible vs the 1.16 GB/h leak).

## 2. What this overturns in the prior RCA
- **"NOT mapping proliferation — count is flat"** (empirical-reframe §1, §3) — *measurement
  artifact.* It read the **aggregate** `anon_mapping_count` (~5,978), which is swamped by
  5,465 tiny `0-64k` mappings; the +83 growth in the big-mapping band is ~1.4 % of the total
  and reads as "flat." The per-band histogram (shipped same commit, never read) shows the
  big band is exactly where the GBs accumulate.
- **"off-heap anon ⇒ go-pion's Go heap (the only non-Rust code)"** (the founding axiom of the
  whole tx5/go-pion arc, `…tx5-gopion-backpressure…` point 1; reopened-handoff §1) —
  *contradicted by the address space.* The Go heap is flat at 52 MB; the 7 GB is in the
  **glibc malloc mmap region (0x77xx) next to the SQLite DBs.** "Flat `[heap]`, growing
  `other`-anon" was interpreted as Go, but it is the textbook signature of a **multithreaded
  native (Rust/C) malloc leak**: glibc serves large/threaded allocations from **64 MB
  secondary arenas** that are anon mmaps classified `other` (never the brk `[heap]`).
- **The falsified tx5 zombie fix is now explained, not just observed:** the leak was never in
  the connection layer or go-pion at all → a per-connection teardown fix could not touch it.

## 3. Refined hypothesis set (to trace/confirm — NOT declared causes)
Every candidate must fit: ~35 MB resident arenas, **count + bytes both monotonic**,
**1-8m→8-64m migration**, glibc-mmap-region & near-SQLite-`-shm`/`holochain`, **Go heap flat**,
∝ traffic (anchors 6× leechers), survived the go-pion fix.
- **H1 — native Rust/holochain heap leak (TOP).** A per-DHT-op / per-receipt / per-gossip
  allocation retained in the conductor's Rust heap (or a C dep), spread across glibc arenas.
  Candidates incl. holochain_p2p send buffers (#5664's non-iroh `BytesMut::put_slice` 65 %),
  workflow/op caches, watch channels (`poll_updated` 96 MB in #5664), the validation-receipt
  re-drive retaining buffers. Identify the conductor's **global allocator first** (glibc vs
  jemalloc vs mimalloc) — it changes both the signature reading and the profiling tool.
- **H2 — SQLite page-cache / connection-pool growth.** The `-shm` adjacency is direct. Per-cell
  SQLCipher DBs (`sqlite-encrypted`), r2d2 pool size × `cache_size`/`mmap_size` pragmas,
  prepared-statement caches, or leaked connections scaling with DHT op volume. ~35 MB ≈ a
  large per-connection page cache.
- **H3 — wasmer linear memory (`wasmer_sys`), LOWER.** Per-DNA wasm instances/module cache.
  Argues against: wasm linear memories usually reserve huge (≥ GB) fixed VMAs → would land in
  `256m+` (which is **empty**). Fits only if wasmer caps reservations in the 8-64 MB range.
- **H4 — go-pion / tx5 / kitsune2, DEMOTED.** Go heap flat at 52 MB exonerates the Go side as
  the bulk leak. Residual only if a CGo **C-side** (not Go-heap) pion allocation leaks into
  glibc arenas — must be reconciled against the flat Go heap and the `-shm` neighbors.

## 4. Decisive cheap next measurements (discriminate H1–H4)
1. **Global allocator of the conductor** — grep `elohim/holochain-conductor` for
   `#[global_allocator]` / jemallocator / mimalloc / tikv-jemalloc. glibc ⇒ the 64 MB-arena
   reading holds; jemalloc ⇒ different arena model, use `jemalloc` prof.
2. **`MALLOC_ARENA_MAX=2` canary** (glibc only) — collapses arena spread. Does NOT fix a true
   leak (bytes still climb) but confirms the arena-fill mechanism and may slow the OOM cadence.
3. **One full `/proc/<conductor-pid>/smaps` dump** — confirm the 192 mappings are 64 MB-reserved
   arenas (heap-arena `Rss`/`Anonymous` with `rw-p`, no inode) vs file/wasm/Go.
4. **Per-process attribution** — `target=elohim_storage::memory_attribution` already separates
   conductor vs storage; confirm the anon is the `holochain` child (it is) and which threads.
5. **Then, the answer-finder:** a heap profiler aimed at the **native** side — jemalloc
   `prof`/`MALLOC_CONF`, or `heaptrack`/`bytehound` on the conductor, NOT Go pprof (the
   `pprof_debug.go` from the prior pass profiles the wrong runtime). If it IS glibc, switch the
   conductor to jemalloc-with-prof for one canary to name the Rust allocation site.

## 5. Evidence sources (all live / local)
- Prometheus (`prometheus`): `elohim_node_conductor_anon_bucket_{bytes,count}{bucket}`,
  `_smaps_anon_bytes{class}`, `_anon_mapping_count`, `_largest_anon_bytes`.
- Loki (`loki`): `{pod="elohim-matthew-alpha-0"} |= "smaps_growth"` / `"smaps_hist"`,
  `target=elohim_storage::memory_attribution`. ⚠ alpha Loki flaky (502 storms) — corroborate.
- Forks (local, populated): `elohim/{tx5,holochain-conductor,kitsune2}`; vendored pion at
  `/tmp/pion-vendor/vendor/github.com/pion/`. `GH_TOKEN` set (curl REST) for upstream.
- Parser: `elohim/elohim-storage/src/services/system_metrics.rs` (smaps bucketing/localizer).
- This reframe drove the diverse-eyes fan-out RCA (see the fan-out's synthesis doc).
