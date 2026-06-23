# Auto-Config Resource Probe — Implementation Plan
> For agentic workers: REQUIRED SUB-SKILL: superpowers:subagent-driven-development (or executing-plans). Steps use checkbox (- [ ]) syntax.

Goal: Read the node's real cgroup CPU/memory ceilings at boot, derive a principled tokio worker-thread count (floored at 4, scaled by CPU quota — replacing the hardcoded 4), wire it into doorway's runtime, and expose detected resources + derived config + human "why" reasons at `GET /admin/auto-preset` — shipping the resource-probe + Auto-derive engine + cgroup **memory reader** that the warm-up-budget freeze cure and the arc-shrink memory fix both depend on. It removes the single-worker failure mode as defense-in-depth, but is NOT itself the freeze cure (the corrected diagnosis is serial warm-up churn against broken upstreams — a named follow-on); on the `cpu:1` doorway pod this derives the same 4 it is hardcoded to today (safe, no prod behavior change there) and scales up on bigger hosts.

Architecture: A new dep-free module `elohim-compute/src/limits.rs` (shared crate both doorway and storage already depend on by path) holds three layers: pure string parsers for cgroup v2/v1 CPU+MEM files (unit-tested with live fixtures), a thin `std::fs` wrapper that resolves the process leaf cgroup via `/proc/self/cgroup`, and a pure `derive(snapshot, overrides) -> DerivedConfig` function with precedence operator-override > Auto-derived > safe-floor. **p2p-class: Category C node-local read-model** — `/admin/auto-preset` is a fresh projection of the live container's cgroup, NOT a DHT entry, NOT a database table, NO coordinator function, NO runtime mutation surface. Detect and derive run at BOOT only; there is no re-derivation on cgroup change in this plan.

Tech Stack: Rust, tokio, hyper (doorway), cgroup v1/v2 sysfs.

---

## Naming deviation from task scope (READ FIRST)

The task scope names the detected-resources type `ResourceSnapshot`. **That name is already taken in `elohim-compute`**: `resources.rs:10` defines an unrelated runtime-telemetry `ResourceSnapshot` (timestamp/requests/active_connections/...), re-exported at `lib.rs:21`. Reusing the name would shadow/collide. **This plan uses `DetectedResources`** for the cgroup-ceiling type (reads naturally as `detect_resources() -> DetectedResources`). It lives in the new `limits.rs` and is **NOT** added to the `lib.rs` root re-export (adding it there would reintroduce the collision). The self-review consistency check at the end applies to `DetectedResources` (the substitute), not `ResourceSnapshot`.

Two other scope fields cannot be filled from inside the dep-free shared crate, so `detect_resources` takes them as injected arguments (caller supplies; both are EXPOSED-only, never consumed in this plan):
- `disk_free_bytes` needs `fs4`, which is NOT an `elohim-compute` dependency. Caller injects `Option<u64>` (doorway passes `None` for now — a real free-space probe is a named follow-on).
- `observed_peer_count` is irreducible runtime state (doorway `AppState` / storage swarm), not derivable in a shared crate. Caller injects `Option<usize>` (doorway passes `None` for now — a real peer-count probe is a named follow-on).

So the signature is `detect_resources(disk_free_bytes: Option<u64>, observed_peer_count: Option<usize>) -> DetectedResources`. The function itself only reads cgroup CPU + MEM.

---

## Canonical type & function names (must match across ALL tasks)

| Symbol | Kind | Home |
|--------|------|------|
| `read_cpu_quota_str(cpu_max: &str) -> Option<f64>` | pure parser (v2) | `limits.rs` |
| `read_cpu_quota_v1_str(quota: &str, period: &str) -> Option<f64>` | pure parser (v1) | `limits.rs` |
| `read_mem_limit_str(mem_max: &str) -> Option<u64>` | pure parser (v2) | `limits.rs` |
| `read_mem_limit_v1_str(limit_in_bytes: &str) -> Option<u64>` | pure parser (v1) | `limits.rs` |
| `read_cpu_quota() -> Option<f64>` | thin fs wrapper | `limits.rs` |
| `read_mem_limit() -> Option<u64>` | thin fs wrapper | `limits.rs` |
| `DetectedResources { cpu_quota: Option<f64>, mem_limit_bytes: Option<u64>, disk_free_bytes: Option<u64>, observed_peer_count: Option<usize> }` | struct | `limits.rs` |
| `detect_resources(disk_free_bytes: Option<u64>, observed_peer_count: Option<usize>) -> DetectedResources` | fn | `limits.rs` |
| `DeriveOverrides { worker_threads: Option<usize> }` | struct | `limits.rs` |
| `DerivedConfig { worker_threads: usize, reasons: Vec<String> }` | struct | `limits.rs` |
| `derive(snapshot: &DetectedResources, overrides: &DeriveOverrides) -> DerivedConfig` | pure fn | `limits.rs` |
| `AutoPresetView { resources, derived, overrides, reasons }` | wire/serialize view | `limits.rs` |

Constants in `limits.rs`: `WORKER_THREADS_FLOOR: usize = 4`, `DEFAULT_CPU_MULTIPLIER_K: f64 = 1.0`.

---

## File Structure

| File | Create/Modify | Responsibility |
|------|---------------|----------------|
| `elohim/elohim-compute/src/limits.rs` | **Create** | Cgroup CPU/MEM readers (pure parsers + leaf-resolving fs wrappers), `DetectedResources` + `detect_resources()`, `DeriveOverrides`, `DerivedConfig`, pure `derive()`, `AutoPresetView`. All unit tests inline `#[cfg(test)]`. |
| `elohim/elohim-compute/src/lib.rs` | **Modify** | Declare `pub mod limits;`. Re-export `derive`, `detect_resources`, `DetectedResources`, `DeriveOverrides`, `DerivedConfig`, `AutoPresetView` selectively (NOT under names that collide with existing `resources` exports). |
| `doorway/doorway-service/src/main.rs` | **Modify** | Replace the worker-thread const fallback with `derive()` at boot (env override preserved); rewrite the three "never CPU-derived" prose sites into the floor-4-then-scale scar; build & stash the boot `DerivedConfig` for the admin route. |
| `doorway/doorway-service/src/server/state.rs` (or wherever `AppState` is defined) | **Modify** | Add an `auto_preset: AutoPresetView` field so the boot-time projection is servable by the read-only handler. |
| `doorway/doorway-service/src/routes/admin.rs` | **Modify** | Add `pub async fn handle_admin_auto_preset(state: Arc<AppState>) -> Response<Full<Bytes>>` returning `json_response(StatusCode::OK, &state.auto_preset)`. |
| `doorway/doorway-service/src/server/http.rs` | **Modify** | Add `(Method::GET, "/admin/auto-preset")` match arm after the `/admin/render-stats` arm (line ~2339). |
| `elohim/elohim-storage/src/main.rs` | **Modify** (item 6 decision) | Wire `import_rt` worker threads to `derive()` (floor 4 preserved); leave `server_rt=2` with a comment. Serving `/admin/auto-preset` from storage is named as a follow-on (storage HTTP surface differs). |

### Build/test commands (verified — use these EXACTLY)

- **elohim-compute** (all parser/derive/snapshot tests — VERIFIED green, 38 passed):
  ```
  cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo test --lib <filter>
  ```
- **doorway-service** (VERIFIED):
  ```
  cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-check RUSTC_WRAPPER="" cargo test --lib <filter>
  ```
- **elohim-storage** (VERIFIED — note the custom getrandom flag; it is a LINK-time symbol):
  ```
  cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo test --lib <filter>
  ```

