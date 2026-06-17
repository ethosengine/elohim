---
title: Conductor anon leak is mmap-count accumulation (many discrete large mappings) — H1 falsified, H4 not-the-slope, H3 leading
kind: backlog
status: confirmed
tags: [decision-record, runtime-memory, conductor, kitsune2, validation-receipt, self-heal, design-decision-toolkit]
occurred_at: 2026-06-17
---

# Conductor anon leak is mmap-count accumulation, not in-place buffer / heap / arena / stack growth

This is the **mechanism** companion to `conductor-memory-attribution-verdict.md` (which settled *where*: the
holochain conductor child). It settles *what shape* of leak, and ranks the §4 hypotheses of
`HANDOFF-2026-06-17-conductor-leak-hunt.md` against the per-class smaps evidence. Per that handoff §7, the
per-hypothesis records (H1/H2/H3/H4) are consolidated here because a **single instrument** — the conductor
smaps breakdown log line — settles all four at once.

**Lever / hypothesis:** Which §4 mechanism explains the conductor anon climb — H1 (in-place validation-receipt
buffer accumulation), H3 (per-connection/per-op kitsune2 mapping accumulation), H4 (glibc arena
fragmentation), with H2 (unresolvable peer-URL failure) as candidate trigger?

**Instrument** (reproducible; the toolkit P2 sampler logs this to stdout → Loki, datasource uid `loki`):
- Per-class smaps breakdown (one line/minute), `target=elohim_storage::memory_attribution`,
  `message="conductor anon smaps breakdown"`, fields `heap_bytes / stack_bytes / other_anon_bytes /
  anon_mapping_count / largest_anon_bytes`:
  `max_over_time({namespace="elohim-alpha", pod="elohim-jessica-alpha-0"} |= "conductor anon smaps breakdown" | json v="fields.<field>" | unwrap v [5m])`
- Per-process thread count, `message="per-process rss split"`, proc=holochain:
  `max_over_time({namespace="elohim-alpha", pod="elohim-jessica-alpha-0"} |= "per-process rss split" | json proc="fields.proc", v="fields.threads" | proc="holochain" | unwrap v [5m])`
- cgroup sawtooth (fused conductor+storage; storage parent is flat ~101 MB so it tracks the conductor):
  `container_memory_rss{namespace="elohim-alpha", container="elohim-node", pod=~"elohim-(jessica|james|matthew)-alpha-0"}` (datasource `prometheus`, cadvisor).
- Receipt-error rate:
  `count_over_time({namespace="elohim-alpha", pod="elohim-jessica-alpha-0"} |= "send_validation_receipts could not find url for peer" [5m])`.
