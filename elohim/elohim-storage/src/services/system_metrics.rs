//! Per-node system metrics probes for the cluster_view local slice.
//!
//! Operational (Category C) per the p2p-design-gate output. None of these
//! values is authoritative on the DHT; they are observation snapshots.
//!
//! ## Boundary: per-node only
//!
//! This module exposes single-node primitives. Cross-node aggregations
//! (sum of CPU cores across a household's devices, total committed
//! storage across all my pods, etc.) are emergent properties of the
//! household-hub surface and live there, not here. See memory:
//! `project_node_metrics_vs_hub_aggregation_boundary`.
//!
//! ## Foundation
//!
//! - **Filesystem**: `fs4` crate (cross-platform statvfs/GetDiskFreeSpaceEx
//!   wrapper) — same primitive as `heartbeat::measure_free_pct`. Stays on
//!   one foundation per `feedback_check_existing_compute_foundation`.
//! - **Memory**: POSIX `sysinfo()` + `getrusage()` via `libc` — works on
//!   Linux, macOS, BSD without `/proc` parsing. Replaces earlier Linux-only
//!   `/proc/self/status` and `/proc/meminfo` readers.

use std::path::Path;

/// Sum the byte sizes of all regular files under `path`, recursively.
///
/// Returns `Ok(0)` when the path does not exist or is empty. Uses a
/// breadth-first walk; symlinks are followed once (no cycle protection
/// since the blob store is flat).
pub fn directory_size(path: &Path) -> std::io::Result<u64> {
    if !path.exists() {
        return Ok(0);
    }

    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            if meta.is_dir() {
                stack.push(entry.path());
            } else if meta.is_file() {
                total = total.saturating_add(meta.len());
            }
        }
    }

    Ok(total)
}

/// Read the filesystem capacity (total bytes) for the volume containing `path`.
///
/// Synthesizes the existing `fs4` cross-platform statvfs/GetDiskFreeSpaceEx
/// wrapper already in use at `heartbeat::measure_free_pct` — keeping all
/// filesystem capacity probes on a single foundation. Returns `None` on
/// syscall failure.
pub fn filesystem_capacity_bytes(path: &Path) -> Option<u64> {
    fs4::total_space(path).ok()
}

/// Read this process's resident set size in bytes via POSIX `getrusage`.
///
/// Cross-platform: works on Linux, macOS, and BSD without `/proc` parsing.
///
/// Note on units: `ru_maxrss` is in KB on Linux but in bytes on macOS.
/// We compile-gate the conversion so callers always get bytes back.
/// Returns `None` on syscall failure.
pub fn process_memory_bytes() -> Option<u64> {
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    // SAFETY: getrusage with RUSAGE_SELF and a valid &mut rusage is sound.
    let rc = unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) };
    if rc != 0 {
        return None;
    }
    let raw = usage.ru_maxrss as u64;
    // Linux: ru_maxrss is in KB. macOS/BSD: bytes.
    #[cfg(target_os = "linux")]
    let bytes = raw.saturating_mul(1024);
    #[cfg(not(target_os = "linux"))]
    let bytes = raw;
    Some(bytes)
}

/// Read total system memory in bytes via POSIX `sysinfo` (Linux) or
/// `sysctlbyname` (macOS/BSD via libc fallback).
///
/// Linux's `libc::sysinfo` is direct; on non-Linux POSIX this returns
/// `None` (callers should treat zero/unknown as "ignore" in aggregation).
/// Future expansion: replace the non-Linux branch with a `sysctlbyname`-
/// based reader when steward conductors target macOS.
pub fn total_memory_bytes() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let mut info: libc::sysinfo = unsafe { std::mem::zeroed() };
        // SAFETY: sysinfo with a valid &mut sysinfo is sound on Linux.
        let rc = unsafe { libc::sysinfo(&mut info) };
        if rc != 0 {
            return None;
        }
        // totalram is in units of mem_unit bytes.
        Some((info.totalram as u64).saturating_mul(info.mem_unit as u64))
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

/// Read the CONTAINER memory ceiling (cgroup limit) in bytes, or `None` when
/// there is no enforced limit / it can't be read.
///
/// `total_memory_bytes()` reports HOST RAM (`libc::sysinfo`), so a 512Mi pod
/// sees host GBs — useless for sizing anything to the *container* budget. This
/// is the memory analog of how `cpu_count()` already respects the cgroup CPU
/// quota. Reads cgroup v2 `/sys/fs/cgroup/memory.max` first (the literal `max`
/// = no limit), then falls back to cgroup v1
/// `/sys/fs/cgroup/memory/memory.limit_in_bytes` (a near-`u64::MAX` sentinel =
/// no limit). It is the hard precondition for the conductor authority-arc Auto
/// policy ([`super::arc_policy::derive`]): you cannot size an arc to a ceiling
/// you cannot read.
#[cfg(target_os = "linux")]
pub fn container_memory_limit_bytes() -> Option<u64> {
    // cgroup v2 (unified hierarchy).
    if let Ok(s) = std::fs::read_to_string("/sys/fs/cgroup/memory.max") {
        if let Some(v) = parse_cgroup_mem_limit(&s) {
            return Some(v);
        }
        // Present but "max" (no limit) — fall through to v1 in case of a hybrid mount.
    }
    // cgroup v1 (legacy hierarchy).
    if let Ok(s) = std::fs::read_to_string("/sys/fs/cgroup/memory/memory.limit_in_bytes") {
        if let Some(v) = parse_cgroup_mem_limit(&s) {
            return Some(v);
        }
    }
    None
}