Notes that are non-negotiable for the executor:
- **NO `cargo nextest`** — it is NOT installed in this container despite the gospel CLAUDE.md claim. Plain `cargo test`.
- **Never `&&`-pipe a gate command** whose exit code matters through another pipe; read `${PIPESTATUS[0]}` if you must pipe to `tail`.
- **Do NOT build into the cargo pool slot** (`/projects/.cargo-target-pool/...`) — it fails with a fingerprint `invoked.timestamp ... No such file or directory` ENOENT in this container. Always use the `/tmp/...` target dirs shown above.
- Disk is at the soft watermark (~78%); `/tmp` targets keep the footprint off `/projects`.

---

## TASK 1 — `limits.rs` skeleton + module wiring (compiles, no logic yet)

Establishes the module so every later test step has a home and a green baseline.

Files:
- Create: `elohim/elohim-compute/src/limits.rs`
- Modify: `elohim/elohim-compute/src/lib.rs` (line 13 area `pub mod resources;`; line 21 area re-exports)

Steps:
- [ ] Create `elohim/elohim-compute/src/limits.rs` with the module doc + constants + a trivial test so the file is non-empty:
  ```rust
  //! Cat C node-local resource probe + Auto-preset derivation.
  //!
  //! Reads the live container's cgroup CPU/MEM ceilings (v2 leaf first, v1
  //! fallback) and derives a safe boot-time runtime config. Pure parsers are
  //! unit-tested against real fixture strings; the thin fs wrappers resolve the
  //! process leaf cgroup via /proc/self/cgroup. This is a FRESH PROJECTION of
  //! the running container — no DHT entry, no DB table, no coordinator fn.
  //!
  //! detect/derive run at BOOT only; this module never re-derives on cgroup change.

  use serde::Serialize;

  /// Tokio worker-thread floor. A cgroup of `cpu: 1` made the default runtime
  /// spin ONE worker; one synchronously-blocked await on it froze the whole
  /// gateway (2026-06-13). Four workers break that single-blocked-await wedge
  /// even at cpu:1 (a futex-blocked worker burns no CPU). NEVER derive below 4.
  pub const WORKER_THREADS_FLOOR: usize = 4;

  /// Multiplier applied to ceil(cpu_quota) when scaling UP on bigger hosts.
  /// Defaulted to 1.0; exposed so a future operator preset can tune it.
  pub const DEFAULT_CPU_MULTIPLIER_K: f64 = 1.0;

  #[cfg(test)]
  mod tests {
      #[test]
      fn module_compiles() {
          assert_eq!(super::WORKER_THREADS_FLOOR, 4);
      }
  }
  ```
- [ ] In `elohim/elohim-compute/src/lib.rs`, add the module declaration after `pub mod resources;` (line 13):
  ```rust
  pub mod limits;
  ```
- [ ] In `elohim/elohim-compute/src/lib.rs`, add a SELECTIVE re-export block after the existing `pub use` lines (after line 21). DO NOT glob-export `limits::*` and DO NOT re-export anything named `ResourceSnapshot` (collision with `resources::ResourceSnapshot`):
  ```rust
  pub use limits::{
      detect_resources, derive, AutoPresetView, DerivedConfig, DetectedResources, DeriveOverrides,
  };
  ```
