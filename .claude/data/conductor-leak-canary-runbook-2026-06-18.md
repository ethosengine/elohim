# Conductor leak — canary runbook (2026-06-18)

Two canaries to close the RCA from `conductor-leak-rca-diverse-eyes-synthesis-2026-06-18.md`:
the leak is a **native (Rust/C) glibc-malloc heap leak in the embedded `holochain` child**
(layer-confirmed ~88%); the **specific call site (Layer C) is still open** and only a native
heap profiler names it. Canary A is a no-rebuild probe/interim mitigation; Canary B is the
site-namer. Both validate against the existing cure signal in
`conductor-leak-deploy-recipe-2026-06-17.md`.

Operator-owned actions (cluster + build host) are flagged 🛠 — I cannot run `kubectl` or the
Go+nix build from the dev container.

> **Sequence + the optimistic read (do A first).** Both canaries are also candidate *fixes*, not
> just diagnostics. **Run Canary A first** — it's free and no-rebuild, and a hard RSS drop is
> genuinely plausible: glibc's per-arena non-return *also* climbs monotonically under churn (a
> rising high-water mark), and the 277 > 192 chained sub-heaps prove **pinning**, not that every
> byte is live (a small live alloc can pin a mostly-free 64 MB sub-heap glibc never returns). If A
> drops RSS hard, the cure is **allocator config** — you may never need the profiler build. Only
> if A *persists* do you build Canary B; and even B can win two ways — jemalloc returns memory to
> the OS far more aggressively than glibc, so it may **flatten on its own** (allocator IS the fix)
> *or* keep climbing and the profile names the site. A flatten is a WIN, not a dud canary.

---

## Canary A — `MALLOC_ARENA_MAX=2` probe (NO rebuild, reversible)

**Why:** glibc serves the conductor's allocations from up to `8×ncpu` 64 MB arenas. Capping
arenas at 2 collapses arena-spread. If the byte slope **flattens**, the GBs were arena
fragmentation (a free fix — keep it). If it **persists**, that re-confirms a true leak (the
synthesis prediction) and the arenas were only the container, not the cause. Either way it may
slow the OOM cadence. Zero rebuild: `process_manager.rs:64` spawns the child with **no
`env_clear()`**, so a container env var is inherited by the `holochain` child.

**Target:** ONE high-fanout, **non-genesis** anchor for fast signal — `james` (NOT
matthew/adam, the genesis/bootstrap pair). Leave the rest of the fleet unchanged as the control.

🛠 **Apply** (rolling-restarts just that pod):
```
kubectl set env statefulset/elohim-james-alpha -c elohim-node \
  MALLOC_ARENA_MAX=2 MALLOC_TRIM_THRESHOLD_=131072 -n elohim-alpha
kubectl rollout status statefulset/elohim-james-alpha -n elohim-alpha
```
🛠 **Revert:**
```
kubectl set env statefulset/elohim-james-alpha -c elohim-node \
  MALLOC_ARENA_MAX- MALLOC_TRIM_THRESHOLD_- -n elohim-alpha
```

**Watch (2–3 h, james vs an untouched anchor):** Prometheus
```
elohim_node_conductor_anon_bucket_bytes{pod="elohim-james-alpha-0",bucket="8m-64m"}      # slope
elohim_node_conductor_anon_bucket_count{pod="elohim-james-alpha-0",bucket="8m-64m"}      # arena count -> should drop toward ~2-ish
elohim_node_conductor_smaps_anon_bytes{pod="elohim-james-alpha-0",class="other"}         # the headline
```
**Verdict:**
- slope **flattens / RSS drops hard** ⇒ the bytes were glibc arena retention (freed-but-not-returned
  from fragmented/chained arenas), NOT a never-freed code leak — and arena-cap is **most of the fix**.
  Bake it fleet-wide; you may not need Canary B at all. (More plausible than the synthesis implied —
  see the sequence note above; monotonic climb does NOT by itself rule this out.)
- slope **persists** ⇒ genuinely live retention; arena count drops but bytes still climb. Proceed to
  Canary B for the site. (Watch whether OOM cadence lengthens — a partial mitigation worth keeping.)