#[cfg(not(target_os = "linux"))]
pub fn container_memory_limit_bytes() -> Option<u64> {
    None
}

/// Parse a cgroup memory-limit file body into an *enforced* byte limit, or
/// `None` when the value means "no limit". cgroup v2 writes the literal `max`;
/// cgroup v1 writes a near-`u64::MAX` sentinel (e.g. `0x7FFFFFFFFFFFF000`) when
/// unlimited. Anything at/above a sane ceiling (1 PiB) is treated as no limit.
/// Pure — unit-tested independently of the filesystem.
fn parse_cgroup_mem_limit(raw: &str) -> Option<u64> {
    let s = raw.trim();
    if s.is_empty() || s == "max" {
        return None;
    }
    let v: u64 = s.parse().ok()?;
    // 1 PiB — above this, treat as the v1 "unlimited" sentinel / absurd value.
    const SANE_CEILING_BYTES: u64 = 1 << 50;
    if v == 0 || v >= SANE_CEILING_BYTES {
        return None;
    }
    Some(v)
}

/// Count CPU cores available to this process.
///
/// Uses `std::thread::available_parallelism`, which respects cgroup CPU
/// quotas on Linux (so a pod with `--cpus=2` reports 2, not the host count).
/// Matches the precedent at `doorway/orchestrator/node_bootstrap.rs:144`
/// so per-node compute features stay on a single foundation. Returns
/// `None` when the platform refuses to answer.
pub fn cpu_count() -> Option<u32> {
    std::thread::available_parallelism()
        .ok()
        .map(|n| n.get() as u32)
}

/// Read POSIX 1/5/15-minute load averages.
///
/// `libc::getloadavg` is POSIX (Linux/macOS/BSD); returns `None` when the
/// syscall fails or the platform does not support it. Values are unitless
/// run-queue averages — interpreted relative to `cpu_count()`.
pub fn load_average() -> Option<(f64, f64, f64)> {
    let mut buf = [0.0f64; 3];
    // SAFETY: getloadavg writes up to `nelem` doubles into `loadavg`. Buffer
    // is sized for 3 elements; the call writes that many or returns -1.
    let n = unsafe { libc::getloadavg(buf.as_mut_ptr(), 3) };
    if n != 3 {
        return None;
    }
    Some((buf[0], buf[1], buf[2]))
}

// ─────────────────────────────────────────────────────────────────────────────
// Memory-attribution instrument (leak-vs-cache discriminator).
//
// The consolidated `elohim-node` container fuses the Holochain conductor child +
// the elohim-storage parent into ONE cgroup with ONE working-set number, so
// `container_memory_working_set_bytes` cannot attribute the OOM climb. These
// readers split it two ways:
//   1. cgroup `memory.stat` → anon / file / slab — the VERDICT (heap-leak vs page
//      cache). SQLite uses pread/pwrite (NOT mmap), so its cache lands in the
//      kernel page cache = the cgroup `file` counter, invisible to per-process
//      RssFile — which is exactly why this cgroup-level read is the primary tool.
//   2. per-process `/proc/<pid>/status` RssAnon — ATTRIBUTION (conductor vs
//      storage). Σ(per-proc RssAnon) ≈ cgroup `anon` confirms attribution.
// See plan: genesis/docs/superpowers/plans/2026-06-16-conductor-memory-attribution-instrument-plan.md
// ─────────────────────────────────────────────────────────────────────────────

/// cgroup `memory.stat` breakdown — the leak-vs-cache verdict, in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CgroupMemBreakdown {
    /// Anonymous (heap) memory. Unreclaimable when swap is off, so an OOM leans
    /// toward this being the driver — the heap-leak suspect.
    pub anon: u64,
    /// File-backed (kernel page cache) memory. SQLite pread/pwrite cache lands
    /// here — the page-cache suspect.
    pub file: u64,
    /// Kernel slab (dentry/inode/socket). Rarely the OOM driver; surfaced anyway.
    pub slab: u64,
}

/// Per-process RSS split from `/proc/<pid>/status`, in bytes (threads = count).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProcRss {
    pub rss_anon: u64,
    pub rss_file: u64,
    pub vm_rss: u64,
    pub threads: u64,
}