- [ ] Run, expect PASS:
  ```
  cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo test --lib limits::tests::module_compiles
  ```
  Expected (the new symbols don't exist yet, so the re-export will FAIL TO COMPILE):
  ```
  error[E0432]: unresolved import `limits::detect_resources`
  ```
- [ ] Because the re-export references symbols not yet defined, TEMPORARILY comment out the `pub use limits::{...}` block (leave a `// TODO: re-export after Task 6` marker). Re-run the same command, expect PASS:
  ```
  test result: ok. 1 passed; 0 failed; ...
  ```
  (The re-export is restored and verified in Task 6.)
- [ ] Commit:
  ```
  git add elohim/elohim-compute/src/limits.rs elohim/elohim-compute/src/lib.rs
  git commit -m "feat(compute): scaffold limits module for auto-config resource probe

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
  ```

---

## TASK 2 — pure CPU parser, cgroup v2 (`read_cpu_quota_str`)

cgroup v2 `cpu.max` is `"<quota> <period>"`; `"max ..."` → unlimited. Quota cores = quota/period.

Files:
- Modify: `elohim/elohim-compute/src/limits.rs` (add fn + tests in the `#[cfg(test)] mod tests`)

Steps:
- [ ] Write the failing tests. Add inside `mod tests`:
  ```rust
  use super::*;

  #[test]
  fn cpu_v2_one_core() {
      assert_eq!(read_cpu_quota_str("100000 100000"), Some(1.0));
  }
  #[test]
  fn cpu_v2_live_fixture_ten_and_half() {
      // live alpha leaf: cpu.max = "1050000 100000" => 10.5 cores
      assert_eq!(read_cpu_quota_str("1050000 100000"), Some(10.5));
  }
  #[test]
  fn cpu_v2_two_cores() {
      assert_eq!(read_cpu_quota_str("200000 100000"), Some(2.0));
  }
  #[test]
  fn cpu_v2_half_core() {
      assert_eq!(read_cpu_quota_str("50000 100000"), Some(0.5));
  }
  #[test]
  fn cpu_v2_unlimited_is_none() {
      assert_eq!(read_cpu_quota_str("max 100000"), None);
  }
  #[test]
  fn cpu_v2_garbage_is_none() {
      assert_eq!(read_cpu_quota_str("garbage"), None);
      assert_eq!(read_cpu_quota_str(""), None);
      assert_eq!(read_cpu_quota_str("100000 0"), None); // zero period guard
  }
  ```
- [ ] Run it, expect FAIL:
  ```
  cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo test --lib limits::tests::cpu_v2
  ```
  Expected:
  ```
  error[E0425]: cannot find function `read_cpu_quota_str` in this scope
  ```
- [ ] Write minimal implementation. Add to `limits.rs` (outside the test module):
  ```rust
  /// Parse cgroup v2 `cpu.max` contents ("<quota> <period>"). Returns the
  /// fractional core quota, or None when unlimited ("max ...") / malformed.
  pub fn read_cpu_quota_str(cpu_max: &str) -> Option<f64> {
      let mut parts = cpu_max.split_whitespace();
      let quota = parts.next()?;
      let period = parts.next()?;
      if quota == "max" {
          return None; // unlimited
      }
      let quota: f64 = quota.parse().ok()?;
      let period: f64 = period.parse().ok()?;
      if period <= 0.0 {
          return None;
      }
      Some(quota / period)
  }
  ```
- [ ] Run, expect PASS:
  ```
  cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo test --lib limits::tests::cpu_v2
  ```
  Expected: `test result: ok. 6 passed; 0 failed;`
- [ ] Commit:
  ```
  git add elohim/elohim-compute/src/limits.rs
  git commit -m "feat(compute): cgroup v2 cpu.max quota parser

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
  ```

---

## TASK 3 — pure CPU parser, cgroup v1 (`read_cpu_quota_v1_str`)

v1 splits the quota and period across two files; quota `"-1"` → unlimited.

Files:
- Modify: `elohim/elohim-compute/src/limits.rs`

Steps:
- [ ] Write the failing tests. Add inside `mod tests`:
  ```rust
  #[test]
  fn cpu_v1_two_cores() {
      assert_eq!(read_cpu_quota_v1_str("200000", "100000"), Some(2.0));
  }
  #[test]
  fn cpu_v1_unlimited_sentinel() {
      assert_eq!(read_cpu_quota_v1_str("-1", "100000"), None);
  }
  #[test]
  fn cpu_v1_garbage_is_none() {
      assert_eq!(read_cpu_quota_v1_str("x", "100000"), None);
      assert_eq!(read_cpu_quota_v1_str("200000", "0"), None);
  }
  ```
- [ ] Run it, expect FAIL:
  ```
  cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo test --lib limits::tests::cpu_v1
  ```
  Expected:
  ```
  error[E0425]: cannot find function `read_cpu_quota_v1_str` in this scope
  ```
- [ ] Write minimal implementation. Add to `limits.rs`:
  ```rust
  /// Parse cgroup v1 cpu quota: `cfs_quota_us` and `cfs_period_us` contents.
  /// `cfs_quota_us == "-1"` means unlimited.
  pub fn read_cpu_quota_v1_str(quota: &str, period: &str) -> Option<f64> {
      let quota = quota.trim();
      if quota == "-1" {
          return None; // unlimited
      }
      let quota: f64 = quota.parse().ok()?;
      let period: f64 = period.trim().parse().ok()?;
      if quota < 0.0 || period <= 0.0 {
          return None;
      }
      Some(quota / period)
  }
  ```
- [ ] Run, expect PASS:
  ```
  cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo test --lib limits::tests::cpu_v1
  ```
  Expected: `test result: ok. 3 passed; 0 failed;`
- [ ] Commit:
  ```
  git add elohim/elohim-compute/src/limits.rs
  git commit -m "feat(compute): cgroup v1 cpu quota parser

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
  ```

---

## TASK 4 — pure MEM parsers, v2 + v1 (`read_mem_limit_str`, `read_mem_limit_v1_str`)

v2 `memory.max` is decimal bytes or `"max"`. v1 `memory.limit_in_bytes` uses a near-`i64::MAX` sentinel for unlimited. **This is the precondition for ALL mem-derived knobs — the reader lands here but no knob consumes it in this plan; we only EXPOSE `mem_limit_bytes`.**

Files:
- Modify: `elohim/elohim-compute/src/limits.rs`

Steps:
- [ ] Write the failing tests. Add inside `mod tests`:
  ```rust
  #[test]
  fn mem_v2_live_fixture_25gib() {
      // live alpha leaf: memory.max = "26843545600" (= 25 GiB)
      assert_eq!(read_mem_limit_str("26843545600"), Some(26_843_545_600));
  }
  #[test]
  fn mem_v2_hundred_mib() {
      assert_eq!(read_mem_limit_str("104857600"), Some(104_857_600));
  }
  #[test]
  fn mem_v2_unlimited_is_none() {
      assert_eq!(read_mem_limit_str("max"), None);
  }
  #[test]
  fn mem_v2_whitespace_and_garbage() {
      assert_eq!(read_mem_limit_str("  104857600\n"), Some(104_857_600));
      assert_eq!(read_mem_limit_str("notanumber"), None);
  }
  #[test]
  fn mem_v1_normal_and_unlimited_sentinel() {
      assert_eq!(read_mem_limit_v1_str("104857600"), Some(104_857_600));
      // v1 unlimited sentinel ~ i64::MAX page-aligned
      assert_eq!(read_mem_limit_v1_str("9223372036854771712"), None);
  }
  ```
- [ ] Run it, expect FAIL:
  ```
  cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo test --lib limits::tests::mem
  ```
  Expected:
  ```
  error[E0425]: cannot find function `read_mem_limit_str` in this scope
  ```
- [ ] Write minimal implementation. Add to `limits.rs`:
  ```rust
  /// Parse cgroup v2 `memory.max` contents: decimal bytes, or "max" = unlimited.
  pub fn read_mem_limit_str(mem_max: &str) -> Option<u64> {
      let s = mem_max.trim();
      if s == "max" {
          return None; // unlimited
      }
      s.parse::<u64>().ok()
  }

  /// v1 unlimited sentinel: kernels report a near-i64::MAX page-aligned value
  /// for `memory.limit_in_bytes` when no limit is set. Treat huge values as
  /// unlimited (threshold well above any real container ceiling).
  const V1_MEM_UNLIMITED_THRESHOLD: u64 = 1 << 62; // ~4.6 EiB

  /// Parse cgroup v1 `memory.limit_in_bytes`. The near-i64::MAX sentinel = unlimited.
  pub fn read_mem_limit_v1_str(limit_in_bytes: &str) -> Option<u64> {
      let v = limit_in_bytes.trim().parse::<u64>().ok()?;
      if v >= V1_MEM_UNLIMITED_THRESHOLD {
          return None; // unlimited sentinel
      }
      Some(v)
  }
  ```
- [ ] Run, expect PASS:
  ```
  cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo test --lib limits::tests::mem
  ```
  Expected: `test result: ok. 5 passed; 0 failed;`
- [ ] Commit:
  ```
  git add elohim/elohim-compute/src/limits.rs
  git commit -m "feat(compute): cgroup v2+v1 memory.max limit parsers (exposed only)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
  ```

---

## TASK 5 — thin fs wrappers that resolve the LEAF cgroup (`read_cpu_quota`, `read_mem_limit`)

**V5 TRAP:** the limits do NOT live at `/sys/fs/cgroup/cpu.max` (root). The root files don't exist on this k8s node. The process's real values live in its LEAF cgroup; resolve it via `/proc/self/cgroup` (strip the `0::` prefix), join under `/sys/fs/cgroup`. Try v2 leaf first, fall back to v1 paths. These wrappers do I/O so they are NOT pure-unit-tested with fixtures; they delegate parsing to the (already-tested) pure parsers and are smoke-checked by reading the live node.

Files:
- Modify: `elohim/elohim-compute/src/limits.rs`

Steps:
- [ ] Write the smoke test (this asserts the function does not panic and returns a sane shape on the host it runs on — the live node is cgroup v2 with `cpu.max = 1050000 100000`). Add inside `mod tests`:
  ```rust
  #[test]
  fn fs_wrappers_do_not_panic_and_return_sane_values() {
      // Live alpha leaf is v2 with a finite quota; CI hosts vary, so only
      // assert the contract: Some => positive/finite. None is allowed
      // (unlimited or non-Linux).
      if let Some(q) = read_cpu_quota() {
          assert!(q.is_finite() && q > 0.0, "cpu quota must be positive, got {q}");
      }
      if let Some(m) = read_mem_limit() {
          assert!(m > 0, "mem limit must be positive, got {m}");
      }
  }
  ```
- [ ] Run it, expect FAIL:
  ```
  cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo test --lib limits::tests::fs_wrappers
  ```
  Expected:
  ```
  error[E0425]: cannot find function `read_cpu_quota` in this scope
  ```
- [ ] Write minimal implementation. Add to `limits.rs`:
  ```rust
  use std::path::PathBuf;

  /// Resolve the process's leaf cgroup directory under /sys/fs/cgroup.
  /// /proc/self/cgroup line is `0::/kubepods/.../<leaf>` on v2; strip `0::`.
  fn leaf_cgroup_dir() -> Option<PathBuf> {
      let content = std::fs::read_to_string("/proc/self/cgroup").ok()?;
      // v2 unified: a single line beginning `0::`.
      let rel = content
          .lines()
          .find_map(|l| l.strip_prefix("0::"))
          .map(|p| p.trim())?;
      // rel begins with '/', e.g. "/kubepods/burstable/pod.../<id>"
      let rel = rel.trim_start_matches('/');
      Some(PathBuf::from("/sys/fs/cgroup").join(rel))
  }

  /// Read the effective CPU quota (fractional cores) from cgroup.
  /// v2 leaf `cpu.max` first; fall back to v1 `cpu/cpu.cfs_quota_us` + `cpu.cfs_period_us`.
  /// None = unlimited or unreadable (caller treats as "use the floor").
  pub fn read_cpu_quota() -> Option<f64> {
      // v2 leaf
      if let Some(dir) = leaf_cgroup_dir() {
          if let Ok(s) = std::fs::read_to_string(dir.join("cpu.max")) {
              return read_cpu_quota_str(&s);
          }
      }
      // v1 fallback (fixed controller paths)
      let quota = std::fs::read_to_string("/sys/fs/cgroup/cpu/cpu.cfs_quota_us").ok()?;
      let period = std::fs::read_to_string("/sys/fs/cgroup/cpu/cpu.cfs_period_us").ok()?;
      read_cpu_quota_v1_str(&quota, &period)
  }

  /// Read the effective memory ceiling in bytes from cgroup.
  /// v2 leaf `memory.max` first; fall back to v1 `memory/memory.limit_in_bytes`.
  /// None = unlimited or unreadable. EXPOSED only — no knob consumes it in this plan.
  pub fn read_mem_limit() -> Option<u64> {
      if let Some(dir) = leaf_cgroup_dir() {
          if let Ok(s) = std::fs::read_to_string(dir.join("memory.max")) {
              return read_mem_limit_str(&s);
          }
      }
      let limit = std::fs::read_to_string("/sys/fs/cgroup/memory/memory.limit_in_bytes").ok()?;
      read_mem_limit_v1_str(&limit)
  }
  ```
- [ ] Run, expect PASS (on the live alpha node it reads `cpu.max = 1050000 100000` → `Some(10.5)`):
  ```
  cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo test --lib limits::tests::fs_wrappers
  ```
  Expected: `test result: ok. 1 passed; 0 failed;`
- [ ] Commit:
  ```
  git add elohim/elohim-compute/src/limits.rs
  git commit -m "feat(compute): leaf-cgroup-resolving cpu/mem fs wrappers (v2 leaf, v1 fallback)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
  ```

---

## TASK 6 — `DetectedResources` + `detect_resources()`; restore lib.rs re-exports

`detect_resources` reads cgroup CPU+MEM; caller injects disk + peer count (both EXPOSED-only). Then the Task-1 re-export block is restored now that the symbols exist (after this task `derive`/`DerivedConfig`/etc. still don't — so re-export only what exists, complete it in Task 7/9).

Files:
- Modify: `elohim/elohim-compute/src/limits.rs`
- Modify: `elohim/elohim-compute/src/lib.rs`

Steps:
- [ ] Write the failing test. Add inside `mod tests`:
  ```rust
  #[test]
  fn detect_resources_injects_disk_and_peers_and_reads_cgroup() {
      let snap = detect_resources(Some(123), Some(2));
      assert_eq!(snap.disk_free_bytes, Some(123));
      assert_eq!(snap.observed_peer_count, Some(2));
      // cgroup fields are environment-dependent; just assert the shape holds.
      if let Some(q) = snap.cpu_quota {
          assert!(q > 0.0);
      }
  }
  ```
- [ ] Run it, expect FAIL:
  ```
  cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo test --lib limits::tests::detect_resources
  ```
  Expected:
  ```
  error[E0422]: cannot find struct ... `DetectedResources`
  ```
- [ ] Write minimal implementation. Add to `limits.rs`:
  ```rust
  /// Detected node-local resource ceilings — a fresh projection of the live
  /// container (Cat C). cpu_quota/mem_limit_bytes are cgroup reads; disk and
  /// peer count are injected by the caller (the shared crate has no fs4 dep and
  /// no peer registry). All fields are EXPOSED; only cpu_quota is consumed.
  #[derive(Debug, Clone, Serialize, Default)]
  #[serde(rename_all = "camelCase")]
  pub struct DetectedResources {
      pub cpu_quota: Option<f64>,
      pub mem_limit_bytes: Option<u64>,
      pub disk_free_bytes: Option<u64>,
      pub observed_peer_count: Option<usize>,
  }

  /// Probe the live container at BOOT. Reads cgroup CPU + MEM ceilings; the
  /// caller injects disk-free (needs fs4) and observed peer count (runtime state).
  pub fn detect_resources(
      disk_free_bytes: Option<u64>,
      observed_peer_count: Option<usize>,
  ) -> DetectedResources {
      DetectedResources {
          cpu_quota: read_cpu_quota(),
          mem_limit_bytes: read_mem_limit(),
          disk_free_bytes,
          observed_peer_count,
      }
  }
  ```
- [ ] Restore the `lib.rs` re-export block but reference ONLY symbols that exist now (`detect_resources`, `DetectedResources`). Replace the commented `// TODO: re-export after Task 6` block with:
  ```rust
  pub use limits::{detect_resources, DetectedResources};
  // derive/DerivedConfig/DeriveOverrides/AutoPresetView re-exported in Task 9.
  ```
- [ ] Run, expect PASS:
  ```
  cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo test --lib limits::tests::detect_resources
  ```
  Expected: `test result: ok. 1 passed; 0 failed;`
- [ ] Commit:
  ```
  git add elohim/elohim-compute/src/limits.rs elohim/elohim-compute/src/lib.rs
  git commit -m "feat(compute): DetectedResources + detect_resources() boot probe

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
  ```

---

## TASK 7 — `DeriveOverrides`, `DerivedConfig`, pure `derive()` with precedence + reasons

This is the heart. Precedence: **operator-override > Auto-derived > safe-floor**. The ONLY consumed knob is `worker_threads = max(4, ceil(cpu_quota) × k)`; `k` defaults to `DEFAULT_CPU_MULTIPLIER_K (1.0)`. `reasons` is a FIELD of `DerivedConfig` (built inside the pure fn, so it is pure-testable). Mirrors the `min_cpu_budget`/`unwrap_or(derived)` purity idiom from `render/capability.rs:193-196` (override > derived > static), but adds the `max(4, ...)` floor + `×k` multiplier that precedent lacks.

**SCAR (non-negotiable):** the floor applies to the AUTO-DERIVED path only. An explicit operator override wins outright and is NOT floored — today's `DOORWAY_WORKER_THREADS` contract accepts any `v > 0` (incl. 1), and the freeze scar is about *silent cgroup collapse*, not explicit operator choice. The required test matrix locks: `cpu_quota=1.0 → 4`, `cpu_quota=8.0, k=1 → 8`, `override=2 → 2` (beats floor), `cpu_quota=None → 4`.

Files:
- Modify: `elohim/elohim-compute/src/limits.rs`

Steps:
- [ ] Write the failing tests. Add inside `mod tests`:
  ```rust
  fn snap_with_cpu(cpu: Option<f64>) -> DetectedResources {
      DetectedResources { cpu_quota: cpu, ..Default::default() }
  }

  #[test]
  fn derive_cpu_one_hits_floor_four() {
      // THE ORIGINAL FREEZE: cpu:1 must NOT yield 1 worker.
      let d = derive(&snap_with_cpu(Some(1.0)), &DeriveOverrides::default());
      assert_eq!(d.worker_threads, 4);
      assert!(d.reasons.iter().any(|r| r.contains("floor")), "reasons: {:?}", d.reasons);
  }
  #[test]
  fn derive_cpu_eight_scales_to_eight() {
      let d = derive(&snap_with_cpu(Some(8.0)), &DeriveOverrides::default());
      assert_eq!(d.worker_threads, 8);
  }
  #[test]
  fn derive_cpu_ten_and_half_ceils_to_eleven() {
      // live alpha leaf quota 10.5 -> ceil -> 11
      let d = derive(&snap_with_cpu(Some(10.5)), &DeriveOverrides::default());
      assert_eq!(d.worker_threads, 11);
  }
  #[test]
  fn derive_unlimited_cpu_hits_floor() {
      let d = derive(&snap_with_cpu(None), &DeriveOverrides::default());
      assert_eq!(d.worker_threads, 4);
      assert!(d.reasons.iter().any(|r| r.contains("floor")));
  }
  #[test]
  fn derive_override_beats_floor_and_auto() {
      // operator override wins outright, NOT floored (override=2 < floor 4).
      let ov = DeriveOverrides { worker_threads: Some(2) };
      let d = derive(&snap_with_cpu(Some(8.0)), &ov);
      assert_eq!(d.worker_threads, 2);
      assert!(d.reasons.iter().any(|r| r.contains("override")), "reasons: {:?}", d.reasons);
  }
  ```
- [ ] Run it, expect FAIL:
  ```
  cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo test --lib limits::tests::derive
  ```
  Expected:
  ```
  error[E0422]: cannot find struct ... `DeriveOverrides`
  ```
- [ ] Write minimal implementation. Add to `limits.rs`:
  ```rust
  /// Operator overrides (top precedence). worker_threads, when Some, wins
  /// outright and is NOT floored — explicit operator choice is honored even
  /// below the auto-floor (the freeze scar guards against SILENT cgroup
  /// collapse, not against a deliberate operator value).
  #[derive(Debug, Clone, Default, Serialize)]
  #[serde(rename_all = "camelCase")]
  pub struct DeriveOverrides {
      pub worker_threads: Option<usize>,
  }

  /// Boot-time derived runtime config. `reasons` explains the "why" of each
  /// value (pure-built here, so it is unit-testable). worker_threads is the
  /// ONLY consumed knob in this plan; every other future knob will be added
  /// here as COMPUTED + EXPOSED before it is consumed.
  #[derive(Debug, Clone, Serialize)]
  #[serde(rename_all = "camelCase")]
  pub struct DerivedConfig {
      pub worker_threads: usize,
      pub reasons: Vec<String>,
  }

  /// Pure, side-effect-free, re-runnable derivation.
  /// Precedence: operator-override > Auto-derived(ceil(cpu_quota)×k) > floor(4).
  pub fn derive(snapshot: &DetectedResources, overrides: &DeriveOverrides) -> DerivedConfig {
      let k = DEFAULT_CPU_MULTIPLIER_K;
      let mut reasons = Vec::new();

      let worker_threads = if let Some(ov) = overrides.worker_threads {
          reasons.push(format!("worker_threads={ov}: operator override (DOORWAY_WORKER_THREADS)"));
          ov
      } else {
          match snapshot.cpu_quota {
              Some(q) => {
                  let scaled = (q * k).ceil() as usize;
                  if scaled <= WORKER_THREADS_FLOOR {
                      reasons.push(format!(
                          "worker_threads={WORKER_THREADS_FLOOR}: floor (cpu quota {q})"
                      ));
                      WORKER_THREADS_FLOOR
                  } else {
                      reasons.push(format!(
                          "worker_threads={scaled}: auto ceil(cpu quota {q} x k={k})"
                      ));
                      scaled
                  }
              }
              None => {
                  reasons.push(format!(
                      "worker_threads={WORKER_THREADS_FLOOR}: floor (cpu quota unknown/unlimited)"
                  ));
                  WORKER_THREADS_FLOOR
              }
          }
      };

      DerivedConfig { worker_threads, reasons }
  }
  ```
- [ ] Run, expect PASS:
  ```
  cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo test --lib limits::tests::derive
  ```
  Expected: `test result: ok. 5 passed; 0 failed;`
- [ ] Commit:
  ```
  git add elohim/elohim-compute/src/limits.rs
  git commit -m "feat(compute): pure derive() with override>auto>floor precedence + reasons

floor 4 applies to the auto path only; cpu:1 -> 4 (freeze antidote),
cpu:8,k=1 -> 8. Operator override wins outright, never floored.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
  ```

---

## TASK 8 — `AutoPresetView` wire shape

The `/admin/auto-preset` body: `{ resources, derived, overrides, reasons[] }`. `reasons` is surfaced at the top level (duplicate of `derived.reasons`) per the route contract.

Files:
- Modify: `elohim/elohim-compute/src/limits.rs`

Steps:
- [ ] Write the failing test. Add inside `mod tests`:
  ```rust
  #[test]
  fn auto_preset_view_serializes_camel_case_with_reasons() {
      let snap = snap_with_cpu(Some(1.0));
      let ov = DeriveOverrides::default();
      let derived = derive(&snap, &ov);
      let view = AutoPresetView::new(snap, derived, ov);
      let json = serde_json::to_value(&view).unwrap();
      assert!(json.get("resources").is_some());
      assert!(json.get("derived").is_some());
      assert!(json.get("overrides").is_some());
      let reasons = json.get("reasons").unwrap().as_array().unwrap();
      assert!(reasons.iter().any(|r| r.as_str().unwrap().contains("floor")));
      // camelCase check on a nested field
      assert!(json.pointer("/resources/cpuQuota").is_some());
  }
  ```
- [ ] Run it, expect FAIL:
  ```
  cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo test --lib limits::tests::auto_preset_view
  ```
  Expected:
  ```
  error[E0433]: failed to resolve ... `AutoPresetView`
  ```
- [ ] Write minimal implementation. Add to `limits.rs`:
  ```rust
  /// The /admin/auto-preset read-model body (Cat C node-local projection).
  #[derive(Debug, Clone, Serialize)]
  #[serde(rename_all = "camelCase")]
  pub struct AutoPresetView {
      pub resources: DetectedResources,
      pub derived: DerivedConfig,
      pub overrides: DeriveOverrides,
      /// Top-level mirror of derived.reasons for at-a-glance "why".
      pub reasons: Vec<String>,
  }

  impl AutoPresetView {
      pub fn new(
          resources: DetectedResources,
          derived: DerivedConfig,
          overrides: DeriveOverrides,
      ) -> Self {
          let reasons = derived.reasons.clone();
          Self { resources, derived, overrides, reasons }
      }
  }
  ```
- [ ] Run, expect PASS:
  ```
  cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo test --lib limits::tests::auto_preset_view
  ```
  Expected: `test result: ok. 1 passed; 0 failed;`
- [ ] Commit:
  ```
  git add elohim/elohim-compute/src/limits.rs
  git commit -m "feat(compute): AutoPresetView wire shape for /admin/auto-preset

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
  ```

---

## TASK 9 — complete lib.rs re-exports + full-crate green

Now all symbols exist; restore the full selective re-export and verify the whole crate.

Files:
- Modify: `elohim/elohim-compute/src/lib.rs`

Steps:
- [ ] Replace the Task-6 re-export line with the full set (still NO `ResourceSnapshot`-named export):
  ```rust
  pub use limits::{
      derive, detect_resources, AutoPresetView, DerivedConfig, DetectedResources, DeriveOverrides,
  };
  ```
- [ ] Run the WHOLE crate, expect PASS (38 prior + the new limits tests):
  ```
  cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo test --lib
  ```
  Expected: `test result: ok.` with the count = 38 + (the new limits tests) and `0 failed`.
- [ ] Run clippy on the crate, expect clean:
  ```
  cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-clippy RUSTC_WRAPPER="" cargo clippy --lib -- -D warnings
  ```
  Expected: `Finished` with no warnings.
- [ ] Commit:
  ```
  git add elohim/elohim-compute/src/lib.rs
  git commit -m "feat(compute): re-export limits API (derive/detect_resources/views)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
  ```

---

## TASK 10 — wire doorway worker_threads to `derive()` at boot (the Auto foundation, wired into doorway) + rewrite the scar prose

Replace the const fallback in `doorway/doorway-service/src/main.rs`. Env override (`DOORWAY_WORKER_THREADS`) STILL WINS — it now feeds `DeriveOverrides.worker_threads`, so the resolver becomes: env override → else `derive(detect_resources(...))`. Rewrite the THREE prose sites (V2: lines 44–46 const comment, 137–139 async_main comment, line 140 log `"(explicit)"`) — re-express the scar as "floored at 4 then CPU-scaled," do NOT delete it. Do NOT touch line 136 `"Workers: {}"` (unrelated WorkerPool).

Files:
- Modify: `doorway/doorway-service/src/main.rs` (const+comment 30–47; resolver 49–55; runtime build 57–75; async_main signature ~77; log 137–140)

Steps:
- [ ] Read `doorway/doorway-service/src/main.rs` lines 30–145 to confirm current text before editing.
- [ ] Rewrite the const doc-comment (lines 30–47). Keep `const DEFAULT_WORKER_THREADS: usize = 4;` (it is the floor). Replace the "never CPU-derived" prose:
  ```rust
  /// Floor for tokio worker threads. The 2026-06-13 doorway freeze: the pod has
  /// `resources.limits.cpu: 1`, so the default runtime spun a SINGLE worker, and
  /// one synchronously-blocked await on it froze the whole gateway (`/health`
  /// included) so kubelet's restart-on-hang never fired. A futex-blocked worker
  /// burns no CPU, so several workers break the single-blocked-await wedge even
  /// at `cpu: 1`.
  ///
  /// We FLOOR the count at 4 so the cgroup can never silently collapse us to one
  /// worker, then scale UP by the cgroup CPU quota (`ceil(cpu.max quota) x k`)
  /// on bigger hosts. An explicit `DOORWAY_WORKER_THREADS` override always wins
  /// (operator > auto > floor) and is honored even below the floor.
  const DEFAULT_WORKER_THREADS: usize = 4;
  ```
- [ ] Replace the env-only resolver (lines 49–55) with override-feeds-derive:
  ```rust
  /// Resolve the tokio worker count: operator override > auto-derived > floor(4).
  /// `DOORWAY_WORKER_THREADS` (parseable usize > 0) feeds the override slot, so an
  /// explicit operator value still wins; otherwise the cgroup-derived value is used.
  fn worker_threads() -> elohim_compute::DerivedConfig {
      let override_threads = std::env::var("DOORWAY_WORKER_THREADS")
          .ok()
          .and_then(|v| v.parse::<usize>().ok())
          .filter(|&v| v > 0);
      // disk-free + peer-count probes are EXPOSED-only follow-ons; pass None.
      let snapshot = elohim_compute::detect_resources(None, None);
      let overrides = elohim_compute::DeriveOverrides { worker_threads: override_threads };
      elohim_compute::derive(&snapshot, &overrides)
  }
  ```
- [ ] Update `main()` (lines 57–75) to use the derived count and pass the full `DerivedConfig` through so the boot projection can be stashed. Change the `let workers = worker_threads();` region to:
  ```rust
      let derived = worker_threads();
      let workers = derived.worker_threads;
      let runtime = tokio::runtime::Builder::new_multi_thread()
          .worker_threads(workers)
          .enable_all()
          .thread_name("doorway-tokio-w")
          .build()
          .expect("failed to build tokio runtime");
      runtime.block_on(async_main(derived))
  ```
  (Keep `dotenvy::dotenv()` BEFORE this block — the env-ordering invariant from V2.)
- [ ] Change `async_main`'s signature (line ~77) from `worker_threads: usize` to `derived: elohim_compute::DerivedConfig`, and update its first uses. At the top of `async_main`, derive the count for logging: `let worker_threads = derived.worker_threads;`.
- [ ] Rewrite the startup log (lines 137–140), re-expressing the scar and surfacing the reason:
  ```rust
      // Tokio worker threads: floored at 4 so a cpu-limited cgroup can never
      // collapse the runtime to a single worker (the 2026-06-13 freeze), then
      // scaled up by the cgroup CPU quota. See DEFAULT_WORKER_THREADS / derive().
      info!("Tokio worker threads: {} ({})", worker_threads,
          derived.reasons.first().map(String::as_str).unwrap_or("derived"));
  ```
  Do NOT touch line 136 `info!("Workers: {}", args.worker_count);`.
- [ ] Verify the crate compiles (this also exercises the type wiring). Expect PASS:
  ```
  cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-check RUSTC_WRAPPER="" cargo build --bins
  ```
  Expected: `Finished` with no errors. (If `elohim_compute` symbols are unresolved, confirm `doorway-service/Cargo.toml:138` already has `elohim-compute = { path = "../../elohim/elohim-compute" }` — it does per V7; no Cargo.toml edit needed.)
- [ ] Commit:
  ```
  git add doorway/doorway-service/src/main.rs
  git commit -m "fix(doorway): derive tokio worker_threads from cgroup at boot (floor 4, CPU-scaled)

THE PRIME FIX. worker_threads = max(4, ceil(cpu quota) x k); DOORWAY_WORKER_THREADS
still wins (operator > auto > floor). Re-expresses the 2026-06-13 freeze scar as
floored-then-scaled rather than 'never CPU-derived'.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
  ```

---

## TASK 11 — stash the boot AutoPresetView in AppState

The read-only `/admin/auto-preset` handler needs the boot projection. Build the `AutoPresetView` ONCE in `async_main` (boot only — no re-derivation) and store it on `AppState`.

Files:
- Modify: the `AppState` definition file (find it: `grep -rn "pub struct AppState" doorway/doorway-service/src`). Likely `doorway/doorway-service/src/server/state.rs`.
- Modify: `doorway/doorway-service/src/main.rs` (where `AppState` is constructed in `async_main`).

Steps:
- [ ] Locate AppState:
  ```
  cd /projects/elohim/doorway/doorway-service && grep -rn "pub struct AppState" src
  ```
  Read the struct and its construction site.
- [ ] Add a field to `AppState` (after an existing simple field; mirror how `render_trace_stats` is held):
  ```rust
      /// Boot-time Auto-preset projection (Cat C node-local; computed once at boot).
      pub auto_preset: elohim_compute::AutoPresetView,
  ```
- [ ] In `async_main`, BEFORE constructing `AppState`, rebuild the view from the already-derived config (re-running `detect_resources` once is fine; it is the same boot, no re-derivation surface is exposed):
  ```rust
      // Cat C node-local projection of the live container — built ONCE at boot.
      let auto_preset = {
          let snapshot = elohim_compute::detect_resources(None, None);
          let overrides = elohim_compute::DeriveOverrides {
              worker_threads: std::env::var("DOORWAY_WORKER_THREADS")
                  .ok()
                  .and_then(|v| v.parse::<usize>().ok())
                  .filter(|&v| v > 0),
          };
          elohim_compute::AutoPresetView::new(snapshot, derived.clone(), overrides)
      };
  ```
  (Requires `derived` to still be in scope and `DerivedConfig: Clone` — it derives `Clone` per Task 7. If `derived` was moved into `async_main` by value, that is exactly the case — clone it here.)
- [ ] Add `auto_preset,` to the `AppState { ... }` constructor.
- [ ] Verify compile, expect PASS:
  ```
  cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-check RUSTC_WRAPPER="" cargo build --bins
  ```
  Expected: `Finished`.
- [ ] Commit:
  ```
  git add doorway/doorway-service/src/server/state.rs doorway/doorway-service/src/main.rs
  git commit -m "feat(doorway): stash boot AutoPresetView on AppState (Cat C, boot-only)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
  ```

---

## TASK 12 — `handle_admin_auto_preset` handler

Mirror `handle_admin_render_stats` (`routes/admin.rs:986`) exactly — same signature, `json_response(StatusCode::OK, ...)`.

Files:
- Modify: `doorway/doorway-service/src/routes/admin.rs` (beside `handle_admin_render_stats` at line 986)

Steps:
- [ ] Write the failing test. Add to the `#[cfg(test)] mod tests` in `admin.rs` (if absent, add a gated module). The test asserts the handler returns 200 and a JSON body with the four keys. Use the existing test helpers for `AppState` construction (grep for how other admin handler tests build state — e.g. a `test_state()` helper; if none exists, this test asserts at the type level via a small constructed `AutoPresetView` and `json_response`). Minimal version that does not need full AppState:
  ```rust
  #[tokio::test]
  async fn auto_preset_handler_returns_200_json() {
      // build a minimal view directly to exercise json_response shaping
      let snap = elohim_compute::detect_resources(None, None);
      let ov = elohim_compute::DeriveOverrides::default();
      let derived = elohim_compute::derive(&snap, &ov);
      let view = elohim_compute::AutoPresetView::new(snap, derived, ov);
      let resp = json_response(hyper::StatusCode::OK, &view);
      assert_eq!(resp.status(), hyper::StatusCode::OK);
      assert_eq!(
          resp.headers().get("Content-Type").unwrap(),
          "application/json"
      );
  }
  ```
- [ ] Run it, expect FAIL (the test references `json_response` with a borrow; if `json_response` takes `T` by value adjust to pass `view` by value — match its real signature at `admin.rs:990`, which is `fn json_response<T: Serialize>(status, body: T)`):
  ```
  cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-check RUSTC_WRAPPER="" cargo test --lib routes::admin::tests::auto_preset_handler
  ```
  Expected on first run: a compile error if `json_response` takes `T` by value and you passed `&view` — fix to `json_response(hyper::StatusCode::OK, view)`. The conceptual FAIL is "handler doesn't exist yet" once you also assert via the handler; keep this test as the json-shape lock.
- [ ] Write the handler. Add to `admin.rs` right after `handle_admin_render_stats` (line 988):
  ```rust
  /// GET /admin/auto-preset — Cat C node-local read-model: detected cgroup
  /// resources + boot-derived runtime config + the "why" reasons. Computed once
  /// at boot (no re-derivation); this is a fresh projection of the live container,
  /// NOT a storage proxy (legitimate doorway-resident Operational state).
  pub async fn handle_admin_auto_preset(state: Arc<AppState>) -> Response<Full<Bytes>> {
      json_response(StatusCode::OK, &state.auto_preset)
  }
  ```
  (If `json_response` takes `T: Serialize` by value, pass `state.auto_preset.clone()` — `AutoPresetView: Clone` per Task 8.)
- [ ] Confirm `routes/mod.rs` re-exports admin handlers via `routes::` (the existing `routes::handle_admin_render_stats` reference in http.rs proves it does — `pub use admin::*` or explicit). If explicit, add `handle_admin_auto_preset` to the list.
- [ ] Run, expect PASS:
  ```
  cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-check RUSTC_WRAPPER="" cargo test --lib routes::admin::tests::auto_preset_handler
  ```
  Expected: `test result: ok. 1 passed;`
- [ ] Commit:
  ```
  git add doorway/doorway-service/src/routes/admin.rs doorway/doorway-service/src/routes/mod.rs
  git commit -m "feat(doorway): handle_admin_auto_preset handler (Cat C read-model)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
  ```

---

## TASK 13 — register `GET /admin/auto-preset` match arm

Add the arm after `/admin/render-stats` (V4: http.rs line ~2339), above the registry wildcard. **No `is_service_path` change and no `build_manifest()` registration needed** — `/admin/*` are explicit match arms, and `is_service_path` already returns true for any `/admin` prefix (http.rs line 1301), so the request reaches the main match block. State this in the arm comment so the reasoning is not re-litigated.

Files:
- Modify: `doorway/doorway-service/src/server/http.rs` (insert after line 2339)

Steps:
- [ ] Read `doorway/doorway-service/src/server/http.rs` lines 2330–2345 to confirm the exact surrounding text and the `to_boxed(routes::...)` shape.
- [ ] Add the arm immediately after the `/admin/render-stats` arm (after line 2339), before `/admin/conductors`:
  ```rust
          // Cat C node-local read-model: cgroup resources + boot-derived config.
          // Explicit /admin arm (not registry/storage-proxy) — is_service_path
          // already covers the /admin prefix, so no manifest registration needed.
          (Method::GET, "/admin/auto-preset") => {
              to_boxed(routes::handle_admin_auto_preset(Arc::clone(&state)).await)
          }
  ```
- [ ] Build the bins, expect PASS:
  ```
  cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-check RUSTC_WRAPPER="" cargo build --bins
  ```
  Expected: `Finished`.
- [ ] Run the full doorway lib test suite to confirm no regression:
  ```
  cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-check RUSTC_WRAPPER="" cargo test --lib 2>&1 | tail -5
  ```
  Expected: `test result: ok. ... 0 failed;` (≥331 tests).
- [ ] Run clippy on doorway, expect clean:
  ```
  cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-clippy RUSTC_WRAPPER="" cargo clippy --lib --bins -- -D warnings
  ```
  Expected: `Finished` with no warnings.
- [ ] Commit:
  ```
  git add doorway/doorway-service/src/server/http.rs
  git commit -m "feat(doorway): register GET /admin/auto-preset arm

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
  ```

---

## TASK 14 — storage boot probe + import_rt wiring (item 6 decision)

**Decision (stated outright):** storage runs TWO runtimes — `server_rt` (worker_threads=2, intentionally small/responsive) and `import_rt` (worker_threads=4, heavy zome work). Wire `import_rt` to the derived value (floor 4 preserved, so it never regresses below today's 4); LEAVE `server_rt=2` as-is with a comment (a small always-responsive HTTP runtime is correct and below the freeze-floor concern only because the gateway freeze was a doorway single-worker issue, not storage). **Serving `/admin/auto-preset` from storage is a named ONE-LINE follow-on** — storage's HTTP surface is `elohim/elohim-storage/src/http.rs` with a different dispatch shape; wiring it there is out of scope for this plan.

Files:
- Modify: `elohim/elohim-storage/src/main.rs` (lines 258–272 region — `server_rt`/`import_rt` builders)

Steps:
- [ ] Read `elohim/elohim-storage/src/main.rs` lines 245–290 to confirm current text (the `server_rt` worker_threads(2) at line 260 and `import_rt` worker_threads(4) at line 268).
- [ ] Before the `import_rt` builder, add the boot probe + derive:
  ```rust
      // Cat C node-local boot probe: derive the import runtime's worker count
      // from the cgroup CPU quota (floor 4 preserved). server_rt stays small.
      let import_derived = {
          let snapshot = elohim_compute::detect_resources(None, None);
          let overrides = elohim_compute::DeriveOverrides {
              worker_threads: std::env::var("STORAGE_IMPORT_WORKER_THREADS")
                  .ok()
                  .and_then(|v| v.parse::<usize>().ok())
                  .filter(|&v| v > 0),
          };
          elohim_compute::derive(&snapshot, &overrides)
      };
      tracing::info!(
          import_workers = import_derived.worker_threads,
          reason = %import_derived.reasons.first().map(String::as_str).unwrap_or("derived"),
          "elohim-storage import runtime worker threads"
      );
  ```
- [ ] Change the `import_rt` builder line `.worker_threads(4)` (line 268) to:
  ```rust
          .worker_threads(import_derived.worker_threads)
  ```
- [ ] Add a comment above `server_rt` (line 259) and leave its `worker_threads(2)` unchanged:
  ```rust
      // server_rt stays small (2) on purpose: always-responsive HTTP/WebSocket.
      // The doorway freeze was a single-worker GATEWAY wedge; storage's HTTP
      // runtime at 2 is above the single-blocked-await floor. Not auto-derived.
  ```
- [ ] If the `import_workers = 4` literal in the startup log (line ~279) is still hardcoded, update it to `import_workers = import_derived.worker_threads`.
- [ ] Build storage (custom getrandom flag — LINK-time), expect PASS:
  ```
  cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo build --bins
  ```
  Expected: `Finished`. (Confirm `elohim/elohim-storage/Cargo.toml:48` has `elohim-compute = { path = "../elohim-compute" }` — it does per V7; no Cargo.toml edit.)
- [ ] Run storage system_metrics tests to confirm no link-time regression:
  ```
  cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo test --lib system_metrics 2>&1 | tail -5
  ```
  Expected: `test result: ok. ... 0 failed;`
- [ ] Commit:
  ```
  git add elohim/elohim-storage/src/main.rs
  git commit -m "feat(storage): derive import_rt worker_threads from cgroup at boot (floor 4)

server_rt stays 2 (intentional). /admin/auto-preset on storage is a follow-on.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
  ```

---

## Out of scope (named follow-on plans — do NOT implement here)

1. **Real `disk_free_bytes` probe** — wire `fs4::available_space` (or `filesystem_capacity_bytes`) into the caller and pass it to `detect_resources` instead of `None`. (Doorway has no fs4 dep today; this is a caller-side follow-on.)
2. **Real `observed_peer_count` probe** — read doorway's `AppState` peer registry / storage's swarm and inject it.
3. **mem-derived knobs** — cache size, per-structure MEMORY budgets, warm-up budget. The mem reader LANDS in this plan but no knob consumes it; deriving mem knobs is unsafe until proven. This plan only EXPOSES `mem_limit_bytes`.
4. **Warm-up budget / timeout consumers** and **inbound admission** — separate control-plane pillars.
5. **Runtime re-derivation on cgroup change** — this plan is BOOT-only. A watcher/re-derive surface is explicitly out.
6. **REA delegates-compute actuation model** — no runtime mutation surface; knobs stay operator-env + boot-Auto.
7. **`GET /admin/auto-preset` served from elohim-storage** — trivial now that the shared crate owns the view, but storage's HTTP dispatch differs; one-line follow-on.
8. **Serving the derived `k` multiplier as an operator preset** — `DEFAULT_CPU_MULTIPLIER_K` is exposed but not yet operator-tunable.
9. **Richer unlimited-host detection** — when `cpu_quota` is `None` (unlimited / `"max"`), this plan floors to 4 rather than deriving from `available_parallelism()`/host cores. A host-core-aware unlimited path is a follow-on.

---

## SELF-REVIEW (run before declaring done)

### Spec coverage (every IN-scope item maps to a task)
- IN (1) cgroup readers, v2 + v1, pure-parse + thin fs wrapper, V5 fixtures → Tasks 2,3,4 (parsers + fixtures) and 5 (leaf-resolving fs wrappers). ✓
- IN (2) `DetectedResources` (substitute for scope's `ResourceSnapshot`) + `detect_resources()` → Task 6. ✓ (Naming deviation documented at top.)
- IN (3) `derive(snapshot, overrides) -> DerivedConfig`, precedence override > Auto > floor, only worker_threads consumed, every other field computed+exposed not consumed → Task 7. (No other DerivedConfig fields exist yet — none are consumed; future knobs land as computed+exposed first.) ✓
- IN (4) doorway worker_threads wired to derive() at boot, DOORWAY_WORKER_THREADS preserved → Task 10. ✓
- IN (5) `GET /admin/auto-preset` returning `{ resources, derived, overrides, reasons[] }` with explanatory reasons → Tasks 8 (view), 11 (stash), 12 (handler), 13 (route). ✓
- IN (6) storage boot detect_resources() — V7 shows shared crate makes it trivial → Task 14 wires import_rt; serving the route from storage named as follow-on. ✓

### Hard constraints / scars
- worker_threads floor is 4; `cpu_quota=1.0 → 4` AND `cpu_quota=8.0,k=1 → 8` tested → Task 7 tests `derive_cpu_one_hits_floor_four`, `derive_cpu_eight_scales_to_eight`. ✓ (Plus `10.5 → 11` and `override=2 → 2` and `None → 4`.)
- No silent re-derivation; detect/derive at BOOT only → Tasks 10/11/14 build the view once at boot; no watcher; Out-of-scope item 5. ✓
- p2p-class Cat C node-local; no DHT entry, no DB table, no coordinator fn → stated in Architecture, limits.rs module doc, handler doc, arm comment. ✓
- mem reader handles host-RAM-vs-cgroup correctly; does NOT reuse `total_memory_bytes` (host RAM per V1); reads `memory.max` → Task 4/5 read cgroup `memory.max`/`memory.limit_in_bytes`, never libc sysinfo. ✓

### Placeholder scan
- Every code step shows REAL code (no `// ...`/`TODO` in shipped impl except the intentional Task-1 re-export marker that is removed in Task 6/9).
- Every command is the real env-prefixed command (RUSTFLAGS + `/tmp` CARGO_TARGET_DIR + `RUSTC_WRAPPER=""`, plain `cargo test`, NO nextest).
- The elohim-compute test command is empirically VERIFIED green (38 passed) before this plan was written.
- `detect_resources` takes injected disk/peer args (no placeholder values inside the dep-free crate); doorway passes `None` (documented EXPOSED-only).

### Type/function-name consistency across ALL tasks
- `DetectedResources` — Tasks 6,7,8,10,11,12,14 (struct, fn returns, view field, callers). Consistent. Fields: `cpu_quota`, `mem_limit_bytes`, `disk_free_bytes`, `observed_peer_count` — used identically in Task 6 def and Task 8 view. ✓
- `detect_resources(Option<u64>, Option<usize>)` — Tasks 6,10,11,14 call with `(None, None)` (doorway/storage) and `(Some(123), Some(2))` (test). Signature identical. ✓
- `derive(&DetectedResources, &DeriveOverrides) -> DerivedConfig` — Tasks 7,8,10,11,14. Borrow signature identical everywhere. ✓
- `DeriveOverrides { worker_threads: Option<usize> }` — Tasks 7,8,10,11,14. ✓
- `DerivedConfig { worker_threads: usize, reasons: Vec<String> }` — Tasks 7,8,10,11. Derives `Clone` (needed in Tasks 10/11). ✓
- `read_cpu_quota`/`read_mem_limit` (fs wrappers) — Task 5 def, called only by `detect_resources` (Task 6). ✓
- `read_cpu_quota_str`/`read_cpu_quota_v1_str`/`read_mem_limit_str`/`read_mem_limit_v1_str` (pure parsers) — Tasks 2,3,4 def, called by Task 5 wrappers. ✓
- `AutoPresetView { resources, derived, overrides, reasons }` — Task 8 def; Tasks 11,12 use; derives `Clone`. ✓
- NO type named `ResourceSnapshot` introduced (collision avoided per top-of-plan deviation); lib.rs re-export (Task 9) excludes it. ✓