**Caveat:** the container env also reaches the `elohim-storage` *parent* (not just the child).
Harmless for a probe (storage isn't the leaker; arena-cap just trims its arena spread).

---

## Canary B — jemalloc heap-profiler conductor (the ONLY site-namer)

The fork change is **committed + pushed**: `ethosengine/holochain@elohim-0.6` tip
**`d0f505f`** carries the `jemalloc-prof` feature (`crates/holochain/Cargo.toml` optional
`tikv-jemallocator` with `profiling` + `unprefixed_malloc_on_supported_platforms`;
`crates/holochain/src/bin/holochain/main.rs` `#[cfg(feature="jemalloc-prof")] #[global_allocator]`).

`unprefixed_malloc_on_supported_platforms` is the key choice: a bare `#[global_allocator]` only
profiles **Rust** allocations; the unprefixed feature routes **C-side** malloc (SQLCipher codec,
OpenSSL, CGo shims) through jemalloc too — so the profile is NOT blind to the SQLCipher-churn
candidate (Layer-C #2). Pyroscope is absent on this Grafana, and the prior `pprof_debug.go`
profiles the **Go** runtime (flat 52 MB) — the wrong runtime. jemalloc-prof is the right tool.

✅ **1. Fork pushed** — done this session (`ethosengine/holochain@elohim-0.6` → `d0f505f`). The
build clones that branch, so it picks up the feature automatically; no further fork action.

✅ **2. Pipeline wired — default IS the profiler, with fleet-safe isolated images** — done this
session: `che-devworkspaces@main` (`e87a680`) makes the `elohim-edgenode` job **default to the
jemalloc-prof build** (the UI param wasn't taking — multibranch param-registration lag — so the
default is baked in). When `jemalloc-prof` is in `HC_FEATURES`, the job pushes ISOLATED names
**`elohim-edgenode-prof` / `elohim-storage-prof`** that NEVER overwrite the fleet's
`elohim-edgenode:latest`; the storage stage points `CONDUCTOR_SOURCE` at the prof conductor.
`containers/elohim-edgenode/Dockerfile` already honors `ARG HC_FEATURES`.

🛠 **2b. Run the `elohim-edgenode` job (main) — now trivial:** set **only** `BUILD_STORAGE_CANARY = true`,
leave everything else default. (`HC_FEATURES` already defaults to include `jemalloc-prof`. To build
the PRODUCTION fleet conductor instead, drop `jemalloc-prof` from `HC_FEATURES` → restores
`elohim-edgenode:latest`.) Produces the deployable **`elohim-storage-prof:<tag>`** embedding the
profiling conductor.

Confirm in the build log: the Checkout stage prints `conductor features: …,jemalloc-prof`, AND
`tikv-jemallocator` / `tikv-jemalloc-sys` **COMPILING** (`Compiling tikv-jemalloc-sys…`), not merely
`Adding …` (the latter is lockfile resolution of the optional dep and does NOT mean it's linked —
this is exactly what made build #5 a stock conductor).
- Verified the feature names resolve on `tikv-jemallocator` 0.6.0/0.6.1 (`profiling`,
  `unprefixed_malloc_on_supported_platforms`), so it won't red on that. If it ever does, they live
  on the `-sys` crate — forward as `tikv-jemalloc-sys/profiling` + `…/unprefixed_malloc_on_supported_platforms`.
- For readable stacks: line-tables (root `.cargo/config.toml`) suffice for jeprof; add `libunwind-dev`
  to the builder if stacks look shallow.
- (Local one-off alternative to the storage step: `elohim/elohim-storage/build-storage-canary.sh` —
  redundant with `BUILD_STORAGE_CANARY=true`, kept only for off-Jenkins builds.)

🛠 **3. Deploy to ONE pod (james — anchor, fast signal, non-genesis).** Step 2b already produced
the deployable image; just point james's `elohim-node` at it:
```
kubectl set image statefulset/elohim-james-alpha \
  elohim-node=harbor.ethosengine.com/ethosengine/elohim-storage-prof:<tag> -n elohim-alpha
```
(wire-compat already proven — kitsune2 0.3.2 unchanged; the profiling image differs only by allocator.)

🛠 **4. Activate profiling** — set on james's `elohim-node` container env (inherited by the child):
```
_RJEM_MALLOC_CONF=prof:true,prof_active:true,abort_conf:true,lg_prof_sample:19,lg_prof_interval:30,prof_prefix:<HOLOCHAIN_DATA_DIR>/jeprof
MALLOC_CONF=prof:true,prof_active:true,abort_conf:true,lg_prof_sample:19,lg_prof_interval:30,prof_prefix:<HOLOCHAIN_DATA_DIR>/jeprof
```
- `<HOLOCHAIN_DATA_DIR>` = the conductor data dir (a PVC mount, set at `process_manager.rs:68`) so
  dumps persist and are retrievable. Set BOTH env vars (tikv builds jemalloc with the `_rjem_`
  prefix → `_RJEM_MALLOC_CONF`; `MALLOC_CONF` is a harmless hedge).
- `abort_conf:true` makes a misread/typo'd config **abort loudly at startup** instead of silently
  disabling profiling (so you don't waste a multi-hour run on a dud conf).
- `lg_prof_interval:30` auto-dumps a `jeprof.*.heap` every 2^30 = 1 GiB allocated (~1/h at the
  anchor leak rate) — hands-off; no signal or mctl call needed. `lg_prof_sample:19` = 512 KB
  sampling (fine grain, low overhead).
- **Confirmation the conf was read:** `.heap` files appearing under the prefix dir within the first
  hour. If none land, the prefix dir doesn't exist / isn't writable, or prof didn't enable — fix
  before waiting longer.

🛠 **5. Name the site** — pull two dumps ~1–2 GB of climb apart and diff:
```
jeprof --text --cum --base=jeprof.<early>.heap /usr/local/bin/holochain jeprof.<late>.heap | head -40
```
The top cumulative-growth stacks ARE the leak. (`jeprof` = the perl script from jemalloc;
`apt install libjemalloc-dev` or fetch from the jemalloc release.) Read the verdict:
- `kitsune2_gossip` / `core_fetch` / op-store frames → kitsune2 op retention (Layer-C #1).
- `sqlcipher_codec` / `sqlite3*` / page-cache frames → SQLCipher codec churn (#2).
- a Rust `Vec`/`HashMap`/cache path in holochain → Rust-side cache growth (#3).
- (pion C-side → not expected; Go heap is flat.)

> **CRITICAL — disambiguate "no SQLite frames" first (don't repeat the saga's misread-negative
> deaths).** Before trusting the verdict, confirm the FIRST dump contains *any* C frames at all —
> `jeprof --text … | grep -iE 'sqlite3|sqlcipher|codec|CRYPTO_|OPENSSL'`. The unprefixed feature
> should route the statically-linked SQLCipher/OpenSSL malloc through jemalloc, so C frames SHOULD
> appear. **Zero C frames ⇒ interposition did NOT take ⇒ the profile is Rust-only and "no SQLite"
> is an artifact, NOT exoneration.** In that case fall back to `heaptrack`/`bytehound` (LD_PRELOAD
> hooks libc malloc unconditionally) — do not conclude the leak is Rust-side.
>
> **Also bound the profiler's blind spot:** jemalloc-prof only sees the malloc family. Direct
> `mmap` (wasmer linear memory, the Go heap) is invisible to it. That's fine here (the evidence
> puts the leak in malloc arenas), but if the profile looks clean while RSS keeps climbing, first
> re-confirm the direct-mmap bands stayed flat (Go `0xc0…` ~52 MB; `256m+` empty) before concluding
> "found nothing."

**No-rebuild alternative to B:** `heaptrack`/`bytehound` hook libc malloc via `LD_PRELOAD`, so they
catch C allocations without the jemalloc rebuild — but need the tool in the image AND the child
launched under it (a `process_manager.rs` spawn wrapper or entrypoint `LD_PRELOAD`). Distro
`libjemalloc.so` is usually built WITHOUT `--enable-prof`, so plain `LD_PRELOAD` of it won't
profile — that's why the in-binary `profiling` feature (Canary B) is the reliable path.

---

## The shared cure-validation gate (for the eventual fix, any candidate)
Once the profiler names the site and a bounded-collection/drop-path fix is built, it deploys via
the SAME harness and proves itself by the SAME signal as the deploy recipe:
**`elohim_node_conductor_smaps_anon_bytes{class="other"}` FLATTENS** on a canary leecher over a
multi-hour window while the pod stays in-mesh (peerCount stable, no partition). Roll wider only
then; **genesis pair (matthew/adam) LAST.** This is the only ground-truth cure proof — the prior
fix tested green in unit tests and did NOT flatten the slope in production.

## Files
- Fork change (staged): `elohim/holochain-conductor/crates/holochain/{Cargo.toml, src/bin/holochain/main.rs}`.
- RCA: `conductor-leak-rca-diverse-eyes-synthesis-2026-06-18.md` (+ `…-native-heap-reframe-…`).
- Deploy/build env + cure signal: `conductor-leak-deploy-recipe-2026-06-17.md`.
- Spawn seam (env inheritance, no env_clear): `elohim/elohim-storage/src/conductor/process_manager.rs:64,68`.