/// Parse a cgroup `memory.stat` body into the anon/file/slab breakdown. Pure —
/// unit-tested independently of the filesystem. Handles cgroup **v2** keys
/// (`anon`, `file`, `slab` or `slab_reclaimable`+`slab_unreclaimable`) and falls
/// back to cgroup **v1** vocabulary (`rss`→anon, `cache`→file). `memory.stat`
/// values are already in bytes (unlike `/proc/<pid>/status`, which is kB).
/// Returns `None` when neither an `anon` nor a v1 `rss` line is present.
pub fn parse_cgroup_memory_stat(raw: &str) -> Option<CgroupMemBreakdown> {
    let mut anon: Option<u64> = None;
    let mut file: Option<u64> = None;
    let mut slab: Option<u64> = None;
    let mut slab_recl: Option<u64> = None;
    let mut slab_unrecl: Option<u64> = None;
    for line in raw.lines() {
        let mut it = line.split_whitespace();
        let (Some(key), Some(val)) = (it.next(), it.next()) else {
            continue;
        };
        let Ok(v) = val.parse::<u64>() else {
            continue;
        };
        match key {
            "anon" => anon = Some(v),                    // cgroup v2
            "rss" if anon.is_none() => anon = Some(v),   // cgroup v1 fallback
            "file" => file = Some(v),                    // cgroup v2
            "cache" if file.is_none() => file = Some(v), // cgroup v1 fallback
            "slab" => slab = Some(v),                    // cgroup v2 explicit
            "slab_reclaimable" => slab_recl = Some(v),
            "slab_unreclaimable" => slab_unrecl = Some(v),
            _ => {}
        }
    }
    let anon = anon?;
    let file = file.unwrap_or(0);
    let slab = slab
        .or(match (slab_recl, slab_unrecl) {
            (Some(a), Some(b)) => Some(a.saturating_add(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        })
        .unwrap_or(0);
    Some(CgroupMemBreakdown { anon, file, slab })
}

/// Parse a `/proc/<pid>/status` body into the RSS split. Pure — unit-tested
/// independently of the filesystem. `RssAnon`/`RssFile`/`VmRSS` are in kB
/// (converted to bytes here); `Threads` is a count. Returns `None` when `VmRSS`
/// is absent (the process is gone or the body is malformed).
pub fn parse_proc_status(raw: &str) -> Option<ProcRss> {
    let mut rss_anon = 0u64;
    let mut rss_file = 0u64;
    let mut vm_rss: Option<u64> = None;
    let mut threads = 0u64;
    for line in raw.lines() {
        let Some((key, rest)) = line.split_once(':') else {
            continue;
        };
        // The value is the first whitespace-separated token after the colon.
        let val_str = rest.split_whitespace().next().unwrap_or("");
        let Ok(v) = val_str.parse::<u64>() else {
            continue;
        };
        match key.trim() {
            "RssAnon" => rss_anon = v.saturating_mul(1024),
            "RssFile" => rss_file = v.saturating_mul(1024),
            "VmRSS" => vm_rss = Some(v.saturating_mul(1024)),
            "Threads" => threads = v,
            _ => {}
        }
    }
    let vm_rss = vm_rss?;
    Some(ProcRss {
        rss_anon,
        rss_file,
        vm_rss,
        threads,
    })
}

/// Read the cgroup memory breakdown (anon/file/slab) — the leak-vs-cache verdict.
/// cgroup v2 `/sys/fs/cgroup/memory.stat` first, then the v1 hierarchy. `None`
/// when neither is readable / parseable.
#[cfg(target_os = "linux")]
pub fn cgroup_memory_breakdown() -> Option<CgroupMemBreakdown> {
    let raw = std::fs::read_to_string("/sys/fs/cgroup/memory.stat")
        .or_else(|_| std::fs::read_to_string("/sys/fs/cgroup/memory/memory.stat"))
        .ok()?;
    parse_cgroup_memory_stat(&raw)
}

#[cfg(not(target_os = "linux"))]
pub fn cgroup_memory_breakdown() -> Option<CgroupMemBreakdown> {
    None
}

/// Current swap charged to this cgroup, in bytes (cgroup v2 `memory.swap.current`).
/// With swap OFF this reads `0`, which is why an OOM leans toward `anon`
/// (unreclaimable) being the driver. `None` when the file is absent.
#[cfg(target_os = "linux")]
pub fn cgroup_swap_current_bytes() -> Option<u64> {
    std::fs::read_to_string("/sys/fs/cgroup/memory.swap.current")
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
}

#[cfg(not(target_os = "linux"))]
pub fn cgroup_swap_current_bytes() -> Option<u64> {
    None
}

/// Effective CPU quota in cores from cgroup v2 `/sys/fs/cgroup/cpu.max`
/// (`"<quota> <period>"`; `"max"` ⇒ unbounded ⇒ `None`). Logged at boot next to
/// the derived `db_max_readers` so the host-vs-cgroup pool-size question is
/// answerable from one line (RCA §4.4).
#[cfg(target_os = "linux")]
pub fn cgroup_cpu_quota_cores() -> Option<f64> {
    let s = std::fs::read_to_string("/sys/fs/cgroup/cpu.max").ok()?;
    let mut it = s.split_whitespace();
    let quota = it.next()?;
    let period = it.next()?;
    if quota == "max" {
        return None;
    }
    let q: f64 = quota.parse().ok()?;
    let p: f64 = period.parse().ok()?;
    if p > 0.0 {
        Some(q / p)
    } else {
        None
    }
}

#[cfg(not(target_os = "linux"))]
pub fn cgroup_cpu_quota_cores() -> Option<f64> {
    None
}

/// Read a process's RSS split from `/proc/<pid>/status` (Linux only). Used to
/// attribute the fused-cgroup working set to the conductor child vs the storage
/// parent. `None` on non-Linux or when the process is gone / unreadable.
pub fn proc_rss(pid: u32) -> Option<ProcRss> {
    #[cfg(target_os = "linux")]
    {
        let raw = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
        parse_proc_status(&raw)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        None
    }
}

/// Anonymous-memory breakdown by mapping class from `/proc/<pid>/smaps` — the
/// heap-leak ATTRIBUTION instrument for a process we do NOT compile (the
/// Holochain conductor child). [`proc_rss`] says HOW MUCH anon; this says WHERE
/// in the address space it lives — a growing `[heap]` vs proliferating malloc
/// arenas (unnamed anon mmaps) vs thread stacks — localizing the leak without
/// recompiling the conductor.
///
/// Limitation: modern Linux does NOT label thread stacks `[stack]` (only the
/// main thread is), so thread stacks fall under `other_anon_bytes`. With a flat
/// thread count the growth still cleanly separates heap-growth from arena-growth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SmapsAnonBreakdown {
    /// Anonymous bytes in the `[heap]` mapping (brk-grown heap).
    pub heap_bytes: u64,
    /// Anonymous bytes in `[stack]` mappings (main-thread stack on modern kernels).
    pub stack_bytes: u64,
    /// Anonymous bytes in unnamed anon mappings (malloc/jemalloc arenas, large
    /// allocations, thread stacks) — the leak's usual home.
    pub other_anon_bytes: u64,
    /// Count of mappings contributing anon (arena-proliferation signal).
    pub anon_mapping_count: u64,
    /// Largest single mapping's anon bytes (a lone growing mapping = the leak locus).
    pub largest_anon_bytes: u64,
}

#[derive(Clone, Copy)]
enum MappingClass {
    Heap,
    Stack,
    OtherAnon,
    File,
}

/// True if `line` is a smaps mapping header (`<hex>-<hex> perms …`), as opposed
/// to a `Key: value` detail line.
fn is_smaps_header(line: &str) -> bool {
    match line.split_whitespace().next() {
        Some(tok) => match tok.split_once('-') {
            Some((a, b)) => {
                !a.is_empty()
                    && !b.is_empty()
                    && a.bytes().all(|c| c.is_ascii_hexdigit())
                    && b.bytes().all(|c| c.is_ascii_hexdigit())
            }
            None => false,
        },
        None => false,
    }
}

/// Classify a smaps mapping header by its trailing pathname field (index 5):
/// `[heap]`, `[stack…]`, `[anon…]`/none → anon, else a file/special path.
fn classify_mapping(header: &str) -> MappingClass {
    match header.split_whitespace().nth(5) {
        None => MappingClass::OtherAnon, // anonymous mapping (no pathname)
        Some("[heap]") => MappingClass::Heap,
        Some(n) if n.starts_with("[stack") => MappingClass::Stack,
        Some(n) if n.starts_with("[anon") => MappingClass::OtherAnon,
        Some(_) => MappingClass::File,
    }
}

/// Anon class of a single mapping (the public face of [`MappingClass`], minus
/// `File` — file-backed mappings are never reported as anon VMAs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnonClass {
    Heap,
    Stack,
    OtherAnon,
}

