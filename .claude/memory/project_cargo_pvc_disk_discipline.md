---
name: project_cargo_pvc_disk_discipline
title: Cargo/PVC disk + native build-env discipline (umbrella)
description: "Cargo disk + build-env: act at 85% PVC, cargo-pool reclaim, disk-guard hook; CARGO_TARGET_DIR per workspace, no nextest here, sweettest needs RUSTFLAGS=\"\"."
metadata:
  type: project
---

# Cargo / PVC disk-pressure discipline (umbrella)

Folds the disk-pressure and cargo-target-pool discipline cluster. Members:

- [[feedback_multi_agent_pvc_pacing]] — never run cargo test/build concurrently across agents; shared target-pool locks + PVC disk/RAM contention crash builds.
- [[feedback_pvc_threshold_and_recovery]] — 118G PVC: above 85% used, act; cargo-pool legacy-targets --clean --yes reclaims ~25-35G; check df <80% before dispatching any cargo agent.
- [[project_devspace_disk_cleanup_procedure]] — pool families dominate disk pressure; act at 85%+; reclaim ladder ends in operator-gated family prune — never prune the active family mid-push.
- [[project_cargo_disk_guard_override]] — at the 85% hard-ceiling the PreToolUse hook DENIES heavy cargo; FORCE_HEAVY_GATES does not bypass it — free non-pool space or bump volume_hard_pct.
- [[project_rust_build_footprint_anatomy]] — 71% of pool = ~1GB DWARF test binaries (79% debuginfo); retention policy not Rust is the cause; evict first; −57% profile landed in root .cargo/config.toml.
- [[feedback_cargo_target_dir_for_native_builds]] — Native (non-WASM) cargo builds need CARGO_TARGET_DIR at the pool slot per workspace; forgotten legacy target/ dirs balloon to ~30G. WASM workspaces stay default.
- [[project_container_cargo_environment_quirks]] — No nextest here (plain cargo test, unpiped exit codes); /projects target dirs ENOENT → use /tmp; WASM-warmed slot corrupts native builds; explicit cd per gate.
- [[project_sweettest_native_build_env]] — Sweettest needs RUSTFLAGS="" (WASM getrandom flag breaks native link), BINDGEN_EXTRA_CLANG_ARGS for clang-21; `just pack` (not build) refreshes the .dna bundle.

