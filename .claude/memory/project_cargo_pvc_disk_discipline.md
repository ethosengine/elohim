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