impl AnonClass {
    /// The Loki/Prometheus label form (matches `set_conductor_smaps`' `class`).
    pub fn as_str(self) -> &'static str {
        match self {
            AnonClass::Heap => "heap",
            AnonClass::Stack => "stack",
            AnonClass::OtherAnon => "other",
        }
    }
}

/// One anonymous-bearing mapping, with the fields needed to LOCALIZE a leak that
/// grows at flat mapping-count/flat-largest (the 2026-06-18 alpha conductor
/// signature): the `start_addr` is the cross-sample diff key (which exact ranges
/// grow), `anon_bytes` the resident anon in it, `neighbor` the nearest preceding
/// file mapping (a hint at what allocator/runtime region this sits next to).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnonVma {
    /// Mapping start address (hex header) — the stable key for cross-tick deltas.
    pub start_addr: u64,
    /// Virtual size of the mapping (header end-start) in bytes.
    pub size_bytes: u64,
    /// Resident anonymous bytes in this mapping (`Anonymous:` line).
    pub anon_bytes: u64,
    /// Mapping class (heap / stack / other-anon).
    pub class: AnonClass,
    /// Basename of the nearest preceding file mapping (localization hint; "" if none).
    pub neighbor: String,
}

/// Per-mapping anon-size buckets (by resident anon bytes). The SHAPE discriminator:
/// Go heap arenas, thread stacks, and small per-op buffers land in distinct bands,
/// so a shift in the histogram localizes a leak the scalar gauges can't.
pub const ANON_SIZE_BUCKETS: &[(&str, u64)] = &[
    ("0-64k", 64 * 1024),
    ("64k-1m", 1024 * 1024),
    ("1m-8m", 8 * 1024 * 1024),
    ("8m-64m", 64 * 1024 * 1024),
    ("64m-256m", 256 * 1024 * 1024),
    ("256m+", u64::MAX),
];

/// A VMA's growth vs a prior snapshot — the leak-localization signal (WHICH
/// address ranges accumulate resident anon between samples).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmaDelta {
    pub vma: AnonVma,
    /// Resident-anon change since the prior sample (bytes; positive = grew).
    pub delta_bytes: i64,
    /// True if this mapping did not exist at the prior sample.
    pub is_new: bool,
}

/// Parse the `<hex>-<hex>` range from a smaps header → (start, size).
fn parse_header_range(header: &str) -> (u64, u64) {
    let tok = header.split_whitespace().next().unwrap_or("");
    match tok.split_once('-') {
        Some((a, b)) => {
            let start = u64::from_str_radix(a, 16).unwrap_or(0);
            let end = u64::from_str_radix(b, 16).unwrap_or(start);
            (start, end.saturating_sub(start))
        }
        None => (0, 0),
    }
}