**2026-08-25 — one slot, two cargo invocations = E0460 doctest red (not a code failure).** `just gate
elohim-storage` and a concurrent `cargo build --features "p2p p2p-iroh"` in the SAME pool slot serialize
on the build-dir lock, but the gate's DOCTEST step then fails `error[E0460]: found possibly newer version
of crate elohim_cache_core` because the feature-different build rewrote rlibs between the test build and
rustdoc. Unit/integration suites had already passed. Re-run the gate alone on a quiet slot (it went green
first try). Also: the gate's `cargo test` re-links `debug/elohim-storage` with DEFAULT features, so
always re-run the feature-full build AFTER the gate before `just mesh start` (start refuses otherwise).

**2026-08-28 — the pool is a CLUSTER-STABILITY control, not just disk hygiene.** The workspace PV
(`storage-workspace836e1cb3b3d7457c`) lives on the root fs of ethosengine, the host running the only k8s
control plane. Netconsole trace (23:00Z): `kswapd0` soft-locked in `ext4_es_scan → __es_shrink →
es_reclaim_extents` (extent-status shrinker; cost ∝ inode count + fragmentation), 14 CPUs followed, API
down for hours — 17th freeze, all 17 in 19:00–05:00 UTC (= devworkspace working hours). The pool was
341K of the PV's ~1.02M inodes (126G), ≈47% of ALL root-fs inodes came from this one PV.
Two enforcement gaps let it grow: hash-gc (step 2) and the keep-warm cap (step 4) both skipped the
*protected* (active) family — `dev` sat at 96G under a 60G cap for weeks. Fixed in `pool-lib.sh`: both
now apply slot-granularly to the active family with guards `slot_in_use` (flock OR a live process
exec'ing from the slot — the mesh runs out of its slots), `slot_pinned` (`touch <slot>/.pin`; the fork
conductor slot is pinned), linked slots, and freshest-per-crate within `keep_warm_slot_days` (7).
Operator asks, in the devfile: `attributes.pod-overrides.spec.securityContext.fsGroupChangePolicy:
OnRootMismatch` (default Always chowns 1.02M files every start); a HELD, commented separate-PVC block
(`cargo-target-pool`) waits on the operator provisioning the claim. Success criteria they watch:
48h freeze-free, `ext4_inode_cache` < ~1M objects (readable in-container: `grep ext4_inode_cache
/proc/slabinfo`), no kswapd lockup on `intel-nuc:/var/log/netconsole-ethosengine.log`.
sccache: dead Aug 23–28 (Garage bucket+keys wiped), restored 2026-08-28 — probe showed 0 errors, 0.10 s
write, 6 ms hit; dev-profile *incremental* crates are non-cacheable by design (that's the old 25% hit
rate), deps cache fine. `RUSTC_WRAPPER` stays '' per the 2026-07-03 decision; flip is one line.
A concurrent session's `cargo test` correctly makes the family `busy` — wait, never override.
Measured on the first apply (2026-08-28 23:57Z): pool 125G→87G, pool inodes 341,508→202,513,
host `ext4_inode_cache` 1,137,138→991,724. Enforce reported "freed 0B" for keep-warm trims — fixed.
- 2026-08-29 post-restart (workspace recreated from devfile): `fsGroupChangePolicy: OnRootMismatch` present in `$DEVWORKSPACE_FLATTENED_DEVFILE` (`/devworkspace-metadata/flattened.devworkspace.yaml` — the in-pod way to confirm a pod-override landed without kubectl). Host `ext4_inode_cache` 740,556 (was 1,137,138), `/projects` inodes 1,936,009, pool 85G, conductor `.pin` survived. Container-level load avg is the HOST's (25 with nothing running here).
- **2026-08-29 RAM, not disk, killed the workspace:** the pod's memory cgroup is **31 GiB hard**
  (`memory.max`; `/proc/meminfo`/`free` show the 64 GB node and lie). `cargo clippy --all-targets` on
  elohim-storage links the 3.2k-test binaries with ~7 parallel `rust-lld` (~1.5 GB) on top of one
  `rustc` at 4.3 GB; with three mesh conductors (~6–7 GB each) + IDE Java + claude sessions resident,
  the OOM-killer first took background cargo chains (why they "were stopped" all afternoon — highest
  oom_score) and at 17:12 took PID 1 (whole container restart, mesh + /tmp gone). Rules: set
  `CARGO_BUILD_JOBS=4` (or `-j4`) for clippy/test while the mesh is up; never run the two feature-set
  clippies back-to-back with conductors resident; read `/sys/fs/cgroup/.../memory.current` vs
  `memory.max` before heavy cargo. A RAM guard mirroring the disk-guard (soft ~24 GB kill newest
  rustc/lld + marker, hard ~28 GB PreToolUse DENY) is designed but not built.

## 2026-08-29 — it is I/O on worn NVMe, not memory; the PVC idea is retired

- Operator's second report: both 2026-08-29 outages were HARD RESETS (firmware "internal CPU shutdown event", md2 dirty) with psi_io some=77%, 45 D-state procs, md2 queue depth 218–277 at 155 ms write latency; both Crucial P1 (QLC) mirror members are past rated TBW (216/260 TB vs 200). Kernel 7.0 was an accidental bump (GRUB_DEFAULT=0) and is not a fix. The freeze-flight-recorder's `sync -f` (syncfs of the whole fs every 5 s) was noise, since fixed by the operator.
- **Separate cargo-pool PVC is RETIRED, never provision it:** openebs-hostpath is a directory on the same /dev/md2p1; both NVMes fully partitioned. Only a physical disk moves the pool off the root fs. Devfile now carries the rationale instead of the held block.
- **In-pod attribution rail (no kubectl):** `/sys/fs/cgroup` in this container is the HOST's root cgroup2 tree — `kubepods/*/pod<uid>/io.stat` (device 9:2 = md2) gives per-pod write bytes; `/proc/pressure/io` is host PSI. Measured idle floor ~13 MB/s ≈ 1.1 TB/day: four other pods 1–3 MB/s each (alpha peers, 4G/8G limits), containerd 2.2 MB/s (container stdout logs), dqlite 1.6 MB/s, this workspace ~1 MB/s idle (3 conductors + 3 storage + 2 doorways ≈ 130 KB/s + 0.5–1 MB/s each). This pod wrote 30 GB in its first 55 min (45% of host) — builds are the bursts; steady writers are mostly not ours. `/proc/<pid>/io` write_bytes includes REAPED children (hooks, tool subshells), so a claude process can "write" MB/s.
- **Guard rules 3–5 in `cargo-disk-guard.py`** (policy `io` in pool-policy.json): host psi_io deny (full10≥20 / some60≥50), ONE BUILD AT A TIME per container (scans /proc for cargo/rustc/rust-lld/gate-runner), JOBS CAP (deny heavy cargo/`just gate` without -j or CARGO_BUILD_JOBS; devfile sets CARGO_BUILD_JOBS=4 for new workspaces). Test seams: CARGO_GUARD_PSI_FILE, CARGO_GUARD_PROC_DIR. Bash rule: `bc` is not installed — use awk for float math.
- Not yet gated: node builds (`ng build`, vitest, playwright) write GBs too — ram-guard's `is_heavy` already classifies them; extend the io rules there if bursts persist. Don't leave the local mesh running idle for days (≈100–250 GB/day of writes).

**bindgen in this container lacks clang's builtin headers** (measured 2026-08-30): any crate whose
build script runs bindgen over C headers (`datachannel-sys` via the sweettest workspace's
`datachannel-vendored` feature) dies with `fatal error: 'stdbool.h' file not found`. Fix:
`export BINDGEN_EXTRA_CLANG_ARGS="-I/usr/lib/clang/21/include"` (clang 21's resource dir; re-derive
with `clang -print-resource-dir` after toolchain bumps). With it, `cargo check --tests` in
`elohim/holochain/tests/sweettest` passes locally.
