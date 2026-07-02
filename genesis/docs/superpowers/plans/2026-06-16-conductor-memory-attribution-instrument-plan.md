---
title: Conductor Memory Attribution Instrument — the leak-vs-cache discriminator (P-ARC §B gate)
id: conductor-memory-attribution-instrument-plan
status: Draft
cites:
  - matthew-edge-resiliency-rca-fanout-synthesis | Matthew Edge Resiliency | sha256:a4fafb4f91612eba | path: genesis/docs/content/elohim-protocol/history/2026-06-15-matthew-edge-resiliency-rca-fanout-synthesis.md
  - genesis/data/timeline/backlog/arc-shrink-ineffective-memory-soak.md
  - genesis/docs/superpowers/plans/2026-06-14-dataplane-arc-plan.md
domain: dataplane / conductor-memory
sprint: dataplane (P-ARC §B leak-vs-bounded prerequisite)
# No doc-level requires_env: MIXED plan. Tasks 1–4 (readers + parsers + sampler) are
# unit-testable on household-nodes. Only Task 6 (deploy + observe the verdict) needs
# the live alpha edge + observability — tagged inline @requires:observability.
---

# Conductor Memory Attribution Instrument — leak-vs-cache discriminator (P-ARC §B gate)

> For agentic workers: REQUIRED SUB-SKILL: superpowers:test-driven-development (pure parsers are TDD'd) + superpowers:subagent-driven-development. Steps use checkbox (- [ ]) syntax.
> Working draft — NOT cite-sealed (seal at land per /plan Step 4).

## 1. CONTEXT / WHY

matthew's `elohim-node` container OOM-flaps (8.4 GB at its 8Gi cgroup ceiling, ~3h sawtooth; its doorway restarts ~4×/hr → the 503). **The operator's directive: "16GB shouldn't be the solution."** It isn't — bumping the cgroup limit changes the *intercept* of the sawtooth, not its *slope*. The soak (`arc-shrink-ineffective-memory-soak.md`) **falsified arc-shrink as the memory lever**: jessica, a confirmed `arc=0` leecher, still sawtooths to *her* 4Gi ceiling every ~40 min — identical shape, only the ceiling (= the cgroup limit) differs. So the runaway is **arc-independent**, and we are **flying blind on which process and which memory class drives it** because the consolidated `elohim-node` container fuses the Holochain conductor child + the elohim-storage parent into ONE cgroup with ONE `container_memory_working_set_bytes`.

This plan builds the **measurement that ends the blindness** — the exact discriminator P-ARC's Decision Memo §B is *hard-gated on*:

> P-ARC §B: "Hard gate on (iii): confirm **leak-vs-bounded-large first** (operator-side `ps -o rss,comm` conductor-child vs storage-parent split, or `target_arc_factor: 0` ablation on one loaded node)."

It does NOT apply a fix. It produces the **verdict** (heap leak vs page cache vs slab) that *selects* the fix. "Apply the matching brake" is the explicit follow-on, branch-chosen by this plan's output (see §7).

### What's already here (compose, don't re-invent)
- `services/system_metrics.rs` owns the pure per-node readers + the `parse_cgroup_mem_limit` pure-parser idiom (table-tested). **It is the home for the new readers.** Its existing `process_memory_bytes()` is `getrusage(RUSAGE_SELF)` — storage *parent* only, **peak** not current, **no anon/file split, no child** → insufficient for the OOM question.
- `conductor/process_manager.rs` `ConductorManager` owns the conductor `Child` (the child pid via `child.id()`). No sampler exists.
- **There is NO Prometheus app-scrape for elohim-storage** (no `/metrics` endpoint, no PodMonitor/ServiceMonitor — only cadvisor/kube-state infra metrics). The crate's metric idiom is a **structured `tracing` log line scraped from Loki** ("the operator's log-scraped metric surface" — verbatim from `identity_namespace.rs` `counter="elohim_identity_namespace_violation_total"`). **Loki is the surface** (matthew's stream: 2.4M entries/988MB in the last hour — guaranteed-observable via the observability MCP).
- The `record_peer_status` heartbeat (`main.rs:678`) is the **60s-cadence `tokio::spawn` precedent** for a sampler.

### THE DECISIVE INSTRUMENT (advisor correction — primary-source-grounded)
Per-process `/proc/<pid>/status` `RssFile` will **NOT** capture the thing we most need. SQLite uses ordinary `pread`/`pwrite` by default (NOT `mmap`), so its page cache lives in the **kernel** page cache — accounted to the **cgroup `file` counter** but invisible to every process's `RssFile`. If matthew's 8 GB is SQLite page cache, a per-process sum would *under-count* and read as a confusing gap, not an answer. Therefore:

1. **PRIMARY (the verdict): cgroup `/sys/fs/cgroup/memory.stat` → `anon` / `file` / `slab`** (+ `memory.swap.current`). This splits exactly the leak-vs-cache question. `container_memory_working_set_bytes` already nets out `inactive_file`, so the 8.4 GB we watch is ≈ `anon + active_file + slab`; `memory.stat` decomposes it.
2. **SECONDARY (the attribution): per-process `/proc/<pid>/status` `RssAnon`** for the conductor child vs the storage parent — the fused-cgroup attribution RCA §4.2 asks for.
3. **Reconciliation note (bake into the verdict doc so the result isn't misread):** Σ(per-proc `RssAnon`) ≈ cgroup `anon` confirms attribution; a large cgroup `file` with small per-proc `RssFile` is **EXPECTED** (kernel syscall page cache), not a bug.

---

## 2. OWNED FILES

This plan creates/mutates EXACTLY:

- **M** `elohim/elohim-storage/src/services/system_metrics.rs` — new pure parsers `parse_cgroup_memory_stat`, `parse_proc_status`; new readers `cgroup_memory_breakdown()`, `proc_rss(pid)`, `cgroup_cpu_quota_cores()`. Additive; no existing reader changed.
- **M** `elohim/elohim-storage/src/conductor/process_manager.rs` — new `pub fn child_pid(&self) -> Option<u32>` accessor. Additive.
- **M** `elohim/elohim-storage/src/main.rs` — one new `tokio::spawn` 60s memory-attribution sampler + a one-shot boot line (near the conductor setup, ~`:640`). Additive.
- **C** `genesis/data/timeline/backlog/conductor-memory-attribution-verdict.md` — the verdict write-up (filled by Task 6 from live Loki evidence).

### Collision / seam statement (compose with P-ARC — sequenced, not parallel)
P-ARC (`2026-06-14-dataplane-arc-plan.md`) also lists `system_metrics.rs` (Task 2: `local_authored_bytes`) and `process_manager.rs` (Task 3: `StaggerGate`) in its owned set. **No live conflict:** P-ARC's whole Tasks section is **banner-GATED "DO NOT DISPATCH UNTIL [operator] CHOSEN"** and its §B gate **waits on this plan's verdict**. This plan therefore sequences *before* P-ARC. The additions are orthogonal (different functions in the same files). **Hand-off note:** when P-ARC dispatches, its `local_authored_bytes`/`StaggerGate` edits land additively alongside these readers/accessor — no rebase hazard beyond a same-file merge. `elohim/elohim-storage/src/main.rs` is owned by no plan (F-COHERENCE confirms it is not in any ledger file-map); this sampler spawn is additive and flagged for the integrator.

---

## 3. p2p-class (cite, do not re-litigate)
All three readers are **Cat-C node-local operational reads** — `/sys/fs/cgroup/*` and `/proc/<pid>/status` snapshots. No DHT entry, no table, no coordinator fn, no route. Consistent with `system_metrics.rs`'s existing module-level Cat-C classification. No `p2p-design-gate` invocation required (no new entity).

---

## 4. BUILD / TEST COMMANDS (elohim-storage = WASM-flagged; verified idiom from P-ARC §5)
```
# Unit tests (Tasks 1, 2 — pure parsers + readers, module-scoped)
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo test --lib system_metrics 2>&1 | tail -40
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo test --lib process_manager 2>&1 | tail -40

# Final gate (whole lib + clippy + fmt)
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo test --lib 2>&1 | tail -40
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo clippy --lib -- -D warnings 2>&1 | tail -40
cd /projects/elohim/elohim/elohim-storage && cargo fmt --check
```
Rules (memory): `RUSTFLAGS='--cfg getrandom_backend="custom"'` for elohim-storage; `RUSTC_WRAPPER=""` (sccache spawn-ENOENT); `/tmp` target (pool fingerprint-ENOENT); **plain `cargo test`, NEVER nextest**; never `&&`-pipe a gate exit code. **Carry-forward:** `tests/phase4_e2e.rs` + `tests/content_safety_integration.rs` are PRE-EXISTING broken (missing `gate_client::testing`) — `--lib` is unaffected; do not chase them.

---

## TASK 1 — Pure parsers (TDD: failing test first)

**Files:** `elohim/elohim-storage/src/services/system_metrics.rs` (append to `#[cfg(test)] mod tests` + add the pure fns near `parse_cgroup_mem_limit`).

Two pure, filesystem-free parsers mirroring the existing `parse_cgroup_mem_limit` table-test pattern:

- `parse_cgroup_memory_stat(raw: &str) -> Option<CgroupMemBreakdown>` — parse `memory.stat` key→value lines. cgroup **v2** keys: `anon`, `file`, `slab` (fallback `slab_reclaimable`+`slab_unreclaimable`). cgroup **v1** fallback vocab: `rss`→anon, `cache`→file. Returns bytes (these files are already in bytes, unlike `/proc/status` kB). `None` if neither `anon` nor `rss` present.
- `parse_proc_status(raw: &str) -> Option<ProcRss>` — parse `/proc/<pid>/status` `Key:\t  N kB` lines: `RssAnon`, `RssFile`, `VmRSS`, `Threads`. Rss* are kB → **×1024 to bytes**; `Threads` is a count. `None` if `VmRSS` absent.

Types (add near the readers):
```rust
/// cgroup memory.stat breakdown — the leak-vs-cache verdict (bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CgroupMemBreakdown {
    pub anon: u64,  // unreclaimable heap (swap off) — the leak suspect
    pub file: u64,  // kernel page cache (SQLite pread/pwrite lands here) — the cache suspect
    pub slab: u64,  // kernel slab (dentry/inode/etc.)
}
/// Per-process RSS split from /proc/<pid>/status (bytes; threads = count).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProcRss {
    pub rss_anon: u64,
    pub rss_file: u64,
    pub vm_rss: u64,
    pub threads: u64,
}
```

- [ ] **Write failing tests** — append table-driven tests (mirror `parse_cgroup_mem_limit_cases`):
```rust
    #[test]
    fn parse_cgroup_memory_stat_v2_and_v1() {
        // cgroup v2 shape.
        let v2 = "anon 6442450944\nfile 1610612736\nkernel_stack 1000\nslab_reclaimable 50\nslab_unreclaimable 150\nsock 0\n";
        let b = parse_cgroup_memory_stat(v2).expect("v2 parses");
        assert_eq!(b.anon, 6_442_450_944);
        assert_eq!(b.file, 1_610_612_736);
        assert_eq!(b.slab, 200, "slab = reclaimable + unreclaimable when no `slab` key");
        // explicit v2 `slab` key wins if present.
        let v2s = "anon 1\nfile 2\nslab 999\nslab_reclaimable 1\n";
        assert_eq!(parse_cgroup_memory_stat(v2s).unwrap().slab, 999);
        // cgroup v1 vocab fallback.
        let v1 = "cache 3221225472\nrss 2147483648\nrss_huge 0\n";
        let b1 = parse_cgroup_memory_stat(v1).expect("v1 parses");
        assert_eq!(b1.anon, 2_147_483_648, "v1 rss -> anon");
        assert_eq!(b1.file, 3_221_225_472, "v1 cache -> file");
        // garbage / empty.
        assert_eq!(parse_cgroup_memory_stat(""), None);
        assert_eq!(parse_cgroup_memory_stat("totally unrelated\n"), None);
    }

    #[test]
    fn parse_proc_status_extracts_rss_split_kb_to_bytes() {
        let s = "Name:\tholochain\nThreads:\t42\nVmRSS:\t  8388608 kB\nRssAnon:\t  6291456 kB\nRssFile:\t  2097152 kB\nVmSwap:\t 0 kB\n";
        let r = parse_proc_status(s).expect("parses");
        assert_eq!(r.vm_rss, 8_388_608 * 1024);
        assert_eq!(r.rss_anon, 6_291_456 * 1024);
        assert_eq!(r.rss_file, 2_097_152 * 1024);
        assert_eq!(r.threads, 42);
        assert_eq!(parse_proc_status("Name:\tx\n"), None, "no VmRSS -> None");
    }
```
- [ ] Run, expect FAIL (`cannot find function parse_cgroup_memory_stat`): `... cargo test --lib system_metrics 2>&1 | tail -40`.
- [ ] Write the two pure parsers (key→value line scan; tolerate tabs/multiple spaces; ignore unknown keys).
- [ ] Run, expect PASS.
- [ ] Commit (selective-stage `system_metrics.rs` only): `feat(elohim-storage): pure cgroup memory.stat + /proc/status RSS-split parsers`.

## TASK 2 — Linux readers wrapping the parsers

**Files:** `system_metrics.rs` (additive readers; `#[cfg(target_os="linux")]` with non-Linux `None` stubs, mirroring `container_memory_limit_bytes`).

- [ ] Implement:
```rust
#[cfg(target_os = "linux")]
pub fn cgroup_memory_breakdown() -> Option<CgroupMemBreakdown> {
    // v2 unified first, then v1 — mirror container_memory_limit_bytes() probe order.
    let raw = std::fs::read_to_string("/sys/fs/cgroup/memory.stat")
        .or_else(|_| std::fs::read_to_string("/sys/fs/cgroup/memory/memory.stat"))
        .ok()?;
    parse_cgroup_memory_stat(&raw)
}

/// Current swap charged to this cgroup (bytes). With swap OFF this is 0, which is
/// why an OOM leans toward `anon` (unreclaimable) being the driver. None if absent.
#[cfg(target_os = "linux")]
pub fn cgroup_swap_current_bytes() -> Option<u64> { /* read memory.swap.current | v1 memsw */ }

/// Effective CPU quota in cores from /sys/fs/cgroup/cpu.max ("quota period"; "max" => None).
#[cfg(target_os = "linux")]
pub fn cgroup_cpu_quota_cores() -> Option<f64> { /* quota/period; None when unbounded */ }

pub fn proc_rss(pid: u32) -> Option<ProcRss> {           // Linux-gated body; None elsewhere
    #[cfg(target_os = "linux")]
    { return std::fs::read_to_string(format!("/proc/{pid}/status")).ok().as_deref().and_then(parse_proc_status); }
    #[cfg(not(target_os = "linux"))]
    { let _ = pid; None }
}
```
  (Non-Linux `None` stubs for the three `#[cfg(target_os="linux")]` fns.)
- [ ] Add a light reader test: `proc_rss(std::process::id())` returns `Some` with `vm_rss > 0` on Linux (the test process is alive); `cgroup_memory_breakdown()` is env-dependent — assert "Some ⇒ anon+file ≤ a sane ceiling" only if Some (like `container_memory_limit_is_optional_and_sane`).
- [ ] Run, expect PASS. Commit: `feat(elohim-storage): cgroup memory.stat + per-process RSS + cpu-quota readers`.

## TASK 3 — `child_pid()` accessor on `ConductorManager`

**Files:** `conductor/process_manager.rs`.

- [ ] Add:
```rust
    /// PID of the live conductor child, or None if not started / already reaped.
    /// The sampler reads this each tick to attribute the fused-cgroup working set
    /// to the conductor vs the storage parent (`std::process::id()`).
    pub fn child_pid(&self) -> Option<u32> {
        self.child.as_ref().and_then(|c| c.id())
    }
```
- [ ] Add a unit test: a fresh `ConductorManager::new(...)` (no `start()`) has `child_pid() == None`. (Do NOT spawn a real conductor in the unit test — `child.id()` semantics are exercised live in Task 6.)
- [ ] Run, expect PASS. Commit: `feat(elohim-storage): ConductorManager::child_pid accessor for memory attribution`.

## TASK 4 — The 60s sampler + boot line in `main.rs`

**Files:** `elohim/elohim-storage/src/main.rs` (one additive `tokio::spawn` near the conductor setup ~`:640`, after the `conductor_manager` binding at `:636`).

Surface = structured `tracing` lines under `target: "memory_attribution"` (Loki-scraped; no Prometheus needed). One boot line; then one `scope="cgroup"` line + one `scope="proc"` line per process, every 60s.

- [ ] Implement (guard on `conductor_manager.is_some()`):
```rust
    if let Some(cm) = &conductor_manager {
        let cm = std::sync::Arc::clone(cm);
        let parent_pid = std::process::id();
        tokio::spawn(async move {
            use elohim_storage::services::system_metrics as sm;
            let cpus = sm::cpu_count().unwrap_or(0);
            let db_max_readers = (2 * cpus).max(8); // conductor default calculate_default_db_max_readers
            info!(target: "memory_attribution", event = "boot",
                cpu_count = cpus, db_max_readers = db_max_readers,
                cgroup_cpu_quota_cores = ?sm::cgroup_cpu_quota_cores(),
                cgroup_mem_limit_bytes = ?sm::container_memory_limit_bytes(),
                "memory-attribution sampler started");
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                tick.tick().await;
                if let Some(b) = sm::cgroup_memory_breakdown() {
                    info!(target: "memory_attribution", scope = "cgroup",
                        anon_bytes = b.anon, file_bytes = b.file, slab_bytes = b.slab,
                        swap_bytes = ?sm::cgroup_swap_current_bytes(),
                        "cgroup memory breakdown (VERDICT: anon=heap/leak, file=page-cache)");
                }
                let child_pid = { cm.lock().await.child_pid() };
                for (name, pid) in [("holochain", child_pid), ("elohim-storage", Some(parent_pid))] {
                    if let Some(pid) = pid {
                        if let Some(r) = sm::proc_rss(pid) {
                            info!(target: "memory_attribution", scope = "proc", proc = name, pid = pid,
                                rss_anon_bytes = r.rss_anon, rss_file_bytes = r.rss_file,
                                vm_rss_bytes = r.vm_rss, threads = r.threads,
                                "per-process rss split (attribution)");
                        }
                    }
                }
            }
        });
    }
```
- [ ] Verify the EnvFilter does NOT suppress `info` for this target (storage runs at info; the RCA's saturation spam is info-level, so info lines reach Loki). If a target filter exists, ensure `memory_attribution=info` is admitted.
- [ ] Gate: `... cargo test --lib 2>&1 | tail -40` (compile + whole lib) → clippy → fmt.
- [ ] Commit: `feat(elohim-storage): 60s memory-attribution sampler (cgroup verdict + per-process attribution)`.

## TASK 5 — Final local gate (FINISH of the build leg)
- [ ] `... cargo test --lib 2>&1 | tail -40` (green), `... cargo clippy --lib -- -D warnings`, `cargo fmt --check` all pass. This is "compile-green" — NOT "done" (see Task 6).

## TASK 6 — Deploy + observe the verdict on JESSICA  ·  @requires:observability  ·  THE REAL FINISH LINE

> **"Done" is a verdict from live Loki, not a green build.** This plan's deliverable is a *measurement*.

> **Subject = jessica (NOT matthew).** jessica is `arc=0` and still OOMs every ~40 min (4× faster to a full sawtooth than matthew's ~3h), AND she is a confirmed leecher holding ~no corpus — so if HER `anon` climbs, that is the leak nailed **independent of arc**, the cleanest possible result.

> **Deploy = push `dev` → edge rebuild → rollout (RESTARTS the alpha edge).** Operator authorized this ("fresh deployments if needed"). **Be the SOLE dispatcher** (concurrent-push mutual-abort history) and **surface the one-liner before pushing.** The image rolls to all alpha pods (last roll picked up `8c217137`).

- [ ] Integrator pushes the commits to `dev`; confirm edge build SUCCESS (ci-observer / Jenkins MCP) and that jessica's pod image bumps (kube_pod_container_info image tag changes from `8c217137`).
- [ ] Wait one full jessica sawtooth (~40–50 min from a fresh restart to OOM). Then query Loki via the observability MCP (datasource `loki`). The subscriber is `tracing_subscriber::fmt().json()`, so event fields land under `fields_*` after `| json`; the conductor child shares the container stdout (non-JSON), so the message-string line filter MUST come before `| json`. The cgroup verdict stream:
```
{namespace="elohim-alpha", pod="elohim-jessica-alpha-0", container="elohim-node"} |= "cgroup memory breakdown" | json
```
  read `fields_anon_bytes` vs `fields_file_bytes` vs `fields_slab_bytes` over the cycle (graph: `... |= "cgroup memory breakdown" | json | unwrap fields_anon_bytes`). And the attribution stream:
```
{namespace="elohim-alpha", pod="elohim-jessica-alpha-0", container="elohim-node"} |= "per-process rss split" | json | fields_proc="holochain"
```
- [ ] **Apply the verdict criteria** and write `genesis/data/timeline/backlog/conductor-memory-attribution-verdict.md`:
  - **`anon` rises monotonically across the sawtooth and resets at restart → HEAP LEAK.** Brake = chase the leak (attribution says conductor vs storage). 
  - **`anon` plateaus while `file`/working-set climbs to the ceiling → PAGE CACHE.** Brake = cap SQLite `cache_size`/`mmap_size` (bounded-cache branch — "traffic control").
  - **`slab` dominant → kernel slab** (dentry/inode/socket) — rarer; investigate fd/inode growth.
  - Record the **reconciliation**: Σ per-proc `RssAnon` ≈ cgroup `anon` (attribution sound); large cgroup `file` + small per-proc `RssFile` is EXPECTED (kernel page cache), NOT a discrepancy.
- [ ] Cross-confirm on matthew (`elohim-matthew-alpha-0`, full-arc) — same query; if matthew's split matches jessica's shape, the verdict is arc-independent (the soak's prediction) and applies to the anchor.

---

## 7. // FOLLOW-ON — "apply the matching brake" (branch-chosen by Task 6's verdict)
- **If LEAK:** a new plan to chase the growing `anon` in the attributed process (heap profiling / bounded buffers / backpressure on the op-integration pipeline). Feeds P-ARC §B's "(iii) shrinks the structure that leaks" branch.
- **If PAGE CACHE:** cap SQLite `cache_size`/`mmap_size` on the conductor DB (the bounded-buffer "traffic control" the operator asked for) + consider `db_max_readers` interaction (more readers ⇒ more cache; RCA §3). Feeds P-ARC §B's "(iii) lowers the plateau" branch.
- **Unblocks P-ARC §B gate** either way: the Decision Memo can choose option (iii) (corpus-off-DHT) with the leak-vs-bounded discriminator answered.
- **Optional Prometheus surface:** if graphing is wanted later, a `/metrics` endpoint + PodMonitor is a separate infra add — NOT needed for this verdict (Loki suffices).
- **Promote to a permanent gauge / `/p2p/status` field:** the sampler can later feed `P2PStatusInfo` (the P-DIAGNOSTIC surface) so the attribution is operator-visible without log-diving. Out of scope here.

## 8. DISPATCH NOTE
- **Commit-only on the shift branch; the integrator (operator) pushes** (memory: commit-only). Task 6's deploy is the one authorized push ("fresh deployments if needed") — sole-dispatcher, surface first.
- **Selective-stage** each commit (shared worktree) — name exact files; never bulk-revert ambient mods.
- **Sequences BEFORE P-ARC** (its §B gate waits on this verdict); the shared `system_metrics.rs`/`process_manager.rs` edits are additive — flag the seam to P-ARC's integrator.
- **Runtime Rust never writes `.claude/data`** — the readers only read `/sys/fs/cgroup` + `/proc`. The verdict doc (Task 6) is authored by the operator/agent from Loki, not by the running node.