/// Basename of a smaps header's pathname field (index 5), or `None` for special
/// `[…]` regions and anonymous mappings.
fn header_pathname_basename(header: &str) -> Option<String> {
    let p = header.split_whitespace().nth(5)?;
    if p.starts_with('[') {
        return None;
    }
    Some(p.rsplit('/').next().unwrap_or(p).to_string())
}

/// Parse a `/proc/<pid>/smaps` body into the list of anon-bearing mappings. Pure,
/// unit-tested. This is the single parse path; [`parse_smaps_anon`] folds over it.
pub fn parse_smaps_vmas(raw: &str) -> Vec<AnonVma> {
    let mut out = Vec::new();
    let mut cur_class = MappingClass::File;
    let mut cur_start = 0u64;
    let mut cur_size = 0u64;
    let mut neighbor = String::new();
    for line in raw.lines() {
        if is_smaps_header(line) {
            cur_class = classify_mapping(line);
            let (start, size) = parse_header_range(line);
            cur_start = start;
            cur_size = size;
            if let MappingClass::File = cur_class {
                if let Some(name) = header_pathname_basename(line) {
                    neighbor = name;
                }
            }
            continue;
        }
        let Some(rest) = line.strip_prefix("Anonymous:") else {
            continue;
        };
        let kb = rest
            .trim()
            .trim_end_matches("kB")
            .trim()
            .parse::<u64>()
            .unwrap_or(0);
        if kb == 0 {
            continue;
        }
        let class = match cur_class {
            MappingClass::Heap => AnonClass::Heap,
            MappingClass::Stack => AnonClass::Stack,
            MappingClass::OtherAnon => AnonClass::OtherAnon,
            MappingClass::File => continue, // CoW anon in a file mapping — not our attribution target
        };
        out.push(AnonVma {
            start_addr: cur_start,
            size_bytes: cur_size,
            anon_bytes: kb.saturating_mul(1024),
            class,
            neighbor: neighbor.clone(),
        });
    }
    out
}

impl SmapsAnonBreakdown {
    /// Fold a parsed VMA list into the scalar breakdown (the existing gauges).
    pub fn from_vmas(vmas: &[AnonVma]) -> Option<Self> {
        if vmas.is_empty() {
            return None;
        }
        let mut b = SmapsAnonBreakdown::default();
        for v in vmas {
            match v.class {
                AnonClass::Heap => b.heap_bytes = b.heap_bytes.saturating_add(v.anon_bytes),
                AnonClass::Stack => b.stack_bytes = b.stack_bytes.saturating_add(v.anon_bytes),
                AnonClass::OtherAnon => {
                    b.other_anon_bytes = b.other_anon_bytes.saturating_add(v.anon_bytes)
                }
            }
            b.anon_mapping_count += 1;
            if v.anon_bytes > b.largest_anon_bytes {
                b.largest_anon_bytes = v.anon_bytes;
            }
        }
        Some(b)
    }
}

/// Parse a `/proc/<pid>/smaps` body into the anon breakdown. Pure — unit-tested
/// independently of the filesystem. `None` when no anon is found.
pub fn parse_smaps_anon(raw: &str) -> Option<SmapsAnonBreakdown> {
    SmapsAnonBreakdown::from_vmas(&parse_smaps_vmas(raw))
}

/// Bucket anon VMAs by resident-anon size → `(label, count, total_anon_bytes)`
/// per [`ANON_SIZE_BUCKETS`] band. Pure.
pub fn anon_size_histogram(vmas: &[AnonVma]) -> Vec<(&'static str, u64, u64)> {
    let mut acc = vec![(0u64, 0u64); ANON_SIZE_BUCKETS.len()];
    for v in vmas {
        let idx = ANON_SIZE_BUCKETS
            .iter()
            .position(|(_, hi)| v.anon_bytes <= *hi)
            .unwrap_or(ANON_SIZE_BUCKETS.len() - 1);
        acc[idx].0 += 1;
        acc[idx].1 = acc[idx].1.saturating_add(v.anon_bytes);
    }
    ANON_SIZE_BUCKETS
        .iter()
        .enumerate()
        .map(|(i, (label, _))| (*label, acc[i].0, acc[i].1))
        .collect()
}

/// Snapshot anon-bytes keyed by mapping start address (the prior-sample state
/// [`top_growing_vmas`] diffs against).
pub fn snapshot_anon_by_addr(vmas: &[AnonVma]) -> std::collections::HashMap<u64, u64> {
    vmas.iter().map(|v| (v.start_addr, v.anon_bytes)).collect()
}

/// The top-`n` anon VMAs by resident-anon GROWTH since `prev` (and any that are
/// brand-new since `prev`). This is the leak localizer: at flat mapping-count /
/// flat-largest, the leak is resident growth spread across a stable population —
/// these are the exact ranges accumulating it. Pure.
pub fn top_growing_vmas(
    prev: &std::collections::HashMap<u64, u64>,
    cur: &[AnonVma],
    n: usize,
) -> Vec<VmaDelta> {
    let mut deltas: Vec<VmaDelta> = cur
        .iter()
        .filter_map(|v| match prev.get(&v.start_addr) {
            Some(&p) => {
                let d = v.anon_bytes as i64 - p as i64;
                (d > 0).then(|| VmaDelta {
                    vma: v.clone(),
                    delta_bytes: d,
                    is_new: false,
                })
            }
            None => Some(VmaDelta {
                vma: v.clone(),
                delta_bytes: v.anon_bytes as i64,
                is_new: true,
            }),
        })
        .collect();
    deltas.sort_by_key(|b| std::cmp::Reverse(b.delta_bytes));
    deltas.truncate(n);
    deltas
}