- NOTE: the toolkit's Prometheus gauges (`elohim_node_conductor_smaps_anon_bytes` etc.) are **NOT scraped**
  (no PodMonitor + default-deny NetworkPolicy) — the discriminator came from the **Loki log lines**, which
  carry the same numbers. (Scrape-wiring is a separate, off-critical-path item — see Brake #4.)

**Measured effect** (jessica current pod, one clean cycle 03:00→03:15Z 2026-06-17; james 03:13→03:15Z):

| field | jessica 03:00 | 03:05 | 03:10 | 03:15 | james 03:13→03:15 |
|---|---|---|---|---|---|
| `other_anon_bytes` | 1.28 GB | 1.60 GB | 3.25 GB | 3.44 GB | 4.15 → 4.21 GB |
| `anon_mapping_count` | 4,351 | 4,667 | 5,496 | 5,254 | 5,341 → 5,365 |
| `largest_anon_bytes` | 77 MB | 77 MB | **158,265,344** | **158,265,344** | **158,265,344** (flat) |
| `heap_bytes` | 22.6 MB | 52.5 MB | 52.5 MB | 52.5 MB | 19.8 MB (flat) |
| `threads` (pid 17) | ~430 | 435 | 435 | 435 | (jessica 409→447 over ~80 min) |

Derived facts:
- The climb lives **entirely in `other_anon`** (unlabeled anonymous mappings). `heap` flat, `largest` flat,
  threads ~flat.
- The climb is **bursty, not linear**: jessica intervals = **65 / 330 / 38 MB/min** (a +1.65 GB burst in the
  03:05–03:10 bin, then near-flat). Do not represent it as a steady per-minute slope.
- **Average mapping size is moving** (other_anon ÷ count): 294 → 344 → 592 → 655 KB across the cycle; marginal
  new mappings ≈ 1.9 MB. All ≫ glibc `MMAP_THRESHOLD` (128 KB) ⇒ these are **direct `mmap` of large
  allocations**, each its own anon mapping.
- **Thread growth accounts for <5% of new mappings**: threads +38 (409→447) over ~80 min while count grows
  ~900–1,100 per cycle. Since post-Linux-4.5 pthread stacks fall in OtherAnon, this rules out **stacks** as
  the driver in the same breath as ruling out arenas.
- `largest_anon_bytes` = **158,265,344 bytes byte-identical on jessica AND james** ⇒ a *fixed* allocation
  (same DNA → same size; almost certainly wasmer linear memory). **Ruled out as the leak** — fixed, not growing.
- Receipt-error rate ≈ **16/s (~960/min)** vs new mappings ~60–115/min ⇒ **~1 new mapping per ~12 receipt
  errors**. The receipt error is a *correlated symptom*, not a 1:1 allocator.
- cadvisor sawtooth persists **byte-for-byte across images** `8c217137` / `ea494df1` / `1907249a` — **expected**,
  because the leaking conductor binary is baked into the holo-host base image
  (`ghcr.io/holo-host/edgenode:…hc0.6.0…`); the elohim-storage tags only change the parent. This positively
  reconfirms conductor-not-storage and means the in-progress edge deploy will not move this verdict.

**Verdict** (per hypothesis — what each rules IN / OUT):

- **H1 — in-place validation-receipt buffer accumulation: FALSIFIED (as stated).** `largest_anon` is flat
  (158 MB, byte-identical, not growing) and `heap` is flat ⇒ **no single structure grows in place**. The leak
  is count-driven, not buffer-growth-driven. Rules OUT "one unbounded receipt/op buffer." (The receipt
  workflow may still be the *trigger* — that is H2, a different claim.)
- **H3 — per-connection / per-op kitsune2 mapping accumulation: LEADING (not confirmed).** Rising
  `anon_mapping_count` + flat `largest` + ~95% non-stack ⇒ **many discrete large `mmap` allocations
  accumulating and never freed** = per-something buffers retained. This is the mechanism *shape*; the specific
  buffer/structure is **not yet pinned** (needs allocation-site profiling — heaptrack / jemalloc-prof on a
  canary via the spawn-wrap seam, handoff §3(b)). Do NOT mark confirmed: narrowed by elimination, not caught
  in the act.
  **Upstream-refined specific candidate (2026-06-17 source-read):** the conductor wires
  `kitsune2_transport_tx5` 0.3.2 (tx5 0.8.1 / go-pion WebRTC + sbd) — **NOT iroh** (the iroh transport, kitsune2
  PR #382, landed *after* the v0.3.2 tag). So the leading specific mechanism is **per-send networking-buffer
  retention on the tx5/holochain_p2p path**: `transport_tx5::send` copies `data.to_vec()` per send (v0.3.2
  `lib.rs:340`), matching the **non-iroh 65% component** of holochain issue **#5664** (`BytesMut::put_slice →
  RawVec::grow_amortized`, 368→431 MB retained). This **rules OUT** the kitsune2 peer-store unbounded-growth
  class (PR #408 CRIT-1 — real at v0.3.2, but `AgentInfo` records are sub-1 KB ⇒ would grow `heap`, not spawn
  >128 KB mmaps; the flat heap falsifies it). The >128 KB-discrete-mmap + flat-heap fingerprint is itself the
  glibc signature of thousands of *large* retained allocations — consistent with buffer retention, not record
  accumulation.
- **H4 — glibc arena fragmentation: FALSIFIED as the *slope* driver.** 5,000+ anon mappings ≫ max glibc
  arenas (~8×4 cpu = 32); the slope is direct-mmap-count growth, which an arena cap cannot touch.
  `MALLOC_ARENA_MAX=2` can trim the **baseline/intercept** at most — exactly what the handoff predicted
  ("likely hits intercept not slope"), so this evidence *confirms the handoff's expectation* rather than
  overturning it. Rules OUT arena fragmentation as the OOM cause. (Conductor allocator = glibc, high
  confidence from the Debian/glibc holo-host base; a live `ldd` on the child remains the one unconfirmed
  Step-0 fact, but it does not change this verdict — even with a cap, the slope is mmap-count.)
- **H2 — unresolvable peer-URL failure as TRIGGER: PROPOSED (separate record from the mechanism).** The
  conductor perpetually logs `send_validation_receipts could not find url for peer` (~16/s); failed peer-URL
  resolution is the leading trigger that drives the connection/gossip machinery to allocate-and-retain.
  **Trigger ≠ mechanism.** Causal proof requires the **flatten-on-resolve** experiment (does the anon slope
  flatten when peers resolve, e.g. after the `MongoK2Store` shared-bootstrap fix on a canary, or as doorway
  `/health` peerCount climbs). **Trigger mechanism CONFIRMED at source (2026-06-17):**
  `validation_receipt_workflow.rs:87` @ rev `a6d4e805` only logs+continues on the peer-URL miss — it does
  **NOT** clear `require_receipt`, so ops stay flagged and are re-fetched/re-driven every workflow tick (the
  ~16/s spam is a DB-backed re-drive loop; the actual allocation happens downstream in the
  `send_validation_receipts` p2p call, i.e. the per-send buffers of H3). holochain **PR #5718** (backport
  **#5719**) fixes exactly this — treats URL-less authors as offline and clears the requirement — merged to
  `develop-0.6` 2026-04, **after our pinned rev, not in a release we run**; its author explicitly flags it as a
  *mitigation*, not the leak fix. So the trigger is real and present in our build; the causal trigger→leak link
  is still what the flatten-on-resolve / #5719 pin-forward experiment proves.

**Brake / action:**
1. The mechanism is mmap-count accumulation in the conductor networking stack → the fix lives **upstream**
   (holochain `0.6.0-dev.28` / kitsune2 `v0.3.2`) and/or is **removed by fixing our bootstrap islanding**
   (H2 trigger; `2026-06-14-federation-bootstrap-plan.md`, `MongoK2Store`). Likely both: report the
   unbounded-retain-on-failure upstream **and** fix bootstrap so resolution succeeds (removes the trigger).
   Upstream-vs-our-config fork = handoff §6. **Upstream search result (2026-06-17,
   `.claude/data/conductor-leak-upstream-research-2026-06-17.md`): KNOWN-OPEN, no clean pin-to-fix** — the
   closest match (#5664) is partly iroh-specific (not our transport), and no released 0.6.x fixes the
   per-send-buffer retention. Two concrete moves: (i) **pin forward to the first 0.6.x containing #5719** to cut
   the trigger re-drive, then **re-measure** — if the anon climb stops, that corroborates the per-send-buffer
   hypothesis (a clean natural experiment, and the cheapest real test left); (ii) **file one upstream issue**
   (holochain/holochain, cross-linking #5664 + kitsune2) framed as *per-send networking-buffer retention on the
   tx5/holochain_p2p path, re-driven by the validation-receipt peer-URL miss* — lead with the >128 KB-discrete-
   mmap fingerprint so maintainers look at buffer retention, not small-record peer-store growth.
2. Do NOT: bump RAM, cap SQLite `cache_size`, shard arc (prior verdict), **or spend a deploy on
   `MALLOC_ARENA_MAX` as a fix** — it cannot address the slope (this record). The slope, not the intercept,
   is the OOM cause.
3. To pin the specific buffer: **heaptrack / jemalloc-prof on a jessica/james canary** via the spawn-wrap
   seam (`process_manager.rs` `.env()` / `Command` builder, handoff §3(b)) — reserved until now because the
   class is finally narrowed; this is the next instrument.
4. **(Off critical path)** wire the toolkit `/metrics` surface into Prometheus — a `PodMonitor` (port
   8090 `/metrics`) + a scoped `NetworkPolicy` (observability → 8090), since elohim-alpha is default-deny and
   has zero monitors. Native trend/alerting hygiene; the Loki log lines already carry the data the hunt needed.

**Lineage:**
- `HANDOFF-2026-06-17-conductor-leak-hunt.md` (§3 instrument options, §3.1 smaps-class decoder, §4 hypotheses)
- `genesis/data/timeline/backlog/conductor-memory-attribution-verdict.md` (attribution: conductor child)
- `genesis/data/timeline/backlog/arc-shrink-ineffective-memory-soak.md` (arc falsified)
- `genesis/data/timeline/backlog/decision-record-discipline.md` (the discipline this conforms to)
- `genesis/docs/superpowers/plans/2026-06-17-design-decision-toolkit-plan.md` (toolkit P2/P3)
- `genesis/docs/superpowers/plans/2026-06-14-federation-bootstrap-plan.md` (H2 trigger fix home)
- `.claude/data/conductor-leak-upstream-research-2026-06-17.md` (upstream search: transport elimination tx5≠iroh, #5664/#5718/#5719/#408 analysis, source-read of `validation_receipt_workflow.rs:87`)
- Instrument source: `elohim/elohim-storage/src/services/system_metrics.rs:443-500` (`parse_smaps_anon`,
  `classify_mapping`), `elohim/elohim-storage/src/metrics.rs` (gauge surface, currently unscraped)
