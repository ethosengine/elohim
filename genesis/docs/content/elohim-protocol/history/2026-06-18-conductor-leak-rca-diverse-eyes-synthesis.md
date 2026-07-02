---
title: "Conductor leak — diverse-eyes RCA synthesis (2026-06-18)"
id: conductor-leak-rca-diverse-eyes-synthesis
type: history-gotcha
status: noted
tier: history
created: 2026-06-18
topic: [conductor-leak, rca, glibc-arena, native-heap, allocator, alpha]
---

# Conductor leak — diverse-eyes RCA synthesis (2026-06-18)

*Lead-investigator synthesis of six independent, adversarially-cross-examined lenses (allocator-forensics, sqlite-persistence, holochain-rust-heap, wasmer-linear-memory, go-pion-steelman, first-principles-wildcard), disciplined by live alpha telemetry queried this session. Weighted by evidence-fit, not averaged. Companion to the analysis doc `2026-06-18-conductor-leak-rca-native-heap-reframe.md`.*

## TL;DR
The leak is a **native, live-retained heap leak in the embedded `holochain` child process, served by stock glibc `malloc` and accumulating in glibc 64 MB secondary arenas (the `0x77xx` anon region)** — traffic-scaled, monotonic-to-OOM, **a true leak, not benign allocator fragmentation**. This is CONFIRMED at the *layer* level (~88). What is NOT yet known is the **specific allocating call site** (Layer C) — `near=<DNA>-shm` is spatial adjacency only and names nothing. The prior tx5-zombie / go-pion theory is **definitively exonerated** (Go heap flat at 52 MB; the #194/#199 fix was deployed fleet-wide and the leak persisted). The single decisive next measurement is a **native heap profiler with backtraces** (jemalloc-`prof` build, or `heaptrack`/`bytehound`) on one anchor for one lifetime — the *only* instrument that names the site; it requires a one-feature rebuild (Pyroscope is unavailable on this Grafana). `MALLOC_ARENA_MAX=2` is a cheap interim probe/mitigation but, given monotonicity, will slow not cure.

## Convergence & divergence

**Where all six lenses converged (the strongest signal — treat as established):**
1. **No custom allocator.** Verified this session: `grep` for `global_allocator|jemalloc|mimalloc|snmalloc|tikv-jemalloc` across `elohim/holochain-conductor/` source + `Cargo.lock` returns **zero hits**. Default Rust-on-Linux = system glibc `ptmalloc2`. (`crates/holochain/src/bin/holochain/main.rs:64` installs only the rustls crypto provider, then `block_forever_on`.)
2. **The Go heap is not the leak.** `0xc000000000` (Go's canonical c-archive arena base) is flat at ~52 MB across every sample. The 7 GB lives in a disjoint `0x77xx` region. This is the clean refutation of the prior "off-heap anon must be go-pion" inference.
3. **The growers are 64 MB-aligned, 64 MB-reserved anon mappings in the glibc mmap zone, neighboring SQLite `-shm` and the `holochain` binary** — the textbook glibc non-main-arena (`HEAP_MAX_SIZE`) signature.
4. **`near=` is spatial, not causal.** All lenses that examined the parser agree: `system_metrics.rs` sets a VMA's `neighbor` to the basename of the *nearest preceding file-backed mapping in address order* (the update happens only inside the `MappingClass::File` branch, then is carried forward to every subsequent anon VMA). So `near=peer-meta-...-shm` is layout coincidence and carries **zero** attribution.
5. **The leak is in the Rust/C heap (Layer A/B), not Go, not wasm, not SQLite-config, not tx5 teardown.**

**Where lenses diverged (and how the evidence breaks the tie):**
- **Mechanism: benign arena fragmentation vs true leak.** allocator-forensics and go-pion-steelman initially leaned "arena over-reservation / fragmentation, fixable by `MALLOC_ARENA_MAX`." Their own adversaries refuted this: **monotonic-to-OOM with no plateau is unreclaimable true-leak behavior; bounded arena reservation plateaus.** Verified live below. → **true leak.**
- **The `8×24=192` arena-cap coincidence.** Multiple lenses were seduced by "8m-64m count climbs to 192 = the 8×ncpu cap." This is a **red herring**: the count climbs *monotonically through* ~190 and resets only at the OOM cliff (matthew: 88→189→OOM-reset to 50). It never *rests* at 192. The match is coincidental.
- **The site (Layer C).** Genuinely unresolved. sqlite-persistence and holochain-rust-heap both *self-refuted* their own named candidates (SQLite-config bounded; the obvious Rust accumulators are bounded/self-evicting). No lens could name the site from source+logs alone.

## Live evidence verified this session (2026-06-18, elohim-matthew-alpha-0, ~4 h window)
- `anon_bucket_bytes{bucket="8m-64m"}`: **2.80 GB → 6.95 GB monotonic** (+~1.16 GB/h), then **OOM-reset to 1.27 GB** and re-climbing. (Discriminator 1.)
- `anon_bucket_count{bucket="8m-64m"}`: **88 → 189 monotonic**, then reset to 50. Never plateaus. (Kill-shot for fragmentation.)
- `anon_bucket_count{bucket="1m-8m"}`: **149 → 83 declining** as 8m-64m climbs (migration confirmed). (Discriminator 2.)
- `anon_bucket_count{bucket="64m-256m"}`: **pinned at exactly 5** the whole window; `256m+` empty. (Static band — the wasmer reservations, not the leak.)
- **Chaining proof:** at one instant matthew big-heap total = 83 (1-8m) + 189 (8m-64m) + 5 (64m-256m) = **277 > the 192 arena cap** → ~85 of these are **chained sub-heaps within arenas**, not one-arena-per-thread. This is the direct evidence (no small-node premise needed) that bytes are *demand-driven retention*, not thread-count reservation.
- **Fleet topology (verified `kube_pod_info`):** ALL alpha edge pods run on the two **24-core** nodes (`ethosengine`: matthew/james/jessica; `shem`: the rest). The cluster has 2/4/8-core nodes too, but **no alpha conductor is on them**. → the tempting "2-core leecher with 190 regions ⇒ must be chained" argument is **unsupported and is NOT used**; the chaining proof above stands on the aggregate instead.
- **Traffic scaling (Discriminator 4):** instant fleet counts — anchors james 156 / adam 103 vs leechers terrance 33 / caleb 35 / nancy 43. Anchors ~6× leecher leak rate.
- **Pyroscope datasource: NOT FOUND** (`list_pyroscope_profile_types` → error). The zero-rebuild attribution path does not exist here.
- **Allocator/spawn seam:** `process_manager.rs:64` `Command::new(&self.conductor_binary)` with `.env("HOLOCHAIN_DATA_DIR", …)` at `:68` and **no `.env_clear()`** → a container env var reaches the child. (Confirms the `MALLOC_*` probe is deployable with no rebuild.)

## Ranked candidates

The six lenses are not six competing root causes. **Three are exonerations all lenses agree on; the real competition is a single unresolved question — the site.** Ranked as layers with honest per-layer confidence.

### Layer A — CONFIRMED (confidence 90): native glibc-malloc anon leak in the holochain child, traffic-scaled; NOT Go / wasm / SQLite-config / tx5
- **Mechanism:** stock glibc `ptmalloc2` (no `#[global_allocator]`, verified) serves the conductor's Rust+C allocations from up to `8×ncpu` 64 MB-reserved non-main arenas (`HEAP_MAX_SIZE`), each an unnamed `rw-p` anon mmap classified `OtherAnon` by `system_metrics.rs`.
- **Fit:** (1) FITS — bytes+count monotonic to OOM, live-verified. (2) FITS — 64 MB-aligned, 0x77xx, migration confirmed. (3) FITS — Go heap flat (the keystone exoneration of go-pion). (4) FITS — anchors ~6× leechers, live fleet snapshot. (5) FITS — tx5 teardown fix is independent of malloc-arena retention.
- **Falsified by:** a full smaps census showing the growers are NOT `rw-p`/no-inode anon (e.g. file-backed, or at the Go `0xc0` base, or ≥1 GiB virtual). None of these hold in the live data.

### Layer B — CONFIRMED (confidence 85): the bytes are LIVE-RETAINED Rust/C allocations (a true leak), not benign arena fragmentation
- **Mechanism:** allocations that are never freed (or freed-but-pinned in chained sub-heaps glibc never `munmap`s), accumulating in the Layer-A arenas.
- **Evidence:** monotonic-to-OOM with **no plateau** at any arena ceiling; count climbs *through* 192; 277 big-heaps > 192 cap (chained sub-heaps). Bounded fragmentation would asymptote; this does not. Swap is off ⇒ unreclaimable.
- **Falsified by:** `MALLOC_ARENA_MAX=2` flattening the byte slope (would prove the GBs were arena-spread fragmentation). Predicted NOT to flatten.

### Layer C — HYPOTHESIS (unresolved — do NOT pick a winner): the specific allocating call site
Each candidate manifests *identically* as 64 MB glibc arenas next to `-shm` — only a profiler stack discriminates them. Tied to what the profiler would show:
1. **kitsune2 op-store / gossip / fetch retention** (HYPOTHESIS). `kitsune2/crates/gossip/src/config.rs:143` `max_gossip_op_bytes = 100 MB`/round; op buffers (`Bytes`/`Vec`) retained in round state, fetch queue, or an undrained channel. *Profiler signature:* top retained stacks in `kitsune2_gossip` / `core_fetch` / op-store. Note: the obvious accumulators (`core_fetch.rs:336-345,383-385,497-499` self-evict; `gossip/src/timeout.rs:46-110` reaps) were read and are **bounded**, so the site, if here, is one not yet read.
2. **SQLCipher per-page codec churn across many DNAs** (HYPOTHESIS). Decrypted-page buffers `malloc`/`free`'d per page touched; pool up to `num_read_threads()*2+1 = 25`/DNA (`holochain_sqlite/src/db/pool.rs:77,134-138`). *Profiler signature:* `sqlcipher_codec`/`sqlite3Malloc`/page-cache. Note: SQLite *config* is bounded (no `cache_size`/`mmap_size` pragma — verified; SQLCipher forces `mmap_size=0`), so this is churn/retention, not a config sink.
3. **holochain Rust-side cache / map growth** (HYPOTHESIS). e.g. an unbounded cache, or #5813's `CONTEXT_MAP` leak-on-panic (`real_ribosome.rs:158,818-843`) — but the latter is **falsified as bulk**: zero host-fn panics in 6 h of logs.
4. **pion C-side allocation** (LOW — near-falsified). pion is pure Go (zero `import "C"` across 1189 vendored files); the only C is a non-allocating callback shim. A Go-side leak would inflate `0xc0` (flat). *Profiler signature:* none expected.

Demoted/falsified candidates (do not re-chase): tx5 dead-peer zombies (deployed + failed); go-pion Go heap / `main.go:189` (flat 0xc0); wasm linear memory (live: max grower virtual = 64 MB, no ≥1 GiB VMA — a wasm Static memory is ≥1 GiB virtual, Dynamic tracks resident; neither matches a fixed-64MB-reserve-then-fill); iroh `BytesMut::put_slice`/`poll_updated`/`magicsock` from upstream #5664 (those are iroh/quinn symbols; `poll_updated` has zero occurrences in the fork — different transport).

## Was go-pion exonerated?
**Yes — definitively, on three independent grounds:**
1. **Flat Go heap.** `0xc000000000` (the c-archive Go arena, `build.rs:46-49` static + `:311-314` `-buildmode=c-archive`) is flat at ~52 MB. Everything go-pion/pion/SCTP/cgo.Handle allocates lives there. Flat 0xc0 ⇒ go-pion not leaking the bulk.
2. **Pure Go.** Zero `import "C"`, zero `.c`/`.h` across all 1189 vendored pion files; the `main.go:189` `// TODO!!! MEMORY LEAK` is a dead branch in steady state (fires only before `OnEvent` registration) and would leak Go-heap handles anyway, not 0x77xx anon.
3. **The experiment of record.** The tx5 zombie/dead-peer teardown fix (#194/#199) was BUILT, DEPLOYED FLEET-WIDE, BINARY-VERIFIED (sha256 36ddf7ab), and the leak **persisted unchanged**. A Go-side PeerConnection-teardown fix cannot touch glibc-arena retention in the Rust/C heap.

The prior investigation's fatal error was reasoning *backwards*: "off-heap anon must be Go because Go is the only non-Rust code." But **Rust's default allocator IS glibc malloc** — off-heap anon at `0x77xx` is the *norm* for a Rust+C process; Go is the oddball with a private `0xc0` arena. Off-heap anon at 0x77xx points *toward* Rust/C, *away* from Go. The single measurement that would have falsified the old theory immediately — profiling the Go heap / checking the address — was never run.

## Decisive measurement plan (cheapest-first)

1. **[free, already done] Confirm monotonic-no-plateau + chaining.** `elohim_node_conductor_anon_bucket_{bytes,count}{bucket=~"...m-...m"}` range over a full pre-OOM lifetime. *Discriminates:* true-leak (climbs through any ceiling) vs fragmentation (plateaus). **Result: true-leak confirmed** (matthew 88→189→OOM; 277 > 192 cap).

2. **[cheap, no rebuild] `MALLOC_ARENA_MAX=2` (+ optional `MALLOC_TRIM_THRESHOLD_=131072`) on ONE anchor.** Inject child-only via `process_manager.rs:64` `.env(...)` (already does `.env("HOLOCHAIN_DATA_DIR",…)`, no `env_clear`) OR set on the `elohim-node` container env (inherited). Watch `elohim_node_conductor_anon_bucket_bytes{bucket="8m-64m"}` ~2-3 h vs an unchanged anchor.
   *Discriminates:* fragmentation-spread vs live-leak. *Predicted (given Layer B):* bytes still climb (now in ≤2 arenas) — confirms not fragmentation. Run it anyway: if it DID flatten it would be a free fix. **It does not name the site.**

3. **[needs ONE rebuild — the only site-namer] Native heap profiler with backtraces on one anchor for one lifetime.** Either relink the conductor with `tikv-jemallocator` + `_RJEM_MALLOC_CONF=prof:true,prof_active:true`, dump at T0 and T1 (~1-2 GB of climb), `jeprof --text --cum --base=T0.heap <binary> T1.heap`; or attach `heaptrack`/`bytehound` to the child.
   *Discriminates ALL Layer-C candidates at once:* stacks in `sqlcipher_codec`/`sqlite3Malloc` → SQLCipher; `kitsune2_gossip`/`core_fetch`/op-store → kitsune2; a Rust `Vec`/`HashMap`/cache path → holochain-rust; (pion C-side → none expected).
   **Cost honesty:** this REQUIRES a rebuild (add a `jemalloc` feature — there is none in the production `--features sqlite-encrypted,wasmer_sys,backend-go-pion` build). One lens wrongly claimed "your org already built tx5 with jemalloc / no rebuild" — that misread the #5664 thread (zippy built the *whole iroh-capable conductor*, a different binary, which showed VmSize 1.15 TB / RSS 462 MB — the opposite shape). Pyroscope is **unavailable** (datasource not found), so go-pprof-style zero-rebuild attribution is off the table.

4. **[corroborator, free] Full `/proc/<conductor-pid>/smaps` census by VIRTUAL Size** (the parser captures `AnonVma.size_bytes`, `system_metrics.rs:483`). Confirms growers are `rw-p`/no-inode/≤64 MB-virtual (refutes wasm's ≥1 GiB VMA at the same time) and that ≥1 GiB-virtual VMAs are a small FLAT set.

## Candidate fixes
**We cannot name a code fix until the profiler (step 3) names the site — stating that honestly is the point; the prior investigation shipped a fix for a mis-named cause and it failed fleet-wide.** What we can commit now:

- **Interim mitigation (no rebuild, reversible):** ship `MALLOC_ARENA_MAX=2` to the `elohim-node` container env. *Expected effect:* slows the OOM cadence by concentrating arenas; **does not cure** (Layer B is a true leak). Validate by the same flatten signal — expect partial slope reduction, not flattening.
- **The actual fix** will be a bounded-collection / drop-path patch at whatever Layer-C site the profiler names (most-likely-ranked: kitsune2 op-store/gossip retention, then SQLCipher codec churn). It will be a `holochain`-binary change, so it deploys via the **existing canary harness** in `2026-06-17-conductor-leak-tx5-zombie-fix-deploy-recipe.md`: build the patched binary (`cargo build --release --bin holochain --no-default-features --features sqlite-encrypted,wasmer_sys,backend-go-pion`, `RUSTFLAGS=""`, `RUSTC_WRAPPER=""`, Go 1.24), embed it in the **elohim-storage** image via `ARG CONDUCTOR_SOURCE_IMAGE` (the conductor is an embedded child, not the edgenode image — landed dev `b33ff524a`), deploy to ONE non-genesis leecher's `elohim-node`, and watch the recipe's **cure signal: `elohim_node_conductor_smaps_anon_bytes{class="other"}` FLATTENS** over a multi-hour window while the pod stays in-mesh (kitsune2 0.3.2↔0.3.0-dev.3 wire-compat check), genesis pair (matthew/adam) rolled LAST.
- **Validation gate is identical for any candidate:** flatten-or-not on `smaps_anon{class=other}` on the canary leecher. That harness already exists and is the right cure-proof regardless of which site wins.

## Open questions / what we still cannot conclude from source alone
1. **The allocating call site (Layer C).** Unresolved. `near=` is spatial-only; every named source accumulator we could read is bounded; the site is either a path not yet read or a churn pattern (SQLCipher codec) with no single owning structure. **Only the profiler stack settles it.**
2. **Leak vs pinned-fragmentation-of-churn, at the fine grain.** Monotonic-no-plateau strongly implies a true leak, but a profiler showing *orphaned* (no live owner) backtraces vs *live-owned* growing collections is what distinguishes "freed-but-pinned in chained sub-heaps" from "genuinely never freed." The fix differs (allocator tuning vs code patch) — though Layer B already rules out *benign* fragmentation.
3. **Whether the floor (leecher ~0.2 GB/h) and the anchor slope are one mechanism at two volumes or two mechanisms.** Traffic-scaling fits one mechanism; not yet proven. The profiler on both an anchor and a quiet leecher would confirm.
4. **Conductor-vs-parent attribution of the network churn.** All partition/retry log lines carry `target=elohim_storage::*` (the parent's libp2p); the conductor shares the same unreachable-peer reality but we have no conductor-side network log proving *it* is the churn driver. Inference, not logged fact.

---
*Files of record (verified this session): `elohim/elohim-storage/src/conductor/process_manager.rs:64,68` (spawn seam, env inheritance, no env_clear); `elohim/elohim-storage/src/services/system_metrics.rs` (parser; `near=` is nearest-preceding-file-VMA, `AnonVma.size_bytes:483`); `elohim/holochain-conductor/crates/holochain/src/bin/holochain/main.rs:64` (no allocator); `elohim/holochain-conductor/crates/holochain_sqlite/src/db/pool.rs:77,134-138` (bounded pool, no cache/mmap pragma); `elohim/kitsune2/crates/gossip/src/config.rs:143` (100 MB op-data candidate); `elohim/kitsune2/crates/core/src/factories/core_fetch.rs:336-345,383-385,497-499` (bounded, self-evicting); `elohim/holochain-conductor/crates/holochain/src/core/ribosome/real_ribosome.rs:158,818-843` (#5813, falsified as bulk). Live: Prometheus `elohim_node_conductor_anon_bucket_*` + `kube_pod_info` + `machine_cpu_cores` (all alpha pods on 24-core ethosengine/shem); Pyroscope datasource absent. Deploy harness: `2026-06-17-conductor-leak-tx5-zombie-fix-deploy-recipe.md`.*