/// Compact one-line rendering of VMA deltas for a Loki log field, e.g.
/// `0x7f12+12.3MB(other,sz150MB,near=holochain,NEW)`.
pub fn fmt_vma_deltas(deltas: &[VmaDelta]) -> String {
    deltas
        .iter()
        .map(|d| {
            format!(
                "0x{:x}+{:.1}MB({},sz{:.0}MB{}{})",
                d.vma.start_addr,
                (d.delta_bytes as f64) / 1_048_576.0,
                d.vma.class.as_str(),
                (d.vma.size_bytes as f64) / 1_048_576.0,
                if d.vma.neighbor.is_empty() {
                    String::new()
                } else {
                    format!(",near={}", d.vma.neighbor)
                },
                if d.is_new { ",NEW" } else { "" },
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Read a process's smaps anon breakdown (Linux only; the heap-leak attribution
/// reader). `None` on non-Linux or when the process is gone / unreadable.
pub fn proc_smaps_anon(pid: u32) -> Option<SmapsAnonBreakdown> {
    #[cfg(target_os = "linux")]
    {
        let raw = std::fs::read_to_string(format!("/proc/{pid}/smaps")).ok()?;
        parse_smaps_anon(&raw)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        None
    }
}

/// Read a process's full anon-VMA list (Linux only; the leak-localization reader).
/// One smaps read feeds both the scalar breakdown ([`SmapsAnonBreakdown::from_vmas`])
/// and the histogram / growth-delta localizers. Empty on non-Linux or if unreadable.
pub fn proc_smaps_vmas(pid: u32) -> Vec<AnonVma> {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string(format!("/proc/{pid}/smaps"))
            .map(|raw| parse_smaps_vmas(&raw))
            .unwrap_or_default()
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn directory_size_returns_zero_for_empty_dir() {
        let tmp = TempDir::new().unwrap();
        assert_eq!(directory_size(tmp.path()).unwrap(), 0);
    }

    #[test]
    fn directory_size_sums_file_bytes() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("a"), b"hello").unwrap(); // 5 bytes
        fs::write(tmp.path().join("b"), b"world!!!").unwrap(); // 8 bytes
        assert_eq!(directory_size(tmp.path()).unwrap(), 13);
    }

    #[test]
    fn directory_size_recurses_into_subdirs() {
        let tmp = TempDir::new().unwrap();
        let sub = tmp.path().join("sub");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("a"), b"hello").unwrap();
        assert_eq!(directory_size(tmp.path()).unwrap(), 5);
    }

    #[test]
    fn directory_size_returns_zero_for_nonexistent_path() {
        assert_eq!(
            directory_size("/nonexistent/path/here".as_ref()).unwrap_or(0),
            0
        );
    }

    #[test]
    fn filesystem_capacity_bytes_returns_some_for_root_or_tmp() {
        // The volume containing /tmp must exist on any test host.
        assert!(filesystem_capacity_bytes("/tmp".as_ref()).unwrap_or(0) > 0);
    }

    #[test]
    fn process_memory_returns_nonzero_when_we_are_running() {
        // We are running, so getrusage(RUSAGE_SELF).ru_maxrss must be > 0
        // on any POSIX host.
        assert!(process_memory_bytes().unwrap_or(0) > 0);
    }

    #[test]
    fn total_memory_returns_nonzero_on_linux() {
        if cfg!(target_os = "linux") {
            assert!(total_memory_bytes().unwrap_or(0) > 0);
        }
    }

    #[test]
    fn cpu_count_returns_nonzero_on_real_host() {
        // Any test host has at least one core.
        assert!(cpu_count().unwrap_or(0) >= 1);
    }

    #[test]
    fn load_average_returns_three_finite_values() {
        // POSIX hosts (linux/macOS) return 3 doubles; CI runners always do.
        if let Some((one, five, fifteen)) = load_average() {
            assert!(one.is_finite() && one >= 0.0);
            assert!(five.is_finite() && five >= 0.0);
            assert!(fifteen.is_finite() && fifteen >= 0.0);
        }
    }

    #[test]
    fn parse_cgroup_mem_limit_cases() {
        // cgroup v2 unlimited.
        assert_eq!(parse_cgroup_mem_limit("max\n"), None);
        // Real enforced limits.
        assert_eq!(parse_cgroup_mem_limit("8589934592\n"), Some(8_589_934_592)); // 8Gi
        assert_eq!(parse_cgroup_mem_limit("805306368"), Some(805_306_368)); // 768Mi
                                                                            // cgroup v1 "unlimited" sentinel (≈ 2^63) → no limit.
        assert_eq!(parse_cgroup_mem_limit("9223372036854771712"), None);
        // Degenerate / unparseable.
        assert_eq!(parse_cgroup_mem_limit(""), None);
        assert_eq!(parse_cgroup_mem_limit("0"), None);
        assert_eq!(parse_cgroup_mem_limit("garbage"), None);
        // Just under the 1 PiB sanity ceiling is honored; at/over is not.
        assert_eq!(
            parse_cgroup_mem_limit(&((1u64 << 50) - 1).to_string()),
            Some((1 << 50) - 1)
        );
        assert_eq!(parse_cgroup_mem_limit(&(1u64 << 50).to_string()), None);
    }

    #[test]
    fn container_memory_limit_is_optional_and_sane() {
        // Environment-dependent: in a limited cgroup it is Some(>0); on a host
        // with no limit it is None. Either way it must never be Some(0) and must
        // never exceed the 1 PiB sanity ceiling.
        if let Some(v) = container_memory_limit_bytes() {
            assert!(v > 0 && v < (1u64 << 50));
        }
    }

    #[test]
    fn parse_cgroup_memory_stat_v2_and_v1() {
        // cgroup v2 shape: slab from reclaimable + unreclaimable when no `slab` key.
        let v2 =
            "anon 6442450944\nfile 1610612736\nkernel_stack 1000\nslab_reclaimable 50\nslab_unreclaimable 150\nsock 0\n";
        let b = parse_cgroup_memory_stat(v2).expect("v2 parses");
        assert_eq!(b.anon, 6_442_450_944);
        assert_eq!(b.file, 1_610_612_736);
        assert_eq!(
            b.slab, 200,
            "slab = reclaimable + unreclaimable when no `slab` key"
        );
        // An explicit v2 `slab` key wins over the reclaimable split.
        let v2s = "anon 1\nfile 2\nslab 999\nslab_reclaimable 1\n";
        assert_eq!(parse_cgroup_memory_stat(v2s).unwrap().slab, 999);
        // cgroup v1 vocabulary fallback: rss -> anon, cache -> file.
        let v1 = "cache 3221225472\nrss 2147483648\nrss_huge 0\n";
        let b1 = parse_cgroup_memory_stat(v1).expect("v1 parses");
        assert_eq!(b1.anon, 2_147_483_648, "v1 rss -> anon");
        assert_eq!(b1.file, 3_221_225_472, "v1 cache -> file");
        assert_eq!(b1.slab, 0, "no slab lines -> 0");
        // Garbage / empty -> None (no anon/rss key).
        assert_eq!(parse_cgroup_memory_stat(""), None);
        assert_eq!(parse_cgroup_memory_stat("totally unrelated\n"), None);
        assert_eq!(
            parse_cgroup_memory_stat("file 5\n"),
            None,
            "file without anon -> None"
        );
    }

    #[test]
    fn parse_proc_status_extracts_rss_split_kb_to_bytes() {
        let s = "Name:\tholochain\nThreads:\t42\nVmRSS:\t  8388608 kB\nRssAnon:\t  6291456 kB\nRssFile:\t  2097152 kB\nVmSwap:\t 0 kB\n";
        let r = parse_proc_status(s).expect("parses");
        assert_eq!(r.vm_rss, 8_388_608 * 1024);
        assert_eq!(r.rss_anon, 6_291_456 * 1024);
        assert_eq!(r.rss_file, 2_097_152 * 1024);
        assert_eq!(r.threads, 42);
        // No VmRSS line -> None (process gone / malformed).
        assert_eq!(parse_proc_status("Name:\tx\nThreads:\t1\n"), None);
    }

    #[test]
    fn proc_rss_self_is_some_on_linux() {
        // We are running, so our own /proc/self status must read with VmRSS > 0.
        if cfg!(target_os = "linux") {
            let r = proc_rss(std::process::id()).expect("self proc status readable on linux");
            assert!(r.vm_rss > 0, "running process must have VmRSS > 0");
        }
    }

    #[test]
    fn cgroup_memory_breakdown_is_sane_if_present() {
        // Environment-dependent: Some in a cgroup, None on a bare host. When Some,
        // anon+file must be below the 1 PiB sanity ceiling.
        if let Some(b) = cgroup_memory_breakdown() {
            assert!(b.anon.saturating_add(b.file) < (1u64 << 50));
        }
    }

    #[test]
    fn parse_smaps_anon_buckets_by_mapping_class() {
        let smaps = "\
561e1a000-561e1b000 rw-p 00000000 00:00 0                                  [heap]
Size:                  4 kB
Rss:                   4 kB
Anonymous:             4 kB
7ffd00000-7ffd21000 rw-p 00000000 00:00 0                                  [stack]
Anonymous:           132 kB
7f0000000000-7f0000800000 rw-p 00000000 00:00 0
Anonymous:          2048 kB
7f0001000000-7f0001100000 rw-p 00000000 00:00 0
Anonymous:           256 kB
7f5500000000-7f5500001000 r-xp 00000000 08:01 1234   /usr/lib/libc.so.6
Anonymous:             0 kB
";
        let b = parse_smaps_anon(smaps).expect("has anon");
        assert_eq!(b.heap_bytes, 4 * 1024);
        assert_eq!(b.stack_bytes, 132 * 1024);
        assert_eq!(
            b.other_anon_bytes,
            (2048 + 256) * 1024,
            "two unnamed anon mmaps sum into other"
        );
        assert_eq!(
            b.anon_mapping_count, 4,
            "heap + stack + 2 anon; the 0-anon file mapping is excluded"
        );
        assert_eq!(
            b.largest_anon_bytes,
            2048 * 1024,
            "biggest single anon mapping = the leak locus"
        );
        // A mapping with no Anonymous line at all -> None.
        assert_eq!(
            parse_smaps_anon("561e-561f rw-p 0 0 0\nSize:  4 kB\n"),
            None
        );
    }

    const SMAPS_FIXTURE: &str = "\
561e1a000-561e1b000 rw-p 00000000 00:00 0                                  [heap]
Anonymous:             4 kB
7f5500000000-7f5500010000 r-xp 00000000 08:01 1234   /usr/local/bin/holochain
Anonymous:             0 kB
7f0000000000-7f0000800000 rw-p 00000000 00:00 0
Anonymous:          2048 kB
7f0001000000-7f0001100000 rw-p 00000000 00:00 0
Anonymous:           256 kB
";

    #[test]
    fn parse_smaps_vmas_captures_addr_size_neighbor_and_class() {
        let vmas = parse_smaps_vmas(SMAPS_FIXTURE);
        assert_eq!(
            vmas.len(),
            3,
            "heap + 2 anon; the 0-anon file mapping is skipped"
        );
        // [heap]
        assert_eq!(vmas[0].start_addr, 0x561e1a000);
        assert_eq!(vmas[0].size_bytes, 0x1000);
        assert_eq!(vmas[0].anon_bytes, 4 * 1024);
        assert_eq!(vmas[0].class, AnonClass::Heap);
        assert_eq!(vmas[0].neighbor, "", "no file mapping precedes the heap");
        // First unnamed anon — sits AFTER the holochain file mapping, so neighbor carries it.
        assert_eq!(vmas[1].start_addr, 0x7f0000000000);
        assert_eq!(vmas[1].size_bytes, 0x800000);
        assert_eq!(vmas[1].anon_bytes, 2048 * 1024);
        assert_eq!(vmas[1].class, AnonClass::OtherAnon);
        assert_eq!(
            vmas[1].neighbor, "holochain",
            "neighbor = basename of the nearest preceding file mapping"
        );
        // Breakdown derived from the same VMAs matches the legacy parse.
        let b = SmapsAnonBreakdown::from_vmas(&vmas).unwrap();
        assert_eq!(b, parse_smaps_anon(SMAPS_FIXTURE).unwrap());
        assert_eq!(b.other_anon_bytes, (2048 + 256) * 1024);
        assert_eq!(b.largest_anon_bytes, 2048 * 1024);
    }

    #[test]
    fn anon_size_histogram_bins_by_resident_anon() {
        let vmas = parse_smaps_vmas(SMAPS_FIXTURE);
        let hist = anon_size_histogram(&vmas);
        // bucket layout: 0-64k, 64k-1m, 1m-8m, 8m-64m, 64m-256m, 256m+
        let by = |label: &str| hist.iter().find(|(l, _, _)| *l == label).copied().unwrap();
        assert_eq!(by("0-64k").1, 1, "the 4kB heap lands in 0-64k");
        assert_eq!(by("0-64k").2, 4 * 1024);
        assert_eq!(by("64k-1m").1, 1, "the 256kB anon lands in 64k-1m");
        assert_eq!(by("1m-8m").1, 1, "the 2MB anon lands in 1m-8m");
        assert_eq!(by("8m-64m").1, 0);
        // Total count across buckets == anon mapping count.
        assert_eq!(hist.iter().map(|(_, c, _)| *c).sum::<u64>(), 3);
    }

    #[test]
    fn top_growing_vmas_ranks_growth_and_flags_new() {
        let prev: std::collections::HashMap<u64, u64> =
            [(0x1000, 100), (0x2000, 5_000)].into_iter().collect();
        let cur = vec![
            AnonVma {
                start_addr: 0x1000,
                size_bytes: 0x1000,
                anon_bytes: 1_100,
                class: AnonClass::OtherAnon,
                neighbor: String::new(),
            }, // +1000
            AnonVma {
                start_addr: 0x2000,
                size_bytes: 0x1000,
                anon_bytes: 5_000,
                class: AnonClass::OtherAnon,
                neighbor: String::new(),
            }, // +0 -> dropped
            AnonVma {
                start_addr: 0x3000,
                size_bytes: 0x1000,
                anon_bytes: 9_000,
                class: AnonClass::Heap,
                neighbor: "libc.so.6".into(),
            }, // NEW
        ];
        let top = top_growing_vmas(&prev, &cur, 8);
        assert_eq!(top.len(), 2, "the unchanged VMA is excluded (delta == 0)");
        assert_eq!(
            top[0].vma.start_addr, 0x3000,
            "new VMA's full size ranks first"
        );
        assert!(top[0].is_new);
        assert_eq!(top[0].delta_bytes, 9_000);
        assert_eq!(top[1].vma.start_addr, 0x1000);
        assert!(!top[1].is_new);
        assert_eq!(top[1].delta_bytes, 1_000);
        // Truncation honored.
        assert_eq!(top_growing_vmas(&prev, &cur, 1).len(), 1);
        let rendered = fmt_vma_deltas(&top);
        assert!(rendered.contains("0x3000+"));
        assert!(rendered.contains("NEW"));
        assert!(rendered.contains("near=libc.so.6"));
    }

    #[test]
    fn proc_smaps_anon_self_is_some_on_linux() {
        if cfg!(target_os = "linux") {
            // We are running, so our own smaps has at least a [heap] with anon.
            let b = proc_smaps_anon(std::process::id()).expect("self smaps readable on linux");
            assert!(
                b.heap_bytes + b.other_anon_bytes > 0,
                "a running process has resident anon"
            );
        }
    }
}
